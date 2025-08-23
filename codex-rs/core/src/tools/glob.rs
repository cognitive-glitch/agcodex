//! High-performance file discovery tool with glob pattern support
//!
//! This module provides efficient file globbing with:
//! - ignore::WalkBuilder for respecting .gitignore files
//! - Parallel directory traversal for performance
//! - Complex glob patterns (*.rs, **/*.js, etc.)
//! - Extension-based filtering
//! - Rich metadata and context-aware output
//! - <100ms performance target for 10k files

use super::output::ComprehensiveToolOutput;
use ignore::DirEntry;
use ignore::Walk;
use ignore::WalkBuilder;
use ignore::WalkParallel;
use ignore::WalkState;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use thiserror::Error;
use tracing::info;
use wildmatch::WildMatch;

/// Errors for glob operations
#[derive(Error, Debug)]
pub enum GlobError {
    #[error("invalid glob pattern '{pattern}': {reason}")]
    InvalidPattern { pattern: String, reason: String },

    #[error("directory not found: {path}")]
    DirectoryNotFound { path: PathBuf },

    #[error("permission denied: {path}")]
    PermissionDenied { path: PathBuf },

    #[error("search timeout after {timeout:?}")]
    SearchTimeout { timeout: Duration },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("search error: {message}")]
    SearchError { message: String },

    #[error("search cancelled after {duration:?}")]
    SearchCancelled { duration: Duration },

    #[error("filter chain error: {message}")]
    FilterChain { message: String },
}

/// Result type for glob operations
pub type GlobResult<T> = std::result::Result<T, GlobError>;

/// Output type for glob operations  
pub type GlobOutput<T> = ComprehensiveToolOutput<T>;

/// Search strategy for optimization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchStrategy {
    /// Use parallel search with multiple threads
    Parallel,
    /// Use sequential search for memory efficiency
    Sequential,
    /// Automatically select based on directory size
    Auto,
}

/// File match result with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMatch {
    /// Absolute path to the file
    pub path: PathBuf,
    /// File size in bytes (None for directories)
    pub size: Option<u64>,
    /// File extension (if any)
    pub extension: Option<String>,
    /// File type classification
    pub file_type: FileType,
    /// Relative path from search root
    pub relative_path: PathBuf,
    /// Last modified time
    pub modified: Option<SystemTime>,
    /// Whether the file is executable
    pub executable: bool,
    /// Content category for easier filtering
    pub content_category: ContentCategory,
    /// Estimated lines of code (for text files)
    pub estimated_lines: Option<usize>,
}

/// File type classification
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileType {
    /// Regular file
    File,
    /// Directory
    Directory,
    /// Symbolic link
    Symlink,
    /// Other file type
    Other,
}

/// Content category for semantic understanding
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentCategory {
    /// Source code files
    Source,
    /// Configuration files
    Config,
    /// Documentation files
    Documentation,
    /// Test files
    Test,
    /// Binary or executable files
    Binary,
    /// Data files (JSON, CSV, etc.)
    Data,
    /// Unknown or unclassified
    Unknown,
}

/// Search query configuration
#[derive(Debug, Clone)]
pub struct GlobQuery {
    /// Base directory to search from
    pub base_dir: PathBuf,
    /// Glob patterns to match (e.g., "*.rs", "**/*.js")
    pub patterns: Vec<String>,
    /// File type filter
    pub file_type: Option<FileType>,
    /// Size constraints
    pub size_filter: Option<SizeFilter>,
    /// Maximum search depth
    pub max_depth: Option<usize>,
    /// Include hidden files/directories
    pub include_hidden: bool,
    /// Follow symbolic links
    pub follow_links: bool,
    /// Case sensitive matching
    pub case_sensitive: bool,
    /// Maximum number of results (0 = unlimited)
    pub max_results: usize,
    /// Search timeout
    pub timeout: Option<Duration>,
}

/// File size filter with min/max constraints
#[derive(Debug, Clone)]
pub struct SizeFilter {
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
}

/// Search statistics
#[derive(Debug, Clone, Default)]
pub struct SearchStats {
    /// Total directories traversed
    pub directories_traversed: usize,
    /// Total files examined
    pub files_examined: usize,
    /// Files filtered out by ignore rules
    pub files_ignored: usize,
}

/// Time-based filter for file modification times
#[derive(Debug, Clone)]
pub struct TimeFilter {
    pub modified_after: Option<SystemTime>,
    pub modified_before: Option<SystemTime>,
}

/// Filter chain for complex file matching
#[derive(Debug, Clone, Default)]
pub struct FilterChain {
    /// Glob patterns to match
    pub glob_patterns: Vec<GlobPattern>,
    /// File size filters
    pub size_filters: Vec<SizeFilter>,
    /// Time-based filters
    pub time_filters: Vec<TimeFilter>,
    /// Content category filters
    pub category_filters: Vec<ContentCategory>,
    /// File type filters
    pub type_filters: Vec<FileType>,
    /// Custom exclude patterns
    pub exclude_patterns: Vec<String>,
}

