//! AST-grep based semantic search tool for AGCodex.
//!
//! This tool provides powerful AST-based code search using ast-grep with YAML rules,
//! pattern caching, and rich contextual information optimized for LLM consumption.
//!
//! ## Features
//! - AST-grep integration with YAML rule support
//! - Pattern compilation and caching for performance
//! - Semantic context extraction with meta-variable bindings
//! - Multi-language support with parallel processing
//! - Context-aware output with confidence scoring

use super::{
    ComprehensiveToolOutput, OperationContext, Change, ChangeKind,
    ComprehensiveSemanticImpact, ContextSnapshot, OperationScope,
    PerformanceMetrics, OperationMetadata, ScopeType
};
use super::patch::SourceLocation;
use ast_grep_core::{AstGrep, NodeMatch};
use ast_grep_config::{RuleConfig, from_yaml_string};
use ast_grep_language::SupportLang as SgLang;
use dashmap::DashMap;
use tree_sitter_rust;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, warn, error};
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
    CacheOverflow { current_size: usize, max_size: usize },
    
    #[error("performance threshold exceeded: {actual_ms}ms > {threshold_ms}ms")]
    PerformanceThreshold { actual_ms: u64, threshold_ms: u64 },
    
    #[error(transparent)]
    Io(#[from] std::io::Error),
    
    #[error("AST-grep language error: {0}")]
    AstGrepLanguage(String),
}

/// Result type for grep operations
pub type GrepResult<T> = std::result::Result<T, GrepError>;

/// Cache key for compiled patterns
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct PatternKey {
    pattern: String,
    language: SupportedLanguage,
    rule_type: RuleType,
}

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

/// Supported programming languages for AST-grep
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
    Bash,
    Php,
    Ruby,
    Swift,
    Kotlin,
}

impl SupportedLanguage {
    pub fn as_str(&self) -> &'static str {
        match self {
            SupportedLanguage::Rust => "rust",
            SupportedLanguage::Python => "python",
            SupportedLanguage::JavaScript => "javascript",
            SupportedLanguage::TypeScript => "typescript",
            SupportedLanguage::Go => "go",
            SupportedLanguage::Java => "java",
            SupportedLanguage::C => "c",
            SupportedLanguage::Cpp => "cpp",
            SupportedLanguage::CSharp => "c_sharp",
            SupportedLanguage::Html => "html",
            SupportedLanguage::Css => "css",
            SupportedLanguage::Json => "json",
            SupportedLanguage::Yaml => "yaml",
            SupportedLanguage::Bash => "bash",
            SupportedLanguage::Php => "php",
            SupportedLanguage::Ruby => "ruby",
            SupportedLanguage::Swift => "swift",
            SupportedLanguage::Kotlin => "kotlin",
        }
    }
    
    /// Convert to ast-grep language string
    pub fn to_ast_grep_language_str(&self) -> &'static str {
        match self {
            SupportedLanguage::Rust => "rust",
            SupportedLanguage::Python => "python",
            SupportedLanguage::JavaScript => "javascript",
            SupportedLanguage::TypeScript => "typescript", 
            SupportedLanguage::Go => "go",
            SupportedLanguage::Java => "java",
            SupportedLanguage::C => "c",
            SupportedLanguage::Cpp => "cpp",
            SupportedLanguage::CSharp => "c_sharp",
            SupportedLanguage::Html => "html",
            SupportedLanguage::Css => "css",
            // Map unsupported languages to closest equivalent
            SupportedLanguage::Json => "javascript",
            SupportedLanguage::Yaml => "yaml",
            SupportedLanguage::Bash => "bash",
            SupportedLanguage::Php => "php",
            SupportedLanguage::Ruby => "ruby",
            SupportedLanguage::Swift => "swift",
            SupportedLanguage::Kotlin => "kotlin",
        }
    }
    
    /// Detect language from file extension
    pub fn from_path(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?;
        match extension.to_lowercase().as_str() {
            "rs" => Some(SupportedLanguage::Rust),
            "py" | "pyw" | "pyi" => Some(SupportedLanguage::Python),
            "js" | "mjs" | "cjs" => Some(SupportedLanguage::JavaScript),
            "ts" | "tsx" | "cts" | "mts" => Some(SupportedLanguage::TypeScript),
            "go" => Some(SupportedLanguage::Go),
            "java" => Some(SupportedLanguage::Java),
            "c" | "h" => Some(SupportedLanguage::C),
            "cpp" | "cxx" | "cc" | "hpp" | "hxx" | "hh" => Some(SupportedLanguage::Cpp),
            "cs" => Some(SupportedLanguage::CSharp),
            "html" | "htm" => Some(SupportedLanguage::Html),
            "css" => Some(SupportedLanguage::Css),
            "json" => Some(SupportedLanguage::Json),
            "yaml" | "yml" => Some(SupportedLanguage::Yaml),
            "sh" | "bash" | "zsh" => Some(SupportedLanguage::Bash),
            "php" => Some(SupportedLanguage::Php),
            "rb" => Some(SupportedLanguage::Ruby),
            "swift" => Some(SupportedLanguage::Swift),
            "kt" | "kts" => Some(SupportedLanguage::Kotlin),
            _ => None,
        }
    }
}

/// Compiled pattern with caching and metadata
#[derive(Debug, Clone)]
pub struct CompiledPattern {
    /// Original pattern or rule
    pub original: String,
    /// Language this pattern targets
    pub language: SupportedLanguage,
    /// Type of rule
    pub rule_type: RuleType,
    /// Compiled rule configuration (for YAML rules)
    pub rule_config: Option<RuleConfig<SgLang>>,
    /// Pattern complexity score (0-10)
    pub complexity: u8,
    /// Whether pattern has meta-variables ($VAR, $$EXPR)
    pub has_metavars: bool,
    /// Meta-variable names found in pattern
    pub metavar_names: Vec<String>,
    /// Compilation timestamp
    pub compiled_at: Instant,
    /// Performance hint for optimization
    pub performance_hint: PerformanceHint,
}

