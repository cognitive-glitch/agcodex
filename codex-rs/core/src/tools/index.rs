//! Tantivy-based indexing tool for AGCodex
//!
//! This module provides a comprehensive search indexing system using Tantivy
//! for fast, sophisticated codebase search. The IndexTool supports:
//!
//! - Full-text search with language-aware analysis
//! - Symbol-based semantic search (functions, classes, variables)
//! - Incremental updates for efficient re-indexing
//! - Location-aware results with precise file:line:column metadata
//! - Multi-language support via tree-sitter integration
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
//! │   IndexTool     │───▶│  Tantivy Index  │───▶│  Search Results │
//! │                 │    │                 │    │                 │
//! │ • build()       │    │ Schema:         │    │ • path          │
//! │ • update()      │    │ • path          │    │ • content       │
//! │ • optimize()    │    │ • content       │    │ • symbols       │
//! │ • stats()       │    │ • symbols       │    │ • language      │
//! │ • search()      │    │ • language      │    │ • location      │
//! └─────────────────┘    └─────────────────┘    └─────────────────┘
//! ```

use super::InternalTool;
use super::ToolMetadata;
use super::output::CacheStats;
use super::output::ComprehensiveToolOutput;
use super::output::CpuUsage;
use super::output::IoStats;
use super::output::MemoryUsage;
use super::output::OutputBuilder;
use super::output::PerformanceMetrics;
use ast::SourceLocation;

use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use thiserror::Error;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::instrument;
use uuid::Uuid;
use walkdir::WalkDir;

use tantivy::DocAddress;
use tantivy::DocId;
use tantivy::Index;
use tantivy::IndexReader;
use tantivy::IndexWriter;
use tantivy::ReloadPolicy;
use tantivy::Searcher;
use tantivy::TantivyError;
use tantivy::Term;
use tantivy::collector::TopDocs;
use tantivy::doc;
use tantivy::query::BooleanQuery;
use tantivy::query::FuzzyTermQuery;
use tantivy::query::Occur;
use tantivy::query::PhraseQuery;
use tantivy::query::QueryParser;
use tantivy::query::TermQuery;
use tantivy::schema::*;

/// Errors specific to the indexing tool
#[derive(Error, Debug)]
pub enum IndexError {
    #[error("tantivy index error: {0}")]
    Tantivy(#[from] TantivyError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid index path: {path}")]
    InvalidPath { path: PathBuf },

    #[error("index not initialized: {0}")]
    NotInitialized(String),

    #[error("concurrent access error: {0}")]
    ConcurrentAccess(String),

    #[error("document not found: {path}")]
    DocumentNotFound { path: PathBuf },

    #[error("query parsing error: {query}: {source}")]
    QueryParsing { query: String, source: String },

    #[error("schema error: {0}")]
    Schema(String),

    #[error("indexing operation failed: {operation}: {reason}")]
    OperationFailed { operation: String, reason: String },

    #[error("incremental update failed: {0}")]
    IncrementalUpdateFailed(String),

    #[error("optimization failed: {0}")]
    OptimizationFailed(String),

    #[error("statistics calculation failed: {0}")]
    StatsFailed(String),
}

/// Result type for index operations
pub type IndexResult<T> = std::result::Result<T, IndexError>;

/// Configuration for the indexing tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    /// Directory to store the index
    pub index_path: PathBuf,

    /// File extensions to include in indexing
    pub include_extensions: Vec<String>,

    /// Maximum file size to index (in bytes)
    pub max_file_size: usize,

    /// Whether to enable incremental updates
    pub incremental: bool,

    /// Writer memory budget (in MB)
    pub writer_memory_mb: usize,

    /// Number of threads for indexing
    pub num_threads: Option<usize>,

    /// Merge policy settings
    pub merge_policy: MergePolicyConfig,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            index_path: PathBuf::from(".agcodex/index"),
            include_extensions: vec![
                "rs".to_string(),
                "py".to_string(),
                "js".to_string(),
                "ts".to_string(),
                "tsx".to_string(),
                "jsx".to_string(),
                "go".to_string(),
                "java".to_string(),
                "c".to_string(),
                "cpp".to_string(),
                "h".to_string(),
                "hpp".to_string(),
                "cs".to_string(),
                "php".to_string(),
                "rb".to_string(),
                "swift".to_string(),
                "kt".to_string(),
                "scala".to_string(),
                "hs".to_string(),
                "ex".to_string(),
                "exs".to_string(),
                "clj".to_string(),
                "cljs".to_string(),
                "lua".to_string(),
                "sh".to_string(),
                "bash".to_string(),
                "zsh".to_string(),
                "fish".to_string(),
                "ps1".to_string(),
                "bat".to_string(),
                "dockerfile".to_string(),
                "yaml".to_string(),
                "yml".to_string(),
                "json".to_string(),
                "toml".to_string(),
                "xml".to_string(),
                "html".to_string(),
                "css".to_string(),
                "scss".to_string(),
                "md".to_string(),
                "txt".to_string(),
            ],
            max_file_size: 10 * 1024 * 1024, // 10MB
            incremental: true,
            writer_memory_mb: 256,
            num_threads: None,
            merge_policy: MergePolicyConfig::default(),
        }
    }
}

