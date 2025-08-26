//! Type definitions for AST compaction
//!
//! This module contains all the core types, enums, and structures used
//! throughout the AST compactor. Designed for zero-copy operations where
//! possible using `Cow<str>` and efficient memory layout.

use std::borrow::Cow;

use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

/// Comprehensive error type for AST compaction operations
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum CompactionError {
    /// Language detection failed
    #[error("Could not detect programming language from source code")]
    LanguageDetectionFailed,

    /// Unsupported language
    #[error("Language '{language}' is not supported")]
    UnsupportedLanguage { language: String },

    /// Tree-sitter parsing error
    #[error("Failed to parse source code: {message}")]
    ParseError { message: String },

    /// Parser initialization error
    #[error("Failed to initialize parser for language '{language}': {error}")]
    ParserInitError { language: String, error: String },

    /// AST traversal error
    #[error("Error traversing AST: {message}")]
    TraversalError { message: String },

    /// Invalid source code encoding
    #[error("Source code contains invalid UTF-8 encoding")]
    InvalidEncoding,

    /// Empty or invalid input
    #[error("Input source code is empty or invalid")]
    EmptyInput,

    /// Extraction error for specific language construct
    #[error("Failed to extract {element_type}: {reason}")]
    ExtractionError {
        element_type: String,
        reason: String,
    },

    /// Memory allocation error
    #[error("Memory allocation failed: {details}")]
    MemoryError { details: String },

    /// Internal consistency error
    #[error("Internal compactor error: {message}")]
    InternalError { message: String },
}

/// Configuration options for AST compaction
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionOptions {
    /// Target programming language (auto-detected if None)
    pub language: Option<Language>,

    /// Whether to preserve documentation comments
    pub preserve_docs: bool,

    /// Extract only function/method signatures (no bodies)
    pub preserve_signatures_only: bool,

    /// Include private/internal elements
    pub include_private: bool,

    /// Include type annotations and hints
    pub include_types: bool,

    /// Use zero-copy string operations where possible
    pub zero_copy: bool,

    /// Maximum depth for AST traversal (prevents infinite recursion)
    pub max_depth: usize,

    /// Custom element filters
    pub element_filters: Vec<ElementFilter>,
}

impl CompactionOptions {
    /// Create new compaction options with sensible defaults
    pub const fn new() -> Self {
        Self {
            language: None,
            preserve_docs: true,
            preserve_signatures_only: false,
            include_private: false,
            include_types: true,
            zero_copy: true,
            max_depth: 100,
            element_filters: Vec::new(),
        }
    }

    /// Set the target language
    pub const fn with_language(mut self, language: Language) -> Self {
        self.language = Some(language);
        self
    }

    /// Configure documentation preservation
    pub const fn preserve_docs(mut self, preserve: bool) -> Self {
        self.preserve_docs = preserve;
        self
    }

    /// Configure signature-only extraction
    pub const fn preserve_signatures_only(mut self, signatures_only: bool) -> Self {
        self.preserve_signatures_only = signatures_only;
        self
    }

    /// Configure private element inclusion
    pub const fn include_private(mut self, include: bool) -> Self {
        self.include_private = include;
        self
    }

    /// Configure type annotation inclusion
    pub const fn include_types(mut self, include: bool) -> Self {
        self.include_types = include;
        self
    }

    /// Configure zero-copy operations
    pub const fn zero_copy(mut self, zero_copy: bool) -> Self {
        self.zero_copy = zero_copy;
        self
    }

    /// Set maximum AST traversal depth
    pub const fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Add custom element filter
    pub fn with_filter(mut self, filter: ElementFilter) -> Self {
        self.element_filters.push(filter);
        self
    }
}

/// Supported programming languages for AST compaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    C,
    Cpp,
    CSharp,
    Unknown,
}

