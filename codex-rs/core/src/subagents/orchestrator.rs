//! Agent orchestration engine for managing and coordinating subagent execution
//!
//! This module provides a comprehensive orchestration system that handles:
//! - Sequential, parallel, and mixed execution strategies
//! - Context management and sharing between agents
//! - Error handling with retry logic and circuit breakers
//! - Progress tracking and cancellation
//! - Resource management and concurrency limits

use super::AgentChain;
use super::AgentInvocation;
use super::ExecutionPlan;
use super::ExecutionStep;
use super::InvocationRequest;
use super::SubagentContext;
use super::SubagentError;
use super::SubagentExecution;
use super::SubagentRegistry;
use super::SubagentStatus;
use crate::modes::OperatingMode;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::SystemTime;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio::sync::Semaphore;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio::time::timeout;
use tracing::debug;
use tracing::info;
use tracing::warn;
use uuid::Uuid;

/// Maximum number of concurrent agent executions
const DEFAULT_MAX_CONCURRENCY: usize = 8;

/// Default timeout for agent execution
const DEFAULT_AGENT_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes

/// Maximum number of retries for transient failures
const MAX_RETRIES: u32 = 3;

/// Backoff duration between retries
const RETRY_BACKOFF: Duration = Duration::from_secs(2);

/// Circuit breaker threshold - opens after this many consecutive failures
const CIRCUIT_BREAKER_THRESHOLD: u32 = 5;

/// Circuit breaker reset duration
const CIRCUIT_BREAKER_RESET: Duration = Duration::from_secs(60);

/// Orchestrator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Maximum number of concurrent agent executions
    pub max_concurrency: usize,

    /// Default timeout for agent execution
    pub agent_timeout: Duration,

    /// Enable retry logic for transient failures
    pub enable_retries: bool,

    /// Maximum number of retries
    pub max_retries: u32,

    /// Backoff duration between retries
    pub retry_backoff: Duration,

    /// Enable circuit breaker pattern
    pub enable_circuit_breaker: bool,

    /// Circuit breaker failure threshold
    pub circuit_breaker_threshold: u32,

    /// Circuit breaker reset duration
    pub circuit_breaker_reset: Duration,

    /// Enable memory pressure monitoring
    pub monitor_memory: bool,

    /// Memory pressure threshold (in MB)
    pub memory_threshold_mb: usize,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_concurrency: DEFAULT_MAX_CONCURRENCY,
            agent_timeout: DEFAULT_AGENT_TIMEOUT,
            enable_retries: true,
            max_retries: MAX_RETRIES,
            retry_backoff: RETRY_BACKOFF,
            enable_circuit_breaker: true,
            circuit_breaker_threshold: CIRCUIT_BREAKER_THRESHOLD,
            circuit_breaker_reset: CIRCUIT_BREAKER_RESET,
            monitor_memory: true,
            memory_threshold_mb: 2048, // 2GB
        }
    }
}

/// Shared context for passing data between agents
#[derive(Debug, Clone)]
pub struct SharedContext {
    /// Data shared between agents
    data: Arc<RwLock<HashMap<String, serde_json::Value>>>,

    /// Output from previous agents (for chaining)
    previous_outputs: Arc<RwLock<Vec<String>>>,

    /// Files modified across all agent executions
    modified_files: Arc<RwLock<Vec<PathBuf>>>,

    /// Accumulated errors (for partial results)
    errors: Arc<RwLock<Vec<String>>>,
}

impl Default for SharedContext {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedContext {
    /// Create a new shared context
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            previous_outputs: Arc::new(RwLock::new(Vec::new())),
            modified_files: Arc::new(RwLock::new(Vec::new())),
            errors: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Set a value in the shared context
    pub async fn set(&self, key: String, value: serde_json::Value) {
        self.data.write().await.insert(key, value);
    }

    /// Get a value from the shared context
    pub async fn get(&self, key: &str) -> Option<serde_json::Value> {
        self.data.read().await.get(key).cloned()
    }

    /// Add output from a completed agent
    pub async fn add_output(&self, output: String) {
        self.previous_outputs.write().await.push(output);
    }

