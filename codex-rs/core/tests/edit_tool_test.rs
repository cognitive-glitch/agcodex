//! Comprehensive tests for the AGCodex Edit Tool.
//!
//! Tests cover line-based editing, text replacement, error handling,
//! performance requirements (<1ms for simple edits), and edge cases.
//!
//! The edit tool provides basic, reliable text editing with LLM-friendly
//! output including context, before/after state tracking, and backup creation.

use std::fs;
use std::time::Duration;
use std::time::Instant;
use tempfile::NamedTempFile;
use tempfile::TempDir;

// Import edit tool from the same crate
use agcodex_core::tools::edit::EditError;
use agcodex_core::tools::edit::EditTool;

/// Sample code files for realistic editing scenarios
const SIMPLE_RUST_CODE: &str = r#"fn main() {
    println!("Hello, world!");
    let x = 42;
    println!("The answer is {}", x);
}"#;

const RUST_STRUCT_CODE: &str = r#"use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
    pub settings: HashMap<String, String>,
}

impl User {
    pub fn new(id: u64, name: String, email: String) -> Self {
        Self {
            id,
            name,
            email,
            settings: HashMap::new(),
        }
    }
    
    pub fn update_setting(&mut self, key: String, value: String) {
        self.settings.insert(key, value);
    }
    
    pub fn get_display_name(&self) -> &str {
        &self.name
    }
}"#;

const PYTHON_CLASS_CODE: &str = r#"class Calculator:
    def __init__(self):
        self.history = []
        self.current_value = 0
    
    def add(self, value):
        self.current_value += value
        self.history.append(f"Added {value}")
        return self.current_value
    
    def subtract(self, value):
        self.current_value -= value
        self.history.append(f"Subtracted {value}")
        return self.current_value
    
    def clear(self):
        self.current_value = 0
        self.history = []
    
    def get_history(self):
        return self.history.copy()
"#;

const TYPESCRIPT_INTERFACE_CODE: &str = r#"interface ApiResponse<T> {
    success: boolean;
    data: T;
    error?: string;
    timestamp: number;
}

class ApiClient {
    private baseUrl: string;
    private timeout: number;
    
    constructor(baseUrl: string, timeout: number = 5000) {
        this.baseUrl = baseUrl;
        this.timeout = timeout;
    }
    
    async get<T>(endpoint: string): Promise<ApiResponse<T>> {
        const url = `${this.baseUrl}${endpoint}`;
        const response = await fetch(url);
        
        return {
            success: response.ok,
            data: await response.json(),
            timestamp: Date.now()
        };
    }
}"#;

/// Performance assertion helper (simplified version for this test)
struct PerformanceAssertions;
impl PerformanceAssertions {
    fn assert_duration_under(duration: Duration, target_ms: u64, operation: &str) {
        let actual_ms = duration.as_millis();
        assert!(
            actual_ms < target_ms as u128,
            "{} took {}ms, target was <{}ms",
            operation,
            actual_ms,
            target_ms
        );
    }
}

/// Test timing utility (simplified version)
struct TestTiming;
impl TestTiming {
    fn time_operation<F, R>(operation: F) -> (R, Duration)
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = operation();
        let duration = start.elapsed();
        (result, duration)
    }
}

#[cfg(test)]
mod basic_functionality_tests {
    use super::*;

    #[test]
    fn test_edit_tool_creation() {
        let edit_tool = EditTool::new();
        // Should create successfully (tool is stateless)
        assert_eq!(std::mem::size_of_val(&edit_tool), 0);
    }

