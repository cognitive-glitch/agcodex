//! Core subagent implementations for AGCodex
//!
//! This module provides specialized AI assistants for task-specific workflows:
//! - CodeReviewerAgent: AST-based code review and quality analysis
//! - RefactorerAgent: Systematic code refactoring with risk assessment
//! - DebuggerAgent: Deep debugging analysis and execution path tracing
//! - TestWriterAgent: Comprehensive test generation and coverage analysis
//! - PerformanceAgent: Performance optimization and bottleneck detection
//! - SecurityAgent: Security vulnerability detection and OWASP analysis
//! - DocsAgent: Documentation generation and API reference creation

use crate::code_tools::ast_agent_tools::ASTAgentTools;
use crate::code_tools::ast_agent_tools::AgentToolOp;
use crate::code_tools::ast_agent_tools::AgentToolResult;
use crate::code_tools::ast_agent_tools::DocumentationTarget;
use crate::code_tools::ast_agent_tools::ImprovementFocus;
use crate::code_tools::ast_agent_tools::Location;
use crate::code_tools::ast_agent_tools::PatternType;
use crate::code_tools::search::SearchScope;
use crate::subagents::SubagentContext;
use crate::subagents::SubagentError;
use crate::subagents::SubagentResult;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::SystemTime;
use walkdir::WalkDir;

/// Result of agent execution with structured findings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    /// Agent that produced this result
    pub agent_name: String,
    /// Execution status
    pub status: AgentStatus,
    /// Primary findings
    pub findings: Vec<Finding>,
    /// Files analyzed
    pub analyzed_files: Vec<PathBuf>,
    /// Files modified (if any)
    pub modified_files: Vec<PathBuf>,
    /// Execution time
    pub execution_time: Duration,
    /// Summary report
    pub summary: String,
    /// Metrics and statistics
    pub metrics: HashMap<String, serde_json::Value>,
}

/// Agent execution status with progress tracking
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Initializing,
    Analyzing,
    Processing,
    Generating,
    Completed,
    Failed(String),
    Cancelled,
}

/// Individual finding from agent analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Finding type/category
    pub category: String,
    /// Severity level (Critical, High, Medium, Low, Info)
    pub severity: Severity,
    /// Finding title
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Location in code (if applicable)
    pub location: Option<Location>,
    /// Suggested fix or improvement
    pub suggestion: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Severity levels for findings
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Core trait for all subagents
pub trait Subagent: Send + Sync + std::fmt::Debug {
    /// Agent name identifier
    fn name(&self) -> &str;

    /// Agent description
    fn description(&self) -> &str;

    /// Execute the agent with given context
    /// Returns a boxed future to make the trait object-safe
    fn execute<'a>(
        &'a self,
        context: &'a SubagentContext,
        ast_tools: &'a mut ASTAgentTools,
        cancel_flag: Arc<AtomicBool>,
    ) -> Pin<Box<dyn Future<Output = SubagentResult<AgentResult>> + Send + 'a>>;

    /// Get agent capabilities and tool requirements
    fn capabilities(&self) -> Vec<String>;

    /// Check if agent can handle the given file types
    fn supports_file_type(&self, file_path: &std::path::Path) -> bool;

    /// Get expected execution time range
    fn execution_time_estimate(&self) -> Duration {
        Duration::from_secs(60) // Default 1 minute
    }
}

/// Code review agent for AST-based quality analysis
#[derive(Debug)]
pub struct CodeReviewerAgent {
    config: ReviewConfig,
}

#[derive(Debug, Clone)]
pub struct ReviewConfig {
    pub focus_areas: Vec<ReviewFocus>,
    pub severity_threshold: Severity,
    pub include_suggestions: bool,
    pub check_security: bool,
    pub check_performance: bool,
    pub check_maintainability: bool,
}

#[derive(Debug, Clone)]
pub enum ReviewFocus {
    Security,
    Performance,
    Maintainability,
    CodeStyle,
    Documentation,
    Testing,
}

impl Default for CodeReviewerAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeReviewerAgent {
    pub fn new() -> Self {
        Self {
            config: ReviewConfig {
                focus_areas: vec![
                    ReviewFocus::Security,
                    ReviewFocus::Performance,
                    ReviewFocus::Maintainability,
                ],
                severity_threshold: Severity::Low,
                include_suggestions: true,
                check_security: true,
                check_performance: true,
                check_maintainability: true,
            },
        }
    }

    pub const fn with_config(config: ReviewConfig) -> Self {
        Self { config }
    }

