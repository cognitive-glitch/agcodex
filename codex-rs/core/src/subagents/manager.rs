//! Agent Manager for spawning and coordinating subagents
//!
//! This module manages agent lifecycles, communication, and coordination,
//! working with the existing orchestrator infrastructure.

use super::SubagentContext;
use super::SubagentError;
use super::SubagentStatus;
use super::agents::AgentResult;
use super::context::AgentMessage;
use super::context::MessagePriority;
use super::context::MessageTarget;
use super::context::MessageType;
use super::orchestrator::AgentOrchestrator;
use super::orchestrator::OrchestratorConfig;
use super::parser::AgentParser;
use super::parser::ParsedInvocation;
use super::registry::SubagentRegistry;
use crate::code_tools::ast_agent_tools::ASTAgentTools;
use crate::modes::OperatingMode;
use chrono::Utc;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::debug;
use tracing::error;
use tracing::info;
use uuid::Uuid;

/// Agent execution statistics
#[derive(Debug, Clone)]
pub struct AgentStats {
    pub total_spawned: u64,
    pub total_completed: u64,
    pub total_failed: u64,
    pub total_cancelled: u64,
    pub avg_execution_time: Duration,
    pub last_execution: Option<SystemTime>,
}

/// Agent Manager for spawning and managing agent lifecycles
pub struct AgentManager {
    /// Agent registry
    registry: Arc<SubagentRegistry>,
    /// Agent parser
    _parser: AgentParser,
    /// Orchestrator for execution
    orchestrator: Arc<AgentOrchestrator>,
    /// Active agent executions
    active_agents: Arc<DashMap<Uuid, AgentHandle>>,
    /// Message bus for inter-agent communication
    message_bus: Arc<MessageBus>,
    /// Global cancellation flag
    global_cancel: Arc<AtomicBool>,
    /// Execution statistics
    stats: Arc<RwLock<HashMap<String, AgentStats>>>,
    /// AST tools for agents
    ast_tools: Arc<RwLock<ASTAgentTools>>,
}

/// Handle to a running agent
pub struct AgentHandle {
    pub id: Uuid,
    pub name: String,
    pub status: Arc<RwLock<SubagentStatus>>,
    pub cancel_flag: Arc<AtomicBool>,
    pub task_handle: JoinHandle<Result<AgentResult, SubagentError>>,
    pub started_at: SystemTime,
    pub context: SubagentContext,
}

/// Message bus for inter-agent communication
pub struct MessageBus {
    /// Broadcast sender for messages
    sender: broadcast::Sender<AgentMessage>,
    /// Keep one receiver alive to prevent channel closure
    _receiver: broadcast::Receiver<AgentMessage>,
    /// Message history
    history: Arc<RwLock<Vec<AgentMessage>>>,
    /// Topic subscriptions
    subscriptions: Arc<DashMap<String, Vec<Uuid>>>,
}

impl MessageBus {
    /// Create a new message bus
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = broadcast::channel(capacity);
        Self {
            sender,
            _receiver: receiver,
            history: Arc::new(RwLock::new(Vec::new())),
            subscriptions: Arc::new(DashMap::new()),
        }
    }

    /// Send a message to all agents
    pub async fn broadcast(&self, message: AgentMessage) -> Result<(), SubagentError> {
        // Store in history
        self.history.write().await.push(message.clone());

        // Broadcast to all listeners
        self.sender.send(message).map_err(|e| {
            SubagentError::ExecutionFailed(format!("Failed to broadcast message: {}", e))
        })?;

        Ok(())
    }

    /// Subscribe to messages
    pub fn subscribe(&self) -> broadcast::Receiver<AgentMessage> {
        self.sender.subscribe()
    }

    /// Subscribe to a specific topic
    pub async fn subscribe_to_topic(&self, topic: String, agent_id: Uuid) {
        self.subscriptions.entry(topic).or_default().push(agent_id);
    }

    /// Get message history
    pub async fn get_history(&self, limit: Option<usize>) -> Vec<AgentMessage> {
        let history = self.history.read().await;
        match limit {
            Some(n) => {
                // Return the last n messages in their original order
                let start = history.len().saturating_sub(n);
                history[start..].to_vec()
            }
            None => history.clone(),
        }
    }
}