    #[test]
    fn test_simple_line_edit() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), SIMPLE_RUST_CODE).unwrap();

        let result =
            EditTool::edit_line(temp_file.path().to_str().unwrap(), 3, "let x = 100;").unwrap();

        assert!(result.success);
        assert_eq!(result.line_changed, Some(3));
        assert_eq!(result.old_content, "    let x = 42;");
        assert_eq!(result.new_content, "    let x = 100;");
        assert!(result.message.contains("Successfully changed line 3"));

        // Verify file was actually changed
        let new_content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(new_content.contains("let x = 100;"));
        assert!(!new_content.contains("let x = 42;"));

        // Verify backup was created
        let backup_path = format!("{}.backup", temp_file.path().to_str().unwrap());
        assert!(std::path::Path::new(&backup_path).exists());
    }

    #[test]
    fn test_simple_text_replacement() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), SIMPLE_RUST_CODE).unwrap();

        let result = EditTool::edit_text(
            temp_file.path().to_str().unwrap(),
            "Hello, world!",
            "Hello, AGCodex!",
        )
        .unwrap();

        assert!(result.success);
        assert_eq!(result.line_changed, Some(2));
        assert_eq!(result.old_content, "Hello, world!");
        assert_eq!(result.new_content, "Hello, AGCodex!");
        assert!(result.message.contains("Successfully replaced"));

        // Verify file was changed
        let new_content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(new_content.contains("Hello, AGCodex!"));
        assert!(!new_content.contains("Hello, world!"));
    }

    #[test]
    fn test_indentation_preservation() {
        let temp_file = NamedTempFile::new().unwrap();
        let indented_code =
            "fn test() {\n    let x = 1;\n        let y = 2;\n            let z = 3;\n}";
        fs::write(temp_file.path(), indented_code).unwrap();

        // Edit line with 4-space indentation
        let result =
            EditTool::edit_line(temp_file.path().to_str().unwrap(), 2, "let x = 42;").unwrap();

        assert_eq!(result.new_content, "    let x = 42;");

        // Edit line with 8-space indentation
        let result =
            EditTool::edit_line(temp_file.path().to_str().unwrap(), 3, "let y = 84;").unwrap();

        assert_eq!(result.new_content, "        let y = 84;");

        // Edit line with 12-space indentation
        let result =
            EditTool::edit_line(temp_file.path().to_str().unwrap(), 4, "let z = 126;").unwrap();

        assert_eq!(result.new_content, "            let z = 126;");
    }

    #[test]
    fn test_tab_indentation_preservation() {
        let temp_file = NamedTempFile::new().unwrap();
        let tab_indented = "fn test() {\n\tlet x = 1;\n\t\tlet y = 2;\n}";
        fs::write(temp_file.path(), tab_indented).unwrap();

        let result =
            EditTool::edit_line(temp_file.path().to_str().unwrap(), 2, "let x = 42;").unwrap();

        assert_eq!(result.new_content, "\tlet x = 42;");

        let result =
            EditTool::edit_line(temp_file.path().to_str().unwrap(), 3, "let y = 84;").unwrap();

        assert_eq!(result.new_content, "\t\tlet y = 84;");
    }

    #[test]
    fn test_context_extraction() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), RUST_STRUCT_CODE).unwrap();

        // Edit line in the middle (should have 3 lines before and after)
        let result = EditTool::edit_line(
            temp_file.path().to_str().unwrap(),
            10, // "id," line
            "id: u32,",
        )
        .unwrap();

        // Should have context before
        assert_eq!(result.context_before.len(), 3);
        assert!(result.context_before[0].contains("Self {"));

        // Should have context after
        assert_eq!(result.context_after.len(), 3);
        assert!(result.context_after[0].contains("name,"));
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[test]
    fn test_file_not_found() {
        let result = EditTool::edit_line("/nonexistent/file.txt", 1, "test");

        assert!(matches!(result, Err(EditError::FileNotFound(_))));

        let result = EditTool::edit_text("/nonexistent/file.txt", "old", "new");

        assert!(matches!(result, Err(EditError::FileNotFound(_))));
    }

    #[test]
    fn test_invalid_line_numbers() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), "line1\nline2\nline3").unwrap();

        // Line 0 (invalid - 1-based indexing)
        let result = EditTool::edit_line(temp_file.path().to_str().unwrap(), 0, "new line");
        assert!(matches!(
            result,
            Err(EditError::InvalidLine {
                line: 0,
                total_lines: 3
            })
        ));

        // Line beyond end of file
        let result = EditTool::edit_line(temp_file.path().to_str().unwrap(), 5, "new line");
        assert!(matches!(
            result,
            Err(EditError::InvalidLine {
                line: 5,
                total_lines: 3
            })
        ));
    }

    #[test]
    fn test_pattern_not_found() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), SIMPLE_RUST_CODE).unwrap();

        let result = EditTool::edit_text(
            temp_file.path().to_str().unwrap(),
            "nonexistent pattern",
            "replacement",
        );

        assert!(matches!(result, Err(EditError::PatternNotFound(_))));
    }

    #[test]
    fn test_ambiguous_matches() {
        let temp_file = NamedTempFile::new().unwrap();
        let ambiguous_code = "let x = 1;\nlet x = 2;\nlet x = 3;\n";
        fs::write(temp_file.path(), ambiguous_code).unwrap();

        let result = EditTool::edit_text(temp_file.path().to_str().unwrap(), "let x", "let y");

        assert!(matches!(
            result,
            Err(EditError::AmbiguousMatch { count: 3, .. })
        ));
    }

    #[test]
    fn test_find_matches_for_ambiguity() {
        let temp_file = NamedTempFile::new().unwrap();
        let code_with_duplicates = "function test() {\n    console.log('test');\n}\nfunction test2() {\n    console.log('test2');\n}\nfunction test3() {\n    console.log('test3');\n}";
        fs::write(temp_file.path(), code_with_duplicates).unwrap();

        let matches =
            EditTool::find_matches(temp_file.path().to_str().unwrap(), "console.log").unwrap();

        assert_eq!(matches.matches.len(), 3);
        assert_eq!(matches.matches[0].line_number, 2);
        assert_eq!(matches.matches[1].line_number, 5);
        assert_eq!(matches.matches[2].line_number, 8);

        // Check context includes surrounding lines
        assert!(matches.matches[0].context.contains("function test()"));
        assert!(matches.matches[0].context.contains("→ 2:"));
    }

    #[test]
    fn test_invalid_utf8_handling() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("invalid.txt");

        // Write invalid UTF-8 bytes
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
        fs::write(&file_path, &invalid_utf8).unwrap();

        let result = EditTool::edit_line(file_path.to_str().unwrap(), 1, "replacement");

        // Should handle gracefully (either as InvalidUtf8 or other IO error)
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod edge_cases_tests {
    use super::*;

    #[test]
    fn test_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), "").unwrap();

        let result = EditTool::edit_line(temp_file.path().to_str().unwrap(), 1, "first line");

        assert!(matches!(
            result,
            Err(EditError::InvalidLine {
                line: 1,
                total_lines: 0
            })
        ));

        let result = EditTool::edit_text(
            temp_file.path().to_str().unwrap(),
            "anything",
            "replacement",
        );

        assert!(matches!(result, Err(EditError::PatternNotFound(_))));
    }

    #[test]
    fn test_single_line_file() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), "single line").unwrap();

        let result =
            EditTool::edit_line(temp_file.path().to_str().unwrap(), 1, "modified line").unwrap();

        assert_eq!(result.line_changed, Some(1));
        assert_eq!(result.old_content, "single line");
        assert_eq!(result.new_content, "modified line");
        assert!(result.context_before.is_empty());
        assert!(result.context_after.is_empty());
    }

    #[test]
    fn test_edit_first_line() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), "line1\nline2\nline3\nline4\nline5").unwrap();

        let result =
            EditTool::edit_line(temp_file.path().to_str().unwrap(), 1, "modified first line")
                .unwrap();

        assert_eq!(result.line_changed, Some(1));
        assert!(result.context_before.is_empty()); // No lines before first line
        assert_eq!(result.context_after.len(), 3); // Should have 3 lines after
    }

    #[test]
    fn test_edit_last_line() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), "line1\nline2\nline3\nline4\nline5").unwrap();

        let result =
            EditTool::edit_line(temp_file.path().to_str().unwrap(), 5, "modified last line")
                .unwrap();

        assert_eq!(result.line_changed, Some(5));
        assert_eq!(result.context_before.len(), 3); // Should have 3 lines before
        assert!(result.context_after.is_empty()); // No lines after last line
    }

    #[test]
    fn test_empty_line_replacement() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), "line1\n    \nline3").unwrap();

        let result = EditTool::edit_line(temp_file.path().to_str().unwrap(), 2, "").unwrap();

        assert_eq!(result.new_content, "");
    }

    #[test]
    fn test_whitespace_only_line() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), "line1\n    \nline3").unwrap();

        let result = EditTool::edit_line(temp_file.path().to_str().unwrap(), 2, "content").unwrap();

        assert_eq!(result.new_content, "    content");
    }

    #[test]
    fn test_very_long_line() {
        let temp_file = NamedTempFile::new().unwrap();
        let long_line = "x".repeat(10000);
        let content = format!("short\n{}\nshort", long_line);
        fs::write(temp_file.path(), content).unwrap();

        let result =
            EditTool::edit_line(temp_file.path().to_str().unwrap(), 2, "replaced").unwrap();

        assert_eq!(result.new_content, "replaced");
        assert_eq!(result.old_content, long_line);
    }
}

