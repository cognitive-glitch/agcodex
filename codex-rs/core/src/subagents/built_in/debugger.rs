//! Debugger Agent - Deep debugging and root cause analysis
//!
//! This agent performs comprehensive debugging analysis:
//! - Trace execution paths
//! - Identify error patterns
//! - Analyze stack traces
//! - Find race conditions
//! - Detect memory leaks

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

/// Debugger Agent implementation
#[derive(Debug)]
pub struct DebuggerAgent {
    name: String,
    description: String,
    mode_override: Option<OperatingMode>,
    tool_permissions: Vec<String>,
    prompt_template: String,
    debug_depth: DebugDepth,
}

/// Debug analysis depth
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugDepth {
    Surface,  // Quick error check
    Standard, // Normal debugging
    Deep,     // Comprehensive analysis with execution tracing
}

impl Default for DebuggerAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl DebuggerAgent {
    /// Create a new debugger agent
    pub fn new() -> Self {
        Self {
            name: "debugger".to_string(),
            description: "Deep debugging analysis and root cause identification".to_string(),
            mode_override: Some(OperatingMode::Review),
            tool_permissions: vec![
                "search".to_string(),
                "tree".to_string(),
                "grep".to_string(),
                "bash".to_string(),
                "think".to_string(),
            ],
            prompt_template: r#"
You are an expert debugger specializing in:
- Root cause analysis
- Execution path tracing
- Error pattern recognition
- Race condition detection
- Memory leak identification
- Performance bottleneck analysis

Focus on finding the root cause, not just symptoms.
Provide clear reproduction steps and fixes.
"#
            .to_string(),
            debug_depth: DebugDepth::Standard,
        }
    }

    /// Set debug depth
    pub const fn with_depth(mut self, depth: DebugDepth) -> Self {
        self.debug_depth = depth;
        self
    }

    /// Analyze error patterns
    async fn analyze_error_patterns(
        &self,
        ast_tools: &mut ASTAgentTools,
        file: &Path,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Check for unhandled errors
        if let Ok(AgentToolResult::Patterns(patterns)) =
            ast_tools.execute(AgentToolOp::FindPatterns {
                pattern_type: crate::code_tools::ast_agent_tools::PatternType::UnhandledError,
                scope: crate::code_tools::search::SearchScope::Files(vec![file.to_path_buf()]),
            })
        {
            for pattern in patterns {
                findings.push(Finding {
                    category: "error-handling".to_string(),
                    severity: Severity::High,
                    title: "Unhandled Error Detected".to_string(),
                    description: format!(
                        "Found unhandled error at {}:{}. This could cause crashes or data loss.",
                        pattern.location.file.display(),
                        pattern.location.line
                    ),
                    location: Some(pattern.location),
                    suggestion: Some("Add proper error handling with recovery logic".to_string()),
                    metadata: HashMap::from([(
                        "debug_type".to_string(),
                        serde_json::json!("unhandled_error"),
                    )]),
                });
            }
        }

        // Check for race conditions
        if let Ok(AgentToolResult::Patterns(patterns)) =
            ast_tools.execute(AgentToolOp::FindPatterns {
                pattern_type: crate::code_tools::ast_agent_tools::PatternType::RaceCondition,
                scope: crate::code_tools::search::SearchScope::Files(vec![file.to_path_buf()]),
            })
        {
            for pattern in patterns {
                findings.push(Finding {
                    category: "concurrency".to_string(),
                    severity: Severity::Critical,
                    title: "Potential Race Condition".to_string(),
                    description: format!(
                        "Found potential race condition at {}:{}. Shared state accessed without synchronization.",
                        pattern.location.file.display(),
                        pattern.location.line
                    ),
                    location: Some(pattern.location),
                    suggestion: Some("Use proper synchronization primitives (mutex, channels, etc.)".to_string()),
                    metadata: HashMap::from([
                        ("debug_type".to_string(), serde_json::json!("race_condition")),
                    ]),
                });
            }
        }

        findings
    }

    /// Trace execution paths
    async fn trace_execution_paths(
        &self,
        ast_tools: &mut ASTAgentTools,
        entry_point: &str,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Analyze call graph from entry point
        if let Ok(AgentToolResult::CallGraph(graph)) =
            ast_tools.execute(AgentToolOp::AnalyzeCallGraph {
                entry_point: entry_point.to_string(),
            })
        {
            // Check for infinite recursion
            for cycle in graph.cycles {
                findings.push(Finding {
                    category: "logic-error".to_string(),
                    severity: Severity::Critical,
                    title: "Infinite Recursion Detected".to_string(),
                    description: format!("Found recursive cycle: {}", cycle.join(" -> ")),
                    location: None,
                    suggestion: Some("Add base case or termination condition".to_string()),
                    metadata: HashMap::from([
                        (
                            "debug_type".to_string(),
                            serde_json::json!("infinite_recursion"),
                        ),
                        ("cycle".to_string(), serde_json::json!(cycle)),
                    ]),
                });
            }

            // Check for unreachable code
            for unreachable in graph.unreachable_functions {
                findings.push(Finding {
                    category: "dead-code".to_string(),
                    severity: Severity::Low,
                    title: format!("Unreachable Code: {}", unreachable),
                    description: format!(
                        "Function '{}' is never called from any entry point",
                        unreachable
                    ),
                    location: None,
                    suggestion: Some("Remove dead code or add proper entry points".to_string()),
                    metadata: HashMap::from([
                        (
                            "debug_type".to_string(),
                            serde_json::json!("unreachable_code"),
                        ),
                        ("function".to_string(), serde_json::json!(unreachable)),
                    ]),
                });
            }
        }

        findings
    }

