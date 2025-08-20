//! High-level session management with auto-save and checkpointing

use crate::compression::CompressionLevel;
use crate::error::PersistenceError;
use crate::error::Result;
use crate::migration::MigrationManager;
use crate::storage::SessionStorage;
use crate::storage::StorageBackend;
use crate::types::Checkpoint;
use crate::types::CheckpointMetadata;
use crate::types::ConversationContext;
use crate::types::ConversationSnapshot;
use crate::types::MessageMetadata;
use crate::types::MessageSnapshot;
use crate::types::SessionIndex;
use crate::types::SessionMetadata;
use crate::types::SessionState;
use chrono::DateTime;
use chrono::Utc;
// Temporarily use local types until core is fixed
// use agcodex_core::models::ResponseItem;
// use agcodex_core::modes::OperatingMode;
use crate::types::OperatingMode;
use crate::types::ResponseItem;
use dirs;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::error;
use tracing::info;
use uuid::Uuid;

/// Configuration for SessionManager
#[derive(Debug, Clone)]
pub struct SessionManagerConfig {
    /// Base path for storage (defaults to ~/.agcodex/history/)
    pub storage_path: PathBuf,
    /// Auto-save interval (defaults to 5 minutes)
    pub auto_save_interval: Duration,
    /// Maximum number of sessions to keep (defaults to 100)
    pub max_sessions: usize,
    /// Maximum size of all sessions in bytes (defaults to 1GB)
    pub max_total_size: u64,
    /// Compression level (defaults to Balanced)
    pub compression_level: CompressionLevel,
    /// Enable auto-save (defaults to true)
    pub enable_auto_save: bool,
    /// Enable memory mapping for metadata (defaults to true)
    pub enable_mmap: bool,
    /// Maximum checkpoints per session (defaults to 10)
    pub max_checkpoints: usize,
}

impl Default for SessionManagerConfig {
    fn default() -> Self {
        let storage_path = dirs::home_dir()
            .map(|p| p.join(".agcodex/history"))
            .unwrap_or_else(|| PathBuf::from(".agcodex/history"));

        Self {
            storage_path,
            auto_save_interval: Duration::from_secs(300), // 5 minutes
            max_sessions: 100,
            max_total_size: 1_073_741_824, // 1GB
            compression_level: CompressionLevel::Balanced,
            enable_auto_save: true,
            enable_mmap: true,
            max_checkpoints: 10,
        }
    }
}

/// Main session manager for AGCodex
pub struct SessionManager {
    config: SessionManagerConfig,
    storage: Arc<SessionStorage>,
    index: Arc<RwLock<SessionIndex>>,
    active_sessions: Arc<RwLock<HashMap<Uuid, ActiveSession>>>,
    auto_save_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

/// Active session being tracked
struct ActiveSession {
    metadata: SessionMetadata,
    conversation: ConversationSnapshot,
    state: SessionState,
    dirty: bool,
    last_saved: DateTime<Utc>,
}

impl SessionManager {
    /// Create a new session manager
    pub async fn new(config: SessionManagerConfig) -> Result<Self> {
        let storage = Arc::new(SessionStorage::new(
            config.storage_path.clone(),
            config.compression_level,
        )?);

        // Load or create index
        let index = match storage.load_index().await {
            Ok(idx) => idx,
            Err(_) => {
                info!("Creating new session index");
                SessionIndex::new()
            }
        };

        let manager = Self {
            config: config.clone(),
            storage: storage.clone(),
            index: Arc::new(RwLock::new(index)),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            auto_save_handle: Arc::new(Mutex::new(None)),
        };

        // Check for needed migrations
        let migration_manager = MigrationManager::new(config.storage_path.clone());
        if let Some(plan) = migration_manager.check_migration_needed()? {
            info!("Migration needed: {:?}", plan);
            match migration_manager.migrate(plan).await {
                Ok(report) => {
                    info!("Migration completed: {:?}", report);
                    // Reload index after migration
                    if let Ok(idx) = storage.load_index().await {
                        *manager.index.write().await = idx;
                    }
                }
                Err(e) => {
                    error!("Migration failed: {}", e);
                    return Err(e);
                }
            }
        }

        // Start auto-save task if enabled
        if config.enable_auto_save {
            manager.start_auto_save().await;
        }

        Ok(manager)
    }

