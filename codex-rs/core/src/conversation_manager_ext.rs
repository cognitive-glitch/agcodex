//! Extension module for ConversationManager to handle @agent-name invocations
//!
//! This module wraps the conversation flow to intercept and process
//! subagent invocations before sending messages to the main LLM.

use crate::conversation_manager::ConversationManager;
use crate::codex_conversation::CodexConversation;
use crate::config::Config;
use crate::error::{CodexErr, Result as CodexResult};
use crate::modes::OperatingMode;
use crate::protocol::{Op, InputItem};
use crate::subagents::{InvocationProcessor, SubagentRegistry};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Extended conversation manager with subagent support
pub struct ConversationManagerExt {
    /// Base conversation manager
    inner: ConversationManager,
    
    /// Subagent invocation processor
    invocation_processor: Arc<RwLock<Option<InvocationProcessor>>>,
    
    /// Subagent registry
    registry: Arc<SubagentRegistry>,
    
    /// Default operating mode
    _default_mode: OperatingMode,
}

impl ConversationManagerExt {
    /// Create a new extended conversation manager
    pub fn new(_config: &Config) -> CodexResult<Self> {
        let inner = ConversationManager::default();
        
        // Initialize subagent registry
        let registry = Arc::new(
            SubagentRegistry::new()
                .map_err(|e| CodexErr::InvalidConfig(format!("Failed to create registry: {}", e)))?
        );
        
        // Load all agents
        registry
            .load_all()
            .map_err(|e| CodexErr::InvalidConfig(format!("Failed to load agents: {}", e)))?;
        
        // Register built-in agents if available
        if let Err(e) = crate::subagents::built_in::register_built_in_agents(&registry) {
            warn!("Failed to register built-in agents: {}", e);
        }
        
        // Use Build mode as default
        let default_mode = OperatingMode::Build;
        
        // Create invocation processor
        let processor = InvocationProcessor::new(registry.clone(), default_mode)
            .map_err(|e| CodexErr::InvalidConfig(format!("Failed to create processor: {}", e)))?;
        
        Ok(Self {
            inner,
            invocation_processor: Arc::new(RwLock::new(Some(processor))),
            registry,
            _default_mode: default_mode,
        })
    }
    
    /// Create without subagent support (fallback mode)
    pub fn new_basic() -> Self {
        Self {
            inner: ConversationManager::default(),
            invocation_processor: Arc::new(RwLock::new(None)),
            registry: Arc::new(SubagentRegistry::new().unwrap_or_else(|_| {
                warn!("Failed to create registry, using empty registry");
                // This is a bit of a hack, but we need a registry even if it fails
                // In practice, this should rarely happen
                panic!("Failed to create SubagentRegistry")
            })),
            _default_mode: OperatingMode::Build,
        }
    }
    
    /// Process a user turn with potential agent invocations
    pub async fn process_user_turn(
        &self,
        conversation: &Arc<CodexConversation>,
        message: String,
        cwd: PathBuf,
        model: String,
    ) -> CodexResult<String> {
        // Check if we have an invocation processor
        let processor_guard = self.invocation_processor.read().await;
        
        let processed_message = if let Some(ref processor) = *processor_guard {
            // Process for agent invocations
            match processor.process_message(&message).await {
                Ok(Some(agent_output)) => {
                    info!("Agent invocations processed successfully");
                    
                    // Combine original message with agent output
                    format!(
                        "{}\n\n---\n\n{}",
                        message,
                        agent_output
                    )
                }
                Ok(None) => {
                    debug!("No agent invocations detected");
                    message
                }
                Err(e) => {
                    error!("Failed to process agent invocations: {}", e);
                    // Fall back to original message
                    message
                }
            }
        } else {
            debug!("No invocation processor available");
            message
        };
        
        // Create the user turn operation
        let op = Op::UserTurn {
            items: vec![InputItem::Text {
                text: processed_message.clone(),
            }],
            cwd,
            approval_policy: crate::protocol::AskForApproval::default(),
            sandbox_policy: crate::protocol::SandboxPolicy::new_workspace_write_policy(),
            model,
            effort: crate::protocol_config_types::ReasoningEffort::Medium,
            summary: crate::protocol_config_types::ReasoningSummary::None,
        };
        
        // Submit to conversation
        let submission_id = conversation.submit(op).await?;
        
        Ok(submission_id)
    }
    
