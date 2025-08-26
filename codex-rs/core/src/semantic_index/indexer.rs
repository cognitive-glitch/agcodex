//! Core Semantic Indexing Engine
//!
//! This module provides the main orchestration layer for semantic indexing,
//! integrating AST compaction, embedding generation, and efficient storage.
//!
//! # Concurrency Architecture
//!
//! ```text
//! SemanticIndexer (main coordinator)
//!     ├── IndexingWorker (background thread)
//!     │   ├── AST Compaction (sync)
//!     │   ├── Chunk Extraction (sync)
//!     │   └── Embedding Generation (async batch)
//!     ├── StorageWorker (async I/O thread)
//!     └── CacheManager (Arc<Mutex<LRU>>)
//! ```
//!
//! # Memory Management
//!
//! - **Zero-copy**: Uses `Cow<str>` from AST compactor
//! - **Reference counting**: `Arc<RwLock<VectorStore>>` for concurrent access
//! - **Cache eviction**: LRU policy with configurable size limits
//! - **Batch processing**: Groups operations to reduce allocation overhead

use crate::ast_compactor::AstCompactor;
use crate::ast_compactor::CompactionOptions;
use crate::ast_compactor::Language;
use crate::code_tools::fd_find::FdFind;

// use crate::parsers::utils::detect_language_from_path; // TODO: Implement when available
use crate::types::FilePath;

use super::ChunkId;
use super::DocumentId;
use super::IndexingMetrics;
use super::Result;
use super::SemanticIndexConfig;
use super::SemanticIndexError;
use super::embeddings::CodeChunk;
use super::embeddings::EmbeddingEngine;
use super::embeddings::EmbeddingVector;
use super::retrieval::RetrievalEngine;
use super::retrieval::SearchQuery;
use super::retrieval::SearchResult;
use super::storage::MemoryStorage;
use super::storage::StorageBackend;

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::time::Instant;
use std::time::SystemTime;
use tokio::sync::Semaphore;

use serde::Deserialize;
use serde::Serialize;

/// Indexing options for fine-grained control
#[derive(Debug, Clone)]
pub struct IndexingOptions {
    /// Languages to include in indexing
    pub languages: Vec<Language>,

    /// File patterns to include (glob-style)
    pub include_patterns: Vec<String>,

    /// File patterns to exclude
    pub exclude_patterns: Vec<String>,

    /// Maximum file size to process (bytes)
    pub max_file_size: usize,

    /// Enable incremental indexing
    pub incremental: bool,

    /// Force reindexing of existing files
    pub force_reindex: bool,

    /// Enable AST compaction before embedding
    pub enable_compaction: bool,

    /// Number of parallel indexing workers
    pub parallel_workers: usize,
}

impl Default for IndexingOptions {
    fn default() -> Self {
        Self {
            languages: vec![
                Language::Rust,
                Language::Python,
                Language::TypeScript,
                Language::JavaScript,
                Language::Go,
            ],
            include_patterns: vec![
                "**/*.rs".to_string(),
                "**/*.py".to_string(),
                "**/*.ts".to_string(),
                "**/*.tsx".to_string(),
                "**/*.js".to_string(),
                "**/*.jsx".to_string(),
                "**/*.go".to_string(),
            ],
            exclude_patterns: vec![
                "**/target/**".to_string(),
                "**/node_modules/**".to_string(),
                "**/.git/**".to_string(),
                "**/.*".to_string(),
            ],
            max_file_size: 1024 * 1024, // 1MB
            incremental: true,
            force_reindex: false,
            enable_compaction: true,
            parallel_workers: 4,
        }
    }
}

/// Document metadata for indexed files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedDocument {
    pub id: DocumentId,
    pub file_path: PathBuf,
    pub language: Language,
    pub file_size: u64,
    pub last_modified: SystemTime,
    pub indexed_at: SystemTime,
    pub chunk_ids: Vec<ChunkId>,
    pub compacted_size: Option<usize>,
    pub checksum: Option<String>, // For incremental updates
}

/// Chunk metadata linking chunks to documents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub chunk_id: ChunkId,
    pub document_id: DocumentId,
    pub start_line: usize,
    pub end_line: usize,
    pub chunk_type: ChunkType,
    pub estimated_tokens: usize,
}

/// Types of code chunks for better organization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChunkType {
    Function,
    Struct,
    Enum,
    Trait,
    Module,
    Comment,
    Unknown,
}

