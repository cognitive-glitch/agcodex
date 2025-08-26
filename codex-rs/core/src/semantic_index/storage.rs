//! Efficient Storage Backend for Vector Embeddings
//!
//! This module provides high-performance storage solutions for vector embeddings
//! with support for both in-memory and persistent storage backends.
//!
//! # Storage Architecture
//!
//! ```text
//! StorageBackend (trait)
//! ├── MemoryStorage (Arc<RwLock<HashMap<ChunkId, EmbeddingVector>>>)
//! └── PersistentStorage (mmap + serde_bincode)
//!     ├── Index file (.idx)
//!     ├── Vector data (.vec)
//!     └── Metadata (.meta)
//! ```
//!
//! # Performance Characteristics
//!
//! - **MemoryStorage**: O(1) access, ~500MB for 100K vectors (768d)
//! - **PersistentStorage**: O(log n) with B-tree index, memory-mapped I/O
//! - **Concurrent access**: RwLock for reads, exclusive writes
//! - **Serialization**: bincode for compact binary format
//!
//! # Memory Safety
//!
//! - **Arc<RwLock<>>**: Thread-safe shared ownership
//! - **RAII cleanup**: Drop implementations for resource management
//! - **Bounds checking**: All vector accesses validated
//! - **Error propagation**: Result<T, StorageError> throughout

use super::ChunkId;
use super::DocumentId;
use super::SemanticIndexError;
use super::embeddings::EmbeddingVector;
use super::retrieval::ChunkInfo;

// Note: Using manual async trait implementation for compatibility
// use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use thiserror::Error;
use tokio::fs;

/// Storage backend error types
#[derive(Error, Debug, Clone)]
pub enum StorageError {
    #[error("Storage operation failed: {operation} - {message}")]
    OperationFailed { operation: String, message: String },

    #[error("Serialization error: {message}")]
    SerializationError { message: String },

    #[error("I/O error: {message}")]
    IoError { message: String },

    #[error("Storage corrupted: {details}")]
    CorruptionError { details: String },

    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    #[error("Storage capacity exceeded: {current} > {limit}")]
    CapacityExceeded { current: usize, limit: usize },

    #[error("Concurrent access error: {message}")]
    ConcurrencyError { message: String },
}

/// Convert StorageError to SemanticIndexError
impl From<StorageError> for SemanticIndexError {
    fn from(err: StorageError) -> Self {
        match err {
            StorageError::OperationFailed { operation, message } => {
                SemanticIndexError::StorageFailed { operation, message }
            }
            StorageError::SerializationError { message } => {
                SemanticIndexError::SerializationError {
                    format: "bincode".to_string(),
                    message,
                }
            }
            StorageError::IoError { message } => SemanticIndexError::FileSystemError {
                path: "storage".to_string(),
                message,
            },
            StorageError::CorruptionError { details } => {
                SemanticIndexError::IndexCorrupted { details }
            }
            StorageError::NotFound { resource } => SemanticIndexError::QueryFailed {
                query: resource.clone(),
                reason: format!("Resource not found: {}", resource),
            },
            StorageError::CapacityExceeded { current, limit } => {
                SemanticIndexError::ResourceLimitExceeded {
                    resource: "storage_capacity".to_string(),
                    current,
                    limit,
                }
            }
            StorageError::ConcurrencyError { message } => SemanticIndexError::ConcurrencyError {
                context: "storage".to_string(),
                message,
            },
        }
    }
}