    /// Analyze memory issues
    async fn analyze_memory_issues(
        &self,
        ast_tools: &mut ASTAgentTools,
        file: &Path,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Check for memory leaks
        if let Ok(AgentToolResult::Patterns(patterns)) =
            ast_tools.execute(AgentToolOp::FindPatterns {
                pattern_type: crate::code_tools::ast_agent_tools::PatternType::MemoryLeak,
                scope: crate::code_tools::search::SearchScope::Files(vec![file.to_path_buf()]),
            })
        {
            for pattern in patterns {
                findings.push(Finding {
                    category: "memory".to_string(),
                    severity: Severity::High,
                    title: "Potential Memory Leak".to_string(),
                    description: format!(
                        "Found potential memory leak at {}:{}. Resource allocated but not freed.",
                        pattern.location.file.display(),
                        pattern.location.line
                    ),
                    location: Some(pattern.location),
                    suggestion: Some(
                        "Ensure all allocated resources are properly freed".to_string(),
                    ),
                    metadata: HashMap::from([(
                        "debug_type".to_string(),
                        serde_json::json!("memory_leak"),
                    )]),
                });
            }
        }

        findings
    }
}

impl Subagent for DebuggerAgent {
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

            // Get debug targets from context
            let files = self.get_debug_targets(context)?;
            let entry_point = context
                .parameters
                .get("entry_point")
                .map(|s| s.as_str())
                .unwrap_or("main");

            for file in &files {
                if cancel_flag.load(Ordering::Acquire) {
                    return Err(SubagentError::ExecutionFailed(
                        "Debugging cancelled".to_string(),
                    ));
                }

                analyzed_files.push(file.clone());

                match self.debug_depth {
                    DebugDepth::Surface => {
                        // Quick error pattern check
                        let error_findings = self.analyze_error_patterns(ast_tools, file).await;
                        all_findings.extend(error_findings);
                    }
                    DebugDepth::Standard => {
                        // Standard debugging
                        let error_findings = self.analyze_error_patterns(ast_tools, file).await;
                        let memory_findings = self.analyze_memory_issues(ast_tools, file).await;
                        all_findings.extend(error_findings);
                        all_findings.extend(memory_findings);
                    }
                    DebugDepth::Deep => {
                        // Deep analysis with execution tracing
                        let error_findings = self.analyze_error_patterns(ast_tools, file).await;
                        let memory_findings = self.analyze_memory_issues(ast_tools, file).await;
                        let trace_findings =
                            self.trace_execution_paths(ast_tools, entry_point).await;
                        all_findings.extend(error_findings);
                        all_findings.extend(memory_findings);
                        all_findings.extend(trace_findings);
                    }
                }
            }

            let critical_count = all_findings
                .iter()
                .filter(|f| f.severity == Severity::Critical)
                .count();
            let high_count = all_findings
                .iter()
                .filter(|f| f.severity == Severity::High)
                .count();

            let summary = format!(
                "Debug analysis completed: {} files analyzed, {} issues found (Critical: {}, High: {})",
                analyzed_files.len(),
                all_findings.len(),
                critical_count,
                high_count
            );

            // Store the length before moving all_findings
            let issues_found = all_findings.len();

            let execution_time = SystemTime::now()
                .duration_since(start_time)
                .unwrap_or_else(|_| Duration::from_secs(0));

            Ok(AgentResult {
                agent_name: self.name.clone(),
                status: AgentStatus::Completed,
                findings: all_findings,
                analyzed_files,
                modified_files: Vec::new(),
                execution_time,
                summary,
                metrics: HashMap::from([
                    ("issues_found".to_string(), serde_json::json!(issues_found)),
                    (
                        "critical_issues".to_string(),
                        serde_json::json!(critical_count),
                    ),
                    (
                        "debug_depth".to_string(),
                        serde_json::json!(format!("{:?}", self.debug_depth)),
                    ),
                ]),
            })
        })
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "error-analysis".to_string(),
            "execution-tracing".to_string(),
            "race-detection".to_string(),
            "memory-analysis".to_string(),
            "root-cause-analysis".to_string(),
        ]
    }

    fn supports_file_type(&self, file_path: &Path) -> bool {
        let supported = ["rs", "py", "js", "ts", "go", "java", "cpp", "c"];
        file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| supported.contains(&ext))
            .unwrap_or(false)
    }

    fn execution_time_estimate(&self) -> Duration {
        match self.debug_depth {
            DebugDepth::Surface => Duration::from_secs(30),
            DebugDepth::Standard => Duration::from_secs(60),
            DebugDepth::Deep => Duration::from_secs(120),
        }
    }
}

impl DebuggerAgent {
    fn get_debug_targets(&self, context: &SubagentContext) -> Result<Vec<PathBuf>, SubagentError> {
        if let Some(files) = context.parameters.get("files") {
            Ok(files.split(',').map(|s| PathBuf::from(s.trim())).collect())
        } else {
            Ok(vec![context.working_directory.clone()])
        }
    }
}