/// Main semantic indexer coordinating all operations
pub struct SemanticIndexer {
    config: SemanticIndexConfig,
    ast_compactor: Arc<Mutex<AstCompactor>>,
    embedding_engine: Arc<EmbeddingEngine>,
    storage: Arc<dyn StorageBackend + Send + Sync>,
    retrieval_engine: Arc<RetrievalEngine>,
    documents: Arc<RwLock<HashMap<DocumentId, IndexedDocument>>>,
    chunks: Arc<RwLock<HashMap<ChunkId, ChunkMetadata>>>,
    indexing_semaphore: Arc<Semaphore>,
    query_semaphore: Arc<Semaphore>,
    metrics: Arc<Mutex<IndexingMetrics>>,
}

impl std::fmt::Debug for SemanticIndexer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SemanticIndexer")
            .field("config", &self.config)
            .field("storage", &"<dyn StorageBackend>")
            .field("documents_count", &self.documents.read().unwrap().len())
            .field("chunks_count", &self.chunks.read().unwrap().len())
            .finish()
    }
}

impl SemanticIndexer {
    /// Create a new semantic indexer with configuration
    pub fn new(config: SemanticIndexConfig) -> Result<Self> {
        let ast_compactor = Arc::new(Mutex::new(AstCompactor::new()));
        let embedding_engine = Arc::new(EmbeddingEngine::new(config.embedding_dimensions));

        let storage: Arc<dyn StorageBackend + Send + Sync> = if config.enable_persistence {
            // TODO: Implement PersistentStorage when needed
            Arc::new(MemoryStorage::new(config.max_documents))
        } else {
            Arc::new(MemoryStorage::new(config.max_documents))
        };

        let retrieval_engine = Arc::new(RetrievalEngine::new(
            config.similarity_threshold,
            config.max_results,
        ));

        let indexing_semaphore = Arc::new(Semaphore::new(config.max_concurrent_indexing));
        let query_semaphore = Arc::new(Semaphore::new(config.max_concurrent_queries));

        Ok(Self {
            config,
            ast_compactor,
            embedding_engine,
            storage,
            retrieval_engine,
            documents: Arc::new(RwLock::new(HashMap::new())),
            chunks: Arc::new(RwLock::new(HashMap::new())),
            indexing_semaphore,
            query_semaphore,
            metrics: Arc::new(Mutex::new(IndexingMetrics::new())),
        })
    }

