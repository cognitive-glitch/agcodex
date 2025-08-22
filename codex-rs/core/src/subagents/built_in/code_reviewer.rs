//! Code Reviewer Agent - Reviews code for quality, security, and maintainability
//!
//! This agent performs comprehensive code review using AST analysis to identify:
//! - Security vulnerabilities (OWASP Top 10)
//! - Performance issues and bottlenecks
//! - Code complexity and maintainability concerns
//! - Best practice violations
//! - Missing error handling

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
use serde::Deserialize;
use serde::Serialize;
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

/// Code Reviewer Agent implementation
#[derive(Debug)]
pub struct CodeReviewerAgent {
    /// Agent name
    name: String,
    /// Agent description
    description: String,
    /// Operating mode override
    mode_override: Option<OperatingMode>,
    /// Tool permissions
    tool_permissions: Vec<String>,
    /// Custom prompt template
    prompt_template: String,
    /// Intelligence level
    intelligence_level: IntelligenceLevel,
}

/// Intelligence level for analysis depth
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntelligenceLevel {
    Light,  // Quick surface-level review
    Medium, // Standard review depth
    Hard,   // Deep analysis with all checks
}

impl Default for CodeReviewerAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeReviewerAgent {
    /// Create a new code reviewer agent
    pub fn new() -> Self {
        Self {
            name: "code-reviewer".to_string(),
            description: "Reviews code for quality, security, and maintainability".to_string(),
            mode_override: Some(OperatingMode::Review),
            tool_permissions: vec![
                "search".to_string(),
                "tree".to_string(),
                "grep".to_string(),
                "glob".to_string(),
            ],
            prompt_template: r#"
You are a senior code reviewer with expertise in:
- Security vulnerability detection (OWASP Top 10)
- Performance optimization
- Code maintainability and readability
- Design patterns and best practices
- Error handling and edge cases

Focus on finding actionable issues with clear remediation steps.
Prioritize security and data integrity issues as Critical/High severity.
"#
            .to_string(),
            intelligence_level: IntelligenceLevel::Medium,
        }
    }

    /// Set intelligence level
    pub const fn with_intelligence(mut self, level: IntelligenceLevel) -> Self {
        self.intelligence_level = level;
        self
    }

    /// Perform security analysis
    async fn analyze_security(&self, ast_tools: &mut ASTAgentTools, file: &Path) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Check for SQL injection vulnerabilities
        if let Ok(AgentToolResult::Patterns(patterns)) =
            ast_tools.execute(AgentToolOp::FindPatterns {
                pattern_type: crate::code_tools::ast_agent_tools::PatternType::SqlInjection,
                scope: crate::code_tools::search::SearchScope::Files(vec![file.to_path_buf()]),
            })
        {
            for pattern in patterns {
                findings.push(Finding {
                    category: "security".to_string(),
                    severity: Severity::Critical,
                    title: "Potential SQL Injection Vulnerability".to_string(),
                    description: format!(
                        "Found potential SQL injection at {}:{}. User input may be directly concatenated into SQL query.",
                        pattern.location.file.display(),
                        pattern.location.line
                    ),
                    location: Some(pattern.location),
                    suggestion: Some("Use parameterized queries or prepared statements instead of string concatenation".to_string()),
                    metadata: HashMap::from([
                        ("vulnerability_type".to_string(), serde_json::json!("sql_injection")),
                        ("owasp_category".to_string(), serde_json::json!("A03:2021")),
                    ]),
                });
            }
        }

        // Check for hardcoded secrets
        if let Ok(AgentToolResult::Patterns(patterns)) =
            ast_tools.execute(AgentToolOp::FindPatterns {
                pattern_type: crate::code_tools::ast_agent_tools::PatternType::HardcodedSecrets,
                scope: crate::code_tools::search::SearchScope::Files(vec![file.to_path_buf()]),
            })
        {
            for pattern in patterns {
                findings.push(Finding {
                    category: "security".to_string(),
                    severity: Severity::High,
                    title: "Hardcoded Secret Detected".to_string(),
                    description: format!(
                        "Found hardcoded secret at {}:{}. This could lead to credential exposure.",
                        pattern.location.file.display(),
                        pattern.location.line
                    ),
                    location: Some(pattern.location),
                    suggestion: Some(
                        "Use environment variables or secure secret management systems".to_string(),
                    ),
                    metadata: HashMap::from([
                        (
                            "vulnerability_type".to_string(),
                            serde_json::json!("hardcoded_secret"),
                        ),
                        ("owasp_category".to_string(), serde_json::json!("A07:2021")),
                    ]),
                });
            }
        }

