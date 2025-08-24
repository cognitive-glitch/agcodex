//! Native fd-find integration using ignore::WalkBuilder for AGCodex.
//!
//! This module provides high-performance file discovery with:
//! - Parallel directory traversal via rayon
//! - Advanced filtering (glob, regex, type, size, depth)
//! - .gitignore respect by default
//! - Cancellation support for long-running searches
//! - Memory-efficient streaming results

use super::CodeTool;
use super::ToolError;
use ignore::WalkBuilder;
use ignore::WalkState;
// Rayon prelude removed - not currently used
use regex_lite::Regex;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::SystemTime;
use wildmatch::WildMatch;

/// High-performance fd-find replacement using ignore crate
#[derive(Debug, Clone, Default)]
pub struct FdFind {
    /// Default maximum search depth (prevents runaway searches)
    pub default_max_depth: Option<usize>,
    /// Default thread count (None = auto-detect)
    pub default_threads: Option<usize>,
}

/// Comprehensive search query for file discovery
#[derive(Debug, Clone)]
pub struct FdQuery {
    /// Base directory to search from
    pub base_dir: PathBuf,
    /// Glob patterns to match (e.g., "*.rs", "**/*.js")
    pub glob_patterns: Vec<String>,
    /// Regex pattern for advanced matching
    pub regex_pattern: Option<String>,
    /// File type filter
    pub file_type: FileTypeFilter,
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
    /// Number of parallel threads (None = auto-detect)
    pub threads: Option<usize>,
}

/// File type filtering options
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileTypeFilter {
    /// All files and directories
    All,
    /// Files only
    FilesOnly,
    /// Directories only
    DirectoriesOnly,
    /// Executable files only
    ExecutableOnly,
    /// Symbolic links only
    SymlinksOnly,
    /// Empty files/directories
    EmptyOnly,
}

/// File size filtering
#[derive(Debug, Clone)]
pub struct SizeFilter {
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
}

/// Search result with metadata
#[derive(Debug, Clone)]
pub struct FdResult {
    /// Full path to the found item
    pub path: PathBuf,
    /// File type
    pub file_type: FdFileType,
    /// File size in bytes (if applicable)
    pub size: Option<u64>,
    /// Last modified time
    pub modified: Option<SystemTime>,
    /// Whether the file is executable
    pub executable: bool,
}

/// File type enumeration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FdFileType {
    File,
    Directory,
    Symlink,
    Other,
}

/// Internal search state for cancellation and progress tracking
#[derive(Debug)]
struct SearchState {
    /// Results collector (thread-safe)
    results: Arc<Mutex<Vec<FdResult>>>,
    /// Cancellation flag
    cancelled: Arc<AtomicBool>,
    /// Number of items processed
    processed: Arc<AtomicUsize>,
    /// Maximum results limit
    max_results: usize,
    /// Search start time for timeout
    start_time: SystemTime,
    /// Search timeout
    timeout: Option<Duration>,
}

/// Compiled search filters for performance
#[derive(Debug)]
struct CompiledFilters {
    /// Compiled glob matchers
    glob_matchers: Vec<WildMatch>,
    /// Compiled regex
    regex: Option<Regex>,
    /// Size filter
    size_filter: Option<SizeFilter>,
    /// File type filter
    file_type: FileTypeFilter,
}

impl Default for FdQuery {
    fn default() -> Self {
        Self {
            base_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            glob_patterns: Vec::new(),
            regex_pattern: None,
            file_type: FileTypeFilter::All,
            size_filter: None,
            max_depth: None,
            include_hidden: false,
            follow_links: false,
            case_sensitive: true,
            max_results: 0,                         // unlimited
            timeout: Some(Duration::from_secs(30)), // 30 second default timeout
            threads: None,                          // auto-detect
        }
    }
}

impl FdFind {
    /// Create a new FdFind instance with default settings
    pub const fn new() -> Self {
        Self {
            default_max_depth: Some(32), // Reasonable default to prevent runaway
            default_threads: None,       // Auto-detect based on CPU cores
        }
    }

    /// Create FdFind with custom defaults
    pub const fn with_defaults(max_depth: Option<usize>, threads: Option<usize>) -> Self {
        Self {
            default_max_depth: max_depth,
            default_threads: threads,
        }
    }

