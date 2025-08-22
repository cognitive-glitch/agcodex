//! Enhanced session storage backend with MessagePack and Zstd compression
//! Implements efficient storage with auto-save and metadata indexing

use crate::error::PersistenceError;
use crate::error::Result;
use crate::types::ConversationSnapshot;
use crate::types::SessionIndex;
use crate::types::SessionMetadata;
use crate::types::SessionState;
use chrono::DateTime;
use chrono::Utc;
use rmp_serde;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::fs::{self};
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use zstd;

/// Magic bytes for AGCodex session files
const AGCX_MAGIC: &[u8] = b"AGCX";

/// Current format version
const FORMAT_VERSION: u32 = 2;

/// Session file header
#[derive(Debug, Serialize, Deserialize)]
struct SessionHeader {
    magic: [u8; 4],
    version: u32,
    session_id: Uuid,
    created_at: DateTime<Utc>,
    compression_level: i32,
}

/// Complete session data structure
#[derive(Debug, Serialize, Deserialize)]
struct SessionData {
    metadata: SessionMetadata,
    conversation: ConversationSnapshot,
    state: SessionState,
    checksum: Option<u32>,
}

/// Session storage configuration
#[derive(Debug, Clone)]
pub struct SessionStoreConfig {
    /// Base directory for session storage
    pub base_path: PathBuf,
    /// Zstd compression level (1-22, default 3)
    pub compression_level: i32,
    /// Enable metadata memory mapping
    pub enable_mmap: bool,
    /// Maximum sessions to keep in index
    pub max_indexed_sessions: usize,
    /// Auto-save interval in seconds
    pub auto_save_interval: u64,
}

impl Default for SessionStoreConfig {
    fn default() -> Self {
        let base_path = dirs::home_dir()
            .map(|p| p.join(".agcodex/history"))
            .unwrap_or_else(|| PathBuf::from(".agcodex/history"));

        Self {
            base_path,
            compression_level: 3,
            enable_mmap: true,
            max_indexed_sessions: 1000,
            auto_save_interval: 300, // 5 minutes
        }
    }
}

/// Enhanced session storage backend
pub struct SessionStore {
    config: SessionStoreConfig,
    index: Arc<RwLock<SessionIndex>>,
    metadata_cache: Arc<RwLock<HashMap<Uuid, SessionMetadata>>>,
    dirty_sessions: Arc<RwLock<HashMap<Uuid, DateTime<Utc>>>>,
}