    /// Get the last output (for chaining)
    pub async fn last_output(&self) -> Option<String> {
        self.previous_outputs.read().await.last().cloned()
    }

    /// Get all outputs
    pub async fn all_outputs(&self) -> Vec<String> {
        self.previous_outputs.read().await.clone()
    }

    /// Add modified files
    pub async fn add_modified_files(&self, files: Vec<PathBuf>) {
        self.modified_files.write().await.extend(files);
    }

    /// Get all modified files
    pub async fn modified_files(&self) -> Vec<PathBuf> {
        self.modified_files.read().await.clone()
    }

    /// Add an error
    pub async fn add_error(&self, error: String) {
        self.errors.write().await.push(error);
    }

    /// Get all errors
    pub async fn errors(&self) -> Vec<String> {
        self.errors.read().await.clone()
    }

    /// Create a snapshot of the context
    pub async fn snapshot(&self) -> ContextSnapshot {
        ContextSnapshot {
            data: self.data.read().await.clone(),
            previous_outputs: self.previous_outputs.read().await.clone(),
            modified_files: self.modified_files.read().await.clone(),
            errors: self.errors.read().await.clone(),
            timestamp: SystemTime::now(),
        }
    }

    /// Restore from a snapshot
    pub async fn restore(&self, snapshot: ContextSnapshot) {
        *self.data.write().await = snapshot.data;
        *self.previous_outputs.write().await = snapshot.previous_outputs;
        *self.modified_files.write().await = snapshot.modified_files;
        *self.errors.write().await = snapshot.errors;
    }

    /// Merge another context into this one
    pub async fn merge(&self, other: &SharedContext) {
        // Merge data
        let other_data = other.data.read().await;
        for (key, value) in other_data.iter() {
            self.data.write().await.insert(key.clone(), value.clone());
        }

        // Merge outputs
        let other_outputs = other.previous_outputs.read().await;
        self.previous_outputs
            .write()
            .await
            .extend(other_outputs.clone());

        // Merge modified files
        let other_files = other.modified_files.read().await;
        self.modified_files
            .write()
            .await
            .extend(other_files.clone());

        // Merge errors
        let other_errors = other.errors.read().await;
        self.errors.write().await.extend(other_errors.clone());
    }
}

/// Snapshot of shared context at a point in time
#[derive(Debug, Clone)]
pub struct ContextSnapshot {
    pub data: HashMap<String, serde_json::Value>,
    pub previous_outputs: Vec<String>,
    pub modified_files: Vec<PathBuf>,
    pub errors: Vec<String>,
    pub timestamp: SystemTime,
}

/// Progress update for agent execution
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    pub execution_id: Uuid,
    pub agent_name: String,
    pub status: SubagentStatus,
    pub message: Option<String>,
    pub progress_percentage: Option<u8>,
    pub timestamp: SystemTime,
}

/// Result of orchestrator execution
#[derive(Debug)]
pub struct OrchestratorResult {
    /// Request that was executed
    pub request: InvocationRequest,

    /// Execution results for each agent
    pub executions: Vec<SubagentExecution>,

    /// Final shared context
    pub context: SharedContext,

    /// Total execution time
    pub total_duration: Duration,

    /// Whether execution was successful (all agents succeeded)
    pub success: bool,

    /// Whether execution was partially successful (some agents succeeded)
    pub partial_success: bool,
}

/// Circuit breaker for handling repeated failures
#[derive(Debug)]
struct CircuitBreaker {
    failure_count: AtomicU32,
    is_open: AtomicBool,
    last_failure_time: Arc<Mutex<Option<SystemTime>>>,
    threshold: u32,
    reset_duration: Duration,
}

impl CircuitBreaker {
    fn new(threshold: u32, reset_duration: Duration) -> Self {
        Self {
            failure_count: AtomicU32::new(0),
            is_open: AtomicBool::new(false),
            last_failure_time: Arc::new(Mutex::new(None)),
            threshold,
            reset_duration,
        }
    }

