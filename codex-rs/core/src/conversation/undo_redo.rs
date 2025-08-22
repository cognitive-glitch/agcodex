//! Undo/Redo system for conversation management
//!
//! This module provides functionality to track conversation states and enable
//! undo/redo operations with branch preservation and memory-efficient snapshots.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::SystemTime;

use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::error::CodexErr;
use crate::error::Result as CodexResult;
use crate::models::ContentItem;
use crate::models::ResponseItem;

/// Maximum number of undo states to keep in memory
const MAX_UNDO_STATES: usize = 50;

/// Maximum size in bytes for a single snapshot before compression
const MAX_SNAPSHOT_SIZE: usize = 10 * 1024 * 1024; // 10MB

/// Represents a complete snapshot of conversation state at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSnapshot {
    /// Unique identifier for this snapshot
    pub id: Uuid,
    /// Timestamp when this snapshot was created
    pub timestamp: SystemTime,
    /// The conversation items at this point
    pub items: Vec<ResponseItem>,
    /// Metadata about the conversation state
    pub metadata: SnapshotMetadata,
    /// Branch information if this is a branch point
    pub branch_info: Option<BranchInfo>,
    /// Size in bytes (estimated)
    pub size_bytes: usize,
    /// Whether this snapshot is compressed
    pub compressed: bool,
}

/// Metadata associated with a conversation snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Turn number in the conversation
    pub turn_number: usize,
    /// Total token count up to this point
    pub total_tokens: usize,
    /// Active model at this point
    pub model: String,
    /// Active mode (Plan/Build/Review)
    pub mode: String,
    /// User who created this turn
    pub user: Option<String>,
    /// Custom tags for this snapshot
    pub tags: Vec<String>,
}

/// Information about a branch in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    /// Name of the branch
    pub name: String,
    /// Parent snapshot ID this branches from
    pub parent_id: Uuid,
    /// Description of why this branch was created
    pub description: Option<String>,
    /// Whether this is the active branch
    pub is_active: bool,
}

/// Manages undo/redo operations for a conversation
pub struct UndoRedoManager {
    /// Stack of undo states (newest last)
    undo_stack: VecDeque<Arc<ConversationSnapshot>>,
    /// Stack of redo states (newest last)
    redo_stack: VecDeque<Arc<ConversationSnapshot>>,
    /// Current conversation state
    current_state: Option<Arc<ConversationSnapshot>>,
    /// All branches indexed by their parent snapshot ID
    branches: std::collections::HashMap<Uuid, Vec<Arc<ConversationSnapshot>>>,
    /// Memory usage tracking
    total_memory_usage: usize,
    /// Maximum memory usage allowed (in bytes)
    max_memory_usage: usize,
}

impl UndoRedoManager {
    /// Create a new undo/redo manager
    pub fn new() -> Self {
        Self {
            undo_stack: VecDeque::with_capacity(MAX_UNDO_STATES),
            redo_stack: VecDeque::new(),
            current_state: None,
            branches: std::collections::HashMap::new(),
            total_memory_usage: 0,
            max_memory_usage: 100 * 1024 * 1024, // 100MB default
        }
    }

    /// Create a new undo/redo manager with custom memory limit
    pub fn with_memory_limit(max_memory_mb: usize) -> Self {
        Self {
            undo_stack: VecDeque::with_capacity(MAX_UNDO_STATES),
            redo_stack: VecDeque::new(),
            current_state: None,
            branches: std::collections::HashMap::new(),
            total_memory_usage: 0,
            max_memory_usage: max_memory_mb * 1024 * 1024,
        }
    }

    /// Save the current conversation state
    pub fn save_state(
        &mut self,
        items: Vec<ResponseItem>,
        metadata: SnapshotMetadata,
    ) -> CodexResult<Uuid> {
        // Clear redo stack when new state is saved
        self.redo_stack.clear();

        // Move current state to undo stack if it exists
        if let Some(current) = self.current_state.take() {
            self.push_to_undo_stack(current);
        }

        // Create new snapshot
        let snapshot = self.create_snapshot(items, metadata)?;
        let snapshot_id = snapshot.id;
        let snapshot_arc = Arc::new(snapshot);

        // Update memory tracking
        self.total_memory_usage += snapshot_arc.size_bytes;
        self.enforce_memory_limit();

        // Set as current state
        self.current_state = Some(snapshot_arc);

        Ok(snapshot_id)
    }