impl SessionStore {
    /// Create new session store
    pub async fn new(config: SessionStoreConfig) -> Result<Self> {
        // Ensure directories exist
        fs::create_dir_all(&config.base_path)?;
        fs::create_dir_all(config.base_path.join("sessions"))?;
        fs::create_dir_all(config.base_path.join("checkpoints"))?;
        fs::create_dir_all(config.base_path.join("metadata"))?;

        // Load or create index
        let index = Self::load_or_create_index(&config.base_path).await?;

        Ok(Self {
            config,
            index: Arc::new(RwLock::new(index)),
            metadata_cache: Arc::new(RwLock::new(HashMap::new())),
            dirty_sessions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Load or create session index
    async fn load_or_create_index(base_path: &Path) -> Result<SessionIndex> {
        let index_path = base_path.join("sessions.idx");

        if index_path.exists() {
            let file = File::open(&index_path)?;
            let reader = BufReader::new(file);

            // Try to deserialize with MessagePack
            match rmp_serde::from_read(reader) {
                Ok(index) => Ok(index),
                Err(_) => {
                    // If corrupted, create new index
                    eprintln!("Warning: Session index corrupted, rebuilding...");
                    Self::rebuild_index(base_path).await
                }
            }
        } else {
            Ok(SessionIndex::new())
        }
    }

    /// Rebuild index from session files
    async fn rebuild_index(base_path: &Path) -> Result<SessionIndex> {
        let mut index = SessionIndex::new();
        let sessions_dir = base_path.join("sessions");

        if sessions_dir.exists() {
            for entry in fs::read_dir(sessions_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("agcx") {
                    // Try to load metadata from file
                    if let Ok(metadata) = Self::load_metadata_from_file(&path).await {
                        index.add_session(metadata);
                    }
                }
            }
        }

        Ok(index)
    }

    /// Load metadata from session file
    async fn load_metadata_from_file(path: &Path) -> Result<SessionMetadata> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Read and validate header
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if magic != AGCX_MAGIC {
            return Err(PersistenceError::InvalidFormat(
                "Invalid magic bytes".to_string(),
            ));
        }

        let mut version_bytes = [0u8; 4];
        reader.read_exact(&mut version_bytes)?;
        let version = u32::from_le_bytes(version_bytes);

        if version != FORMAT_VERSION {
            return Err(PersistenceError::IncompatibleVersion {
                expected: FORMAT_VERSION as u16,
                actual: version as u16,
            });
        }

        // Read compressed data length
        let mut len_bytes = [0u8; 8];
        reader.read_exact(&mut len_bytes)?;
        let compressed_len = u64::from_le_bytes(len_bytes) as usize;

        // Read compressed data
        let mut compressed_data = vec![0u8; compressed_len];
        reader.read_exact(&mut compressed_data)?;

        // Decompress
        let decompressed = zstd::decode_all(&compressed_data[..])
            .map_err(|e| PersistenceError::Compression(e.to_string()))?;

        // Deserialize
        let session_data: SessionData = rmp_serde::from_slice(&decompressed)?;

        Ok(session_data.metadata)
    }

    /// Save session to storage
    pub async fn save_session(
        &self,
        id: Uuid,
        metadata: &SessionMetadata,
        conversation: &ConversationSnapshot,
        state: &SessionState,
    ) -> Result<()> {
        let session_path = self
            .config
            .base_path
            .join("sessions")
            .join(format!("{}.agcx", id));

        // Create session data
        let session_data = SessionData {
            metadata: metadata.clone(),
            conversation: conversation.clone(),
            state: state.clone(),
            checksum: None, // TODO: Add CRC32 checksum
        };

        // Serialize with MessagePack
        let serialized = rmp_serde::to_vec(&session_data)?;

        // Compress with Zstd
        let compressed = zstd::encode_all(&serialized[..], self.config.compression_level)
            .map_err(|e| PersistenceError::Compression(e.to_string()))?;

        // Calculate compression ratio
        let compression_ratio = compressed.len() as f64 / serialized.len() as f64;

        // Write to file with header
        let file = File::create(&session_path)?;
        let mut writer = BufWriter::new(file);

        // Write header
        writer.write_all(AGCX_MAGIC)?;
        writer.write_all(&FORMAT_VERSION.to_le_bytes())?;
        writer.write_all(&(compressed.len() as u64).to_le_bytes())?;

        // Write compressed data
        writer.write_all(&compressed)?;
        writer.flush()?;

        // Update metadata with actual file size
        let mut updated_metadata = metadata.clone();
        updated_metadata.file_size = compressed.len() as u64;
        updated_metadata.compression_ratio = compression_ratio as f32;

        // Update index
        self.index
            .write()
            .await
            .add_session(updated_metadata.clone());

        // Update metadata cache
        self.metadata_cache
            .write()
            .await
            .insert(id, updated_metadata);

        // Remove from dirty list
        self.dirty_sessions.write().await.remove(&id);

        // Also save metadata separately for fast browsing
        self.save_metadata_cache(id, metadata).await?;

        Ok(())
    }

    /// Save metadata to separate cache file
    async fn save_metadata_cache(&self, id: Uuid, metadata: &SessionMetadata) -> Result<()> {
        let metadata_path = self
            .config
            .base_path
            .join("metadata")
            .join(format!("{}.meta", id));

        let file = File::create(&metadata_path)?;
        let mut writer = BufWriter::new(file);

        // Use bincode for metadata (faster than MessagePack for small data)
        bincode::serde::encode_into_std_write(metadata, &mut writer, bincode::config::standard())?;

        Ok(())
    }

    /// Load session from storage
    pub async fn load_session(
        &self,
        id: Uuid,
    ) -> Result<(SessionMetadata, ConversationSnapshot, SessionState)> {
        let session_path = self
            .config
            .base_path
            .join("sessions")
            .join(format!("{}.agcx", id));

        if !session_path.exists() {
            return Err(PersistenceError::SessionNotFound(id));
        }

        let file = File::open(&session_path)?;
        let mut reader = BufReader::new(file);

        // Read and validate header
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if magic != AGCX_MAGIC {
            return Err(PersistenceError::InvalidFormat(
                "Invalid magic bytes".to_string(),
            ));
        }

        let mut version_bytes = [0u8; 4];
        reader.read_exact(&mut version_bytes)?;
        let version = u32::from_le_bytes(version_bytes);

        if version != FORMAT_VERSION {
            return Err(PersistenceError::IncompatibleVersion {
                expected: FORMAT_VERSION as u16,
                actual: version as u16,
            });
        }

        // Read compressed data length
        let mut len_bytes = [0u8; 8];
        reader.read_exact(&mut len_bytes)?;
        let compressed_len = u64::from_le_bytes(len_bytes) as usize;

        // Read compressed data
        let mut compressed_data = vec![0u8; compressed_len];
        reader.read_exact(&mut compressed_data)?;

        // Decompress
        let decompressed = zstd::decode_all(&compressed_data[..])
            .map_err(|e| PersistenceError::Compression(e.to_string()))?;

        // Deserialize
        let session_data: SessionData = rmp_serde::from_slice(&decompressed)?;

        // TODO: Verify checksum if present

        Ok((
            session_data.metadata,
            session_data.conversation,
            session_data.state,
        ))
    }

    /// Delete session from storage
    pub async fn delete_session(&self, id: Uuid) -> Result<()> {
        // Remove session file
        let session_path = self
            .config
            .base_path
            .join("sessions")
            .join(format!("{}.agcx", id));
        if session_path.exists() {
            fs::remove_file(session_path)?;
        }

        // Remove metadata cache
        let metadata_path = self
            .config
            .base_path
            .join("metadata")
            .join(format!("{}.meta", id));
        if metadata_path.exists() {
            fs::remove_file(metadata_path)?;
        }

        // Remove checkpoints
        let checkpoint_dir = self.config.base_path.join("checkpoints");
        if checkpoint_dir.exists() {
            let pattern = format!("{}_", id);
            for entry in fs::read_dir(checkpoint_dir)? {
                let entry = entry?;
                let path = entry.path();
                if let Some(name) = path.file_name()
                    && name.to_string_lossy().starts_with(&pattern)
                {
                    fs::remove_file(path)?;
                }
            }
        }

        // Update index
        self.index.write().await.remove_session(&id);

        // Update cache
        self.metadata_cache.write().await.remove(&id);
        self.dirty_sessions.write().await.remove(&id);

        Ok(())
    }

    /// List all sessions with metadata
    pub async fn list_sessions(&self) -> Result<Vec<SessionMetadata>> {
        let index = self.index.read().await;
        let mut sessions: Vec<SessionMetadata> = index.sessions.values().cloned().collect();

        // Sort by last accessed time (most recent first)
        sessions.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));

        Ok(sessions)
    }

