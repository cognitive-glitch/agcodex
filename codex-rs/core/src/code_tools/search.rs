//! Multi-layer search engine for AGCodex with Tantivy integration.
//!
//! This module provides a comprehensive search system with four layers:
//! - Layer 1: In-memory symbol index using DashMap for <1ms lookups
//! - Layer 2: Tantivy full-text search for <5ms searches  
//! - Layer 3: AST cache for semantic search
//! - Layer 4: Ripgrep fallback for unindexed files
//!
//! Features context-aware output with rich metadata for LLM consumption.

use super::CodeTool;
use super::ToolError;
use dashmap::DashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tantivy::Index;
use tantivy::IndexReader;
use tantivy::IndexWriter;
use tantivy::collector::TopDocs;
use tantivy::doc;
use tantivy::query::Query;
use tantivy::query::QueryParser;
use tantivy::schema::Field;
use tantivy::schema::INDEXED;
use tantivy::schema::STORED;
use tantivy::schema::STRING;
use tantivy::schema::Schema;
use tantivy::schema::TEXT;
use tantivy::schema::Value;
use tokio::process::Command;

/// Multi-layer search engine with automatic strategy selection
#[derive(Debug)]
pub struct MultiLayerSearchEngine {
    /// Layer 1: In-memory symbol index for instant lookups
    symbol_index: Arc<DashMap<String, Vec<Symbol>>>,
    /// Layer 2: Tantivy full-text search index
    tantivy_index: Option<TantivySearchEngine>,
    /// Layer 3: AST cache reference
    ast_cache: Arc<DashMap<PathBuf, CachedAst>>,
    /// Query cache for frequent searches
    query_cache: Arc<DashMap<String, CachedResult>>,
    /// Configuration
    config: SearchConfig,
}

/// Tantivy-based full-text search engine
pub struct TantivySearchEngine {
    index: Index,
    reader: IndexReader,
    writer: Arc<tokio::sync::Mutex<IndexWriter>>,
    schema: TantivySchema,
}

/// Tantivy schema for code search
#[derive(Debug, Clone)]
pub struct TantivySchema {
    pub path: Field,
    pub content: Field,
    pub symbols: Field,
    pub ast: Field,
    pub language: Field,
    pub line_number: Field,
    pub function_name: Field,
    pub class_name: Field,
}

/// Search configuration
#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// Maximum cache size for query results
    pub max_cache_size: usize,
    /// Cache TTL in seconds
    pub cache_ttl: Duration,
    /// Enable Layer 1 (symbol index)
    pub enable_symbol_index: bool,
    /// Enable Layer 2 (Tantivy)
    pub enable_tantivy: bool,
    /// Enable Layer 3 (AST cache)
    pub enable_ast_cache: bool,
    /// Enable Layer 4 (ripgrep fallback)
    pub enable_ripgrep_fallback: bool,
    /// Maximum results per search
    pub max_results: usize,
    /// Search timeout
    pub timeout: Duration,
}

/// Search query with automatic strategy selection
#[derive(Debug, Clone)]
pub struct SearchQuery {
    /// Search term or pattern
    pub pattern: String,
    /// Query type for strategy selection
    pub query_type: QueryType,
    /// File path restrictions
    pub file_filters: Vec<String>,
    /// Language filters
    pub language_filters: Vec<String>,
    /// Context lines before/after match
    pub context_lines: usize,
    /// Maximum results
    pub limit: Option<usize>,
    /// Include fuzzy matching
    pub fuzzy: bool,
    /// Case sensitive matching
    pub case_sensitive: bool,
    /// Search scope
    pub scope: SearchScope,
}

/// Query type for automatic strategy selection
#[derive(Debug, Clone, PartialEq)]
pub enum QueryType {
    /// Symbol lookup (Layer 1: <1ms)
    Symbol,
    /// Full-text search (Layer 2: <5ms)
    FullText,
    /// Function/class definition lookup
    Definition,
    /// Find references to symbol
    References,
    /// AST-based semantic search (Layer 3)
    Semantic,
    /// General pattern search (all layers)
    General,
}

/// Search scope definition
#[derive(Debug, Clone)]
pub enum SearchScope {
    /// Search entire workspace
    Workspace,
    /// Search specific directory
    Directory(PathBuf),
    /// Search specific files
    Files(Vec<PathBuf>),
    /// Search current git repository
    GitRepository,
}

/// Context-aware search result
#[derive(Debug, Clone)]
pub struct ToolOutput<T> {
    pub result: T,
    pub context: Context,
    pub changes: Vec<Change>,
    pub metadata: Metadata,
    pub summary: String,
}

