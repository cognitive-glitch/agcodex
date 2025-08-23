//! Tree-sitter primary structural tool.

use super::CodeTool;
use super::ToolError;
use super::queries::CompiledQuery;
use super::queries::QueryLibrary;
use super::queries::QueryType;
use ast::AstEngine;
use ast::CompressionLevel;
use ast::Language;
use ast::LanguageRegistry;
use ast::ParsedAst;
use dashmap::DashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tree_sitter::Query;
use tree_sitter::QueryCursor;
use tree_sitter::StreamingIterator;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct TreeSitterTool {
    engine: Arc<AstEngine>,
    registry: Arc<LanguageRegistry>,
    runtime: Arc<Runtime>,
    query_engine: Arc<QueryEngine>,
    /// New comprehensive query library
    query_library: Arc<QueryLibrary>,
}

/// Query engine for compiling and caching tree-sitter queries
#[derive(Debug)]
struct QueryEngine {
    /// Cache of compiled queries per language
    query_cache: DashMap<(Language, String), Arc<Query>>,
    _registry: Arc<LanguageRegistry>,
}

impl QueryEngine {
    fn new(registry: Arc<LanguageRegistry>) -> Self {
        Self {
            query_cache: DashMap::new(),
            _registry: registry,
        }
    }

    /// Compile a tree-sitter query pattern for a specific language
    fn compile_query(&self, language: Language, pattern: &str) -> Result<Arc<Query>, ToolError> {
        // Check cache first
        let cache_key = (language, pattern.to_string());
        if let Some(query) = self.query_cache.get(&cache_key) {
            return Ok(query.clone());
        }

        // Compile the query
        let ts_language = language.parser();
        let query = Query::new(&ts_language, pattern)
            .map_err(|e| ToolError::InvalidQuery(format!("Failed to compile query: {}", e)))?;

        let query = Arc::new(query);
        self.query_cache.insert(cache_key, query.clone());
        Ok(query)
    }

    /// Execute a query against a parsed AST
    fn execute_query(&self, query: &Query, ast: &ParsedAst, source: &[u8]) -> Vec<TsQueryMatch> {
        let mut cursor = QueryCursor::new();
        let mut results = Vec::new();

        // Iterate over matches manually
        let mut query_matches = cursor.matches(query, ast.tree.root_node(), source);
        loop {
            query_matches.advance();
            let Some(m) = query_matches.get() else {
                break;
            };
            for capture in m.captures {
                let node = capture.node;
                let text = std::str::from_utf8(&source[node.byte_range()])
                    .unwrap_or("")
                    .to_string();

                results.push(TsQueryMatch {
                    _capture_name: query.capture_names()[capture.index as usize].to_string(),
                    node_kind: node.kind().to_string(),
                    text,
                    _start_byte: node.start_byte(),
                    _end_byte: node.end_byte(),
                    start_position: (node.start_position().row, node.start_position().column),
                    end_position: (node.end_position().row, node.end_position().column),
                });
            }
        }

        results
    }
}

/// Result of a query execution
#[derive(Debug, Clone)]
struct TsQueryMatch {
    _capture_name: String,
    node_kind: String,
    text: String,
    _start_byte: usize,
    _end_byte: usize,
    start_position: (usize, usize),
    end_position: (usize, usize),
}

#[derive(Debug, Clone)]
pub struct TsQuery {
    pub language: Option<String>,
    pub pattern: String,
    pub files: Vec<PathBuf>,
    pub search_type: TsSearchType,
}

#[derive(Debug, Clone)]
pub enum TsSearchType {
    Pattern, // AST pattern matching
    Symbol,  // Symbol search
    Query,   // Tree-sitter query language
}

#[derive(Debug, Clone)]
pub struct TsMatch {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub matched_text: String,
    pub node_kind: String,
    pub context: Option<String>,
}

impl TreeSitterTool {
    pub fn new() -> Self {
        let registry = Arc::new(LanguageRegistry::new());
        let query_library = Arc::new(QueryLibrary::new());

        // Precompile common queries for better performance
        if let Err(e) = query_library.precompile_all() {
            eprintln!("Warning: Failed to precompile queries: {}", e);
        }

        Self {
            engine: Arc::new(AstEngine::new(CompressionLevel::Medium)),
            registry: registry.clone(),
            runtime: Arc::new(Runtime::new().expect("Failed to create tokio runtime")),
            query_engine: Arc::new(QueryEngine::new(registry)),
            query_library,
        }
    }

    /// Find target files based on language or pattern
    fn find_target_files(&self, query: &TsQuery) -> Result<Vec<PathBuf>, ToolError> {
        let mut files = Vec::new();

        // If specific files are provided, use them
        if !query.files.is_empty() {
            return Ok(query.files.clone());
        }

        // Otherwise, search for files in the current directory
        let current_dir = std::env::current_dir().map_err(ToolError::Io)?;

        for entry in WalkDir::new(current_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();

            // Try to detect language
            if let Ok(detected_lang) = self.registry.detect_language(path) {
                // If language filter is specified, check it
                if let Some(ref lang_filter) = query.language {
                    if detected_lang.name() == lang_filter {
                        files.push(path.to_path_buf());
                    }
                } else {
                    // No language filter, include all parseable files
                    files.push(path.to_path_buf());
                }
            }
        }

        Ok(files)
    }

