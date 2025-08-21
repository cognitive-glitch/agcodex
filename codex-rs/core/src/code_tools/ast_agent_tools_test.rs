//! Basic tests for AST-based agent tools
//!
//! This module provides foundational tests for the AST agent tools system.
//! As the AST tools are implemented, these tests can be expanded.

use super::ast_agent_tools::*;
use super::{CodeTool, ToolError};
use std::path::PathBuf;
use tempfile::NamedTempFile;
use std::io::Write;

// Test fixtures
mod fixtures {
    use super::*;
    
    pub fn create_simple_rust_file() -> (NamedTempFile, String) {
        let mut file = NamedTempFile::new().unwrap();
        let content = r#"
pub struct Calculator {
    value: f64,
}

impl Calculator {
    pub fn new() -> Self {
        Self { value: 0.0 }
    }
    
    pub fn add(&mut self, x: f64) -> f64 {
        self.value += x;
        self.value
    }
    
    pub fn get_value(&self) -> f64 {
        self.value
    }
}

fn unused_function() {
    println!("This function is never called");
}
"#;
        file.write_all(content.as_bytes()).unwrap();
        (file, content.to_string())
    }
}

#[cfg(test)]
mod basic_tests {
    use super::*;
    use super::fixtures::*;

    #[tokio::test]
    async fn test_ast_agent_tools_creation() {
        let tools = ASTAgentTools::new();
        
        // Test that the tools can be created successfully
        assert!(tools.parsers.is_empty());
        assert!(tools.semantic_cache.is_empty());
    }

    #[tokio::test]
    async fn test_ast_tools_with_simple_rust() {
        let tools = ASTAgentTools::new();
        let (file, _content) = create_simple_rust_file();
        let file_path = PathBuf::from(file.path());
        
        // Test basic functionality - this will need to be updated as the tools are implemented
        // For now, just test that the tools don't crash on valid Rust code
        
        // Test extract functions operation (when implemented)
        let op = AgentToolOp::ExtractFunctions {
            file: file_path.clone(),
            language: "rust".to_string(),
        };
        
        let result = tools.search(op);
        
        // The result might be an error if not implemented yet, but it shouldn't panic
        match result {
            Ok(AgentToolResult::Functions(functions)) => {
                // If implemented, verify we get reasonable results
                assert!(!functions.is_empty());
                
                // Look for expected functions
                let function_names: Vec<&str> = functions.iter().map(|f| f.name.as_str()).collect();
                assert!(function_names.contains(&"new") || function_names.contains(&"add"));
            }
            Err(ToolError::NotImplemented(_)) => {
                // Not yet implemented - that's fine for now
                println!("Extract functions not yet implemented");
            }
            Err(e) => {
                panic!("Unexpected error: {:?}", e);
            }
            _ => {
                panic!("Unexpected result type");
            }
        }
    }

