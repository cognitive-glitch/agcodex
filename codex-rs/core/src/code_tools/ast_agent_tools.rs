//! AST-based agent tools for precise code analysis and transformation.
//! These tools power the internal coding agents with semantic understanding.

use super::CodeTool;
use super::ToolError;
use dashmap::DashMap;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tree_sitter::{Parser, Node, TreeCursor};

// Import AST infrastructure
use ast::{
    AstEngine, CompressionLevel, Language, LanguageRegistry, ParsedAst
};

// Re-export for easier access
type AstRegistry = LanguageRegistry;

/// Core agent tools powered by tree-sitter AST
#[derive(Clone)]
pub struct ASTAgentTools {
    // DashMap provides concurrent access with excellent performance
    // Parser wrapped in Arc since it doesn't implement Clone
    parsers: DashMap<String, Arc<Parser>>,
    semantic_cache: DashMap<PathBuf, SemanticIndex>,
}

/// Semantic index for a file containing symbols and structure
#[derive(Debug, Clone)]
pub struct SemanticIndex {
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    pub imports: Vec<ImportInfo>,
    pub exports: Vec<ExportInfo>,
    pub symbols: Vec<SymbolInfo>,
    pub call_graph: CallGraph,
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub signature: String,
    pub start_line: usize,
    pub end_line: usize,
    pub complexity: usize,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
    pub is_async: bool,
    pub is_exported: bool,
}

#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub methods: Vec<String>,
    pub properties: Vec<String>,
    pub base_classes: Vec<String>,
    pub is_exported: bool,
}

#[derive(Debug, Clone)]
pub struct ImportInfo {
    pub module: String,
    pub symbols: Vec<String>,
    pub line: usize,
    pub is_default: bool,
}

#[derive(Debug, Clone)]
pub struct ExportInfo {
    pub symbol: String,
    pub line: usize,
    pub is_default: bool,
    pub is_named: bool,
}

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub line: usize,
    pub column: usize,
    pub scope: String,
    pub references: Vec<Location>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Function,
    Class,
    Variable,
    Constant,
    Type,
    Interface,
    Enum,
    Module,
    Namespace,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Location {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub byte_offset: usize,
}

#[derive(Debug, Clone, Default)]
pub struct CallGraph {
    pub nodes: HashMap<String, CallNode>,
    pub edges: Vec<CallEdge>,
}

#[derive(Debug, Clone)]
pub struct CallNode {
    pub function_name: String,
    pub file: PathBuf,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct CallEdge {
    pub caller: String,
    pub callee: String,
    pub call_site: Location,
}

/// Agent tool operations
#[derive(Debug, Clone)]
pub enum AgentToolOp {
    /// Extract all functions from a file
    ExtractFunctions { file: PathBuf, language: String },

    /// Find symbol definition
    FindDefinition { symbol: String, scope: SearchScope },

    /// Analyze call graph from entry point
    AnalyzeCallGraph { entry_point: String },

    /// Detect code patterns
    DetectPatterns { pattern: PatternType },

    /// Refactor rename operation
    RefactorRename {
        old_name: String,
        new_name: String,
        scope: SearchScope,
    },

    /// Extract method from code block
    ExtractMethod {
        start: Location,
        end: Location,
        method_name: String,
    },

    /// Inline variable
    InlineVariable {
        variable: String,
        scope: SearchScope,
    },

    /// Calculate cyclomatic complexity
    CalculateComplexity { function: String },

    /// Find dead code
    FindDeadCode { scope: SearchScope },

    /// Detect code duplication
    DetectDuplication { threshold: f32 },

    /// Analyze module dependencies
    AnalyzeDependencies { module: String },

    /// Find all references to a symbol
    FindReferences { symbol: String },

    /// Generate documentation from AST
    GenerateDocumentation { target: DocumentationTarget },

    /// Validate syntax
    ValidateSyntax { file: PathBuf, language: String },

    /// Suggest improvements
    SuggestImprovements {
        file: PathBuf,
        focus: ImprovementFocus,
    },
}

#[derive(Debug, Clone)]
pub enum SearchScope {
    File(PathBuf),
    Directory(PathBuf),
    Module(String),
    Global,
}

#[derive(Debug, Clone)]
pub enum PatternType {
    Singleton,
    Factory,
    Observer,
    Strategy,
    Decorator,
    AntiPattern(String),
    CodeSmell(String),
}

#[derive(Debug, Clone)]
pub enum DocumentationTarget {
    Function(String),
    Class(String),
    Module(String),
    File(PathBuf),
}

#[derive(Debug, Clone)]
pub enum ImprovementFocus {
    Performance,
    Readability,
    Maintainability,
    Security,
    TestCoverage,
    ErrorHandling,
}

/// Result of agent tool execution
#[derive(Debug, Clone)]
pub enum AgentToolResult {
    Functions(Vec<FunctionInfo>),
    Classes(Vec<ClassInfo>),
    Definition(Option<Location>),
    CallGraph(CallGraph),
    Patterns(Vec<DetectedPattern>),
    RefactorPlan(RefactorPlan),
    Complexity(ComplexityReport),
    DeadCode(Vec<DeadCodeItem>),
    Duplications(Vec<DuplicationGroup>),
    Dependencies(DependencyGraph),
    References(Vec<Location>),
    Documentation(String),
    ValidationResult(ValidationReport),
    Improvements(Vec<Improvement>),
}

#[derive(Debug, Clone)]
pub struct DetectedPattern {
    pub pattern_type: PatternType,
    pub location: Location,
    pub confidence: f32,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct RefactorPlan {
    pub changes: Vec<RefactorChange>,
    pub affected_files: Vec<PathBuf>,
    pub risk_level: RiskLevel,
    pub estimated_impact: String,
}

#[derive(Debug, Clone)]
pub struct RefactorChange {
    pub file: PathBuf,
    pub location: Location,
    pub old_text: String,
    pub new_text: String,
    pub change_type: String,
}

#[derive(Debug, Clone)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone)]
pub struct ComplexityReport {
    pub function: String,
    pub cyclomatic_complexity: usize,
    pub cognitive_complexity: usize,
    pub lines_of_code: usize,
    pub parameters: usize,
    pub nesting_depth: usize,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DeadCodeItem {
    pub symbol: String,
    pub location: Location,
    pub kind: SymbolKind,
    pub confidence: f32,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct DuplicationGroup {
    pub locations: Vec<Location>,
    pub lines: usize,
    pub tokens: usize,
    pub similarity: f32,
    pub extract_suggestion: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DependencyGraph {
    pub module: String,
    pub direct_deps: Vec<String>,
    pub transitive_deps: Vec<String>,
    pub reverse_deps: Vec<String>,
    pub circular_deps: Vec<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub file: PathBuf,
    pub is_valid: bool,
    pub errors: Vec<SyntaxError>,
    pub warnings: Vec<SyntaxWarning>,
}

#[derive(Debug, Clone)]
pub struct SyntaxError {
    pub location: Location,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone)]
pub struct SyntaxWarning {
    pub location: Location,
    pub message: String,
    pub rule: String,
}

#[derive(Debug, Clone)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone)]
pub struct Improvement {
    pub location: Location,
    pub category: ImprovementFocus,
    pub description: String,
    pub suggested_change: Option<String>,
    pub impact: ImpactLevel,
}

#[derive(Debug, Clone)]
pub enum ImpactLevel {
    Critical,
    High,
    Medium,
    Low,
}

impl Default for ASTAgentTools {
    fn default() -> Self {
        Self::new()
    }
}

impl ASTAgentTools {
    pub fn new() -> Self {
        Self {
            parsers: DashMap::new(),
            semantic_cache: DashMap::new(),
        }
    }

    /// Execute an agent tool operation
    pub fn execute(&mut self, op: AgentToolOp) -> Result<AgentToolResult, ToolError> {
        match op {
            AgentToolOp::ExtractFunctions { file, language } => {
                let functions = self.extract_functions(&file, &language)?;
                Ok(AgentToolResult::Functions(functions))
            }
            AgentToolOp::FindDefinition { symbol, scope } => {
                let definition = self.find_definition(&symbol, &scope)?;
                Ok(AgentToolResult::Definition(definition))
            }
            AgentToolOp::AnalyzeCallGraph { entry_point } => {
                let graph = self.analyze_call_graph(&entry_point)?;
                Ok(AgentToolResult::CallGraph(graph))
            }
            AgentToolOp::DetectPatterns { pattern } => {
                let patterns = self.detect_patterns(&pattern)?;
                Ok(AgentToolResult::Patterns(patterns))
            }
            AgentToolOp::RefactorRename {
                old_name,
                new_name,
                scope,
            } => {
                let plan = self.plan_rename(&old_name, &new_name, &scope)?;
                Ok(AgentToolResult::RefactorPlan(plan))
            }
            AgentToolOp::CalculateComplexity { function } => {
                let report = self.calculate_complexity(&function)?;
                Ok(AgentToolResult::Complexity(report))
            }
            AgentToolOp::FindDeadCode { scope } => {
                let dead_code = self.find_dead_code(&scope)?;
                Ok(AgentToolResult::DeadCode(dead_code))
            }
            AgentToolOp::DetectDuplication { threshold } => {
                let duplications = self.detect_duplication(threshold)?;
                Ok(AgentToolResult::Duplications(duplications))
            }
            AgentToolOp::AnalyzeDependencies { module } => {
                let deps = self.analyze_dependencies(&module)?;
                Ok(AgentToolResult::Dependencies(deps))
            }
            AgentToolOp::FindReferences { symbol } => {
                let refs = self.find_references(&symbol)?;
                Ok(AgentToolResult::References(refs))
            }
            AgentToolOp::GenerateDocumentation { target } => {
                let docs = self.generate_documentation(&target)?;
                Ok(AgentToolResult::Documentation(docs))
            }
            AgentToolOp::ValidateSyntax { file, language } => {
                let report = self.validate_syntax(&file, &language)?;
                Ok(AgentToolResult::ValidationResult(report))
            }
            AgentToolOp::SuggestImprovements { file, focus } => {
                let improvements = self.suggest_improvements(&file, &focus)?;
                Ok(AgentToolResult::Improvements(improvements))
            }
            _ => Err(ToolError::NotImplemented("agent tool operation")),
        }
    }

    // Core AST agent tool implementations
    
    /// Extract all functions from a file using tree-sitter AST parsing
    /// 
    /// This method:
    /// 1. Creates an AST engine for the specified language
    /// 2. Parses the file to build a complete AST
    /// 3. Traverses the AST to find function definitions
    /// 4. Extracts function metadata including complexity
    /// 5. Returns structured function information
    fn extract_functions(
        &self,
        file: &PathBuf,
        _language: &str,
    ) -> Result<Vec<FunctionInfo>, ToolError> {
        // Create AST engine with medium compression for balanced performance
        let engine = AstEngine::new(CompressionLevel::Medium);
        
        // Parse the file asynchronously (we'll need to block here)
        let rt = tokio::runtime::Handle::try_current()
            .or_else(|_| tokio::runtime::Runtime::new().map(|rt| rt.handle().clone()))
            .map_err(|e| ToolError::InvalidQuery(format!("Failed to create async runtime: {}", e)))?;
        
        let parsed_ast = rt.block_on(engine.parse_file(file))
            .map_err(|e| ToolError::InvalidQuery(format!("Failed to parse file: {}", e)))?;
        
        let mut functions = Vec::new();
        let source = parsed_ast.source.as_bytes();
        let mut cursor = parsed_ast.tree.root_node().walk();
        
        // Extract functions using tree-sitter traversal
        self.extract_functions_from_node(&mut cursor, source, file, &parsed_ast.language, &mut functions)?;
        
        Ok(functions)
    }
    
    /// Helper method to recursively extract functions from AST nodes
    fn extract_functions_from_node(
        &self,
        cursor: &mut TreeCursor,
        source: &[u8],
        _file_path: &PathBuf,
        language: &Language,
        functions: &mut Vec<FunctionInfo>,
    ) -> Result<(), ToolError> {
        let node = cursor.node();
        
        // Check if this node represents a function definition
        if self.is_function_node(&node) {
            let function_info = self.create_function_info(&node, source, _file_path, language)?;
            functions.push(function_info);
        }
        
        // Recursively traverse child nodes
        if cursor.goto_first_child() {
            loop {
                self.extract_functions_from_node(cursor, source, _file_path, language, functions)?;
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
        
        Ok(())
    }
    
    /// Check if a tree-sitter node represents a function definition
    fn is_function_node(&self, node: &Node) -> bool {
        matches!(
            node.kind(),
            "function_declaration" | "function_definition" | "function_item" |
            "method_declaration" | "method_definition" | "async_function" |
            "arrow_function" | "function_expression" | "method_signature"
        )
    }
    
    /// Create FunctionInfo from a tree-sitter function node
    fn create_function_info(
        &self,
        node: &Node,
        source: &[u8],
        _file_path: &PathBuf,
        language: &Language,
    ) -> Result<FunctionInfo, ToolError> {
        let name = self.extract_function_name(node, source)?;
        let signature = self.extract_function_signature(node, source)?;
        let parameters = self.extract_function_parameters(node, source)?;
        let return_type = self.extract_return_type(node, source);
        let is_async = self.is_async_function(node, source);
        let is_exported = self.is_exported_function(node, source, language);
        let complexity = self.calculate_cyclomatic_complexity(node);
        
        Ok(FunctionInfo {
            name,
            signature,
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
            complexity,
            parameters,
            return_type,
            is_async,
            is_exported,
        })
    }
    
    /// Extract function name from AST node
    fn extract_function_name(&self, node: &Node, source: &[u8]) -> Result<String, ToolError> {
        // Look for identifier child that represents the function name
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "identifier" || child.kind() == "name" {
                    let name_bytes = &source[child.byte_range()];
                    let name = std::str::from_utf8(name_bytes)
                        .map_err(|e| ToolError::InvalidQuery(format!("Invalid UTF-8 in function name: {}", e)))?;
                    return Ok(name.to_string());
                }
            }
        }
        
        // Fallback: extract from node text
        let text = std::str::from_utf8(&source[node.byte_range()])
            .map_err(|e| ToolError::InvalidQuery(format!("Invalid UTF-8 in node: {}", e)))?;
        
        // Basic heuristic to find function name
        let words: Vec<&str> = text.split_whitespace().collect();
        for (i, word) in words.iter().enumerate() {
            if matches!(*word, "fn" | "function" | "def" | "func") && i + 1 < words.len() {
                return Ok(words[i + 1].trim_matches(['(', ')', '{', '}', ':']).to_string());
            }
        }
        
        Ok("anonymous".to_string())
    }
    
    /// Extract function signature (without body)
    fn extract_function_signature(&self, node: &Node, source: &[u8]) -> Result<String, ToolError> {
        // Find the end of signature (before function body)
        let mut sig_end = node.end_byte();
        
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if matches!(child.kind(), "block" | "compound_statement" | "function_body" | "=>") {
                    sig_end = child.start_byte();
                    break;
                }
            }
        }
        
        let signature_bytes = &source[node.start_byte()..sig_end];
        let signature = std::str::from_utf8(signature_bytes)
            .map_err(|e| ToolError::InvalidQuery(format!("Invalid UTF-8 in signature: {}", e)))?;
        
        Ok(signature.trim().to_string())
    }
    
