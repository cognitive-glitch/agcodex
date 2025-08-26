//! Unified code tools scaffolding for AGCodex.
//!
//! This module provides a comprehensive suite of code analysis and search tools with:
//! - Unified trait system for polymorphic tool usage
//! - Async execution with proper cancellation support
//! - Streaming results for large codebases
//! - Builder patterns for complex queries
//! - Integration with existing sandbox/security systems
//! - Performance optimization through caching and result streaming
//!
//! ## Architecture Overview
//!
//! The module is organized into several layers:
//! 1. **Traits Layer**: Unified interfaces (`UnifiedSearch`, `ToolDiscovery`, etc.)
//! 2. **Tool Implementations**: Specific wrappers (ripgrep, fd, ast-grep, srgn)
//! 3. **Query Builders**: Fluent APIs for complex search configurations
//! 4. **Execution Layer**: Async command execution with sandboxing
//! 5. **Result Processing**: Streaming parsers and result aggregation
//!
//! ## Policy Notes
//! - Tree-sitter is the primary structural engine
//! - AST-Grep is offered as optional internal tooling
//! - Comby is intentionally excluded per project policy
//! - All external tools are executed through the existing sandbox system

use dashmap::DashMap;
use std::fmt;
use std::sync::Arc;
use thiserror::Error;

use which;

/// Comprehensive error types for code tools operations
#[derive(Debug, Error)]
pub enum ToolError {
    #[error("tool not implemented: {0}")]
    NotImplemented(&'static str),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("invalid query: {0}")]
    InvalidQuery(String),

    #[error("parse error: {0}")]
    ParseError(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("tool execution failed: {0}")]
    ExecutionError(String),

    #[error("operation cancelled")]
    Cancelled,

    #[error("timeout exceeded: {0}ms")]
    Timeout(u64),

    #[error("security violation: {0}")]
    SecurityViolation(String),

    #[error("serialization error: {0}")]
    SerializationError(String),

    #[error("regex error: {0}")]
    RegexError(String),
}

/// Legacy trait for backward compatibility
///
/// Note: New code should use `UnifiedSearch` trait instead
pub trait CodeTool {
    type Query;
    type Output;
    fn search(&self, _query: Self::Query) -> Result<Self::Output, ToolError>;
}

// Core modules
pub mod traits;

// Tool implementations
pub mod ast_grep;
pub mod fd_find;
pub mod ripgrep;
pub mod srgn;

// Existing modules
pub mod ast_agent_tools;
pub mod queries;
pub mod search;
pub mod tree_sitter;

// Re-export commonly used types for convenience
pub use traits::CommandExecutor;
pub use traits::ProgressReporter;
pub use traits::ResultCache;
pub use traits::SearchConfig;
pub use traits::SearchConfigBuilder;
pub use traits::SearchResult;
pub use traits::SearchStats;
pub use traits::SearchType;
pub use traits::StreamingSearchResult;
pub use traits::ToolDiscovery;
pub use traits::UnifiedSearch;

pub use ripgrep::Ripgrep;
pub use ripgrep::RipgrepQuery;
pub use ripgrep::RipgrepQueryBuilder;
pub use srgn::Srgn;
pub use srgn::SrgnLanguage;
pub use srgn::SrgnOperation;
pub use srgn::SrgnQuery;
pub use srgn::SrgnQueryBuilder;

// Intentionally no `comby` module: Comby is not used in AGCodex.

// Tool registry for dynamic tool management
#[derive(Clone)]
pub struct ToolRegistry {
    /// Available tools mapped by name
    tools:
        Arc<DashMap<String, Box<dyn UnifiedSearch<Query = Box<dyn std::any::Any + Send + Sync>>>>>,
    /// Tool metadata
    metadata: Arc<DashMap<String, ToolMetadata>>,
}

/// Manual Debug implementation since trait objects can't auto-derive Debug
impl fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("tools_count", &self.tools.len())
            .field("metadata", &self.metadata)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct ToolMetadata {
    pub name: String,
    pub version: String,
    pub search_type: SearchType,
    pub is_available: bool,
    pub capabilities: Vec<String>,
}

impl ToolRegistry {
    /// Create a new tool registry
    pub fn new() -> Self {
        Self {
            tools: Arc::new(DashMap::new()),
            metadata: Arc::new(DashMap::new()),
        }
    }

    /// Register a tool with the registry
    pub fn register_tool(&self, name: String, metadata: ToolMetadata) {
        self.metadata.insert(name.clone(), metadata);
    }

