//! Integration examples for the simplified ThinkTool with AGCodex
//!
//! This module demonstrates how the simplified internal reasoning think tool integrates
//! with AGCodex's architecture and workflow.

use crate::tools::think::{ThinkTool, ThinkResult, ThinkError};

/// Example: Using think tool for code refactoring decisions
pub fn example_refactoring_reasoning() -> Result<(), Box<dyn std::error::Error>> {
    let problem = "This function has O(n²) complexity and high memory usage. How should we refactor it for better performance while maintaining readability?";
    
    let result = ThinkTool::think(problem)?;
    
    println!("🧠 Refactoring Reasoning Session");
    println!("Problem: {}", problem);
    println!("Confidence: {:.2}", result.confidence);
    println!("Steps: {}", result.steps.len());
    
    println!("\n--- Reasoning Steps ---");
    for step in &result.steps {
        println!("{}. {}", step.step_number, step.thought);
        println!("   Reasoning: {}", step.reasoning);
        println!();
    }
    
    println!("--- Conclusion ---");
    println!("{}", result.conclusion);
    
    Ok(())
}

/// Example: Using think tool for architecture decisions
pub fn example_architecture_reasoning() -> Result<(), Box<dyn std::error::Error>> {
    let problem = "We need to design a caching layer that can handle 100k+ concurrent users with sub-100ms latency. Should we use Redis, Memcached, or an in-memory solution?";
    
    let result = ThinkTool::think(problem)?;
    
    println!("🏗️ Architecture Decision Reasoning");
    println!("Problem: {}", problem);
    println!("Confidence: {:.2}", result.confidence);
    
    println!("\n--- Decision Process ---");
    for step in &result.steps {
        println!("Step {}: {}", step.step_number, step.thought);
        println!("Analysis: {}", step.reasoning);
        println!();
    }
    
    println!("--- Final Recommendation ---");
    println!("{}", result.conclusion);
    
    Ok(())
}

/// Example: Sequential reasoning for debugging
pub fn example_debugging_reasoning() -> Result<(), Box<dyn std::error::Error>> {
    let problem = "Users are experiencing database timeouts every 5 minutes, affecting 15% of requests. Error logs show connection pool exhaustion. What's the systematic approach to debug and fix this?";
    
    let result = ThinkTool::think(problem)?;
    
    println!("🐛 Debugging Reasoning Session");
    println!("Problem: {}", problem);
    println!("Confidence: {:.2}", result.confidence);
    
    println!("\n--- Debugging Steps ---");
    for step in &result.steps {
        println!("{}. {}", step.step_number, step.thought);
        println!("   Analysis: {}", step.reasoning);
        println!();
    }
    
    println!("--- Resolution Strategy ---");
    println!("{}", result.conclusion);
    
    Ok(())
}

/// Example: Creative problem-solving for new feature design
pub fn example_creative_reasoning() -> Result<(), Box<dyn std::error::Error>> {
    let problem = "Users want a 'smart suggestions' feature that learns from their coding patterns. How can we implement this creatively within 2 weeks while maintaining our existing UX?";
    
    let result = ThinkTool::think(problem)?;
    
    println!("🎨 Creative Problem-Solving Session");
    println!("Challenge: {}", problem);
    println!("Confidence: {:.2}", result.confidence);
    
    println!("\n--- Creative Process ---");
    for step in &result.steps {
        println!("Step {}: {}", step.step_number, step.thought);
        println!("Insight: {}", step.reasoning);
        println!();
    }
    
    println!("--- Implementation Strategy ---");
    println!("{}", result.conclusion);
    
    Ok(())
}

/// Integration with AGCodex workflow
pub struct AGCodexThinkingIntegration;

impl AGCodexThinkingIntegration {
    pub fn new() -> Self {
        Self
    }

    /// Think before executing a complex task
    pub fn think_before_action(&self, task_description: &str) -> Result<String, Box<dyn std::error::Error>> {
        let result = ThinkTool::think(task_description)?;
        
        // Generate action plan based on reasoning
        let mut action_plan = format!(
            "# Reasoning-Based Action Plan\n\n\
            **Task:** {}\n\
            **Confidence:** {:.2}\n\
            **Steps Analyzed:** {}\n\n\
            ## Reasoning Process\n",
            task_description,
            result.confidence,
            result.steps.len()
        );

        for step in &result.steps {
            action_plan.push_str(&format!(
                "### Step {}: {}\n{}\n\n",
                step.step_number,
                step.thought,
                step.reasoning
            ));
        }

        action_plan.push_str(&format!(
            "## Recommended Approach\n{}\n",
            result.conclusion
        ));

        Ok(action_plan)
    }

