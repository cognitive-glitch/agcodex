//! Integration example for the Double-Planning Strategy Tool
//!
//! This example demonstrates how to use the PlanTool with the AGCodex subagent system
//! to create, decompose, and execute complex development tasks.

use crate::tools::plan::PlanTool;

/// Example usage of the simplified planning tool
pub async fn example_usage() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("🎯 AGCodex Double-Planning Strategy Tool Example");
    tracing::info!("=================================================");

    // Initialize the planning tool
    let plan_tool = PlanTool::new();

    // Example 1: Simple Feature Addition
    tracing::info!("\n📋 Example 1: Adding a Dark Mode Toggle");
    tracing::info!("-----------------------------------------");

    let goal = "Add dark mode toggle to the application settings with persistent user preferences";

    // Step 1: Create simple plan
    let plan_result = plan_tool.plan(goal)?;
    tracing::info!("✅ Plan Created with {} tasks", plan_result.tasks.len());

    // For demonstration purposes, create a simple meta task
    let meta_task = crate::tools::plan::MetaTask {
        name: "Dark Mode Toggle".to_string(),
        description: goal.to_string(),
    };

    tracing::info!("✅ Meta Task Created:");
    tracing::info!("   Name: {}", meta_task.name);
    tracing::info!("   Description: {}", meta_task.description);

    // Step 2: Use the tasks from our plan result
    let sub_tasks = &plan_result.tasks;

    tracing::info!("\n🔧 Sub-Tasks Generated ({}):", sub_tasks.len());
    for (i, task) in sub_tasks.iter().enumerate() {
        tracing::info!("   {}. {} [ID: {}]", i + 1, task.description, task.id);
        tracing::info!(
            "      Dependencies: {} | Parallelizable: {}",
            task.depends_on.len(),
            task.can_parallelize
        );
    }

    // Step 3: Use parallelization from plan result
    let task_groups = &plan_result.parallel_groups;

    tracing::info!(
        "\n⚡ Task Groups for Parallel Execution ({}):",
        task_groups.len()
    );
    for (i, group) in task_groups.iter().enumerate() {
        tracing::info!("   Group {}: {} tasks", i + 1, group.len());
        for task_id in group {
            if let Some(task) = sub_tasks.iter().find(|t| &t.id == task_id) {
                tracing::info!("     - {}", task.description);
            }
        }
    }

    // Step 4: Show execution summary
    tracing::info!("\n🚀 Plan Summary:");
    tracing::info!("   Total Tasks: {}", plan_result.tasks.len());
    tracing::info!("   Complexity: {:?}", plan_result.estimated_complexity);
    tracing::info!("   Parallel Groups: {}", plan_result.parallel_groups.len());

    tracing::info!("\n🎉 Planning completed successfully!");

    // Example 2: Complex Refactoring
    tracing::info!("\n\n📋 Example 2: Complex Refactoring Task");
    tracing::info!("--------------------------------------");

    let refactor_goal = "Refactor the authentication system to use async/await patterns and add comprehensive error handling";

    let refactor_result = plan_tool.plan(refactor_goal)?;
    tracing::info!("✅ Refactoring Plan:");
    tracing::info!("   Complexity: {:?}", refactor_result.estimated_complexity);
    tracing::info!("   Tasks: {}", refactor_result.tasks.len());
    tracing::info!(
        "   Parallel Groups: {}",
        refactor_result.parallel_groups.len()
    );

    // Show refactoring tasks
    tracing::info!("\n🔧 Refactoring Sub-Tasks:");
    for (i, task) in refactor_result.tasks.iter().enumerate() {
        tracing::info!("   {}. {}", i + 1, task.description);
    }

    tracing::info!("\n✨ All examples completed successfully!");

    Ok(())
}

/// Helper function to create example context (simplified)
#[allow(dead_code)]
fn create_example_context() -> crate::tools::plan::PlanContext {
    crate::tools::plan::PlanContext {
        goal: "Example context".to_string(),
        constraints: vec!["No breaking changes".to_string()],
    }
}

/// Simulate analysis functionality (simplified)
pub async fn demonstrate_analysis_capabilities() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("\n🔍 AGCodex Analysis Capabilities");
    tracing::info!("=================================");

    let plan_tool = PlanTool::new();

    // Test different types of goals
    let test_goals = vec![
        "Fix memory leak in user session management",
        "Optimize database query performance",
        "Add comprehensive test suite",
        "Implement rate limiting for API endpoints",
        "Refactor legacy code to modern patterns",
    ];

    for goal in test_goals {
        tracing::info!("\n🎯 Planning: {}", goal);

        let result = plan_tool.plan(goal)?;
        tracing::info!("   Tasks: {}", result.tasks.len());
        tracing::info!("   Complexity: {:?}", result.estimated_complexity);
        tracing::info!("   Parallel Groups: {}", result.parallel_groups.len());

        if !result.tasks.is_empty() {
            tracing::info!("   First Task: {}", result.tasks[0].description);
        }
    }

    tracing::info!("\n✅ Analysis demonstration completed!");
    Ok(())
}
