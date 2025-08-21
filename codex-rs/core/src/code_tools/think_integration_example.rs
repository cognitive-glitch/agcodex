//! Integration examples for the ThinkTool with AGCodex
//!
//! This module demonstrates how the internal reasoning think tool integrates
//! with AGCodex's architecture and workflow.

use crate::code_tools::think::{ThinkTool, ThinkQuery, ProblemType, Context};
use crate::code_tools::CodeTool;
use std::collections::HashMap;

/// Example: Using think tool for code refactoring decisions
pub fn example_refactoring_reasoning() -> Result<(), Box<dyn std::error::Error>> {
    let think_tool = ThinkTool::new();
    
    let mut context = Context::default();
    context.references.push("src/main.rs:45-67".to_string());
    context.references.push("src/lib.rs:123-150".to_string());
    context.variables.insert("complexity".to_string(), "O(n²)".to_string());
    context.variables.insert("memory_usage".to_string(), "high".to_string());
    context.assumptions.push("Performance is critical".to_string());
    context.assumptions.push("Memory constraints exist".to_string());

    let query = ThinkQuery {
        problem: "This function has O(n²) complexity and high memory usage. How should we refactor it for better performance while maintaining readability?".to_string(),
        problem_type: Some(ProblemType::Systematic),
        preferred_strategy: Some("shannon".to_string()),
        context: Some(context),
        confidence_threshold: Some(0.8),
    };

    let output = think_tool.search(query)?;
    
    println!("🧠 Reasoning Session Started");
    println!("Strategy: {}", output.strategy);
    println!("Problem Type: {:?}", output.problem_type);
    println!("Confidence: {:.2}", output.confidence);
    println!("\nSummary: {}", output.summary);
    
    if let Some(next_action) = output.next_action {
        println!("Next Action: {}", next_action);
    }
    
    println!("\n--- Reasoning Trace ---");
    println!("{}", output.reasoning_trace);
    
    Ok(())
}

/// Example: Using think tool for architecture decisions
pub fn example_architecture_reasoning() -> Result<(), Box<dyn std::error::Error>> {
    let think_tool = ThinkTool::new();
    
    let mut context = Context::default();
    context.references.push("architecture/current_design.md".to_string());
    context.variables.insert("users".to_string(), "100k+".to_string());
    context.variables.insert("latency_requirement".to_string(), "<100ms".to_string());
    context.assumptions.push("Microservices architecture".to_string());
    context.assumptions.push("Cloud deployment".to_string());

    let query = ThinkQuery {
        problem: "We need to design a caching layer that can handle 100k+ concurrent users with sub-100ms latency. Should we use Redis, Memcached, or an in-memory solution?".to_string(),
        problem_type: Some(ProblemType::Evaluation), // Will use actor-critic
        preferred_strategy: None, // Auto-select
        context: Some(context),
        confidence_threshold: None, // Use default
    };

    let output = think_tool.search(query)?;
    
    println!("🏗️ Architecture Decision Reasoning");
    println!("Strategy: {} (auto-selected)", output.strategy);
    println!("Problem Type: {:?}", output.problem_type);
    
    println!("\n--- Reasoning Process ---");
    println!("{}", output.reasoning_trace);
    
    Ok(())
}

/// Example: Sequential reasoning for debugging
pub fn example_debugging_reasoning() -> Result<(), Box<dyn std::error::Error>> {
    let think_tool = ThinkTool::new();
    
    let mut context = Context::default();
    context.references.push("logs/error_2025-08-21.log".to_string());
    context.references.push("src/database/connection.rs:89".to_string());
    context.variables.insert("error_frequency".to_string(), "every 5 minutes".to_string());
    context.variables.insert("affected_users".to_string(), "15%".to_string());
    context.assumptions.push("Database connection issue".to_string());
    context.assumptions.push("Load-related problem".to_string());

    let query = ThinkQuery {
        problem: "Users are experiencing database timeouts every 5 minutes, affecting 15% of requests. Error logs show connection pool exhaustion. What's the systematic approach to debug and fix this?".to_string(),
        problem_type: Some(ProblemType::Sequential),
        preferred_strategy: Some("sequential".to_string()),
        context: Some(context),
        confidence_threshold: Some(0.7),
    };

    let output = think_tool.search(query)?;
    
    println!("🐛 Debugging Reasoning Session");
    println!("Using sequential thinking for step-by-step analysis");
    println!("Confidence threshold: 0.7");
    
    println!("\n{}", output.reasoning_trace);
    
    Ok(())
}