    /// Index a single file with progress tracking
    pub async fn index_file<P: AsRef<Path>>(&self, file_path: P) -> Result<DocumentId> {
        let _permit = self.indexing_semaphore.acquire().await.map_err(|e| {
            SemanticIndexError::ConcurrencyError {
                context: "acquiring indexing semaphore".to_string(),
                message: e.to_string(),
            }
        })?;

        let start_time = Instant::now();
        let file_path = file_path.as_ref();

        // Validate file path
        let validated_path = FilePath::try_from(file_path.to_path_buf()).map_err(|e| {
            SemanticIndexError::FileSystemError {
                path: file_path.display().to_string(),
                message: format!("Invalid file path: {}", e),
            }
        })?;

        // Check file size limits
        let metadata = tokio::fs::metadata(&file_path).await.map_err(|e| {
            SemanticIndexError::FileSystemError {
                path: file_path.display().to_string(),
                message: format!("Cannot read file metadata: {}", e),
            }
        })?;

        if metadata.len() > self.config.max_chunk_size as u64 * 10 {
            return Err(SemanticIndexError::ResourceLimitExceeded {
                resource: "file_size".to_string(),
                current: metadata.len() as usize,
                limit: self.config.max_chunk_size * 10,
            });
        }

        // Detect language from file extension
        let language = match file_path.extension().and_then(|ext| ext.to_str()) {
            Some("rs") => Language::Rust,
            Some("py") => Language::Python,
            Some("ts") | Some("tsx") => Language::TypeScript,
            Some("js") | Some("jsx") => Language::JavaScript,
            Some("go") => Language::Go,
            _ => Language::Unknown,
        };

        // Read file content
        let content = tokio::fs::read_to_string(&file_path).await.map_err(|e| {
            SemanticIndexError::FileSystemError {
                path: file_path.display().to_string(),
                message: format!("Cannot read file: {}", e),
            }
        })?;

        // Create document record
        let document_id = DocumentId::new_v4();
        let document = IndexedDocument {
            id: document_id,
            file_path: file_path.to_path_buf(),
            language,
            file_size: metadata.len(),
            last_modified: metadata.modified().unwrap_or(SystemTime::now()),
            indexed_at: SystemTime::now(),
            chunk_ids: Vec::new(),
            compacted_size: None,
            checksum: None, // TODO: Implement checksums for incremental updates
        };

        // Process content (AST compaction + chunking)
        let chunks = self
            .process_content(&content, &language, document_id)
            .await?;

        // Generate embeddings for all chunks
        let embeddings = self.generate_embeddings(&chunks).await?;

        // Store embeddings and metadata
        for (chunk, embedding) in chunks.into_iter().zip(embeddings) {
            self.storage
                .store_embedding(chunk.id, embedding)
                .await
                .map_err(|e| SemanticIndexError::StorageFailed {
                    operation: "store_embedding".to_string(),
                    message: e.to_string(),
                })?;

            // Store chunk metadata
            let metadata = ChunkMetadata {
                chunk_id: chunk.id,
                document_id,
                start_line: chunk.start_line.unwrap_or(0),
                end_line: chunk.end_line.unwrap_or(0),
                chunk_type: classify_chunk_type(&chunk.content),
                estimated_tokens: chunk.content.len() / 4, // Rough estimate
            };

            self.chunks.write().unwrap().insert(chunk.id, metadata);
        }

        // Update document with chunk IDs
        let mut doc = document;
        doc.chunk_ids = self
            .chunks
            .read()
            .unwrap()
            .values()
            .filter(|meta| meta.document_id == document_id)
            .map(|meta| meta.chunk_id)
            .collect();

        // Cache chunk count before moving doc
        let chunk_count = doc.chunk_ids.len();

        // Store document metadata
        self.documents.write().unwrap().insert(document_id, doc);

        // Update metrics
        let indexing_time = start_time.elapsed().as_millis() as f32;
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.documents_indexed += 1;
            metrics.chunks_created += chunk_count;
            metrics.embeddings_generated += chunk_count;

            // Update average indexing time
            let total_docs = metrics.documents_indexed as f32;
            metrics.avg_indexing_time_ms =
                (metrics.avg_indexing_time_ms * (total_docs - 1.0) + indexing_time) / total_docs;
        }

