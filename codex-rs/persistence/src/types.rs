//! Core types for session persistence

use chrono::DateTime;
use chrono::Utc;
// Import and re-export types from core crate
pub use agcodex_core::models::ContentItem;
pub use agcodex_core::models::ResponseItem;
pub use agcodex_core::modes::OperatingMode;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Session metadata stored in bincode format for fast access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub id: Uuid,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub message_count: usize,
    pub turn_count: usize,
    pub current_mode: OperatingMode,
    pub model: String,
    pub tags: Vec<String>,
    pub is_favorite: bool,
    pub file_size: u64,
    pub compression_ratio: f32,
    pub format_version: u16,
    pub checkpoints: Vec<CheckpointMetadata>,
}

/// Metadata for a checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub message_index: usize,
    pub description: Option<String>,
}

/// Complete checkpoint data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub metadata: CheckpointMetadata,
    pub conversation: ConversationSnapshot,
    pub state: SessionState,
}

/// Snapshot of a conversation at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSnapshot {
    pub id: Uuid,
    pub messages: Vec<MessageSnapshot>,
    pub context: ConversationContext,
    pub mode_history: Vec<(OperatingMode, DateTime<Utc>)>,
}

/// Individual message snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSnapshot {
    pub item: ResponseItem,
    pub timestamp: DateTime<Utc>,
    pub turn_index: usize,
    pub metadata: MessageMetadata,
}

/// Additional metadata for messages
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageMetadata {
    pub edited: bool,
    pub edit_history: Vec<(DateTime<Utc>, ResponseItem)>,
    pub branch_point: bool,
    pub branch_id: Option<Uuid>,
    pub parent_message_id: Option<Uuid>,
    pub message_id: Uuid,
    pub file_context: Vec<FileContext>,
    pub tool_calls: Vec<ToolCallMetadata>,
}

/// File context at the time of message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContext {
    pub path: PathBuf,
    pub line_range: Option<(usize, usize)>,
    pub snippet: Option<String>,
    pub language: Option<String>,
}

/// Tool call metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallMetadata {
    pub tool_name: String,
    pub execution_time_ms: u64,
    pub success: bool,
    pub error: Option<String>,
}

/// Conversation context for restoration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    pub working_directory: PathBuf,
    pub environment_variables: HashMap<String, String>,
    pub open_files: Vec<PathBuf>,
    pub ast_index_state: Option<AstIndexState>,
    pub embedding_cache: Option<EmbeddingCacheState>,
}

/// AST index state for context restoration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstIndexState {
    pub indexed_files: Vec<PathBuf>,
    pub total_symbols: usize,
    pub cache_size_bytes: u64,
    pub last_update: DateTime<Utc>,
}

/// Embedding cache state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingCacheState {
    pub cached_chunks: usize,
    pub model: String,
    pub cache_size_bytes: u64,
}

/// Session state for UI restoration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub cursor_position: usize,
    pub scroll_offset: usize,
    pub selected_message: Option<Uuid>,
    pub expanded_messages: Vec<Uuid>,
    pub active_panel: String,
    pub panel_sizes: HashMap<String, f32>,
    pub search_query: Option<String>,
    pub filter_settings: FilterSettings,
}

/// Filter settings for message display
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilterSettings {
    pub show_system: bool,
    pub show_tool_calls: bool,
    pub show_reasoning: bool,
    pub role_filter: Option<String>,
    pub date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
}

/// Index of all sessions for fast lookup
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionIndex {
    pub sessions: HashMap<Uuid, SessionMetadata>,
    pub recent_sessions: Vec<Uuid>,
    pub favorite_sessions: Vec<Uuid>,
    pub tag_index: HashMap<String, Vec<Uuid>>,
    pub last_updated: DateTime<Utc>,
    pub total_size_bytes: u64,
}

impl SessionIndex {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            recent_sessions: Vec::new(),
            favorite_sessions: Vec::new(),
            tag_index: HashMap::new(),
            last_updated: Utc::now(),
            total_size_bytes: 0,
        }
    }

    pub fn add_session(&mut self, metadata: SessionMetadata) {
        let id = metadata.id;

        // Update favorites
        if metadata.is_favorite && !self.favorite_sessions.contains(&id) {
            self.favorite_sessions.push(id);
        }

        // Update tag index
        for tag in &metadata.tags {
            self.tag_index.entry(tag.clone()).or_default().push(id);
        }

        // Update recent sessions (keep last 20)
        self.recent_sessions.retain(|&sid| sid != id);
        self.recent_sessions.insert(0, id);
        if self.recent_sessions.len() > 20 {
            self.recent_sessions.truncate(20);
        }

        // Update total size
        self.total_size_bytes += metadata.file_size;

        // Insert metadata
        self.sessions.insert(id, metadata);
        self.last_updated = Utc::now();
    }

    pub fn remove_session(&mut self, id: &Uuid) -> Option<SessionMetadata> {
        if let Some(metadata) = self.sessions.remove(id) {
            // Update recent sessions
            self.recent_sessions.retain(|sid| sid != id);

            // Update favorites
            self.favorite_sessions.retain(|sid| sid != id);

            // Update tag index
            for tag in &metadata.tags {
                if let Some(sessions) = self.tag_index.get_mut(tag) {
                    sessions.retain(|sid| sid != id);
                    if sessions.is_empty() {
                        self.tag_index.remove(tag);
                    }
                }
            }

            // Update total size
            self.total_size_bytes -= metadata.file_size;
            self.last_updated = Utc::now();

            Some(metadata)
        } else {
            None
        }
    }

    pub fn search(&self, query: &str) -> Vec<&SessionMetadata> {
        let query_lower = query.to_lowercase();
        self.sessions
            .values()
            .filter(|metadata| {
                metadata.title.to_lowercase().contains(&query_lower)
                    || metadata
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .collect()
    }
}