/// Glob pattern with compilation options
#[derive(Debug, Clone)]
pub struct GlobPattern {
    pub pattern: String,
    pub case_sensitive: bool,
    pub negate: bool,
}

/// Compiled filters for efficient matching
#[derive(Debug, Clone)]
pub struct CompiledFilters {
    /// Compiled glob patterns
    glob_matchers: Vec<CompiledGlobPattern>,
    /// Size filters (no compilation needed)
    size_filters: Vec<SizeFilter>,
    /// Time filters (no compilation needed)
    time_filters: Vec<TimeFilter>,
    /// Category filters (no compilation needed)
    category_filters: Vec<ContentCategory>,
    /// Type filters (no compilation needed)
    type_filters: Vec<FileType>,
    /// Compiled exclude patterns
    exclude_matchers: Vec<WildMatch>,
}

/// Compiled glob pattern for efficient matching
#[derive(Debug, Clone)]
struct CompiledGlobPattern {
    pattern: String,
    matcher: WildMatch,
    negate: bool,
}

/// Content classifier for determining file types
#[derive(Debug, Clone)]
pub struct ContentClassifier {
    source_extensions: HashSet<String>,
    config_extensions: HashSet<String>,
    doc_extensions: HashSet<String>,
    test_patterns: Vec<String>,
}

impl FilterChain {
    /// Add a glob pattern to the filter chain
    pub fn add_glob(
        &mut self,
        pattern: &str,
        case_sensitive: bool,
        negate: bool,
    ) -> GlobResult<()> {
        // Validate pattern
        if pattern.is_empty() {
            return Err(GlobError::InvalidPattern {
                pattern: pattern.to_string(),
                reason: "Pattern cannot be empty".to_string(),
            });
        }

        self.glob_patterns.push(GlobPattern {
            pattern: pattern.to_string(),
            case_sensitive,
            negate,
        });
        Ok(())
    }
}

impl CompiledFilters {
    /// Compile a filter chain into optimized matchers
    pub fn compile(chain: &FilterChain) -> GlobResult<Self> {
        let mut glob_matchers = Vec::new();

        for pattern in &chain.glob_patterns {
            let matcher = WildMatch::new(&pattern.pattern);
            glob_matchers.push(CompiledGlobPattern {
                pattern: pattern.pattern.clone(),
                matcher,
                negate: pattern.negate,
            });
        }

        let exclude_matchers = chain
            .exclude_patterns
            .iter()
            .map(|p| WildMatch::new(p))
            .collect();

        Ok(Self {
            glob_matchers,
            size_filters: chain.size_filters.clone(),
            time_filters: chain.time_filters.clone(),
            category_filters: chain.category_filters.clone(),
            type_filters: chain.type_filters.clone(),
            exclude_matchers,
        })
    }

    /// Check if a file matches all filters
    pub fn matches(&self, file: &FileMatch) -> bool {
        // Check glob patterns
        if !self.glob_matchers.is_empty() {
            // For simple patterns without path separators, match against filename only
            // For patterns with path separators, match against the relative path
            let mut matched = false;

            for pattern in &self.glob_matchers {
                let is_match = if pattern.pattern.contains('/') || pattern.pattern.contains("**") {
                    // Match against relative path for complex patterns
                    pattern
                        .matcher
                        .matches(&file.relative_path.to_string_lossy())
                } else {
                    // For simple patterns (e.g., "*.rs"), only match files in the root directory
                    // Check that the relative path doesn't contain any directory separators
                    let relative_path_str = file.relative_path.to_string_lossy();
                    if relative_path_str.contains('/') || relative_path_str.contains('\\') {
                        // File is in a subdirectory, don't match for simple patterns
                        false
                    } else if let Some(file_name) = file.path.file_name() {
                        // File is in root directory, check if filename matches
                        pattern.matcher.matches(&file_name.to_string_lossy())
                    } else {
                        false
                    }
                };

                if pattern.negate {
                    if is_match {
                        return false;
                    }
                } else if is_match {
                    matched = true;
                }
            }

            if !matched && !self.glob_matchers.iter().all(|p| p.negate) {
                return false;
            }
        }

        // Check exclude patterns - match against relative path
        let relative_path_str = file.relative_path.to_string_lossy();
        for exclude in &self.exclude_matchers {
            if exclude.matches(&relative_path_str) {
                return false;
            }
        }

        // Check size filters
        if let Some(size) = file.size {
            for filter in &self.size_filters {
                if let Some(min) = filter.min_size
                    && size < min
                {
                    return false;
                }
                if let Some(max) = filter.max_size
                    && size > max
                {
                    return false;
                }
            }
        }

        // Check time filters
        if let Some(modified) = file.modified {
            for filter in &self.time_filters {
                if let Some(after) = filter.modified_after
                    && modified < after
                {
                    return false;
                }
                if let Some(before) = filter.modified_before
                    && modified > before
                {
                    return false;
                }
            }
        }

        // Check category filters
        if !self.category_filters.is_empty()
            && !self.category_filters.contains(&file.content_category)
        {
            return false;
        }

        // Check type filters
        if !self.type_filters.is_empty() && !self.type_filters.contains(&file.file_type) {
            return false;
        }

        true
    }
}

