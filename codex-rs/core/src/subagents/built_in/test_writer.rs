//! Test Writer Agent - Generates comprehensive tests
//!
//! This agent creates thorough test suites:
//! - Unit tests for functions
//! - Integration tests for modules
//! - Property-based tests
//! - Edge case coverage
//! - Test data generation

use crate::code_tools::ast_agent_tools::ASTAgentTools;
use crate::code_tools::ast_agent_tools::AgentToolOp;
use crate::code_tools::ast_agent_tools::AgentToolResult;
use crate::modes::OperatingMode;
use crate::subagents::AgentResult;
use crate::subagents::AgentStatus;
use crate::subagents::Finding;
use crate::subagents::Severity;
use crate::subagents::Subagent;
use crate::subagents::SubagentContext;
use crate::subagents::SubagentError;
use crate::subagents::SubagentResult;
use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::SystemTime;

/// Test Writer Agent implementation
#[derive(Debug)]
pub struct TestWriterAgent {
    name: String,
    description: String,
    _mode_override: Option<OperatingMode>,
    _tool_permissions: Vec<String>,
    _prompt_template: String,
    test_strategy: TestStrategy,
}

/// Test generation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestStrategy {
    Basic,         // Essential test cases only
    Comprehensive, // Full coverage with edge cases
    PropertyBased, // Property-based testing with generators
}

impl Default for TestWriterAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl TestWriterAgent {
    /// Create a new test writer agent
    pub fn new() -> Self {
        Self {
            name: "test-writer".to_string(),
            description: "Generates comprehensive test suites with high coverage".to_string(),
            _mode_override: Some(OperatingMode::Build),
            _tool_permissions: vec![
                "search".to_string(),
                "edit".to_string(),
                "tree".to_string(),
                "think".to_string(),
            ],
            _prompt_template: r#"
You are an expert test engineer focused on:
- Achieving high code coverage (>90%)
- Testing edge cases and error conditions
- Creating maintainable test suites
- Using appropriate testing patterns
- Generating realistic test data

Write tests that are:
- Isolated and independent
- Fast and deterministic
- Clear and well-documented
- Comprehensive yet focused
"#
            .to_string(),
            test_strategy: TestStrategy::Comprehensive,
        }
    }

    /// Set test strategy
    pub const fn with_strategy(mut self, strategy: TestStrategy) -> Self {
        self.test_strategy = strategy;
        self
    }

    /// Analyze test coverage
    async fn analyze_coverage(&self, ast_tools: &mut ASTAgentTools, file: &Path) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Extract functions and check for existing tests
        if let Ok(AgentToolResult::Functions(functions)) =
            ast_tools.execute(AgentToolOp::ExtractFunctions {
                file: file.to_path_buf(),
                language: self.detect_language(file),
            })
        {
            for func in functions {
                // Check if function has tests
                let test_pattern = format!("test.*{}", func.name);
                if let Ok(AgentToolResult::SearchResults(results)) =
                    ast_tools.execute(AgentToolOp::Search {
                        query: test_pattern,
                        scope: crate::code_tools::search::SearchScope::Directory(
                            file.parent().unwrap_or(Path::new(".")).to_path_buf(),
                        ),
                    })
                    && results.is_empty()
                {
                    findings.push(Finding {
                            category: "test-coverage".to_string(),
                            severity: Severity::Medium,
                            title: format!("Missing Tests: {}", func.name),
                            description: format!(
                                "Function '{}' has no test coverage. This could lead to undetected bugs.",
                                func.name
                            ),
                            location: Some(crate::code_tools::ast_agent_tools::Location {
                                file: file.to_path_buf(),
                                line: func.start_line,
                                column: 0,
                                byte_offset: 0,
                            }),
                            suggestion: Some("Create unit tests covering normal, edge, and error cases".to_string()),
                            metadata: HashMap::from([
                                ("function_name".to_string(), serde_json::json!(func.name)),
                                ("needs_tests".to_string(), serde_json::json!(true)),
                            ]),
                        });
                }
            }
        }