    /// Start auto-save task
    async fn start_auto_save(&self) {
        let storage = self.storage.clone();
        let active_sessions = self.active_sessions.clone();
        let index = self.index.clone();
        let interval_duration = self.config.auto_save_interval;

        let handle = tokio::spawn(async move {
            let mut ticker = interval(interval_duration);
            ticker.tick().await; // Skip first immediate tick

            loop {
                ticker.tick().await;

                // Save all dirty sessions
                let sessions = active_sessions.read().await;
                for (id, session) in sessions.iter() {
                    if session.dirty {
                        if let Err(e) = storage
                            .save_session(
                                *id,
                                &session.metadata,
                                &session.conversation,
                                &session.state,
                            )
                            .await
                        {
                            error!("Auto-save failed for session {}: {}", id, e);
                        } else {
                            info!("Auto-saved session {}", id);
                        }
                    }
                }

                // Save index
                if let Err(e) = storage.save_index(&*index.read().await).await {
                    error!("Failed to save index: {}", e);
                }
            }
        });

        *self.auto_save_handle.lock().await = Some(handle);
    }

    /// Create a new session
    pub async fn create_session(
        &self,
        title: String,
        model: String,
        mode: OperatingMode,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let metadata = SessionMetadata {
            id,
            title,
            created_at: now,
            updated_at: now,
            last_accessed: now,
            message_count: 0,
            turn_count: 0,
            current_mode: mode,
            model,
            tags: Vec::new(),
            is_favorite: false,
            file_size: 0,
            compression_ratio: 0.0,
            format_version: crate::FORMAT_VERSION,
            checkpoints: Vec::new(),
        };

        let conversation = ConversationSnapshot {
            id,
            messages: Vec::new(),
            context: ConversationContext {
                working_directory: std::env::current_dir().unwrap_or_default(),
                environment_variables: HashMap::new(),
                open_files: Vec::new(),
                ast_index_state: None,
                embedding_cache: None,
            },
            mode_history: vec![(mode, now)],
        };

        let state = SessionState {
            cursor_position: 0,
            scroll_offset: 0,
            selected_message: None,
            expanded_messages: Vec::new(),
            active_panel: "main".to_string(),
            panel_sizes: HashMap::new(),
            search_query: None,
            filter_settings: Default::default(),
        };

        // Add to active sessions
        self.active_sessions.write().await.insert(
            id,
            ActiveSession {
                metadata: metadata.clone(),
                conversation,
                state,
                dirty: true,
                last_saved: now,
            },
        );

        // Update index
        self.index.write().await.add_session(metadata);

        info!("Created new session: {}", id);
        Ok(id)
    }

    /// Load a session from storage
    pub async fn load_session(&self, id: Uuid) -> Result<()> {
        // Check if already loaded
        if self.active_sessions.read().await.contains_key(&id) {
            return Ok(());
        }

        // Load from storage
        let (mut metadata, conversation, state) = self.storage.load_session(id).await?;

        // Update last accessed time
        metadata.last_accessed = Utc::now();

        // Add to active sessions
        self.active_sessions.write().await.insert(
            id,
            ActiveSession {
                metadata: metadata.clone(),
                conversation,
                state,
                dirty: false,
                last_saved: Utc::now(),
            },
        );

        // Update index
        self.index.write().await.add_session(metadata);

        info!("Loaded session: {}", id);
        Ok(())
    }

    /// Save a session to storage
    pub async fn save_session(&self, id: Uuid) -> Result<()> {
        let mut sessions = self.active_sessions.write().await;

        let session = sessions
            .get_mut(&id)
            .ok_or(PersistenceError::SessionNotFound(id))?;

        // Update metadata
        session.metadata.updated_at = Utc::now();
        session.metadata.message_count = session.conversation.messages.len();

        // Calculate file size and compression ratio (approximation)
        let uncompressed_size =
            bincode::serde::encode_to_vec(&session.conversation, bincode::config::standard())?
                .len();
        session.metadata.file_size = uncompressed_size as u64;
        session.metadata.compression_ratio = 0.7; // Estimate

        // Save to storage
        self.storage
            .save_session(id, &session.metadata, &session.conversation, &session.state)
            .await?;

        // Mark as clean
        session.dirty = false;
        session.last_saved = Utc::now();

        // Update index
        self.index
            .write()
            .await
            .add_session(session.metadata.clone());

        info!("Saved session: {}", id);
        Ok(())
    }