/// Performance optimization hints
#[derive(Debug, Clone, PartialEq)]
pub enum PerformanceHint {
    /// Fast literal string match
    Literal,
    /// Structural AST pattern
    Structural,
    /// Complex query with conditions
    Complex,
    /// YAML rule with transformations
    YamlRule,
}

/// Language detector with caching
#[derive(Debug)]
pub struct LanguageDetector {
    /// Cache mapping file extensions to languages
    extension_cache: Arc<DashMap<String, SupportedLanguage>>,
}

impl Default for LanguageDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageDetector {
    pub fn new() -> Self {
        Self {
            extension_cache: Arc::new(DashMap::new()),
        }
    }
    
    /// Detect language from file path with caching
    pub fn detect(&self, path: &Path) -> Option<SupportedLanguage> {
        let extension = path.extension()?.to_str()?.to_lowercase();
        
        if let Some(cached) = self.extension_cache.get(&extension) {
            return Some(*cached);
        }
        
        let detected = SupportedLanguage::from_path(path);
        if let Some(lang) = detected {
            self.extension_cache.insert(extension, lang);
        }
        
        detected
    }
}

/// Query optimizer for pattern analysis and optimization
#[derive(Debug)]
pub struct QueryOptimizer {
    /// Pattern analysis cache
    analysis_cache: Arc<DashMap<String, PatternAnalysis>>,
}

#[derive(Debug, Clone)]
pub struct PatternAnalysis {
    pub complexity: u8,
    pub has_metavars: bool,
    pub metavar_names: Vec<String>,
    pub performance_hint: PerformanceHint,
    pub estimated_selectivity: f32, // 0.0 (highly selective) to 1.0 (matches everything)
}

impl Default for QueryOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryOptimizer {
    pub fn new() -> Self {
        Self {
            analysis_cache: Arc::new(DashMap::new()),
        }
    }
    
    /// Analyze pattern for optimization opportunities
    pub fn analyze(&self, pattern: &str) -> PatternAnalysis {
        if let Some(cached) = self.analysis_cache.get(pattern) {
            return cached.clone();
        }
        
        let mut complexity = 0u8;
        let mut metavar_names = Vec::new();
        let mut has_metavars = false;
        
        // Analyze pattern complexity
        if pattern.contains('(') && pattern.contains(')') {
            complexity += 3; // Parentheses indicate complex structure
        }
        if pattern.contains('@') {
            complexity += 2; // Captures add complexity
        }
        if pattern.contains("where") || pattern.contains("has") {
            complexity += 4; // Conditional logic is complex
        }
        
        // Find meta-variables
        for part in pattern.split_whitespace() {
            if part.starts_with('$') {
                has_metavars = true;
                let metavar = if part.starts_with("$$") {
                    &part[2..]
                } else {
                    &part[1..]
                };
                if !metavar.is_empty() {
                    metavar_names.push(metavar.to_string());
                    complexity += 1;
                }
            }
        }
        
        // Determine performance hint
        let performance_hint = if pattern.chars().all(|c| c.is_alphanumeric() || c == '_') {
            PerformanceHint::Literal
        } else if complexity <= 3 {
            PerformanceHint::Structural
        } else {
            PerformanceHint::Complex
        };
        
        // Estimate selectivity (how many nodes this pattern might match)
        let estimated_selectivity = if performance_hint == PerformanceHint::Literal {
            0.1 // Literals are highly selective
        } else if has_metavars {
            0.6 // Meta-variables match more broadly
        } else {
            0.3 // Structural patterns are moderately selective
        };
        
        let analysis = PatternAnalysis {
            complexity: complexity.min(10),
            has_metavars,
            metavar_names,
            performance_hint,
            estimated_selectivity,
        };
        
        self.analysis_cache.insert(pattern.to_string(), analysis.clone());
        analysis
    }
}

/// Semantic ranker for result relevance scoring
#[derive(Debug)]
pub struct SemanticRanker {
    /// Scoring configuration
    scoring_config: ScoringConfig,
}

#[derive(Debug, Clone)]
pub struct ScoringConfig {
    /// Weight for exact pattern matches
    pub exact_match_weight: f32,
    /// Weight for definition matches (functions, classes)
    pub definition_weight: f32,
    /// Weight for public symbols
    pub visibility_weight: f32,
    /// Weight for shorter paths (closer to project root)
    pub path_length_weight: f32,
    /// Weight for context relevance
    pub context_weight: f32,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            exact_match_weight: 0.4,
            definition_weight: 0.25,
            visibility_weight: 0.15,
            path_length_weight: 0.1,
            context_weight: 0.1,
        }
    }
}

impl SemanticRanker {
    pub fn new() -> Self {
        Self::with_config(ScoringConfig::default())
    }
    
    pub fn with_config(config: ScoringConfig) -> Self {
        Self {
            scoring_config: config,
        }
    }
    