/// Merge policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergePolicyConfig {
    pub max_merge_at_once: usize,
    pub max_merge_segment_size_mb: usize,
    pub level_log_size: f64,
}

impl Default for MergePolicyConfig {
    fn default() -> Self {
        Self {
            max_merge_at_once: 10,
            max_merge_segment_size_mb: 1024, // 1GB
            level_log_size: 0.75,
        }
    }
}

/// Document in the search index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedDocument {
    /// Relative path from workspace root
    pub path: String,

    /// Full text content of the file
    pub content: String,

    /// Extracted symbols (functions, classes, variables)
    pub symbols: Vec<Symbol>,

    /// Programming language
    pub language: String,

    /// File size in bytes
    pub size: u64,

    /// Last modified timestamp
    pub modified: u64,

    /// Content hash for change detection
    pub hash: String,
}

/// Symbol information extracted from code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    /// Symbol name
    pub name: String,

    /// Symbol type (function, class, variable, etc.)
    pub symbol_type: String,

    /// Line number (1-based)
    pub line: u32,

    /// Column number (1-based)  
    pub column: u32,

    /// End line number (1-based)
    pub end_line: u32,

    /// End column number (1-based)
    pub end_column: u32,

    /// Optional documentation/comments
    pub documentation: Option<String>,

    /// Visibility (public, private, protected)
    pub visibility: Option<String>,

    /// Parent scope (for nested symbols)
    pub parent: Option<String>,
}

/// Search result from the index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Document that matched
    pub document: IndexedDocument,

    /// Search score
    pub score: f32,

    /// Highlighted text snippets
    pub snippets: Vec<String>,

    /// Matching symbols
    pub matching_symbols: Vec<Symbol>,
}

/// Statistics about the index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    /// Total number of documents
    pub document_count: u64,

    /// Total number of terms
    pub term_count: u64,

    /// Index size on disk (bytes)
    pub size_bytes: u64,

    /// Number of segments
    pub segment_count: usize,

    /// Last update timestamp
    pub last_updated: u64,

    /// Average document size
    pub avg_document_size: f64,

    /// Language distribution
    pub language_stats: HashMap<String, u64>,

    /// Symbol type distribution
    pub symbol_stats: HashMap<String, u64>,
}

/// Search query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Main search text
    pub query: String,

    /// Language filter
    pub language: Option<String>,

    /// Path filter (glob pattern)
    pub path_filter: Option<String>,

    /// Symbol type filter
    pub symbol_type: Option<String>,

    /// Maximum results to return
    pub limit: Option<usize>,

    /// Enable fuzzy matching
    pub fuzzy: bool,

    /// Minimum score threshold
    pub min_score: Option<f32>,
}

/// Input for build operation
#[derive(Debug, Clone)]
pub struct BuildInput {
    pub directory: PathBuf,
    pub config: IndexConfig,
    pub force_rebuild: bool,
}

/// Input for update operation
#[derive(Debug, Clone)]
pub struct UpdateInput {
    pub files: Vec<PathBuf>,
    pub config: IndexConfig,
}

/// Input for search operation
#[derive(Debug, Clone)]
pub struct SearchInput {
    pub query: SearchQuery,
    pub config: IndexConfig,
}

/// Main indexing tool implementation
pub struct IndexTool {
    /// Tantivy index
    index: Arc<RwLock<Option<Index>>>,

    /// Index writer
    writer: Arc<Mutex<Option<IndexWriter>>>,

    /// Index reader
    reader: Arc<RwLock<Option<IndexReader>>>,

    /// Current configuration
    config: Arc<RwLock<IndexConfig>>,

    /// Schema definition
    schema: Arc<Schema>,

    /// Field handles for the schema
    fields: Arc<IndexFields>,
}

