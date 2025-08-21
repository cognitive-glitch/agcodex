//! Internal agent tools for AGCodex
//!
//! This module provides sophisticated tools for task planning, code analysis,
//! and automated workflow management. These tools are designed to work seamlessly
//! with the subagent orchestration system.

pub mod glob;
pub mod index;
pub mod integration_example;
pub mod output;
pub mod patch;
pub mod plan;
pub mod tree;

#[cfg(test)]
mod integration_test_glob;

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
pub use patch::ApiChange;
pub use patch::AppliedTransformation;
pub use patch::AstSummary;
pub use patch::BeforeAfterComparison;
pub use patch::BehavioralChange;
pub use patch::ChangeType;
pub use patch::CodeChange;
pub use patch::DependencyChange;
pub use patch::Parameter;
pub use patch::PatchError;
pub use patch::PatchInput;
pub use patch::PatchOptions;
pub use patch::PatchOutput;
pub use patch::PatchResult;
pub use patch::PatchTool;
pub use patch::PerformanceImpact;
pub use patch::RiskLevel;
pub use patch::SemanticImpact;
pub use patch::SemanticTransformation;
pub use patch::StructuralDifference;
pub use patch::TransformationCondition;
pub use patch::TransformationType;
pub use patch::ValidationResult;
pub use plan::AgentType;
pub use plan::DependencyGraph;
pub use plan::MetaTask;
pub use plan::MetaTaskPlanner;
pub use plan::PlanContext;
pub use plan::PlanError;
pub use plan::PlanExecutionPlan;
pub use plan::PlanExecutionStep;
pub use plan::PlanIntelligenceLevel;
pub use plan::PlanResult;
pub use plan::PlanTool;
pub use plan::SubTask;
pub use plan::SubTaskPlanner;
pub use plan::TaskGroup;
pub use plan::TaskPriority;
pub use plan::TaskStatus;
pub use plan::ToolOutput;
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