    async fn record_success(&self) {
        self.failure_count.store(0, Ordering::SeqCst);
        self.is_open.store(false, Ordering::SeqCst);
        *self.last_failure_time.lock().await = None;
    }

    async fn record_failure(&self) {
        let count = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
        *self.last_failure_time.lock().await = Some(SystemTime::now());

        if count >= self.threshold {
            self.is_open.store(true, Ordering::SeqCst);
            warn!(
                "Circuit breaker opened after {} consecutive failures",
                count
            );
        }
    }

    async fn is_open(&self) -> bool {
        if !self.is_open.load(Ordering::SeqCst) {
            return false;
        }

        // Check if we should try to close the circuit
        if let Some(last_failure) = *self.last_failure_time.lock().await
            && let Ok(elapsed) = SystemTime::now().duration_since(last_failure)
            && elapsed > self.reset_duration
        {
            self.is_open.store(false, Ordering::SeqCst);
            self.failure_count.store(0, Ordering::SeqCst);
            info!("Circuit breaker reset after {:?}", elapsed);
            return false;
        }

        true
    }
}

/// Agent orchestrator for managing and coordinating subagent execution
#[derive(Debug)]
pub struct AgentOrchestrator {
    /// Configuration
    config: OrchestratorConfig,

    /// Agent registry
    registry: Arc<SubagentRegistry>,

    /// Semaphore for concurrency control
    concurrency_limiter: Arc<Semaphore>,

    /// Circuit breakers per agent
    circuit_breakers: Arc<RwLock<HashMap<String, Arc<CircuitBreaker>>>>,

    /// Progress channel sender
    progress_tx: mpsc::UnboundedSender<ProgressUpdate>,

    /// Progress channel receiver
    progress_rx: Arc<Mutex<mpsc::UnboundedReceiver<ProgressUpdate>>>,

    /// Cancellation flag
    cancelled: Arc<AtomicBool>,

    /// Active execution count
    active_executions: Arc<AtomicUsize>,

    /// Current operating mode
    operating_mode: OperatingMode,
}

impl AgentOrchestrator {
    /// Create a new agent orchestrator
    pub fn new(
        registry: Arc<SubagentRegistry>,
        config: OrchestratorConfig,
        operating_mode: OperatingMode,
    ) -> Self {
        let (progress_tx, progress_rx) = mpsc::unbounded_channel();

        Self {
            config: config.clone(),
            registry,
            concurrency_limiter: Arc::new(Semaphore::new(config.max_concurrency)),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            progress_tx,
            progress_rx: Arc::new(Mutex::new(progress_rx)),
            cancelled: Arc::new(AtomicBool::new(false)),
            active_executions: Arc::new(AtomicUsize::new(0)),
            operating_mode,
        }
    }

    /// Execute an invocation plan
    pub async fn execute_plan(
        &self,
        request: InvocationRequest,
    ) -> Result<OrchestratorResult, SubagentError> {
        let start_time = SystemTime::now();
        info!("Starting orchestrator execution for request {}", request.id);

        // Check for cancellation
        if self.cancelled.load(Ordering::SeqCst) {
            return Err(SubagentError::ExecutionFailed(
                "Execution cancelled".to_string(),
            ));
        }

        // Check memory pressure if monitoring is enabled
        if self.config.monitor_memory {
            self.check_memory_pressure().await?;
        }

        // Create shared context
        let context = SharedContext::new();

        // Execute based on plan type
        let executions = match &request.execution_plan {
            ExecutionPlan::Single(invocation) => {
                vec![self.execute_single(invocation.clone(), &context).await?]
            }
            ExecutionPlan::Sequential(chain) => {
                self.execute_sequential(chain.clone(), &context).await?
            }
            ExecutionPlan::Parallel(invocations) => {
                self.execute_parallel(invocations.clone(), &context).await?
            }
            ExecutionPlan::Mixed(steps) => self.execute_mixed(steps.clone(), &context).await?,
        };

        // Calculate total duration
        let total_duration = SystemTime::now()
            .duration_since(start_time)
            .unwrap_or_default();

        // Determine success status
        let success = executions
            .iter()
            .all(|e| e.status == SubagentStatus::Completed);
        let partial_success = executions
            .iter()
            .any(|e| e.status == SubagentStatus::Completed);

        info!(
            "Orchestrator execution completed for request {} in {:?} (success: {}, partial: {})",
            request.id, total_duration, success, partial_success
        );

        Ok(OrchestratorResult {
            request,
            executions,
            context,
            total_duration,
            success,
            partial_success,
        })
    }

