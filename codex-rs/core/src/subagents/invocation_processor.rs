//! Subagent invocation processing for conversation flow
//!
//! This module hooks into the conversation manager to intercept messages
//! with @agent-name patterns and execute them with isolated contexts.

use super::agents::{AgentResult, Severity};
use super::invocation::{
    AgentInvocation, ExecutionPlan, ExecutionStep, InvocationParser, InvocationRequest,
};
use super::registry::SubagentRegistry;
use super::{SubagentContext, SubagentError, SubagentResult};
use crate::code_tools::ast_agent_tools::ASTAgentTools;
use crate::modes::OperatingMode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, warn};

/// Maximum time to wait for any single agent execution
const DEFAULT_AGENT_TIMEOUT: Duration = Duration::from_secs(120);

/// Maximum parallel agents to execute simultaneously
const MAX_PARALLEL_AGENTS: usize = 8;

/// Processor for handling @agent-name invocations in conversations
pub struct InvocationProcessor {
    /// Parser for detecting and parsing invocations
    parser: InvocationParser,
    
    /// Registry of available agents
    registry: Arc<SubagentRegistry>,
    
    /// AST tools for agent operations
    ast_tools: Arc<tokio::sync::Mutex<ASTAgentTools>>,
    
    /// Default operating mode
    default_mode: OperatingMode,
}

impl InvocationProcessor {
    /// Create a new invocation processor
    pub fn new(
        registry: Arc<SubagentRegistry>,
        default_mode: OperatingMode,
    ) -> SubagentResult<Self> {
        let parser = InvocationParser::with_registry(registry.clone())?;
        let ast_tools = Arc::new(tokio::sync::Mutex::new(ASTAgentTools::new()));
        
        Ok(Self {
            parser,
            registry,
            ast_tools,
            default_mode,
        })
    }
    
    /// Process a user message for agent invocations
    /// Returns None if no invocations found, or a modified message with agent results
    pub async fn process_message(&self, message: &str) -> SubagentResult<Option<String>> {
        // Parse for invocations
        let invocation_request = match self.parser.parse(message)? {
            Some(req) => req,
            None => return Ok(None), // No agents to invoke
        };
        
        info!(
            "Processing invocation request {} with plan: {:?}",
            invocation_request.id, invocation_request.execution_plan
        );
        
        // Execute the plan
        let results = self.execute_plan(&invocation_request).await?;
        
        // Merge results into response
        let merged_response = self.merge_results(&invocation_request, results)?;
        
        Ok(Some(merged_response))
    }
    
    /// Execute an invocation plan
    async fn execute_plan(
        &self,
        request: &InvocationRequest,
    ) -> SubagentResult<Vec<AgentResult>> {
        match &request.execution_plan {
            ExecutionPlan::Single(invocation) => {
                let result = self.execute_single_agent(invocation, &request.context).await?;
                Ok(vec![result])
            }
            
            ExecutionPlan::Sequential(chain) => {
                let mut results = Vec::new();
                let mut previous_output = None;
                
                for invocation in &chain.agents {
                    let mut context = request.context.clone();
                    
                    // Pass output from previous agent if configured
                    if chain.pass_output {
                        if let Some(ref prev) = previous_output {
                            context = format!("{}\n\nPrevious agent output:\n{}", context, prev);
                        }
                    }
                    
                    let result = self.execute_single_agent(invocation, &context).await?;
                    previous_output = Some(result.summary.clone());
                    results.push(result);
                }
                
                Ok(results)
            }
            
            ExecutionPlan::Parallel(invocations) => {
                self.execute_parallel_agents(invocations, &request.context).await
            }
            
            ExecutionPlan::Conditional(cond_exec) => {
                // For now, execute conditionally based on simple checks
                // TODO: Implement proper condition evaluation
                warn!("Conditional execution not fully implemented, executing all agents");
                self.execute_parallel_agents(&cond_exec.agents, &request.context).await
            }
            
            ExecutionPlan::Mixed(steps) => {
                self.execute_mixed_plan(steps, &request.context).await
            }
        }
    }
    
