//! Context-aware output structures for AGCodex tools
//!
//! This module provides unified, rich output structures that give agents and LLMs
//! comprehensive context about tool operations. All tools should use these common
//! structures to ensure consistent, analyzable results.
//!
//! ## Design Principles
//! - **Context-first**: Every operation includes before/after context
//! - **Location-aware**: Precise file:line:column information for all changes
//! - **Semantic understanding**: Impact analysis beyond text-level changes
//! - **LLM-optimized**: Structured data with human-readable summaries
//! - **Serializable**: Full serde support for persistence and transmission

// use ast::AstNode; // unused
use ast::Language;
pub use ast::SourceLocation;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;
use std::time::Duration;
use std::time::SystemTime;
use uuid::Uuid;

/// Primary comprehensive output structure for all AGCodex tools
///
/// This provides a unified, context-rich structure that enhances
/// the simpler tool-specific output implementations with comprehensive
/// information about tool operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveToolOutput<T> {
    /// The primary result of the tool operation
    pub result: T,

    /// Rich contextual information about the operation
    pub context: OperationContext,

    /// All changes made during the operation
    pub changes: Vec<Change>,

    /// Comprehensive metadata about the operation
    pub metadata: OperationMetadata,

    /// Human-readable summary optimized for LLM consumption
    pub summary: String,

    /// Performance and execution metrics
    pub performance: PerformanceMetrics,

    /// Warnings, errors, or other diagnostic information
    pub diagnostics: Vec<Diagnostic>,
}

/// Rich contextual information about a tool operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationContext {
    /// State before the operation
    pub before: ContextSnapshot,

    /// State after the operation (if applicable)
    pub after: Option<ContextSnapshot>,

    /// Surrounding context for understanding the change
    pub surrounding: Vec<ContextLine>,

    /// Primary location where the operation occurred
    pub location: SourceLocation,

    /// Scope of the operation (file, function, block, etc.)
    pub scope: OperationScope,

    /// Language-specific context
    pub language_context: Option<LanguageContext>,

    /// Project-level context
    pub project_context: Option<ProjectContext>,
}

/// Snapshot of state at a specific point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSnapshot {
    /// Content at the time of snapshot
    pub content: String,

    /// Timestamp when the snapshot was taken
    pub timestamp: SystemTime,

    /// Hash or checksum of the content for integrity
    pub content_hash: String,

    /// AST representation if available
    pub ast_summary: Option<ComprehensiveAstSummary>,

    /// Symbols present at this point
    pub symbols: Vec<ComprehensiveSymbol>,
}

/// A line of context with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextLine {
    /// Line number (1-based)
    pub line_number: usize,

    /// Content of the line
    pub content: String,

    /// Type of context this line represents
    pub line_type: ContextLineType,

    /// Indentation level for formatting preservation
    pub indentation: usize,

    /// Whether this line was modified
    pub modified: bool,
}

/// Types of context lines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextLineType {
    /// Line before the operation
    Before,
    /// Line after the operation
    After,
    /// Line that was changed
    Changed,
    /// Line added by the operation
    Added,
    /// Line removed by the operation
    Removed,
    /// Separator or structural element
    Separator,
}

/// Represents a single change made during an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    /// Unique identifier for this change
    pub id: Uuid,

    /// Type and nature of the change
    pub kind: ChangeKind,

    /// Original content (if any)
    pub old: Option<String>,

    /// New content (if any)
    pub new: Option<String>,

    /// Line range affected by the change
    pub line_range: Range<usize>,

    /// Character range within the file
    pub char_range: Range<usize>,

    /// Location where the change occurred
    pub location: SourceLocation,

    /// Semantic impact of this change
    pub semantic_impact: ComprehensiveSemanticImpact,

    /// Symbols affected by this change
    pub affected_symbols: Vec<String>,

    /// Confidence level (0.0 to 1.0)
    pub confidence: f32,

    /// Human-readable description
    pub description: String,
}

/// Types of changes with rich semantic information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeKind {
    /// Content was added
    Added {
        reason: String,
        insertion_point: SourceLocation,
    },

    /// Content was modified
    Modified {
        why: String,
        modification_type: ModificationType,
    },

    /// Content was deleted
    Deleted {
        justification: String,
        preservation_note: Option<String>,
    },

    /// Code was refactored (structural change preserving behavior)
    Refactored {
        from_pattern: String,
        to_pattern: String,
        refactor_type: RefactorType,
    },

    /// Content was moved from one location to another
    Moved {
        from: SourceLocation,
        to: SourceLocation,
        move_reason: String,
    },

    /// Symbol was renamed
    Renamed {
        old_name: String,
        new_name: String,
        symbol_type: String,
    },

    /// Import or dependency was added/removed/modified
    ImportChanged {
        import_type: ImportChangeType,
        module_name: String,
        impact: String,
    },
}

