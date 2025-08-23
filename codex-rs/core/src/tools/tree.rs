//! Tree-sitter AST Tool for AGCodex
//!
//! This module provides comprehensive AST parsing and querying capabilities using tree-sitter
//! with support for multiple programming languages. It includes:
//! - Multi-language parser support with automatic language detection
//! - Query-based AST pattern matching
//! - Semantic code diffing
//! - Symbol extraction and analysis
//! - Efficient caching for performance
//!
//! ## Supported Languages
//! - Rust, Python, JavaScript, TypeScript
//! - Go, Java, C, C++, Bash
//! - More languages can be added via the parser registry

use super::InternalTool;
use super::ToolMetadata;
use super::output::ComprehensiveToolOutput as ToolOutput;
use crate::subagents::IntelligenceLevel;
use ast::SourceLocation;

use dashmap::DashMap;
use lru::LruCache;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;
use thiserror::Error;
use tokio::fs;
use tracing::debug;
use tracing::info;
// use tracing::warn; // unused
use tree_sitter::Language;
use tree_sitter::Node;
use tree_sitter::Parser;
use tree_sitter::Query;
use tree_sitter::QueryCursor;
use tree_sitter::StreamingIterator;
use tree_sitter::Tree;

// Import all 27 tree-sitter language parsers
extern crate tree_sitter_bash;
extern crate tree_sitter_c;
extern crate tree_sitter_c_sharp;
extern crate tree_sitter_clojure;
extern crate tree_sitter_cpp;
extern crate tree_sitter_css;
extern crate tree_sitter_dockerfile;
extern crate tree_sitter_elixir;
extern crate tree_sitter_go;
extern crate tree_sitter_haskell;
extern crate tree_sitter_hcl;
extern crate tree_sitter_html;
extern crate tree_sitter_java;
extern crate tree_sitter_javascript;
extern crate tree_sitter_json;
extern crate tree_sitter_kotlin_ng;
// extern crate tree_sitter_latex; // Disabled due to linking issues
extern crate tree_sitter_lua;
extern crate tree_sitter_make;
extern crate tree_sitter_markdown;
extern crate tree_sitter_nix;
extern crate tree_sitter_objc;
extern crate tree_sitter_ocaml;
extern crate tree_sitter_php;
extern crate tree_sitter_python;
extern crate tree_sitter_rst;
extern crate tree_sitter_ruby;
extern crate tree_sitter_rust;
extern crate tree_sitter_scala;
extern crate tree_sitter_swift;
extern crate tree_sitter_toml;
extern crate tree_sitter_typescript;
extern crate tree_sitter_yaml;
extern crate tree_sitter_zig;

/// Errors specific to the tree tool
#[derive(Error, Debug)]
pub enum TreeError {
    #[error("unsupported language: {language}")]
    UnsupportedLanguage { language: String },

    #[error("failed to parse code: {reason}")]
    ParseFailed { reason: String },

    #[error("query compilation failed: {query} - {reason}")]
    QueryCompilationFailed { query: String, reason: String },

    #[error("query execution failed: {reason}")]
    QueryExecutionFailed { reason: String },

    #[error("file reading failed: {path} - {reason}")]
    FileReadFailed { path: PathBuf, reason: String },

    #[error("language detection failed for file: {path}")]
    LanguageDetectionFailed { path: PathBuf },

    #[error("AST node access failed: {reason}")]
    NodeAccessFailed { reason: String },

    #[error("diff computation failed: {reason}")]
    DiffFailed { reason: String },

    #[error("symbol extraction failed: {reason}")]
    SymbolExtractionFailed { reason: String },

    #[error("cache operation failed: {reason}")]
    CacheFailed { reason: String },
}

/// Result type for tree operations
pub type TreeResult<T> = std::result::Result<T, TreeError>;

/// Supported programming languages (27 total)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SupportedLanguage {
    // Core systems languages
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    C,
    Cpp,
    CSharp,
    Bash,

    // Web languages
    Html,
    Css,
    Json,
    Yaml,
    Toml,

    // Scripting languages
    Ruby,
    Php,
    Lua,

    // Functional languages
    Haskell,
    Elixir,
    Scala,
    Ocaml,
    Clojure,

    // Systems languages
    Zig,
    Swift,
    Kotlin,
    ObjectiveC,

    // Config/Build languages
    Dockerfile,
    Hcl,
    Nix,
    Make,

    // Documentation languages
    Markdown,
    // Latex, // Disabled due to linking issues
    Rst,
}

