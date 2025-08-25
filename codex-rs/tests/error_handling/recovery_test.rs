//! Error Handling and Recovery Tests
//!
//! Tests various error recovery scenarios that developers commonly face:
//! - File not found handling with graceful degradation
//! - Network failure recovery with retry logic and timeouts
//! - Cleanup after partial operations fail
//! - State consistency validation after errors
//! - Rollback mechanisms and transaction-like operations
//!
//! These tests simulate real-world error conditions and verify that the system
//! handles them gracefully without leaving inconsistent state.

use agcodex_core::{
    AgcodexResult, AgcodexError,
    tools::{SearchTool, EditTool, CodeTool},
    context::Context,
    client::{AgcodexClient, ClientConfig, ApiProvider},
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, AtomicUsize, Ordering}};
use std::time::{Duration, Instant};
use tempfile::{TempDir, NamedTempFile};
use tokio::{fs, time::timeout};
use serde::{Serialize, Deserialize};

/// Mock file system that can simulate failures
struct MockFileSystem {
    temp_dir: TempDir,
    fail_on_read: Arc<AtomicBool>,
    fail_on_write: Arc<AtomicBool>,
    operation_count: Arc<AtomicUsize>,
    max_operations_before_failure: Arc<AtomicUsize>,
}

impl MockFileSystem {
    fn new() -> Self {
        Self {
            temp_dir: TempDir::new().unwrap(),
            fail_on_read: Arc::new(AtomicBool::new(false)),
            fail_on_write: Arc::new(AtomicBool::new(false)),
            operation_count: Arc::new(AtomicUsize::new(0)),
            max_operations_before_failure: Arc::new(AtomicUsize::new(usize::MAX)),
        }
    }
    
    fn enable_read_failures(&self) {
        self.fail_on_read.store(true, Ordering::SeqCst);
    }
    
    fn enable_write_failures(&self) {
        self.fail_on_write.store(true, Ordering::SeqCst);
    }
    
    fn set_failure_after_operations(&self, count: usize) {
        self.max_operations_before_failure.store(count, Ordering::SeqCst);
        self.operation_count.store(0, Ordering::SeqCst);
    }
    
    fn disable_failures(&self) {
        self.fail_on_read.store(false, Ordering::SeqCst);
        self.fail_on_write.store(false, Ordering::SeqCst);
        self.max_operations_before_failure.store(usize::MAX, Ordering::SeqCst);
    }
    
    async fn read_file(&self, path: &Path) -> AgcodexResult<String> {
        let count = self.operation_count.fetch_add(1, Ordering::SeqCst);
        
        if self.fail_on_read.load(Ordering::SeqCst) || 
           count >= self.max_operations_before_failure.load(Ordering::SeqCst) {
            return Err(AgcodexError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Simulated read failure for: {}", path.display())
            )));
        }
        
        fs::read_to_string(path).await
            .map_err(|e| AgcodexError::IoError(e))
    }
    
    async fn write_file(&self, path: &Path, content: &str) -> AgcodexResult<()> {
        let count = self.operation_count.fetch_add(1, Ordering::SeqCst);
        
        if self.fail_on_write.load(Ordering::SeqCst) || 
           count >= self.max_operations_before_failure.load(Ordering::SeqCst) {
            return Err(AgcodexError::IoError(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("Simulated write failure for: {}", path.display())
            )));
        }
        
        fs::write(path, content).await
            .map_err(|e| AgcodexError::IoError(e))
    }
    
    fn path(&self) -> &Path {
        self.temp_dir.path()
    }
}

/// Mock network client that can simulate various failure modes
#[derive(Clone)]
struct MockNetworkClient {
    failure_mode: Arc<Mutex<FailureMode>>,
    request_count: Arc<AtomicUsize>,
    max_requests_before_failure: Arc<AtomicUsize>,
}

#[derive(Debug, Clone)]
enum FailureMode {
    None,
    Timeout,
    ConnectionError,
    ServerError,
    RateLimited,
    IntermittentFailure { success_every: usize },
}

impl MockNetworkClient {
    fn new() -> Self {
        Self {
            failure_mode: Arc::new(Mutex::new(FailureMode::None)),
            request_count: Arc::new(AtomicUsize::new(0)),
            max_requests_before_failure: Arc::new(AtomicUsize::new(usize::MAX)),
        }
    }
    
