//! Parser module for agent invocations
//!
//! This module provides a specialized parser for @agent-name invocations,
//! working alongside the existing invocation.rs infrastructure.

use super::SubagentError;
use super::SubagentRegistry;
use super::invocation::ExecutionPlan;
use super::invocation::InvocationParser;
use super::invocation::InvocationRequest;
use crate::modes::OperatingMode;
use std::collections::HashMap;
use std::sync::Arc;

/// Enhanced parser for agent invocations with additional features
pub struct AgentParser {
    /// Base invocation parser
    base_parser: InvocationParser,
    /// Registry for agent validation
    _registry: Option<Arc<SubagentRegistry>>,
    /// Default operating mode
    default_mode: OperatingMode,
}

impl AgentParser {
    /// Create a new agent parser
    pub fn new() -> Self {
        Self {
            base_parser: InvocationParser::new(),
            _registry: None,
            default_mode: OperatingMode::Build,
        }
    }

    /// Create parser with registry for validation
    pub fn with_registry(registry: Arc<SubagentRegistry>) -> Self {
        Self {
            base_parser: InvocationParser::with_registry(registry.clone()),
            _registry: Some(registry),
            default_mode: OperatingMode::Build,
        }
    }

    /// Set the default operating mode
    pub const fn with_default_mode(mut self, mode: OperatingMode) -> Self {
        self.default_mode = mode;
        self
    }

    /// Parse @agent-name invocations from user input
    ///
    /// Supports:
    /// - Single agents: @agent-refactor
    /// - Sequential chains: @agent-refactor → @agent-test
    /// - Parallel execution: @agent-security + @agent-performance
    /// - Mixed execution: @agent-analyze → (@agent-fix + @agent-document)
    pub fn parse(&self, input: &str) -> Result<Option<ParsedInvocation>, SubagentError> {
        // Use base parser for core parsing
        let base_result = self.base_parser.parse(input)?;

        match base_result {
            Some(request) => {
                // Enhance with additional parsing features
                let enhanced = self.enhance_invocation(request)?;
                Ok(Some(enhanced))
            }
            None => Ok(None),
        }
    }

    /// Extract agent parameters from input
    pub fn extract_parameters(&self, input: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();

        // Extract mode override
        if input.contains("--mode=plan") {
            params.insert("mode".to_string(), "plan".to_string());
        } else if input.contains("--mode=review") {
            params.insert("mode".to_string(), "review".to_string());
        } else if input.contains("--mode=build") {
            params.insert("mode".to_string(), "build".to_string());
        }

        // Extract intelligence level
        if input.contains("--intelligence=light") {
            params.insert("intelligence".to_string(), "light".to_string());
        } else if input.contains("--intelligence=medium") {
            params.insert("intelligence".to_string(), "medium".to_string());
        } else if input.contains("--intelligence=hard") {
            params.insert("intelligence".to_string(), "hard".to_string());
        }

        // Extract timeout
        if let Some(timeout_match) = input.find("--timeout=") {
            let start = timeout_match + 10;
            if let Some(end) = input[start..]
                .find(' ')
                .map(|i| start + i)
                .or(Some(input.len()))
            {
                let timeout_str = &input[start..end];
                params.insert("timeout".to_string(), timeout_str.to_string());
            }
        }

        params
    }

    /// Extract context for agents (non-agent text)
    pub fn extract_context(&self, input: &str) -> String {
        let mut context = input.to_string();

        // Remove agent invocations
        let agent_pattern = regex_lite::Regex::new(r"@[a-zA-Z0-9_-]+").unwrap();
        context = agent_pattern.replace_all(&context, "").to_string();

        // Remove operators
        context = context.replace('→', " ");
        context = context.replace('+', " ");

        // Remove parameters
        context = context.replace(|c: char| c == '-' && context.contains("--"), " ");

        // Clean up whitespace
        context
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string()
    }

    /// Enhance invocation with additional metadata
    fn enhance_invocation(
        &self,
        request: InvocationRequest,
    ) -> Result<ParsedInvocation, SubagentError> {
        let global_params = self.extract_parameters(&request.original_input);

        // Apply mode overrides from parameters
        let mode = global_params
            .get("mode")
            .and_then(|m| match m.as_str() {
                "plan" => Some(OperatingMode::Plan),
                "build" => Some(OperatingMode::Build),
                "review" => Some(OperatingMode::Review),
                _ => None,
            })
            .unwrap_or(self.default_mode);

        // Apply intelligence overrides
        let intelligence = global_params.get("intelligence").cloned();

        // Apply timeout overrides
        let timeout = global_params
            .get("timeout")
            .and_then(|t| t.parse::<u64>().ok())
            .map(std::time::Duration::from_secs);

        Ok(ParsedInvocation {
            id: request.id,
            original_input: request.original_input,
            execution_plan: request.execution_plan,
            context: request.context,
            mode_override: Some(mode),
            intelligence_override: intelligence,
            timeout_override: timeout,
            global_parameters: global_params,
        })
    }

