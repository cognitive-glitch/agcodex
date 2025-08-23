//! Internal agent tools for AGCodex
//!
//! This module provides sophisticated tools for task planning, code analysis,
//! and automated workflow management. These tools are designed to work seamlessly
//! with the subagent orchestration system.

pub mod edit;
pub mod glob;
pub mod index;
pub mod integration_example;
pub mod output;
pub mod patch;
pub mod plan;
pub mod think;
pub mod tree;

// Unified tool registry and adapters
pub mod adapters;
pub mod registry;

#[cfg(test)]
mod integration_test_glob;

#[cfg(test)]
mod integration_test_registry;

// Use simplified grep implementation to avoid ast-grep API issues
pub mod grep_simple;
pub use grep_simple::GrepConfig;
pub use grep_simple::GrepError;
pub use grep_simple::GrepMatch;
pub use grep_simple::GrepQuery;
pub use grep_simple::GrepResult;
pub use grep_simple::GrepTool;
pub use grep_simple::RuleType;
pub use grep_simple::SupportedLanguage as GrepSupportedLanguage;

pub use edit::AmbiguousMatches;
pub use edit::EditError;
pub use edit::EditResult;
pub use edit::EditTool;
pub use edit::MatchCandidate;
pub use glob::FileMatch;
pub use glob::FileType;
pub use glob::GlobError;
pub use glob::GlobResult;
pub use glob::GlobTool;
pub use index::BuildInput;
pub use index::IndexConfig;
pub use index::IndexError;
pub use index::IndexResult;
pub use index::IndexStats;
pub use index::IndexTool;
pub use index::IndexedDocument;
pub use index::MergePolicyConfig;
pub use index::SearchInput;
pub use index::SearchQuery;
pub use index::SearchResult;
pub use index::Symbol;
pub use index::UpdateInput;
pub use output::{
    ApiCompatibility,
    BreakingChange,
    BreakingSeverity,
    CacheStats,
    Change,
    ChangeBuilder,
    ChangeKind,
    ComplexityChange,
    ComplexityMetrics,
    ComprehensiveAstSummary,
    ComprehensivePerformanceImpact,
    ComprehensiveSemanticImpact,
    ComprehensiveSymbol,
    // Core output types
    ComprehensiveToolOutput,
    ContextLine,
    ContextLineType,
    ContextSnapshot,
    CoverageImpact,
    CpuUsage,
    DependencyInfo,
    Diagnostic,
    DiagnosticLevel,
    HalsteadMetrics,
    ImpactLevel,
    ImpactScope,
    ImportChangeType,
    IoStats,
    LanguageContext,
    MemoryImpact,
    MemoryUsage,
    ModificationType,
    OperationContext,
    OperationMetadata,
    OperationScope,
    // Builder patterns
    OutputBuilder,
    PerformanceChange,
    PerformanceMetrics,
    // Performance timing
    PerformanceTimer,
    ProjectContext,
    RefactorType,
    // Extension traits
    ResultExt,
    ScopeType,
    SecurityImpact,
    SecurityLevel,
    SecurityVulnerability,
    SymbolType,
    TestCategory,
    TestImpact,
    VersionImpact,
    Visibility,
    multi_file_changes,
    simple_error,
    // Convenience functions
    simple_success,
    single_file_modification,
};
// Simplified patch tool exports
pub use patch::ExtractStats;
pub use patch::ImportStats;
pub use patch::PatchError;
pub use patch::PatchResult;
pub use patch::PatchTool;
pub use patch::RenameScope;
pub use patch::RenameStats;
pub use plan::AgentType;
pub use plan::DependencyGraph;
pub use plan::MetaTask;
pub use plan::MetaTaskPlanner;
pub use plan::Plan;
pub use plan::PlanContext;
pub use plan::PlanError;
pub use plan::PlanExecutionPlan;
pub use plan::PlanExecutionStep;
pub use plan::PlanIntelligenceLevel;
pub use plan::PlanTool;
pub use plan::SubTask;
pub use plan::SubTaskPlanner;
pub use plan::TaskGroup;
pub use plan::TaskPriority;
pub use plan::TaskStatus;
// ToolOutput is already exported from output module, no need to import from plan
pub use think::ThinkError;
pub use think::ThinkResult;
pub use think::ThinkStep;
pub use think::ThinkTool;
pub use tree::DiffNode;
pub use tree::ModifiedNode;
pub use tree::MovedNode;
pub use tree::ParsedAst;
pub use tree::Point;
pub use tree::QueryCapture;
pub use tree::QueryMatch;
pub use tree::SemanticDiff;
pub use tree::SupportedLanguage;
pub use tree::Symbol as TreeSymbol;
pub use tree::SymbolKind;
pub use tree::TreeError;
pub use tree::TreeInput;
pub use tree::TreeOutput;
pub use tree::TreeResult;
pub use tree::TreeTool;

