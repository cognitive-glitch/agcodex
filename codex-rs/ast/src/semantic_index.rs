//! Semantic indexing for symbols, relationships, and call graphs

use crate::error::AstError;
use crate::error::AstResult;
use crate::types::AstNodeKind;
use crate::types::ParsedAst;
use crate::types::SourceLocation;
use crate::types::Visibility;
use dashmap::DashMap;
// use std::collections::{HashMap, HashSet}; // unused
use std::path::Path;
use std::path::PathBuf;
use tree_sitter::Node;
use tree_sitter::TreeCursor;

/// Symbol in the codebase
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub location: SourceLocation,
    pub visibility: Visibility,
    pub signature: String,
    pub documentation: Option<String>,
    pub references: Vec<SourceLocation>,
    pub definitions: Vec<SourceLocation>,
    pub call_sites: Vec<SourceLocation>,
}

/// Symbol kind classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Interface,
    Trait,
    Module,
    Variable,
    Constant,
    Type,
    Property,
    Field,
    Parameter,
}

impl SymbolKind {
    /// Convert from AST node kind
    pub const fn from_ast_kind(kind: AstNodeKind) -> Self {
        match kind {
            AstNodeKind::Function => Self::Function,
            AstNodeKind::Class => Self::Class,
            AstNodeKind::Struct => Self::Struct,
            AstNodeKind::Enum => Self::Enum,
            AstNodeKind::Interface => Self::Interface,
            AstNodeKind::Trait => Self::Trait,
            AstNodeKind::Module => Self::Module,
            AstNodeKind::Variable => Self::Variable,
            AstNodeKind::Constant => Self::Constant,
            AstNodeKind::Type => Self::Type,
            _ => Self::Variable,
        }
    }
}

/// Semantic index for code intelligence
#[derive(Debug)]
pub struct SemanticIndex {
    /// Symbol table: symbol_id -> Symbol
    symbols: DashMap<String, Symbol>,
    /// File index: file_path -> symbol_ids
    file_symbols: DashMap<PathBuf, Vec<String>>,
    /// Call graph: caller_id -> [callee_ids]
    call_graph: DashMap<String, Vec<String>>,
    /// Inheritance graph: parent_id -> [child_ids]
    inheritance_graph: DashMap<String, Vec<String>>,
    /// Import graph: file_path -> [imported_files]
    import_graph: DashMap<PathBuf, Vec<PathBuf>>,
}

impl SemanticIndex {
    /// Create a new semantic index
    pub fn new() -> Self {
        Self {
            symbols: DashMap::new(),
            file_symbols: DashMap::new(),
            call_graph: DashMap::new(),
            inheritance_graph: DashMap::new(),
            import_graph: DashMap::new(),
        }
    }

    /// Index an AST for semantic information
    pub fn index_ast(&mut self, path: &Path, ast: &ParsedAst) -> AstResult<()> {
        let source = ast.source.as_bytes();
        let mut cursor = ast.tree.root_node().walk();
        let mut symbols = Vec::new();

        // Extract symbols from AST
        self.extract_symbols(&mut cursor, source, path, &mut symbols)?;

        // Store file symbols
        let symbol_ids: Vec<String> = symbols.iter().map(|s| self.get_symbol_id(s)).collect();
        self.file_symbols.insert(path.to_path_buf(), symbol_ids);

        // Insert symbols into index
        for symbol in symbols {
            let id = self.get_symbol_id(&symbol);
            self.symbols.insert(id, symbol);
        }

        // Build relationships
        self.build_relationships(path, ast)?;

        Ok(())
    }