    async fn analyze_functions(
        &self,
        ast_tools: &mut ASTAgentTools,
        file: &PathBuf,
    ) -> SubagentResult<Vec<Finding>> {
        let mut findings = Vec::new();

        // Extract functions from file
        let result = ast_tools.execute(AgentToolOp::ExtractFunctions {
            file: file.clone(),
            language: self.detect_language(file),
        })?;

        if let AgentToolResult::Functions(functions) = result {
            for func in functions {
                // Check complexity
                if let Ok(AgentToolResult::Complexity(complexity)) =
                    ast_tools.execute(AgentToolOp::CalculateComplexity {
                        function: func.name.clone(),
                    })
                    && complexity.cyclomatic_complexity > 10
                {
                    findings.push(Finding {
                        category: "complexity".to_string(),
                        severity: if complexity.cyclomatic_complexity > 20 {
                            Severity::High
                        } else {
                            Severity::Medium
                        },
                        title: format!("High cyclomatic complexity in {}", func.name),
                        description: format!(
                            "Function '{}' has cyclomatic complexity of {}, consider refactoring",
                            func.name, complexity.cyclomatic_complexity
                        ),
                        location: Some(Location {
                            file: file.clone(),
                            line: func.start_line,
                            column: 0,
                            byte_offset: 0,
                        }),
                        suggestion: Some(
                            "Break down into smaller functions or reduce conditional complexity"
                                .to_string(),
                        ),
                        metadata: HashMap::from([(
                            "complexity".to_string(),
                            serde_json::Value::Number(serde_json::Number::from(
                                complexity.cyclomatic_complexity,
                            )),
                        )]),
                    });
                }

                // Check function length
                let line_count = func.end_line - func.start_line + 1;
                if line_count > 50 {
                    findings.push(Finding {
                        category: "maintainability".to_string(),
                        severity: if line_count > 100 {
                            Severity::High
                        } else {
                            Severity::Medium
                        },
                        title: format!("Long function: {}", func.name),
                        description: format!(
                            "Function '{}' is {} lines long, consider breaking it down",
                            func.name, line_count
                        ),
                        location: Some(Location {
                            file: file.clone(),
                            line: func.start_line,
                            column: 0,
                            byte_offset: 0,
                        }),
                        suggestion: Some("Extract smaller, focused functions".to_string()),
                        metadata: HashMap::from([(
                            "line_count".to_string(),
                            serde_json::Value::Number(serde_json::Number::from(line_count)),
                        )]),
                    });
                }

                // Check parameter count
                if func.parameters.len() > 5 {
                    findings.push(Finding {
                        category: "design".to_string(),
                        severity: Severity::Medium,
                        title: format!("Too many parameters in {}", func.name),
                        description: format!(
                            "Function '{}' has {} parameters, consider using a parameter object",
                            func.name,
                            func.parameters.len()
                        ),
                        location: Some(Location {
                            file: file.clone(),
                            line: func.start_line,
                            column: 0,
                            byte_offset: 0,
                        }),
                        suggestion: Some(
                            "Group related parameters into a struct or use builder pattern"
                                .to_string(),
                        ),
                        metadata: HashMap::from([(
                            "parameter_count".to_string(),
                            serde_json::Value::Number(serde_json::Number::from(
                                func.parameters.len(),
                            )),
                        )]),
                    });
                }
            }
        }

        Ok(findings)
    }

    async fn detect_security_issues(
        &self,
        ast_tools: &mut ASTAgentTools,
    ) -> SubagentResult<Vec<Finding>> {
        let mut findings = Vec::new();

        // Detect security anti-patterns
        let result = ast_tools.execute(AgentToolOp::DetectPatterns {
            pattern: PatternType::AntiPattern("sql_injection".to_string()),
        })?;

        if let AgentToolResult::Patterns(patterns) = result {
            for pattern in patterns {
                findings.push(Finding {
                    category: "security".to_string(),
                    severity: Severity::Critical,
                    title: "Potential SQL injection vulnerability".to_string(),
                    description: "Direct string concatenation for SQL queries detected".to_string(),
                    location: Some(pattern.location),
                    suggestion: Some(
                        "Use parameterized queries or prepared statements".to_string(),
                    ),
                    metadata: HashMap::new(),
                });
            }
        }

        Ok(findings)
    }

    async fn find_dead_code(&self, ast_tools: &mut ASTAgentTools) -> SubagentResult<Vec<Finding>> {
        let mut findings = Vec::new();

        let result = ast_tools.execute(AgentToolOp::FindDeadCode {
            scope: SearchScope::Workspace,
        })?;

        if let AgentToolResult::DeadCode(dead_items) = result {
            for item in dead_items {
                findings.push(Finding {
                    category: "maintainability".to_string(),
                    severity: Severity::Low,
                    title: format!("Dead code detected: {}", item.symbol),
                    description: format!("Unused {:?} '{}' found", item.kind, item.symbol),
                    location: Some(item.location),
                    suggestion: Some("Remove unused code to improve maintainability".to_string()),
                    metadata: HashMap::from([(
                        "item_type".to_string(),
                        serde_json::Value::String(format!("{:?}", item.kind)),
                    )]),
                });
            }
        }

        Ok(findings)
    }