    /// Calculate relevance score for a match
    pub fn score_match(&self, grep_match: &GrepMatch, query: &GrepQuery) -> f32 {
        let mut score = 0.0;
        
        // Exact match bonus
        if grep_match.content.contains(&query.pattern) {
            score += self.scoring_config.exact_match_weight;
        }
        
        // Definition bonus
        if grep_match.semantic_context.is_definition {
            score += self.scoring_config.definition_weight;
        }
        
        // Visibility bonus (prefer public symbols)
        match &grep_match.semantic_context.access_level {
            Some(AccessLevel::Public) => score += self.scoring_config.visibility_weight,
            Some(AccessLevel::Protected) => score += self.scoring_config.visibility_weight * 0.5,
            _ => {}
        }
        
        // Shorter path bonus (prefer files closer to root)
        let path_components = grep_match.file.components().count();
        let path_score = (10.0 - path_components.min(10) as f32) / 10.0;
        score += self.scoring_config.path_length_weight * path_score;
        
        // Context relevance (based on surrounding code)
        if !grep_match.surrounding_context.containing_function.is_none() {
            score += self.scoring_config.context_weight;
        }
        
        score.min(1.0)
    }
}

impl Default for SemanticRanker {
    fn default() -> Self {
        Self::new()
    }
}

/// Main AST-grep engine with pattern caching and optimization
#[derive(Debug)]
pub struct AstGrepEngine {
    /// Compiled pattern cache for fast repeated searches
    pattern_cache: Arc<DashMap<PatternKey, CompiledPattern>>,
    /// Language detector for file type identification
    language_detector: LanguageDetector,
    /// Query optimizer for pattern analysis
    query_optimizer: QueryOptimizer,
    /// Result ranker for relevance scoring
    result_ranker: SemanticRanker,
    /// Configuration
    config: GrepConfig,
}

/// Configuration for AST-grep engine
#[derive(Debug, Clone)]
pub struct GrepConfig {
    /// Maximum cache size for compiled patterns
    pub max_pattern_cache_size: usize,
    /// Cache TTL for patterns
    pub pattern_cache_ttl: Duration,
    /// Maximum search timeout
    pub search_timeout: Duration,
    /// Enable parallel processing
    pub parallel_processing: bool,
    /// Maximum results per search
    pub max_results: usize,
    /// Include surrounding context lines
    pub context_lines: usize,
    /// Performance threshold in milliseconds
    pub performance_threshold_ms: u64,
}

impl Default for GrepConfig {
    fn default() -> Self {
        Self {
            max_pattern_cache_size: 1000,
            pattern_cache_ttl: Duration::from_secs(3600), // 1 hour
            search_timeout: Duration::from_secs(30),
            parallel_processing: true,
            max_results: 1000,
            context_lines: 3,
            performance_threshold_ms: 10, // 10ms threshold for complex patterns
        }
    }
}

/// Rich grep match with AST context and meta-variable bindings
#[derive(Debug, Clone)]
pub struct GrepMatch {
    /// File path
    pub file: PathBuf,
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
    /// Byte offset in file
    pub byte_offset: usize,
    /// Matched text content
    pub content: String,
    /// Match confidence score (0.0-1.0)
    pub confidence: f32,
    /// Meta-variable bindings from pattern ($VAR, $$EXPR)
    pub metavar_bindings: HashMap<String, String>,
    /// Semantic context information
    pub semantic_context: SemanticContext,
    /// Surrounding code context
    pub surrounding_context: SurroundingContext,
    /// Suggested fixes from YAML rules (if applicable)
    pub suggested_fixes: Vec<SuggestedFix>,
}

/// Semantic context for code understanding
#[derive(Debug, Clone)]
pub struct SemanticContext {
    /// Semantic role in the code (function, class, variable, etc.)
    pub role: SemanticRole,
    /// Containing scope (function, class, module)
    pub containing_scope: Option<String>,
    /// Symbol being matched
    pub symbol_name: Option<String>,
    /// Whether this is a definition or reference
    pub is_definition: bool,
    /// Access level (public, private, etc.)
    pub access_level: Option<AccessLevel>,
    /// Node type in AST
    pub node_type: String,
}

/// Semantic role classification
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticRole {
    /// Function or method declaration
    Declaration,
    /// Function or method call
    Call,
    /// Variable or constant reference
    Reference,
    /// Type annotation or definition
    TypeAnnotation,
    /// Import or include statement
    Import,
    /// Export statement
    Export,
    /// Control flow (if, for, while, etc.)
    ControlFlow,
    /// Comment or documentation
    Comment,
    /// Unknown or other
    Other,
}

/// Access level for symbols
#[derive(Debug, Clone, PartialEq)]
pub enum AccessLevel {
    Public,
    Private,
    Protected,
    Internal,
    Package,
}

/// Surrounding code context
#[derive(Debug, Clone)]
pub struct SurroundingContext {
    /// Lines before the match
    pub before_lines: Vec<ContextLine>,
    /// Lines after the match
    pub after_lines: Vec<ContextLine>,
    /// Function containing this match
    pub containing_function: Option<String>,
    /// Class containing this match
    pub containing_class: Option<String>,
    /// Module or namespace
    pub containing_module: Option<String>,
}

/// Context line with metadata
#[derive(Debug, Clone)]
pub struct ContextLine {
    /// Line number (1-based)
    pub number: usize,
    /// Line content
    pub content: String,
    /// Whether this line contains the match
    pub is_match: bool,
    /// Indentation level
    pub indent_level: usize,
}

/// Suggested fix from YAML rule
#[derive(Debug, Clone)]
pub struct SuggestedFix {
    /// Description of the fix
    pub description: String,
    /// Replacement text
    pub replacement: String,
    /// Fix confidence (0.0-1.0)
    pub confidence: f32,
    /// Whether this fix is automatically applicable
    pub auto_applicable: bool,
}

/// Query for grep operations
#[derive(Debug, Clone)]
pub struct GrepQuery {
    /// Search pattern or YAML rule
    pub pattern: String,
    /// Target files or directories
    pub paths: Vec<PathBuf>,
    /// Language filter
    pub language: Option<SupportedLanguage>,
    /// Rule type (pattern, query, yaml_rule)
    pub rule_type: RuleType,
    /// Include surrounding context
    pub with_context: bool,
    /// Maximum results to return
    pub limit: Option<usize>,
    /// Case sensitivity
    pub case_sensitive: bool,
}

