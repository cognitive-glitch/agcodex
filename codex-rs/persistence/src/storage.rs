//! Storage backend for session persistence

use crate::AGCX_MAGIC;
use crate::FORMAT_VERSION;
use crate::compression::CompressionLevel;
use crate::compression::Compressor;
use crate::error::PersistenceError;
use crate::error::Result;
use crate::types::ConversationSnapshot;
use crate::types::MessageSnapshot;
use crate::types::SessionIndex;
use crate::types::SessionMetadata;
use crate::types::SessionState;
use memmap2::MmapOptions;
use std::fs::File;
use std::fs::{self};
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use tokio::fs as async_fs;
use uuid::Uuid;

/// Storage backend trait for different storage implementations
pub trait StorageBackend: Send + Sync {
    /// Save session to storage
    fn save_session(
        &self,
        id: Uuid,
        metadata: &SessionMetadata,
        conversation: &ConversationSnapshot,
        state: &SessionState,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Load session from storage
    fn load_session(
        &self,
        id: Uuid,
    ) -> impl std::future::Future<
        Output = Result<(SessionMetadata, ConversationSnapshot, SessionState)>,
    > + Send;

    /// Delete session from storage
    fn delete_session(&self, id: Uuid) -> impl std::future::Future<Output = Result<()>> + Send;

    /// List all sessions
    fn list_sessions(
        &self,
    ) -> impl std::future::Future<Output = Result<Vec<SessionMetadata>>> + Send;

    /// Load session index
    fn load_index(&self) -> impl std::future::Future<Output = Result<SessionIndex>> + Send;

    /// Save session index
    fn save_index(
        &self,
        index: &SessionIndex,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}

/// File-based storage implementation
pub struct SessionStorage {
    base_path: PathBuf,
    compressor: Compressor,
    use_mmap: bool,
}

impl SessionStorage {
    /// Create a new session storage
    pub fn new(base_path: PathBuf, compression_level: CompressionLevel) -> Result<Self> {
        // Ensure base path exists
        fs::create_dir_all(&base_path)?;

        Ok(Self {
            base_path,
            compressor: Compressor::new(compression_level),
            use_mmap: true,
        })
    }

    /// Get the path for a session file
    fn session_path(&self, id: Uuid) -> PathBuf {
        self.base_path.join(format!("{}.agcx", id))
    }

    /// Get the path for the session index
    fn index_path(&self) -> PathBuf {
        self.base_path.join("sessions.idx")
    }

    /// Get the checkpoint directory
    fn _checkpoint_dir(&self) -> PathBuf {
        self.base_path.join("checkpoints")
    }

    /// Write session file with header
    fn write_session_file(
        &self,
        path: &Path,
        metadata: &SessionMetadata,
        conversation: &ConversationSnapshot,
        state: &SessionState,
    ) -> Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // Write magic bytes and version
        writer.write_all(AGCX_MAGIC)?;
        writer.write_all(&FORMAT_VERSION.to_le_bytes())?;

        // Serialize metadata with bincode
        let metadata_bytes = bincode::serde::encode_to_vec(metadata, bincode::config::standard())?;
        let metadata_len = metadata_bytes.len() as u32;
        writer.write_all(&metadata_len.to_le_bytes())?;
        writer.write_all(&metadata_bytes)?;

        // Serialize messages with MessagePack and compress
        let messages_bytes = rmp_serde::to_vec(&conversation.messages)?;
        let compressed_messages = self.compressor.compress(&messages_bytes)?;
        let messages_len = compressed_messages.len() as u32;
        writer.write_all(&messages_len.to_le_bytes())?;
        writer.write_all(&compressed_messages)?;

        // Serialize context with MessagePack and compress
        let context_bytes = rmp_serde::to_vec(&conversation.context)?;
        let compressed_context = self.compressor.compress(&context_bytes)?;
        let context_len = compressed_context.len() as u32;
        writer.write_all(&context_len.to_le_bytes())?;
        writer.write_all(&compressed_context)?;

        // Serialize state with bincode
        let state_bytes = bincode::serde::encode_to_vec(state, bincode::config::standard())?;
        writer.write_all(&(state_bytes.len() as u32).to_le_bytes())?;
        writer.write_all(&state_bytes)?;

        writer.flush()?;
        Ok(())
    }

    /// Read session file with header validation
    fn read_session_file(
        &self,
        path: &Path,
    ) -> Result<(SessionMetadata, ConversationSnapshot, SessionState)> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Validate magic bytes
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if magic != AGCX_MAGIC {
            return Err(PersistenceError::InvalidMagic);
        }

        // Validate version
        let mut version_bytes = [0u8; 2];
        reader.read_exact(&mut version_bytes)?;
        let version = u16::from_le_bytes(version_bytes);
        if version != FORMAT_VERSION {
            return Err(PersistenceError::UnsupportedVersion(
                version,
                FORMAT_VERSION,
            ));
        }

        // Read metadata
        let mut metadata_len_bytes = [0u8; 4];
        reader.read_exact(&mut metadata_len_bytes)?;
        let metadata_len = u32::from_le_bytes(metadata_len_bytes) as usize;
        let mut metadata_bytes = vec![0u8; metadata_len];
        reader.read_exact(&mut metadata_bytes)?;
        let (metadata, _): (SessionMetadata, _) =
            bincode::serde::decode_from_slice(&metadata_bytes, bincode::config::standard())?;

        // Read and decompress messages
        let mut messages_len_bytes = [0u8; 4];
        reader.read_exact(&mut messages_len_bytes)?;
        let messages_len = u32::from_le_bytes(messages_len_bytes) as usize;
        let mut compressed_messages = vec![0u8; messages_len];
        reader.read_exact(&mut compressed_messages)?;
        let messages_bytes = self.compressor.decompress(&compressed_messages)?;
        let messages: Vec<MessageSnapshot> = rmp_serde::from_slice(&messages_bytes)?;

        // Read and decompress context
        let mut context_len_bytes = [0u8; 4];
        reader.read_exact(&mut context_len_bytes)?;
        let context_len = u32::from_le_bytes(context_len_bytes) as usize;
        let mut compressed_context = vec![0u8; context_len];
        reader.read_exact(&mut compressed_context)?;
        let context_bytes = self.compressor.decompress(&compressed_context)?;
        let context = rmp_serde::from_slice(&context_bytes)?;

        // Read state
        let mut state_len_bytes = [0u8; 4];
        reader.read_exact(&mut state_len_bytes)?;
        let state_len = u32::from_le_bytes(state_len_bytes) as usize;
        let mut state_bytes = vec![0u8; state_len];
        reader.read_exact(&mut state_bytes)?;
        let (state, _): (SessionState, _) =
            bincode::serde::decode_from_slice(&state_bytes, bincode::config::standard())?;

        let conversation = ConversationSnapshot {
            id: metadata.id,
            messages,
            context,
            mode_history: vec![(metadata.current_mode, metadata.created_at)],
        };

        Ok((metadata, conversation, state))
    }

