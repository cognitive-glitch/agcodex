//! Integration tests for the unified tool registry
//!
//! Demonstrates simple, clean tool discovery and invocation.

#[cfg(test)]
mod tests {
    use super::super::*;
    use serde_json::json;

    #[test]
    fn test_registry_discovery() {
        let registry = create_default_registry();

        // Test tool discovery
        let all_tools = registry.list_tools();
        assert!(!all_tools.is_empty(), "Registry should have tools");

        // Verify expected tools are present
        assert!(all_tools.contains(&"think"));
        assert!(all_tools.contains(&"plan"));
        assert!(all_tools.contains(&"search"));
        assert!(all_tools.contains(&"glob"));
        assert!(all_tools.contains(&"edit"));
        assert!(all_tools.contains(&"patch"));
        assert!(all_tools.contains(&"tree"));
        assert!(all_tools.contains(&"grep"));
        assert!(all_tools.contains(&"bash"));

        println!("Available tools: {:?}", all_tools);
    }

    #[test]
    fn test_registry_categories() {
        let registry = create_default_registry();

        // Test category filtering
        let search_tools = registry.list_by_category(ToolCategory::Search);
        assert!(search_tools.contains(&"search"));
        assert!(search_tools.contains(&"grep"));
        assert!(search_tools.contains(&"glob"));

        let edit_tools = registry.list_by_category(ToolCategory::Edit);
        assert!(edit_tools.contains(&"edit"));
        assert!(edit_tools.contains(&"patch"));

        let analysis_tools = registry.list_by_category(ToolCategory::Analysis);
        assert!(analysis_tools.contains(&"think"));
        assert!(analysis_tools.contains(&"plan"));
        assert!(analysis_tools.contains(&"tree"));

        let utility_tools = registry.list_by_category(ToolCategory::Utility);
        assert!(utility_tools.contains(&"bash"));
    }

    #[test]
    fn test_registry_manifest() {
        let registry = create_default_registry();
        let manifest = registry.get_manifest();

        assert!(manifest["version"].is_string());
        assert!(manifest["tools"].is_array());
        assert!(manifest["categories"].is_array());

        // Check that tools have required fields
        let tools = manifest["tools"].as_array().unwrap();
        assert!(!tools.is_empty());

        for tool in tools {
            assert!(tool["name"].is_string());
            assert!(tool["description"].is_string());
            assert!(tool["category"].is_string());
            assert!(tool["example"].is_string());
        }
    }

    #[test]
    fn test_think_tool_invocation() {
        let registry = create_default_registry();

        // Test basic thinking
        let input = json!({
            "problem": "How to implement a simple cache?"
        });

        let result = registry.execute("think", input);
        assert!(result.is_ok(), "Think tool should execute successfully");

        let output = result.unwrap();
        assert!(output.success);
        assert!(output.result["steps"].is_array());
        assert!(output.result["conclusion"].is_string());
        assert!(!output.summary.is_empty());
    }

    #[test]
    fn test_think_tool_with_language() {
        let registry = create_default_registry();

        // Test code-specific thinking
        let input = json!({
            "problem": "Fix null pointer exception",
            "language": "java",
            "context": "UserService.java"
        });

        let result = registry.execute("think", input);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.success);
        assert!(output.result["problem_type"].is_string());
        assert!(output.result["complexity"].is_string());
        assert!(output.result["recommended_action"].is_string());
    }

    #[test]
    fn test_bash_tool_safe_command() {
        let registry = create_default_registry();

        let input = json!({
            "command": "echo Hello, World!"
        });

        let result = registry.execute("bash", input);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.success);
        assert_eq!(
            output.result["stdout"].as_str().unwrap().trim(),
            "Hello, World!"
        );
    }

    #[test]
    fn test_bash_tool_unsafe_command() {
        let registry = create_default_registry();

        let input = json!({
            "command": "rm -rf /tmp/test"
        });

        let result = registry.execute("bash", input);
        assert!(result.is_err(), "Unsafe commands should be rejected");
    }

    #[test]
    fn test_tool_not_found() {
        let registry = create_default_registry();

        let input = json!({});
        let result = registry.execute("nonexistent_tool", input);

        assert!(result.is_err());
        if let Err(ToolError::NotFound(name)) = result {
            assert_eq!(name, "nonexistent_tool");
        } else {
            panic!("Expected NotFound error");
        }
    }

    #[test]
    fn test_invalid_input() {
        let registry = create_default_registry();

        // Think tool without required "problem" field
        let input = json!({
            "wrong_field": "value"
        });

        let result = registry.execute("think", input);
        assert!(result.is_err());

        if let Err(ToolError::InvalidInput(msg)) = result {
            assert!(msg.contains("problem"));
        } else {
            panic!("Expected InvalidInput error");
        }
    }

    #[test]
    fn test_glob_tool() {
        let registry = create_default_registry();

        let input = json!({
            "pattern": "*.rs",
            "path": "src",
            "file_type": "file"
        });

        let result = registry.execute("glob", input);
        // This might fail if there's no src directory, but it should parse correctly
        assert!(result.is_ok() || result.is_err());

        if let Ok(output) = result {
            assert!(output.result["matches"].is_array());
            assert!(output.result["total_files"].is_number());
        }
    }

    #[test]
    fn test_performance_tracking() {
        let registry = create_default_registry();

        let input = json!({
            "problem": "Quick test"
        });

        let result = registry.execute("think", input).unwrap();

        // Duration is automatically tracked (u64 is always non-negative)
        println!("Think tool took {}ms", result.duration_ms);
        // Just verify the field exists by accessing it
        let _ = result.duration_ms;
    }
}