/// Example: Creative problem-solving for new feature design
pub fn example_creative_reasoning() -> Result<(), Box<dyn std::error::Error>> {
    let think_tool = ThinkTool::new();
    
    let mut context = Context::default();
    context.references.push("requirements/user_feedback.md".to_string());
    context.references.push("competitive_analysis.md".to_string());
    context.variables.insert("target_users".to_string(), "power users".to_string());
    context.variables.insert("development_time".to_string(), "2 weeks".to_string());
    context.assumptions.push("Limited development resources".to_string());
    context.assumptions.push("Must maintain existing UX".to_string());

    let query = ThinkQuery {
        problem: "Users want a 'smart suggestions' feature that learns from their coding patterns. How can we implement this creatively within 2 weeks while maintaining our existing UX?".to_string(),
        problem_type: Some(ProblemType::Creative),
        preferred_strategy: Some("actor-critic".to_string()),
        context: Some(context),
        confidence_threshold: Some(0.75),
    };

    let output = think_tool.search(query)?;
    
    println!("🎨 Creative Problem-Solving Session");
    println!("Using actor-critic approach for balanced innovation");
    
    println!("\n{}", output.reasoning_trace);
    
    Ok(())
}

/// Integration with AGCodex workflow
pub struct AGCodexThinkingIntegration {
    think_tool: ThinkTool,
}

impl AGCodexThinkingIntegration {
    pub fn new() -> Self {
        Self {
            think_tool: ThinkTool::new(),
        }
    }

    /// Think before executing a complex task
    pub fn think_before_action(&self, task_description: &str) -> Result<String, Box<dyn std::error::Error>> {
        let problem_type = self.think_tool.detect_problem_type(task_description);
        
        let query = ThinkQuery {
            problem: task_description.to_string(),
            problem_type: Some(problem_type.clone()),
            preferred_strategy: None, // Auto-select
            context: None,
            confidence_threshold: Some(0.8),
        };

        let output = self.think_tool.search(query)?;
        
        // Generate action plan based on reasoning
        let action_plan = format!(
            "# Reasoning-Based Action Plan\n\n\
            **Task:** {}\n\
            **Strategy:** {} (auto-selected for {:?})\n\
            **Confidence:** {:.2}\n\n\
            ## Reasoning Process\n\
            {}\n\n\
            ## Recommended Next Steps\n\
            {}\n",
            task_description,
            output.strategy,
            output.problem_type,
            output.confidence,
            output.reasoning_trace,
            output.next_action.unwrap_or_else(|| "Proceed with implementation".to_string())
        );

        Ok(action_plan)
    }

    /// Auto-reasoning for code review
    pub fn auto_code_review_reasoning(&self, file_path: &str, changes: &str) -> Result<String, Box<dyn std::error::Error>> {
        let mut context = Context::default();
        context.references.push(file_path.to_string());
        context.variables.insert("changes_size".to_string(), changes.len().to_string());

        let query = ThinkQuery {
            problem: format!("Review these code changes for quality, security, and maintainability:\n\n{}", changes),
            problem_type: Some(ProblemType::Evaluation),
            preferred_strategy: Some("actor-critic".to_string()),
            context: Some(context),
            confidence_threshold: Some(0.85),
        };

        let output = self.think_tool.search(query)?;
        
        Ok(format!(
            "# Automated Code Review Reasoning\n\n\
            **File:** {}\n\
            **Review Strategy:** Actor-Critic Analysis\n\
            **Confidence:** {:.2}\n\n\
            {}",
            file_path,
            output.confidence,
            output.reasoning_trace
        ))
    }
}

impl Default for AGCodexThinkingIntegration {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refactoring_reasoning_example() {
        let result = example_refactoring_reasoning();
        assert!(result.is_ok(), "Refactoring reasoning should work");
    }

    #[test]
    fn test_integration_think_before_action() {
        let integration = AGCodexThinkingIntegration::new();
        let result = integration.think_before_action("Refactor the database layer for better performance");
        
        assert!(result.is_ok());
        let action_plan = result.unwrap();
        assert!(action_plan.contains("Reasoning-Based Action Plan"));
        assert!(action_plan.contains("Strategy:"));
    }

    #[test]
    fn test_auto_code_review_reasoning() {
        let integration = AGCodexThinkingIntegration::new();
        let result = integration.auto_code_review_reasoning(
            "src/main.rs",
            "fn main() {\n    println!(\"Hello, world!\");\n}"
        );
        
        assert!(result.is_ok());
        let review = result.unwrap();
        assert!(review.contains("Automated Code Review Reasoning"));
        assert!(review.contains("Actor-Critic Analysis"));
    }
}