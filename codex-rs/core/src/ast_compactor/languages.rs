//! Language-specific AST handlers and parsers
//!
//! This module provides language-specific implementations for parsing and
//! extracting elements from different programming languages. Each language
//! handler implements the `LanguageHandler` trait to provide uniform access
//! to language-specific functionality.

use crate::ast_compactor::types::CompactionError;
use crate::ast_compactor::types::ConstantDefinition;
use crate::ast_compactor::types::ElementMetadata;
use crate::ast_compactor::types::ElementType;
use crate::ast_compactor::types::EnumDefinition;
use crate::ast_compactor::types::ExtractedElement;
use crate::ast_compactor::types::FieldDefinition;
use crate::ast_compactor::types::FunctionSignature;
use crate::ast_compactor::types::ImportStatement;
use crate::ast_compactor::types::Language;
use crate::ast_compactor::types::Parameter;
use crate::ast_compactor::types::SourceLocation;
use crate::ast_compactor::types::StructDefinition;
use crate::ast_compactor::types::TraitDefinition;
use crate::ast_compactor::types::TypeDefinition;
use crate::ast_compactor::types::Visibility;

use tree_sitter::Node;
use tree_sitter::Tree;

/// Trait for language-specific AST handlers
pub trait LanguageHandler {
    /// Get the tree-sitter language for parsing
    fn language(&self) -> tree_sitter::Language;

    /// Get language-specific queries for extracting elements
    fn queries(&self) -> &LanguageQueries;

    /// Extract all elements from parsed AST
    fn extract_elements<'a>(
        &'a self,
        tree: &'a Tree,
        source: &'a str,
    ) -> Result<Vec<ExtractedElement<'a>>, CompactionError>;

    /// Detect if source code is this language
    fn detect_language(&self, source: &str) -> bool;

    /// Format extracted element for output
    fn format_element(&self, element: &ExtractedElement<'_>, signatures_only: bool) -> String;
}

/// Language-specific tree-sitter queries
#[derive(Debug, Clone)]
pub struct LanguageQueries {
    pub functions: String,
    pub structs: String,
    pub classes: String,
    pub interfaces: String,
    pub traits: String,
    pub enums: String,
    pub types: String,
    pub constants: String,
    pub variables: String,
    pub imports: String,
    pub exports: String,
    pub comments: String,
}

/// Factory for creating language handlers
pub struct LanguageHandlerFactory;

impl LanguageHandlerFactory {
    /// Create appropriate handler for given language
    pub fn create_handler(language: Language) -> Box<dyn LanguageHandler> {
        match language {
            Language::Rust => Box::new(RustHandler::new()),
            Language::Python => Box::new(PythonHandler::new()),
            Language::JavaScript => Box::new(JavaScriptHandler::new()),
            Language::TypeScript => Box::new(TypeScriptHandler::new()),
            Language::Go => Box::new(GoHandler::new()),
            Language::Java => Box::new(JavaHandler::new()),
            Language::C => Box::new(CHandler::new()),
            Language::Cpp => Box::new(CppHandler::new()),
            Language::CSharp => Box::new(CSharpHandler::new()),
            Language::Unknown => Box::new(PlaceholderHandler::new("unknown")),
        }
    }

    /// Auto-detect language from source code
    pub fn detect_language(source: &str) -> Option<Language> {
        let handlers: Vec<(Language, Box<dyn LanguageHandler>)> = vec![
            (Language::Rust, Box::new(RustHandler::new())),
            (Language::Python, Box::new(PythonHandler::new())),
            (Language::TypeScript, Box::new(TypeScriptHandler::new())),
            (Language::JavaScript, Box::new(JavaScriptHandler::new())),
            (Language::Go, Box::new(GoHandler::new())),
            (Language::Java, Box::new(JavaHandler::new())),
            (Language::C, Box::new(CHandler::new())),
            (Language::Cpp, Box::new(CppHandler::new())),
            (Language::CSharp, Box::new(CSharpHandler::new())),
        ];

        for (lang, handler) in handlers {
            if handler.detect_language(source) {
                return Some(lang);
            }
        }

        None
    }
}

/// Rust language handler
pub struct RustHandler {
    queries: LanguageQueries,
}

