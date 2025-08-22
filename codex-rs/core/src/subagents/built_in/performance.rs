//! Performance Agent - Optimizes performance bottlenecks
//!
//! This agent identifies and resolves performance issues:
//! - Algorithm complexity analysis
//! - Memory usage optimization
//! - Cache efficiency improvements
//! - Database query optimization
//! - Concurrency enhancements

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

/// Performance Agent implementation
#[derive(Debug)]
pub struct PerformanceAgent {
    name: String,
    description: String,
    mode_override: Option<OperatingMode>,
    tool_permissions: Vec<String>,
    prompt_template: String,
    optimization_level: OptimizationLevel,
}

/// Optimization aggressiveness level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    Safe,       // Only guaranteed improvements
    Balanced,   // Standard optimizations with profiling
    Aggressive, // All possible optimizations, may require benchmarking
}

impl Default for PerformanceAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceAgent {
    /// Create a new performance agent
    pub fn new() -> Self {
        Self {
            name: "performance".to_string(),
            description: "Identifies and optimizes performance bottlenecks".to_string(),
            mode_override: Some(OperatingMode::Review),
            tool_permissions: vec![
                "search".to_string(),
                "tree".to_string(),
                "grep".to_string(),
                "edit".to_string(),
                "think".to_string(),
            ],
            prompt_template: r#"
You are a performance optimization expert focused on:
- Algorithm complexity reduction (O(n²) → O(n log n))
- Memory usage optimization
- Cache efficiency improvements
- Query optimization (N+1, indexes, etc.)
- Concurrency and parallelization
- I/O optimization

Always measure before and after optimization.
Consider trade-offs between time and space complexity.
"#
            .to_string(),
            optimization_level: OptimizationLevel::Balanced,
        }
    }

    /// Set optimization level
    pub const fn with_optimization_level(mut self, level: OptimizationLevel) -> Self {
        self.optimization_level = level;
        self
    }

    /// Analyze algorithm complexity
    async fn analyze_complexity(&self, ast_tools: &mut ASTAgentTools, file: &Path) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Find nested loops (potential O(n²) or worse)
        if let Ok(AgentToolResult::Patterns(patterns)) =
            ast_tools.execute(AgentToolOp::FindPatterns {
                pattern_type: crate::code_tools::ast_agent_tools::PatternType::NestedLoop,
                scope: crate::code_tools::search::SearchScope::Files(vec![file.to_path_buf()]),
            })
        {
            for pattern in patterns {
                // Analyze loop depth
                if let Ok(AgentToolResult::LoopAnalysis(analysis)) =
                    ast_tools.execute(AgentToolOp::AnalyzeLoop {
                        location: pattern.location.clone(),
                    })
                    && analysis.nesting_depth >= 2
                {
                    let complexity = match analysis.nesting_depth {
                        2 => "O(n²)",
                        3 => "O(n³)",
                        _ => "O(n^k) where k > 3",
                    };

                    findings.push(Finding {
                            category: "algorithm-complexity".to_string(),
                            severity: if analysis.nesting_depth > 2 {
                                Severity::Critical
                            } else {
                                Severity::High
                            },
                            title: format!("High Algorithmic Complexity: {}", complexity),
                            description: format!(
                                "Found {}-level nested loops at {}:{}. This results in {} time complexity.",
                                analysis.nesting_depth,
                                pattern.location.file.display(),
                                pattern.location.line,
                                complexity
                            ),
                            location: Some(pattern.location),
                            suggestion: Some(
                                "Consider using more efficient algorithms like hash maps, sorting, or divide-and-conquer".to_string()
                            ),
                            metadata: HashMap::from([
                                ("complexity".to_string(), serde_json::json!(complexity)),
                                ("nesting_depth".to_string(), serde_json::json!(analysis.nesting_depth)),
                            ]),
                        });
                }
            }
        }

        // Check for inefficient string concatenation in loops
        if let Ok(AgentToolResult::Patterns(patterns)) =
            ast_tools.execute(AgentToolOp::FindPatterns {
                pattern_type:
                    crate::code_tools::ast_agent_tools::PatternType::StringConcatenationInLoop,
                scope: crate::code_tools::search::SearchScope::Files(vec![file.to_path_buf()]),
            })
        {
            for pattern in patterns {
                findings.push(Finding {
                    category: "memory-performance".to_string(),
                    severity: Severity::Medium,
                    title: "Inefficient String Concatenation".to_string(),
                    description: format!(
                        "String concatenation in loop at {}:{}. This creates O(n²) memory allocations.",
                        pattern.location.file.display(),
                        pattern.location.line
                    ),
                    location: Some(pattern.location),
                    suggestion: Some("Use StringBuilder, StringBuffer, or join operations instead".to_string()),
                    metadata: HashMap::from([
                        ("issue_type".to_string(), serde_json::json!("string_concatenation")),
                    ]),
                });
            }
        }

