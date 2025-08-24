//! MCP tool definitions and integration for AGCodex subagents
//!
//! This module provides MCP (Model Context Protocol) tool definitions for each agent,
//! enabling them to be invoked via MCP clients and to call other MCP tools.

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value as JsonValue;
use serde_json::json;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;
use tracing::error;
use tracing::info;

use crate::models::FunctionCallOutputPayload;
use crate::models::ResponseInputItem;
use crate::openai_tools::JsonSchema;
use crate::openai_tools::ResponsesApiTool;
use crate::subagents::AgentContext;
use crate::subagents::AgentInvocation;
use crate::subagents::AgentOrchestrator;
use crate::subagents::SharedContext;
use crate::subagents::SubagentError;
use crate::subagents::SubagentResult;

/// MCP tool descriptor for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpAgentTool {
    /// Tool name (e.g., "agent_code_reviewer")
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// JSON schema for parameters
    pub parameters: JsonSchema,
    /// Agent name this tool maps to
    pub agent_name: String,
    /// Whether this tool requires confirmation
    pub requires_confirmation: bool,
    /// Maximum execution time in seconds
    pub timeout_seconds: Option<u64>,
}

/// MCP tool provider for subagents
#[derive(Debug, Clone)]
pub struct McpAgentToolProvider {
    /// Registry of available agent tools
    tools: Arc<RwLock<HashMap<String, McpAgentTool>>>,
    /// Agent orchestrator for execution
    orchestrator: Arc<AgentOrchestrator>,
}

