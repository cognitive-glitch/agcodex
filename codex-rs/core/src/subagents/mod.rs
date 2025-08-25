//! Subagent system for AGCodex
//!
//! The subagent system enables specialized AI assistants for task-specific workflows.
//! Each subagent operates with its own context, custom prompts, and tool permissions.
//!
//! ## Key Features
//! - **Specialized Agents**: Code review, refactoring, debugging, testing, etc.
//! - **Mode Override**: Agents can override operating mode (Plan/Build/Review)
//! - **Template System**: Reusable agent configurations
//! - **Hot Reload**: Dynamic loading of agent configurations
//! - **Chaining & Parallel**: Sequential (→) or parallel (+) execution
//!
//! ## Usage
//! ```
//! @agent-code-reviewer - Review code for quality and security
//! @agent-refactorer → @agent-test-writer - Chain agents sequentially
//! @agent-performance + @agent-security - Run agents in parallel
//! ```
//!
//! ## Configuration
//! Agents are configured via TOML files in:
//! - `~/.agcodex/agents/global/` - Available everywhere
//! - `./.agcodex/agents/` - Project-specific agents
//! - `templates/` - Reusable templates

pub mod agents;
pub mod built_in;
pub mod config;
pub mod context;
pub mod invocation;
pub mod manager;
pub mod mcp_tools;
pub mod orchestrator;
pub mod parser;
pub mod registry;
pub mod worktree;
pub mod yaml_loader;

// Re-export main types for convenience
pub use agents::AgentRegistry;
pub use agents::AgentResult;
pub use agents::AgentStatus;
pub use agents::CodeReviewerAgent;
pub use agents::DebuggerAgent;
pub use agents::DocsAgent;
pub use agents::Finding;
pub use agents::PerformanceAgent;
pub use agents::RefactorerAgent;
pub use agents::SecurityAgent;
pub use agents::Severity;
pub use agents::Subagent;
pub use agents::TestWriterAgent;
pub use built_in::create_default_registry;
pub use built_in::register_built_in_agents;
pub use config::IntelligenceLevel;
pub use config::SubagentConfig;
pub use config::ToolPermission;
pub use context::AgentContext;
pub use context::AgentContextSnapshot;
pub use context::AgentMessage;
pub use context::CancellationToken;
pub use context::ContextError;
pub use context::ContextFinding;
pub use context::ContextResult;
pub use context::ExecutionMetricsSnapshot;
pub use context::FindingSeverity;
pub use context::MessagePriority;
pub use context::MessageReceiver;
pub use context::MessageTarget;
pub use context::MessageType;
pub use context::ProgressEvent;
pub use context::ProgressInfo;
pub use context::ProgressStage;
pub use context::ProgressTracker;
pub use invocation::AgentChain;
pub use invocation::AgentInvocation;
pub use invocation::ExecutionPlan;
pub use invocation::ExecutionStep;
pub use invocation::InvocationParser;
pub use invocation::InvocationRequest;
pub use manager::AgentHandle;
pub use manager::AgentManager;
pub use manager::AgentStats;
pub use manager::MessageBus;
pub use mcp_tools::McpAgentHandler;
pub use mcp_tools::McpAgentTool;
pub use mcp_tools::McpAgentToolProvider;
pub use orchestrator::AgentOrchestrator;
pub use orchestrator::ContextSnapshot;
pub use orchestrator::OrchestratorConfig;
pub use orchestrator::OrchestratorResult;
pub use orchestrator::ProgressUpdate;
pub use orchestrator::SharedContext;
pub use parser::AgentParser;
pub use parser::ChainOperator;
pub use parser::ParsedInvocation;
pub use registry::SubagentRegistry;
pub use registry::SubagentRegistryError;
pub use worktree::AgentWorktree;
pub use worktree::ConflictStrategy;
pub use worktree::MergeResult;
pub use worktree::WorktreeManager;
pub use worktree::WorktreePool;

use crate::modes::OperatingMode;
use std::collections::HashMap;
use uuid::Uuid;

/// Result type for subagent operations
pub type SubagentResult<T> = std::result::Result<T, SubagentError>;

/// Errors that can occur in the subagent system
#[derive(thiserror::Error, Debug)]
pub enum SubagentError {
    #[error("agent not found: {name}")]
    AgentNotFound { name: String },

    #[error("invalid agent configuration: {0}")]
    InvalidConfig(String),

    #[error("agent execution failed: {0}")]
    ExecutionFailed(String),

    #[error("circular dependency detected in agent chain: {chain:?}")]
    CircularDependency { chain: Vec<String> },

    #[error("agent timeout: {name}")]
    Timeout { name: String },

    #[error("tool permission denied: {tool} for agent {agent}")]
    ToolPermissionDenied { tool: String, agent: String },