    /// Extract context around a match
    fn extract_context(
        &self,
        source: &str,
        start_line: usize,
        end_line: usize,
        context_lines: usize,
    ) -> String {
        let lines: Vec<&str> = source.lines().collect();
        let total_lines = lines.len();

        let context_start = start_line.saturating_sub(context_lines);
        let context_end = (end_line + context_lines).min(total_lines - 1);

        let mut result = String::new();
        for i in context_start..=context_end {
            if i < lines.len() {
                if i == start_line {
                    result.push_str(">>> ");
                }
                result.push_str(lines[i]);
                result.push('\n');
            }
        }

        result
    }

    /// Search within a parsed tree using pattern or query
    async fn search_in_tree(
        &self,
        ast: &ParsedAst,
        file_path: &Path,
        query: &TsQuery,
    ) -> Result<Vec<TsMatch>, ToolError> {
        let source = ast.source.as_bytes();
        let mut matches = Vec::new();

        match query.search_type {
            TsSearchType::Pattern => {
                // Try structured query first, fall back to pattern conversion
                let query_type = self.infer_query_type(&query.pattern);

                let compiled_query = if let Some(qt) = query_type {
                    // Use structured query from library
                    match self.get_structured_query(ast.language, qt) {
                        Ok(structured) => structured.query.clone(),
                        Err(_) => {
                            // Fall back to pattern conversion
                            let query_pattern = self.convert_pattern_to_query(&query.pattern);
                            self.query_engine
                                .compile_query(ast.language, &query_pattern)?
                        }
                    }
                } else {
                    // Use pattern conversion
                    let query_pattern = self.convert_pattern_to_query(&query.pattern);
                    self.query_engine
                        .compile_query(ast.language, &query_pattern)?
                };

                let query_matches = self
                    .query_engine
                    .execute_query(&compiled_query, ast, source);

                for qm in query_matches {
                    matches.push(TsMatch {
                        file: file_path.display().to_string(),
                        line: qm.start_position.0 + 1,
                        column: qm.start_position.1,
                        end_line: qm.end_position.0 + 1,
                        end_column: qm.end_position.1,
                        matched_text: qm.text.clone(),
                        node_kind: qm.node_kind,
                        context: Some(self.extract_context(
                            &ast.source,
                            qm.start_position.0,
                            qm.end_position.0,
                            2,
                        )),
                    });
                }
            }
            TsSearchType::Query => {
                // Direct tree-sitter query language
                let compiled_query = self
                    .query_engine
                    .compile_query(ast.language, &query.pattern)?;

                let query_matches = self
                    .query_engine
                    .execute_query(&compiled_query, ast, source);

                for qm in query_matches {
                    matches.push(TsMatch {
                        file: file_path.display().to_string(),
                        line: qm.start_position.0 + 1,
                        column: qm.start_position.1,
                        end_line: qm.end_position.0 + 1,
                        end_column: qm.end_position.1,
                        matched_text: qm.text.clone(),
                        node_kind: qm.node_kind,
                        context: Some(self.extract_context(
                            &ast.source,
                            qm.start_position.0,
                            qm.end_position.0,
                            2,
                        )),
                    });
                }
            }
            TsSearchType::Symbol => {
                // Use existing symbol search
                let symbols = self
                    .engine
                    .search_symbols(&query.pattern)
                    .await
                    .map_err(|e| ToolError::InvalidQuery(format!("Symbol search error: {}", e)))?;

                for s in symbols {
                    if PathBuf::from(&s.location.file_path) == file_path {
                        matches.push(TsMatch {
                            file: file_path.display().to_string(),
                            line: s.location.start_line,
                            column: s.location.start_column,
                            end_line: s.location.end_line,
                            end_column: s.location.end_column,
                            matched_text: s.name.clone(),
                            node_kind: format!("{:?}", s.kind),
                            context: Some(s.signature),
                        });
                    }
                }
            }
        }

        Ok(matches)
    }

    /// Infer query type from pattern string
    fn infer_query_type(&self, pattern: &str) -> Option<QueryType> {
        if pattern.starts_with("function ") || pattern.contains("function") {
            Some(QueryType::Functions)
        } else if pattern.starts_with("class ") || pattern.contains("class") {
            Some(QueryType::Classes)
        } else if pattern.starts_with("import ") || pattern.contains("import") {
            Some(QueryType::Imports)
        } else if pattern.starts_with("method ") || pattern.contains("method") {
            Some(QueryType::Methods)
        } else {
            None
        }
    }