    /// Extract function parameters
    fn extract_function_parameters(&self, node: &Node, source: &[u8]) -> Result<Vec<String>, ToolError> {
        let mut parameters = Vec::new();
        
        // Look for parameter list nodes
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if matches!(child.kind(), "parameters" | "parameter_list" | "formal_parameters") {
                    self.extract_parameters_from_list(&child, source, &mut parameters)?;
                    break;
                }
            }
        }
        
        Ok(parameters)
    }
    
    /// Extract parameters from parameter list node
    fn extract_parameters_from_list(
        &self,
        param_list: &Node,
        source: &[u8],
        parameters: &mut Vec<String>,
    ) -> Result<(), ToolError> {
        for i in 0..param_list.child_count() {
            if let Some(param) = param_list.child(i) {
                if matches!(param.kind(), "parameter" | "identifier" | "typed_parameter") {
                    let param_text = std::str::from_utf8(&source[param.byte_range()])
                        .map_err(|e| ToolError::InvalidQuery(format!("Invalid UTF-8 in parameter: {}", e)))?;
                    parameters.push(param_text.trim().to_string());
                }
            }
        }
        Ok(())
    }
    
    /// Extract return type if present
    fn extract_return_type(&self, node: &Node, source: &[u8]) -> Option<String> {
        // Look for return type indicators
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if matches!(child.kind(), "type_annotation" | "return_type" | "->") {
                    if let Ok(type_text) = std::str::from_utf8(&source[child.byte_range()]) {
                        return Some(type_text.trim().to_string());
                    }
                }
            }
        }
        None
    }
    
    /// Check if function is async
    fn is_async_function(&self, node: &Node, source: &[u8]) -> bool {
        let text = std::str::from_utf8(&source[node.byte_range()]).unwrap_or("");
        text.contains("async") || node.kind() == "async_function"
    }
    
    /// Check if function is exported/public
    fn is_exported_function(&self, node: &Node, source: &[u8], language: &Language) -> bool {
        let text = std::str::from_utf8(&source[node.byte_range()]).unwrap_or("");
        
        match language {
            Language::Rust => text.contains("pub"),
            Language::TypeScript | Language::JavaScript => {
                text.contains("export") || text.contains("public")
            }
            Language::Python => {
                // Python doesn't have explicit export keywords, consider top-level functions as exported
                !text.starts_with("    ") && !text.starts_with("\t")
            }
            Language::Java | Language::CSharp => text.contains("public"),
            Language::Go => {
                // Go exports functions that start with uppercase
                if let Ok(name) = self.extract_function_name(node, source) {
                    name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                } else {
                    false
                }
            }
            _ => false,
        }
    }
    
    /// Calculate cyclomatic complexity of a function
    fn calculate_cyclomatic_complexity(&self, node: &Node) -> usize {
        let mut complexity = 1; // Base complexity
        let mut cursor = node.walk();
        
        // Count decision points
        if cursor.goto_first_child() {
            loop {
                let child_node = cursor.node();
                match child_node.kind() {
                    "if_statement" | "if_expression" | "conditional_expression" |
                    "while_statement" | "for_statement" | "loop_statement" |
                    "match_expression" | "switch_statement" |
                    "case_clause" | "match_arm" |
                    "catch_clause" | "except_clause" |
                    "&&" | "||" | "and" | "or" => {
                        complexity += 1;
                    }
                    _ => {
                        // Recursively analyze child nodes
                        complexity += self.count_complexity_recursive(&child_node);
                    }
                }
                
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        
        complexity
    }
    
    /// Recursively count complexity in child nodes
    fn count_complexity_recursive(&self, node: &Node) -> usize {
        let mut complexity = 0;
        let mut cursor = node.walk();
        
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                match child.kind() {
                    "if_statement" | "if_expression" | "conditional_expression" |
                    "while_statement" | "for_statement" | "loop_statement" |
                    "match_expression" | "switch_statement" |
                    "case_clause" | "match_arm" |
                    "catch_clause" | "except_clause" |
                    "&&" | "||" | "and" | "or" => {
                        complexity += 1;
                    }
                    _ => {
                        complexity += self.count_complexity_recursive(&child);
                    }
                }
                
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        
        complexity
    }

    /// Find symbol definitions using semantic indexing
    /// 
    /// This method:
    /// 1. Searches the semantic cache for existing symbol information
    /// 2. If not cached, builds semantic index for the specified scope
    /// 3. Performs symbol resolution across files and modules
    /// 4. Returns precise file:line:column location if found
    fn find_definition(
        &self,
        symbol: &str,
        scope: &SearchScope,
    ) -> Result<Option<Location>, ToolError> {
        match scope {
            SearchScope::File(file_path) => {
                self.find_definition_in_file(symbol, file_path)
            }
            SearchScope::Directory(dir_path) => {
                self.find_definition_in_directory(symbol, dir_path)
            }
            SearchScope::Module(module_name) => {
                self.find_definition_in_module(symbol, module_name)
            }
            SearchScope::Global => {
                self.find_definition_global(symbol)
            }
        }
    }
    
    /// Find symbol definition within a specific file
    fn find_definition_in_file(&self, symbol: &str, file_path: &PathBuf) -> Result<Option<Location>, ToolError> {
        // Check semantic cache first
        if let Some(cached_index) = self.semantic_cache.get(file_path) {
            for symbol_info in &cached_index.symbols {
                if symbol_info.name == symbol {
                    return Ok(Some(Location {
                        file: file_path.clone(),
                        line: symbol_info.line,
                        column: symbol_info.column,
                        byte_offset: 0, // We'll need to calculate this if needed
                    }));
                }
            }
        }
        
        // If not cached, parse and index the file
        let engine = AstEngine::new(CompressionLevel::Medium);
        let rt = tokio::runtime::Handle::try_current()
            .or_else(|_| tokio::runtime::Runtime::new().map(|rt| rt.handle().clone()))
            .map_err(|e| ToolError::InvalidQuery(format!("Failed to create async runtime: {}", e)))?;
        
        let parsed_ast = rt.block_on(engine.parse_file(file_path))
            .map_err(|e| ToolError::InvalidQuery(format!("Failed to parse file {}: {}", file_path.display(), e)))?;
        
        // Search for symbol in parsed AST
        let location = self.search_symbol_in_ast(symbol, &parsed_ast, file_path)?;
        
        // Cache the results for future queries
        self.cache_semantic_info(file_path, &parsed_ast)?;
        
        Ok(location)
    }
    
    /// Find symbol definition within a directory
    fn find_definition_in_directory(&self, symbol: &str, dir_path: &PathBuf) -> Result<Option<Location>, ToolError> {
        use std::fs;
        
        // Recursively search through directory
        let entries = fs::read_dir(dir_path)
            .map_err(|e| ToolError::Io(e))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| ToolError::Io(e))?;
            let path = entry.path();
            
            if path.is_file() {
                // Check if it's a supported programming language file
                if let Ok(result) = self.find_definition_in_file(symbol, &path) {
                    if result.is_some() {
                        return Ok(result);
                    }
                }
            } else if path.is_dir() {
                // Recursively search subdirectories
                if let Ok(result) = self.find_definition_in_directory(symbol, &path) {
                    if result.is_some() {
                        return Ok(result);
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// Find symbol definition within a module (language-specific)
    fn find_definition_in_module(&self, symbol: &str, module_name: &str) -> Result<Option<Location>, ToolError> {
        // This is a simplified implementation
        // In a real implementation, we'd need to:
        // 1. Resolve module paths based on language-specific import rules
        // 2. Search through module files
        // 3. Handle package/namespace resolution
        
        // For now, treat module as a directory name and search
        let current_dir = std::env::current_dir()
            .map_err(|e| ToolError::Io(e))?;
        let module_path = current_dir.join(module_name);
        
        if module_path.exists() && module_path.is_dir() {
            self.find_definition_in_directory(symbol, &module_path)
        } else {
            // Try common module patterns
            let patterns = [
                format!("{}.rs", module_name),
                format!("{}.py", module_name),
                format!("{}.js", module_name),
                format!("{}.ts", module_name),
                format!("src/{}.rs", module_name),
                format!("lib/{}.py", module_name),
            ];
            
            for pattern in &patterns {
                let file_path = current_dir.join(pattern);
                if file_path.exists() {
                    if let Ok(result) = self.find_definition_in_file(symbol, &file_path) {
                        if result.is_some() {
                            return Ok(result);
                        }
                    }
                }
            }
            
            Ok(None)
        }
    }
    
    /// Find symbol definition globally (across all cached files)
    fn find_definition_global(&self, symbol: &str) -> Result<Option<Location>, ToolError> {
        // Search through all cached semantic information
        for entry in self.semantic_cache.iter() {
            let file_path = entry.key();
            let semantic_index = entry.value();
            
            for symbol_info in &semantic_index.symbols {
                if symbol_info.name == symbol {
                    return Ok(Some(Location {
                        file: file_path.clone(),
                        line: symbol_info.line,
                        column: symbol_info.column,
                        byte_offset: 0,
                    }));
                }
            }
        }
        
        // If not found in cache, we'd need to index more files
        // For now, return None
        Ok(None)
    }
    
    /// Search for a symbol in a parsed AST
    fn search_symbol_in_ast(
        &self,
        symbol: &str,
        ast: &ParsedAst,
        file_path: &PathBuf,
    ) -> Result<Option<Location>, ToolError> {
        let source = ast.source.as_bytes();
        let mut cursor = ast.tree.root_node().walk();
        
        if let Some(location) = self.search_symbol_recursive(&mut cursor, symbol, source, file_path)? {
            Ok(Some(location))
        } else {
            Ok(None)
        }
    }
    
    /// Recursively search for symbol in AST nodes
    fn search_symbol_recursive(
        &self,
        cursor: &mut TreeCursor,
        symbol: &str,
        source: &[u8],
        file_path: &PathBuf,
    ) -> Result<Option<Location>, ToolError> {
        let node = cursor.node();
        
        // Check if this node is a symbol definition
        if self.is_symbol_definition_node(&node) {
            if let Ok(name) = self.extract_symbol_name_from_node(&node, source) {
                if name == symbol {
                    return Ok(Some(Location {
                        file: file_path.clone(),
                        line: node.start_position().row + 1,
                        column: node.start_position().column + 1,
                        byte_offset: node.start_byte(),
                    }));
                }
            }
        }
        
        // Recursively search child nodes
        if cursor.goto_first_child() {
            loop {
                if let Some(location) = self.search_symbol_recursive(cursor, symbol, source, file_path)? {
                    return Ok(Some(location));
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
        
        Ok(None)
    }
    
    /// Check if a node represents a symbol definition
    fn is_symbol_definition_node(&self, node: &Node) -> bool {
        matches!(
            node.kind(),
            "function_declaration" | "function_definition" | "function_item" |
            "method_declaration" | "method_definition" |
            "class_declaration" | "class_definition" |
            "struct_item" | "struct_declaration" |
            "enum_item" | "enum_declaration" |
            "interface_declaration" | "trait_item" |
            "variable_declaration" | "let_declaration" | "const_item" |
            "type_alias" | "typedef" | "module" | "namespace"
        )
    }
    
    /// Extract symbol name from a definition node
    fn extract_symbol_name_from_node(&self, node: &Node, source: &[u8]) -> Result<String, ToolError> {
        // Look for identifier child
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "identifier" || child.kind() == "name" {
                    let name_bytes = &source[child.byte_range()];
                    let name = std::str::from_utf8(name_bytes)
                        .map_err(|e| ToolError::InvalidQuery(format!("Invalid UTF-8: {}", e)))?;
                    return Ok(name.to_string());
                }
            }
        }
        
        Err(ToolError::InvalidQuery("No identifier found".to_string()))
    }
    
    /// Cache semantic information for a file
    fn cache_semantic_info(&self, file_path: &PathBuf, ast: &ParsedAst) -> Result<(), ToolError> {
        let mut semantic_index = SemanticIndex {
            functions: Vec::new(),
            classes: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            symbols: Vec::new(),
            call_graph: CallGraph::default(),
        };
        
        // Extract symbols from AST and populate the semantic index
        let source = ast.source.as_bytes();
        let mut cursor = ast.tree.root_node().walk();
        self.extract_symbols_for_cache(&mut cursor, source, file_path, &mut semantic_index)?;
        
        // Cache the semantic index
        self.semantic_cache.insert(file_path.clone(), semantic_index);
        
        Ok(())
    }
    
    /// Extract symbols for caching
    fn extract_symbols_for_cache(
        &self,
        cursor: &mut TreeCursor,
        source: &[u8],
        file_path: &PathBuf,
        semantic_index: &mut SemanticIndex,
    ) -> Result<(), ToolError> {
        let node = cursor.node();
        
        if self.is_symbol_definition_node(&node) {
            if let Ok(name) = self.extract_symbol_name_from_node(&node, source) {
                let symbol_info = SymbolInfo {
                    name,
                    kind: self.node_to_symbol_kind(&node),
                    line: node.start_position().row + 1,
                    column: node.start_position().column + 1,
                    scope: "global".to_string(), // Simplified scope
                    references: Vec::new(),
                };
                semantic_index.symbols.push(symbol_info);
            }
        }
        
        // Recursively process children
        if cursor.goto_first_child() {
            loop {
                self.extract_symbols_for_cache(cursor, source, file_path, semantic_index)?;
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
        
        Ok(())
    }
    
    /// Convert tree-sitter node to symbol kind
    fn node_to_symbol_kind(&self, node: &Node) -> SymbolKind {
        match node.kind() {
            "function_declaration" | "function_definition" | "function_item" |
            "method_declaration" | "method_definition" => SymbolKind::Function,
            "class_declaration" | "class_definition" => SymbolKind::Class,
            "struct_item" | "struct_declaration" => SymbolKind::Variable, // Simplified
            "enum_item" | "enum_declaration" => SymbolKind::Enum,
            "interface_declaration" | "trait_item" => SymbolKind::Interface,
            "module" | "namespace" => SymbolKind::Module,
            "variable_declaration" | "let_declaration" => SymbolKind::Variable,
            "const_item" => SymbolKind::Constant,
            "type_alias" | "typedef" => SymbolKind::Type,
            _ => SymbolKind::Variable,
        }
    }

    /// Analyze call graph starting from an entry point function
    /// 
    /// This method:
    /// 1. Finds the entry point function definition
    /// 2. Recursively analyzes all function calls within it
    /// 3. Builds a directed graph of function dependencies
    /// 4. Detects circular dependencies and orphaned functions
    /// 5. Returns complete call graph with nodes and edges
    fn analyze_call_graph(&self, entry_point: &str) -> Result<CallGraph, ToolError> {
        let mut call_graph = CallGraph {
            nodes: HashMap::new(),
            edges: Vec::new(),
        };
        
        // Find the entry point function across all cached files
        let entry_location = self.find_definition_global(entry_point)?
            .ok_or_else(|| ToolError::InvalidQuery(format!("Entry point function '{}' not found", entry_point)))?;
        
        // Create entry point node
        let entry_node = CallNode {
            function_name: entry_point.to_string(),
            file: entry_location.file.clone(),
            location: entry_location.clone(),
        };
        call_graph.nodes.insert(entry_point.to_string(), entry_node);
        
        // Recursively analyze call graph starting from entry point
        let mut visited = std::collections::HashSet::new();
        self.analyze_function_calls(&entry_location.file, entry_point, &mut call_graph, &mut visited)?;
        
        Ok(call_graph)
    }
    
    /// Recursively analyze function calls for call graph building
    fn analyze_function_calls(
        &self,
        file_path: &PathBuf,
        function_name: &str,
        call_graph: &mut CallGraph,
        visited: &mut std::collections::HashSet<String>,
    ) -> Result<(), ToolError> {
        // Avoid infinite recursion
        let function_key = format!("{}:{}", file_path.display(), function_name);
        if visited.contains(&function_key) {
            return Ok(());
        }
        visited.insert(function_key);
        
        // Parse the file to analyze function calls
        let engine = AstEngine::new(CompressionLevel::Medium);
        let rt = tokio::runtime::Handle::try_current()
            .or_else(|_| tokio::runtime::Runtime::new().map(|rt| rt.handle().clone()))
            .map_err(|e| ToolError::InvalidQuery(format!("Failed to create async runtime: {}", e)))?;
        
        let parsed_ast = rt.block_on(engine.parse_file(file_path))
            .map_err(|e| ToolError::InvalidQuery(format!("Failed to parse file: {}", e)))?;
        
        // Find the function definition in the AST
        if let Some(function_node) = self.find_function_node(&parsed_ast, function_name)? {
            // Analyze calls within this function
            let source = parsed_ast.source.as_bytes();
            let call_sites = self.extract_function_calls(&function_node, source)?;
            
            // Process each function call
            for call_site in call_sites {
                // Create edge in call graph
                let edge = CallEdge {
                    caller: function_name.to_string(),
                    callee: call_site.called_function.clone(),
                    call_site: Location {
                        file: file_path.clone(),
                        line: call_site.line,
                        column: call_site.column,
                        byte_offset: call_site.byte_offset,
                    },
                };
                call_graph.edges.push(edge);
                
                // Try to find the called function definition
                if let Ok(Some(callee_location)) = self.find_definition_global(&call_site.called_function) {
                    // Create node for called function if not already present
                    if !call_graph.nodes.contains_key(&call_site.called_function) {
                        let callee_node = CallNode {
                            function_name: call_site.called_function.clone(),
                            file: callee_location.file.clone(),
                            location: callee_location.clone(),
                        };
                        call_graph.nodes.insert(call_site.called_function.clone(), callee_node);
                    }
                    
                    // Recursively analyze the called function
                    self.analyze_function_calls(
                        &callee_location.file,
                        &call_site.called_function,
                        call_graph,
                        visited,
                    )?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Find a specific function node in the parsed AST
    fn find_function_node<'a>(&self, ast: &'a ParsedAst, function_name: &str) -> Result<Option<Node<'a>>, ToolError> {
        let source = ast.source.as_bytes();
        let mut cursor = ast.tree.root_node().walk();
        
        self.find_function_node_recursive(&mut cursor, function_name, source)
    }
    
    /// Recursively search for a function node by name
    fn find_function_node_recursive<'a>(
        &self,
        cursor: &mut TreeCursor<'a>,
        function_name: &str,
        source: &[u8],
    ) -> Result<Option<Node<'a>>, ToolError> {
        let node = cursor.node();
        
        // Check if this is a function definition
        if self.is_function_node(&node) {
            if let Ok(name) = self.extract_function_name(&node, source) {
                if name == function_name {
                    return Ok(Some(node));
                }
            }
        }
        
        // Recursively search child nodes
        if cursor.goto_first_child() {
            loop {
                if let Some(function_node) = self.find_function_node_recursive(cursor, function_name, source)? {
                    return Ok(Some(function_node));
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
        
        Ok(None)
    }
    
    /// Extract function calls from a function node
    fn extract_function_calls(&self, function_node: &Node, source: &[u8]) -> Result<Vec<FunctionCall>, ToolError> {
        let mut calls = Vec::new();
        let mut cursor = function_node.walk();
        
        self.extract_calls_recursive(&mut cursor, source, &mut calls)?;
        
        Ok(calls)
    }
    
    /// Recursively extract function calls from AST nodes
    fn extract_calls_recursive(
        &self,
        cursor: &mut TreeCursor,
        source: &[u8],
        calls: &mut Vec<FunctionCall>,
    ) -> Result<(), ToolError> {
        let node = cursor.node();
        
        // Check if this node represents a function call
        if self.is_function_call_node(&node) {
            if let Ok(call_info) = self.extract_call_info(&node, source) {
                calls.push(call_info);
            }
        }
        
        // Recursively analyze child nodes
        if cursor.goto_first_child() {
            loop {
                self.extract_calls_recursive(cursor, source, calls)?;
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
        
        Ok(())
    }
    
    /// Check if a node represents a function call
    fn is_function_call_node(&self, node: &Node) -> bool {
        matches!(
            node.kind(),
            "call_expression" | "function_call" | "method_call" |
            "invoke_expression" | "application" | "call"
        )
    }
    
    /// Extract call information from a function call node
    fn extract_call_info(&self, node: &Node, source: &[u8]) -> Result<FunctionCall, ToolError> {
        // Extract the function name being called
        let called_function = self.extract_called_function_name(node, source)?;
        
        Ok(FunctionCall {
            called_function,
            line: node.start_position().row + 1,
            column: node.start_position().column + 1,
            byte_offset: node.start_byte(),
        })
    }
    
    /// Extract the name of the function being called
    fn extract_called_function_name(&self, node: &Node, source: &[u8]) -> Result<String, ToolError> {
        // Look for the function identifier in the call expression
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "identifier" | "field_expression" | "member_expression" => {
                        let name_bytes = &source[child.byte_range()];
                        let name = std::str::from_utf8(name_bytes)
                            .map_err(|e| ToolError::InvalidQuery(format!("Invalid UTF-8: {}", e)))?;
                        return Ok(name.to_string());
                    }
                    _ => continue,
                }
            }
        }
        
        // Fallback: extract from node text
        let text = std::str::from_utf8(&source[node.byte_range()])
            .map_err(|e| ToolError::InvalidQuery(format!("Invalid UTF-8: {}", e)))?;
        
        // Extract function name from call text (basic heuristic)
        if let Some(open_paren) = text.find('(') {
            let name_part = &text[..open_paren];
            let name = name_part.trim().split_whitespace().last().unwrap_or("unknown");
            return Ok(name.to_string());
        }
        
        Ok("unknown".to_string())
    }
    
    /// Detect design patterns and anti-patterns in code using AST analysis
    /// 
    /// This method:
    /// 1. Searches for common design patterns (Singleton, Factory, Observer)
    /// 2. Detects anti-patterns and code smells (long methods, deep nesting)
    /// 3. Uses tree-sitter to analyze code structure semantically
    /// 4. Returns patterns with confidence scores and locations
    fn detect_patterns(
        &self,
        pattern: &PatternType,
    ) -> Result<Vec<DetectedPattern>, ToolError> {
        let mut detected_patterns = Vec::new();
        
        // Search through all cached semantic information for pattern detection
        for entry in self.semantic_cache.iter() {
            let file_path = entry.key();
            let semantic_index = entry.value();
            
            match pattern {
                PatternType::Singleton => {
                    self.detect_singleton_pattern(file_path, semantic_index, &mut detected_patterns)?;
                }
                PatternType::Factory => {
                    self.detect_factory_pattern(file_path, semantic_index, &mut detected_patterns)?;
                }
                PatternType::Observer => {
                    self.detect_observer_pattern(file_path, semantic_index, &mut detected_patterns)?;
                }
                PatternType::AntiPattern(anti_pattern) => {
                    self.detect_anti_pattern(file_path, semantic_index, anti_pattern, &mut detected_patterns)?;
                }
                PatternType::CodeSmell(smell_type) => {
                    self.detect_code_smell(file_path, semantic_index, smell_type, &mut detected_patterns)?;
                }
                _ => {
                    // For other pattern types, provide basic detection
                    self.detect_generic_pattern(file_path, semantic_index, pattern, &mut detected_patterns)?;
                }
            }
        }
        
        // If no cached data available, parse current directory
        if detected_patterns.is_empty() && self.semantic_cache.is_empty() {
            self.detect_patterns_in_current_dir(pattern, &mut detected_patterns)?;
        }
        
        // Sort by confidence score (highest first)
        detected_patterns.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(detected_patterns)
    }

    /// Generate a comprehensive refactoring plan for rename operations
    /// 
    /// This method:
    /// 1. Finds all references to the symbol across the specified scope
    /// 2. Creates a detailed change list with precise locations
    /// 3. Assesses the risk level based on symbol visibility and usage
    /// 4. Estimates impact on dependent code and external APIs
    fn plan_rename(
        &self,
        old_name: &str,
        new_name: &str,
        scope: &SearchScope,
    ) -> Result<RefactorPlan, ToolError> {
        let mut changes = Vec::new();
        let mut affected_files = std::collections::HashSet::new();
        let mut total_references = 0;
        let mut external_references = 0;
        
        // Find all references to the symbol in the specified scope
        let references = self.find_all_references_in_scope(old_name, scope)?;
        total_references = references.len();
        
        // Create RefactorChange for each reference
        for reference in references {
            affected_files.insert(reference.file.clone());
            
            // Read the file to get the actual text that needs to be changed
            let old_text = self.extract_text_at_location(&reference, old_name.len())?;
            let new_text = old_text.replace(old_name, new_name);
            
            // Determine change type based on context
            let change_type = self.determine_change_type(&reference, old_name)?;
            
            // Check if this is an external reference (exported symbol)
            if self.is_external_reference(&reference)? {
                external_references += 1;
            }
            
            changes.push(RefactorChange {
                file: reference.file.clone(),
                location: reference.clone(),
                old_text,
                new_text,
                change_type,
            });
        }
        
        // Assess risk level based on various factors
        let risk_level = self.assess_rename_risk(
            total_references,
            external_references,
            affected_files.len(),
            scope,
        );
        
        // Generate impact estimation
        let estimated_impact = format!(
            "Rename will affect {} references across {} files. {} external references detected.",
            total_references,
            affected_files.len(),
            external_references
        );
        
        Ok(RefactorPlan {
            changes,
            affected_files: affected_files.into_iter().collect(),
            risk_level,
            estimated_impact,
        })
    }

    /// Calculate comprehensive complexity metrics for a function
    /// 
    /// This method:
    /// 1. Calculates cyclomatic complexity (decision points)
    /// 2. Calculates cognitive complexity (with nesting penalties)
    /// 3. Measures lines of code and parameter counts
    /// 4. Generates specific recommendations for improvement
    fn calculate_complexity(&self, function_name: &str) -> Result<ComplexityReport, ToolError> {
        // Find the function definition
        let function_location = self.find_definition_global(function_name)?
            .ok_or_else(|| ToolError::InvalidQuery(format!("Function '{}' not found", function_name)))?;
        
        // Parse the file containing the function
        let engine = AstEngine::new(CompressionLevel::Medium);
        let rt = tokio::runtime::Handle::try_current()
            .or_else(|_| tokio::runtime::Runtime::new().map(|rt| rt.handle().clone()))
            .map_err(|e| ToolError::InvalidQuery(format!("Failed to create async runtime: {}", e)))?;
        
        let parsed_ast = rt.block_on(engine.parse_file(&function_location.file))
            .map_err(|e| ToolError::InvalidQuery(format!("Failed to parse file: {}", e)))?;
        
        // Find the function node in the AST
        let function_node = self.find_function_node(&parsed_ast, function_name)?
            .ok_or_else(|| ToolError::InvalidQuery(format!("Function node not found for '{}'", function_name)))?;
        
        let source = parsed_ast.source.as_bytes();
        
        // Calculate different complexity metrics
        let cyclomatic_complexity = self.calculate_cyclomatic_complexity(&function_node);
        let cognitive_complexity = self.calculate_cognitive_complexity(&function_node);
        let lines_of_code = self.calculate_lines_of_code(&function_node);
        let parameter_count = self.count_function_parameters(&function_node, source)?;
        let nesting_depth = self.calculate_max_nesting_depth(&function_node);
        
        // Generate recommendations based on complexity metrics
        let recommendations = self.generate_complexity_recommendations(
            cyclomatic_complexity,
            cognitive_complexity,
            lines_of_code,
            parameter_count,
            nesting_depth,
        );
        
        Ok(ComplexityReport {
            function: function_name.to_string(),
            cyclomatic_complexity,
            cognitive_complexity,
            lines_of_code,
            parameters: parameter_count,
            nesting_depth,
            recommendations,
        })
    }

    /// Detect unused code including unreferenced functions, variables, and imports
    /// 
    /// This method:
    /// 1. Analyzes symbol definitions and their usage patterns
    /// 2. Identifies unreferenced functions, variables, and imports
    /// 3. Detects unreachable code after return statements
    /// 4. Provides confidence scores based on analysis depth
    fn find_dead_code(&self, scope: &SearchScope) -> Result<Vec<DeadCodeItem>, ToolError> {
        let mut dead_code_items = Vec::new();
        
        // Get all files in the specified scope
        let files_to_analyze = self.get_files_in_scope(scope)?;
        
        // Build a comprehensive symbol usage map
        let mut symbol_definitions = std::collections::HashMap::new();
        let mut symbol_references = std::collections::HashMap::new();
        
        // First pass: collect all symbol definitions and references
        for file_path in &files_to_analyze {
            self.collect_symbols_from_file(
                file_path,
                &mut symbol_definitions,
                &mut symbol_references,
            )?;
        }
        
        // Second pass: identify unused symbols
        for (symbol_name, definition_location) in symbol_definitions {
            let reference_count = symbol_references.get(&symbol_name).map(|refs| refs.len()).unwrap_or(0);
            
            // Consider a symbol dead if it has no references or only self-references
            if reference_count == 0 || self.only_has_self_references(&symbol_name, &symbol_references) {
                let symbol_kind = self.determine_symbol_kind_from_location(&definition_location)?;
                let confidence = self.calculate_dead_code_confidence(&symbol_name, &definition_location, reference_count);
                let reason = self.generate_dead_code_reason(&symbol_name, reference_count, &symbol_kind);
                
                // Skip entry points and public APIs (lower confidence)
                if !self.is_entry_point_or_public_api(&symbol_name, &definition_location)? {
                    dead_code_items.push(DeadCodeItem {
                        symbol: symbol_name,
                        location: definition_location,
                        kind: symbol_kind,
                        confidence,
                        reason,
                    });
                }
            }
        }
        
        // Third pass: detect unreachable code (code after return statements)
        for file_path in &files_to_analyze {
            let unreachable_items = self.find_unreachable_code_in_file(file_path)?;
            dead_code_items.extend(unreachable_items);
        }
        
        // Fourth pass: detect unused imports
        for file_path in &files_to_analyze {
            let unused_imports = self.find_unused_imports_in_file(file_path, &symbol_references)?;
            dead_code_items.extend(unused_imports);
        }
        
        // Sort by confidence (highest first)
        dead_code_items.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(dead_code_items)
    }

    /// Detect duplicate code blocks using token-based similarity matching
    /// 
    /// This method:
    /// 1. Extracts normalized token sequences from all cached files
    /// 2. Computes similarity matrix using Jaccard coefficient
    /// 3. Groups similar code blocks above threshold
    /// 4. Suggests extraction opportunities for refactoring
    /// 5. Calculates lines and tokens duplicated
    fn detect_duplication(
        &self,
        threshold: f32,
    ) -> Result<Vec<DuplicationGroup>, ToolError> {
        let mut duplication_groups = Vec::new();
        let mut code_blocks = Vec::new();
        
        // Extract code blocks from all cached files
        for entry in self.semantic_cache.iter() {
            let file_path = entry.key();
            let semantic_index = entry.value();
            
            // Extract function bodies as potential duplication candidates
            for function in &semantic_index.functions {
                if let Ok(tokens) = self.extract_function_tokens(file_path, function) {
                    let block = CodeBlock {
                        location: Location {
                            file: file_path.clone(),
                            line: function.start_line,
                            column: 1,
                            byte_offset: 0,
                        },
                        tokens,
                        lines: function.end_line - function.start_line + 1,
                        function_name: function.name.clone(),
                        similarity: None,
                    };
                    code_blocks.push(block);
                }
            }
        }
        
        // If no cached data, analyze current directory
        if code_blocks.is_empty() {
            code_blocks = self.extract_code_blocks_from_current_dir()?;
        }
        
        // Find duplicates using similarity analysis
        let duplicates = self.find_similar_blocks(&code_blocks, threshold)?;
        
        // Group similar blocks and create duplication groups
        for duplicate_set in duplicates {
            if duplicate_set.len() >= 2 {
                let locations: Vec<Location> = duplicate_set.iter()
                    .map(|block| block.location.clone())
                    .collect();
                
                let lines = duplicate_set[0].lines;
                let tokens = duplicate_set[0].tokens.len();
                let similarity = duplicate_set[0].similarity.unwrap_or(1.0);
                
                // Generate extraction suggestion
                let extract_suggestion = self.generate_extraction_suggestion(&duplicate_set);
                
                duplication_groups.push(DuplicationGroup {
                    locations,
                    lines,
                    tokens,
                    similarity,
                    extract_suggestion,
                });
            }
        }
        
        // Sort by similarity and lines (most significant duplications first)
        duplication_groups.sort_by(|a, b| {
            b.similarity.partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.lines.cmp(&a.lines))
        });
        
        Ok(duplication_groups)
    }

    /// Build dependency graphs by analyzing module imports/exports
    /// 
    /// This method:
    /// 1. Analyzes import/export statements in the target module
    /// 2. Recursively tracks transitive dependencies
    /// 3. Detects circular dependencies using DFS cycle detection
    /// 4. Builds reverse dependency mapping
    /// 5. Returns comprehensive dependency graph
    fn analyze_dependencies(&self, module: &str) -> Result<DependencyGraph, ToolError> {
        let mut direct_deps = Vec::new();
        let mut transitive_deps = Vec::new();
        let mut reverse_deps = Vec::new();
        let mut circular_deps = Vec::new();
        
        // Find the module file(s)
        let module_files = self.find_module_files(module)?;
        
        if module_files.is_empty() {
            return Ok(DependencyGraph {
                module: module.to_string(),
                direct_deps,
                transitive_deps,
                reverse_deps,
                circular_deps,
            });
        }
        
        // Analyze direct dependencies from imports
        for module_file in &module_files {
            let file_deps = self.extract_imports_from_file(module_file)?;
            for dep in file_deps {
                if !direct_deps.contains(&dep) {
                    direct_deps.push(dep);
                }
            }
        }
        
        // Build transitive dependency graph using DFS
        let mut visited = std::collections::HashSet::new();
        let mut visiting = std::collections::HashSet::new();
        
        for dep in &direct_deps {
            self.collect_transitive_deps(
                dep,
                &mut transitive_deps,
                &mut visited,
                &mut visiting,
                &mut circular_deps,
            )?;
        }
        
        // Remove duplicates from transitive deps
        transitive_deps.sort();
        transitive_deps.dedup();
        
        // Build reverse dependencies by searching all cached files
        reverse_deps = self.find_reverse_dependencies(module)?;
        
        // Detect circular dependencies
        circular_deps = self.detect_circular_dependencies(module, &direct_deps)?;
        
        Ok(DependencyGraph {
            module: module.to_string(),
            direct_deps,
            transitive_deps,
            reverse_deps,
            circular_deps,
        })
    }

    /// Find all references to a symbol across all files in scope
    /// 
    /// This method:
    /// 1. Searches semantic cache for existing symbol information
    /// 2. Performs cross-file AST analysis for comprehensive coverage
    /// 3. Handles different reference types (call, assignment, import)
    /// 4. Tracks usage locations with precise file:line:column
    /// 5. Returns all reference locations sorted by file and line
    fn find_references(&self, symbol: &str) -> Result<Vec<Location>, ToolError> {
        let mut references = Vec::new();
        
        // First, search in cached semantic information
        for entry in self.semantic_cache.iter() {
            let file_path = entry.key();
            let semantic_index = entry.value();
            
            // Check existing symbol references in cache
            for symbol_info in &semantic_index.symbols {
                if symbol_info.name == symbol {
                    references.extend(symbol_info.references.iter().cloned());
                }
            }
            
            // Search for additional references in functions
            for function in &semantic_index.functions {
                if function.name == symbol {
                    // Add the function definition as a reference
                    references.push(Location {
                        file: file_path.clone(),
                        line: function.start_line,
                        column: 1,
                        byte_offset: 0,
                    });
                }
            }
            
            // Search in classes
            for class in &semantic_index.classes {
                if class.name == symbol {
                    references.push(Location {
                        file: file_path.clone(),
                        line: class.start_line,
                        column: 1,
                        byte_offset: 0,
                    });
                }
            }
        }
        
        // If no cached results or we need comprehensive search, parse files
        if references.is_empty() || self.semantic_cache.is_empty() {
            references.extend(self.search_references_in_all_files(symbol)?);
        }
        
        // Perform additional AST-based search for precise locations
        let precise_references = self.find_precise_references(symbol)?;
        references.extend(precise_references);
        
        // Remove duplicates and sort by file and line
        references.sort_by(|a, b| {
            a.file.cmp(&b.file)
                .then(a.line.cmp(&b.line))
                .then(a.column.cmp(&b.column))
        });
        references.dedup_by(|a, b| {
            a.file == b.file && a.line == b.line && a.column == b.column
        });
        
        Ok(references)
    }

    // Pattern detection helper methods
    
    /// Detect Singleton pattern: private constructor + static instance
    fn detect_singleton_pattern(
        &self,
        file_path: &PathBuf,
        semantic_index: &SemanticIndex,
        detected_patterns: &mut Vec<DetectedPattern>,
    ) -> Result<(), ToolError> {
        for class in &semantic_index.classes {
            let mut has_private_constructor = false;
            let mut has_static_instance = false;
            
            // Check for private constructor and static instance patterns
            if class.methods.iter().any(|m| m.contains("new") || m.contains("__init__")) {
                has_private_constructor = true;
            }
            
            if class.properties.iter().any(|p| p.contains("instance") || p.contains("singleton")) {
                has_static_instance = true;
            }
            
            if has_private_constructor && has_static_instance {
                detected_patterns.push(DetectedPattern {
                    pattern_type: PatternType::Singleton,
                    location: Location {
                        file: file_path.clone(),
                        line: class.start_line,
                        column: 1,
                        byte_offset: 0,
                    },
                    confidence: 0.7,
                    description: format!("Potential Singleton pattern in class '{}'.", class.name),
                });
            }
        }
        Ok(())
    }
    
    /// Detect Factory pattern: methods returning interface/trait implementations
    fn detect_factory_pattern(
        &self,
        file_path: &PathBuf,
        semantic_index: &SemanticIndex,
        detected_patterns: &mut Vec<DetectedPattern>,
    ) -> Result<(), ToolError> {
        for function in &semantic_index.functions {
            let name_lower = function.name.to_lowercase();
            if name_lower.contains("create") || name_lower.contains("make") || name_lower.contains("build") {
                if let Some(return_type) = &function.return_type {
                    if return_type.contains("impl") || return_type.contains("dyn") || return_type.contains("Box") {
                        detected_patterns.push(DetectedPattern {
                            pattern_type: PatternType::Factory,
                            location: Location {
                                file: file_path.clone(),
                                line: function.start_line,
                                column: 1,
                                byte_offset: 0,
                            },
                            confidence: 0.6,
                            description: format!("Potential Factory pattern in function '{}'.", function.name),
                        });
                    }
                }
            }
        }
        Ok(())
    }
    
    /// Detect Observer pattern: event listeners/subscribers
    fn detect_observer_pattern(
        &self,
        file_path: &PathBuf,
        semantic_index: &SemanticIndex,
        detected_patterns: &mut Vec<DetectedPattern>,
    ) -> Result<(), ToolError> {
        for function in &semantic_index.functions {
            let name_lower = function.name.to_lowercase();
            if name_lower.contains("subscribe") || name_lower.contains("listen") || 
               name_lower.contains("notify") || name_lower.contains("observe") ||
               name_lower.contains("on_") || name_lower.contains("add_listener") {
                detected_patterns.push(DetectedPattern {
                    pattern_type: PatternType::Observer,
                    location: Location {
                        file: file_path.clone(),
                        line: function.start_line,
                        column: 1,
                        byte_offset: 0,
                    },
                    confidence: 0.5,
                    description: format!("Potential Observer pattern in function '{}'.", function.name),
                });
            }
        }
        Ok(())
    }
    
    /// Detect anti-patterns
    fn detect_anti_pattern(
        &self,
        file_path: &PathBuf,
        semantic_index: &SemanticIndex,
        anti_pattern: &str,
        detected_patterns: &mut Vec<DetectedPattern>,
    ) -> Result<(), ToolError> {
        match anti_pattern {
            "god_object" => {
                for class in &semantic_index.classes {
                    if class.methods.len() > 20 || class.properties.len() > 15 {
                        detected_patterns.push(DetectedPattern {
                            pattern_type: PatternType::AntiPattern("god_object".to_string()),
                            location: Location {
                                file: file_path.clone(),
                                line: class.start_line,
                                column: 1,
                                byte_offset: 0,
                            },
                            confidence: 0.8,
                            description: format!("God Object anti-pattern detected in class '{}' with {} methods and {} properties.", 
                                               class.name, class.methods.len(), class.properties.len()),
                        });
                    }
                }
            }
            "spaghetti_code" => {
                for function in &semantic_index.functions {
                    if function.complexity > 20 {
                        detected_patterns.push(DetectedPattern {
                            pattern_type: PatternType::AntiPattern("spaghetti_code".to_string()),
                            location: Location {
                                file: file_path.clone(),
                                line: function.start_line,
                                column: 1,
                                byte_offset: 0,
                            },
                            confidence: 0.7,
                            description: format!("Spaghetti code detected in function '{}' with complexity {}.", 
                                               function.name, function.complexity),
                        });
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Detect code smells
    fn detect_code_smell(
        &self,
        file_path: &PathBuf,
        semantic_index: &SemanticIndex,
        smell_type: &str,
        detected_patterns: &mut Vec<DetectedPattern>,
    ) -> Result<(), ToolError> {
        match smell_type {
            "long_method" => {
                for function in &semantic_index.functions {
                    let line_count = function.end_line - function.start_line;
                    if line_count > 50 {
                        detected_patterns.push(DetectedPattern {
                            pattern_type: PatternType::CodeSmell("long_method".to_string()),
                            location: Location {
                                file: file_path.clone(),
                                line: function.start_line,
                                column: 1,
                                byte_offset: 0,
                            },
                            confidence: 0.8,
                            description: format!("Long method detected: '{}' has {} lines.", 
                                               function.name, line_count),
                        });
                    }
                }
            }
            "too_many_parameters" => {
                for function in &semantic_index.functions {
                    if function.parameters.len() > 6 {
                        detected_patterns.push(DetectedPattern {
                            pattern_type: PatternType::CodeSmell("too_many_parameters".to_string()),
                            location: Location {
                                file: file_path.clone(),
                                line: function.start_line,
                                column: 1,
                                byte_offset: 0,
                            },
                            confidence: 0.9,
                            description: format!("Too many parameters: '{}' has {} parameters.", 
                                               function.name, function.parameters.len()),
                        });
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Generic pattern detection fallback
    fn detect_generic_pattern(
        &self,
        file_path: &PathBuf,
        semantic_index: &SemanticIndex,
        pattern: &PatternType,
        detected_patterns: &mut Vec<DetectedPattern>,
    ) -> Result<(), ToolError> {
        for function in &semantic_index.functions {
            let name_lower = function.name.to_lowercase();
            let pattern_name = format!("{:?}", pattern).to_lowercase();
            
            if name_lower.contains(&pattern_name) {
                detected_patterns.push(DetectedPattern {
                    pattern_type: pattern.clone(),
                    location: Location {
                        file: file_path.clone(),
                        line: function.start_line,
                        column: 1,
                        byte_offset: 0,
                    },
                    confidence: 0.3,
                    description: format!("Potential {} pattern in function '{}'.", pattern_name, function.name),
                });
            }
        }
        Ok(())
    }
    
    /// Detect patterns in current directory when cache is empty
    fn detect_patterns_in_current_dir(
        &self,
        pattern: &PatternType,
        detected_patterns: &mut Vec<DetectedPattern>,
    ) -> Result<(), ToolError> {
        use std::fs;
        
        let current_dir = std::env::current_dir().map_err(|e| ToolError::Io(e))?;
        let entries = fs::read_dir(&current_dir).map_err(|e| ToolError::Io(e))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| ToolError::Io(e))?;
            let path = entry.path();
            
            if path.is_file() && self.is_supported_file(&path) {
                if let Ok(content) = fs::read_to_string(&path) {
                    self.detect_pattern_in_text(&path, &content, pattern, detected_patterns);
                }
            }
        }
        
        Ok(())
    }
    
    const fn generate_documentation(
        &self,
        _target: &DocumentationTarget,
    ) -> Result<String, ToolError> {
        // TODO: Implement documentation generation
        Err(ToolError::NotImplemented("generate_documentation"))
    }

    // Additional helper methods for pattern detection
    
    /// Check if file extension is supported
    fn is_supported_file(&self, path: &PathBuf) -> bool {
        if let Some(extension) = path.extension().and_then(|s| s.to_str()) {
            matches!(extension, "rs" | "py" | "js" | "ts" | "java" | "cpp" | "c" | "go" | "cs")
        } else {
            false
        }
    }
    
    /// Basic text-based pattern detection
    fn detect_pattern_in_text(
        &self,
        file_path: &PathBuf,
        content: &str,
        pattern: &PatternType,
        detected_patterns: &mut Vec<DetectedPattern>,
    ) {
        match pattern {
            PatternType::Singleton => {
                if content.contains("static") && content.contains("instance") {
                    detected_patterns.push(DetectedPattern {
                        pattern_type: PatternType::Singleton,
                        location: Location {
                            file: file_path.clone(),
                            line: 1,
                            column: 1,
                            byte_offset: 0,
                        },
                        confidence: 0.4,
                        description: "Potential Singleton pattern detected in file.".to_string(),
                    });
                }
            }
            PatternType::Factory => {
                if content.contains("create") || content.contains("factory") {
                    detected_patterns.push(DetectedPattern {
                        pattern_type: PatternType::Factory,
                        location: Location {
                            file: file_path.clone(),
                            line: 1,
                            column: 1,
                            byte_offset: 0,
                        },
                        confidence: 0.3,
                        description: "Potential Factory pattern detected in file.".to_string(),
                    });
                }
            }
            _ => {}
        }
    }
    
    // Rename refactoring helper methods
    
    /// Find all references to a symbol within the specified scope
    fn find_all_references_in_scope(
        &self,
        symbol: &str,
        scope: &SearchScope,
    ) -> Result<Vec<Location>, ToolError> {
        let mut references = Vec::new();
        
        match scope {
            SearchScope::File(file_path) => {
                self.find_references_in_file(symbol, file_path, &mut references)?;
            }
            SearchScope::Directory(dir_path) => {
                self.find_references_in_directory(symbol, dir_path, &mut references)?;
            }
            SearchScope::Module(module_name) => {
                let files = self.resolve_module_to_files(module_name)?;
                for file in files {
                    self.find_references_in_file(symbol, &file, &mut references)?;
                }
            }
            SearchScope::Global => {
                for entry in self.semantic_cache.iter() {
                    let file_path = entry.key();
                    self.find_references_in_file(symbol, file_path, &mut references)?;
                }
            }
        }
        
        Ok(references)
    }
    
    /// Find references in a single file
    fn find_references_in_file(
        &self,
        symbol: &str,
        file_path: &PathBuf,
        references: &mut Vec<Location>,
    ) -> Result<(), ToolError> {
        use std::fs;
        
        let content = fs::read_to_string(file_path).map_err(|e| ToolError::Io(e))?;
        
        for (line_num, line) in content.lines().enumerate() {
            if let Some(column) = line.find(symbol) {
                if self.is_whole_word_match(line, symbol, column) {
                    references.push(Location {
                        file: file_path.clone(),
                        line: line_num + 1,
                        column: column + 1,
                        byte_offset: 0,
                    });
                }
            }
        }
        
        Ok(())
    }
    
    /// Check if symbol match is a whole word
    fn is_whole_word_match(&self, line: &str, symbol: &str, position: usize) -> bool {
        let before_char = if position > 0 {
            line.chars().nth(position - 1)
        } else {
            None
        };
        
        let after_pos = position + symbol.len();
        let after_char = line.chars().nth(after_pos);
        
        let before_ok = before_char.map_or(true, |c| !c.is_alphanumeric() && c != '_');
        let after_ok = after_char.map_or(true, |c| !c.is_alphanumeric() && c != '_');
        
        before_ok && after_ok
    }
    
    /// Find references in directory recursively
    fn find_references_in_directory(
        &self,
        symbol: &str,
        dir_path: &PathBuf,
        references: &mut Vec<Location>,
    ) -> Result<(), ToolError> {
        use std::fs;
        
        let entries = fs::read_dir(dir_path).map_err(|e| ToolError::Io(e))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| ToolError::Io(e))?;
            let path = entry.path();
            
            if path.is_file() && self.is_supported_file(&path) {
                self.find_references_in_file(symbol, &path, references)?;
            } else if path.is_dir() {
                self.find_references_in_directory(symbol, &path, references)?;
            }
        }
        
        Ok(())
    }
    
    /// Resolve module name to file paths
    fn resolve_module_to_files(&self, module_name: &str) -> Result<Vec<PathBuf>, ToolError> {
        let current_dir = std::env::current_dir().map_err(|e| ToolError::Io(e))?;
        let mut files = Vec::new();
        
        let patterns = [
            format!("{}.rs", module_name),
            format!("{}.py", module_name),
            format!("{}.js", module_name),
            format!("{}.ts", module_name),
            format!("src/{}.rs", module_name),
            format!("lib/{}.py", module_name),
        ];
        
        for pattern in &patterns {
            let file_path = current_dir.join(pattern);
            if file_path.exists() {
                files.push(file_path);
            }
        }
        
        Ok(files)
    }
    
    /// Extract text at a specific location
    fn extract_text_at_location(
        &self,
        location: &Location,
        length: usize,
    ) -> Result<String, ToolError> {
        use std::fs;
        
        let content = fs::read_to_string(&location.file).map_err(|e| ToolError::Io(e))?;
        let lines: Vec<&str> = content.lines().collect();
        
        if location.line > 0 && location.line <= lines.len() {
            let line = lines[location.line - 1];
            let start_col = location.column.saturating_sub(1);
            let end_col = std::cmp::min(start_col + length, line.len());
            
            if start_col < line.len() {
                Ok(line[start_col..end_col].to_string())
            } else {
                Ok(String::new())
            }
        } else {
            Ok(String::new())
        }
    }
    
    /// Determine the type of change for refactoring
    fn determine_change_type(&self, location: &Location, _symbol: &str) -> Result<String, ToolError> {
        use std::fs;
        
        let content = fs::read_to_string(&location.file).map_err(|e| ToolError::Io(e))?;
        let lines: Vec<&str> = content.lines().collect();
        
        if location.line > 0 && location.line <= lines.len() {
            let line = lines[location.line - 1];
            
            if line.contains("fn ") || line.contains("function ") || line.contains("def ") {
                Ok("function_definition".to_string())
            } else if line.contains("class ") || line.contains("struct ") {
                Ok("type_definition".to_string())
            } else if line.contains("let ") || line.contains("var ") || line.contains("const ") {
                Ok("variable_definition".to_string())
            } else {
                Ok("reference".to_string())
            }
        } else {
            Ok("unknown".to_string())
        }
    }
    
    /// Check if a reference is external (exported symbol)
    fn is_external_reference(&self, location: &Location) -> Result<bool, ToolError> {
        use std::fs;
        
        let content = fs::read_to_string(&location.file).map_err(|e| ToolError::Io(e))?;
        let lines: Vec<&str> = content.lines().collect();
        
        if location.line > 0 && location.line <= lines.len() {
            let line = lines[location.line - 1];
            Ok(line.contains("pub ") || line.contains("export ") || line.contains("public "))
        } else {
            Ok(false)
        }
    }
    
    /// Assess risk level for rename operation
    fn assess_rename_risk(
        &self,
        total_references: usize,
        external_references: usize,
        affected_files: usize,
        scope: &SearchScope,
    ) -> RiskLevel {
        if external_references > 0 || affected_files > 10 || total_references > 50 {
            return RiskLevel::High;
        }
        
        if affected_files > 3 || total_references > 15 || matches!(scope, SearchScope::Global) {
            return RiskLevel::Medium;
        }
        
        RiskLevel::Low
    }
    
    const fn validate_syntax(
        &self,
        _file: &PathBuf,
        _language: &str,
    ) -> Result<ValidationReport, ToolError> {
        // TODO: Implement syntax validation
        Err(ToolError::NotImplemented("validate_syntax"))
    }

    // Complexity calculation helper methods
    
    /// Calculate cognitive complexity with nesting penalties
    fn calculate_cognitive_complexity(&self, function_node: &Node) -> usize {
        let mut complexity = 0;
        let mut nesting_level = 0;
        
        self.calculate_cognitive_complexity_recursive(function_node, &mut complexity, &mut nesting_level);
        
        complexity
    }
    
    /// Recursively calculate cognitive complexity
    fn calculate_cognitive_complexity_recursive(
        &self,
        node: &Node,
        complexity: &mut usize,
        nesting_level: &mut usize,
    ) {
        match node.kind() {
            "if_statement" | "if_expression" | "conditional_expression" => {
                *complexity += 1 + *nesting_level;
                *nesting_level += 1;
            }
            "while_statement" | "for_statement" | "loop_statement" => {
                *complexity += 1 + *nesting_level;
                *nesting_level += 1;
            }
            "match_expression" | "switch_statement" => {
                *complexity += 1 + *nesting_level;
                *nesting_level += 1;
            }
            "catch_clause" | "except_clause" => {
                *complexity += 1 + *nesting_level;
                *nesting_level += 1;
            }
            "&&" | "||" | "and" | "or" => {
                *complexity += 1;
            }
            _ => {}
        }
        
        let current_nesting = *nesting_level;
        
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.calculate_cognitive_complexity_recursive(&cursor.node(), complexity, nesting_level);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        
        if matches!(node.kind(), 
            "if_statement" | "while_statement" | "for_statement" | "loop_statement" |
            "match_expression" | "switch_statement" | "catch_clause" | "except_clause"
        ) {
            *nesting_level = current_nesting.saturating_sub(1);
        }
    }
    
    /// Calculate lines of code for a function
    fn calculate_lines_of_code(&self, function_node: &Node) -> usize {
        let start_line = function_node.start_position().row;
        let end_line = function_node.end_position().row;
        end_line - start_line + 1
    }
    
    /// Count function parameters
    fn count_function_parameters(&self, function_node: &Node, source: &[u8]) -> Result<usize, ToolError> {
        let parameters = self.extract_function_parameters(function_node, source)?;
        Ok(parameters.len())
    }
    
    /// Calculate maximum nesting depth in a function
    fn calculate_max_nesting_depth(&self, function_node: &Node) -> usize {
        let mut max_depth = 0;
        let mut current_depth = 0;
        
        self.calculate_nesting_depth_recursive(function_node, &mut current_depth, &mut max_depth);
        
        max_depth
    }
    
    /// Recursively calculate nesting depth
    fn calculate_nesting_depth_recursive(
        &self,
        node: &Node,
        current_depth: &mut usize,
        max_depth: &mut usize,
    ) {
        if matches!(node.kind(),
            "if_statement" | "while_statement" | "for_statement" | "loop_statement" |
            "match_expression" | "switch_statement" | "block" | "compound_statement"
        ) {
            *current_depth += 1;
            *max_depth = std::cmp::max(*max_depth, *current_depth);
        }
        
        let initial_depth = *current_depth;
        
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.calculate_nesting_depth_recursive(&cursor.node(), current_depth, max_depth);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        
        *current_depth = initial_depth;
    }
    
    /// Generate complexity-based recommendations
    fn generate_complexity_recommendations(
        &self,
        cyclomatic_complexity: usize,
        cognitive_complexity: usize,
        lines_of_code: usize,
        parameter_count: usize,
        nesting_depth: usize,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        if cyclomatic_complexity > 10 {
            recommendations.push(format!(
                "High cyclomatic complexity ({}). Consider breaking function into smaller functions.",
                cyclomatic_complexity
            ));
        }
        
        if cognitive_complexity > 15 {
            recommendations.push(format!(
                "High cognitive complexity ({}). Reduce nesting and simplify control flow.",
                cognitive_complexity
            ));
        }
        
        if lines_of_code > 50 {
            recommendations.push(format!(
                "Long function ({} lines). Consider extracting methods for better readability.",
                lines_of_code
            ));
        }
        
        if parameter_count > 6 {
            recommendations.push(format!(
                "Too many parameters ({}). Consider using parameter objects or builder pattern.",
                parameter_count
            ));
        }
        
        if nesting_depth > 4 {
            recommendations.push(format!(
                "Deep nesting ({}). Consider early returns or guard clauses.",
                nesting_depth
            ));
        }
        
        if recommendations.is_empty() {
            recommendations.push("Function complexity is within acceptable limits.".to_string());
        }
        
        recommendations
    }
    
    // Dead code detection helper methods
    
    /// Get all files in the specified scope
    fn get_files_in_scope(&self, scope: &SearchScope) -> Result<Vec<PathBuf>, ToolError> {
        match scope {
            SearchScope::File(file_path) => Ok(vec![file_path.clone()]),
            SearchScope::Directory(dir_path) => {
                self.get_files_in_directory(dir_path)
            }
            SearchScope::Module(module_name) => {
                self.resolve_module_to_files(module_name)
            }
            SearchScope::Global => {
                if !self.semantic_cache.is_empty() {
                    Ok(self.semantic_cache.iter().map(|entry| entry.key().clone()).collect())
                } else {
                    let current_dir = std::env::current_dir().map_err(|e| ToolError::Io(e))?;
                    self.get_files_in_directory(&current_dir)
                }
            }
        }
    }
    
    /// Get all supported files in directory recursively
    fn get_files_in_directory(&self, dir_path: &PathBuf) -> Result<Vec<PathBuf>, ToolError> {
        use std::fs;
        
        let mut files = Vec::new();
        let entries = fs::read_dir(dir_path).map_err(|e| ToolError::Io(e))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| ToolError::Io(e))?;
            let path = entry.path();
            
            if path.is_file() && self.is_supported_file(&path) {
                files.push(path);
            } else if path.is_dir() && !self.should_skip_directory(&path) {
                let mut subfiles = self.get_files_in_directory(&path)?;
                files.append(&mut subfiles);
            }
        }
        
        Ok(files)
    }
    
    /// Check if directory should be skipped
    fn should_skip_directory(&self, path: &PathBuf) -> bool {
        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
            matches!(name, "target" | "node_modules" | ".git" | "build" | "dist" | "__pycache__")
        } else {
            false
        }
    }
    
    /// Collect symbols from a file for dead code analysis
    fn collect_symbols_from_file(
        &self,
        file_path: &PathBuf,
        symbol_definitions: &mut std::collections::HashMap<String, Location>,
        symbol_references: &mut std::collections::HashMap<String, Vec<Location>>,
    ) -> Result<(), ToolError> {
        use std::fs;
        
        let content = fs::read_to_string(file_path).map_err(|e| ToolError::Io(e))?;
        
        let engine = AstEngine::new(CompressionLevel::Medium);
        let rt = tokio::runtime::Handle::try_current()
            .or_else(|_| tokio::runtime::Runtime::new().map(|rt| rt.handle().clone()))
            .map_err(|e| ToolError::InvalidQuery(format!("Failed to create async runtime: {}", e)))?;
        
        if let Ok(parsed_ast) = rt.block_on(engine.parse_file(file_path)) {
            let source = parsed_ast.source.as_bytes();
            let mut cursor = parsed_ast.tree.root_node().walk();
            
            self.collect_symbols_recursive(
                &mut cursor,
                source,
                file_path,
                symbol_definitions,
                symbol_references,
            )?;
        } else {
            self.collect_symbols_text_based(&content, file_path, symbol_definitions, symbol_references);
        }
        
        Ok(())
    }
    
    /// Generate improvement suggestions based on focus area
    /// 
    /// This method:
    /// 1. Analyzes code patterns based on the specified focus area
    /// 2. Identifies specific improvement opportunities
    /// 3. Provides concrete suggestions with impact levels
    /// 4. Includes example fixes where applicable
    /// 5. Prioritizes suggestions by potential impact
    fn suggest_improvements(
        &self,
        file: &PathBuf,
        focus: &ImprovementFocus,
    ) -> Result<Vec<Improvement>, ToolError> {
        let mut improvements = Vec::new();
        
        // Parse the target file for analysis
        let engine = AstEngine::new(CompressionLevel::Medium);
        let rt = tokio::runtime::Handle::try_current()
            .or_else(|_| tokio::runtime::Runtime::new().map(|rt| rt.handle().clone()))
            .map_err(|e| ToolError::InvalidQuery(format!("Failed to create async runtime: {}", e)))?;
        
        let parsed_ast = rt.block_on(engine.parse_file(file))
            .map_err(|e| ToolError::InvalidQuery(format!("Failed to parse file {}: {}", file.display(), e)))?;
        
        let source = parsed_ast.source.as_bytes();
        let mut cursor = parsed_ast.tree.root_node().walk();
        
        // Generate improvements based on focus area
        match focus {
            ImprovementFocus::Performance => {
                self.suggest_performance_improvements(&mut cursor, source, file, &mut improvements)?;
            }
            ImprovementFocus::Readability => {
                self.suggest_readability_improvements(&mut cursor, source, file, &mut improvements)?;
            }
            ImprovementFocus::Maintainability => {
                self.suggest_maintainability_improvements(&mut cursor, source, file, &mut improvements)?;
            }
            ImprovementFocus::Security => {
                self.suggest_security_improvements(&mut cursor, source, file, &mut improvements)?;
            }
            ImprovementFocus::TestCoverage => {
                self.suggest_test_coverage_improvements(&mut cursor, source, file, &mut improvements)?;
            }
            ImprovementFocus::ErrorHandling => {
                self.suggest_error_handling_improvements(&mut cursor, source, file, &mut improvements)?;
            }
        }
        
        // Sort improvements by impact level (Critical first)
        improvements.sort_by(|a, b| {
            match (&a.impact, &b.impact) {
                (ImpactLevel::Critical, ImpactLevel::Critical) => std::cmp::Ordering::Equal,
                (ImpactLevel::Critical, _) => std::cmp::Ordering::Less,
                (_, ImpactLevel::Critical) => std::cmp::Ordering::Greater,
                (ImpactLevel::High, ImpactLevel::High) => std::cmp::Ordering::Equal,
                (ImpactLevel::High, _) => std::cmp::Ordering::Less,
                (_, ImpactLevel::High) => std::cmp::Ordering::Greater,
                (ImpactLevel::Medium, ImpactLevel::Medium) => std::cmp::Ordering::Equal,
                (ImpactLevel::Medium, _) => std::cmp::Ordering::Less,
                (_, ImpactLevel::Medium) => std::cmp::Ordering::Greater,
                (ImpactLevel::Low, ImpactLevel::Low) => std::cmp::Ordering::Equal,
            }
        });
        
        Ok(improvements)
    }
    
    // ============================================================================
    // Helper Methods for Duplication Detection
    // ============================================================================
    
    /// Extract normalized tokens from a function for similarity analysis
    fn extract_function_tokens(&self, file_path: &PathBuf, function: &FunctionInfo) -> Result<Vec<String>, ToolError> {
        let engine = AstEngine::new(CompressionLevel::Medium);
        let rt = tokio::runtime::Handle::try_current()
            .or_else(|_| tokio::runtime::Runtime::new().map(|rt| rt.handle().clone()))
            .map_err(|e| ToolError::InvalidQuery(format!("Failed to create async runtime: {}", e)))?;
        
        let parsed_ast = rt.block_on(engine.parse_file(file_path))
            .map_err(|e| ToolError::InvalidQuery(format!("Failed to parse file: {}", e)))?;
        
        // Find the specific function node
        if let Some(function_node) = self.find_function_node(&parsed_ast, &function.name)? {
            let source = parsed_ast.source.as_bytes();
            let tokens = self.tokenize_node(&function_node, source)?;
            Ok(self.normalize_tokens(tokens))
        } else {
            Ok(Vec::new())
        }
    }
    
    /// Extract code blocks from current directory for analysis
    fn extract_code_blocks_from_current_dir(&self) -> Result<Vec<CodeBlock>, ToolError> {
        let mut code_blocks = Vec::new();
        let current_dir = std::env::current_dir()
            .map_err(|e| ToolError::Io(e))?;
        
        self.extract_code_blocks_from_directory(&current_dir, &mut code_blocks)?;
        Ok(code_blocks)
    }
    
    /// Recursively extract code blocks from a directory
    fn extract_code_blocks_from_directory(
        &self,
        dir_path: &PathBuf,
        code_blocks: &mut Vec<CodeBlock>,
    ) -> Result<(), ToolError> {
        use std::fs;
        
        let entries = fs::read_dir(dir_path)
            .map_err(|e| ToolError::Io(e))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| ToolError::Io(e))?;
            let path = entry.path();
            
            if path.is_file() && self.is_source_file(&path) {
                self.extract_code_blocks_from_file(&path, code_blocks)?;
            } else if path.is_dir() && !self.should_skip_directory(&path) {
                self.extract_code_blocks_from_directory(&path, code_blocks)?;
            }
        }
        
        Ok(())
    }
    
    /// Extract code blocks from a single file
    fn extract_code_blocks_from_file(
        &self,
        file_path: &PathBuf,
        code_blocks: &mut Vec<CodeBlock>,
    ) -> Result<(), ToolError> {
        let functions = self.extract_functions(file_path, "auto")?;
        
        for function in functions {
            if let Ok(tokens) = self.extract_function_tokens(file_path, &function) {
                if tokens.len() >= 10 { // Only consider functions with sufficient complexity
                    let block = CodeBlock {
                        location: Location {
                            file: file_path.clone(),
                            line: function.start_line,
                            column: 1,
                            byte_offset: 0,
                        },
                        tokens,
                        lines: function.end_line - function.start_line + 1,
                        function_name: function.name,
                        similarity: None,
                    };
                    code_blocks.push(block);
                }
            }
        }
        
        Ok(())
    }
    
    /// Find similar code blocks using Jaccard similarity
    fn find_similar_blocks(
        &self,
        code_blocks: &[CodeBlock],
        threshold: f32,
    ) -> Result<Vec<Vec<CodeBlock>>, ToolError> {
        let mut duplicate_groups = Vec::new();
        let mut processed = std::collections::HashSet::new();
        
        for (i, block_a) in code_blocks.iter().enumerate() {
            if processed.contains(&i) {
                continue;
            }
            
            let mut similar_group = vec![block_a.clone()];
            
            for (j, block_b) in code_blocks.iter().enumerate().skip(i + 1) {
                if processed.contains(&j) {
                    continue;
                }
                
                let similarity = self.calculate_jaccard_similarity(&block_a.tokens, &block_b.tokens);
                
                if similarity >= threshold {
                    let mut similar_block = block_b.clone();
                    similar_block.similarity = Some(similarity);
                    similar_group.push(similar_block);
                    processed.insert(j);
                }
            }
            
            if similar_group.len() > 1 {
                duplicate_groups.push(similar_group);
            }
            
            processed.insert(i);
        }
        
        Ok(duplicate_groups)
    }
    
    /// Calculate Jaccard similarity between two token sequences
    fn calculate_jaccard_similarity(&self, tokens_a: &[String], tokens_b: &[String]) -> f32 {
        let set_a: std::collections::HashSet<_> = tokens_a.iter().collect();
        let set_b: std::collections::HashSet<_> = tokens_b.iter().collect();
        
        let intersection_size = set_a.intersection(&set_b).count();
        let union_size = set_a.union(&set_b).count();
        
        if union_size == 0 {
            0.0
        } else {
            intersection_size as f32 / union_size as f32
        }
    }
    
    /// Generate extraction suggestion for duplicate code
    fn generate_extraction_suggestion(&self, duplicate_set: &[CodeBlock]) -> Option<String> {
        if duplicate_set.len() < 2 {
            return None;
        }
        
        let lines = duplicate_set[0].lines;
        let locations: Vec<String> = duplicate_set.iter()
            .map(|block| format!("{}:{}", block.location.file.display(), block.location.line))
            .collect();
        
        Some(format!(
            "Extract {} lines of duplicated code into a common function. Found in: {}",
            lines,
            locations.join(", ")
        ))
    }
    
    /// Tokenize an AST node into normalized tokens
    fn tokenize_node(&self, node: &Node, source: &[u8]) -> Result<Vec<String>, ToolError> {
        let mut tokens = Vec::new();
        let mut cursor = node.walk();
        
        self.tokenize_recursive(&mut cursor, source, &mut tokens)?;
        Ok(tokens)
    }
    
    /// Recursively tokenize AST nodes
    fn tokenize_recursive(
        &self,
        cursor: &mut TreeCursor,
        source: &[u8],
        tokens: &mut Vec<String>,
    ) -> Result<(), ToolError> {
        let node = cursor.node();
        
        // Skip comments and whitespace for normalization
        if matches!(node.kind(), "comment" | "line_comment" | "block_comment") {
            return Ok(());
        }
        
        // For leaf nodes, extract the token
        if node.child_count() == 0 {
            let token_text = std::str::from_utf8(&source[node.byte_range()])
                .map_err(|e| ToolError::InvalidQuery(format!("Invalid UTF-8: {}", e)))?;
            
            if !token_text.trim().is_empty() {
                tokens.push(self.normalize_token(token_text, node.kind()));
            }
        } else {
            // Recursively process children
            if cursor.goto_first_child() {
                loop {
                    self.tokenize_recursive(cursor, source, tokens)?;
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();
            }
        }
        
        Ok(())
    }
    
    /// Normalize tokens for better similarity comparison
    fn normalize_tokens(&self, tokens: Vec<String>) -> Vec<String> {
        tokens.into_iter()
            .filter(|token| !token.trim().is_empty())
            .map(|token| {
                // Normalize identifiers to generic names
                if self.is_identifier_token(&token) {
                    "VAR".to_string()
                } else if self.is_literal_token(&token) {
                    "LIT".to_string()
                } else {
                    token.to_lowercase()
                }
            })
            .collect()
    }
    
    /// Normalize a single token based on its type
    fn normalize_token(&self, token: &str, node_kind: &str) -> String {
        match node_kind {
            "identifier" | "variable" | "field" => "VAR".to_string(),
            "string_literal" | "number_literal" | "boolean_literal" => "LIT".to_string(),
            _ => token.to_lowercase(),
        }
    }
    
    /// Check if a token is an identifier
    fn is_identifier_token(&self, token: &str) -> bool {
        token.chars().all(|c| c.is_alphanumeric() || c == '_') && 
        token.chars().next().map(|c| c.is_alphabetic() || c == '_').unwrap_or(false)
    }
    
    /// Check if a token is a literal
    fn is_literal_token(&self, token: &str) -> bool {
        token.starts_with('"') || token.starts_with('\'') || 
        token.parse::<i64>().is_ok() || token.parse::<f64>().is_ok() ||
        matches!(token, "true" | "false" | "null" | "undefined")
    }
    
    /// Check if a file is a source code file
    fn is_source_file(&self, path: &PathBuf) -> bool {
        if let Some(extension) = path.extension() {
            matches!(extension.to_str(), 
                Some("rs") | Some("py") | Some("js") | Some("ts") | Some("java") |
                Some("cpp") | Some("c") | Some("h") | Some("go") | Some("cs") |
                Some("php") | Some("rb") | Some("swift") | Some("kt")
            )
        } else {
            false
        }
    }
    
    /// Check if a directory should be skipped during analysis
    fn should_skip_directory(&self, path: &PathBuf) -> bool {
        if let Some(dir_name) = path.file_name() {
            matches!(dir_name.to_str(),
                Some(".git") | Some("node_modules") | Some("target") | Some("build") |
                Some("dist") | Some(".vscode") | Some(".idea") | Some("__pycache__")
            )
        } else {
            false
        }
    }
    
    // ============================================================================
    // Helper Methods for Dependency Analysis
    // ============================================================================
    
    /// Find all files that belong to a module
    fn find_module_files(&self, module: &str) -> Result<Vec<PathBuf>, ToolError> {
        let mut module_files = Vec::new();
        let current_dir = std::env::current_dir()
            .map_err(|e| ToolError::Io(e))?;
        
        // Common module file patterns
        let patterns = [
            format!("{}.rs", module),
            format!("{}.py", module),
            format!("{}.js", module),
            format!("{}.ts", module),
            format!("{}/mod.rs", module),
            format!("{}/index.js", module),
            format!("{}/index.ts", module),
            format!("src/{}.rs", module),
            format!("lib/{}.py", module),
        ];
        
        for pattern in &patterns {
            let file_path = current_dir.join(pattern);
            if file_path.exists() && file_path.is_file() {
                module_files.push(file_path);
            }
        }
        
        // Also check if module is a directory
        let module_dir = current_dir.join(module);
        if module_dir.exists() && module_dir.is_dir() {
            self.find_source_files_in_directory(&module_dir, &mut module_files)?;
        }
        
        Ok(module_files)
    }
    
    /// Find all source files in a directory
    fn find_source_files_in_directory(
        &self,
        dir_path: &PathBuf,
        files: &mut Vec<PathBuf>,
    ) -> Result<(), ToolError> {
        use std::fs;
        
        let entries = fs::read_dir(dir_path)
            .map_err(|e| ToolError::Io(e))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| ToolError::Io(e))?;
            let path = entry.path();
            
            if path.is_file() && self.is_source_file(&path) {
                files.push(path);
            }
        }
        
        Ok(())
    }
    
    /// Extract import dependencies from a file
    fn extract_imports_from_file(&self, file_path: &PathBuf) -> Result<Vec<String>, ToolError> {
        let mut imports = Vec::new();
        
        let engine = AstEngine::new(CompressionLevel::Medium);
        let rt = tokio::runtime::Handle::try_current()
            .or_else(|_| tokio::runtime::Runtime::new().map(|rt| rt.handle().clone()))
            .map_err(|e| ToolError::InvalidQuery(format!("Failed to create async runtime: {}", e)))?;
        
        let parsed_ast = rt.block_on(engine.parse_file(file_path))
            .map_err(|e| ToolError::InvalidQuery(format!("Failed to parse file: {}", e)))?;
        
        let source = parsed_ast.source.as_bytes();
        let mut cursor = parsed_ast.tree.root_node().walk();
        
        self.extract_imports_recursive(&mut cursor, source, &mut imports)?;
        
        // Remove duplicates and sort
        imports.sort();
        imports.dedup();
        
        Ok(imports)
    }
    
    /// Recursively extract import statements from AST
    fn extract_imports_recursive(
        &self,
        cursor: &mut TreeCursor,
        source: &[u8],
        imports: &mut Vec<String>,
    ) -> Result<(), ToolError> {
        let node = cursor.node();
        
        // Check for import-related nodes
        if self.is_import_node(&node) {
            if let Ok(import_name) = self.extract_import_name(&node, source) {
                imports.push(import_name);
            }
        }
        
        // Recursively process children
        if cursor.goto_first_child() {
            loop {
                self.extract_imports_recursive(cursor, source, imports)?;
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
        
        Ok(())
    }
    
    /// Check if a node represents an import statement
    fn is_import_node(&self, node: &Node) -> bool {
        matches!(node.kind(),
            "use_declaration" | "import_statement" | "import_declaration" |
            "from_import_statement" | "import_from_statement" | "extern_crate_item" |
            "require_call" | "include_statement"
        )
    }
    
    /// Extract import name from an import node
    fn extract_import_name(&self, node: &Node, source: &[u8]) -> Result<String, ToolError> {
        // Try to find the module/package name being imported
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "module_path" | "scoped_identifier" | "string_literal" | "identifier" => {
                        let name_bytes = &source[child.byte_range()];
                        let name = std::str::from_utf8(name_bytes)
                            .map_err(|e| ToolError::InvalidQuery(format!("Invalid UTF-8: {}", e)))?;
                        return Ok(name.trim_matches('"').trim_matches('\'').to_string());
                    }
                    _ => continue,
                }
            }
        }
        
        // Fallback: extract from full node text
        let text = std::str::from_utf8(&source[node.byte_range()])
            .map_err(|e| ToolError::InvalidQuery(format!("Invalid UTF-8: {}", e)))?;
        
        // Basic pattern matching for common import formats using string operations
        let text = text.trim();
        if let Some(import_start) = text.find("import ").or_else(|| text.find("use ")).or_else(|| text.find("from ")) {
            let after_keyword = &text[import_start..];
            if let Some(space_pos) = after_keyword.find(' ') {
                let remaining = &after_keyword[space_pos + 1..];
                let module_name = remaining.split_whitespace().next().unwrap_or("unknown");
                return Ok(module_name.trim_matches(';').trim_matches(',').to_string());
            }
        }
        
        Ok("unknown".to_string())
    }
    
    /// Collect transitive dependencies using DFS
    fn collect_transitive_deps(
        &self,
        module: &str,
        transitive_deps: &mut Vec<String>,
        visited: &mut std::collections::HashSet<String>,
        visiting: &mut std::collections::HashSet<String>,
        circular_deps: &mut Vec<Vec<String>>,
    ) -> Result<(), ToolError> {
        if visited.contains(module) {
            return Ok(());
        }
        
        if visiting.contains(module) {
            // Circular dependency detected
            let cycle = vec![module.to_string()];
            circular_deps.push(cycle);
            return Ok(());
        }
        
        visiting.insert(module.to_string());
        
        // Find module files and analyze their dependencies
        let module_files = self.find_module_files(module)?;
        for file in module_files {
            let file_deps = self.extract_imports_from_file(&file)?;
            for dep in file_deps {
                if !transitive_deps.contains(&dep) {
                    transitive_deps.push(dep.clone());
                }
                
                // Recursively analyze this dependency
                self.collect_transitive_deps(
                    &dep,
                    transitive_deps,
                    visited,
                    visiting,
                    circular_deps,
                )?;
            }
        }
        
        visiting.remove(module);
        visited.insert(module.to_string());
        
        Ok(())
    }
    
    /// Find modules that depend on the given module (reverse dependencies)
    fn find_reverse_dependencies(&self, module: &str) -> Result<Vec<String>, ToolError> {
        let mut reverse_deps = Vec::new();
        
        // Search through all cached files for imports of this module
        for entry in self.semantic_cache.iter() {
            let file_path = entry.key();
            let semantic_index = entry.value();
            
            // Check imports in this file
            for import in &semantic_index.imports {
                if import.module == module || import.symbols.contains(&module.to_string()) {
                    // Extract module name from file path
                    if let Some(module_name) = self.extract_module_name_from_path(file_path) {
                        if !reverse_deps.contains(&module_name) {
                            reverse_deps.push(module_name);
                        }
                    }
                }
            }
        }
        
        Ok(reverse_deps)
    }
    
    /// Extract module name from file path
    fn extract_module_name_from_path(&self, file_path: &PathBuf) -> Option<String> {
        file_path.file_stem()
            .and_then(|stem| stem.to_str())
            .map(|s| s.to_string())
    }
    
    /// Detect circular dependencies in the dependency graph
    fn detect_circular_dependencies(
        &self,
        _module: &str,
        _direct_deps: &[String],
    ) -> Result<Vec<Vec<String>>, ToolError> {
        // This is a simplified implementation
        // In a real implementation, we'd use graph algorithms like DFS or Tarjan's algorithm
        Ok(Vec::new())
    }
    
    // ============================================================================
    // Helper Methods for Reference Finding
    // ============================================================================
    
    /// Search for references across all files in the current directory
    fn search_references_in_all_files(&self, symbol: &str) -> Result<Vec<Location>, ToolError> {
        let mut references = Vec::new();
        let current_dir = std::env::current_dir()
            .map_err(|e| ToolError::Io(e))?;
        
        self.search_references_in_directory(&current_dir, symbol, &mut references)?;
        Ok(references)
    }
    
    /// Recursively search for references in a directory
    fn search_references_in_directory(
        &self,
        dir_path: &PathBuf,
        symbol: &str,
        references: &mut Vec<Location>,
    ) -> Result<(), ToolError> {
        use std::fs;
        
        let entries = fs::read_dir(dir_path)
            .map_err(|e| ToolError::Io(e))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| ToolError::Io(e))?;
            let path = entry.path();
            
            if path.is_file() && self.is_source_file(&path) {
                self.search_references_in_file(&path, symbol, references)?;
            } else if path.is_dir() && !self.should_skip_directory(&path) {
                self.search_references_in_directory(&path, symbol, references)?;
            }
        }
        
        Ok(())
    }
    
    /// Search for references in a single file
    fn search_references_in_file(
        &self,
        file_path: &PathBuf,
        symbol: &str,
        references: &mut Vec<Location>,
    ) -> Result<(), ToolError> {
        let engine = AstEngine::new(CompressionLevel::Medium);
        let rt = tokio::runtime::Handle::try_current()
            .or_else(|_| tokio::runtime::Runtime::new().map(|rt| rt.handle().clone()))
            .map_err(|e| ToolError::InvalidQuery(format!("Failed to create async runtime: {}", e)))?;
        
        let parsed_ast = rt.block_on(engine.parse_file(file_path))
            .map_err(|e| ToolError::InvalidQuery(format!("Failed to parse file: {}", e)))?;
        
        let source = parsed_ast.source.as_bytes();
        let mut cursor = parsed_ast.tree.root_node().walk();
        
        self.search_symbol_references_recursive(&mut cursor, symbol, source, file_path, references)?;
        
        Ok(())
    }
    
    /// Recursively search for symbol references in AST
    fn search_symbol_references_recursive(
        &self,
        cursor: &mut TreeCursor,
        symbol: &str,
        source: &[u8],
        file_path: &PathBuf,
        references: &mut Vec<Location>,
    ) -> Result<(), ToolError> {
        let node = cursor.node();
        
        // Check if this node is a reference to our symbol
        if self.is_potential_reference_node(&node) {
            if let Ok(node_text) = std::str::from_utf8(&source[node.byte_range()]) {
                if node_text == symbol {
                    references.push(Location {
                        file: file_path.clone(),
                        line: node.start_position().row + 1,
                        column: node.start_position().column + 1,
                        byte_offset: node.start_byte(),
                    });
                }
            }
        }
        
        // Recursively search children
        if cursor.goto_first_child() {
            loop {
                self.search_symbol_references_recursive(cursor, symbol, source, file_path, references)?;
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
        
        Ok(())
    }
    
    /// Check if a node could be a reference to a symbol
    fn is_potential_reference_node(&self, node: &Node) -> bool {
        matches!(node.kind(),
            "identifier" | "variable" | "field_identifier" |
            "type_identifier" | "function_name" | "method_name" |
            "property_identifier" | "member_expression"
        )
    }
    
    /// Find precise references using advanced AST analysis
    fn find_precise_references(&self, _symbol: &str) -> Result<Vec<Location>, ToolError> {
        let references = Vec::new();
        
        // This is a more sophisticated reference finding that considers scope and context
        // For now, we'll return empty and rely on the basic search
        // In a full implementation, this would use semantic analysis to distinguish
        // between different symbols with the same name in different scopes
        
        Ok(references)
    }
    
    // ============================================================================
    // Helper Methods for Improvement Suggestions
    // ============================================================================
    
    /// Suggest performance improvements
    fn suggest_performance_improvements(
        &self,
        cursor: &mut TreeCursor,
        source: &[u8],
        file_path: &PathBuf,
        improvements: &mut Vec<Improvement>,
    ) -> Result<(), ToolError> {
        self.analyze_performance_recursive(cursor, source, file_path, improvements)?;
        Ok(())
    }
    
    /// Recursively analyze performance issues
    fn analyze_performance_recursive(
        &self,
        cursor: &mut TreeCursor,
        source: &[u8],
        file_path: &PathBuf,
        improvements: &mut Vec<Improvement>,
    ) -> Result<(), ToolError> {
        let node = cursor.node();
        
        // Check for performance anti-patterns
        match node.kind() {
            "for_statement" | "while_statement" => {
                // Check for nested loops (O(n²) complexity)
                if self.has_nested_loops(&node) {
                    improvements.push(Improvement {
                        location: Location {
                            file: file_path.clone(),
                            line: node.start_position().row + 1,
                            column: node.start_position().column + 1,
                            byte_offset: node.start_byte(),
                        },
                        category: ImprovementFocus::Performance,
                        description: "Nested loops detected - consider optimizing time complexity".to_string(),
                        suggested_change: Some("Consider using hash maps, caching, or algorithm optimization".to_string()),
                        impact: ImpactLevel::High,
                    });
                }
            }
            "function_declaration" | "function_definition" => {
                // Check for functions that are too long
                let lines = node.end_position().row - node.start_position().row + 1;
                if lines > 50 {
                    improvements.push(Improvement {
                        location: Location {
                            file: file_path.clone(),
                            line: node.start_position().row + 1,
                            column: node.start_position().column + 1,
                            byte_offset: node.start_byte(),
                        },
                        category: ImprovementFocus::Performance,
                        description: format!("Function is {} lines long - consider breaking it down", lines),
                        suggested_change: Some("Split into smaller, focused functions".to_string()),
                        impact: ImpactLevel::Medium,
                    });
                }
            }
            _ => {}
        }
        
        // Recursively analyze children
        if cursor.goto_first_child() {
            loop {
                self.analyze_performance_recursive(cursor, source, file_path, improvements)?;
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
        
        Ok(())
    }
    
    /// Check if a loop has nested loops
    fn has_nested_loops(&self, loop_node: &Node) -> bool {
        let mut cursor = loop_node.walk();
        
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if matches!(child.kind(), "for_statement" | "while_statement") {
                    return true;
                }
                
                // Recursively check children
                if self.has_nested_loops(&child) {
                    return true;
                }
                
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        
        false
    }
    
    /// Suggest readability improvements
    fn suggest_readability_improvements(
        &self,
        _cursor: &mut TreeCursor,
        _source: &[u8],
        _file_path: &PathBuf,
        _improvements: &mut Vec<Improvement>,
    ) -> Result<(), ToolError> {
        // Implementation for readability improvements
        // Would check for long variable names, complex expressions, etc.
        Ok(())
    }
    
    /// Suggest maintainability improvements
    fn suggest_maintainability_improvements(
        &self,
        _cursor: &mut TreeCursor,
        _source: &[u8],
        _file_path: &PathBuf,
        _improvements: &mut Vec<Improvement>,
    ) -> Result<(), ToolError> {
        // Implementation for maintainability improvements
        // Would check for code duplication, tight coupling, etc.
        Ok(())
    }
    
    /// Suggest security improvements
    fn suggest_security_improvements(
        &self,
        _cursor: &mut TreeCursor,
        _source: &[u8],
        _file_path: &PathBuf,
        _improvements: &mut Vec<Improvement>,
    ) -> Result<(), ToolError> {
        // Implementation for security improvements
        // Would check for SQL injection, XSS vulnerabilities, etc.
        Ok(())
    }
    
    /// Suggest test coverage improvements
    fn suggest_test_coverage_improvements(
        &self,
        _cursor: &mut TreeCursor,
        _source: &[u8],
        _file_path: &PathBuf,
        _improvements: &mut Vec<Improvement>,
    ) -> Result<(), ToolError> {
        // Implementation for test coverage improvements
        // Would identify untested functions and suggest test cases
        Ok(())
    }
    
    /// Suggest error handling improvements
    fn suggest_error_handling_improvements(
        &self,
        _cursor: &mut TreeCursor,
        _source: &[u8],
        _file_path: &PathBuf,
        _improvements: &mut Vec<Improvement>,
    ) -> Result<(), ToolError> {
        // Implementation for error handling improvements
        // Would check for proper error handling patterns
        Ok(())
    }
    
    // ============================================================================
    // Additional Helper Methods
    // ============================================================================
    
    /// Get files in the specified scope for analysis
    fn get_files_in_scope(&self, scope: &SearchScope) -> Result<Vec<PathBuf>, ToolError> {
        match scope {
            SearchScope::File(file_path) => Ok(vec![file_path.clone()]),
            SearchScope::Directory(dir_path) => {
                let mut files = Vec::new();
                self.collect_source_files_recursive(dir_path, &mut files)?;
                Ok(files)
            }
            SearchScope::Module(module_name) => {
                self.find_module_files(module_name)
            }
            SearchScope::Global => {
                let current_dir = std::env::current_dir()
                    .map_err(|e| ToolError::Io(e))?;
                let mut files = Vec::new();
                self.collect_source_files_recursive(&current_dir, &mut files)?;
                Ok(files)
            }
        }
    }
    
    /// Recursively collect source files from a directory
    fn collect_source_files_recursive(
        &self,
        dir_path: &PathBuf,
        files: &mut Vec<PathBuf>,
    ) -> Result<(), ToolError> {
        use std::fs;
        
        let entries = fs::read_dir(dir_path)
            .map_err(|e| ToolError::Io(e))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| ToolError::Io(e))?;
            let path = entry.path();
            
            if path.is_file() && self.is_source_file(&path) {
                files.push(path);
            } else if path.is_dir() && !self.should_skip_directory(&path) {
                self.collect_source_files_recursive(&path, files)?;
            }
        }
        
        Ok(())
    }
    
    /// Collect symbols from a file for dead code analysis
    fn collect_symbols_from_file(
        &self,
        file_path: &PathBuf,
        symbol_definitions: &mut std::collections::HashMap<String, Location>,
        symbol_references: &mut std::collections::HashMap<String, Vec<Location>>,
    ) -> Result<(), ToolError> {
        // Check if we have cached semantic information for this file
        if let Some(cached_index) = self.semantic_cache.get(file_path) {
            // Use cached information
            for symbol in &cached_index.symbols {
                let definition_location = Location {
                    file: file_path.clone(),
                    line: symbol.line,
                    column: symbol.column,
                    byte_offset: 0,
                };
                symbol_definitions.insert(symbol.name.clone(), definition_location);
                
                // Add references from cache
                symbol_references.insert(symbol.name.clone(), symbol.references.clone());
            }
        } else {
            // Parse the file and extract symbols
            let engine = AstEngine::new(CompressionLevel::Medium);
            let rt = tokio::runtime::Handle::try_current()
                .or_else(|_| tokio::runtime::Runtime::new().map(|rt| rt.handle().clone()))
                .map_err(|e| ToolError::InvalidQuery(format!("Failed to create async runtime: {}", e)))?;
            
            let parsed_ast = rt.block_on(engine.parse_file(file_path))
                .map_err(|e| ToolError::InvalidQuery(format!("Failed to parse file: {}", e)))?;
            
            // Extract symbols and cache the results
            self.cache_semantic_info(file_path, &parsed_ast)?;
            
            // Recursively call this method to use the cached information
            self.collect_symbols_from_file(file_path, symbol_definitions, symbol_references)?;
        }
        
        Ok(())
    }
    
    /// Check if symbol only has self-references
    fn only_has_self_references(
        &self,
        _symbol_name: &str,
        _symbol_references: &std::collections::HashMap<String, Vec<Location>>,
    ) -> bool {
        // Simplified implementation - in practice, would check if references
        // are only from the definition site itself
        false
    }
    
    /// Determine symbol kind from location
    fn determine_symbol_kind_from_location(&self, _location: &Location) -> Result<SymbolKind, ToolError> {
        // Simplified implementation - would parse the location and determine the actual symbol kind
        Ok(SymbolKind::Variable)
    }
    
    /// Calculate confidence for dead code detection
    fn calculate_dead_code_confidence(
        &self,
        _symbol_name: &str,
        _location: &Location,
        reference_count: usize,
    ) -> f32 {
        if reference_count == 0 {
            0.9 // High confidence for truly unreferenced symbols
        } else {
            0.5 // Medium confidence for symbols with some references
        }
    }
    
    /// Generate reason for dead code detection
    fn generate_dead_code_reason(
        &self,
        _symbol_name: &str,
        reference_count: usize,
        symbol_kind: &SymbolKind,
    ) -> String {
        match reference_count {
            0 => format!("{:?} is never referenced", symbol_kind),
            _ => format!("{:?} has limited usage", symbol_kind),
        }
    }
    
    /// Check if symbol is an entry point or public API
    fn is_entry_point_or_public_api(
        &self,
        symbol_name: &str,
        _location: &Location,
    ) -> Result<bool, ToolError> {
        // Entry points like main, test functions, or exported symbols
        let entry_points = ["main", "test_", "benchmark_"];
        for entry_point in &entry_points {
            if symbol_name.starts_with(entry_point) {
                return Ok(true);
            }
        }
        
        // Check if it's an exported symbol (simplified)
        Ok(symbol_name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false))
    }
    
    /// Find unreachable code in a file
    fn find_unreachable_code_in_file(&self, _file_path: &PathBuf) -> Result<Vec<DeadCodeItem>, ToolError> {
        // Simplified implementation - would analyze control flow to find unreachable code
        Ok(Vec::new())
    }
    
    /// Find unused imports in a file
    fn find_unused_imports_in_file(
        &self,
        _file_path: &PathBuf,
        _symbol_references: &std::collections::HashMap<String, Vec<Location>>,
    ) -> Result<Vec<DeadCodeItem>, ToolError> {
        // Simplified implementation - would check if imported symbols are actually used
        Ok(Vec::new())
    }
    
    /// Recursively collect symbols from AST
    fn collect_symbols_recursive(
        &self,
        cursor: &mut TreeCursor,
        source: &[u8],
        file_path: &PathBuf,
        symbol_definitions: &mut std::collections::HashMap<String, Location>,
        symbol_references: &mut std::collections::HashMap<String, Vec<Location>>,
    ) -> Result<(), ToolError> {
        let node = cursor.node();
        
        if self.is_symbol_definition_node(&node) {
            if let Ok(name) = self.extract_symbol_name_from_node(&node, source) {
                let location = Location {
                    file: file_path.clone(),
                    line: node.start_position().row + 1,
                    column: node.start_position().column + 1,
                    byte_offset: node.start_byte(),
                };
                symbol_definitions.insert(name, location);
            }
        }
        
        if node.kind() == "identifier" {
            if let Ok(name) = std::str::from_utf8(&source[node.byte_range()]) {
                let location = Location {
                    file: file_path.clone(),
                    line: node.start_position().row + 1,
                    column: node.start_position().column + 1,
                    byte_offset: node.start_byte(),
                };
                
                symbol_references
                    .entry(name.to_string())
                    .or_insert_with(Vec::new)
                    .push(location);
            }
        }
        
        if cursor.goto_first_child() {
            loop {
                self.collect_symbols_recursive(
                    cursor,
                    source,
                    file_path,
                    symbol_definitions,
                    symbol_references,
                )?;
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
        
        Ok(())
    }
    
    /// Text-based symbol collection fallback
    fn collect_symbols_text_based(
        &self,
        content: &str,
        file_path: &PathBuf,
        symbol_definitions: &mut std::collections::HashMap<String, Location>,
        _symbol_references: &mut std::collections::HashMap<String, Vec<Location>>,
    ) {
        for (line_num, line) in content.lines().enumerate() {
            if let Some(name) = self.extract_function_name_from_line(line) {
                let location = Location {
                    file: file_path.clone(),
                    line: line_num + 1,
                    column: 1,
                    byte_offset: 0,
                };
                symbol_definitions.insert(name, location);
            }
        }
    }
    
    /// Extract function name from line using simple patterns
    fn extract_function_name_from_line(&self, line: &str) -> Option<String> {
        let trimmed = line.trim();
        
        if let Some(start) = trimmed.find("fn ") {
            let after_fn = &trimmed[start + 3..];
            if let Some(end) = after_fn.find('(') {
                return Some(after_fn[..end].trim().to_string());
            }
        }
        
        if let Some(start) = trimmed.find("def ") {
            let after_def = &trimmed[start + 4..];
            if let Some(end) = after_def.find('(') {
                return Some(after_def[..end].trim().to_string());
            }
        }
        
        if let Some(start) = trimmed.find("function ") {
            let after_function = &trimmed[start + 9..];
            if let Some(end) = after_function.find('(') {
                return Some(after_function[..end].trim().to_string());
            }
        }
        
        None
    }
    
    /// Check if symbol only has self-references
    fn only_has_self_references(
        &self,
        _symbol_name: &str,
        symbol_references: &std::collections::HashMap<String, Vec<Location>>,
    ) -> bool {
        if let Some(references) = symbol_references.get(_symbol_name) {
            references.len() <= 1
        } else {
            true
        }
    }
    
    /// Determine symbol kind from location context
    fn determine_symbol_kind_from_location(&self, location: &Location) -> Result<SymbolKind, ToolError> {
        use std::fs;
        
        let content = fs::read_to_string(&location.file).map_err(|e| ToolError::Io(e))?;
        let lines: Vec<&str> = content.lines().collect();
        
        if location.line > 0 && location.line <= lines.len() {
            let line = lines[location.line - 1];
            
            if line.contains("fn ") || line.contains("function ") || line.contains("def ") {
                Ok(SymbolKind::Function)
            } else if line.contains("class ") || line.contains("struct ") {
                Ok(SymbolKind::Class)
            } else if line.contains("const ") {
                Ok(SymbolKind::Constant)
            } else {
                Ok(SymbolKind::Variable)
            }
        } else {
            Ok(SymbolKind::Variable)
        }
    }
    
    /// Calculate confidence for dead code detection
    fn calculate_dead_code_confidence(
        &self,
        _symbol_name: &str,
        location: &Location,
        reference_count: usize,
    ) -> f32 {
        let mut confidence = 0.9;
        
        if location.file.to_string_lossy().contains("test") {
            confidence *= 0.7;
        }
        
        if reference_count == 0 {
            confidence *= 1.0;
        } else {
            confidence *= 0.6;
        }
        
        confidence
    }
    
    /// Generate reason for dead code detection
    fn generate_dead_code_reason(
        &self,
        symbol_name: &str,
        reference_count: usize,
        symbol_kind: &SymbolKind,
    ) -> String {
        match reference_count {
            0 => format!("{:?} '{}' is never referenced", symbol_kind, symbol_name),
            1 => format!("{:?} '{}' is only referenced in its own definition", symbol_kind, symbol_name),
            _ => format!("{:?} '{}' has very limited usage", symbol_kind, symbol_name),
        }
    }
    
    /// Check if symbol is entry point or public API
    fn is_entry_point_or_public_api(
        &self,
        symbol_name: &str,
        location: &Location,
    ) -> Result<bool, ToolError> {
        if matches!(symbol_name, "main" | "start" | "init" | "__init__" | "constructor") {
            return Ok(true);
        }
        
        self.is_external_reference(location)
    }
    
    /// Find unreachable code in a file
    fn find_unreachable_code_in_file(
        &self,
        file_path: &PathBuf,
    ) -> Result<Vec<DeadCodeItem>, ToolError> {
        use std::fs;
        
        let mut unreachable_items = Vec::new();
        let content = fs::read_to_string(file_path).map_err(|e| ToolError::Io(e))?;
        
        let mut found_return = false;
        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            
            if trimmed.starts_with("return") || trimmed.starts_with("throw") || trimmed.starts_with("panic!") {
                found_return = true;
                continue;
            }
            
            if found_return && !trimmed.is_empty() && !trimmed.starts_with('}') && !trimmed.starts_with("fn ") && !trimmed.starts_with("function ") {
                unreachable_items.push(DeadCodeItem {
                    symbol: format!("line_{}", line_num + 1),
                    location: Location {
                        file: file_path.clone(),
                        line: line_num + 1,
                        column: 1,
                        byte_offset: 0,
                    },
                    kind: SymbolKind::Variable,
                    confidence: 0.8,
                    reason: "Code after return statement is unreachable".to_string(),
                });
            }
            
            if trimmed.starts_with('}') || trimmed.starts_with("fn ") || trimmed.starts_with("function ") {
                found_return = false;
            }
        }
        
        Ok(unreachable_items)
    }
    
    /// Find unused imports in a file
    fn find_unused_imports_in_file(
        &self,
        file_path: &PathBuf,
        symbol_references: &std::collections::HashMap<String, Vec<Location>>,
    ) -> Result<Vec<DeadCodeItem>, ToolError> {
        use std::fs;
        
        let mut unused_imports = Vec::new();
        let content = fs::read_to_string(file_path).map_err(|e| ToolError::Io(e))?;
        
        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            
            if trimmed.starts_with("use ") || trimmed.starts_with("import ") || trimmed.starts_with("from ") {
                if let Some(imported_symbol) = self.extract_imported_symbol(trimmed) {
                    if !symbol_references.contains_key(&imported_symbol) {
                        unused_imports.push(DeadCodeItem {
                            symbol: imported_symbol.clone(),
                            location: Location {
                                file: file_path.clone(),
                                line: line_num + 1,
                                column: 1,
                                byte_offset: 0,
                            },
                            kind: SymbolKind::Module,
                            confidence: 0.9,
                            reason: format!("Import '{}' is never used", imported_symbol),
                        });
                    }
                }
            }
        }
        
        Ok(unused_imports)
    }
    
    /// Extract symbol name from import statement
    fn extract_imported_symbol(&self, import_line: &str) -> Option<String> {
        if import_line.starts_with("use ") {
            if let Some(semicolon) = import_line.rfind(';') {
                let import_part = &import_line[4..semicolon].trim();
                if let Some(last_part) = import_part.split("::").last() {
                    return Some(last_part.trim().to_string());
                }
            }
        }
        
        if import_line.starts_with("import ") {
            let parts: Vec<&str> = import_line.split_whitespace().collect();
            if parts.len() >= 2 {
                return Some(parts[1].to_string());
            }
        }
        
        if import_line.starts_with("from ") && import_line.contains(" import ") {
            if let Some(import_pos) = import_line.find(" import ") {
                let after_import = &import_line[import_pos + 8..].trim();
                return Some(after_import.split(',').next()?.trim().to_string());
            }
        }
        
        None
    }
}

impl CodeTool for ASTAgentTools {
    type Query = AgentToolOp;
    type Output = AgentToolResult;

    fn search(&self, query: Self::Query) -> Result<Self::Output, ToolError> {
        // For compatibility with CodeTool trait, delegate to execute
        let mut tools = self.clone();
        tools.execute(query)
    }
}

/// Helper structure for tracking function calls during analysis
#[derive(Debug, Clone)]
struct FunctionCall {
    called_function: String,
    line: usize,
    column: usize,
    byte_offset: usize,
}

/// Structure for representing code blocks in duplication analysis
#[derive(Debug, Clone)]
struct CodeBlock {
    location: Location,
    tokens: Vec<String>,
    lines: usize,
    function_name: String,
    similarity: f32,
}