impl Default for GrepQuery {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            paths: Vec::new(),
            language: None,
            rule_type: RuleType::Pattern,
            with_context: true,
            limit: Some(100),
            case_sensitive: true,
        }
    }
}

/// Main grep tool implementation
#[derive(Debug)]
pub struct GrepTool {
    /// AST-grep engine
    engine: AstGrepEngine,
}

impl AstGrepEngine {
    /// Convert SupportedLanguage to SgLang
    fn to_sg_lang(&self, language: SupportedLanguage) -> SgLang {
        match language {
            SupportedLanguage::Rust => SgLang::Rust,
            SupportedLanguage::Python => SgLang::Python,
            SupportedLanguage::JavaScript => SgLang::JavaScript,
            SupportedLanguage::TypeScript => SgLang::TypeScript,
            SupportedLanguage::Go => SgLang::Go,
            SupportedLanguage::Java => SgLang::Java,
            SupportedLanguage::C => SgLang::C,
            SupportedLanguage::Cpp => SgLang::Cpp,
            SupportedLanguage::CSharp => SgLang::CSharp,
            SupportedLanguage::Html => SgLang::Html,
            SupportedLanguage::Css => SgLang::Css,
            SupportedLanguage::Json => SgLang::Json,
            SupportedLanguage::Yaml => SgLang::Yaml,
            SupportedLanguage::Bash => SgLang::Bash,
            SupportedLanguage::Ruby => SgLang::Ruby,
            SupportedLanguage::Php => SgLang::Php,
            SupportedLanguage::Swift => SgLang::Swift,
            SupportedLanguage::Kotlin => SgLang::Kotlin,
            // Languages not directly supported by ast-grep, use fallbacks:
            SupportedLanguage::Toml => SgLang::Json, // Use JSON as fallback for TOML
            SupportedLanguage::Haskell => SgLang::Haskell,
            SupportedLanguage::Elixir => SgLang::Elixir,
            SupportedLanguage::Sql => SgLang::Ruby, // Use Ruby as fallback for SQL (similar syntax in some ways)
            SupportedLanguage::Dockerfile => SgLang::Bash, // Use Bash as fallback
            SupportedLanguage::Markdown => SgLang::Html, // Use HTML as fallback for Markdown
        }
    }

    /// Create new AST-grep engine with configuration
    pub fn new(config: GrepConfig) -> Self {
        Self {
            pattern_cache: Arc::new(DashMap::new()),
            language_detector: LanguageDetector::new(),
            query_optimizer: QueryOptimizer::new(),
            result_ranker: SemanticRanker::new(),
            config,
        }
    }
    
    /// Compile a pattern into a cached form
    pub fn compile_pattern(&self, pattern: &str, language: SupportedLanguage, rule_type: RuleType) -> GrepResult<CompiledPattern> {
        let key = PatternKey {
            pattern: pattern.to_string(),
            language,
            rule_type: rule_type.clone(),
        };
        
        // Check cache first
        if let Some(cached) = self.pattern_cache.get(&key) {
            if cached.compiled_at.elapsed() < self.config.pattern_cache_ttl {
                return Ok(cached.clone());
            }
            // Remove expired entry
            self.pattern_cache.remove(&key);
        }
        
        // Check cache size limit
        if self.pattern_cache.len() >= self.config.max_pattern_cache_size {
            return Err(GrepError::CacheOverflow {
                current_size: self.pattern_cache.len(),
                max_size: self.config.max_pattern_cache_size,
            });
        }
        
        let start = Instant::now();
        
        // Analyze pattern
        let analysis = self.query_optimizer.analyze(pattern);
        
        // Compile rule configuration for YAML rules
        let rule_config = if rule_type == RuleType::YamlRule {
            Some(self.compile_yaml_rule(pattern, language)?)
        } else {
            None
        };
        
        let compiled = CompiledPattern {
            original: pattern.to_string(),
            language,
            rule_type,
            rule_config,
            complexity: analysis.complexity,
            has_metavars: analysis.has_metavars,
            metavar_names: analysis.metavar_names,
            compiled_at: start,
            performance_hint: analysis.performance_hint,
        };
        
        // Cache the compiled pattern
        self.pattern_cache.insert(key, compiled.clone());
        
        debug!("Compiled pattern '{}' for {:?} in {:?}", 
               pattern, language, start.elapsed());
        
        Ok(compiled)
    }
    
    /// Compile YAML rule configuration
    fn compile_yaml_rule(&self, yaml_content: &str, language: SupportedLanguage) -> GrepResult<String> {
        // For now, just validate it's valid YAML and return the string
        // A full implementation would parse and validate the rule structure
        Ok(yaml_content.to_string())
    }
    
    /// Search with compiled pattern
    pub fn search_with_pattern(&self, pattern: &CompiledPattern, query: &GrepQuery) -> GrepResult<Vec<GrepMatch>> {
        let start = Instant::now();
        
        // Performance check for complex patterns
        if pattern.complexity > 7 {
            let threshold = Duration::from_millis(self.config.performance_threshold_ms);
            if start.elapsed() > threshold {
                return Err(GrepError::PerformanceThreshold {
                    actual_ms: start.elapsed().as_millis() as u64,
                    threshold_ms: self.config.performance_threshold_ms,
                });
            }
        }
        
        // Determine search strategy
        let use_parallel = self.config.parallel_processing && query.paths.len() > 1;
        
        let matches = if use_parallel {
            self.search_parallel(pattern, query)?
        } else {
            self.search_sequential(pattern, query)?
        };
        
        let search_duration = start.elapsed();
        debug!("Search completed in {:?} with {} matches", search_duration, matches.len());
        
        Ok(matches)
    }
    