        findings
    }

    /// Analyze code complexity
    async fn analyze_complexity(&self, ast_tools: &mut ASTAgentTools, file: &Path) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Extract functions and analyze complexity
        if let Ok(AgentToolResult::Functions(functions)) =
            ast_tools.execute(AgentToolOp::ExtractFunctions {
                file: file.to_path_buf(),
                language: self.detect_language(file),
            })
        {
            for func in functions {
                // Check cyclomatic complexity
                if let Ok(AgentToolResult::Complexity(complexity)) =
                    ast_tools.execute(AgentToolOp::CalculateComplexity {
                        function: func.name.clone(),
                    })
                {
                    if complexity.cyclomatic_complexity > 10 {
                        findings.push(Finding {
                            category: "complexity".to_string(),
                            severity: if complexity.cyclomatic_complexity > 20 {
                                Severity::High
                            } else {
                                Severity::Medium
                            },
                            title: format!("High Complexity: {}", func.name),
                            description: format!(
                                "Function '{}' has cyclomatic complexity of {}. Complex functions are harder to test and maintain.",
                                func.name, complexity.cyclomatic_complexity
                            ),
                            location: Some(crate::code_tools::ast_agent_tools::Location {
                                file: file.to_path_buf(),
                                line: func.start_line,
                                column: 0,
                                byte_offset: 0,
                            }),
                            suggestion: Some("Consider breaking this function into smaller, more focused functions".to_string()),
                            metadata: HashMap::from([
                                ("complexity".to_string(), serde_json::json!(complexity.cyclomatic_complexity)),
                                ("threshold".to_string(), serde_json::json!(10)),
                            ]),
                        });
                    }

                    // Check cognitive complexity
                    if complexity.cognitive_complexity > 15 {
                        findings.push(Finding {
                            category: "readability".to_string(),
                            severity: Severity::Medium,
                            title: format!("High Cognitive Complexity: {}", func.name),
                            description: format!(
                                "Function '{}' has cognitive complexity of {}. This makes the code harder to understand.",
                                func.name, complexity.cognitive_complexity
                            ),
                            location: Some(crate::code_tools::ast_agent_tools::Location {
                                file: file.to_path_buf(),
                                line: func.start_line,
                                column: 0,
                                byte_offset: 0,
                            }),
                            suggestion: Some("Simplify control flow and reduce nesting levels".to_string()),
                            metadata: HashMap::from([
                                ("cognitive_complexity".to_string(), serde_json::json!(complexity.cognitive_complexity)),
                                ("threshold".to_string(), serde_json::json!(15)),
                            ]),
                        });
                    }
                }

                // Check function length
                let line_count = func.end_line - func.start_line + 1;
                if line_count > 50 {
                    findings.push(Finding {
                        category: "maintainability".to_string(),
                        severity: if line_count > 100 { Severity::High } else { Severity::Medium },
                        title: format!("Long Function: {}", func.name),
                        description: format!(
                            "Function '{}' is {} lines long. Long functions are harder to understand and test.",
                            func.name, line_count
                        ),
                        location: Some(crate::code_tools::ast_agent_tools::Location {
                            file: file.to_path_buf(),
                            line: func.start_line,
                            column: 0,
                            byte_offset: 0,
                        }),
                        suggestion: Some("Extract cohesive blocks into separate functions".to_string()),
                        metadata: HashMap::from([
                            ("line_count".to_string(), serde_json::json!(line_count)),
                            ("threshold".to_string(), serde_json::json!(50)),
                        ]),
                    });
                }
            }
        }

        findings
    }

    /// Check for performance issues
    async fn analyze_performance(
        &self,
        ast_tools: &mut ASTAgentTools,
        file: &Path,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Check for N+1 query patterns
        if let Ok(AgentToolResult::Patterns(patterns)) =
            ast_tools.execute(AgentToolOp::FindPatterns {
                pattern_type: crate::code_tools::ast_agent_tools::PatternType::NPlusOneQuery,
                scope: crate::code_tools::search::SearchScope::Files(vec![file.to_path_buf()]),
            })
        {
            for pattern in patterns {
                findings.push(Finding {
                    category: "performance".to_string(),
                    severity: Severity::High,
                    title: "Potential N+1 Query Problem".to_string(),
                    description: format!(
                        "Found potential N+1 query pattern at {}:{}. This can cause severe performance issues.",
                        pattern.location.file.display(),
                        pattern.location.line
                    ),
                    location: Some(pattern.location),
                    suggestion: Some("Use eager loading, batch queries, or data loaders to fetch related data efficiently".to_string()),
                    metadata: HashMap::from([
                        ("issue_type".to_string(), serde_json::json!("n_plus_one")),
                    ]),
                });
            }
        }

        // Check for inefficient loops
        if let Ok(AgentToolResult::Patterns(patterns)) =
            ast_tools.execute(AgentToolOp::FindPatterns {
                pattern_type: crate::code_tools::ast_agent_tools::PatternType::InefficientLoop,
                scope: crate::code_tools::search::SearchScope::Files(vec![file.to_path_buf()]),
            })
        {
            for pattern in patterns {
                findings.push(Finding {
                    category: "performance".to_string(),
                    severity: Severity::Medium,
                    title: "Inefficient Loop Pattern".to_string(),
                    description: format!(
                        "Found inefficient loop at {}:{}. Consider optimizing this loop for better performance.",
                        pattern.location.file.display(),
                        pattern.location.line
                    ),
                    location: Some(pattern.location),
                    suggestion: Some("Consider using more efficient algorithms or data structures".to_string()),
                    metadata: HashMap::from([
                        ("issue_type".to_string(), serde_json::json!("inefficient_loop")),
                    ]),
                });
            }
        }

        findings
    }

    /// Detect language from file extension
    fn detect_language(&self, file: &Path) -> String {
        file.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext {
                "rs" => "rust",
                "py" => "python",
                "js" | "jsx" => "javascript",
                "ts" | "tsx" => "typescript",
                "go" => "go",
                "java" => "java",
                "cpp" | "cc" | "cxx" => "cpp",
                "c" => "c",
                "rb" => "ruby",
                _ => "text",
            })
            .unwrap_or("text")
            .to_string()
    }
}