    /// Helper: Find files by extension
    pub fn find_files_by_extension<P: AsRef<Path>>(
        &self,
        base_dir: P,
        extensions: &[&str],
    ) -> Result<Vec<FdResult>, ToolError> {
        let patterns: Vec<String> = extensions
            .iter()
            .map(|ext| format!("**/*.{}", ext.trim_start_matches('.')))
            .collect();

        let query = FdQuery {
            base_dir: base_dir.as_ref().to_path_buf(),
            glob_patterns: patterns,
            file_type: FileTypeFilter::FilesOnly,
            ..Default::default()
        };

        self.search(query)
    }

    /// Helper: Find directories by name pattern
    pub fn find_directories_by_name<P: AsRef<Path>>(
        &self,
        base_dir: P,
        name_pattern: &str,
    ) -> Result<Vec<FdResult>, ToolError> {
        let query = FdQuery {
            base_dir: base_dir.as_ref().to_path_buf(),
            glob_patterns: vec![format!("**/{}", name_pattern)],
            file_type: FileTypeFilter::DirectoriesOnly,
            ..Default::default()
        };

        self.search(query)
    }

    /// Helper: Find files modified since a given time
    pub fn find_modified_since<P: AsRef<Path>>(
        &self,
        base_dir: P,
        since: SystemTime,
    ) -> Result<Vec<FdResult>, ToolError> {
        let query = FdQuery {
            base_dir: base_dir.as_ref().to_path_buf(),
            file_type: FileTypeFilter::FilesOnly,
            ..Default::default()
        };

        let results = self.search(query)?;
        Ok(results
            .into_iter()
            .filter(|r| r.modified.map(|modified| modified > since).unwrap_or(false))
            .collect())
    }

    /// Helper: Find files by content type (based on extension)
    pub fn find_by_content_type<P: AsRef<Path>>(
        &self,
        base_dir: P,
        content_type: ContentType,
    ) -> Result<Vec<FdResult>, ToolError> {
        let extensions = match content_type {
            ContentType::Source => {
                vec![
                    "rs", "py", "js", "ts", "jsx", "tsx", "go", "java", "c", "cpp", "cc", "cxx",
                    "h", "hpp", "cs", "php", "rb", "scala", "kt", "swift", "zig", "odin",
                ]
            }
            ContentType::Config => {
                vec![
                    "toml", "yaml", "yml", "json", "xml", "ini", "conf", "config", "env",
                ]
            }
            ContentType::Documentation => {
                vec!["md", "rst", "txt", "adoc", "org", "tex", "html"]
            }
            ContentType::Build => {
                vec![
                    "Makefile",
                    "makefile",
                    "Dockerfile",
                    "dockerfile",
                    "BUILD",
                    "build",
                    "cmake",
                    "meson",
                    "ninja",
                    "gradle",
                    "pom",
                ]
            }
        };

        self.find_files_by_extension(base_dir, &extensions)
    }

    /// Compile search filters for performance
    fn compile_filters(&self, query: &FdQuery) -> Result<CompiledFilters, ToolError> {
        // Compile glob patterns
        let mut glob_matchers = Vec::new();
        for pattern in &query.glob_patterns {
            let matcher = if query.case_sensitive {
                WildMatch::new(pattern)
            } else {
                WildMatch::new(&pattern.to_lowercase())
            };
            glob_matchers.push(matcher);
        }

        // Compile regex if provided
        let regex = if let Some(ref pattern) = query.regex_pattern {
            Some(
                regex_lite::RegexBuilder::new(pattern)
                    .case_insensitive(!query.case_sensitive)
                    .build()
                    .map_err(|e| ToolError::InvalidQuery(format!("Invalid regex: {}", e)))?,
            )
        } else {
            None
        };

        Ok(CompiledFilters {
            glob_matchers,
            regex,
            size_filter: query.size_filter.clone(),
            file_type: query.file_type.clone(),
        })
    }