    /// Search sessions by query
    pub async fn search_sessions(&self, query: &str) -> Result<Vec<SessionMetadata>> {
        let index = self.index.read().await;
        let results = index.search(query);
        Ok(results.into_iter().cloned().collect())
    }

    /// Mark session as dirty (needs saving)
    pub async fn mark_dirty(&self, id: Uuid) {
        self.dirty_sessions.write().await.insert(id, Utc::now());
    }

    /// Get dirty sessions that need saving
    pub async fn get_dirty_sessions(&self) -> Vec<Uuid> {
        self.dirty_sessions.read().await.keys().cloned().collect()
    }

    /// Save all dirty sessions
    pub async fn save_dirty_sessions(&self) -> Result<()> {
        let dirty_ids = self.get_dirty_sessions().await;

        for id in dirty_ids {
            // Load session data from cache or active sessions
            // This would typically be called by SessionManager which has the active data
            // Here we just mark them as no longer dirty if they exist
            if self.metadata_cache.read().await.contains_key(&id) {
                self.dirty_sessions.write().await.remove(&id);
            }
        }

        // Save index
        self.save_index().await?;

        Ok(())
    }

    /// Save session index
    pub async fn save_index(&self) -> Result<()> {
        let index_path = self.config.base_path.join("sessions.idx");
        let index = self.index.read().await;

        let file = File::create(&index_path)?;
        let mut writer = BufWriter::new(file);

        // Serialize with MessagePack
        rmp_serde::encode::write(&mut writer, &*index)?;

        Ok(())
    }

