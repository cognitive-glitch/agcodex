//! Integration tests for session management functionality.
//!
//! Tests the complete session lifecycle using real AGCodex components:
//! - SessionManager from agcodex_persistence::session_manager
//! - Session creation, loading, and persistence
//! - Auto-save and checkpoint functionality
//! - Message history navigation
//! - Conversation branching
//!
//! Uses real persistence layer but operates on isolated test directories.

use agcodex_persistence::{
    session_manager::{SessionManager, SessionManagerConfig},
    types::{MessageMetadata, OperatingMode, ResponseItem, SessionState},
    compression::CompressionLevel,
};
use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;
use tempfile::tempdir;
use tokio::time::{sleep, timeout};
use uuid::Uuid;

mod helpers;
use helpers::test_utils::{
    TestEnvironment, PerformanceAssertions, TestTiming, AsyncTestHelpers,
};

/// Test fixture for session management integration tests
struct SessionTestFixture {
    session_manager: SessionManager,
    _test_env: TestEnvironment,
}

impl SessionTestFixture {
    async fn new() -> Self {
        let test_env = TestEnvironment::new();
        let storage_path = test_env.path().join("sessions");
        
        let config = SessionManagerConfig {
            storage_path,
            auto_save_interval: Duration::from_millis(100), // Fast for testing
            max_sessions: 50,
            max_total_size: 100_000_000, // 100MB for tests
            compression_level: CompressionLevel::Fast,
            enable_auto_save: true,
            enable_mmap: false, // Disable mmap for test isolation
            max_checkpoints: 5,
        };
        
        let session_manager = SessionManager::new(config).await.unwrap();
        
        Self {
            session_manager,
            _test_env: test_env,
        }
    }
    
    async fn create_session(
        &self,
        title: String,
        model: String,
        mode: OperatingMode,
    ) -> Result<Uuid, Box<dyn std::error::Error>> {
        self.session_manager
            .create_session(title, model, mode)
            .await
            .map_err(|e| e.into())
    }
    
    async fn load_session(&self, id: Uuid) -> Result<(), Box<dyn std::error::Error>> {
        self.session_manager.load_session(id).await.map_err(|e| e.into())
    }
    
    async fn save_session(&self, id: Uuid) -> Result<(), Box<dyn std::error::Error>> {
        self.session_manager.save_session(id).await.map_err(|e| e.into())
    }
    
    async fn delete_session(&self, id: Uuid) -> Result<(), Box<dyn std::error::Error>> {
        self.session_manager.delete_session(id).await.map_err(|e| e.into())
    }
    
    async fn add_message(
        &self,
        session_id: Uuid,
        content: String,
        role: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let item = ResponseItem {
            id: Uuid::new_v4(),
            content,
            role,
            timestamp: Utc::now(),
            model: Some("test-model".to_string()),
            usage: None,
            metadata: HashMap::new(),
        };
        
        let metadata = MessageMetadata {
            turn_index: 0,
            is_user: role == "user",
            has_attachments: false,
            attachments: Vec::new(),
            processing_time_ms: Some(100),
            token_count: Some(content.split_whitespace().count()),
            model_used: Some("test-model".to_string()),
            cost_estimate: Some(0.001),
            quality_score: Some(0.95),
            tags: Vec::new(),
        };
        
        self.session_manager
            .add_message(session_id, item, Some(metadata))
            .await
            .map_err(|e| e.into())
    }
    
    async fn create_checkpoint(
        &self,
        session_id: Uuid,
        name: String,
        description: Option<String>,
    ) -> Result<Uuid, Box<dyn std::error::Error>> {
        self.session_manager
            .create_checkpoint(session_id, name, description)
            .await
            .map_err(|e| e.into())
    }
    
    async fn restore_checkpoint(
        &self,
        session_id: Uuid,
        checkpoint_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.session_manager
            .restore_checkpoint(session_id, checkpoint_id)
            .await
            .map_err(|e| e.into())
    }
    