impl SupportedLanguage {
    /// Get file extensions for this language
    pub const fn extensions(&self) -> &'static [&'static str] {
        match self {
            // Core systems languages
            SupportedLanguage::Rust => &["rs"],
            SupportedLanguage::Python => &["py", "pyw", "pyi"],
            SupportedLanguage::JavaScript => &["js", "mjs", "cjs"],
            SupportedLanguage::TypeScript => &["ts", "tsx", "cts", "mts"],
            SupportedLanguage::Go => &["go"],
            SupportedLanguage::Java => &["java"],
            SupportedLanguage::C => &["c", "h"],
            SupportedLanguage::Cpp => &["cpp", "cxx", "cc", "hpp", "hxx", "hh"],
            SupportedLanguage::CSharp => &["cs"],
            SupportedLanguage::Bash => &["sh", "bash", "zsh"],

            // Web languages
            SupportedLanguage::Html => &["html", "htm"],
            SupportedLanguage::Css => &["css"],
            SupportedLanguage::Json => &["json"],
            SupportedLanguage::Yaml => &["yaml", "yml"],
            SupportedLanguage::Toml => &["toml"],

            // Scripting languages
            SupportedLanguage::Ruby => &["rb"],
            SupportedLanguage::Php => &["php"],
            SupportedLanguage::Lua => &["lua"],

            // Functional languages
            SupportedLanguage::Haskell => &["hs"],
            SupportedLanguage::Elixir => &["ex", "exs"],
            SupportedLanguage::Scala => &["scala", "sc"],
            SupportedLanguage::Ocaml => &["ml", "mli"],
            SupportedLanguage::Clojure => &["clj", "cljs", "cljc"],

            // Systems languages
            SupportedLanguage::Zig => &["zig"],
            SupportedLanguage::Swift => &["swift"],
            SupportedLanguage::Kotlin => &["kt", "kts"],
            SupportedLanguage::ObjectiveC => &["m", "mm"],

            // Config/Build languages
            SupportedLanguage::Dockerfile => &["dockerfile"],
            SupportedLanguage::Hcl => &["hcl", "tf"],
            SupportedLanguage::Nix => &["nix"],
            SupportedLanguage::Make => &["makefile", "mk"],

            // Documentation languages
            SupportedLanguage::Markdown => &["md", "markdown"],
            // SupportedLanguage::Latex => &["tex", "latex"], // Disabled
            SupportedLanguage::Rst => &["rst"],
        }
    }

    /// Get the tree-sitter language grammar
    pub fn grammar(&self) -> Language {
        match self {
            // Core systems languages
            SupportedLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
            SupportedLanguage::Python => tree_sitter_python::LANGUAGE.into(),
            SupportedLanguage::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            SupportedLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            SupportedLanguage::Go => tree_sitter_go::LANGUAGE.into(),
            SupportedLanguage::Java => tree_sitter_java::LANGUAGE.into(),
            SupportedLanguage::C => tree_sitter_c::LANGUAGE.into(),
            SupportedLanguage::Cpp => tree_sitter_cpp::LANGUAGE.into(),
            SupportedLanguage::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
            SupportedLanguage::Bash => tree_sitter_bash::LANGUAGE.into(),

            // Web languages
            SupportedLanguage::Html => tree_sitter_html::LANGUAGE.into(),
            SupportedLanguage::Css => tree_sitter_css::LANGUAGE.into(),
            SupportedLanguage::Json => tree_sitter_json::LANGUAGE.into(),
            SupportedLanguage::Yaml => tree_sitter_yaml::LANGUAGE.into(),
            // TODO: Fix version compatibility issue with tree_sitter_toml
            SupportedLanguage::Toml => todo!("TOML parser has version compatibility issues"),

            // Scripting languages
            SupportedLanguage::Ruby => tree_sitter_ruby::LANGUAGE.into(),
            // TODO: Fix API compatibility for PHP
            SupportedLanguage::Php => todo!("PHP tree-sitter API compatibility"),
            SupportedLanguage::Lua => tree_sitter_lua::LANGUAGE.into(),

            // Functional languages
            SupportedLanguage::Haskell => tree_sitter_haskell::LANGUAGE.into(),
            SupportedLanguage::Elixir => tree_sitter_elixir::LANGUAGE.into(),
            SupportedLanguage::Scala => tree_sitter_scala::LANGUAGE.into(),
            // TODO: Fix API compatibility for OCaml
            SupportedLanguage::Ocaml => todo!("OCaml tree-sitter API compatibility"),
            SupportedLanguage::Clojure => tree_sitter_clojure::LANGUAGE.into(),

            // Systems languages
            SupportedLanguage::Zig => tree_sitter_zig::LANGUAGE.into(),
            SupportedLanguage::Swift => tree_sitter_swift::LANGUAGE.into(),
            SupportedLanguage::Kotlin => tree_sitter_kotlin_ng::LANGUAGE.into(),
            SupportedLanguage::ObjectiveC => tree_sitter_objc::LANGUAGE.into(),

            // Config/Build languages
            // TODO: Fix version compatibility issue with tree_sitter_dockerfile
            SupportedLanguage::Dockerfile => {
                todo!("Dockerfile parser has version compatibility issues")
            }
            SupportedLanguage::Hcl => tree_sitter_hcl::LANGUAGE.into(),
            SupportedLanguage::Nix => tree_sitter_nix::LANGUAGE.into(),
            SupportedLanguage::Make => tree_sitter_make::LANGUAGE.into(),

            // Documentation languages
            // TODO: Fix version compatibility issue with tree_sitter_markdown
            SupportedLanguage::Markdown => {
                todo!("Markdown parser has version compatibility issues")
            }
            // SupportedLanguage::Latex => tree_sitter_latex::LANGUAGE.into(), // Disabled
            SupportedLanguage::Rst => tree_sitter_rst::LANGUAGE.into(),
        }
    }

    /// Detect language from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        let ext = ext.to_lowercase();

        // Check all 27 supported languages
        Self::all_languages()
            .iter()
            .find(|&&lang| lang.extensions().contains(&ext.as_str()))
            .copied()
    }

    /// Get all supported languages (27 total)
    pub const fn all_languages() -> &'static [Self] {
        &[
            // Core systems languages
            SupportedLanguage::Rust,
            SupportedLanguage::Python,
            SupportedLanguage::JavaScript,
            SupportedLanguage::TypeScript,
            SupportedLanguage::Go,
            SupportedLanguage::Java,
            SupportedLanguage::C,
            SupportedLanguage::Cpp,
            SupportedLanguage::CSharp,
            SupportedLanguage::Bash,
            // Web languages
            SupportedLanguage::Html,
            SupportedLanguage::Css,
            SupportedLanguage::Json,
            SupportedLanguage::Yaml,
            SupportedLanguage::Toml,
            // Scripting languages
            SupportedLanguage::Ruby,
            SupportedLanguage::Php,
            SupportedLanguage::Lua,
            // Functional languages
            SupportedLanguage::Haskell,
            SupportedLanguage::Elixir,
            SupportedLanguage::Scala,
            SupportedLanguage::Ocaml,
            SupportedLanguage::Clojure,
            // Systems languages
            SupportedLanguage::Zig,
            SupportedLanguage::Swift,
            SupportedLanguage::Kotlin,
            SupportedLanguage::ObjectiveC,
            // Config/Build languages
            SupportedLanguage::Dockerfile,
            SupportedLanguage::Hcl,
            SupportedLanguage::Nix,
            SupportedLanguage::Make,
            // Documentation languages
            SupportedLanguage::Markdown,
            // SupportedLanguage::Latex, // Disabled
            SupportedLanguage::Rst,
        ]
    }

    /// Get name as string
    pub const fn as_str(&self) -> &'static str {
        match self {
            // Core systems languages
            SupportedLanguage::Rust => "rust",
            SupportedLanguage::Python => "python",
            SupportedLanguage::JavaScript => "javascript",
            SupportedLanguage::TypeScript => "typescript",
            SupportedLanguage::Go => "go",
            SupportedLanguage::Java => "java",
            SupportedLanguage::C => "c",
            SupportedLanguage::Cpp => "cpp",
            SupportedLanguage::CSharp => "csharp",
            SupportedLanguage::Bash => "bash",

            // Web languages
            SupportedLanguage::Html => "html",
            SupportedLanguage::Css => "css",
            SupportedLanguage::Json => "json",
            SupportedLanguage::Yaml => "yaml",
            SupportedLanguage::Toml => "toml",

            // Scripting languages
            SupportedLanguage::Ruby => "ruby",
            SupportedLanguage::Php => "php",
            SupportedLanguage::Lua => "lua",

            // Functional languages
            SupportedLanguage::Haskell => "haskell",
            SupportedLanguage::Elixir => "elixir",
            SupportedLanguage::Scala => "scala",
            SupportedLanguage::Ocaml => "ocaml",
            SupportedLanguage::Clojure => "clojure",

            // Systems languages
            SupportedLanguage::Zig => "zig",
            SupportedLanguage::Swift => "swift",
            SupportedLanguage::Kotlin => "kotlin",
            SupportedLanguage::ObjectiveC => "objc",

            // Config/Build languages
            SupportedLanguage::Dockerfile => "dockerfile",
            SupportedLanguage::Hcl => "hcl",
            SupportedLanguage::Nix => "nix",
            SupportedLanguage::Make => "make",

            // Documentation languages
            SupportedLanguage::Markdown => "markdown",
            // SupportedLanguage::Latex => "latex", // Disabled
            SupportedLanguage::Rst => "rst",
        }
    }
}