    /// Add a message to a session
    pub async fn add_message(
        &self,
        session_id: Uuid,
        item: ResponseItem,
        metadata: Option<MessageMetadata>,
    ) -> Result<()> {
        let mut sessions = self.active_sessions.write().await;

        let session = sessions
            .get_mut(&session_id)
            .ok_or(PersistenceError::SessionNotFound(session_id))?;

        let message = MessageSnapshot {
            item,
            timestamp: Utc::now(),
            turn_index: session.conversation.messages.len(),
            metadata: metadata.unwrap_or_default(),
        };

        session.conversation.messages.push(message);
        session.metadata.message_count += 1;
        session.metadata.updated_at = Utc::now();
        session.dirty = true;

        Ok(())
    }

    /// Create a checkpoint
    pub async fn create_checkpoint(
        &self,
        session_id: Uuid,
        name: String,
        description: Option<String>,
    ) -> Result<Uuid> {
        let mut sessions = self.active_sessions.write().await;

        let session = sessions
            .get_mut(&session_id)
            .ok_or(PersistenceError::SessionNotFound(session_id))?;

        let checkpoint_id = Uuid::new_v4();
        let checkpoint_metadata = CheckpointMetadata {
            id: checkpoint_id,
            name,
            created_at: Utc::now(),
            message_index: session.conversation.messages.len(),
            description,
        };

        // Add to session metadata
        session
            .metadata
            .checkpoints
            .push(checkpoint_metadata.clone());

        // Limit number of checkpoints
        if session.metadata.checkpoints.len() > self.config.max_checkpoints {
            session.metadata.checkpoints.remove(0);
        }

        session.dirty = true;

        // Save checkpoint data
        let checkpoint = Checkpoint {
            metadata: checkpoint_metadata,
            conversation: session.conversation.clone(),
            state: session.state.clone(),
        };

        // Save checkpoint to disk
        let checkpoint_path = self
            .config
            .storage_path
            .join("checkpoints")
            .join(format!("{}_{}.ckpt", session_id, checkpoint_id));

        tokio::fs::create_dir_all(checkpoint_path.parent().unwrap()).await?;

        let checkpoint_bytes =
            bincode::serde::encode_to_vec(&checkpoint, bincode::config::standard())?;
        let compressed = zstd::encode_all(&checkpoint_bytes[..], 3)
            .map_err(|e| PersistenceError::Compression(e.to_string()))?;

        tokio::fs::write(&checkpoint_path, &compressed).await?;

        info!(
            "Created checkpoint {} for session {}",
            checkpoint_id, session_id
        );
        Ok(checkpoint_id)
    }

    /// Restore from a checkpoint
    pub async fn restore_checkpoint(&self, session_id: Uuid, checkpoint_id: Uuid) -> Result<()> {
        let checkpoint_path = self
            .config
            .storage_path
            .join("checkpoints")
            .join(format!("{}_{}.ckpt", session_id, checkpoint_id));

        if !checkpoint_path.exists() {
            return Err(PersistenceError::InvalidCheckpoint(format!(
                "Checkpoint {} not found",
                checkpoint_id
            )));
        }

        // Load checkpoint
        let compressed = tokio::fs::read(&checkpoint_path).await?;
        let checkpoint_bytes = zstd::decode_all(&compressed[..])
            .map_err(|e| PersistenceError::Compression(e.to_string()))?;
        let (checkpoint, _): (Checkpoint, _) =
            bincode::serde::decode_from_slice(&checkpoint_bytes, bincode::config::standard())?;

        // Restore to active session
        let mut sessions = self.active_sessions.write().await;

        if let Some(session) = sessions.get_mut(&session_id) {
            session.conversation = checkpoint.conversation;
            session.state = checkpoint.state;
            session.dirty = true;
            info!(
                "Restored checkpoint {} for session {}",
                checkpoint_id, session_id
            );
        } else {
            return Err(PersistenceError::SessionNotFound(session_id));
        }

        Ok(())
    }

    /// Delete a session
    pub async fn delete_session(&self, id: Uuid) -> Result<()> {
        // Remove from active sessions
        self.active_sessions.write().await.remove(&id);

        // Remove from index
        self.index.write().await.remove_session(&id);

        // Delete from storage
        self.storage.delete_session(id).await?;

        // Delete checkpoints
        let checkpoint_dir = self.config.storage_path.join("checkpoints");
        if checkpoint_dir.exists() {
            let pattern = format!("{}_", id);
            let mut entries = tokio::fs::read_dir(&checkpoint_dir).await?;

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if let Some(name) = path.file_name()
                    && name.to_string_lossy().starts_with(&pattern)
                {
                    tokio::fs::remove_file(&path).await?;
                }
            }
        }