    /// Validate agent chain for circular dependencies
    pub fn validate_chain(&self, agents: &[String]) -> Result<(), SubagentError> {
        let mut seen = std::collections::HashSet::new();

        for agent in agents {
            if !seen.insert(agent.clone()) {
                return Err(SubagentError::CircularDependency {
                    chain: agents.to_vec(),
                });
            }
        }

        Ok(())
    }

    /// Support for chaining operators
    pub fn parse_chain_operators(&self, input: &str) -> ChainOperator {
        if input.contains('→') && input.contains('+') {
            ChainOperator::Mixed
        } else if input.contains('→') {
            ChainOperator::Sequential
        } else if input.contains('+') {
            ChainOperator::Parallel
        } else {
            ChainOperator::Single
        }
    }
}

/// Enhanced parsed invocation with additional metadata
#[derive(Debug, Clone)]
pub struct ParsedInvocation {
    /// Unique identifier
    pub id: uuid::Uuid,
    /// Original input string
    pub original_input: String,
    /// Execution plan
    pub execution_plan: ExecutionPlan,
    /// Extracted context
    pub context: String,
    /// Mode override for all agents
    pub mode_override: Option<OperatingMode>,
    /// Intelligence level override
    pub intelligence_override: Option<String>,
    /// Timeout override
    pub timeout_override: Option<std::time::Duration>,
    /// Global parameters
    pub global_parameters: HashMap<String, String>,
}

/// Chain operator types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainOperator {
    Single,
    Sequential,
    Parallel,
    Mixed,
}

impl Default for AgentParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_agent() {
        let parser = AgentParser::new();
        let result = parser.parse("@agent-refactor clean up this code").unwrap();

        assert!(result.is_some());
        let parsed = result.unwrap();
        assert!(matches!(parsed.execution_plan, ExecutionPlan::Single(_)));
    }

    #[test]
    fn test_parse_sequential_chain() {
        let parser = AgentParser::new();
        let result = parser
            .parse("@agent-refactor → @agent-test → @agent-document")
            .unwrap();

        assert!(result.is_some());
        let parsed = result.unwrap();
        assert!(matches!(
            parsed.execution_plan,
            ExecutionPlan::Sequential(_)
        ));
    }

    #[test]
    fn test_parse_parallel_execution() {
        let parser = AgentParser::new();
        let result = parser
            .parse("@agent-security + @agent-performance + @agent-docs")
            .unwrap();

        assert!(result.is_some());
        let parsed = result.unwrap();
        assert!(matches!(parsed.execution_plan, ExecutionPlan::Parallel(_)));
    }

    #[test]
    fn test_extract_parameters() {
        let parser = AgentParser::new();
        let params = parser
            .extract_parameters("@agent-refactor --mode=review --intelligence=hard --timeout=300");

        assert_eq!(params.get("mode").unwrap(), "review");
        assert_eq!(params.get("intelligence").unwrap(), "hard");
        assert_eq!(params.get("timeout").unwrap(), "300");
    }

    #[test]
    fn test_extract_context() {
        let parser = AgentParser::new();
        let context = parser
            .extract_context("Please @agent-refactor this code and then @agent-test it thoroughly");

        assert_eq!(context, "Please this code and then it thoroughly");
    }

    #[test]
    fn test_chain_operator_detection() {
        let parser = AgentParser::new();

        assert_eq!(
            parser.parse_chain_operators("@agent-refactor"),
            ChainOperator::Single
        );
        assert_eq!(
            parser.parse_chain_operators("@agent-a → @agent-b"),
            ChainOperator::Sequential
        );
        assert_eq!(
            parser.parse_chain_operators("@agent-a + @agent-b"),
            ChainOperator::Parallel
        );
        assert_eq!(
            parser.parse_chain_operators("@agent-a → @agent-b + @agent-c"),
            ChainOperator::Mixed
        );
    }

    #[test]
    fn test_circular_dependency_detection() {
        let parser = AgentParser::new();

        // No circular dependency
        assert!(
            parser
                .validate_chain(&["agent-a".to_string(), "agent-b".to_string()])
                .is_ok()
        );

        // Circular dependency
        assert!(
            parser
                .validate_chain(&[
                    "agent-a".to_string(),
                    "agent-b".to_string(),
                    "agent-a".to_string(),
                ])
                .is_err()
        );
    }
}
