//! Comprehensive Semantic Indexing System for codex-rs
//!
//! This module provides a production-ready semantic indexing system that transforms
//! code into searchable vector embeddings with sub-100ms retrieval latency.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  Semantic Index System                      │
//! ├─────────────────────────────────────────────────────────────┤
//! │ IndexManager                                                │
//! │   ├── VectorStore (Arc<RwLock<>>)                          │
//! │   ├── EmbeddingCache (Arc<Mutex<LRUCache>>)                │
//! │   ├── StorageBackend (async trait)                         │
//! │   └── RetrievalEngine (concurrent queries)                 │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Performance Targets
//!
//! - **Indexing**: 1000+ files/minute with AST compaction
//! - **Retrieval**: <100ms p99 latency for 100K+ documents  
//! - **Memory**: <100MB for 10K indexed documents
//! - **Throughput**: 50+ concurrent queries/second
//!
//! # Concurrency Model
//!
//! ```text
//! IndexingWorker (background thread)
//!     ↓ Arc<Semaphore> (rate limiting)
//! EmbeddingWorker[] (parallel tokio tasks)
//!     ↓ Arc<RwLock<VectorStore>>
//! StorageWorker (async I/O)
//!     ↓ happens-before relationship
//! RetrievalWorker[] (concurrent queries)
//! ```
//!
//! # Memory Safety & Ownership
//!
//! - `IndexManager` owns `VectorStore` with `Arc<RwLock<>>` for concurrent access
//! - `EmbeddingCache` uses `Arc<Mutex<LRU>>` for thread-safe caching
//! - Zero-copy string handling with `Cow<str>` from AST compactor
//! - RAII cleanup with `Drop` implementations for resource management
//!
//! # Integration Points
//!
//! - **AST Compactor**: Uses `ast_compactor::AstCompactor` for code compaction
//! - **Tree-sitter**: Integrates with `parsers` module for syntax analysis
//! - **File Discovery**: Leverages `code_tools::fd_find` for file enumeration
//! - **Error Handling**: Built on `error::CodexErr` with recovery strategies
//! - **Type Safety**: Uses `types::FilePath` and validation patterns

pub mod embeddings;
pub mod indexer;
pub mod retrieval;
pub mod storage;

#[cfg(test)]
mod tests;

// Re-export core types for convenience
pub use embeddings::CodeChunk;
pub use embeddings::EmbeddingEngine;
pub use embeddings::EmbeddingVector;
pub use indexer::IndexingOptions;
pub use indexer::SemanticIndexer;
pub use retrieval::RelevanceScore;
pub use retrieval::RetrievalEngine;
pub use retrieval::SearchQuery;
pub use retrieval::SearchResult;
pub use storage::MemoryStorage;
pub use storage::PersistentStorage;
pub use storage::StorageBackend;

// SemanticIndexConfig is defined below and is already public

// Type aliases for common patterns
pub type Result<T> = std::result::Result<T, SemanticIndexError>;
pub type DocumentId = uuid::Uuid;
pub type ChunkId = uuid::Uuid;
pub type Timestamp = std::time::SystemTime;

// Core error types for semantic indexing

use crate::error::CodexErr;
use thiserror::Error;

/// Comprehensive error types for semantic indexing operations
#[derive(Error, Debug, Clone)]
pub enum SemanticIndexError {
    /// AST compaction failed during indexing
    #[error("AST compaction failed for {file}: {message}")]
    CompactionFailed { file: String, message: String },

    /// Embedding generation failed
    #[error("Embedding generation failed for chunk {chunk_id}: {reason}")]
    EmbeddingFailed { chunk_id: ChunkId, reason: String },

    /// Vector storage operation failed
    #[error("Storage operation failed: {operation} - {message}")]
    StorageFailed { operation: String, message: String },

    /// Index corruption detected
    #[error("Index corruption detected: {details}")]
    IndexCorrupted { details: String },

    /// Query parsing or execution failed
    #[error("Query failed: {query} - {reason}")]
    QueryFailed { query: String, reason: String },