#[cfg(test)]
mod multi_line_replacement_tests {
    use super::*;

    #[test]
    fn test_multi_line_pattern_replacement() {
        let temp_file = NamedTempFile::new().unwrap();
        let multi_line_code = "fn old_function() {\n    println!(\"old\");\n}";
        fs::write(temp_file.path(), multi_line_code).unwrap();

        let result = EditTool::edit_text(
            temp_file.path().to_str().unwrap(),
            "fn old_function() {\n    println!(\"old\");\n}",
            "fn new_function() {\n    println!(\"new\");\n}",
        )
        .unwrap();

        assert!(result.success);
        let new_content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(new_content.contains("fn new_function()"));
        assert!(new_content.contains("println!(\"new\")"));
    }

    #[test]
    fn test_code_block_replacement() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), RUST_STRUCT_CODE).unwrap();

        // Replace the new function
        let old_function = r#"pub fn new(id: u64, name: String, email: String) -> Self {
        Self {
            id,
            name,
            email,
            settings: HashMap::new(),
        }
    }"#;

        let new_function = r#"pub fn new(id: u64, name: String, email: String) -> Self {
        Self {
            id,
            name,
            email,
            settings: HashMap::new(),
            created_at: std::time::SystemTime::now(),
        }
    }"#;

        let result = EditTool::edit_text(
            temp_file.path().to_str().unwrap(),
            old_function,
            new_function,
        )
        .unwrap();

        assert!(result.success);
        let new_content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(new_content.contains("created_at: std::time::SystemTime::now()"));
    }
}

