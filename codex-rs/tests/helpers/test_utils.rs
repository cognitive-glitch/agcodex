//! Test utilities and helpers for AGCodex test suite.
//!
//! Provides common functionality for setting up test environments,
//! creating mock data, and validating test results.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tempfile::TempDir;
use uuid::Uuid;

/// Test configuration and environment setup
pub struct TestEnvironment {
    pub temp_dir: TempDir,
    pub storage_path: PathBuf,
    pub config_path: PathBuf,
}

impl TestEnvironment {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let storage_path = temp_dir.path().join(".agcodex");
        let config_path = storage_path.join("config.toml");
        
        // Create directories
        fs::create_dir_all(&storage_path).expect("Failed to create storage directory");
        
        Self {
            temp_dir,
            storage_path,
            config_path,
        }
    }
    
    pub fn create_sample_config(&self) -> PathBuf {
        let config_content = r#"
[intelligence]
level = "medium"
reasoning_effort = "high"
verbosity = "high"

[tui]
enable_mouse = true
auto_save_interval = 300

[modes]
default = "build"
allow_mode_cycling = true

[session]
compression = "zstd"
auto_save = true
checkpoint_interval = 60
"#;
        
        fs::write(&self.config_path, config_content)
            .expect("Failed to write test config");
        
        self.config_path.clone()
    }
    
    pub fn create_sample_files(&self) -> HashMap<String, PathBuf> {
        let mut files = HashMap::new();
        
        // Create sample source files for testing
        let samples = vec![
            ("main.rs", include_str!("../fixtures/sample_code/rust_sample.rs")),
            ("app.py", include_str!("../fixtures/sample_code/python_sample.py")),
            ("server.go", include_str!("../fixtures/sample_code/go_sample.go")),
            ("component.tsx", include_str!("../fixtures/sample_code/typescript_sample.tsx")),
        ];
        
        for (filename, content) in samples {
            let file_path = self.temp_dir.path().join(filename);
            fs::write(&file_path, content).expect("Failed to write sample file");
            files.insert(filename.to_string(), file_path);
        }
        
        files
    }
    
    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }
}

/// Mock data generators for testing
pub struct MockDataGenerator;

impl MockDataGenerator {
    pub fn session_id() -> Uuid {
        Uuid::new_v4()
    }
    
    pub fn session_title(prefix: &str) -> String {
        format!("{} Session {}", prefix, Uuid::new_v4().to_string()[..8].to_uppercase())
    }
    
    pub fn conversation_message(role: &str, content: &str) -> MockMessage {
        MockMessage {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            role: role.to_string(),
            content: content.to_string(),
        }
    }
    
    pub fn file_contexts() -> Vec<String> {
        vec![
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
            "tests/integration.rs".to_string(),
            "Cargo.toml".to_string(),
            "README.md".to_string(),
        ]
    }
    