impl McpAgentToolProvider {
    /// Create a new MCP agent tool provider
    pub fn new(orchestrator: Arc<AgentOrchestrator>) -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            orchestrator,
        }
    }

    /// Register standard agent tools
    pub async fn register_standard_tools(&self) -> SubagentResult<()> {
        let mut tools = self.tools.write().await;

        // Code Reviewer Agent Tool
        tools.insert(
            "agent_code_reviewer".to_string(),
            McpAgentTool {
                name: "agent_code_reviewer".to_string(),
                description: "Review code for quality, security, and maintainability".to_string(),
                parameters: JsonSchema::Object {
                    properties: BTreeMap::from([
                        (
                            "files".to_string(),
                            JsonSchema::Array {
                                items: Box::new(JsonSchema::String { description: None }),
                                description: Some("Files to review".to_string()),
                            },
                        ),
                        (
                            "focus".to_string(),
                            JsonSchema::String {
                                description: Some(
                                    "Review focus area: security, performance, quality, or all"
                                        .to_string(),
                                ),
                            },
                        ),
                    ]),
                    required: Some(vec!["files".to_string()]),
                    additional_properties: Some(false),
                },
                agent_name: "code-reviewer".to_string(),
                requires_confirmation: false,
                timeout_seconds: Some(120),
            },
        );

        // Refactorer Agent Tool
        tools.insert(
            "agent_refactorer".to_string(),
            McpAgentTool {
                name: "agent_refactorer".to_string(),
                description: "Refactor code for better structure and maintainability".to_string(),
                parameters: JsonSchema::Object {
                    properties: BTreeMap::from([
                        (
                            "target".to_string(),
                            JsonSchema::String {
                                description: Some("File or directory to refactor".to_string()),
                            },
                        ),
                        (
                            "pattern".to_string(),
                            JsonSchema::String {
                                description: Some("Refactoring pattern to apply".to_string()),
                            },
                        ),
                        (
                            "dry_run".to_string(),
                            JsonSchema::Boolean {
                                description: Some("Preview changes without applying".to_string()),
                            },
                        ),
                    ]),
                    required: Some(vec!["target".to_string(), "pattern".to_string()]),
                    additional_properties: Some(false),
                },
                agent_name: "refactorer".to_string(),
                requires_confirmation: true,
                timeout_seconds: Some(180),
            },
        );

        // Debugger Agent Tool
        tools.insert(
            "agent_debugger".to_string(),
            McpAgentTool {
                name: "agent_debugger".to_string(),
                description: "Debug code issues and find root causes".to_string(),
                parameters: JsonSchema::Object {
                    properties: BTreeMap::from([
                        (
                            "error".to_string(),
                            JsonSchema::String {
                                description: Some("Error message or stack trace".to_string()),
                            },
                        ),
                        (
                            "context".to_string(),
                            JsonSchema::String {
                                description: Some("Additional context about the issue".to_string()),
                            },
                        ),
                        (
                            "files".to_string(),
                            JsonSchema::Array {
                                items: Box::new(JsonSchema::String { description: None }),
                                description: Some("Related files to analyze".to_string()),
                            },
                        ),
                    ]),
                    required: Some(vec!["error".to_string()]),
                    additional_properties: Some(false),
                },
                agent_name: "debugger".to_string(),
                requires_confirmation: false,
                timeout_seconds: Some(150),
            },
        );

        // Test Writer Agent Tool
        tools.insert(
            "agent_test_writer".to_string(),
            McpAgentTool {
                name: "agent_test_writer".to_string(),
                description: "Generate comprehensive test suites".to_string(),
                parameters: JsonSchema::Object {
                    properties: BTreeMap::from([
                        (
                            "target".to_string(),
                            JsonSchema::String {
                                description: Some("File or module to test".to_string()),
                            },
                        ),
                        (
                            "test_type".to_string(),
                            JsonSchema::String {
                                description: Some(
                                    "Type of tests to generate: unit, integration, e2e, or all"
                                        .to_string(),
                                ),
                            },
                        ),
                        (
                            "framework".to_string(),
                            JsonSchema::String {
                                description: Some("Testing framework to use".to_string()),
                            },
                        ),
                    ]),
                    required: Some(vec!["target".to_string()]),
                    additional_properties: Some(false),
                },
                agent_name: "test-writer".to_string(),
                requires_confirmation: true,
                timeout_seconds: Some(120),
            },
        );

        // Performance Agent Tool
        tools.insert(
            "agent_performance".to_string(),
            McpAgentTool {
                name: "agent_performance".to_string(),
                description: "Analyze and optimize performance bottlenecks".to_string(),
                parameters: JsonSchema::Object {
                    properties: BTreeMap::from([
                        (
                            "target".to_string(),
                            JsonSchema::String {
                                description: Some("Code to analyze for performance".to_string()),
                            },
                        ),
                        (
                            "profile_data".to_string(),
                            JsonSchema::String {
                                description: Some("Optional profiling data".to_string()),
                            },
                        ),
                        (
                            "optimization_level".to_string(),
                            JsonSchema::String {
                                description: Some("How aggressive to be with optimizations: basic, aggressive, or extreme".to_string()),
                            },
                        ),
                    ]),
                    required: Some(vec!["target".to_string()]),
                    additional_properties: Some(false),
                },
                agent_name: "performance".to_string(),
                requires_confirmation: true,
                timeout_seconds: Some(240),
            },
        );

        // Security Agent Tool
        tools.insert(
            "agent_security".to_string(),
            McpAgentTool {
                name: "agent_security".to_string(),
                description: "Scan for security vulnerabilities and suggest fixes".to_string(),
                parameters: JsonSchema::Object {
                    properties: BTreeMap::from([
                        (
                            "target".to_string(),
                            JsonSchema::String {
                                description: Some("Code to scan for vulnerabilities".to_string()),
                            },
                        ),
                        (
                            "scan_type".to_string(),
                            JsonSchema::String {
                                description: Some(
                                    "Type of security scan: owasp, cve, secrets, or all"
                                        .to_string(),
                                ),
                            },
                        ),
                        (
                            "fix_suggestions".to_string(),
                            JsonSchema::Boolean {
                                description: Some("Generate fix suggestions".to_string()),
                            },
                        ),
                    ]),
                    required: Some(vec!["target".to_string()]),
                    additional_properties: Some(false),
                },
                agent_name: "security".to_string(),
                requires_confirmation: false,
                timeout_seconds: Some(180),
            },
        );

        // Documentation Agent Tool
        tools.insert(
            "agent_docs".to_string(),
            McpAgentTool {
                name: "agent_docs".to_string(),
                description: "Generate comprehensive documentation".to_string(),
                parameters: JsonSchema::Object {
                    properties: BTreeMap::from([
                        (
                            "target".to_string(),
                            JsonSchema::String {
                                description: Some("Code to document".to_string()),
                            },
                        ),
                        (
                            "doc_type".to_string(),
                            JsonSchema::String {
                                description: Some(
                                    "Type of documentation: api, tutorial, reference, or all"
                                        .to_string(),
                                ),
                            },
                        ),
                        (
                            "format".to_string(),
                            JsonSchema::String {
                                description: Some(
                                    "Output format: markdown, html, or rst".to_string(),
                                ),
                            },
                        ),
                    ]),
                    required: Some(vec!["target".to_string()]),
                    additional_properties: Some(false),
                },
                agent_name: "docs".to_string(),
                requires_confirmation: false,
                timeout_seconds: Some(90),
            },
        );

        // Agent Chain Tool (for sequential execution)
        tools.insert(
            "agent_chain".to_string(),
            McpAgentTool {
                name: "agent_chain".to_string(),
                description: "Execute multiple agents in sequence".to_string(),
                parameters: JsonSchema::Object {
                    properties: BTreeMap::from([
                        (
                            "agents".to_string(),
                            JsonSchema::Array {
                                items: Box::new(JsonSchema::String { description: None }),
                                description: Some("Agent names to execute in order".to_string()),
                            },
                        ),
                        (
                            "context".to_string(),
                            JsonSchema::Object {
                                properties: BTreeMap::new(),
                                required: None,
                                additional_properties: Some(true),
                            },
                        ),
                        (
                            "stop_on_error".to_string(),
                            JsonSchema::Boolean {
                                description: Some("Stop chain if an agent fails".to_string()),
                            },
                        ),
                    ]),
                    required: Some(vec!["agents".to_string()]),
                    additional_properties: Some(false),
                },
                agent_name: "_chain".to_string(), // Special internal agent
                requires_confirmation: true,
                timeout_seconds: Some(600),
            },
        );

        // Agent Parallel Tool (for parallel execution)
        tools.insert(
            "agent_parallel".to_string(),
            McpAgentTool {
                name: "agent_parallel".to_string(),
                description: "Execute multiple agents in parallel".to_string(),
                parameters: JsonSchema::Object {
                    properties: BTreeMap::from([
                        (
                            "agents".to_string(),
                            JsonSchema::Array {
                                items: Box::new(JsonSchema::String { description: None }),
                                description: Some("Agent names to execute in parallel".to_string()),
                            },
                        ),
                        (
                            "context".to_string(),
                            JsonSchema::Object {
                                properties: BTreeMap::new(),
                                required: None,
                                additional_properties: Some(true),
                            },
                        ),
                        (
                            "max_concurrency".to_string(),
                            JsonSchema::Number {
                                description: Some("Maximum agents to run concurrently".to_string()),
                            },
                        ),
                    ]),
                    required: Some(vec!["agents".to_string()]),
                    additional_properties: Some(false),
                },
                agent_name: "_parallel".to_string(), // Special internal agent
                requires_confirmation: true,
                timeout_seconds: Some(600),
            },
        );

        Ok(())
    }

    /// Get all available MCP tools for agents
    pub async fn get_tools(&self) -> Vec<ResponsesApiTool> {
        let tools = self.tools.read().await;
        tools
            .values()
            .map(|tool| ResponsesApiTool {
                name: tool.name.clone(),
                description: tool.description.clone(),
                strict: false,
                parameters: tool.parameters.clone(),
            })
            .collect()
    }

    /// Discover tools available from connected MCP servers
    pub async fn discover_mcp_tools(&self, _server_name: &str) -> SubagentResult<Vec<String>> {
        // This would query the MCP server for available tools
        // For now, return a placeholder list
        Ok(vec![
            "read_file".to_string(),
            "write_file".to_string(),
            "run_command".to_string(),
            "search_code".to_string(),
        ])
    }

    /// Execute an MCP tool call for an agent
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        arguments: JsonValue,
        context: &AgentContext,
    ) -> SubagentResult<JsonValue> {
        let tools = self.tools.read().await;

        let tool = tools
            .get(tool_name)
            .ok_or_else(|| SubagentError::AgentNotFound {
                name: tool_name.to_string(),
            })?;

        info!(
            "Executing MCP tool '{}' for agent '{}'",
            tool_name, tool.agent_name
        );

        // Special handling for chain and parallel tools
        if tool.agent_name == "_chain" {
            return self.execute_chain(arguments, context).await;
        } else if tool.agent_name == "_parallel" {
            return self.execute_parallel(arguments, context).await;
        }

        // Execute the agent
        let invocation = AgentInvocation {
            agent_name: tool.agent_name.clone(),
            parameters: self.json_to_params(arguments)?,
            raw_parameters: String::new(),
            position: 0,
            mode_override: None,
            intelligence_override: None,
        };

        // Create a SharedContext for the orchestrator
        let shared_context = SharedContext::new();
        let result = self
            .orchestrator
            .execute_single(invocation, &shared_context)
            .await?;

        // Convert result to JSON
        Ok(json!({
            "success": true,
            "agent": tool.agent_name,
            "output": result.output,
            "modified_files": result.modified_files,
            "duration_ms": result.duration().map(|d| d.as_millis()),
        }))
    }

    /// Execute agents in sequence
    async fn execute_chain(
        &self,
        arguments: JsonValue,
        context: &AgentContext,
    ) -> SubagentResult<JsonValue> {
        let agents = arguments["agents"]
            .as_array()
            .ok_or_else(|| SubagentError::InvalidConfig("agents must be an array".to_string()))?;

        let stop_on_error = arguments["stop_on_error"].as_bool().unwrap_or(true);

        let mut results = Vec::new();

        for agent_name in agents {
            let agent_name = agent_name.as_str().ok_or_else(|| {
                SubagentError::InvalidConfig("agent name must be a string".to_string())
            })?;

            let invocation = AgentInvocation {
                agent_name: agent_name.to_string(),
                parameters: HashMap::new(),
                raw_parameters: String::new(),
                position: 0,
                mode_override: None,
                intelligence_override: None,
            };

            // Create a SharedContext for the orchestrator
            let shared_context = SharedContext::new();
            match self
                .orchestrator
                .execute_single(invocation, &shared_context)
                .await
            {
                Ok(result) => {
                    // Update context with results for next agent
                    context
                        .send_message(crate::subagents::context::AgentMessage {
                            id: uuid::Uuid::new_v4(),
                            from: agent_name.to_string(),
                            to: crate::subagents::context::MessageTarget::Broadcast,
                            message_type: crate::subagents::context::MessageType::Result,
                            priority: crate::subagents::context::MessagePriority::Normal,
                            payload: serde_json::json!({
                                "output": result.output.clone().unwrap_or_default()
                            }),
                            timestamp: chrono::Utc::now(),
                        })
                        .await
                        .ok();
                    results.push(json!({
                        "agent": agent_name,
                        "success": true,
                        "output": result.output,
                    }));
                }
                Err(e) => {
                    results.push(json!({
                        "agent": agent_name,
                        "success": false,
                        "error": e.to_string(),
                    }));
                    if stop_on_error {
                        break;
                    }
                }
            }
        }

        Ok(json!({
            "chain_results": results,
            "completed": results.len() == agents.len(),
        }))
    }

    /// Execute agents in parallel
    async fn execute_parallel(
        &self,
        arguments: JsonValue,
        _context: &AgentContext,
    ) -> SubagentResult<JsonValue> {
        let agents = arguments["agents"]
            .as_array()
            .ok_or_else(|| SubagentError::InvalidConfig("agents must be an array".to_string()))?;

        let max_concurrency = arguments["max_concurrency"]
            .as_u64()
            .unwrap_or(4)
            .min(agents.len() as u64) as usize;

        let mut handles = Vec::new();
        let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrency));

        for agent_name in agents {
            let agent_name = agent_name
                .as_str()
                .ok_or_else(|| {
                    SubagentError::InvalidConfig("agent name must be a string".to_string())
                })?
                .to_string();

            let orchestrator = self.orchestrator.clone();
            let permit = semaphore.clone().acquire_owned().await
                .map_err(|e| SubagentError::ExecutionFailed(format!("Semaphore error: {}", e)))?;

            let handle = tokio::spawn(async move {
                let invocation = AgentInvocation {
                    agent_name: agent_name.clone(),
                    parameters: HashMap::new(),
                    raw_parameters: String::new(),
                    position: 0,
                    mode_override: None,
                    intelligence_override: None,
                };

                // Create a SharedContext for the orchestrator
                let shared_context = SharedContext::new();
                let result = orchestrator
                    .execute_single(invocation, &shared_context)
                    .await;
                drop(permit); // Release semaphore

                match result {
                    Ok(execution) => json!({
                        "agent": agent_name,
                        "success": true,
                        "output": execution.output,
                        "duration_ms": execution.duration().map(|d| d.as_millis()),
                    }),
                    Err(e) => json!({
                        "agent": agent_name,
                        "success": false,
                        "error": e.to_string(),
                    }),
                }
            });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    error!("Failed to join agent task: {}", e);
                    results.push(json!({
                        "error": "Task join error",
                    }));
                }
            }
        }

        Ok(json!({
            "parallel_results": results,
            "total_agents": agents.len(),
        }))
    }

    /// Convert JSON arguments to parameter map
    fn json_to_params(&self, json: JsonValue) -> SubagentResult<HashMap<String, String>> {
        let obj = json.as_object().ok_or_else(|| {
            SubagentError::InvalidConfig("arguments must be an object".to_string())
        })?;

        let mut params = HashMap::new();
        for (key, value) in obj {
            let string_value = match value {
                JsonValue::String(s) => s.clone(),
                JsonValue::Number(n) => n.to_string(),
                JsonValue::Bool(b) => b.to_string(),
                JsonValue::Array(_) | JsonValue::Object(_) => serde_json::to_string(value)
                    .map_err(|e| SubagentError::InvalidConfig(e.to_string()))?,
                JsonValue::Null => String::new(),
            };
            params.insert(key.clone(), string_value);
        }

        Ok(params)
    }

    /// Stream results back via MCP protocol
    pub async fn stream_results(
        &self,
        call_id: String,
        agent_name: String,
        output: String,
    ) -> ResponseInputItem {
        ResponseInputItem::FunctionCallOutput {
            call_id,
            output: FunctionCallOutputPayload {
                content: json!({
                    "agent": agent_name,
                    "output": output,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                })
                .to_string(),
                success: Some(true),
            },
        }
    }
}