    /// Get a structured query using the new query library
    fn get_structured_query(
        &self,
        language: ast::Language,
        query_type: QueryType,
    ) -> Result<Arc<CompiledQuery>, ToolError> {
        self.query_library
            .get_query(language, query_type)
            .map_err(|e| ToolError::InvalidQuery(format!("Query library error: {}", e)))
    }

    /// Convert a simple pattern to tree-sitter query syntax (legacy support)
    fn convert_pattern_to_query(&self, pattern: &str) -> String {
        // This is a simplified conversion - in practice, you'd want more sophisticated parsing
        // For now, we'll handle common patterns

        if pattern.starts_with("function ") {
            let func_name = pattern.trim_start_matches("function ").trim();
            if func_name == "*" {
                // Match all functions
                "[
                    (function_declaration) @func
                    (function_definition) @func
                    (method_declaration) @func
                    (method_definition) @func
                ]"
                .to_string()
            } else {
                // Match specific function name
                format!(
                    "[
                        (function_declaration name: (identifier) @name (#eq? @name \"{}\"))
                        (function_definition name: (identifier) @name (#eq? @name \"{}\"))
                        (method_declaration name: (identifier) @name (#eq? @name \"{}\"))
                        (method_definition name: (identifier) @name (#eq? @name \"{}\"))
                    ] @func",
                    func_name, func_name, func_name, func_name
                )
            }
        } else if pattern.starts_with("class ") {
            let class_name = pattern.trim_start_matches("class ").trim();
            if class_name == "*" {
                "[
                    (class_declaration) @class
                    (class_definition) @class
                ] @class"
                    .to_string()
            } else {
                format!(
                    "[
                        (class_declaration name: (identifier) @name (#eq? @name \"{}\"))
                        (class_definition name: (identifier) @name (#eq? @name \"{}\"))
                    ] @class",
                    class_name, class_name
                )
            }
        } else if pattern.starts_with("import ") {
            "[
                (import_statement) @import
                (import_declaration) @import
                (use_declaration) @import
            ] @import"
                .to_string()
        } else {
            // Default: try to match as identifier
            format!("(identifier) @id (#eq? @id \"{}\")", pattern)
        }
    }

    /// Execute a structured query using the query library
    pub async fn search_structured(
        &self,
        language: ast::Language,
        query_type: QueryType,
        files: Vec<PathBuf>,
    ) -> Result<Vec<TsMatch>, ToolError> {
        let compiled_query = self.get_structured_query(language, query_type)?;
        let mut all_matches = Vec::new();

        for file_path in &files {
            // Parse the file using AstEngine
            let ast = self
                .engine
                .parse_file(file_path)
                .await
                .map_err(|e| ToolError::InvalidQuery(format!("Parse error: {}", e)))?;

            // Skip if language doesn't match
            if ast.language != language {
                continue;
            }

            let source = ast.source.as_bytes();
            let query_matches =
                self.query_engine
                    .execute_query(&compiled_query.query, &ast, source);

            for qm in query_matches {
                all_matches.push(TsMatch {
                    file: file_path.display().to_string(),
                    line: qm.start_position.0 + 1,
                    column: qm.start_position.1,
                    end_line: qm.end_position.0 + 1,
                    end_column: qm.end_position.1,
                    matched_text: qm.text.clone(),
                    node_kind: qm.node_kind,
                    context: Some(self.extract_context(
                        &ast.source,
                        qm.start_position.0,
                        qm.end_position.0,
                        2,
                    )),
                });
            }
        }

        Ok(all_matches)
    }

    /// Get query library statistics
    pub fn query_stats(&self) -> crate::code_tools::queries::QueryLibraryStats {
        self.query_library.stats()
    }

    /// Check if a language supports a specific query type
    pub fn supports_query(&self, language: ast::Language, query_type: &QueryType) -> bool {
        self.query_library.supports_query(language, query_type)
    }

    async fn search_async(&self, mut query: TsQuery) -> Result<Vec<TsMatch>, ToolError> {
        // Find target files if not specified
        if query.files.is_empty() {
            query.files = self.find_target_files(&query)?;
        }

        let mut all_matches = Vec::new();

        for file_path in &query.files {
            // Parse the file using AstEngine
            let ast = self
                .engine
                .parse_file(file_path)
                .await
                .map_err(|e| ToolError::InvalidQuery(format!("Parse error: {}", e)))?;

            // Execute search within the tree
            let matches = self.search_in_tree(&ast, file_path, &query).await?;
            all_matches.extend(matches);
        }

        Ok(all_matches)
    }
}

impl CodeTool for TreeSitterTool {
    type Query = TsQuery;
    type Output = Vec<TsMatch>;

    fn search(&self, query: Self::Query) -> Result<Self::Output, ToolError> {
        self.runtime.block_on(self.search_async(query))
    }
}

impl Default for TreeSitterTool {
    fn default() -> Self {
        Self::new()
    }
}
