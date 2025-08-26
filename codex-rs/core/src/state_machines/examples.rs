//! Comprehensive examples demonstrating state machine patterns and usage.
//!
//! This module contains practical examples of how to use the state machine
//! patterns correctly, with focus on compile-time safety and runtime efficiency.

use super::*;
use crate::builders::AstLanguage;
use crate::builders::AstQuery;
use crate::builders::SearchQuery;
use crate::types::FilePath;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Example 1: Basic Builder Pattern Usage
///
/// This shows the correct way to use typestate builders with compile-time
/// enforcement of construction order.
pub fn example_search_query_builder() -> Result<SearchQuery, crate::builders::BuilderError> {
    println!("=== Search Query Builder Example ===");

    // ✅ This compiles - correct state transitions
    let query = SearchQuery::builder()
        .pattern("fn main")? // Init -> Validated
        .scope_current_dir() // Validated -> Ready
        .case_sensitive(true) // Ready -> Ready (configuration)
        .max_results(Some(100)) // Ready -> Ready (configuration)
        .include_hidden(false) // Ready -> Ready (configuration)
        .build()?; // Ready -> SearchQuery

    println!("Built query: pattern = '{}'", query.pattern().as_str());
    println!("Query scope: {:?}", query.scope());
    println!("Case sensitive: {}", query.config().case_sensitive);

    Ok(query)
}

/// Example 2: AST Query Builder
///
/// Shows how to build AST queries with proper state transitions.
pub fn example_ast_query_builder() -> Result<AstQuery, crate::builders::BuilderError> {
    println!("=== AST Query Builder Example ===");

    let file1 = FilePath::new("src/main.rs").unwrap();
    let file2 = FilePath::new("src/lib.rs").unwrap();

    let query = AstQuery::builder()
        .language(AstLanguage::Rust) // Init -> Validated
        .add_file(file1) // Validated -> Validated (accumulate)
        .add_file(file2) // Validated -> Validated (accumulate)
        .select_node_type("function_item") // Validated -> Ready
        .max_depth(Some(5)) // Ready -> Ready (config)
        .include_metadata(true) // Ready -> Ready (config)
        .context_lines(Some(3)) // Ready -> Ready (config)
        .build()?; // Ready -> AstQuery

    println!("Built AST query for language: {:?}", query.language());
    println!("Target files: {}", query.target_files().len());
    println!("Max depth: {:?}", query.config().max_depth);

    Ok(query)
}

/// Example 3: Runtime State Machine Tracking
///
/// Demonstrates how to use TrackedStateMachine for runtime state management.
pub fn example_runtime_state_machine() {
    println!("=== Runtime State Machine Example ===");

    // Create a tracked state machine starting in Ready state
    let mut machine = TrackedStateMachine::<Ready>::new();

    // Record some state transitions
    let transitions = vec![
        StateTransition::new("Ready", "Processing")
            .with_metadata("operation", "file_scan")
            .with_metadata("user", "system"),
        StateTransition::new("Processing", "Completed")
            .with_metadata("files_processed", "1247")
            .with_metadata("duration_ms", "2341"),
    ];

    for transition in transitions {
        machine.record_transition(transition);
    }

    // Query the state machine
    println!("Total transitions: {}", machine.transition_history().len());
    println!(
        "Transitions to 'Processing': {}",
        machine.count_transitions_to("Processing")
    );
    println!(
        "Transitions to 'Completed': {}",
        machine.count_transitions_to("Completed")
    );

    if let Some(last_transition) = machine.last_transition() {
        println!(
            "Last transition: {} -> {}",
            last_transition.from_state, last_transition.to_state
        );
        println!("Metadata: {:?}", last_transition.metadata);
    }

    if let Ok(uptime) = machine.uptime() {
        println!("Machine uptime: {:?}", uptime);
    }
}