#[cfg(test)]
mod realistic_code_editing_tests {
    use super::*;

    #[test]
    fn test_rust_function_parameter_change() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), RUST_STRUCT_CODE).unwrap();

        let result = EditTool::edit_text(
            temp_file.path().to_str().unwrap(),
            "pub fn new(id: u64, name: String, email: String)",
            "pub fn new(id: u64, name: String, email: String, active: bool)",
        )
        .unwrap();

        assert!(result.success);
        let new_content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(new_content.contains("active: bool"));
    }

    #[test]
    fn test_python_method_modification() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), PYTHON_CLASS_CODE).unwrap();

        let result = EditTool::edit_text(
            temp_file.path().to_str().unwrap(),
            "def add(self, value):",
            "def add(self, value: float) -> float:",
        )
        .unwrap();

        assert!(result.success);
        let new_content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(new_content.contains("def add(self, value: float) -> float:"));
    }

    #[test]
    fn test_typescript_type_annotation() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), TYPESCRIPT_INTERFACE_CODE).unwrap();

        let result = EditTool::edit_text(
            temp_file.path().to_str().unwrap(),
            "timeout: number = 5000",
            "timeout: number = 10000",
        )
        .unwrap();

        assert!(result.success);
        let new_content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(new_content.contains("timeout: number = 10000"));
    }

    #[test]
    fn test_variable_renaming() {
        let temp_file = NamedTempFile::new().unwrap();
        let code = "let oldVar = 42;\nconsole.log(oldVar);\nreturn oldVar * 2;";
        fs::write(temp_file.path(), code).unwrap();

        // This should fail due to ambiguity (multiple occurrences)
        let result = EditTool::edit_text(temp_file.path().to_str().unwrap(), "oldVar", "newVar");

        assert!(matches!(result, Err(EditError::AmbiguousMatch { .. })));

        // But we can edit each occurrence specifically
        let result = EditTool::edit_text(
            temp_file.path().to_str().unwrap(),
            "let oldVar = 42;",
            "let newVar = 42;",
        )
        .unwrap();

        assert!(result.success);
    }

    #[test]
    fn test_import_statement_modification() {
        let temp_file = NamedTempFile::new().unwrap();
        let import_code = "use std::collections::HashMap;\nuse std::fs;\n\nfn main() {}";
        fs::write(temp_file.path(), import_code).unwrap();

        let result = EditTool::edit_text(
            temp_file.path().to_str().unwrap(),
            "use std::collections::HashMap;",
            "use std::collections::{HashMap, HashSet};",
        )
        .unwrap();

        assert!(result.success);
        let new_content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(new_content.contains("HashMap, HashSet"));
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;

    #[test]
    fn test_simple_edit_performance() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), SIMPLE_RUST_CODE).unwrap();

        let (result, duration) = TestTiming::time_operation(|| {
            EditTool::edit_line(temp_file.path().to_str().unwrap(), 2, "modified line")
        });

        assert!(result.is_ok());
        // Should complete in under 1ms for simple edits
        PerformanceAssertions::assert_duration_under(duration, 1, "simple line edit");
    }

    #[test]
    fn test_text_replacement_performance() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), RUST_STRUCT_CODE).unwrap();

        let (result, duration) = TestTiming::time_operation(|| {
            EditTool::edit_text(
                temp_file.path().to_str().unwrap(),
                "HashMap::new()",
                "HashMap::default()",
            )
        });

        assert!(result.is_ok());
        PerformanceAssertions::assert_duration_under(duration, 1, "simple text replacement");
    }

    #[test]
    fn test_large_file_edit_performance() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("large.rs");

        // Create a larger file by repeating the struct code
        let large_content = RUST_STRUCT_CODE.repeat(100);
        fs::write(&file_path, &large_content).unwrap();

        let (result, duration) = TestTiming::time_operation(|| {
            EditTool::edit_text(
                file_path.to_str().unwrap(),
                "pub struct User {",
                "pub struct Person {",
            )
        });

        // Should handle first occurrence and detect ambiguity quickly
        assert!(result.is_err()); // Should be ambiguous
        // Should still complete quickly even for large files
        PerformanceAssertions::assert_duration_under(
            duration,
            10,
            "large file ambiguity detection",
        );
    }

    #[test]
    fn test_find_matches_performance() {
        let temp_file = NamedTempFile::new().unwrap();
        let repeated_pattern = "test\n".repeat(1000);
        fs::write(temp_file.path(), repeated_pattern).unwrap();

        let (result, duration) = TestTiming::time_operation(|| {
            EditTool::find_matches(temp_file.path().to_str().unwrap(), "test")
        });

        assert!(result.is_ok());
        let matches = result.unwrap();
        assert_eq!(matches.matches.len(), 1000);

        // Should complete quickly even with many matches
        PerformanceAssertions::assert_duration_under(duration, 50, "find 1000 matches");
    }

    #[test]
    fn test_repeated_edit_performance() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), SIMPLE_RUST_CODE).unwrap();

        let start = Instant::now();

        for i in 1..=5 {
            let result = EditTool::edit_line(
                temp_file.path().to_str().unwrap(),
                2,
                &format!("println!(\"Hello, iteration {}\");", i),
            );
            assert!(result.is_ok());
        }

        let total_duration = start.elapsed();

        // 5 edits should complete in under 5ms total
        PerformanceAssertions::assert_duration_under(total_duration, 5, "5 sequential edits");
    }
}

