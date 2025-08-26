//! Type-safe builder for AST queries with compile-time validation.

use std::marker::PhantomData;

use serde::Deserialize;
use serde::Serialize;

use super::BuilderError;
use super::BuilderResult;
use super::BuilderState;
use super::Init;
use super::Ready;
use super::Validated;
use crate::types::FilePath;
use crate::types::QueryPattern;

/// AST query language support
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AstLanguage {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    C,
    Cpp,
}

impl AstLanguage {
    /// Get the tree-sitter language name
    pub const fn tree_sitter_name(&self) -> &'static str {
        match self {
            AstLanguage::Rust => "rust",
            AstLanguage::TypeScript => "typescript",
            AstLanguage::JavaScript => "javascript",
            AstLanguage::Python => "python",
            AstLanguage::Go => "go",
            AstLanguage::Java => "java",
            AstLanguage::C => "c",
            AstLanguage::Cpp => "cpp",
        }
    }

    /// Get common file extensions
    pub const fn extensions(&self) -> &'static [&'static str] {
        match self {
            AstLanguage::Rust => &["rs"],
            AstLanguage::TypeScript => &["ts", "tsx"],
            AstLanguage::JavaScript => &["js", "jsx", "mjs"],
            AstLanguage::Python => &["py", "pyx"],
            AstLanguage::Go => &["go"],
            AstLanguage::Java => &["java"],
            AstLanguage::C => &["c", "h"],
            AstLanguage::Cpp => &["cpp", "cxx", "cc", "hpp", "hxx"],
        }
    }
}

/// AST node selector patterns
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AstSelector {
    /// Select by node type (e.g., "function_declaration")
    NodeType(String),
    /// Select by attribute value
    Attribute { name: String, value: String },
    /// Select nodes containing specific text
    Text(QueryPattern),
    /// Select by position in tree
    Position {
        depth: Option<usize>,
        index: Option<usize>,
    },
    /// Composite selector (AND logic)
    And(Vec<AstSelector>),
    /// Alternative selector (OR logic)
    Or(Vec<AstSelector>),
    /// Negation selector (NOT logic)
    Not(Box<AstSelector>),
}

/// AST query configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstQueryConfig {
    /// Maximum depth to traverse
    pub max_depth: Option<usize>,
    /// Maximum number of matches
    pub max_matches: Option<usize>,
    /// Include node metadata in results
    pub include_metadata: bool,
    /// Include surrounding context
    pub context_lines: Option<usize>,
}

impl Default for AstQueryConfig {
    fn default() -> Self {
        Self {
            max_depth: Some(100),
            max_matches: Some(500),
            include_metadata: true,
            context_lines: Some(3),
        }
    }
}

/// Final AST query object
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstQuery {
    pub language: AstLanguage,
    pub selector: AstSelector,
    pub target_files: Vec<FilePath>,
    pub config: AstQueryConfig,
}

impl AstQuery {
    /// Create a new builder
    pub fn builder() -> AstQueryBuilder<Init> {
        AstQueryBuilder::new()
    }

    /// Get the target language
    pub const fn language(&self) -> &AstLanguage {
        &self.language
    }

    /// Get the selector
    pub const fn selector(&self) -> &AstSelector {
        &self.selector
    }

    /// Get target files
    pub fn target_files(&self) -> &[FilePath] {
        &self.target_files
    }

    /// Get query configuration
    pub const fn config(&self) -> &AstQueryConfig {
        &self.config
    }
}

/// Type-safe builder for AST queries
#[derive(Debug)]
pub struct AstQueryBuilder<S: BuilderState> {
    language: Option<AstLanguage>,
    selector: Option<AstSelector>,
    target_files: Vec<FilePath>,
    config: AstQueryConfig,
    _state: PhantomData<S>,
}

impl AstQueryBuilder<Init> {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            language: None,
            selector: None,
            target_files: Vec::new(),
            config: AstQueryConfig::default(),
            _state: PhantomData,
        }
    }

    /// Set the target language (transitions to Validated state)
    pub fn language(mut self, language: AstLanguage) -> AstQueryBuilder<Validated> {
        self.language = Some(language);

        AstQueryBuilder {
            language: self.language,
            selector: self.selector,
            target_files: self.target_files,
            config: self.config,
            _state: PhantomData,
        }
    }
}

