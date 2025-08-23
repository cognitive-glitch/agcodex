//! Simplified AST-grep based semantic search tool for AGCodex.
//!
//! This is a simplified implementation that uses text-based pattern matching
//! instead of the complex ast-grep API to avoid compilation issues.

use super::Change;
use super::ChangeKind;
use super::ComprehensiveSemanticImpact;
use super::ComprehensiveToolOutput;
use super::ContextSnapshot;
use super::OperationContext;
use super::OperationMetadata;
use super::OperationScope;
use super::PerformanceMetrics;
use super::ScopeType;
// Import SourceLocation from ast module
use ast::SourceLocation;
use dashmap::DashMap;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use thiserror::Error;
use tracing::error;
use uuid::Uuid;

/// Errors specific to grep operations
#[derive(Debug, Error)]
pub enum GrepError {
    #[error("invalid pattern: {pattern} - {reason}")]
    InvalidPattern { pattern: String, reason: String },

    #[error("unsupported language: {language}")]
    UnsupportedLanguage { language: String },

    #[error("query compilation failed: {query} - {reason}")]
    QueryCompilationFailed { query: String, reason: String },

    #[error("YAML rule parsing failed: {rule} - {reason}")]
    YamlRuleFailed { rule: String, reason: String },

    #[error("search timeout after {duration:?}")]
    SearchTimeout { duration: Duration },

    #[error("file access error: {path} - {reason}")]
    FileAccess { path: PathBuf, reason: String },

    #[error("parse error for {path}: {reason}")]
    ParseError { path: PathBuf, reason: String },

    #[error("pattern cache overflow: {current_size} >= {max_size}")]
    CacheOverflow {
        current_size: usize,
        max_size: usize,
    },

    #[error("performance threshold exceeded: {actual_ms}ms > {threshold_ms}ms")]
    PerformanceThreshold { actual_ms: u64, threshold_ms: u64 },

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Result type for grep operations
pub type GrepResult<T> = std::result::Result<T, GrepError>;

/// Type of pattern/rule being used
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuleType {
    /// Simple string pattern
    Pattern,
    /// Tree-sitter query
    Query,
    /// YAML rule configuration
    YamlRule,
}

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SupportedLanguage {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    C,
    Cpp,
    CSharp,
    Html,
    Css,
    Json,
    Yaml,
    Toml,
    Bash,
    Ruby,
    Php,
    Haskell,
    Elixir,
    Swift,
    Kotlin,
    Sql,
    Dockerfile,
    Markdown,
}

impl SupportedLanguage {
    pub const fn as_str(&self) -> &str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Go => "go",
            Self::Java => "java",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::CSharp => "csharp",
            Self::Html => "html",
            Self::Css => "css",
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Toml => "toml",
            Self::Bash => "bash",
            Self::Ruby => "ruby",
            Self::Php => "php",
            Self::Haskell => "haskell",
            Self::Elixir => "elixir",
            Self::Swift => "swift",
            Self::Kotlin => "kotlin",
            Self::Sql => "sql",
            Self::Dockerfile => "dockerfile",
            Self::Markdown => "markdown",
        }
    }
}

/// A match found during grep search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepMatch {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub matched_text: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
    pub confidence: f32,
    pub byte_offset: usize,
}

/// Search query for grep operations
#[derive(Debug, Clone)]
pub struct GrepQuery {
    pub pattern: String,
    pub paths: Vec<PathBuf>,
    pub language: Option<SupportedLanguage>,
    pub rule_type: RuleType,
    pub max_results: Option<usize>,
    pub include_hidden: bool,
    pub follow_symlinks: bool,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub context_lines: usize,
}

impl Default for GrepQuery {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            paths: Vec::new(),
            language: None,
            rule_type: RuleType::Pattern,
            max_results: Some(1000),
            include_hidden: false,
            follow_symlinks: false,
            case_sensitive: true,
            whole_word: false,
            context_lines: 3,
        }
    }
}