    /// Create checkpoint for a session
    pub async fn create_checkpoint(
        &self,
        session_id: Uuid,
        checkpoint_id: Uuid,
        conversation: &ConversationSnapshot,
        state: &SessionState,
    ) -> Result<()> {
        let checkpoint_path = self
            .config
            .base_path
            .join("checkpoints")
            .join(format!("{}_{}.ckpt", session_id, checkpoint_id));

        // Serialize checkpoint data
        let checkpoint_data = (conversation, state);
        let serialized = rmp_serde::to_vec(&checkpoint_data)?;

        // Compress with higher level for checkpoints (they're accessed less frequently)
        let compressed = zstd::encode_all(&serialized[..], 6)
            .map_err(|e| PersistenceError::Compression(e.to_string()))?;

        // Write to file
        let file = File::create(&checkpoint_path)?;
        let mut writer = BufWriter::new(file);

        // Write header
        writer.write_all(AGCX_MAGIC)?;
        writer.write_all(&FORMAT_VERSION.to_le_bytes())?;
        writer.write_all(&(compressed.len() as u64).to_le_bytes())?;

        // Write compressed data
        writer.write_all(&compressed)?;
        writer.flush()?;

        Ok(())
    }

    /// Load checkpoint
    pub async fn load_checkpoint(
        &self,
        session_id: Uuid,
        checkpoint_id: Uuid,
    ) -> Result<(ConversationSnapshot, SessionState)> {
        let checkpoint_path = self
            .config
            .base_path
            .join("checkpoints")
            .join(format!("{}_{}.ckpt", session_id, checkpoint_id));

        if !checkpoint_path.exists() {
            return Err(PersistenceError::InvalidCheckpoint(format!(
                "Checkpoint {} not found",
                checkpoint_id
            )));
        }

        let file = File::open(&checkpoint_path)?;
        let mut reader = BufReader::new(file);

        // Read and validate header
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if magic != AGCX_MAGIC {
            return Err(PersistenceError::InvalidFormat(
                "Invalid checkpoint magic bytes".to_string(),
            ));
        }

        let mut version_bytes = [0u8; 4];
        reader.read_exact(&mut version_bytes)?;
        let version = u32::from_le_bytes(version_bytes);

        if version != FORMAT_VERSION {
            return Err(PersistenceError::IncompatibleVersion {
                expected: FORMAT_VERSION as u16,
                actual: version as u16,
            });
        }

        // Read compressed data
        let mut len_bytes = [0u8; 8];
        reader.read_exact(&mut len_bytes)?;
        let compressed_len = u64::from_le_bytes(len_bytes) as usize;

        let mut compressed_data = vec![0u8; compressed_len];
        reader.read_exact(&mut compressed_data)?;

        // Decompress
        let decompressed = zstd::decode_all(&compressed_data[..])
            .map_err(|e| PersistenceError::Compression(e.to_string()))?;

        // Deserialize
        let checkpoint_data: (ConversationSnapshot, SessionState) =
            rmp_serde::from_slice(&decompressed)?;

        Ok(checkpoint_data)
    }

    /// Get storage statistics
    pub async fn get_statistics(&self) -> Result<StorageStatistics> {
        let index = self.index.read().await;
        let sessions_dir = self.config.base_path.join("sessions");
        let checkpoints_dir = self.config.base_path.join("checkpoints");

        let mut total_size = 0u64;
        let mut session_count = 0usize;
        let mut checkpoint_count = 0usize;

        // Count sessions and calculate size
        if sessions_dir.exists() {
            for entry in fs::read_dir(sessions_dir)? {
                let entry = entry?;
                let metadata = entry.metadata()?;
                if metadata.is_file() {
                    total_size += metadata.len();
                    session_count += 1;
                }
            }
        }

        // Count checkpoints
        if checkpoints_dir.exists() {
            for entry in fs::read_dir(checkpoints_dir)? {
                let entry = entry?;
                if entry.metadata()?.is_file() {
                    checkpoint_count += 1;
                }
            }
        }

        Ok(StorageStatistics {
            total_sessions: session_count,
            total_checkpoints: checkpoint_count,
            total_size_bytes: total_size,
            indexed_sessions: index.sessions.len(),
            oldest_session: index
                .sessions
                .values()
                .min_by_key(|s| s.created_at)
                .map(|s| s.created_at),
            newest_session: index
                .sessions
                .values()
                .max_by_key(|s| s.created_at)
                .map(|s| s.created_at),
        })
    }