impl Default for RustHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl RustHandler {
    pub fn new() -> Self {
        Self {
            queries: LanguageQueries {
                functions: r#"
                    (function_item
                        (visibility_modifier)? @visibility
                        "fn" @keyword
                        name: (identifier) @name
                        parameters: (parameters) @params
                        return_type: (type_identifier)? @return_type
                        body: (block)? @body
                    ) @function
                "#
                .to_string(),
                structs: r#"
                    (struct_item
                        (visibility_modifier)? @visibility
                        "struct" @keyword
                        name: (type_identifier) @name
                        (generic_parameters)? @generics
                        body: (field_declaration_list)? @fields
                    ) @struct
                "#
                .to_string(),
                classes: "".to_string(),    // Rust doesn't have classes
                interfaces: "".to_string(), // Rust uses traits
                traits: r#"
                    (trait_item
                        (visibility_modifier)? @visibility
                        "trait" @keyword
                        name: (type_identifier) @name
                        (generic_parameters)? @generics
                        body: (declaration_list) @body
                    ) @trait
                "#
                .to_string(),
                enums: r#"
                    (enum_item
                        (visibility_modifier)? @visibility
                        "enum" @keyword
                        name: (type_identifier) @name
                        (generic_parameters)? @generics
                        body: (enum_variant_list) @variants
                    ) @enum
                "#
                .to_string(),
                types: r#"
                    (type_item
                        (visibility_modifier)? @visibility
                        "type" @keyword
                        name: (type_identifier) @name
                        (generic_parameters)? @generics
                        type: (_) @type
                    ) @type_alias
                "#
                .to_string(),
                constants: r#"
                    (const_item
                        (visibility_modifier)? @visibility
                        "const" @keyword
                        name: (identifier) @name
                        type: (type_identifier) @type
                        value: (_) @value
                    ) @constant
                "#
                .to_string(),
                variables: r#"
                    (let_declaration
                        "let" @keyword
                        (mutable_specifier)? @mut
                        pattern: (identifier) @name
                        type: (type_identifier)? @type
                        value: (_)? @value
                    ) @variable
                "#
                .to_string(),
                imports: r#"
                    (use_declaration
                        "use" @keyword
                        argument: (_) @path
                    ) @import
                "#
                .to_string(),
                exports: "".to_string(), // Rust exports are handled via visibility
                comments: r#"
                    [
                        (line_comment)
                        (block_comment)
                    ] @comment
                "#
                .to_string(),
            },
        }
    }
}

impl LanguageHandler for RustHandler {
    fn language(&self) -> tree_sitter::Language {
        tree_sitter_rust::LANGUAGE.into()
    }

    fn queries(&self) -> &LanguageQueries {
        &self.queries
    }

    fn extract_elements<'a>(
        &'a self,
        tree: &'a Tree,
        source: &'a str,
    ) -> Result<Vec<ExtractedElement<'a>>, CompactionError> {
        let mut elements = Vec::new();
        let root_node = tree.root_node();

        // Extract functions
        self.extract_functions(root_node, source, &mut elements)?;

        // Extract structs
        self.extract_structs(root_node, source, &mut elements)?;

        // Extract traits
        self.extract_traits(root_node, source, &mut elements)?;

        // Extract enums
        self.extract_enums(root_node, source, &mut elements)?;

        // Extract type aliases
        self.extract_types(root_node, source, &mut elements)?;

        // Extract constants
        self.extract_constants(root_node, source, &mut elements)?;

        // Extract imports
        self.extract_imports(root_node, source, &mut elements)?;

        Ok(elements)
    }

    fn detect_language(&self, source: &str) -> bool {
        // Rust-specific patterns
        source.contains("fn ")
            || source.contains("impl ")
            || source.contains("trait ")
            || source.contains("struct ")
            || source.contains("enum ")
            || source.contains("use ")
            || source.contains("mod ")
            || source.contains("pub ")
    }

    fn format_element(&self, element: &ExtractedElement<'_>, signatures_only: bool) -> String {
        match &element.metadata {
            ElementMetadata::Function(func) => {
                if signatures_only {
                    self.format_function_signature(element, func)
                } else {
                    element.source.to_string()
                }
            }
            ElementMetadata::Struct(_) => element.source.to_string(),
            ElementMetadata::Trait(_) => element.source.to_string(),
            ElementMetadata::Enum(_) => element.source.to_string(),
            ElementMetadata::Type(_) => element.source.to_string(),
            ElementMetadata::Constant(_) => element.source.to_string(),
            ElementMetadata::Import(_) => element.source.to_string(),
            _ => element.source.to_string(),
        }
    }
}