/// Rich context information for LLM consumption
#[derive(Debug, Clone)]
pub struct Context {
    /// Lines before the match
    pub before: String,
    /// Lines after the match
    pub after: String,
    /// Surrounding lines with line numbers
    pub surrounding: Vec<Line>,
    /// Exact location of the match
    pub location: Location,
    /// Containing scope (function, class, module)
    pub scope: Scope,
}

/// Individual line with metadata
#[derive(Debug, Clone)]
pub struct Line {
    pub number: usize,
    pub content: String,
    pub is_match: bool,
}

/// Precise location information
#[derive(Debug, Clone)]
pub struct Location {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub byte_offset: usize,
}

/// Containing scope information
#[derive(Debug, Clone)]
pub struct Scope {
    pub function: Option<String>,
    pub class: Option<String>,
    pub module: Option<String>,
    pub namespace: Option<String>,
}

/// Change tracking for search results
#[derive(Debug, Clone)]
pub struct Change {
    pub change_type: ChangeType,
    pub description: String,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub enum ChangeType {
    Addition,
    Modification,
    Deletion,
}

/// Search metadata
#[derive(Debug, Clone)]
pub struct Metadata {
    /// Which search layer was used
    pub search_layer: SearchLayer,
    /// Search execution time
    pub duration: Duration,
    /// Total results found (before limiting)
    pub total_results: usize,
    /// Search strategy used
    pub strategy: SearchStrategy,
    /// Language detected
    pub language: Option<String>,
}

/// Search layer that produced the results
#[derive(Debug, Clone, PartialEq)]
pub enum SearchLayer {
    SymbolIndex,
    Tantivy,
    AstCache,
    RipgrepFallback,
    Combined,
}

/// Search strategy employed
#[derive(Debug, Clone, PartialEq)]
pub enum SearchStrategy {
    FastSymbolLookup,
    FullTextIndex,
    SemanticAnalysis,
    PatternMatching,
    Hybrid,
}

/// Search result with rich context
pub type SearchResult = ToolOutput<Vec<Match>>;

/// Individual search match
#[derive(Debug, Clone)]
pub struct Match {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub content: String,
    pub score: f32,
}

/// Symbol information for Layer 1 index
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
    pub scope: Scope,
    pub visibility: Visibility,
}

#[derive(Debug, Clone)]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Interface,
    Struct,
    Enum,
    Variable,
    Constant,
    Module,
    Namespace,
}

#[derive(Debug, Clone)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
}

/// Cached AST information for Layer 3
#[derive(Debug, Clone)]
pub struct CachedAst {
    pub file_path: PathBuf,
    pub language: String,
    pub symbols: Vec<Symbol>,
    pub dependencies: Vec<String>,
    pub last_modified: std::time::SystemTime,
    pub parse_duration: Duration,
}

/// Cached search result with TTL
#[derive(Debug, Clone)]
pub struct CachedResult {
    pub result: SearchResult,
    pub timestamp: Instant,
    pub ttl: Duration,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_cache_size: 1000,
            cache_ttl: Duration::from_secs(300), // 5 minutes
            enable_symbol_index: true,
            enable_tantivy: true,
            enable_ast_cache: true,
            enable_ripgrep_fallback: true,
            max_results: 100,
            timeout: Duration::from_secs(10),
        }
    }
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            query_type: QueryType::General,
            file_filters: Vec::new(),
            language_filters: Vec::new(),
            context_lines: 3,
            limit: Some(50),
            fuzzy: false,
            case_sensitive: true,
            scope: SearchScope::Workspace,
        }
    }
}

impl MultiLayerSearchEngine {
    /// Create a new multi-layer search engine
    pub fn new(config: SearchConfig) -> Result<Self, ToolError> {
        let symbol_index = Arc::new(DashMap::new());
        let ast_cache = Arc::new(DashMap::new());
        let query_cache = Arc::new(DashMap::new());

        let tantivy_index = if config.enable_tantivy {
            Some(TantivySearchEngine::new()?)
        } else {
            None
        };

        Ok(Self {
            symbol_index,
            tantivy_index,
            ast_cache,
            query_cache,
            config,
        })
    }