    /// Check if a path matches the compiled filters
    fn matches_filters(
        &self,
        path: &Path,
        metadata: &std::fs::Metadata,
        filters: &CompiledFilters,
        case_sensitive: bool,
    ) -> bool {
        // File type filtering
        match filters.file_type {
            FileTypeFilter::FilesOnly => {
                if !metadata.is_file() {
                    return false;
                }
            }
            FileTypeFilter::DirectoriesOnly => {
                if !metadata.is_dir() {
                    return false;
                }
            }
            FileTypeFilter::SymlinksOnly => {
                if !metadata.file_type().is_symlink() {
                    return false;
                }
            }
            FileTypeFilter::ExecutableOnly => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if metadata.permissions().mode() & 0o111 == 0 {
                        return false;
                    }
                }
                #[cfg(not(unix))]
                {
                    // On non-Unix systems, check file extension
                    if let Some(ext) = path.extension() {
                        let ext_str = ext.to_string_lossy().to_lowercase();
                        if !["exe", "bat", "cmd", "com"].contains(&ext_str.as_str()) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
            }
            FileTypeFilter::EmptyOnly => {
                if metadata.is_file() && metadata.len() > 0 {
                    return false;
                }
                if metadata.is_dir() {
                    // Check if directory is empty (expensive, but required for this filter)
                    if let Ok(mut entries) = std::fs::read_dir(path)
                        && entries.next().is_some()
                    {
                        return false;
                    }
                }
            }
            FileTypeFilter::All => {} // No filtering
        }

        // Size filtering
        if let Some(ref size_filter) = filters.size_filter {
            let file_size = metadata.len();
            if let Some(min_size) = size_filter.min_size
                && file_size < min_size
            {
                return false;
            }
            if let Some(max_size) = size_filter.max_size
                && file_size > max_size
            {
                return false;
            }
        }

        // Glob pattern matching
        if !filters.glob_matchers.is_empty() {
            let path_str = path.to_string_lossy();
            let test_str = if case_sensitive {
                path_str.as_ref()
            } else {
                &path_str.to_lowercase()
            };

            let matches_glob = filters
                .glob_matchers
                .iter()
                .any(|matcher| matcher.matches(test_str));

            if !matches_glob {
                return false;
            }
        }

        // Regex pattern matching
        if let Some(ref regex) = filters.regex {
            let path_str = path.to_string_lossy();
            if !regex.is_match(&path_str) {
                return false;
            }
        }

        true
    }

    /// Convert std::fs::Metadata to FdResult
    fn create_result(&self, path: PathBuf, metadata: std::fs::Metadata) -> FdResult {
        let file_type = if metadata.is_file() {
            FdFileType::File
        } else if metadata.is_dir() {
            FdFileType::Directory
        } else if metadata.file_type().is_symlink() {
            FdFileType::Symlink
        } else {
            FdFileType::Other
        };

        let size = if metadata.is_file() {
            Some(metadata.len())
        } else {
            None
        };

        let modified = metadata.modified().ok();

        let executable = {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                metadata.permissions().mode() & 0o111 != 0
            }
            #[cfg(not(unix))]
            {
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    ["exe", "bat", "cmd", "com"].contains(&ext_str.as_str())
                } else {
                    false
                }
            }
        };

        FdResult {
            path,
            file_type,
            size,
            modified,
            executable,
        }
    }

    /// Internal async search implementation
    fn search_internal(&self, mut query: FdQuery) -> Result<Vec<FdResult>, ToolError> {
        // Apply defaults
        if query.max_depth.is_none() {
            query.max_depth = self.default_max_depth;
        }
        if query.threads.is_none() {
            query.threads = self.default_threads;
        }

        // Validate base directory
        if !query.base_dir.exists() {
            return Err(ToolError::InvalidQuery(format!(
                "Base directory does not exist: {}",
                query.base_dir.display()
            )));
        }

        // Compile filters
        let filters = self.compile_filters(&query)?;

        // Set up search state
        let search_state = SearchState {
            results: Arc::new(Mutex::new(Vec::new())),
            cancelled: Arc::new(AtomicBool::new(false)),
            processed: Arc::new(AtomicUsize::new(0)),
            max_results: query.max_results,
            start_time: SystemTime::now(),
            timeout: query.timeout,
        };

        // Configure WalkBuilder
        let mut builder = WalkBuilder::new(&query.base_dir);
        builder
            .hidden(!query.include_hidden)
            .follow_links(query.follow_links)
            .git_ignore(true) // Respect .gitignore by default
            .git_exclude(true)
            .git_global(true);

        if let Some(max_depth) = query.max_depth {
            builder.max_depth(Some(max_depth));
        }

        // Configure parallelism
        let thread_count = query.threads.unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
        });
        builder.threads(thread_count);

        // Clone state for the walker closure
        let state_clone = search_state.clone();
        let filters = Arc::new(filters);
        let case_sensitive = query.case_sensitive;

        // Execute parallel walk
        builder.build_parallel().run(|| {
            let state = state_clone.clone();
            let filters = filters.clone();
            let fd_find = self.clone();

            Box::new(move |result| {
                // Check for cancellation
                if state.cancelled.load(Ordering::Relaxed) {
                    return WalkState::Quit;
                }

                // Check timeout
                if let Some(timeout) = state.timeout
                    && state.start_time.elapsed().unwrap_or(Duration::ZERO) > timeout
                {
                    state.cancelled.store(true, Ordering::Relaxed);
                    return WalkState::Quit;
                }

                match result {
                    Ok(entry) => {
                        let path = entry.path();

                        // Get metadata
                        let metadata = match entry.metadata() {
                            Ok(meta) => meta,
                            Err(_) => return WalkState::Continue,
                        };

                        // Apply filters
                        if fd_find.matches_filters(path, &metadata, &filters, case_sensitive) {
                            let result = fd_find.create_result(path.to_path_buf(), metadata);

                            // Add to results (thread-safe)
                            {
                                let mut results = match state.results.lock() {
                                    Ok(results) => results,
                                    Err(_) => return WalkState::Quit, // Poisoned mutex, abort search
                                };
                                results.push(result);

                                // Check max results limit
                                if state.max_results > 0 && results.len() >= state.max_results {
                                    state.cancelled.store(true, Ordering::Relaxed);
                                    return WalkState::Quit;
                                }
                            }
                        }

                        // Update processed count
                        state.processed.fetch_add(1, Ordering::Relaxed);
                        WalkState::Continue
                    }
                    Err(_) => WalkState::Continue, // Skip errors, continue walking
                }
            })
        });

        // Extract final results
        // Use clone and lock instead of try_unwrap to avoid Arc reference issues
        let mut results = search_state.results.lock()
            .map_err(|_| ToolError::InvalidQuery("Search results mutex was poisoned".to_string()))?
            .clone();

        // Enforce max_results limit (in case parallel threads added extra results)
        if query.max_results > 0 && results.len() > query.max_results {
            results.truncate(query.max_results);
        }

        Ok(results)
    }
}