#[cfg(test)]
mod backup_and_safety_tests {
    use super::*;

    #[test]
    fn test_backup_file_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let original_content = "original content";
        fs::write(temp_file.path(), original_content).unwrap();

        let result =
            EditTool::edit_line(temp_file.path().to_str().unwrap(), 1, "modified content").unwrap();

        assert!(result.success);

        // Verify backup exists and contains original content
        let backup_path = format!("{}.backup", temp_file.path().to_str().unwrap());
        assert!(std::path::Path::new(&backup_path).exists());

        let backup_content = fs::read_to_string(&backup_path).unwrap();
        assert_eq!(backup_content, original_content);
    }

    #[test]
    fn test_atomic_write() {
        let temp_file = NamedTempFile::new().unwrap();
        let original_content = "line1\nline2\nline3";
        fs::write(temp_file.path(), original_content).unwrap();

        // Perform edit
        let result =
            EditTool::edit_line(temp_file.path().to_str().unwrap(), 2, "modified line2").unwrap();

        assert!(result.success);

        // File should contain complete, valid content (not partial)
        let new_content = fs::read_to_string(temp_file.path()).unwrap();
        let lines: Vec<&str> = new_content.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[1], "modified line2");

        // Temporary file should be cleaned up
        let temp_path = format!("{}.tmp", temp_file.path().to_str().unwrap());
        assert!(!std::path::Path::new(&temp_path).exists());
    }
}