        info!("Deleted session: {}", id);
        Ok(())
    }

    /// List all sessions
    pub async fn list_sessions(&self) -> Result<Vec<SessionMetadata>> {
        let sessions = self.storage.list_sessions().await?;
        Ok(sessions)
    }

    /// Search sessions
    pub async fn search_sessions(&self, query: &str) -> Vec<SessionMetadata> {
        let index = self.index.read().await;
        index.search(query).into_iter().cloned().collect()
    }

    /// Get session metadata
    pub async fn get_session_metadata(&self, id: Uuid) -> Result<SessionMetadata> {
        // Check active sessions first
        if let Some(session) = self.active_sessions.read().await.get(&id) {
            return Ok(session.metadata.clone());
        }

        // Check index
        if let Some(metadata) = self.index.read().await.sessions.get(&id) {
            return Ok(metadata.clone());
        }

        Err(PersistenceError::SessionNotFound(id))
    }

    /// Update session state (cursor, scroll, etc.)
    pub async fn update_session_state(&self, session_id: Uuid, state: SessionState) -> Result<()> {
        let mut sessions = self.active_sessions.write().await;

        let session = sessions
            .get_mut(&session_id)
            .ok_or(PersistenceError::SessionNotFound(session_id))?;

        session.state = state;
        session.dirty = true;

        Ok(())
    }

    /// Switch operating mode for a session
    pub async fn switch_mode(&self, session_id: Uuid, new_mode: OperatingMode) -> Result<()> {
        let mut sessions = self.active_sessions.write().await;

        let session = sessions
            .get_mut(&session_id)
            .ok_or(PersistenceError::SessionNotFound(session_id))?;

        session.metadata.current_mode = new_mode;
        session
            .conversation
            .mode_history
            .push((new_mode, Utc::now()));
        session.dirty = true;

        Ok(())
    }

    /// Clean up old sessions based on max_sessions and max_total_size
    pub async fn cleanup_old_sessions(&self) -> Result<()> {
        let index = self.index.read().await;

        // Check if cleanup is needed
        if index.sessions.len() <= self.config.max_sessions
            && index.total_size_bytes <= self.config.max_total_size
        {
            return Ok(());
        }

        // Sort sessions by last accessed time
        let mut sessions: Vec<_> = index.sessions.values().collect();
        sessions.sort_by(|a, b| a.last_accessed.cmp(&b.last_accessed));

        // Calculate how many to remove
        let to_remove = if index.sessions.len() > self.config.max_sessions {
            index.sessions.len() - self.config.max_sessions
        } else {
            0
        };

        // Remove oldest sessions
        for metadata in sessions.iter().take(to_remove) {
            if !metadata.is_favorite {
                self.delete_session(metadata.id).await?;
                info!("Cleaned up old session: {}", metadata.id);
            }
        }

        Ok(())
    }

    /// Shutdown the session manager
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down SessionManager");

        // Stop auto-save task
        if let Some(handle) = self.auto_save_handle.lock().await.take() {
            handle.abort();
        }

        // Save all dirty sessions
        let sessions = self.active_sessions.read().await;
        for (id, session) in sessions.iter() {
            if session.dirty {
                self.storage
                    .save_session(
                        *id,
                        &session.metadata,
                        &session.conversation,
                        &session.state,
                    )
                    .await?;
            }
        }

        // Save index
        self.storage.save_index(&*self.index.read().await).await?;

        info!("SessionManager shutdown complete");
        Ok(())
    }
}

impl Drop for SessionManager {
    fn drop(&mut self) {
        // Try to save on drop (best effort)
        let storage = self.storage.clone();
        let sessions = self.active_sessions.clone();
        let index = self.index.clone();

        tokio::spawn(async move {
            let sessions = sessions.read().await;
            for (id, session) in sessions.iter() {
                if session.dirty {
                    let _ = storage
                        .save_session(
                            *id,
                            &session.metadata,
                            &session.conversation,
                            &session.state,
                        )
                        .await;
                }
            }
            let _ = storage.save_index(&*index.read().await).await;
        });
    }
}