impl Subagent for CodeReviewerAgent {
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
            let files_to_analyze = self.get_files_to_analyze(context)?;

            for file in &files_to_analyze {
                if cancel_flag.load(Ordering::Acquire) {
                    return Err(SubagentError::ExecutionFailed(
                        "Review cancelled".to_string(),
                    ));
                }

                analyzed_files.push(file.clone());

                // Perform different types of analysis based on intelligence level
                match self.intelligence_level {
                    IntelligenceLevel::Light => {
                        // Quick complexity check only
                        let complexity_findings = self.analyze_complexity(ast_tools, file).await;
                        all_findings.extend(complexity_findings);
                    }
                    IntelligenceLevel::Medium => {
                        // Standard review: complexity + security
                        let complexity_findings = self.analyze_complexity(ast_tools, file).await;
                        let security_findings = self.analyze_security(ast_tools, file).await;
                        all_findings.extend(complexity_findings);
                        all_findings.extend(security_findings);
                    }
                    IntelligenceLevel::Hard => {
                        // Deep analysis: all checks
                        let complexity_findings = self.analyze_complexity(ast_tools, file).await;
                        let security_findings = self.analyze_security(ast_tools, file).await;
                        let performance_findings = self.analyze_performance(ast_tools, file).await;
                        all_findings.extend(complexity_findings);
                        all_findings.extend(security_findings);
                        all_findings.extend(performance_findings);
                    }
                }
            }

            // Sort findings by severity
            all_findings.sort_by(|a, b| a.severity.cmp(&b.severity));

            // Generate summary
            let critical_count = all_findings
                .iter()
                .filter(|f| f.severity == Severity::Critical)
                .count();
            let high_count = all_findings
                .iter()
                .filter(|f| f.severity == Severity::High)
                .count();
            let medium_count = all_findings
                .iter()
                .filter(|f| f.severity == Severity::Medium)
                .count();
            let low_count = all_findings
                .iter()
                .filter(|f| f.severity == Severity::Low)
                .count();

            // Store the length before moving all_findings
            let total_findings = all_findings.len();

            let summary = format!(
                "Code review completed: {} files analyzed, {} findings (Critical: {}, High: {}, Medium: {}, Low: {})",
                analyzed_files.len(),
                total_findings,
                critical_count,
                high_count,
                medium_count,
                low_count
            );

            let execution_time = SystemTime::now()
                .duration_since(start_time)
                .unwrap_or_else(|_| Duration::from_secs(0));

            Ok(AgentResult {
                agent_name: self.name.clone(),
                status: AgentStatus::Completed,
                findings: all_findings,
                analyzed_files,
                modified_files: Vec::new(), // Code review doesn't modify files
                execution_time,
                summary,
                metrics: HashMap::from([
                    (
                        "total_findings".to_string(),
                        serde_json::json!(total_findings),
                    ),
                    (
                        "critical_findings".to_string(),
                        serde_json::json!(critical_count),
                    ),
                    ("high_findings".to_string(), serde_json::json!(high_count)),
                    (
                        "intelligence_level".to_string(),
                        serde_json::json!(self.intelligence_level),
                    ),
                ]),
            })
        })
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "ast-analysis".to_string(),
            "security-scanning".to_string(),
            "complexity-analysis".to_string(),
            "performance-analysis".to_string(),
            "best-practices".to_string(),
        ]
    }

    fn supports_file_type(&self, file_path: &Path) -> bool {
        let supported_extensions = [
            "rs", "py", "js", "ts", "jsx", "tsx", "go", "java", "cpp", "c", "rb",
        ];

        file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| supported_extensions.contains(&ext))
            .unwrap_or(false)
    }

    fn execution_time_estimate(&self) -> Duration {
        match self.intelligence_level {
            IntelligenceLevel::Light => Duration::from_secs(30),
            IntelligenceLevel::Medium => Duration::from_secs(60),
            IntelligenceLevel::Hard => Duration::from_secs(120),
        }
    }
}

impl CodeReviewerAgent {
    /// Get files to analyze from context
    fn get_files_to_analyze(
        &self,
        context: &SubagentContext,
    ) -> Result<Vec<PathBuf>, SubagentError> {
        // Extract file paths from context parameters
        if let Some(files) = context.parameters.get("files") {
            let paths: Vec<PathBuf> = files.split(',').map(|s| PathBuf::from(s.trim())).collect();
            Ok(paths)
        } else {
            // Default to current directory
            Ok(vec![context.working_directory.clone()])
        }
    }
}