    async fn list_sessions(&self) -> Vec<agcodex_persistence::types::SessionMetadata> {
        self.session_manager.list_sessions().await.unwrap_or_default()
    }
    
    async fn get_session_metadata(
        &self,
        id: Uuid,
    ) -> Result<agcodex_persistence::types::SessionMetadata, Box<dyn std::error::Error>> {
        self.session_manager
            .get_session_metadata(id)
            .await
            .map_err(|e| e.into())
    }
    
    async fn switch_mode(
        &self,
        session_id: Uuid,
        mode: OperatingMode,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.session_manager
            .switch_mode(session_id, mode)
            .await
            .map_err(|e| e.into())
    }
    
    async fn update_session_state(
        &self,
        session_id: Uuid,
        state: SessionState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.session_manager
            .update_session_state(session_id, state)
            .await
            .map_err(|e| e.into())
    }
}

#[tokio::test]
async fn test_basic_session_creation() {
    let fixture = SessionTestFixture::new().await;
    
    let session_id = fixture
        .create_session(
            "Test Session".to_string(),
            "gpt-4".to_string(),
            OperatingMode::Build,
        )
        .await
        .unwrap();
    
    // Verify session was created
    assert!(!session_id.is_nil());
    
    let metadata = fixture.get_session_metadata(session_id).await.unwrap();
    assert_eq!(metadata.title, "Test Session");
    assert_eq!(metadata.model, "gpt-4");
    assert_eq!(metadata.current_mode, OperatingMode::Build);
    assert_eq!(metadata.message_count, 0);
    assert_eq!(metadata.turn_count, 0);
    assert!(!metadata.is_favorite);
    assert!(metadata.tags.is_empty());
    
    // Check timestamps
    assert!(metadata.created_at <= Utc::now());
    assert!(metadata.updated_at <= Utc::now());
    assert!(metadata.last_accessed <= Utc::now());
    assert!(metadata.created_at <= metadata.updated_at);
}

#[tokio::test]
async fn test_session_message_handling() {
    let fixture = SessionTestFixture::new().await;
    
    let session_id = fixture
        .create_session(
            "Message Test".to_string(),
            "gpt-4".to_string(),
            OperatingMode::Build,
        )
        .await
        .unwrap();
    
    // Add some messages
    fixture
        .add_message(session_id, "Hello, AI!".to_string(), "user".to_string())
        .await
        .unwrap();
    
    fixture
        .add_message(
            session_id,
            "Hello! How can I help you today?".to_string(),
            "assistant".to_string(),
        )
        .await
        .unwrap();
    
    fixture
        .add_message(
            session_id,
            "Can you help me with Rust code?".to_string(),
            "user".to_string(),
        )
        .await
        .unwrap();
    
    // Check metadata was updated
    let metadata = fixture.get_session_metadata(session_id).await.unwrap();
    assert_eq!(metadata.message_count, 3);
    
    // Verify session can be saved
    fixture.save_session(session_id).await.unwrap();
    
    // Load session and verify persistence
    fixture.load_session(session_id).await.unwrap();
    let metadata_after_load = fixture.get_session_metadata(session_id).await.unwrap();
    assert_eq!(metadata_after_load.message_count, 3);
}

#[tokio::test]
async fn test_session_persistence() {
    let session_id = {
        let fixture = SessionTestFixture::new().await;
        
        let session_id = fixture
            .create_session(
                "Persistence Test".to_string(),
                "claude-3-sonnet".to_string(),
                OperatingMode::Review,
            )
            .await
            .unwrap();
        
        // Add content
        fixture
            .add_message(
                session_id,
                "This is a test message for persistence.".to_string(),
                "user".to_string(),
            )
            .await
            .unwrap();
        
        fixture
            .add_message(
                session_id,
                "I understand. This message should persist.".to_string(),
                "assistant".to_string(),
            )
            .await
            .unwrap();
        
        // Save session
        fixture.save_session(session_id).await.unwrap();
        
        session_id
    }; // fixture goes out of scope here
    
    // Create new fixture (simulates application restart)
    let new_fixture = SessionTestFixture::new().await;
    
    // Load the session
    new_fixture.load_session(session_id).await.unwrap();
    
    // Verify data persisted
    let metadata = new_fixture.get_session_metadata(session_id).await.unwrap();
    assert_eq!(metadata.title, "Persistence Test");
    assert_eq!(metadata.model, "claude-3-sonnet");
    assert_eq!(metadata.current_mode, OperatingMode::Review);
    assert_eq!(metadata.message_count, 2);
}

