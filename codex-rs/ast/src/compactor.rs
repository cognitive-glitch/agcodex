//! AI Distiller-style code compaction for 70-95% compression

use crate::error::AstError;
use crate::error::AstResult;
use crate::types::ParsedAst;
// use crate::language_registry::Language; // unused
use std::collections::HashSet;
use tree_sitter::Node;
use tree_sitter::TreeCursor;

/// Compression level for AST compaction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionLevel {
    /// Light: 70% compression - preserve more details
    Light,
    /// Standard: 85% compression - balanced
    Standard,
    /// Medium: alias for Standard
    Medium,
    /// Maximum: 95% compression - extreme compaction
    Maximum,
    /// Hard: alias for Maximum
    Hard,
}

impl CompressionLevel {
    /// Get target compression ratio
    pub const fn target_ratio(&self) -> f64 {
        match self {
            Self::Light => 0.70,
            Self::Standard | Self::Medium => 0.85,
            Self::Maximum | Self::Hard => 0.95,
        }
    }

    /// Should preserve comments
    pub const fn preserve_comments(&self) -> bool {
        matches!(self, Self::Light)
    }

    /// Should preserve implementation details
    pub const fn preserve_implementation(&self) -> bool {
        matches!(self, Self::Light)
    }

    /// Should preserve private members
    pub const fn preserve_private(&self) -> bool {
        !matches!(self, Self::Maximum | Self::Hard)
    }
}

/// AST compactor for code compression
#[derive(Debug)]
pub struct AstCompactor {
    compression_level: CompressionLevel,
    preserved_node_types: HashSet<String>,
    removed_node_types: HashSet<String>,
}

impl AstCompactor {
    /// Create a new compactor with specified compression level
    pub fn new(compression_level: CompressionLevel) -> Self {
        let mut compactor = Self {
            compression_level,
            preserved_node_types: HashSet::new(),
            removed_node_types: HashSet::new(),
        };

        // Configure based on compression level
        compactor.configure_for_level();
        compactor
    }

    /// Configure node types based on compression level
    fn configure_for_level(&mut self) {
        // Always preserve these structural elements
        self.preserved_node_types
            .insert("function_declaration".to_string());
        self.preserved_node_types
            .insert("function_item".to_string());
        self.preserved_node_types
            .insert("class_declaration".to_string());
        self.preserved_node_types.insert("struct_item".to_string());
        self.preserved_node_types.insert("impl_item".to_string());
        self.preserved_node_types.insert("enum_item".to_string());
        self.preserved_node_types
            .insert("interface_declaration".to_string());
        self.preserved_node_types.insert("type_alias".to_string());
        self.preserved_node_types
            .insert("import_statement".to_string());
        self.preserved_node_types
            .insert("use_declaration".to_string());

        match self.compression_level {
            CompressionLevel::Light => {
                // Preserve more implementation details
                self.preserved_node_types
                    .insert("method_declaration".to_string());
                self.preserved_node_types
                    .insert("function_definition".to_string());
                self.preserved_node_types.insert("comment".to_string());
                self.preserved_node_types.insert("doc_comment".to_string());
            }
            CompressionLevel::Standard | CompressionLevel::Medium => {
                // Remove most implementation but keep public API
                self.removed_node_types.insert("block".to_string());
                self.removed_node_types
                    .insert("compound_statement".to_string());
                self.removed_node_types.insert("comment".to_string());
                self.removed_node_types.insert("line_comment".to_string());
            }
            CompressionLevel::Maximum | CompressionLevel::Hard => {
                // Extreme compaction - only signatures
                self.removed_node_types.insert("block".to_string());
                self.removed_node_types
                    .insert("compound_statement".to_string());
                self.removed_node_types.insert("comment".to_string());
                self.removed_node_types.insert("doc_comment".to_string());
                self.removed_node_types.insert("private".to_string());
                self.removed_node_types.insert("protected".to_string());
                self.removed_node_types.insert("internal".to_string());
            }
        }
    }

    /// Compact an AST to compressed representation
    pub fn compact(&self, ast: &ParsedAst) -> AstResult<String> {
        let mut output = String::new();
        let source = ast.source.as_bytes();

        // Add file header with language
        output.push_str(&format!("// Language: {}\n", ast.language.name()));
        output.push_str("// Compacted representation\n\n");

        // Walk the tree and extract relevant nodes
        let mut cursor = ast.tree.root_node().walk();
        self.compact_node(&mut cursor, source, &mut output, 0)?;

        // Calculate compression ratio
        let original_size = ast.source.len();
        let compressed_size = output.len();
        let ratio = 1.0 - (compressed_size as f64 / original_size as f64);

        // Add compression stats
        output.push_str(&format!(
            "\n// Compression: {:.1}% ({}→{} bytes)",
            ratio * 100.0,
            original_size,
            compressed_size
        ));

        Ok(output)
    }

