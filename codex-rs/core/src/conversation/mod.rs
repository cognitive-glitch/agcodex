//! Conversation management utilities
//!
//! This module provides functionality for managing conversation state,
//! including undo/redo operations, branching, and checkpointing.

pub mod undo_redo;

#[cfg(test)]
mod test_integration;

pub use undo_redo::BranchInfo;
pub use undo_redo::ConversationDiff;
pub use undo_redo::ConversationSnapshot;
pub use undo_redo::MemoryInfo;
pub use undo_redo::SnapshotMetadata;
pub use undo_redo::UndoRedoManager;