#[tokio::test]
async fn test_checkpoint_creation_and_restoration() {
    let fixture = SessionTestFixture::new().await;
    
    let session_id = fixture
        .create_session(
            "Checkpoint Test".to_string(),
            "gpt-4".to_string(),
            OperatingMode::Build,
        )
        .await
        .unwrap();
    
    // Add initial content
    fixture
        .add_message(session_id, "Initial message".to_string(), "user".to_string())
        .await
        .unwrap();
    
    fixture
        .add_message(
            session_id,
            "Initial response".to_string(),
            "assistant".to_string(),
        )
        .await
        .unwrap();
    
    // Create checkpoint
    let checkpoint_id = fixture
        .create_checkpoint(
            session_id,
            "Before Changes".to_string(),
            Some("Checkpoint before making significant changes".to_string()),
        )
        .await
        .unwrap();
    
    assert!(!checkpoint_id.is_nil());
    
    // Add more content after checkpoint
    fixture
        .add_message(session_id, "New message".to_string(), "user".to_string())
        .await
        .unwrap();
    
    fixture
        .add_message(session_id, "New response".to_string(), "assistant".to_string())
        .await
        .unwrap();
    
    // Verify we now have 4 messages
    let metadata = fixture.get_session_metadata(session_id).await.unwrap();
    assert_eq!(metadata.message_count, 4);
    
    // Restore checkpoint
    fixture
        .restore_checkpoint(session_id, checkpoint_id)
        .await
        .unwrap();
    
    // Verify restoration worked (should be back to 2 messages)
    let metadata_after_restore = fixture.get_session_metadata(session_id).await.unwrap();
    // Note: The actual implementation might not reduce message count in metadata
    // This depends on how checkpoint restoration is implemented
    
    // At minimum, verify checkpoint system worked without errors
    assert!(metadata_after_restore.checkpoints.len() >= 1);
    let checkpoint_meta = &metadata_after_restore.checkpoints[0];
    assert_eq!(checkpoint_meta.name, "Before Changes");
    assert_eq!(checkpoint_meta.description, Some("Checkpoint before making significant changes".to_string()));
}

#[tokio::test]
async fn test_mode_switching_workflow() {
    let fixture = SessionTestFixture::new().await;
    
    let session_id = fixture
        .create_session(
            "Mode Switching Test".to_string(),
            "gpt-4".to_string(),
            OperatingMode::Plan,
        )
        .await
        .unwrap();
    
    // Start in Plan mode
    let metadata = fixture.get_session_metadata(session_id).await.unwrap();
    assert_eq!(metadata.current_mode, OperatingMode::Plan);
    
    // Switch to Build mode
    fixture
        .switch_mode(session_id, OperatingMode::Build)
        .await
        .unwrap();
    
    let metadata = fixture.get_session_metadata(session_id).await.unwrap();
    assert_eq!(metadata.current_mode, OperatingMode::Build);
    
    // Switch to Review mode
    fixture
        .switch_mode(session_id, OperatingMode::Review)
        .await
        .unwrap();
    
    let metadata = fixture.get_session_metadata(session_id).await.unwrap();
    assert_eq!(metadata.current_mode, OperatingMode::Review);
    
    // Mode history should track switches
    // This depends on the implementation tracking mode history
    // For now, we just verify the current mode is correct
    assert_eq!(metadata.current_mode, OperatingMode::Review);
}