    /// Resource limits exceeded
    #[error("Resource limit exceeded: {resource} ({current} > {limit})")]
    ResourceLimitExceeded {
        resource: String,
        current: usize,
        limit: usize,
    },

    /// Cache operation failed
    #[error("Cache operation failed: {operation} - {message}")]
    CacheFailed { operation: String, message: String },

    /// Concurrency or locking error
    #[error("Concurrency error: {context} - {message}")]
    ConcurrencyError { context: String, message: String },

    /// File system operation failed
    #[error("File system error: {path} - {message}")]
    FileSystemError { path: String, message: String },

    /// Serialization/deserialization failed
    #[error("Serialization error: {format} - {message}")]
    SerializationError { format: String, message: String },
}

/// Convert semantic index errors to CodexErr for unified error handling
impl From<SemanticIndexError> for CodexErr {
    fn from(err: SemanticIndexError) -> Self {
        match err {
            SemanticIndexError::CompactionFailed { file, message } => {
                CodexErr::semantic_index_error(
                    "AST compaction",
                    format!("Failed to compact {}: {}", file, message),
                )
            }
            SemanticIndexError::EmbeddingFailed { chunk_id, reason } => {
                CodexErr::semantic_index_error(
                    "embedding generation",
                    format!("Failed to generate embedding for {}: {}", chunk_id, reason),
                )
            }
            SemanticIndexError::StorageFailed { operation, message } => {
                CodexErr::semantic_index_error(&operation, message)
            }
            SemanticIndexError::IndexCorrupted { details } => CodexErr::semantic_index_error(
                "index validation",
                format!("Index corruption: {}", details),
            ),
            SemanticIndexError::QueryFailed { query, reason } => CodexErr::semantic_index_error(
                "query execution",
                format!("Query '{}' failed: {}", query, reason),
            ),
            SemanticIndexError::ResourceLimitExceeded {
                resource,
                current,
                limit,
            } => CodexErr::MemoryLimitExceeded { current, limit },
            SemanticIndexError::CacheFailed { operation, message } => {
                CodexErr::semantic_index_error(format!("cache {}", operation), message)
            }
            SemanticIndexError::ConcurrencyError { context, message } => {
                CodexErr::semantic_index_error(&context, format!("Concurrency issue: {}", message))
            }
            SemanticIndexError::FileSystemError { path, message } => {
                CodexErr::semantic_index_error(
                    "file system",
                    format!("File system error for {}: {}", path, message),
                )
            }
            SemanticIndexError::SerializationError { format, message } => {
                CodexErr::semantic_index_error(
                    "serialization",
                    format!("{} serialization failed: {}", format, message),
                )
            }
        }
    }
}

/// Configuration for semantic indexing system
#[derive(Debug, Clone)]
pub struct SemanticIndexConfig {
    /// Maximum number of documents to keep in memory
    pub max_documents: usize,

    /// Maximum size of embedding cache
    pub cache_size: usize,

    /// Embedding vector dimensions (simulated)
    pub embedding_dimensions: usize,

    /// Maximum chunk size for code segments
    pub max_chunk_size: usize,

    /// Minimum chunk overlap for better context
    pub chunk_overlap: usize,

    /// Maximum number of concurrent indexing tasks
    pub max_concurrent_indexing: usize,

    /// Maximum number of concurrent retrieval queries
    pub max_concurrent_queries: usize,

    /// Storage persistence enabled
    pub enable_persistence: bool,

    /// Storage file path for persistent indexes
    pub storage_path: Option<std::path::PathBuf>,

    /// Similarity threshold for relevance filtering
    pub similarity_threshold: f32,

    /// Maximum results to return per query
    pub max_results: usize,
}

impl Default for SemanticIndexConfig {
    fn default() -> Self {
        Self {
            max_documents: 100_000,
            cache_size: 1_000,
            embedding_dimensions: 768, // Standard for many embedding models
            max_chunk_size: 2048,      // ~500 tokens
            chunk_overlap: 256,        // 25% overlap
            max_concurrent_indexing: 4,
            max_concurrent_queries: 16,
            enable_persistence: true,
            storage_path: None, // Will use system temp dir
            similarity_threshold: 0.7,
            max_results: 50,
        }
    }
}

