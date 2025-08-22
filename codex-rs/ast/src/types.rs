//! Core types for AST operations

use crate::language_registry::Language;
use serde::Deserialize;
use serde::Serialize;
use tree_sitter::Tree;

/// Parsed AST with metadata
#[derive(Debug, Clone)]
pub struct ParsedAst {
    pub tree: Tree,
    pub source: String,
    pub language: Language,
    pub root_node: AstNode,
}

/// AST node representation with location metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstNode {
    pub kind: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_position: (usize, usize), // (row, column)
    pub end_position: (usize, usize),   // (row, column)
    pub children_count: usize,
}

/// Source location with precise file:line:column metadata
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file_path: String,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub byte_range: (usize, usize),
}

impl SourceLocation {
    /// Create a new source location
    pub fn new(
        file_path: impl Into<String>,
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
        byte_range: (usize, usize),
    ) -> Self {
        Self {
            file_path: file_path.into(),
            start_line,
            start_column,
            end_line,
            end_column,
            byte_range,
        }
    }

    /// Format as file:line:column string
    pub fn as_string(&self) -> String {
        format!(
            "{}:{}:{}",
            self.file_path, self.start_line, self.start_column
        )
    }

    /// Format with range
    pub fn to_range_string(&self) -> String {
        if self.start_line == self.end_line {
            format!(
                "{}:{}:{}-{}",
                self.file_path, self.start_line, self.start_column, self.end_column
            )
        } else {
            format!(
                "{}:{}:{}-{}:{}",
                self.file_path, self.start_line, self.start_column, self.end_line, self.end_column
            )
        }
    }
}

impl std::fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}",
            self.file_path, self.start_line, self.start_column
        )
    }
}

/// AST node kind classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AstNodeKind {
    Function,
    Class,
    Struct,
    Enum,
    Interface,
    Trait,
    Module,
    Variable,
    Constant,
    Type,
    Import,
    Comment,
    Other,
}

impl AstNodeKind {
    /// Determine node kind from tree-sitter node type
    pub fn from_node_type(node_type: &str) -> Self {
        match node_type {
            "function_declaration"
            | "function_definition"
            | "function_item"
            | "method_declaration"
            | "method_definition" => Self::Function,

            "class_declaration" | "class_definition" => Self::Class,

            "struct_item" | "struct_declaration" => Self::Struct,

            "enum_item" | "enum_declaration" => Self::Enum,

            "interface_declaration" | "protocol_declaration" => Self::Interface,

            "trait_item" | "trait_declaration" => Self::Trait,

            "module" | "module_declaration" | "namespace" => Self::Module,

            "variable_declaration" | "let_declaration" | "const_item" => Self::Variable,

            "constant_declaration" => Self::Constant,

            "type_alias" | "type_definition" | "typedef" => Self::Type,

            "import_statement" | "use_declaration" | "import_declaration" => Self::Import,

            "comment" | "line_comment" | "block_comment" | "doc_comment" => Self::Comment,

            _ => Self::Other,
        }
    }
}

/// Code chunk for hierarchical organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChunk {
    pub id: String,
    pub level: ChunkLevel,
    pub kind: AstNodeKind,
    pub name: String,
    pub content: String,
    pub location: SourceLocation,
    pub parent_id: Option<String>,
    pub children_ids: Vec<String>,
    pub metadata: ChunkMetadata,
}

/// Hierarchical chunk level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ChunkLevel {
    File,
    Module,
    Class,
    Function,
    Block,
}

impl ChunkLevel {
    /// Get display name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Module => "Module",
            Self::Class => "Class",
            Self::Function => "Function",
            Self::Block => "Block",
        }
    }
}

/// Metadata for a code chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub language: Language,
    pub complexity: usize,
    pub token_count: usize,
    pub line_count: usize,
    pub has_tests: bool,
    pub has_docs: bool,
    pub visibility: Visibility,
    pub dependencies: Vec<String>,
    pub symbols_defined: Vec<String>,
    pub symbols_used: Vec<String>,
}

/// Code visibility level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Protected,
    Private,
    Internal,
    Package,
}

impl Visibility {
    /// Parse visibility from source text
    pub fn from_text(text: &str) -> Self {
        if text.contains("public") || text.contains("pub") || text.contains("export") {
            Self::Public
        } else if text.contains("protected") {
            Self::Protected
        } else if text.contains("private") || text.contains("priv") {
            Self::Private
        } else if text.contains("internal") {
            Self::Internal
        } else if text.contains("package") {
            Self::Package
        } else {
            // Default visibility varies by language
            Self::Public
        }
    }
}