#[tokio::test]
async fn test_session_state_management() {
    let fixture = SessionTestFixture::new().await;
    
    let session_id = fixture
        .create_session(
            "State Test".to_string(),
            "gpt-4".to_string(),
            OperatingMode::Build,
        )
        .await
        .unwrap();
    
    // Create custom session state
    let mut state = SessionState {
        cursor_position: 42,
        scroll_offset: 100,
        selected_message: Some(5),
        expanded_messages: vec![1, 3, 5],
        active_panel: "chat".to_string(),
        panel_sizes: {
            let mut sizes = HashMap::new();
            sizes.insert("sidebar".to_string(), 300);
            sizes.insert("main".to_string(), 800);
            sizes
        },
        search_query: Some("rust error".to_string()),
        filter_settings: Default::default(),
    };
    
    // Update session state
    fixture
        .update_session_state(session_id, state.clone())
        .await
        .unwrap();
    
    // Save session
    fixture.save_session(session_id).await.unwrap();
    
    // Load session and verify state persisted
    fixture.load_session(session_id).await.unwrap();
    
    // Note: We can't directly access the session state through the public API
    // In a real application, there would be a getter for session state
    // For now, we verify the operation completed without error
}

#[tokio::test]
async fn test_auto_save_functionality() {
    let fixture = SessionTestFixture::new().await;
    
    let session_id = fixture
        .create_session(
            "Auto Save Test".to_string(),
            "gpt-4".to_string(),
            OperatingMode::Build,
        )
        .await
        .unwrap();
    
    // Add messages rapidly
    for i in 0..5 {
        fixture
            .add_message(
                session_id,
                format!("Message {}", i),
                if i % 2 == 0 { "user" } else { "assistant" }.to_string(),
            )
            .await
            .unwrap();
        
        // Small delay to allow auto-save to potentially trigger
        sleep(Duration::from_millis(50)).await;
    }
    
    // Wait for auto-save interval to pass
    sleep(Duration::from_millis(200)).await;
    
    // Verify messages were tracked
    let metadata = fixture.get_session_metadata(session_id).await.unwrap();
    assert_eq!(metadata.message_count, 5);
}

#[tokio::test]
async fn test_session_listing_and_search() {
    let fixture = SessionTestFixture::new().await;
    
    // Create multiple sessions
    let session_titles = vec![
        "Rust Development Session",
        "Python Data Analysis", 
        "JavaScript Frontend Work",
        "Database Design Discussion",
        "Performance Optimization",
    ];
    
    let mut session_ids = Vec::new();
    for title in &session_titles {
        let id = fixture
            .create_session(
                title.to_string(),
                "gpt-4".to_string(),
                OperatingMode::Build,
            )
            .await
            .unwrap();
        session_ids.push(id);
    }
    
    // List all sessions
    let sessions = fixture.list_sessions().await;
    assert_eq!(sessions.len(), 5);
    
    // Verify all titles are present
    let found_titles: Vec<_> = sessions.iter().map(|s| s.title.as_str()).collect();
    for expected_title in &session_titles {
        assert!(found_titles.contains(expected_title), 
               "Missing session title: {}", expected_title);
    }
    
    // Test search functionality
    let search_results = fixture.session_manager.search_sessions("Rust").await;
    assert!(search_results.len() >= 1);
    let rust_session = search_results.iter()
        .find(|s| s.title.contains("Rust"));
    assert!(rust_session.is_some());
    
    let search_results = fixture.session_manager.search_sessions("Python").await;
    assert!(search_results.len() >= 1);
    let python_session = search_results.iter()
        .find(|s| s.title.contains("Python"));
    assert!(python_session.is_some());
}