impl<S: BuilderState> AstQueryBuilder<S> {
    /// Set maximum traversal depth
    pub const fn max_depth(mut self, depth: Option<usize>) -> Self {
        self.config.max_depth = depth;
        self
    }

    /// Set maximum number of matches
    pub const fn max_matches(mut self, matches: Option<usize>) -> Self {
        self.config.max_matches = matches;
        self
    }

    /// Include metadata in results
    pub const fn include_metadata(mut self, include: bool) -> Self {
        self.config.include_metadata = include;
        self
    }

    /// Set context lines around matches
    pub const fn context_lines(mut self, lines: Option<usize>) -> Self {
        self.config.context_lines = lines;
        self
    }

    /// Add a target file
    pub fn add_file(mut self, file: FilePath) -> Self {
        self.target_files.push(file);
        self
    }

    /// Add multiple target files
    pub fn add_files(mut self, files: impl IntoIterator<Item = FilePath>) -> Self {
        self.target_files.extend(files);
        self
    }
}

impl AstQueryBuilder<Validated> {
    /// Set selector for node type (transitions to Ready state)
    pub fn select_node_type(mut self, node_type: impl Into<String>) -> AstQueryBuilder<Ready> {
        self.selector = Some(AstSelector::NodeType(node_type.into()));

        AstQueryBuilder {
            language: self.language,
            selector: self.selector,
            target_files: self.target_files,
            config: self.config,
            _state: PhantomData,
        }
    }