impl AgentManager {
    /// Create a new agent manager
    pub fn new(registry: Arc<SubagentRegistry>, orchestrator_config: OrchestratorConfig) -> Self {
        let parser = AgentParser::with_registry(registry.clone());
        let orchestrator = Arc::new(AgentOrchestrator::new(
            registry.clone(),
            orchestrator_config,
            crate::modes::OperatingMode::Build, // Default to Build mode
        ));
        let ast_tools = Arc::new(RwLock::new(ASTAgentTools::new()));

        Self {
            registry,
            _parser: parser,
            orchestrator,
            active_agents: Arc::new(DashMap::new()),
            message_bus: Arc::new(MessageBus::new(1000)),
            global_cancel: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(RwLock::new(HashMap::new())),
            ast_tools,
        }
    }

    /// Spawn an agent from a parsed invocation
    pub async fn spawn_from_invocation(
        &self,
        invocation: ParsedInvocation,
    ) -> Result<Vec<Uuid>, SubagentError> {
        let mut agent_ids = Vec::new();

        // Execute based on the execution plan
        match invocation.execution_plan {
            super::invocation::ExecutionPlan::Single(agent_inv) => {
                let id = self
                    .spawn_agent(
                        agent_inv.agent_name,
                        self.create_context(invocation.context, invocation.mode_override),
                    )
                    .await?;
                agent_ids.push(id);
            }
            super::invocation::ExecutionPlan::Sequential(chain) => {
                for agent_inv in chain.agents {
                    let id = self
                        .spawn_agent(
                            agent_inv.agent_name,
                            self.create_context(
                                invocation.context.clone(),
                                invocation.mode_override,
                            ),
                        )
                        .await?;
                    agent_ids.push(id);

                    // Wait for completion before spawning next
                    self.wait_for_agent(id).await?;
                }
            }
            super::invocation::ExecutionPlan::Parallel(agents) => {
                let mut tasks = Vec::new();
                for agent_inv in agents {
                    let manager = self.clone_for_async();
                    let context = invocation.context.clone();
                    let mode = invocation.mode_override;

                    tasks.push(tokio::spawn(async move {
                        manager
                            .spawn_agent(
                                agent_inv.agent_name,
                                manager.create_context(context, mode),
                            )
                            .await
                    }));
                }

                for task in tasks {
                    let id = task.await.map_err(|e| {
                        SubagentError::ExecutionFailed(format!("Task join error: {}", e))
                    })??;
                    agent_ids.push(id);
                }
            }
            _ => {
                // Use orchestrator for complex plans
                self.orchestrator
                    .execute_plan(super::invocation::InvocationRequest {
                        id: invocation.id,
                        original_input: invocation.original_input,
                        execution_plan: invocation.execution_plan,
                        context: invocation.context,
                    })
                    .await?;
            }
        }

        Ok(agent_ids)
    }