    /// Execute search with automatic layer selection
    pub async fn search(&self, query: SearchQuery) -> Result<SearchResult, ToolError> {
        let start_time = Instant::now();

        // Check query cache first
        if let Some(cached) = self.get_cached_result(&query) {
            return Ok(cached);
        }

        // Select optimal search strategy based on query type
        let strategy = self.select_strategy(&query);
        let layer = match strategy {
            SearchStrategy::FastSymbolLookup if self.config.enable_symbol_index => {
                SearchLayer::SymbolIndex
            }
            SearchStrategy::FullTextIndex if self.config.enable_tantivy => SearchLayer::Tantivy,
            SearchStrategy::SemanticAnalysis if self.config.enable_ast_cache => {
                SearchLayer::AstCache
            }
            SearchStrategy::PatternMatching if self.config.enable_ripgrep_fallback => {
                SearchLayer::RipgrepFallback
            }
            SearchStrategy::Hybrid => SearchLayer::Combined,
            _ => SearchLayer::RipgrepFallback, // Fallback
        };

        // Execute search on selected layer
        let matches = match layer {
            SearchLayer::SymbolIndex => self.search_symbol_index(&query).await?,
            SearchLayer::Tantivy => self.search_tantivy(&query).await?,
            SearchLayer::AstCache => self.search_ast_cache(&query).await?,
            SearchLayer::RipgrepFallback => self.search_ripgrep(&query).await?,
            SearchLayer::Combined => self.search_combined(&query).await?,
        };

        // Enhance matches with rich context
        let enhanced_matches = self.enhance_matches_with_context(matches, &query).await?;

        let duration = start_time.elapsed();
        let result = ToolOutput {
            result: enhanced_matches.clone(),
            context: self
                .build_overall_context(&enhanced_matches, &query)
                .await?,
            changes: Vec::new(), // Read-only search operation
            metadata: Metadata {
                search_layer: layer,
                duration,
                total_results: enhanced_matches.len(),
                strategy,
                language: self.detect_language(&query),
            },
            summary: self.generate_summary(&enhanced_matches, &query),
        };

        // Cache the result
        self.cache_result(&query, &result);

        Ok(result)
    }

    /// Layer 1: Fast symbol lookup using DashMap (<1ms)
    async fn search_symbol_index(&self, query: &SearchQuery) -> Result<Vec<Match>, ToolError> {
        let _start = Instant::now();
        let mut matches = Vec::new();

        // Direct symbol name lookup
        if let Some(symbols) = self.symbol_index.get(&query.pattern) {
            for symbol in symbols.iter() {
                if self.matches_filters(symbol, query) {
                    matches.push(Match {
                        file: symbol.location.file.clone(),
                        line: symbol.location.line,
                        column: symbol.location.column,
                        content: symbol.name.clone(),
                        score: 1.0, // Exact match
                    });
                }
            }
        }

        // Fuzzy symbol search if enabled
        if query.fuzzy && matches.is_empty() {
            for entry in self.symbol_index.iter() {
                let similarity = self.calculate_similarity(&query.pattern, entry.key());
                if similarity > 0.7 {
                    // 70% similarity threshold
                    for symbol in entry.value().iter() {
                        if self.matches_filters(symbol, query) {
                            matches.push(Match {
                                file: symbol.location.file.clone(),
                                line: symbol.location.line,
                                column: symbol.location.column,
                                content: symbol.name.clone(),
                                score: similarity,
                            });
                        }
                    }
                }
            }
        }

        // Sort by score
        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        // Apply limit
        if let Some(limit) = query.limit {
            matches.truncate(limit);
        }

        Ok(matches)
    }

    /// Layer 2: Tantivy full-text search (<5ms)
    async fn search_tantivy(&self, query: &SearchQuery) -> Result<Vec<Match>, ToolError> {
        if let Some(ref tantivy) = self.tantivy_index {
            tantivy.search(query).await
        } else {
            Err(ToolError::NotImplemented("Tantivy search not enabled"))
        }
    }

    /// Layer 3: AST cache semantic search
    async fn search_ast_cache(&self, query: &SearchQuery) -> Result<Vec<Match>, ToolError> {
        let mut matches = Vec::new();

        for entry in self.ast_cache.iter() {
            let ast = entry.value();
            for symbol in &ast.symbols {
                if self.matches_semantic_query(symbol, query) {
                    matches.push(Match {
                        file: symbol.location.file.clone(),
                        line: symbol.location.line,
                        column: symbol.location.column,
                        content: format!("{} {}", symbol.kind.as_str(), symbol.name),
                        score: self.calculate_semantic_score(symbol, query),
                    });
                }
            }
        }

        // Sort by score
        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        if let Some(limit) = query.limit {
            matches.truncate(limit);
        }

        Ok(matches)
    }