/// Performance metrics for monitoring and optimization
#[derive(Debug, Clone, Default)]
pub struct IndexingMetrics {
    /// Total documents indexed
    pub documents_indexed: usize,

    /// Total chunks created
    pub chunks_created: usize,

    /// Total embedding vectors generated
    pub embeddings_generated: usize,

    /// Cache hit rate (0.0 to 1.0)
    pub cache_hit_rate: f32,

    /// Average indexing time per document (ms)
    pub avg_indexing_time_ms: f32,

    /// Average retrieval time (ms)
    pub avg_retrieval_time_ms: f32,

    /// Total memory usage (bytes)
    pub memory_usage_bytes: usize,

    /// Number of storage I/O operations
    pub storage_operations: usize,

    /// Error count by type
    pub error_counts: std::collections::HashMap<String, usize>,
}

impl IndexingMetrics {
    /// Create new empty metrics
    pub fn new() -> Self {
        Self::default()
    }

    /// Update cache hit rate
    pub fn update_cache_hit_rate(&mut self, hits: usize, total: usize) {
        if total > 0 {
            self.cache_hit_rate = hits as f32 / total as f32;
        }
    }

    /// Record an error
    pub fn record_error(&mut self, error_type: &str) {
        *self.error_counts.entry(error_type.to_string()).or_insert(0) += 1;
    }

    /// Calculate efficiency score (0.0 to 1.0)
    pub fn efficiency_score(&self) -> f32 {
        let mut score = 0.0;
        let mut factors = 0;

        // Cache efficiency
        if self.cache_hit_rate > 0.0 {
            score += self.cache_hit_rate;
            factors += 1;
        }

        // Speed efficiency (inverse of time)
        if self.avg_retrieval_time_ms > 0.0 && self.avg_retrieval_time_ms < 1000.0 {
            score += (1000.0 - self.avg_retrieval_time_ms) / 1000.0;
            factors += 1;
        }

        // Error rate (inverse)
        let total_errors: usize = self.error_counts.values().sum();
        let total_operations = self.documents_indexed + self.storage_operations;
        if total_operations > 0 {
            let error_rate = total_errors as f32 / total_operations as f32;
            score += (1.0 - error_rate).max(0.0);
            factors += 1;
        }

        if factors > 0 {
            score / factors as f32
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod inline_tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SemanticIndexConfig::default();
        assert_eq!(config.max_documents, 100_000);
        assert_eq!(config.embedding_dimensions, 768);
        assert!(config.enable_persistence);
    }

    #[test]
    fn test_metrics_efficiency_score() {
        let mut metrics = IndexingMetrics::new();
        metrics.cache_hit_rate = 0.8;
        metrics.avg_retrieval_time_ms = 50.0;
        metrics.documents_indexed = 100;
        metrics.storage_operations = 10;
        // No errors recorded

        let score = metrics.efficiency_score();
        assert!(score > 0.8); // Should be high efficiency
    }

    #[test]
    fn test_error_conversion() {
        let index_err = SemanticIndexError::EmbeddingFailed {
            chunk_id: ChunkId::new_v4(),
            reason: "Model unavailable".to_string(),
        };

        let codex_err: CodexErr = index_err.into();
        assert_eq!(
            codex_err.error_code(),
            crate::error::ErrorCode::SemanticIndexQueryFailed
        );
    }

    #[test]
    fn test_metrics_error_recording() {
        let mut metrics = IndexingMetrics::new();
        metrics.record_error("embedding_failed");
        metrics.record_error("embedding_failed");
        metrics.record_error("storage_failed");

        assert_eq!(metrics.error_counts.get("embedding_failed"), Some(&2));
        assert_eq!(metrics.error_counts.get("storage_failed"), Some(&1));
    }
}
