//! Error types for persistence operations

use std::io;
use thiserror::Error;
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, PersistenceError>;

#[derive(Error, Debug)]
pub enum PersistenceError {
    /// I/O errors during file operations
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Bincode encoding errors
    #[error("Bincode encoding error: {0}")]
    BincodeEncode(#[from] bincode::error::EncodeError),

    /// Bincode decoding errors
    #[error("Bincode decoding error: {0}")]
    BincodeDecode(#[from] bincode::error::DecodeError),

    /// MessagePack serialization errors
    #[error("MessagePack serialization error: {0}")]
    MessagePack(#[from] rmp_serde::encode::Error),

    /// MessagePack deserialization errors
    #[error("MessagePack deserialization error: {0}")]
    MessagePackDecode(#[from] rmp_serde::decode::Error),

    /// Compression errors
    #[error("Compression error: {0}")]
    Compression(String),

    /// Session not found
    #[error("Session not found: {0}")]
    SessionNotFound(Uuid),

    /// Invalid magic bytes in file header
    #[error("Invalid file format: expected AGCX magic bytes")]
    InvalidMagic,

    /// Unsupported format version
    #[error("Unsupported format version: {0} (expected {1})")]
    UnsupportedVersion(u16, u16),

    /// Corrupt session data
    #[error("Corrupt session data: {0}")]
    CorruptData(String),

    /// Invalid format
    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    /// Incompatible version
    #[error("Incompatible version: expected {expected}, got {actual}")]
    IncompatibleVersion { expected: u16, actual: u16 },

    /// Storage path does not exist
    #[error("Storage path does not exist: {0}")]
    PathNotFound(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Session already exists
    #[error("Session already exists: {0}")]
    SessionAlreadyExists(Uuid),

    /// Invalid checkpoint
    #[error("Invalid checkpoint: {0}")]
    InvalidCheckpoint(String),

    /// Migration required
    #[error("Migration required from version {0} to {1}")]
    MigrationRequired(u16, u16),

    /// Migration failed
    #[error("Migration failed: {0}")]
    MigrationFailed(String),

    /// Lock acquisition failed
    #[error("Failed to acquire lock: {0}")]
    LockFailed(String),

    /// Memory map error
    #[error("Memory mapping failed: {0}")]
    MemoryMapFailed(String),

    /// Index corruption
    #[error("Session index corrupted: {0}")]
    IndexCorrupted(String),

    /// Auto-save failed
    #[error("Auto-save failed: {0}")]
    AutoSaveFailed(String),

    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
