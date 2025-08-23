//! Integration test for GlobTool with other AGCodex components
//!
//! This test ensures the GlobTool integrates properly with the codebase
//! and provides the expected API interface.

#[cfg(test)]
mod tests {
    use crate::tools::glob::ContentCategory;

    use crate::tools::GlobTool;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Create a test workspace with various file types
    fn create_test_workspace() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Initialize as a git repository so .gitignore works properly
        // If git is not available, we'll also create a .ignore file as fallback
        let git_init = std::process::Command::new("git")
            .arg("init")
            .current_dir(root)
            .output()
            .is_ok();

        // Create source files
        fs::create_dir(root.join("src")).unwrap();
        fs::write(root.join("src").join("main.rs"), "fn main() {}").unwrap();
        fs::write(root.join("src").join("lib.rs"), "pub mod lib {}").unwrap();

        // Create test files
        fs::create_dir(root.join("tests")).unwrap();
        fs::write(
            root.join("tests").join("integration_test.rs"),
            "#[test] fn test() {}",
        )
        .unwrap();
        fs::write(root.join("src").join("main_test.rs"), "mod tests {}").unwrap();

        // Create config files
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        fs::write(root.join("config.json"), "{}").unwrap();

        // Create documentation
        fs::write(root.join("README.md"), "# Test Project").unwrap();

        // Create gitignore with proper line endings
        fs::write(root.join(".gitignore"), "target/\n*.tmp\n").unwrap();
        
        // Also create .ignore file for non-git environments
        // The ignore crate respects .ignore files even without git
        if !git_init {
            fs::write(root.join(".ignore"), "target/\n*.tmp\n").unwrap();
        }

        // Create ignored files and directories
        fs::create_dir(root.join("target")).unwrap();
        fs::write(root.join("target").join("debug.txt"), "debug info").unwrap();
        fs::write(root.join("ignored.tmp"), "temporary").unwrap();

        temp_dir
    }

    #[test]
    fn test_basic_glob_functionality() {
        let workspace = create_test_workspace();
        let glob_tool = GlobTool::new(workspace.path().to_path_buf());

        // Test basic glob pattern
        let result = glob_tool.glob("*.rs").unwrap();

        // Should find source files but not deeply nested ones with this pattern
        assert!(!result.result.is_empty());
        assert!(result.summary.contains("Found"));
        assert!(result.metadata.operation == "file_discovery");
        assert!(result.performance.execution_time.as_millis() > 0);
    }

    #[test]
    fn test_find_type_method() {
        let workspace = create_test_workspace();
        let glob_tool = GlobTool::new(workspace.path().to_path_buf());

        // Test find_type method
        let result = glob_tool.find_type("rs").unwrap();

        // Should find all .rs files recursively
        assert!(result.result.len() >= 3); // main.rs, lib.rs, main_test.rs, integration_test.rs
        assert!(
            result
                .result
                .iter()
                .all(|f| f.extension.as_ref().unwrap() == "rs")
        );
    }

    #[test]
    fn test_file_type_classification() {
        let workspace = create_test_workspace();
        let glob_tool = GlobTool::new(workspace.path().to_path_buf());

        let result = glob_tool.glob("*").unwrap();

        // Verify content category classification
        let has_source = result
            .result
            .iter()
            .any(|f| f.content_category == ContentCategory::Source);
        let has_config = result
            .result
            .iter()
            .any(|f| f.content_category == ContentCategory::Config);
        let has_docs = result
            .result
            .iter()
            .any(|f| f.content_category == ContentCategory::Documentation);
        let has_test = result
            .result
            .iter()
            .any(|f| f.content_category == ContentCategory::Test);

        assert!(has_source, "Should classify some files as source");
        assert!(has_config, "Should classify some files as config");
        assert!(has_docs, "Should classify some files as documentation");
        assert!(has_test, "Should classify some files as test");
    }

    #[test]
    fn test_gitignore_support() {
        let workspace = create_test_workspace();
        let glob_tool = GlobTool::new(workspace.path().to_path_buf())
            .with_respect_ignore(true);  // Explicitly enable .gitignore respect

        // Test with recursive pattern to find all files
        let result = glob_tool.glob("**/*").unwrap();

        // Should not find ignored files
        assert!(
            !result
                .result
                .iter()
                .any(|f| f.relative_path.to_string_lossy().contains("target")),
            "Should not find files in target directory"
        );
        assert!(
            !result
                .result
                .iter()
                .any(|f| f.path.file_name().unwrap() == "ignored.tmp"),
            "Should not find ignored.tmp file"
        );
        
        // Test with simple pattern for .tmp files
        let tmp_result = glob_tool.glob("*.tmp").unwrap();
        assert_eq!(tmp_result.result.len(), 0, "Should not find any .tmp files due to .gitignore");
    }

    #[test]
    fn test_find_in_directory() {
        let workspace = create_test_workspace();
        let glob_tool = GlobTool::new(workspace.path().to_path_buf());

        // Test searching in subdirectory
        let src_dir = workspace.path().join("src");
        let result = glob_tool.find_in_directory(&src_dir, "*.rs").unwrap();

        // Should find files in src directory only
        assert!(!result.result.is_empty());
        assert!(result.result.iter().all(|f| f.path.starts_with(&src_dir)));
    }

    #[test]
    fn test_parallel_vs_sequential() {
        let workspace = create_test_workspace();

        let parallel_tool = GlobTool::new(workspace.path().to_path_buf()).with_parallelism(4);
        let sequential_tool = GlobTool::new(workspace.path().to_path_buf()).with_parallelism(1);

        let parallel_result = parallel_tool.glob("*").unwrap();
        let sequential_result = sequential_tool.glob("*").unwrap();

        // Both should find the same number of files
        assert_eq!(parallel_result.result.len(), sequential_result.result.len());

        // Both should complete successfully with different execution characteristics
        assert!(parallel_result.performance.execution_time.as_millis() > 0);
        assert!(sequential_result.performance.execution_time.as_millis() > 0);
    }

    #[test]
    fn test_max_results_limit() {
        let workspace = create_test_workspace();
        let glob_tool = GlobTool::new(workspace.path().to_path_buf()).with_max_results(Some(2));

        let result = glob_tool.glob("*").unwrap();

        // Should be limited to max results
        assert!(result.result.len() <= 2);
    }

    #[test]
    fn test_error_handling() {
        // Test with non-existent directory
        let non_existent = PathBuf::from("/non/existent/path");
        let glob_tool = GlobTool::new(non_existent);

        let result = glob_tool.glob("*");
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_output_structure() {
        let workspace = create_test_workspace();
        let glob_tool = GlobTool::new(workspace.path().to_path_buf());

        let result = glob_tool.glob("*.rs").unwrap();

        // Verify complete ToolOutput structure
        assert!(!result.result.is_empty());
        assert!(result.metadata.tool == "glob");
        assert!(!result.summary.is_empty());
        assert!(result.performance.memory_usage.peak_bytes > 0);
        assert!(result.metadata.completed_at >= result.metadata.started_at);

        // Verify metadata completeness
        assert_eq!(result.metadata.operation, "file_discovery");
        assert!(result.performance.io_stats.read_ops > 0);
        assert!(result.performance.execution_time.as_millis() > 0);
    }
}