#[cfg(test)]
mod line_ending_tests {
    use super::*;

    #[test]
    fn test_windows_line_endings() {
        let temp_file = NamedTempFile::new().unwrap();
        let windows_content = "line1\r\nline2\r\nline3\r\n";
        fs::write(temp_file.path(), windows_content).unwrap();

        let result =
            EditTool::edit_line(temp_file.path().to_str().unwrap(), 2, "modified line2").unwrap();

        assert!(result.success);

        // Should normalize to Unix line endings
        let new_content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(!new_content.contains("\r"));
        assert!(new_content.contains("modified line2"));
    }

    #[test]
    fn test_mixed_line_endings() {
        let temp_file = NamedTempFile::new().unwrap();
        let mixed_content = "line1\nline2\r\nline3\n";
        fs::write(temp_file.path(), mixed_content).unwrap();

        let result = EditTool::edit_text(
            temp_file.path().to_str().unwrap(),
            "line2",
            "modified line2",
        )
        .unwrap();

        assert!(result.success);

        let new_content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(!new_content.contains("\r"));
        assert!(new_content.contains("modified line2"));
    }
}

#[cfg(test)]
mod before_after_state_tests {
    use super::*;

    #[test]
    fn test_before_after_state_tracking() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), RUST_STRUCT_CODE).unwrap();

        let result = EditTool::edit_line(
            temp_file.path().to_str().unwrap(),
            5, // pub name: String, line
            "pub display_name: String,",
        )
        .unwrap();

        assert_eq!(result.old_content, "    pub name: String,");
        assert_eq!(result.new_content, "    pub display_name: String,");

        // Context should show the struct definition
        assert!(
            result
                .context_before
                .iter()
                .any(|line| line.contains("pub struct User"))
        );
        assert!(
            result
                .context_after
                .iter()
                .any(|line| line.contains("pub email: String,"))
        );
    }

    #[test]
    fn test_message_content() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), "test line").unwrap();

        let result =
            EditTool::edit_line(temp_file.path().to_str().unwrap(), 1, "modified test line")
                .unwrap();

        assert!(result.message.contains("Successfully changed line 1"));
        assert!(result.message.contains("from 'test line'"));
        assert!(result.message.contains("to 'modified test line'"));
    }

    #[test]
    fn test_file_path_tracking() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), "content").unwrap();

        let file_path = temp_file.path().to_str().unwrap();
        let result = EditTool::edit_line(file_path, 1, "new content").unwrap();

        assert_eq!(result.file_path, file_path);
    }
}

