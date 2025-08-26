//! SRGN (Syntactic Regex with Named Groups) integration.
//!
//! This module provides integration with srgn for advanced text manipulation:
//! - Syntax-aware search and replace
//! - Named capture groups for complex transformations
//! - Language-specific processing
//! - AST-aware replacements
//! - Integration with existing safety/sandbox systems

use async_trait::async_trait;
use futures::Stream;
use futures::stream;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Stdio;
use std::task::Context;
use std::task::Poll;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;

use tokio::process::Child;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;
use tracing::error;
use tracing::info;
use which::which;

use super::CodeTool;
use super::ToolError;
use super::traits::*;
use crate::is_safe_command::is_known_safe_command;

/// SRGN operation types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SrgnOperation {
    /// Search and replace with regex
    Replace {
        /// Search pattern (regex with optional named groups)
        pattern: String,
        /// Replacement template
        replacement: String,
    },
    /// Extract matches only
    Extract {
        /// Pattern to extract
        pattern: String,
        /// Output format template
        format: Option<String>,
    },
    /// Validate syntax without modification
    Validate {
        /// Pattern to validate
        pattern: String,
    },
}

/// Language modes supported by SRGN
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SrgnLanguage {
    /// Auto-detect language from file extension
    Auto,
    /// Rust language
    Rust,
    /// Python language
    Python,
    /// JavaScript/TypeScript
    JavaScript,
    /// TypeScript
    TypeScript,
    /// Go language
    Go,
    /// Java language
    Java,
    /// C language
    C,
    /// C++ language
    Cpp,
    /// C# language
    CSharp,
    /// Custom language with tree-sitter grammar
    Custom(String),
}

impl SrgnLanguage {
    /// Convert to srgn command line argument
    pub fn to_arg(&self) -> Option<String> {
        match self {
            Self::Auto => None,
            Self::Rust => Some("rust".to_string()),
            Self::Python => Some("python".to_string()),
            Self::JavaScript => Some("javascript".to_string()),
            Self::TypeScript => Some("typescript".to_string()),
            Self::Go => Some("go".to_string()),
            Self::Java => Some("java".to_string()),
            Self::C => Some("c".to_string()),
            Self::Cpp => Some("cpp".to_string()),
            Self::CSharp => Some("c_sharp".to_string()),
            Self::Custom(name) => Some(name.clone()),
        }
    }
}

/// SRGN query configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SrgnQuery {
    /// The operation to perform
    pub operation: SrgnOperation,
    /// Input files or directories
    pub inputs: Vec<PathBuf>,
    /// Target language for syntax-aware processing
    pub language: SrgnLanguage,
    /// File patterns to include
    pub include_patterns: Vec<String>,
    /// File patterns to exclude
    pub exclude_patterns: Vec<String>,
    /// Perform dry run without modifying files
    pub dry_run: bool,
    /// Create backup files before modification
    pub backup: bool,
    /// Recursive directory processing
    pub recursive: bool,
    /// Follow symbolic links
    pub follow_symlinks: bool,
    /// Maximum depth for recursive processing
    pub max_depth: Option<usize>,
    /// Additional SRGN flags
    pub additional_flags: Vec<String>,
}

impl Default for SrgnQuery {
    fn default() -> Self {
        Self {
            operation: SrgnOperation::Extract {
                pattern: String::new(),
                format: None,
            },
            inputs: vec![],
            language: SrgnLanguage::Auto,
            include_patterns: vec![],
            exclude_patterns: vec![],
            dry_run: false,
            backup: false,
            recursive: true,
            follow_symlinks: false,
            max_depth: None,
            additional_flags: vec![],
        }
    }
}

/// Builder for SRGN queries
#[derive(Debug, Clone)]
pub struct SrgnQueryBuilder {
    query: SrgnQuery,
}

impl SrgnQueryBuilder {
    /// Create a new builder with a replace operation
    pub fn replace(pattern: impl Into<String>, replacement: impl Into<String>) -> Self {
        Self {
            query: SrgnQuery {
                operation: SrgnOperation::Replace {
                    pattern: pattern.into(),
                    replacement: replacement.into(),
                },
                ..Default::default()
            },
        }
    }