    /// Auto-reasoning for code review
    pub fn auto_code_review_reasoning(&self, file_path: &str, changes: &str) -> Result<String, Box<dyn std::error::Error>> {
        let problem = format!(
            "Review these code changes in {} for quality, security, and maintainability:\n\n{}",
            file_path,
            changes
        );

        let result = ThinkTool::think(&problem)?;
        
        let mut review = format!(
            "# Automated Code Review Reasoning\n\n\
            **File:** {}\n\
            **Changes Size:** {} characters\n\
            **Review Confidence:** {:.2}\n\n\
            ## Review Process\n",
            file_path,
            changes.len(),
            result.confidence
        );

        for step in &result.steps {
            review.push_str(&format!(
                "### {}\n{}\n\n",
                step.thought,
                step.reasoning
            ));
        }

        review.push_str(&format!(
            "## Review Summary\n{}\n",
            result.conclusion
        ));

        Ok(review)
    }

    /// Generate reasoning for error analysis
    pub fn error_analysis_reasoning(&self, error_message: &str, context: &str) -> Result<String, Box<dyn std::error::Error>> {
        let problem = format!(
            "Analyze this error and provide debugging guidance:\n\nError: {}\nContext: {}",
            error_message,
            context
        );

        let result = ThinkTool::think(&problem)?;

        let mut analysis = format!(
            "# Error Analysis Reasoning\n\n\
            **Error:** {}\n\
            **Analysis Confidence:** {:.2}\n\n\
            ## Investigation Steps\n",
            error_message,
            result.confidence
        );

        for step in &result.steps {
            analysis.push_str(&format!(
                "**Step {}:** {}\n\n*Analysis:* {}\n\n",
                step.step_number,
                step.thought,
                step.reasoning
            ));
        }

        analysis.push_str(&format!(
            "## Recommended Solution Approach\n{}\n",
            result.conclusion
        ));

        Ok(analysis)
    }

    /// Performance optimization reasoning
    pub fn performance_optimization_reasoning(&self, performance_issue: &str) -> Result<String, Box<dyn std::error::Error>> {
        let problem = format!(
            "Analyze this performance issue and suggest optimization strategies: {}",
            performance_issue
        );

        let result = ThinkTool::think(&problem)?;

        let mut optimization = format!(
            "# Performance Optimization Reasoning\n\n\
            **Issue:** {}\n\
            **Analysis Confidence:** {:.2}\n\n\
            ## Optimization Analysis\n",
            performance_issue,
            result.confidence
        );

        for step in &result.steps {
            optimization.push_str(&format!(
                "### {}\n{}\n\n",
                step.thought,
                step.reasoning
            ));
        }

        optimization.push_str(&format!(
            "## Optimization Strategy\n{}\n",
            result.conclusion
        ));

        Ok(optimization)
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
    fn test_architecture_reasoning_example() {
        let result = example_architecture_reasoning();
        assert!(result.is_ok(), "Architecture reasoning should work");
    }

    #[test]
    fn test_debugging_reasoning_example() {
        let result = example_debugging_reasoning();
        assert!(result.is_ok(), "Debugging reasoning should work");
    }

    #[test]
    fn test_creative_reasoning_example() {
        let result = example_creative_reasoning();
        assert!(result.is_ok(), "Creative reasoning should work");
    }

    #[test]
    fn test_integration_think_before_action() {
        let integration = AGCodexThinkingIntegration::new();
        let result = integration.think_before_action("Refactor the database layer for better performance");
        
        assert!(result.is_ok());
        let action_plan = result.unwrap();
        assert!(action_plan.contains("Reasoning-Based Action Plan"));
        assert!(action_plan.contains("Confidence:"));
        assert!(action_plan.contains("Reasoning Process"));
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
        assert!(review.contains("Review Process"));
        assert!(review.contains("Review Summary"));
    }

    #[test]
    fn test_error_analysis_reasoning() {
        let integration = AGCodexThinkingIntegration::new();
        let result = integration.error_analysis_reasoning(
            "segmentation fault",
            "occurred in memory allocation function"
        );
        
        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert!(analysis.contains("Error Analysis Reasoning"));
        assert!(analysis.contains("Investigation Steps"));
    }

    #[test]
    fn test_performance_optimization_reasoning() {
        let integration = AGCodexThinkingIntegration::new();
        let result = integration.performance_optimization_reasoning(
            "Database queries are taking 5+ seconds"
        );
        
        assert!(result.is_ok());
        let optimization = result.unwrap();
        assert!(optimization.contains("Performance Optimization Reasoning"));
        assert!(optimization.contains("Optimization Analysis"));
    }

    #[test]
    fn test_simple_think_tool_usage() {
        let result = ThinkTool::think("How to improve code readability?");
        
        assert!(result.is_ok());
        let think_result = result.unwrap();
        assert!(think_result.steps.len() >= 3);
        assert!(!think_result.conclusion.is_empty());
        assert!(think_result.confidence > 0.0 && think_result.confidence <= 1.0);
    }
}