        findings
    }

    /// Analyze database query performance
    async fn analyze_queries(&self, ast_tools: &mut ASTAgentTools, file: &Path) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Check for N+1 queries
        if let Ok(AgentToolResult::Patterns(patterns)) =
            ast_tools.execute(AgentToolOp::FindPatterns {
                pattern_type: crate::code_tools::ast_agent_tools::PatternType::NPlusOneQuery,
                scope: crate::code_tools::search::SearchScope::Files(vec![file.to_path_buf()]),
            })
        {
            for pattern in patterns {
                findings.push(Finding {
                    category: "database-performance".to_string(),
                    severity: Severity::High,
                    title: "N+1 Query Problem".to_string(),
                    description: format!(
                        "N+1 query pattern detected at {}:{}. This causes exponential database queries.",
                        pattern.location.file.display(),
                        pattern.location.line
                    ),
                    location: Some(pattern.location),
                    suggestion: Some(
                        "Use eager loading (includes/joins) or batch loading to fetch related data efficiently".to_string()
                    ),
                    metadata: HashMap::from([
                        ("issue_type".to_string(), serde_json::json!("n_plus_one")),
                    ]),
                });
            }
        }

        // Check for missing indexes
        if let Ok(AgentToolResult::Patterns(patterns)) =
            ast_tools.execute(AgentToolOp::FindPatterns {
                pattern_type: crate::code_tools::ast_agent_tools::PatternType::UnindexedQuery,
                scope: crate::code_tools::search::SearchScope::Files(vec![file.to_path_buf()]),
            })
        {
            for pattern in patterns {
                findings.push(Finding {
                    category: "database-performance".to_string(),
                    severity: Severity::Medium,
                    title: "Potentially Missing Index".to_string(),
                    description: format!(
                        "Query without index hint at {}:{}. This may cause full table scans.",
                        pattern.location.file.display(),
                        pattern.location.line
                    ),
                    location: Some(pattern.location),
                    suggestion: Some(
                        "Add appropriate database indexes for frequently queried columns"
                            .to_string(),
                    ),
                    metadata: HashMap::from([(
                        "issue_type".to_string(),
                        serde_json::json!("missing_index"),
                    )]),
                });
            }
        }

        findings
    }

    /// Analyze memory usage patterns
    async fn analyze_memory(&self, ast_tools: &mut ASTAgentTools, file: &Path) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Check for large allocations
        if let Ok(AgentToolResult::Patterns(patterns)) =
            ast_tools.execute(AgentToolOp::FindPatterns {
                pattern_type: crate::code_tools::ast_agent_tools::PatternType::LargeAllocation,
                scope: crate::code_tools::search::SearchScope::Files(vec![file.to_path_buf()]),
            })
        {
            for pattern in patterns {
                findings.push(Finding {
                    category: "memory-performance".to_string(),
                    severity: Severity::Medium,
                    title: "Large Memory Allocation".to_string(),
                    description: format!(
                        "Large memory allocation at {}:{}. Consider streaming or chunking.",
                        pattern.location.file.display(),
                        pattern.location.line
                    ),
                    location: Some(pattern.location),
                    suggestion: Some(
                        "Use streaming, pagination, or lazy loading for large data sets"
                            .to_string(),
                    ),
                    metadata: HashMap::from([(
                        "issue_type".to_string(),
                        serde_json::json!("large_allocation"),
                    )]),
                });
            }
        }

        // Check for memory leaks
        if let Ok(AgentToolResult::Patterns(patterns)) =
            ast_tools.execute(AgentToolOp::FindPatterns {
                pattern_type: crate::code_tools::ast_agent_tools::PatternType::MemoryLeak,
                scope: crate::code_tools::search::SearchScope::Files(vec![file.to_path_buf()]),
            })
        {
            for pattern in patterns {
                findings.push(Finding {
                    category: "memory-performance".to_string(),
                    severity: Severity::Critical,
                    title: "Potential Memory Leak".to_string(),
                    description: format!(
                        "Potential memory leak at {}:{}. Resources not properly released.",
                        pattern.location.file.display(),
                        pattern.location.line
                    ),
                    location: Some(pattern.location),
                    suggestion: Some(
                        "Ensure all resources are properly freed using RAII or try-finally blocks"
                            .to_string(),
                    ),
                    metadata: HashMap::from([(
                        "issue_type".to_string(),
                        serde_json::json!("memory_leak"),
                    )]),
                });
            }
        }

        findings
    }

    /// Suggest concurrency improvements
    async fn analyze_concurrency(
        &self,
        ast_tools: &mut ASTAgentTools,
        file: &Path,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Check for synchronous I/O that could be async
        if let Ok(AgentToolResult::Patterns(patterns)) =
            ast_tools.execute(AgentToolOp::FindPatterns {
                pattern_type: crate::code_tools::ast_agent_tools::PatternType::BlockingIO,
                scope: crate::code_tools::search::SearchScope::Files(vec![file.to_path_buf()]),
            })
        {
            for pattern in patterns {
                findings.push(Finding {
                    category: "concurrency-performance".to_string(),
                    severity: Severity::Medium,
                    title: "Blocking I/O Operation".to_string(),
                    description: format!(
                        "Blocking I/O at {}:{}. This reduces throughput and scalability.",
                        pattern.location.file.display(),
                        pattern.location.line
                    ),
                    location: Some(pattern.location),
                    suggestion: Some(
                        "Use async/await or non-blocking I/O for better concurrency".to_string(),
                    ),
                    metadata: HashMap::from([(
                        "issue_type".to_string(),
                        serde_json::json!("blocking_io"),
                    )]),
                });
            }
        }

        findings
    }
}

