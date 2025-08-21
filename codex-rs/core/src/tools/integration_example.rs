//! Integration example for the Double-Planning Strategy Tool
//!
//! This example demonstrates how to use the PlanTool with the AGCodex subagent system
//! to create, decompose, and execute complex development tasks.

use crate::modes::OperatingMode;
use crate::subagents::SubagentContext;
use crate::tools::plan::AgentType;
use crate::tools::plan::PlanTool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

/// Example usage of the double-planning strategy tool
pub async fn example_usage() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 AGCodex Double-Planning Strategy Tool Example");
    println!("=================================================");

    // Initialize the planning tool (in real usage, you'd pass actual semantic index and embeddings manager)
    let plan_tool = PlanTool::new(None, None);

    // Example 1: Simple Feature Addition
    println!("\n📋 Example 1: Adding a Dark Mode Toggle");
    println!("-----------------------------------------");

    let goal = "Add dark mode toggle to the application settings with persistent user preferences";

    // Step 1: Create meta task
    let meta_result = plan_tool.create_meta(goal).await?;
    let meta_task = meta_result.result;

    println!("✅ Meta Task Created:");
    println!("   ID: {}", meta_task.id);
    println!("   Goal: {}", meta_task.goal);
    println!("   Complexity: {}/10", meta_task.context.complexity_level);
    println!(
        "   Estimated Impact: {:.1}%",
        meta_task.estimated_impact * 100.0
    );
    println!("   Confidence: {:.1}%", meta_task.confidence * 100.0);
    println!("   Languages: {:?}", meta_task.context.languages);
    println!(
        "   Intelligence Required: {:?}",
        meta_task.intelligence_required
    );

    // Step 2: Decompose into sub-tasks
    let decompose_result = plan_tool.decompose(&meta_task).await?;
    let sub_tasks = decompose_result.result;

    println!("\n🔧 Sub-Tasks Generated ({}):", sub_tasks.len());
    for (i, task) in sub_tasks.iter().enumerate() {
        println!(
            "   {}. {} [{}]",
            i + 1,
            task.description,
            task.assigned_agent
                .as_ref()
                .map(|a| format!("{}", a))
                .unwrap_or("Unassigned".to_string())
        );
        println!(
            "      Duration: {}min | Priority: {:?} | Parallelizable: {}",
            task.estimated_duration.as_secs() / 60,
            task.priority,
            task.parallelizable
        );
    }

    // Step 3: Analyze parallelization opportunities
    let parallel_result = plan_tool.parallelize(&sub_tasks)?;
    let task_groups = parallel_result.result;

    println!(
        "\n⚡ Task Groups for Parallel Execution ({}):",
        task_groups.len()
    );
    for (i, group) in task_groups.iter().enumerate() {
        let group_type = if group.is_parallel() {
            "PARALLEL"
        } else {
            "SEQUENTIAL"
        };
        println!(
            "   Group {}: {} ({} tasks, {}min)",
            i + 1,
            group_type,
            group.tasks().len(),
            group.estimated_duration().as_secs() / 60
        );

        for task in group.tasks() {
            println!(
                "     - {} [{}]",
                task.description,
                task.assigned_agent
                    .as_ref()
                    .map(|a| format!("{}", a))
                    .unwrap_or("?".to_string())
            );
        }
    }

    // Step 4: Create execution plan
    let context = create_example_context();
    let execution_result = plan_tool.execute_plan(&meta_task, context).await?;
    let execution_plan = execution_result.result;

    println!("\n🚀 Execution Plan Created:");
    println!("   Plan ID: {}", execution_plan.id);
    println!("   Steps: {}", execution_plan.steps.len());
    println!(
        "   Total Estimated Duration: {}min",
        execution_plan.estimated_total_duration.as_secs() / 60
    );

    for (i, step) in execution_plan.steps.iter().enumerate() {
        println!("   Step {}: {} ({})", i + 1, step.name, step.status);
        println!("     Tasks: {}", step.agent_assignments.len());
        for (agent, tasks) in &step.agent_assignments {
            println!("       {}: {} tasks", agent, tasks.len());
        }
    }

    println!("\n📊 Planning Analysis Summary:");
    println!(
        "   Meta-Task Confidence: {:.1}%",
        meta_task.confidence * 100.0
    );
    println!("   Total Sub-Tasks: {}", sub_tasks.len());
    println!(
        "   Parallelizable Tasks: {}",
        sub_tasks.iter().filter(|t| t.parallelizable).count()
    );
    println!("   Task Groups: {}", task_groups.len());
    println!(
        "   Sequential Time: {}min",
        sub_tasks
            .iter()
            .map(|t| t.estimated_duration)
            .sum::<Duration>()
            .as_secs()
            / 60
    );
    println!(
        "   Parallel Time: {}min",
        task_groups
            .iter()
            .map(|g| g.estimated_duration())
            .sum::<Duration>()
            .as_secs()
            / 60
    );

    let time_savings = (sub_tasks
        .iter()
        .map(|t| t.estimated_duration)
        .sum::<Duration>()
        .as_secs() as i64)
        - (task_groups
            .iter()
            .map(|g| g.estimated_duration())
            .sum::<Duration>()
            .as_secs() as i64);
    println!(
        "   Time Savings: {}min ({:.1}% faster)",
        time_savings / 60,
        (time_savings as f64
            / sub_tasks
                .iter()
                .map(|t| t.estimated_duration)
                .sum::<Duration>()
                .as_secs() as f64)
            * 100.0
    );

    // Example 2: Complex Refactoring
    println!("\n\n📋 Example 2: Complex Refactoring Task");
    println!("--------------------------------------");

    let refactor_goal = "Refactor the authentication system to use async/await patterns and add comprehensive error handling";

    let refactor_meta = plan_tool.create_meta(refactor_goal).await?.result;
    let refactor_tasks = plan_tool.decompose(&refactor_meta).await?.result;
    let refactor_groups = plan_tool.parallelize(&refactor_tasks)?.result;

    println!("✅ Refactoring Plan:");
    println!(
        "   Complexity: {}/10",
        refactor_meta.context.complexity_level
    );
    println!("   Sub-Tasks: {}", refactor_tasks.len());
    println!("   Task Groups: {}", refactor_groups.len());
    println!(
        "   Estimated Impact: {:.1}%",
        refactor_meta.estimated_impact * 100.0
    );

    // Show agent type distribution
    let mut agent_distribution: HashMap<Option<AgentType>, usize> = HashMap::new();
    for task in &refactor_tasks {
        *agent_distribution
            .entry(task.assigned_agent.clone())
            .or_insert(0) += 1;
    }

    println!("\n🤖 Agent Assignment Distribution:");
    for (agent, count) in agent_distribution {
        let agent_name = agent
            .as_ref()
            .map(|a| format!("{}", a))
            .unwrap_or("Unassigned".to_string());
        println!("   {}: {} tasks", agent_name, count);
    }

    println!("\n🎉 Double-Planning Strategy Tool Demo Complete!");
    println!("   The tool successfully created meta-tasks, decomposed them into");
    println!("   parallelizable sub-tasks, and generated execution plans that can");
    println!("   be executed by the AGCodex subagent orchestration system.");

    Ok(())
}