impl Default for ContentClassifier {
    fn default() -> Self {
        let mut source_extensions = HashSet::new();
        source_extensions.extend(
            [
                "rs", "py", "js", "ts", "jsx", "tsx", "go", "java", "c", "cpp", "cc", "cxx", "h",
                "hpp", "cs", "php", "rb", "swift", "kt", "scala", "hs", "clj", "ex", "exs",
            ]
            .iter()
            .map(|s| (*s).to_string()),
        );

        let mut config_extensions = HashSet::new();
        config_extensions.extend(
            [
                "toml", "yaml", "yml", "json", "xml", "ini", "cfg", "conf", "config", "env",
            ]
            .iter()
            .map(|s| (*s).to_string()),
        );

        let mut doc_extensions = HashSet::new();
        doc_extensions.extend(
            ["md", "txt", "rst", "adoc", "tex", "pdf", "doc", "docx"]
                .iter()
                .map(|s| (*s).to_string()),
        );

        let test_patterns = vec![
            "test".to_string(),
            "spec".to_string(),
            "_test".to_string(),
            ".test.".to_string(),
            ".spec.".to_string(),
        ];

        Self {
            source_extensions,
            config_extensions,
            doc_extensions,
            test_patterns,
        }
    }
}

impl ContentClassifier {
    /// Classify a file based on its path and extension
    pub fn classify_path(&self, path: &Path) -> ContentCategory {
        // Check if it looks like a test file first
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            for pattern in &self.test_patterns {
                if file_name.contains(pattern) {
                    return ContentCategory::Test;
                }
            }
        }

        // Check by extension
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();

            if self.source_extensions.contains(&ext_lower) {
                return ContentCategory::Source;
            }

            if self.config_extensions.contains(&ext_lower) {
                return ContentCategory::Config;
            }

            if self.doc_extensions.contains(&ext_lower) {
                return ContentCategory::Documentation;
            }

            // Check for binary extensions
            match ext_lower.as_str() {
                "exe" | "dll" | "so" | "dylib" | "a" | "lib" | "o" | "obj" | "bin" => {
                    return ContentCategory::Binary;
                }
                _ => {}
            }
        }

        ContentCategory::Unknown
    }
}

/// High-performance file discovery tool
#[derive(Clone)]
pub struct GlobTool {
    /// Base directory for searches
    base_dir: PathBuf,
    /// Default filter chain
    default_filters: FilterChain,
    /// Number of threads for parallel search
    parallelism: usize,
    /// Whether to respect .gitignore and similar files
    respect_ignore: bool,
    /// Maximum number of results to return
    max_results: Option<usize>,
    /// Whether to follow symbolic links
    follow_links: bool,
    /// Whether to include hidden files
    include_hidden: bool,
    /// Search timeout
    timeout: Option<Duration>,
    /// Custom ignore patterns
    custom_ignores: Vec<String>,
    /// File content classifier
    classifier: Arc<ContentClassifier>,
    /// Cancellation support
    cancellation: Arc<AtomicBool>,
}