impl Subagent for PerformanceAgent {
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

            // Get files to analyze from context
            let files = self.get_performance_targets(context)?;

            for file in &files {
                if cancel_flag.load(Ordering::Acquire) {
                    return Err(SubagentError::ExecutionFailed(
                        "Performance analysis cancelled".to_string(),
                    ));
                }

                analyzed_files.push(file.clone());

                match self.optimization_level {
                    OptimizationLevel::Safe => {
                        // Only obvious performance issues
                        let complexity_findings = self.analyze_complexity(ast_tools, file).await;
                        all_findings.extend(complexity_findings);
                    }
                    OptimizationLevel::Balanced => {
                        // Standard performance analysis
                        let complexity_findings = self.analyze_complexity(ast_tools, file).await;
                        let query_findings = self.analyze_queries(ast_tools, file).await;
                        let memory_findings = self.analyze_memory(ast_tools, file).await;
                        all_findings.extend(complexity_findings);
                        all_findings.extend(query_findings);
                        all_findings.extend(memory_findings);
                    }
                    OptimizationLevel::Aggressive => {
                        // All performance optimizations
                        let complexity_findings = self.analyze_complexity(ast_tools, file).await;
                        let query_findings = self.analyze_queries(ast_tools, file).await;
                        let memory_findings = self.analyze_memory(ast_tools, file).await;
                        let concurrency_findings = self.analyze_concurrency(ast_tools, file).await;
                        all_findings.extend(complexity_findings);
                        all_findings.extend(query_findings);
                        all_findings.extend(memory_findings);
                        all_findings.extend(concurrency_findings);
                    }
                }
            }

            // Sort by severity
            all_findings.sort_by(|a, b| a.severity.cmp(&b.severity));

            let critical_count = all_findings
                .iter()
                .filter(|f| f.severity == Severity::Critical)
                .count();
            let high_count = all_findings
                .iter()
                .filter(|f| f.severity == Severity::High)
                .count();

            let summary = format!(
                "Performance analysis completed: {} files analyzed, {} issues found (Critical: {}, High: {})",
                analyzed_files.len(),
                all_findings.len(),
                critical_count,
                high_count
            );

            // Store the length before moving all_findings
            let performance_issues = all_findings.len();

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
                    (
                        "performance_issues".to_string(),
                        serde_json::json!(performance_issues),
                    ),
                    (
                        "critical_issues".to_string(),
                        serde_json::json!(critical_count),
                    ),
                    (
                        "optimization_level".to_string(),
                        serde_json::json!(format!("{:?}", self.optimization_level)),
                    ),
                ]),
            })
        })
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "complexity-analysis".to_string(),
            "query-optimization".to_string(),
            "memory-profiling".to_string(),
            "concurrency-analysis".to_string(),
            "cache-optimization".to_string(),
        ]
    }

    fn supports_file_type(&self, file_path: &Path) -> bool {
        let supported = [
            "rs", "py", "js", "ts", "go", "java", "cpp", "c", "rb", "scala",
        ];
        file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| supported.contains(&ext))
            .unwrap_or(false)
    }

    fn execution_time_estimate(&self) -> Duration {
        match self.optimization_level {
            OptimizationLevel::Safe => Duration::from_secs(45),
            OptimizationLevel::Balanced => Duration::from_secs(90),
            OptimizationLevel::Aggressive => Duration::from_secs(150),
        }
    }
}

impl PerformanceAgent {
    fn get_performance_targets(
        &self,
        context: &SubagentContext,
    ) -> Result<Vec<PathBuf>, SubagentError> {
        if let Some(files) = context.parameters.get("files") {
            Ok(files.split(',').map(|s| PathBuf::from(s.trim())).collect())
        } else {
            Ok(vec![context.working_directory.clone()])
        }
    }
}