    pub fn ast_node_sample() -> MockAstNode {
        MockAstNode {
            node_type: "function_declaration".to_string(),
            start_line: 10,
            end_line: 25,
            children: vec![
                MockAstNode {
                    node_type: "identifier".to_string(),
                    start_line: 10,
                    end_line: 10,
                    children: vec![],
                },
                MockAstNode {
                    node_type: "block".to_string(),
                    start_line: 11,
                    end_line: 25,
                    children: vec![],
                },
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub struct MockMessage {
    pub id: Uuid,
    pub timestamp: SystemTime,
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct MockAstNode {
    pub node_type: String,
    pub start_line: usize,
    pub end_line: usize,
    pub children: Vec<MockAstNode>,
}

/// Performance assertion helpers
pub struct PerformanceAssertions;

impl PerformanceAssertions {
    /// Assert that operation completes within target time
    pub fn assert_duration_under(duration: Duration, target_ms: u64, operation: &str) {
        let actual_ms = duration.as_millis();
        assert!(
            actual_ms < target_ms as u128,
            "{} took {}ms, target was <{}ms",
            operation,
            actual_ms,
            target_ms
        );
    }
    
    /// Assert that cache hit rate meets target
    pub fn assert_cache_hit_rate(hits: usize, total: usize, target_percent: f64) {
        let actual_rate = if total > 0 { hits as f64 / total as f64 } else { 0.0 };
        assert!(
            actual_rate >= target_percent / 100.0,
            "Cache hit rate {:.1}% below target {:.1}%",
            actual_rate * 100.0,
            target_percent
        );
    }
    
    /// Assert that compression ratio meets target
    pub fn assert_compression_ratio(original_size: usize, compressed_size: usize, target_percent: f64) {
        let ratio = if original_size > 0 {
            1.0 - (compressed_size as f64 / original_size as f64)
        } else {
            0.0
        };
        
        assert!(
            ratio >= target_percent / 100.0,
            "Compression ratio {:.1}% below target {:.1}%",
            ratio * 100.0,
            target_percent
        );
    }
    
    /// Assert that memory usage is reasonable
    pub fn assert_memory_usage_reasonable(usage_bytes: usize, max_mb: usize) {
        let usage_mb = usage_bytes / (1024 * 1024);
        assert!(
            usage_mb <= max_mb,
            "Memory usage {}MB exceeds limit of {}MB",
            usage_mb,
            max_mb
        );
    }
}

/// Test result validation helpers
pub struct TestValidation;

impl TestValidation {
    /// Validate that all expected files exist
    pub fn assert_files_exist(files: &[PathBuf]) {
        for file in files {
            assert!(file.exists(), "Expected file does not exist: {}", file.display());
        }
    }
    
    /// Validate that session data is consistent
    pub fn assert_session_data_consistent(session: &MockSessionData) {
        assert!(!session.id.is_nil(), "Session ID should not be nil");
        assert!(!session.title.is_empty(), "Session title should not be empty");
        assert!(
            session.last_modified >= session.created_at,
            "Last modified should be >= created time"
        );
        assert!(
            session.conversation_count <= session.max_conversations,
            "Conversation count exceeds maximum"
        );
    }
    
    /// Validate that mode restrictions are enforced
    pub fn assert_mode_restrictions_enforced(mode: &str, restrictions: &MockModeRestrictions) {
        match mode {
            "Plan" => {
                assert!(!restrictions.allow_file_write, "Plan mode should not allow file writes");
                assert!(!restrictions.allow_command_exec, "Plan mode should not allow command execution");
            }
            "Build" => {
                assert!(restrictions.allow_file_write, "Build mode should allow file writes");
                assert!(restrictions.allow_command_exec, "Build mode should allow command execution");
            }
            "Review" => {
                assert!(restrictions.allow_file_write, "Review mode should allow limited file writes");
                assert!(!restrictions.allow_command_exec, "Review mode should not allow command execution");
                assert!(
                    restrictions.max_file_size.unwrap_or(0) <= 10_000,
                    "Review mode should limit file size"
                );
            }
            _ => panic!("Unknown mode: {}", mode),
        }
    }
    
    /// Validate AST parsing results
    pub fn assert_ast_parse_result_valid(result: &MockParseResult, expected_language: &str) {
        assert_eq!(result.language, expected_language, "Parsed language mismatch");
        assert!(result.node_count > 0, "Should have parsed at least one AST node");
        assert!(result.parse_time.as_millis() < 100, "Parse time should be reasonable");
        assert!(result.memory_usage > 0, "Should have used some memory");
    }
}

#[derive(Debug, Clone)]
pub struct MockSessionData {
    pub id: Uuid,
    pub title: String,
    pub created_at: SystemTime,
    pub last_modified: SystemTime,
    pub conversation_count: usize,
    pub max_conversations: usize,
}

#[derive(Debug, Clone)]
pub struct MockModeRestrictions {
    pub allow_file_write: bool,
    pub allow_command_exec: bool,
    pub max_file_size: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct MockParseResult {
    pub language: String,
    pub node_count: usize,
    pub parse_time: Duration,
    pub memory_usage: usize,
}

/// Async test helpers
pub struct AsyncTestHelpers;

impl AsyncTestHelpers {
    /// Wait for a condition with timeout
    pub async fn wait_for_condition<F>(
        condition: F,
        timeout: Duration,
        check_interval: Duration,
    ) -> Result<(), &'static str>
    where
        F: Fn() -> bool,
    {
        let start = std::time::Instant::now();
        
        while start.elapsed() < timeout {
            if condition() {
                return Ok(());
            }
            tokio::time::sleep(check_interval).await;
        }
        
        Err("Condition not met within timeout")
    }
    
    /// Simulate rapid user input for TUI testing
    pub async fn simulate_rapid_input<F>(
        mut input_generator: F,
        count: usize,
        interval: Duration,
    ) -> Vec<String>
    where
        F: FnMut(usize) -> String,
    {
        let mut results = Vec::new();
        
        for i in 0..count {
            let input = input_generator(i);
            results.push(input);
            
            if i < count - 1 {
                tokio::time::sleep(interval).await;
            }
        }
        
        results
    }
}

/// Sample code snippets for testing various languages
pub struct CodeSamples;

impl CodeSamples {
    pub fn rust_function() -> &'static str {
        r#"
pub fn fibonacci(n: u32) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}
"#
    }
    
    pub fn python_class() -> &'static str {
        r#"
class Calculator:
    def __init__(self):
        self.history = []
    
    def add(self, a, b):
        result = a + b
        self.history.append(f"{a} + {b} = {result}")
        return result
    
    def get_history(self):
        return self.history.copy()
"#
    }
    
    pub fn typescript_interface() -> &'static str {
        r#"
interface UserProfile {
    id: number;
    name: string;
    email: string;
    preferences: {
        theme: 'light' | 'dark';
        notifications: boolean;
    };
}

class UserManager {
    private users: Map<number, UserProfile> = new Map();
    
    addUser(user: UserProfile): void {
        this.users.set(user.id, user);
    }
    
    getUser(id: number): UserProfile | undefined {
        return this.users.get(id);
    }
}
"#
    }
    
    pub fn go_struct() -> &'static str {
        r#"
package main

import "fmt"

type Task struct {
    ID        int    `json:"id"`
    Title     string `json:"title"`
    Completed bool   `json:"completed"`
}

func (t *Task) Toggle() {
    t.Completed = !t.Completed
}

func (t Task) String() string {
    status := "pending"
    if t.Completed {
        status = "completed"
    }
    return fmt.Sprintf("Task %d: %s (%s)", t.ID, t.Title, status)
}
"#
    }
    
    pub fn for_language(language: &str) -> Option<&'static str> {
        match language {
            "rust" => Some(Self::rust_function()),
            "python" => Some(Self::python_class()),
            "typescript" => Some(Self::typescript_interface()),
            "go" => Some(Self::go_struct()),
            _ => None,
        }
    }
}