impl RustHandler {
    fn extract_functions<'a>(
        &'a self,
        node: Node<'a>,
        source: &'a str,
        elements: &mut Vec<ExtractedElement<'a>>,
    ) -> Result<(), CompactionError> {
        let mut cursor = node.walk();

        if node.kind() == "function_item" {
            let name = self.extract_function_name(node, source)?;
            let visibility = self.extract_visibility(node);
            let signature = self.extract_function_signature(node, source)?;
            let location = self.node_to_location(node);
            let documentation = self.extract_documentation(node, source);
            let source_text = self.node_text(node, source);

            elements.push(ExtractedElement {
                element_type: ElementType::Function,
                name: name.into(),
                source: source_text.into(),
                visibility,
                documentation: documentation.map(Into::into),
                location,
                metadata: ElementMetadata::Function(signature),
            });
        }

        // Recursively check children
        if cursor.goto_first_child() {
            loop {
                self.extract_functions(cursor.node(), source, elements)?;
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(())
    }

    fn extract_structs<'a>(
        &'a self,
        node: Node<'a>,
        source: &'a str,
        elements: &mut Vec<ExtractedElement<'a>>,
    ) -> Result<(), CompactionError> {
        let mut cursor = node.walk();

        if node.kind() == "struct_item" {
            let name = self.extract_struct_name(node, source)?;
            let visibility = self.extract_visibility(node);
            let struct_def = self.extract_struct_definition(node, source)?;
            let location = self.node_to_location(node);
            let documentation = self.extract_documentation(node, source);
            let source_text = self.node_text(node, source);

            elements.push(ExtractedElement {
                element_type: ElementType::Struct,
                name: name.into(),
                source: source_text.into(),
                visibility,
                documentation: documentation.map(Into::into),
                location,
                metadata: ElementMetadata::Struct(struct_def),
            });
        }

        // Recursively check children
        if cursor.goto_first_child() {
            loop {
                self.extract_structs(cursor.node(), source, elements)?;
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(())
    }

    fn extract_traits<'a>(
        &'a self,
        node: Node<'a>,
        source: &'a str,
        elements: &mut Vec<ExtractedElement<'a>>,
    ) -> Result<(), CompactionError> {
        let mut cursor = node.walk();

        if node.kind() == "trait_item" {
            let name = self.extract_trait_name(node, source)?;
            let visibility = self.extract_visibility(node);
            let trait_def = self.extract_trait_definition(node, source)?;
            let location = self.node_to_location(node);
            let documentation = self.extract_documentation(node, source);
            let source_text = self.node_text(node, source);

            elements.push(ExtractedElement {
                element_type: ElementType::Trait,
                name: name.into(),
                source: source_text.into(),
                visibility,
                documentation: documentation.map(Into::into),
                location,
                metadata: ElementMetadata::Trait(trait_def),
            });
        }

        // Recursively check children
        if cursor.goto_first_child() {
            loop {
                self.extract_traits(cursor.node(), source, elements)?;
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(())
    }

    fn extract_enums<'a>(
        &'a self,
        node: Node<'a>,
        source: &'a str,
        elements: &mut Vec<ExtractedElement<'a>>,
    ) -> Result<(), CompactionError> {
        let mut cursor = node.walk();

        if node.kind() == "enum_item" {
            let name = self.extract_enum_name(node, source)?;
            let visibility = self.extract_visibility(node);
            let enum_def = self.extract_enum_definition(node, source)?;
            let location = self.node_to_location(node);
            let documentation = self.extract_documentation(node, source);
            let source_text = self.node_text(node, source);

            elements.push(ExtractedElement {
                element_type: ElementType::Enum,
                name: name.into(),
                source: source_text.into(),
                visibility,
                documentation: documentation.map(Into::into),
                location,
                metadata: ElementMetadata::Enum(enum_def),
            });
        }

        // Recursively check children
        if cursor.goto_first_child() {
            loop {
                self.extract_enums(cursor.node(), source, elements)?;
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(())
    }

    fn extract_types<'a>(
        &'a self,
        node: Node<'a>,
        source: &'a str,
        elements: &mut Vec<ExtractedElement<'a>>,
    ) -> Result<(), CompactionError> {
        let mut cursor = node.walk();

        if node.kind() == "type_item" {
            let name = self.extract_type_name(node, source)?;
            let visibility = self.extract_visibility(node);
            let type_def = self.extract_type_definition(node, source)?;
            let location = self.node_to_location(node);
            let documentation = self.extract_documentation(node, source);
            let source_text = self.node_text(node, source);

            elements.push(ExtractedElement {
                element_type: ElementType::Type,
                name: name.into(),
                source: source_text.into(),
                visibility,
                documentation: documentation.map(Into::into),
                location,
                metadata: ElementMetadata::Type(type_def),
            });
        }

        // Recursively check children
        if cursor.goto_first_child() {
            loop {
                self.extract_types(cursor.node(), source, elements)?;
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(())
    }

    fn extract_constants<'a>(
        &'a self,
        node: Node<'a>,
        source: &'a str,
        elements: &mut Vec<ExtractedElement<'a>>,
    ) -> Result<(), CompactionError> {
        let mut cursor = node.walk();

        if node.kind() == "const_item" {
            let name = self.extract_const_name(node, source)?;
            let visibility = self.extract_visibility(node);
            let const_def = self.extract_const_definition(node, source)?;
            let location = self.node_to_location(node);
            let documentation = self.extract_documentation(node, source);
            let source_text = self.node_text(node, source);

            elements.push(ExtractedElement {
                element_type: ElementType::Constant,
                name: name.into(),
                source: source_text.into(),
                visibility,
                documentation: documentation.map(Into::into),
                location,
                metadata: ElementMetadata::Constant(const_def),
            });
        }

        // Recursively check children
        if cursor.goto_first_child() {
            loop {
                self.extract_constants(cursor.node(), source, elements)?;
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(())
    }

    fn extract_imports<'a>(
        &'a self,
        node: Node<'a>,
        source: &'a str,
        elements: &mut Vec<ExtractedElement<'a>>,
    ) -> Result<(), CompactionError> {
        let mut cursor = node.walk();

        if node.kind() == "use_declaration" {
            let path = self.extract_use_path(node, source)?;
            let import_stmt = self.extract_import_statement(node, source)?;
            let location = self.node_to_location(node);
            let source_text = self.node_text(node, source);

            elements.push(ExtractedElement {
                element_type: ElementType::Import,
                name: path.into(),
                source: source_text.into(),
                visibility: Visibility::Private, // use declarations are typically private
                documentation: None,
                location,
                metadata: ElementMetadata::Import(import_stmt),
            });
        }

        // Recursively check children
        if cursor.goto_first_child() {
            loop {
                self.extract_imports(cursor.node(), source, elements)?;
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(())
    }

    // Helper methods for Rust-specific extraction
    fn extract_function_name(
        &self,
        node: Node<'_>,
        source: &str,
    ) -> Result<String, CompactionError> {
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                if cursor.node().kind() == "identifier" {
                    return Ok(self.node_text(cursor.node(), source));
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        Err(CompactionError::ExtractionError {
            element_type: "function name".to_string(),
            reason: "No identifier found".to_string(),
        })
    }

    fn extract_function_signature(
        &self,
        node: Node<'_>,
        source: &str,
    ) -> Result<FunctionSignature<'_>, CompactionError> {
        let mut parameters = Vec::new();
        let mut return_type = None;
        let generic_params = Vec::new(); // TODO: Extract generic parameters
        let is_async = false; // TODO: Check for async keyword
        let is_unsafe = false; // TODO: Check for unsafe keyword
        let is_const = false; // TODO: Check for const keyword

        // Extract parameters
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                if cursor.node().kind() == "parameters" {
                    parameters = self.extract_parameters(cursor.node(), source)?;
                } else if cursor.node().kind() == "type_identifier" {
                    return_type = Some(self.node_text(cursor.node(), source).into());
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(FunctionSignature {
            parameters,
            return_type,
            generic_params,
            is_async,
            is_unsafe,
            is_const,
        })
    }

    fn extract_parameters(
        &self,
        node: Node<'_>,
        source: &str,
    ) -> Result<Vec<Parameter<'_>>, CompactionError> {
        let mut parameters = Vec::new();
        let mut cursor = node.walk();

        if cursor.goto_first_child() {
            loop {
                if cursor.node().kind() == "parameter" {
                    let param = self.extract_parameter(cursor.node(), source)?;
                    parameters.push(param);
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(parameters)
    }

    fn extract_parameter(
        &self,
        node: Node<'_>,
        source: &str,
    ) -> Result<Parameter<'_>, CompactionError> {
        let mut name = None;
        let mut param_type = None;
        let default_value = None; // Rust doesn't have default parameters
        let is_optional = false;
        let is_variadic = false; // TODO: Check for variadic parameters

        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                match cursor.node().kind() {
                    "identifier" => {
                        name = Some(self.node_text(cursor.node(), source).into());
                    }
                    "type_identifier" => {
                        param_type = Some(self.node_text(cursor.node(), source).into());
                    }
                    _ => {}
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        let param_name = name.unwrap_or_else(|| "self".into());

        Ok(Parameter {
            name: param_name,
            param_type,
            default_value,
            is_optional,
            is_variadic,
        })
    }

    fn extract_struct_name(&self, node: Node<'_>, source: &str) -> Result<String, CompactionError> {
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                if cursor.node().kind() == "type_identifier" {
                    return Ok(self.node_text(cursor.node(), source));
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        Err(CompactionError::ExtractionError {
            element_type: "struct name".to_string(),
            reason: "No type_identifier found".to_string(),
        })
    }

    fn extract_struct_definition(
        &self,
        node: Node<'_>,
        source: &str,
    ) -> Result<StructDefinition<'_>, CompactionError> {
        let mut fields = Vec::new();
        let generic_params = Vec::new(); // TODO: Extract generic parameters
        let derives = Vec::new(); // TODO: Extract derive attributes

        // Extract fields from field_declaration_list
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                if cursor.node().kind() == "field_declaration_list" {
                    fields = self.extract_struct_fields(cursor.node(), source)?;
                    break;
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(StructDefinition {
            fields,
            generic_params,
            derives,
        })
    }

    fn extract_struct_fields(
        &self,
        node: Node<'_>,
        source: &str,
    ) -> Result<Vec<FieldDefinition<'_>>, CompactionError> {
        let mut fields = Vec::new();
        let mut cursor = node.walk();

        if cursor.goto_first_child() {
            loop {
                if cursor.node().kind() == "field_declaration" {
                    let field = self.extract_field_definition(cursor.node(), source)?;
                    fields.push(field);
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(fields)
    }

    fn extract_field_definition(
        &self,
        node: Node<'_>,
        source: &str,
    ) -> Result<FieldDefinition<'_>, CompactionError> {
        let mut name = None;
        let mut field_type = None;
        let visibility = self.extract_visibility(node);
        let documentation = self.extract_documentation(node, source);

        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                match cursor.node().kind() {
                    "field_identifier" => {
                        name = Some(self.node_text(cursor.node(), source).into());
                    }
                    "type_identifier" => {
                        field_type = Some(self.node_text(cursor.node(), source).into());
                    }
                    _ => {}
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        let field_name = name.ok_or_else(|| CompactionError::ExtractionError {
            element_type: "field name".to_string(),
            reason: "No field_identifier found".to_string(),
        })?;

        let field_type = field_type.ok_or_else(|| CompactionError::ExtractionError {
            element_type: "field type".to_string(),
            reason: "No type_identifier found".to_string(),
        })?;

        Ok(FieldDefinition {
            name: field_name,
            field_type,
            visibility,
            documentation: documentation.map(Into::into),
        })
    }

    // Placeholder implementations for other extraction methods
    fn extract_trait_name(&self, node: Node<'_>, source: &str) -> Result<String, CompactionError> {
        self.extract_struct_name(node, source) // Same logic for now
    }

    const fn extract_trait_definition(
        &self,
        _node: Node<'_>,
        _source: &str,
    ) -> Result<TraitDefinition<'_>, CompactionError> {
        Ok(TraitDefinition {
            parent_traits: Vec::new(),
            associated_types: Vec::new(),
            generic_params: Vec::new(),
        })
    }

    fn extract_enum_name(&self, node: Node<'_>, source: &str) -> Result<String, CompactionError> {
        self.extract_struct_name(node, source) // Same logic for now
    }

    const fn extract_enum_definition(
        &self,
        _node: Node<'_>,
        _source: &str,
    ) -> Result<EnumDefinition<'_>, CompactionError> {
        Ok(EnumDefinition {
            variants: Vec::new(),
            generic_params: Vec::new(),
        })
    }

    fn extract_type_name(&self, node: Node<'_>, source: &str) -> Result<String, CompactionError> {
        self.extract_struct_name(node, source) // Same logic for now
    }

    fn extract_type_definition(
        &self,
        _node: Node<'_>,
        _source: &str,
    ) -> Result<TypeDefinition<'_>, CompactionError> {
        Ok(TypeDefinition {
            underlying_type: "()".into(),
            generic_params: Vec::new(),
            constraints: Vec::new(),
        })
    }

    fn extract_const_name(&self, node: Node<'_>, source: &str) -> Result<String, CompactionError> {
        self.extract_function_name(node, source) // Same logic for now
    }

    const fn extract_const_definition(
        &self,
        _node: Node<'_>,
        _source: &str,
    ) -> Result<ConstantDefinition<'_>, CompactionError> {
        Ok(ConstantDefinition {
            const_type: None,
            value: None,
        })
    }

    fn extract_use_path(&self, node: Node<'_>, source: &str) -> Result<String, CompactionError> {
        Ok(self.node_text(node, source))
    }

    fn extract_import_statement(
        &self,
        _node: Node<'_>,
        _source: &str,
    ) -> Result<ImportStatement<'_>, CompactionError> {
        Ok(ImportStatement {
            module_path: "".into(),
            imported_items: Vec::new(),
            alias: None,
        })
    }

    fn extract_visibility(&self, node: Node<'_>) -> Visibility {
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                if cursor.node().kind() == "visibility_modifier" {
                    return Visibility::Public;
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        Visibility::Private
    }

    const fn extract_documentation(&self, _node: Node<'_>, _source: &str) -> Option<String> {
        // TODO: Extract doc comments
        None
    }

    fn node_to_location(&self, node: Node<'_>) -> SourceLocation {
        let start = node.start_position();
        let end = node.end_position();
        SourceLocation::new(
            start.row,
            start.column,
            end.row,
            end.column,
            node.start_byte(),
            node.end_byte(),
        )
    }

    fn node_text(&self, node: Node<'_>, source: &str) -> String {
        source[node.start_byte()..node.end_byte()].to_string()
    }

    fn format_function_signature(
        &self,
        element: &ExtractedElement<'_>,
        func: &FunctionSignature<'_>,
    ) -> String {
        let vis = match element.visibility {
            Visibility::Public => "pub ",
            _ => "",
        };

        let params: Vec<String> = func
            .parameters
            .iter()
            .map(|p| {
                if let Some(ref param_type) = p.param_type {
                    format!("{}: {}", p.name, param_type)
                } else {
                    p.name.to_string()
                }
            })
            .collect();

        let return_part = func
            .return_type
            .as_ref()
            .map(|t| format!(" -> {}", t))
            .unwrap_or_default();

        format!(
            "{}fn {}({}){};",
            vis,
            element.name,
            params.join(", "),
            return_part
        )
    }
}

// Placeholder implementations for other language handlers
macro_rules! impl_placeholder_handler {
    ($handler:ident, $lang_func:ident, $detect_patterns:expr) => {
        pub struct $handler;

        impl $handler {
            pub fn new() -> Self {
                Self
            }
        }

        impl LanguageHandler for $handler {
            fn language(&self) -> tree_sitter::Language {
                // Placeholder - would use actual tree-sitter language
                tree_sitter_rust::LANGUAGE.into()
            }

            fn queries(&self) -> &LanguageQueries {
                // Placeholder - would implement language-specific queries
                static QUERIES: LanguageQueries = LanguageQueries {
                    functions: String::new(),
                    structs: String::new(),
                    classes: String::new(),
                    interfaces: String::new(),
                    traits: String::new(),
                    enums: String::new(),
                    types: String::new(),
                    constants: String::new(),
                    variables: String::new(),
                    imports: String::new(),
                    exports: String::new(),
                    comments: String::new(),
                };
                &QUERIES
            }

            fn extract_elements<'a>(
                &self,
                _tree: &'a Tree,
                _source: &'a str,
            ) -> Result<Vec<ExtractedElement<'a>>, CompactionError> {
                // Placeholder - would implement language-specific extraction
                Ok(Vec::new())
            }

            fn detect_language(&self, source: &str) -> bool {
                let patterns: &[&str] = $detect_patterns;
                patterns.iter().any(|pattern| source.contains(pattern))
            }

            fn format_element(
                &self,
                element: &ExtractedElement<'_>,
                _signatures_only: bool,
            ) -> String {
                element.source.to_string()
            }
        }
    };
}

// Placeholder implementations for other language handlers
// These will be fully implemented as separate features
pub struct PythonHandler;
impl Default for PythonHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonHandler {
    pub const fn new() -> Self {
        Self
    }
}
impl LanguageHandler for PythonHandler {
    fn language(&self) -> tree_sitter::Language {
        tree_sitter_python::LANGUAGE.into()
    }
    fn queries(&self) -> &LanguageQueries {
        static QUERIES: LanguageQueries = LanguageQueries {
            functions: String::new(),
            structs: String::new(),
            classes: String::new(),
            interfaces: String::new(),
            traits: String::new(),
            enums: String::new(),
            types: String::new(),
            constants: String::new(),
            variables: String::new(),
            imports: String::new(),
            exports: String::new(),
            comments: String::new(),
        };
        &QUERIES
    }
    fn extract_elements<'a>(
        &self,
        _tree: &'a Tree,
        _source: &'a str,
    ) -> Result<Vec<ExtractedElement<'a>>, CompactionError> {
        Ok(Vec::new())
    }
    fn detect_language(&self, source: &str) -> bool {
        source.contains("def ")
            || source.contains("class ")
            || source.contains("import ")
            || source.contains("from ")
    }
    fn format_element(&self, element: &ExtractedElement<'_>, _signatures_only: bool) -> String {
        element.source.to_string()
    }
}

pub struct JavaScriptHandler;
impl Default for JavaScriptHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaScriptHandler {
    pub const fn new() -> Self {
        Self
    }
}
impl LanguageHandler for JavaScriptHandler {
    fn language(&self) -> tree_sitter::Language {
        tree_sitter_javascript::LANGUAGE.into()
    }
    fn queries(&self) -> &LanguageQueries {
        static QUERIES: LanguageQueries = LanguageQueries {
            functions: String::new(),
            structs: String::new(),
            classes: String::new(),
            interfaces: String::new(),
            traits: String::new(),
            enums: String::new(),
            types: String::new(),
            constants: String::new(),
            variables: String::new(),
            imports: String::new(),
            exports: String::new(),
            comments: String::new(),
        };
        &QUERIES
    }
    fn extract_elements<'a>(
        &self,
        _tree: &'a Tree,
        _source: &'a str,
    ) -> Result<Vec<ExtractedElement<'a>>, CompactionError> {
        Ok(Vec::new())
    }
    fn detect_language(&self, source: &str) -> bool {
        source.contains("function ")
            || source.contains("const ")
            || source.contains("let ")
            || source.contains("var ")
    }
    fn format_element(&self, element: &ExtractedElement<'_>, _signatures_only: bool) -> String {
        element.source.to_string()
    }
}

pub struct TypeScriptHandler;
impl Default for TypeScriptHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeScriptHandler {
    pub const fn new() -> Self {
        Self
    }
}
impl LanguageHandler for TypeScriptHandler {
    fn language(&self) -> tree_sitter::Language {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }
    fn queries(&self) -> &LanguageQueries {
        static QUERIES: LanguageQueries = LanguageQueries {
            functions: String::new(),
            structs: String::new(),
            classes: String::new(),
            interfaces: String::new(),
            traits: String::new(),
            enums: String::new(),
            types: String::new(),
            constants: String::new(),
            variables: String::new(),
            imports: String::new(),
            exports: String::new(),
            comments: String::new(),
        };
        &QUERIES
    }
    fn extract_elements<'a>(
        &self,
        _tree: &'a Tree,
        _source: &'a str,
    ) -> Result<Vec<ExtractedElement<'a>>, CompactionError> {
        Ok(Vec::new())
    }
    fn detect_language(&self, source: &str) -> bool {
        source.contains("interface ")
            || source.contains("type ")
            || source.contains(": string")
            || source.contains(": number")
    }
    fn format_element(&self, element: &ExtractedElement<'_>, _signatures_only: bool) -> String {
        element.source.to_string()
    }
}

pub struct GoHandler;
impl Default for GoHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl GoHandler {
    pub const fn new() -> Self {
        Self
    }
}
impl LanguageHandler for GoHandler {
    fn language(&self) -> tree_sitter::Language {
        tree_sitter_go::LANGUAGE.into()
    }
    fn queries(&self) -> &LanguageQueries {
        static QUERIES: LanguageQueries = LanguageQueries {
            functions: String::new(),
            structs: String::new(),
            classes: String::new(),
            interfaces: String::new(),
            traits: String::new(),
            enums: String::new(),
            types: String::new(),
            constants: String::new(),
            variables: String::new(),
            imports: String::new(),
            exports: String::new(),
            comments: String::new(),
        };
        &QUERIES
    }
    fn extract_elements<'a>(
        &self,
        _tree: &'a Tree,
        _source: &'a str,
    ) -> Result<Vec<ExtractedElement<'a>>, CompactionError> {
        Ok(Vec::new())
    }
    fn detect_language(&self, source: &str) -> bool {
        source.contains("func ") || source.contains("package ") || source.contains("type ")
    }
    fn format_element(&self, element: &ExtractedElement<'_>, _signatures_only: bool) -> String {
        element.source.to_string()
    }
}