/// Configuration for grep operations
#[derive(Debug, Clone)]
pub struct GrepConfig {
    pub max_file_size: usize,
    pub max_pattern_cache_size: usize,
    pub parallel_threshold: usize,
    pub timeout: Duration,
    pub performance_threshold_ms: u64,
}

impl Default for GrepConfig {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024, // 10MB
            max_pattern_cache_size: 1000,
            parallel_threshold: 10,
            timeout: Duration::from_secs(30),
            performance_threshold_ms: 5000,
        }
    }
}

/// Simplified grep engine
pub struct SimpleGrepEngine {
    _config: GrepConfig,
    _pattern_cache: Arc<DashMap<String, Vec<GrepMatch>>>,
}

impl SimpleGrepEngine {
    pub fn new(config: GrepConfig) -> Self {
        Self {
            _config: config,
            _pattern_cache: Arc::new(DashMap::new()),
        }
    }

    /// Search files with pattern
    pub fn search_files(&self, query: &GrepQuery) -> GrepResult<Vec<GrepMatch>> {
        let mut all_matches = Vec::new();

        for path in &query.paths {
            if path.is_file() {
                let matches = self.search_file(path, &query.pattern, query)?;
                all_matches.extend(matches);
            } else if path.is_dir() {
                let matches = self.search_directory(path, &query.pattern, query)?;
                all_matches.extend(matches);
            }

            // Check max results
            if let Some(max) = query.max_results
                && all_matches.len() >= max
            {
                all_matches.truncate(max);
                break;
            }
        }

        Ok(all_matches)
    }

    /// Search a single file
    fn search_file(
        &self,
        path: &Path,
        pattern: &str,
        query: &GrepQuery,
    ) -> GrepResult<Vec<GrepMatch>> {
        let content = std::fs::read_to_string(path)?;
        let lines: Vec<&str> = content.lines().collect();
        let mut matches = Vec::new();

        for (line_idx, line) in lines.iter().enumerate() {
            if self.line_matches(line, pattern, query) {
                let line_num = line_idx + 1;

                // Get context
                let context_before = if line_idx > 0 {
                    let start = line_idx.saturating_sub(query.context_lines);
                    lines[start..line_idx]
                        .iter()
                        .map(|s| (*s).to_string())
                        .collect()
                } else {
                    Vec::new()
                };

                let context_after = if line_idx < lines.len() - 1 {
                    let end = std::cmp::min(line_idx + 1 + query.context_lines, lines.len());
                    lines[line_idx + 1..end]
                        .iter()
                        .map(|s| (*s).to_string())
                        .collect()
                } else {
                    Vec::new()
                };

                matches.push(GrepMatch {
                    file: path.to_path_buf(),
                    line: line_num,
                    column: 1,
                    end_line: line_num,
                    end_column: line.len(),
                    matched_text: (*line).to_string(),
                    context_before,
                    context_after,
                    confidence: 1.0,
                    byte_offset: 0, // Simplified
                });
            }
        }

        Ok(matches)
    }

    /// Search a directory recursively
    fn search_directory(
        &self,
        dir: &Path,
        pattern: &str,
        query: &GrepQuery,
    ) -> GrepResult<Vec<GrepMatch>> {
        let mut all_matches = Vec::new();

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Ok(matches) = self.search_file(&path, pattern, query) {
                    all_matches.extend(matches);
                }
            } else if path.is_dir()
                && !path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .starts_with('.')
                && let Ok(matches) = self.search_directory(&path, pattern, query)
            {
                all_matches.extend(matches);
            }
        }

        Ok(all_matches)
    }

    /// Check if a line matches the pattern
    fn line_matches(&self, line: &str, pattern: &str, query: &GrepQuery) -> bool {
        if query.case_sensitive {
            line.contains(pattern)
        } else {
            line.to_lowercase().contains(&pattern.to_lowercase())
        }
    }
}

/// Main grep tool
pub struct GrepTool {
    engine: SimpleGrepEngine,
}

impl GrepTool {
    pub fn new(config: GrepConfig) -> Self {
        Self {
            engine: SimpleGrepEngine::new(config),
        }
    }