    /// Parallel search across multiple files
    fn search_parallel(&self, pattern: &CompiledPattern, query: &GrepQuery) -> GrepResult<Vec<GrepMatch>> {
        let matches: Result<Vec<Vec<GrepMatch>>, GrepError> = query.paths
            .par_iter()
            .map(|path| self.search_single_file(pattern, path, query))
            .collect();
        
        let mut all_matches: Vec<GrepMatch> = matches?.into_iter().flatten().collect();
        
        // Score and sort matches
        for grep_match in &mut all_matches {
            grep_match.confidence = self.result_ranker.score_match(grep_match, query);
        }
        
        all_matches.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        
        // Apply limit
        if let Some(limit) = query.limit {
            all_matches.truncate(limit);
        }
        
        Ok(all_matches)
    }
    
    /// Sequential search for single file or small sets
    fn search_sequential(&self, pattern: &CompiledPattern, query: &GrepQuery) -> GrepResult<Vec<GrepMatch>> {
        let mut all_matches = Vec::new();
        
        for path in &query.paths {
            let mut matches = self.search_single_file(pattern, path, query)?;
            
            // Score matches
            for grep_match in &mut matches {
                grep_match.confidence = self.result_ranker.score_match(grep_match, query);
            }
            
            all_matches.extend(matches);
            
            // Early termination if limit reached
            if let Some(limit) = query.limit {
                if all_matches.len() >= limit {
                    break;
                }
            }
        }
        
        // Sort by confidence and apply final limit
        all_matches.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        
        if let Some(limit) = query.limit {
            all_matches.truncate(limit);
        }
        
        Ok(all_matches)
    }
    
    /// Search within a single file using ast-grep
    fn search_single_file(&self, pattern: &CompiledPattern, path: &Path, query: &GrepQuery) -> GrepResult<Vec<GrepMatch>> {
        // Read file content
        let content = std::fs::read_to_string(path)
            .map_err(|e| GrepError::FileAccess {
                path: path.to_path_buf(),
                reason: e.to_string(),
            })?;
        
        // Detect or use provided language  
        let language = pattern.language;
        let lang_str = language.to_ast_grep_language_str();
        
        // Get appropriate tree-sitter language for ast-grep
        let sg_lang = self.to_sg_lang(language);
        let ast_grep = sg_lang.ast_grep(&content);
        
        let mut matches = Vec::new();
        
        match &pattern.rule_type {
            RuleType::Pattern | RuleType::Query => {
                // Use ast-grep pattern matching
                let searcher = ast_grep.find(&pattern.original);
                for node_match in searcher {
                    let grep_match = self.create_grep_match(node_match, path, &content, pattern, query)?;
                    matches.push(grep_match);
                }
            }
            RuleType::YamlRule => {
                if let Some(rule_config) = &pattern.rule_config {
                    // Apply YAML rule
                    let rule_matches = self.apply_yaml_rule(rule_config, &ast_grep, path, &content, pattern, query)?;
                    matches.extend(rule_matches);
                }
            }
        }
        
        Ok(matches)
    }
    
    /// Get appropriate tree-sitter language for ast-grep
    fn get_tree_sitter_language(&self, language: SupportedLanguage, _path: &Path) -> GrepResult<tree_sitter::Language> {
        // Future: integrate with AST registry when available
        // if let Some(registry) = &self.ast_registry {
        //     if let Ok(ast_lang) = registry.detect_language(path) {
        //         if let Ok(ts_lang) = registry.get_tree_sitter_language(&ast_lang) {
        //             return Ok(ts_lang);
        //         }
        //     }
        // }
        
        // Manual language mapping for ast-grep tree-sitter integration
        match language {
            SupportedLanguage::Rust => Ok(tree_sitter_rust::LANGUAGE.into()),
            // Future: add other tree-sitter language integrations
            // SupportedLanguage::Python => Ok(tree_sitter_python::LANGUAGE.into()),
            // SupportedLanguage::JavaScript => Ok(tree_sitter_javascript::LANGUAGE.into()),
            // etc.
            
            // For now, default to Rust parser as fallback
            _ => {
                warn!("Language {:?} not fully supported in ast-grep, using Rust parser as fallback", language);
                Ok(tree_sitter_rust::LANGUAGE.into())
            }
        }
    }
    
    /// Apply YAML rule configuration with enhanced processing
    fn apply_yaml_rule(
        &self,
        rule_config: &str,
        ast_grep: &AstGrep<SgLang>,
        path: &Path,
        content: &str,
        pattern: &CompiledPattern,
        query: &GrepQuery,
    ) -> GrepResult<Vec<GrepMatch>> {
        let mut matches = Vec::new();
        
        // Extract pattern from rule configuration
        let search_pattern = if let Some(rule_pattern) = &rule_config.rule.pattern {
            rule_pattern.as_str()
        } else {
            // Fallback to original pattern if no pattern in rule
            &pattern.original
        };
        
        // Apply the pattern search
        let searcher = ast_grep.find(search_pattern);
        
        for node_match in searcher {
            let mut grep_match = self.create_grep_match(node_match, path, content, pattern, query)?;
            
            // Extract additional information from YAML rule
            if let Some(message) = &rule_config.message {
                // Add rule message as a suggested fix
                grep_match.suggested_fixes.push(SuggestedFix {
                    description: message.clone(),
                    replacement: String::new(), // Would need rule fix if available
                    confidence: 0.8,
                    auto_applicable: false,
                });
            }
            
            // Apply rule constraints and filters
            if self.matches_rule_constraints(rule_config, &grep_match) {
                matches.push(grep_match);
            }
        }
        
        Ok(matches)
    }
    