    /// Clean up old sessions based on age and count limits
    pub async fn cleanup_old_sessions(
        &self,
        max_age_days: Option<i64>,
        max_count: Option<usize>,
    ) -> Result<Vec<Uuid>> {
        let mut deleted = Vec::new();
        let index = self.index.read().await.clone();

        // Get all sessions sorted by last accessed time
        let mut sessions: Vec<_> = index.sessions.values().collect();
        sessions.sort_by(|a, b| a.last_accessed.cmp(&b.last_accessed));

        // Delete by age
        if let Some(max_age) = max_age_days {
            let cutoff = Utc::now() - chrono::Duration::days(max_age);
            for session in &sessions {
                if session.last_accessed < cutoff && !session.is_favorite {
                    self.delete_session(session.id).await?;
                    deleted.push(session.id);
                }
            }
        }

        // Delete by count
        if let Some(max) = max_count {
            let remaining = sessions.len().saturating_sub(deleted.len());
            if remaining > max {
                let to_delete = remaining - max;
                for session in sessions.iter().take(to_delete) {
                    if !deleted.contains(&session.id) && !session.is_favorite {
                        self.delete_session(session.id).await?;
                        deleted.push(session.id);
                    }
                }
            }
        }

        Ok(deleted)
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStatistics {
    pub total_sessions: usize,
    pub total_checkpoints: usize,
    pub total_size_bytes: u64,
    pub indexed_sessions: usize,
    pub oldest_session: Option<DateTime<Utc>>,
    pub newest_session: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ConversationContext;
    use crate::types::OperatingMode;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_session_store_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = SessionStoreConfig {
            base_path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let store = SessionStore::new(config).await.unwrap();
        assert!(temp_dir.path().join("sessions").exists());
        assert!(temp_dir.path().join("checkpoints").exists());
        assert!(temp_dir.path().join("metadata").exists());
    }

    #[tokio::test]
    async fn test_save_and_load_session() {
        let temp_dir = TempDir::new().unwrap();
        let config = SessionStoreConfig {
            base_path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let store = SessionStore::new(config).await.unwrap();

        // Create test data
        let id = Uuid::new_v4();
        let metadata = SessionMetadata {
            id,
            title: "Test Session".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_accessed: Utc::now(),
            message_count: 1,
            turn_count: 1,
            current_mode: OperatingMode::Build,
            model: "gpt-4".to_string(),
            tags: vec!["test".to_string()],
            is_favorite: false,
            file_size: 0,
            compression_ratio: 0.0,
            format_version: FORMAT_VERSION as u16,
            checkpoints: vec![],
        };

        let conversation = ConversationSnapshot {
            id,
            messages: vec![],
            context: ConversationContext {
                working_directory: PathBuf::from("."),
                environment_variables: HashMap::new(),
                open_files: Vec::new(),
                ast_index_state: None,
                embedding_cache: None,
            },
            mode_history: vec![(OperatingMode::Build, Utc::now())],
        };

        let state = SessionState::default();

        // Save session
        store
            .save_session(id, &metadata, &conversation, &state)
            .await
            .unwrap();

        // Load session
        let (loaded_meta, loaded_conv, loaded_state) = store.load_session(id).await.unwrap();

        assert_eq!(loaded_meta.id, id);
        assert_eq!(loaded_meta.title, "Test Session");
        assert_eq!(loaded_conv.id, id);
    }

    #[tokio::test]
    async fn test_list_and_search_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let config = SessionStoreConfig {
            base_path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let store = SessionStore::new(config).await.unwrap();

        // Create multiple test sessions
        for i in 0..3 {
            let id = Uuid::new_v4();
            let metadata = SessionMetadata {
                id,
                title: format!("Test Session {}", i),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                last_accessed: Utc::now(),
                message_count: i,
                turn_count: i,
                current_mode: OperatingMode::Build,
                model: "gpt-4".to_string(),
                tags: vec![],
                is_favorite: false,
                file_size: 0,
                compression_ratio: 0.0,
                format_version: FORMAT_VERSION as u16,
                checkpoints: vec![],
            };

            let conversation = ConversationSnapshot {
                id,
                messages: vec![],
                context: ConversationContext {
                    working_directory: PathBuf::from("."),
                    environment_variables: HashMap::new(),
                    open_files: Vec::new(),
                    ast_index_state: None,
                    embedding_cache: None,
                },
                mode_history: vec![],
            };

            let state = SessionState::default();

            store
                .save_session(id, &metadata, &conversation, &state)
                .await
                .unwrap();
        }

        // List sessions
        let sessions = store.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 3);

        // Search sessions
        let results = store.search_sessions("Test Session 1").await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].title.contains("Test Session 1"));
    }
}