/// Example 4: Concurrent State Machine Access
///
/// Shows how to safely share state machines across threads.
pub fn example_concurrent_state_machines() {
    println!("=== Concurrent State Machines Example ===");

    let machine = Arc::new(TrackedStateMachine::<Ready>::new());

    // Spawn multiple threads to demonstrate thread-safe access
    let handles: Vec<_> = (0..4)
        .map(|i| {
            let machine_clone = Arc::clone(&machine);
            thread::spawn(move || {
                println!(
                    "Thread {} - Machine created at: {:?}",
                    i,
                    machine_clone.created_at()
                );

                // Simulate some work
                thread::sleep(Duration::from_millis(100 * i));

                if let Ok(uptime) = machine_clone.uptime() {
                    println!("Thread {} - Current uptime: {:?}", i, uptime);
                }

                // In real applications, state transitions would be coordinated
                // through proper synchronization primitives like channels or mutexes
            })
        })
        .collect();

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    println!("All threads completed successfully");
}

/// Example 5: State Validation and Error Handling
///
/// Demonstrates proper error handling and state validation patterns.
pub fn example_state_validation() {
    println!("=== State Validation Example ===");

    // Example of validation that would catch errors
    fn validate_transition(from: &'static str, to: &'static str) -> Result<(), StateMachineError> {
        // Custom validation logic
        match (from, to) {
            ("Uninitialized", "Ready") => Ok(()),
            ("Ready", "Processing") => Ok(()),
            ("Processing", "Completed") => Ok(()),
            ("Processing", "Failed") => Ok(()),
            ("Completed", "Ready") => Ok(()), // Allow restart
            (f, t) => Err(StateMachineError::InvalidTransition { from: f, to: t }),
        }
    }

    // Test valid transitions
    let valid_cases = vec![
        ("Uninitialized", "Ready"),
        ("Ready", "Processing"),
        ("Processing", "Completed"),
    ];

    for (from, to) in valid_cases {
        match validate_transition(from, to) {
            Ok(()) => println!("✅ Valid transition: {} -> {}", from, to),
            Err(e) => println!("❌ Invalid transition: {}", e),
        }
    }

    // Test invalid transitions
    let invalid_cases = vec![
        ("Uninitialized", "Processing"), // Skip validation
        ("Completed", "Processing"),     // Can't go backwards
        ("Failed", "Processing"),        // Can't resume directly
    ];

    for (from, to) in invalid_cases {
        match validate_transition(from, to) {
            Ok(()) => println!("✅ Valid transition: {} -> {}", from, to),
            Err(e) => println!("❌ Invalid transition: {}", e),
        }
    }
}

/// Example 6: Performance Demonstration
///
/// Shows that typestate patterns have zero runtime cost.
pub fn example_performance_characteristics() {
    println!("=== Performance Characteristics Example ===");

    use std::mem;

    // Demonstrate zero-cost abstractions
    println!(
        "Size of StatePhantom<Uninitialized>: {} bytes",
        mem::size_of::<StatePhantom<Uninitialized>>()
    );
    println!(
        "Size of StatePhantom<Processing>: {} bytes",
        mem::size_of::<StatePhantom<Processing>>()
    );
    println!(
        "Size of StatePhantom<Ready>: {} bytes",
        mem::size_of::<StatePhantom<Ready>>()
    );

    // Show that different state phantoms are equal (they're all empty)
    let phantom1 = StatePhantom::<Ready>::new();
    let phantom2 = StatePhantom::<Ready>::new();
    assert_eq!(phantom1, phantom2);

    println!("✅ PhantomData comparison works correctly");

    // Demonstrate runtime state machine costs
    let machine = TrackedStateMachine::<Ready>::new();
    println!(
        "Size of empty TrackedStateMachine: {} bytes",
        mem::size_of_val(&machine)
    );

    // Add some transitions and measure growth
    let mut machine_with_data = TrackedStateMachine::<Ready>::new();
    for i in 0..10 {
        let transition =
            StateTransition::new("Ready", "Processing").with_metadata("iteration", i.to_string());
        machine_with_data.record_transition(transition);
    }

    println!(
        "Size with 10 transitions: {} bytes",
        mem::size_of_val(&machine_with_data)
    );
    println!(
        "Transitions stored: {}",
        machine_with_data.transition_history().len()
    );
}