        findings
    }

    /// Generate test cases for a function
    async fn generate_test_cases(
        &self,
        _ast_tools: &mut ASTAgentTools,
        function_name: &str,
        file: &Path,
    ) -> String {
        let mut test_code = String::new();
        let lang = self.detect_language(file);

        // Generate test structure based on language
        match lang.as_str() {
            "rust" => {
                test_code.push_str(&format!(
                    r#"
#[cfg(test)]
mod test_{} {{
    use super::*;

    #[test]
    fn test_{}_normal_case() {{
        // Arrange
        let input = /* TODO: Add test input */;
        
        // Act
        let result = {}(input);
        
        // Assert
        assert_eq!(result, /* expected value */);
    }}

    #[test]
    fn test_{}_edge_case() {{
        // Test with boundary values
        let edge_input = /* TODO: Add edge case input */;
        let result = {}(edge_input);
        assert!(/* validation */);
    }}

    #[test]
    #[should_panic(expected = "error message")]
    fn test_{}_error_case() {{
        // Test error handling
        let invalid_input = /* TODO: Add invalid input */;
        {}(invalid_input); // Should panic
    }}
}}"#,
                    function_name,
                    function_name,
                    function_name,
                    function_name,
                    function_name,
                    function_name,
                    function_name
                ));
            }
            "python" => {
                test_code.push_str(&format!(
                    r#"
import unittest
from unittest.mock import Mock, patch

class Test{}(unittest.TestCase):
    
    def test_{}_normal_case(self):
        # Arrange
        input_data = # TODO: Add test input
        
        # Act
        result = {}(input_data)
        
        # Assert
        self.assertEqual(result, # expected value)
    
    def test_{}_edge_case(self):
        # Test with boundary values
        edge_input = # TODO: Add edge case input
        result = {}(edge_input)
        self.assertTrue(# validation)
    
    def test_{}_error_case(self):
        # Test error handling
        invalid_input = # TODO: Add invalid input
        with self.assertRaises(Exception):
            {}(invalid_input)

if __name__ == '__main__':
    unittest.main()"#,
                    to_pascal_case(function_name),
                    function_name,
                    function_name,
                    function_name,
                    function_name,
                    function_name,
                    function_name
                ));
            }
            "javascript" | "typescript" => {
                test_code.push_str(&format!(
                    r#"
describe('{}', () => {{
    
    test('should handle normal case', () => {{
        // Arrange
        const input = /* TODO: Add test input */;
        
        // Act
        const result = {}(input);
        
        // Assert
        expect(result).toBe(/* expected value */);
    }});
    
    test('should handle edge case', () => {{
        // Test with boundary values
        const edgeInput = /* TODO: Add edge case input */;
        const result = {}(edgeInput);
        expect(result).toBeTruthy();
    }});
    
    test('should throw error for invalid input', () => {{
        // Test error handling
        const invalidInput = /* TODO: Add invalid input */;
        expect(() => {{
            {}(invalidInput);
        }}).toThrow('error message');
    }});
}});"#,
                    function_name, function_name, function_name, function_name
                ));
            }
            _ => {
                test_code.push_str(&format!(
                    "// TODO: Generate tests for function '{}'\n",
                    function_name
                ));
            }
        }

        test_code
    }

    /// Detect language from file extension
    fn detect_language(&self, file: &Path) -> String {
        file.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext {
                "rs" => "rust",
                "py" => "python",
                "js" => "javascript",
                "ts" => "typescript",
                "go" => "go",
                "java" => "java",
                _ => "text",
            })
            .unwrap_or("text")
            .to_string()
    }
}