// Simplified placeholders for remaining languages
pub struct JavaHandler;
impl Default for JavaHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaHandler {
    pub const fn new() -> Self {
        Self
    }
}
impl LanguageHandler for JavaHandler {
    fn language(&self) -> tree_sitter::Language {
        tree_sitter_java::LANGUAGE.into()
    }
    fn queries(&self) -> &LanguageQueries {
        static QUERIES: LanguageQueries = LanguageQueries {
            functions: String::new(),
            structs: String::new(),
            classes: String::new(),
            interfaces: String::new(),
            traits: String::new(),
            enums: String::new(),
            types: String::new(),
            constants: String::new(),
            variables: String::new(),
            imports: String::new(),
            exports: String::new(),
            comments: String::new(),
        };
        &QUERIES
    }
    fn extract_elements<'a>(
        &self,
        _tree: &'a Tree,
        _source: &'a str,
    ) -> Result<Vec<ExtractedElement<'a>>, CompactionError> {
        Ok(Vec::new())
    }
    fn detect_language(&self, source: &str) -> bool {
        source.contains("class ") || source.contains("public ") || source.contains("interface ")
    }
    fn format_element(&self, element: &ExtractedElement<'_>, _signatures_only: bool) -> String {
        element.source.to_string()
    }
}

pub struct CHandler;
impl Default for CHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl CHandler {
    pub const fn new() -> Self {
        Self
    }
}
impl LanguageHandler for CHandler {
    fn language(&self) -> tree_sitter::Language {
        tree_sitter_c::LANGUAGE.into()
    }
    fn queries(&self) -> &LanguageQueries {
        static QUERIES: LanguageQueries = LanguageQueries {
            functions: String::new(),
            structs: String::new(),
            classes: String::new(),
            interfaces: String::new(),
            traits: String::new(),
            enums: String::new(),
            types: String::new(),
            constants: String::new(),
            variables: String::new(),
            imports: String::new(),
            exports: String::new(),
            comments: String::new(),
        };
        &QUERIES
    }
    fn extract_elements<'a>(
        &self,
        _tree: &'a Tree,
        _source: &'a str,
    ) -> Result<Vec<ExtractedElement<'a>>, CompactionError> {
        Ok(Vec::new())
    }
    fn detect_language(&self, source: &str) -> bool {
        source.contains("#include") || source.contains("void ") || source.contains("int ")
    }
    fn format_element(&self, element: &ExtractedElement<'_>, _signatures_only: bool) -> String {
        element.source.to_string()
    }
}