    /// Execute a single agent
    pub async fn execute_single(
        &self,
        invocation: AgentInvocation,
        shared_context: &SharedContext,
    ) -> Result<SubagentExecution, SubagentError> {
        // Check circuit breaker
        if self.config.enable_circuit_breaker {
            let breaker = self
                .get_or_create_circuit_breaker(&invocation.agent_name)
                .await;
            if breaker.is_open().await {
                return Err(SubagentError::ExecutionFailed(format!(
                    "Circuit breaker open for agent {}",
                    invocation.agent_name
                )));
            }
        }

        // Acquire concurrency permit
        let _permit = self.concurrency_limiter.acquire().await.map_err(|e| {
            SubagentError::ExecutionFailed(format!("Failed to acquire permit: {}", e))
        })?;

        self.active_executions.fetch_add(1, Ordering::SeqCst);
        let result = self.execute_with_retry(invocation, shared_context).await;
        self.active_executions.fetch_sub(1, Ordering::SeqCst);

        // Update circuit breaker
        if self.config.enable_circuit_breaker {
            let breaker = self
                .get_or_create_circuit_breaker(
                    &result
                        .as_ref()
                        .map(|e| e.agent_name.clone())
                        .unwrap_or_default(),
                )
                .await;

            match &result {
                Ok(execution) if execution.status == SubagentStatus::Completed => {
                    breaker.record_success().await;
                }
                _ => {
                    breaker.record_failure().await;
                }
            }
        }

        result
    }

    /// Execute agents sequentially
    pub async fn execute_sequential(
        &self,
        chain: AgentChain,
        shared_context: &SharedContext,
    ) -> Result<Vec<SubagentExecution>, SubagentError> {
        let mut executions = Vec::new();

        for invocation in chain.agents {
            // Check for cancellation
            if self.cancelled.load(Ordering::SeqCst) {
                warn!("Sequential execution cancelled");
                break;
            }

            let execution = self.execute_single(invocation, shared_context).await?;

            // Pass output to context if chaining is enabled
            if chain.pass_output
                && let Some(output) = &execution.output
            {
                shared_context.add_output(output.clone()).await;
            }

            // Add modified files to context
            shared_context
                .add_modified_files(execution.modified_files.clone())
                .await;

            executions.push(execution);
        }

        Ok(executions)
    }

    /// Execute agents in parallel
    pub async fn execute_parallel(
        &self,
        invocations: Vec<AgentInvocation>,
        shared_context: &SharedContext,
    ) -> Result<Vec<SubagentExecution>, SubagentError> {
        let mut tasks = Vec::new();

        for invocation in invocations {
            let self_clone = self.clone_for_task();
            let context_clone = shared_context.clone();

            let task =
                tokio::spawn(
                    async move { self_clone.execute_single(invocation, &context_clone).await },
                );

            tasks.push(task);
        }

        // Wait for all tasks to complete
        let mut executions = Vec::new();
        let mut errors = Vec::new();

        for task in tasks {
            match task.await {
                Ok(Ok(execution)) => {
                    // Merge modified files
                    shared_context
                        .add_modified_files(execution.modified_files.clone())
                        .await;
                    executions.push(execution);
                }
                Ok(Err(e)) => {
                    errors.push(e.to_string());
                    shared_context.add_error(e.to_string()).await;
                }
                Err(e) => {
                    let error = format!("Task join error: {}", e);
                    errors.push(error.clone());
                    shared_context.add_error(error).await;
                }
            }
        }

        // Return partial results if some succeeded
        if !executions.is_empty() {
            Ok(executions)
        } else if !errors.is_empty() {
            Err(SubagentError::ExecutionFailed(format!(
                "All parallel executions failed: {}",
                errors.join(", ")
            )))
        } else {
            Err(SubagentError::ExecutionFailed(
                "No executions completed".to_string(),
            ))
        }
    }