    /// Execute a single agent
    async fn execute_single_agent(
        &self,
        invocation: &AgentInvocation,
        context_str: &str,
    ) -> SubagentResult<AgentResult> {
        info!("Executing agent: {}", invocation.agent_name);
        
        // Get agent from registry
        let agent = self
            .registry
            .get_executable_agent(&invocation.agent_name)
            .ok_or_else(|| SubagentError::AgentNotFound {
                name: invocation.agent_name.clone(),
            })?;
        
        // Create isolated context
        let mode = invocation.mode_override.unwrap_or(self.default_mode);
        let mut parameters = invocation.parameters.clone();
        parameters.insert("context".to_string(), context_str.to_string());
        
        let context = SubagentContext::new(mode, parameters);
        
        // Set up cancellation
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let cancel_flag_clone = cancel_flag.clone();
        
        // Execute with timeout
        let mut ast_tools = self.ast_tools.lock().await;
        let start_time = Instant::now();
        
        let execution_future = agent.execute(&context, &mut *ast_tools, cancel_flag_clone);
        
        match timeout(DEFAULT_AGENT_TIMEOUT, execution_future).await {
            Ok(Ok(result)) => {
                info!(
                    "Agent {} completed in {:?}",
                    invocation.agent_name,
                    start_time.elapsed()
                );
                Ok(result)
            }
            Ok(Err(e)) => {
                error!("Agent {} failed: {}", invocation.agent_name, e);
                Err(e)
            }
            Err(_) => {
                cancel_flag.store(true, Ordering::Relaxed);
                error!(
                    "Agent {} timed out after {:?}",
                    invocation.agent_name, DEFAULT_AGENT_TIMEOUT
                );
                Err(SubagentError::Timeout {
                    agent: invocation.agent_name.clone(),
                    duration: DEFAULT_AGENT_TIMEOUT,
                })
            }
        }
    }
    
    /// Execute agents in parallel
    async fn execute_parallel_agents(
        &self,
        invocations: &[AgentInvocation],
        context_str: &str,
    ) -> SubagentResult<Vec<AgentResult>> {
        info!("Executing {} agents in parallel", invocations.len());
        
        // Limit parallelism
        let chunks = invocations.chunks(MAX_PARALLEL_AGENTS);
        let mut all_results = Vec::new();
        
        for chunk in chunks {
            let mut handles = Vec::new();
            
            for invocation in chunk {
                let invocation = invocation.clone();
                let context_str = context_str.to_string();
                let registry = self.registry.clone();
                let ast_tools = self.ast_tools.clone();
                let default_mode = self.default_mode;
                
                let handle = tokio::spawn(async move {
                    Self::execute_agent_task(
                        invocation,
                        context_str,
                        registry,
                        ast_tools,
                        default_mode,
                    )
                    .await
                });
                
                handles.push(handle);
            }
            
            // Collect results from this chunk
            for handle in handles {
                match handle.await {
                    Ok(Ok(result)) => all_results.push(result),
                    Ok(Err(e)) => {
                        warn!("Agent execution failed: {}", e);
                        // Continue with other agents
                    }
                    Err(e) => {
                        error!("Task join error: {}", e);
                        // Continue with other agents
                    }
                }
            }
        }
        
        Ok(all_results)
    }
    
    /// Execute a mixed plan with barriers
    async fn execute_mixed_plan(
        &self,
        steps: &[ExecutionStep],
        context_str: &str,
    ) -> SubagentResult<Vec<AgentResult>> {
        let mut all_results = Vec::new();
        let mut current_context = context_str.to_string();
        
        for step in steps {
            match step {
                ExecutionStep::Single(invocation) => {
                    let result = self.execute_single_agent(invocation, &current_context).await?;
                    current_context = format!(
                        "{}\n\n{} output:\n{}",
                        current_context, invocation.agent_name, result.summary
                    );
                    all_results.push(result);
                }
                
                ExecutionStep::Parallel(invocations) => {
                    let results = self
                        .execute_parallel_agents(invocations, &current_context)
                        .await?;
                    
                    // Update context with all parallel results
                    for result in &results {
                        current_context = format!(
                            "{}\n\n{} output:\n{}",
                            current_context, result.agent_name, result.summary
                        );
                    }
                    
                    all_results.extend(results);
                }
                
                ExecutionStep::Conditional(cond_exec) => {
                    // TODO: Implement proper condition evaluation
                    warn!("Conditional execution in mixed plan not fully implemented");
                    let results = self
                        .execute_parallel_agents(&cond_exec.agents, &current_context)
                        .await?;
                    all_results.extend(results);
                }
                
                ExecutionStep::Barrier => {
                    debug!("Execution barrier - all previous steps completed");
                    // Barrier doesn't need action, just ensures sequential ordering
                }
            }
        }
        
        Ok(all_results)
    }
    
