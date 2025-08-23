//! Execution context and progress tracking for AGCodex agents
//!
//! This module provides:
//! - Shared execution context with AST cache and findings
//! - Real-time progress reporting with ETA calculation
//! - Context save/restore with compression
//! - Inter-agent messaging with priorities
//! - Performance metrics tracking

use crate::code_tools::ast_agent_tools::Location as SourceLocation;
use crate::modes::OperatingMode;
use ast::types::ParsedAst;
use chrono::DateTime;
use chrono::Utc;
use dashmap::DashMap;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;
use thiserror::Error;
use tokio::sync::RwLock;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Errors that can occur in context operations
#[derive(Error, Debug)]
pub enum ContextError {
    #[error("context snapshot version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u32, actual: u32 },

    #[error("compression failed: {0}")]
    CompressionFailed(String),

    #[error("decompression failed: {0}")]
    DecompressionFailed(String),

    #[error("serialization failed: {0}")]
    SerializationFailed(String),

    #[error("message channel closed")]
    ChannelClosed,

    #[error("operation cancelled")]
    Cancelled,

    #[error("metric calculation failed: {0}")]
    MetricError(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type ContextResult<T> = Result<T, ContextError>;

/// Finding discovered by an agent (renamed to avoid conflict with agents::Finding)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFinding {
    pub id: Uuid,
    pub agent: String,
    pub severity: FindingSeverity,
    pub category: String,
    pub message: String,
    pub location: Option<SourceLocation>,
    pub suggestion: Option<String>,
    pub confidence: f32, // 0.0 to 1.0
    pub timestamp: DateTime<Utc>,
}

/// Severity levels for findings
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FindingSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Shared execution context for agents
#[derive(Clone)]
pub struct AgentContext {
    /// AST cache for parsed files
    pub ast_cache: Arc<DashMap<PathBuf, ParsedAst>>,

    /// Shared findings from all agents
    pub shared_findings: Arc<RwLock<Vec<ContextFinding>>>,

    /// User parameters for the session
    pub parameters: Arc<HashMap<String, serde_json::Value>>,

    /// Current operating mode
    pub mode: OperatingMode,

    /// Progress tracker for this context
    pub progress: Arc<ProgressTracker>,

    /// Session history access (optional)
    pub session_history: Arc<RwLock<Option<Vec<String>>>>,

    /// Cancellation token for graceful shutdown
    pub cancellation_token: Arc<CancellationToken>,

    /// Message bus for inter-agent communication
    message_bus: Arc<MessageBus>,

    /// Execution metrics
    metrics: Arc<ExecutionMetrics>,

    /// Context metadata
    metadata: Arc<DashMap<String, serde_json::Value>>,
}

impl AgentContext {
    /// Create a new agent context
    pub fn new(mode: OperatingMode, parameters: HashMap<String, serde_json::Value>) -> Self {
        Self {
            ast_cache: Arc::new(DashMap::new()),
            shared_findings: Arc::new(RwLock::new(Vec::new())),
            parameters: Arc::new(parameters),
            mode,
            progress: Arc::new(ProgressTracker::new()),
            session_history: Arc::new(RwLock::new(None)),
            cancellation_token: Arc::new(CancellationToken::new()),
            message_bus: Arc::new(MessageBus::new()),
            metrics: Arc::new(ExecutionMetrics::new()),
            metadata: Arc::new(DashMap::new()),
        }
    }

    /// Add a finding to the shared context
    pub async fn add_finding(&self, finding: ContextFinding) -> ContextResult<()> {
        self.check_cancelled()?;
        let mut findings = self.shared_findings.write().await;
        findings.push(finding);
        self.metrics.increment_findings();
        Ok(())
    }

    /// Get all findings matching a severity level
    pub async fn get_findings_by_severity(&self, severity: FindingSeverity) -> Vec<ContextFinding> {
        let findings = self.shared_findings.read().await;
        findings
            .iter()
            .filter(|f| f.severity == severity)
            .cloned()
            .collect()
    }

    /// Cache a parsed AST
    pub fn cache_ast(&self, path: PathBuf, ast: ParsedAst) {
        self.ast_cache.insert(path, ast);
        self.metrics.increment_files_processed();
    }