pub struct CppHandler;
impl Default for CppHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl CppHandler {
    pub const fn new() -> Self {
        Self
    }
}
impl LanguageHandler for CppHandler {
    fn language(&self) -> tree_sitter::Language {
        tree_sitter_cpp::LANGUAGE.into()
    }
    fn queries(&self) -> &LanguageQueries {
        static QUERIES: LanguageQueries = LanguageQueries {
            functions: String::new(),
            structs: String::new(),
            classes: String::new(),
            interfaces: String::new(),
            traits: String::new(),
            enums: String::new(),
            types: String::new(),
            constants: String::new(),
            variables: String::new(),
            imports: String::new(),
            exports: String::new(),
            comments: String::new(),
        };
        &QUERIES
    }
    fn extract_elements<'a>(
        &self,
        _tree: &'a Tree,
        _source: &'a str,
    ) -> Result<Vec<ExtractedElement<'a>>, CompactionError> {
        Ok(Vec::new())
    }
    fn detect_language(&self, source: &str) -> bool {
        source.contains("#include") || source.contains("class ") || source.contains("namespace ")
    }
    fn format_element(&self, element: &ExtractedElement<'_>, _signatures_only: bool) -> String {
        element.source.to_string()
    }
}

pub struct CSharpHandler;
impl Default for CSharpHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl CSharpHandler {
    pub const fn new() -> Self {
        Self
    }
}
impl LanguageHandler for CSharpHandler {
    fn language(&self) -> tree_sitter::Language {
        tree_sitter_c_sharp::LANGUAGE.into()
    }
    fn queries(&self) -> &LanguageQueries {
        static QUERIES: LanguageQueries = LanguageQueries {
            functions: String::new(),
            structs: String::new(),
            classes: String::new(),
            interfaces: String::new(),
            traits: String::new(),
            enums: String::new(),
            types: String::new(),
            constants: String::new(),
            variables: String::new(),
            imports: String::new(),
            exports: String::new(),
            comments: String::new(),
        };
        &QUERIES
    }
    fn extract_elements<'a>(
        &self,
        _tree: &'a Tree,
        _source: &'a str,
    ) -> Result<Vec<ExtractedElement<'a>>, CompactionError> {
        Ok(Vec::new())
    }
    fn detect_language(&self, source: &str) -> bool {
        source.contains("class ") || source.contains("namespace ") || source.contains("using ")
    }
    fn format_element(&self, element: &ExtractedElement<'_>, _signatures_only: bool) -> String {
        element.source.to_string()
    }
}