    /// Helper function for spawned agent tasks
    async fn execute_agent_task(
        invocation: AgentInvocation,
        context_str: String,
        registry: Arc<SubagentRegistry>,
        ast_tools: Arc<tokio::sync::Mutex<ASTAgentTools>>,
        default_mode: OperatingMode,
    ) -> SubagentResult<AgentResult> {
        // Get agent from registry
        let agent = registry
            .get_executable_agent(&invocation.agent_name)
            .ok_or_else(|| SubagentError::AgentNotFound {
                name: invocation.agent_name.clone(),
            })?;
        
        // Create isolated context
        let mode = invocation.mode_override.unwrap_or(default_mode);
        let mut parameters = invocation.parameters.clone();
        parameters.insert("context".to_string(), context_str);
        
        let context = SubagentContext::new(mode, parameters);
        
        // Set up cancellation
        let cancel_flag = Arc::new(AtomicBool::new(false));
        
        // Execute with timeout
        let mut ast_tools = ast_tools.lock().await;
        let start_time = Instant::now();
        
        let execution_future = agent.execute(&context, &mut *ast_tools, cancel_flag.clone());
        
        match timeout(DEFAULT_AGENT_TIMEOUT, execution_future).await {
            Ok(Ok(result)) => {
                info!(
                    "Agent {} completed in {:?}",
                    invocation.agent_name,
                    start_time.elapsed()
                );
                Ok(result)
            }
            Ok(Err(e)) => {
                error!("Agent {} failed: {}", invocation.agent_name, e);
                Err(e)
            }
            Err(_) => {
                cancel_flag.store(true, Ordering::Relaxed);
                error!(
                    "Agent {} timed out after {:?}",
                    invocation.agent_name, DEFAULT_AGENT_TIMEOUT
                );
                Err(SubagentError::Timeout {
                    agent: invocation.agent_name.clone(),
                    duration: DEFAULT_AGENT_TIMEOUT,
                })
            }
        }
    }
    
    /// Merge agent results into a formatted response
    fn merge_results(
        &self,
        request: &InvocationRequest,
        results: Vec<AgentResult>,
    ) -> SubagentResult<String> {
        let mut response = String::new();
        
        // Add original context if significant
        if !request.context.is_empty() && request.context.len() > 20 {
            response.push_str(&format!("Context: {}\n\n", request.context));
        }
        
        // Add agent results
        response.push_str("## Agent Analysis Results\n\n");
        
        for result in results {
            response.push_str(&format!("### {} Agent\n", result.agent_name));
            response.push_str(&format!("Status: {:?}\n", result.status));
            response.push_str(&format!("Execution time: {:?}\n\n", result.execution_time));
            
            // Add summary
            if !result.summary.is_empty() {
                response.push_str("**Summary:**\n");
                response.push_str(&result.summary);
                response.push_str("\n\n");
            }
            
            // Add key findings
            if !result.findings.is_empty() {
                response.push_str("**Key Findings:**\n");
                for finding in result.findings.iter().take(5) {
                    // Only show top 5 findings
                    response.push_str(&format!(
                        "- **{}** ({}): {}\n",
                        finding.title,
                        format_severity(finding.severity),
                        finding.description
                    ));
                    
                    if let Some(ref suggestion) = finding.suggestion {
                        response.push_str(&format!("  → Suggestion: {}\n", suggestion));
                    }
                }
                
                if result.findings.len() > 5 {
                    response.push_str(&format!(
                        "\n*...and {} more findings*\n",
                        result.findings.len() - 5
                    ));
                }
                response.push_str("\n");
            }
            
            // Add metrics if present
            if !result.metrics.is_empty() {
                response.push_str("**Metrics:**\n");
                for (key, value) in &result.metrics {
                    response.push_str(&format!("- {}: {}\n", key, value));
                }
                response.push_str("\n");
            }
            
            // Add file information
            if !result.analyzed_files.is_empty() {
                response.push_str(&format!(
                    "Analyzed {} files\n",
                    result.analyzed_files.len()
                ));
            }
            
            if !result.modified_files.is_empty() {
                response.push_str(&format!(
                    "Modified {} files: {:?}\n",
                    result.modified_files.len(),
                    result.modified_files
                ));
            }
            
            response.push_str("\n---\n\n");
        }
        
        Ok(response)
    }
}

/// Format severity for display
fn format_severity(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "🔴 Critical",
        Severity::High => "🟠 High",
        Severity::Medium => "🟡 Medium",
        Severity::Low => "🔵 Low",
        Severity::Info => "ℹ️ Info",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_no_invocations() {
        let registry = Arc::new(SubagentRegistry::new().unwrap());
        let processor = InvocationProcessor::new(registry, OperatingMode::Build).unwrap();
        
        let result = processor
            .process_message("This is a normal message without agents")
            .await
            .unwrap();
        
        assert!(result.is_none());
    }
    
    #[tokio::test]
    async fn test_single_agent_invocation() {
        let registry = Arc::new(SubagentRegistry::new().unwrap());
        let processor = InvocationProcessor::new(registry, OperatingMode::Build).unwrap();
        
        // This will fail because no agents are registered, but it tests the parsing
        let message = "@code-reviewer check this function for issues";
        let result = processor.process_message(message).await;
        
        // Should fail with AgentNotFound
        assert!(matches!(
            result,
            Err(SubagentError::AgentNotFound { .. })
        ));
    }
}