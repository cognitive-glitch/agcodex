//! Unified traits and interfaces for code tools integration.
//!
//! This module provides the foundational traits that all code tools implement,
//! enabling polymorphic usage, result streaming, and consistent error handling.

use async_trait::async_trait;
use futures::Stream;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use tokio::process::Child;
use tokio_util::sync::CancellationToken;

use super::ToolError;

/// Represents different types of search operations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchType {
    /// Text-based content search
    Content,
    /// File and directory name search  
    Files,
    /// AST-based structural search
    Structural,
    /// Hybrid search combining multiple approaches
    Hybrid,
}

/// Configuration for search operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Maximum results to return (None = unlimited)
    pub max_results: Option<usize>,
    /// Search timeout in milliseconds
    pub timeout_ms: Option<u64>,
    /// Case sensitivity
    pub case_sensitive: bool,
    /// Include hidden files/directories
    pub include_hidden: bool,
    /// Follow symbolic links
    pub follow_symlinks: bool,
    /// Additional environment variables
    pub env_vars: HashMap<String, String>,
    /// Working directory for search
    pub working_dir: Option<PathBuf>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_results: None,
            timeout_ms: Some(30_000), // 30 seconds default timeout
            case_sensitive: false,
            include_hidden: false,
            follow_symlinks: false,
            env_vars: HashMap::new(),
            working_dir: None,
        }
    }
}

/// Result of a search operation with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Path where match was found
    pub path: PathBuf,
    /// Line number (1-indexed, None for file-only results)
    pub line_number: Option<usize>,
    /// Column number (1-indexed, None if not available)
    pub column_number: Option<usize>,
    /// Matched content or context
    pub content: Option<String>,
    /// Match score/relevance (0.0-1.0)
    pub score: f64,
    /// Additional metadata specific to the tool
    pub metadata: HashMap<String, String>,
}

/// Streaming search results with progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingSearchResult {
    /// The actual search result
    pub result: Option<SearchResult>,
    /// Progress indication (0.0-1.0, None if unknown)
    pub progress: Option<f64>,
    /// Status message for the user
    pub status: String,
    /// Whether this is the final result
    pub is_final: bool,
}

/// Statistics about a completed search operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchStats {
    /// Total files examined
    pub files_examined: usize,
    /// Total matches found
    pub matches_found: usize,
    /// Time taken in milliseconds
    pub duration_ms: u64,
    /// Memory usage peak in bytes
    pub memory_peak_bytes: Option<usize>,
    /// Whether the search was cancelled
    pub was_cancelled: bool,
    /// Tool-specific statistics
    pub tool_specific: HashMap<String, String>,
}

/// Unified search interface that all code tools implement
#[async_trait]
pub trait UnifiedSearch: Send + Sync + fmt::Debug {
    /// The query type this tool accepts
    type Query: Send + Sync + fmt::Debug;

    /// Get the search type this tool supports
    fn search_type(&self) -> SearchType;

    /// Get the name of this tool
    fn tool_name(&self) -> &'static str;

    /// Check if the tool is available on the system
    async fn is_available(&self) -> Result<bool, ToolError>;

    /// Execute a search and return streaming results
    async fn search_streaming(
        &self,
        query: Self::Query,
        config: SearchConfig,
        cancel_token: CancellationToken,
    ) -> Result<
        Box<dyn Stream<Item = Result<StreamingSearchResult, ToolError>> + Send + Unpin>,
        ToolError,
    >;

    /// Execute a search and collect all results
    async fn search(
        &self,
        query: Self::Query,
        config: SearchConfig,
        cancel_token: CancellationToken,
    ) -> Result<Vec<SearchResult>, ToolError> {
        use futures::StreamExt;

        let mut stream = self.search_streaming(query, config, cancel_token).await?;
        let mut results = Vec::new();

        while let Some(item) = stream.next().await {
            match item? {
                StreamingSearchResult {
                    result: Some(r),
                    is_final: true,
                    ..
                } => {
                    results.push(r);
                    break;
                }
                StreamingSearchResult {
                    result: Some(r), ..
                } => {
                    results.push(r);
                }
                _ => {}
            }
        }

        Ok(results)
    }

    /// Get statistics from the last search operation
    async fn get_last_search_stats(&self) -> Option<SearchStats>;

    /// Validate a query before execution
    fn validate_query(&self, query: &Self::Query) -> Result<(), ToolError>;