/// Create an example subagent context for testing
fn create_example_context() -> SubagentContext {
    SubagentContext {
        execution_id: Uuid::new_v4(),
        mode: OperatingMode::Build,
        available_tools: vec![
            "Read".to_string(),
            "Write".to_string(),
            "Bash".to_string(),
            "Grep".to_string(),
            "AST-Search".to_string(),
        ],
        conversation_context: "User requested dark mode implementation with persistent preferences"
            .to_string(),
        working_directory: PathBuf::from("/home/user/project"),
        parameters: {
            let mut params = HashMap::new();
            params.insert("style".to_string(), "modern".to_string());
            params.insert("theme".to_string(), "adaptive".to_string());
            params
        },
        metadata: {
            let mut meta = HashMap::new();
            meta.insert(
                "project_type".to_string(),
                serde_json::json!("web-application"),
            );
            meta.insert(
                "tech_stack".to_string(),
                serde_json::json!(["rust", "typescript", "react"]),
            );
            meta
        },
    }
}

/// Demonstrate the planning tool with different goal types
pub async fn demonstrate_goal_types() -> Result<(), Box<dyn std::error::Error>> {
    let plan_tool = PlanTool::new(None, None);

    let goal_examples = vec![
        (
            "Add Feature",
            "Add user authentication with OAuth2 integration",
        ),
        ("Fix Bug", "Fix memory leak in the connection pool manager"),
        (
            "Optimize Performance",
            "Optimize database queries for user dashboard loading",
        ),
        (
            "Add Tests",
            "Add comprehensive integration tests for the API endpoints",
        ),
        (
            "Refactor",
            "Refactor the event handling system to use pub/sub pattern",
        ),
        (
            "Security",
            "Add rate limiting and input validation to all API endpoints",
        ),
    ];

    println!("\n🔍 Goal Type Analysis Demo");
    println!("===========================");

    for (goal_type, goal) in goal_examples {
        println!("\n📝 Goal Type: {}", goal_type);
        println!("   Goal: {}", goal);

        let meta_task = plan_tool.create_meta(goal).await?.result;
        let sub_tasks = plan_tool.decompose(&meta_task).await?.result;

        println!("   Sub-Tasks: {}", sub_tasks.len());
        println!("   Complexity: {}/10", meta_task.context.complexity_level);
        println!("   Confidence: {:.1}%", meta_task.confidence * 100.0);

        // Show primary agent types used
        let primary_agents: Vec<String> = sub_tasks
            .iter()
            .filter_map(|t| t.assigned_agent.as_ref())
            .map(|a| format!("{}", a))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .take(3)
            .collect();

        println!("   Primary Agents: {}", primary_agents.join(", "));

        if meta_task.context.affects_architecture {
            println!("   ⚠️  Affects Architecture");
        }
        if meta_task.context.security_sensitive {
            println!("   🔒 Security Sensitive");
        }
        if meta_task.context.performance_critical {
            println!("   ⚡ Performance Critical");
        }
    }

    Ok(())
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_integration_example() {
        // This would run the full integration example
        // For now, just test that we can create the tool
        let plan_tool = PlanTool::new(None, None);

        let result = plan_tool.create_meta("Test goal").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_goal_type_analysis() {
        let plan_tool = PlanTool::new(None, None);

        let goals = vec![
            "Add dark mode toggle",
            "Fix authentication bug",
            "Optimize query performance",
            "Add unit tests",
        ];

        for goal in goals {
            let result = plan_tool.create_meta(goal).await;
            assert!(result.is_ok());

            let meta_task = result.unwrap().result;
            assert!(!meta_task.goal.is_empty());
            assert!(meta_task.confidence > 0.0);
            assert!(meta_task.context.complexity_level >= 1);
            assert!(meta_task.context.complexity_level <= 10);
        }
    }
}

/// Example of how to integrate with the AGCodex TUI
pub fn tui_integration_example() {
    println!("🖥️  TUI Integration Example");
    println!("===========================");
    println!("In the AGCodex TUI, users would interact with the planning tool like this:");
    println!();
    println!("1. User types: 'Plan: Add user registration system'");
    println!("2. TUI calls PlanTool::create_meta() in the background");
    println!("3. TUI displays meta-task with complexity and confidence estimates");
    println!("4. User confirms or modifies the plan");
    println!("5. TUI calls PlanTool::decompose() to break down into sub-tasks");
    println!("6. TUI shows parallelization opportunities and time estimates");
    println!("7. User triggers execution via Shift+Enter");
    println!("8. TUI calls PlanTool::execute_plan() and starts agent orchestration");
    println!("9. Progress bars show parallel task execution in real-time");
    println!("10. User can cancel, pause, or branch the plan at any time");
    println!();
    println!("The planning tool integrates seamlessly with:");
    println!("- Mode switching (Plan/Build/Review modes affect complexity estimation)");
    println!("- Session persistence (plans are saved to ~/.agcodex/history)");
    println!("- Agent orchestration (automatic worktree management)");
    println!("- AST-based context analysis (smart language/framework detection)");
}