    #[tokio::test]
    async fn test_ast_tools_error_handling() {
        let tools = ASTAgentTools::new();
        
        // Test with nonexistent file
        let nonexistent_file = PathBuf::from("/nonexistent/file.rs");
        let op = AgentToolOp::ExtractFunctions {
            file: nonexistent_file,
            language: "rust".to_string(),
        };
        
        let result = tools.search(op);
        
        // Should handle the error gracefully
        assert!(result.is_err());
        
        match result.unwrap_err() {
            ToolError::Io(_) => {
                // Expected - file doesn't exist
            }
            ToolError::NotImplemented(_) => {
                // Also acceptable if not implemented
            }
            ToolError::InvalidQuery(_) => {
                // Also acceptable
            }
            e => {
                println!("Got error (acceptable): {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_ast_tools_language_detection() {
        let tools = ASTAgentTools::new();
        let (file, _content) = create_simple_rust_file();
        let file_path = PathBuf::from(file.path());
        
        // Test with different language strings
        let languages = vec!["rust", "rs", "Rust", "RUST"];
        
        for lang in languages {
            let op = AgentToolOp::ExtractFunctions {
                file: file_path.clone(),
                language: lang.to_string(),
            };
            
            let result = tools.search(op);
            
            // Should not panic regardless of implementation status
            match result {
                Ok(_) => {
                    // Good - language was recognized
                }
                Err(ToolError::NotImplemented(_)) => {
                    // Acceptable - not implemented yet
                }
                Err(ToolError::UnsupportedLanguage(_)) => {
                    // Acceptable - language not supported yet
                }
                Err(e) => {
                    // Other errors are also acceptable at this stage
                    println!("Language '{}' resulted in error: {:?}", lang, e);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_multiple_operations() {
        let tools = ASTAgentTools::new();
        let (file, _content) = create_simple_rust_file();
        let file_path = PathBuf::from(file.path());
        
        let operations = vec![
            AgentToolOp::ExtractFunctions {
                file: file_path.clone(),
                language: "rust".to_string(),
            },
            AgentToolOp::ValidateSyntax {
                file: file_path.clone(),
                language: "rust".to_string(),
            },
        ];
        
        for op in operations {
            let result = tools.search(op);
            
            // None of these should panic
            match result {
                Ok(_) => {
                    // Success
                }
                Err(e) => {
                    // Error is acceptable
                    println!("Operation resulted in error (acceptable): {:?}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use super::fixtures::*;

    #[tokio::test]
    async fn test_concurrent_access() {
        let tools = std::sync::Arc::new(ASTAgentTools::new());
        let (file, _content) = create_simple_rust_file();
        let file_path = std::sync::Arc::new(PathBuf::from(file.path()));
        
        let mut handles = vec![];
        
        // Spawn multiple concurrent tasks
        for i in 0..5 {
            let tools_clone = std::sync::Arc::clone(&tools);
            let file_path_clone = std::sync::Arc::clone(&file_path);
            
            let handle = tokio::spawn(async move {
                let op = AgentToolOp::ExtractFunctions {
                    file: (*file_path_clone).clone(),
                    language: "rust".to_string(),
                };
                
                let result = tools_clone.search(op);
                
                // Should not panic
                match result {
                    Ok(_) => format!("Task {} succeeded", i),
                    Err(e) => format!("Task {} failed with: {:?}", i, e),
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        let results = futures::future::join_all(handles).await;
        
        // All tasks should complete without panicking
        for (i, result) in results.into_iter().enumerate() {
            match result {
                Ok(message) => {
                    println!("Task {}: {}", i, message);
                }
                Err(e) => {
                    panic!("Task {} panicked: {:?}", i, e);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_large_file_handling() {
        let tools = ASTAgentTools::new();
        
        // Create a larger file
        let mut file = NamedTempFile::new().unwrap();
        let mut content = String::new();
        
        // Generate multiple similar functions
        for i in 0..50 {
            content.push_str(&format!(r#"
pub fn function_{i}(x: i32) -> i32 {{
    let result = x * 2;
    result + {i}
}}
"#, i = i));
        }
        
        file.write_all(content.as_bytes()).unwrap();
        let file_path = PathBuf::from(file.path());
        
        let start = std::time::Instant::now();
        
        let op = AgentToolOp::ExtractFunctions {
            file: file_path,
            language: "rust".to_string(),
        };
        
        let _result = tools.search(op);
        
        let duration = start.elapsed();
        
        // Should complete within reasonable time (even if it fails)
        assert!(duration < std::time::Duration::from_secs(5));
        
        println!("Large file processing took: {:?}", duration);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use super::fixtures::*;

    #[tokio::test]
    async fn test_ast_tools_code_tool_trait() {
        let tools = ASTAgentTools::new();
        let (file, _content) = create_simple_rust_file();
        let file_path = PathBuf::from(file.path());
        
        // Test that AST tools implement the CodeTool trait correctly
        let query = AgentToolOp::ExtractFunctions {
            file: file_path,
            language: "rust".to_string(),
        };
        
        // This should compile and not panic
        let _result = <ASTAgentTools as CodeTool>::search(&tools, query);
        
        // Result content doesn't matter for this test - just that the trait works
    }

    #[tokio::test]
    async fn test_tool_error_conversion() {
        // Test that various error types can be created and converted properly
        let errors = vec![
            ToolError::NotImplemented("test operation"),
            ToolError::InvalidQuery("invalid query".to_string()),
            ToolError::ParseError("parse failed".to_string()),
            ToolError::NotFound("symbol not found".to_string()),
            ToolError::UnsupportedLanguage("unknown_lang".to_string()),
        ];
        
        for error in errors {
            let error_string = error.to_string();
            assert!(!error_string.is_empty());
            
            // Test that error implements required traits
            let _: Box<dyn std::error::Error> = Box::new(error);
        }
    }
}

// Module to test that the basic AST structure compiles
#[cfg(test)]
mod structure_tests {
    use super::*;

    #[test]
    fn test_semantic_index_creation() {
        let index = SemanticIndex {
            functions: vec![],
            classes: vec![],
            imports: vec![],
            exports: vec![],
            symbols: vec![],
            call_graph: CallGraph {
                functions: std::collections::HashMap::new(),
                dependencies: std::collections::HashMap::new(),
                reverse_dependencies: std::collections::HashMap::new(),
            },
        };
        
        assert_eq!(index.functions.len(), 0);
        assert_eq!(index.classes.len(), 0);
        assert_eq!(index.symbols.len(), 0);
    }

    #[test]
    fn test_function_info_creation() {
        let func_info = FunctionInfo {
            name: "test_function".to_string(),
            signature: "fn test_function() -> bool".to_string(),
            start_line: 10,
            end_line: 15,
            complexity: 3,
            parameters: vec!["param1".to_string(), "param2".to_string()],
            return_type: Some("bool".to_string()),
            is_async: false,
            is_exported: true,
        };
        
        assert_eq!(func_info.name, "test_function");
        assert_eq!(func_info.complexity, 3);
        assert_eq!(func_info.parameters.len(), 2);
        assert!(!func_info.is_async);
        assert!(func_info.is_exported);
    }

    #[test]
    fn test_symbol_kinds() {
        let kinds = vec![
            SymbolKind::Function,
            SymbolKind::Class,
            SymbolKind::Variable,
            SymbolKind::Constant,
            SymbolKind::Type,
            SymbolKind::Interface,
            SymbolKind::Enum,
            SymbolKind::Module,
            SymbolKind::Namespace,
        ];
        
        // Test that all variants can be created
        for kind in kinds {
            let symbol_info = SymbolInfo {
                name: "test_symbol".to_string(),
                kind,
                line: 1,
                column: 1,
                scope: "global".to_string(),
                references: vec![],
            };
            
            assert_eq!(symbol_info.name, "test_symbol");
            assert_eq!(symbol_info.line, 1);
            assert_eq!(symbol_info.scope, "global");
        }
    }
}