/// Placeholder handler for unknown or unsupported languages
pub struct PlaceholderHandler {
    _name: &'static str,
}
impl PlaceholderHandler {
    pub const fn new(name: &'static str) -> Self {
        Self { _name: name }
    }
}
impl LanguageHandler for PlaceholderHandler {
    fn language(&self) -> tree_sitter::Language {
        // Use Rust parser as fallback for unknown languages
        tree_sitter_rust::LANGUAGE.into()
    }
    fn queries(&self) -> &LanguageQueries {
        static QUERIES: LanguageQueries = LanguageQueries {
            functions: String::new(),
            structs: String::new(),
            classes: String::new(),
            interfaces: String::new(),
            traits: String::new(),
            enums: String::new(),
            types: String::new(),
            constants: String::new(),
            variables: String::new(),
            imports: String::new(),
            exports: String::new(),
            comments: String::new(),
        };
        &QUERIES
    }
    fn extract_elements<'a>(
        &self,
        _tree: &'a Tree,
        _source: &'a str,
    ) -> Result<Vec<ExtractedElement<'a>>, CompactionError> {
        Ok(Vec::new())
    }
    fn detect_language(&self, _source: &str) -> bool {
        false // Placeholder never detects any language
    }
    fn format_element(&self, element: &ExtractedElement<'_>, _signatures_only: bool) -> String {
        element.source.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection() {
        assert_eq!(
            LanguageHandlerFactory::detect_language("fn main() { println!(); }"),
            Some(Language::Rust)
        );

        assert_eq!(
            LanguageHandlerFactory::detect_language("def hello(): pass"),
            Some(Language::Python)
        );

        assert_eq!(
            LanguageHandlerFactory::detect_language("function hello() {}"),
            Some(Language::JavaScript)
        );

        assert_eq!(
            LanguageHandlerFactory::detect_language("interface User { name: string; }"),
            Some(Language::TypeScript)
        );
    }

    #[test]
    fn test_rust_handler_detection() {
        let handler = RustHandler::new();

        assert!(handler.detect_language("fn main() {}"));
        assert!(handler.detect_language("struct User { name: String }"));
        assert!(handler.detect_language("impl User {}"));
        assert!(!handler.detect_language("def hello(): pass"));
    }

    #[test]
    fn test_rust_handler_queries() {
        let handler = RustHandler::new();
        let queries = handler.queries();

        assert!(!queries.functions.is_empty());
        assert!(!queries.structs.is_empty());
        assert!(!queries.traits.is_empty());
        assert!(queries.classes.is_empty()); // Rust doesn't have classes
    }
}