    /// Undo the last operation
    pub fn undo(&mut self) -> CodexResult<Option<ConversationSnapshot>> {
        if self.undo_stack.is_empty() {
            return Ok(None);
        }

        // Move current state to redo stack
        if let Some(current) = self.current_state.take() {
            self.push_to_redo_stack(current);
        }

        // Pop from undo stack and set as current
        if let Some(previous) = self.undo_stack.pop_back() {
            let snapshot = (*previous).clone();
            self.current_state = Some(previous);
            Ok(Some(snapshot))
        } else {
            Ok(None)
        }
    }

    /// Redo the last undone operation
    pub fn redo(&mut self) -> CodexResult<Option<ConversationSnapshot>> {
        if self.redo_stack.is_empty() {
            return Ok(None);
        }

        // Move current state to undo stack
        if let Some(current) = self.current_state.take() {
            self.push_to_undo_stack(current);
        }

        // Pop from redo stack and set as current
        if let Some(next) = self.redo_stack.pop_back() {
            let snapshot = (*next).clone();
            self.current_state = Some(next);
            Ok(Some(snapshot))
        } else {
            Ok(None)
        }
    }

    /// Create a branch from the current state
    pub fn create_branch(
        &mut self,
        branch_name: String,
        description: Option<String>,
        items: Vec<ResponseItem>,
        metadata: SnapshotMetadata,
    ) -> CodexResult<Uuid> {
        let parent_id = self
            .current_state
            .as_ref()
            .map(|s| s.id)
            .ok_or(CodexErr::NoBranchPointAvailable)?;

        let branch_info = BranchInfo {
            name: branch_name,
            parent_id,
            description,
            is_active: false,
        };

        let mut snapshot = self.create_snapshot(items, metadata)?;
        snapshot.branch_info = Some(branch_info);

        let snapshot_id = snapshot.id;
        let snapshot_arc = Arc::new(snapshot);

        // Add to branches map
        self.branches
            .entry(parent_id)
            .or_default()
            .push(snapshot_arc.clone());

        // Update memory tracking
        self.total_memory_usage += snapshot_arc.size_bytes;
        self.enforce_memory_limit();

        Ok(snapshot_id)
    }