        Ok(document_id)
    }

    /// Index multiple files in a directory with parallel processing
    pub async fn index_directory<P: AsRef<Path>>(
        &self,
        directory: P,
        options: &IndexingOptions,
    ) -> Result<Vec<DocumentId>> {
        let directory = directory.as_ref();

        // Discover files using fd_find
        let fd_find = FdFind::new();
        let mut all_files = Vec::new();

        for pattern in &options.include_patterns {
            let files = fd_find
                .find_files(
                    directory.to_string_lossy().as_ref(),
                    &[pattern.as_str()],
                    &options
                        .exclude_patterns
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>(),
                )
                .map_err(|e| SemanticIndexError::FileSystemError {
                    path: directory.display().to_string(),
                    message: format!("File discovery failed: {}", e),
                })?;

            all_files.extend(files);
        }

        // Remove duplicates and filter by file size
        all_files.sort_by(|a, b| a.path.cmp(&b.path));
        all_files.dedup_by(|a, b| a.path == b.path);

        let mut valid_files = Vec::new();
        for file_path in all_files {
            if let Ok(metadata) = tokio::fs::metadata(&file_path.path).await
                && metadata.len() <= options.max_file_size as u64 {
                    valid_files.push(file_path.path);
                }
        }

        tracing::info!(
            "Indexing {} files in directory: {}",
            valid_files.len(),
            directory.display()
        );

        // Process files in parallel batches
        let chunk_size = options.parallel_workers;
        let mut document_ids = Vec::new();

        for chunk in valid_files.chunks(chunk_size) {
            let mut handles = Vec::new();

            for file_path in chunk {
                let indexer = self.clone();
                let file_path = file_path.clone();

                let handle = tokio::spawn(async move { indexer.index_file(&file_path).await });

                handles.push(handle);
            }

            // Wait for batch completion
            for handle in handles {
                match handle.await {
                    Ok(Ok(doc_id)) => document_ids.push(doc_id),
                    Ok(Err(e)) => {
                        tracing::warn!("Failed to index file: {}", e);
                        self.metrics.lock().unwrap().record_error("indexing_failed");
                    }
                    Err(e) => {
                        tracing::warn!("Task join error: {}", e);
                        self.metrics.lock().unwrap().record_error("task_failed");
                    }
                }
            }
        }

        tracing::info!(
            "Successfully indexed {} out of {} files",
            document_ids.len(),
            valid_files.len()
        );

        Ok(document_ids)
    }

    /// Search the indexed content with semantic similarity
    pub async fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>> {
        let _permit = self.query_semaphore.acquire().await.map_err(|e| {
            SemanticIndexError::ConcurrencyError {
                context: "acquiring query semaphore".to_string(),
                message: e.to_string(),
            }
        })?;

        let start_time = Instant::now();

        // Generate query embedding
        let query_embedding = self
            .embedding_engine
            .generate_query_embedding(&query.text)
            .await
            .map_err(|e| SemanticIndexError::EmbeddingFailed {
                chunk_id: ChunkId::new_v4(),
                reason: format!("Query embedding failed: {}", e),
            })?;

        // Perform retrieval
        let results = self
            .retrieval_engine
            .search(&query_embedding, &query, self.storage.as_ref())
            .await
            .map_err(|e| SemanticIndexError::QueryFailed {
                query: query.text.clone(),
                reason: e.to_string(),
            })?;

        // Update metrics
        let query_time = start_time.elapsed().as_millis() as f32;
        {
            let mut metrics = self.metrics.lock().unwrap();
            let total_queries = metrics.storage_operations + 1;
            metrics.avg_retrieval_time_ms =
                (metrics.avg_retrieval_time_ms * (total_queries - 1) as f32 + query_time)
                    / total_queries as f32;
            metrics.storage_operations = total_queries;
        }

        Ok(results)
    }

    /// Get indexing metrics and performance statistics
    pub fn get_metrics(&self) -> IndexingMetrics {
        self.metrics.lock().unwrap().clone()
    }

    /// Get information about a specific document
    pub fn get_document_info(&self, document_id: DocumentId) -> Option<IndexedDocument> {
        self.documents.read().unwrap().get(&document_id).cloned()
    }

    /// List all indexed documents with filtering
    pub fn list_documents(&self, language_filter: Option<Language>) -> Vec<IndexedDocument> {
        self.documents
            .read()
            .unwrap()
            .values()
            .filter(|doc| language_filter.is_none_or(|lang| doc.language == lang))
            .cloned()
            .collect()
    }

    /// Remove a document from the index
    pub async fn remove_document(&self, document_id: DocumentId) -> Result<bool> {
        let document = { self.documents.write().unwrap().remove(&document_id) };

        if let Some(doc) = document {
            // Remove all associated chunks
            for chunk_id in &doc.chunk_ids {
                self.storage
                    .remove_embedding(*chunk_id)
                    .await
                    .map_err(|e| SemanticIndexError::StorageFailed {
                        operation: "remove_embedding".to_string(),
                        message: e.to_string(),
                    })?;

                self.chunks.write().unwrap().remove(chunk_id);
            }

            // Update metrics
            {
                let mut metrics = self.metrics.lock().unwrap();
                metrics.documents_indexed = metrics.documents_indexed.saturating_sub(1);
                metrics.chunks_created = metrics.chunks_created.saturating_sub(doc.chunk_ids.len());
                metrics.embeddings_generated = metrics
                    .embeddings_generated
                    .saturating_sub(doc.chunk_ids.len());
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Clear the entire index
    pub async fn clear_index(&self) -> Result<()> {
        // Clear all data structures
        self.documents.write().unwrap().clear();
        self.chunks.write().unwrap().clear();

        // Clear storage
        self.storage
            .clear()
            .await
            .map_err(|e| SemanticIndexError::StorageFailed {
                operation: "clear".to_string(),
                message: e.to_string(),
            })?;

        // Reset metrics
        *self.metrics.lock().unwrap() = IndexingMetrics::new();

        Ok(())
    }

    /// Process file content into chunks with AST compaction
    async fn process_content(
        &self,
        content: &str,
        language: &Language,
        document_id: DocumentId,
    ) -> Result<Vec<CodeChunk>> {
        let chunks = if language != &Language::Unknown {
            // Use AST compaction for supported languages
            let compaction_options = CompactionOptions::new()
                .with_language(*language)
                .preserve_docs(true)
                .preserve_signatures_only(false);

            let compacted = {
                let mut compactor = self.ast_compactor.lock().unwrap();
                compactor
                    .compact(content, &compaction_options)
                    .map_err(|e| SemanticIndexError::CompactionFailed {
                        file: document_id.to_string(),
                        message: e.to_string(),
                    })?
            };

            // Extract semantic chunks from compacted AST
            self.extract_semantic_chunks(&compacted.compacted_code, language)
        } else {
            // Fall back to simple text chunking for unknown languages
            self.extract_text_chunks(content, language)
        };

        Ok(chunks)
    }

    /// Extract semantic chunks from AST-compacted code
    fn extract_semantic_chunks(&self, compacted_code: &str, language: &Language) -> Vec<CodeChunk> {
        let mut chunks = Vec::new();
        let lines: Vec<&str> = compacted_code.lines().collect();

        let mut current_chunk = String::new();
        let mut start_line = 0;
        let mut current_line = 0;

        for (idx, line) in lines.iter().enumerate() {
            current_line = idx;
            current_chunk.push_str(line);
            current_chunk.push('\n');

            // Chunk boundary conditions based on language
            let is_boundary = match language {
                Language::Rust => {
                    line.starts_with("pub fn ")
                        || line.starts_with("fn ")
                        || line.starts_with("pub struct ")
                        || line.starts_with("struct ")
                        || line.starts_with("impl ")
                        || line.contains("}\n")
                }
                Language::Python => line.starts_with("def ") || line.starts_with("class "),
                Language::TypeScript | Language::JavaScript => {
                    line.starts_with("function ")
                        || line.starts_with("class ")
                        || line.contains("=> {")
                        || line.contains("}\n")
                }
                Language::Go => line.starts_with("func ") || line.starts_with("type "),
                _ => false,
            };

            // Create chunk when boundary is hit or size limit reached
            if (is_boundary && !current_chunk.trim().is_empty())
                || current_chunk.len() > self.config.max_chunk_size
            {
                if !current_chunk.trim().is_empty() {
                    chunks.push(CodeChunk {
                        id: ChunkId::new_v4(),
                        content: current_chunk.trim().to_string(),
                        language: *language,
                        start_line: Some(start_line),
                        end_line: Some(current_line),
                        file_path: None, // Will be set by caller
                    });
                }

                current_chunk.clear();
                start_line = current_line;
            }
        }

        // Add final chunk if any content remains
        if !current_chunk.trim().is_empty() {
            chunks.push(CodeChunk {
                id: ChunkId::new_v4(),
                content: current_chunk.trim().to_string(),
                language: *language,
                start_line: Some(start_line),
                end_line: Some(current_line),
                file_path: None,
            });
        }

        chunks
    }

    /// Extract simple text chunks for unsupported languages
    fn extract_text_chunks(&self, content: &str, language: &Language) -> Vec<CodeChunk> {
        let mut chunks = Vec::new();
        let chars: Vec<char> = content.chars().collect();

        let chunk_size = self.config.max_chunk_size;
        let overlap = self.config.chunk_overlap;

        let mut start = 0;
        while start < chars.len() {
            let end = (start + chunk_size).min(chars.len());
            let chunk_content: String = chars[start..end].iter().collect();

            if !chunk_content.trim().is_empty() {
                chunks.push(CodeChunk {
                    id: ChunkId::new_v4(),
                    content: chunk_content,
                    language: *language,
                    start_line: None,
                    end_line: None,
                    file_path: None,
                });
            }

            start += chunk_size - overlap;
        }

        chunks
    }

    /// Generate embeddings for multiple chunks in parallel
    async fn generate_embeddings(&self, chunks: &[CodeChunk]) -> Result<Vec<EmbeddingVector>> {
        let mut embeddings = Vec::with_capacity(chunks.len());

        // Process in batches to avoid overwhelming the system
        let batch_size = 10;
        for chunk_batch in chunks.chunks(batch_size) {
            let mut batch_handles = Vec::new();

            for chunk in chunk_batch {
                let engine = Arc::clone(&self.embedding_engine);
                let content = chunk.content.clone();

                let handle = tokio::spawn(async move { engine.generate_embedding(&content).await });

                batch_handles.push(handle);
            }

            // Collect batch results
            for handle in batch_handles {
                let embedding = handle
                    .await
                    .map_err(|e| SemanticIndexError::EmbeddingFailed {
                        chunk_id: ChunkId::new_v4(),
                        reason: format!("Task join error: {}", e),
                    })?
                    .map_err(|e| SemanticIndexError::EmbeddingFailed {
                        chunk_id: ChunkId::new_v4(),
                        reason: e.to_string(),
                    })?;

                embeddings.push(embedding);
            }
        }

        Ok(embeddings)
    }
}

// Enable cloning for parallel processing
impl Clone for SemanticIndexer {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            ast_compactor: Arc::clone(&self.ast_compactor),
            embedding_engine: Arc::clone(&self.embedding_engine),
            storage: Arc::clone(&self.storage),
            retrieval_engine: Arc::clone(&self.retrieval_engine),
            documents: Arc::clone(&self.documents),
            chunks: Arc::clone(&self.chunks),
            indexing_semaphore: Arc::clone(&self.indexing_semaphore),
            query_semaphore: Arc::clone(&self.query_semaphore),
            metrics: Arc::clone(&self.metrics),
        }
    }
}