    /// Get a cached AST if available
    pub fn get_cached_ast(&self, path: &PathBuf) -> Option<ParsedAst> {
        self.ast_cache.get(path).map(|entry| entry.clone())
    }

    /// Check if operation has been cancelled
    pub fn check_cancelled(&self) -> ContextResult<()> {
        if self.cancellation_token.is_cancelled() {
            Err(ContextError::Cancelled)
        } else {
            Ok(())
        }
    }

    /// Send a message to other agents
    pub async fn send_message(&self, message: AgentMessage) -> ContextResult<()> {
        self.message_bus.send(message).await
    }

    /// Subscribe to messages for a specific agent
    pub fn subscribe(&self, agent_name: String) -> MessageReceiver {
        self.message_bus.subscribe(agent_name)
    }

    /// Create a snapshot of the current context
    pub async fn snapshot(&self) -> ContextResult<AgentContextSnapshot> {
        let findings = self.shared_findings.read().await;
        let history = self.session_history.read().await;

        let snapshot = AgentContextSnapshot {
            version: SNAPSHOT_VERSION,
            timestamp: Utc::now(),
            findings: findings.clone(),
            parameters: (*self.parameters).clone(),
            mode: self.mode,
            session_history: history.clone(),
            metadata: self.export_metadata(),
            metrics: self.metrics.snapshot(),
        };

        Ok(snapshot)
    }

    /// Restore context from a snapshot
    pub async fn restore(&mut self, snapshot: AgentContextSnapshot) -> ContextResult<()> {
        if snapshot.version != SNAPSHOT_VERSION {
            return Err(ContextError::VersionMismatch {
                expected: SNAPSHOT_VERSION,
                actual: snapshot.version,
            });
        }

        *self.shared_findings.write().await = snapshot.findings;
        self.parameters = Arc::new(snapshot.parameters);
        self.mode = snapshot.mode;
        *self.session_history.write().await = snapshot.session_history;
        self.import_metadata(snapshot.metadata);
        self.metrics.restore(snapshot.metrics);

        Ok(())
    }

    /// Export metadata as a HashMap
    fn export_metadata(&self) -> HashMap<String, serde_json::Value> {
        self.metadata
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// Import metadata from a HashMap
    fn import_metadata(&self, metadata: HashMap<String, serde_json::Value>) {
        self.metadata.clear();
        for (key, value) in metadata {
            self.metadata.insert(key, value);
        }
    }

    /// Get execution metrics
    pub fn metrics(&self) -> ExecutionMetricsSnapshot {
        self.metrics.snapshot()
    }
}

/// Progress tracker for real-time reporting
pub struct ProgressTracker {
    stages: RwLock<Vec<ProgressStage>>,
    current_stage: AtomicUsize,
    progress: AtomicU8,
    tx: mpsc::UnboundedSender<ProgressEvent>,
    rx: RwLock<mpsc::UnboundedReceiver<ProgressEvent>>,
    start_time: Instant,
    stage_history: RwLock<Vec<StageHistory>>,
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            stages: RwLock::new(Vec::new()),
            current_stage: AtomicUsize::new(0),
            progress: AtomicU8::new(0),
            tx,
            rx: RwLock::new(rx),
            start_time: Instant::now(),
            stage_history: RwLock::new(Vec::new()),
        }
    }

    /// Set the stages for this operation
    pub async fn set_stages(&self, stages: Vec<ProgressStage>) {
        *self.stages.write().await = stages;
        self.current_stage.store(0, Ordering::SeqCst);
        self.progress.store(0, Ordering::SeqCst);
    }

    /// Move to the next stage
    pub async fn next_stage(&self) -> ContextResult<()> {
        let stages = self.stages.read().await;
        let current = self.current_stage.load(Ordering::SeqCst);

        if current < stages.len() {
            // Record completion of current stage
            if current > 0 {
                let mut history = self.stage_history.write().await;
                history.push(StageHistory {
                    stage_index: current - 1,
                    duration: self.start_time.elapsed(),
                    completed_at: Instant::now(),
                });
            }

            self.current_stage.fetch_add(1, Ordering::SeqCst);
            self.progress.store(0, Ordering::SeqCst);

            let _ = self.tx.send(ProgressEvent::StageChanged {
                stage: current + 1,
                total_stages: stages.len(),
            });
        }

        Ok(())
    }

