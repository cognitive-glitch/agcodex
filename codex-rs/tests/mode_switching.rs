//! Integration tests for operating mode switching functionality.
//!
//! Tests the complete mode switching workflow using real AGCodex components:
//! - ModeManager from agcodex_core::modes
//! - Mode enforcement and restrictions
//! - Visual indicators and state persistence
//! - Performance characteristics
//!
//! Mocks external API calls but uses real mode management logic.

use agcodex_core::modes::{ModeManager, OperatingMode};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;

mod helpers;
use helpers::test_utils::{
    TestEnvironment, PerformanceAssertions, TestTiming, AsyncTestHelpers,
};

/// Test fixture for mode switching integration tests
struct ModeTestFixture {
    mode_manager: Arc<Mutex<ModeManager>>,
    _test_env: TestEnvironment,
}

impl ModeTestFixture {
    fn new() -> Self {
        let test_env = TestEnvironment::new();
        let mode_manager = Arc::new(Mutex::new(ModeManager::new(OperatingMode::Build)));
        
        Self {
            mode_manager,
            _test_env: test_env,
        }
    }
    
    fn with_initial_mode(mode: OperatingMode) -> Self {
        let test_env = TestEnvironment::new();
        let mode_manager = Arc::new(Mutex::new(ModeManager::new(mode)));
        
        Self {
            mode_manager,
            _test_env: test_env,
        }
    }
    
    fn current_mode(&self) -> OperatingMode {
        self.mode_manager.lock().unwrap().current_mode()
    }
    
    fn cycle_mode(&self) -> OperatingMode {
        self.mode_manager.lock().unwrap().cycle()
    }
    
    fn switch_mode(&self, mode: OperatingMode) {
        self.mode_manager.lock().unwrap().switch_mode(mode);
    }
    
    fn mode_history_len(&self) -> usize {
        self.mode_manager.lock().unwrap().mode_history.len()
    }
    
    fn validate_file_write(&self, path: &str, size: Option<usize>) -> Result<(), String> {
        self.mode_manager.lock().unwrap().validate_file_write(path, size)
    }
    
    fn validate_command_execution(&self, command: &str) -> Result<(), String> {
        self.mode_manager.lock().unwrap().validate_command_execution(command)
    }
    
    fn validate_git_operation(&self, operation: &str) -> Result<(), String> {
        self.mode_manager.lock().unwrap().validate_git_operation(operation)
    }
    
    fn current_visuals(&self) -> String {
        self.mode_manager.lock().unwrap().current_visuals().indicator.to_string()
    }
}

#[tokio::test]
async fn test_basic_mode_cycling() {
    let fixture = ModeTestFixture::new();
    
    // Start in Build mode
    assert_eq!(fixture.current_mode(), OperatingMode::Build);
    assert_eq!(fixture.current_visuals(), "🔨 BUILD");
    
    // Cycle to Review
    let new_mode = fixture.cycle_mode();
    assert_eq!(new_mode, OperatingMode::Review);
    assert_eq!(fixture.current_mode(), OperatingMode::Review);
    assert_eq!(fixture.current_visuals(), "🔍 REVIEW");
    
    // Cycle to Plan
    let new_mode = fixture.cycle_mode();
    assert_eq!(new_mode, OperatingMode::Plan);
    assert_eq!(fixture.current_mode(), OperatingMode::Plan);
    assert_eq!(fixture.current_visuals(), "📋 PLAN");
    
    // Cycle back to Build
    let new_mode = fixture.cycle_mode();
    assert_eq!(new_mode, OperatingMode::Build);
    assert_eq!(fixture.current_mode(), OperatingMode::Build);
    assert_eq!(fixture.current_visuals(), "🔨 BUILD");
    
    // Verify history was tracked
    assert_eq!(fixture.mode_history_len(), 3);
}

#[tokio::test]
async fn test_plan_mode_restrictions() {
    let fixture = ModeTestFixture::with_initial_mode(OperatingMode::Plan);
    
    // Plan mode should be read-only
    assert!({
        let manager = fixture.mode_manager.lock().unwrap();
        manager.is_read_only()
    });
    
    // File writes should be disallowed
    assert!(fixture.validate_file_write("test.txt", Some(100)).is_err());
    assert!(fixture.validate_file_write("large_file.txt", Some(50_000)).is_err());
    
    // Command execution should be disallowed
    assert!(fixture.validate_command_execution("ls -la").is_err());
    assert!(fixture.validate_command_execution("rm -rf /tmp/file").is_err());
    
    // Git operations should be disallowed
    assert!(fixture.validate_git_operation("commit").is_err());
    assert!(fixture.validate_git_operation("push").is_err());
    
    // Network access should be allowed (for research)
    assert!({
        let manager = fixture.mode_manager.lock().unwrap();
        manager.can_access_network()
    });
}