/// Types of modifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModificationType {
    /// Content replacement
    Replacement,
    /// Parameter change
    ParameterChange,
    /// Type change
    TypeChange,
    /// Logic enhancement
    LogicEnhancement,
    /// Performance optimization
    Optimization,
    /// Bug fix
    BugFix,
    /// Formatting change
    Formatting,
}

/// Types of refactoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RefactorType {
    /// Extract method/function
    ExtractMethod,
    /// Inline method/function
    InlineMethod,
    /// Move method/function
    MoveMethod,
    /// Rename symbol
    RenameSymbol,
    /// Extract variable
    ExtractVariable,
    /// Extract constant
    ExtractConstant,
    /// Change method signature
    ChangeSignature,
    /// Convert to/from lambda
    LambdaConversion,
}

/// Import/dependency changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImportChangeType {
    Added,
    Removed,
    Modified,
    Reorganized,
}

/// Comprehensive semantic impact analysis of changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveSemanticImpact {
    /// Level of impact
    pub level: ImpactLevel,

    /// Scope of the impact
    pub scope: ImpactScope,

    /// Breaking change analysis
    pub breaking_changes: Vec<BreakingChange>,

    /// API compatibility assessment
    pub api_compatibility: ApiCompatibility,

    /// Performance implications
    pub performance_impact: ComprehensivePerformanceImpact,

    /// Security implications
    pub security_impact: SecurityImpact,

    /// Test impact analysis
    pub test_impact: TestImpact,
}

/// Levels of impact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactLevel {
    /// No significant impact
    None,
    /// Impact contained to immediate context
    Local,
    /// Impact affects module/file interface
    Interface,
    /// Impact affects overall system architecture
    Architectural,
    /// Impact requires careful review
    Critical,
}

/// Scope of impact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactScope {
    /// Single line/statement
    Statement,
    /// Single function/method
    Function,
    /// Single class/struct
    Class,
    /// Single file/module
    Module,
    /// Multiple files
    Package,
    /// Entire project
    Project,
    /// External dependencies
    Dependencies,
}

/// Breaking change analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakingChange {
    /// Description of the breaking change
    pub description: String,

    /// Affected API elements
    pub affected_apis: Vec<String>,

    /// Migration strategy
    pub migration_strategy: Option<String>,

    /// Severity of the break
    pub severity: BreakingSeverity,
}

/// Severity of breaking changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BreakingSeverity {
    /// Minor breaking change
    Minor,
    /// Major breaking change requiring updates
    Major,
    /// Critical breaking change requiring significant refactoring
    Critical,
}

/// API compatibility assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCompatibility {
    /// Whether the change is backward compatible
    pub backward_compatible: bool,

    /// Version compatibility level
    pub version_impact: VersionImpact,

    /// Deprecated features affected
    pub deprecated_usage: Vec<String>,

    /// New features introduced
    pub new_features: Vec<String>,
}

/// Version impact levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VersionImpact {
    /// Patch version (bug fixes)
    Patch,
    /// Minor version (new features, backward compatible)
    Minor,
    /// Major version (breaking changes)
    Major,
}

/// Comprehensive performance impact analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensivePerformanceImpact {
    /// Expected performance change
    pub expected_change: PerformanceChange,

    /// Complexity analysis
    pub complexity_change: Option<ComplexityChange>,

    /// Memory usage impact
    pub memory_impact: MemoryImpact,

    /// CPU usage impact
    pub cpu_impact: String,

    /// I/O impact
    pub io_impact: String,
}

/// Expected performance changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceChange {
    /// Significant improvement
    Improvement(String),
    /// No significant change
    Neutral,
    /// Performance degradation
    Degradation(String),
    /// Unknown impact
    Unknown,
}

/// Complexity analysis changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityChange {
    /// Time complexity before -> after
    pub time_complexity: (String, String),

    /// Space complexity before -> after
    pub space_complexity: (String, String),

    /// Cyclomatic complexity change
    pub cyclomatic_complexity_delta: i32,
}

/// Memory usage impact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryImpact {
    /// Estimated memory change in bytes
    pub estimated_bytes_delta: Option<i64>,

    /// Allocation pattern changes
    pub allocation_changes: Vec<String>,

    /// Memory leak risks
    pub leak_risks: Vec<String>,
}

/// Security impact analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityImpact {
    /// Security level assessment
    pub level: SecurityLevel,

    /// Potential vulnerabilities introduced
    pub vulnerabilities: Vec<SecurityVulnerability>,

    /// Security improvements
    pub improvements: Vec<String>,

    /// Required security review
    pub requires_review: bool,
}

/// Security impact levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityLevel {
    /// No security impact
    None,
    /// Low security impact
    Low,
    /// Medium security impact
    Medium,
    /// High security impact requiring review
    High,
    /// Critical security impact requiring immediate review
    Critical,
}

