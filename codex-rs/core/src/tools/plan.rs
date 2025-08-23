//! Task Planning Tool for AGCodex
//!
//! This module provides simple task decomposition with dependency analysis
//! and parallelization detection.
//!
//! ## Core Features
//! - Goal decomposition into actionable tasks
//! - Dependency graph construction
//! - Parallelization group identification
//! - Complexity estimation

use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

/// Errors specific to the planning tool
#[derive(Error, Debug)]
pub enum PlanError {
    #[error("circular dependency detected in task chain: {chain:?}")]
    CircularDependency { chain: Vec<String> },

    #[error("invalid goal format: {0}")]
    InvalidGoal(String),

    #[error("task decomposition failed: {0}")]
    DecompositionFailed(String),

    #[error("dependency resolution failed: {0}")]
    DependencyResolutionFailed(String),

    #[error("parallelization analysis failed: {0}")]
    ParallelizationFailed(String),
}

/// Result type for planning operations
pub type PlanResult<T> = std::result::Result<T, PlanError>;

/// Task identifier type
pub type TaskId = Uuid;

/// Complexity estimation for goals
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Complexity {
    Simple,
    Medium,
    Complex,
}

/// Simple task representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier for this task
    pub id: TaskId,

    /// Description of what to do
    pub description: String,

    /// Tasks this depends on
    pub depends_on: Vec<TaskId>,

    /// Whether this task can run in parallel with others
    pub can_parallelize: bool,
}

impl Task {
    /// Create a new task
    pub fn new(description: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            description,
            depends_on: Vec::new(),
            can_parallelize: true,
        }
    }

    /// Add a dependency to this task
    pub fn depends_on(mut self, dependency: TaskId) -> Self {
        self.depends_on.push(dependency);
        self
    }

    /// Mark this task as non-parallelizable
    pub const fn sequential(mut self) -> Self {
        self.can_parallelize = false;
        self
    }
}

/// Result of plan analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    /// List of tasks to execute
    pub tasks: Vec<Task>,

    /// Dependency relationships: task_id -> list of tasks it depends on
    pub dependency_graph: HashMap<TaskId, Vec<TaskId>>,

    /// Groups of tasks that can run in parallel
    pub parallel_groups: Vec<Vec<TaskId>>,

    /// Estimated complexity of the overall goal
    pub estimated_complexity: Complexity,
}

impl Plan {
    /// Create a new plan result
    pub fn new(tasks: Vec<Task>, complexity: Complexity) -> Self {
        let dependency_graph = Self::build_dependency_graph(&tasks);
        let parallel_groups = Self::identify_parallel_groups(&tasks, &dependency_graph);

        Self {
            tasks,
            dependency_graph,
            parallel_groups,
            estimated_complexity: complexity,
        }
    }

    /// Build dependency graph from tasks
    fn build_dependency_graph(tasks: &[Task]) -> HashMap<TaskId, Vec<TaskId>> {
        tasks
            .iter()
            .map(|task| (task.id, task.depends_on.clone()))
            .collect()
    }

    /// Find tasks that can run in parallel
    fn identify_parallel_groups(
        tasks: &[Task],
        _dependency_graph: &HashMap<TaskId, Vec<TaskId>>,
    ) -> Vec<Vec<TaskId>> {
        let mut groups = Vec::new();
        let mut processed = std::collections::HashSet::new();

        // Find tasks with no dependencies first
        let independent_tasks: Vec<TaskId> = tasks
            .iter()
            .filter(|task| task.depends_on.is_empty() && task.can_parallelize)
            .map(|task| task.id)
            .collect();

        if !independent_tasks.is_empty() {
            groups.push(independent_tasks.clone());
            processed.extend(independent_tasks);
        }

        // Group remaining tasks by dependency level
        while processed.len() < tasks.len() {
            let current_level: Vec<TaskId> = tasks
                .iter()
                .filter(|task| {
                    !processed.contains(&task.id)
                        && task.depends_on.iter().all(|dep| processed.contains(dep))
                        && task.can_parallelize
                })
                .map(|task| task.id)
                .collect();

            if current_level.is_empty() {
                // Handle sequential tasks or circular dependencies
                let remaining: Vec<TaskId> = tasks
                    .iter()
                    .filter(|task| !processed.contains(&task.id))
                    .map(|task| task.id)
                    .collect();

                for task_id in remaining {
                    groups.push(vec![task_id]);
                    processed.insert(task_id);
                }
                break;
            }

            if current_level.len() > 1 {
                groups.push(current_level.clone());
            } else {
                // Single task gets its own group
                groups.extend(current_level.iter().map(|&id| vec![id]));
            }

            processed.extend(current_level);
        }

        groups
    }
}

/// Simple task planning tool
pub struct PlanTool;

impl PlanTool {
    /// Create a new planning tool
    pub const fn new() -> Self {
        Self
    }