#[tokio::test]
async fn test_build_mode_permissions() {
    let fixture = ModeTestFixture::with_initial_mode(OperatingMode::Build);
    
    // Build mode should allow everything
    assert!(!{
        let manager = fixture.mode_manager.lock().unwrap();
        manager.is_read_only()
    });
    
    // File writes should be allowed
    assert!(fixture.validate_file_write("test.txt", Some(100)).is_ok());
    assert!(fixture.validate_file_write("large_file.txt", Some(1_000_000)).is_ok());
    
    // Command execution should be allowed
    assert!(fixture.validate_command_execution("ls -la").is_ok());
    assert!(fixture.validate_command_execution("cargo build").is_ok());
    
    // Git operations should be allowed
    assert!(fixture.validate_git_operation("commit").is_ok());
    assert!(fixture.validate_git_operation("push").is_ok());
    
    // All capabilities should be enabled
    let manager = fixture.mode_manager.lock().unwrap();
    assert!(manager.allows_write());
    assert!(manager.allows_execution());
    assert!(manager.can_access_network());
    assert!(manager.can_perform_git_operations());
}

#[tokio::test]
async fn test_review_mode_limitations() {
    let fixture = ModeTestFixture::with_initial_mode(OperatingMode::Review);
    
    // Review mode allows limited writes
    assert!(!{
        let manager = fixture.mode_manager.lock().unwrap();
        manager.is_read_only()
    });
    
    // Small file writes should be allowed (under 10KB limit)
    assert!(fixture.validate_file_write("small_file.txt", Some(5_000)).is_ok());
    assert!(fixture.validate_file_write("at_limit.txt", Some(10_000)).is_ok());
    
    // Large file writes should be rejected
    assert!(fixture.validate_file_write("large_file.txt", Some(15_000)).is_err());
    assert!(fixture.validate_file_write("huge_file.txt", Some(100_000)).is_err());
    
    // Command execution should be disallowed
    assert!(fixture.validate_command_execution("rm -rf /tmp").is_err());
    assert!(fixture.validate_command_execution("cargo build").is_err());
    
    // Git operations should be disallowed
    assert!(fixture.validate_git_operation("commit").is_err());
    assert!(fixture.validate_git_operation("push").is_err());
    
    // Network access should be allowed
    assert!({
        let manager = fixture.mode_manager.lock().unwrap();
        manager.can_access_network()
    });
}

#[tokio::test]
async fn test_mode_switching_workflow() {
    let fixture = ModeTestFixture::new();
    
    // Typical user workflow: Build -> Review -> Plan -> Build
    
    // Start in Build mode for development
    assert_eq!(fixture.current_mode(), OperatingMode::Build);
    assert!(fixture.validate_file_write("src/main.rs", Some(5000)).is_ok());
    assert!(fixture.validate_command_execution("cargo test").is_ok());
    
    // Switch to Review mode for code review
    fixture.switch_mode(OperatingMode::Review);
    assert_eq!(fixture.current_mode(), OperatingMode::Review);
    assert!(fixture.validate_file_write("small_fix.rs", Some(1000)).is_ok());
    assert!(fixture.validate_file_write("large_refactor.rs", Some(50000)).is_err());
    assert!(fixture.validate_command_execution("cargo test").is_err());
    
    // Switch to Plan mode for analysis
    fixture.switch_mode(OperatingMode::Plan);
    assert_eq!(fixture.current_mode(), OperatingMode::Plan);
    assert!(fixture.validate_file_write("any_file.rs", Some(100)).is_err());
    assert!(fixture.validate_command_execution("ls").is_err());
    
    // Back to Build mode for implementation
    fixture.switch_mode(OperatingMode::Build);
    assert_eq!(fixture.current_mode(), OperatingMode::Build);
    assert!(fixture.validate_file_write("implementation.rs", Some(100000)).is_ok());
    assert!(fixture.validate_command_execution("cargo build").is_ok());
    
    // Verify mode history tracks all switches
    assert_eq!(fixture.mode_history_len(), 3);
}