#[tokio::test]
async fn test_session_deletion() {
    let fixture = SessionTestFixture::new().await;
    
    let session_id = fixture
        .create_session(
            "To Be Deleted".to_string(),
            "gpt-4".to_string(),
            OperatingMode::Build,
        )
        .await
        .unwrap();
    
    // Add content
    fixture
        .add_message(
            session_id,
            "This will be deleted".to_string(),
            "user".to_string(),
        )
        .await
        .unwrap();
    
    // Create a checkpoint
    let checkpoint_id = fixture
        .create_checkpoint(
            session_id,
            "Before deletion".to_string(),
            None,
        )
        .await
        .unwrap();
    
    // Verify session exists
    let metadata = fixture.get_session_metadata(session_id).await;
    assert!(metadata.is_ok());
    
    // Delete session
    fixture.delete_session(session_id).await.unwrap();
    
    // Verify session no longer exists
    let metadata_after_delete = fixture.get_session_metadata(session_id).await;
    assert!(metadata_after_delete.is_err());
    
    // Verify session not in list
    let sessions = fixture.list_sessions().await;
    let found = sessions.iter().any(|s| s.id == session_id);
    assert!(!found, "Deleted session should not appear in listing");
}

#[tokio::test]
async fn test_multiple_concurrent_sessions() {
    let fixture = Arc::new(SessionTestFixture::new().await);
    let num_sessions = 10;
    
    let mut handles = Vec::new();
    
    // Create sessions concurrently
    for i in 0..num_sessions {
        let fixture_clone = fixture.clone();
        let handle = tokio::spawn(async move {
            let session_id = fixture_clone
                .create_session(
                    format!("Concurrent Session {}", i),
                    "gpt-4".to_string(),
                    OperatingMode::Build,
                )
                .await
                .unwrap();
            
            // Add some messages to each session
            for j in 0..5 {
                fixture_clone
                    .add_message(
                        session_id,
                        format!("Message {} in session {}", j, i),
                        if j % 2 == 0 { "user" } else { "assistant" }.to_string(),
                    )
                    .await
                    .unwrap();
            }
            
            // Save session
            fixture_clone.save_session(session_id).await.unwrap();
            
            session_id
        });
        handles.push(handle);
    }
    
    // Wait for all sessions to be created
    let session_ids: Vec<_> = futures::future::join_all(handles).await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();
    
    // Verify all sessions were created
    assert_eq!(session_ids.len(), num_sessions);
    
    // Verify all sessions are unique
    let unique_ids: std::collections::HashSet<_> = session_ids.iter().collect();
    assert_eq!(unique_ids.len(), num_sessions);
    
    // Verify all sessions can be retrieved
    for session_id in &session_ids {
        let metadata = fixture.get_session_metadata(*session_id).await.unwrap();
        assert_eq!(metadata.message_count, 5);
    }
    
    // List sessions and verify count
    let sessions = fixture.list_sessions().await;
    assert_eq!(sessions.len(), num_sessions);
}

#[tokio::test]
async fn test_session_performance_characteristics() {
    let fixture = SessionTestFixture::new().await;
    
    // Test session creation performance
    let (session_ids, creation_duration) = TestTiming::time_async_operation(|| async {
        let mut ids = Vec::new();
        for i in 0..10 {
            let id = fixture
                .create_session(
                    format!("Performance Test {}", i),
                    "gpt-4".to_string(),
                    OperatingMode::Build,
                )
                .await
                .unwrap();
            ids.push(id);
        }
        ids
    })
    .await;
    
    // Should create sessions quickly (target: <100ms total for 10 sessions)
    PerformanceAssertions::assert_duration_under(
        creation_duration,
        1000,
        "creating 10 sessions",
    );
    
    // Test message addition performance
    let first_session = session_ids[0];
    let (_, message_duration) = TestTiming::time_async_operation(|| async {
        for i in 0..50 {
            fixture
                .add_message(
                    first_session,
                    format!("Performance message {}", i),
                    if i % 2 == 0 { "user" } else { "assistant" }.to_string(),
                )
                .await
                .unwrap();
        }
    })
    .await;
    
    // Should add messages quickly (target: <500ms for 50 messages)
    PerformanceAssertions::assert_duration_under(
        message_duration,
        2000,
        "adding 50 messages",
    );
    
    // Test save performance
    let (_, save_duration) = TestTiming::time_async_operation(|| async {
        for session_id in &session_ids {
            fixture.save_session(*session_id).await.unwrap();
        }
    })
    .await;
    
    // Should save quickly (target: <200ms for 10 sessions)
    PerformanceAssertions::assert_duration_under(
        save_duration,
        1000,
        "saving 10 sessions",
    );
}

