//! Session persistence for AGCodex with Zstd compression
//!
//! This crate provides efficient session storage and retrieval for AGCodex conversations,
//! with support for auto-saving, checkpointing, and fast loading through memory-mapped files.

pub mod compression;
pub mod error;
pub mod migration;
pub mod session_manager;
pub mod storage;
pub mod types;

#[cfg(test)]
mod tests;

pub use compression::CompressionLevel;
pub use compression::Compressor;
pub use error::PersistenceError;
pub use error::Result;
pub use migration::MigrationManager;
pub use session_manager::SessionManager;
pub use session_manager::SessionManagerConfig;
pub use storage::SessionStorage;
pub use storage::StorageBackend;
pub use types::Checkpoint;
pub use types::ConversationSnapshot;
pub use types::MessageSnapshot;
pub use types::SessionIndex;
pub use types::SessionMetadata;
pub use types::SessionState;

/// Version header for AGCodex session files
pub const AGCX_MAGIC: &[u8] = b"AGCX";

/// Current session format version
pub const FORMAT_VERSION: u16 = 1;