impl Language {
    /// Get the canonical name for the language
    pub const fn name(self) -> &'static str {
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
            Self::Unknown => "unknown",
        }
    }

    /// Get file extensions for this language
    pub const fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::Rust => &["rs"],
            Self::Python => &["py", "pyi"],
            Self::JavaScript => &["js", "mjs"],
            Self::TypeScript => &["ts", "tsx"],
            Self::Go => &["go"],
            Self::Java => &["java"],
            Self::C => &["c", "h"],
            Self::Cpp => &["cpp", "cxx", "cc", "hpp", "hxx"],
            Self::CSharp => &["cs"],
            Self::Unknown => &[],
        }
    }

    /// Get common comment patterns for this language
    pub const fn comment_patterns(self) -> (&'static str, Option<(&'static str, &'static str)>) {
        match self {
            Self::Rust
            | Self::JavaScript
            | Self::TypeScript
            | Self::Go
            | Self::Java
            | Self::C
            | Self::Cpp
            | Self::CSharp
            | Self::Unknown => ("//", Some(("/*", "*/"))),
            Self::Python => ("#", Some(("\"\"\"", "\"\"\""))),
        }
    }
}

impl std::str::FromStr for Language {
    type Err = CompactionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "rust" | "rs" => Ok(Self::Rust),
            "python" | "py" => Ok(Self::Python),
            "javascript" | "js" => Ok(Self::JavaScript),
            "typescript" | "ts" => Ok(Self::TypeScript),
            "go" => Ok(Self::Go),
            "java" => Ok(Self::Java),
            "c" => Ok(Self::C),
            "cpp" | "c++" | "cxx" => Ok(Self::Cpp),
            "csharp" | "c#" | "cs" => Ok(Self::CSharp),
            _ => Err(CompactionError::UnsupportedLanguage {
                language: s.to_string(),
            }),
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Custom filter for specific AST elements
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementFilter {
    /// Name pattern to match (regex)
    pub name_pattern: String,

    /// Element type to filter
    pub element_type: ElementType,

    /// Whether to include (true) or exclude (false) matches
    pub include: bool,
}

/// Types of AST elements that can be extracted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElementType {
    Function,
    Method,
    Struct,
    Class,
    Interface,
    Trait,
    Enum,
    Type,
    Constant,
    Variable,
    Import,
    Export,
    Comment,
}

impl std::fmt::Display for ElementType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Struct => "struct",
            Self::Class => "class",
            Self::Interface => "interface",
            Self::Trait => "trait",
            Self::Enum => "enum",
            Self::Type => "type",
            Self::Constant => "constant",
            Self::Variable => "variable",
            Self::Import => "import",
            Self::Export => "export",
            Self::Comment => "comment",
        };
        write!(f, "{}", name)
    }
}

/// Extracted AST element with metadata
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedElement<'a> {
    /// Type of the element
    pub element_type: ElementType,

    /// Name/identifier of the element
    pub name: Cow<'a, str>,

    /// Source code representation (zero-copy when possible)
    pub source: Cow<'a, str>,

    /// Visibility (public, private, etc.)
    pub visibility: Visibility,

    /// Associated documentation
    pub documentation: Option<Cow<'a, str>>,

    /// Source location information
    pub location: SourceLocation,

    /// Additional metadata specific to element type
    pub metadata: ElementMetadata<'a>,
}

/// Visibility levels for extracted elements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
    Package,
}

impl std::fmt::Display for Visibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Public => "public",
            Self::Private => "private",
            Self::Protected => "protected",
            Self::Internal => "internal",
            Self::Package => "package",
        };
        write!(f, "{}", name)
    }
}

/// Source location information
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

impl SourceLocation {
    /// Create a new source location
    pub const fn new(
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
        start_byte: usize,
        end_byte: usize,
    ) -> Self {
        Self {
            start_line,
            start_column,
            end_line,
            end_column,
            start_byte,
            end_byte,
        }
    }

    /// Get the span in bytes
    pub const fn byte_span(self) -> (usize, usize) {
        (self.start_byte, self.end_byte)
    }

    /// Get the line span
    pub const fn line_span(self) -> (usize, usize) {
        (self.start_line, self.end_line)
    }
}

