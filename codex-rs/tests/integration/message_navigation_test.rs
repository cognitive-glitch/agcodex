//! Integration tests for the Message Navigation System
//!
//! This test module verifies that all three components of the message navigation
//! system work together correctly:
//! - Message Jump (Ctrl+J)
//! - History Browser (Ctrl+H)
//! - Undo/Redo System

use agcodex_core::conversation::{
    ConversationSnapshot, SnapshotMetadata, UndoRedoManager,
};
use agcodex_core::models::{ContentItem, ResponseItem};
use agcodex_tui::{HistoryBrowser, MessageJump, RoleFilter};

/// Helper function to create a test message
fn create_test_message(role: &str, content: &str) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: role.to_string(),
        content: vec![ContentItem::OutputText {
            text: content.to_string(),
        }],
    }
}

/// Helper function to create test metadata
fn create_test_metadata(turn: usize, mode: &str) -> SnapshotMetadata {
    SnapshotMetadata {
        turn_number: turn,
        total_tokens: turn * 100,
        model: "test-model".to_string(),
        mode: mode.to_string(),
        user: Some("test-user".to_string()),
        tags: Vec::new(),
    }
}

#[test]
fn test_message_jump_integration() {
    // Create a message jump widget
    let mut jump = MessageJump::new();
    
    // Create test conversation
    let messages = vec![
        create_test_message("user", "Hello, how are you?"),
        create_test_message("assistant", "I'm doing well, thank you!"),
        create_test_message("user", "Can you help me with Rust?"),
        create_test_message("assistant", "Of course! What would you like to know?"),
    ];
    
    // Show the jump widget with messages
    jump.show(messages.clone());
    assert!(jump.is_visible());
    
    // Test filtering by role
    jump.cycle_role_filter();
    assert_eq!(jump.role_filter(), RoleFilter::User);
    
    // Test search functionality
    jump.set_search_query("Rust".to_string());
    assert_eq!(jump.search_query(), "Rust");
    
    // Test navigation
    jump.move_down();
    jump.move_up();
    
    // Get selected message
    let selected = jump.selected_message();
    assert!(selected.is_some());
    
    // Hide the widget
    jump.hide();
    assert!(!jump.is_visible());
}

#[test]
fn test_history_browser_integration() {
    // Create a history browser
    let mut browser = HistoryBrowser::new();
    
    // Create test conversation with branches
    let messages = vec![
        create_test_message("user", "Initial question"),
        create_test_message("assistant", "Initial response"),
        create_test_message("user", "Follow-up question"),
    ];
    
    // Show the browser
    browser.show(messages.clone());
    assert!(browser.is_visible());
    
    // Test navigation
    browser.move_down();
    browser.move_up();
    
    // Test branch creation
    let branch_created = browser.create_branch_from_selected(
        "Alternative path".to_string(),
        create_test_message("assistant", "Alternative response"),
    );
    assert!(branch_created);
    
    // Test preview toggle
    browser.toggle_preview();
    
    // Hide the browser
    browser.hide();
    assert!(!browser.is_visible());
}

#[test]
fn test_undo_redo_manager_integration() {
    // Create an undo/redo manager
    let mut manager = UndoRedoManager::new();
    
    // Create initial conversation state
    let state1 = vec![
        create_test_message("user", "First message"),
    ];
    let metadata1 = create_test_metadata(1, "Build");
    let id1 = manager.save_state(state1.clone(), metadata1).unwrap();
    assert!(manager.current_state().is_some());
    
    // Add second state
    let state2 = vec![
        create_test_message("user", "First message"),
        create_test_message("assistant", "First response"),
    ];
    let metadata2 = create_test_metadata(2, "Build");
    let _id2 = manager.save_state(state2.clone(), metadata2).unwrap();
    
    // Add third state
    let state3 = vec![
        create_test_message("user", "First message"),
        create_test_message("assistant", "First response"),
        create_test_message("user", "Second message"),
    ];
    let metadata3 = create_test_metadata(3, "Build");
    let _id3 = manager.save_state(state3.clone(), metadata3).unwrap();
    
    // Test undo
    let undone = manager.undo().unwrap();
    assert!(undone.is_some());
    assert_eq!(undone.unwrap().metadata.turn_number, 2);
    
    // Test undo again
    let undone2 = manager.undo().unwrap();
    assert!(undone2.is_some());
    assert_eq!(undone2.unwrap().metadata.turn_number, 1);
    
    // Test redo
    let redone = manager.redo().unwrap();
    assert!(redone.is_some());
    assert_eq!(redone.unwrap().metadata.turn_number, 2);
    
    // Test branch creation
    let branch_state = vec![
        create_test_message("user", "First message"),
        create_test_message("assistant", "Alternative response"),
    ];
    let branch_metadata = create_test_metadata(2, "Plan");
    let branch_id = manager.create_branch(
        "Alternative".to_string(),
        Some("Testing alternative approach".to_string()),
        branch_state,
        branch_metadata,
    ).unwrap();
    
    // Switch to branch
    let switched = manager.switch_to_branch(branch_id).unwrap();
    assert!(switched.is_some());
    assert_eq!(switched.unwrap().metadata.mode, "Plan");
    
    // Test checkpoint creation
    let checkpoint_id = manager.create_checkpoint("test_checkpoint".to_string()).unwrap();
    
    // Make more changes
    let state4 = vec![
        create_test_message("user", "After checkpoint"),
    ];
    let metadata4 = create_test_metadata(4, "Review");
    manager.save_state(state4, metadata4).unwrap();
    
    // Restore checkpoint
    let restored = manager.restore_checkpoint(checkpoint_id).unwrap();
    assert!(restored.is_some());
    
    // Check memory info
    let mem_info = manager.memory_info();
    assert!(mem_info.total_usage_bytes > 0);
    assert_eq!(mem_info.undo_stack_size > 0, true);
}