/// Parsed AST with metadata
#[derive(Debug)]
pub struct ParsedAst {
    pub tree: Tree,
    pub language: SupportedLanguage,
    pub source_code: String,
    pub parse_time: Duration,
    pub node_count: usize,
}

impl ParsedAst {
    /// Get the root node
    pub fn root_node(&self) -> Node {
        self.tree.root_node()
    }

    /// Check if there are any parse errors
    pub fn has_errors(&self) -> bool {
        self.root_node().has_error()
    }

    /// Get all error nodes
    pub fn error_nodes(&self) -> Vec<Node> {
        let mut errors = Vec::new();
        self.collect_error_nodes(self.root_node(), &mut errors);
        errors
    }

    fn collect_error_nodes<'a>(&self, node: Node<'a>, errors: &mut Vec<Node<'a>>) {
        if node.is_error() || node.is_missing() {
            errors.push(node);
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.collect_error_nodes(child, errors);
            }
        }
    }
}

/// Query result containing matched nodes and captures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMatch {
    pub pattern_index: u32,
    pub captures: Vec<QueryCapture>,
    pub start_byte: u32,
    pub end_byte: u32,
    pub start_point: Point,
    pub end_point: Point,
}

/// A capture from a query match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCapture {
    pub name: String,
    pub text: String,
    pub start_byte: u32,
    pub end_byte: u32,
    pub start_point: Point,
    pub end_point: Point,
}

/// Point in source code (row, column)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Point {
    pub row: u32,
    pub column: u32,
}

impl From<tree_sitter::Point> for Point {
    fn from(point: tree_sitter::Point) -> Self {
        Self {
            row: point.row as u32,
            column: point.column as u32,
        }
    }
}

/// Symbol information extracted from AST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub start_point: Point,
    pub end_point: Point,
    pub start_byte: u32,
    pub end_byte: u32,
    pub parent: Option<String>,
    pub visibility: Option<String>,
    pub type_signature: Option<String>,
}

/// Types of symbols that can be extracted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Interface,
    Struct,
    Enum,
    Variable,
    Constant,
    Field,
    Parameter,
    Module,
    Namespace,
    Type,
    Trait,
    Impl,
    Macro,
    Other(String),
}

/// Semantic differences between two ASTs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticDiff {
    pub added: Vec<DiffNode>,
    pub removed: Vec<DiffNode>,
    pub modified: Vec<ModifiedNode>,
    pub moved: Vec<MovedNode>,
    pub similarity_score: f64,
}

/// A node in a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffNode {
    pub kind: String,
    pub text: String,
    pub start_point: Point,
    pub end_point: Point,
}

/// A modified node in a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifiedNode {
    pub old: DiffNode,
    pub new: DiffNode,
    pub similarity: f64,
}

/// A moved node in a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovedNode {
    pub node: DiffNode,
    pub old_position: Point,
    pub new_position: Point,
}

/// Input for tree operations
#[derive(Debug, Clone)]
pub enum TreeInput {
    /// Parse source code
    Parse {
        code: String,
        language: Option<SupportedLanguage>,
        file_path: Option<PathBuf>,
    },
    /// Parse from file
    ParseFile { file_path: PathBuf },
    /// Query AST with pattern
    Query {
        code: String,
        language: Option<SupportedLanguage>,
        pattern: String,
        file_path: Option<PathBuf>,
    },
    /// Query file with pattern
    QueryFile { file_path: PathBuf, pattern: String },
    /// Extract symbols from code
    ExtractSymbols {
        code: String,
        language: Option<SupportedLanguage>,
        file_path: Option<PathBuf>,
    },
    /// Extract symbols from file
    ExtractSymbolsFromFile { file_path: PathBuf },
    /// Compare two pieces of code
    Diff {
        old_code: String,
        new_code: String,
        language: Option<SupportedLanguage>,
    },
    /// Compare two files
    DiffFiles {
        old_file: PathBuf,
        new_file: PathBuf,
    },
}

/// Output from tree operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TreeOutput {
    /// Parsed AST information
    Parsed {
        language: SupportedLanguage,
        node_count: usize,
        has_errors: bool,
        error_count: usize,
        parse_time_ms: u64,
    },
    /// Query results
    QueryResults {
        matches: Vec<QueryMatch>,
        total_matches: usize,
        query_time_ms: u64,
    },
    /// Extracted symbols
    Symbols {
        symbols: Vec<Symbol>,
        total_symbols: usize,
        extraction_time_ms: u64,
    },
    /// Semantic diff
    Diff {
        diff: SemanticDiff,
        diff_time_ms: u64,
    },
}

/// Cache key for parsed ASTs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    content_hash: u64,
    language: SupportedLanguage,
}

/// Thread-safe wrapper for Parser
struct ThreadSafeParser {
    parser: Mutex<Parser>,
    _language: SupportedLanguage,
}

impl ThreadSafeParser {
    fn new(language: SupportedLanguage) -> TreeResult<Self> {
        let mut parser = Parser::new();
        parser
            .set_language(&language.grammar())
            .map_err(|_| TreeError::ParseFailed {
                reason: format!("Failed to initialize {} parser", language.as_str()),
            })?;

        Ok(Self {
            parser: Mutex::new(parser),
            _language: language,
        })
    }

    fn parse(&self, code: &str) -> TreeResult<Tree> {
        let mut parser = self.parser.lock().map_err(|e| TreeError::ParseFailed {
            reason: format!("Parser lock failed: {}", e),
        })?;

        parser
            .parse(code, None)
            .ok_or_else(|| TreeError::ParseFailed {
                reason: "Parser returned None".to_string(),
            })
    }
}