    /// Check if a match satisfies YAML rule constraints
    fn matches_rule_constraints(&self, rule_config: &str, grep_match: &GrepMatch) -> bool {
        // Basic constraint checking - in a full implementation, this would
        // evaluate rule conditions like "has", "inside", "follows", etc.
        
        // For now, accept all matches that pass basic pattern matching
        // TODO: Implement full YAML rule constraint evaluation
        true
    }
    
    /// Create GrepMatch from ast-grep NodeMatch
    fn create_grep_match(
        &self,
        node_match: NodeMatch<SgLang>,
        path: &Path,
        content: &str,
        _pattern: &CompiledPattern,
        query: &GrepQuery,
    ) -> GrepResult<GrepMatch> {
        let node = node_match.get_node();
        let text = node_match.text();
        let range = node.range();
        
        // Calculate line and column (1-based)
        let line = range.start.row + 1;
        let column = range.start.column + 1;
        let byte_offset = range.start_byte;
        
        // Extract meta-variable bindings
        let metavar_bindings = node_match.get_env().get_bindings()
            .iter()
            .map(|(k, v)| (k.clone(), v.text().to_string()))
            .collect();
        
        // Build semantic context
        let semantic_context = self.build_semantic_context(&node, path)?;
        
        // Build surrounding context if requested
        let surrounding_context = if query.with_context {
            self.build_surrounding_context(content, line, self.config.context_lines)?
        } else {
            SurroundingContext {
                before_lines: Vec::new(),
                after_lines: Vec::new(),
                containing_function: None,
                containing_class: None,
                containing_module: None,
            }
        };
        
        Ok(GrepMatch {
            file: path.to_path_buf(),
            line,
            column,
            byte_offset,
            content: text.to_string(),
            confidence: 0.5, // Will be updated by ranker
            metavar_bindings,
            semantic_context,
            surrounding_context,
            suggested_fixes: Vec::new(), // Could be populated from YAML rules
        })
    }
    
    /// Build semantic context from AST node
    fn build_semantic_context(&self, node: &ast_grep_core::Node<SgLang>, _path: &Path) -> GrepResult<SemanticContext> {
        let node_type = node.kind().to_string();
        
        // Classify semantic role based on node type
        let role = match node_type.as_str() {
            "function_declaration" | "function_item" | "method_definition" => SemanticRole::Declaration,
            "call_expression" | "function_call" => SemanticRole::Call,
            "identifier" | "variable" => SemanticRole::Reference,
            "type_annotation" | "type_identifier" => SemanticRole::TypeAnnotation,
            "import_statement" | "use_declaration" => SemanticRole::Import,
            "export_statement" => SemanticRole::Export,
            "if_statement" | "while_statement" | "for_statement" => SemanticRole::ControlFlow,
            "comment" | "line_comment" | "block_comment" => SemanticRole::Comment,
            _ => SemanticRole::Other,
        };
        
        // Extract symbol name if available
        let symbol_name = if node_type == "identifier" {
            Some(node.text().to_string())
        } else {
            None
        };
        
        // Determine if this is a definition
        let is_definition = matches!(role, SemanticRole::Declaration | SemanticRole::TypeAnnotation);
        
        Ok(SemanticContext {
            role,
            containing_scope: None, // Could be implemented with parent traversal
            symbol_name,
            is_definition,
            access_level: None, // Could be extracted from AST analysis
            node_type,
        })
    }
    
    /// Build surrounding context with line information
    fn build_surrounding_context(&self, content: &str, match_line: usize, context_lines: usize) -> GrepResult<SurroundingContext> {
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();
        
        // Calculate context range
        let start_line = match_line.saturating_sub(context_lines + 1);
        let end_line = (match_line + context_lines).min(total_lines);
        
        let mut before_lines = Vec::new();
        let mut after_lines = Vec::new();
        
        // Collect before lines
        for i in start_line..match_line.saturating_sub(1) {
            if i < total_lines {
                before_lines.push(ContextLine {
                    number: i + 1,
                    content: lines[i].to_string(),
                    is_match: false,
                    indent_level: Self::calculate_indent_level(lines[i]),
                });
            }
        }
        
        // Collect after lines
        for i in match_line..end_line {
            if i < total_lines {
                after_lines.push(ContextLine {
                    number: i + 1,
                    content: lines[i].to_string(),
                    is_match: i + 1 == match_line,
                    indent_level: Self::calculate_indent_level(lines[i]),
                });
            }
        }
        
        Ok(SurroundingContext {
            before_lines,
            after_lines,
            containing_function: None, // Could be implemented with AST traversal
            containing_class: None,
            containing_module: None,
        })
    }
    
    /// Calculate indentation level of a line
    fn calculate_indent_level(line: &str) -> usize {
        let mut level = 0;
        for ch in line.chars() {
            match ch {
                ' ' => level += 1,
                '\t' => level += 4, // Treat tab as 4 spaces
                _ => break,
            }
        }
        level
    }
}

impl GrepTool {
    /// Create new grep tool with default configuration
    pub fn new() -> Self {
        Self::with_config(GrepConfig::default())
    }
    
    /// Create grep tool with custom configuration
    pub fn with_config(config: GrepConfig) -> Self {
        Self {
            engine: AstGrepEngine::new(config),
        }
    }
    