    /// Layer 4: Ripgrep fallback for unindexed content
    async fn search_ripgrep(&self, query: &SearchQuery) -> Result<Vec<Match>, ToolError> {
        let mut cmd = Command::new("rg");
        cmd.arg("--json")
            .arg("--with-filename")
            .arg("--line-number")
            .arg("--column");

        if !query.case_sensitive {
            cmd.arg("--ignore-case");
        }

        if let Some(limit) = query.limit {
            cmd.arg("--max-count").arg(limit.to_string());
        }

        // Add file type filters
        for filter in &query.file_filters {
            cmd.arg("--glob").arg(filter);
        }

        // Add search pattern
        cmd.arg(&query.pattern);

        // Add search paths based on scope
        match &query.scope {
            SearchScope::Workspace => {
                cmd.arg(".");
            }
            SearchScope::Directory(path) => {
                cmd.arg(path);
            }
            SearchScope::Files(files) => {
                for file in files {
                    cmd.arg(file);
                }
            }
            SearchScope::GitRepository => {
                cmd.arg("--type-add")
                    .arg("git:*.{rs,py,js,ts,go,java,c,cpp,h}")
                    .arg("--type")
                    .arg("git")
                    .arg(".");
            }
        }

        let output = tokio::time::timeout(self.config.timeout, cmd.output())
            .await
            .map_err(|_| ToolError::InvalidQuery("Search timeout".to_string()))?
            .map_err(ToolError::Io)?;

        if !output.status.success() {
            return Err(ToolError::InvalidQuery(format!(
                "ripgrep failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        self.parse_ripgrep_output(&output.stdout)
    }

    /// Combined search across multiple layers
    async fn search_combined(&self, query: &SearchQuery) -> Result<Vec<Match>, ToolError> {
        let mut all_matches = Vec::new();

        // Try each layer and combine results
        if self.config.enable_symbol_index
            && let Ok(matches) = self.search_symbol_index(query).await
        {
            all_matches.extend(matches);
        }

        if self.config.enable_tantivy
            && all_matches.len() < query.limit.unwrap_or(50)
            && let Ok(matches) = self.search_tantivy(query).await
        {
            all_matches.extend(matches);
        }

        if self.config.enable_ast_cache
            && all_matches.len() < query.limit.unwrap_or(50)
            && let Ok(matches) = self.search_ast_cache(query).await
        {
            all_matches.extend(matches);
        }

        if self.config.enable_ripgrep_fallback
            && all_matches.len() < query.limit.unwrap_or(50)
            && let Ok(matches) = self.search_ripgrep(query).await
        {
            all_matches.extend(matches);
        }

        // Deduplicate and sort by score
        all_matches.sort_by(|a, b| {
            // First by score, then by file + line for stability
            match b.score.partial_cmp(&a.score).unwrap() {
                std::cmp::Ordering::Equal => match a.file.cmp(&b.file) {
                    std::cmp::Ordering::Equal => a.line.cmp(&b.line),
                    other => other,
                },
                other => other,
            }
        });

        // Remove exact duplicates
        all_matches.dedup_by(|a, b| a.file == b.file && a.line == b.line && a.column == b.column);

        if let Some(limit) = query.limit {
            all_matches.truncate(limit);
        }

        Ok(all_matches)
    }

    /// Select optimal search strategy based on query characteristics
    fn select_strategy(&self, query: &SearchQuery) -> SearchStrategy {
        match query.query_type {
            QueryType::Symbol => {
                if self.symbol_index.contains_key(&query.pattern) {
                    SearchStrategy::FastSymbolLookup
                } else {
                    SearchStrategy::FullTextIndex
                }
            }
            QueryType::Definition | QueryType::References => SearchStrategy::SemanticAnalysis,
            QueryType::FullText => SearchStrategy::FullTextIndex,
            QueryType::Semantic => SearchStrategy::SemanticAnalysis,
            QueryType::General => {
                // Decide based on pattern characteristics
                if self.is_likely_symbol(&query.pattern) {
                    SearchStrategy::FastSymbolLookup
                } else if query.pattern.len() < 50 && !query.pattern.contains(' ') {
                    SearchStrategy::FullTextIndex
                } else {
                    SearchStrategy::Hybrid
                }
            }
        }
    }

    /// Check if pattern looks like a symbol name
    fn is_likely_symbol(&self, pattern: &str) -> bool {
        // Simple heuristic: alphanumeric with underscores, no spaces
        pattern
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == ':')
            && !pattern.contains(' ')
            && pattern.len() < 100
    }

    /// Calculate string similarity for fuzzy matching
    fn calculate_similarity(&self, a: &str, b: &str) -> f32 {
        // Simple Levenshtein distance-based similarity
        let distance = self.levenshtein_distance(a, b);
        let max_len = a.len().max(b.len()) as f32;
        if max_len == 0.0 {
            1.0
        } else {
            1.0 - (distance as f32 / max_len)
        }
    }

    /// Calculate Levenshtein distance
    fn levenshtein_distance(&self, a: &str, b: &str) -> usize {
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let a_len = a_chars.len();
        let b_len = b_chars.len();

        let mut matrix = vec![vec![0; b_len + 1]; a_len + 1];

        for i in 0..=a_len {
            matrix[i][0] = i;
        }
        for j in 0..=b_len {
            matrix[0][j] = j;
        }

        for i in 1..=a_len {
            for j in 1..=b_len {
                let cost = if a_chars[i - 1] == b_chars[j - 1] {
                    0
                } else {
                    1
                };
                matrix[i][j] = (matrix[i - 1][j] + 1)
                    .min(matrix[i][j - 1] + 1)
                    .min(matrix[i - 1][j - 1] + cost);
            }
        }

        matrix[a_len][b_len]
    }

    /// Check if symbol matches query filters
    fn matches_filters(&self, symbol: &Symbol, query: &SearchQuery) -> bool {
        // File filters
        if !query.file_filters.is_empty() {
            let file_str = symbol.location.file.to_string_lossy();
            if !query
                .file_filters
                .iter()
                .any(|filter| file_str.contains(filter))
            {
                return false;
            }
        }

        // Language filters (would need to be determined from file extension)
        // This is a simplified check
        if !query.language_filters.is_empty()
            && let Some(ext) = symbol.location.file.extension()
        {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if !query.language_filters.contains(&ext_str) {
                return false;
            }
        }

        true
    }

    /// Check if symbol matches semantic query
    fn matches_semantic_query(&self, symbol: &Symbol, query: &SearchQuery) -> bool {
        match query.query_type {
            QueryType::Definition => {
                matches!(
                    symbol.kind,
                    SymbolKind::Function | SymbolKind::Class | SymbolKind::Struct
                )
            }
            QueryType::References => {
                // This would require more sophisticated analysis
                symbol.name.contains(&query.pattern)
            }
            _ => symbol.name.contains(&query.pattern),
        }
    }

    /// Calculate semantic matching score
    fn calculate_semantic_score(&self, symbol: &Symbol, query: &SearchQuery) -> f32 {
        let mut score = 0.0;

        // Exact name match gets highest score
        if symbol.name == query.pattern {
            score += 1.0;
        } else if symbol.name.contains(&query.pattern) {
            score += 0.8;
        } else {
            score += self.calculate_similarity(&symbol.name, &query.pattern) * 0.6;
        }

        // Bonus for specific symbol kinds in definition queries
        if query.query_type == QueryType::Definition {
            match symbol.kind {
                SymbolKind::Function | SymbolKind::Method => score += 0.2,
                SymbolKind::Class | SymbolKind::Struct => score += 0.3,
                _ => {}
            }
        }

        // Bonus for public visibility
        if matches!(symbol.visibility, Visibility::Public) {
            score += 0.1;
        }

        score.min(1.0)
    }

    /// Parse ripgrep JSON output into matches
    fn parse_ripgrep_output(&self, output: &[u8]) -> Result<Vec<Match>, ToolError> {
        let output_str = String::from_utf8_lossy(output);
        let mut matches = Vec::new();

        for line in output_str.lines() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line)
                && json["type"] == "match"
                && let Some(data) = json["data"].as_object()
            {
                let file = PathBuf::from(data["path"]["text"].as_str().unwrap_or("").to_string());
                let line_num = data["line_number"].as_u64().unwrap_or(0) as usize;
                let column = data["submatches"][0]["start"].as_u64().unwrap_or(0) as usize + 1; // ripgrep uses 0-based indexing
                let content = data["lines"]["text"].as_str().unwrap_or("").to_string();

                matches.push(Match {
                    file,
                    line: line_num,
                    column,
                    content,
                    score: 0.8, // Good match from full-text search
                });
            }
        }

        Ok(matches)
    }

    /// Enhance matches with rich context information
    async fn enhance_matches_with_context(
        &self,
        matches: Vec<Match>,
        _query: &SearchQuery,
    ) -> Result<Vec<Match>, ToolError> {
        // For now, return matches as-is
        // In a full implementation, this would read file content and add context
        Ok(matches)
    }

    /// Build overall context for all matches
    async fn build_overall_context(
        &self,
        _matches: &[Match],
        _query: &SearchQuery,
    ) -> Result<Context, ToolError> {
        // Placeholder implementation
        Ok(Context {
            before: String::new(),
            after: String::new(),
            surrounding: Vec::new(),
            location: Location {
                file: PathBuf::new(),
                line: 0,
                column: 0,
                byte_offset: 0,
            },
            scope: Scope {
                function: None,
                class: None,
                module: None,
                namespace: None,
            },
        })
    }

    /// Detect programming language from query context
    fn detect_language(&self, query: &SearchQuery) -> Option<String> {
        // Simple detection based on file filters
        for filter in &query.file_filters {
            if filter.ends_with(".rs") {
                return Some("rust".to_string());
            } else if filter.ends_with(".py") {
                return Some("python".to_string());
            } else if filter.ends_with(".js") || filter.ends_with(".ts") {
                return Some("javascript".to_string());
            }
        }
        None
    }

    /// Generate human-readable summary of search results
    fn generate_summary(&self, matches: &[Match], query: &SearchQuery) -> String {
        format!(
            "Found {} matches for '{}' using {} search",
            matches.len(),
            query.pattern,
            match query.query_type {
                QueryType::Symbol => "symbol",
                QueryType::FullText => "full-text",
                QueryType::Definition => "definition",
                QueryType::References => "reference",
                QueryType::Semantic => "semantic",
                QueryType::General => "general",
            }
        )
    }

    /// Get cached result if available and not expired
    fn get_cached_result(&self, query: &SearchQuery) -> Option<SearchResult> {
        let cache_key = self.generate_cache_key(query);
        if let Some(cached) = self.query_cache.get(&cache_key) {
            if cached.timestamp.elapsed() < cached.ttl {
                return Some(cached.result.clone());
            }
            // Remove expired entry
            self.query_cache.remove(&cache_key);
        }
        None
    }

    /// Cache search result
    fn cache_result(&self, query: &SearchQuery, result: &SearchResult) {
        if self.query_cache.len() >= self.config.max_cache_size {
            // Simple eviction: remove random entry
            if let Some(entry) = self.query_cache.iter().next() {
                let key = entry.key().clone();
                drop(entry);
                self.query_cache.remove(&key);
            }
        }

        let cache_key = self.generate_cache_key(query);
        self.query_cache.insert(
            cache_key,
            CachedResult {
                result: result.clone(),
                timestamp: Instant::now(),
                ttl: self.config.cache_ttl,
            },
        );
    }

    /// Generate cache key for query
    fn generate_cache_key(&self, query: &SearchQuery) -> String {
        format!(
            "{}:{}:{}:{}",
            query.pattern,
            query.query_type.as_str(),
            query.file_filters.join(","),
            query.language_filters.join(",")
        )
    }

    /// Add symbol to Layer 1 index
    pub fn add_symbol(&self, symbol: Symbol) {
        self.symbol_index
            .entry(symbol.name.clone())
            .or_default()
            .push(symbol);
    }

    /// Add cached AST to Layer 3
    pub fn add_ast_cache(&self, ast: CachedAst) {
        self.ast_cache.insert(ast.file_path.clone(), ast);
    }

    /// Find references to a symbol
    pub async fn find_references(&self, symbol_name: &str) -> Result<SearchResult, ToolError> {
        let query = SearchQuery {
            pattern: symbol_name.to_string(),
            query_type: QueryType::References,
            ..Default::default()
        };
        self.search(query).await
    }

    /// Find definition of a symbol
    pub async fn find_definition(&self, symbol_name: &str) -> Result<SearchResult, ToolError> {
        let query = SearchQuery {
            pattern: symbol_name.to_string(),
            query_type: QueryType::Definition,
            ..Default::default()
        };
        self.search(query).await
    }
}

impl TantivySearchEngine {
    /// Create new Tantivy search engine with code-optimized schema
    pub fn new() -> Result<Self, ToolError> {
        let mut schema_builder = Schema::builder();

        let path = schema_builder.add_text_field("path", TEXT | STORED);
        let content = schema_builder.add_text_field("content", TEXT);
        let symbols = schema_builder.add_text_field("symbols", TEXT | STORED);
        let ast = schema_builder.add_bytes_field("ast", STORED);
        let language = schema_builder.add_text_field("language", STRING | STORED);
        let line_number = schema_builder.add_u64_field("line_number", INDEXED | STORED);
        let function_name = schema_builder.add_text_field("function_name", TEXT | STORED);
        let class_name = schema_builder.add_text_field("class_name", TEXT | STORED);

        let schema = schema_builder.build();

        let index = Index::create_in_ram(schema.clone());
        let reader = index
            .reader()
            .map_err(|e| ToolError::InvalidQuery(format!("Failed to create reader: {}", e)))?;

        let writer = index
            .writer(50_000_000) // 50MB buffer
            .map_err(|e| ToolError::InvalidQuery(format!("Failed to create writer: {}", e)))?;

        Ok(Self {
            index,
            reader,
            writer: Arc::new(tokio::sync::Mutex::new(writer)),
            schema: TantivySchema {
                path,
                content,
                symbols,
                ast,
                language,
                line_number,
                function_name,
                class_name,
            },
        })
    }