    /// Execute mixed execution plan
    pub async fn execute_mixed(
        &self,
        steps: Vec<ExecutionStep>,
        shared_context: &SharedContext,
    ) -> Result<Vec<SubagentExecution>, SubagentError> {
        let mut all_executions = Vec::new();

        for step in steps {
            // Check for cancellation
            if self.cancelled.load(Ordering::SeqCst) {
                warn!("Mixed execution cancelled");
                break;
            }

            match step {
                ExecutionStep::Single(invocation) => {
                    let execution = self.execute_single(invocation, shared_context).await?;
                    all_executions.push(execution);
                }
                ExecutionStep::Parallel(invocations) => {
                    let executions = self.execute_parallel(invocations, shared_context).await?;
                    all_executions.extend(executions);
                }
                ExecutionStep::Barrier => {
                    // Wait for all active executions to complete
                    while self.active_executions.load(Ordering::SeqCst) > 0 {
                        sleep(Duration::from_millis(100)).await;
                    }
                    debug!("Barrier: All previous executions completed");
                }
            }
        }

        Ok(all_executions)
    }

    /// Execute with retry logic
    async fn execute_with_retry(
        &self,
        invocation: AgentInvocation,
        shared_context: &SharedContext,
    ) -> Result<SubagentExecution, SubagentError> {
        let mut attempts = 0;
        let max_attempts = if self.config.enable_retries {
            self.config.max_retries + 1
        } else {
            1
        };

        loop {
            attempts += 1;

            // Create execution
            let mut execution = SubagentExecution::new(invocation.agent_name.clone());

            // Send progress update
            self.send_progress(ProgressUpdate {
                execution_id: execution.id,
                agent_name: invocation.agent_name.clone(),
                status: SubagentStatus::Running,
                message: Some(format!(
                    "Starting execution (attempt {}/{})",
                    attempts, max_attempts
                )),
                progress_percentage: Some(0),
                timestamp: SystemTime::now(),
            })
            .await;

            // Execute with timeout
            let result = timeout(
                self.config.agent_timeout,
                self.execute_agent_internal(&invocation, shared_context, &mut execution),
            )
            .await;

            match result {
                Ok(Ok(())) => {
                    // Success
                    self.send_progress(ProgressUpdate {
                        execution_id: execution.id,
                        agent_name: invocation.agent_name.clone(),
                        status: SubagentStatus::Completed,
                        message: Some("Execution completed successfully".to_string()),
                        progress_percentage: Some(100),
                        timestamp: SystemTime::now(),
                    })
                    .await;

                    return Ok(execution);
                }
                Ok(Err(e)) if attempts < max_attempts && self.is_retriable_error(&e) => {
                    // Retriable error
                    warn!(
                        "Agent {} failed with retriable error (attempt {}/{}): {}",
                        invocation.agent_name, attempts, max_attempts, e
                    );

                    self.send_progress(ProgressUpdate {
                        execution_id: execution.id,
                        agent_name: invocation.agent_name.clone(),
                        status: SubagentStatus::Running,
                        message: Some(format!("Retrying after error: {}", e)),
                        progress_percentage: None,
                        timestamp: SystemTime::now(),
                    })
                    .await;

                    // Backoff before retry
                    sleep(self.config.retry_backoff * attempts).await;
                    continue;
                }
                Ok(Err(e)) => {
                    // Non-retriable error or max attempts reached
                    execution.fail(e.to_string());

                    self.send_progress(ProgressUpdate {
                        execution_id: execution.id,
                        agent_name: invocation.agent_name.clone(),
                        status: SubagentStatus::Failed(e.to_string()),
                        message: Some(format!("Execution failed: {}", e)),
                        progress_percentage: None,
                        timestamp: SystemTime::now(),
                    })
                    .await;

                    return Err(e);
                }
                Err(_) => {
                    // Timeout
                    let error = format!(
                        "Agent {} timed out after {:?}",
                        invocation.agent_name, self.config.agent_timeout
                    );
                    execution.fail(error.clone());

                    self.send_progress(ProgressUpdate {
                        execution_id: execution.id,
                        agent_name: invocation.agent_name.clone(),
                        status: SubagentStatus::Failed(error.clone()),
                        message: Some(error.clone()),
                        progress_percentage: None,
                        timestamp: SystemTime::now(),
                    })
                    .await;

                    if attempts < max_attempts {
                        warn!(
                            "Agent {} timed out (attempt {}/{})",
                            invocation.agent_name, attempts, max_attempts
                        );
                        sleep(self.config.retry_backoff * attempts).await;
                        continue;
                    }

                    return Err(SubagentError::Timeout {
                        name: invocation.agent_name,
                    });
                }
            }
        }
    }