    #[error("mode restriction violation: {mode:?} does not allow {operation}")]
    ModeRestriction {
        mode: OperatingMode,
        operation: String,
    },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] toml::de::Error),

    #[error("tool error: {0}")]
    Tool(#[from] crate::code_tools::ToolError),

    #[error("file watching error: {0}")]
    FileWatcher(String),

    #[error("agent template not found: {name}")]
    TemplateNotFound { name: String },

    #[error("regex compilation error: {0}")]
    RegexError(String),

    #[error("missing field in parsing: {field}")]
    MissingField { field: String },

    #[error("path conversion error: {path}")]
    PathConversion { path: String },

    #[error("parse error: {0}")]
    ParseError(String),

    #[error("poison error: {0}")]
    PoisonError(String),

    #[error("join error: {0}")]
    JoinError(String),
}

/// Context passed to subagents during execution
#[derive(Debug, Clone)]
pub struct SubagentContext {
    /// Unique identifier for this execution
    pub execution_id: Uuid,

    /// Current operating mode
    pub mode: OperatingMode,

    /// Available tools for this agent
    pub available_tools: Vec<String>,

    /// Conversation history (limited)
    pub conversation_context: String,

    /// Current working directory
    pub working_directory: std::path::PathBuf,

    /// User-provided parameters
    pub parameters: HashMap<String, String>,

    /// Agent-specific metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Status of a subagent execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubagentStatus {
    /// Agent is waiting to start
    Pending,
    /// Agent is currently running
    Running,
    /// Agent completed successfully
    Completed,
    /// Agent failed with an error
    Failed(String),
    /// Agent was cancelled
    Cancelled,
}

/// Result of a subagent execution
#[derive(Debug, Clone)]
pub struct SubagentExecution {
    /// Unique identifier for this execution
    pub id: Uuid,

    /// Name of the agent that was executed
    pub agent_name: String,

    /// Current status
    pub status: SubagentStatus,

    /// Agent output/response
    pub output: Option<String>,

    /// Files modified by the agent
    pub modified_files: Vec<std::path::PathBuf>,

    /// Execution start time
    pub started_at: std::time::SystemTime,

    /// Execution end time (if completed)
    pub completed_at: Option<std::time::SystemTime>,

    /// Error information (if failed)
    pub error: Option<String>,
}

impl SubagentExecution {
    /// Create a new pending execution
    pub fn new(agent_name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            agent_name,
            status: SubagentStatus::Pending,
            output: None,
            modified_files: Vec::new(),
            started_at: std::time::SystemTime::now(),
            completed_at: None,
            error: None,
        }
    }

    /// Mark the execution as started
    pub fn start(&mut self) {
        self.status = SubagentStatus::Running;
        self.started_at = std::time::SystemTime::now();
    }

    /// Mark the execution as completed with output
    pub fn complete(&mut self, output: String, modified_files: Vec<std::path::PathBuf>) {
        self.status = SubagentStatus::Completed;
        self.output = Some(output);
        self.modified_files = modified_files;
        self.completed_at = Some(std::time::SystemTime::now());
    }

    /// Mark the execution as failed with an error
    pub fn fail(&mut self, error: String) {
        self.status = SubagentStatus::Failed(error.clone());
        self.error = Some(error);
        self.completed_at = Some(std::time::SystemTime::now());
    }

    /// Get the execution duration
    pub fn duration(&self) -> Option<std::time::Duration> {
        self.completed_at
            .and_then(|end| end.duration_since(self.started_at).ok())
    }
}

#[cfg(test)]
mod basic_tests {
    use super::*;

    #[test]
    fn test_subagent_execution_lifecycle() {
        let mut execution = SubagentExecution::new("test-agent".to_string());

        assert_eq!(execution.status, SubagentStatus::Pending);
        assert_eq!(execution.agent_name, "test-agent");

        execution.start();
        assert_eq!(execution.status, SubagentStatus::Running);

        execution.complete("Success!".to_string(), vec![]);
        assert_eq!(execution.status, SubagentStatus::Completed);
        assert_eq!(execution.output.as_ref().unwrap(), "Success!");
        assert!(execution.duration().is_some());
    }

    #[test]
    fn test_subagent_execution_failure() {
        let mut execution = SubagentExecution::new("failing-agent".to_string());

        execution.start();
        execution.fail("Something went wrong".to_string());

        assert!(matches!(execution.status, SubagentStatus::Failed(_)));
        assert_eq!(execution.error.as_ref().unwrap(), "Something went wrong");
        assert!(execution.duration().is_some());
    }
}

// Include comprehensive test suite from tests.rs
#[cfg(test)]
mod tests;

// Include YAML integration tests
#[cfg(test)]
mod test_yaml_integration;