    /// Update progress within current stage (0-100)
    pub fn update_progress(&self, progress: u8) {
        let clamped = progress.min(100);
        self.progress.store(clamped, Ordering::SeqCst);

        let _ = self.tx.send(ProgressEvent::Progress {
            percentage: clamped,
            stage: self.current_stage.load(Ordering::SeqCst),
        });
    }

    /// Set a detailed status message
    pub fn set_status(&self, message: String) {
        let _ = self.tx.send(ProgressEvent::Status { message });
    }

    /// Calculate ETA based on historical data
    pub async fn calculate_eta(&self) -> Option<Duration> {
        let stages = self.stages.read().await;
        let current_stage = self.current_stage.load(Ordering::SeqCst);
        let history = self.stage_history.read().await;

        if stages.is_empty() || current_stage >= stages.len() {
            return None;
        }

        // Calculate average time per stage from history
        if history.is_empty() {
            // Estimate based on current progress
            let elapsed = self.start_time.elapsed();
            let progress = self.progress.load(Ordering::SeqCst) as f64 / 100.0;

            if progress > 0.01 {
                let estimated_total = elapsed.as_secs_f64() / progress;
                let remaining = estimated_total - elapsed.as_secs_f64();
                return Some(Duration::from_secs_f64(remaining));
            }
        } else {
            // Use historical data for better estimates
            let avg_stage_time = history
                .iter()
                .map(|h| h.duration.as_secs_f64())
                .sum::<f64>()
                / history.len() as f64;

            let remaining_stages = stages.len() - current_stage;
            let estimated_remaining = avg_stage_time * remaining_stages as f64;

            return Some(Duration::from_secs_f64(estimated_remaining));
        }

        None
    }

    /// Get current progress info
    pub async fn get_info(&self) -> ProgressInfo {
        let stages = self.stages.read().await;
        let current_stage = self.current_stage.load(Ordering::SeqCst);
        let progress = self.progress.load(Ordering::SeqCst);

        ProgressInfo {
            current_stage,
            total_stages: stages.len(),
            stage_progress: progress,
            current_stage_name: stages.get(current_stage).map(|s| s.name.clone()),
            eta: self.calculate_eta().await,
            elapsed: self.start_time.elapsed(),
        }
    }

    /// Receive progress updates
    pub async fn recv(&self) -> Option<ProgressEvent> {
        self.rx.write().await.recv().await
    }
}

/// A stage in the progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressStage {
    pub name: String,
    pub description: String,
    pub weight: f32, // Relative weight for overall progress calculation
}

/// History of completed stages
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct StageHistory {
    stage_index: usize,
    duration: Duration,
    completed_at: Instant,
}

/// Progress update message (renamed to avoid conflict with orchestrator::ProgressUpdate)
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    StageChanged { stage: usize, total_stages: usize },
    Progress { percentage: u8, stage: usize },
    Status { message: String },
    Completed,
    Failed { error: String },
}

/// Current progress information
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    pub current_stage: usize,
    pub total_stages: usize,
    pub stage_progress: u8,
    pub current_stage_name: Option<String>,
    pub eta: Option<Duration>,
    pub elapsed: Duration,
}

/// Context snapshot for save/restore (renamed to avoid conflict with orchestrator::ContextSnapshot)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContextSnapshot {
    pub version: u32,
    pub timestamp: DateTime<Utc>,
    pub findings: Vec<ContextFinding>,
    pub parameters: HashMap<String, serde_json::Value>,
    pub mode: OperatingMode,
    pub session_history: Option<Vec<String>>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub metrics: ExecutionMetricsSnapshot,
}

const SNAPSHOT_VERSION: u32 = 1;

impl AgentContextSnapshot {
    /// Compress the snapshot using zstd
    pub fn compress(&self) -> ContextResult<Vec<u8>> {
        let config = bincode::config::standard();
        let serialized = bincode::serde::encode_to_vec(self, config)
            .map_err(|e| ContextError::SerializationFailed(e.to_string()))?;

        // Note: zstd compression would be added here if the crate is added to dependencies
        // For now, return uncompressed
        Ok(serialized)
    }

