//! Unified tool registry for AGCodex
//!
//! Provides a simple, discoverable interface for all tools.
//! Avoids complexity and makes tools easy to use for LLMs.

use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

/// Unified error type for all tools
#[derive(Debug, Error)]
pub enum ToolError {
    #[error("tool not found: {0}")]
    NotFound(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("execution failed: {0}")]
    ExecutionFailed(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(String),
}

impl From<serde_json::Error> for ToolError {
    fn from(e: serde_json::Error) -> Self {
        ToolError::Serialization(e.to_string())
    }
}

/// Tool category for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCategory {
    /// Search tools: search, grep, glob
    Search,
    /// Edit tools: edit, patch
    Edit,
    /// Analysis tools: think, plan, tree
    Analysis,
    /// Utility tools: index, bash
    Utility,
}

impl ToolCategory {
    pub const fn as_str(&self) -> &'static str {
        match self {
            ToolCategory::Search => "search",
            ToolCategory::Edit => "edit",
            ToolCategory::Analysis => "analysis",
            ToolCategory::Utility => "utility",
        }
    }
}

/// Simple tool executor function signature
pub type ToolExecutor = fn(Value) -> Result<ToolOutput, ToolError>;

/// Information about a registered tool
#[derive(Clone)]
pub struct ToolInfo {
    /// Tool name (e.g., "search", "edit", "think")
    pub name: &'static str,
    /// Brief description for discovery
    pub description: &'static str,
    /// Category for organization
    pub category: ToolCategory,
    /// Example usage
    pub example: &'static str,
    /// Executor function
    pub execute: ToolExecutor,
}

/// Simple, LLM-friendly output format
#[derive(Debug, Clone)]
pub struct ToolOutput {
    /// Whether the operation succeeded
    pub success: bool,
    /// Main result as JSON
    pub result: Value,
    /// One-line summary for LLMs
    pub summary: String,
    /// Performance metric in milliseconds
    pub duration_ms: u64,
}

impl ToolOutput {
    /// Create a successful output
    pub fn success(result: Value, summary: impl Into<String>) -> Self {
        ToolOutput {
            success: true,
            result,
            summary: summary.into(),
            duration_ms: 0,
        }
    }

    /// Create a failed output
    pub fn failure(error: impl Into<String>) -> Self {
        ToolOutput {
            success: false,
            result: Value::Null,
            summary: error.into(),
            duration_ms: 0,
        }
    }

    /// Set the duration
    pub const fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }
}

/// Unified tool registry for discovery and invocation
pub struct ToolRegistry {
    tools: HashMap<String, ToolInfo>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        ToolRegistry {
            tools: HashMap::new(),
        }
    }

    /// Register a tool
    pub fn register(&mut self, info: ToolInfo) {
        self.tools.insert(info.name.to_string(), info);
    }

    /// List all available tools
    pub fn list_tools(&self) -> Vec<&str> {
        let mut tools: Vec<&str> = self.tools.keys().map(|s| s.as_str()).collect();
        tools.sort();
        tools
    }

    /// List tools by category
    pub fn list_by_category(&self, category: ToolCategory) -> Vec<&str> {
        let mut tools: Vec<&str> = self
            .tools
            .values()
            .filter(|info| info.category == category)
            .map(|info| info.name)
            .collect();
        tools.sort();
        tools
    }

    /// Get tool information
    pub fn get_info(&self, name: &str) -> Option<&ToolInfo> {
        self.tools.get(name)
    }

    /// Execute a tool by name
    pub fn execute(&self, name: &str, input: Value) -> Result<ToolOutput, ToolError> {
        let start = std::time::Instant::now();

        let info = self
            .tools
            .get(name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;

        let mut output = (info.execute)(input)?;
        output.duration_ms = start.elapsed().as_millis() as u64;

        Ok(output)
    }

    /// Get a discovery manifest for LLMs
    pub fn get_manifest(&self) -> Value {
        let tools: Vec<Value> = self
            .tools
            .values()
            .map(|info| {
                serde_json::json!({
                    "name": info.name,
                    "description": info.description,
                    "category": info.category.as_str(),
                    "example": info.example,
                })
            })
            .collect();

        serde_json::json!({
            "version": "1.0",
            "tools": tools,
            "categories": ["search", "edit", "analysis", "utility"],
        })
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_tool(_input: Value) -> Result<ToolOutput, ToolError> {
        Ok(ToolOutput::success(
            serde_json::json!({"test": "result"}),
            "Test completed",
        ))
    }

    #[test]
    fn test_registry_basic() {
        let mut registry = ToolRegistry::new();

        registry.register(ToolInfo {
            name: "test",
            description: "Test tool",
            category: ToolCategory::Utility,
            example: "{}",
            execute: dummy_tool,
        });

        assert_eq!(registry.list_tools(), vec!["test"]);
        assert!(registry.get_info("test").is_some());
        assert!(registry.get_info("nonexistent").is_none());
    }

    #[test]
    fn test_registry_execute() {
        let mut registry = ToolRegistry::new();

        registry.register(ToolInfo {
            name: "test",
            description: "Test tool",
            category: ToolCategory::Utility,
            example: "{}",
            execute: dummy_tool,
        });

        let result = registry.execute("test", serde_json::json!({})).unwrap();
        assert!(result.success);
        assert_eq!(result.summary, "Test completed");
    }

    #[test]
    fn test_registry_categories() {
        let mut registry = ToolRegistry::new();

        registry.register(ToolInfo {
            name: "search",
            description: "Search tool",
            category: ToolCategory::Search,
            example: "{}",
            execute: dummy_tool,
        });

        registry.register(ToolInfo {
            name: "edit",
            description: "Edit tool",
            category: ToolCategory::Edit,
            example: "{}",
            execute: dummy_tool,
        });

        assert_eq!(
            registry.list_by_category(ToolCategory::Search),
            vec!["search"]
        );
        assert_eq!(registry.list_by_category(ToolCategory::Edit), vec!["edit"]);
        assert!(registry.list_by_category(ToolCategory::Analysis).is_empty());
    }

    #[test]
    fn test_tool_not_found() {
        let registry = ToolRegistry::new();
        let result = registry.execute("nonexistent", serde_json::json!({}));

        assert!(matches!(result, Err(ToolError::NotFound(_))));
    }

    #[test]
    fn test_manifest() {
        let mut registry = ToolRegistry::new();

        registry.register(ToolInfo {
            name: "test",
            description: "Test tool",
            category: ToolCategory::Utility,
            example: r#"{"input": "test"}"#,
            execute: dummy_tool,
        });

        let manifest = registry.get_manifest();
        assert!(manifest["tools"].is_array());
        assert_eq!(manifest["tools"][0]["name"], "test");
    }
}