#[tokio::test]
async fn test_rapid_mode_switching() {
    let fixture = ModeTestFixture::new();
    let iterations = 100;
    
    let (_, duration) = TestTiming::time_async_operation(|| async {
        for i in 0..iterations {
            // Cycle through all modes rapidly
            fixture.cycle_mode();
            
            // Verify state is consistent after each switch
            let current = fixture.current_mode();
            let expected_position = (i + 1) % 3;
            let expected_mode = match expected_position {
                0 => OperatingMode::Build,
                1 => OperatingMode::Review,
                2 => OperatingMode::Plan,
                _ => unreachable!(),
            };
            
            // Note: Starting from Build, first cycle goes to Review
            let actual_expected = match (i + 1) % 3 {
                1 => OperatingMode::Review,  // Build -> Review
                2 => OperatingMode::Plan,    // Review -> Plan
                0 => OperatingMode::Build,   // Plan -> Build
                _ => unreachable!(),
            };
            
            assert_eq!(current, actual_expected, "Mode mismatch at iteration {}", i);
        }
    }).await;
    
    // Should handle rapid switching efficiently (target: <50ms per switch)
    let avg_per_switch = duration.as_millis() as f64 / iterations as f64;
    assert!(avg_per_switch < 1.0, "Mode switching too slow: {:.2}ms per switch", avg_per_switch);
    
    // Verify all switches were tracked
    assert_eq!(fixture.mode_history_len(), iterations);
}