impl Default for GlobTool {
    fn default() -> Self {
        Self::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

impl Default for GlobQuery {
    fn default() -> Self {
        Self {
            base_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            patterns: Vec::new(),
            file_type: None,
            size_filter: None,
            max_depth: Some(32), // Reasonable default to prevent runaway
            include_hidden: false,
            follow_links: false,
            case_sensitive: true,
            max_results: 0, // unlimited
            timeout: Some(Duration::from_secs(30)),
        }
    }
}

impl GlobTool {
    /// Create a new glob tool with specified base directory
    pub fn new(base_dir: PathBuf) -> Self {
        let thread_count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        Self {
            base_dir,
            default_filters: FilterChain::default(),
            parallelism: thread_count,
            respect_ignore: true,
            max_results: Some(10_000), // Performance limit for large codebases
            follow_links: false,
            include_hidden: false,
            timeout: Some(Duration::from_secs(30)),
            custom_ignores: Vec::new(),
            classifier: Arc::new(ContentClassifier::default()),
            cancellation: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Builder: Set parallelism (number of threads)
    pub fn with_parallelism(mut self, threads: usize) -> Self {
        self.parallelism = threads.max(1);
        self
    }

    /// Builder: Set whether to respect ignore files
    pub const fn with_respect_ignore(mut self, respect: bool) -> Self {
        self.respect_ignore = respect;
        self
    }

    /// Builder: Set maximum results limit
    pub const fn with_max_results(mut self, max_results: Option<usize>) -> Self {
        self.max_results = max_results;
        self
    }

    /// Builder: Set whether to follow symbolic links
    pub const fn with_follow_links(mut self, follow_links: bool) -> Self {
        self.follow_links = follow_links;
        self
    }

    /// Builder: Set whether to include hidden files
    pub const fn with_include_hidden(mut self, include_hidden: bool) -> Self {
        self.include_hidden = include_hidden;
        self
    }

    /// Builder: Set search timeout
    pub const fn with_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }

    /// Builder: Add custom ignore patterns
    pub fn with_custom_ignores(mut self, ignores: Vec<String>) -> Self {
        self.custom_ignores = ignores;
        self
    }

    /// Builder: Set default filter chain
    pub fn with_filter_chain(mut self, filters: FilterChain) -> Self {
        self.default_filters = filters;
        self
    }

    /// Core API: Find files by glob pattern with high performance
    pub fn glob(&self, pattern: &str) -> GlobResult<GlobOutput<Vec<FileMatch>>> {
        let mut filters = self.default_filters.clone();
        filters.add_glob(pattern, true, false)?;
        self.search_with_filters(filters)
    }

    /// Core API: Find files by extension (*.ext pattern)
    pub fn find_type(&self, extension: &str) -> GlobResult<GlobOutput<Vec<FileMatch>>> {
        // We need to match files at all levels, including root
        // Using two patterns: *.ext for root files and **/*.ext for nested files
        let ext_clean = extension.trim_start_matches('.').trim_start_matches('*');
        
        // Create a filter chain with both patterns
        let mut filters = self.default_filters.clone();
        
        // Add pattern for root-level files
        filters.add_glob(&format!("*.{}", ext_clean), true, false)?;
        
        // Add pattern for files in subdirectories
        filters.add_glob(&format!("**/*.{}", ext_clean), true, false)?;
        
        self.search_with_filters(filters)
    }

    /// Advanced API: Search with custom FilterChain
    pub fn search_with_filters(
        &self,
        filters: FilterChain,
    ) -> GlobResult<GlobOutput<Vec<FileMatch>>> {
        use super::output::*;
        let timer = PerformanceTimer::new();

        // Reset cancellation flag
        self.cancellation.store(false, Ordering::Relaxed);

        // Validate base directory
        if !self.base_dir.exists() {
            return Err(GlobError::DirectoryNotFound {
                path: self.base_dir.clone(),
            });
        }

        // Compile filters for performance
        let compiled_filters = CompiledFilters::compile(&filters)?;

        // Execute search with selected strategy
        let strategy = self.select_strategy();
        let (matches, stats) = match strategy {
            SearchStrategy::Parallel => self.search_parallel(&compiled_filters)?,
            SearchStrategy::Sequential => self.search_sequential(&compiled_filters)?,
            SearchStrategy::Auto => {
                if self.estimate_directory_size() > 1000 {
                    self.search_parallel(&compiled_filters)?
                } else {
                    self.search_sequential(&compiled_filters)?
                }
            }
        };

        let duration = timer.elapsed();
        let matches_count = matches.len();

        // Generate comprehensive context
        let summary = format!(
            "Found {} files in {} ({}ms, {} examined, {} ignored)",
            matches_count,
            self.base_dir.display(),
            duration.as_millis(),
            stats.files_examined,
            stats.files_ignored
        );

        // Build rich output using the comprehensive output builder
        let location = SourceLocation::new(self.base_dir.to_string_lossy(), 0, 0, 0, 0, (0, 0));

        let output = OutputBuilder::new(
            matches,
            "glob",
            "file_discovery".to_string(),
            location.clone(),
        )
        .context(OperationContext {
            before: ContextSnapshot {
                content: String::new(),
                timestamp: SystemTime::now(),
                content_hash: String::new(),
                ast_summary: None,
                symbols: Vec::new(),
            },
            after: None,
            surrounding: vec![
                ContextLine {
                    line_number: 0,
                    content: format!("Search root: {}", self.base_dir.display()),
                    line_type: ContextLineType::Separator,
                    indentation: 0,
                    modified: false,
                },
                ContextLine {
                    line_number: 0,
                    content: format!("Strategy: {:?}", strategy),
                    line_type: ContextLineType::Separator,
                    indentation: 0,
                    modified: false,
                },
            ],
            location: location.clone(),
            scope: OperationScope {
                scope_type: ScopeType::File,
                name: self
                    .base_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
                path: vec![self.base_dir.to_string_lossy().to_string()],
                file_path: self.base_dir.clone(),
                line_range: 0..0,
            },
            language_context: None,
            project_context: None,
        })
        .performance(PerformanceMetrics {
            execution_time: duration,
            phase_times: HashMap::new(),
            memory_usage: MemoryUsage {
                peak_bytes: (matches_count * std::mem::size_of::<FileMatch>()) as u64,
                average_bytes: 0,
                allocations: 0,
                deallocations: 0,
                efficiency_score: 0.9,
            },
            cpu_usage: CpuUsage {
                cpu_time: duration,
                utilization_percent: 0.0,
                context_switches: 0,
            },
            io_stats: IoStats {
                bytes_read: 0,
                bytes_written: 0,
                read_ops: stats.files_examined as u64,
                write_ops: 0,
                io_wait_time: Duration::from_millis(0),
            },
            cache_stats: CacheStats {
                hit_rate: 0.0,
                hits: 0,
                misses: stats.files_examined as u64,
                cache_size: 0,
                efficiency_score: 0.0,
            },
        })
        .summary(summary)
        .build();

        info!(
            "Glob search completed: {} matches in {:?}",
            matches_count, duration
        );
        Ok(output)
    }

    /// Cancel ongoing search operation
    pub fn cancel(&self) {
        self.cancellation.store(true, Ordering::Relaxed);
    }

    /// Check if search was cancelled
    fn is_cancelled(&self) -> bool {
        self.cancellation.load(Ordering::Relaxed)
    }

    /// Select optimal search strategy based on directory estimation
    fn select_strategy(&self) -> SearchStrategy {
        if self.parallelism == 1 {
            SearchStrategy::Sequential
        } else if self.estimate_directory_size() > 500 {
            SearchStrategy::Parallel
        } else {
            SearchStrategy::Sequential
        }
    }

    /// Advanced API: Search in specific directory with pattern
    pub fn find_in_directory(
        &self,
        dir: &Path,
        pattern: &str,
    ) -> GlobResult<GlobOutput<Vec<FileMatch>>> {
        let scoped_tool = GlobTool::new(dir.to_path_buf())
            .with_parallelism(self.parallelism)
            .with_max_results(self.max_results)
            .with_follow_links(self.follow_links)
            .with_include_hidden(self.include_hidden)
            .with_timeout(self.timeout)
            .with_custom_ignores(self.custom_ignores.clone())
            .with_filter_chain(self.default_filters.clone());

        scoped_tool.glob(pattern)
    }

    /// Advanced API: Find files by content category
    pub fn find_by_category(
        &self,
        category: ContentCategory,
    ) -> GlobResult<GlobOutput<Vec<FileMatch>>> {
        let mut filters = self.default_filters.clone();
        filters.category_filters.push(category);
        self.search_with_filters(filters)
    }

    /// Advanced API: Find files with size constraints
    pub fn find_by_size(
        &self,
        min_size: Option<u64>,
        max_size: Option<u64>,
    ) -> GlobResult<GlobOutput<Vec<FileMatch>>> {
        let mut filters = self.default_filters.clone();
        filters.size_filters.push(SizeFilter { min_size, max_size });
        self.search_with_filters(filters)
    }

    /// Advanced API: Find files modified after a specific time
    pub fn find_modified_since(&self, since: SystemTime) -> GlobResult<GlobOutput<Vec<FileMatch>>> {
        let mut filters = self.default_filters.clone();
        filters.time_filters.push(TimeFilter {
            modified_after: Some(since),
            modified_before: None,
        });
        self.search_with_filters(filters)
    }

    /// High-performance parallel search implementation
    fn search_parallel(
        &self,
        filters: &CompiledFilters,
    ) -> GlobResult<(Vec<FileMatch>, SearchStats)> {
        let matches = Arc::new(Mutex::new(Vec::new()));
        let stats = Arc::new(Mutex::new(SearchStats::default()));
        let start_time = Instant::now();

        let walker = self.create_parallel_walker()?;

        walker.run(|| {
            let matches = matches.clone();
            let stats = stats.clone();
            let filters = filters.clone();
            let classifier = self.classifier.clone();
            let base_dir = self.base_dir.clone();
            let max_results = self.max_results;
            let timeout = self.timeout;
            let cancellation = self.cancellation.clone();

            Box::new(move |entry_result| {
                // Check for cancellation or timeout
                if cancellation.load(Ordering::Relaxed) {
                    return WalkState::Quit;
                }

                if let Some(timeout) = timeout
                    && start_time.elapsed() > timeout
                {
                    cancellation.store(true, Ordering::Relaxed);
                    return WalkState::Quit;
                }

                match entry_result {
                    Ok(entry) => {
                        let _path = entry.path();

                        // Update statistics
                        {
                            let mut stats = stats.lock().unwrap();
                            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                                stats.directories_traversed += 1;
                                return WalkState::Continue;
                            } else {
                                stats.files_examined += 1;
                            }
                        }

                        // Create and filter file match
                        if let Some(file_match) =
                            Self::create_file_match(&entry, &classifier, &base_dir)
                        {
                            if filters.matches(&file_match) {
                                let mut matches = matches.lock().unwrap();

                                // Check max results limit
                                if let Some(max) = max_results
                                    && matches.len() >= max
                                {
                                    return WalkState::Quit;
                                }

                                matches.push(file_match);
                            } else {
                                let mut stats = stats.lock().unwrap();
                                stats.files_ignored += 1;
                            }
                        }
                    }
                    Err(_) => {
                        let mut stats = stats.lock().unwrap();
                        stats.files_ignored += 1;
                    }
                }
                WalkState::Continue
            })
        });

        let matches = Arc::try_unwrap(matches)
            .map_err(|_| GlobError::FilterChain {
                message: "Failed to unwrap matches".to_string(),
            })?
            .into_inner()
            .map_err(|_| GlobError::FilterChain {
                message: "Failed to acquire matches lock".to_string(),
            })?;

        let stats = Arc::try_unwrap(stats)
            .map_err(|_| GlobError::FilterChain {
                message: "Failed to unwrap stats".to_string(),
            })?
            .into_inner()
            .map_err(|_| GlobError::FilterChain {
                message: "Failed to acquire stats lock".to_string(),
            })?;

        Ok((matches, stats))
    }

    /// Memory-efficient sequential search implementation
    fn search_sequential(
        &self,
        filters: &CompiledFilters,
    ) -> GlobResult<(Vec<FileMatch>, SearchStats)> {
        let mut matches = Vec::new();
        let mut stats = SearchStats::default();
        let start_time = Instant::now();

        let walker = self.create_sequential_walker();

        for entry_result in walker {
            // Check for cancellation or timeout
            if self.is_cancelled() {
                return Err(GlobError::SearchCancelled {
                    duration: start_time.elapsed(),
                });
            }

            if let Some(timeout) = self.timeout
                && start_time.elapsed() > timeout
            {
                return Err(GlobError::SearchTimeout { timeout });
            }

            match entry_result {
                Ok(entry) => {
                    let _path = entry.path();

                    // Handle directories
                    if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                        stats.directories_traversed += 1;
                        continue;
                    }

                    stats.files_examined += 1;

                    // Check max results limit
                    if let Some(max) = self.max_results
                        && matches.len() >= max
                    {
                        break;
                    }

                    // Create and filter file match
                    if let Some(file_match) =
                        Self::create_file_match(&entry, &self.classifier, &self.base_dir)
                    {
                        if filters.matches(&file_match) {
                            matches.push(file_match);
                        } else {
                            stats.files_ignored += 1;
                        }
                    }
                }
                Err(_) => {
                    stats.files_ignored += 1;
                }
            }
        }

        Ok((matches, stats))
    }

    /// Create optimized parallel walker
    fn create_parallel_walker(&self) -> GlobResult<WalkParallel> {
        let mut builder = WalkBuilder::new(&self.base_dir);
        self.configure_builder(&mut builder)?;
        builder.threads(self.parallelism);
        Ok(builder.build_parallel())
    }

    /// Create memory-efficient sequential walker  
    fn create_sequential_walker(&self) -> Walk {
        let mut builder = WalkBuilder::new(&self.base_dir);
        let _ = self.configure_builder(&mut builder); // Ignore configuration errors
        builder.build()
    }

    /// Configure walker builder with comprehensive options
    fn configure_builder(&self, builder: &mut WalkBuilder) -> GlobResult<()> {
        builder
            .follow_links(self.follow_links)
            .hidden(!self.include_hidden)
            .ignore(self.respect_ignore)
            .git_ignore(self.respect_ignore)
            .git_global(self.respect_ignore)
            .git_exclude(self.respect_ignore)
            .parents(self.respect_ignore)
            .require_git(false); // Don't require git for ignore files to work

        // Add custom ignore patterns
        for ignore_pattern in &self.custom_ignores {
            builder.add_custom_ignore_filename(ignore_pattern);
        }

        // Validate base directory accessibility
        if !self.base_dir.exists() {
            return Err(GlobError::DirectoryNotFound {
                path: self.base_dir.clone(),
            });
        }

        if !self.base_dir.is_dir() {
            return Err(GlobError::FilterChain {
                message: format!("Path is not a directory: {}", self.base_dir.display()),
            });
        }

        Ok(())
    }

    /// Create comprehensive FileMatch with rich metadata
    fn create_file_match(
        entry: &DirEntry,
        classifier: &ContentClassifier,
        base_dir: &Path,
    ) -> Option<FileMatch> {
        let path = entry.path();

        // Get file metadata with error handling
        let metadata = match entry.metadata() {
            Ok(meta) => meta,
            Err(_) => return None, // Skip inaccessible files
        };

        let file_type = if metadata.is_file() {
            FileType::File
        } else if metadata.is_dir() {
            FileType::Directory
        } else if metadata.file_type().is_symlink() {
            FileType::Symlink
        } else {
            FileType::Other
        };

        let size = if file_type == FileType::File {
            Some(metadata.len())
        } else {
            None
        };

        let extension = path
            .extension()
            .and_then(OsStr::to_str)
            .map(|s| s.to_lowercase());

        let content_category = classifier.classify_path(path);

        let relative_path = path.strip_prefix(base_dir).unwrap_or(path).to_path_buf();

        let modified = metadata.modified().ok();

        let executable = {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                metadata.permissions().mode() & 0o111 != 0
            }
            #[cfg(not(unix))]
            {
                extension
                    .as_ref()
                    .map(|ext| matches!(ext.as_str(), "exe" | "bat" | "cmd" | "com"))
                    .unwrap_or(false)
            }
        };

        // Estimate lines for text files
        let estimated_lines = if matches!(
            content_category,
            ContentCategory::Source | ContentCategory::Config | ContentCategory::Documentation
        ) {
            size.and_then(|s| {
                if s < 1024 * 1024 {
                    // Only estimate for files < 1MB
                    Some((s / 40).max(1) as usize) // Rough estimate: 40 bytes per line
                } else {
                    None
                }
            })
        } else {
            None
        };

        Some(FileMatch {
            path: path.to_path_buf(),
            size,
            extension,
            file_type,
            relative_path,
            modified,
            executable,
            content_category,
            estimated_lines,
        })
    }