/// Test timing utilities
pub struct TestTiming;

impl TestTiming {
    /// Time an operation and return both result and duration
    pub fn time_operation<F, R>(operation: F) -> (R, Duration)
    where
        F: FnOnce() -> R,
    {
        let start = std::time::Instant::now();
        let result = operation();
        let duration = start.elapsed();
        (result, duration)
    }
    
    /// Time an async operation
    pub async fn time_async_operation<F, Fut, R>(operation: F) -> (R, Duration)
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = R>,
    {
        let start = std::time::Instant::now();
        let result = operation().await;
        let duration = start.elapsed();
        (result, duration)
    }
    
    /// Benchmark an operation multiple times and return statistics
    pub fn benchmark_operation<F, R>(operation: F, iterations: usize) -> BenchmarkStats
    where
        F: Fn() -> R,
    {
        let mut durations = Vec::with_capacity(iterations);
        
        for _ in 0..iterations {
            let (_, duration) = Self::time_operation(&operation);
            durations.push(duration);
        }
        
        BenchmarkStats::from_durations(durations)
    }
}

#[derive(Debug, Clone)]
pub struct BenchmarkStats {
    pub min: Duration,
    pub max: Duration,
    pub average: Duration,
    pub median: Duration,
    pub iterations: usize,
}

impl BenchmarkStats {
    pub fn from_durations(mut durations: Vec<Duration>) -> Self {
        durations.sort();
        
        let min = durations[0];
        let max = durations[durations.len() - 1];
        let median = durations[durations.len() / 2];
        
        let total_nanos: u64 = durations.iter().map(|d| d.as_nanos() as u64).sum();
        let average = Duration::from_nanos(total_nanos / durations.len() as u64);
        
        Self {
            min,
            max,
            average,
            median,
            iterations: durations.len(),
        }
    }
    
    pub fn assert_average_under(&self, target: Duration, operation: &str) {
        assert!(
            self.average < target,
            "{} average {}ms exceeds target {}ms (over {} iterations)",
            operation,
            self.average.as_millis(),
            target.as_millis(),
            self.iterations
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_environment_setup() {
        let env = TestEnvironment::new();
        assert!(env.temp_dir.path().exists());
        assert!(env.storage_path.ends_with(".agcodex"));
    }
    
    #[test]
    fn test_mock_data_generation() {
        let session_id = MockDataGenerator::session_id();
        assert!(!session_id.is_nil());
        
        let title = MockDataGenerator::session_title("Test");
        assert!(title.starts_with("Test Session"));
        
        let message = MockDataGenerator::conversation_message("user", "Hello");
        assert_eq!(message.role, "user");
        assert_eq!(message.content, "Hello");
    }
    
    #[test]
    fn test_performance_assertions() {
        let fast_duration = Duration::from_millis(5);
        PerformanceAssertions::assert_duration_under(fast_duration, 10, "fast operation");
        
        PerformanceAssertions::assert_cache_hit_rate(90, 100, 80.0);
        PerformanceAssertions::assert_compression_ratio(1000, 200, 70.0);
    }
    
    #[test]
    fn test_timing_utilities() {
        let (result, duration) = TestTiming::time_operation(|| {
            std::thread::sleep(Duration::from_millis(1));
            42
        });
        
        assert_eq!(result, 42);
        assert!(duration >= Duration::from_millis(1));
    }
    
    #[test]
    fn test_benchmark_stats() {
        let durations = vec![
            Duration::from_millis(5),
            Duration::from_millis(10),
            Duration::from_millis(7),
            Duration::from_millis(12),
            Duration::from_millis(8),
        ];
        
        let stats = BenchmarkStats::from_durations(durations);
        assert_eq!(stats.iterations, 5);
        assert_eq!(stats.min, Duration::from_millis(5));
        assert_eq!(stats.max, Duration::from_millis(12));
    }
}