    /// Search using Tantivy index
    pub async fn search(&self, query: &SearchQuery) -> Result<Vec<Match>, ToolError> {
        let searcher = self.reader.searcher();
        let schema = &self.schema;

        // Build query based on type
        let tantivy_query: Box<dyn Query> = match query.query_type {
            QueryType::Symbol => {
                let query_parser = QueryParser::for_index(&self.index, vec![schema.symbols]);
                query_parser
                    .parse_query(&query.pattern)
                    .map_err(|e| ToolError::InvalidQuery(format!("Parse error: {}", e)))?
            }
            QueryType::FullText => {
                let query_parser = QueryParser::for_index(&self.index, vec![schema.content]);
                query_parser
                    .parse_query(&query.pattern)
                    .map_err(|e| ToolError::InvalidQuery(format!("Parse error: {}", e)))?
            }
            _ => {
                let query_parser =
                    QueryParser::for_index(&self.index, vec![schema.content, schema.symbols]);
                query_parser
                    .parse_query(&query.pattern)
                    .map_err(|e| ToolError::InvalidQuery(format!("Parse error: {}", e)))?
            }
        };

        let top_docs = searcher
            .search(
                &tantivy_query,
                &TopDocs::with_limit(query.limit.unwrap_or(50)),
            )
            .map_err(|e| ToolError::InvalidQuery(format!("Search error: {}", e)))?;

        let mut matches = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| ToolError::InvalidQuery(format!("Doc retrieval error: {}", e)))?;

