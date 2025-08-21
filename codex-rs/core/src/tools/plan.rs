//! Double-Planning Strategy Tool for AGCodex
//!
//! This module implements a sophisticated two-level planning system:
//! 1. **MetaTaskPlanner**: Analyzes goals and creates high-level meta-tasks
//! 2. **SubTaskPlanner**: Decomposes meta-tasks into parallelizable sub-tasks
//!
//! ## Core Features
//! - Context-aware codebase analysis (language, frameworks, complexity)
//! - Automatic dependency graph construction
//! - Parallelization optimization with task grouping
//! - Integration with subagent orchestration system
//! - Impact and confidence estimation

use crate::context_engine::SemanticIndex;
use crate::embeddings::EmbeddingsManager;
use crate::subagents::AgentOrchestrator;
use crate::subagents::IntelligenceLevel;
use crate::subagents::SubagentContext;

use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::SystemTime;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;
use uuid::Uuid;

/// Errors specific to the planning tool
#[derive(Error, Debug)]
pub enum PlanError {
    #[error("circular dependency detected in task chain: {chain:?}")]
    CircularDependency { chain: Vec<String> },

    #[error("invalid goal format: {0}")]
    InvalidGoal(String),

    #[error("context analysis failed: {0}")]
    ContextAnalysisFailed(String),

    #[error("task decomposition failed: {0}")]
    DecompositionFailed(String),

    #[error("dependency resolution failed: {0}")]
    DependencyResolutionFailed(String),

    #[error("parallelization analysis failed: {0}")]
    ParallelizationFailed(String),

    #[error("agent assignment failed: {0}")]
    AgentAssignmentFailed(String),

    #[error("subagent error: {0}")]
    Subagent(#[from] crate::subagents::SubagentError),

    #[error("context engine error: {0}")]
    ContextEngine(String),
}

/// Result type for planning operations
pub type PlanResult<T> = std::result::Result<T, PlanError>;

/// Intelligence levels for planning analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanIntelligenceLevel {
    /// Basic planning with minimal analysis
    Light,
    /// Standard planning with moderate analysis  
    Medium,
    /// Maximum intelligence with deep analysis
    Hard,
}

impl From<PlanIntelligenceLevel> for IntelligenceLevel {
    fn from(level: PlanIntelligenceLevel) -> Self {
        match level {
            PlanIntelligenceLevel::Light => IntelligenceLevel::Light,
            PlanIntelligenceLevel::Medium => IntelligenceLevel::Medium,
            PlanIntelligenceLevel::Hard => IntelligenceLevel::Hard,
        }
    }
}

/// Status of a meta task or sub-task
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is waiting for dependencies
    Pending,
    /// Task is ready to execute
    Ready,
    /// Task is currently running
    InProgress,
    /// Task completed successfully  
    Completed,
    /// Task failed with an error
    Failed(String),
    /// Task was cancelled
    Cancelled,
}

/// Priority levels for tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

/// Agent type identifier for task assignment
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentType {
    CodeReviewer,
    Refactorer,
    Debugger,
    TestWriter,
    Performance,
    Security,
    Documentation,
    Architect,
    Custom(String),
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentType::CodeReviewer => write!(f, "code-reviewer"),
            AgentType::Refactorer => write!(f, "refactorer"),
            AgentType::Debugger => write!(f, "debugger"),
            AgentType::TestWriter => write!(f, "test-writer"),
            AgentType::Performance => write!(f, "performance"),
            AgentType::Security => write!(f, "security"),
            AgentType::Documentation => write!(f, "docs"),
            AgentType::Architect => write!(f, "architect"),
            AgentType::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// A high-level meta task that represents a major goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaTask {
    /// Unique identifier for this meta task
    pub id: Uuid,

    /// Human-readable goal description
    pub goal: String,

    /// Sub-tasks that compose this meta task
    pub sub_tasks: Vec<SubTask>,

    /// Dependency relationships between sub-tasks
    pub dependencies: DependencyGraph,

    /// Current status of the meta task
    pub status: TaskStatus,

    /// Intelligence level required for analysis
    pub intelligence_required: PlanIntelligenceLevel,

    /// Estimated impact on codebase (0.0-1.0)
    pub estimated_impact: f32,

    /// Confidence in plan accuracy (0.0-1.0)  
    pub confidence: f32,

    /// Languages and frameworks involved
    pub context: PlanContext,

    /// When this meta task was created
    pub created_at: SystemTime,

    /// Estimated total duration
    pub estimated_duration: Duration,
}

impl MetaTask {
    /// Create a new meta task
    pub fn new(goal: String, intelligence_required: PlanIntelligenceLevel) -> Self {
        Self {
            id: Uuid::new_v4(),
            goal,
            sub_tasks: Vec::new(),
            dependencies: DependencyGraph::new(),
            status: TaskStatus::Pending,
            intelligence_required,
            estimated_impact: 0.0,
            confidence: 0.0,
            context: PlanContext::default(),
            created_at: SystemTime::now(),
            estimated_duration: Duration::from_secs(0),
        }
    }

    /// Check if the meta task is ready for execution
    pub fn is_ready(&self) -> bool {
        matches!(self.status, TaskStatus::Ready) && !self.sub_tasks.is_empty()
    }

    /// Get tasks that can run in parallel
    pub fn get_parallel_tasks(&self) -> Vec<TaskGroup> {
        self.dependencies.get_parallel_groups(&self.sub_tasks)
    }

    /// Update status based on sub-task completion
    pub fn update_status(&mut self) {
        if self.sub_tasks.is_empty() {
            self.status = TaskStatus::Pending;
            return;
        }

        let completed = self
            .sub_tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Completed))
            .count();
        let failed = self
            .sub_tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Failed(_)))
            .count();
        let in_progress = self
            .sub_tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::InProgress))
            .count();

        if failed > 0 {
            self.status = TaskStatus::Failed(format!("{} sub-tasks failed", failed));
        } else if completed == self.sub_tasks.len() {
            self.status = TaskStatus::Completed;
        } else if in_progress > 0 {
            self.status = TaskStatus::InProgress;
        } else {
            self.status = TaskStatus::Ready;
        }
    }
}

/// A specific sub-task within a meta task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    /// Unique identifier for this sub-task
    pub id: Uuid,

    /// Parent meta task ID
    pub parent_meta: Uuid,

    /// Detailed description of what to do
    pub description: String,

    /// Agent type best suited for this task
    pub assigned_agent: Option<AgentType>,

    /// Whether this task can run in parallel with others
    pub parallelizable: bool,

    /// Estimated duration for completion
    pub estimated_duration: Duration,

    /// Current status
    pub status: TaskStatus,

    /// Priority level
    pub priority: TaskPriority,

    /// Files that will be modified
    pub target_files: Vec<PathBuf>,

    /// Expected output or deliverable
    pub expected_output: String,

    /// Dependencies on other sub-tasks
    pub dependencies: Vec<Uuid>,

    /// When this sub-task was created
    pub created_at: SystemTime,
}