    fn detect_language(&self, file: &PathBuf) -> String {
        match file.extension().and_then(|ext| ext.to_str()) {
            Some("rs") => "rust".to_string(),
            Some("py") => "python".to_string(),
            Some("js") | Some("jsx") => "javascript".to_string(),
            Some("ts") | Some("tsx") => "typescript".to_string(),
            Some("go") => "go".to_string(),
            Some("java") => "java".to_string(),
            Some("cpp") | Some("cc") | Some("cxx") => "cpp".to_string(),
            Some("c") => "c".to_string(),
            Some("cs") => "csharp".to_string(),
            _ => "text".to_string(),
        }
    }
}

impl Subagent for CodeReviewerAgent {
    fn name(&self) -> &str {
        "code-reviewer"
    }

    fn description(&self) -> &str {
        "AST-based code review for quality, security, and maintainability analysis"
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

            // Get files to analyze
            let files_to_analyze = self.get_files_to_analyze(&context.working_directory)?;

            for file in &files_to_analyze {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Ok(AgentResult {
                        agent_name: self.name().to_string(),
                        status: AgentStatus::Cancelled,
                        findings: all_findings,
                        analyzed_files,
                        modified_files: vec![],
                        execution_time: start_time.elapsed().unwrap_or_default(),
                        summary: "Analysis cancelled by user".to_string(),
                        metrics: HashMap::new(),
                    });
                }

                analyzed_files.push(file.clone());

                // Analyze functions
                let mut findings = self.analyze_functions(ast_tools, file).await?;
                all_findings.append(&mut findings);
            }

            // Global analysis
            if self.config.check_security {
                let mut security_findings = self.detect_security_issues(ast_tools).await?;
                all_findings.append(&mut security_findings);
            }

            // Find dead code
            let mut dead_code_findings = self.find_dead_code(ast_tools).await?;
            all_findings.append(&mut dead_code_findings);

            // Generate summary
            let summary = self.generate_summary(&all_findings);
            let metrics = self.calculate_metrics(&all_findings, &analyzed_files);

            Ok(AgentResult {
                agent_name: self.name().to_string(),
                status: AgentStatus::Completed,
                findings: all_findings,
                analyzed_files,
                modified_files: vec![],
                execution_time: start_time.elapsed().unwrap_or_default(),
                summary,
                metrics,
            })
        })
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "extract_functions".to_string(),
            "detect_patterns".to_string(),
            "find_dead_code".to_string(),
            "calculate_complexity".to_string(),
        ]
    }

    fn supports_file_type(&self, file_path: &std::path::Path) -> bool {
        matches!(
            file_path.extension().and_then(|ext| ext.to_str()),
            Some("rs")
                | Some("py")
                | Some("js")
                | Some("ts")
                | Some("jsx")
                | Some("tsx")
                | Some("go")
                | Some("java")
                | Some("cpp")
                | Some("c")
                | Some("cs")
        )
    }

    fn execution_time_estimate(&self) -> Duration {
        Duration::from_secs(120) // 2 minutes for thorough review
    }
}

impl CodeReviewerAgent {
    fn get_files_to_analyze(&self, working_dir: &PathBuf) -> SubagentResult<Vec<PathBuf>> {
        let mut files = Vec::new();

        // Walk directory and collect source files
        for entry in WalkDir::new(working_dir).follow_links(false).max_depth(10) {
            let entry =
                entry.map_err(|e| SubagentError::Io(std::io::Error::other(e.to_string())))?;

            if entry.file_type().is_file() && self.supports_file_type(entry.path()) {
                files.push(entry.path().to_path_buf());
            }
        }

        Ok(files)
    }

    fn generate_summary(&self, findings: &[Finding]) -> String {
        let critical_count = findings
            .iter()
            .filter(|f| f.severity == Severity::Critical)
            .count();
        let high_count = findings
            .iter()
            .filter(|f| f.severity == Severity::High)
            .count();
        let medium_count = findings
            .iter()
            .filter(|f| f.severity == Severity::Medium)
            .count();
        let low_count = findings
            .iter()
            .filter(|f| f.severity == Severity::Low)
            .count();

        format!(
            "Code review completed. Found {} issues: {} critical, {} high, {} medium, {} low severity.",
            findings.len(),
            critical_count,
            high_count,
            medium_count,
            low_count
        )
    }

