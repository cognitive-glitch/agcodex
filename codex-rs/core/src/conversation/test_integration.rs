//! Unit tests for conversation navigation integration

#[cfg(test)]
mod tests {

    use super::super::undo_redo::SnapshotMetadata;
    use super::super::undo_redo::UndoRedoManager;
    use crate::models::ContentItem;
    use crate::models::ResponseItem;

    fn create_test_message(role: &str, content: &str) -> ResponseItem {
        ResponseItem::Message {
            id: None,
            role: role.to_string(),
            content: vec![ContentItem::OutputText {
                text: content.to_string(),
            }],
        }
    }

    fn create_test_metadata(turn: usize) -> SnapshotMetadata {
        SnapshotMetadata {
            turn_number: turn,
            total_tokens: turn * 100,
            model: "test-model".to_string(),
            mode: "Build".to_string(),
            user: None,
            tags: Vec::new(),
        }
    }

    #[test]
    fn test_undo_redo_basic() {
        let mut manager = UndoRedoManager::new();

        // Save first state
        let items1 = vec![create_test_message("user", "Hello")];
        manager
            .save_state(items1.clone(), create_test_metadata(1))
            .unwrap();

        // Save second state
        let items2 = vec![
            create_test_message("user", "Hello"),
            create_test_message("assistant", "Hi there"),
        ];
        manager
            .save_state(items2.clone(), create_test_metadata(2))
            .unwrap();

        // Test undo
        let undone = manager.undo().unwrap();
        assert!(undone.is_some());
        assert_eq!(undone.unwrap().items.len(), 1);

        // Test redo
        let redone = manager.redo().unwrap();
        assert!(redone.is_some());
        assert_eq!(redone.unwrap().items.len(), 2);
    }

    #[test]
    fn test_branching() {
        let mut manager = UndoRedoManager::new();

        // Create initial state
        let items = vec![create_test_message("user", "main")];
        manager.save_state(items, create_test_metadata(1)).unwrap();

        // Create a branch
        let branch_items = vec![create_test_message("user", "branch")];
        let branch_id = manager
            .create_branch(
                "Alternative".to_string(),
                Some("Test branch".to_string()),
                branch_items,
                create_test_metadata(2),
            )
            .unwrap();

        // Verify branch exists
        let branches = manager.get_branches();
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].1.name, "Alternative");

        // Switch to branch
        let switched = manager.switch_to_branch(branch_id).unwrap();
        assert!(switched.is_some());
    }

    #[test]
    fn test_checkpoint() {
        let mut manager = UndoRedoManager::new();

        // Create initial state
        let items = vec![create_test_message("user", "checkpoint test")];
        manager.save_state(items, create_test_metadata(1)).unwrap();

        // Create checkpoint
        let checkpoint_id = manager
            .create_checkpoint("test_checkpoint".to_string())
            .unwrap();

        // Make more changes
        let items2 = vec![create_test_message("user", "after checkpoint")];
        manager.save_state(items2, create_test_metadata(2)).unwrap();

        // Restore checkpoint
        let restored = manager.restore_checkpoint(checkpoint_id).unwrap();
        assert!(restored.is_some());
        assert!(
            restored
                .unwrap()
                .metadata
                .tags
                .contains(&"checkpoint:test_checkpoint".to_string())
        );
    }

    #[test]
    fn test_memory_management() {
        let mut manager = UndoRedoManager::with_memory_limit(1); // 1MB limit

        // Add many states to test memory enforcement
        for i in 0..100 {
            let large_content = "x".repeat(20000); // ~20KB each
            let messages = vec![create_test_message("user", &large_content)];
            manager
                .save_state(messages, create_test_metadata(i))
                .unwrap();
        }

        // Check that memory is within limits
        let info = manager.memory_info();
        assert!(info.total_usage_bytes <= info.max_usage_bytes);
    }
}