#[cfg(test)]
mod comprehensive_integration_tests {
    use super::*;

    #[test]
    fn test_complete_code_refactoring_workflow() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), RUST_STRUCT_CODE).unwrap();

        let file_path = temp_file.path().to_str().unwrap();

        // Step 1: Add a new field
        let result = EditTool::edit_text(
            file_path,
            "pub settings: HashMap<String, String>,",
            "pub settings: HashMap<String, String>,\n    pub active: bool,",
        )
        .unwrap();
        assert!(result.success);

        // Step 2: Update constructor
        let result = EditTool::edit_text(
            file_path,
            "settings: HashMap::new(),",
            "settings: HashMap::new(),\n            active: true,",
        )
        .unwrap();
        assert!(result.success);

        // Step 3: Add a new method
        let result = EditTool::edit_text(
            file_path,
            "    pub fn get_display_name(&self) -> &str {\n        &self.name\n    }",
            "    pub fn get_display_name(&self) -> &str {\n        &self.name\n    }\n    \n    pub fn is_active(&self) -> bool {\n        self.active\n    }"
        ).unwrap();
        assert!(result.success);

        // Verify final result
        let final_content = fs::read_to_string(file_path).unwrap();
        assert!(final_content.contains("pub active: bool,"));
        assert!(final_content.contains("active: true,"));
        assert!(final_content.contains("pub fn is_active(&self) -> bool"));
    }

    #[test]
    fn test_error_recovery_workflow() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), SIMPLE_RUST_CODE).unwrap();

        let file_path = temp_file.path().to_str().unwrap();

        // Attempt an invalid edit
        let result = EditTool::edit_line(file_path, 999, "invalid");
        assert!(result.is_err());

        // File should be unchanged
        let content = fs::read_to_string(file_path).unwrap();
        assert_eq!(content, SIMPLE_RUST_CODE);

        // Valid edit should still work
        let result = EditTool::edit_line(file_path, 2, "println!(\"Hello, recovery!\");").unwrap();
        assert!(result.success);

        let final_content = fs::read_to_string(file_path).unwrap();
        assert!(final_content.contains("Hello, recovery!"));
    }

    #[test]
    fn test_concurrent_edit_simulation() {
        let temp_file = NamedTempFile::new().unwrap();
        let multi_line = "line1\nline2\nline3\nline4\nline5";
        fs::write(temp_file.path(), multi_line).unwrap();

        let file_path = temp_file.path().to_str().unwrap();

        // Simulate rapid edits (as might happen from multiple agents)
        let start = Instant::now();

        for i in 1..=5 {
            let result = EditTool::edit_line(file_path, i, &format!("modified line{}", i));
            assert!(result.is_ok());
        }

        let duration = start.elapsed();

        // Should complete all edits quickly
        assert!(duration.as_millis() < 10);

        // Verify all changes were applied
        let final_content = fs::read_to_string(file_path).unwrap();
        for i in 1..=5 {
            assert!(final_content.contains(&format!("modified line{}", i)));
        }
    }
}