#[tokio::test]
async fn test_concurrent_mode_access() {
    let fixture = Arc::new(ModeTestFixture::new());
    let mut handles = Vec::new();
    
    // Spawn multiple tasks that access mode concurrently
    for i in 0..10 {
        let fixture_clone = fixture.clone();
        let handle = tokio::spawn(async move {
            // Each task switches modes and validates state
            for _ in 0..10 {
                fixture_clone.cycle_mode();
                let mode = fixture_clone.current_mode();
                
                // Verify mode-specific behavior
                match mode {
                    OperatingMode::Plan => {
                        assert!(fixture_clone.validate_file_write("test.txt", Some(100)).is_err());
                    }
                    OperatingMode::Build => {
                        assert!(fixture_clone.validate_file_write("test.txt", Some(100)).is_ok());
                        assert!(fixture_clone.validate_command_execution("ls").is_ok());
                    }
                    OperatingMode::Review => {
                        assert!(fixture_clone.validate_file_write("small.txt", Some(1000)).is_ok());
                        assert!(fixture_clone.validate_file_write("large.txt", Some(50000)).is_err());
                    }
                }
                
                // Small delay to allow interleaving
                tokio::time::sleep(Duration::from_micros(10)).await;
            }
            
            i // Return task id for verification
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    let results: Vec<_> = futures::future::join_all(handles).await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();
    
    // Verify all tasks completed successfully
    assert_eq!(results.len(), 10);
    for (i, &task_id) in results.iter().enumerate() {
        assert_eq!(i, task_id);
    }
    
    // Mode should be in a valid state
    let final_mode = fixture.current_mode();
    assert!(matches!(final_mode, OperatingMode::Plan | OperatingMode::Build | OperatingMode::Review));
}

#[tokio::test]
async fn test_mode_restriction_error_messages() {
    let fixture = ModeTestFixture::with_initial_mode(OperatingMode::Plan);
    
    // Test file write restriction message
    let file_write_error = fixture.validate_file_write("test.txt", Some(100)).unwrap_err();
    assert!(file_write_error.contains("Plan mode"));
    assert!(file_write_error.contains("read-only"));
    assert!(file_write_error.contains("Shift+Tab"));
    
    // Test command execution restriction message
    let command_error = fixture.validate_command_execution("ls").unwrap_err();
    assert!(command_error.contains("Plan mode"));
    assert!(command_error.contains("read-only"));
    
    // Test Review mode file size restriction
    fixture.switch_mode(OperatingMode::Review);
    let large_file_error = fixture.validate_file_write("large.txt", Some(50000)).unwrap_err();
    assert!(large_file_error.contains("Review mode"));
    assert!(large_file_error.contains("10000 bytes") || large_file_error.contains("byte limit"));
    assert!(large_file_error.contains("Build mode"));
}

#[tokio::test]
async fn test_mode_visual_indicators() {
    let fixture = ModeTestFixture::new();
    
    // Test visual indicators for each mode
    let test_cases = [
        (OperatingMode::Plan, "📋 PLAN"),
        (OperatingMode::Build, "🔨 BUILD"),
        (OperatingMode::Review, "🔍 REVIEW"),
    ];
    
    for (mode, expected_indicator) in test_cases {
        fixture.switch_mode(mode);
        assert_eq!(fixture.current_visuals(), expected_indicator);
        
        // Verify visual consistency with mode
        let manager = fixture.mode_manager.lock().unwrap();
        let visuals = manager.current_visuals();
        assert_eq!(visuals.indicator, expected_indicator);
        
        // Check description is appropriate
        match mode {
            OperatingMode::Plan => assert!(visuals.description.contains("Read-only")),
            OperatingMode::Build => assert!(visuals.description.contains("Full access")),
            OperatingMode::Review => assert!(visuals.description.contains("Quality")),
        }
    }
}

#[tokio::test]
async fn test_mode_switching_performance() {
    let fixture = ModeTestFixture::new();
    let iterations = 1000;
    
    let (_, total_duration) = TestTiming::time_async_operation(|| async {
        for _ in 0..iterations {
            fixture.cycle_mode();
        }
    }).await;
    
    // Performance assertions
    PerformanceAssertions::assert_duration_under(total_duration, 100, "1000 mode switches");
    
    let avg_per_switch = total_duration.as_nanos() as f64 / iterations as f64;
    assert!(avg_per_switch < 100_000.0, "Average switch time {}ns too high", avg_per_switch); // < 0.1ms per switch
}

#[tokio::test]
async fn test_mode_state_consistency() {
    let fixture = ModeTestFixture::new();
    
    // Test that mode state remains consistent across operations
    for _ in 0..50 {
        let mode_before = fixture.current_mode();
        
        // Perform various operations based on current mode
        match mode_before {
            OperatingMode::Plan => {
                // All write operations should fail
                assert!(fixture.validate_file_write("test.txt", Some(100)).is_err());
                assert!(fixture.validate_command_execution("test").is_err());
                assert!(fixture.validate_git_operation("commit").is_err());
            }
            OperatingMode::Build => {
                // All operations should succeed
                assert!(fixture.validate_file_write("test.txt", Some(100_000)).is_ok());
                assert!(fixture.validate_command_execution("test").is_ok());
                assert!(fixture.validate_git_operation("commit").is_ok());
            }
            OperatingMode::Review => {
                // Limited operations should succeed
                assert!(fixture.validate_file_write("small.txt", Some(5_000)).is_ok());
                assert!(fixture.validate_file_write("large.txt", Some(50_000)).is_err());
                assert!(fixture.validate_command_execution("test").is_err());
                assert!(fixture.validate_git_operation("commit").is_err());
            }
        }
        
        // Mode should not change from validation operations
        let mode_after = fixture.current_mode();
        assert_eq!(mode_before, mode_after, "Mode changed unexpectedly during validation");
        
        // Switch to next mode
        fixture.cycle_mode();
    }
}

#[tokio::test]
async fn test_mode_thread_safety() {
    let fixture = Arc::new(ModeTestFixture::new());
    let num_threads = 20;
    let operations_per_thread = 100;
    
    let mut handles = Vec::new();
    
    for thread_id in 0..num_threads {
        let fixture_clone = fixture.clone();
        let handle = tokio::spawn(async move {
            let mut successful_operations = 0;
            
            for operation_id in 0..operations_per_thread {
                // Mix of mode switches and validation operations
                if operation_id % 3 == 0 {
                    fixture_clone.cycle_mode();
                }
                
                // Validate current permissions
                let current_mode = fixture_clone.current_mode();
                let file_result = fixture_clone.validate_file_write("test.txt", Some(1000));
                let cmd_result = fixture_clone.validate_command_execution("test");
                
                // Results should be consistent with current mode
                match current_mode {
                    OperatingMode::Plan => {
                        if file_result.is_err() && cmd_result.is_err() {
                            successful_operations += 1;
                        }
                    }
                    OperatingMode::Build => {
                        if file_result.is_ok() && cmd_result.is_ok() {
                            successful_operations += 1;
                        }
                    }
                    OperatingMode::Review => {
                        if file_result.is_ok() && cmd_result.is_err() {
                            successful_operations += 1;
                        }
                    }
                }
                
                // Small delay to encourage race conditions
                if thread_id % 2 == 0 {
                    tokio::time::sleep(Duration::from_nanos(1)).await;
                }
            }
            
            successful_operations
        });
        handles.push(handle);
    }
    
    // Wait for all threads and collect results
    let results: Vec<_> = futures::future::join_all(handles).await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();
    
    // All threads should have mostly successful operations (allowing for timing)
    let total_successful: usize = results.iter().sum();
    let total_operations = num_threads * operations_per_thread;
    let success_rate = total_successful as f64 / total_operations as f64;
    
    // Should have at least 90% consistent behavior despite concurrency
    assert!(success_rate >= 0.90, 
           "Thread safety concern: only {:.1}% operations were consistent", 
           success_rate * 100.0);
    
    println!("Thread safety test: {}/{} ({:.1}%) operations were consistent", 
             total_successful, total_operations, success_rate * 100.0);
}