    /// Estimate directory size for strategy selection optimization
    fn estimate_directory_size(&self) -> usize {
        // Fast estimation: count immediate entries (files + directories)
        std::fs::read_dir(&self.base_dir)
            .map(|entries| entries.count())
            .unwrap_or(0)
    }
}

/// Internal statistics tracking
#[derive(Default)]
#[allow(dead_code)]
struct GlobStats {
    files_examined: usize,
    directories_traversed: usize,
    files_ignored: usize,
}

/// File type categories for semantic classification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileTypeCategory {
    Source,
    Config,
    Documentation,
    Test,
    Binary,
    Generated,
    Unknown,
}

/// File extension classifier for determining file types
pub struct FileExtensionClassifier {
    source_extensions: HashSet<String>,
    config_extensions: HashSet<String>,
    doc_extensions: HashSet<String>,
    test_patterns: Vec<Regex>,
}

impl Default for FileExtensionClassifier {
    fn default() -> Self {
        let mut source_extensions = HashSet::new();
        // Programming languages
        source_extensions.extend(
            [
                "rs", "py", "js", "ts", "jsx", "tsx", "go", "java", "c", "cpp", "cc", "cxx", "h",
                "hpp", "cs", "php", "rb", "swift", "kt", "scala", "hs", "clj", "ex", "exs", "sh",
                "bash", "zsh", "fish", "ps1", "bat", "cmd",
            ]
            .iter()
            .map(|s| (*s).to_string()),
        );

        let mut config_extensions = HashSet::new();
        // Configuration files
        config_extensions.extend(
            [
                "toml",
                "yaml",
                "yml",
                "json",
                "xml",
                "ini",
                "cfg",
                "conf",
                "config",
                "env",
                "properties",
                "dockerfile",
            ]
            .iter()
            .map(|s| (*s).to_string()),
        );

        let mut doc_extensions = HashSet::new();
        // Documentation files
        doc_extensions.extend(
            ["md", "txt", "rst", "adoc", "tex", "pdf", "doc", "docx"]
                .iter()
                .map(|s| (*s).to_string()),
        );

        // Test file patterns (regex for flexibility)
        let test_patterns = vec![
            Regex::new(r"(?i)test").unwrap(),
            Regex::new(r"(?i)spec").unwrap(),
            Regex::new(r"_test\.[^.]+$").unwrap(),
            Regex::new(r"\.test\.[^.]+$").unwrap(),
            Regex::new(r"\.spec\.[^.]+$").unwrap(),
        ];

        Self {
            source_extensions,
            config_extensions,
            doc_extensions,
            test_patterns,
        }
    }
}