    /// Get the default configuration for this tool
    fn default_config(&self) -> SearchConfig {
        SearchConfig::default()
    }
}

/// Tool discovery and management trait
#[async_trait]
pub trait ToolDiscovery: Send + Sync {
    /// Discover available tools on the system
    async fn discover_tools() -> Result<Vec<String>, ToolError>;

    /// Get the version of a specific tool
    async fn get_tool_version(tool_name: &str) -> Result<String, ToolError>;

    /// Get the path to a tool executable
    async fn get_tool_path(tool_name: &str) -> Result<PathBuf, ToolError>;

    /// Validate that all required tools are available
    async fn validate_tools(required: &[&str]) -> Result<(), ToolError>;
}

/// Command execution abstraction for better testing and sandboxing
#[async_trait]
pub trait CommandExecutor: Send + Sync {
    /// Execute a command and return the child process
    async fn execute(
        &self,
        command: &str,
        args: &[&str],
        config: &SearchConfig,
    ) -> Result<Child, ToolError>;

    /// Check if a command is allowed by security policy
    fn is_command_allowed(&self, command: &str, args: &[&str]) -> bool;

    /// Get environment variables for command execution
    fn get_env_vars(&self, config: &SearchConfig) -> HashMap<String, String>;
}

/// Result caching interface for performance optimization
#[async_trait]
pub trait ResultCache: Send + Sync {
    /// Get cached results for a query hash
    async fn get(&self, query_hash: &str) -> Option<Vec<SearchResult>>;

    /// Store results in cache with TTL
    async fn put(&self, query_hash: &str, results: Vec<SearchResult>, ttl_secs: u64);

    /// Clear all cached results
    async fn clear(&self);

    /// Get cache statistics
    async fn stats(&self) -> HashMap<String, String>;
}

/// Progress reporting for long-running operations
pub trait ProgressReporter: Send + Sync {
    /// Report progress with a message
    fn report_progress(&self, progress: f64, message: &str);

    /// Report an error during operation
    fn report_error(&self, error: &ToolError);

    /// Report completion
    fn report_completion(&self, stats: &SearchStats);
}

/// Builder pattern for complex search configurations
#[derive(Debug, Clone)]
pub struct SearchConfigBuilder {
    config: SearchConfig,
}

impl SearchConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: SearchConfig::default(),
        }
    }

    pub const fn max_results(mut self, max: usize) -> Self {
        self.config.max_results = Some(max);
        self
    }

    pub const fn timeout_ms(mut self, timeout: u64) -> Self {
        self.config.timeout_ms = Some(timeout);
        self
    }

    pub const fn case_sensitive(mut self, sensitive: bool) -> Self {
        self.config.case_sensitive = sensitive;
        self
    }

    pub const fn include_hidden(mut self, hidden: bool) -> Self {
        self.config.include_hidden = hidden;
        self
    }

    pub const fn follow_symlinks(mut self, follow: bool) -> Self {
        self.config.follow_symlinks = follow;
        self
    }

    pub fn env_var(mut self, key: String, value: String) -> Self {
        self.config.env_vars.insert(key, value);
        self
    }

    pub fn working_dir(mut self, dir: PathBuf) -> Self {
        self.config.working_dir = Some(dir);
        self
    }

    pub fn build(self) -> SearchConfig {
        self.config
    }
}

impl Default for SearchConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_config_builder() {
        let config = SearchConfigBuilder::new()
            .max_results(100)
            .timeout_ms(5000)
            .case_sensitive(true)
            .include_hidden(true)
            .env_var("CUSTOM".to_string(), "value".to_string())
            .build();

        assert_eq!(config.max_results, Some(100));
        assert_eq!(config.timeout_ms, Some(5000));
        assert!(config.case_sensitive);
        assert!(config.include_hidden);
        assert_eq!(config.env_vars.get("CUSTOM"), Some(&"value".to_string()));
    }

    #[test]
    fn test_search_result_creation() {
        let result = SearchResult {
            path: PathBuf::from("/test/file.rs"),
            line_number: Some(42),
            column_number: Some(10),
            content: Some("fn test() {".to_string()),
            score: 0.95,
            metadata: HashMap::new(),
        };

        assert_eq!(result.path, PathBuf::from("/test/file.rs"));
        assert_eq!(result.line_number, Some(42));
        assert!((result.score - 0.95).abs() < f64::EPSILON);
    }
}