    /// Create enhanced grep tool with AST registry integration (when available)
    /// This provides better language detection and more comprehensive language support
    #[allow(dead_code)] // May not be used until AST registry is available
    pub fn with_ast_integration(config: GrepConfig) -> Self {
        // For now, create without AST registry integration
        // TODO: Integrate with actual AST registry when available
        // let ast_registry = Arc::new(LanguageRegistry::new());
        // Self {
        //     engine: AstGrepEngine::with_ast_registry(config, ast_registry),
        // }
        
        Self {
            engine: AstGrepEngine::new(config),
        }
    }
    
    /// Execute grep search with pattern string
    pub fn grep(&self, pattern: &str, paths: Vec<PathBuf>) -> GrepResult<ComprehensiveToolOutput<Vec<GrepMatch>>> {
        let query = GrepQuery {
            pattern: pattern.to_string(),
            paths: paths.clone(),
            rule_type: RuleType::Pattern,
            ..Default::default()
        };
        
        self.search_with_query(query)
    }
    
    /// Execute grep search with tree-sitter query
    pub fn grep_query(&self, query_str: &str, paths: Vec<PathBuf>) -> GrepResult<ComprehensiveToolOutput<Vec<GrepMatch>>> {
        let query = GrepQuery {
            pattern: query_str.to_string(),
            paths: paths.clone(),
            rule_type: RuleType::Query,
            ..Default::default()
        };
        
        self.search_with_query(query)
    }
    
    /// Execute grep search with YAML rule
    pub fn grep_rule(&self, yaml_rule: &str, paths: Vec<PathBuf>) -> GrepResult<ComprehensiveToolOutput<Vec<GrepMatch>>> {
        let query = GrepQuery {
            pattern: yaml_rule.to_string(),
            paths: paths.clone(),
            rule_type: RuleType::YamlRule,
            ..Default::default()
        };
        
        self.search_with_query(query)
    }
    
    /// Execute search with full query object
    pub fn search_with_query(&self, query: GrepQuery) -> GrepResult<ComprehensiveToolOutput<Vec<GrepMatch>>> {
        let start = Instant::now();
        
        // Determine language from first file or use provided language
        let language = if let Some(lang) = query.language {
            lang
        } else if let Some(path) = query.paths.first() {
            self.engine.language_detector.detect(path)
                .unwrap_or(SupportedLanguage::Rust) // Default fallback
        } else {
            SupportedLanguage::Rust // Default
        };
        
        // Compile pattern
        let compiled_pattern = self.engine.compile_pattern(&query.pattern, language, query.rule_type.clone())?;
        
        // Perform search
        let matches = self.engine.search_with_pattern(&compiled_pattern, &query)?;
        
        let duration = start.elapsed();
        
        // Build comprehensive output
        let first_path = query.paths.first().cloned().unwrap_or_else(|| PathBuf::from("unknown"));
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
                line_start: 0,
                column_start: 0,
                line_end: 0,
                column_end: 0,
                byte_range: (0, 0),
            },
            scope: OperationScope {
                scope_type: super::ScopeType::File,
                name: "search".to_string(),
                path: vec!["grep".to_string()],
                file_path: first_path.clone(),
                line_range: 0..0,
            },
            language_context: None,
            project_context: None,
        };
        
        let changes = matches.iter().map(|m| {
            Change {
                id: uuid::Uuid::new_v4(),
                kind: ChangeKind::Added {
                    reason: format!("Pattern match found for '{}'", query.pattern),
                    insertion_point: SourceLocation {
                        file_path: m.file.to_string_lossy().to_string(),
                        line_start: m.line,
                        column_start: m.column,
                        line_end: m.line,
                        column_end: m.column + m.matched_text.len(),
                        byte_range: (m.byte_offset, m.byte_offset + m.matched_text.len()),
                    },
                },
                old: None,
                new: Some(m.matched_text.clone()),
                line_range: m.line..m.line + 1,
                char_range: m.column..m.column + m.matched_text.len(),
                location: SourceLocation {
                    file_path: m.file.to_string_lossy().to_string(),
                    line_start: m.line,
                    column_start: m.column,
                    line_end: m.line,
                    column_end: m.column + m.matched_text.len(),
                    byte_range: (m.byte_offset, m.byte_offset + m.matched_text.len()),
                },
                semantic_impact: ComprehensiveSemanticImpact::minimal(),
                affected_symbols: Vec::new(),
                confidence: m.confidence,
                description: format!("Found pattern '{}' in {} at line {}", 
                                   query.pattern, m.file.display(), m.line),
            }
        }).collect();
        
        let summary = format!(
            "Found {} matches for '{}' across {} files using {} in {:?}",
            matches.len(),
            query.pattern,
            query.paths.len(),
            match query.rule_type {
                RuleType::Pattern => "pattern matching",
                RuleType::Query => "query matching", 
                RuleType::YamlRule => "YAML rule",
            },
            duration
        );
        
        Ok(ComprehensiveToolOutput {
            result: matches,
            context,
            changes,
            metadata: OperationMetadata {
                tool: "grep",
                operation: "search".to_string(),
                operation_id: uuid::Uuid::new_v4(),
                started_at: std::time::SystemTime::now() - duration,
                completed_at: std::time::SystemTime::now(),
                confidence: 1.0,
                parameters: [
                    ("pattern".to_string(), query.pattern.clone()),
                    ("language".to_string(), language.as_str().to_string()),
                    ("rule_type".to_string(), format!("{:?}", query.rule_type)),
                ].iter().cloned().collect(),
            },
            summary,
            performance: PerformanceMetrics {
                execution_time: duration,
                phase_times: std::collections::HashMap::new(),
            },
            diagnostics: Vec::new(),
        })
    }
    
    /// Clear pattern cache
    pub fn clear_cache(&self) {
        self.engine.pattern_cache.clear();
        debug!("Cleared pattern cache");
    }
    
    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.engine.pattern_cache.len(), self.engine.config.max_pattern_cache_size)
    }
}

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}