    /// Internal agent execution logic
    async fn execute_agent_internal(
        &self,
        invocation: &AgentInvocation,
        shared_context: &SharedContext,
        execution: &mut SubagentExecution,
    ) -> Result<(), SubagentError> {
        execution.start();

        // Load agent configuration
        let _agent = self
            .registry
            .get_agent(&invocation.agent_name)
            .ok_or_else(|| SubagentError::AgentNotFound {
                name: invocation.agent_name.clone(),
            })?;

        // Create agent context
        // Note: Since we now have the actual Subagent trait object, we use the agent's capabilities
        let agent_context = SubagentContext {
            execution_id: execution.id,
            mode: self.operating_mode, // Use orchestrator's mode (no override from trait)
            available_tools: vec![],   // TODO: Get capabilities from the agent
            conversation_context: shared_context.last_output().await.unwrap_or_default(),
            working_directory: std::env::current_dir().unwrap_or_default(),
            parameters: invocation.parameters.clone(),
            metadata: HashMap::new(),
        };

        // Simulate agent execution (in real implementation, this would call the actual agent)
        info!(
            "Executing agent {} with context: {:?}",
            invocation.agent_name, agent_context
        );

        // Send progress updates
        for i in 1..=10 {
            if self.cancelled.load(Ordering::SeqCst) {
                return Err(SubagentError::ExecutionFailed(
                    "Execution cancelled".to_string(),
                ));
            }

            self.send_progress(ProgressUpdate {
                execution_id: execution.id,
                agent_name: invocation.agent_name.clone(),
                status: SubagentStatus::Running,
                message: Some(format!("Processing step {}/10", i)),
                progress_percentage: Some((i * 10) as u8),
                timestamp: SystemTime::now(),
            })
            .await;

            sleep(Duration::from_millis(100)).await;
        }

        // Mock successful completion
        let output = format!(
            "Agent {} completed successfully with parameters: {:?}",
            invocation.agent_name, invocation.parameters
        );

        execution.complete(output.clone(), vec![]);

        Ok(())
    }

    /// Check if an error is retriable
    const fn is_retriable_error(&self, error: &SubagentError) -> bool {
        matches!(
            error,
            SubagentError::Timeout { .. }
                | SubagentError::ExecutionFailed(_)
                | SubagentError::Io(_)
        )
    }

    /// Get or create a circuit breaker for an agent
    async fn get_or_create_circuit_breaker(&self, agent_name: &str) -> Arc<CircuitBreaker> {
        let mut breakers = self.circuit_breakers.write().await;

        breakers
            .entry(agent_name.to_string())
            .or_insert_with(|| {
                Arc::new(CircuitBreaker::new(
                    self.config.circuit_breaker_threshold,
                    self.config.circuit_breaker_reset,
                ))
            })
            .clone()
    }

    /// Check memory pressure
    async fn check_memory_pressure(&self) -> Result<(), SubagentError> {
        // This is a simplified implementation
        // In production, you would use system metrics
        let memory_usage_mb = 500; // Mock value

        if memory_usage_mb > self.config.memory_threshold_mb {
            Err(SubagentError::ExecutionFailed(format!(
                "Memory pressure too high: {}MB > {}MB threshold",
                memory_usage_mb, self.config.memory_threshold_mb
            )))
        } else {
            Ok(())
        }
    }