    /// Create a new builder with an extract operation
    pub fn extract(pattern: impl Into<String>) -> Self {
        Self {
            query: SrgnQuery {
                operation: SrgnOperation::Extract {
                    pattern: pattern.into(),
                    format: None,
                },
                ..Default::default()
            },
        }
    }

    /// Create a new builder with a validate operation
    pub fn validate(pattern: impl Into<String>) -> Self {
        Self {
            query: SrgnQuery {
                operation: SrgnOperation::Validate {
                    pattern: pattern.into(),
                },
                ..Default::default()
            },
        }
    }

    /// Set the target language
    pub fn language(mut self, lang: SrgnLanguage) -> Self {
        self.query.language = lang;
        self
    }

    /// Add input paths
    pub fn inputs<P: AsRef<Path>>(mut self, paths: impl IntoIterator<Item = P>) -> Self {
        self.query.inputs = paths
            .into_iter()
            .map(|p| p.as_ref().to_path_buf())
            .collect();
        self
    }

    /// Add a single input path
    pub fn input<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.query.inputs.push(path.as_ref().to_path_buf());
        self
    }

    /// Add include pattern
    pub fn include_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.query.include_patterns.push(pattern.into());
        self
    }

    /// Add exclude pattern
    pub fn exclude_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.query.exclude_patterns.push(pattern.into());
        self
    }

    /// Enable dry run mode
    pub const fn dry_run(mut self) -> Self {
        self.query.dry_run = true;
        self
    }

    /// Enable backup creation
    pub const fn backup(mut self) -> Self {
        self.query.backup = true;
        self
    }

    /// Disable recursive processing
    pub const fn no_recursive(mut self) -> Self {
        self.query.recursive = false;
        self
    }

    /// Follow symbolic links
    pub const fn follow_symlinks(mut self) -> Self {
        self.query.follow_symlinks = true;
        self
    }

    /// Set maximum recursion depth
    pub const fn max_depth(mut self, depth: usize) -> Self {
        self.query.max_depth = Some(depth);
        self
    }

    /// Add additional flag
    pub fn flag(mut self, flag: impl Into<String>) -> Self {
        self.query.additional_flags.push(flag.into());
        self
    }

    /// Set extract format template
    pub fn extract_format(mut self, format: impl Into<String>) -> Self {
        if let SrgnOperation::Extract { ref mut format, .. } = self.query.operation {
            *format = Some(format.as_ref().unwrap().clone());
        }
        self
    }

    /// Build the query
    pub fn build(self) -> SrgnQuery {
        self.query
    }
}

/// Result from SRGN operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SrgnResult {
    /// Path that was processed
    pub path: PathBuf,
    /// Number of matches/replacements made
    pub match_count: usize,
    /// Whether the file was modified
    pub modified: bool,
    /// Extracted content (for extract operations)
    pub extracted: Option<Vec<String>>,
    /// Error message if processing failed
    pub error: Option<String>,
}

/// Streaming wrapper for SRGN output
struct SrgnStream {
    child: Child,
    lines_stream: Pin<Box<dyn Stream<Item = std::io::Result<String>> + Send>>,
    stats: SearchStats,
    cancel_token: CancellationToken,
    is_finished: bool,
}

impl SrgnStream {
    fn new(mut child: Child, cancel_token: CancellationToken) -> Result<Self, ToolError> {
        let stdout = child.stdout.take().ok_or_else(|| {
            ToolError::Io(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "Failed to capture srgn stdout",
            ))
        })?;

        let reader = BufReader::new(stdout);
        let lines_stream = Box::pin(stream::unfold(reader, |mut reader| async move {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(0) => None, // EOF
                Ok(_) => Some((Ok(line), reader)),
                Err(e) => Some((Err(e), reader)),
            }
        }));

        Ok(Self {
            child,
            lines_stream,
            stats: SearchStats {
                files_examined: 0,
                matches_found: 0,
                duration_ms: 0,
                memory_peak_bytes: None,
                was_cancelled: false,
                tool_specific: HashMap::new(),
            },
            cancel_token,
            is_finished: false,
        })
    }
}

impl Stream for SrgnStream {
    type Item = Result<StreamingSearchResult, ToolError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.is_finished {
            return Poll::Ready(None);
        }