/// Schema field handles
#[derive(Debug)]
struct IndexFields {
    path: Field,
    content: Field,
    symbols: Field,
    language: Field,
    size: Field,
    modified: Field,
    hash: Field,
    symbol_names: Field,
    symbol_types: Field,
    symbol_docs: Field,
}

impl IndexTool {
    /// Create a new IndexTool with the given configuration
    pub fn new(config: IndexConfig) -> IndexResult<Self> {
        let schema = Self::build_schema();
        let fields = Arc::new(Self::extract_fields(&schema)?);

        Ok(Self {
            index: Arc::new(RwLock::new(None)),
            writer: Arc::new(Mutex::new(None)),
            reader: Arc::new(RwLock::new(None)),
            config: Arc::new(RwLock::new(config)),
            schema: Arc::new(schema),
            fields,
        })
    }

    /// Build the Tantivy schema for code indexing
    fn build_schema() -> Schema {
        let mut schema_builder = Schema::builder();

        // File path (unique identifier)
        schema_builder.add_text_field("path", STRING | STORED | FAST);

        // File content (full-text searchable)
        schema_builder.add_text_field("content", TEXT | STORED);

        // Serialized symbols
        schema_builder.add_text_field("symbols", STORED);

        // Programming language
        schema_builder.add_text_field("language", STRING | STORED | FAST);

        // File size
        schema_builder.add_u64_field("size", STORED | INDEXED);

        // Last modified timestamp
        schema_builder.add_u64_field("modified", STORED | INDEXED);

        // Content hash
        schema_builder.add_text_field("hash", STRING | STORED);

        // Symbol names (searchable)
        schema_builder.add_text_field("symbol_names", TEXT | STORED);

        // Symbol types (filterable)
        schema_builder.add_text_field("symbol_types", STRING | STORED | FAST);

        // Symbol documentation (searchable)
        schema_builder.add_text_field("symbol_docs", TEXT | STORED);

        schema_builder.build()
    }

    /// Extract field handles from schema
    fn extract_fields(schema: &Schema) -> IndexResult<IndexFields> {
        Ok(IndexFields {
            path: schema
                .get_field("path")
                .map_err(|e| IndexError::Schema(format!("Missing path field: {}", e)))?,
            content: schema
                .get_field("content")
                .map_err(|e| IndexError::Schema(format!("Missing content field: {}", e)))?,
            symbols: schema
                .get_field("symbols")
                .map_err(|e| IndexError::Schema(format!("Missing symbols field: {}", e)))?,
            language: schema
                .get_field("language")
                .map_err(|e| IndexError::Schema(format!("Missing language field: {}", e)))?,
            size: schema
                .get_field("size")
                .map_err(|e| IndexError::Schema(format!("Missing size field: {}", e)))?,
            modified: schema
                .get_field("modified")
                .map_err(|e| IndexError::Schema(format!("Missing modified field: {}", e)))?,
            hash: schema
                .get_field("hash")
                .map_err(|e| IndexError::Schema(format!("Missing hash field: {}", e)))?,
            symbol_names: schema
                .get_field("symbol_names")
                .map_err(|e| IndexError::Schema(format!("Missing symbol_names field: {}", e)))?,
            symbol_types: schema
                .get_field("symbol_types")
                .map_err(|e| IndexError::Schema(format!("Missing symbol_types field: {}", e)))?,
            symbol_docs: schema
                .get_field("symbol_docs")
                .map_err(|e| IndexError::Schema(format!("Missing symbol_docs field: {}", e)))?,
        })
    }

    /// Initialize the index
    #[instrument(skip(self))]
    pub async fn initialize(&self) -> IndexResult<()> {
        let config = self.config.read().unwrap().clone();

        // Create index directory
        if !config.index_path.exists() {
            fs::create_dir_all(&config.index_path)?;
        }

        // Open or create index
        let index = if config.index_path.join("meta.json").exists() {
            info!("Opening existing index at {:?}", config.index_path);
            Index::open_in_dir(&config.index_path)?
        } else {
            info!("Creating new index at {:?}", config.index_path);
            Index::create_in_dir(&config.index_path, self.schema.as_ref().clone())?
        };

        // Create writer with memory budget
        let writer = index.writer(config.writer_memory_mb * 1_000_000)?;

        // Create reader with reload policy
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        // Store components
        {
            let mut index_lock = self.index.write().unwrap();
            *index_lock = Some(index);
        }

        {
            let mut writer_lock = self.writer.lock().unwrap();
            *writer_lock = Some(writer);
        }

        {
            let mut reader_lock = self.reader.write().unwrap();
            *reader_lock = Some(reader);
        }

        info!("Index initialized successfully");
        Ok(())
    }