    /// Switch to a specific branch
    pub fn switch_to_branch(
        &mut self,
        branch_id: Uuid,
    ) -> CodexResult<Option<ConversationSnapshot>> {
        // Find the branch snapshot
        let branch_snapshot = self
            .branches
            .values()
            .flatten()
            .find(|s| s.id == branch_id)
            .cloned();

        if let Some(snapshot) = branch_snapshot {
            // Save current state to undo stack
            if let Some(current) = self.current_state.take() {
                self.push_to_undo_stack(current);
            }

            // Clear redo stack when switching branches
            self.redo_stack.clear();

            // Set branch as current
            let result = (*snapshot).clone();
            self.current_state = Some(snapshot);

            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    /// Get all available branches
    pub fn get_branches(&self) -> Vec<(Uuid, BranchInfo)> {
        self.branches
            .values()
            .flatten()
            .filter_map(|s| s.branch_info.as_ref().map(|b| (s.id, b.clone())))
            .collect()
    }

    /// Get the current state
    pub fn current_state(&self) -> Option<&ConversationSnapshot> {
        self.current_state.as_deref()
    }

    /// Get undo history (for visualization)
    pub fn undo_history(&self) -> Vec<&ConversationSnapshot> {
        self.undo_stack.iter().map(|s| s.as_ref()).collect()
    }

    /// Get redo history (for visualization)
    pub fn redo_history(&self) -> Vec<&ConversationSnapshot> {
        self.redo_stack.iter().map(|s| s.as_ref()).collect()
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.current_state = None;
        self.branches.clear();
        self.total_memory_usage = 0;
    }

    /// Create a checkpoint (special snapshot with tag)
    pub fn create_checkpoint(&mut self, name: String) -> CodexResult<Uuid> {
        if let Some(current) = &self.current_state {
            let mut checkpoint = (**current).clone();
            checkpoint.id = Uuid::new_v4();
            checkpoint.timestamp = SystemTime::now();
            checkpoint
                .metadata
                .tags
                .push(format!("checkpoint:{}", name));

            let checkpoint_id = checkpoint.id;
            let checkpoint_arc = Arc::new(checkpoint);

            // Store in branches as a special branch
            self.branches
                .entry(current.id)
                .or_default()
                .push(checkpoint_arc);

            Ok(checkpoint_id)
        } else {
            Err(CodexErr::NoCurrentStateForCheckpoint)
        }
    }

    /// Restore from a checkpoint
    pub fn restore_checkpoint(
        &mut self,
        checkpoint_id: Uuid,
    ) -> CodexResult<Option<ConversationSnapshot>> {
        self.switch_to_branch(checkpoint_id)
    }

    /// Get memory usage information
    pub fn memory_info(&self) -> MemoryInfo {
        MemoryInfo {
            total_usage_bytes: self.total_memory_usage,
            max_usage_bytes: self.max_memory_usage,
            undo_stack_size: self.undo_stack.len(),
            redo_stack_size: self.redo_stack.len(),
            branch_count: self.branches.values().map(|v| v.len()).sum(),
            usage_percentage: (self.total_memory_usage as f64 / self.max_memory_usage as f64)
                * 100.0,
        }
    }

    // Private helper methods

    fn create_snapshot(
        &self,
        items: Vec<ResponseItem>,
        metadata: SnapshotMetadata,
    ) -> CodexResult<ConversationSnapshot> {
        let size_bytes = Self::estimate_size(&items);
        let compressed = size_bytes > MAX_SNAPSHOT_SIZE;

        // In a real implementation, we would compress large snapshots here
        let snapshot = ConversationSnapshot {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            items,
            metadata,
            branch_info: None,
            size_bytes,
            compressed,
        };

        Ok(snapshot)
    }

    fn push_to_undo_stack(&mut self, snapshot: Arc<ConversationSnapshot>) {
        // Enforce maximum undo states
        while self.undo_stack.len() >= MAX_UNDO_STATES {
            if let Some(removed) = self.undo_stack.pop_front() {
                self.total_memory_usage =
                    self.total_memory_usage.saturating_sub(removed.size_bytes);
            }
        }
        self.undo_stack.push_back(snapshot);
    }

    fn push_to_redo_stack(&mut self, snapshot: Arc<ConversationSnapshot>) {
        self.redo_stack.push_back(snapshot);
    }

    fn enforce_memory_limit(&mut self) {
        // Remove oldest snapshots if memory limit is exceeded
        while self.total_memory_usage > self.max_memory_usage && !self.undo_stack.is_empty() {
            if let Some(removed) = self.undo_stack.pop_front() {
                self.total_memory_usage =
                    self.total_memory_usage.saturating_sub(removed.size_bytes);
            }
        }
    }

    fn estimate_size(items: &[ResponseItem]) -> usize {
        // Simple size estimation based on content
        items
            .iter()
            .map(|item| match item {
                ResponseItem::Message { content, .. } => {
                    content
                        .iter()
                        .map(|c| match c {
                            ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                                text.len()
                            }
                            ContentItem::InputImage { .. } => 1024, // Estimate for image metadata
                        })
                        .sum::<usize>()
                }
                ResponseItem::Reasoning {
                    summary, content, ..
                } => {
                    // Estimate size based on summary and optional content
                    let summary_size: usize = summary.iter().map(|_| 100).sum(); // Estimate 100 bytes per summary item
                    let content_size = content.as_ref().map(|c| c.len() * 50).unwrap_or(0); // Estimate 50 bytes per content item
                    summary_size + content_size
                }
                ResponseItem::FunctionCall { arguments, .. } => arguments.len(),
                ResponseItem::FunctionCallOutput { output, .. } => {
                    // Estimate based on the payload content
                    output.content.len() + 100 // Content plus overhead
                }
                _ => 256, // Default estimate for other types
            })
            .sum()
    }
}

impl Default for UndoRedoManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory usage information
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    pub total_usage_bytes: usize,
    pub max_usage_bytes: usize,
    pub undo_stack_size: usize,
    pub redo_stack_size: usize,
    pub branch_count: usize,
    pub usage_percentage: f64,
}

/// Diff between two conversation states (for efficient storage)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationDiff {
    /// Items added in this diff
    pub added: Vec<ResponseItem>,
    /// Indices of items removed
    pub removed: Vec<usize>,
    /// Items that were modified (index, new_item)
    pub modified: Vec<(usize, ResponseItem)>,
}

impl ConversationDiff {
    /// Create a diff between two conversation states
    pub fn create(old: &[ResponseItem], new: &[ResponseItem]) -> Self {
        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut modified = Vec::new();

        // Simple diff algorithm - can be optimized with proper diffing
        let min_len = old.len().min(new.len());

        // Check for modifications in common range
        for i in 0..min_len {
            if !Self::items_equal(&old[i], &new[i]) {
                modified.push((i, new[i].clone()));
            }
        }

        // Check for additions
        if new.len() > old.len() {
            added.extend(new[old.len()..].iter().cloned());
        }

        // Check for removals
        if old.len() > new.len() {
            for i in new.len()..old.len() {
                removed.push(i);
            }
        }

        Self {
            added,
            removed,
            modified,
        }
    }

