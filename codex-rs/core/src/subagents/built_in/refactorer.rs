//! Refactorer Agent - Performs systematic code refactoring
//!
//! This agent analyzes code structure and performs safe refactoring operations:
//! - Extract method/function
//! - Rename variables and functions
//! - Remove duplicate code
//! - Simplify complex expressions
//! - Apply design patterns

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

/// Refactorer Agent implementation
#[derive(Debug)]
pub struct RefactorerAgent {
    name: String,
    description: String,
    _mode_override: Option<OperatingMode>,
    _tool_permissions: Vec<String>,
    _prompt_template: String,
    risk_tolerance: RiskLevel,
}

/// Risk level for refactoring operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Conservative, // Only safe, guaranteed refactorings
    Moderate,     // Standard refactorings with good test coverage
    Aggressive,   // Complex refactorings that may require manual review
}

impl Default for RefactorerAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl RefactorerAgent {
    /// Create a new refactorer agent
    pub fn new() -> Self {
        Self {
            name: "refactorer".to_string(),
            description: "Performs systematic code refactoring with risk assessment".to_string(),
            _mode_override: Some(OperatingMode::Build),
            _tool_permissions: vec![
                "search".to_string(),
                "edit".to_string(),
                "patch".to_string(),
                "tree".to_string(),
                "grep".to_string(),
            ],
            _prompt_template: r#"
You are an expert refactoring specialist focused on:
- Improving code structure without changing behavior
- Reducing complexity and duplication
- Applying SOLID principles
- Enhancing readability and maintainability
- Ensuring backwards compatibility

Always verify that refactorings preserve functionality.
Create comprehensive test cases before major changes.
"#
            .to_string(),
            risk_tolerance: RiskLevel::Moderate,
        }
    }

    /// Set risk tolerance level
    pub const fn with_risk_level(mut self, level: RiskLevel) -> Self {
        self.risk_tolerance = level;
        self
    }

    /// Identify refactoring opportunities
    async fn identify_refactoring_opportunities(
        &self,
        ast_tools: &mut ASTAgentTools,
        file: &Path,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Check for duplicate code
        if let Ok(AgentToolResult::DuplicateCode(duplicates)) =
            ast_tools.execute(AgentToolOp::FindDuplicateCode {
                min_lines: 5,
                similarity_threshold: 0.85,
            })
        {
            for duplicate in duplicates {
                findings.push(Finding {
                    category: "refactoring".to_string(),
                    severity: Severity::Medium,
                    title: "Duplicate Code Detected".to_string(),
                    description: format!(
                        "Found {} duplicate code blocks with {} lines each",
                        duplicate.locations.len(),
                        duplicate.line_count
                    ),
                    location: duplicate.locations.first().cloned(),
                    suggestion: Some("Extract duplicate code into a shared function".to_string()),
                    metadata: HashMap::from([
                        (
                            "refactoring_type".to_string(),
                            serde_json::json!("extract_method"),
                        ),
                        (
                            "duplicate_count".to_string(),
                            serde_json::json!(duplicate.locations.len()),
                        ),
                        (
                            "line_count".to_string(),
                            serde_json::json!(duplicate.line_count),
                        ),
                    ]),
                });
            }
        }

        // Check for long parameter lists
        if let Ok(AgentToolResult::Functions(functions)) =
            ast_tools.execute(AgentToolOp::ExtractFunctions {
                file: file.to_path_buf(),
                language: self.detect_language(file),
            })
        {
            for func in functions {
                if func.parameters.len() > 4 {
                    findings.push(Finding {
                        category: "refactoring".to_string(),
                        severity: Severity::Low,
                        title: format!("Long Parameter List: {}", func.name),
                        description: format!(
                            "Function '{}' has {} parameters. Consider using a parameter object.",
                            func.name,
                            func.parameters.len()
                        ),
                        location: Some(crate::code_tools::ast_agent_tools::Location {
                            file: file.to_path_buf(),
                            line: func.start_line,
                            column: 0,
                            byte_offset: 0,
                        }),
                        suggestion: Some(
                            "Introduce a parameter object or builder pattern".to_string(),
                        ),
                        metadata: HashMap::from([
                            (
                                "refactoring_type".to_string(),
                                serde_json::json!("introduce_parameter_object"),
                            ),
                            (
                                "parameter_count".to_string(),
                                serde_json::json!(func.parameters.len()),
                            ),
                        ]),
                    });
                }
            }
        }

        findings
    }