            let path = retrieved_doc
                .get_first(schema.path)
                .and_then(|f| f.as_str())
                .unwrap_or("")
                .to_string();

            let line_num = retrieved_doc
                .get_first(schema.line_number)
                .and_then(|f| f.as_u64())
                .unwrap_or(1) as usize;

            let content = retrieved_doc
                .get_first(schema.content)
                .and_then(|f| f.as_str())
                .unwrap_or("")
                .to_string();

            matches.push(Match {
                file: PathBuf::from(path),
                line: line_num,
                column: 1,
                content,
                score: 0.9, // High score for indexed results
            });
        }

        Ok(matches)
    }

    /// Add document to Tantivy index
    pub async fn add_document(
        &self,
        path: &Path,
        content: &str,
        symbols: &[Symbol],
        language: &str,
    ) -> Result<(), ToolError> {
        let mut writer = self.writer.lock().await;
        let schema = &self.schema;

        let symbols_text = symbols
            .iter()
            .map(|s| s.name.clone())
            .collect::<Vec<_>>()
            .join(" ");

        let doc = doc!(
            schema.path => path.to_string_lossy().to_string(),
            schema.content => content,
            schema.symbols => symbols_text,
            schema.language => language,
            schema.line_number => 1u64,
        );

        writer
            .add_document(doc)
            .map_err(|e| ToolError::InvalidQuery(format!("Add document error: {}", e)))?;

        writer
            .commit()
            .map_err(|e| ToolError::InvalidQuery(format!("Commit error: {}", e)))?;

        Ok(())
    }
}