/// Security vulnerability description
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityVulnerability {
    /// Vulnerability type (e.g., "SQL Injection", "XSS")
    pub vulnerability_type: String,

    /// Description of the vulnerability
    pub description: String,

    /// Severity level
    pub severity: SecurityLevel,

    /// Mitigation strategies
    pub mitigation: Vec<String>,
}

/// Test impact analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestImpact {
    /// Tests that may be affected
    pub affected_tests: Vec<String>,

    /// New tests required
    pub required_tests: Vec<String>,

    /// Test coverage impact
    pub coverage_impact: CoverageImpact,

    /// Test categories affected
    pub test_categories: Vec<TestCategory>,
}

/// Test coverage impact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageImpact {
    /// Expected coverage change (percentage points)
    pub coverage_delta: Option<f32>,

    /// New uncovered lines
    pub uncovered_lines: Vec<usize>,

    /// Coverage recommendations
    pub recommendations: Vec<String>,
}

/// Categories of tests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestCategory {
    Unit,
    Integration,
    EndToEnd,
    Performance,
    Security,
    Regression,
}

/// Scope of an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationScope {
    /// Type of scope
    pub scope_type: ScopeType,

    /// Name or identifier of the scope
    pub name: String,

    /// Hierarchical path (e.g., "module::class::function")
    pub path: Vec<String>,

    /// File path where the scope is defined
    pub file_path: PathBuf,

    /// Line range of the scope
    pub line_range: Range<usize>,
}

/// Types of operation scopes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScopeType {
    /// Global scope
    Global,
    /// File/module scope
    File,
    /// Namespace scope
    Namespace,
    /// Class/struct scope
    Class,
    /// Function/method scope
    Function,
    /// Block scope
    Block,
    /// Expression scope
    Expression,
}

/// Language-specific context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageContext {
    /// Programming language
    pub language: Language,

    /// Language version or standard
    pub version: Option<String>,

    /// Language-specific features used
    pub features: Vec<String>,

    /// Framework or library context
    pub frameworks: Vec<String>,

    /// Compilation or runtime context
    pub runtime_context: Vec<String>,
}

/// Project-level context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    /// Project name
    pub name: Option<String>,

    /// Project type (library, application, etc.)
    pub project_type: Option<String>,

    /// Build system (Cargo, npm, Maven, etc.)
    pub build_system: Option<String>,

    /// Dependencies affected
    pub dependencies: Vec<DependencyInfo>,

    /// Configuration files affected
    pub config_files: Vec<PathBuf>,
}

/// Dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    /// Dependency name
    pub name: String,

    /// Version constraint
    pub version: Option<String>,

    /// Type of dependency (runtime, dev, build, etc.)
    pub dependency_type: String,

    /// Whether this is a new, modified, or removed dependency
    pub change_type: String,
}

/// Comprehensive operation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationMetadata {
    /// Tool that performed the operation
    pub tool: String,

    /// Specific operation performed
    pub operation: String,

    /// Operation ID for tracking
    pub operation_id: Uuid,

    /// Timestamp when operation started
    pub started_at: SystemTime,

    /// Timestamp when operation completed
    pub completed_at: SystemTime,

    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,

    /// Operation parameters
    pub parameters: HashMap<String, String>,

    /// Tool version
    pub tool_version: String,

    /// User or agent that initiated the operation
    pub initiated_by: Option<String>,

    /// Session or context ID
    pub session_id: Option<Uuid>,
}

/// Performance metrics for operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Total execution time
    pub execution_time: Duration,

    /// Time breakdown by operation phase
    pub phase_times: HashMap<String, Duration>,

    /// Memory usage statistics
    pub memory_usage: MemoryUsage,

    /// CPU usage statistics
    pub cpu_usage: CpuUsage,

    /// I/O statistics
    pub io_stats: IoStats,

    /// Caching statistics
    pub cache_stats: CacheStats,
}

/// Memory usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    /// Peak memory usage in bytes
    pub peak_bytes: u64,

    /// Average memory usage in bytes
    pub average_bytes: u64,

    /// Number of allocations
    pub allocations: u64,

    /// Number of deallocations
    pub deallocations: u64,

    /// Memory efficiency score (0.0 to 1.0)
    pub efficiency_score: f32,
}

/// CPU usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuUsage {
    /// CPU time used
    pub cpu_time: Duration,

    /// CPU utilization percentage
    pub utilization_percent: f32,

    /// Number of context switches
    pub context_switches: u64,
}

/// I/O statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoStats {
    /// Bytes read from storage
    pub bytes_read: u64,

    /// Bytes written to storage
    pub bytes_written: u64,

    /// Number of read operations
    pub read_ops: u64,

    /// Number of write operations
    pub write_ops: u64,

    /// I/O wait time
    pub io_wait_time: Duration,
}

