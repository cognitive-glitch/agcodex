//! Ripgrep (rg) integration for high-performance text search.
//!
//! This module provides a comprehensive wrapper around ripgrep with:
//! - Async execution with proper cancellation support
//! - Streaming results for large codebases
//! - JSON output parsing for structured results
//! - Builder pattern for complex queries
//! - Integration with existing sandbox/security systems

use async_trait::async_trait;
use futures::Stream;
use futures::stream;
use grep_regex::RegexMatcher;
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
use tracing::debug;
use tracing::error;
use tracing::info;
use which::which;

use super::CodeTool;
use super::ToolError;
use super::traits::*;
use crate::is_safe_command::is_known_safe_command;

/// Ripgrep search query with comprehensive options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RipgrepQuery {
    /// The search pattern (regex or literal)
    pub pattern: String,
    /// Paths to search (empty = current directory)
    pub paths: Vec<PathBuf>,
    /// File type filters (e.g., "rust", "js")
    pub file_types: Vec<String>,
    /// Glob patterns to include
    pub include_globs: Vec<String>,
    /// Glob patterns to exclude
    pub exclude_globs: Vec<String>,
    /// Whether to use regex (vs literal search)
    pub use_regex: bool,
    /// Context lines before matches
    pub before_context: Option<usize>,
    /// Context lines after matches
    pub after_context: Option<usize>,
    /// Maximum number of matches per file
    pub max_matches_per_file: Option<usize>,
    /// Search only file names (not content)
    pub files_only: bool,
    /// Invert match (show non-matching lines)
    pub invert_match: bool,
    /// Word boundaries required
    pub word_boundaries: bool,
    /// Multiline matching mode
    pub multiline: bool,
    /// Additional ripgrep flags
    pub additional_flags: Vec<String>,
}

impl Default for RipgrepQuery {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            paths: vec![],
            file_types: vec![],
            include_globs: vec![],
            exclude_globs: vec![],
            use_regex: true,
            before_context: None,
            after_context: None,
            max_matches_per_file: None,
            files_only: false,
            invert_match: false,
            word_boundaries: false,
            multiline: false,
            additional_flags: vec![],
        }
    }
}

/// Builder for ripgrep queries
#[derive(Debug, Clone)]
pub struct RipgrepQueryBuilder {
    query: RipgrepQuery,
}

impl RipgrepQueryBuilder {
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            query: RipgrepQuery {
                pattern: pattern.into(),
                ..Default::default()
            },
        }
    }

    pub fn paths<P: AsRef<Path>>(mut self, paths: impl IntoIterator<Item = P>) -> Self {
        self.query.paths = paths
            .into_iter()
            .map(|p| p.as_ref().to_path_buf())
            .collect();
        self
    }

    pub fn file_types(mut self, types: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.query.file_types = types.into_iter().map(|t| t.into()).collect();
        self
    }

    pub fn include_glob(mut self, glob: impl Into<String>) -> Self {
        self.query.include_globs.push(glob.into());
        self
    }

    pub fn exclude_glob(mut self, glob: impl Into<String>) -> Self {
        self.query.exclude_globs.push(glob.into());
        self
    }

    pub const fn literal_search(mut self) -> Self {
        self.query.use_regex = false;
        self
    }

    pub const fn regex_search(mut self) -> Self {
        self.query.use_regex = true;
        self
    }

    pub const fn context(mut self, before: usize, after: usize) -> Self {
        self.query.before_context = Some(before);
        self.query.after_context = Some(after);
        self
    }

    pub const fn before_context(mut self, lines: usize) -> Self {
        self.query.before_context = Some(lines);
        self
    }

    pub const fn after_context(mut self, lines: usize) -> Self {
        self.query.after_context = Some(lines);
        self
    }

    pub const fn max_matches_per_file(mut self, max: usize) -> Self {
        self.query.max_matches_per_file = Some(max);
        self
    }

    pub const fn files_only(mut self) -> Self {
        self.query.files_only = true;
        self
    }

    pub const fn invert_match(mut self) -> Self {
        self.query.invert_match = true;
        self
    }

    pub const fn word_boundaries(mut self) -> Self {
        self.query.word_boundaries = true;
        self
    }

    pub const fn multiline(mut self) -> Self {
        self.query.multiline = true;
        self
    }

    pub fn flag(mut self, flag: impl Into<String>) -> Self {
        self.query.additional_flags.push(flag.into());
        self
    }

    pub fn build(self) -> RipgrepQuery {
        self.query
    }
}