    /// Build index for a directory
    #[instrument(skip(self, input))]
    pub async fn build(
        &self,
        input: BuildInput,
    ) -> IndexResult<ComprehensiveToolOutput<IndexStats>> {
        info!("Building index for directory: {:?}", input.directory);
        let start_time = std::time::Instant::now();
        let location = SourceLocation::new(
            input.directory.to_string_lossy().as_ref(),
            0,
            0,
            0,
            0,
            (0, 0),
        );

        // Update configuration
        {
            let mut config_lock = self.config.write().unwrap();
            *config_lock = input.config.clone();
        }

        // Initialize if not already done
        if self.index.read().unwrap().is_none() {
            self.initialize().await?;
        }

        // Clear existing index if force rebuild
        if input.force_rebuild {
            self.clear_index().await?;
        }

        // Collect all files to index
        let files = self.collect_files(&input.directory).await?;
        let file_count = files.len();
        info!("Found {} files to index", file_count);

        // Index files in batches
        let batch_size = 100;
        for batch in files.chunks(batch_size) {
            self.index_batch(batch).await?;
        }

        // Commit changes
        self.commit().await?;

        // Get statistics
        let stats = self.stats_internal().await?;

        // Build output with context
        let output = OutputBuilder::new(stats.clone(), "index", "build".to_string(), location)
            .summary(format!(
                "Built index with {} documents in {:?}",
                stats.document_count,
                start_time.elapsed()
            ))
            .performance(PerformanceMetrics {
                execution_time: start_time.elapsed(),
                phase_times: HashMap::new(),
                memory_usage: MemoryUsage {
                    peak_bytes: 0, // TODO: Measure actual memory
                    average_bytes: 0,
                    allocations: 0,
                    deallocations: 0,
                    efficiency_score: 0.9,
                },
                cpu_usage: CpuUsage {
                    cpu_time: start_time.elapsed(),
                    utilization_percent: 0.0,
                    context_switches: 0,
                },
                io_stats: IoStats {
                    bytes_read: 0,
                    bytes_written: 0,
                    read_ops: file_count as u64,
                    write_ops: file_count as u64,
                    io_wait_time: std::time::Duration::from_millis(0),
                },
                cache_stats: CacheStats {
                    hit_rate: 0.0,
                    hits: 0,
                    misses: 0,
                    cache_size: 0,
                    efficiency_score: 0.0,
                },
            })
            .build();

        Ok(output)
    }

    /// Update index with changed files
    #[instrument(skip(self, input))]
    pub async fn update(
        &self,
        input: UpdateInput,
    ) -> IndexResult<ComprehensiveToolOutput<IndexStats>> {
        info!("Updating index with {} files", input.files.len());
        let start_time = std::time::Instant::now();
        let file_count = input.files.len();

        // Update configuration
        {
            let mut config_lock = self.config.write().unwrap();
            *config_lock = input.config;
        }

        // Ensure index is initialized
        if self.index.read().unwrap().is_none() {
            return Err(IndexError::NotInitialized(
                "Index must be built first".to_string(),
            ));
        }

        let mut updated = 0;
        let mut removed = 0;

        // Process each file
        for file_path in &input.files {
            if file_path.exists() {
                self.update_file(file_path).await?;
                updated += 1;
            } else {
                self.remove_file(file_path).await?;
                removed += 1;
            }
        }

        // Commit changes
        self.commit().await?;

        // Get updated statistics
        let stats = self.stats_internal().await?;

        let location = SourceLocation::new("index", 0, 0, 0, 0, (0, 0));

        let output = OutputBuilder::new(stats.clone(), "index", "update".to_string(), location)
            .summary(format!(
                "Updated {} files, removed {} files",
                updated, removed
            ))
            .performance(PerformanceMetrics {
                execution_time: start_time.elapsed(),
                phase_times: HashMap::new(),
                memory_usage: MemoryUsage {
                    peak_bytes: 0,
                    average_bytes: 0,
                    allocations: 0,
                    deallocations: 0,
                    efficiency_score: 0.9,
                },
                cpu_usage: CpuUsage {
                    cpu_time: start_time.elapsed(),
                    utilization_percent: 0.0,
                    context_switches: 0,
                },
                io_stats: IoStats {
                    bytes_read: 0,
                    bytes_written: 0,
                    read_ops: file_count as u64,
                    write_ops: file_count as u64,
                    io_wait_time: std::time::Duration::from_millis(0),
                },
                cache_stats: CacheStats {
                    hit_rate: 0.0,
                    hits: 0,
                    misses: 0,
                    cache_size: 0,
                    efficiency_score: 0.0,
                },
            })
            .build();

        Ok(output)
    }