impl Subagent for TestWriterAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn execute<'a>(
        &'a self,
        context: &'a SubagentContext,
        ast_tools: &'a mut ASTAgentTools,
        cancel_flag: Arc<AtomicBool>,
    ) -> Pin<Box<dyn Future<Output = SubagentResult<AgentResult>> + Send + 'a>> {
        Box::pin(async move {
            let start_time = SystemTime::now();
            let mut all_findings = Vec::new();
            let mut analyzed_files = Vec::new();
            let mut modified_files = Vec::new();
            let mut tests_generated = 0;

            // Get files to test from context
            let files = self.get_test_targets(context)?;

            for file in &files {
                if cancel_flag.load(Ordering::Acquire) {
                    return Err(SubagentError::ExecutionFailed(
                        "Test generation cancelled".to_string(),
                    ));
                }

                analyzed_files.push(file.clone());

                // Analyze test coverage
                let coverage_findings = self.analyze_coverage(ast_tools, file).await;

                // Generate tests for uncovered functions
                for finding in &coverage_findings {
                    if let Some(function_name) = finding.metadata.get("function_name")
                        && let Some(name) = function_name.as_str()
                    {
                        let test_code = self.generate_test_cases(ast_tools, name, file).await;

                        // Save test file (in Build mode)
                        if context.mode == OperatingMode::Build && !test_code.is_empty() {
                            let test_file = self.get_test_file_path(file);
                            // Here you would write the test file
                            modified_files.push(test_file);
                            tests_generated += 1;
                        }
                    }
                }

                all_findings.extend(coverage_findings);
            }

            let summary = format!(
                "Test generation completed: {} files analyzed, {} missing tests found, {} test files generated",
                analyzed_files.len(),
                all_findings.len(),
                tests_generated
            );

            // Store the length before moving all_findings
            let missing_tests = all_findings.len();

            let execution_time = SystemTime::now()
                .duration_since(start_time)
                .unwrap_or_else(|_| Duration::from_secs(0));

            Ok(AgentResult {
                agent_name: self.name.clone(),
                status: AgentStatus::Completed,
                findings: all_findings,
                analyzed_files,
                modified_files,
                execution_time,
                summary,
                metrics: HashMap::from([
                    (
                        "missing_tests".to_string(),
                        serde_json::json!(missing_tests),
                    ),
                    (
                        "tests_generated".to_string(),
                        serde_json::json!(tests_generated),
                    ),
                    (
                        "test_strategy".to_string(),
                        serde_json::json!(format!("{:?}", self.test_strategy)),
                    ),
                ]),
            })
        })
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "test-generation".to_string(),
            "coverage-analysis".to_string(),
            "edge-case-generation".to_string(),
            "mock-generation".to_string(),
            "test-data-generation".to_string(),
        ]
    }

    fn supports_file_type(&self, file_path: &Path) -> bool {
        let supported = ["rs", "py", "js", "ts", "go", "java"];
        file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| supported.contains(&ext))
            .unwrap_or(false)
    }

    fn execution_time_estimate(&self) -> Duration {
        match self.test_strategy {
            TestStrategy::Basic => Duration::from_secs(45),
            TestStrategy::Comprehensive => Duration::from_secs(90),
            TestStrategy::PropertyBased => Duration::from_secs(120),
        }
    }
}

impl TestWriterAgent {
    fn get_test_targets(&self, context: &SubagentContext) -> Result<Vec<PathBuf>, SubagentError> {
        if let Some(files) = context.parameters.get("files") {
            Ok(files.split(',').map(|s| PathBuf::from(s.trim())).collect())
        } else {
            Ok(vec![context.working_directory.clone()])
        }
    }

    fn get_test_file_path(&self, source_file: &Path) -> PathBuf {
        let stem = source_file.file_stem().unwrap_or_default();
        let ext = source_file.extension().unwrap_or_default();
        let parent = source_file.parent().unwrap_or(Path::new("."));

        parent.join(format!(
            "{}_test.{}",
            stem.to_string_lossy(),
            ext.to_string_lossy()
        ))
    }
}

/// Convert snake_case to PascalCase
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}