    /// Extract symbols from AST nodes
    fn extract_symbols(
        &self,
        cursor: &mut TreeCursor,
        source: &[u8],
        file_path: &Path,
        symbols: &mut Vec<Symbol>,
    ) -> AstResult<()> {
        let node = cursor.node();
        let node_type = node.kind();
        let _ast_kind = AstNodeKind::from_node_type(node_type);

        // Check if this is a symbol definition
        if self.is_symbol_definition(&node) {
            let symbol = self.create_symbol(&node, source, file_path)?;
            symbols.push(symbol);
        }

        // Recurse into children
        if cursor.goto_first_child() {
            loop {
                self.extract_symbols(cursor, source, file_path, symbols)?;
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }

        Ok(())
    }

    /// Check if a node represents a symbol definition
    fn is_symbol_definition(&self, node: &Node) -> bool {
        let node_type = node.kind();
        matches!(
            node_type,
            "function_declaration"
                | "function_definition"
                | "function_item"
                | "method_declaration"
                | "method_definition"
                | "class_declaration"
                | "class_definition"
                | "struct_item"
                | "struct_declaration"
                | "enum_item"
                | "enum_declaration"
                | "interface_declaration"
                | "protocol_declaration"
                | "trait_item"
                | "trait_declaration"
                | "module"
                | "module_declaration"
                | "variable_declaration"
                | "const_item"
                | "let_declaration"
                | "type_alias"
                | "typedef"
        )
    }

    /// Create a symbol from an AST node
    fn create_symbol(&self, node: &Node, source: &[u8], file_path: &Path) -> AstResult<Symbol> {
        let name = self.extract_symbol_name(node, source)?;
        let kind = SymbolKind::from_ast_kind(AstNodeKind::from_node_type(node.kind()));

        let location = SourceLocation::new(
            file_path.display().to_string(),
            node.start_position().row + 1,
            node.start_position().column + 1,
            node.end_position().row + 1,
            node.end_position().column + 1,
            (node.start_byte(), node.end_byte()),
        );

        let signature = self.extract_signature(node, source)?;
        let visibility = self.extract_visibility(node, source);
        let documentation = self.extract_documentation(node, source);

        Ok(Symbol {
            name,
            kind,
            location: location.clone(),
            visibility,
            signature,
            documentation,
            references: Vec::new(),
            definitions: vec![location],
            call_sites: Vec::new(),
        })
    }

    /// Extract symbol name from node
    fn extract_symbol_name(&self, node: &Node, source: &[u8]) -> AstResult<String> {
        // Look for identifier child node
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i)
                && (child.kind() == "identifier" || child.kind() == "name")
            {
                let name = std::str::from_utf8(&source[child.byte_range()])
                    .map_err(|e| AstError::ParserError(e.to_string()))?;
                return Ok(name.to_string());
            }
        }

        // Fallback: use first non-keyword text
        let text = std::str::from_utf8(&source[node.byte_range()])
            .map_err(|e| AstError::ParserError(e.to_string()))?;

        // Extract name from text (simple heuristic)
        let words: Vec<&str> = text.split_whitespace().collect();
        for word in words {
            if !Self::is_keyword(word) && word.chars().any(|c| c.is_alphabetic()) {
                return Ok(word.to_string());
            }
        }