    /// Apply safe refactorings
    async fn apply_refactorings(
        &self,
        ast_tools: &mut ASTAgentTools,
        findings: &[Finding],
    ) -> Vec<PathBuf> {
        let mut modified_files = Vec::new();

        for finding in findings {
            // Only apply refactorings based on risk level
            let should_apply = match self.risk_tolerance {
                RiskLevel::Conservative => finding.severity == Severity::Low,
                RiskLevel::Moderate => finding.severity != Severity::Critical,
                RiskLevel::Aggressive => true,
            };

            if should_apply && let Some(refactoring_type) = finding.metadata.get("refactoring_type")
            {
                match refactoring_type.as_str() {
                    Some("extract_method") => {
                        // Apply extract method refactoring
                        if let Some(location) = &finding.location
                            && let Ok(AgentToolResult::Refactored(result)) =
                                ast_tools.execute(AgentToolOp::RefactorExtractMethod {
                                    location: location.clone(),
                                    new_name: "extracted_function".to_string(),
                                })
                            && result.success
                        {
                            modified_files.push(location.file.clone());
                        }
                    }
                    Some("introduce_parameter_object") => {
                        // Apply parameter object refactoring
                        if let Some(location) = &finding.location
                            && let Ok(AgentToolResult::Refactored(result)) =
                                ast_tools.execute(AgentToolOp::RefactorIntroduceParameterObject {
                                    location: location.clone(),
                                    object_name: "Parameters".to_string(),
                                })
                            && result.success
                        {
                            modified_files.push(location.file.clone());
                        }
                    }
                    _ => {}
                }
            }
        }

        modified_files
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
                _ => "text",
            })
            .unwrap_or("text")
            .to_string()
    }
}

impl Subagent for RefactorerAgent {
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
            let mut modified_files = Vec::new();

            // Get files to refactor from context
            let files = self.get_files_from_context(context)?;

            for file in &files {
                if cancel_flag.load(Ordering::Acquire) {
                    return Err(SubagentError::ExecutionFailed(
                        "Refactoring cancelled".to_string(),
                    ));
                }

                analyzed_files.push(file.clone());

                // Identify refactoring opportunities
                let findings = self
                    .identify_refactoring_opportunities(ast_tools, file)
                    .await;
                all_findings.extend(findings.clone());

                // Apply refactorings if in Build mode
                if context.mode == OperatingMode::Build {
                    let modified = self.apply_refactorings(ast_tools, &findings).await;
                    modified_files.extend(modified);
                }
            }

            let summary = format!(
                "Refactoring analysis completed: {} files analyzed, {} opportunities found, {} files modified",
                analyzed_files.len(),
                all_findings.len(),
                modified_files.len()
            );

            // Store lengths before moving the values
            let refactoring_opportunities = all_findings.len();
            let files_modified = modified_files.len();

            let execution_time = SystemTime::now()
                .duration_since(start_time)
                .unwrap_or_else(|_| Duration::from_secs(0));

            Ok(AgentResult {
                agent_name: self.name.clone(),
                status: AgentStatus::Completed,
                findings: all_findings,
                analyzed_files,
                modified_files,
                execution_time,
                summary,
                metrics: HashMap::from([
                    (
                        "refactoring_opportunities".to_string(),
                        serde_json::json!(refactoring_opportunities),
                    ),
                    (
                        "files_modified".to_string(),
                        serde_json::json!(files_modified),
                    ),
                    (
                        "risk_level".to_string(),
                        serde_json::json!(format!("{:?}", self.risk_tolerance)),
                    ),
                ]),
            })
        })
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "code-refactoring".to_string(),
            "duplicate-detection".to_string(),
            "pattern-application".to_string(),
            "complexity-reduction".to_string(),
        ]
    }

    fn supports_file_type(&self, file_path: &Path) -> bool {
        let supported = ["rs", "py", "js", "ts", "jsx", "tsx", "go", "java"];
        file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| supported.contains(&ext))
            .unwrap_or(false)
    }

    fn execution_time_estimate(&self) -> Duration {
        Duration::from_secs(90)
    }
}

impl RefactorerAgent {
    fn get_files_from_context(
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