/// Semantic diff engine for computing AST differences
struct SemanticDiffEngine;

impl SemanticDiffEngine {
    /// Compute semantic differences between two ASTs
    fn compute_diff(old_ast: &ParsedAst, new_ast: &ParsedAst) -> TreeResult<SemanticDiff> {
        if old_ast.language != new_ast.language {
            return Err(TreeError::DiffFailed {
                reason: format!(
                    "Language mismatch: {} vs {}",
                    old_ast.language.as_str(),
                    new_ast.language.as_str()
                ),
            });
        }

        let old_root = old_ast.root_node();
        let new_root = new_ast.root_node();

        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut modified = Vec::new();
        let mut moved = Vec::new();

        // Collect all nodes from both trees
        let old_nodes = Self::collect_nodes(old_root, &old_ast.source_code);
        let new_nodes = Self::collect_nodes(new_root, &new_ast.source_code);

        // Create maps for efficient lookup
        let old_map: HashMap<String, Vec<DiffNode>> = Self::group_by_kind(&old_nodes);
        let new_map: HashMap<String, Vec<DiffNode>> = Self::group_by_kind(&new_nodes);

        // Find added, removed, and modified nodes
        for (kind, new_nodes_of_kind) in &new_map {
            if let Some(old_nodes_of_kind) = old_map.get(kind) {
                // Compare nodes of the same kind
                let (mod_pairs, add_nodes) =
                    Self::match_nodes(old_nodes_of_kind, new_nodes_of_kind);
                modified.extend(mod_pairs);
                added.extend(add_nodes);
            } else {
                // All nodes of this kind are added
                added.extend(new_nodes_of_kind.clone());
            }
        }

        for (kind, old_nodes_of_kind) in &old_map {
            if !new_map.contains_key(kind) {
                // All nodes of this kind are removed
                removed.extend(old_nodes_of_kind.clone());
            }
        }

        // Detect moved nodes (same content, different position)
        moved.extend(Self::detect_moved_nodes(&old_nodes, &new_nodes));

        // Calculate similarity score
        let similarity_score = Self::calculate_similarity(&old_nodes, &new_nodes);

        Ok(SemanticDiff {
            added,
            removed,
            modified,
            moved,
            similarity_score,
        })
    }

    fn collect_nodes(node: Node, source: &str) -> Vec<DiffNode> {
        let mut nodes = Vec::new();
        Self::collect_nodes_recursive(node, source, &mut nodes);
        nodes
    }

    fn collect_nodes_recursive(node: Node, source: &str, nodes: &mut Vec<DiffNode>) {
        let text = node
            .utf8_text(source.as_bytes())
            .unwrap_or("<invalid utf8>");

        nodes.push(DiffNode {
            kind: node.kind().to_string(),
            text: text.to_string(),
            start_point: node.start_position().into(),
            end_point: node.end_position().into(),
        });

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                Self::collect_nodes_recursive(child, source, nodes);
            }
        }
    }

    fn group_by_kind(nodes: &[DiffNode]) -> HashMap<String, Vec<DiffNode>> {
        let mut map = HashMap::new();
        for node in nodes {
            map.entry(node.kind.clone())
                .or_insert_with(Vec::new)
                .push(node.clone());
        }
        map
    }

    fn match_nodes(
        old_nodes: &[DiffNode],
        new_nodes: &[DiffNode],
    ) -> (Vec<ModifiedNode>, Vec<DiffNode>) {
        let mut modified = Vec::new();
        let mut added = Vec::new();
        let mut used_old = vec![false; old_nodes.len()];

        for new_node in new_nodes {
            let mut best_match = None;
            let mut best_similarity = 0.0;

            for (i, old_node) in old_nodes.iter().enumerate() {
                if used_old[i] {
                    continue;
                }

                let similarity = Self::text_similarity(&old_node.text, &new_node.text);
                if similarity > best_similarity && similarity > 0.3 {
                    best_similarity = similarity;
                    best_match = Some(i);
                }
            }

            if let Some(idx) = best_match {
                used_old[idx] = true;
                if best_similarity < 0.9 {
                    modified.push(ModifiedNode {
                        old: old_nodes[idx].clone(),
                        new: new_node.clone(),
                        similarity: best_similarity,
                    });
                }
            } else {
                added.push(new_node.clone());
            }
        }

        (modified, added)
    }

    fn detect_moved_nodes(old_nodes: &[DiffNode], new_nodes: &[DiffNode]) -> Vec<MovedNode> {
        let mut moved = Vec::new();

        for old_node in old_nodes {
            for new_node in new_nodes {
                if old_node.kind == new_node.kind
                    && old_node.text == new_node.text
                    && (old_node.start_point.row != new_node.start_point.row
                        || old_node.start_point.column != new_node.start_point.column)
                {
                    moved.push(MovedNode {
                        node: new_node.clone(),
                        old_position: old_node.start_point,
                        new_position: new_node.start_point,
                    });
                }
            }
        }

        moved
    }

    fn text_similarity(text1: &str, text2: &str) -> f64 {
        if text1 == text2 {
            return 1.0;
        }

        let len1 = text1.len() as f64;
        let len2 = text2.len() as f64;
        let max_len = len1.max(len2);

        if max_len == 0.0 {
            return 1.0;
        }

        // Simple edit distance approximation
        let common_chars = text1.chars().filter(|c| text2.contains(*c)).count() as f64;

        common_chars / max_len
    }

    fn calculate_similarity(old_nodes: &[DiffNode], new_nodes: &[DiffNode]) -> f64 {
        if old_nodes.is_empty() && new_nodes.is_empty() {
            return 1.0;
        }

        let total_nodes = (old_nodes.len() + new_nodes.len()) as f64;
        let common_nodes = old_nodes
            .iter()
            .filter(|old| new_nodes.iter().any(|new| old.text == new.text))
            .count() as f64;

        (common_nodes * 2.0) / total_nodes
    }
}

/// Query library for common AST patterns
struct QueryLibrary {
    _queries: HashMap<String, HashMap<SupportedLanguage, String>>,
}