/// Caching statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Cache hit rate (0.0 to 1.0)
    pub hit_rate: f32,

    /// Number of cache hits
    pub hits: u64,

    /// Number of cache misses
    pub misses: u64,

    /// Cache size in entries
    pub cache_size: u64,

    /// Cache efficiency score (0.0 to 1.0)
    pub efficiency_score: f32,
}

/// Diagnostic information (warnings, errors, info)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Diagnostic level
    pub level: DiagnosticLevel,

    /// Diagnostic message
    pub message: String,

    /// Location where the diagnostic applies
    pub location: Option<SourceLocation>,

    /// Error code or identifier
    pub code: Option<String>,

    /// Suggestions for resolution
    pub suggestions: Vec<String>,

    /// Related diagnostics
    pub related: Vec<Uuid>,
}

/// Levels of diagnostics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiagnosticLevel {
    /// Informational message
    Info,
    /// Warning that should be addressed
    Warning,
    /// Error that must be fixed
    Error,
    /// Hint or suggestion
    Hint,
}

/// Comprehensive AST summary for structural understanding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveAstSummary {
    /// Root node type
    pub root_type: String,

    /// Number of nodes in the AST
    pub node_count: usize,

    /// Maximum depth of the AST
    pub max_depth: usize,

    /// Top-level symbols
    pub symbols: Vec<ComprehensiveSymbol>,

    /// Complexity metrics
    pub complexity: ComplexityMetrics,

    /// AST hash for comparison
    pub ast_hash: String,
}

/// Comprehensive symbol information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveSymbol {
    /// Symbol name
    pub name: String,

    /// Symbol type (function, class, variable, etc.)
    pub symbol_type: SymbolType,

    /// Location where the symbol is defined
    pub location: SourceLocation,

    /// Visibility (public, private, etc.)
    pub visibility: Visibility,

    /// Symbol signature (for functions, methods)
    pub signature: Option<String>,

    /// Documentation or comments
    pub documentation: Option<String>,
}

/// Types of symbols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolType {
    Function,
    Method,
    Class,
    Struct,
    Interface,
    Trait,
    Variable,
    Constant,
    Type,
    Macro,
    Module,
    Namespace,
}

/// Symbol visibility levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
    PackagePrivate,
}

/// Code complexity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    /// Cyclomatic complexity
    pub cyclomatic: u32,

    /// Cognitive complexity
    pub cognitive: u32,

    /// Halstead complexity metrics
    pub halstead: Option<HalsteadMetrics>,

    /// Lines of code
    pub lines_of_code: u32,

    /// Number of functions/methods
    pub function_count: u32,

    /// Nesting depth
    pub max_nesting_depth: u32,
}

/// Halstead complexity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HalsteadMetrics {
    /// Number of distinct operators
    pub distinct_operators: u32,

    /// Number of distinct operands
    pub distinct_operands: u32,

    /// Total number of operators
    pub total_operators: u32,

    /// Total number of operands
    pub total_operands: u32,

    /// Program vocabulary
    pub vocabulary: u32,

    /// Program length
    pub length: u32,

    /// Calculated volume
    pub volume: f64,

    /// Difficulty level
    pub difficulty: f64,

    /// Effort required
    pub effort: f64,
}

// Extension traits for easier usage

/// Extension trait for Result<T, E> to easily convert to ComprehensiveToolOutput
pub trait ResultExt<T, E> {
    /// Convert a Result to a ComprehensiveToolOutput
    fn to_tool_output(
        self,
        tool: &'static str,
        operation: String,
        location: SourceLocation,
    ) -> ComprehensiveToolOutput<Option<T>>
    where
        E: std::fmt::Display;
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    fn to_tool_output(
        self,
        tool: &'static str,
        operation: String,
        location: SourceLocation,
    ) -> ComprehensiveToolOutput<Option<T>>
    where
        E: std::fmt::Display,
    {
        match self {
            Ok(value) => OutputBuilder::new(Some(value), tool, operation, location).build(),
            Err(error) => OutputBuilder::new(None, tool, operation, location)
                .error(error.to_string())
                .confidence(0.0)
                .build(),
        }
    }
}

// Helper methods for building outputs
impl<T> ComprehensiveToolOutput<T> {
    /// Create a new ComprehensiveToolOutput with minimal required fields
    pub fn new(result: T, tool: &str, operation: String, location: SourceLocation) -> Self {
        let operation_id = Uuid::new_v4();
        let now = SystemTime::now();

        Self {
            result,
            context: OperationContext::minimal(location),
            changes: Vec::new(),
            metadata: OperationMetadata {
                tool: tool.to_string(),
                operation,
                operation_id,
                started_at: now,
                completed_at: now,
                confidence: 1.0,
                parameters: HashMap::new(),
                tool_version: "0.1.0".to_string(),
                initiated_by: None,
                session_id: None,
            },
            summary: "Operation completed successfully".to_string(),
            performance: PerformanceMetrics::default(),
            diagnostics: Vec::new(),
        }
    }

