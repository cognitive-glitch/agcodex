//! Core AST compaction logic and orchestration
//!
//! This module provides the main `AstCompactor` struct that orchestrates
//! the entire compaction process. It handles language detection, parser
//! management, element extraction, and output formatting.

use crate::ast_compactor::languages::LanguageHandlerFactory;
use crate::ast_compactor::types::ClassDefinition;
use crate::ast_compactor::types::CommentContent;
use crate::ast_compactor::types::CompactionError;
use crate::ast_compactor::types::CompactionMetrics;
use crate::ast_compactor::types::CompactionOptions;
use crate::ast_compactor::types::CompactionResult;
use crate::ast_compactor::types::ConstantDefinition;
use crate::ast_compactor::types::ElementMetadata;
use crate::ast_compactor::types::ElementType;
use crate::ast_compactor::types::EnumDefinition;
use crate::ast_compactor::types::EnumVariant;
use crate::ast_compactor::types::ExportStatement;
use crate::ast_compactor::types::ExtractedElement;
use crate::ast_compactor::types::FieldDefinition;
use crate::ast_compactor::types::FunctionSignature;
use crate::ast_compactor::types::ImportStatement;
use crate::ast_compactor::types::InterfaceDefinition;
use crate::ast_compactor::types::Language;
use crate::ast_compactor::types::MemoryStats;
use crate::ast_compactor::types::Parameter;
use crate::ast_compactor::types::StructDefinition;
use crate::ast_compactor::types::TraitDefinition;
use crate::ast_compactor::types::TypeDefinition;
use crate::ast_compactor::types::VariableDefinition;
use crate::ast_compactor::types::Visibility;

use std::collections::HashMap;
use std::time::Instant;
use tracing::debug;
use tracing::info;
use tracing::instrument;
use tracing::trace;
use tracing::warn;
use tree_sitter::Parser;
use tree_sitter::Tree;

/// Cache entry for parsed ASTs
#[derive(Debug, Clone)]
struct CacheEntry {
    tree: Tree,
    source_hash: u64,
    language: Language,
    timestamp: Instant,
}

/// Main AST compactor with caching and optimization
pub struct AstCompactor {
    /// Parser cache for reusing tree-sitter parsers
    parser_cache: HashMap<Language, Parser>,

    /// AST cache for avoiding re-parsing identical source
    ast_cache: HashMap<u64, CacheEntry>,

    /// Maximum cache size to prevent memory bloat
    max_cache_size: usize,

    /// Statistics tracking
    stats: CompactorStats,
}

/// Statistics for compactor operations
#[derive(Debug, Default, Clone)]
pub struct CompactorStats {
    /// Total number of compactions performed
    pub total_compactions: usize,

    /// Cache hit rate
    pub cache_hits: usize,

    /// Cache misses
    pub cache_misses: usize,

    /// Total processing time in milliseconds
    pub total_processing_time_ms: u64,

    /// Average compression ratio
    pub average_compression_ratio: f64,
}

impl Default for AstCompactor {
    fn default() -> Self {
        Self::new()
    }
}

/// Manual Debug implementation since Parser doesn't implement Debug
impl std::fmt::Debug for AstCompactor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AstCompactor")
            .field(
                "parser_cache",
                &format!(
                    "HashMap<Language, Parser> with {} entries",
                    self.parser_cache.len()
                ),
            )
            .field("ast_cache", &self.ast_cache)
            .field("max_cache_size", &self.max_cache_size)
            .field("stats", &self.stats)
            .finish()
    }
}

impl AstCompactor {
    /// Create a new AST compactor with default settings
    pub fn new() -> Self {
        Self {
            parser_cache: HashMap::new(),
            ast_cache: HashMap::new(),
            max_cache_size: 100, // Cache up to 100 parsed ASTs
            stats: CompactorStats::default(),
        }
    }

    /// Create a new AST compactor with custom cache size
    pub fn with_cache_size(cache_size: usize) -> Self {
        Self {
            parser_cache: HashMap::new(),
            ast_cache: HashMap::new(),
            max_cache_size: cache_size,
            stats: CompactorStats::default(),
        }
    }