    /// Plan a goal into actionable tasks with dependency analysis
    pub fn plan(&self, goal: &str) -> PlanResult<Plan> {
        if goal.trim().is_empty() {
            return Err(PlanError::InvalidGoal("Goal cannot be empty".to_string()));
        }

        let tasks = self.decompose_goal(goal)?;
        let complexity = self.estimate_complexity(goal, &tasks);

        Ok(Plan::new(tasks, complexity))
    }

    /// Decompose a goal into concrete tasks
    fn decompose_goal(&self, goal: &str) -> PlanResult<Vec<Task>> {
        let goal_lower = goal.to_lowercase();
        let mut tasks;

        // Pattern-based task decomposition
        if goal_lower.contains("add")
            && (goal_lower.contains("feature") || goal_lower.contains("component"))
        {
            tasks = self.decompose_add_feature(goal);
        } else if goal_lower.contains("refactor") {
            tasks = self.decompose_refactor(goal);
        } else if goal_lower.contains("fix") || goal_lower.contains("bug") {
            tasks = self.decompose_bug_fix(goal);
        } else if goal_lower.contains("test") {
            tasks = self.decompose_testing(goal);
        } else if goal_lower.contains("optimize") || goal_lower.contains("performance") {
            tasks = self.decompose_optimization(goal);
        } else {
            tasks = self.decompose_generic(goal);
        }

        self.setup_dependencies(&mut tasks)?;
        Ok(tasks)
    }

    /// Decompose "add feature" goals
    fn decompose_add_feature(&self, goal: &str) -> Vec<Task> {
        let analyze = Task::new(format!("Analyze requirements for: {}", goal));
        let implement =
            Task::new(format!("Implement core functionality for: {}", goal)).depends_on(analyze.id);
        let test = Task::new(format!("Write tests for: {}", goal)).depends_on(implement.id);
        let document =
            Task::new(format!("Document implementation for: {}", goal)).depends_on(implement.id);
        let review = Task::new(format!("Review implementation for: {}", goal))
            .depends_on(implement.id)
            .depends_on(test.id)
            .sequential();

        vec![analyze, implement, test, document, review]
    }

    /// Decompose refactoring goals
    fn decompose_refactor(&self, goal: &str) -> Vec<Task> {
        let analyze = Task::new(format!("Analyze current code for: {}", goal));
        let refactor = Task::new(format!("Execute refactoring for: {}", goal))
            .depends_on(analyze.id)
            .sequential();
        let validate = Task::new(format!("Validate refactoring with tests for: {}", goal))
            .depends_on(refactor.id)
            .sequential();

        vec![analyze, refactor, validate]
    }

    /// Decompose bug fix goals
    fn decompose_bug_fix(&self, goal: &str) -> Vec<Task> {
        let investigate = Task::new(format!("Investigate and diagnose: {}", goal));
        let fix = Task::new(format!("Implement fix for: {}", goal))
            .depends_on(investigate.id)
            .sequential();
        let test = Task::new(format!("Add regression tests for: {}", goal))
            .depends_on(fix.id)
            .sequential();

        vec![investigate, fix, test]
    }

    /// Decompose testing goals
    fn decompose_testing(&self, goal: &str) -> Vec<Task> {
        let analyze = Task::new(format!("Analyze testing requirements for: {}", goal));
        let unit_tests =
            Task::new(format!("Write unit tests for: {}", goal)).depends_on(analyze.id);
        let integration_tests =
            Task::new(format!("Write integration tests for: {}", goal)).depends_on(analyze.id);

        vec![analyze, unit_tests, integration_tests]
    }

    /// Decompose optimization goals
    fn decompose_optimization(&self, goal: &str) -> Vec<Task> {
        let analyze = Task::new(format!("Analyze performance bottlenecks for: {}", goal));
        let optimize = Task::new(format!("Implement optimizations for: {}", goal))
            .depends_on(analyze.id)
            .sequential();
        let validate = Task::new(format!("Validate performance improvements for: {}", goal))
            .depends_on(optimize.id)
            .sequential();

        vec![analyze, optimize, validate]
    }

    /// Generic decomposition for unknown goal types
    fn decompose_generic(&self, goal: &str) -> Vec<Task> {
        let analyze = Task::new(format!("Analyze requirements for: {}", goal));
        let implement =
            Task::new(format!("Implement solution for: {}", goal)).depends_on(analyze.id);
        let validate =
            Task::new(format!("Validate and test solution for: {}", goal)).depends_on(implement.id);

        vec![analyze, implement, validate]
    }

    /// Setup additional dependencies between tasks
    fn setup_dependencies(&self, tasks: &mut [Task]) -> PlanResult<()> {
        // Check for circular dependencies
        self.check_circular_dependencies(tasks)?;
        Ok(())
    }

    /// Check for circular dependencies
    fn check_circular_dependencies(&self, tasks: &[Task]) -> PlanResult<()> {
        for task in tasks {
            if self.has_circular_dependency(task, tasks, &mut std::collections::HashSet::new())? {
                return Err(PlanError::CircularDependency {
                    chain: vec![format!("Task: {}", task.description)],
                });
            }
        }
        Ok(())
    }