    /// Spawn a single agent
    pub async fn spawn_agent(
        &self,
        agent_name: String,
        context: SubagentContext,
    ) -> Result<Uuid, SubagentError> {
        // Get executable agent from registry
        let agent = self
            .registry
            .get_executable_agent(&agent_name)
            .ok_or_else(|| SubagentError::AgentNotFound {
                name: agent_name.clone(),
            })?;

        let agent_id = Uuid::new_v4();
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let status = Arc::new(RwLock::new(SubagentStatus::Pending));

        // Update stats
        self.update_stats(&agent_name, |stats| {
            stats.total_spawned += 1;
            stats.last_execution = Some(SystemTime::now());
        })
        .await;

        // Create agent handle
        let agent_clone = agent.clone();
        let context_clone = context.clone();
        let cancel_clone = cancel_flag.clone();
        let status_clone = status.clone();
        let message_bus = self.message_bus.clone();
        let agent_name_clone = agent_name.clone();
        let ast_tools = self.ast_tools.clone();

        let task_handle = tokio::spawn(async move {
            // Update status to running
            *status_clone.write().await = SubagentStatus::Running;

            // Notify via message bus
            let _ = message_bus
                .broadcast(AgentMessage {
                    id: Uuid::new_v4(),
                    from: agent_id.to_string(),
                    to: MessageTarget::Broadcast,
                    message_type: MessageType::Info,
                    priority: MessagePriority::Normal,
                    payload: serde_json::json!({
                        "status": "started",
                        "agent": agent_name_clone
                    }),
                    timestamp: Utc::now(),
                })
                .await;

            // Execute agent
            let mut tools = ast_tools.write().await;
            let result = agent_clone
                .execute(&context_clone, &mut tools, cancel_clone)
                .await;

            // Update status based on result
            match &result {
                Ok(agent_result) => {
                    *status_clone.write().await = SubagentStatus::Completed;

                    // Broadcast completion
                    let _ = message_bus
                        .broadcast(AgentMessage {
                            id: Uuid::new_v4(),
                            from: agent_id.to_string(),
                            to: MessageTarget::Broadcast,
                            message_type: MessageType::Result,
                            priority: MessagePriority::High,
                            payload: serde_json::json!({
                                "summary": agent_result.summary.clone(),
                                "status": "completed",
                                "findings": agent_result.findings.len()
                            }),
                            timestamp: Utc::now(),
                        })
                        .await;
                }
                Err(e) => {
                    *status_clone.write().await = SubagentStatus::Failed(e.to_string());

                    // Broadcast failure
                    let _ = message_bus
                        .broadcast(AgentMessage {
                            id: Uuid::new_v4(),
                            from: agent_id.to_string(),
                            to: MessageTarget::Broadcast,
                            message_type: MessageType::Error,
                            priority: MessagePriority::Critical,
                            payload: serde_json::json!({
                                "error": format!("Agent {} failed: {}", agent_name_clone, e)
                            }),
                            timestamp: Utc::now(),
                        })
                        .await;
                }
            }

            result
        });

        // Store handle
        self.active_agents.insert(
            agent_id,
            AgentHandle {
                id: agent_id,
                name: agent_name.clone(),
                status,
                cancel_flag,
                task_handle,
                started_at: SystemTime::now(),
                context,
            },
        );

        info!("Spawned agent {} with ID {}", agent_name, agent_id);

        Ok(agent_id)
    }

    /// Wait for an agent to complete
    pub async fn wait_for_agent(&self, agent_id: Uuid) -> Result<AgentResult, SubagentError> {
        let handle =
            self.active_agents
                .remove(&agent_id)
                .ok_or_else(|| SubagentError::AgentNotFound {
                    name: format!("Agent with ID {}", agent_id),
                })?;

        let (_, agent_handle) = handle;
        let agent_name = agent_handle.name.clone();
        let started_at = agent_handle.started_at;

        // Wait for task completion
        let result = agent_handle
            .task_handle
            .await
            .map_err(|e| SubagentError::ExecutionFailed(format!("Task join error: {}", e)))?;

        // Update stats
        let execution_time = SystemTime::now()
            .duration_since(started_at)
            .unwrap_or_else(|_| Duration::from_secs(0));

        self.update_stats(&agent_name, |stats| {
            match &result {
                Ok(_) => stats.total_completed += 1,
                Err(_) => stats.total_failed += 1,
            }

            // Update average execution time
            let total = stats.total_completed + stats.total_failed;
            if total > 0 {
                let current_avg = stats.avg_execution_time.as_secs_f64();
                let new_avg = (current_avg * (total - 1) as f64 + execution_time.as_secs_f64())
                    / total as f64;
                stats.avg_execution_time = Duration::from_secs_f64(new_avg);
            }
        })
        .await;

        result
    }