    /// Execute search with full query object
    pub fn search_with_query(
        &self,
        query: GrepQuery,
    ) -> GrepResult<ComprehensiveToolOutput<Vec<GrepMatch>>> {
        let start = Instant::now();

        // Perform search
        let matches = self.engine.search_files(&query)?;

        let duration = start.elapsed();

        // Build comprehensive output
        let first_path = query
            .paths
            .first()
            .cloned()
            .unwrap_or_else(|| PathBuf::from("unknown"));
        let context = OperationContext {
            before: ContextSnapshot {
                content: format!("Searching for pattern: {}", query.pattern),
                timestamp: std::time::SystemTime::now(),
                content_hash: format!("{:x}", md5::compute(&query.pattern)),
                ast_summary: None,
                symbols: Vec::new(),
            },
            after: None,
            surrounding: Vec::new(),
            location: SourceLocation {
                file_path: first_path.to_string_lossy().to_string(),
                start_line: 0,
                start_column: 0,
                end_line: 0,
                end_column: 0,
                byte_range: (0, 0),
            },
            scope: OperationScope {
                scope_type: ScopeType::File,
                name: "search".to_string(),
                path: vec!["grep".to_string()],
                file_path: first_path.clone(),
                line_range: 0..0,
            },
            language_context: None,
            project_context: None,
        };

        let changes = matches
            .iter()
            .map(|m| Change {
                id: Uuid::new_v4(),
                kind: ChangeKind::Added {
                    reason: format!("Pattern match found for '{}'", query.pattern),
                    insertion_point: SourceLocation {
                        file_path: m.file.to_string_lossy().to_string(),
                        start_line: m.line,
                        start_column: m.column,
                        end_line: m.line,
                        end_column: m.column + m.matched_text.len(),
                        byte_range: (m.byte_offset, m.byte_offset + m.matched_text.len()),
                    },
                },
                old: None,
                new: Some(m.matched_text.clone()),
                line_range: m.line..m.line + 1,
                char_range: m.column..m.column + m.matched_text.len(),
                location: SourceLocation {
                    file_path: m.file.to_string_lossy().to_string(),
                    start_line: m.line,
                    start_column: m.column,
                    end_line: m.line,
                    end_column: m.column + m.matched_text.len(),
                    byte_range: (m.byte_offset, m.byte_offset + m.matched_text.len()),
                },
                semantic_impact: ComprehensiveSemanticImpact::minimal(),
                affected_symbols: Vec::new(),
                confidence: m.confidence,
                description: format!(
                    "Found pattern '{}' in {} at line {}",
                    query.pattern,
                    m.file.display(),
                    m.line
                ),
            })
            .collect();

        let summary = format!(
            "Found {} matches for '{}' across {} files in {:?}",
            matches.len(),
            query.pattern,
            query.paths.len(),
            duration
        );

        Ok(ComprehensiveToolOutput {
            result: matches,
            context,
            changes,
            metadata: OperationMetadata {
                tool: "grep".to_string(),
                operation: "search".to_string(),
                operation_id: Uuid::new_v4(),
                started_at: std::time::SystemTime::now() - duration,
                completed_at: std::time::SystemTime::now(),
                confidence: 1.0,
                parameters: [
                    ("pattern".to_string(), query.pattern.clone()),
                    ("rule_type".to_string(), format!("{:?}", query.rule_type)),
                ]
                .iter()
                .cloned()
                .collect(),
                initiated_by: Some("user".to_string()),
                session_id: Some(Uuid::new_v4()),
                tool_version: "1.0.0".to_string(),
            },
            summary,
            performance: PerformanceMetrics {
                execution_time: duration,
                phase_times: std::collections::HashMap::new(),
                memory_usage: super::MemoryUsage::default(),
                cpu_usage: super::CpuUsage::default(),
                io_stats: super::IoStats::default(),
                cache_stats: super::CacheStats::default(),
            },
            diagnostics: Vec::new(),
        })
    }
}