/// Classify chunk type based on content analysis
fn classify_chunk_type(content: &str) -> ChunkType {
    let content_lower = content.to_lowercase();

    if content_lower.contains("fn ")
        || content_lower.contains("function ")
        || content_lower.contains("def ")
    {
        ChunkType::Function
    } else if content_lower.contains("struct ") || content_lower.contains("class ") {
        ChunkType::Struct
    } else if content_lower.contains("enum ") {
        ChunkType::Enum
    } else if content_lower.contains("trait ") || content_lower.contains("interface ") {
        ChunkType::Trait
    } else if content_lower.contains("mod ") || content_lower.contains("module ") {
        ChunkType::Module
    } else if content.trim().starts_with("//")
        || content.trim().starts_with("/*")
        || content.trim().starts_with("#")
    {
        ChunkType::Comment
    } else {
        ChunkType::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs;

    #[tokio::test]
    async fn test_indexer_creation() {
        let config = SemanticIndexConfig::default();
        let indexer = SemanticIndexer::new(config).unwrap();
        let metrics = indexer.get_metrics();

        assert_eq!(metrics.documents_indexed, 0);
        assert_eq!(metrics.chunks_created, 0);
    }

    #[tokio::test]
    async fn test_rust_file_indexing() {
        let config = SemanticIndexConfig::default();
        let indexer = SemanticIndexer::new(config).unwrap();

        // Create a temporary Rust file
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        let rust_code = r#"
pub struct User {
    pub id: u64,
    name: String,
}

impl User {
    pub fn new(id: u64, name: String) -> Self {
        Self { id, name }
    }
    
    pub fn get_name(&self) -> &str {
        &self.name
    }
}
"#;

        fs::write(&file_path, rust_code).await.unwrap();

        // Index the file
        let doc_id = indexer.index_file(&file_path).await.unwrap();

        // Verify indexing
        let doc_info = indexer.get_document_info(doc_id).unwrap();
        assert_eq!(doc_info.language, Language::Rust);
        assert!(!doc_info.chunk_ids.is_empty());

        let metrics = indexer.get_metrics();
        assert_eq!(metrics.documents_indexed, 1);
        assert!(metrics.chunks_created > 0);
    }

    #[tokio::test]
    async fn test_chunk_classification() {
        assert_eq!(classify_chunk_type("fn main() {"), ChunkType::Function);
        assert_eq!(classify_chunk_type("pub struct User {"), ChunkType::Struct);
        assert_eq!(classify_chunk_type("enum Color {"), ChunkType::Enum);
        assert_eq!(
            classify_chunk_type("// This is a comment"),
            ChunkType::Comment
        );
        assert_eq!(classify_chunk_type("let x = 5;"), ChunkType::Unknown);
    }

    #[tokio::test]
    async fn test_indexing_options_default() {
        let options = IndexingOptions::default();

        assert!(options.languages.contains(&Language::Rust));
        assert!(options.include_patterns.contains(&"**/*.rs".to_string()));
        assert!(
            options
                .exclude_patterns
                .contains(&"**/target/**".to_string())
        );
        assert_eq!(options.parallel_workers, 4);
        assert!(options.enable_compaction);
    }

    #[tokio::test]
    async fn test_clear_index() {
        let config = SemanticIndexConfig::default();
        let indexer = SemanticIndexer::new(config).unwrap();

        // Create a temporary file and index it
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        fs::write(&file_path, "fn test() {}").await.unwrap();

        indexer.index_file(&file_path).await.unwrap();

        // Verify it was indexed
        let metrics_before = indexer.get_metrics();
        assert_eq!(metrics_before.documents_indexed, 1);

        // Clear the index
        indexer.clear_index().await.unwrap();

        // Verify it was cleared
        let metrics_after = indexer.get_metrics();
        assert_eq!(metrics_after.documents_indexed, 0);
        assert_eq!(metrics_after.chunks_created, 0);
    }
}