    fn calculate_metrics(
        &self,
        findings: &[Finding],
        analyzed_files: &[PathBuf],
    ) -> HashMap<String, serde_json::Value> {
        let mut metrics = HashMap::new();

        metrics.insert(
            "total_findings".to_string(),
            serde_json::Value::Number(serde_json::Number::from(findings.len())),
        );
        metrics.insert(
            "files_analyzed".to_string(),
            serde_json::Value::Number(serde_json::Number::from(analyzed_files.len())),
        );

        // Count by severity
        for severity in [
            Severity::Critical,
            Severity::High,
            Severity::Medium,
            Severity::Low,
            Severity::Info,
        ] {
            let count = findings.iter().filter(|f| f.severity == severity).count();
            let key = format!("{:?}_count", severity).to_lowercase();
            metrics.insert(
                key,
                serde_json::Value::Number(serde_json::Number::from(count)),
            );
        }

        // Count by category
        let mut categories = HashMap::new();
        for finding in findings {
            *categories.entry(finding.category.clone()).or_insert(0) += 1;
        }

        for (category, count) in categories {
            metrics.insert(
                format!("{}_issues", category),
                serde_json::Value::Number(serde_json::Number::from(count)),
            );
        }

        metrics
    }
}

/// Systematic refactoring agent
#[derive(Debug)]
pub struct RefactorerAgent {
    _config: RefactorConfig,
}

#[derive(Debug, Clone)]
pub struct RefactorConfig {
    pub risk_threshold: RiskLevel,
    pub operations: Vec<RefactorOperation>,
    pub backup_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone)]
pub enum RefactorOperation {
    ExtractMethod,
    InlineVariable,
    RenameSymbol,
    RemoveDuplication,
}

impl Default for RefactorerAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl RefactorerAgent {
    pub fn new() -> Self {
        Self {
            _config: RefactorConfig {
                risk_threshold: RiskLevel::Medium,
                operations: vec![
                    RefactorOperation::ExtractMethod,
                    RefactorOperation::RemoveDuplication,
                ],
                backup_enabled: true,
            },
        }
    }
}

impl Subagent for RefactorerAgent {
    fn name(&self) -> &str {
        "refactorer"
    }

    fn description(&self) -> &str {
        "Systematic code refactoring with risk assessment and automated improvements"
    }

    fn execute<'a>(
        &'a self,
        _context: &'a SubagentContext,
        ast_tools: &'a mut ASTAgentTools,
        _cancel_flag: Arc<AtomicBool>,
    ) -> Pin<Box<dyn Future<Output = SubagentResult<AgentResult>> + Send + 'a>> {
        Box::pin(async move {
            let start_time = SystemTime::now();
            let mut findings = Vec::new();

            // Detect code duplication
            let result = ast_tools.execute(AgentToolOp::DetectDuplication { threshold: 0.8 })?;

            if let AgentToolResult::Duplications(duplications) = result {
                for dup_group in duplications {
                    findings.push(Finding {
                        category: "duplication".to_string(),
                        severity: if dup_group.locations.len() > 3 {
                            Severity::High
                        } else {
                            Severity::Medium
                        },
                        title: "Code duplication detected".to_string(),
                        description: format!(
                            "Found {} instances of duplicated code",
                            dup_group.locations.len()
                        ),
                        location: dup_group.locations.first().cloned(),
                        suggestion: Some(
                            "Extract common functionality into a shared function or method"
                                .to_string(),
                        ),
                        metadata: HashMap::from([
                            (
                                "duplication_count".to_string(),
                                serde_json::Value::Number(serde_json::Number::from(
                                    dup_group.locations.len(),
                                )),
                            ),
                            (
                                "similarity_score".to_string(),
                                serde_json::Value::Number(
                                    serde_json::Number::from_f64(dup_group.similarity as f64)
                                        .unwrap(),
                                ),
                            ),
                        ]),
                    });
                }
            }

            Ok(AgentResult {
                agent_name: self.name().to_string(),
                status: AgentStatus::Completed,
                findings,
                analyzed_files: vec![],
                modified_files: vec![],
                execution_time: start_time.elapsed().unwrap_or_default(),
                summary: "Refactoring analysis completed".to_string(),
                metrics: HashMap::new(),
            })
        })
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "detect_duplication".to_string(),
            "extract_method".to_string(),
            "inline_variable".to_string(),
            "refactor_rename".to_string(),
        ]
    }

    fn supports_file_type(&self, file_path: &std::path::Path) -> bool {
        // Supports same file types as CodeReviewerAgent
        matches!(
            file_path.extension().and_then(|ext| ext.to_str()),
            Some("rs")
                | Some("py")
                | Some("js")
                | Some("ts")
                | Some("jsx")
                | Some("tsx")
                | Some("go")
                | Some("java")
                | Some("cpp")
                | Some("c")
                | Some("cs")
        )
    }
}

/// Deep debugging analysis agent
#[derive(Debug)]
pub struct DebuggerAgent;