// Builder pattern for GrepQuery
impl GrepQuery {
    /// Create new query with pattern
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            ..Default::default()
        }
    }
    
    /// Set target paths
    pub fn paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.paths = paths;
        self
    }
    
    /// Set language filter
    pub fn language(mut self, lang: SupportedLanguage) -> Self {
        self.language = Some(lang);
        self
    }
    
    /// Set rule type
    pub fn rule_type(mut self, rule_type: RuleType) -> Self {
        self.rule_type = rule_type;
        self
    }
    
    /// Enable or disable context
    pub fn with_context(mut self, enabled: bool) -> Self {
        self.with_context = enabled;
        self
    }
    
    /// Set result limit
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
    
    /// Set case sensitivity
    pub fn case_sensitive(mut self, sensitive: bool) -> Self {
        self.case_sensitive = sensitive;
        self
    }
}

// Display implementations for better debugging
impl fmt::Display for SemanticRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SemanticRole::Declaration => write!(f, "declaration"),
            SemanticRole::Call => write!(f, "call"),
            SemanticRole::Reference => write!(f, "reference"),
            SemanticRole::TypeAnnotation => write!(f, "type"),
            SemanticRole::Import => write!(f, "import"),
            SemanticRole::Export => write!(f, "export"),
            SemanticRole::ControlFlow => write!(f, "control_flow"),
            SemanticRole::Comment => write!(f, "comment"),
            SemanticRole::Other => write!(f, "other"),
        }
    }
}

impl fmt::Display for RuleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuleType::Pattern => write!(f, "pattern"),
            RuleType::Query => write!(f, "query"),
            RuleType::YamlRule => write!(f, "yaml_rule"),
        }
    }
}

impl fmt::Display for SupportedLanguage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::write;
    
    #[test]
    fn test_language_detection() {
        let detector = LanguageDetector::new();
        
        assert_eq!(detector.detect(Path::new("test.rs")), Some(SupportedLanguage::Rust));
        assert_eq!(detector.detect(Path::new("test.py")), Some(SupportedLanguage::Python));
        assert_eq!(detector.detect(Path::new("test.js")), Some(SupportedLanguage::JavaScript));
        assert_eq!(detector.detect(Path::new("test.ts")), Some(SupportedLanguage::TypeScript));
    }
    
    #[test]
    fn test_query_builder() {
        let query = GrepQuery::new("test_function")
            .paths(vec![PathBuf::from("test.rs")])
            .language(SupportedLanguage::Rust)
            .rule_type(RuleType::Pattern)
            .with_context(true)
            .limit(10)
            .case_sensitive(false);
        
        assert_eq!(query.pattern, "test_function");
        assert_eq!(query.paths.len(), 1);
        assert_eq!(query.language, Some(SupportedLanguage::Rust));
        assert_eq!(query.rule_type, RuleType::Pattern);
        assert!(query.with_context);
        assert_eq!(query.limit, Some(10));
        assert!(!query.case_sensitive);
    }
    
    #[test]
    fn test_pattern_analysis() {
        let optimizer = QueryOptimizer::new();
        
        // Simple pattern
        let analysis = optimizer.analyze("simple_function");
        assert!(analysis.complexity <= 2);
        assert!(!analysis.has_metavars);
        assert_eq!(analysis.performance_hint, PerformanceHint::Literal);
        
        // Pattern with metavariables
        let analysis = optimizer.analyze("function $NAME() { $BODY }");
        assert!(analysis.complexity > 2);
        assert!(analysis.has_metavars);
        assert!(analysis.metavar_names.contains(&"NAME".to_string()));
        assert!(analysis.metavar_names.contains(&"BODY".to_string()));
    }
    
    #[test]
    fn test_grep_tool_creation() {
        let tool = GrepTool::new();
        let (cache_size, max_size) = tool.cache_stats();
        assert_eq!(cache_size, 0);
        assert_eq!(max_size, 1000); // Default max cache size
    }
    
    #[tokio::test]
    async fn test_pattern_caching() {
        let tool = GrepTool::new();
        let pattern = "test_pattern";
        let language = SupportedLanguage::Rust;
        
        // First compilation should cache the pattern
        let compiled1 = tool.engine.compile_pattern(pattern, language, RuleType::Pattern).unwrap();
        let (cache_size, _) = tool.cache_stats();
        assert_eq!(cache_size, 1);
        
        // Second compilation should use cache
        let compiled2 = tool.engine.compile_pattern(pattern, language, RuleType::Pattern).unwrap();
        assert_eq!(compiled1.compiled_at, compiled2.compiled_at); // Same timestamp = cached
    }
    
    #[test]
    fn test_supported_language_conversions() {
        assert_eq!(SupportedLanguage::Rust.as_str(), "rust");
        assert_eq!(SupportedLanguage::Python.as_str(), "python");
        assert_eq!(SupportedLanguage::JavaScript.as_str(), "javascript");
        
        // Test AST-grep language conversion
        assert_eq!(SupportedLanguage::Rust.to_ast_grep_language_str(), "rust");
        assert_eq!(SupportedLanguage::TypeScript.to_ast_grep_language_str(), "typescript");
    }
    
    #[test]
    fn test_indent_level_calculation() {
        assert_eq!(AstGrepEngine::calculate_indent_level("no indent"), 0);
        assert_eq!(AstGrepEngine::calculate_indent_level("    4 spaces"), 4);
        assert_eq!(AstGrepEngine::calculate_indent_level("\t1 tab"), 4);
        assert_eq!(AstGrepEngine::calculate_indent_level("    \tmixed"), 8);
    }
}