/// Parsed JSON output from ripgrep
#[derive(Debug, Clone, Deserialize)]
struct RipgrepJsonOutput {
    #[serde(rename = "type")]
    output_type: String,
    data: RipgrepData,
}

#[derive(Debug, Clone, Deserialize)]
struct RipgrepData {
    path: Option<RipgrepPath>,
    lines: Option<RipgrepLines>,
    line_number: Option<u64>,
    absolute_offset: Option<u64>,
    submatches: Option<Vec<RipgrepSubmatch>>,
}

#[derive(Debug, Clone, Deserialize)]
struct RipgrepPath {
    text: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RipgrepLines {
    text: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RipgrepSubmatch {
    #[serde(rename = "match")]
    match_text: RipgrepMatchText,
    start: u64,
    end: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct RipgrepMatchText {
    text: String,
}

/// Streaming wrapper for ripgrep output
struct RipgrepStream {
    child: Child,
    lines_stream: Pin<Box<dyn Stream<Item = std::io::Result<String>> + Send>>,
    stats: SearchStats,
    cancel_token: CancellationToken,
    is_finished: bool,
}

impl RipgrepStream {
    fn new(mut child: Child, cancel_token: CancellationToken) -> Result<Self, ToolError> {
        let stdout = child.stdout.take().ok_or_else(|| {
            ToolError::Io(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "Failed to capture ripgrep stdout",
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

impl Stream for RipgrepStream {
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
                status: "Search cancelled".to_string(),
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
                        "Search completed: {} matches in {} files",
                        self.stats.matches_found, self.stats.files_examined
                    ),
                    is_final: true,
                })))
            }
            Poll::Ready(Some(Ok(line))) => {
                // Parse the JSON line
                let line = line.trim();
                match serde_json::from_str::<RipgrepJsonOutput>(line) {
                    Ok(output) => {
                        match output.output_type.as_str() {
                            "match" => {
                                self.stats.matches_found += 1;

                                let result = SearchResult {
                                    path: output
                                        .data
                                        .path
                                        .map(|p| PathBuf::from(p.text))
                                        .unwrap_or_default(),
                                    line_number: output.data.line_number.map(|n| n as usize),
                                    column_number: output
                                        .data
                                        .submatches
                                        .and_then(|subs| subs.first().map(|s| s.start as usize)),
                                    content: output.data.lines.map(|l| l.text),
                                    score: 1.0, // Ripgrep doesn't provide relevance scores
                                    metadata: HashMap::new(),
                                };

                                Poll::Ready(Some(Ok(StreamingSearchResult {
                                    result: Some(result),
                                    progress: None,
                                    status: "Match found".to_string(),
                                    is_final: false,
                                })))
                            }
                            "begin" => {
                                if let Some(path) = output.data.path {
                                    self.stats.files_examined += 1;
                                    Poll::Ready(Some(Ok(StreamingSearchResult {
                                        result: None,
                                        progress: None,
                                        status: format!("Searching: {}", path.text),
                                        is_final: false,
                                    })))
                                } else {
                                    // Continue to next line
                                    cx.waker().wake_by_ref();
                                    Poll::Pending
                                }
                            }
                            "end" | "summary" => {
                                // Continue to next line for summary info
                                cx.waker().wake_by_ref();
                                Poll::Pending
                            }
                            _ => {
                                debug!("Unknown ripgrep output type: {}", output.output_type);
                                cx.waker().wake_by_ref();
                                Poll::Pending
                            }
                        }
                    }
                    Err(e) => {
                        error!(
                            "Failed to parse ripgrep JSON output: {} - Line: {}",
                            e, line
                        );
                        cx.waker().wake_by_ref();
                        Poll::Pending
                    }
                }
            }
            Poll::Ready(Some(Err(e))) => {
                error!("Error reading from ripgrep: {}", e);
                self.is_finished = true;
                Poll::Ready(Some(Err(ToolError::Io(e))))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// High-performance ripgrep integration
#[derive(Debug, Clone)]
pub struct Ripgrep {
    /// Path to the rg binary
    rg_path: PathBuf,
    /// Last search statistics
    last_stats: Option<SearchStats>,
}

impl Ripgrep {
    /// Create a new Ripgrep instance
    pub async fn new() -> Result<Self, ToolError> {
        let rg_path = which("rg")
            .map_err(|_| ToolError::NotFound("ripgrep (rg) not found in PATH".to_string()))?;

        Ok(Self {
            rg_path,
            last_stats: None,
        })
    }

    /// Create ripgrep command with arguments
    fn build_command(&self, query: &RipgrepQuery, config: &SearchConfig) -> Vec<String> {
        let mut args = vec![
            "--json".to_string(), // Always use JSON output for parsing
            "--no-heading".to_string(),
            "--with-filename".to_string(),
        ];

        // Pattern matching mode
        if query.use_regex {
            args.push("--regexp".to_string());
        } else {
            args.push("--fixed-strings".to_string());
        }

        // Case sensitivity
        if !config.case_sensitive {
            args.push("--ignore-case".to_string());
        }

        // Hidden files
        if config.include_hidden {
            args.push("--hidden".to_string());
        }

        // Follow symlinks
        if config.follow_symlinks {
            args.push("--follow".to_string());
        }

        // Context lines
        if let Some(before) = query.before_context {
            args.push(format!("--before-context={}", before));
        }
        if let Some(after) = query.after_context {
            args.push(format!("--after-context={}", after));
        }

        // Max matches per file
        if let Some(max) = query.max_matches_per_file {
            args.push(format!("--max-count={}", max));
        }

        // Max results
        if let Some(max) = config.max_results {
            args.push(format!("--max-count={}", max));
        }

        // File types
        for file_type in &query.file_types {
            args.push(format!("--type={}", file_type));
        }

        // Include globs
        for glob in &query.include_globs {
            args.push(format!("--glob={}", glob));
        }

        // Exclude globs
        for glob in &query.exclude_globs {
            args.push(format!("--glob=!{}", glob));
        }

        // Special modes
        if query.files_only {
            args.push("--files-with-matches".to_string());
        }

        if query.invert_match {
            args.push("--invert-match".to_string());
        }

        if query.word_boundaries {
            args.push("--word-regexp".to_string());
        }

        if query.multiline {
            args.push("--multiline".to_string());
        }

        // Additional flags
        args.extend(query.additional_flags.clone());

        // Pattern
        args.push(query.pattern.clone());

        // Paths
        if query.paths.is_empty() {
            args.push(".".to_string()); // Current directory
        } else {
            for path in &query.paths {
                args.push(path.to_string_lossy().to_string());
            }
        }

        args
    }
}

impl CodeTool for Ripgrep {
    type Query = RipgrepQuery;
    type Output = Vec<SearchResult>;

    fn search(&self, query: Self::Query) -> Result<Self::Output, ToolError> {
        // This is a sync adapter for the async implementation
        tokio::runtime::Handle::current().block_on(async {
            let config = SearchConfig::default();
            let cancel_token = CancellationToken::new();

            <Self as UnifiedSearch>::search(self, query, config, cancel_token).await
        })
    }
}

#[async_trait]
impl UnifiedSearch for Ripgrep {
    type Query = RipgrepQuery;

    fn search_type(&self) -> SearchType {
        SearchType::Content
    }

    fn tool_name(&self) -> &'static str {
        "ripgrep"
    }

    async fn is_available(&self) -> Result<bool, ToolError> {
        Ok(self.rg_path.exists())
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
        let command_args: Vec<String> = std::iter::once("rg".to_string())
            .chain(args.iter().cloned())
            .collect();
        if !is_known_safe_command(&command_args) {
            return Err(ToolError::InvalidQuery(
                "Ripgrep command rejected by security policy".to_string(),
            ));
        }

        // Execute the command
        let mut cmd = Command::new(&self.rg_path);
        cmd.args(&args[1..]) // Skip the "rg" part
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
            "Executing ripgrep: {} {}",
            self.rg_path.display(),
            args.join(" ")
        );

        let child = cmd.spawn().map_err(ToolError::Io)?;

        let stream = RipgrepStream::new(child, cancel_token)?;
        Ok(Box::new(stream))
    }

    async fn get_last_search_stats(&self) -> Option<SearchStats> {
        self.last_stats.clone()
    }

    fn validate_query(&self, query: &Self::Query) -> Result<(), ToolError> {
        if query.pattern.is_empty() {
            return Err(ToolError::InvalidQuery(
                "Search pattern cannot be empty".to_string(),
            ));
        }

        // Validate regex if using regex mode
        if query.use_regex
            && let Err(e) = RegexMatcher::new(&query.pattern) {
                return Err(ToolError::InvalidQuery(format!(
                    "Invalid regex pattern: {}",
                    e
                )));
            }

        // Validate paths exist
        for path in &query.paths {
            if !path.exists() {
                return Err(ToolError::InvalidQuery(format!(
                    "Path does not exist: {}",
                    path.display()
                )));
            }
        }

        Ok(())
    }
}

#[async_trait]
impl ToolDiscovery for Ripgrep {
    async fn discover_tools() -> Result<Vec<String>, ToolError> {
        match which("rg") {
            Ok(_) => Ok(vec!["ripgrep".to_string()]),
            Err(_) => Ok(vec![]),
        }
    }

    async fn get_tool_version(tool_name: &str) -> Result<String, ToolError> {
        if tool_name != "ripgrep" && tool_name != "rg" {
            return Err(ToolError::NotFound(format!(
                "Tool not managed by this provider: {}",
                tool_name
            )));
        }

        let output = Command::new("rg")
            .arg("--version")
            .output()
            .await
            .map_err(ToolError::Io)?;

        if !output.status.success() {
            return Err(ToolError::InvalidQuery(
                "Failed to get ripgrep version".to_string(),
            ));
        }

        let version_text = String::from_utf8_lossy(&output.stdout);
        Ok(version_text.lines().next().unwrap_or("Unknown").to_string())
    }

    async fn get_tool_path(tool_name: &str) -> Result<PathBuf, ToolError> {
        if tool_name != "ripgrep" && tool_name != "rg" {
            return Err(ToolError::NotFound(format!(
                "Tool not managed by this provider: {}",
                tool_name
            )));
        }

        which("rg").map_err(|_| ToolError::NotFound("ripgrep (rg) not found in PATH".to_string()))
    }

    async fn validate_tools(required: &[&str]) -> Result<(), ToolError> {
        for &tool in required {
            if tool == "ripgrep" || tool == "rg" {
                Self::get_tool_path(tool).await?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    

    #[tokio::test]
    async fn test_ripgrep_query_builder() {
        let query = RipgrepQueryBuilder::new("test")
            .paths(["src"])
            .file_types(["rust"])
            .include_glob("*.rs")
            .exclude_glob("*test*")
            .literal_search()
            .context(2, 2)
            .max_matches_per_file(10)
            .word_boundaries()
            .build();

        assert_eq!(query.pattern, "test");
        assert_eq!(query.paths, vec![PathBuf::from("src")]);
        assert_eq!(query.file_types, vec!["rust"]);
        assert_eq!(query.include_globs, vec!["*.rs"]);
        assert_eq!(query.exclude_globs, vec!["*test*"]);
        assert!(!query.use_regex);
        assert_eq!(query.before_context, Some(2));
        assert_eq!(query.after_context, Some(2));
        assert_eq!(query.max_matches_per_file, Some(10));
        assert!(query.word_boundaries);
    }

    #[tokio::test]
    async fn test_query_validation() {
        let rg = Ripgrep::new().await;
        if rg.is_err() {
            // Skip test if ripgrep not available
            return;
        }
        let rg = rg.unwrap();

        // Empty pattern should fail
        let empty_query = RipgrepQuery {
            pattern: String::new(),
            ..Default::default()
        };
        assert!(rg.validate_query(&empty_query).is_err());

        // Valid pattern should pass
        let valid_query = RipgrepQuery {
            pattern: "test".to_string(),
            ..Default::default()
        };
        assert!(rg.validate_query(&valid_query).is_ok());

        // Invalid regex should fail
        let invalid_regex = RipgrepQuery {
            pattern: "[unclosed".to_string(),
            use_regex: true,
            ..Default::default()
        };
        assert!(rg.validate_query(&invalid_regex).is_err());
    }

    #[tokio::test]
    async fn test_command_building() {
        let rg = Ripgrep::new().await;
        if rg.is_err() {
            return; // Skip if not available
        }
        let rg = rg.unwrap();

        let query = RipgrepQuery {
            pattern: "test".to_string(),
            file_types: vec!["rust".to_string()],
            before_context: Some(2),
            use_regex: false,
            ..Default::default()
        };

        let config = SearchConfig {
            case_sensitive: false,
            max_results: Some(100),
            ..Default::default()
        };

        let args = rg.build_command(&query, &config);

        assert!(args.contains(&"--json".to_string()));
        assert!(args.contains(&"--fixed-strings".to_string()));
        assert!(args.contains(&"--ignore-case".to_string()));
        assert!(args.contains(&"--type=rust".to_string()));
        assert!(args.contains(&"--before-context=2".to_string()));
        assert!(args.contains(&"--max-count=100".to_string()));
        assert!(args.contains(&"test".to_string()));
        assert!(args.contains(&".".to_string())); // Default path
    }
}