    /// Decompress a snapshot
    pub fn decompress(data: &[u8]) -> ContextResult<Self> {
        // Note: zstd decompression would be added here if the crate is added to dependencies
        // For now, treat as uncompressed
        let config = bincode::config::standard();
        let (snapshot, _) = bincode::serde::decode_from_slice(data, config)
            .map_err(|e| ContextError::SerializationFailed(e.to_string()))?;
        Ok(snapshot)
    }

    /// Merge with another snapshot (for parallel agent results)
    pub fn merge(&mut self, other: AgentContextSnapshot) -> ContextResult<()> {
        // Merge findings
        self.findings.extend(other.findings);

        // Merge metadata (other overwrites self for conflicts)
        for (key, value) in other.metadata {
            self.metadata.insert(key, value);
        }

        // Merge metrics
        self.metrics.merge(other.metrics);

        self.timestamp = Utc::now();
        Ok(())
    }
}

/// Inter-agent message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: Uuid,
    pub from: String,
    pub to: MessageTarget,
    pub message_type: MessageType,
    pub priority: MessagePriority,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

/// Message targeting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageTarget {
    Agent(String),
    Broadcast,
    Group(Vec<String>),
}

/// Message types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageType {
    Info,
    Warning,
    Error,
    Result,
    Request,
    Response,
    Coordination,
}

/// Message priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MessagePriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Message bus for inter-agent communication
struct MessageBus {
    subscribers: Arc<DashMap<String, mpsc::UnboundedSender<AgentMessage>>>,
    _broadcast_tx: mpsc::UnboundedSender<AgentMessage>,
    _broadcast_rx: Arc<RwLock<mpsc::UnboundedReceiver<AgentMessage>>>,
}

impl MessageBus {
    fn new() -> Self {
        let (broadcast_tx, broadcast_rx) = mpsc::unbounded_channel();
        Self {
            subscribers: Arc::new(DashMap::new()),
            _broadcast_tx: broadcast_tx,
            _broadcast_rx: Arc::new(RwLock::new(broadcast_rx)),
        }
    }

    async fn send(&self, message: AgentMessage) -> ContextResult<()> {
        match &message.to {
            MessageTarget::Agent(name) => {
                if let Some(tx) = self.subscribers.get(name) {
                    tx.send(message).map_err(|_| ContextError::ChannelClosed)?;
                }
            }
            MessageTarget::Broadcast => {
                for entry in self.subscribers.iter() {
                    let _ = entry.value().send(message.clone());
                }
            }
            MessageTarget::Group(agents) => {
                for agent in agents {
                    if let Some(tx) = self.subscribers.get(agent) {
                        let _ = tx.send(message.clone());
                    }
                }
            }
        }
        Ok(())
    }

    fn subscribe(&self, agent_name: String) -> MessageReceiver {
        let (tx, rx) = mpsc::unbounded_channel();
        self.subscribers.insert(agent_name.clone(), tx);
        MessageReceiver { rx, agent_name }
    }
}

/// Message receiver for an agent
pub struct MessageReceiver {
    rx: mpsc::UnboundedReceiver<AgentMessage>,
    agent_name: String,
}

impl MessageReceiver {
    /// Receive the next message
    pub async fn recv(&mut self) -> Option<AgentMessage> {
        self.rx.recv().await
    }

    /// Try to receive without blocking
    pub fn try_recv(&mut self) -> Option<AgentMessage> {
        self.rx.try_recv().ok()
    }

    /// Get the agent name for this receiver
    pub fn agent_name(&self) -> &str {
        &self.agent_name
    }
}

/// Execution metrics for performance tracking
struct ExecutionMetrics {
    start_time: Instant,
    files_processed: AtomicUsize,
    findings_generated: AtomicUsize,
    memory_allocated: AtomicUsize,
    cache_hits: AtomicUsize,
    cache_misses: AtomicUsize,
}