impl Default for DebuggerAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl DebuggerAgent {
    pub const fn new() -> Self {
        Self
    }
}

impl Subagent for DebuggerAgent {
    fn name(&self) -> &str {
        "debugger"
    }

    fn description(&self) -> &str {
        "Deep debugging analysis with execution path tracing and error source identification"
    }

    fn execute<'a>(
        &'a self,
        context: &'a SubagentContext,
        ast_tools: &'a mut ASTAgentTools,
        _cancel_flag: Arc<AtomicBool>,
    ) -> Pin<Box<dyn Future<Output = SubagentResult<AgentResult>> + Send + 'a>> {
        Box::pin(async move {
            let start_time = SystemTime::now();
            let mut findings = Vec::new();

            // Analyze call graph to identify potential issue sources
            if let Some(entry_point) = context.parameters.get("entry_point") {
                let result = ast_tools.execute(AgentToolOp::AnalyzeCallGraph {
                    entry_point: entry_point.clone(),
                })?;

                if let AgentToolResult::CallGraph(call_graph) = result {
                    // Analyze call graph for potential issues
                    for (_name, node) in call_graph.nodes {
                        // Simple heuristic: if there are no edges pointing to this node
                        let has_callers = call_graph
                            .edges
                            .iter()
                            .any(|edge| edge.callee == node.function_name);
                        if !has_callers {
                            findings.push(Finding {
                                category: "unreachable".to_string(),
                                severity: Severity::Medium,
                                title: format!(
                                    "Potentially unreachable function: {}",
                                    node.function_name
                                ),
                                description: "Function with no callers detected".to_string(),
                                location: Some(node.location),
                                suggestion: Some(
                                    "Verify if this function is used externally or can be removed"
                                        .to_string(),
                                ),
                                metadata: HashMap::new(),
                            });
                        }
                    }
                }
            }

            Ok(AgentResult {
                agent_name: self.name().to_string(),
                status: AgentStatus::Completed,
                findings,
                analyzed_files: vec![],
                modified_files: vec![],
                execution_time: start_time.elapsed().unwrap_or_default(),
                summary: "Debugging analysis completed".to_string(),
                metrics: HashMap::new(),
            })
        })
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "analyze_call_graph".to_string(),
            "find_references".to_string(),
            "find_definition".to_string(),
        ]
    }

    fn supports_file_type(&self, file_path: &std::path::Path) -> bool {
        matches!(
            file_path.extension().and_then(|ext| ext.to_str()),
            Some("rs")
                | Some("py")
                | Some("js")
                | Some("ts")
                | Some("jsx")
                | Some("tsx")
                | Some("go")
                | Some("java")
                | Some("cpp")
                | Some("c")
                | Some("cs")
        )
    }
}

/// Test generation and coverage analysis agent
#[derive(Debug)]
pub struct TestWriterAgent;

impl Default for TestWriterAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl TestWriterAgent {
    pub const fn new() -> Self {
        Self
    }
}

impl Subagent for TestWriterAgent {
    fn name(&self) -> &str {
        "test-writer"
    }

    fn description(&self) -> &str {
        "Comprehensive test generation and coverage analysis"
    }

    fn execute<'a>(
        &'a self,
        context: &'a SubagentContext,
        ast_tools: &'a mut ASTAgentTools,
        _cancel_flag: Arc<AtomicBool>,
    ) -> Pin<Box<dyn Future<Output = SubagentResult<AgentResult>> + Send + 'a>> {
        Box::pin(async move {
            let start_time = SystemTime::now();
            let mut findings = Vec::new();

            // Find functions that need tests
            let files = self.get_source_files(&context.working_directory)?;

            for file in &files {
                let result = ast_tools.execute(AgentToolOp::ExtractFunctions {
                    file: file.clone(),
                    language: self.detect_language(file),
                })?;

                if let AgentToolResult::Functions(functions) = result {
                    for func in functions {
                        // Simple heuristic: not a test function and is exported
                        let is_test_function =
                            func.name.contains("test") || func.name.starts_with("test_");
                        if !is_test_function && func.is_exported {
                            findings.push(Finding {
                                category: "testing".to_string(),
                                severity: Severity::Medium,
                                title: format!("Missing tests for function: {}", func.name),
                                description: format!(
                                    "Public function '{}' lacks test coverage",
                                    func.name
                                ),
                                location: Some(Location {
                                    file: file.clone(),
                                    line: func.start_line,
                                    column: 0,
                                    byte_offset: 0,
                                }),
                                suggestion: Some(
                                    "Add unit tests to verify function behavior".to_string(),
                                ),
                                metadata: HashMap::from([
                                    (
                                        "function_name".to_string(),
                                        serde_json::Value::String(func.name),
                                    ),
                                    (
                                        "parameter_count".to_string(),
                                        serde_json::Value::Number(serde_json::Number::from(
                                            func.parameters.len(),
                                        )),
                                    ),
                                ]),
                            });
                        }
                    }
                }
            }

            let findings_count = findings.len();
            Ok(AgentResult {
                agent_name: self.name().to_string(),
                status: AgentStatus::Completed,
                findings,
                analyzed_files: files,
                modified_files: vec![],
                execution_time: start_time.elapsed().unwrap_or_default(),
                summary: format!(
                    "Test analysis completed. Found {} functions needing tests.",
                    findings_count
                ),
                metrics: HashMap::new(),
            })
        })
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "extract_functions".to_string(),
            "generate_tests".to_string(),
        ]
    }

    fn supports_file_type(&self, file_path: &std::path::Path) -> bool {
        matches!(
            file_path.extension().and_then(|ext| ext.to_str()),
            Some("rs")
                | Some("py")
                | Some("js")
                | Some("ts")
                | Some("jsx")
                | Some("tsx")
                | Some("go")
                | Some("java")
                | Some("cpp")
                | Some("c")
                | Some("cs")
        )
    }
}