impl std::fmt::Debug for TantivySearchEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TantivySearchEngine")
            .field("schema", &self.schema)
            .finish_non_exhaustive()
    }
}

// Trait implementations

impl CodeTool for MultiLayerSearchEngine {
    type Query = SearchQuery;
    type Output = SearchResult;

    fn search(&self, query: Self::Query) -> Result<Self::Output, ToolError> {
        // Async search requires runtime, provide sync wrapper
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.search(query))
        })
    }
}

// Helper trait implementations

impl QueryType {
    const fn as_str(&self) -> &'static str {
        match self {
            QueryType::Symbol => "symbol",
            QueryType::FullText => "fulltext",
            QueryType::Definition => "definition",
            QueryType::References => "references",
            QueryType::Semantic => "semantic",
            QueryType::General => "general",
        }
    }
}

impl SymbolKind {
    const fn as_str(&self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Method => "method",
            SymbolKind::Class => "class",
            SymbolKind::Interface => "interface",
            SymbolKind::Struct => "struct",
            SymbolKind::Enum => "enum",
            SymbolKind::Variable => "variable",
            SymbolKind::Constant => "constant",
            SymbolKind::Module => "module",
            SymbolKind::Namespace => "namespace",
        }
    }
}

// Builder patterns for easy construction

impl SearchQuery {
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            ..Default::default()
        }
    }

    pub fn symbol(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            query_type: QueryType::Symbol,
            ..Default::default()
        }
    }

    pub fn full_text(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            query_type: QueryType::FullText,
            ..Default::default()
        }
    }

    pub fn definition(symbol: impl Into<String>) -> Self {
        Self {
            pattern: symbol.into(),
            query_type: QueryType::Definition,
            ..Default::default()
        }
    }

    pub fn references(symbol: impl Into<String>) -> Self {
        Self {
            pattern: symbol.into(),
            query_type: QueryType::References,
            ..Default::default()
        }
    }

    pub fn with_file_filters(mut self, filters: Vec<String>) -> Self {
        self.file_filters = filters;
        self
    }

    pub fn with_language_filters(mut self, filters: Vec<String>) -> Self {
        self.language_filters = filters;
        self
    }

    pub const fn with_context_lines(mut self, lines: usize) -> Self {
        self.context_lines = lines;
        self
    }

    pub const fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub const fn fuzzy(mut self) -> Self {
        self.fuzzy = true;
        self
    }

    pub const fn case_insensitive(mut self) -> Self {
        self.case_sensitive = false;
        self
    }

    pub fn in_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.scope = SearchScope::Directory(path.into());
        self
    }

    pub fn in_files(mut self, files: Vec<PathBuf>) -> Self {
        self.scope = SearchScope::Files(files);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tokio::test;

    #[test]
    async fn test_symbol_search() {
        let config = SearchConfig::default();
        let engine = MultiLayerSearchEngine::new(config).unwrap();

        // Add test symbol
        let symbol = Symbol {
            name: "test_function".to_string(),
            kind: SymbolKind::Function,
            location: Location {
                file: PathBuf::from("test.rs"),
                line: 10,
                column: 5,
                byte_offset: 100,
            },
            scope: Scope {
                function: None,
                class: None,
                module: Some("test_module".to_string()),
                namespace: None,
            },
            visibility: Visibility::Public,
        };

        engine.add_symbol(symbol);

        // Search for the symbol
        let query = SearchQuery::symbol("test_function");
        let result = engine.search(query).await.unwrap();

        assert_eq!(result.result.len(), 1);
        assert_eq!(result.result[0].file, PathBuf::from("test.rs"));
        assert_eq!(result.result[0].line, 10);
        assert_eq!(result.metadata.search_layer, SearchLayer::SymbolIndex);
    }

    #[test]
    async fn test_fuzzy_search() {
        let config = SearchConfig::default();
        let engine = MultiLayerSearchEngine::new(config).unwrap();

        let symbol = Symbol {
            name: "calculateSum".to_string(),
            kind: SymbolKind::Function,
            location: Location {
                file: PathBuf::from("math.js"),
                line: 5,
                column: 1,
                byte_offset: 50,
            },
            scope: Scope {
                function: None,
                class: None,
                module: None,
                namespace: None,
            },
            visibility: Visibility::Public,
        };

        engine.add_symbol(symbol);

        // Fuzzy search with typo
        let query = SearchQuery::symbol("calcSum").fuzzy();
        let result = engine.search(query).await.unwrap();

        assert!(!result.result.is_empty());
        assert!(result.result[0].score > 0.7);
    }

    #[test]
    async fn test_search_strategy_selection() {
        let config = SearchConfig::default();
        let engine = MultiLayerSearchEngine::new(config).unwrap();

        // Symbol-like pattern should select FastSymbolLookup
        let query = SearchQuery::new("function_name");
        let strategy = engine.select_strategy(&query);
        assert_eq!(strategy, SearchStrategy::FullTextIndex); // No symbol in index

        // Long text should select different strategy
        let query = SearchQuery::new("this is a long text search query");
        let strategy = engine.select_strategy(&query);
        assert_eq!(strategy, SearchStrategy::Hybrid);
    }

    #[test]
    async fn test_cache_functionality() {
        let config = SearchConfig {
            max_cache_size: 2,
            cache_ttl: Duration::from_millis(100),
            ..Default::default()
        };
        let engine = MultiLayerSearchEngine::new(config).unwrap();

        let query = SearchQuery::new("test");

        // First search - should miss cache
        let _result1 = engine.search(query.clone()).await.unwrap();

        // Second search - should hit cache
        let _result2 = engine.search(query.clone()).await.unwrap();

        // Wait for cache to expire
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Third search - should miss cache (expired)
        let _result3 = engine.search(query).await.unwrap();
    }

    #[tokio::test]
    async fn test_similarity_calculation() {
        let config = SearchConfig::default();
        let engine = MultiLayerSearchEngine::new(config).unwrap();

        assert_eq!(engine.calculate_similarity("hello", "hello"), 1.0);
        assert_eq!(engine.calculate_similarity("", ""), 1.0);

        let similarity = engine.calculate_similarity("hello", "helo");
        assert!(similarity > 0.8);

        let similarity = engine.calculate_similarity("test", "completely_different");
        assert!(similarity < 0.3);
    }

    #[tokio::test]
    async fn test_query_builder() {
        let query = SearchQuery::symbol("test_function")
            .with_file_filters(vec!["*.rs".to_string()])
            .with_language_filters(vec!["rust".to_string()])
            .with_context_lines(5)
            .with_limit(10)
            .fuzzy()
            .case_insensitive()
            .in_directory("/path/to/project");

        assert_eq!(query.pattern, "test_function");
        assert_eq!(query.query_type, QueryType::Symbol);
        assert_eq!(query.file_filters, vec!["*.rs"]);
        assert_eq!(query.language_filters, vec!["rust"]);
        assert_eq!(query.context_lines, 5);
        assert_eq!(query.limit, Some(10));
        assert!(query.fuzzy);
        assert!(!query.case_sensitive);
        match query.scope {
            SearchScope::Directory(path) => assert_eq!(path, PathBuf::from("/path/to/project")),
            _ => panic!("Expected Directory scope"),
        }
    }
}
