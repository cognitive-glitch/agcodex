//! Built-in subagents for AGCodex
//!
//! This module contains the core set of specialized agents that come
//! pre-configured with AGCodex. Each agent focuses on a specific aspect
//! of software development and can be invoked using @agent-name syntax.

pub mod code_reviewer;
pub mod debugger;
pub mod performance;
pub mod refactorer;
pub mod test_writer;

// Re-export all agents for convenience
pub use code_reviewer::CodeReviewerAgent;
pub use code_reviewer::IntelligenceLevel;
pub use debugger::DebugDepth;
pub use debugger::DebuggerAgent;
pub use performance::OptimizationLevel;
pub use performance::PerformanceAgent;
pub use refactorer::RefactorerAgent;
pub use refactorer::RiskLevel;
pub use test_writer::TestStrategy;
pub use test_writer::TestWriterAgent;

use crate::subagents::Subagent;
use crate::subagents::SubagentRegistry;
use std::sync::Arc;

/// Register all built-in agents with the registry
pub fn register_built_in_agents(registry: &SubagentRegistry) {
    // Register code reviewer
    registry.register(
        "code-reviewer",
        Arc::new(CodeReviewerAgent::new()) as Arc<dyn Subagent>,
    );

    // Register refactorer
    registry.register(
        "refactorer",
        Arc::new(RefactorerAgent::new()) as Arc<dyn Subagent>,
    );

    // Register debugger
    registry.register(
        "debugger",
        Arc::new(DebuggerAgent::new()) as Arc<dyn Subagent>,
    );

    // Register test writer
    registry.register(
        "test-writer",
        Arc::new(TestWriterAgent::new()) as Arc<dyn Subagent>,
    );

    // Register performance optimizer
    registry.register(
        "performance",
        Arc::new(PerformanceAgent::new()) as Arc<dyn Subagent>,
    );

    // Register aliases for common variations
    registry.register(
        "reviewer",
        Arc::new(CodeReviewerAgent::new()) as Arc<dyn Subagent>,
    );

    registry.register(
        "refactor",
        Arc::new(RefactorerAgent::new()) as Arc<dyn Subagent>,
    );

    registry.register("debug", Arc::new(DebuggerAgent::new()) as Arc<dyn Subagent>);

    registry.register(
        "test",
        Arc::new(TestWriterAgent::new()) as Arc<dyn Subagent>,
    );

    registry.register(
        "perf",
        Arc::new(PerformanceAgent::new()) as Arc<dyn Subagent>,
    );
}

/// Create a registry with all built-in agents pre-registered
pub fn create_default_registry() -> SubagentRegistry {
    let registry = SubagentRegistry::new().expect("Failed to create registry");
    register_built_in_agents(&registry);
    registry
}

/// Built-in agent descriptions for help text
pub fn get_agent_descriptions() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "@agent-code-reviewer",
            "Reviews code for quality, security, and maintainability issues",
        ),
        (
            "@agent-refactorer",
            "Performs systematic code refactoring with risk assessment",
        ),
        (
            "@agent-debugger",
            "Deep debugging analysis and root cause identification",
        ),
        (
            "@agent-test-writer",
            "Generates comprehensive test suites with high coverage",
        ),
        (
            "@agent-performance",
            "Identifies and optimizes performance bottlenecks",
        ),
    ]
}

/// Get example agent invocations
pub fn get_agent_examples() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "@agent-code-reviewer",
            "Review this file for security issues",
        ),
        (
            "@agent-refactorer → @agent-test-writer",
            "Refactor and then add tests",
        ),
        (
            "@agent-performance + @agent-security",
            "Run performance and security analysis in parallel",
        ),
        (
            "@agent-debugger if errors",
            "Debug only if errors are detected",
        ),
        (
            "@agent-test-writer --strategy=comprehensive",
            "Generate comprehensive test suite",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_built_in_agents_registration() {
        let registry = create_default_registry();

        // Check that all main agents are registered
        assert!(registry.get_agent("code-reviewer").is_some());
        assert!(registry.get_agent("refactorer").is_some());
        assert!(registry.get_agent("debugger").is_some());
        assert!(registry.get_agent("test-writer").is_some());
        assert!(registry.get_agent("performance").is_some());

        // Check aliases
        assert!(registry.get_agent("reviewer").is_some());
        assert!(registry.get_agent("refactor").is_some());
        assert!(registry.get_agent("debug").is_some());
        assert!(registry.get_agent("test").is_some());
        assert!(registry.get_agent("perf").is_some());
    }

    #[test]
    fn test_agent_descriptions() {
        let descriptions = get_agent_descriptions();
        assert_eq!(descriptions.len(), 5);

        // Verify each agent has a description
        for (name, desc) in descriptions {
            assert!(name.starts_with("@agent-"));
            assert!(!desc.is_empty());
        }
    }

    #[test]
    fn test_agent_examples() {
        let examples = get_agent_examples();
        assert!(!examples.is_empty());

        // Verify examples contain agent invocations
        for (invocation, _description) in examples {
            assert!(invocation.contains("@agent-"));
        }
    }
}