    /// Add a change to the output
    pub fn add_change(mut self, change: Change) -> Self {
        self.changes.push(change);
        self
    }

    /// Add multiple changes to the output
    pub fn add_changes(mut self, changes: Vec<Change>) -> Self {
        self.changes.extend(changes);
        self
    }

    /// Add a diagnostic message
    pub fn add_diagnostic(mut self, diagnostic: Diagnostic) -> Self {
        self.diagnostics.push(diagnostic);
        self
    }

    /// Set the summary
    pub fn with_summary(mut self, summary: String) -> Self {
        self.summary = summary;
        self
    }

    /// Set the confidence level
    pub const fn with_confidence(mut self, confidence: f32) -> Self {
        self.metadata.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Check if the operation had any errors
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| matches!(d.level, DiagnosticLevel::Error))
    }

    /// Check if the operation had any warnings
    pub fn has_warnings(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| matches!(d.level, DiagnosticLevel::Warning))
    }

    /// Get the highest impact level of all changes
    pub fn max_impact_level(&self) -> ImpactLevel {
        self.changes
            .iter()
            .map(|c| &c.semantic_impact.level)
            .max_by_key(|level| match level {
                ImpactLevel::None => 0,
                ImpactLevel::Local => 1,
                ImpactLevel::Interface => 2,
                ImpactLevel::Architectural => 3,
                ImpactLevel::Critical => 4,
            })
            .cloned()
            .unwrap_or(ImpactLevel::None)
    }
}

impl OperationContext {
    /// Create minimal operation context
    pub fn minimal(location: SourceLocation) -> Self {
        Self {
            before: ContextSnapshot::empty(),
            after: None,
            surrounding: Vec::new(),
            location,
            scope: OperationScope::minimal(),
            language_context: None,
            project_context: None,
        }
    }
}

impl ContextSnapshot {
    /// Create an empty context snapshot
    pub fn empty() -> Self {
        Self {
            content: String::new(),
            timestamp: SystemTime::now(),
            content_hash: "empty".to_string(),
            ast_summary: None,
            symbols: Vec::new(),
        }
    }
}

impl OperationScope {
    /// Create minimal operation scope
    pub fn minimal() -> Self {
        Self {
            scope_type: ScopeType::Global,
            name: "global".to_string(),
            path: vec!["global".to_string()],
            file_path: PathBuf::new(),
            line_range: 0..0,
        }
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            execution_time: Duration::from_millis(0),
            phase_times: HashMap::new(),
            memory_usage: MemoryUsage::default(),
            cpu_usage: CpuUsage::default(),
            io_stats: IoStats::default(),
            cache_stats: CacheStats::default(),
        }
    }
}

impl Default for MemoryUsage {
    fn default() -> Self {
        Self {
            peak_bytes: 0,
            average_bytes: 0,
            allocations: 0,
            deallocations: 0,
            efficiency_score: 1.0,
        }
    }
}

impl Default for CpuUsage {
    fn default() -> Self {
        Self {
            cpu_time: Duration::from_millis(0),
            utilization_percent: 0.0,
            context_switches: 0,
        }
    }
}

impl Default for IoStats {
    fn default() -> Self {
        Self {
            bytes_read: 0,
            bytes_written: 0,
            read_ops: 0,
            write_ops: 0,
            io_wait_time: Duration::from_millis(0),
        }
    }
}

impl Default for CacheStats {
    fn default() -> Self {
        Self {
            hit_rate: 0.0,
            hits: 0,
            misses: 0,
            cache_size: 0,
            efficiency_score: 0.0,
        }
    }
}

impl Change {
    /// Create a simple addition change
    pub fn addition(location: SourceLocation, content: String, reason: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind: ChangeKind::Added {
                reason,
                insertion_point: location.clone(),
            },
            old: None,
            new: Some(content),
            line_range: location.start_line..location.start_line + 1,
            char_range: location.start_column..location.start_column + 1,
            location,
            semantic_impact: ComprehensiveSemanticImpact::minimal(),
            affected_symbols: Vec::new(),
            confidence: 1.0,
            description: "Content added".to_string(),
        }
    }

    /// Create a simple modification change
    pub fn modification(
        location: SourceLocation,
        old_content: String,
        new_content: String,
        why: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind: ChangeKind::Modified {
                why,
                modification_type: ModificationType::Replacement,
            },
            old: Some(old_content),
            new: Some(new_content),
            line_range: location.start_line..location.start_line + 1,
            char_range: location.start_column..location.start_column + 1,
            location,
            semantic_impact: ComprehensiveSemanticImpact::minimal(),
            affected_symbols: Vec::new(),
            confidence: 1.0,
            description: "Content modified".to_string(),
        }
    }

    /// Create a simple deletion change
    pub fn deletion(location: SourceLocation, content: String, justification: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind: ChangeKind::Deleted {
                justification,
                preservation_note: None,
            },
            old: Some(content),
            new: None,
            line_range: location.start_line..location.start_line + 1,
            char_range: location.start_column..location.start_column + 1,
            location,
            semantic_impact: ComprehensiveSemanticImpact::minimal(),
            affected_symbols: Vec::new(),
            confidence: 1.0,
            description: "Content deleted".to_string(),
        }
    }
}