/// Example 7: Builder Error Recovery
///
/// Shows how to handle and recover from builder errors.
pub fn example_error_recovery() {
    println!("=== Error Recovery Example ===");

    // Attempt to build with invalid data
    let result = SearchQuery::builder()
        .pattern("") // This should fail - empty pattern
        .and_then(|builder| builder.scope_current_dir().build());

    match result {
        Ok(query) => {
            println!("✅ Successfully built query: {:?}", query.pattern());
        }
        Err(e) => {
            println!("❌ Builder error: {}", e);

            // Show recovery - build a valid query instead
            match SearchQuery::builder()
                .pattern("fn main")
                .and_then(|builder| {
                    builder.scope_current_dir().case_sensitive(false).build()
                }) {
                Ok(query) => println!("✅ Recovery successful: '{}'", query.pattern().as_str()),
                Err(recovery_error) => println!("❌ Recovery failed: {}", recovery_error),
            }
        }
    }
}

/// Run all examples
pub fn run_all_examples() {
    println!("🚀 Running State Machine Examples...\n");

    if let Err(e) = example_search_query_builder() {
        println!("Search query builder failed: {}", e);
    }
    println!();

    if let Err(e) = example_ast_query_builder() {
        println!("AST query builder failed: {}", e);
    }
    println!();

    example_runtime_state_machine();
    println!();

    example_concurrent_state_machines();
    println!();

    example_state_validation();
    println!();

    example_performance_characteristics();
    println!();

    example_error_recovery();
    println!();

    println!("✅ All examples completed successfully!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_query_builder_example() {
        let result = example_search_query_builder();
        assert!(result.is_ok());

        let query = result.unwrap();
        assert_eq!(query.pattern().as_str(), "fn main");
        assert!(query.config().case_sensitive);
    }

    #[test]
    fn test_ast_query_builder_example() {
        let result = example_ast_query_builder();
        assert!(result.is_ok());

        let query = result.unwrap();
        assert_eq!(query.language(), &AstLanguage::Rust);
        assert_eq!(query.target_files().len(), 2);
    }

    #[test]
    fn test_runtime_state_machine_tracking() {
        let mut machine = TrackedStateMachine::<Ready>::new();

        let transition = StateTransition::new("Ready", "Processing").with_metadata("test", "value");
        machine.record_transition(transition);

        assert_eq!(machine.transition_history().len(), 1);
        assert_eq!(machine.count_transitions_to("Processing"), 1);

        let last = machine.last_transition().unwrap();
        assert_eq!(last.from_state, "Ready");
        assert_eq!(last.to_state, "Processing");
        assert_eq!(last.metadata.get("test"), Some(&"value".to_string()));
    }

    #[test]
    fn test_phantom_data_zero_cost() {
        use std::mem;

        // Verify zero-cost abstractions
        assert_eq!(mem::size_of::<StatePhantom<Ready>>(), 0);
        assert_eq!(mem::size_of::<StatePhantom<Processing>>(), 0);
        assert_eq!(mem::size_of::<StatePhantom<Completed>>(), 0);

        // Verify equality works
        let p1 = StatePhantom::<Ready>::new();
        let p2 = StatePhantom::<Ready>::new();
        assert_eq!(p1, p2);
    }

    #[test]
    fn test_state_validator() {
        // Test that state validator can create transitions
        let transition = StateValidator::create_transition::<Ready, Processing>();

        assert_eq!(transition.from_state, std::any::type_name::<Ready>());
        assert_eq!(transition.to_state, std::any::type_name::<Processing>());
        assert!(transition.timestamp <= std::time::SystemTime::now());
    }

    #[test]
    fn test_transition_metadata() {
        let transition = StateTransition::new("Test1", "Test2")
            .with_metadata("key1", "value1")
            .with_metadata("key2", "value2");

        assert_eq!(transition.from_state, "Test1");
        assert_eq!(transition.to_state, "Test2");
        assert_eq!(transition.metadata.len(), 2);
        assert_eq!(transition.metadata.get("key1"), Some(&"value1".to_string()));
        assert_eq!(transition.metadata.get("key2"), Some(&"value2".to_string()));
    }
}