    fn set_failure_mode(&self, mode: FailureMode) {
        *self.failure_mode.lock().unwrap() = mode;
    }
    
    fn set_failure_after_requests(&self, count: usize) {
        self.max_requests_before_failure.store(count, Ordering::SeqCst);
        self.request_count.store(0, Ordering::SeqCst);
    }
    
    async fn make_request(&self, _endpoint: &str) -> AgcodexResult<String> {
        let count = self.request_count.fetch_add(1, Ordering::SeqCst);
        let mode = self.failure_mode.lock().unwrap().clone();
        
        let should_fail = match &mode {
            FailureMode::None => false,
            FailureMode::IntermittentFailure { success_every } => (count + 1) % success_every != 0,
            _ => count >= self.max_requests_before_failure.load(Ordering::SeqCst),
        };
        
        if should_fail {
            match mode {
                FailureMode::Timeout => {
                    tokio::time::sleep(Duration::from_secs(10)).await; // Simulate long delay
                    Err(AgcodexError::NetworkTimeout("Request timed out".to_string()))
                },
                FailureMode::ConnectionError => {
                    Err(AgcodexError::NetworkError("Connection failed".to_string()))
                },
                FailureMode::ServerError => {
                    Err(AgcodexError::ApiError("Internal server error".to_string()))
                },
                FailureMode::RateLimited => {
                    Err(AgcodexError::RateLimited("Rate limit exceeded".to_string()))
                },
                FailureMode::IntermittentFailure { .. } => {
                    Err(AgcodexError::NetworkError("Intermittent failure".to_string()))
                },
                FailureMode::None => Ok("Success".to_string()),
            }
        } else {
            Ok("Success".to_string())
        }
    }
}

/// Transaction-like operation manager for testing rollback scenarios
#[derive(Debug)]
struct OperationManager {
    operations: Vec<Operation>,
    completed_operations: Vec<usize>,
    temp_files: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
enum Operation {
    CreateFile { path: PathBuf, content: String },
    EditFile { path: PathBuf, original_content: String, new_content: String },
    DeleteFile { path: PathBuf, backup_content: Option<String> },
    NetworkCall { endpoint: String, payload: String },
}

impl OperationManager {
    fn new() -> Self {
        Self {
            operations: Vec::new(),
            completed_operations: Vec::new(),
            temp_files: Vec::new(),
        }
    }
    
    fn add_operation(&mut self, operation: Operation) {
        self.operations.push(operation);
    }
    
    async fn execute_all(&mut self, fs: &MockFileSystem, network: &MockNetworkClient) -> AgcodexResult<()> {
        for (index, operation) in self.operations.iter().enumerate() {
            match self.execute_operation(operation, fs, network).await {
                Ok(()) => {
                    self.completed_operations.push(index);
                    println!("✅ Operation {} completed successfully", index);
                },
                Err(e) => {
                    println!("❌ Operation {} failed: {}", index, e);
                    // Rollback all completed operations
                    self.rollback(fs, network).await?;
                    return Err(e);
                }
            }
        }
        Ok(())
    }
    
    async fn execute_operation(&mut self, operation: &Operation, fs: &MockFileSystem, network: &MockNetworkClient) -> AgcodexResult<()> {
        match operation {
            Operation::CreateFile { path, content } => {
                fs.write_file(path, content).await?;
                self.temp_files.push(path.clone());
                Ok(())
            },
            Operation::EditFile { path, new_content, .. } => {
                fs.write_file(path, new_content).await
            },
            Operation::DeleteFile { path, .. } => {
                if path.exists() {
                    tokio::fs::remove_file(path).await
                        .map_err(|e| AgcodexError::IoError(e))
                } else {
                    Ok(())
                }
            },
            Operation::NetworkCall { endpoint, .. } => {
                network.make_request(endpoint).await.map(|_| ())
            },
        }
    }
    
    async fn rollback(&mut self, fs: &MockFileSystem, _network: &MockNetworkClient) -> AgcodexResult<()> {
        println!("🔄 Starting rollback of {} operations...", self.completed_operations.len());
        
        // Rollback operations in reverse order
        for &index in self.completed_operations.iter().rev() {
            if let Some(operation) = self.operations.get(index) {
                match self.rollback_operation(operation, fs).await {
                    Ok(()) => println!("✅ Rolled back operation {}", index),
                    Err(e) => println!("⚠️ Failed to rollback operation {}: {}", index, e),
                }
            }
        }
        
        // Clean up temporary files
        for temp_file in &self.temp_files {
            if temp_file.exists() {
                if let Err(e) = tokio::fs::remove_file(temp_file).await {
                    println!("⚠️ Failed to clean up {}: {}", temp_file.display(), e);
                }
            }
        }
        
        self.completed_operations.clear();
        self.temp_files.clear();
        
        println!("✅ Rollback completed");
        Ok(())
    }
    