impl QueryLibrary {
    fn new() -> Self {
        let mut queries = HashMap::new();

        // Function definitions
        let mut function_queries = HashMap::new();
        function_queries.insert(
            SupportedLanguage::Rust,
            "(function_item name: (identifier) @name) @function".to_string(),
        );
        function_queries.insert(
            SupportedLanguage::Python,
            "(function_definition name: (identifier) @name) @function".to_string(),
        );
        function_queries.insert(
            SupportedLanguage::JavaScript,
            "(function_declaration name: (identifier) @name) @function".to_string(),
        );
        function_queries.insert(
            SupportedLanguage::TypeScript,
            "(function_declaration name: (identifier) @name) @function".to_string(),
        );
        function_queries.insert(
            SupportedLanguage::Go,
            "(function_declaration name: (identifier) @name) @function".to_string(),
        );
        function_queries.insert(
            SupportedLanguage::Java,
            "(method_declaration name: (identifier) @name) @function".to_string(),
        );
        queries.insert("functions".to_string(), function_queries);

        // Class definitions
        let mut class_queries = HashMap::new();
        class_queries.insert(
            SupportedLanguage::Rust,
            "(struct_item name: (type_identifier) @name) @class".to_string(),
        );
        class_queries.insert(
            SupportedLanguage::Python,
            "(class_definition name: (identifier) @name) @class".to_string(),
        );
        class_queries.insert(
            SupportedLanguage::JavaScript,
            "(class_declaration name: (identifier) @name) @class".to_string(),
        );
        class_queries.insert(
            SupportedLanguage::TypeScript,
            "(class_declaration name: (identifier) @name) @class".to_string(),
        );
        class_queries.insert(
            SupportedLanguage::Java,
            "(class_declaration name: (identifier) @name) @class".to_string(),
        );
        queries.insert("classes".to_string(), class_queries);

        // Variable definitions
        let mut variable_queries = HashMap::new();
        variable_queries.insert(
            SupportedLanguage::Rust,
            "(let_declaration pattern: (identifier) @name) @variable".to_string(),
        );
        variable_queries.insert(
            SupportedLanguage::Python,
            "(assignment left: (identifier) @name) @variable".to_string(),
        );
        variable_queries.insert(
            SupportedLanguage::JavaScript,
            "(variable_declaration (variable_declarator name: (identifier) @name)) @variable"
                .to_string(),
        );
        queries.insert("variables".to_string(), variable_queries);

        Self { _queries: queries }
    }

    fn _get_query(&self, pattern_name: &str, language: SupportedLanguage) -> Option<&String> {
        self._queries.get(pattern_name)?.get(&language)
    }
}

/// Enhanced tree-sitter tool with comprehensive language support
pub struct TreeTool {
    /// Thread-safe parsers for all supported languages
    parsers: Arc<DashMap<SupportedLanguage, ThreadSafeParser>>,
    /// AST cache with timed eviction
    ast_cache: Arc<Mutex<LruCache<CacheKey, (Arc<ParsedAst>, Instant)>>>,
    /// Semantic diff engine
    _diff_engine: SemanticDiffEngine,
    /// Query library for common patterns
    _query_library: QueryLibrary,
    /// Performance configuration
    intelligence_level: IntelligenceLevel,
    /// Maximum cache size
    max_cache_size: NonZeroUsize,
    /// Cache TTL for ASTs
    cache_ttl: Duration,
}

impl TreeTool {
    /// Create a new enhanced tree tool instance
    pub fn new(intelligence_level: IntelligenceLevel) -> TreeResult<Self> {
        let (cache_size, cache_ttl) = match intelligence_level {
            IntelligenceLevel::Light => (NonZeroUsize::new(100).unwrap(), Duration::from_secs(300)), // 5 minutes
            IntelligenceLevel::Medium => {
                (NonZeroUsize::new(500).unwrap(), Duration::from_secs(900))
            } // 15 minutes
            IntelligenceLevel::Hard => {
                (NonZeroUsize::new(2000).unwrap(), Duration::from_secs(1800))
            } // 30 minutes
        };

        let mut tool = Self {
            parsers: Arc::new(DashMap::new()),
            ast_cache: Arc::new(Mutex::new(LruCache::new(cache_size))),
            _diff_engine: SemanticDiffEngine,
            _query_library: QueryLibrary::new(),
            intelligence_level,
            max_cache_size: cache_size,
            cache_ttl,
        };

        // Initialize parsers for all 27 supported languages
        tool.initialize_parsers()?;

        Ok(tool)
    }

    /// Initialize parsers for all 27 supported languages
    fn initialize_parsers(&mut self) -> TreeResult<()> {
        let languages = SupportedLanguage::all_languages();

        for &language in languages {
            let parser = ThreadSafeParser::new(language)?;
            self.parsers.insert(language, parser);
            debug!("Initialized parser for {}", language.as_str());
        }

        info!("Initialized {} language parsers", languages.len());
        Ok(())
    }

    /// Parse source code into AST with enhanced caching
    pub async fn parse(
        &self,
        code: String,
        language: Option<SupportedLanguage>,
        file_path: Option<PathBuf>,
    ) -> TreeResult<Arc<ParsedAst>> {
        let start_time = Instant::now();

        // Detect language if not provided
        let detected_language = match language {
            Some(lang) => lang,
            None => {
                if let Some(ref path) = file_path {
                    self.detect_language(path)?
                } else {
                    return Err(TreeError::LanguageDetectionFailed {
                        path: file_path.unwrap_or_else(|| PathBuf::from("unknown")),
                    });
                }
            }
        };

        // Generate cache key
        let content_hash = self.hash_content(&code);
        let cache_key = CacheKey {
            content_hash,
            language: detected_language,
        };

        // Check cache first with TTL
        {
            let mut cache = self.ast_cache.lock().map_err(|e| TreeError::CacheFailed {
                reason: format!("Cache lock failed: {}", e),
            })?;

            if let Some((cached_ast, cached_time)) = cache.get(&cache_key) {
                // Check if cache entry is still valid
                if cached_time.elapsed() < self.cache_ttl {
                    debug!(
                        "Cache hit for {} code ({} bytes)",
                        detected_language.as_str(),
                        code.len()
                    );
                    return Ok(cached_ast.clone());
                } else {
                    // Remove expired entry
                    cache.pop(&cache_key);
                }
            }
        }

        // Get parser for the detected language
        let parser_ref =
            self.parsers
                .get(&detected_language)
                .ok_or_else(|| TreeError::UnsupportedLanguage {
                    language: detected_language.as_str().to_string(),
                })?;

        // Parse the code using thread-safe parser
        let tree = parser_ref.parse(&code)?;

        let parse_time = start_time.elapsed();
        let node_count = self.count_nodes(tree.root_node());

        let parsed_ast = Arc::new(ParsedAst {
            tree,
            language: detected_language,
            source_code: code,
            parse_time,
            node_count,
        });

        // Cache the result with timestamp
        {
            let mut cache = self.ast_cache.lock().map_err(|e| TreeError::CacheFailed {
                reason: format!("Cache lock failed: {}", e),
            })?;
            cache.put(cache_key, (parsed_ast.clone(), Instant::now()));
        }

        debug!(
            "Parsed {} code in {:?} ({} nodes)",
            detected_language.as_str(),
            parse_time,
            node_count
        );

        Ok(parsed_ast)
    }