impl SubTask {
    /// Create a new sub-task
    pub fn new(parent_meta: Uuid, description: String, estimated_duration: Duration) -> Self {
        Self {
            id: Uuid::new_v4(),
            parent_meta,
            description,
            assigned_agent: None,
            parallelizable: true,
            estimated_duration,
            status: TaskStatus::Pending,
            priority: TaskPriority::Medium,
            target_files: Vec::new(),
            expected_output: String::new(),
            dependencies: Vec::new(),
            created_at: SystemTime::now(),
        }
    }

    /// Check if this task is ready to execute (all dependencies completed)
    pub fn is_ready(&self, completed_tasks: &HashSet<Uuid>) -> bool {
        matches!(self.status, TaskStatus::Pending | TaskStatus::Ready)
            && self
                .dependencies
                .iter()
                .all(|dep_id| completed_tasks.contains(dep_id))
    }
}

/// Represents the context and complexity of a planning task
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanContext {
    /// Programming languages involved
    pub languages: Vec<String>,

    /// Frameworks and libraries detected
    pub frameworks: Vec<String>,

    /// Estimated complexity level (1-10)
    pub complexity_level: u8,

    /// Size of codebase in lines
    pub codebase_size: usize,

    /// Number of files involved
    pub file_count: usize,

    /// Whether this affects core architecture
    pub affects_architecture: bool,

    /// Security considerations
    pub security_sensitive: bool,

    /// Performance implications
    pub performance_critical: bool,
}

/// Manages dependency relationships between tasks
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyGraph {
    /// Adjacency list representation: task_id -> dependent_task_ids
    pub edges: HashMap<Uuid, Vec<Uuid>>,

    /// Reverse mapping for quick lookups: task_id -> tasks_that_depend_on_it
    pub reverse_edges: HashMap<Uuid, Vec<Uuid>>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a dependency relationship: `dependent` depends on `dependency`
    pub fn add_dependency(&mut self, dependent: Uuid, dependency: Uuid) {
        self.edges.entry(dependency).or_default().push(dependent);
        self.reverse_edges
            .entry(dependent)
            .or_default()
            .push(dependency);
    }

    /// Check for circular dependencies using DFS
    pub fn has_circular_dependency(&self) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for node in self.edges.keys() {
            if !visited.contains(node) {
                if self.has_cycle_dfs(*node, &mut visited, &mut rec_stack) {
                    return true;
                }
            }
        }
        false
    }

    /// DFS helper for cycle detection
    fn has_cycle_dfs(
        &self,
        node: Uuid,
        visited: &mut HashSet<Uuid>,
        rec_stack: &mut HashSet<Uuid>,
    ) -> bool {
        visited.insert(node);
        rec_stack.insert(node);

        if let Some(neighbors) = self.edges.get(&node) {
            for &neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    if self.has_cycle_dfs(neighbor, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(&neighbor) {
                    return true;
                }
            }
        }

        rec_stack.remove(&node);
        false
    }

    /// Get tasks that can run in parallel by grouping independent tasks
    pub fn get_parallel_groups(&self, tasks: &[SubTask]) -> Vec<TaskGroup> {
        let mut groups = Vec::new();
        let mut assigned = HashSet::new();

        // Group tasks by dependency level
        let levels = self.topological_levels(tasks);

        for level_tasks in levels {
            if level_tasks.len() == 1 {
                // Single task group
                let task = &level_tasks[0];
                if !assigned.contains(&task.id) {
                    groups.push(TaskGroup::Sequential(vec![task.clone()]));
                    assigned.insert(task.id);
                }
            } else {
                // Parallel task group
                let parallel_tasks: Vec<SubTask> = level_tasks
                    .into_iter()
                    .filter(|t| t.parallelizable && !assigned.contains(&t.id))
                    .collect();

                if !parallel_tasks.is_empty() {
                    for task in &parallel_tasks {
                        assigned.insert(task.id);
                    }
                    groups.push(TaskGroup::Parallel(parallel_tasks));
                }
            }
        }

        groups
    }

    /// Organize tasks into dependency levels for parallel execution
    fn topological_levels(&self, tasks: &[SubTask]) -> Vec<Vec<SubTask>> {
        let mut levels = Vec::new();
        let mut remaining: HashSet<Uuid> = tasks.iter().map(|t| t.id).collect();
        let task_map: HashMap<Uuid, &SubTask> = tasks.iter().map(|t| (t.id, t)).collect();

        while !remaining.is_empty() {
            let mut current_level = Vec::new();

            // Find tasks with no remaining dependencies
            for &task_id in &remaining {
                let task = task_map[&task_id];
                let has_unresolved_deps =
                    task.dependencies.iter().any(|dep| remaining.contains(dep));

                if !has_unresolved_deps {
                    current_level.push(task.clone());
                }
            }

            if current_level.is_empty() {
                warn!("Circular dependency detected in task graph");
                break;
            }

            // Remove processed tasks
            for task in &current_level {
                remaining.remove(&task.id);
            }

            levels.push(current_level);
        }

        levels
    }
}

/// Groups of tasks for execution  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskGroup {
    /// Tasks that must run sequentially
    Sequential(Vec<SubTask>),
    /// Tasks that can run in parallel
    Parallel(Vec<SubTask>),
}

impl TaskGroup {
    /// Get all tasks in this group
    pub fn tasks(&self) -> &[SubTask] {
        match self {
            TaskGroup::Sequential(tasks) | TaskGroup::Parallel(tasks) => tasks,
        }
    }

    /// Check if this group can run in parallel
    pub fn is_parallel(&self) -> bool {
        matches!(self, TaskGroup::Parallel(_))
    }

    /// Get estimated duration (parallel groups use max, sequential uses sum)
    pub fn estimated_duration(&self) -> Duration {
        match self {
            TaskGroup::Sequential(tasks) => tasks.iter().map(|t| t.estimated_duration).sum(),
            TaskGroup::Parallel(tasks) => tasks
                .iter()
                .map(|t| t.estimated_duration)
                .max()
                .unwrap_or_default(),
        }
    }
}