impl ComprehensiveSemanticImpact {
    /// Create minimal semantic impact
    pub fn minimal() -> Self {
        Self {
            level: ImpactLevel::Local,
            scope: ImpactScope::Statement,
            breaking_changes: Vec::new(),
            api_compatibility: ApiCompatibility::compatible(),
            performance_impact: ComprehensivePerformanceImpact::neutral(),
            security_impact: SecurityImpact::none(),
            test_impact: TestImpact::minimal(),
        }
    }
}

impl ApiCompatibility {
    /// Create compatible API compatibility
    pub const fn compatible() -> Self {
        Self {
            backward_compatible: true,
            version_impact: VersionImpact::Patch,
            deprecated_usage: Vec::new(),
            new_features: Vec::new(),
        }
    }
}

impl ComprehensivePerformanceImpact {
    /// Create neutral performance impact
    pub fn neutral() -> Self {
        Self {
            expected_change: PerformanceChange::Neutral,
            complexity_change: None,
            memory_impact: MemoryImpact::neutral(),
            cpu_impact: "No significant impact".to_string(),
            io_impact: "No significant impact".to_string(),
        }
    }
}

impl MemoryImpact {
    /// Create neutral memory impact
    pub const fn neutral() -> Self {
        Self {
            estimated_bytes_delta: None,
            allocation_changes: Vec::new(),
            leak_risks: Vec::new(),
        }
    }
}

impl SecurityImpact {
    /// Create no security impact
    pub const fn none() -> Self {
        Self {
            level: SecurityLevel::None,
            vulnerabilities: Vec::new(),
            improvements: Vec::new(),
            requires_review: false,
        }
    }
}

impl TestImpact {
    /// Create minimal test impact
    pub const fn minimal() -> Self {
        Self {
            affected_tests: Vec::new(),
            required_tests: Vec::new(),
            coverage_impact: CoverageImpact::neutral(),
            test_categories: Vec::new(),
        }
    }
}

impl CoverageImpact {
    /// Create neutral coverage impact
    pub const fn neutral() -> Self {
        Self {
            coverage_delta: None,
            uncovered_lines: Vec::new(),
            recommendations: Vec::new(),
        }
    }
}

impl Diagnostic {
    /// Create an info diagnostic
    pub const fn info(message: String) -> Self {
        Self {
            level: DiagnosticLevel::Info,
            message,
            location: None,
            code: None,
            suggestions: Vec::new(),
            related: Vec::new(),
        }
    }

    /// Create a warning diagnostic
    pub const fn warning(message: String) -> Self {
        Self {
            level: DiagnosticLevel::Warning,
            message,
            location: None,
            code: None,
            suggestions: Vec::new(),
            related: Vec::new(),
        }
    }

    /// Create an error diagnostic
    pub const fn error(message: String) -> Self {
        Self {
            level: DiagnosticLevel::Error,
            message,
            location: None,
            code: None,
            suggestions: Vec::new(),
            related: Vec::new(),
        }
    }

    /// Add a suggestion to the diagnostic
    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    /// Set the location for the diagnostic
    pub fn at_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }

    /// Set the error code
    pub fn with_code(mut self, code: String) -> Self {
        self.code = Some(code);
        self
    }
}

// Builder patterns for convenient output construction

/// Builder for constructing ComprehensiveToolOutput instances
pub struct OutputBuilder<T> {
    result: T,
    tool: String,
    operation: String,
    location: SourceLocation,
    changes: Vec<Change>,
    diagnostics: Vec<Diagnostic>,
    confidence: f32,
    summary: Option<String>,
    context: Option<OperationContext>,
    performance: Option<PerformanceMetrics>,
}

impl<T> OutputBuilder<T> {
    /// Create a new output builder
    pub fn new(result: T, tool: &str, operation: String, location: SourceLocation) -> Self {
        Self {
            result,
            tool: tool.to_string(),
            operation,
            location,
            changes: Vec::new(),
            diagnostics: Vec::new(),
            confidence: 1.0,
            summary: None,
            context: None,
            performance: None,
        }
    }

    /// Add a change to the output
    pub fn change(mut self, change: Change) -> Self {
        self.changes.push(change);
        self
    }

    /// Add multiple changes
    pub fn changes(mut self, changes: Vec<Change>) -> Self {
        self.changes.extend(changes);
        self
    }