    /// Load metadata using memory mapping for fast access
    pub fn load_metadata_mmap(&self, path: &Path) -> Result<SessionMetadata> {
        if !self.use_mmap {
            let (metadata, _, _) = self.read_session_file(path)?;
            return Ok(metadata);
        }

        let file = File::open(path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };

        // Validate magic and version
        if mmap.len() < 6 {
            return Err(PersistenceError::CorruptData("File too small".to_string()));
        }

        if &mmap[0..4] != AGCX_MAGIC {
            return Err(PersistenceError::InvalidMagic);
        }

        let version = u16::from_le_bytes([mmap[4], mmap[5]]);
        if version != FORMAT_VERSION {
            return Err(PersistenceError::UnsupportedVersion(
                version,
                FORMAT_VERSION,
            ));
        }

        // Read metadata length and deserialize
        let metadata_len = u32::from_le_bytes([mmap[6], mmap[7], mmap[8], mmap[9]]) as usize;
        let metadata_end = 10 + metadata_len;

        if mmap.len() < metadata_end {
            return Err(PersistenceError::CorruptData(
                "Incomplete metadata".to_string(),
            ));
        }

        let (metadata, _): (SessionMetadata, _) = bincode::serde::decode_from_slice(
            &mmap[10..metadata_end],
            bincode::config::standard(),
        )?;
        Ok(metadata)
    }
}

impl StorageBackend for SessionStorage {
    async fn save_session(
        &self,
        id: Uuid,
        metadata: &SessionMetadata,
        conversation: &ConversationSnapshot,
        state: &SessionState,
    ) -> Result<()> {
        let path = self.session_path(id);

        // Write to temporary file first
        let temp_path = path.with_extension("tmp");
        self.write_session_file(&temp_path, metadata, conversation, state)?;

        // Atomic rename
        async_fs::rename(&temp_path, &path).await?;

        Ok(())
    }

    async fn load_session(
        &self,
        id: Uuid,
    ) -> Result<(SessionMetadata, ConversationSnapshot, SessionState)> {
        let path = self.session_path(id);

        if !path.exists() {
            return Err(PersistenceError::SessionNotFound(id));
        }

        // Use blocking I/O in a spawn_blocking task
        let path_clone = path.clone();
        let compressor = self.compressor.clone();

        tokio::task::spawn_blocking(move || {
            let storage = SessionStorage {
                base_path: PathBuf::new(),
                compressor,
                use_mmap: false,
            };
            storage.read_session_file(&path_clone)
        })
        .await
        .map_err(|e| PersistenceError::Io(std::io::Error::other(e)))?
    }

    async fn delete_session(&self, id: Uuid) -> Result<()> {
        let path = self.session_path(id);

        if path.exists() {
            async_fs::remove_file(&path).await?;
        }

        Ok(())
    }

    async fn list_sessions(&self) -> Result<Vec<SessionMetadata>> {
        let mut sessions = Vec::new();

        let mut entries = async_fs::read_dir(&self.base_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.extension().and_then(|ext| ext.to_str()) == Some("agcx") {
                match self.load_metadata_mmap(&path) {
                    Ok(metadata) => sessions.push(metadata),
                    Err(e) => {
                        tracing::warn!("Failed to load session metadata from {:?}: {}", path, e);
                    }
                }
            }
        }

        // Sort by last accessed time
        sessions.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));

        Ok(sessions)
    }

    async fn load_index(&self) -> Result<SessionIndex> {
        let path = self.index_path();

        if !path.exists() {
            return Ok(SessionIndex::new());
        }

        let bytes = async_fs::read(&path).await?;
        let (index, _) = bincode::serde::decode_from_slice(&bytes, bincode::config::standard())?;
        Ok(index)
    }

    async fn save_index(&self, index: &SessionIndex) -> Result<()> {
        let path = self.index_path();
        let bytes = bincode::serde::encode_to_vec(index, bincode::config::standard())?;

        // Write to temporary file first
        let temp_path = path.with_extension("tmp");
        async_fs::write(&temp_path, &bytes).await?;

        // Atomic rename
        async_fs::rename(&temp_path, &path).await?;

        Ok(())
    }
}

// Clone implementation for Compressor to use in async contexts
impl Clone for Compressor {
    fn clone(&self) -> Self {
        Self::new(CompressionLevel::Balanced)
    }
}