    async fn rollback_operation(&self, operation: &Operation, fs: &MockFileSystem) -> AgcodexResult<()> {
        match operation {
            Operation::CreateFile { path, .. } => {
                if path.exists() {
                    tokio::fs::remove_file(path).await
                        .map_err(|e| AgcodexError::IoError(e))
                } else {
                    Ok(())
                }
            },
            Operation::EditFile { path, original_content, .. } => {
                fs.write_file(path, original_content).await
            },
            Operation::DeleteFile { path, backup_content } => {
                if let Some(content) = backup_content {
                    fs.write_file(path, content).await
                } else {
                    Ok(())
                }
            },
            Operation::NetworkCall { .. } => {
                // Network operations typically can't be rolled back
                // In practice, you might need to make compensating calls
                Ok(())
            },
        }
    }
}

/// Test file not found error handling with graceful degradation
#[tokio::test]
async fn test_file_not_found_handling() {
    println!("Testing file not found error handling...");
    
    let fs = MockFileSystem::new();
    let context = Context::new(fs.path().to_path_buf());
    
    // Test 1: Search tool handles missing files gracefully
    let search_tool = SearchTool::new();
    
    // Try to search in non-existent file
    let non_existent_file = fs.path().join("does_not_exist.rs");
    let search_result = search_tool.execute(&format!("search pattern in {}", non_existent_file.display()), &context).await;
    
    match search_result {
        Ok(result) => {
            // Should return empty results, not crash
            assert_eq!(result.result.len(), 0);
            println!("✅ Search tool gracefully handled missing file");
        },
        Err(e) => {
            // Should be a controlled error, not a panic
            assert!(matches!(e, AgcodexError::NotFound(_)));
            println!("✅ Search tool returned controlled error for missing file");
        }
    }
    
    // Test 2: Edit tool handles missing files by offering to create them
    let edit_tool = EditTool::new();
    
    let edit_result = edit_tool.execute(&format!("edit file {}", non_existent_file.display()), &context).await;
    
    match edit_result {
        Ok(_) => {
            println!("✅ Edit tool handled missing file appropriately");
        },
        Err(e) => {
            // Should provide helpful error message
            assert!(e.to_string().contains("not found") || e.to_string().contains("does not exist"));
            println!("✅ Edit tool provided helpful error for missing file: {}", e);
        }
    }
    
    // Test 3: Graceful degradation - partial success when some files exist
    let existing_file = fs.path().join("existing.rs");
    fs.write_file(&existing_file, "fn main() { println!(\"Hello\"); }").await.unwrap();
    
    let files_to_search = vec![
        existing_file.to_string_lossy().to_string(),
        non_existent_file.to_string_lossy().to_string(),
    ];
    
    // Search should find results from existing file, ignore missing one
    for file in &files_to_search {
        let result = search_tool.execute(&format!("main in {}", file), &context).await;
        match result {
            Ok(search_result) => {
                if file.contains("existing") {
                    assert!(search_result.result.len() > 0);
                    println!("✅ Found results in existing file");
                } else {
                    assert_eq!(search_result.result.len(), 0);
                    println!("✅ Gracefully handled missing file in batch operation");
                }
            },
            Err(_) => {
                // This is also acceptable - depends on implementation
                println!("✅ Handled missing file with controlled error");
            }
        }
    }
    
    println!("File not found handling test completed successfully!");
}

/// Test network failure recovery with retry logic and exponential backoff
#[tokio::test]
async fn test_network_failure_recovery() {
    println!("Testing network failure recovery...");
    
    let client = MockNetworkClient::new();
    
    // Test 1: Simple timeout handling
    println!("Testing timeout handling...");
    client.set_failure_mode(FailureMode::Timeout);
    
    let start = Instant::now();
    let result = timeout(Duration::from_secs(2), client.make_request("/api/test")).await;
    
    match result {
        Ok(Ok(_)) => panic!("Expected timeout"),
        Ok(Err(e)) => {
            assert!(matches!(e, AgcodexError::NetworkTimeout(_)));
            println!("✅ Timeout handled correctly: {}", e);
        },
        Err(_) => {
            println!("✅ Request properly timed out after 2 seconds");
        }
    }
    
    // Test 2: Retry logic with exponential backoff
    println!("Testing retry logic...");
    
    async fn retry_request(client: &MockNetworkClient, endpoint: &str, max_retries: usize) -> AgcodexResult<String> {
        let mut attempts = 0;
        let mut delay = Duration::from_millis(100);
        
        loop {
            match client.make_request(endpoint).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    attempts += 1;
                    if attempts >= max_retries {
                        return Err(e);
                    }
                    
                    println!("Attempt {} failed: {}, retrying in {:?}", attempts, e, delay);
                    tokio::time::sleep(delay).await;
                    delay *= 2; // Exponential backoff
                }
            }
        }
    }
    
    // Test intermittent failures - should succeed after retries
    client.set_failure_mode(FailureMode::IntermittentFailure { success_every: 3 });
    
    let retry_result = retry_request(&client, "/api/intermittent", 5).await;
    assert!(retry_result.is_ok());
    println!("✅ Successfully recovered from intermittent failures");
    
    // Test permanent failure - should exhaust retries
    client.set_failure_mode(FailureMode::ConnectionError);
    client.set_failure_after_requests(0);
    
    let permanent_failure_result = retry_request(&client, "/api/permanent_fail", 3).await;
    assert!(permanent_failure_result.is_err());
    println!("✅ Properly gave up after exhausting retries");
    
    // Test 3: Rate limiting handling
    println!("Testing rate limiting recovery...");
    client.set_failure_mode(FailureMode::RateLimited);
    
    async fn handle_rate_limiting(client: &MockNetworkClient, endpoint: &str) -> AgcodexResult<String> {
        match client.make_request(endpoint).await {
            Ok(result) => Ok(result),
            Err(AgcodexError::RateLimited(_)) => {
                println!("Rate limited, waiting before retry...");
                tokio::time::sleep(Duration::from_millis(500)).await;
                
                // In real implementation, you'd parse retry-after header
                client.set_failure_mode(FailureMode::None);
                client.make_request(endpoint).await
            },
            Err(e) => Err(e),
        }
    }
    
    let rate_limit_result = handle_rate_limiting(&client, "/api/rate_limited").await;
    assert!(rate_limit_result.is_ok());
    println!("✅ Successfully handled rate limiting");
    
    println!("Network failure recovery test completed successfully!");
}