        // Check for cancellation
        if self.cancel_token.is_cancelled() {
            self.stats.was_cancelled = true;
            self.is_finished = true;

            // Kill the child process
            let _ = self.child.kill();

            return Poll::Ready(Some(Ok(StreamingSearchResult {
                result: None,
                progress: None,
                status: "SRGN operation cancelled".to_string(),
                is_final: true,
            })));
        }

        // Poll the lines stream for the next line
        match self.lines_stream.as_mut().poll_next(cx) {
            Poll::Ready(None) => {
                // EOF reached
                self.is_finished = true;

                Poll::Ready(Some(Ok(StreamingSearchResult {
                    result: None,
                    progress: Some(1.0),
                    status: format!(
                        "SRGN completed: {} matches in {} files",
                        self.stats.matches_found, self.stats.files_examined
                    ),
                    is_final: true,
                })))
            }
            Poll::Ready(Some(Ok(line))) => {
                let line = line.trim();

                // Parse SRGN output format
                // SRGN typically outputs: <path>: <match_info>
                if let Some((path_str, info)) = line.split_once(':') {
                    self.stats.files_examined += 1;

                    // Count matches/replacements mentioned in the info
                    let match_count =
                        info.matches("match").count() + info.matches("replace").count();
                    self.stats.matches_found += match_count;

                    let result = SearchResult {
                        path: PathBuf::from(path_str.trim()),
                        line_number: None, // SRGN doesn't typically report line numbers
                        column_number: None,
                        content: Some(info.trim().to_string()),
                        score: 1.0,
                        metadata: {
                            let mut meta = HashMap::new();
                            meta.insert("match_count".to_string(), match_count.to_string());
                            meta
                        },
                    };

                    Poll::Ready(Some(Ok(StreamingSearchResult {
                        result: Some(result),
                        progress: None,
                        status: format!("Processed: {}", path_str.trim()),
                        is_final: false,
                    })))
                } else {
                    // Status line or other output
                    Poll::Ready(Some(Ok(StreamingSearchResult {
                        result: None,
                        progress: None,
                        status: line.to_string(),
                        is_final: false,
                    })))
                }
            }
            Poll::Ready(Some(Err(e))) => {
                error!("Error reading from srgn: {}", e);
                self.is_finished = true;
                Poll::Ready(Some(Err(ToolError::Io(e))))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// SRGN (Syntactic Regex with Named Groups) integration
#[derive(Debug, Clone)]
pub struct Srgn {
    /// Path to the srgn binary
    srgn_path: PathBuf,
    /// Last search statistics
    last_stats: Option<SearchStats>,
}

impl Srgn {
    /// Create a new SRGN instance
    pub async fn new() -> Result<Self, ToolError> {
        let srgn_path =
            which("srgn").map_err(|_| ToolError::NotFound("srgn not found in PATH".to_string()))?;

        Ok(Self {
            srgn_path,
            last_stats: None,
        })
    }

    /// Build command arguments for SRGN
    fn build_command(&self, query: &SrgnQuery, config: &SearchConfig) -> Vec<String> {
        let mut args = vec![];

        // Operation-specific arguments
        match &query.operation {
            SrgnOperation::Replace {
                pattern,
                replacement,
            } => {
                args.push("replace".to_string());
                args.push(pattern.clone());
                args.push(replacement.clone());
            }
            SrgnOperation::Extract { pattern, format } => {
                args.push("extract".to_string());
                args.push(pattern.clone());
                if let Some(fmt) = format {
                    args.push("--format".to_string());
                    args.push(fmt.clone());
                }
            }
            SrgnOperation::Validate { pattern } => {
                args.push("validate".to_string());
                args.push(pattern.clone());
            }
        }

        // Language specification
        if let Some(lang) = query.language.to_arg() {
            args.push("--language".to_string());
            args.push(lang);
        }

        // Include patterns
        for pattern in &query.include_patterns {
            args.push("--include".to_string());
            args.push(pattern.clone());
        }

        // Exclude patterns
        for pattern in &query.exclude_patterns {
            args.push("--exclude".to_string());
            args.push(pattern.clone());
        }

        // Mode flags
        if query.dry_run {
            args.push("--dry-run".to_string());
        }

        if query.backup {
            args.push("--backup".to_string());
        }

        if !query.recursive {
            args.push("--no-recursive".to_string());
        }

        if query.follow_symlinks {
            args.push("--follow-symlinks".to_string());
        }

        if let Some(depth) = query.max_depth {
            args.push("--max-depth".to_string());
            args.push(depth.to_string());
        }

        // Hidden files
        if config.include_hidden {
            args.push("--hidden".to_string());
        }

        // Timeout
        if let Some(timeout) = config.timeout_ms {
            args.push("--timeout".to_string());
            args.push((timeout / 1000).to_string()); // Convert to seconds
        }

        // Max results
        if let Some(max) = config.max_results {
            args.push("--max-results".to_string());
            args.push(max.to_string());
        }

        // Additional flags
        args.extend(query.additional_flags.clone());

        // Input paths
        if query.inputs.is_empty() {
            args.push(".".to_string()); // Current directory
        } else {
            for path in &query.inputs {
                args.push(path.to_string_lossy().to_string());
            }
        }

        args
    }
}

impl CodeTool for Srgn {
    type Query = SrgnQuery;
    type Output = Vec<SearchResult>;

    fn search(&self, query: Self::Query) -> Result<Self::Output, ToolError> {
        // Sync adapter for async implementation
        tokio::runtime::Handle::current().block_on(async {
            let config = SearchConfig::default();
            let cancel_token = CancellationToken::new();

            <Self as UnifiedSearch>::search(self, query, config, cancel_token).await
        })
    }
}

#[async_trait]
impl UnifiedSearch for Srgn {
    type Query = SrgnQuery;

    fn search_type(&self) -> SearchType {
        SearchType::Structural // SRGN is syntax-aware
    }

    fn tool_name(&self) -> &'static str {
        "srgn"
    }

    async fn is_available(&self) -> Result<bool, ToolError> {
        Ok(self.srgn_path.exists())
    }

    async fn search_streaming(
        &self,
        query: Self::Query,
        config: SearchConfig,
        cancel_token: CancellationToken,
    ) -> Result<
        Box<dyn Stream<Item = Result<StreamingSearchResult, ToolError>> + Send + Unpin>,
        ToolError,
    > {
        // Validate the query
        self.validate_query(&query)?;

        let args = self.build_command(&query, &config);

        // Security check
        let command_args: Vec<String> = std::iter::once("srgn".to_string())
            .chain(args.iter().cloned())
            .collect();
        if !is_known_safe_command(&command_args) {
            return Err(ToolError::InvalidQuery(
                "SRGN command rejected by security policy".to_string(),
            ));
        }

        // Execute the command
        let mut cmd = Command::new(&self.srgn_path);
        cmd.args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        // Set working directory
        if let Some(cwd) = &config.working_dir {
            cmd.current_dir(cwd);
        }

        // Set environment variables
        for (key, value) in &config.env_vars {
            cmd.env(key, value);
        }

        info!(
            "Executing srgn: {} {}",
            self.srgn_path.display(),
            args.join(" ")
        );

        let child = cmd.spawn().map_err(ToolError::Io)?;

        let stream = SrgnStream::new(child, cancel_token)?;
        Ok(Box::new(stream))
    }

    async fn get_last_search_stats(&self) -> Option<SearchStats> {
        self.last_stats.clone()
    }

    fn validate_query(&self, query: &Self::Query) -> Result<(), ToolError> {
        // Validate operation pattern
        let pattern = match &query.operation {
            SrgnOperation::Replace {
                pattern,
                replacement,
            } => {
                if replacement.is_empty() {
                    return Err(ToolError::InvalidQuery(
                        "Replacement string cannot be empty".to_string(),
                    ));
                }
                pattern
            }
            SrgnOperation::Extract { pattern, .. } => pattern,
            SrgnOperation::Validate { pattern } => pattern,
        };

        if pattern.is_empty() {
            return Err(ToolError::InvalidQuery(
                "Pattern cannot be empty".to_string(),
            ));
        }

        // Validate input paths exist
        for path in &query.inputs {
            if !path.exists() {
                return Err(ToolError::InvalidQuery(format!(
                    "Input path does not exist: {}",
                    path.display()
                )));
            }
        }

        Ok(())
    }
}

#[async_trait]
impl ToolDiscovery for Srgn {
    async fn discover_tools() -> Result<Vec<String>, ToolError> {
        match which("srgn") {
            Ok(_) => Ok(vec!["srgn".to_string()]),
            Err(_) => Ok(vec![]),
        }
    }

    async fn get_tool_version(tool_name: &str) -> Result<String, ToolError> {
        if tool_name != "srgn" {
            return Err(ToolError::NotFound(format!(
                "Tool not managed by this provider: {}",
                tool_name
            )));
        }

        let output = Command::new("srgn")
            .arg("--version")
            .output()
            .await
            .map_err(ToolError::Io)?;

        if !output.status.success() {
            return Err(ToolError::InvalidQuery(
                "Failed to get srgn version".to_string(),
            ));
        }

        let version_text = String::from_utf8_lossy(&output.stdout);
        Ok(version_text.lines().next().unwrap_or("Unknown").to_string())
    }

    async fn get_tool_path(tool_name: &str) -> Result<PathBuf, ToolError> {
        if tool_name != "srgn" {
            return Err(ToolError::NotFound(format!(
                "Tool not managed by this provider: {}",
                tool_name
            )));
        }

        which("srgn").map_err(|_| ToolError::NotFound("srgn not found in PATH".to_string()))
    }

    async fn validate_tools(required: &[&str]) -> Result<(), ToolError> {
        for &tool in required {
            if tool == "srgn" {
                Self::get_tool_path(tool).await?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srgn_query_builder_replace() {
        let query = SrgnQueryBuilder::replace("old_pattern", "new_replacement")
            .language(SrgnLanguage::Rust)
            .inputs(["src"])
            .include_pattern("*.rs")
            .dry_run()
            .backup()
            .build();

        match query.operation {
            SrgnOperation::Replace {
                pattern,
                replacement,
            } => {
                assert_eq!(pattern, "old_pattern");
                assert_eq!(replacement, "new_replacement");
            }
            _ => panic!("Expected Replace operation"),
        }

        assert_eq!(query.language, SrgnLanguage::Rust);
        assert_eq!(query.inputs, vec![PathBuf::from("src")]);
        assert_eq!(query.include_patterns, vec!["*.rs"]);
        assert!(query.dry_run);
        assert!(query.backup);
    }

    #[test]
    fn test_srgn_query_builder_extract() {
        let query = SrgnQueryBuilder::extract("pattern")
            .extract_format("$1: $2")
            .language(SrgnLanguage::Python)
            .max_depth(3)
            .build();

        match query.operation {
            SrgnOperation::Extract { pattern, format } => {
                assert_eq!(pattern, "pattern");
                assert_eq!(format, Some("$1: $2".to_string()));
            }
            _ => panic!("Expected Extract operation"),
        }

        assert_eq!(query.language, SrgnLanguage::Python);
        assert_eq!(query.max_depth, Some(3));
    }

    #[test]
    fn test_srgn_language_conversion() {
        assert_eq!(SrgnLanguage::Auto.to_arg(), None);
        assert_eq!(SrgnLanguage::Rust.to_arg(), Some("rust".to_string()));
        assert_eq!(SrgnLanguage::Python.to_arg(), Some("python".to_string()));
        assert_eq!(
            SrgnLanguage::Custom("mylang".to_string()).to_arg(),
            Some("mylang".to_string())
        );
    }

    #[tokio::test]
    async fn test_query_validation() {
        let srgn = Srgn::new().await;
        if srgn.is_err() {
            // Skip test if srgn not available
            return;
        }
        let srgn = srgn.unwrap();

        // Empty pattern should fail
        let empty_query = SrgnQuery {
            operation: SrgnOperation::Replace {
                pattern: String::new(),
                replacement: "test".to_string(),
            },
            ..Default::default()
        };
        assert!(srgn.validate_query(&empty_query).is_err());

        // Empty replacement should fail
        let empty_replacement = SrgnQuery {
            operation: SrgnOperation::Replace {
                pattern: "test".to_string(),
                replacement: String::new(),
            },
            ..Default::default()
        };
        assert!(srgn.validate_query(&empty_replacement).is_err());

        // Valid query should pass
        let valid_query = SrgnQuery {
            operation: SrgnOperation::Replace {
                pattern: "test".to_string(),
                replacement: "replacement".to_string(),
            },
            ..Default::default()
        };
        assert!(srgn.validate_query(&valid_query).is_ok());
    }
}