/// Element-specific metadata
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementMetadata<'a> {
    Function(FunctionSignature<'a>),
    Struct(StructDefinition<'a>),
    Class(ClassDefinition<'a>),
    Interface(InterfaceDefinition<'a>),
    Trait(TraitDefinition<'a>),
    Enum(EnumDefinition<'a>),
    Type(TypeDefinition<'a>),
    Constant(ConstantDefinition<'a>),
    Variable(VariableDefinition<'a>),
    Import(ImportStatement<'a>),
    Export(ExportStatement<'a>),
    Comment(CommentContent<'a>),
}

/// Function signature information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSignature<'a> {
    pub parameters: Vec<Parameter<'a>>,
    pub return_type: Option<Cow<'a, str>>,
    pub generic_params: Vec<Cow<'a, str>>,
    pub is_async: bool,
    pub is_unsafe: bool,
    pub is_const: bool,
}

/// Function parameter
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter<'a> {
    pub name: Cow<'a, str>,
    pub param_type: Option<Cow<'a, str>>,
    pub default_value: Option<Cow<'a, str>>,
    pub is_optional: bool,
    pub is_variadic: bool,
}

/// Struct definition information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructDefinition<'a> {
    pub fields: Vec<FieldDefinition<'a>>,
    pub generic_params: Vec<Cow<'a, str>>,
    pub derives: Vec<Cow<'a, str>>,
}

/// Struct field definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDefinition<'a> {
    pub name: Cow<'a, str>,
    pub field_type: Cow<'a, str>,
    pub visibility: Visibility,
    pub documentation: Option<Cow<'a, str>>,
}

/// Class definition information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassDefinition<'a> {
    pub parent_class: Option<Cow<'a, str>>,
    pub interfaces: Vec<Cow<'a, str>>,
    pub generic_params: Vec<Cow<'a, str>>,
    pub is_abstract: bool,
    pub is_final: bool,
}

/// Interface definition information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceDefinition<'a> {
    pub parent_interfaces: Vec<Cow<'a, str>>,
    pub generic_params: Vec<Cow<'a, str>>,
}

/// Trait definition information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitDefinition<'a> {
    pub parent_traits: Vec<Cow<'a, str>>,
    pub associated_types: Vec<Cow<'a, str>>,
    pub generic_params: Vec<Cow<'a, str>>,
}

/// Enum definition information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDefinition<'a> {
    pub variants: Vec<EnumVariant<'a>>,
    pub generic_params: Vec<Cow<'a, str>>,
}

/// Enum variant
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariant<'a> {
    pub name: Cow<'a, str>,
    pub fields: Vec<FieldDefinition<'a>>,
    pub discriminant: Option<Cow<'a, str>>,
}

/// Type definition information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDefinition<'a> {
    pub underlying_type: Cow<'a, str>,
    pub generic_params: Vec<Cow<'a, str>>,
    pub constraints: Vec<Cow<'a, str>>,
}

/// Constant definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantDefinition<'a> {
    pub const_type: Option<Cow<'a, str>>,
    pub value: Option<Cow<'a, str>>,
}

/// Variable definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableDefinition<'a> {
    pub var_type: Option<Cow<'a, str>>,
    pub is_mutable: bool,
    pub initial_value: Option<Cow<'a, str>>,
}

/// Import statement
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportStatement<'a> {
    pub module_path: Cow<'a, str>,
    pub imported_items: Vec<Cow<'a, str>>,
    pub alias: Option<Cow<'a, str>>,
}

/// Export statement
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportStatement<'a> {
    pub exported_items: Vec<Cow<'a, str>>,
    pub module_path: Option<Cow<'a, str>>,
}

/// Comment content
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentContent<'a> {
    pub content: Cow<'a, str>,
    pub is_documentation: bool,
    pub comment_type: CommentType,
}

/// Type of comment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentType {
    Line,
    Block,
    Documentation,
}

/// Result of AST compaction operation
#[derive(Debug, Clone)]
pub struct CompactionResult<'a> {
    /// The compacted source code
    pub compacted_code: String,

    /// Original source code size in bytes
    pub original_size: usize,

    /// Compressed size in bytes
    pub compressed_size: usize,

    /// Compression ratio (0.0 to 1.0)
    pub compression_ratio: f64,

    /// Detected or specified language
    pub language: Language,

    /// Extracted elements with metadata
    pub elements: Vec<ExtractedElement<'a>>,

    /// Processing time in milliseconds
    pub processing_time_ms: u64,

    /// Additional metrics
    pub metrics: CompactionMetrics,
}