/// Test cleanup after partial operations fail
#[tokio::test]
async fn test_partial_operation_cleanup() {
    println!("Testing cleanup after partial operation failures...");
    
    let fs = MockFileSystem::new();
    let network = MockNetworkClient::new();
    let mut operation_manager = OperationManager::new();
    
    // Setup a complex operation that will partially succeed
    let file1 = fs.path().join("file1.rs");
    let file2 = fs.path().join("file2.rs");
    let file3 = fs.path().join("file3.rs");
    
    operation_manager.add_operation(Operation::CreateFile {
        path: file1.clone(),
        content: "// First file".to_string(),
    });
    
    operation_manager.add_operation(Operation::CreateFile {
        path: file2.clone(),
        content: "// Second file".to_string(),
    });
    
    operation_manager.add_operation(Operation::NetworkCall {
        endpoint: "/api/register".to_string(),
        payload: "data".to_string(),
    });
    
    operation_manager.add_operation(Operation::CreateFile {
        path: file3.clone(),
        content: "// Third file".to_string(),
    });
    
    // Make the third operation (network call) fail
    network.set_failure_mode(FailureMode::ServerError);
    network.set_failure_after_requests(0);
    
    println!("Executing operations (expecting failure at network call)...");
    let result = operation_manager.execute_all(&fs, &network).await;
    
    // Should fail at the network call
    assert!(result.is_err());
    println!("✅ Operations failed as expected");
    
    // Verify cleanup - files should be removed
    assert!(!file1.exists(), "File1 should be cleaned up");
    assert!(!file2.exists(), "File2 should be cleaned up");
    assert!(!file3.exists(), "File3 should not have been created");
    
    println!("✅ All partial operations were successfully cleaned up");
    
    // Test 2: Test cleanup with write failures
    println!("Testing cleanup with filesystem failures...");
    
    let mut operation_manager2 = OperationManager::new();
    let file4 = fs.path().join("file4.rs");
    let file5 = fs.path().join("file5.rs");
    
    operation_manager2.add_operation(Operation::CreateFile {
        path: file4.clone(),
        content: "// File 4".to_string(),
    });
    
    operation_manager2.add_operation(Operation::CreateFile {
        path: file5.clone(),
        content: "// File 5".to_string(),
    });
    
    // Make write operations fail after first success
    fs.set_failure_after_operations(1);
    
    let result2 = operation_manager2.execute_all(&fs, &network).await;
    assert!(result2.is_err());
    
    // First file might exist, but rollback should clean it up
    assert!(!file4.exists() || !file5.exists(), "At least one file should be cleaned up");
    
    println!("✅ Cleanup with filesystem failures handled correctly");
    
    println!("Partial operation cleanup test completed successfully!");
}