    /// Apply this diff to a conversation state
    pub fn apply(&self, items: &mut Vec<ResponseItem>) {
        // Apply modifications
        for (index, new_item) in &self.modified {
            if *index < items.len() {
                items[*index] = new_item.clone();
            }
        }

        // Remove items (in reverse order to maintain indices)
        for &index in self.removed.iter().rev() {
            if index < items.len() {
                items.remove(index);
            }
        }

        // Add new items
        items.extend(self.added.iter().cloned());
    }

    fn items_equal(a: &ResponseItem, b: &ResponseItem) -> bool {
        // Simple equality check - could be optimized
        match (a, b) {
            (
                ResponseItem::Message {
                    role: r1,
                    content: c1,
                    ..
                },
                ResponseItem::Message {
                    role: r2,
                    content: c2,
                    ..
                },
            ) => r1 == r2 && Self::content_equal(c1, c2),
            _ => false, // Different types or other items
        }
    }

    fn content_equal(a: &[ContentItem], b: &[ContentItem]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        a.iter()
            .zip(b.iter())
            .all(|(a_item, b_item)| match (a_item, b_item) {
                (
                    ContentItem::InputText { text: t1 } | ContentItem::OutputText { text: t1 },
                    ContentItem::InputText { text: t2 } | ContentItem::OutputText { text: t2 },
                ) => t1 == t2,
                _ => false,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_save_and_undo() {
        let mut manager = UndoRedoManager::new();

        // Save first state
        let items1 = vec![create_test_message("user", "Hello")];
        let id1 = manager
            .save_state(items1.clone(), create_test_metadata(1))
            .unwrap();
        assert!(manager.current_state().is_some());

        // Save second state
        let items2 = vec![
            create_test_message("user", "Hello"),
            create_test_message("assistant", "Hi there"),
        ];
        let _id2 = manager
            .save_state(items2.clone(), create_test_metadata(2))
            .unwrap();

        // Undo should return to first state
        let undone = manager.undo().unwrap();
        assert!(undone.is_some());
        assert_eq!(undone.unwrap().items.len(), 1);
    }

    #[test]
    fn test_undo_redo() {
        let mut manager = UndoRedoManager::new();

        // Create three states
        let items1 = vec![create_test_message("user", "1")];
        manager.save_state(items1, create_test_metadata(1)).unwrap();

        let items2 = vec![create_test_message("user", "2")];
        manager.save_state(items2, create_test_metadata(2)).unwrap();

        let items3 = vec![create_test_message("user", "3")];
        manager.save_state(items3, create_test_metadata(3)).unwrap();

        // Undo twice
        manager.undo().unwrap();
        let state = manager.undo().unwrap().unwrap();
        assert_eq!(state.metadata.turn_number, 1);

        // Redo once
        let state = manager.redo().unwrap().unwrap();
        assert_eq!(state.metadata.turn_number, 2);
    }

    #[test]
    fn test_branching() {
        let mut manager = UndoRedoManager::new();

        // Create initial state
        let items1 = vec![create_test_message("user", "main")];
        manager.save_state(items1, create_test_metadata(1)).unwrap();

        // Create a branch
        let branch_items = vec![create_test_message("user", "branch")];
        let branch_id = manager
            .create_branch(
                "Alternative".to_string(),
                Some("Testing branch".to_string()),
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
    fn test_memory_limit() {
        let mut manager = UndoRedoManager::with_memory_limit(1); // 1MB limit

        // Add many states to exceed memory limit
        for i in 0..100 {
            let items = vec![create_test_message("user", &"x".repeat(20000))]; // ~20KB each
            manager.save_state(items, create_test_metadata(i)).unwrap();
        }

        // Check that old states were removed to stay under limit
        let info = manager.memory_info();
        assert!(info.total_usage_bytes <= info.max_usage_bytes);
        assert!(manager.undo_stack.len() < 100);
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
    fn test_conversation_diff() {
        let old = vec![
            create_test_message("user", "Hello"),
            create_test_message("assistant", "Hi"),
        ];

        let new = vec![
            create_test_message("user", "Hello"),
            create_test_message("assistant", "Hi there!"),
            create_test_message("user", "How are you?"),
        ];

        let diff = ConversationDiff::create(&old, &new);
        assert_eq!(diff.modified.len(), 1);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.removed.len(), 0);

        // Apply diff
        let mut result = old.clone();
        diff.apply(&mut result);
        assert_eq!(result.len(), new.len());
    }
}