impl ExecutionMetrics {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
            files_processed: AtomicUsize::new(0),
            findings_generated: AtomicUsize::new(0),
            memory_allocated: AtomicUsize::new(0),
            cache_hits: AtomicUsize::new(0),
            cache_misses: AtomicUsize::new(0),
        }
    }

    fn increment_files_processed(&self) {
        self.files_processed.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_findings(&self) {
        self.findings_generated.fetch_add(1, Ordering::Relaxed);
    }

    fn _record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    fn _record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot(&self) -> ExecutionMetricsSnapshot {
        let cache_total =
            self.cache_hits.load(Ordering::Relaxed) + self.cache_misses.load(Ordering::Relaxed);

        let cache_hit_rate = if cache_total > 0 {
            self.cache_hits.load(Ordering::Relaxed) as f64 / cache_total as f64
        } else {
            0.0
        };

        ExecutionMetricsSnapshot {
            elapsed: self.start_time.elapsed(),
            files_processed: self.files_processed.load(Ordering::Relaxed),
            findings_generated: self.findings_generated.load(Ordering::Relaxed),
            memory_allocated: self.memory_allocated.load(Ordering::Relaxed),
            cache_hit_rate,
            files_per_second: self.calculate_throughput(),
        }
    }

    fn calculate_throughput(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.files_processed.load(Ordering::Relaxed) as f64 / elapsed
        } else {
            0.0
        }
    }

    fn restore(&self, snapshot: ExecutionMetricsSnapshot) {
        self.files_processed
            .store(snapshot.files_processed, Ordering::Relaxed);
        self.findings_generated
            .store(snapshot.findings_generated, Ordering::Relaxed);
        self.memory_allocated
            .store(snapshot.memory_allocated, Ordering::Relaxed);
    }
}

/// Snapshot of execution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetricsSnapshot {
    pub elapsed: Duration,
    pub files_processed: usize,
    pub findings_generated: usize,
    pub memory_allocated: usize,
    pub cache_hit_rate: f64,
    pub files_per_second: f64,
}

impl ExecutionMetricsSnapshot {
    /// Merge with another metrics snapshot
    pub fn merge(&mut self, other: ExecutionMetricsSnapshot) {
        self.files_processed += other.files_processed;
        self.findings_generated += other.findings_generated;
        self.memory_allocated = self.memory_allocated.max(other.memory_allocated);

        // Weighted average for cache hit rate
        let total = self.files_processed + other.files_processed;
        if total > 0 {
            self.cache_hit_rate = (self.cache_hit_rate * self.files_processed as f64
                + other.cache_hit_rate * other.files_processed as f64)
                / total as f64;
        }
    }
}

/// Cancellation token for graceful shutdown
#[derive(Debug, Clone)]
pub struct CancellationToken {
    inner: Arc<CancellationTokenInner>,
}