    /// Send a progress update
    async fn send_progress(&self, update: ProgressUpdate) {
        if let Err(e) = self.progress_tx.send(update) {
            warn!("Failed to send progress update: {}", e);
        }
    }

    /// Get progress receiver
    pub async fn progress_receiver(&self) -> mpsc::UnboundedReceiver<ProgressUpdate> {
        // This would normally return a cloned receiver
        // For simplicity, we're creating a new channel here
        let (_tx, rx) = mpsc::unbounded_channel();
        rx
    }

    /// Cancel ongoing executions
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        info!("Orchestrator execution cancelled");
    }

    /// Reset cancellation flag
    pub fn reset_cancellation(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
    }

    /// Get active execution count
    pub fn active_count(&self) -> usize {
        self.active_executions.load(Ordering::SeqCst)
    }

    /// Clone for spawning tasks
    fn clone_for_task(&self) -> Arc<Self> {
        // In a real implementation, this would return Arc<Self>
        // For now, we'll create a simplified version
        Arc::new(Self {
            config: self.config.clone(),
            registry: self.registry.clone(),
            concurrency_limiter: self.concurrency_limiter.clone(),
            circuit_breakers: self.circuit_breakers.clone(),
            progress_tx: self.progress_tx.clone(),
            progress_rx: self.progress_rx.clone(),
            cancelled: self.cancelled.clone(),
            active_executions: self.active_executions.clone(),
            operating_mode: self.operating_mode,
        })
    }
}

/// Conditional execution support
impl AgentOrchestrator {
    /// Execute with conditional logic
    pub async fn execute_conditional<F>(
        &self,
        invocation: AgentInvocation,
        shared_context: &SharedContext,
        condition: F,
    ) -> Result<Option<SubagentExecution>, SubagentError>
    where
        F: Fn(&SharedContext) -> futures::future::BoxFuture<'_, bool> + Send + Sync,
    {
        // Check condition
        if condition(shared_context).await {
            Ok(Some(self.execute_single(invocation, shared_context).await?))
        } else {
            info!("Skipping agent {} due to condition", invocation.agent_name);
            Ok(None)
        }
    }