/// Tool trait for internal AGCodex tools
#[async_trait::async_trait]
pub trait InternalTool {
    type Input;
    type Output;
    type Error;

    /// Execute the tool with the given input
    async fn execute(&self, input: Self::Input) -> Result<Self::Output, Self::Error>;

    /// Get tool metadata
    fn metadata(&self) -> ToolMetadata;
}

/// Metadata about a tool
#[derive(Debug, Clone)]
pub struct ToolMetadata {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
}

/// Common result type for all internal tools
pub type ToolResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

// Re-export registry types for unified tool access
pub use registry::ToolCategory;
pub use registry::ToolError;
pub use registry::ToolInfo;
pub use registry::ToolOutput;
pub use registry::ToolRegistry;

/// Create the default tool registry with all tools registered
///
/// This provides a simple, unified interface to all AGCodex tools.
///
/// # Example
/// ```
/// let registry = create_default_registry();
/// let tools = registry.list_tools();
/// println!("Available tools: {:?}", tools);
/// ```
pub fn create_default_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    // Register search tools
    registry.register(ToolInfo {
        name: "search",
        description: "Search for code patterns and symbols",
        category: ToolCategory::Search,
        example: r#"{"query": "function main", "path": "src/"}"#,
        execute: adapters::adapt_search_tool,
    });

    registry.register(ToolInfo {
        name: "grep",
        description: "Pattern-based text search",
        category: ToolCategory::Search,
        example: r#"{"pattern": "TODO", "path": "."}"#,
        execute: adapters::adapt_grep_tool,
    });

    registry.register(ToolInfo {
        name: "glob",
        description: "Find files by pattern",
        category: ToolCategory::Search,
        example: r#"{"pattern": "*.rs", "path": "src/"}"#,
        execute: adapters::adapt_glob_tool,
    });

    // Register edit tools
    registry.register(ToolInfo {
        name: "edit",
        description: "Simple text replacement in files",
        category: ToolCategory::Edit,
        example: r#"{"file": "main.rs", "old_text": "foo", "new_text": "bar"}"#,
        execute: adapters::adapt_edit_tool,
    });

    registry.register(ToolInfo {
        name: "patch",
        description: "Bulk code transformations",
        category: ToolCategory::Edit,
        example: r#"{"operation": "rename_symbol", "old_name": "foo", "new_name": "bar"}"#,
        execute: adapters::adapt_patch_tool,
    });

    // Register analysis tools
    registry.register(ToolInfo {
        name: "think",
        description: "Step-by-step reasoning about problems",
        category: ToolCategory::Analysis,
        example: r#"{"problem": "How to optimize database queries?"}"#,
        execute: adapters::adapt_think_tool,
    });

    registry.register(ToolInfo {
        name: "plan",
        description: "Create task plans with dependencies",
        category: ToolCategory::Analysis,
        example: r#"{"description": "Refactor authentication", "constraints": ["maintain API"]}"#,
        execute: adapters::adapt_plan_tool,
    });

    registry.register(ToolInfo {
        name: "tree",
        description: "Parse and analyze code structure",
        category: ToolCategory::Analysis,
        example: r#"{"file": "main.rs", "language": "rust"}"#,
        execute: adapters::adapt_tree_tool,
    });

    // Register utility tools
    registry.register(ToolInfo {
        name: "bash",
        description: "Execute safe shell commands",
        category: ToolCategory::Utility,
        example: r#"{"command": "ls -la"}"#,
        execute: adapters::adapt_bash_tool,
    });

    registry
}