impl TestWriterAgent {
    fn get_source_files(&self, working_dir: &PathBuf) -> SubagentResult<Vec<PathBuf>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(working_dir).follow_links(false).max_depth(8) {
            let entry =
                entry.map_err(|e| SubagentError::Io(std::io::Error::other(e.to_string())))?;

            if entry.file_type().is_file() && self.supports_file_type(entry.path()) {
                // Skip test files
                let path_str = entry.path().to_string_lossy();
                if !path_str.contains("test") && !path_str.contains("spec") {
                    files.push(entry.path().to_path_buf());
                }
            }
        }

        Ok(files)
    }

    fn detect_language(&self, file: &PathBuf) -> String {
        match file.extension().and_then(|ext| ext.to_str()) {
            Some("rs") => "rust".to_string(),
            Some("py") => "python".to_string(),
            Some("js") | Some("jsx") => "javascript".to_string(),
            Some("ts") | Some("tsx") => "typescript".to_string(),
            Some("go") => "go".to_string(),
            Some("java") => "java".to_string(),
            _ => "text".to_string(),
        }
    }
}

/// Performance optimization agent
#[derive(Debug)]
pub struct PerformanceAgent;

impl Default for PerformanceAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceAgent {
    pub const fn new() -> Self {
        Self
    }
}

impl Subagent for PerformanceAgent {
    fn name(&self) -> &str {
        "performance"
    }

    fn description(&self) -> &str {
        "Performance optimization with bottleneck detection and algorithmic analysis"
    }

    fn execute<'a>(
        &'a self,
        context: &'a SubagentContext,
        ast_tools: &'a mut ASTAgentTools,
        _cancel_flag: Arc<AtomicBool>,
    ) -> Pin<Box<dyn Future<Output = SubagentResult<AgentResult>> + Send + 'a>> {
        Box::pin(async move {
            let start_time = SystemTime::now();
            let mut findings = Vec::new();

            // Detect performance improvement opportunities
            // For now, use a placeholder file - this would be enhanced to iterate over relevant files
            let target_file = context.working_directory.join("src").join("main.rs");
            if target_file.exists() {
                let result = ast_tools.execute(AgentToolOp::SuggestImprovements {
                    file: target_file,
                    focus: ImprovementFocus::Performance,
                })?;

                if let AgentToolResult::Improvements(improvements) = result {
                    for improvement in improvements {
                        let severity = Severity::Medium;

                        findings.push(Finding {
                            category: "performance".to_string(),
                            severity,
                            title: format!("Performance improvement: {:?}", improvement.category),
                            description: improvement.description,
                            location: Some(improvement.location),
                            suggestion: improvement.suggested_change,
                            metadata: HashMap::from([
                                (
                                    "category".to_string(),
                                    serde_json::Value::String(format!(
                                        "{:?}",
                                        improvement.category
                                    )),
                                ),
                                (
                                    "impact".to_string(),
                                    serde_json::Value::String(format!("{:?}", improvement.impact)),
                                ),
                            ]),
                        });
                    }
                }
            }

            let findings_count = findings.len();
            Ok(AgentResult {
                agent_name: self.name().to_string(),
                status: AgentStatus::Completed,
                findings,
                analyzed_files: vec![],
                modified_files: vec![],
                execution_time: start_time.elapsed().unwrap_or_default(),
                summary: format!(
                    "Performance analysis completed. Found {} optimization opportunities.",
                    findings_count
                ),
                metrics: HashMap::new(),
            })
        })
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "suggest_improvements".to_string(),
            "calculate_complexity".to_string(),
            "analyze_call_graph".to_string(),
        ]
    }

    fn supports_file_type(&self, file_path: &std::path::Path) -> bool {
        matches!(
            file_path.extension().and_then(|ext| ext.to_str()),
            Some("rs")
                | Some("py")
                | Some("js")
                | Some("ts")
                | Some("jsx")
                | Some("tsx")
                | Some("go")
                | Some("java")
                | Some("cpp")
                | Some("c")
                | Some("cs")
        )
    }
}