/// Output format for tool results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput<T> {
    /// The main result
    pub result: T,

    /// Success status
    pub success: bool,

    /// Any warnings or additional information
    pub messages: Vec<String>,

    /// Execution metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl<T> ToolOutput<T> {
    /// Create a successful output
    pub fn success(result: T) -> Self {
        Self {
            result,
            success: true,
            messages: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Create a failed output with error message
    pub fn failure(result: T, error: String) -> Self {
        Self {
            result,
            success: false,
            messages: vec![error],
            metadata: HashMap::new(),
        }
    }

    /// Add a message to the output
    pub fn with_message(mut self, message: String) -> Self {
        self.messages.push(message);
        self
    }

    /// Add metadata to the output
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Meta-level task planner that analyzes goals and creates meta-tasks
pub struct MetaTaskPlanner {
    /// Semantic index for analyzing codebase  
    semantic_index: Option<Arc<SemanticIndex>>,

    /// Embeddings manager for semantic analysis
    embeddings_manager: Option<Arc<EmbeddingsManager>>,

    /// Configuration for planning behavior
    config: PlannerConfig,
}

/// Sub-task planner that decomposes meta-tasks into executable sub-tasks
pub struct SubTaskPlanner {
    /// Configuration for decomposition
    config: PlannerConfig,

    /// Cache of previous decompositions for similar tasks
    decomposition_cache: Arc<Mutex<HashMap<String, Vec<SubTask>>>>,
}

/// Configuration for planners
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerConfig {
    /// Maximum number of sub-tasks per meta-task
    pub max_sub_tasks: usize,

    /// Default intelligence level
    pub default_intelligence: PlanIntelligenceLevel,

    /// Enable caching of decompositions
    pub enable_caching: bool,

    /// Timeout for analysis operations
    pub analysis_timeout: Duration,

    /// Maximum parallel task groups
    pub max_parallel_groups: usize,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            max_sub_tasks: 20,
            default_intelligence: PlanIntelligenceLevel::Medium,
            enable_caching: true,
            analysis_timeout: Duration::from_secs(30),
            max_parallel_groups: 4,
        }
    }
}

impl MetaTaskPlanner {
    /// Create a new meta task planner
    pub fn new(
        semantic_index: Option<Arc<SemanticIndex>>,
        embeddings_manager: Option<Arc<EmbeddingsManager>>,
    ) -> Self {
        Self {
            semantic_index,
            embeddings_manager,
            config: PlannerConfig::default(),
        }
    }

    /// Create a meta task from a goal string
    pub async fn create_meta(&self, goal: &str) -> PlanResult<MetaTask> {
        info!("Creating meta task for goal: {}", goal);

        if goal.trim().is_empty() {
            return Err(PlanError::InvalidGoal("Goal cannot be empty".to_string()));
        }

        let mut meta_task = MetaTask::new(goal.to_string(), self.config.default_intelligence);

        // Analyze the context
        if let Some(index) = &self.semantic_index {
            meta_task.context = self.analyze_context(goal, index).await?;
            meta_task.estimated_impact = self.estimate_impact(&meta_task.context);
            meta_task.confidence = self.estimate_confidence(&meta_task.context);
        }

        meta_task.status = TaskStatus::Ready;

        info!(
            "Created meta task {} with {} confidence",
            meta_task.id, meta_task.confidence
        );

        Ok(meta_task)
    }

    /// Analyze codebase context for a goal
    async fn analyze_context(&self, goal: &str, _index: &SemanticIndex) -> PlanResult<PlanContext> {
        debug!("Analyzing context for goal: {}", goal);

        let mut plan_context = PlanContext {
            complexity_level: 5,  // Default to medium complexity
            codebase_size: 10000, // Placeholder
            file_count: 100,      // Placeholder
            ..Default::default()
        };

        // Detect languages from goal and codebase
        plan_context.languages = self.detect_languages(goal);

        // Detect frameworks
        plan_context.frameworks = self.detect_frameworks(goal);

        // Analyze complexity
        plan_context.complexity_level = self.analyze_complexity(goal, &plan_context);

        // Check architectural impact
        plan_context.affects_architecture = self.affects_architecture(goal);

        // Security analysis
        plan_context.security_sensitive = self.is_security_sensitive(goal);

        // Performance analysis
        plan_context.performance_critical = self.is_performance_critical(goal);

        Ok(plan_context)
    }

    /// Detect programming languages involved
    fn detect_languages(&self, goal: &str) -> Vec<String> {
        let mut languages = Vec::new();
        let goal_lower = goal.to_lowercase();

        // Simple keyword-based detection (could be enhanced with ML)
        if goal_lower.contains("rust") || goal_lower.contains("cargo") {
            languages.push("rust".to_string());
        }
        if goal_lower.contains("javascript")
            || goal_lower.contains("js")
            || goal_lower.contains("node")
        {
            languages.push("javascript".to_string());
        }
        if goal_lower.contains("typescript") || goal_lower.contains("ts") {
            languages.push("typescript".to_string());
        }
        if goal_lower.contains("python") || goal_lower.contains("py") {
            languages.push("python".to_string());
        }
        if goal_lower.contains("java") {
            languages.push("java".to_string());
        }

        // If no languages detected, assume current project language(s)
        if languages.is_empty() {
            languages.push("rust".to_string()); // Default for AGCodex
        }

        languages
    }

    /// Detect frameworks and libraries  
    fn detect_frameworks(&self, goal: &str) -> Vec<String> {
        let mut frameworks = Vec::new();
        let goal_lower = goal.to_lowercase();

        // Framework detection
        if goal_lower.contains("react") {
            frameworks.push("react".to_string());
        }
        if goal_lower.contains("vue") {
            frameworks.push("vue".to_string());
        }
        if goal_lower.contains("angular") {
            frameworks.push("angular".to_string());
        }
        if goal_lower.contains("tokio") {
            frameworks.push("tokio".to_string());
        }
        if goal_lower.contains("axum") || goal_lower.contains("warp") {
            frameworks.push("web-server".to_string());
        }

        frameworks
    }

    /// Analyze complexity level (1-10)
    fn analyze_complexity(&self, goal: &str, context: &PlanContext) -> u8 {
        let mut complexity = 3; // Base complexity

        let goal_lower = goal.to_lowercase();

        // Increase complexity based on keywords
        if goal_lower.contains("refactor") || goal_lower.contains("restructure") {
            complexity += 2;
        }
        if goal_lower.contains("performance") || goal_lower.contains("optimize") {
            complexity += 2;
        }
        if goal_lower.contains("security") || goal_lower.contains("authentication") {
            complexity += 2;
        }
        if goal_lower.contains("database") || goal_lower.contains("migration") {
            complexity += 1;
        }
        if goal_lower.contains("api") || goal_lower.contains("endpoint") {
            complexity += 1;
        }

        // Adjust based on context
        if context.languages.len() > 2 {
            complexity += 1;
        }
        if context.frameworks.len() > 1 {
            complexity += 1;
        }

        complexity.min(10)
    }

    /// Check if goal affects core architecture
    fn affects_architecture(&self, goal: &str) -> bool {
        let goal_lower = goal.to_lowercase();
        goal_lower.contains("architecture")
            || goal_lower.contains("restructure")
            || goal_lower.contains("refactor")
            || goal_lower.contains("framework")
            || goal_lower.contains("design pattern")
    }

    /// Check if goal involves security
    fn is_security_sensitive(&self, goal: &str) -> bool {
        let goal_lower = goal.to_lowercase();
        goal_lower.contains("security")
            || goal_lower.contains("authentication")
            || goal_lower.contains("authorization")
            || goal_lower.contains("encryption")
            || goal_lower.contains("vulnerability")
    }