    /// Add a diagnostic
    pub fn diagnostic(mut self, diagnostic: Diagnostic) -> Self {
        self.diagnostics.push(diagnostic);
        self
    }

    /// Add an info diagnostic
    pub fn info(mut self, message: String) -> Self {
        self.diagnostics.push(Diagnostic::info(message));
        self
    }

    /// Add a warning diagnostic
    pub fn warning(mut self, message: String) -> Self {
        self.diagnostics.push(Diagnostic::warning(message));
        self
    }

    /// Add an error diagnostic
    pub fn error(mut self, message: String) -> Self {
        self.diagnostics.push(Diagnostic::error(message));
        self
    }

    /// Set the confidence level
    pub const fn confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Set the summary
    pub fn summary(mut self, summary: String) -> Self {
        self.summary = Some(summary);
        self
    }

    /// Set the operation context
    pub fn context(mut self, context: OperationContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Set performance metrics
    pub fn performance(mut self, performance: PerformanceMetrics) -> Self {
        self.performance = Some(performance);
        self
    }

    /// Build the final ComprehensiveToolOutput
    pub fn build(self) -> ComprehensiveToolOutput<T> {
        let operation_id = Uuid::new_v4();
        let now = SystemTime::now();

        let summary = self.summary.unwrap_or_else(|| {
            if self
                .diagnostics
                .iter()
                .any(|d| matches!(d.level, DiagnosticLevel::Error))
            {
                "Operation completed with errors".to_string()
            } else if self
                .diagnostics
                .iter()
                .any(|d| matches!(d.level, DiagnosticLevel::Warning))
            {
                "Operation completed with warnings".to_string()
            } else {
                "Operation completed successfully".to_string()
            }
        });

        ComprehensiveToolOutput {
            result: self.result,
            context: self
                .context
                .unwrap_or_else(|| OperationContext::minimal(self.location.clone())),
            changes: self.changes,
            metadata: OperationMetadata {
                tool: self.tool,
                operation: self.operation,
                operation_id,
                started_at: now,
                completed_at: now,
                confidence: self.confidence,
                parameters: HashMap::new(),
                tool_version: "0.1.0".to_string(),
                initiated_by: None,
                session_id: None,
            },
            summary,
            performance: self.performance.unwrap_or_default(),
            diagnostics: self.diagnostics,
        }
    }
}

/// Builder for constructing Change instances
pub struct ChangeBuilder {
    location: SourceLocation,
    old: Option<String>,
    new: Option<String>,
    description: Option<String>,
    confidence: f32,
    affected_symbols: Vec<String>,
    impact: Option<ComprehensiveSemanticImpact>,
}

impl ChangeBuilder {
    /// Create a new change builder
    pub const fn new(location: SourceLocation) -> Self {
        Self {
            location,
            old: None,
            new: None,
            description: None,
            confidence: 1.0,
            affected_symbols: Vec::new(),
            impact: None,
        }
    }

    /// Set the old content
    pub fn old_content(mut self, content: String) -> Self {
        self.old = Some(content);
        self
    }

    /// Set the new content
    pub fn new_content(mut self, content: String) -> Self {
        self.new = Some(content);
        self
    }

    /// Set the description
    pub fn description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set the confidence
    pub const fn confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Add an affected symbol
    pub fn symbol(mut self, symbol: String) -> Self {
        self.affected_symbols.push(symbol);
        self
    }

    /// Set the semantic impact
    pub fn impact(mut self, impact: ComprehensiveSemanticImpact) -> Self {
        self.impact = Some(impact);
        self
    }

    /// Build an addition change
    pub fn addition(self, reason: String) -> Change {
        let description = self
            .description
            .unwrap_or_else(|| "Content added".to_string());
        Change {
            id: Uuid::new_v4(),
            kind: ChangeKind::Added {
                reason,
                insertion_point: self.location.clone(),
            },
            old: self.old,
            new: self.new,
            line_range: self.location.start_line..self.location.start_line + 1,
            char_range: self.location.start_column..self.location.start_column + 1,
            location: self.location,
            semantic_impact: self
                .impact
                .unwrap_or_else(ComprehensiveSemanticImpact::minimal),
            affected_symbols: self.affected_symbols,
            confidence: self.confidence,
            description,
        }
    }

    /// Build a modification change
    pub fn modification(self, why: String, modification_type: ModificationType) -> Change {
        let description = self
            .description
            .unwrap_or_else(|| "Content modified".to_string());
        Change {
            id: Uuid::new_v4(),
            kind: ChangeKind::Modified {
                why,
                modification_type,
            },
            old: self.old,
            new: self.new,
            line_range: self.location.start_line..self.location.start_line + 1,
            char_range: self.location.start_column..self.location.start_column + 1,
            location: self.location,
            semantic_impact: self
                .impact
                .unwrap_or_else(ComprehensiveSemanticImpact::minimal),
            affected_symbols: self.affected_symbols,
            confidence: self.confidence,
            description,
        }
    }