/// MCP tool call handler for agents
pub struct McpAgentHandler {
    provider: Arc<McpAgentToolProvider>,
}

impl McpAgentHandler {
    pub const fn new(provider: Arc<McpAgentToolProvider>) -> Self {
        Self { provider }
    }

    /// Handle an incoming MCP tool call for an agent
    pub async fn handle_tool_call(
        &self,
        tool_name: &str,
        arguments: JsonValue,
        context: AgentContext,
    ) -> SubagentResult<JsonValue> {
        debug!("Handling MCP tool call: {}", tool_name);
        self.provider
            .execute_tool(tool_name, arguments, &context)
            .await
    }

    /// Register this handler with an MCP server
    pub async fn register_with_server(&self, server_name: &str) -> SubagentResult<()> {
        info!("Registering agent tools with MCP server: {}", server_name);

        // Get all tools
        let tools = self.provider.get_tools().await;

        // Here we would send tool definitions to the MCP server
        // This is a placeholder for the actual MCP protocol implementation
        for tool in tools {
            debug!("Registered tool: {} with server {}", tool.name, server_name);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subagents::OrchestratorConfig;

    #[tokio::test]
    async fn test_mcp_tool_registration() {
        let registry = Arc::new(crate::subagents::registry::SubagentRegistry::new().unwrap());
        let config = OrchestratorConfig::default();
        let orchestrator = Arc::new(AgentOrchestrator::new(
            registry,
            config,
            crate::modes::OperatingMode::Build,
        ));
        let provider = McpAgentToolProvider::new(orchestrator);

        provider.register_standard_tools().await.unwrap();
        let tools = provider.get_tools().await;

        assert!(!tools.is_empty());
        assert!(tools.iter().any(|t| t.name == "agent_code_reviewer"));
        assert!(tools.iter().any(|t| t.name == "agent_chain"));
    }

    #[tokio::test]
    async fn test_json_to_params_conversion() {
        let registry = Arc::new(crate::subagents::registry::SubagentRegistry::new().unwrap());
        let config = OrchestratorConfig::default();
        let orchestrator = Arc::new(AgentOrchestrator::new(
            registry,
            config,
            crate::modes::OperatingMode::Build,
        ));
        let provider = McpAgentToolProvider::new(orchestrator);

        let json = json!({
            "file": "main.rs",
            "line": 42,
            "enabled": true,
            "tags": ["rust", "async"],
        });

        let params = provider.json_to_params(json).unwrap();
        assert_eq!(params.get("file").unwrap(), "main.rs");
        assert_eq!(params.get("line").unwrap(), "42");
        assert_eq!(params.get("enabled").unwrap(), "true");
        assert!(params.get("tags").unwrap().contains("rust"));
    }
}