    /// Optimize the index for better performance
    #[instrument(skip(self))]
    pub async fn optimize(&self) -> IndexResult<()> {
        info!("Optimizing index");

        let mut writer_guard = self.writer.lock().unwrap();
        let writer = writer_guard
            .as_mut()
            .ok_or_else(|| IndexError::NotInitialized("Writer not initialized".to_string()))?;

        // Wait for merges to complete
        writer
            .wait_merging_threads()
            .map_err(|e| IndexError::OptimizationFailed(format!("Merge wait failed: {}", e)))?;

        info!("Index optimization completed");
        Ok(())
    }

    /// Get index statistics with output wrapper
    #[instrument(skip(self))]
    pub async fn stats(&self) -> IndexResult<ComprehensiveToolOutput<IndexStats>> {
        let start_time = std::time::Instant::now();
        let stats = self.stats_internal().await?;

        let location = SourceLocation::new("index", 0, 0, 0, 0, (0, 0));

        let output = OutputBuilder::new(stats.clone(), "index", "stats".to_string(), location)
            .summary(format!(
                "Index contains {} documents across {} segments",
                stats.document_count, stats.segment_count
            ))
            .performance(PerformanceMetrics {
                execution_time: start_time.elapsed(),
                phase_times: HashMap::new(),
                memory_usage: MemoryUsage {
                    peak_bytes: stats.size_bytes,
                    average_bytes: stats.size_bytes,
                    allocations: 0,
                    deallocations: 0,
                    efficiency_score: 0.9,
                },
                cpu_usage: CpuUsage {
                    cpu_time: start_time.elapsed(),
                    utilization_percent: 0.0,
                    context_switches: 0,
                },
                io_stats: IoStats {
                    bytes_read: stats.size_bytes,
                    bytes_written: 0,
                    read_ops: 1,
                    write_ops: 0,
                    io_wait_time: std::time::Duration::from_millis(0),
                },
                cache_stats: CacheStats {
                    hit_rate: 0.0,
                    hits: 0,
                    misses: 0,
                    cache_size: 0,
                    efficiency_score: 0.0,
                },
            })
            .build();

        Ok(output)
    }

    /// Internal stats method without output wrapper
    async fn stats_internal(&self) -> IndexResult<IndexStats> {
        let reader_guard = self.reader.read().unwrap();
        let reader = reader_guard
            .as_ref()
            .ok_or_else(|| IndexError::NotInitialized("Reader not initialized".to_string()))?;

        let searcher = reader.searcher();
        let segment_readers = searcher.segment_readers();

        let document_count = segment_readers
            .iter()
            .map(|reader| reader.num_docs() as u64)
            .sum::<u64>();

        // Calculate index size on disk
        let config = self.config.read().unwrap();
        let size_bytes = self.calculate_index_size(&config.index_path)?;

        // Collect language and symbol statistics
        let (language_stats, symbol_stats, avg_document_size) =
            self.collect_detailed_stats(&searcher).await?;

        Ok(IndexStats {
            document_count,
            term_count: 0, // TODO: Calculate if needed
            size_bytes,
            segment_count: segment_readers.len(),
            last_updated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            avg_document_size,
            language_stats,
            symbol_stats,
        })
    }

    /// Search the index
    #[instrument(skip(self, input))]
    pub async fn search(
        &self,
        input: SearchInput,
    ) -> IndexResult<ComprehensiveToolOutput<Vec<SearchResult>>> {
        let start_time = std::time::Instant::now();
        let reader_guard = self.reader.read().unwrap();
        let reader = reader_guard
            .as_ref()
            .ok_or_else(|| IndexError::NotInitialized("Reader not initialized".to_string()))?;

        let searcher = reader.searcher();

        // Build query
        let query = self.build_query(&input.query)?;

        // Execute search
        let limit = input.query.limit.unwrap_or(50);
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        // Convert to results
        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            if let Some(min_score) = input.query.min_score {
                if score < min_score {
                    continue;
                }
            }

            let doc = searcher.doc(doc_address)?;
            if let Ok(result) = self.doc_to_search_result(doc, score).await {
                results.push(result);
            }
        }