    /// Recursively compact a node
    fn compact_node(
        &self,
        cursor: &mut TreeCursor,
        source: &[u8],
        output: &mut String,
        depth: usize,
    ) -> AstResult<()> {
        let node = cursor.node();
        let node_type = node.kind();

        // Skip removed node types
        if self.removed_node_types.contains(node_type) {
            return Ok(());
        }

        // Check if this is a significant node to preserve
        if self.should_preserve_node(&node) {
            let indent = "  ".repeat(depth);

            // Extract node text based on type
            match node_type {
                "function_declaration" | "function_definition" | "function_item" => {
                    self.extract_function_signature(&node, source, output, &indent)?;
                }
                "class_declaration" | "struct_item" | "impl_item" => {
                    self.extract_class_structure(&node, source, output, &indent)?;
                }
                "import_statement" | "use_declaration" => {
                    let text = std::str::from_utf8(&source[node.byte_range()])
                        .map_err(|e| AstError::ParserError(e.to_string()))?;
                    output.push_str(&indent);
                    output.push_str(text.trim());
                    output.push('\n');
                }
                _ => {
                    // For other preserved nodes, extract simplified representation
                    if node.child_count() == 0 {
                        // Leaf node - include text if significant
                        if self.is_significant_leaf(&node) {
                            let text = std::str::from_utf8(&source[node.byte_range()])
                                .map_err(|e| AstError::ParserError(e.to_string()))?;
                            output.push_str(&indent);
                            output.push_str(text.trim());
                            output.push('\n');
                        }
                    }
                }
            }
        }

        // Process children
        if cursor.goto_first_child() {
            loop {
                self.compact_node(cursor, source, output, depth + 1)?;
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }

        Ok(())
    }

    /// Check if a node should be preserved
    fn should_preserve_node(&self, node: &Node) -> bool {
        let node_type = node.kind();

        // Check preserved types
        if self.preserved_node_types.contains(node_type) {
            return true;
        }

        // Check visibility for Maximum compression
        if matches!(
            self.compression_level,
            CompressionLevel::Maximum | CompressionLevel::Hard
        ) {
            // Only preserve public members
            if let Some(parent) = node.parent() {
                let parent_text = parent.kind();
                if parent_text.contains("private") || parent_text.contains("protected") {
                    return false;
                }
            }
        }

        // Check for significant structural nodes
        matches!(
            node_type,
            "module"
                | "namespace"
                | "package_declaration"
                | "trait_item"
                | "interface_declaration"
                | "protocol_declaration"
        )
    }

    /// Extract function signature without implementation
    fn extract_function_signature(
        &self,
        node: &Node,
        source: &[u8],
        output: &mut String,
        indent: &str,
    ) -> AstResult<()> {
        // Find the signature part (before the body)
        let mut sig_end = node.start_byte();

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                let child_type = child.kind();
                if child_type == "block"
                    || child_type == "compound_statement"
                    || child_type == "function_body"
                {
                    sig_end = child.start_byte();
                    break;
                }
            }
        }

        if sig_end == node.start_byte() {
            sig_end = node.end_byte();
        }

        let signature = std::str::from_utf8(&source[node.start_byte()..sig_end])
            .map_err(|e| AstError::ParserError(e.to_string()))?;

        output.push_str(indent);
        output.push_str(signature.trim());

        // Add semicolon if not present and not preserving implementation
        if !self.compression_level.preserve_implementation() && !signature.trim().ends_with(';') {
            output.push(';');
        }
        output.push('\n');