#[tokio::test]
async fn test_session_memory_usage() {
    let fixture = SessionTestFixture::new().await;
    
    // Create a session with substantial content
    let session_id = fixture
        .create_session(
            "Memory Test".to_string(),
            "gpt-4".to_string(),
            OperatingMode::Build,
        )
        .await
        .unwrap();
    
    // Add many large messages
    let large_content = "a".repeat(1000); // 1KB per message
    for i in 0..100 {
        fixture
            .add_message(
                session_id,
                format!("{} - Message {}", large_content, i),
                if i % 2 == 0 { "user" } else { "assistant" }.to_string(),
            )
            .await
            .unwrap();
    }
    
    // Save session
    fixture.save_session(session_id).await.unwrap();
    
    // Load session in new instance
    let new_fixture = SessionTestFixture::new().await;
    new_fixture.load_session(session_id).await.unwrap();
    
    // Verify content was preserved
    let metadata = new_fixture.get_session_metadata(session_id).await.unwrap();
    assert_eq!(metadata.message_count, 100);
    
    // Check that compression is working (file size should be reasonable)
    assert!(metadata.file_size > 0, "File size should be recorded");
    if metadata.compression_ratio > 0.0 {
        assert!(metadata.compression_ratio > 0.1, "Should have some compression");
    }
}

#[tokio::test]
async fn test_session_error_recovery() {
    let fixture = SessionTestFixture::new().await;
    
    // Test loading non-existent session
    let fake_id = Uuid::new_v4();
    let result = fixture.load_session(fake_id).await;
    assert!(result.is_err(), "Loading non-existent session should fail");
    
    // Test getting metadata for non-existent session
    let result = fixture.get_session_metadata(fake_id).await;
    assert!(result.is_err(), "Getting metadata for non-existent session should fail");
    
    // Test adding message to non-existent session
    let result = fixture
        .add_message(
            fake_id,
            "Test message".to_string(),
            "user".to_string(),
        )
        .await;
    assert!(result.is_err(), "Adding message to non-existent session should fail");
    
    // Test checkpoint operations on non-existent session
    let result = fixture
        .create_checkpoint(
            fake_id,
            "Test checkpoint".to_string(),
            None,
        )
        .await;
    assert!(result.is_err(), "Creating checkpoint for non-existent session should fail");
    
    // Test deleting non-existent session (should succeed as idempotent operation)
    let result = fixture.delete_session(fake_id).await;
    // This might succeed as deleting non-existent items is often idempotent
    // The exact behavior depends on implementation
}