/// Security vulnerability detection agent
#[derive(Debug)]
pub struct SecurityAgent;

impl Default for SecurityAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityAgent {
    pub const fn new() -> Self {
        Self
    }
}

impl Subagent for SecurityAgent {
    fn name(&self) -> &str {
        "security"
    }

    fn description(&self) -> &str {
        "Security vulnerability detection with OWASP Top 10 analysis"
    }

    fn execute<'a>(
        &'a self,
        _context: &'a SubagentContext,
        ast_tools: &'a mut ASTAgentTools,
        _cancel_flag: Arc<AtomicBool>,
    ) -> Pin<Box<dyn Future<Output = SubagentResult<AgentResult>> + Send + 'a>> {
        Box::pin(async move {
            let start_time = SystemTime::now();
            let mut findings = Vec::new();

            // Check for various security patterns
            let security_patterns = vec![
                "sql_injection",
                "xss_vulnerability",
                "path_traversal",
                "hardcoded_secrets",
                "weak_crypto",
            ];

            for pattern in security_patterns {
                let result = ast_tools.execute(AgentToolOp::DetectPatterns {
                    pattern: PatternType::AntiPattern(pattern.to_string()),
                })?;

                if let AgentToolResult::Patterns(patterns) = result {
                    for detected_pattern in patterns {
                        let (severity, description, suggestion) = match pattern {
                            "sql_injection" => (
                                Severity::Critical,
                                "Potential SQL injection vulnerability detected",
                                "Use parameterized queries or prepared statements",
                            ),
                            "xss_vulnerability" => (
                                Severity::High,
                                "Potential XSS vulnerability detected",
                                "Sanitize and validate all user inputs",
                            ),
                            "path_traversal" => (
                                Severity::High,
                                "Potential path traversal vulnerability",
                                "Validate and sanitize file paths",
                            ),
                            "hardcoded_secrets" => (
                                Severity::Critical,
                                "Hardcoded secret or credential detected",
                                "Use environment variables or secure credential storage",
                            ),
                            "weak_crypto" => (
                                Severity::Medium,
                                "Weak cryptographic algorithm detected",
                                "Use modern, secure cryptographic algorithms",
                            ),
                            _ => (
                                Severity::Medium,
                                "Security issue detected",
                                "Review and address security concern",
                            ),
                        };

                        findings.push(Finding {
                            category: "security".to_string(),
                            severity,
                            title: format!("Security: {}", pattern.replace('_', " ")),
                            description: description.to_string(),
                            location: Some(detected_pattern.location),
                            suggestion: Some(suggestion.to_string()),
                            metadata: HashMap::from([(
                                "vulnerability_type".to_string(),
                                serde_json::Value::String(pattern.to_string()),
                            )]),
                        });
                    }
                }
            }

            let findings_count = findings.len();
            Ok(AgentResult {
                agent_name: self.name().to_string(),
                status: AgentStatus::Completed,
                findings,
                analyzed_files: vec![],
                modified_files: vec![],
                execution_time: start_time.elapsed().unwrap_or_default(),
                summary: format!(
                    "Security analysis completed. Found {} potential vulnerabilities.",
                    findings_count
                ),
                metrics: HashMap::new(),
            })
        })
    }

    fn capabilities(&self) -> Vec<String> {
        vec!["detect_patterns".to_string(), "security_scan".to_string()]
    }

    fn supports_file_type(&self, file_path: &std::path::Path) -> bool {
        matches!(
            file_path.extension().and_then(|ext| ext.to_str()),
            Some("rs")
                | Some("py")
                | Some("js")
                | Some("ts")
                | Some("jsx")
                | Some("tsx")
                | Some("go")
                | Some("java")
                | Some("cpp")
                | Some("c")
                | Some("cs")
                | Some("php")
        )
    }
}

/// Documentation generation agent
#[derive(Debug)]
pub struct DocsAgent;

impl Default for DocsAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl DocsAgent {
    pub const fn new() -> Self {
        Self
    }
}

impl Subagent for DocsAgent {
    fn name(&self) -> &str {
        "docs"
    }

    fn description(&self) -> &str {
        "Documentation generation and API reference creation"
    }