    /// Get available tool names
    pub fn get_available_tools(&self) -> Vec<String> {
        self.metadata
            .iter()
            .filter_map(|entry| {
                if entry.value().is_available {
                    Some(entry.key().clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get tool metadata
    pub fn get_tool_metadata(&self, name: &str) -> Option<ToolMetadata> {
        self.metadata.get(name).map(|entry| entry.value().clone())
    }

    /// Check if a tool is available
    pub fn is_tool_available(&self, name: &str) -> bool {
        self.metadata
            .get(name)
            .map(|entry| entry.value().is_available)
            .unwrap_or(false)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Factory for creating tool instances
pub struct ToolFactory;

impl ToolFactory {
    /// Create a ripgrep instance if available
    pub async fn create_ripgrep() -> Result<Ripgrep, ToolError> {
        Ripgrep::new().await
    }

    /// Create an SRGN instance if available
    pub async fn create_srgn() -> Result<Srgn, ToolError> {
        Srgn::new().await
    }

    /// Auto-discover and create available tools
    pub async fn discover_all_tools() -> ToolRegistry {
        let registry = ToolRegistry::new();

        // Try to create ripgrep
        if let Ok(_) = Self::create_ripgrep().await {
            registry.register_tool(
                "ripgrep".to_string(),
                ToolMetadata {
                    name: "ripgrep".to_string(),
                    version: "unknown".to_string(), // Could query actual version
                    search_type: SearchType::Content,
                    is_available: true,
                    capabilities: vec![
                        "regex".to_string(),
                        "literal".to_string(),
                        "context".to_string(),
                        "file_types".to_string(),
                    ],
                },
            );
        }

        // Try to create SRGN
        if let Ok(_) = Self::create_srgn().await {
            registry.register_tool(
                "srgn".to_string(),
                ToolMetadata {
                    name: "srgn".to_string(),
                    version: "unknown".to_string(),
                    search_type: SearchType::Structural,
                    is_available: true,
                    capabilities: vec![
                        "syntax_aware".to_string(),
                        "replace".to_string(),
                        "extract".to_string(),
                        "validate".to_string(),
                    ],
                },
            );
        }

        registry
    }
}

/// Utility functions for tool management
pub mod utils {
    use super::*;
    use std::collections::HashMap;

    /// Get system information about available code tools
    pub async fn get_tool_system_info() -> HashMap<String, String> {
        let mut info = HashMap::new();

        // Check ripgrep
        match which::which("rg") {
            Ok(path) => {
                info.insert(
                    "ripgrep_path".to_string(),
                    path.to_string_lossy().to_string(),
                );

                // Try to get version
                if let Ok(output) = tokio::process::Command::new("rg")
                    .arg("--version")
                    .output()
                    .await
                    && output.status.success() {
                        let version = String::from_utf8_lossy(&output.stdout)
                            .lines()
                            .next()
                            .unwrap_or("Unknown")
                            .to_string();
                        info.insert("ripgrep_version".to_string(), version);
                    }
            }
            Err(_) => {
                info.insert("ripgrep_status".to_string(), "not_found".to_string());
            }
        }

        // Check fd
        match which::which("fd") {
            Ok(path) => {
                info.insert("fd_path".to_string(), path.to_string_lossy().to_string());
            }
            Err(_) => {
                info.insert("fd_status".to_string(), "not_found".to_string());
            }
        }

        // Check srgn
        match which::which("srgn") {
            Ok(path) => {
                info.insert("srgn_path".to_string(), path.to_string_lossy().to_string());
            }
            Err(_) => {
                info.insert("srgn_status".to_string(), "not_found".to_string());
            }
        }

        // Check ast-grep
        match which::which("ast-grep") {
            Ok(path) => {
                info.insert(
                    "ast_grep_path".to_string(),
                    path.to_string_lossy().to_string(),
                );
            }
            Err(_) => {
                info.insert("ast_grep_status".to_string(), "not_found".to_string());
            }
        }

        info
    }

    /// Validate that required tools are available
    pub async fn validate_required_tools(tools: &[&str]) -> Result<(), ToolError> {
        let mut missing = Vec::new();

        for &tool in tools {
            match which::which(tool) {
                Ok(_) => {}
                Err(_) => missing.push(tool),
            }
        }

        if !missing.is_empty() {
            return Err(ToolError::NotFound(format!(
                "Required tools not found: {}",
                missing.join(", ")
            )));
        }

        Ok(())
    }
}

// Include comprehensive tests
#[cfg(test)]
mod ast_agent_tools_test;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_registry_creation() {
        let registry = ToolRegistry::new();
        assert!(registry.get_available_tools().is_empty());
    }

    #[tokio::test]
    async fn test_tool_metadata() {
        let registry = ToolRegistry::new();
        let metadata = ToolMetadata {
            name: "test_tool".to_string(),
            version: "1.0.0".to_string(),
            search_type: SearchType::Content,
            is_available: true,
            capabilities: vec!["test".to_string()],
        };

        registry.register_tool("test_tool".to_string(), metadata.clone());

        assert!(registry.is_tool_available("test_tool"));
        assert_eq!(
            registry.get_available_tools(),
            vec!["test_tool".to_string()]
        );

        let retrieved = registry.get_tool_metadata("test_tool").unwrap();
        assert_eq!(retrieved.name, "test_tool");
        assert_eq!(retrieved.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_tool_factory_discovery() {
        let registry = ToolFactory::discover_all_tools().await;
        // Should not panic and should return a registry
        let tools = registry.get_available_tools();
        // May be empty if tools not installed, but shouldn't crash
        println!("Discovered tools: {:?}", tools);
    }

    #[tokio::test]
    async fn test_system_info() {
        let info = utils::get_tool_system_info().await;
        // Should return some information
        assert!(!info.is_empty());
        println!("System info: {:?}", info);
    }
}