    /// Check if goal is performance critical
    fn is_performance_critical(&self, goal: &str) -> bool {
        let goal_lower = goal.to_lowercase();
        goal_lower.contains("performance")
            || goal_lower.contains("optimize")
            || goal_lower.contains("speed")
            || goal_lower.contains("latency")
            || goal_lower.contains("throughput")
    }

    /// Estimate impact on codebase (0.0-1.0)
    fn estimate_impact(&self, context: &PlanContext) -> f32 {
        let mut impact = 0.3; // Base impact

        // Adjust based on complexity
        impact += (context.complexity_level as f32) * 0.05;

        // Adjust based on scope
        if context.affects_architecture {
            impact += 0.3;
        }
        if context.security_sensitive {
            impact += 0.2;
        }
        if context.performance_critical {
            impact += 0.2;
        }

        impact.min(1.0)
    }

    /// Estimate confidence in plan (0.0-1.0)
    fn estimate_confidence(&self, context: &PlanContext) -> f32 {
        let mut confidence: f32 = 0.8; // Base confidence

        // Reduce confidence for high complexity
        if context.complexity_level > 7 {
            confidence -= 0.2;
        }

        // Reduce confidence for architectural changes
        if context.affects_architecture {
            confidence -= 0.1;
        }

        // Increase confidence for well-defined tasks
        if context.languages.len() <= 2 && context.frameworks.len() <= 1 {
            confidence += 0.1;
        }

        confidence.clamp(0.1, 1.0)
    }
}