    /// Set selector for attribute value
    pub fn select_attribute(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> AstQueryBuilder<Ready> {
        self.selector = Some(AstSelector::Attribute {
            name: name.into(),
            value: value.into(),
        });

        AstQueryBuilder {
            language: self.language,
            selector: self.selector,
            target_files: self.target_files,
            config: self.config,
            _state: PhantomData,
        }
    }

    /// Set selector for text content
    pub fn select_text(
        mut self,
        pattern: impl TryInto<QueryPattern>,
    ) -> BuilderResult<AstQueryBuilder<Ready>> {
        let pattern = pattern.try_into().map_err(|_| BuilderError::InvalidField {
            field: "text_pattern",
            value: "invalid pattern".to_string(),
        })?;

        self.selector = Some(AstSelector::Text(pattern));

        Ok(AstQueryBuilder {
            language: self.language,
            selector: self.selector,
            target_files: self.target_files,
            config: self.config,
            _state: PhantomData,
        })
    }

    /// Set selector for position
    pub fn select_position(
        mut self,
        depth: Option<usize>,
        index: Option<usize>,
    ) -> AstQueryBuilder<Ready> {
        self.selector = Some(AstSelector::Position { depth, index });

        AstQueryBuilder {
            language: self.language,
            selector: self.selector,
            target_files: self.target_files,
            config: self.config,
            _state: PhantomData,
        }
    }

    /// Set composite AND selector
    pub fn select_and(mut self, selectors: Vec<AstSelector>) -> AstQueryBuilder<Ready> {
        self.selector = Some(AstSelector::And(selectors));

        AstQueryBuilder {
            language: self.language,
            selector: self.selector,
            target_files: self.target_files,
            config: self.config,
            _state: PhantomData,
        }
    }

    /// Set composite OR selector
    pub fn select_or(mut self, selectors: Vec<AstSelector>) -> AstQueryBuilder<Ready> {
        self.selector = Some(AstSelector::Or(selectors));

        AstQueryBuilder {
            language: self.language,
            selector: self.selector,
            target_files: self.target_files,
            config: self.config,
            _state: PhantomData,
        }
    }

    /// Set negation selector
    pub fn select_not(mut self, selector: AstSelector) -> AstQueryBuilder<Ready> {
        self.selector = Some(AstSelector::Not(Box::new(selector)));

        AstQueryBuilder {
            language: self.language,
            selector: self.selector,
            target_files: self.target_files,
            config: self.config,
            _state: PhantomData,
        }
    }
}

impl AstQueryBuilder<Ready> {
    /// Build the final AstQuery
    pub fn build(self) -> BuilderResult<AstQuery> {
        let language = self
            .language
            .ok_or(BuilderError::MissingField { field: "language" })?;
        let selector = self
            .selector
            .ok_or(BuilderError::MissingField { field: "selector" })?;

        if self.target_files.is_empty() {
            return Err(BuilderError::MissingField {
                field: "target_files",
            });
        }

        Ok(AstQuery {
            language,
            selector,
            target_files: self.target_files,
            config: self.config,
        })
    }
}

impl Default for AstQueryBuilder<Init> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Create a query to find all functions in Rust files
pub fn find_rust_functions(files: Vec<FilePath>) -> BuilderResult<AstQuery> {
    AstQuery::builder()
        .language(AstLanguage::Rust)
        .add_files(files)
        .select_node_type("function_item")
        .build()
}

/// Create a query to find all structs in Rust files
pub fn find_rust_structs(files: Vec<FilePath>) -> BuilderResult<AstQuery> {
    AstQuery::builder()
        .language(AstLanguage::Rust)
        .add_files(files)
        .select_node_type("struct_item")
        .build()
}

/// Create a query to find TypeScript interfaces
pub fn find_typescript_interfaces(files: Vec<FilePath>) -> BuilderResult<AstQuery> {
    AstQuery::builder()
        .language(AstLanguage::TypeScript)
        .add_files(files)
        .select_node_type("interface_declaration")
        .build()
}

/// Create a query to find Python classes
pub fn find_python_classes(files: Vec<FilePath>) -> BuilderResult<AstQuery> {
    AstQuery::builder()
        .language(AstLanguage::Python)
        .add_files(files)
        .select_node_type("class_definition")
        .build()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ast_query_builder_flow() {
        let file = FilePath::new("test.rs").unwrap();

        let query = AstQuery::builder()
            .language(AstLanguage::Rust)
            .add_file(file)
            .select_node_type("function_item")
            .max_depth(Some(50))
            .build()
            .unwrap();

        assert_eq!(query.language(), &AstLanguage::Rust);
        assert!(matches!(query.selector(), AstSelector::NodeType(_)));
        assert_eq!(query.config().max_depth, Some(50));
    }

    #[test]
    fn test_ast_language_properties() {
        assert_eq!(AstLanguage::Rust.tree_sitter_name(), "rust");
        assert!(AstLanguage::Rust.extensions().contains(&"rs"));

        assert_eq!(AstLanguage::TypeScript.tree_sitter_name(), "typescript");
        assert!(AstLanguage::TypeScript.extensions().contains(&"ts"));
        assert!(AstLanguage::TypeScript.extensions().contains(&"tsx"));
    }

    #[test]
    fn test_composite_selectors() {
        let file = FilePath::new("test.rs").unwrap();

        let query = AstQuery::builder()
            .language(AstLanguage::Rust)
            .add_file(file)
            .select_and(vec![
                AstSelector::NodeType("function_item".to_string()),
                AstSelector::Attribute {
                    name: "visibility".to_string(),
                    value: "pub".to_string(),
                },
            ])
            .build()
            .unwrap();

        if let AstSelector::And(selectors) = query.selector() {
            assert_eq!(selectors.len(), 2);
        } else {
            panic!("Expected And selector");
        }
    }

    #[test]
    fn test_builder_requires_files() {
        let result = AstQuery::builder()
            .language(AstLanguage::Rust)
            .select_node_type("function_item")
            .build();

        assert!(result.is_err());

        if let Err(BuilderError::MissingField { field }) = result {
            assert_eq!(field, "target_files");
        }
    }

    #[test]
    fn test_convenience_functions() {
        let file = FilePath::new("test.rs").unwrap();
        let files = vec![file];

        let query = find_rust_functions(files).unwrap();
        assert_eq!(query.language(), &AstLanguage::Rust);

        if let AstSelector::NodeType(node_type) = query.selector() {
            assert_eq!(node_type, "function_item");
        } else {
            panic!("Expected NodeType selector");
        }
    }
}