impl FileExtensionClassifier {
    /// Classify a file based on its path and extension
    pub fn classify_file(&self, path: &Path) -> FileTypeCategory {
        // Check if it looks like a test file first
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            for pattern in &self.test_patterns {
                if pattern.is_match(file_name) {
                    return FileTypeCategory::Test;
                }
            }
        }

        // Check by extension
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();

            if self.source_extensions.contains(&ext_lower) {
                return FileTypeCategory::Source;
            }

            if self.config_extensions.contains(&ext_lower) {
                return FileTypeCategory::Config;
            }

            if self.doc_extensions.contains(&ext_lower) {
                return FileTypeCategory::Documentation;
            }

            // Check for binary extensions
            match ext_lower.as_str() {
                "exe" | "dll" | "so" | "dylib" | "a" | "lib" | "o" | "obj" | "bin" => {
                    return FileTypeCategory::Binary;
                }
                "class" | "jar" | "pyc" | "pyo" | "rlib" | "node" => {
                    return FileTypeCategory::Generated;
                }
                _ => {}
            }
        }

        // Check for common generated file patterns
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            let file_name_lower = file_name.to_lowercase();
            if file_name_lower.starts_with("generated_")
                || file_name_lower.contains("autogenerated")
                || file_name_lower.starts_with("build")
                || file_name_lower == "cargo.lock"
                || file_name_lower == "package-lock.json"
                || file_name_lower == "yarn.lock"
            {
                return FileTypeCategory::Generated;
            }
        }

        FileTypeCategory::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_dir() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Initialize as a git repository so .gitignore works properly
        // If git is not available, we'll also create a .ignore file as fallback
        let git_init = std::process::Command::new("git")
            .arg("init")
            .current_dir(path)
            .output()
            .is_ok();

        // Create test files
        fs::write(path.join("main.rs"), "fn main() {}").unwrap();
        fs::write(path.join("lib.js"), "console.log('hello')").unwrap();
        fs::write(path.join("config.toml"), "[package]").unwrap();
        fs::write(path.join("README.md"), "# Test").unwrap();
        fs::write(path.join("test_main.rs"), "mod tests {}").unwrap();

        // Create subdirectory
        fs::create_dir(path.join("src")).unwrap();
        fs::write(path.join("src").join("lib.rs"), "pub mod lib;").unwrap();

        // Create .gitignore with proper line endings
        fs::write(path.join(".gitignore"), "target/\n*.tmp\n").unwrap();

        // Also create .ignore file for non-git environments
        // The ignore crate respects .ignore files even without git
        if !git_init {
            fs::write(path.join(".ignore"), "target/\n*.tmp\n").unwrap();
        }

        // Create ignored file and directory
        fs::write(path.join("ignored.tmp"), "temporary").unwrap();
        fs::create_dir(path.join("target")).unwrap();
        fs::write(path.join("target").join("debug.txt"), "debug info").unwrap();

        temp_dir
    }

    #[test]
    fn test_glob_rust_files() {
        let temp_dir = create_test_dir();
        let glob_tool = GlobTool::new(temp_dir.path().to_path_buf());

        let result = glob_tool.glob("*.rs").unwrap();

        // Should find main.rs and test_main.rs (but not src/lib.rs with this pattern)
        assert_eq!(result.result.len(), 2);
        assert!(
            result
                .result
                .iter()
                .any(|m| m.path.file_name().unwrap() == "main.rs")
        );
        assert!(
            result
                .result
                .iter()
                .any(|m| m.path.file_name().unwrap() == "test_main.rs")
        );
        assert!(result.summary.contains("Found 2 files"));
    }

    #[test]
    fn test_find_type() {
        let temp_dir = create_test_dir();
        let glob_tool = GlobTool::new(temp_dir.path().to_path_buf());

        let result = glob_tool.find_type("rs").unwrap();

        // Should find all .rs files
        assert_eq!(result.result.len(), 3); // main.rs, test_main.rs, src/lib.rs
        assert!(
            result
                .result
                .iter()
                .all(|m| m.extension.as_ref().unwrap() == "rs")
        );
    }

    #[test]
    fn test_file_classification() {
        let classifier = FileExtensionClassifier::default();

        assert_eq!(
            classifier.classify_file(Path::new("main.rs")),
            FileTypeCategory::Source
        );
        assert_eq!(
            classifier.classify_file(Path::new("config.toml")),
            FileTypeCategory::Config
        );
        assert_eq!(
            classifier.classify_file(Path::new("README.md")),
            FileTypeCategory::Documentation
        );
        assert_eq!(
            classifier.classify_file(Path::new("test_main.rs")),
            FileTypeCategory::Test
        );
        assert_eq!(
            classifier.classify_file(Path::new("main.spec.js")),
            FileTypeCategory::Test
        );
    }

    #[test]
    fn test_gitignore_respected() {
        let temp_dir = create_test_dir();

        // Ensure .gitignore is properly set up
        let gitignore_path = temp_dir.path().join(".gitignore");
        assert!(gitignore_path.exists(), ".gitignore should exist");

        // Test that ignored.tmp exists
        let ignored_file = temp_dir.path().join("ignored.tmp");
        assert!(ignored_file.exists(), "ignored.tmp should exist");

        let glob_tool = GlobTool::new(temp_dir.path().to_path_buf()).with_respect_ignore(true); // Explicitly enable .gitignore respect
        let result = glob_tool.glob("*.tmp").unwrap();

        // Should not find ignored.tmp due to .gitignore
        // The walker with git_ignore(true) should filter out the file
        assert_eq!(
            result.result.len(),
            0,
            "Should find 0 files (ignored.tmp should be filtered by .gitignore)"
        );
        assert!(result.result.is_empty());
    }

    #[test]
    fn test_parallel_vs_sequential() {
        let temp_dir = create_test_dir();

        let parallel_tool = GlobTool::new(temp_dir.path().to_path_buf()).with_parallelism(4);
        let sequential_tool = GlobTool::new(temp_dir.path().to_path_buf()).with_parallelism(1);

        let parallel_result = parallel_tool.glob("*").unwrap();
        let sequential_result = sequential_tool.glob("*").unwrap();

        // Results should be the same (order may differ)
        assert_eq!(parallel_result.result.len(), sequential_result.result.len());
    }

    #[test]
    fn test_max_results() {
        let temp_dir = create_test_dir();
        let glob_tool = GlobTool::new(temp_dir.path().to_path_buf()).with_max_results(Some(2));

        let result = glob_tool.glob("*").unwrap();

        // Should be limited to 2 results
        assert!(result.result.len() <= 2);
    }
}