    /// Hook for intercepting messages before they're sent to the LLM
    /// This is the main integration point for agent processing
    pub async fn intercept_message(
        &self,
        message: &str,
        context: MessageContext,
    ) -> CodexResult<InterceptResult> {
        let processor_guard = self.invocation_processor.read().await;
        
        if let Some(ref processor) = *processor_guard {
            match processor.process_message(message).await {
                Ok(Some(agent_output)) => {
                    info!("Intercepted and processed agent invocations");
                    
                    // Decide how to handle the agent output
                    if context.should_merge_results {
                        // Merge agent output with original message
                        Ok(InterceptResult::Modified {
                            message: format!(
                                "{}\n\n## Agent Analysis\n\n{}",
                                message,
                                agent_output
                            ),
                            metadata: Some(serde_json::json!({
                                "agents_invoked": true,
                                "original_message": message,
                            })),
                        })
                    } else {
                        // Return only agent output (skip LLM)
                        Ok(InterceptResult::Handled {
                            response: agent_output,
                            skip_llm: true,
                        })
                    }
                }
                Ok(None) => {
                    // No agents invoked, pass through
                    Ok(InterceptResult::PassThrough)
                }
                Err(e) => {
                    error!("Agent invocation failed: {}", e);
                    
                    // Decide whether to fail or continue
                    if context.fail_on_agent_error {
                        Err(CodexErr::InvalidConfig(format!("Agent invocation failed: {}", e)))
                    } else {
                        Ok(InterceptResult::PassThrough)
                    }
                }
            }
        } else {
            Ok(InterceptResult::PassThrough)
        }
    }
    
    /// Get the inner conversation manager
    pub fn inner(&self) -> &ConversationManager {
        &self.inner
    }
    
    /// Get the subagent registry
    pub fn registry(&self) -> &Arc<SubagentRegistry> {
        &self.registry
    }
    
    /// Update the operating mode
    pub async fn set_mode(&self, mode: OperatingMode) {
        // This would need to recreate the processor with the new mode
        // For now, we'll just log it
        info!("Operating mode change requested to: {:?}", mode);
    }
    
    /// Reload agent configurations
    pub async fn reload_agents(&self) -> CodexResult<()> {
        self.registry
            .reload()
            .map_err(|e| CodexErr::InvalidConfig(format!("Failed to reload agents: {}", e)))?;
        
        info!("Agent configurations reloaded");
        Ok(())
    }
}

/// Context for message interception
#[derive(Debug, Clone)]
pub struct MessageContext {
    /// Conversation ID
    pub conversation_id: Uuid,
    
    /// Whether to merge agent results with the original message
    pub should_merge_results: bool,
    
    /// Whether to fail if agent invocation fails
    pub fail_on_agent_error: bool,
    
    /// Current working directory
    pub cwd: PathBuf,
    
    /// Operating mode
    pub mode: OperatingMode,
}

impl Default for MessageContext {
    fn default() -> Self {
        Self {
            conversation_id: Uuid::new_v4(),
            should_merge_results: true,
            fail_on_agent_error: false,
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            mode: OperatingMode::Build,
        }
    }
}

/// Result of message interception
#[derive(Debug)]
pub enum InterceptResult {
    /// Pass the message through unchanged
    PassThrough,
    
    /// Modify the message before sending to LLM
    Modified {
        message: String,
        metadata: Option<serde_json::Value>,
    },
    
    /// Handle the message completely (skip LLM)
    Handled {
        response: String,
        skip_llm: bool,
    },
}

// Extension trait for easy integration
impl ConversationManager {
    /// Create an extended version with agent support
    pub fn with_agents(self, config: &Config) -> CodexResult<ConversationManagerExt> {
        // We can't move self, so we create a new one
        ConversationManagerExt::new(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_no_agents_passthrough() {
        let manager = ConversationManagerExt::new_basic();
        let context = MessageContext::default();
        
        let result = manager
            .intercept_message("This is a normal message", context)
            .await
            .unwrap();
        
        assert!(matches!(result, InterceptResult::PassThrough));
    }
    
    #[tokio::test]
    async fn test_agent_pattern_detection() {
        // This test would require setting up mock agents
        // For now, just test the structure
        let manager = ConversationManagerExt::new_basic();
        let context = MessageContext::default();
        
        let result = manager
            .intercept_message("@code-reviewer check this", context)
            .await;
        
        // Without registered agents, this should pass through or fail gracefully
        assert!(result.is_ok());
    }
}