    /// Query AST with tree-sitter query patterns
    pub async fn query(
        &self,
        code: String,
        language: Option<SupportedLanguage>,
        pattern: String,
        file_path: Option<PathBuf>,
    ) -> TreeResult<Vec<QueryMatch>> {
        let start_time = Instant::now();

        // Parse the code first
        let parsed_ast = self.parse(code, language, file_path).await?;

        // Get or compile the query
        let query = self.get_or_compile_query(&pattern, parsed_ast.language)?;

        // Execute the query
        let mut cursor = QueryCursor::new();
        let mut results = Vec::new();

        // Iterate over matches manually - QueryMatches doesn't implement Iterator
        let mut matches_iter = cursor.matches(
            &query,
            parsed_ast.root_node(),
            parsed_ast.source_code.as_bytes(),
        );

        // Process matches directly from the streaming iterator
        loop {
            matches_iter.advance();
            let Some(query_match) = matches_iter.get() else {
                break;
            };
            let start_byte = query_match
                .captures
                .iter()
                .map(|c| c.node.start_byte())
                .min()
                .unwrap_or(0);
            let end_byte = query_match
                .captures
                .iter()
                .map(|c| c.node.end_byte())
                .max()
                .unwrap_or(0);
            let start_point = query_match
                .captures
                .iter()
                .map(|c| c.node.start_position())
                .min()
                .unwrap_or_default();
            let end_point = query_match
                .captures
                .iter()
                .map(|c| c.node.end_position())
                .max()
                .unwrap_or_default();

            let captures: Vec<QueryCapture> = query_match
                .captures
                .iter()
                .map(|capture| {
                    let node = capture.node;
                    let capture_name = query.capture_names()[capture.index as usize].to_string();
                    let text = node
                        .utf8_text(parsed_ast.source_code.as_bytes())
                        .unwrap_or("<invalid utf8>")
                        .to_string();

                    QueryCapture {
                        name: capture_name,
                        text,
                        start_byte: node.start_byte() as u32,
                        end_byte: node.end_byte() as u32,
                        start_point: node.start_position().into(),
                        end_point: node.end_position().into(),
                    }
                })
                .collect();

            results.push(QueryMatch {
                pattern_index: query_match.pattern_index as u32,
                captures,
                start_byte: start_byte as u32,
                end_byte: end_byte as u32,
                start_point: start_point.into(),
                end_point: end_point.into(),
            });
        }

        let query_time = start_time.elapsed();
        debug!(
            "Query executed in {:?}, found {} matches",
            query_time,
            results.len()
        );

        Ok(results)
    }

    /// Extract symbols from parsed AST
    pub async fn get_symbols(
        &self,
        code: String,
        language: Option<SupportedLanguage>,
        file_path: Option<PathBuf>,
    ) -> TreeResult<Vec<Symbol>> {
        let start_time = Instant::now();

        // Parse the code first
        let parsed_ast = self.parse(code, language, file_path).await?;

        // Use language-specific symbol extraction
        let symbols = match parsed_ast.language {
            SupportedLanguage::Rust => self.extract_rust_symbols(&parsed_ast)?,
            SupportedLanguage::Python => self.extract_python_symbols(&parsed_ast)?,
            SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => {
                self.extract_js_symbols(&parsed_ast)?
            }
            SupportedLanguage::Go => self.extract_go_symbols(&parsed_ast)?,
            SupportedLanguage::Java => self.extract_java_symbols(&parsed_ast)?,
            SupportedLanguage::C | SupportedLanguage::Cpp => self.extract_c_symbols(&parsed_ast)?,
            SupportedLanguage::CSharp => self.extract_c_symbols(&parsed_ast)?, // Use C extraction for now
            SupportedLanguage::Bash => self.extract_bash_symbols(&parsed_ast)?,
            // For now, return empty vectors for unsupported languages
            SupportedLanguage::Html
            | SupportedLanguage::Css
            | SupportedLanguage::Json
            | SupportedLanguage::Yaml
            | SupportedLanguage::Toml
            | SupportedLanguage::Ruby
            | SupportedLanguage::Php
            | SupportedLanguage::Lua
            | SupportedLanguage::Haskell
            | SupportedLanguage::Elixir
            | SupportedLanguage::Scala
            | SupportedLanguage::Ocaml
            | SupportedLanguage::Clojure
            | SupportedLanguage::Zig
            | SupportedLanguage::Swift
            | SupportedLanguage::Kotlin
            | SupportedLanguage::ObjectiveC
            | SupportedLanguage::Dockerfile
            | SupportedLanguage::Hcl
            | SupportedLanguage::Nix
            | SupportedLanguage::Make
            | SupportedLanguage::Markdown
            // | SupportedLanguage::Latex // Disabled
            | SupportedLanguage::Rst => {
                // TODO: Implement specific symbol extraction for these languages
                vec![]
            }
        };

        let extraction_time = start_time.elapsed();
        debug!(
            "Extracted {} symbols in {:?}",
            symbols.len(),
            extraction_time
        );

        Ok(symbols)
    }

    /// Compare two pieces of code semantically
    pub async fn diff(
        &self,
        old_code: String,
        new_code: String,
        language: Option<SupportedLanguage>,
    ) -> TreeResult<SemanticDiff> {
        let start_time = Instant::now();

        // Parse both pieces of code
        let old_ast = self.parse(old_code, language, None).await?;
        let new_ast = self.parse(new_code, language, None).await?;

        // Ensure same language
        if old_ast.language != new_ast.language {
            return Err(TreeError::DiffFailed {
                reason: format!(
                    "Language mismatch: {} vs {}",
                    old_ast.language.as_str(),
                    new_ast.language.as_str()
                ),
            });
        }

        // Compute semantic diff
        let diff = self.compute_semantic_diff(&old_ast, &new_ast)?;

        let diff_time = start_time.elapsed();
        debug!("Computed diff in {:?}", diff_time);

        Ok(diff)
    }