    /// Build a deletion change
    pub fn deletion(self, justification: String) -> Change {
        let description = self
            .description
            .unwrap_or_else(|| "Content deleted".to_string());
        Change {
            id: Uuid::new_v4(),
            kind: ChangeKind::Deleted {
                justification,
                preservation_note: None,
            },
            old: self.old,
            new: self.new,
            line_range: self.location.start_line..self.location.start_line + 1,
            char_range: self.location.start_column..self.location.start_column + 1,
            location: self.location,
            semantic_impact: self
                .impact
                .unwrap_or_else(ComprehensiveSemanticImpact::minimal),
            affected_symbols: self.affected_symbols,
            confidence: self.confidence,
            description,
        }
    }
}

// Convenience functions for common scenarios

/// Create a simple successful output for tools that don't need complex analysis
pub fn simple_success<T>(
    result: T,
    tool: &'static str,
    operation: String,
    summary: String,
) -> ComprehensiveToolOutput<T> {
    let location = SourceLocation::new("unknown", 0, 0, 0, 0, (0, 0));
    OutputBuilder::new(result, tool, operation, location)
        .summary(summary)
        .build()
}

/// Create a simple error output for tools that encounter failures
pub fn simple_error<T>(
    result: T,
    tool: &'static str,
    operation: String,
    error_message: String,
) -> ComprehensiveToolOutput<T> {
    let location = SourceLocation::new("unknown", 0, 0, 0, 0, (0, 0));
    OutputBuilder::new(result, tool, operation, location)
        .error(error_message)
        .confidence(0.0)
        .build()
}

/// Create an output with a single file modification
pub fn single_file_modification<T>(
    result: T,
    tool: &'static str,
    operation: String,
    file_path: &str,
    line: usize,
    column: usize,
    old_content: String,
    new_content: String,
    reason: String,
) -> ComprehensiveToolOutput<T> {
    let location = SourceLocation::new(file_path, line, column, line, column, (0, 0));
    let change = ChangeBuilder::new(location.clone())
        .old_content(old_content)
        .new_content(new_content)
        .modification(reason.clone(), ModificationType::Replacement);

    OutputBuilder::new(result, tool, operation, location)
        .change(change)
        .summary(format!(
            "Modified {} at line {}: {}",
            file_path, line, reason
        ))
        .build()
}

/// Create an output with multiple file changes
pub fn multi_file_changes<T>(
    result: T,
    tool: &'static str,
    operation: String,
    changes: Vec<Change>,
) -> ComprehensiveToolOutput<T> {
    let location = if let Some(first_change) = changes.first() {
        first_change.location.clone()
    } else {
        SourceLocation::new("unknown", 0, 0, 0, 0, (0, 0))
    };

    let summary = if changes.is_empty() {
        "No changes made".to_string()
    } else {
        format!("Made {} changes across files", changes.len())
    };

    OutputBuilder::new(result, tool, operation, location)
        .changes(changes)
        .summary(summary)
        .build()
}

/// Performance timing helper for measuring tool operations
pub struct PerformanceTimer {
    started_at: SystemTime,
    phase_times: HashMap<String, Duration>,
    current_phase: Option<String>,
    current_phase_start: Option<SystemTime>,
}

impl PerformanceTimer {
    /// Create a new performance timer
    pub fn new() -> Self {
        Self {
            started_at: SystemTime::now(),
            phase_times: HashMap::new(),
            current_phase: None,
            current_phase_start: None,
        }
    }

    /// Start timing a specific phase
    pub fn start_phase(&mut self, phase_name: String) {
        // Start new phase
        self.current_phase = Some(phase_name);
        self.current_phase_start = Some(SystemTime::now());
    }

    /// End the current phase
    pub fn end_current_phase(&mut self) {
        if let (Some(phase_name), Some(start_time)) =
            (self.current_phase.take(), self.current_phase_start.take())
            && let Ok(duration) = start_time.elapsed()
        {
            self.phase_times.insert(phase_name, duration);
        }
    }

    /// Get the elapsed time since the timer was created
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed().unwrap_or_default()
    }

    /// Get the performance metrics
    pub fn metrics(mut self) -> PerformanceMetrics {
        self.end_current_phase();

        let execution_time = self.started_at.elapsed().unwrap_or_default();

        PerformanceMetrics {
            execution_time,
            phase_times: self.phase_times,
            memory_usage: MemoryUsage::default(),
            cpu_usage: CpuUsage::default(),
            io_stats: IoStats::default(),
            cache_stats: CacheStats::default(),
        }
    }
}

impl Default for PerformanceTimer {
    fn default() -> Self {
        Self::new()
    }
}