    /// Cancel an agent
    pub async fn cancel_agent(&self, agent_id: Uuid) -> Result<(), SubagentError> {
        if let Some(handle) = self.active_agents.get(&agent_id) {
            handle.cancel_flag.store(true, Ordering::Release);
            *handle.status.write().await = SubagentStatus::Cancelled;

            // Update stats
            self.update_stats(&handle.name, |stats| {
                stats.total_cancelled += 1;
            })
            .await;

            info!("Cancelled agent {} (ID: {})", handle.name, agent_id);
            Ok(())
        } else {
            Err(SubagentError::AgentNotFound {
                name: format!("Agent with ID {}", agent_id),
            })
        }
    }

    /// Cancel all active agents
    pub async fn cancel_all(&self) {
        self.global_cancel.store(true, Ordering::Release);

        for entry in self.active_agents.iter() {
            let handle = entry.value();
            handle.cancel_flag.store(true, Ordering::Release);
            *handle.status.write().await = SubagentStatus::Cancelled;
        }

        info!("Cancelled all active agents");
    }

    /// Get status of an agent
    pub async fn get_agent_status(&self, agent_id: Uuid) -> Option<SubagentStatus> {
        self.active_agents
            .get(&agent_id)
            .map(|handle| handle.status.clone())
            .and_then(|status| {
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current()
                        .block_on(async { Some(status.read().await.clone()) })
                })
            })
    }

    /// Get all active agents
    pub fn get_active_agents(&self) -> Vec<(Uuid, String, SubagentStatus)> {
        self.active_agents
            .iter()
            .map(|entry| {
                let handle = entry.value();
                let status = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current()
                        .block_on(async { handle.status.read().await.clone() })
                });
                (*entry.key(), handle.name.clone(), status)
            })
            .collect()
    }

    /// Get agent statistics
    pub async fn get_stats(&self, agent_name: Option<&str>) -> HashMap<String, AgentStats> {
        let stats = self.stats.read().await;

        match agent_name {
            Some(name) => stats
                .get(name)
                .map(|s| HashMap::from([(name.to_string(), s.clone())]))
                .unwrap_or_default(),
            None => stats.clone(),
        }
    }

    /// Handle agent communication via message bus
    pub async fn handle_communication(&self) -> Result<(), SubagentError> {
        let mut receiver = self.message_bus.subscribe();

        loop {
            tokio::select! {
                Ok(message) = receiver.recv() => {
                    self.process_message(message).await?;
                }
                _ = tokio::time::sleep(Duration::from_secs(1)) => {
                    if self.global_cancel.load(Ordering::Acquire) {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// Process a message from the message bus
    async fn process_message(&self, message: AgentMessage) -> Result<(), SubagentError> {
        debug!("Processing message: {:?}", message.message_type);

        match message.message_type {
            MessageType::Request => {
                // Handle inter-agent requests
                if let MessageTarget::Agent(target_id) = &message.to
                    && let Ok(target_uuid) = Uuid::parse_str(target_id)
                    && let Some(handle) = self.active_agents.get(&target_uuid)
                {
                    // Agent-specific handling would go here
                    debug!("Forwarding request to agent {}", handle.name);
                }
            }
            MessageType::Response | MessageType::Result => {
                // Log results for monitoring
                info!("Agent result received: {:?}", message.payload);
            }
            MessageType::Error => {
                // Log errors for debugging
                error!("Agent error: {:?}", message.payload);
            }
            MessageType::Info => {
                // Status updates for monitoring
                debug!("Agent info: {:?}", message.payload);
            }
            MessageType::Coordination => {
                // Coordination messages between agents
                debug!("Agent coordination: {:?}", message.payload);
            }
            _ => {}
        }

        Ok(())
    }

    /// Support context inheritance between agents
    pub async fn inherit_context(
        &self,
        from_agent: Uuid,
        to_context: &mut SubagentContext,
    ) -> Result<(), SubagentError> {
        // Get message history from the source agent
        let messages = self.message_bus.get_history(Some(10)).await;

        let relevant_messages: Vec<_> = messages
            .into_iter()
            .filter(|m| m.from == from_agent.to_string())
            .collect();

        // Add relevant context to the new agent
        for message in relevant_messages {
            if message.message_type == MessageType::Result {
                to_context.conversation_context.push('\n');
                if let Some(summary) = message.payload.get("summary")
                    && let Some(s) = summary.as_str()
                {
                    to_context.conversation_context.push_str(s);
                }
            }
        }

        Ok(())
    }

    /// Create a context for agent execution
    fn create_context(&self, context_str: String, mode: Option<OperatingMode>) -> SubagentContext {
        SubagentContext {
            execution_id: Uuid::new_v4(),
            mode: mode.unwrap_or(OperatingMode::Build),
            available_tools: vec![
                "search".to_string(),
                "edit".to_string(),
                "think".to_string(),
                "plan".to_string(),
            ],
            conversation_context: context_str,
            working_directory: std::env::current_dir().unwrap_or_default(),
            parameters: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Update agent statistics
    async fn update_stats<F>(&self, agent_name: &str, updater: F)
    where
        F: FnOnce(&mut AgentStats),
    {
        let mut stats = self.stats.write().await;
        let agent_stats = stats
            .entry(agent_name.to_string())
            .or_insert_with(|| AgentStats {
                total_spawned: 0,
                total_completed: 0,
                total_failed: 0,
                total_cancelled: 0,
                avg_execution_time: Duration::from_secs(0),
                last_execution: None,
            });
        updater(agent_stats);
    }

    /// Clone manager for async operations
    fn clone_for_async(&self) -> Self {
        Self {
            registry: self.registry.clone(),
            _parser: AgentParser::with_registry(self.registry.clone()),
            orchestrator: self.orchestrator.clone(),
            active_agents: self.active_agents.clone(),
            message_bus: self.message_bus.clone(),
            global_cancel: self.global_cancel.clone(),
            stats: self.stats.clone(),
            ast_tools: self.ast_tools.clone(),
        }
    }
}

/// Track agent status and results
#[derive(Debug, Clone)]
pub struct AgentTracker {
    pub agent_id: Uuid,
    pub agent_name: String,
    pub status: SubagentStatus,
    pub result: Option<AgentResult>,
    pub started_at: SystemTime,
    pub completed_at: Option<SystemTime>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_manager_creation() {
        let registry = Arc::new(SubagentRegistry::new().unwrap());
        let config = OrchestratorConfig::default();
        let manager = AgentManager::new(registry.clone(), config);

        assert_eq!(manager.get_active_agents().len(), 0);
    }

    #[tokio::test]
    async fn test_message_bus() {
        let bus = MessageBus::new(100);

        let message = AgentMessage {
            id: Uuid::new_v4(),
            from: Uuid::new_v4().to_string(),
            to: MessageTarget::Broadcast,
            message_type: MessageType::Info,
            priority: MessagePriority::Normal,
            payload: serde_json::json!({
                "content": "Test message"
            }),
            timestamp: Utc::now(),
        };

        bus.broadcast(message.clone()).await.unwrap();

        let history = bus.get_history(None).await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].payload["content"], "Test message");
    }

    #[test]
    fn test_context_creation() {
        let registry = Arc::new(SubagentRegistry::new().unwrap());
        let config = OrchestratorConfig::default();
        let manager = AgentManager::new(registry.clone(), config);

        let context = manager.create_context("test context".to_string(), Some(OperatingMode::Plan));

        assert_eq!(context.mode, OperatingMode::Plan);
        assert_eq!(context.conversation_context, "test context");
        assert!(!context.available_tools.is_empty());
    }
}