    /// Parse file and return AST
    pub async fn parse_file(&self, file_path: PathBuf) -> TreeResult<Arc<ParsedAst>> {
        let code = fs::read_to_string(&file_path)
            .await
            .map_err(|e| TreeError::FileReadFailed {
                path: file_path.clone(),
                reason: e.to_string(),
            })?;

        self.parse(code, None, Some(file_path)).await
    }

    /// Query file with pattern
    pub async fn query_file(
        &self,
        file_path: PathBuf,
        pattern: String,
    ) -> TreeResult<Vec<QueryMatch>> {
        let code = fs::read_to_string(&file_path)
            .await
            .map_err(|e| TreeError::FileReadFailed {
                path: file_path.clone(),
                reason: e.to_string(),
            })?;

        self.query(code, None, pattern, Some(file_path)).await
    }

    /// Extract symbols from file
    pub async fn extract_symbols_from_file(&self, file_path: PathBuf) -> TreeResult<Vec<Symbol>> {
        let code = fs::read_to_string(&file_path)
            .await
            .map_err(|e| TreeError::FileReadFailed {
                path: file_path.clone(),
                reason: e.to_string(),
            })?;

        self.get_symbols(code, None, Some(file_path)).await
    }

    /// Compare two files
    pub async fn diff_files(
        &self,
        old_file: PathBuf,
        new_file: PathBuf,
    ) -> TreeResult<SemanticDiff> {
        let old_code =
            fs::read_to_string(&old_file)
                .await
                .map_err(|e| TreeError::FileReadFailed {
                    path: old_file,
                    reason: e.to_string(),
                })?;

        let new_code =
            fs::read_to_string(&new_file)
                .await
                .map_err(|e| TreeError::FileReadFailed {
                    path: new_file,
                    reason: e.to_string(),
                })?;

        self.diff(old_code, new_code, None).await
    }

    /// Detect language from file path
    fn detect_language(&self, path: &Path) -> TreeResult<SupportedLanguage> {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| TreeError::LanguageDetectionFailed {
                path: path.to_path_buf(),
            })?;

        SupportedLanguage::from_extension(extension).ok_or_else(|| TreeError::UnsupportedLanguage {
            language: extension.to_string(),
        })
    }

    /// Get or compile a query for the specified language
    fn get_or_compile_query(
        &self,
        pattern: &str,
        language: SupportedLanguage,
    ) -> TreeResult<Query> {
        let _cache_key = format!("{}:{}", language.as_str(), pattern);

        // Compile the query - we can't cache Query objects as they don't implement Clone
        let query = Query::new(&language.grammar(), pattern).map_err(|e| {
            TreeError::QueryCompilationFailed {
                query: pattern.to_string(),
                reason: e.to_string(),
            }
        })?;

        debug!("Compiled query: {}", pattern);

        Ok(query)
    }

    /// Generate hash for content (simple but effective)
    fn hash_content(&self, content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hash;
        use std::hash::Hasher;

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    /// Count total nodes in AST
    fn count_nodes(&self, node: Node) -> usize {
        let mut count = 1; // Count this node
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                count += self.count_nodes(child);
            }
        }
        count
    }

    /// Extract symbols for Rust code
    const fn extract_rust_symbols(&self, _ast: &ParsedAst) -> TreeResult<Vec<Symbol>> {
        // This would be implemented with Rust-specific query patterns
        // For now, return a basic implementation
        Ok(vec![])
    }

    /// Extract symbols for Python code
    const fn extract_python_symbols(&self, _ast: &ParsedAst) -> TreeResult<Vec<Symbol>> {
        // Python-specific symbol extraction
        Ok(vec![])
    }

    /// Extract symbols for JavaScript/TypeScript code
    const fn extract_js_symbols(&self, _ast: &ParsedAst) -> TreeResult<Vec<Symbol>> {
        // JavaScript/TypeScript-specific symbol extraction
        Ok(vec![])
    }

    /// Extract symbols for Go code
    const fn extract_go_symbols(&self, _ast: &ParsedAst) -> TreeResult<Vec<Symbol>> {
        // Go-specific symbol extraction
        Ok(vec![])
    }

    /// Extract symbols for Java code
    const fn extract_java_symbols(&self, _ast: &ParsedAst) -> TreeResult<Vec<Symbol>> {
        // Java-specific symbol extraction
        Ok(vec![])
    }

    /// Extract symbols for C/C++ code
    const fn extract_c_symbols(&self, _ast: &ParsedAst) -> TreeResult<Vec<Symbol>> {
        // C/C++-specific symbol extraction
        Ok(vec![])
    }

    /// Extract symbols for Bash code
    const fn extract_bash_symbols(&self, _ast: &ParsedAst) -> TreeResult<Vec<Symbol>> {
        // Bash-specific symbol extraction
        Ok(vec![])
    }

    /// Compute semantic diff using enhanced diff engine
    fn compute_semantic_diff(
        &self,
        old_ast: &ParsedAst,
        new_ast: &ParsedAst,
    ) -> TreeResult<SemanticDiff> {
        SemanticDiffEngine::compute_diff(old_ast, new_ast)
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> Result<HashMap<String, serde_json::Value>, TreeError> {
        let mut stats = HashMap::new();

        let ast_cache_size = self
            .ast_cache
            .lock()
            .map_err(|e| TreeError::CacheFailed {
                reason: format!("Cache lock failed: {}", e),
            })?
            .len();

        stats.insert(
            "ast_cache_size".to_string(),
            serde_json::Value::Number(ast_cache_size.into()),
        );

        stats.insert(
            "max_cache_size".to_string(),
            serde_json::Value::Number(self.max_cache_size.get().into()),
        );

        Ok(stats)
    }

    /// Clear all caches
    pub fn clear_caches(&self) -> TreeResult<()> {
        let mut ast_cache = self.ast_cache.lock().map_err(|e| TreeError::CacheFailed {
            reason: format!("Cache lock failed: {}", e),
        })?;
        ast_cache.clear();

        info!("All caches cleared");
        Ok(())
    }
}

#[async_trait::async_trait]
impl InternalTool for TreeTool {
    type Input = TreeInput;
    type Output = ToolOutput<TreeOutput>;
    type Error = TreeError;