    /// Execute with dependencies
    pub async fn execute_with_dependencies(
        &self,
        invocation: AgentInvocation,
        dependencies: Vec<String>,
        shared_context: &SharedContext,
    ) -> Result<SubagentExecution, SubagentError> {
        // Check that all dependencies have been executed
        let outputs = shared_context.all_outputs().await;

        for dep in dependencies {
            if !outputs.iter().any(|o| o.contains(&dep)) {
                return Err(SubagentError::ExecutionFailed(format!(
                    "Dependency {} not satisfied for agent {}",
                    dep, invocation.agent_name
                )));
            }
        }

        self.execute_single(invocation, shared_context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subagents::config::SubagentConfig;

    async fn create_test_orchestrator() -> AgentOrchestrator {
        let registry = Arc::new(SubagentRegistry::new());

        // Add test agents
        let test_agent = SubagentConfig {
            name: "test-agent".to_string(),
            description: "Test agent".to_string(),
            mode_override: None,
            intelligence: crate::subagents::config::IntelligenceLevel::Medium,
            tools: std::collections::HashMap::new(),
            prompt: "Test prompt".to_string(),
            parameters: vec![],
            template: None,
            timeout_seconds: 10,
            chainable: true,
            parallelizable: true,
            metadata: std::collections::HashMap::new(),
            file_patterns: vec![],
        };

        registry.register(test_agent).await.unwrap();

        AgentOrchestrator::new(
            registry,
            OrchestratorConfig::default(),
            OperatingMode::Build,
        )
    }

    #[tokio::test]
    async fn test_single_agent_execution() {
        let orchestrator = create_test_orchestrator().await;

        let invocation = AgentInvocation {
            agent_name: "test-agent".to_string(),
            parameters: HashMap::new(),
            raw_parameters: String::new(),
            position: 0,
            intelligence_override: None,
            mode_override: None,
        };

        let context = SharedContext::new();
        let result = orchestrator.execute_single(invocation, &context).await;

        assert!(result.is_ok());
        let execution = result.unwrap();
        assert_eq!(execution.status, SubagentStatus::Completed);
    }

    #[tokio::test]
    async fn test_sequential_execution() {
        let orchestrator = create_test_orchestrator().await;

        let chain = AgentChain {
            agents: vec![
                AgentInvocation {
                    agent_name: "test-agent".to_string(),
                    parameters: HashMap::new(),
                    raw_parameters: String::new(),
                    position: 0,
                    intelligence_override: None,
                    mode_override: None,
                },
                AgentInvocation {
                    agent_name: "test-agent".to_string(),
                    parameters: HashMap::new(),
                    raw_parameters: String::new(),
                    position: 1,
                    intelligence_override: None,
                    mode_override: None,
                },
            ],
            pass_output: true,
        };

        let context = SharedContext::new();
        let result = orchestrator.execute_sequential(chain, &context).await;

        assert!(result.is_ok());
        let executions = result.unwrap();
        assert_eq!(executions.len(), 2);
        assert!(
            executions
                .iter()
                .all(|e| e.status == SubagentStatus::Completed)
        );
    }

    #[tokio::test]
    async fn test_parallel_execution() {
        let orchestrator = create_test_orchestrator().await;

        let invocations = vec![
            AgentInvocation {
                agent_name: "test-agent".to_string(),
                parameters: HashMap::new(),
                raw_parameters: String::new(),
                position: 0,
                intelligence_override: None,
                mode_override: None,
            },
            AgentInvocation {
                agent_name: "test-agent".to_string(),
                parameters: HashMap::new(),
                raw_parameters: String::new(),
                position: 1,
                intelligence_override: None,
                mode_override: None,
            },
        ];

        let context = SharedContext::new();
        let result = orchestrator.execute_parallel(invocations, &context).await;

        assert!(result.is_ok());
        let executions = result.unwrap();
        assert_eq!(executions.len(), 2);
    }

    #[tokio::test]
    async fn test_context_sharing() {
        let context = SharedContext::new();

        // Set and get values
        context
            .set("key1".to_string(), serde_json::json!("value1"))
            .await;
        let value = context.get("key1").await;
        assert_eq!(value, Some(serde_json::json!("value1")));

        // Add outputs
        context.add_output("output1".to_string()).await;
        context.add_output("output2".to_string()).await;
        assert_eq!(context.last_output().await, Some("output2".to_string()));
        assert_eq!(context.all_outputs().await.len(), 2);

        // Snapshot and restore
        let snapshot = context.snapshot().await;
        context
            .set("key2".to_string(), serde_json::json!("value2"))
            .await;
        assert!(context.get("key2").await.is_some());

        context.restore(snapshot).await;
        assert!(context.get("key2").await.is_none());
        assert_eq!(context.get("key1").await, Some(serde_json::json!("value1")));
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let breaker = CircuitBreaker::new(3, Duration::from_secs(1));

        // Record failures
        for _ in 0..3 {
            breaker.record_failure().await;
        }

        // Circuit should be open
        assert!(breaker.is_open().await);

        // Wait for reset
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Circuit should be closed
        assert!(!breaker.is_open().await);

        // Record success
        breaker.record_success().await;
        assert!(!breaker.is_open().await);
    }

    #[tokio::test]
    async fn test_cancellation() {
        let orchestrator = create_test_orchestrator().await;

        orchestrator.cancel();

        let invocation = AgentInvocation {
            agent_name: "test-agent".to_string(),
            parameters: HashMap::new(),
            raw_parameters: String::new(),
            position: 0,
            intelligence_override: None,
            mode_override: None,
        };

        let context = SharedContext::new();
        let result = orchestrator.execute_single(invocation, &context).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SubagentError::ExecutionFailed(_)
        ));
    }
}