        Ok("anonymous".to_string())
    }

    /// Check if a word is a keyword
    fn is_keyword(word: &str) -> bool {
        matches!(
            word,
            "fn" | "function"
                | "def"
                | "class"
                | "struct"
                | "enum"
                | "interface"
                | "trait"
                | "impl"
                | "module"
                | "namespace"
                | "const"
                | "let"
                | "var"
                | "type"
                | "public"
                | "private"
                | "protected"
                | "static"
                | "async"
                | "export"
                | "import"
        )
    }

    /// Extract signature from node
    fn extract_signature(&self, node: &Node, source: &[u8]) -> AstResult<String> {
        // Get text up to body/block
        let mut sig_end = node.end_byte();

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i)
                && (child.kind() == "block"
                    || child.kind() == "compound_statement"
                    || child.kind() == "function_body")
            {
                sig_end = child.start_byte();
                break;
            }
        }

        let signature = std::str::from_utf8(&source[node.start_byte()..sig_end])
            .map_err(|e| AstError::ParserError(e.to_string()))?;

        Ok(signature.trim().to_string())
    }

    /// Extract visibility from node
    fn extract_visibility(&self, node: &Node, source: &[u8]) -> Visibility {
        let text = std::str::from_utf8(&source[node.byte_range()]).unwrap_or("");
        Visibility::from_text(text)
    }

    /// Extract documentation from node
    fn extract_documentation(&self, node: &Node, source: &[u8]) -> Option<String> {
        // Look for preceding comment node
        if let Some(prev) = node.prev_sibling()
            && prev.kind().contains("comment")
        {
            let doc = std::str::from_utf8(&source[prev.byte_range()]).ok()?;
            return Some(doc.to_string());
        }
        None
    }

    /// Build relationships between symbols
    const fn build_relationships(&mut self, _path: &Path, _ast: &ParsedAst) -> AstResult<()> {
        // TODO: Implement call graph building
        // TODO: Implement inheritance graph building
        // TODO: Implement import graph building
        Ok(())
    }

    /// Generate symbol ID
    fn get_symbol_id(&self, symbol: &Symbol) -> String {
        format!(
            "{}:{}:{}",
            symbol.location.file_path, symbol.kind as u8, symbol.name
        )
    }

    /// Search for symbols by query
    pub fn search(&self, query: &str) -> Vec<Symbol> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for entry in self.symbols.iter() {
            let symbol = entry.value();
            if symbol.name.to_lowercase().contains(&query_lower) {
                results.push(symbol.clone());
            }
        }

        results
    }

    /// Get call graph for a function
    pub fn get_call_graph(&self, path: &Path, function_name: &str) -> Vec<Symbol> {
        // Find the function symbol
        let symbol_id = format!(
            "{}:{}:{}",
            path.display(),
            SymbolKind::Function as u8,
            function_name
        );

        // Get callees
        if let Some(callees) = self.call_graph.get(&symbol_id) {
            let mut results = Vec::new();
            for callee_id in callees.value() {
                if let Some(symbol) = self.symbols.get(callee_id) {
                    results.push(symbol.clone());
                }
            }
            return results;
        }

        Vec::new()
    }

    /// Get symbols defined in a file
    pub fn get_file_symbols(&self, path: &Path) -> Vec<Symbol> {
        if let Some(symbol_ids) = self.file_symbols.get(&path.to_path_buf()) {
            let mut results = Vec::new();
            for id in symbol_ids.value() {
                if let Some(symbol) = self.symbols.get(id) {
                    results.push(symbol.clone());
                }
            }
            return results;
        }
        Vec::new()
    }

    /// Clear the index
    pub fn clear(&mut self) {
        self.symbols.clear();
        self.file_symbols.clear();
        self.call_graph.clear();
        self.inheritance_graph.clear();
        self.import_graph.clear();
    }
}

impl Default for SemanticIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language_registry::Language;
    use crate::language_registry::LanguageRegistry;

    #[test]
    fn test_semantic_indexing() {
        let mut index = SemanticIndex::new();
        let registry = LanguageRegistry::new();

        let code = r#"
pub fn calculate(x: i32, y: i32) -> i32 {
    add(x, y)
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub struct Calculator {
    value: i32,
}

impl Calculator {
    pub fn new() -> Self {
        Self { value: 0 }
    }
}
"#;

        let ast = registry.parse(&Language::Rust, code).unwrap();
        let path = Path::new("test.rs");

        index.index_ast(path, &ast).unwrap();

        // Search for symbols
        let results = index.search("calc");
        assert!(!results.is_empty());
        assert!(results.iter().any(|s| s.name.contains("calculate")));

        // Get file symbols
        let file_symbols = index.get_file_symbols(path);
        assert!(file_symbols.len() >= 3); // calculate, add, Calculator
    }
}