    async fn execute(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
        let start_time = Instant::now();

        let (result, operation) = match input {
            TreeInput::Parse {
                code,
                language,
                file_path,
            } => {
                let ast = self.parse(code, language, file_path).await?;
                let output = TreeOutput::Parsed {
                    language: ast.language,
                    node_count: ast.node_count,
                    has_errors: ast.has_errors(),
                    error_count: ast.error_nodes().len(),
                    parse_time_ms: ast.parse_time.as_millis() as u64,
                };
                (output, "parse")
            }
            TreeInput::ParseFile { file_path } => {
                let ast = self.parse_file(file_path).await?;
                let output = TreeOutput::Parsed {
                    language: ast.language,
                    node_count: ast.node_count,
                    has_errors: ast.has_errors(),
                    error_count: ast.error_nodes().len(),
                    parse_time_ms: ast.parse_time.as_millis() as u64,
                };
                (output, "parse_file")
            }
            TreeInput::Query {
                code,
                language,
                pattern,
                file_path,
            } => {
                let matches = self.query(code, language, pattern, file_path).await?;
                let query_time = start_time.elapsed();
                let output = TreeOutput::QueryResults {
                    total_matches: matches.len(),
                    matches,
                    query_time_ms: query_time.as_millis() as u64,
                };
                (output, "query")
            }
            TreeInput::QueryFile { file_path, pattern } => {
                let matches = self.query_file(file_path, pattern).await?;
                let query_time = start_time.elapsed();
                let output = TreeOutput::QueryResults {
                    total_matches: matches.len(),
                    matches,
                    query_time_ms: query_time.as_millis() as u64,
                };
                (output, "query_file")
            }
            TreeInput::ExtractSymbols {
                code,
                language,
                file_path,
            } => {
                let symbols = self.get_symbols(code, language, file_path).await?;
                let extraction_time = start_time.elapsed();
                let output = TreeOutput::Symbols {
                    total_symbols: symbols.len(),
                    symbols,
                    extraction_time_ms: extraction_time.as_millis() as u64,
                };
                (output, "extract_symbols")
            }
            TreeInput::ExtractSymbolsFromFile { file_path } => {
                let symbols = self.extract_symbols_from_file(file_path).await?;
                let extraction_time = start_time.elapsed();
                let output = TreeOutput::Symbols {
                    total_symbols: symbols.len(),
                    symbols,
                    extraction_time_ms: extraction_time.as_millis() as u64,
                };
                (output, "extract_symbols_from_file")
            }
            TreeInput::Diff {
                old_code,
                new_code,
                language,
            } => {
                let diff = self.diff(old_code, new_code, language).await?;
                let diff_time = start_time.elapsed();
                let output = TreeOutput::Diff {
                    diff,
                    diff_time_ms: diff_time.as_millis() as u64,
                };
                (output, "diff")
            }
            TreeInput::DiffFiles { old_file, new_file } => {
                let diff = self.diff_files(old_file, new_file).await?;
                let diff_time = start_time.elapsed();
                let output = TreeOutput::Diff {
                    diff,
                    diff_time_ms: diff_time.as_millis() as u64,
                };
                (output, "diff_files")
            }
        };

        let execution_time = start_time.elapsed();
        let cache_stats = self.cache_stats().unwrap_or_default();

        // Create a minimal location for the result
        let location = SourceLocation {
            file_path: "<tree-tool>".to_string(),
            start_line: 0,
            start_column: 0,
            end_line: 0,
            end_column: 0,
            byte_range: (0, 0),
        };

        let mut output = ToolOutput::new(
            result,
            "tree",
            format!("Tree operation: {}", operation),
            location,
        );

        // Add metadata
        output.metadata.parameters.insert(
            "execution_time_ms".to_string(),
            (execution_time.as_millis() as u64).to_string(),
        );
        output
            .metadata
            .parameters
            .insert("cache_stats".to_string(), format!("{:?}", cache_stats));
        output.metadata.parameters.insert(
            "intelligence_level".to_string(),
            match self.intelligence_level {
                IntelligenceLevel::Light => "light".to_string(),
                IntelligenceLevel::Medium => "medium".to_string(),
                IntelligenceLevel::Hard => "hard".to_string(),
            },
        );

        Ok(output)
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "TreeTool".to_string(),
            description: "Multi-language AST parsing and querying with tree-sitter".to_string(),
            version: "1.0.0".to_string(),
            author: "AGCodex".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rust_parsing() {
        let tool = TreeTool::new(IntelligenceLevel::Medium).unwrap();
        let rust_code = r#"
            fn hello_world() -> String {
                "Hello, world!".to_string()
            }
        "#;

        let result = tool
            .parse(rust_code.to_string(), Some(SupportedLanguage::Rust), None)
            .await;
        assert!(result.is_ok());

        let ast = result.unwrap();
        assert_eq!(ast.language, SupportedLanguage::Rust);
        assert!(!ast.has_errors());
        assert!(ast.node_count > 0);
    }

    #[tokio::test]
    async fn test_language_detection() {
        let tool = TreeTool::new(IntelligenceLevel::Light).unwrap();

        assert_eq!(
            tool.detect_language(Path::new("test.rs")).unwrap(),
            SupportedLanguage::Rust
        );
        assert_eq!(
            tool.detect_language(Path::new("test.py")).unwrap(),
            SupportedLanguage::Python
        );
        assert_eq!(
            tool.detect_language(Path::new("test.js")).unwrap(),
            SupportedLanguage::JavaScript
        );
    }

    #[tokio::test]
    async fn test_query_functionality() {
        let tool = TreeTool::new(IntelligenceLevel::Medium).unwrap();
        let rust_code = r#"
            fn main() {
                println!("Hello, world!");
            }
            fn helper() {
                println!("Helper function");
            }
        "#;

        // Query for all function definitions
        let query_pattern = "(function_item name: (identifier) @func_name)";
        let result = tool
            .query(
                rust_code.to_string(),
                Some(SupportedLanguage::Rust),
                query_pattern.to_string(),
                None,
            )
            .await;

        assert!(result.is_ok());
        let matches = result.unwrap();
        assert_eq!(matches.len(), 2); // Should find main and helper functions
    }

    #[tokio::test]
    async fn test_cache_functionality() {
        let tool = TreeTool::new(IntelligenceLevel::Light).unwrap();
        let code = "fn test() {}".to_string();

        // Parse twice - second should be cached
        let _result1 = tool
            .parse(code.clone(), Some(SupportedLanguage::Rust), None)
            .await
            .unwrap();
        let _result2 = tool
            .parse(code, Some(SupportedLanguage::Rust), None)
            .await
            .unwrap();

        let stats = tool.cache_stats().unwrap();
        let cache_size: i64 = stats.get("ast_cache_size").unwrap().as_i64().unwrap();
        assert_eq!(cache_size, 1); // Should have 1 cached entry
    }
}