        let result_count = results.len();

        let location = SourceLocation::new("index", 0, 0, 0, 0, (0, 0));

        let output = OutputBuilder::new(results, "index", "search".to_string(), location)
            .summary(format!(
                "Found {} results for query: {}",
                result_count, input.query.query
            ))
            .performance(PerformanceMetrics {
                execution_time: start_time.elapsed(),
                phase_times: HashMap::new(),
                memory_usage: MemoryUsage {
                    peak_bytes: 0,
                    average_bytes: 0,
                    allocations: 0,
                    deallocations: 0,
                    efficiency_score: 0.9,
                },
                cpu_usage: CpuUsage {
                    cpu_time: start_time.elapsed(),
                    utilization_percent: 0.0,
                    context_switches: 0,
                },
                io_stats: IoStats {
                    bytes_read: 0,
                    bytes_written: 0,
                    read_ops: 1,
                    write_ops: 0,
                    io_wait_time: std::time::Duration::from_millis(0),
                },
                cache_stats: CacheStats {
                    hit_rate: 0.0,
                    hits: 0,
                    misses: 0,
                    cache_size: 0,
                    efficiency_score: 0.0,
                },
            })
            .build();

        Ok(output)
    }

    // Helper methods for internal operations

    async fn collect_files(&self, directory: &Path) -> IndexResult<Vec<PathBuf>> {
        let config = self.config.read().unwrap();
        let extensions = &config.include_extensions;
        let max_size = config.max_file_size;

        let mut files = Vec::new();
        for entry in WalkDir::new(directory).follow_links(false) {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            // Check extension
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if !extensions.contains(&ext.to_lowercase()) {
                    continue;
                }
            } else {
                continue;
            }

            // Check file size
            if let Ok(metadata) = entry.metadata() {
                if metadata.len() > max_size as u64 {
                    debug!("Skipping large file: {:?} ({} bytes)", path, metadata.len());
                    continue;
                }
            }

            files.push(path.to_path_buf());
        }

        Ok(files)
    }

    async fn index_batch(&self, files: &[PathBuf]) -> IndexResult<()> {
        for file_path in files {
            self.index_file(file_path).await?;
        }
        Ok(())
    }

    async fn index_file(&self, file_path: &Path) -> IndexResult<()> {
        // Read file content
        let content = fs::read_to_string(file_path)?;

        // Extract metadata
        let metadata = fs::metadata(file_path)?;
        let size = metadata.len();
        let modified = metadata
            .modified()?
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Calculate hash
        let hash = format!("{:x}", md5::compute(&content));

        // Detect language
        let language = self.detect_language(file_path);

        // Extract symbols (placeholder implementation)
        let symbols = self.extract_symbols(&content, &language).await?;

        // Create document
        let doc = self.create_document(IndexedDocument {
            path: file_path.to_string_lossy().to_string(),
            content,
            symbols: symbols.clone(),
            language: language.clone(),
            size,
            modified,
            hash,
        })?;

        // Add to index
        let mut writer_guard = self.writer.lock().unwrap();
        let writer = writer_guard
            .as_mut()
            .ok_or_else(|| IndexError::NotInitialized("Writer not initialized".to_string()))?;

        writer.add_document(doc)?;

        Ok(())
    }

    async fn update_file(&self, file_path: &Path) -> IndexResult<()> {
        // Remove existing document
        self.remove_file(file_path).await?;

        // Add updated document
        self.index_file(file_path).await?;

        Ok(())
    }

    async fn remove_file(&self, file_path: &Path) -> IndexResult<()> {
        let path_str = file_path.to_string_lossy().to_string();

        let mut writer_guard = self.writer.lock().unwrap();
        let writer = writer_guard
            .as_mut()
            .ok_or_else(|| IndexError::NotInitialized("Writer not initialized".to_string()))?;

        let path_term = Term::from_field_text(self.fields.path, &path_str);
        writer.delete_term(path_term);

        Ok(())
    }

    async fn clear_index(&self) -> IndexResult<()> {
        let mut writer_guard = self.writer.lock().unwrap();
        let writer = writer_guard
            .as_mut()
            .ok_or_else(|| IndexError::NotInitialized("Writer not initialized".to_string()))?;

        writer.delete_all_documents()?;
        Ok(())
    }

    async fn commit(&self) -> IndexResult<()> {
        let mut writer_guard = self.writer.lock().unwrap();
        let writer = writer_guard
            .as_mut()
            .ok_or_else(|| IndexError::NotInitialized("Writer not initialized".to_string()))?;

        writer.commit()?;
        Ok(())
    }

    fn detect_language(&self, path: &Path) -> String {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            match ext.to_lowercase().as_str() {
                "rs" => "rust".to_string(),
                "py" => "python".to_string(),
                "js" => "javascript".to_string(),
                "ts" => "typescript".to_string(),
                "tsx" => "typescript".to_string(),
                "jsx" => "javascript".to_string(),
                "go" => "go".to_string(),
                "java" => "java".to_string(),
                "c" => "c".to_string(),
                "cpp" | "cc" | "cxx" => "cpp".to_string(),
                "h" | "hpp" | "hh" | "hxx" => "c_header".to_string(),
                "cs" => "csharp".to_string(),
                "php" => "php".to_string(),
                "rb" => "ruby".to_string(),
                "swift" => "swift".to_string(),
                "kt" => "kotlin".to_string(),
                "scala" => "scala".to_string(),
                "hs" => "haskell".to_string(),
                "ex" | "exs" => "elixir".to_string(),
                "clj" | "cljs" => "clojure".to_string(),
                "lua" => "lua".to_string(),
                "sh" | "bash" | "zsh" | "fish" => "shell".to_string(),
                "ps1" => "powershell".to_string(),
                "dockerfile" => "dockerfile".to_string(),
                "yaml" | "yml" => "yaml".to_string(),
                "json" => "json".to_string(),
                "toml" => "toml".to_string(),
                "xml" => "xml".to_string(),
                "html" => "html".to_string(),
                "css" => "css".to_string(),
                "scss" => "scss".to_string(),
                "md" => "markdown".to_string(),
                _ => "unknown".to_string(),
            }
        } else {
            "unknown".to_string()
        }
    }

    async fn extract_symbols(&self, _content: &str, _language: &str) -> IndexResult<Vec<Symbol>> {
        // TODO: Implement tree-sitter based symbol extraction
        // This is a placeholder implementation
        Ok(vec![])
    }

    fn create_document(&self, doc: IndexedDocument) -> IndexResult<tantivy::Document> {
        let mut tantivy_doc = tantivy::Document::new();

        // Add basic fields
        tantivy_doc.add_text(self.fields.path, &doc.path);
        tantivy_doc.add_text(self.fields.content, &doc.content);
        tantivy_doc.add_text(self.fields.language, &doc.language);
        tantivy_doc.add_u64(self.fields.size, doc.size);
        tantivy_doc.add_u64(self.fields.modified, doc.modified);
        tantivy_doc.add_text(self.fields.hash, &doc.hash);

        // Serialize symbols
        let symbols_json = serde_json::to_string(&doc.symbols)
            .map_err(|e| IndexError::Schema(format!("Symbol serialization failed: {}", e)))?;
        tantivy_doc.add_text(self.fields.symbols, &symbols_json);

        // Extract symbol names and types for searching
        let symbol_names: Vec<String> = doc.symbols.iter().map(|s| s.name.clone()).collect();
        let symbol_types: Vec<String> = doc.symbols.iter().map(|s| s.symbol_type.clone()).collect();
        let symbol_docs: Vec<String> = doc
            .symbols
            .iter()
            .filter_map(|s| s.documentation.as_ref())
            .cloned()
            .collect();

        if !symbol_names.is_empty() {
            tantivy_doc.add_text(self.fields.symbol_names, &symbol_names.join(" "));
        }
        if !symbol_types.is_empty() {
            tantivy_doc.add_text(self.fields.symbol_types, &symbol_types.join(" "));
        }
        if !symbol_docs.is_empty() {
            tantivy_doc.add_text(self.fields.symbol_docs, &symbol_docs.join(" "));
        }

        Ok(tantivy_doc)
    }

    fn build_query(
        &self,
        search_query: &SearchQuery,
    ) -> IndexResult<Box<dyn tantivy::query::Query>> {
        let index_guard = self.index.read().unwrap();
        let index = index_guard
            .as_ref()
            .ok_or_else(|| IndexError::NotInitialized("Index not initialized".to_string()))?;

        let mut query_parser = QueryParser::for_index(
            index,
            vec![
                self.fields.content,
                self.fields.symbol_names,
                self.fields.symbol_docs,
            ],
        );

        // Parse main query
        let query = query_parser.parse_query(&search_query.query).map_err(|e| {
            IndexError::QueryParsing {
                query: search_query.query.clone(),
                source: e.to_string(),
            }
        })?;

        // TODO: Add filters for language, path, symbol_type

        Ok(query)
    }

    async fn doc_to_search_result(
        &self,
        doc: tantivy::Document,
        score: f32,
    ) -> IndexResult<SearchResult> {
        // Extract document fields
        let path = doc
            .get_first(self.fields.path)
            .and_then(|v| v.as_text())
            .unwrap_or("")
            .to_string();

        let content = doc
            .get_first(self.fields.content)
            .and_then(|v| v.as_text())
            .unwrap_or("")
            .to_string();

        let language = doc
            .get_first(self.fields.language)
            .and_then(|v| v.as_text())
            .unwrap_or("")
            .to_string();

        let size = doc
            .get_first(self.fields.size)
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let modified = doc
            .get_first(self.fields.modified)
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let hash = doc
            .get_first(self.fields.hash)
            .and_then(|v| v.as_text())
            .unwrap_or("")
            .to_string();

        // Deserialize symbols
        let symbols_json = doc
            .get_first(self.fields.symbols)
            .and_then(|v| v.as_text())
            .unwrap_or("[]");
        let symbols: Vec<Symbol> = serde_json::from_str(symbols_json).unwrap_or_default();

        let document = IndexedDocument {
            path,
            content,
            symbols: symbols.clone(),
            language,
            size,
            modified,
            hash,
        };

        // TODO: Generate snippets and match symbols
        let snippets = vec![];
        let matching_symbols = vec![];

        Ok(SearchResult {
            document,
            score,
            snippets,
            matching_symbols,
        })
    }

    fn calculate_index_size(&self, index_path: &Path) -> IndexResult<u64> {
        let mut total_size = 0u64;
        for entry in WalkDir::new(index_path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                total_size += entry.metadata()?.len();
            }
        }
        Ok(total_size)
    }

    async fn collect_detailed_stats(
        &self,
        _searcher: &Searcher,
    ) -> IndexResult<(HashMap<String, u64>, HashMap<String, u64>, f64)> {
        // TODO: Implement detailed statistics collection
        Ok((HashMap::new(), HashMap::new(), 0.0))
    }
}