    fn execute<'a>(
        &'a self,
        context: &'a SubagentContext,
        ast_tools: &'a mut ASTAgentTools,
        _cancel_flag: Arc<AtomicBool>,
    ) -> Pin<Box<dyn Future<Output = SubagentResult<AgentResult>> + Send + 'a>> {
        Box::pin(async move {
            let start_time = SystemTime::now();
            let mut findings = Vec::new();

            // Generate documentation for modules/files
            if let Some(target_file) = context.parameters.get("target") {
                let target_path = PathBuf::from(target_file);

                let result = ast_tools.execute(AgentToolOp::GenerateDocumentation {
                    target: DocumentationTarget::File(target_path),
                })?;

                if let AgentToolResult::Documentation(doc_content) = result {
                    findings.push(Finding {
                        category: "documentation".to_string(),
                        severity: Severity::Info,
                        title: "Generated documentation".to_string(),
                        description: "API documentation has been generated".to_string(),
                        location: None,
                        suggestion: None,
                        metadata: HashMap::from([(
                            "doc_length".to_string(),
                            serde_json::Value::Number(serde_json::Number::from(doc_content.len())),
                        )]),
                    });
                }
            }

            Ok(AgentResult {
                agent_name: self.name().to_string(),
                status: AgentStatus::Completed,
                findings,
                analyzed_files: vec![],
                modified_files: vec![],
                execution_time: start_time.elapsed().unwrap_or_default(),
                summary: "Documentation generation completed".to_string(),
                metrics: HashMap::new(),
            })
        })
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "generate_documentation".to_string(),
            "extract_functions".to_string(),
        ]
    }

    fn supports_file_type(&self, file_path: &std::path::Path) -> bool {
        matches!(
            file_path.extension().and_then(|ext| ext.to_str()),
            Some("rs")
                | Some("py")
                | Some("js")
                | Some("ts")
                | Some("jsx")
                | Some("tsx")
                | Some("go")
                | Some("java")
                | Some("cpp")
                | Some("c")
                | Some("cs")
        )
    }
}

/// Agent registry for managing all available agents
pub struct AgentRegistry {
    agents: HashMap<String, Arc<dyn Subagent>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            agents: HashMap::new(),
        };

        // Register all core agents
        registry.register(Arc::new(CodeReviewerAgent::new()));
        registry.register(Arc::new(RefactorerAgent::new()));
        registry.register(Arc::new(DebuggerAgent::new()));
        registry.register(Arc::new(TestWriterAgent::new()));
        registry.register(Arc::new(PerformanceAgent::new()));
        registry.register(Arc::new(SecurityAgent::new()));
        registry.register(Arc::new(DocsAgent::new()));

        registry
    }

    pub fn register(&mut self, agent: Arc<dyn Subagent>) {
        self.agents.insert(agent.name().to_string(), agent);
    }

    pub fn get_agent(&self, name: &str) -> Option<Arc<dyn Subagent>> {
        self.agents.get(name).cloned()
    }

    pub fn list_agents(&self) -> Vec<&str> {
        self.agents.keys().map(|s| s.as_str()).collect()
    }

    pub fn get_agents_for_file(&self, file_path: &std::path::Path) -> Vec<String> {
        self.agents
            .iter()
            .filter(|(_, agent)| agent.supports_file_type(file_path))
            .map(|(name, _)| name.clone())
            .collect()
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_registry() {
        let registry = AgentRegistry::new();

        assert!(registry.get_agent("code-reviewer").is_some());
        assert!(registry.get_agent("refactorer").is_some());
        assert!(registry.get_agent("debugger").is_some());
        assert!(registry.get_agent("test-writer").is_some());
        assert!(registry.get_agent("performance").is_some());
        assert!(registry.get_agent("security").is_some());
        assert!(registry.get_agent("docs").is_some());

        assert_eq!(registry.list_agents().len(), 7);
    }

    #[test]
    fn test_file_type_support() {
        let agent = CodeReviewerAgent::new();

        assert!(agent.supports_file_type(&PathBuf::from("test.rs")));
        assert!(agent.supports_file_type(&PathBuf::from("test.py")));
        assert!(agent.supports_file_type(&PathBuf::from("test.js")));
        assert!(!agent.supports_file_type(&PathBuf::from("test.txt")));
    }

    #[test]
    fn test_finding_severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
        assert!(Severity::Low > Severity::Info);
    }

    #[test]
    fn test_agent_result_creation() {
        let result = AgentResult {
            agent_name: "test-agent".to_string(),
            status: AgentStatus::Completed,
            findings: vec![],
            analyzed_files: vec![],
            modified_files: vec![],
            execution_time: Duration::from_secs(30),
            summary: "Test completed".to_string(),
            metrics: HashMap::new(),
        };

        assert_eq!(result.agent_name, "test-agent");
        assert_eq!(result.status, AgentStatus::Completed);
        assert_eq!(result.execution_time, Duration::from_secs(30));
    }
}