    /// DFS check for circular dependencies
    fn has_circular_dependency(
        &self,
        task: &Task,
        all_tasks: &[Task],
        visited: &mut std::collections::HashSet<TaskId>,
    ) -> PlanResult<bool> {
        if visited.contains(&task.id) {
            return Ok(true);
        }

        visited.insert(task.id);

        for &dep_id in &task.depends_on {
            if let Some(dep_task) = all_tasks.iter().find(|t| t.id == dep_id)
                && self.has_circular_dependency(dep_task, all_tasks, visited)?
            {
                return Ok(true);
            }
        }

        visited.remove(&task.id);
        Ok(false)
    }

    /// Estimate complexity based on goal analysis
    fn estimate_complexity(&self, goal: &str, tasks: &[Task]) -> Complexity {
        let goal_lower = goal.to_lowercase();
        let task_count = tasks.len();

        // High complexity indicators
        if goal_lower.contains("refactor")
            || goal_lower.contains("architecture")
            || goal_lower.contains("performance")
            || goal_lower.contains("security")
            || task_count > 6
        {
            return Complexity::Complex;
        }

        // Medium complexity indicators
        if goal_lower.contains("feature")
            || goal_lower.contains("component")
            || goal_lower.contains("api")
            || task_count > 3
        {
            return Complexity::Medium;
        }

        // Default to simple
        Complexity::Simple
    }
}

impl Default for PlanTool {
    fn default() -> Self {
        Self::new()
    }
}

// Missing types expected by mod.rs - minimal stub implementations
pub type AgentType = String;
pub type DependencyGraph = HashMap<TaskId, Vec<TaskId>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaTask {
    pub name: String,
    pub description: String,
}

pub struct MetaTaskPlanner;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanContext {
    pub goal: String,
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExecutionPlan {
    pub steps: Vec<PlanExecutionStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExecutionStep {
    pub description: String,
    pub task_ids: Vec<TaskId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanIntelligenceLevel {
    Basic,
    Advanced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub id: TaskId,
    pub description: String,
    pub parent_id: Option<TaskId>,
}

pub struct SubTaskPlanner;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskGroup {
    pub name: String,
    pub tasks: Vec<TaskId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Complete,
    Failed,
}

// Simple ToolOutput for plan module (different from the comprehensive one in output module)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput<T> {
    pub result: T,
    pub summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_creation() {
        let tool = PlanTool::new();
        let result = tool.plan("Add dark mode toggle to the UI").unwrap();

        assert!(!result.tasks.is_empty());
        assert!(!result.parallel_groups.is_empty());
        assert_eq!(result.estimated_complexity, Complexity::Medium);
    }

    #[test]
    fn test_add_feature_decomposition() {
        let tool = PlanTool::new();
        let result = tool.plan("Add user authentication feature").unwrap();

        assert!(result.tasks.len() >= 3); // Should have multiple tasks
        assert!(
            result
                .tasks
                .iter()
                .any(|t| t.description.contains("Analyze"))
        );
        assert!(
            result
                .tasks
                .iter()
                .any(|t| t.description.contains("Implement"))
        );
    }

    #[test]
    fn test_refactor_decomposition() {
        let tool = PlanTool::new();
        let result = tool.plan("Refactor authentication system").unwrap();

        assert_eq!(result.estimated_complexity, Complexity::Complex);
        assert!(
            result
                .tasks
                .iter()
                .any(|t| t.description.contains("Analyze"))
        );
        assert!(
            result
                .tasks
                .iter()
                .any(|t| t.description.contains("refactoring"))
        );
    }

    #[test]
    fn test_parallel_group_identification() {
        let tool = PlanTool::new();
        let result = tool.plan("Add comprehensive test suite").unwrap();

        // Should have some parallel tasks (unit and integration tests)
        let has_parallel_group = result.parallel_groups.iter().any(|group| group.len() > 1);
        assert!(has_parallel_group);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let tool = PlanTool::new();

        // Create tasks with circular dependencies
        let task1 = Task::new("Task 1".to_string());
        let task2 = Task::new("Task 2".to_string()).depends_on(task1.id);
        let task3 = Task::new("Task 3".to_string())
            .depends_on(task2.id)
            .depends_on(task1.id); // This should be fine

        let tasks = vec![task1, task2, task3];
        let result = tool.check_circular_dependencies(&tasks);
        assert!(result.is_ok());
    }

    #[test]
    fn test_complexity_estimation() {
        let tool = PlanTool::new();

        let simple_result = tool.plan("Fix typo in readme").unwrap();
        assert_eq!(simple_result.estimated_complexity, Complexity::Simple);

        let complex_result = tool
            .plan("Refactor entire authentication architecture")
            .unwrap();
        assert_eq!(complex_result.estimated_complexity, Complexity::Complex);
    }
}