#[test]
fn test_full_navigation_workflow() {
    // This test simulates a complete workflow using all three components
    
    // 1. Create conversation with undo/redo manager
    let mut undo_manager = UndoRedoManager::new();
    
    let messages = vec![
        create_test_message("user", "What is Rust?"),
        create_test_message("assistant", "Rust is a systems programming language."),
        create_test_message("user", "What are its main features?"),
        create_test_message("assistant", "Memory safety, concurrency, and performance."),
    ];
    
    // Save states progressively
    for i in 1..=messages.len() {
        let state = messages[..i].to_vec();
        let metadata = create_test_metadata(i, "Build");
        undo_manager.save_state(state, metadata).unwrap();
    }
    
    // 2. Use message jump to navigate
    let mut jump = MessageJump::new();
    jump.show(messages.clone());
    
    // Search for specific content
    jump.set_search_query("memory".to_string());
    let selected = jump.selected_message();
    assert!(selected.is_some());
    
    // 3. Use history browser to visualize
    let mut browser = HistoryBrowser::new();
    browser.show(messages.clone());
    
    // Navigate through history
    browser.move_down();
    browser.move_down();
    
    // Create a branch at current position
    let branch_message = create_test_message("assistant", "Alternative explanation about memory safety.");
    browser.create_branch_from_selected("Alternative".to_string(), branch_message);
    
    // 4. Undo to a previous state
    undo_manager.undo().unwrap();
    undo_manager.undo().unwrap();
    
    // Get current state after undo
    let current = undo_manager.current_state();
    assert!(current.is_some());
    assert_eq!(current.unwrap().metadata.turn_number, 2);
    
    // 5. Create a checkpoint here
    let checkpoint_id = undo_manager.create_checkpoint("important_state".to_string()).unwrap();
    
    // 6. Make more changes
    let new_messages = vec![
        create_test_message("user", "Tell me more"),
        create_test_message("assistant", "Here's more information..."),
    ];
    
    for msg in new_messages {
        let mut state = current.unwrap().items.clone();
        state.push(msg);
        let metadata = create_test_metadata(state.len(), "Build");
        undo_manager.save_state(state, metadata).unwrap();
    }
    
    // 7. Restore checkpoint
    let restored = undo_manager.restore_checkpoint(checkpoint_id).unwrap();
    assert!(restored.is_some());
    assert!(restored.unwrap().metadata.tags.contains(&"checkpoint:important_state".to_string()));
    
    // Clean up
    jump.hide();
    browser.hide();
    undo_manager.clear();
}

#[test]
fn test_memory_management() {
    // Test memory limits and cleanup
    let mut manager = UndoRedoManager::with_memory_limit(1); // 1MB limit
    
    // Add many large states to test memory enforcement
    for i in 0..100 {
        let large_content = "x".repeat(20000); // ~20KB per message
        let messages = vec![create_test_message("user", &large_content)];
        let metadata = create_test_metadata(i, "Build");
        manager.save_state(messages, metadata).unwrap();
    }
    
    // Check that memory is within limits
    let mem_info = manager.memory_info();
    assert!(mem_info.total_usage_bytes <= mem_info.max_usage_bytes);
    assert!(mem_info.usage_percentage <= 100.0);
    
    // Verify old states were removed
    let undo_history = manager.undo_history();
    assert!(undo_history.len() < 100);
}

#[test]
fn test_role_filtering() {
    let mut jump = MessageJump::new();
    
    let messages = vec![
        create_test_message("user", "User message 1"),
        create_test_message("assistant", "Assistant message 1"),
        create_test_message("system", "System message"),
        create_test_message("user", "User message 2"),
        create_test_message("assistant", "Assistant message 2"),
    ];
    
    jump.show(messages);
    
    // Test each filter
    let filters = [
        RoleFilter::All,
        RoleFilter::User,
        RoleFilter::Assistant,
        RoleFilter::System,
        RoleFilter::Function,
        RoleFilter::Other,
    ];
    
    for filter in filters {
        while jump.role_filter() != filter {
            jump.cycle_role_filter();
        }
        
        // Verify filter is applied
        if let Some(selected) = jump.selected_message() {
            match filter {
                RoleFilter::All => {}, // All messages should be visible
                RoleFilter::User => assert_eq!(selected.role, "user"),
                RoleFilter::Assistant => assert_eq!(selected.role, "assistant"),
                RoleFilter::System => assert_eq!(selected.role, "system"),
                _ => {}, // Other filters might have no matches
            }
        }
    }
}