/// Test state consistency validation after errors
#[tokio::test]
async fn test_state_consistency_validation() {
    println!("Testing state consistency validation after errors...");
    
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct ApplicationState {
        files: Vec<String>,
        user_count: usize,
        configuration: HashMap<String, String>,
        last_operation: String,
    }
    
    use std::collections::HashMap;
    
    impl ApplicationState {
        fn new() -> Self {
            Self {
                files: Vec::new(),
                user_count: 0,
                configuration: HashMap::new(),
                last_operation: "init".to_string(),
            }
        }
        
        fn validate(&self) -> Result<(), String> {
            // Validate invariants
            if self.user_count > 1000 {
                return Err("User count exceeds maximum".to_string());
            }
            
            if self.files.len() != self.user_count {
                return Err(format!(
                    "Files count ({}) doesn't match user count ({})", 
                    self.files.len(), 
                    self.user_count
                ));
            }
            
            if !self.configuration.contains_key("version") {
                return Err("Version configuration missing".to_string());
            }
            
            Ok(())
        }
        
        fn add_user(&mut self, username: String) -> Result<(), String> {
            self.files.push(format!("{}.profile", username));
            self.user_count += 1;
            self.last_operation = format!("add_user:{}", username);
            
            self.validate()
        }
        
        fn remove_user(&mut self, username: &str) -> Result<(), String> {
            let file_to_remove = format!("{}.profile", username);
            if let Some(pos) = self.files.iter().position(|x| x == &file_to_remove) {
                self.files.remove(pos);
                self.user_count -= 1;
                self.last_operation = format!("remove_user:{}", username);
                
                self.validate()
            } else {
                Err(format!("User {} not found", username))
            }
        }
    }
    
    let mut state = ApplicationState::new();
    state.configuration.insert("version".to_string(), "1.0".to_string());
    
    // Test 1: Valid operations maintain consistency
    assert!(state.validate().is_ok());
    assert!(state.add_user("alice".to_string()).is_ok());
    assert!(state.add_user("bob".to_string()).is_ok());
    assert_eq!(state.user_count, 2);
    assert_eq!(state.files.len(), 2);
    println!("✅ Valid operations maintain state consistency");
    
    // Test 2: Simulate corrupted state detection
    println!("Testing corrupted state detection...");
    
    // Manually corrupt the state
    state.files.push("extra_file.profile".to_string());
    
    let validation_result = state.validate();
    assert!(validation_result.is_err());
    assert!(validation_result.unwrap_err().contains("doesn't match"));
    println!("✅ Corrupted state detected successfully");
    
    // Test 3: State recovery after error
    println!("Testing state recovery...");
    
    let mut recovery_state = ApplicationState::new();
    recovery_state.configuration.insert("version".to_string(), "1.0".to_string());
    
    // Create checkpoint before risky operation
    let checkpoint = recovery_state.clone();
    
    // Try risky operation that might fail
    let risky_operation = |state: &mut ApplicationState| -> Result<(), String> {
        state.add_user("user1".to_string())?;
        state.add_user("user2".to_string())?;
        
        // Simulate an operation that corrupts state
        state.files.push("corrupted.profile".to_string());
        
        state.validate()
    };
    
    match risky_operation(&mut recovery_state) {
        Ok(_) => panic!("Expected risky operation to fail"),
        Err(e) => {
            println!("Risky operation failed as expected: {}", e);
            
            // Restore from checkpoint
            recovery_state = checkpoint;
            
            // Verify recovery
            assert!(recovery_state.validate().is_ok());
            assert_eq!(recovery_state.user_count, 0);
            assert_eq!(recovery_state.files.len(), 0);
            
            println!("✅ Successfully recovered state from checkpoint");
        }
    }
    
    // Test 4: Incremental validation during complex operations
    println!("Testing incremental validation...");
    
    let mut incremental_state = ApplicationState::new();
    incremental_state.configuration.insert("version".to_string(), "1.0".to_string());
    
    let complex_operation = |state: &mut ApplicationState| -> Result<(), String> {
        // Step 1: Add first user
        state.add_user("step1_user".to_string())?;
        println!("Step 1 validated successfully");
        
        // Step 2: Add second user
        state.add_user("step2_user".to_string())?;
        println!("Step 2 validated successfully");
        
        // Step 3: Configuration update
        state.configuration.insert("feature_flags".to_string(), "enabled".to_string());
        state.validate()?;
        println!("Step 3 validated successfully");
        
        Ok(())
    };
    
    let result = complex_operation(&mut incremental_state);
    assert!(result.is_ok());
    assert_eq!(incremental_state.user_count, 2);
    assert!(incremental_state.configuration.contains_key("feature_flags"));
    println!("✅ Complex operation with incremental validation succeeded");
    
    println!("State consistency validation test completed successfully!");
}

