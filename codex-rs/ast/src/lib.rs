//! AGCodex AST Module - Tree-sitter based code intelligence
//!
//! This crate provides comprehensive AST parsing, analysis, and compaction
//! for 50+ programming languages using tree-sitter.

pub mod compactor;
pub mod error;
pub mod language_registry;
pub mod parser_cache;
pub mod semantic_index;
pub mod types;

pub use compactor::AstCompactor;
pub use compactor::CompressionLevel;
pub use error::AstError;
pub use error::AstResult;
pub use language_registry::Language;
pub use language_registry::LanguageRegistry;
pub use parser_cache::ParserCache;
pub use semantic_index::SemanticIndex;
pub use semantic_index::Symbol;
pub use semantic_index::SymbolKind;
pub use types::AstNode;
pub use types::AstNodeKind;
pub use types::ParsedAst;
pub use types::SourceLocation;

use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main AST engine for AGCodex
#[derive(Debug)]
pub struct AstEngine {
    registry: Arc<LanguageRegistry>,
    cache: Arc<RwLock<ParserCache>>,
    compactor: Arc<AstCompactor>,
    semantic_index: Arc<RwLock<SemanticIndex>>,
}

impl AstEngine {
    /// Create a new AST engine with specified compression level
    pub fn new(compression_level: CompressionLevel) -> Self {
        Self {
            registry: Arc::new(LanguageRegistry::new()),
            cache: Arc::new(RwLock::new(ParserCache::new(1024 * 1024 * 100))), // 100MB default
            compactor: Arc::new(AstCompactor::new(compression_level)),
            semantic_index: Arc::new(RwLock::new(SemanticIndex::new())),
        }
    }

    /// Parse a file and return its AST
    pub async fn parse_file(&self, path: &Path) -> AstResult<ParsedAst> {
        // Check cache first
        {
            let mut cache = self.cache.write().await;
            if let Some(ast) = cache.get(path) {
                return Ok((*ast).clone());
            }
        }

        // Detect language and parse
        let language = self.registry.detect_language(path)?;
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| AstError::IoError(e.to_string()))?;

        let parsed = self.registry.parse(&language, &content)?;

        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.insert(path.to_path_buf(), parsed.clone());
        }

        // Update semantic index
        {
            let mut index = self.semantic_index.write().await;
            index.index_ast(path, &parsed)?;
        }

        Ok(parsed)
    }

    /// Compact code using AI Distiller-style compression
    pub async fn compact_code(&self, path: &Path) -> AstResult<String> {
        let ast = self.parse_file(path).await?;
        self.compactor.compact(&ast)
    }

    /// Search for symbols across the codebase
    pub async fn search_symbols(&self, query: &str) -> AstResult<Vec<Symbol>> {
        let index = self.semantic_index.read().await;
        Ok(index.search(query))
    }

    /// Get call graph for a function
    pub async fn get_call_graph(&self, path: &Path, function_name: &str) -> AstResult<Vec<Symbol>> {
        let index = self.semantic_index.read().await;
        Ok(index.get_call_graph(path, function_name))
    }

    /// Clear the cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_ast_engine_basic() {
        let engine = AstEngine::new(CompressionLevel::Medium);
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.rs");

        fs::write(
            &file_path,
            "fn main() {\n    println!(\"Hello, world!\");\n}",
        )
        .unwrap();

        let ast = engine.parse_file(&file_path).await.unwrap();
        assert!(!ast.root_node.kind.is_empty());
    }

    #[tokio::test]
    async fn test_compression_levels() {
        let light_engine = AstEngine::new(CompressionLevel::Light);
        let hard_engine = AstEngine::new(CompressionLevel::Hard);

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.py");

        let code = r#"
def calculate_fibonacci(n):
    """Calculate fibonacci number"""
    if n <= 1:
        return n
    else:
        return calculate_fibonacci(n-1) + calculate_fibonacci(n-2)

class MathOperations:
    def __init__(self):
        self.cache = {}
    
    def add(self, a, b):
        return a + b
    
    def multiply(self, a, b):
        return a * b
"#;

        fs::write(&file_path, code).unwrap();

        let light_compact = light_engine.compact_code(&file_path).await.unwrap();
        let hard_compact = hard_engine.compact_code(&file_path).await.unwrap();

        // Hard compression should be shorter
        assert!(hard_compact.len() < light_compact.len());
    }
}