#[derive(Debug)]
struct CancellationTokenInner {
    cancelled: AtomicBool,
    waiters: RwLock<Vec<tokio::sync::oneshot::Sender<()>>>,
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationToken {
    /// Create a new cancellation token
    pub fn new() -> Self {
        Self {
            inner: Arc::new(CancellationTokenInner {
                cancelled: AtomicBool::new(false),
                waiters: RwLock::new(Vec::new()),
            }),
        }
    }

    /// Cancel all operations
    pub async fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::SeqCst);
        let mut waiters = self.inner.waiters.write().await;
        for waiter in waiters.drain(..) {
            let _ = waiter.send(());
        }
    }

    /// Check if cancelled
    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
    }

    /// Wait for cancellation
    pub async fn cancelled(&self) {
        // Early check for already cancelled
        if self.is_cancelled() {
            return;
        }

        let (tx, rx) = tokio::sync::oneshot::channel();

        // Add waiter while holding the lock
        {
            let mut waiters = self.inner.waiters.write().await;

            // Double-check cancellation state after acquiring the lock
            // This prevents the race condition where cancellation happens
            // between the first check and adding the waiter
            if self.is_cancelled() {
                return;
            }

            waiters.push(tx);
        }

        // Now wait for cancellation signal
        let _ = rx.await;
    }

    /// Create a child token that cancels when parent cancels
    pub fn child(&self) -> CancellationToken {
        let child = CancellationToken::new();
        let child_clone = child.clone();
        let parent = self.clone();

        tokio::spawn(async move {
            parent.cancelled().await;
            child_clone.cancel().await;
        });

        child
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_context_creation() {
        let params = HashMap::new();
        let context = AgentContext::new(OperatingMode::Build, params);

        assert_eq!(context.mode, OperatingMode::Build);
        assert!(!context.cancellation_token.is_cancelled());
    }

    #[tokio::test]
    async fn test_finding_management() {
        let context = AgentContext::new(OperatingMode::Review, HashMap::new());

        let finding = ContextFinding {
            id: Uuid::new_v4(),
            agent: "test-agent".to_string(),
            severity: FindingSeverity::Warning,
            category: "code-quality".to_string(),
            message: "Test finding".to_string(),
            location: None,
            suggestion: None,
            confidence: 0.8,
            timestamp: Utc::now(),
        };

        context.add_finding(finding.clone()).await.unwrap();

        let warnings = context
            .get_findings_by_severity(FindingSeverity::Warning)
            .await;
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].message, "Test finding");
    }

    #[tokio::test]
    async fn test_progress_tracking() {
        let tracker = ProgressTracker::new();

        let stages = vec![
            ProgressStage {
                name: "Analysis".to_string(),
                description: "Analyzing code".to_string(),
                weight: 1.0,
            },
            ProgressStage {
                name: "Processing".to_string(),
                description: "Processing results".to_string(),
                weight: 1.0,
            },
        ];

        tracker.set_stages(stages).await;
        tracker.update_progress(50);

        let info = tracker.get_info().await;
        assert_eq!(info.current_stage, 0);
        assert_eq!(info.total_stages, 2);
        assert_eq!(info.stage_progress, 50);
    }

    #[tokio::test]
    async fn test_context_snapshot() {
        let context = AgentContext::new(OperatingMode::Plan, HashMap::new());

        let finding = ContextFinding {
            id: Uuid::new_v4(),
            agent: "test-agent".to_string(),
            severity: FindingSeverity::Info,
            category: "test".to_string(),
            message: "Test finding".to_string(),
            location: None,
            suggestion: None,
            confidence: 1.0,
            timestamp: Utc::now(),
        };

        context.add_finding(finding.clone()).await.unwrap();

        let snapshot = context.snapshot().await.unwrap();
        assert_eq!(snapshot.findings.len(), 1);
        assert_eq!(snapshot.mode, OperatingMode::Plan);

        // Test compression/decompression
        let compressed = snapshot.compress().unwrap();
        let restored = AgentContextSnapshot::decompress(&compressed).unwrap();
        assert_eq!(restored.findings.len(), 1);
    }

    #[tokio::test]
    async fn test_cancellation_token() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());

        let token_clone = token.clone();
        let handle = tokio::spawn(async move {
            token_clone.cancelled().await;
        });

        tokio::time::sleep(Duration::from_millis(10)).await;
        token.cancel().await;

        handle.await.unwrap();
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn test_message_bus() {
        let context = AgentContext::new(OperatingMode::Build, HashMap::new());

        let mut receiver = context.subscribe("agent-1".to_string());

        let message = AgentMessage {
            id: Uuid::new_v4(),
            from: "agent-2".to_string(),
            to: MessageTarget::Agent("agent-1".to_string()),
            message_type: MessageType::Info,
            priority: MessagePriority::Normal,
            payload: serde_json::json!({"test": "data"}),
            timestamp: Utc::now(),
        };

        context.send_message(message.clone()).await.unwrap();

        let received = receiver.recv().await.unwrap();
        assert_eq!(received.from, "agent-2");
        assert_eq!(received.payload["test"], "data");
    }

    #[tokio::test]
    async fn test_metrics_tracking() {
        let context = AgentContext::new(OperatingMode::Build, HashMap::new());

        // Simulate processing
        context.metrics.increment_files_processed();
        context.metrics.increment_files_processed();
        context.metrics.increment_findings();

        let metrics = context.metrics();
        assert_eq!(metrics.files_processed, 2);
        assert_eq!(metrics.findings_generated, 1);
    }
}