/// Content type categories for easier searching
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentType {
    /// Source code files
    Source,
    /// Configuration files
    Config,
    /// Documentation files
    Documentation,
    /// Build system files
    Build,
}

/// Implement SearchState Clone for the closure
impl Clone for SearchState {
    fn clone(&self) -> Self {
        Self {
            results: self.results.clone(),
            cancelled: self.cancelled.clone(),
            processed: self.processed.clone(),
            max_results: self.max_results,
            start_time: self.start_time,
            timeout: self.timeout,
        }
    }
}

/// CodeTool implementation for FdFind
impl CodeTool for FdFind {
    type Query = FdQuery;
    type Output = Vec<FdResult>;

    fn search(&self, query: Self::Query) -> Result<Self::Output, ToolError> {
        self.search_internal(query)
    }
}

/// Legacy compatibility: Convert FdResult to String paths
impl FdFind {
    /// Search and return only file paths (for legacy compatibility)
    pub fn search_paths(&self, query: FdQuery) -> Result<Vec<String>, ToolError> {
        let results = self.search(query)?;
        Ok(results
            .into_iter()
            .map(|r| r.path.to_string_lossy().to_string())
            .collect())
    }
}

/// Builder pattern for FdQuery construction
impl FdQuery {
    /// Create a new query for the given base directory
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
            ..Default::default()
        }
    }

    /// Add a glob pattern
    pub fn glob(mut self, pattern: &str) -> Self {
        self.glob_patterns.push(pattern.to_string());
        self
    }

    /// Add multiple glob patterns
    pub fn globs(mut self, patterns: &[&str]) -> Self {
        self.glob_patterns
            .extend(patterns.iter().map(|s| (*s).to_string()));
        self
    }

    /// Set regex pattern
    pub fn regex(mut self, pattern: &str) -> Self {
        self.regex_pattern = Some(pattern.to_string());
        self
    }

    /// Set file type filter
    pub const fn file_type(mut self, file_type: FileTypeFilter) -> Self {
        self.file_type = file_type;
        self
    }

    /// Set size filter
    pub const fn size_range(mut self, min: Option<u64>, max: Option<u64>) -> Self {
        self.size_filter = Some(SizeFilter {
            min_size: min,
            max_size: max,
        });
        self
    }

    /// Set maximum search depth
    pub const fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Include hidden files
    pub const fn include_hidden(mut self, include: bool) -> Self {
        self.include_hidden = include;
        self
    }

    /// Follow symbolic links
    pub const fn follow_links(mut self, follow: bool) -> Self {
        self.follow_links = follow;
        self
    }

    /// Set case sensitivity
    pub const fn case_sensitive(mut self, sensitive: bool) -> Self {
        self.case_sensitive = sensitive;
        self
    }

    /// Limit maximum results
    pub const fn max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }

    /// Set search timeout
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set number of threads
    pub const fn threads(mut self, threads: usize) -> Self {
        self.threads = Some(threads);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_structure() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // Create test directory structure
        fs::create_dir_all(base.join("src")).unwrap();
        fs::create_dir_all(base.join("target/debug")).unwrap();
        fs::create_dir_all(base.join(".git")).unwrap();
        fs::create_dir_all(base.join("docs")).unwrap();

        // Create test files
        fs::write(base.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(base.join("src/lib.rs"), "pub mod test;").unwrap();
        fs::write(base.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        fs::write(base.join("README.md"), "# Test Project").unwrap();
        fs::write(base.join(".gitignore"), "target/").unwrap();
        fs::write(base.join("target/debug/test"), "binary").unwrap();
        fs::write(base.join("docs/guide.md"), "# Guide").unwrap();

        // Create hidden file
        fs::write(base.join(".hidden"), "hidden content").unwrap();

        temp_dir
    }

    #[test]
    fn test_basic_search() {
        let temp_dir = create_test_structure();
        let fd_find = FdFind::new();

        let query = FdQuery::new(temp_dir.path()).file_type(FileTypeFilter::FilesOnly);

        let results = fd_find.search(query).unwrap();
        assert!(!results.is_empty());

        // Should find regular files but not the target directory (due to .gitignore)
        let paths: Vec<_> = results.iter().map(|r| &r.path).collect();
        assert!(paths.iter().any(|p| p.file_name().unwrap() == "main.rs"));
        assert!(paths.iter().any(|p| p.file_name().unwrap() == "Cargo.toml"));
    }

    #[test]
    fn test_glob_pattern_search() {
        let temp_dir = create_test_structure();
        let fd_find = FdFind::new();

        let query = FdQuery::new(temp_dir.path())
            .globs(&["*.rs", "*.toml"])
            .file_type(FileTypeFilter::FilesOnly);

        let results = fd_find.search(query).unwrap();

        for result in &results {
            let filename = result.path.file_name().unwrap().to_string_lossy();
            assert!(filename.ends_with(".rs") || filename.ends_with(".toml"));
        }
    }

    #[test]
    fn test_find_files_by_extension() {
        let temp_dir = create_test_structure();
        let fd_find = FdFind::new();

        let results = fd_find
            .find_files_by_extension(temp_dir.path(), &["rs", "md"])
            .unwrap();

        assert!(!results.is_empty());
        for result in &results {
            let ext = result.path.extension().unwrap().to_string_lossy();
            assert!(ext == "rs" || ext == "md");
        }
    }

    #[test]
    fn test_find_directories_by_name() {
        let temp_dir = create_test_structure();
        let fd_find = FdFind::new();

        let results = fd_find
            .find_directories_by_name(temp_dir.path(), "src")
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].file_type, FdFileType::Directory);
        assert_eq!(results[0].path.file_name().unwrap(), "src");
    }

    #[test]
    fn test_regex_search() {
        let temp_dir = create_test_structure();
        let fd_find = FdFind::new();

        let query = FdQuery::new(temp_dir.path())
            .regex(r".*\.(rs|toml)$")
            .file_type(FileTypeFilter::FilesOnly);

        let results = fd_find.search(query).unwrap();

        for result in &results {
            let filename = result.path.to_string_lossy();
            assert!(filename.ends_with(".rs") || filename.ends_with(".toml"));
        }
    }

    #[test]
    fn test_max_results_limit() {
        let temp_dir = create_test_structure();
        let fd_find = FdFind::new();

        let query = FdQuery::new(temp_dir.path())
            .file_type(FileTypeFilter::All)
            .max_results(3);

        let results = fd_find.search(query).unwrap();
        assert!(results.len() <= 3);
    }

    #[test]
    fn test_hidden_files() {
        let temp_dir = create_test_structure();
        let fd_find = FdFind::new();

        // Search without hidden files
        let query_no_hidden = FdQuery::new(temp_dir.path())
            .file_type(FileTypeFilter::FilesOnly)
            .include_hidden(false);

        let results_no_hidden = fd_find.search(query_no_hidden).unwrap();
        let hidden_found = results_no_hidden
            .iter()
            .any(|r| r.path.file_name().unwrap() == ".hidden");
        assert!(!hidden_found);

        // Search with hidden files
        let query_with_hidden = FdQuery::new(temp_dir.path())
            .file_type(FileTypeFilter::FilesOnly)
            .include_hidden(true);

        let results_with_hidden = fd_find.search(query_with_hidden).unwrap();
        let hidden_found = results_with_hidden
            .iter()
            .any(|r| r.path.file_name().unwrap() == ".hidden");
        assert!(hidden_found);
    }

    #[test]
    fn test_depth_limiting() {
        let temp_dir = create_test_structure();
        let fd_find = FdFind::new();

        let query = FdQuery::new(temp_dir.path())
            .file_type(FileTypeFilter::All)
            .max_depth(1); // Only immediate children

        let results = fd_find.search(query).unwrap();

        // Should not find files in subdirectories
        let deep_file_found = results
            .iter()
            .any(|r| r.path.file_name().unwrap() == "main.rs");
        assert!(!deep_file_found);
    }

    #[test]
    fn test_size_filtering() {
        let temp_dir = create_test_structure();
        let fd_find = FdFind::new();

        // Create a large file for testing
        let large_content = "x".repeat(1024); // 1KB
        fs::write(temp_dir.path().join("large.txt"), &large_content).unwrap();

        let query = FdQuery::new(temp_dir.path())
            .file_type(FileTypeFilter::FilesOnly)
            .size_range(Some(500), Some(2000)); // 500B to 2KB

        let results = fd_find.search(query).unwrap();

        // Should find the large file
        let large_found = results
            .iter()
            .any(|r| r.path.file_name().unwrap() == "large.txt");
        assert!(large_found);

        // Verify size information
        let large_result = results
            .iter()
            .find(|r| r.path.file_name().unwrap() == "large.txt")
            .unwrap();
        assert!(large_result.size.unwrap() >= 500);
    }

    #[test]
    fn test_content_type_search() {
        let temp_dir = create_test_structure();
        let fd_find = FdFind::new();

        let results = fd_find
            .find_by_content_type(temp_dir.path(), ContentType::Source)
            .unwrap();

        // Should find .rs files
        let rust_found = results.iter().any(|r| r.path.extension().unwrap() == "rs");
        assert!(rust_found);
    }

    #[test]
    fn test_case_sensitivity() {
        let temp_dir = create_test_structure();

        // Create files with different cases
        fs::write(temp_dir.path().join("Test.TXT"), "content").unwrap();
        fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

        let fd_find = FdFind::new();

        // Case sensitive search
        let query_sensitive = FdQuery::new(temp_dir.path())
            .glob("*.TXT")
            .case_sensitive(true);

        let results_sensitive = fd_find.search(query_sensitive).unwrap();
        assert_eq!(results_sensitive.len(), 1);
        assert_eq!(results_sensitive[0].path.file_name().unwrap(), "Test.TXT");

        // Case insensitive search
        let query_insensitive = FdQuery::new(temp_dir.path())
            .glob("*.txt")
            .case_sensitive(false);

        let results_insensitive = fd_find.search(query_insensitive).unwrap();
        assert_eq!(results_insensitive.len(), 2);
    }

    #[test]
    fn test_builder_pattern() {
        let temp_dir = create_test_structure();
        let fd_find = FdFind::new();

        let query = FdQuery::new(temp_dir.path())
            .globs(&["*.rs", "*.toml"])
            .file_type(FileTypeFilter::FilesOnly)
            .max_depth(5)
            .include_hidden(false)
            .case_sensitive(true)
            .max_results(10)
            .timeout(Duration::from_secs(5));

        let results = fd_find.search(query).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_error_handling() {
        let fd_find = FdFind::new();

        // Test non-existent directory
        let query = FdQuery::new("/non/existent/path");
        let result = fd_find.search(query);
        assert!(result.is_err());

        // Test invalid regex
        let temp_dir = create_test_structure();
        let query = FdQuery::new(temp_dir.path()).regex("[invalid regex");
        let result = fd_find.search(query);
        assert!(result.is_err());
    }
}