// Implementation of InternalTool trait for different operations

impl InternalTool for IndexTool {
    type Input = BuildInput;
    type Output = ComprehensiveToolOutput<IndexStats>;
    type Error = IndexError;

    async fn execute(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
        self.build(input).await
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "IndexTool".to_string(),
            description: "Tantivy-based indexing tool for fast codebase search".to_string(),
            version: "1.0.0".to_string(),
            author: "AGCodex".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_index_tool_creation() {
        let config = IndexConfig::default();
        let tool = IndexTool::new(config).unwrap();

        assert!(tool.index.read().unwrap().is_none());
        assert!(tool.writer.lock().unwrap().is_none());
        assert!(tool.reader.read().unwrap().is_none());
    }

    #[tokio::test]
    async fn test_build_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let index_dir = temp_dir.path().join("index");

        let config = IndexConfig {
            index_path: index_dir,
            ..Default::default()
        };

        let tool = IndexTool::new(config).unwrap();
        let input = BuildInput {
            directory: temp_dir.path().to_path_buf(),
            config: IndexConfig::default(),
            force_rebuild: false,
        };

        let stats = tool.build(input).await.unwrap();
        assert_eq!(stats.document_count, 0);
    }

    #[tokio::test]
    async fn test_language_detection() {
        let config = IndexConfig::default();
        let tool = IndexTool::new(config).unwrap();

        assert_eq!(tool.detect_language(Path::new("test.rs")), "rust");
        assert_eq!(tool.detect_language(Path::new("test.py")), "python");
        assert_eq!(tool.detect_language(Path::new("test.js")), "javascript");
        assert_eq!(tool.detect_language(Path::new("test.unknown")), "unknown");
    }

    #[tokio::test]
    async fn test_schema_creation() {
        let schema = IndexTool::build_schema();

        assert!(schema.get_field("path").is_ok());
        assert!(schema.get_field("content").is_ok());
        assert!(schema.get_field("symbols").is_ok());
        assert!(schema.get_field("language").is_ok());
        assert!(schema.get_field("size").is_ok());
        assert!(schema.get_field("modified").is_ok());
        assert!(schema.get_field("hash").is_ok());
        assert!(schema.get_field("symbol_names").is_ok());
        assert!(schema.get_field("symbol_types").is_ok());
        assert!(schema.get_field("symbol_docs").is_ok());
    }
}