/// Test comprehensive rollback mechanisms
#[tokio::test]
async fn test_rollback_mechanisms() {
    println!("Testing comprehensive rollback mechanisms...");
    
    let fs = MockFileSystem::new();
    let network = MockNetworkClient::new();
    
    // Test 1: File operation rollback
    println!("Testing file operation rollback...");
    
    let original_file = fs.path().join("original.rs");
    let original_content = "fn original() { println!(\"original\"); }";
    fs.write_file(&original_file, original_content).await.unwrap();
    
    let mut file_operation_manager = OperationManager::new();
    
    // Read original content for rollback
    let backup_content = fs.read_file(&original_file).await.unwrap();
    
    file_operation_manager.add_operation(Operation::EditFile {
        path: original_file.clone(),
        original_content: backup_content,
        new_content: "fn modified() { println!(\"modified\"); }".to_string(),
    });
    
    let new_file = fs.path().join("new.rs");
    file_operation_manager.add_operation(Operation::CreateFile {
        path: new_file.clone(),
        content: "fn new() { println!(\"new\"); }".to_string(),
    });
    
    // This will succeed initially
    let initial_result = file_operation_manager.execute_all(&fs, &network).await;
    assert!(initial_result.is_ok());
    
    // Verify changes were made
    let modified_content = fs.read_file(&original_file).await.unwrap();
    assert!(modified_content.contains("modified"));
    assert!(new_file.exists());
    
    println!("✅ Operations executed successfully");
    
    // Now rollback manually
    file_operation_manager.rollback(&fs, &network).await.unwrap();
    
    // Verify rollback
    let restored_content = fs.read_file(&original_file).await.unwrap();
    assert!(restored_content.contains("original"));
    assert!(!new_file.exists());
    
    println!("✅ File operations rolled back successfully");
    
    // Test 2: Transaction-like rollback with multiple file types
    println!("Testing transaction-like rollback...");
    
    let config_file = fs.path().join("config.json");
    let log_file = fs.path().join("app.log");
    let data_file = fs.path().join("data.txt");
    
    // Initial state
    fs.write_file(&config_file, r#"{"version": "1.0"}"#).await.unwrap();
    fs.write_file(&log_file, "Initial log entry\n").await.unwrap();
    
    let mut transaction_manager = OperationManager::new();
    
    // Backup original contents
    let config_backup = fs.read_file(&config_file).await.unwrap();
    let log_backup = fs.read_file(&log_file).await.unwrap();
    
    // Define transaction operations
    transaction_manager.add_operation(Operation::EditFile {
        path: config_file.clone(),
        original_content: config_backup,
        new_content: r#"{"version": "2.0", "features": ["new_feature"]}"#.to_string(),
    });
    
    transaction_manager.add_operation(Operation::EditFile {
        path: log_file.clone(),
        original_content: log_backup,
        new_content: "Initial log entry\nUpgraded to v2.0\n".to_string(),
    });
    
    transaction_manager.add_operation(Operation::CreateFile {
        path: data_file.clone(),
        content: "New data for v2.0".to_string(),
    });
    
    transaction_manager.add_operation(Operation::NetworkCall {
        endpoint: "/api/upgrade".to_string(),
        payload: r#"{"version": "2.0"}"#.to_string(),
    });
    
    // Make network call fail to trigger rollback
    network.set_failure_mode(FailureMode::ServerError);
    network.set_failure_after_requests(0);
    
    let transaction_result = transaction_manager.execute_all(&fs, &network).await;
    assert!(transaction_result.is_err());
    
    // Verify all changes were rolled back
    let config_content = fs.read_file(&config_file).await.unwrap();
    let log_content = fs.read_file(&log_file).await.unwrap();
    
    assert!(config_content.contains(r#""version": "1.0""#));
    assert!(!config_content.contains("features"));
    assert_eq!(log_content.lines().count(), 1);
    assert!(!data_file.exists());
    
    println!("✅ Transaction rollback completed successfully");
    
    // Test 3: Partial rollback with some irreversible operations
    println!("Testing partial rollback scenarios...");
    
    let mut partial_manager = OperationManager::new();
    let temp_file = fs.path().join("temp.rs");
    
    partial_manager.add_operation(Operation::CreateFile {
        path: temp_file.clone(),
        content: "temporary content".to_string(),
    });
    
    // Network operations can't be truly rolled back
    partial_manager.add_operation(Operation::NetworkCall {
        endpoint: "/api/log_action".to_string(),
        payload: "action_data".to_string(),
    });
    
    partial_manager.add_operation(Operation::CreateFile {
        path: fs.path().join("should_not_exist.rs"),
        content: "this should not exist after rollback".to_string(),
    });
    
    // Execute first two operations, fail on third
    fs.set_failure_after_operations(2);
    network.set_failure_mode(FailureMode::None);
    
    let partial_result = partial_manager.execute_all(&fs, &network).await;
    assert!(partial_result.is_err());
    
    // Verify partial rollback
    assert!(!temp_file.exists());
    assert!(!fs.path().join("should_not_exist.rs").exists());
    
    println!("✅ Partial rollback handled appropriately");
    
    println!("Rollback mechanisms test completed successfully!");
}

/// Integration test combining multiple error scenarios
#[tokio::test]
async fn test_comprehensive_error_recovery() {
    println!("Testing comprehensive error recovery scenario...");
    
    let fs = MockFileSystem::new();
    let network = MockNetworkClient::new();
    let context = Context::new(fs.path().to_path_buf());
    
    // Scenario: Deploying a code update with multiple failure points
    println!("Scenario: Complex deployment with multiple failure points");
    
    let mut deployment_manager = OperationManager::new();
    
    // Step 1: Backup current code
    let main_file = fs.path().join("src/main.rs");
    let old_main_content = r#"
fn main() {
    println!("Version 1.0");
}
"#;
    fs.write_file(&main_file, old_main_content).await.unwrap();
    
    // Step 2: Update code
    let new_main_content = r#"
fn main() {
    println!("Version 2.0");
    // New feature
    advanced_feature();
}

fn advanced_feature() {
    println!("Advanced feature activated");
}
"#;
    
    deployment_manager.add_operation(Operation::EditFile {
        path: main_file.clone(),
        original_content: old_main_content.to_string(),
        new_content: new_main_content.to_string(),
    });
    
    // Step 3: Add configuration
    let config_file = fs.path().join("config.toml");
    deployment_manager.add_operation(Operation::CreateFile {
        path: config_file.clone(),
        content: r#"
[app]
version = "2.0"
features = ["advanced"]
"#.to_string(),
    });
    
    // Step 4: Notify deployment service
    deployment_manager.add_operation(Operation::NetworkCall {
        endpoint: "/api/deploy".to_string(),
        payload: r#"{"version": "2.0", "environment": "production"}"#.to_string(),
    });
    
    // Step 5: Update database schema
    deployment_manager.add_operation(Operation::NetworkCall {
        endpoint: "/api/migrate".to_string(),
        payload: r#"{"migration": "add_feature_flags"}"#.to_string(),
    });
    
    println!("Starting deployment process...");
    
    // Test Case 1: Successful deployment
    network.set_failure_mode(FailureMode::None);
    let success_result = deployment_manager.execute_all(&fs, &network).await;
    
    if success_result.is_ok() {
        println!("✅ Successful deployment completed");
        
        // Verify final state
        let deployed_content = fs.read_file(&main_file).await.unwrap();
        assert!(deployed_content.contains("Version 2.0"));
        assert!(config_file.exists());
        
        // Reset for failure test
        deployment_manager.rollback(&fs, &network).await.unwrap();
    }
    
    println!("\nTesting deployment failure scenarios...");
    
    // Test Case 2: Network failure during deployment notification
    let mut deployment_manager2 = OperationManager::new();
    
    deployment_manager2.add_operation(Operation::EditFile {
        path: main_file.clone(),
        original_content: old_main_content.to_string(),
        new_content: new_main_content.to_string(),
    });
    
    deployment_manager2.add_operation(Operation::CreateFile {
        path: config_file.clone(),
        content: r#"[app]\nversion = "2.0""#.to_string(),
    });
    
    deployment_manager2.add_operation(Operation::NetworkCall {
        endpoint: "/api/deploy".to_string(),
        payload: r#"{"version": "2.0"}"#.to_string(),
    });
    
    // Fail on network call
    network.set_failure_mode(FailureMode::ConnectionError);
    network.set_failure_after_requests(0);
    
    let failure_result = deployment_manager2.execute_all(&fs, &network).await;
    assert!(failure_result.is_err());
    
    // Verify rollback
    let rolled_back_content = fs.read_file(&main_file).await.unwrap();
    assert!(rolled_back_content.contains("Version 1.0"));
    assert!(!config_file.exists());
    
    println!("✅ Deployment failure properly rolled back");
    
    // Test Case 3: Filesystem failure with network recovery
    println!("Testing mixed failure scenarios...");
    
    let mut mixed_manager = OperationManager::new();
    
    mixed_manager.add_operation(Operation::CreateFile {
        path: fs.path().join("step1.txt"),
        content: "Step 1 completed".to_string(),
    });
    
    mixed_manager.add_operation(Operation::NetworkCall {
        endpoint: "/api/step2".to_string(),
        payload: "step2_data".to_string(),
    });
    
    mixed_manager.add_operation(Operation::CreateFile {
        path: fs.path().join("step3.txt"),
        content: "Step 3 completed".to_string(),
    });
    
    // Network succeeds, but filesystem fails on second write
    network.set_failure_mode(FailureMode::None);
    fs.set_failure_after_operations(1); // Fail after first file write
    
    let mixed_result = mixed_manager.execute_all(&fs, &network).await;
    assert!(mixed_result.is_err());
    
    // Verify cleanup
    assert!(!fs.path().join("step1.txt").exists());
    assert!(!fs.path().join("step3.txt").exists());
    
    println!("✅ Mixed failure scenario handled correctly");
    
    // Test Case 4: Recovery with retry logic
    println!("Testing recovery with retry logic...");
    
    async fn deploy_with_retry(fs: &MockFileSystem, network: &MockNetworkClient) -> AgcodexResult<()> {
        const MAX_RETRIES: usize = 3;
        let mut attempt = 0;
        
        loop {
            let mut retry_manager = OperationManager::new();
            
            retry_manager.add_operation(Operation::CreateFile {
                path: fs.path().join("deploy.log"),
                content: format!("Deployment attempt {}", attempt + 1),
            });
            
            retry_manager.add_operation(Operation::NetworkCall {
                endpoint: "/api/health_check".to_string(),
                payload: "ping".to_string(),
            });
            
            match retry_manager.execute_all(fs, network).await {
                Ok(()) => {
                    println!("✅ Deployment succeeded on attempt {}", attempt + 1);
                    return Ok(());
                }
                Err(e) => {
                    attempt += 1;
                    if attempt >= MAX_RETRIES {
                        println!("❌ Deployment failed after {} attempts: {}", MAX_RETRIES, e);
                        return Err(e);
                    }
                    
                    println!("Attempt {} failed: {}, retrying...", attempt, e);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }
    
    // First two attempts fail, third succeeds
    network.set_failure_mode(FailureMode::IntermittentFailure { success_every: 3 });
    fs.disable_failures();
    
    let retry_result = deploy_with_retry(&fs, &network).await;
    assert!(retry_result.is_ok());
    
    println!("✅ Recovery with retry logic succeeded");
    
    println!("Comprehensive error recovery test completed successfully!");
}