impl SubTaskPlanner {
    /// Create a new sub-task planner
    pub fn new() -> Self {
        Self {
            config: PlannerConfig::default(),
            decomposition_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Decompose a meta task into sub-tasks
    pub async fn decompose(&self, meta: &MetaTask) -> PlanResult<Vec<SubTask>> {
        info!("Decomposing meta task: {}", meta.goal);

        // Check cache first
        if self.config.enable_caching {
            let cache = self
                .decomposition_cache
                .lock()
                .map_err(|e| PlanError::DecompositionFailed(format!("Cache lock failed: {}", e)))?;

            if let Some(cached) = cache.get(&meta.goal) {
                debug!("Using cached decomposition for: {}", meta.goal);
                return Ok(cached.clone());
            }
        }

        let sub_tasks = self.analyze_and_decompose(meta).await?;

        // Cache the result
        if self.config.enable_caching {
            let mut cache = self
                .decomposition_cache
                .lock()
                .map_err(|e| PlanError::DecompositionFailed(format!("Cache lock failed: {}", e)))?;
            cache.insert(meta.goal.clone(), sub_tasks.clone());
        }

        info!("Decomposed into {} sub-tasks", sub_tasks.len());
        Ok(sub_tasks)
    }

    /// Perform the actual decomposition analysis
    async fn analyze_and_decompose(&self, meta: &MetaTask) -> PlanResult<Vec<SubTask>> {
        let mut sub_tasks = Vec::new();
        let goal = &meta.goal;
        let goal_lower = goal.to_lowercase();

        // Pattern-based decomposition (could be enhanced with ML)
        if goal_lower.contains("add")
            && (goal_lower.contains("feature") || goal_lower.contains("component"))
        {
            sub_tasks.extend(self.decompose_add_feature(meta, goal));
        } else if goal_lower.contains("refactor") {
            sub_tasks.extend(self.decompose_refactor(meta, goal));
        } else if goal_lower.contains("fix") || goal_lower.contains("bug") {
            sub_tasks.extend(self.decompose_bug_fix(meta, goal));
        } else if goal_lower.contains("test") {
            sub_tasks.extend(self.decompose_testing(meta, goal));
        } else if goal_lower.contains("optimize") || goal_lower.contains("performance") {
            sub_tasks.extend(self.decompose_optimization(meta, goal));
        } else {
            // Generic decomposition
            sub_tasks.extend(self.decompose_generic(meta, goal));
        }

        // Assign agent types and set up dependencies
        self.assign_agents(&mut sub_tasks);
        self.setup_dependencies(&mut sub_tasks);

        Ok(sub_tasks)
    }

    /// Decompose "add feature" type goals
    fn decompose_add_feature(&self, meta: &MetaTask, goal: &str) -> Vec<SubTask> {
        let mut tasks = Vec::new();

        // 1. Design and planning
        tasks.push(SubTask {
            id: Uuid::new_v4(),
            parent_meta: meta.id,
            description: format!("Design architecture for: {}", goal),
            assigned_agent: Some(AgentType::Architect),
            parallelizable: false,
            estimated_duration: Duration::from_secs(1800), // 30 minutes
            status: TaskStatus::Pending,
            priority: TaskPriority::High,
            target_files: Vec::new(),
            expected_output: "Architectural design and implementation plan".to_string(),
            dependencies: Vec::new(),
            created_at: SystemTime::now(),
        });

        // 2. Core implementation
        tasks.push(SubTask {
            id: Uuid::new_v4(),
            parent_meta: meta.id,
            description: format!("Implement core functionality for: {}", goal),
            assigned_agent: Some(AgentType::Refactorer),
            parallelizable: false,
            estimated_duration: Duration::from_secs(3600), // 1 hour
            status: TaskStatus::Pending,
            priority: TaskPriority::High,
            target_files: Vec::new(),
            expected_output: "Core implementation with proper error handling".to_string(),
            dependencies: vec![tasks[0].id], // Depends on design
            created_at: SystemTime::now(),
        });

        // 3. Tests (can run in parallel with documentation)
        tasks.push(SubTask {
            id: Uuid::new_v4(),
            parent_meta: meta.id,
            description: format!("Write comprehensive tests for: {}", goal),
            assigned_agent: Some(AgentType::TestWriter),
            parallelizable: true,
            estimated_duration: Duration::from_secs(2400), // 40 minutes
            status: TaskStatus::Pending,
            priority: TaskPriority::Medium,
            target_files: Vec::new(),
            expected_output: "Unit, integration, and edge case tests".to_string(),
            dependencies: vec![tasks[1].id], // Depends on implementation
            created_at: SystemTime::now(),
        });

        // 4. Documentation (can run in parallel with tests)
        tasks.push(SubTask {
            id: Uuid::new_v4(),
            parent_meta: meta.id,
            description: format!("Document implementation for: {}", goal),
            assigned_agent: Some(AgentType::Documentation),
            parallelizable: true,
            estimated_duration: Duration::from_secs(1200), // 20 minutes
            status: TaskStatus::Pending,
            priority: TaskPriority::Low,
            target_files: Vec::new(),
            expected_output: "API documentation and usage examples".to_string(),
            dependencies: vec![tasks[1].id], // Depends on implementation
            created_at: SystemTime::now(),
        });

        // 5. Code review
        tasks.push(SubTask {
            id: Uuid::new_v4(),
            parent_meta: meta.id,
            description: format!("Review implementation for: {}", goal),
            assigned_agent: Some(AgentType::CodeReviewer),
            parallelizable: false,
            estimated_duration: Duration::from_secs(900), // 15 minutes
            status: TaskStatus::Pending,
            priority: TaskPriority::High,
            target_files: Vec::new(),
            expected_output: "Code review with quality and security analysis".to_string(),
            dependencies: vec![tasks[1].id, tasks[2].id], // After implementation and tests
            created_at: SystemTime::now(),
        });

        tasks
    }

    /// Decompose refactoring goals
    fn decompose_refactor(&self, meta: &MetaTask, goal: &str) -> Vec<SubTask> {
        let mut tasks = Vec::new();

        // 1. Analysis
        tasks.push(SubTask {
            id: Uuid::new_v4(),
            parent_meta: meta.id,
            description: format!("Analyze current code for: {}", goal),
            assigned_agent: Some(AgentType::Architect),
            parallelizable: false,
            estimated_duration: Duration::from_secs(1200),
            status: TaskStatus::Pending,
            priority: TaskPriority::High,
            target_files: Vec::new(),
            expected_output: "Analysis report with refactoring recommendations".to_string(),
            dependencies: Vec::new(),
            created_at: SystemTime::now(),
        });

        // 2. Refactoring
        tasks.push(SubTask {
            id: Uuid::new_v4(),
            parent_meta: meta.id,
            description: format!("Execute refactoring for: {}", goal),
            assigned_agent: Some(AgentType::Refactorer),
            parallelizable: false,
            estimated_duration: Duration::from_secs(2700),
            status: TaskStatus::Pending,
            priority: TaskPriority::High,
            target_files: Vec::new(),
            expected_output: "Refactored code with improved structure".to_string(),
            dependencies: vec![tasks[0].id],
            created_at: SystemTime::now(),
        });

        // 3. Test validation
        tasks.push(SubTask {
            id: Uuid::new_v4(),
            parent_meta: meta.id,
            description: format!("Validate refactoring with tests for: {}", goal),
            assigned_agent: Some(AgentType::TestWriter),
            parallelizable: false,
            estimated_duration: Duration::from_secs(1800),
            status: TaskStatus::Pending,
            priority: TaskPriority::High,
            target_files: Vec::new(),
            expected_output: "Test validation confirming refactoring success".to_string(),
            dependencies: vec![tasks[1].id],
            created_at: SystemTime::now(),
        });

        tasks
    }

    /// Decompose bug fix goals
    fn decompose_bug_fix(&self, meta: &MetaTask, goal: &str) -> Vec<SubTask> {
        let mut tasks = Vec::new();

        // 1. Investigation
        tasks.push(SubTask {
            id: Uuid::new_v4(),
            parent_meta: meta.id,
            description: format!("Investigate and diagnose: {}", goal),
            assigned_agent: Some(AgentType::Debugger),
            parallelizable: false,
            estimated_duration: Duration::from_secs(1800),
            status: TaskStatus::Pending,
            priority: TaskPriority::Critical,
            target_files: Vec::new(),
            expected_output: "Root cause analysis and fix strategy".to_string(),
            dependencies: Vec::new(),
            created_at: SystemTime::now(),
        });

        // 2. Fix implementation
        tasks.push(SubTask {
            id: Uuid::new_v4(),
            parent_meta: meta.id,
            description: format!("Implement fix for: {}", goal),
            assigned_agent: Some(AgentType::Refactorer),
            parallelizable: false,
            estimated_duration: Duration::from_secs(1200),
            status: TaskStatus::Pending,
            priority: TaskPriority::Critical,
            target_files: Vec::new(),
            expected_output: "Bug fix with proper error handling".to_string(),
            dependencies: vec![tasks[0].id],
            created_at: SystemTime::now(),
        });

        // 3. Regression tests
        tasks.push(SubTask {
            id: Uuid::new_v4(),
            parent_meta: meta.id,
            description: format!("Add regression tests for: {}", goal),
            assigned_agent: Some(AgentType::TestWriter),
            parallelizable: false,
            estimated_duration: Duration::from_secs(900),
            status: TaskStatus::Pending,
            priority: TaskPriority::High,
            target_files: Vec::new(),
            expected_output: "Regression tests to prevent future occurrences".to_string(),
            dependencies: vec![tasks[1].id],
            created_at: SystemTime::now(),
        });

        tasks
    }

    /// Decompose testing goals
    fn decompose_testing(&self, meta: &MetaTask, goal: &str) -> Vec<SubTask> {
        let mut tasks = Vec::new();

        // 1. Test analysis
        tasks.push(SubTask {
            id: Uuid::new_v4(),
            parent_meta: meta.id,
            description: format!("Analyze testing requirements for: {}", goal),
            assigned_agent: Some(AgentType::TestWriter),
            parallelizable: false,
            estimated_duration: Duration::from_secs(900),
            status: TaskStatus::Pending,
            priority: TaskPriority::Medium,
            target_files: Vec::new(),
            expected_output: "Test plan and coverage analysis".to_string(),
            dependencies: Vec::new(),
            created_at: SystemTime::now(),
        });

        // 2. Unit tests
        tasks.push(SubTask {
            id: Uuid::new_v4(),
            parent_meta: meta.id,
            description: format!("Write unit tests for: {}", goal),
            assigned_agent: Some(AgentType::TestWriter),
            parallelizable: true,
            estimated_duration: Duration::from_secs(1800),
            status: TaskStatus::Pending,
            priority: TaskPriority::High,
            target_files: Vec::new(),
            expected_output: "Comprehensive unit test suite".to_string(),
            dependencies: vec![tasks[0].id],
            created_at: SystemTime::now(),
        });

        // 3. Integration tests
        tasks.push(SubTask {
            id: Uuid::new_v4(),
            parent_meta: meta.id,
            description: format!("Write integration tests for: {}", goal),
            assigned_agent: Some(AgentType::TestWriter),
            parallelizable: true,
            estimated_duration: Duration::from_secs(2100),
            status: TaskStatus::Pending,
            priority: TaskPriority::Medium,
            target_files: Vec::new(),
            expected_output: "Integration tests for component interactions".to_string(),
            dependencies: vec![tasks[0].id],
            created_at: SystemTime::now(),
        });

        tasks
    }

    /// Decompose optimization goals
    fn decompose_optimization(&self, meta: &MetaTask, goal: &str) -> Vec<SubTask> {
        let mut tasks = Vec::new();

        // 1. Performance analysis
        tasks.push(SubTask {
            id: Uuid::new_v4(),
            parent_meta: meta.id,
            description: format!("Analyze performance bottlenecks for: {}", goal),
            assigned_agent: Some(AgentType::Performance),
            parallelizable: false,
            estimated_duration: Duration::from_secs(1500),
            status: TaskStatus::Pending,
            priority: TaskPriority::High,
            target_files: Vec::new(),
            expected_output: "Performance analysis and optimization plan".to_string(),
            dependencies: Vec::new(),
            created_at: SystemTime::now(),
        });

        // 2. Implementation
        tasks.push(SubTask {
            id: Uuid::new_v4(),
            parent_meta: meta.id,
            description: format!("Implement optimizations for: {}", goal),
            assigned_agent: Some(AgentType::Performance),
            parallelizable: false,
            estimated_duration: Duration::from_secs(2400),
            status: TaskStatus::Pending,
            priority: TaskPriority::High,
            target_files: Vec::new(),
            expected_output: "Optimized implementation with benchmarks".to_string(),
            dependencies: vec![tasks[0].id],
            created_at: SystemTime::now(),
        });

        // 3. Validation
        tasks.push(SubTask {
            id: Uuid::new_v4(),
            parent_meta: meta.id,
            description: format!("Validate performance improvements for: {}", goal),
            assigned_agent: Some(AgentType::Performance),
            parallelizable: false,
            estimated_duration: Duration::from_secs(900),
            status: TaskStatus::Pending,
            priority: TaskPriority::Medium,
            target_files: Vec::new(),
            expected_output: "Performance validation and benchmark results".to_string(),
            dependencies: vec![tasks[1].id],
            created_at: SystemTime::now(),
        });

        tasks
    }

    /// Generic decomposition for unknown goal types
    fn decompose_generic(&self, meta: &MetaTask, goal: &str) -> Vec<SubTask> {
        let mut tasks = Vec::new();

        // 1. Analysis
        tasks.push(SubTask {
            id: Uuid::new_v4(),
            parent_meta: meta.id,
            description: format!("Analyze requirements for: {}", goal),
            assigned_agent: Some(AgentType::Architect),
            parallelizable: false,
            estimated_duration: Duration::from_secs(1200),
            status: TaskStatus::Pending,
            priority: TaskPriority::Medium,
            target_files: Vec::new(),
            expected_output: "Requirements analysis and implementation strategy".to_string(),
            dependencies: Vec::new(),
            created_at: SystemTime::now(),
        });

        // 2. Implementation
        tasks.push(SubTask {
            id: Uuid::new_v4(),
            parent_meta: meta.id,
            description: format!("Implement solution for: {}", goal),
            assigned_agent: Some(AgentType::Refactorer),
            parallelizable: false,
            estimated_duration: Duration::from_secs(2400),
            status: TaskStatus::Pending,
            priority: TaskPriority::High,
            target_files: Vec::new(),
            expected_output: "Working implementation of requested changes".to_string(),
            dependencies: vec![tasks[0].id],
            created_at: SystemTime::now(),
        });

        // 3. Validation
        tasks.push(SubTask {
            id: Uuid::new_v4(),
            parent_meta: meta.id,
            description: format!("Validate and test solution for: {}", goal),
            assigned_agent: Some(AgentType::TestWriter),
            parallelizable: false,
            estimated_duration: Duration::from_secs(1800),
            status: TaskStatus::Pending,
            priority: TaskPriority::Medium,
            target_files: Vec::new(),
            expected_output: "Validation tests confirming solution works".to_string(),
            dependencies: vec![tasks[1].id],
            created_at: SystemTime::now(),
        });

        tasks
    }

    /// Assign appropriate agent types to tasks
    fn assign_agents(&self, tasks: &mut [SubTask]) {
        for task in tasks {
            if task.assigned_agent.is_none() {
                let description_lower = task.description.to_lowercase();

                if description_lower.contains("test") {
                    task.assigned_agent = Some(AgentType::TestWriter);
                } else if description_lower.contains("review") {
                    task.assigned_agent = Some(AgentType::CodeReviewer);
                } else if description_lower.contains("debug") || description_lower.contains("fix") {
                    task.assigned_agent = Some(AgentType::Debugger);
                } else if description_lower.contains("performance")
                    || description_lower.contains("optimize")
                {
                    task.assigned_agent = Some(AgentType::Performance);
                } else if description_lower.contains("security") {
                    task.assigned_agent = Some(AgentType::Security);
                } else if description_lower.contains("document") {
                    task.assigned_agent = Some(AgentType::Documentation);
                } else if description_lower.contains("design")
                    || description_lower.contains("architect")
                {
                    task.assigned_agent = Some(AgentType::Architect);
                } else {
                    task.assigned_agent = Some(AgentType::Refactorer); // Default
                }
            }
        }
    }

    /// Set up dependency relationships between tasks  
    fn setup_dependencies(&self, tasks: &mut [SubTask]) {
        // Dependencies are already set up in decomposition methods
        // This method can be used for additional dependency analysis

        // Ensure test tasks depend on implementation tasks
        for i in 0..tasks.len() {
            if tasks[i].description.to_lowercase().contains("test") {
                for j in 0..tasks.len() {
                    if i != j
                        && tasks[j].description.to_lowercase().contains("implement")
                        && !tasks[i].dependencies.contains(&tasks[j].id)
                    {
                        tasks[i].dependencies.push(tasks[j].id);
                    }
                }
            }
        }

        // Ensure review tasks depend on implementation tasks
        for i in 0..tasks.len() {
            if tasks[i].description.to_lowercase().contains("review") {
                for j in 0..tasks.len() {
                    if i != j
                        && (tasks[j].description.to_lowercase().contains("implement")
                            || tasks[j].description.to_lowercase().contains("test"))
                        && !tasks[i].dependencies.contains(&tasks[j].id)
                    {
                        tasks[i].dependencies.push(tasks[j].id);
                    }
                }
            }
        }
    }

    /// Analyze which tasks can be parallelized
    pub fn parallelize(&self, tasks: &[SubTask]) -> PlanResult<Vec<TaskGroup>> {
        let mut dependency_graph = DependencyGraph::new();

        // Build dependency graph
        for task in tasks {
            for dep_id in &task.dependencies {
                dependency_graph.add_dependency(task.id, *dep_id);
            }
        }

        // Check for circular dependencies
        if dependency_graph.has_circular_dependency() {
            return Err(PlanError::CircularDependency {
                chain: vec!["Detected in dependency graph".to_string()],
            });
        }

        // Get parallel groups
        let groups = dependency_graph.get_parallel_groups(tasks);

        info!(
            "Parallelized {} tasks into {} groups",
            tasks.len(),
            groups.len()
        );

        Ok(groups)
    }
}

impl Default for SubTaskPlanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Main planning tool that orchestrates meta and sub-task planning
pub struct PlanTool {
    /// Meta-level planner
    meta_planner: MetaTaskPlanner,

    /// Sub-task planner
    sub_planner: SubTaskPlanner,

    /// Agent orchestrator for execution
    orchestrator: Arc<RwLock<Option<AgentOrchestrator>>>,
}

impl PlanTool {
    /// Create a new planning tool
    pub fn new(
        semantic_index: Option<Arc<SemanticIndex>>,
        embeddings_manager: Option<Arc<EmbeddingsManager>>,
    ) -> Self {
        Self {
            meta_planner: MetaTaskPlanner::new(semantic_index, embeddings_manager),
            sub_planner: SubTaskPlanner::new(),
            orchestrator: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a meta task from a goal
    pub async fn create_meta(&self, goal: &str) -> PlanResult<ToolOutput<MetaTask>> {
        info!("Creating meta task for goal: {}", goal);

        let meta_task = self.meta_planner.create_meta(goal).await?;

        let metadata = {
            let mut map = HashMap::new();
            map.insert("task_id".to_string(), serde_json::json!(meta_task.id));
            map.insert(
                "complexity".to_string(),
                serde_json::json!(meta_task.context.complexity_level),
            );
            map.insert(
                "estimated_impact".to_string(),
                serde_json::json!(meta_task.estimated_impact),
            );
            map.insert(
                "confidence".to_string(),
                serde_json::json!(meta_task.confidence),
            );
            map
        };

        Ok(ToolOutput::success(meta_task)
            .with_metadata(
                "created_at".to_string(),
                serde_json::json!(
                    SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                ),
            )
            .with_metadata("analysis".to_string(), serde_json::json!(metadata)))
    }

    /// Decompose a meta task into sub-tasks
    pub async fn decompose(&self, meta: &MetaTask) -> PlanResult<ToolOutput<Vec<SubTask>>> {
        info!("Decomposing meta task: {}", meta.id);

        let sub_tasks = self.sub_planner.decompose(meta).await?;

        let metadata = {
            let mut map = HashMap::new();
            map.insert("meta_task_id".to_string(), serde_json::json!(meta.id));
            map.insert(
                "sub_task_count".to_string(),
                serde_json::json!(sub_tasks.len()),
            );

            let total_duration: Duration = sub_tasks.iter().map(|t| t.estimated_duration).sum();
            map.insert(
                "total_estimated_duration_secs".to_string(),
                serde_json::json!(total_duration.as_secs()),
            );

            let parallel_count = sub_tasks.iter().filter(|t| t.parallelizable).count();
            map.insert(
                "parallelizable_tasks".to_string(),
                serde_json::json!(parallel_count),
            );

            map
        };

        Ok(ToolOutput::success(sub_tasks)
            .with_message(format!(
                "Decomposed into {} sub-tasks",
                metadata["sub_task_count"]
            ))
            .with_metadata(
                "decomposition_analysis".to_string(),
                serde_json::json!(metadata),
            ))
    }

    /// Analyze parallelization opportunities
    pub fn parallelize(&self, tasks: &[SubTask]) -> PlanResult<ToolOutput<Vec<TaskGroup>>> {
        info!("Analyzing parallelization for {} tasks", tasks.len());

        let groups = self.sub_planner.parallelize(tasks)?;

        let parallel_groups = groups.iter().filter(|g| g.is_parallel()).count();
        let sequential_groups = groups.len() - parallel_groups;

        let total_parallel_duration: Duration = groups
            .iter()
            .filter(|g| g.is_parallel())
            .map(|g| g.estimated_duration())
            .sum();

        let total_sequential_duration: Duration = groups
            .iter()
            .filter(|g| !g.is_parallel())
            .map(|g| g.estimated_duration())
            .sum();

        let metadata = {
            let mut map = HashMap::new();
            map.insert("total_groups".to_string(), serde_json::json!(groups.len()));
            map.insert(
                "parallel_groups".to_string(),
                serde_json::json!(parallel_groups),
            );
            map.insert(
                "sequential_groups".to_string(),
                serde_json::json!(sequential_groups),
            );
            map.insert(
                "parallel_duration_secs".to_string(),
                serde_json::json!(total_parallel_duration.as_secs()),
            );
            map.insert(
                "sequential_duration_secs".to_string(),
                serde_json::json!(total_sequential_duration.as_secs()),
            );
            map.insert(
                "estimated_total_duration_secs".to_string(),
                serde_json::json!((total_parallel_duration + total_sequential_duration).as_secs()),
            );
            map
        };

        let group_len = groups.len();
        Ok(ToolOutput::success(groups)
            .with_message(format!(
                "Created {} task groups ({} parallel, {} sequential)",
                group_len, parallel_groups, sequential_groups
            ))
            .with_metadata(
                "parallelization_analysis".to_string(),
                serde_json::json!(metadata),
            ))
    }

    /// Set the agent orchestrator for execution
    pub async fn set_orchestrator(&self, orchestrator: AgentOrchestrator) -> PlanResult<()> {
        let mut guard = self.orchestrator.write().await;
        *guard = Some(orchestrator);
        Ok(())
    }

    /// Execute a plan using the agent orchestrator
    pub async fn execute_plan(
        &self,
        meta_task: &MetaTask,
        context: SubagentContext,
    ) -> PlanResult<ToolOutput<PlanExecutionPlan>> {
        info!("Executing plan for meta task: {}", meta_task.id);

        let orchestrator_guard = self.orchestrator.read().await;
        let _orchestrator = orchestrator_guard.as_ref().ok_or_else(|| {
            PlanError::AgentAssignmentFailed("No orchestrator configured".to_string())
        })?;

        // Convert task groups to execution plan
        let task_groups = meta_task.get_parallel_tasks();
        let mut execution_steps: Vec<PlanExecutionStep> = Vec::new();

        for (i, group) in task_groups.iter().enumerate() {
            let step = PlanExecutionStep {
                id: Uuid::new_v4(),
                name: format!("TaskGroup {}", i + 1),
                description: format!("Execute {} tasks", group.tasks().len()),
                agent_assignments: group
                    .tasks()
                    .iter()
                    .map(|t| {
                        (
                            t.assigned_agent
                                .as_ref()
                                .unwrap_or(&AgentType::Refactorer)
                                .to_string(),
                            vec![t.description.clone()],
                        )
                    })
                    .collect(),
                dependencies: if i > 0 {
                    vec![execution_steps[i - 1].id]
                } else {
                    Vec::new()
                },
                parallel_execution: group.is_parallel(),
                estimated_duration: group.estimated_duration(),
                status: "Pending".to_string(),
            };
            execution_steps.push(step);
        }

        let execution_plan = PlanExecutionPlan {
            id: Uuid::new_v4(),
            meta_task_id: meta_task.id,
            steps: execution_steps,
            created_at: SystemTime::now(),
            estimated_total_duration: task_groups.iter().map(|g| g.estimated_duration()).sum(),
            context: context.clone(),
        };

        let metadata = {
            let mut map = HashMap::new();
            map.insert(
                "execution_plan_id".to_string(),
                serde_json::json!(execution_plan.id),
            );
            map.insert(
                "step_count".to_string(),
                serde_json::json!(execution_plan.steps.len()),
            );
            map.insert(
                "estimated_duration_secs".to_string(),
                serde_json::json!(execution_plan.estimated_total_duration.as_secs()),
            );
            map
        };

        Ok(ToolOutput::success(execution_plan)
            .with_message(format!(
                "Created execution plan with {} steps",
                metadata["step_count"]
            ))
            .with_metadata("execution_plan".to_string(), serde_json::json!(metadata)))
    }

    /// Get the status of all active meta tasks
    pub fn get_active_tasks(&self) -> Vec<MetaTask> {
        // This would be implemented with persistent storage
        // For now, return empty list
        Vec::new()
    }

    /// Cancel a running meta task
    pub async fn cancel_task(&self, task_id: Uuid) -> PlanResult<bool> {
        // This would interact with the orchestrator to cancel execution
        info!("Cancelling task: {}", task_id);
        Ok(true)
    }
}

/// Plan execution types for integration with subagents
#[derive(Debug, Clone)]
pub struct PlanExecutionPlan {
    /// Unique identifier for this execution plan
    pub id: Uuid,

    /// Associated meta task
    pub meta_task_id: Uuid,

    /// Execution steps
    pub steps: Vec<PlanExecutionStep>,

    /// When this plan was created
    pub created_at: SystemTime,

    /// Estimated total duration
    pub estimated_total_duration: Duration,

    /// Execution context
    pub context: SubagentContext,
}

/// Individual execution step within a plan
#[derive(Debug, Clone)]
pub struct PlanExecutionStep {
    /// Unique identifier
    pub id: Uuid,

    /// Step name
    pub name: String,

    /// Step description
    pub description: String,

    /// Agent assignments (agent_name -> tasks)
    pub agent_assignments: HashMap<String, Vec<String>>,

    /// Dependencies on other steps
    pub dependencies: Vec<Uuid>,

    /// Whether this step can run in parallel
    pub parallel_execution: bool,

    /// Estimated duration
    pub estimated_duration: Duration,

    /// Current status (as string to avoid serde issues)
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_meta_task_creation() {
        let planner = MetaTaskPlanner::new(None, None);
        let result = planner.create_meta("Add dark mode toggle to the UI").await;

        assert!(result.is_ok());
        let meta_task = result.unwrap();
        assert_eq!(meta_task.goal, "Add dark mode toggle to the UI");
        assert!(matches!(meta_task.status, TaskStatus::Ready));
        assert!(meta_task.confidence > 0.0);
    }

    #[tokio::test]
    async fn test_sub_task_decomposition() {
        let meta_planner = MetaTaskPlanner::new(None, None);
        let sub_planner = SubTaskPlanner::new();

        let meta_task = meta_planner
            .create_meta("Add user authentication")
            .await
            .unwrap();
        let sub_tasks = sub_planner.decompose(&meta_task).await.unwrap();

        assert!(!sub_tasks.is_empty());
        assert!(sub_tasks.iter().any(|t| t.assigned_agent.is_some()));

        // Check that all sub-tasks belong to the meta task
        for task in &sub_tasks {
            assert_eq!(task.parent_meta, meta_task.id);
        }
    }

    #[test]
    fn test_dependency_graph() {
        let mut graph = DependencyGraph::new();
        let task1 = Uuid::new_v4();
        let task2 = Uuid::new_v4();
        let task3 = Uuid::new_v4();

        // task2 depends on task1, task3 depends on task2
        graph.add_dependency(task2, task1);
        graph.add_dependency(task3, task2);

        assert!(!graph.has_circular_dependency());

        // Add circular dependency: task1 depends on task3
        graph.add_dependency(task1, task3);
        assert!(graph.has_circular_dependency());
    }

    #[test]
    fn test_task_parallelization() {
        let planner = SubTaskPlanner::new();
        let meta_id = Uuid::new_v4();

        let task1 = SubTask::new(meta_id, "Task 1".to_string(), Duration::from_secs(60));
        let mut task2 = SubTask::new(meta_id, "Task 2".to_string(), Duration::from_secs(90));
        let mut task3 = SubTask::new(meta_id, "Task 3".to_string(), Duration::from_secs(120));

        // task2 depends on task1, task3 depends on task1 (task2 and task3 can run in parallel)
        task2.dependencies = vec![task1.id];
        task3.dependencies = vec![task1.id];

        let tasks = vec![task1, task2, task3];
        let groups = planner.parallelize(&tasks).unwrap();

        assert_eq!(groups.len(), 2); // One sequential (task1), one parallel (task2, task3)
    }

    #[tokio::test]
    async fn test_plan_tool_integration() {
        let tool = PlanTool::new(None, None);

        // Create meta task
        let meta_result = tool
            .create_meta("Implement user registration")
            .await
            .unwrap();
        let meta_task = meta_result.result;

        // Decompose into sub-tasks
        let decompose_result = tool.decompose(&meta_task).await.unwrap();
        let sub_tasks = decompose_result.result;

        // Analyze parallelization
        let parallel_result = tool.parallelize(&sub_tasks).unwrap();
        let task_groups = parallel_result.result;

        assert!(!sub_tasks.is_empty());
        assert!(!task_groups.is_empty());
        assert!(parallel_result.success);
    }

    #[test]
    fn test_task_group_duration_calculation() {
        let meta_id = Uuid::new_v4();
        let task1 = SubTask::new(meta_id, "Task 1".to_string(), Duration::from_secs(60));
        let task2 = SubTask::new(meta_id, "Task 2".to_string(), Duration::from_secs(90));

        let sequential_group = TaskGroup::Sequential(vec![task1.clone(), task2.clone()]);
        let parallel_group = TaskGroup::Parallel(vec![task1, task2]);

        assert_eq!(
            sequential_group.estimated_duration(),
            Duration::from_secs(150)
        ); // 60 + 90
        assert_eq!(parallel_group.estimated_duration(), Duration::from_secs(90)); // max(60, 90)
    }

    #[test]
    fn test_agent_type_assignment() {
        let mut sub_planner = SubTaskPlanner::new();
        let meta_id = Uuid::new_v4();

        let mut tasks = vec![
            SubTask::new(
                meta_id,
                "Write comprehensive tests".to_string(),
                Duration::from_secs(1800),
            ),
            SubTask::new(
                meta_id,
                "Review code for quality".to_string(),
                Duration::from_secs(900),
            ),
            SubTask::new(
                meta_id,
                "Debug authentication issue".to_string(),
                Duration::from_secs(1200),
            ),
            SubTask::new(
                meta_id,
                "Optimize database queries".to_string(),
                Duration::from_secs(2400),
            ),
        ];

        sub_planner.assign_agents(&mut tasks);

        assert_eq!(tasks[0].assigned_agent, Some(AgentType::TestWriter));
        assert_eq!(tasks[1].assigned_agent, Some(AgentType::CodeReviewer));
        assert_eq!(tasks[2].assigned_agent, Some(AgentType::Debugger));
        assert_eq!(tasks[3].assigned_agent, Some(AgentType::Performance));
    }
}