        Ok(())
    }

    /// Extract class/struct structure
    fn extract_class_structure(
        &self,
        node: &Node,
        source: &[u8],
        output: &mut String,
        indent: &str,
    ) -> AstResult<()> {
        // Get class/struct header
        let mut header_end = node.start_byte();
        let mut found_body = false;

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                let child_type = child.kind();
                if child_type == "field_declaration_list"
                    || child_type == "declaration_list"
                    || child_type == "class_body"
                    || child_type == "{"
                {
                    header_end = child.start_byte();
                    found_body = true;
                    break;
                }
            }
        }

        if !found_body {
            // No body found, include entire node
            let text = std::str::from_utf8(&source[node.byte_range()])
                .map_err(|e| AstError::ParserError(e.to_string()))?;
            output.push_str(indent);
            output.push_str(text.trim());
            output.push('\n');
            return Ok(());
        }

        let header = std::str::from_utf8(&source[node.start_byte()..header_end])
            .map_err(|e| AstError::ParserError(e.to_string()))?;

        output.push_str(indent);
        output.push_str(header.trim());
        output.push_str(" {\n");

        // Extract members based on compression level
        if !matches!(
            self.compression_level,
            CompressionLevel::Maximum | CompressionLevel::Hard
        ) {
            self.extract_class_members(node, source, output, &format!("{}  ", indent))?;
        }

        output.push_str(indent);
        output.push_str("}\n");

        Ok(())
    }

    /// Extract class members
    fn extract_class_members(
        &self,
        node: &Node,
        source: &[u8],
        output: &mut String,
        indent: &str,
    ) -> AstResult<()> {
        let mut cursor = node.walk();

        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                let child_type = child.kind();

                // Check for member nodes
                if matches!(
                    child_type,
                    "field_declaration"
                        | "method_declaration"
                        | "function_declaration"
                        | "property_declaration"
                        | "field"
                        | "method"
                ) {
                    // Check visibility
                    if matches!(
                        self.compression_level,
                        CompressionLevel::Maximum | CompressionLevel::Hard
                    ) {
                        // Skip private/protected members
                        let child_text =
                            std::str::from_utf8(&source[child.byte_range()]).unwrap_or("");
                        if child_text.contains("private") || child_text.contains("protected") {
                            continue;
                        }
                    }

                    // Extract member signature
                    self.extract_member_signature(&child, source, output, indent)?;
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(())
    }

    /// Extract member signature
    fn extract_member_signature(
        &self,
        node: &Node,
        source: &[u8],
        output: &mut String,
        indent: &str,
    ) -> AstResult<()> {
        // Find signature without body
        let mut sig_end = node.end_byte();

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i)
                && (child.kind() == "block" || child.kind() == "compound_statement")
            {
                sig_end = child.start_byte();
                break;
            }
        }

        let signature = std::str::from_utf8(&source[node.start_byte()..sig_end])
            .map_err(|e| AstError::ParserError(e.to_string()))?;

        output.push_str(indent);
        output.push_str(signature.trim());
        if !signature.trim().ends_with(';') {
            output.push(';');
        }
        output.push('\n');

        Ok(())
    }

    /// Check if a leaf node is significant
    fn is_significant_leaf(&self, node: &Node) -> bool {
        let node_type = node.kind();
        matches!(
            node_type,
            "identifier"
                | "type_identifier"
                | "string_literal"
                | "number_literal"
                | "boolean_literal"
        ) && node.parent().is_some_and(|p| self.should_preserve_node(&p))
    }

    /// Calculate compression statistics
    pub fn calculate_stats(&self, original: &str, compressed: &str) -> CompressionStats {
        let original_size = original.len();
        let compressed_size = compressed.len();
        let original_lines = original.lines().count();
        let compressed_lines = compressed.lines().count();

        CompressionStats {
            original_bytes: original_size,
            compressed_bytes: compressed_size,
            compression_ratio: 1.0 - (compressed_size as f64 / original_size as f64),
            original_lines,
            compressed_lines,
            line_reduction: 1.0 - (compressed_lines as f64 / original_lines as f64),
        }
    }
}

/// Compression statistics
#[derive(Debug, Clone)]
pub struct CompressionStats {
    pub original_bytes: usize,
    pub compressed_bytes: usize,
    pub compression_ratio: f64,
    pub original_lines: usize,
    pub compressed_lines: usize,
    pub line_reduction: f64,
}

impl CompressionStats {
    /// Check if target compression was achieved
    pub fn meets_target(&self, level: CompressionLevel) -> bool {
        self.compression_ratio >= level.target_ratio()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language_registry::LanguageRegistry;

    #[test]
    fn test_compression_levels() {
        let light = CompressionLevel::Light;
        let standard = CompressionLevel::Standard;
        let maximum = CompressionLevel::Maximum;

        assert_eq!(light.target_ratio(), 0.70);
        assert_eq!(standard.target_ratio(), 0.85);
        assert_eq!(maximum.target_ratio(), 0.95);

        assert!(light.preserve_comments());
        assert!(!standard.preserve_comments());
        assert!(!maximum.preserve_comments());
    }

    #[test]
    fn test_compaction() {
        let registry = LanguageRegistry::new();
        let compactor = AstCompactor::new(CompressionLevel::Standard);

        let code = r#"
// This is a comment
fn calculate_fibonacci(n: u32) -> u32 {
    // Implementation details
    if n <= 1 {
        return n;
    }
    calculate_fibonacci(n - 1) + calculate_fibonacci(n - 2)
}

pub struct Calculator {
    value: i32,
}

impl Calculator {
    pub fn new() -> Self {
        Self { value: 0 }
    }
    
    pub fn add(&mut self, x: i32) {
        self.value += x;
    }
    
    private fn reset(&mut self) {
        self.value = 0;
    }
}
"#;

        let ast = registry.parse(&crate::Language::Rust, code).unwrap();
        let compressed = compactor.compact(&ast).unwrap();

        // Should be significantly smaller
        assert!(compressed.len() < code.len());

        // Should preserve structure
        assert!(compressed.contains("fn calculate_fibonacci"));
        assert!(compressed.contains("pub struct Calculator"));
        assert!(compressed.contains("pub fn new"));
        assert!(compressed.contains("pub fn add"));

        // Should not have implementation details in Standard mode
        assert!(!compressed.contains("if n <= 1"));
        assert!(!compressed.contains("self.value += x"));
    }
}