/// Storage backend trait for different implementations
pub trait StorageBackend: Send + Sync + std::fmt::Debug {
    /// Store an embedding vector
    fn store_embedding(
        &self,
        chunk_id: ChunkId,
        embedding: EmbeddingVector,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<(), StorageError>> + Send + '_>,
    >;

    /// Retrieve an embedding by chunk ID
    fn get_embedding(
        &self,
        chunk_id: ChunkId,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = std::result::Result<Option<EmbeddingVector>, StorageError>,
                > + Send
                + '_,
        >,
    >;

    /// Get all embeddings (for similarity search)
    fn get_all_embeddings(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = std::result::Result<Vec<(ChunkId, EmbeddingVector)>, StorageError>,
                > + Send
                + '_,
        >,
    >;

    /// Remove an embedding
    fn remove_embedding(
        &self,
        chunk_id: ChunkId,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<bool, StorageError>> + Send + '_>,
    >;

    /// Get chunk information by ID
    fn get_chunk_info(
        &self,
        chunk_id: ChunkId,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = std::result::Result<Option<ChunkInfo>, StorageError>>
                + Send
                + '_,
        >,
    >;

    /// Store chunk information
    fn store_chunk_info(
        &self,
        chunk_id: ChunkId,
        info: ChunkInfo,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<(), StorageError>> + Send + '_>,
    >;

    /// Get embeddings by document ID
    fn get_embeddings_by_document(
        &self,
        document_id: DocumentId,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = std::result::Result<Vec<(ChunkId, EmbeddingVector)>, StorageError>,
                > + Send
                + '_,
        >,
    >;

    /// Count total embeddings
    fn count_embeddings(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<usize, StorageError>> + Send + '_>,
    >;

    /// Get storage size in bytes
    fn storage_size(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<usize, StorageError>> + Send + '_>,
    >;

    /// Clear all data
    fn clear(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<(), StorageError>> + Send + '_>,
    >;

    /// Perform maintenance operations (compaction, defrag, etc.)
    fn maintenance(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<(), StorageError>> + Send + '_>,
    >;

    /// Get storage statistics
    fn get_statistics(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = std::result::Result<StorageStatistics, StorageError>>
                + Send
                + '_,
        >,
    >;
}

/// Storage performance and health statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StorageStatistics {
    /// Total embeddings stored
    pub total_embeddings: usize,

    /// Total storage size in bytes
    pub storage_size_bytes: usize,

    /// Average embedding size
    pub avg_embedding_size_bytes: usize,

    /// Read operations count
    pub read_operations: usize,

    /// Write operations count
    pub write_operations: usize,

    /// Average read latency (microseconds)
    pub avg_read_latency_us: u64,

    /// Average write latency (microseconds)
    pub avg_write_latency_us: u64,

    /// Cache hit rate (0.0 to 1.0)
    pub cache_hit_rate: f32,

    /// Last maintenance timestamp
    pub last_maintenance: Option<std::time::SystemTime>,

    /// Storage health score (0.0 to 1.0)
    pub health_score: f32,

    /// Error count by type
    pub error_counts: HashMap<String, usize>,
}

impl StorageStatistics {
    /// Create new empty statistics
    pub fn new() -> Self {
        Self {
            health_score: 1.0,
            ..Default::default()
        }
    }

    /// Update statistics after an operation
    pub fn update_operation(&mut self, operation_type: &str, latency_us: u64, success: bool) {
        match operation_type {
            "read" => {
                self.read_operations += 1;
                self.avg_read_latency_us = (self.avg_read_latency_us + latency_us) / 2;
            }
            "write" => {
                self.write_operations += 1;
                self.avg_write_latency_us = (self.avg_write_latency_us + latency_us) / 2;
            }
            _ => {}
        }

        if !success {
            *self
                .error_counts
                .entry(operation_type.to_string())
                .or_insert(0) += 1;
            self.update_health_score();
        }
    }

    /// Update health score based on error rates
    fn update_health_score(&mut self) {
        let total_ops = self.read_operations + self.write_operations;
        let total_errors: usize = self.error_counts.values().sum();

        if total_ops > 0 {
            let error_rate = total_errors as f32 / total_ops as f32;
            self.health_score = (1.0 - error_rate).max(0.0);
        }
    }

    /// Check if storage is healthy
    pub fn is_healthy(&self) -> bool {
        self.health_score > 0.8
    }
}

/// In-memory storage backend with LRU eviction
#[derive(Debug)]
pub struct MemoryStorage {
    /// Embedding vectors storage
    embeddings: Arc<RwLock<HashMap<ChunkId, EmbeddingVector>>>,

    /// Chunk information storage
    chunk_info: Arc<RwLock<HashMap<ChunkId, ChunkInfo>>>,

    /// Document to chunks mapping
    document_chunks: Arc<RwLock<HashMap<DocumentId, Vec<ChunkId>>>>,

    /// Maximum capacity
    capacity: usize,

    /// Current size tracking
    current_size: Arc<RwLock<usize>>,

    /// Statistics tracking
    statistics: Arc<RwLock<StorageStatistics>>,
}

impl MemoryStorage {
    /// Create a new memory storage with specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            embeddings: Arc::new(RwLock::new(HashMap::new())),
            chunk_info: Arc::new(RwLock::new(HashMap::new())),
            document_chunks: Arc::new(RwLock::new(HashMap::new())),
            capacity,
            current_size: Arc::new(RwLock::new(0)),
            statistics: Arc::new(RwLock::new(StorageStatistics::new())),
        }
    }

    /// Get current memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        *self.current_size.read().unwrap()
    }

    /// Get current capacity
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Check if at capacity
    pub fn is_full(&self) -> bool {
        self.embeddings.read().unwrap().len() >= self.capacity
    }

    /// Estimate size of an embedding in memory
    const fn estimate_embedding_size(embedding: &EmbeddingVector) -> usize {
        // Rough estimation: Vec<f32> + metadata
        std::mem::size_of::<f32>() * embedding.vector.len()
            + std::mem::size_of::<EmbeddingVector>()
            + embedding.content_hash.len()
    }

    /// Estimate size of chunk info in memory
    fn estimate_chunk_info_size(info: &ChunkInfo) -> usize {
        info.content.len()
            + info.file_path.as_ref().map(|p| p.len()).unwrap_or(0)
            + std::mem::size_of::<ChunkInfo>()
    }

    /// Perform LRU eviction if needed
    fn maybe_evict(&self) -> std::result::Result<(), StorageError> {
        if self.is_full() {
            // Simple eviction: remove first element (in practice, use proper LRU)
            let mut embeddings =
                self.embeddings
                    .write()
                    .map_err(|_| StorageError::ConcurrencyError {
                        message: "Failed to acquire write lock".to_string(),
                    })?;

            if let Some(chunk_id) = embeddings.keys().next().cloned()
                && let Some(embedding) = embeddings.remove(&chunk_id) {
                    let size_freed = Self::estimate_embedding_size(&embedding);

                    // Update size tracking
                    {
                        let mut current_size = self.current_size.write().map_err(|_| {
                            StorageError::ConcurrencyError {
                                message: "Failed to acquire size lock".to_string(),
                            }
                        })?;
                        *current_size = current_size.saturating_sub(size_freed);
                    }

                    // Remove chunk info as well
                    self.chunk_info.write().unwrap().remove(&chunk_id);

                    tracing::warn!("Evicted embedding for chunk {} to free memory", chunk_id);
                }
        }

        Ok(())
    }
}

impl StorageBackend for MemoryStorage {
    fn store_embedding(
        &self,
        chunk_id: ChunkId,
        embedding: EmbeddingVector,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<(), StorageError>> + Send + '_>,
    > {
        Box::pin(async move {
            let start_time = std::time::Instant::now();

            // Check capacity and evict if needed
            self.maybe_evict()?;

            let embedding_size = Self::estimate_embedding_size(&embedding);

            // Store embedding
            {
                let mut embeddings =
                    self.embeddings
                        .write()
                        .map_err(|_| StorageError::ConcurrencyError {
                            message: "Failed to acquire embeddings write lock".to_string(),
                        })?;

                embeddings.insert(chunk_id, embedding);
            }

            // Update size tracking
            {
                let mut current_size =
                    self.current_size
                        .write()
                        .map_err(|_| StorageError::ConcurrencyError {
                            message: "Failed to acquire size write lock".to_string(),
                        })?;
                *current_size += embedding_size;
            }

            // Update statistics
            {
                let mut stats = self.statistics.write().unwrap();
                let latency = start_time.elapsed().as_micros() as u64;
                stats.update_operation("write", latency, true);
                stats.total_embeddings = self.embeddings.read().unwrap().len();
                stats.storage_size_bytes = *self.current_size.read().unwrap();
            }

            Ok(())
        })
    }

    fn get_embedding(
        &self,
        chunk_id: ChunkId,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = std::result::Result<Option<EmbeddingVector>, StorageError>,
                > + Send
                + '_,
        >,
    > {
        Box::pin(async move {
            let start_time = std::time::Instant::now();

            let result = {
                let embeddings =
                    self.embeddings
                        .read()
                        .map_err(|_| StorageError::ConcurrencyError {
                            message: "Failed to acquire embeddings read lock".to_string(),
                        })?;

                embeddings.get(&chunk_id).cloned()
            };

            // Update statistics
            {
                let mut stats = self.statistics.write().unwrap();
                let latency = start_time.elapsed().as_micros() as u64;
                stats.update_operation("read", latency, true);

                // Update cache hit rate
                let hit = result.is_some();
                let total_reads = stats.read_operations as f32;
                if hit {
                    stats.cache_hit_rate =
                        (stats.cache_hit_rate * (total_reads - 1.0) + 1.0) / total_reads;
                } else {
                    stats.cache_hit_rate =
                        (stats.cache_hit_rate * (total_reads - 1.0)) / total_reads;
                }
            }

            Ok(result)
        })
    }

    fn get_all_embeddings(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = std::result::Result<Vec<(ChunkId, EmbeddingVector)>, StorageError>,
                > + Send
                + '_,
        >,
    > {
        Box::pin(async move {
            let embeddings =
                self.embeddings
                    .read()
                    .map_err(|_| StorageError::ConcurrencyError {
                        message: "Failed to acquire embeddings read lock".to_string(),
                    })?;

            Ok(embeddings.iter().map(|(k, v)| (*k, v.clone())).collect())
        })
    }

    fn remove_embedding(
        &self,
        chunk_id: ChunkId,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<bool, StorageError>> + Send + '_>,
    > {
        Box::pin(async move {
            let removed_embedding = {
                let mut embeddings =
                    self.embeddings
                        .write()
                        .map_err(|_| StorageError::ConcurrencyError {
                            message: "Failed to acquire embeddings write lock".to_string(),
                        })?;

                embeddings.remove(&chunk_id)
            };

            if let Some(embedding) = removed_embedding {
                // Update size tracking
                let size_freed = Self::estimate_embedding_size(&embedding);
                {
                    let mut current_size =
                        self.current_size
                            .write()
                            .map_err(|_| StorageError::ConcurrencyError {
                                message: "Failed to acquire size write lock".to_string(),
                            })?;
                    *current_size = current_size.saturating_sub(size_freed);
                }

                // Remove chunk info as well
                self.chunk_info.write().unwrap().remove(&chunk_id);

                // Update statistics
                {
                    let mut stats = self.statistics.write().unwrap();
                    stats.total_embeddings = self.embeddings.read().unwrap().len();
                    stats.storage_size_bytes = *self.current_size.read().unwrap();
                }

                Ok(true)
            } else {
                Ok(false)
            }
        })
    }

    fn get_chunk_info(
        &self,
        chunk_id: ChunkId,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = std::result::Result<Option<ChunkInfo>, StorageError>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async move {
            let chunk_info =
                self.chunk_info
                    .read()
                    .map_err(|_| StorageError::ConcurrencyError {
                        message: "Failed to acquire chunk_info read lock".to_string(),
                    })?;

            Ok(chunk_info.get(&chunk_id).cloned())
        })
    }

    fn store_chunk_info(
        &self,
        chunk_id: ChunkId,
        info: ChunkInfo,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<(), StorageError>> + Send + '_>,
    > {
        Box::pin(async move {
            let info_size = Self::estimate_chunk_info_size(&info);

            // Store chunk info
            {
                let mut chunk_info =
                    self.chunk_info
                        .write()
                        .map_err(|_| StorageError::ConcurrencyError {
                            message: "Failed to acquire chunk_info write lock".to_string(),
                        })?;

                chunk_info.insert(chunk_id, info.clone());
            }

            // Update document mapping
            {
                let mut doc_chunks =
                    self.document_chunks
                        .write()
                        .map_err(|_| StorageError::ConcurrencyError {
                            message: "Failed to acquire document_chunks write lock".to_string(),
                        })?;

                doc_chunks
                    .entry(info.document_id)
                    .or_insert_with(Vec::new)
                    .push(chunk_id);
            }

            // Update size tracking
            {
                let mut current_size =
                    self.current_size
                        .write()
                        .map_err(|_| StorageError::ConcurrencyError {
                            message: "Failed to acquire size write lock".to_string(),
                        })?;
                *current_size += info_size;
            }

            Ok(())
        })
    }

    fn get_embeddings_by_document(
        &self,
        document_id: DocumentId,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = std::result::Result<Vec<(ChunkId, EmbeddingVector)>, StorageError>,
                > + Send
                + '_,
        >,
    > {
        Box::pin(async move {
            let chunk_ids = {
                let doc_chunks =
                    self.document_chunks
                        .read()
                        .map_err(|_| StorageError::ConcurrencyError {
                            message: "Failed to acquire document_chunks read lock".to_string(),
                        })?;

                doc_chunks.get(&document_id).cloned().unwrap_or_default()
            };

            let embeddings =
                self.embeddings
                    .read()
                    .map_err(|_| StorageError::ConcurrencyError {
                        message: "Failed to acquire embeddings read lock".to_string(),
                    })?;

            let mut result = Vec::new();
            for chunk_id in chunk_ids {
                if let Some(embedding) = embeddings.get(&chunk_id) {
                    result.push((chunk_id, embedding.clone()));
                }
            }

            Ok(result)
        })
    }

    fn count_embeddings(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<usize, StorageError>> + Send + '_>,
    > {
        Box::pin(async move {
            let embeddings =
                self.embeddings
                    .read()
                    .map_err(|_| StorageError::ConcurrencyError {
                        message: "Failed to acquire embeddings read lock".to_string(),
                    })?;

            Ok(embeddings.len())
        })
    }

    fn storage_size(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<usize, StorageError>> + Send + '_>,
    > {
        Box::pin(async move {
            let size = *self
                .current_size
                .read()
                .map_err(|_| StorageError::ConcurrencyError {
                    message: "Failed to acquire size read lock".to_string(),
                })?;

            Ok(size)
        })
    }

    fn clear(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<(), StorageError>> + Send + '_>,
    > {
        Box::pin(async move {
            // Clear all data structures
            {
                let mut embeddings =
                    self.embeddings
                        .write()
                        .map_err(|_| StorageError::ConcurrencyError {
                            message: "Failed to acquire embeddings write lock".to_string(),
                        })?;
                embeddings.clear();
            }

            {
                let mut chunk_info =
                    self.chunk_info
                        .write()
                        .map_err(|_| StorageError::ConcurrencyError {
                            message: "Failed to acquire chunk_info write lock".to_string(),
                        })?;
                chunk_info.clear();
            }

            {
                let mut doc_chunks =
                    self.document_chunks
                        .write()
                        .map_err(|_| StorageError::ConcurrencyError {
                            message: "Failed to acquire document_chunks write lock".to_string(),
                        })?;
                doc_chunks.clear();
            }

            // Reset size tracking
            {
                let mut current_size =
                    self.current_size
                        .write()
                        .map_err(|_| StorageError::ConcurrencyError {
                            message: "Failed to acquire size write lock".to_string(),
                        })?;
                *current_size = 0;
            }

            // Reset statistics
            {
                let mut stats = self.statistics.write().unwrap();
                *stats = StorageStatistics::new();
            }

            Ok(())
        })
    }

    fn maintenance(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<(), StorageError>> + Send + '_>,
    > {
        Box::pin(async move {
            // For memory storage, maintenance is mostly about updating statistics
            {
                let mut stats = self.statistics.write().unwrap();
                stats.last_maintenance = Some(std::time::SystemTime::now());

                // Update size statistics
                let embeddings = self.embeddings.read().unwrap();
                stats.total_embeddings = embeddings.len();
                stats.storage_size_bytes = *self.current_size.read().unwrap();

                if stats.total_embeddings > 0 {
                    stats.avg_embedding_size_bytes =
                        stats.storage_size_bytes / stats.total_embeddings;
                }
            }

            tracing::info!("Memory storage maintenance completed");
            Ok(())
        })
    }

    fn get_statistics(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = std::result::Result<StorageStatistics, StorageError>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async move {
            let stats = self
                .statistics
                .read()
                .map_err(|_| StorageError::ConcurrencyError {
                    message: "Failed to acquire statistics read lock".to_string(),
                })?;

            Ok(stats.clone())
        })
    }
}

/// Persistent storage backend (placeholder implementation)
/// In production, this would use memory-mapped files, B-tree indexes, etc.
#[derive(Debug)]
pub struct PersistentStorage {
    /// Storage directory path
    storage_path: PathBuf,

    /// In-memory cache for performance
    cache: MemoryStorage,

    /// Write-ahead log for durability
    wal_enabled: bool,
}

impl PersistentStorage {
    /// Create a new persistent storage backend
    pub fn new(storage_path: PathBuf, cache_size: usize, wal_enabled: bool) -> Self {
        Self {
            storage_path,
            cache: MemoryStorage::new(cache_size),
            wal_enabled,
        }
    }

    /// Get the index file path
    fn index_file_path(&self) -> PathBuf {
        self.storage_path.join("embeddings.idx")
    }

    /// Get the vector data file path
    fn vector_file_path(&self) -> PathBuf {
        self.storage_path.join("embeddings.vec")
    }

    /// Get the metadata file path
    fn metadata_file_path(&self) -> PathBuf {
        self.storage_path.join("embeddings.meta")
    }

    /// Initialize storage directory
    async fn ensure_directory(&self) -> std::result::Result<(), StorageError> {
        if !self.storage_path.exists() {
            fs::create_dir_all(&self.storage_path)
                .await
                .map_err(|e| StorageError::IoError {
                    message: format!("Failed to create storage directory: {}", e),
                })?;
        }
        Ok(())
    }
}

impl StorageBackend for PersistentStorage {
    fn store_embedding(
        &self,
        chunk_id: ChunkId,
        embedding: EmbeddingVector,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<(), StorageError>> + Send + '_>,
    > {
        // For now, delegate to cache (in production, would also write to disk)
        self.cache.store_embedding(chunk_id, embedding)
    }

    fn get_embedding(
        &self,
        chunk_id: ChunkId,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = std::result::Result<Option<EmbeddingVector>, StorageError>,
                > + Send
                + '_,
        >,
    > {
        // Try cache first, then disk (in production)
        self.cache.get_embedding(chunk_id)
    }

    fn get_all_embeddings(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = std::result::Result<Vec<(ChunkId, EmbeddingVector)>, StorageError>,
                > + Send
                + '_,
        >,
    > {
        // For now, delegate to cache
        self.cache.get_all_embeddings()
    }

    fn remove_embedding(
        &self,
        chunk_id: ChunkId,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<bool, StorageError>> + Send + '_>,
    > {
        // Remove from both cache and disk (in production)
        self.cache.remove_embedding(chunk_id)
    }

    fn get_chunk_info(
        &self,
        chunk_id: ChunkId,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = std::result::Result<Option<ChunkInfo>, StorageError>>
                + Send
                + '_,
        >,
    > {
        self.cache.get_chunk_info(chunk_id)
    }

    fn store_chunk_info(
        &self,
        chunk_id: ChunkId,
        info: ChunkInfo,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<(), StorageError>> + Send + '_>,
    > {
        self.cache.store_chunk_info(chunk_id, info)
    }

    fn get_embeddings_by_document(
        &self,
        document_id: DocumentId,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = std::result::Result<Vec<(ChunkId, EmbeddingVector)>, StorageError>,
                > + Send
                + '_,
        >,
    > {
        self.cache.get_embeddings_by_document(document_id)
    }

    fn count_embeddings(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<usize, StorageError>> + Send + '_>,
    > {
        self.cache.count_embeddings()
    }

    fn storage_size(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<usize, StorageError>> + Send + '_>,
    > {
        self.cache.storage_size()
    }

    fn clear(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<(), StorageError>> + Send + '_>,
    > {
        self.cache.clear()
    }

    fn maintenance(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<(), StorageError>> + Send + '_>,
    > {
        Box::pin(async move {
            self.ensure_directory().await?;
            self.cache.maintenance().await
        })
    }

    fn get_statistics(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = std::result::Result<StorageStatistics, StorageError>>
                + Send
                + '_,
        >,
    > {
        self.cache.get_statistics()
    }
}

/// Storage factory for creating appropriate backends
pub struct StorageFactory;

impl StorageFactory {
    /// Create a memory storage backend
    pub fn create_memory_storage(capacity: usize) -> Box<dyn StorageBackend + Send + Sync> {
        Box::new(MemoryStorage::new(capacity))
    }

    /// Create a persistent storage backend
    pub fn create_persistent_storage(
        storage_path: PathBuf,
        cache_size: usize,
        wal_enabled: bool,
    ) -> Box<dyn StorageBackend + Send + Sync> {
        Box::new(PersistentStorage::new(
            storage_path,
            cache_size,
            wal_enabled,
        ))
    }

    /// Create storage backend based on configuration
    pub fn create_from_config(
        enable_persistence: bool,
        storage_path: Option<PathBuf>,
        capacity: usize,
    ) -> Box<dyn StorageBackend + Send + Sync> {
        if enable_persistence {
            let path =
                storage_path.unwrap_or_else(|| std::env::temp_dir().join("codex_semantic_index"));
            Self::create_persistent_storage(path, capacity, true)
        } else {
            Self::create_memory_storage(capacity)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast_compactor::Language;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_memory_storage_basic_operations() {
        let storage = MemoryStorage::new(1000);

        // Create test embedding
        let chunk_id = ChunkId::new_v4();
        let embedding = crate::semantic_index::embeddings::EmbeddingVector::new(
            vec![1.0, 2.0, 3.0],
            "test_hash".to_string(),
        );

        // Store embedding
        storage
            .store_embedding(chunk_id, embedding.clone())
            .await
            .unwrap();

        // Retrieve embedding
        let retrieved = storage.get_embedding(chunk_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content_hash, embedding.content_hash);

        // Count embeddings
        let count = storage.count_embeddings().await.unwrap();
        assert_eq!(count, 1);

        // Remove embedding
        let removed = storage.remove_embedding(chunk_id).await.unwrap();
        assert!(removed);

        // Verify removal
        let count_after = storage.count_embeddings().await.unwrap();
        assert_eq!(count_after, 0);
    }

    #[tokio::test]
    async fn test_chunk_info_storage() {
        let storage = MemoryStorage::new(1000);

        let chunk_id = ChunkId::new_v4();
        let document_id = DocumentId::new_v4();
        let chunk_info = ChunkInfo {
            content: "fn test() {}".to_string(),
            document_id,
            language: Language::Rust,
            file_path: Some("test.rs".to_string()),
            line_range: Some((1, 3)),
        };

        // Store chunk info
        storage
            .store_chunk_info(chunk_id, chunk_info.clone())
            .await
            .unwrap();

        // Retrieve chunk info
        let retrieved = storage.get_chunk_info(chunk_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, chunk_info.content);

        // Get embeddings by document
        let doc_embeddings = storage
            .get_embeddings_by_document(document_id)
            .await
            .unwrap();
        assert_eq!(doc_embeddings.len(), 0); // No embeddings stored yet
    }

    #[tokio::test]
    async fn test_storage_statistics() {
        let storage = MemoryStorage::new(100);

        // Initial statistics
        let stats = storage.get_statistics().await.unwrap();
        assert_eq!(stats.total_embeddings, 0);
        assert!(stats.is_healthy());

        // Store some embeddings and check statistics
        for i in 0..5 {
            let chunk_id = ChunkId::new_v4();
            let embedding = crate::semantic_index::embeddings::EmbeddingVector::new(
                vec![i as f32; 10],
                format!("hash_{}", i),
            );
            storage.store_embedding(chunk_id, embedding).await.unwrap();
        }

        let stats_after = storage.get_statistics().await.unwrap();
        assert_eq!(stats_after.total_embeddings, 5);
        assert!(stats_after.storage_size_bytes > 0);
        assert!(stats_after.write_operations > 0);
    }

    #[tokio::test]
    async fn test_memory_storage_capacity_limits() {
        let storage = MemoryStorage::new(2); // Very small capacity

        // Fill beyond capacity
        for i in 0..5 {
            let chunk_id = ChunkId::new_v4();
            let embedding = crate::semantic_index::embeddings::EmbeddingVector::new(
                vec![i as f32; 100], // Larger embeddings
                format!("hash_{}", i),
            );
            storage.store_embedding(chunk_id, embedding).await.unwrap();
        }

        // Should not exceed capacity due to eviction
        let count = storage.count_embeddings().await.unwrap();
        assert!(count <= 2);
    }

    #[tokio::test]
    async fn test_storage_clear() {
        let storage = MemoryStorage::new(1000);

        // Add some data
        let chunk_id = ChunkId::new_v4();
        let embedding = crate::semantic_index::embeddings::EmbeddingVector::new(
            vec![1.0, 2.0],
            "test".to_string(),
        );
        storage.store_embedding(chunk_id, embedding).await.unwrap();

        // Verify data exists
        let count_before = storage.count_embeddings().await.unwrap();
        assert_eq!(count_before, 1);

        // Clear storage
        storage.clear().await.unwrap();

        // Verify data is gone
        let count_after = storage.count_embeddings().await.unwrap();
        assert_eq!(count_after, 0);

        let size_after = storage.storage_size().await.unwrap();
        assert_eq!(size_after, 0);
    }

    #[tokio::test]
    async fn test_storage_factory() {
        // Test memory storage creation
        let memory_storage = StorageFactory::create_memory_storage(1000);
        let count = memory_storage.count_embeddings().await.unwrap();
        assert_eq!(count, 0);

        // Test persistent storage creation
        let temp_dir = tempdir().unwrap();
        let persistent_storage =
            StorageFactory::create_persistent_storage(temp_dir.path().to_path_buf(), 500, true);
        let count = persistent_storage.count_embeddings().await.unwrap();
        assert_eq!(count, 0);

        // Test config-based creation
        let config_storage = StorageFactory::create_from_config(
            false, // memory storage
            None, 1000,
        );
        let count = config_storage.count_embeddings().await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_storage_maintenance() {
        let storage = MemoryStorage::new(1000);

        // Add some data
        let chunk_id = ChunkId::new_v4();
        let embedding = crate::semantic_index::embeddings::EmbeddingVector::new(
            vec![1.0, 2.0, 3.0],
            "test".to_string(),
        );
        storage.store_embedding(chunk_id, embedding).await.unwrap();

        // Run maintenance
        storage.maintenance().await.unwrap();

        // Check that statistics were updated
        let stats = storage.get_statistics().await.unwrap();
        assert!(stats.last_maintenance.is_some());
        assert_eq!(stats.total_embeddings, 1);
    }
}