impl<'a> CompactionResult<'a> {
    /// Create a new compaction result
    pub fn new(
        compacted_code: String,
        original_size: usize,
        language: Language,
        elements: Vec<ExtractedElement<'a>>,
        processing_time_ms: u64,
    ) -> Self {
        let compressed_size = compacted_code.len();
        let compression_ratio = if original_size > 0 {
            1.0 - (compressed_size as f64 / original_size as f64)
        } else {
            0.0
        };

        Self {
            compacted_code,
            original_size,
            compressed_size,
            compression_ratio,
            language,
            elements,
            processing_time_ms,
            metrics: CompactionMetrics::default(),
        }
    }

    /// Get compression percentage
    pub fn compression_percentage(&self) -> f64 {
        self.compression_ratio * 100.0
    }

    /// Get elements of a specific type
    pub fn elements_of_type(&self, element_type: ElementType) -> Vec<&ExtractedElement<'a>> {
        self.elements
            .iter()
            .filter(|e| e.element_type == element_type)
            .collect()
    }

    /// Get public elements only
    pub fn public_elements(&self) -> Vec<&ExtractedElement<'a>> {
        self.elements
            .iter()
            .filter(|e| e.visibility == Visibility::Public)
            .collect()
    }
}

/// Additional metrics about the compaction process
#[derive(Debug, Clone, Default)]
pub struct CompactionMetrics {
    /// Number of functions extracted
    pub functions_count: usize,

    /// Number of types extracted
    pub types_count: usize,

    /// Number of documentation comments preserved
    pub docs_count: usize,

    /// Parse tree depth
    pub ast_depth: usize,

    /// Number of AST nodes processed
    pub nodes_processed: usize,

    /// Memory usage statistics
    pub memory_stats: MemoryStats,
}

/// Memory usage statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    /// Peak memory usage in bytes
    pub peak_memory_bytes: usize,

    /// Number of allocations avoided through zero-copy
    pub zero_copy_optimizations: usize,

    /// Total string allocations
    pub string_allocations: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_str() {
        assert_eq!("rust".parse::<Language>().unwrap(), Language::Rust);
        assert_eq!("python".parse::<Language>().unwrap(), Language::Python);
        assert_eq!(
            "typescript".parse::<Language>().unwrap(),
            Language::TypeScript
        );

        assert!("unknown".parse::<Language>().is_err());
    }

    #[test]
    fn test_compaction_options_builder() {
        let options = CompactionOptions::new()
            .with_language(Language::Rust)
            .preserve_docs(true)
            .preserve_signatures_only(true)
            .include_private(false);

        assert_eq!(options.language, Some(Language::Rust));
        assert!(options.preserve_docs);
        assert!(options.preserve_signatures_only);
        assert!(!options.include_private);
    }

    #[test]
    fn test_source_location() {
        let loc = SourceLocation::new(1, 0, 5, 10, 0, 100);

        assert_eq!(loc.line_span(), (1, 5));
        assert_eq!(loc.byte_span(), (0, 100));
    }

    #[test]
    fn test_compression_ratio_calculation() {
        let result = CompactionResult::new(
            "fn test();".to_string(),
            100,
            Language::Rust,
            Vec::new(),
            10,
        );

        // compressed_size = 10, original_size = 100
        // ratio = 1.0 - (10/100) = 0.9
        assert!((result.compression_ratio - 0.9).abs() < f64::EPSILON);
        assert!((result.compression_percentage() - 90.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_element_filtering() {
        let elements = vec![ExtractedElement {
            element_type: ElementType::Function,
            name: "test_func".into(),
            source: "fn test_func() {}".into(),
            visibility: Visibility::Public,
            documentation: None,
            location: SourceLocation::new(1, 0, 1, 15, 0, 15),
            metadata: ElementMetadata::Function(FunctionSignature {
                parameters: Vec::new(),
                return_type: None,
                generic_params: Vec::new(),
                is_async: false,
                is_unsafe: false,
                is_const: false,
            }),
        }];

        let result = CompactionResult::new(
            "fn test_func();".to_string(),
            100,
            Language::Rust,
            elements,
            10,
        );

        let functions = result.elements_of_type(ElementType::Function);
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "test_func");

        let public_elements = result.public_elements();
        assert_eq!(public_elements.len(), 1);
    }
}