    /// Main compaction entry point
    #[instrument(skip(self, source, options), fields(source_len = source.len()))]
    pub fn compact<'a>(
        &mut self,
        source: &'a str,
        options: &CompactionOptions,
    ) -> Result<CompactionResult<'a>, CompactionError> {
        let start_time = Instant::now();

        // Validate input
        if source.is_empty() {
            return Err(CompactionError::EmptyInput);
        }

        // Detect language if not specified
        let language = match options.language {
            Some(lang) => lang,
            None => self.detect_language(source)?,
        };

        debug!("Detected language: {}", language);

        // Parse source code into AST
        let tree = self.parse_source(source, language)?;

        // Extract elements from AST
        let elements = self.extract_elements(&tree, source, language, options)?;

        // Format output
        let compacted_code = self.format_output(&elements, options)?;

        // Convert to owned elements to avoid lifetime issues
        let owned_elements = self.convert_to_owned_elements(elements);

        // Calculate metrics
        let processing_time = start_time.elapsed();
        let processing_time_ms = processing_time.as_millis() as u64;

        let mut result = CompactionResult::new(
            compacted_code,
            source.len(),
            language,
            owned_elements,
            processing_time_ms,
        );

        // Populate detailed metrics
        result.metrics = self.calculate_metrics(&result, options);

        // Update internal statistics
        self.update_stats(&result);

        info!(
            "Compaction completed: {:.1}% reduction in {}ms",
            result.compression_percentage(),
            processing_time_ms
        );

        Ok(result)
    }

    /// Detect programming language from source code
    #[instrument(skip(self, source), fields(source_len = source.len()))]
    fn detect_language(&self, source: &str) -> Result<Language, CompactionError> {
        trace!("Detecting language from source patterns");

        LanguageHandlerFactory::detect_language(source)
            .ok_or(CompactionError::LanguageDetectionFailed)
    }

    /// Parse source code into AST with caching
    #[instrument(skip(self, source), fields(source_len = source.len(), language = %language))]
    fn parse_source(&mut self, source: &str, language: Language) -> Result<Tree, CompactionError> {
        // Calculate hash for cache lookup
        let source_hash = self.calculate_hash(source);

        // Check AST cache first
        if let Some(entry) = self.ast_cache.get(&source_hash)
            && entry.language == language && entry.source_hash == source_hash {
                debug!("AST cache hit for hash: {}", source_hash);
                self.stats.cache_hits += 1;
                return Ok(entry.tree.clone());
            }

        debug!("AST cache miss, parsing source");
        self.stats.cache_misses += 1;

        // Get or create parser for this language
        let parser = self.get_or_create_parser(language)?;

        // Parse the source
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| CompactionError::ParseError {
                message: "Tree-sitter failed to parse source".to_string(),
            })?;

        // Cache the result if not over limit
        if self.ast_cache.len() < self.max_cache_size {
            let entry = CacheEntry {
                tree: tree.clone(),
                source_hash,
                language,
                timestamp: Instant::now(),
            };
            self.ast_cache.insert(source_hash, entry);
        } else {
            // Optionally implement LRU eviction here
            warn!("AST cache full, not caching this parse result");
        }

        Ok(tree)
    }

    /// Get or create a parser for the specified language
    fn get_or_create_parser(&mut self, language: Language) -> Result<&mut Parser, CompactionError> {
        if let std::collections::hash_map::Entry::Vacant(e) = self.parser_cache.entry(language) {
            debug!("Creating new parser for language: {}", language);

            let handler = LanguageHandlerFactory::create_handler(language);
            let mut parser = Parser::new();

            parser.set_language(&handler.language()).map_err(|e| {
                CompactionError::ParserInitError {
                    language: language.to_string(),
                    error: e.to_string(),
                }
            })?;

            e.insert(parser);
        }

        Ok(self.parser_cache.get_mut(&language).unwrap())
    }

    /// Extract elements from parsed AST
    #[instrument(skip(self, tree, source, options), fields(language = %language))]
    fn extract_elements<'a>(
        &self,
        tree: &'a Tree,
        source: &'a str,
        language: Language,
        options: &CompactionOptions,
    ) -> Result<Vec<ExtractedElement<'a>>, CompactionError> {
        let handler = LanguageHandlerFactory::create_handler(language);

        let mut elements = handler.extract_elements(tree, source)?;

        // Apply filtering based on options
        self.filter_elements(&mut elements, options);

        // Sort elements by importance and location
        self.sort_elements(&mut elements);

        debug!("Extracted {} elements", elements.len());

        // Convert to owned elements to avoid lifetime issues
        let owned_elements = self.convert_to_owned_elements(elements);

        Ok(owned_elements)
    }

    /// Convert borrowed elements to owned elements to avoid lifetime issues
    fn convert_to_owned_elements(
        &self,
        elements: Vec<ExtractedElement<'_>>,
    ) -> Vec<ExtractedElement<'static>> {
        elements
            .into_iter()
            .map(|element| ExtractedElement {
                element_type: element.element_type,
                name: std::borrow::Cow::Owned(element.name.into_owned()),
                source: std::borrow::Cow::Owned(element.source.into_owned()),
                visibility: element.visibility,
                documentation: element
                    .documentation
                    .map(|doc| std::borrow::Cow::Owned(doc.into_owned())),
                location: element.location,
                metadata: self.convert_metadata_to_owned(element.metadata),
            })
            .collect()
    }

    /// Convert metadata to owned version
    fn convert_metadata_to_owned(&self, metadata: ElementMetadata<'_>) -> ElementMetadata<'static> {
        match metadata {
            ElementMetadata::Function(func) => ElementMetadata::Function(FunctionSignature {
                parameters: func
                    .parameters
                    .into_iter()
                    .map(|param| Parameter {
                        name: std::borrow::Cow::Owned(param.name.into_owned()),
                        param_type: param
                            .param_type
                            .map(|t| std::borrow::Cow::Owned(t.into_owned())),
                        default_value: param
                            .default_value
                            .map(|v| std::borrow::Cow::Owned(v.into_owned())),
                        is_optional: param.is_optional,
                        is_variadic: param.is_variadic,
                    })
                    .collect(),
                return_type: func
                    .return_type
                    .map(|t| std::borrow::Cow::Owned(t.into_owned())),
                generic_params: func
                    .generic_params
                    .into_iter()
                    .map(|p| std::borrow::Cow::Owned(p.into_owned()))
                    .collect(),
                is_async: func.is_async,
                is_unsafe: func.is_unsafe,
                is_const: func.is_const,
            }),
            ElementMetadata::Type(type_def) => ElementMetadata::Type(TypeDefinition {
                underlying_type: std::borrow::Cow::Owned(type_def.underlying_type.into_owned()),
                generic_params: type_def
                    .generic_params
                    .into_iter()
                    .map(|p| std::borrow::Cow::Owned(p.into_owned()))
                    .collect(),
                constraints: type_def
                    .constraints
                    .into_iter()
                    .map(|c| std::borrow::Cow::Owned(c.into_owned()))
                    .collect(),
            }),
            ElementMetadata::Struct(struct_def) => ElementMetadata::Struct(StructDefinition {
                fields: struct_def
                    .fields
                    .into_iter()
                    .map(|field| FieldDefinition {
                        name: std::borrow::Cow::Owned(field.name.into_owned()),
                        field_type: std::borrow::Cow::Owned(field.field_type.into_owned()),
                        visibility: field.visibility,
                        documentation: field
                            .documentation
                            .map(|d| std::borrow::Cow::Owned(d.into_owned())),
                    })
                    .collect(),
                generic_params: struct_def
                    .generic_params
                    .into_iter()
                    .map(|p| std::borrow::Cow::Owned(p.into_owned()))
                    .collect(),
                derives: struct_def
                    .derives
                    .into_iter()
                    .map(|d| std::borrow::Cow::Owned(d.into_owned()))
                    .collect(),
            }),
            ElementMetadata::Constant(const_def) => ElementMetadata::Constant(ConstantDefinition {
                value: const_def
                    .value
                    .map(|v| std::borrow::Cow::Owned(v.into_owned())),
                const_type: const_def
                    .const_type
                    .map(|t| std::borrow::Cow::Owned(t.into_owned())),
            }),
            ElementMetadata::Import(import) => ElementMetadata::Import(ImportStatement {
                module_path: std::borrow::Cow::Owned(import.module_path.into_owned()),
                imported_items: import
                    .imported_items
                    .into_iter()
                    .map(|item| std::borrow::Cow::Owned(item.into_owned()))
                    .collect(),
                alias: import
                    .alias
                    .map(|a| std::borrow::Cow::Owned(a.into_owned())),
            }),
            ElementMetadata::Class(class_def) => ElementMetadata::Class(ClassDefinition {
                parent_class: class_def
                    .parent_class
                    .map(|e| std::borrow::Cow::Owned(e.into_owned())),
                interfaces: class_def
                    .interfaces
                    .into_iter()
                    .map(|i| std::borrow::Cow::Owned(i.into_owned()))
                    .collect(),
                generic_params: class_def
                    .generic_params
                    .into_iter()
                    .map(|p| std::borrow::Cow::Owned(p.into_owned()))
                    .collect(),
                is_abstract: class_def.is_abstract,
                is_final: class_def.is_final,
            }),
            ElementMetadata::Interface(interface_def) => {
                ElementMetadata::Interface(InterfaceDefinition {
                    parent_interfaces: interface_def
                        .parent_interfaces
                        .into_iter()
                        .map(|e| std::borrow::Cow::Owned(e.into_owned()))
                        .collect(),
                    generic_params: interface_def
                        .generic_params
                        .into_iter()
                        .map(|p| std::borrow::Cow::Owned(p.into_owned()))
                        .collect(),
                })
            }
            ElementMetadata::Trait(trait_def) => ElementMetadata::Trait(TraitDefinition {
                parent_traits: trait_def
                    .parent_traits
                    .into_iter()
                    .map(|e| std::borrow::Cow::Owned(e.into_owned()))
                    .collect(),
                associated_types: trait_def
                    .associated_types
                    .into_iter()
                    .map(|t| std::borrow::Cow::Owned(t.into_owned()))
                    .collect(),
                generic_params: trait_def
                    .generic_params
                    .into_iter()
                    .map(|p| std::borrow::Cow::Owned(p.into_owned()))
                    .collect(),
            }),
            ElementMetadata::Enum(enum_def) => ElementMetadata::Enum(EnumDefinition {
                variants: enum_def
                    .variants
                    .into_iter()
                    .map(|v| EnumVariant {
                        name: std::borrow::Cow::Owned(v.name.into_owned()),
                        discriminant: v
                            .discriminant
                            .map(|d| std::borrow::Cow::Owned(d.into_owned())),
                        fields: v
                            .fields
                            .into_iter()
                            .map(|field| FieldDefinition {
                                name: std::borrow::Cow::Owned(field.name.into_owned()),
                                field_type: std::borrow::Cow::Owned(field.field_type.into_owned()),
                                visibility: field.visibility,
                                documentation: field
                                    .documentation
                                    .map(|d| std::borrow::Cow::Owned(d.into_owned())),
                            })
                            .collect(),
                    })
                    .collect(),
                generic_params: enum_def
                    .generic_params
                    .into_iter()
                    .map(|p| std::borrow::Cow::Owned(p.into_owned()))
                    .collect(),
            }),
            ElementMetadata::Variable(var_def) => ElementMetadata::Variable(VariableDefinition {
                var_type: var_def
                    .var_type
                    .map(|t| std::borrow::Cow::Owned(t.into_owned())),
                initial_value: var_def
                    .initial_value
                    .map(|v| std::borrow::Cow::Owned(v.into_owned())),
                is_mutable: var_def.is_mutable,
            }),
            ElementMetadata::Export(export_def) => ElementMetadata::Export(ExportStatement {
                exported_items: export_def
                    .exported_items
                    .into_iter()
                    .map(|item| std::borrow::Cow::Owned(item.into_owned()))
                    .collect(),
                module_path: export_def
                    .module_path
                    .map(|s| std::borrow::Cow::Owned(s.into_owned())),
            }),
            ElementMetadata::Comment(comment) => ElementMetadata::Comment(CommentContent {
                content: std::borrow::Cow::Owned(comment.content.into_owned()),
                is_documentation: comment.is_documentation,
                comment_type: comment.comment_type,
            }),
        }
    }

    /// Filter elements based on compaction options
    fn filter_elements(
        &self,
        elements: &mut Vec<ExtractedElement<'_>>,
        options: &CompactionOptions,
    ) {
        elements.retain(|element| {
            // Filter by visibility
            if !options.include_private && element.visibility == Visibility::Private {
                return false;
            }

            // Apply custom filters
            for filter in &options.element_filters {
                if element.element_type == filter.element_type {
                    // Check if name matches pattern (simplified - would use regex in real implementation)
                    if element.name.contains(&filter.name_pattern) {
                        return filter.include;
                    }
                }
            }

            true
        });
    }

    /// Sort elements by importance and source location
    fn sort_elements(&self, elements: &mut Vec<ExtractedElement<'_>>) {
        elements.sort_by(|a, b| {
            // First sort by importance (based on element type)
            let importance_a = self.element_importance(a.element_type);
            let importance_b = self.element_importance(b.element_type);

            match importance_b.cmp(&importance_a) {
                std::cmp::Ordering::Equal => {
                    // Then by source location
                    a.location.start_line.cmp(&b.location.start_line)
                }
                other => other,
            }
        });
    }

    /// Get importance score for element type (higher = more important)
    const fn element_importance(&self, element_type: ElementType) -> u8 {
        match element_type {
            ElementType::Interface | ElementType::Trait => 100,
            ElementType::Struct | ElementType::Class => 90,
            ElementType::Enum | ElementType::Type => 80,
            ElementType::Function | ElementType::Method => 70,
            ElementType::Constant => 60,
            ElementType::Variable => 50,
            ElementType::Import | ElementType::Export => 40,
            ElementType::Comment => 10,
        }
    }

    /// Format extracted elements into compacted output
    #[instrument(skip(self, elements, options), fields(element_count = elements.len()))]
    fn format_output(
        &self,
        elements: &[ExtractedElement<'_>],
        options: &CompactionOptions,
    ) -> Result<String, CompactionError> {
        let mut output = Vec::new();

        // Group elements by type for better organization
        let mut groups: HashMap<ElementType, Vec<&ExtractedElement<'_>>> = HashMap::new();

        for element in elements {
            groups
                .entry(element.element_type)
                .or_default()
                .push(element);
        }

        // Output in order of importance
        let ordered_types = [
            ElementType::Import,
            ElementType::Export,
            ElementType::Interface,
            ElementType::Trait,
            ElementType::Struct,
            ElementType::Class,
            ElementType::Enum,
            ElementType::Type,
            ElementType::Constant,
            ElementType::Function,
            ElementType::Method,
            ElementType::Variable,
        ];

        for element_type in &ordered_types {
            if let Some(group) = groups.get(element_type)
                && !group.is_empty() {
                    // Add section header
                    output.push(format!("// {} definitions", element_type));
                    output.push(String::new());

                    for element in group {
                        // Add documentation if available and requested
                        if options.preserve_docs
                            && let Some(ref doc) = element.documentation {
                                output.push(doc.to_string());
                            }

                        // Format the element based on language and options
                        let language = groups
                            .keys()
                            .next()
                            .map(|_| Language::Rust) // Default to Rust for now
                            .unwrap_or(Language::Rust);

                        let handler = LanguageHandlerFactory::create_handler(language);
                        let formatted =
                            handler.format_element(element, options.preserve_signatures_only);
                        output.push(formatted);
                        output.push(String::new()); // Empty line between elements
                    }
                }
        }

        // Remove trailing empty lines
        while output.last() == Some(&String::new()) {
            output.pop();
        }

        Ok(output.join("\n"))
    }

    /// Calculate comprehensive metrics for the compaction result
    fn calculate_metrics(
        &self,
        result: &CompactionResult<'_>,
        _options: &CompactionOptions,
    ) -> CompactionMetrics {
        let functions_count = result.elements_of_type(ElementType::Function).len()
            + result.elements_of_type(ElementType::Method).len();

        let types_count = result.elements_of_type(ElementType::Struct).len()
            + result.elements_of_type(ElementType::Class).len()
            + result.elements_of_type(ElementType::Interface).len()
            + result.elements_of_type(ElementType::Trait).len()
            + result.elements_of_type(ElementType::Enum).len()
            + result.elements_of_type(ElementType::Type).len();

        let docs_count = result
            .elements
            .iter()
            .filter(|e| e.documentation.is_some())
            .count();

        // Calculate AST depth (simplified)
        let ast_depth = result
            .elements
            .iter()
            .map(|e| e.location.end_line - e.location.start_line)
            .max()
            .unwrap_or(0);

        CompactionMetrics {
            functions_count,
            types_count,
            docs_count,
            ast_depth,
            nodes_processed: result.elements.len(),
            memory_stats: MemoryStats::default(), // Would implement real memory tracking
        }
    }

    /// Update internal statistics
    fn update_stats(&mut self, result: &CompactionResult<'_>) {
        self.stats.total_compactions += 1;
        self.stats.total_processing_time_ms += result.processing_time_ms;

        // Update rolling average compression ratio
        let new_avg = (self.stats.average_compression_ratio
            * (self.stats.total_compactions - 1) as f64
            + result.compression_ratio)
            / self.stats.total_compactions as f64;
        self.stats.average_compression_ratio = new_avg;
    }

    /// Calculate simple hash of source code for caching
    fn calculate_hash(&self, source: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hash;
        use std::hash::Hasher;

        let mut hasher = DefaultHasher::new();
        source.hash(&mut hasher);
        hasher.finish()
    }

    /// Get current compactor statistics
    pub const fn stats(&self) -> &CompactorStats {
        &self.stats
    }

    /// Get cache hit rate as percentage
    pub fn cache_hit_rate(&self) -> f64 {
        let total_requests = self.stats.cache_hits + self.stats.cache_misses;
        if total_requests == 0 {
            0.0
        } else {
            (self.stats.cache_hits as f64 / total_requests as f64) * 100.0
        }
    }

    /// Clear all caches and reset statistics
    pub fn clear_caches(&mut self) {
        self.parser_cache.clear();
        self.ast_cache.clear();
        self.stats = CompactorStats::default();
        info!("Cleared AST compactor caches and reset statistics");
    }

    /// Get memory usage information
    pub fn memory_info(&self) -> (usize, usize) {
        let parser_count = self.parser_cache.len();
        let ast_cache_count = self.ast_cache.len();
        (parser_count, ast_cache_count)
    }
}

// Additional utility implementations
impl CompactorStats {
    /// Get average processing time per compaction
    pub fn average_processing_time_ms(&self) -> f64 {
        if self.total_compactions == 0 {
            0.0
        } else {
            self.total_processing_time_ms as f64 / self.total_compactions as f64
        }
    }

    /// Get compactions per second based on total time
    pub fn compactions_per_second(&self) -> f64 {
        if self.total_processing_time_ms == 0 {
            0.0
        } else {
            (self.total_compactions as f64 * 1000.0) / self.total_processing_time_ms as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast_compactor::types::Language;

    #[test]
    fn test_new_compactor() {
        let compactor = AstCompactor::new();
        assert_eq!(compactor.max_cache_size, 100);
        assert!(compactor.parser_cache.is_empty());
        assert!(compactor.ast_cache.is_empty());
    }

    #[test]
    fn test_custom_cache_size() {
        let compactor = AstCompactor::with_cache_size(50);
        assert_eq!(compactor.max_cache_size, 50);
    }

    #[test]
    fn test_language_detection() {
        let compactor = AstCompactor::new();

        assert_eq!(
            compactor.detect_language("fn main() {}").unwrap(),
            Language::Rust
        );

        assert_eq!(
            compactor.detect_language("def hello(): pass").unwrap(),
            Language::Python
        );
    }

    #[test]
    fn test_language_detection_failure() {
        let compactor = AstCompactor::new();

        assert!(matches!(
            compactor.detect_language(""),
            Err(CompactionError::LanguageDetectionFailed)
        ));

        assert!(matches!(
            compactor.detect_language("random text that looks like nothing"),
            Err(CompactionError::LanguageDetectionFailed)
        ));
    }

    #[test]
    fn test_empty_input() {
        let mut compactor = AstCompactor::new();
        let options = CompactionOptions::new();

        assert!(matches!(
            compactor.compact("", &options),
            Err(CompactionError::EmptyInput)
        ));
    }

    #[test]
    fn test_hash_calculation() {
        let compactor = AstCompactor::new();

        let hash1 = compactor.calculate_hash("fn main() {}");
        let hash2 = compactor.calculate_hash("fn main() {}");
        let hash3 = compactor.calculate_hash("fn test() {}");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_element_importance() {
        let compactor = AstCompactor::new();

        assert!(
            compactor.element_importance(ElementType::Interface)
                > compactor.element_importance(ElementType::Function)
        );
        assert!(
            compactor.element_importance(ElementType::Function)
                > compactor.element_importance(ElementType::Variable)
        );
        assert!(
            compactor.element_importance(ElementType::Variable)
                > compactor.element_importance(ElementType::Comment)
        );
    }

    #[test]
    fn test_stats_calculations() {
        let stats = CompactorStats {
            total_compactions: 5,
            total_processing_time_ms: 500,
            cache_hits: 3,
            cache_misses: 2,
            average_compression_ratio: 0.75,
        };

        assert_eq!(stats.average_processing_time_ms(), 100.0);
        assert_eq!(stats.compactions_per_second(), 10.0);

        let compactor = AstCompactor {
            stats,
            parser_cache: HashMap::new(),
            ast_cache: HashMap::new(),
            max_cache_size: 100,
        };

        assert_eq!(compactor.cache_hit_rate(), 60.0);
    }

    #[test]
    fn test_basic_rust_compaction() {
        let mut compactor = AstCompactor::new();
        let source = r#"
            pub fn hello() {
                println!("Hello, world!");
            }
            
            pub struct User {
                pub name: String,
            }
        "#;

        let options = CompactionOptions::new()
            .with_language(Language::Rust)
            .preserve_signatures_only(true);

        let result = compactor.compact(source, &options);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.language, Language::Rust);
        assert!(result.compression_ratio > 0.0);
        assert!(!result.elements.is_empty());
    }

    #[test]
    fn test_cache_functionality() {
        let mut compactor = AstCompactor::new();
        let source = "fn test() {}";
        let options = CompactionOptions::new().with_language(Language::Rust);

        // First compaction should be a cache miss
        let _ = compactor.compact(source, &options).unwrap();
        assert_eq!(compactor.stats.cache_misses, 1);
        assert_eq!(compactor.stats.cache_hits, 0);

        // Second compaction of same source should be a cache hit
        let _ = compactor.compact(source, &options).unwrap();
        assert_eq!(compactor.stats.cache_misses, 1);
        assert_eq!(compactor.stats.cache_hits, 1);
    }

    #[test]
    fn test_memory_info() {
        let mut compactor = AstCompactor::new();
        let (parsers, asts) = compactor.memory_info();
        assert_eq!((parsers, asts), (0, 0));

        // After compaction, should have cached parser and AST
        let source = "fn test() {}";
        let options = CompactionOptions::new().with_language(Language::Rust);
        let _ = compactor.compact(source, &options).unwrap();

        let (parsers, asts) = compactor.memory_info();
        assert_eq!((parsers, asts), (1, 1));
    }

    #[test]
    fn test_clear_caches() {
        let mut compactor = AstCompactor::new();
        let source = "fn test() {}";
        let options = CompactionOptions::new().with_language(Language::Rust);

        // Populate caches
        let _ = compactor.compact(source, &options).unwrap();
        assert_ne!(compactor.memory_info(), (0, 0));
        assert_ne!(compactor.stats.total_compactions, 0);

        // Clear should reset everything
        compactor.clear_caches();
        assert_eq!(compactor.memory_info(), (0, 0));
        assert_eq!(compactor.stats.total_compactions, 0);
    }
}