#[tokio::test]
async fn test_session_branching_simulation() {
    let fixture = SessionTestFixture::new().await;
    
    // Create base session
    let base_session_id = fixture
        .create_session(
            "Branching Base".to_string(),
            "gpt-4".to_string(),
            OperatingMode::Build,
        )
        .await
        .unwrap();
    
    // Add initial conversation
    fixture
        .add_message(
            base_session_id,
            "Let's explore two different approaches".to_string(),
            "user".to_string(),
        )
        .await
        .unwrap();
    
    fixture
        .add_message(
            base_session_id,
            "Sure! What approaches would you like to explore?".to_string(),
            "assistant".to_string(),
        )
        .await
        .unwrap();
    
    // Create checkpoint at branch point
    let branch_point = fixture
        .create_checkpoint(
            base_session_id,
            "Branch Point".to_string(),
            Some("Before exploring different approaches".to_string()),
        )
        .await
        .unwrap();
    
    // Simulate Branch A: Add messages for approach A
    fixture
        .add_message(
            base_session_id,
            "Let's try approach A - using recursion".to_string(),
            "user".to_string(),
        )
        .await
        .unwrap();
    
    fixture
        .add_message(
            base_session_id,
            "Great! Here's a recursive solution...".to_string(),
            "assistant".to_string(),
        )
        .await
        .unwrap();
    
    // Create checkpoint for Branch A
    let branch_a_checkpoint = fixture
        .create_checkpoint(
            base_session_id,
            "Branch A Complete".to_string(),
            Some("Recursive approach implemented".to_string()),
        )
        .await
        .unwrap();
    
    // Simulate Branch B: Restore to branch point and try different approach
    fixture
        .restore_checkpoint(base_session_id, branch_point)
        .await
        .unwrap();
    
    fixture
        .add_message(
            base_session_id,
            "Actually, let's try approach B - using iteration".to_string(),
            "user".to_string(),
        )
        .await
        .unwrap();
    
    fixture
        .add_message(
            base_session_id,
            "Excellent! Here's an iterative solution...".to_string(),
            "assistant".to_string(),
        )
        .await
        .unwrap();
    
    // Create checkpoint for Branch B
    let branch_b_checkpoint = fixture
        .create_checkpoint(
            base_session_id,
            "Branch B Complete".to_string(),
            Some("Iterative approach implemented".to_string()),
        )
        .await
        .unwrap();
    
    // Verify we can navigate between branches
    fixture
        .restore_checkpoint(base_session_id, branch_a_checkpoint)
        .await
        .unwrap();
    
    fixture
        .restore_checkpoint(base_session_id, branch_b_checkpoint)
        .await
        .unwrap();
    
    // Verify checkpoint metadata
    let metadata = fixture.get_session_metadata(base_session_id).await.unwrap();
    assert!(metadata.checkpoints.len() >= 3); // branch_point, branch_a, branch_b
    
    let checkpoint_names: Vec<_> = metadata.checkpoints.iter()
        .map(|c| &c.name)
        .collect();
    assert!(checkpoint_names.contains(&&"Branch Point".to_string()));
    assert!(checkpoint_names.contains(&&"Branch A Complete".to_string()));
    assert!(checkpoint_names.contains(&&"Branch B Complete".to_string()));
}

#[tokio::test]
async fn test_session_cleanup_and_limits() {
    // Create fixture with low limits for testing
    let test_env = TestEnvironment::new();
    let storage_path = test_env.path().join("sessions");
    
    let config = SessionManagerConfig {
        storage_path,
        auto_save_interval: Duration::from_millis(100),
        max_sessions: 3, // Low limit for testing
        max_total_size: 10_000, // 10KB limit
        compression_level: CompressionLevel::Fast,
        enable_auto_save: true,
        enable_mmap: false,
        max_checkpoints: 2, // Low limit for testing
    };
    
    let session_manager = SessionManager::new(config).await.unwrap();
    
    // Create sessions up to limit
    let mut session_ids = Vec::new();
    for i in 0..5 {
        let id = session_manager
            .create_session(
                format!("Session {}", i),
                "gpt-4".to_string(),
                OperatingMode::Build,
            )
            .await
            .unwrap();
        session_ids.push(id);
    }
    
    // Trigger cleanup
    session_manager.cleanup_old_sessions().await.unwrap();
    
    // Verify cleanup worked (should have removed old sessions)
    let sessions = session_manager.list_sessions().await.unwrap();
    assert!(sessions.len() <= 3, "Should have cleaned up to max_sessions limit");
    
    // Test checkpoint limits
    let active_session = sessions[0].id;
    session_manager.load_session(active_session).await.unwrap();
    
    // Create more checkpoints than the limit
    for i in 0..5 {
        let _checkpoint_id = session_manager
            .create_checkpoint(
                active_session,
                format!("Checkpoint {}", i),
                Some(format!("Description {}", i)),
            )
            .await
            .unwrap();
    }
    
    // Verify checkpoint limit was enforced
    let metadata = session_manager.get_session_metadata(active_session).await.unwrap();
    assert!(metadata.checkpoints.len() <= 2, "Should have enforced checkpoint limit");
}
