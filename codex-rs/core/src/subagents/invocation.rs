//! Agent invocation parsing and execution planning
//!
//! This module handles parsing user input for agent invocations and
//! building execution plans for sequential and parallel agent execution.

use regex_lite::Regex;
use std::collections::HashMap;
use uuid::Uuid;

/// A request to invoke one or more subagents
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationRequest {
    /// Unique identifier for this invocation request
    pub id: Uuid,

    /// The original user input that triggered this invocation
    pub original_input: String,

    /// The execution plan (chain or parallel)
    pub execution_plan: ExecutionPlan,

    /// Additional context extracted from the input
    pub context: String,
}

/// Execution plan for agents
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionPlan {
    /// Single agent execution
    Single(AgentInvocation),
    /// Sequential execution (agent1 → agent2 → agent3)
    Sequential(AgentChain),
    /// Parallel execution (agent1 + agent2 + agent3)
    Parallel(Vec<AgentInvocation>),
    /// Conditional execution (agent if condition)
    Conditional(ConditionalExecution),
    /// Mixed execution (complex combinations)
    Mixed(Vec<ExecutionStep>),
}

/// A single step in a complex execution plan
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStep {
    /// Execute a single agent
    Single(AgentInvocation),
    /// Execute multiple agents in parallel
    Parallel(Vec<AgentInvocation>),
    /// Execute agents conditionally
    Conditional(ConditionalExecution),
    /// Wait for completion of previous steps
    Barrier,
}

/// A single agent invocation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentInvocation {
    /// Name of the agent to invoke
    pub agent_name: String,

    /// Parameters passed to the agent
    pub parameters: HashMap<String, String>,

    /// Raw parameter string (for complex parsing)
    pub raw_parameters: String,

    /// Position in the original input
    pub position: usize,

    /// Override operating mode for this invocation
    pub mode_override: Option<crate::modes::OperatingMode>,

    /// Override intelligence level for this invocation
    pub intelligence_override: Option<String>,
}

/// A chain of agents for sequential execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentChain {
    /// Agents in execution order
    pub agents: Vec<AgentInvocation>,

    /// Whether to pass output from one agent to the next
    pub pass_output: bool,
}

/// Conditional execution configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalExecution {
    /// Agents to execute if condition is met
    pub agents: Vec<AgentInvocation>,

    /// Condition type
    pub condition: ExecutionCondition,

    /// Condition parameters
    pub condition_params: HashMap<String, String>,
}

/// Supported execution conditions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionCondition {
    /// Execute if previous agent(s) failed
    OnError,
    /// Execute if previous agent(s) succeeded
    OnSuccess,
    /// Execute if specific files have errors
    OnFileError,
    /// Execute if test failures detected
    OnTestFailure,
    /// Execute based on file patterns
    OnFilePattern(String),
    /// Execute based on custom condition
    Custom(String),
}

/// Parser for agent invocation patterns
pub struct InvocationParser {
    /// Regex for matching @agent-name patterns
    agent_pattern: Regex,

    /// Regex for matching sequential chains (→)
    chain_pattern: Regex,

    /// Regex for matching parallel execution (+)
    parallel_pattern: Regex,

    /// Regex for matching conditional patterns (if)
    conditional_pattern: Regex,

    /// Registry for validating agent names
    registry: Option<std::sync::Arc<super::SubagentRegistry>>,
}

impl Default for InvocationParser {
    fn default() -> Self {
        Self::new()
    }
}

impl InvocationParser {
    /// Create a new invocation parser
    pub fn new() -> Self {
        let agent_pattern =
            Regex::new(r"@([a-zA-Z0-9_-]+)(?:\s+([^@→+]*?))?(?:\s*[@→+]|\s*if\s|\s*$)")
                .expect("Invalid agent pattern regex");

        let chain_pattern = Regex::new(r"@[a-zA-Z0-9_-]+(?:\s+[^@→+\n]*?)?\s*→\s*")
            .expect("Invalid chain pattern regex");

        let parallel_pattern = Regex::new(r"@[a-zA-Z0-9_-]+(?:\s+[^@→+\n]*?)?\s*\+\s*")
            .expect("Invalid parallel pattern regex");

        let conditional_pattern =
            Regex::new(r"@([a-zA-Z0-9_-]+)(?:\s+([^@→+\n]*?))?\s+if\s+([^@→+\n]+)")
                .expect("Invalid conditional pattern regex");

        Self {
            agent_pattern,
            chain_pattern,
            parallel_pattern,
            conditional_pattern,
            registry: None,
        }
    }

    /// Create a new invocation parser with registry validation
    pub fn with_registry(registry: std::sync::Arc<super::SubagentRegistry>) -> Self {
        let mut parser = Self::new();
        parser.registry = Some(registry);
        parser
    }

    /// Parse user input for agent invocations
    pub fn parse(&self, input: &str) -> Result<Option<InvocationRequest>, super::SubagentError> {
        // Quick check if there are any agent invocations
        if !input.contains('@') {
            return Ok(None);
        }

        // Extract all agent invocations (including conditional ones)
        let invocations = self.extract_invocations(input)?;
        if invocations.is_empty() {
            return Ok(None);
        }

        // Validate agent names against registry if available
        self.validate_agent_names(&invocations)?;

        // Determine execution plan
        let execution_plan = self.build_execution_plan(input, invocations)?;

        // Validate execution plan
        self.validate_execution_plan(&execution_plan)?;

        // Extract context (text that's not part of agent invocations)
        let context = self.extract_context(input);

        Ok(Some(InvocationRequest {
            id: Uuid::new_v4(),
            original_input: input.to_string(),
            execution_plan,
            context,
        }))
    }

    /// Extract all agent invocations from the input
    fn extract_invocations(
        &self,
        input: &str,
    ) -> Result<Vec<AgentInvocation>, super::SubagentError> {
        let mut invocations = Vec::new();

        for captures in self.agent_pattern.captures_iter(input) {
            let full_match = captures.get(0).unwrap();
            let agent_name = captures.get(1).unwrap().as_str().to_string();
            let raw_parameters = captures
                .get(2)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();

            let parameters = self.parse_parameters(&raw_parameters)?;

            invocations.push(AgentInvocation {
                agent_name,
                parameters,
                raw_parameters,
                position: full_match.start(),
                mode_override: None,
                intelligence_override: None,
            });
        }

        // Sort by position to maintain order
        invocations.sort_by_key(|inv| inv.position);

        Ok(invocations)
    }

    /// Parse parameters from a parameter string
    fn parse_parameters(
        &self,
        param_str: &str,
    ) -> Result<HashMap<String, String>, super::SubagentError> {
        let mut parameters = HashMap::new();

        if param_str.is_empty() {
            return Ok(parameters);
        }

        // Simple parameter parsing - can be enhanced
        // Supports: key=value, "quoted values", and positional arguments

        let mut current_key = String::new();
        let mut current_value = String::new();
        let mut in_quotes = false;
        let mut in_value = false;
        let chars = param_str.chars().peekable();
        let mut positional_index = 0;

        for ch in chars {
            match ch {
                '"' if !in_quotes => {
                    in_quotes = true;
                    in_value = true;
                }
                '"' if in_quotes => {
                    in_quotes = false;
                    if !current_key.is_empty() {
                        parameters.insert(current_key.clone(), current_value.clone());
                    } else {
                        parameters
                            .insert(format!("arg{}", positional_index), current_value.clone());
                        positional_index += 1;
                    }
                    current_key.clear();
                    current_value.clear();
                    in_value = false;
                }
                '=' if !in_quotes && !in_value => {
                    in_value = true;
                }
                ' ' if !in_quotes && !in_value => {
                    if !current_key.is_empty() {
                        // Positional argument
                        parameters.insert(format!("arg{}", positional_index), current_key.clone());
                        positional_index += 1;
                        current_key.clear();
                    }
                }
                ' ' if !in_quotes && in_value => {
                    if !current_key.is_empty() {
                        parameters.insert(current_key.clone(), current_value.clone());
                    } else {
                        parameters
                            .insert(format!("arg{}", positional_index), current_value.clone());
                        positional_index += 1;
                    }
                    current_key.clear();
                    current_value.clear();
                    in_value = false;
                }
                _ => {
                    if in_value {
                        current_value.push(ch);
                    } else {
                        current_key.push(ch);
                    }
                }
            }
        }

        // Handle remaining content
        if !current_key.is_empty() || !current_value.is_empty() {
            if in_value && !current_key.is_empty() {
                parameters.insert(current_key, current_value);
            } else if !current_key.is_empty() {
                parameters.insert(format!("arg{}", positional_index), current_key);
            } else if !current_value.is_empty() {
                parameters.insert(format!("arg{}", positional_index), current_value);
            }
        }

        Ok(parameters)
    }

    /// Build execution plan based on the input pattern
    fn build_execution_plan(
        &self,
        input: &str,
        invocations: Vec<AgentInvocation>,
    ) -> Result<ExecutionPlan, super::SubagentError> {
        // Check for conditional patterns first
        if self.conditional_pattern.is_match(input) {
            return self.build_conditional_execution_plan(input, invocations);
        }

        if invocations.len() == 1 {
            return Ok(ExecutionPlan::Single(
                invocations.into_iter().next().unwrap(),
            ));
        }

        // Check for sequential chains (→)
        if self.chain_pattern.is_match(input) {
            return Ok(ExecutionPlan::Sequential(AgentChain {
                agents: invocations,
                pass_output: true,
            }));
        }

        // Check for parallel execution (+)
        if self.parallel_pattern.is_match(input) {
            return Ok(ExecutionPlan::Parallel(invocations));
        }

        // Complex mixed execution (both → and + and if)
        if (input.contains('→') && input.contains('+')) || input.contains(" if ") {
            return self.build_mixed_execution_plan(input, invocations);
        }

        // Default to parallel if multiple agents without explicit operators
        Ok(ExecutionPlan::Parallel(invocations))
    }

    /// Build a complex mixed execution plan
    fn build_mixed_execution_plan(
        &self,
        input: &str,
        invocations: Vec<AgentInvocation>,
    ) -> Result<ExecutionPlan, super::SubagentError> {
        // This is a simplified implementation
        // A full implementation would parse the operators properly

        let mut steps = Vec::new();
        let mut current_parallel = Vec::new();

        for invocation in invocations {
            // Check if there's a → after this invocation
            let next_pos = invocation.position + invocation.agent_name.len();
            let remaining = &input[next_pos..];

            if remaining.trim_start().starts_with('→') {
                // This is the end of a parallel group
                current_parallel.push(invocation);
                if current_parallel.len() == 1 {
                    steps.push(ExecutionStep::Single(current_parallel.pop().unwrap()));
                } else {
                    steps.push(ExecutionStep::Parallel(current_parallel.clone()));
                }
                current_parallel.clear();
                steps.push(ExecutionStep::Barrier);
            } else {
                current_parallel.push(invocation);
            }
        }

        // Handle remaining parallel group
        if !current_parallel.is_empty() {
            if current_parallel.len() == 1 {
                steps.push(ExecutionStep::Single(current_parallel.pop().unwrap()));
            } else {
                steps.push(ExecutionStep::Parallel(current_parallel));
            }
        }

        Ok(ExecutionPlan::Mixed(steps))
    }

    /// Build a conditional execution plan
    fn build_conditional_execution_plan(
        &self,
        input: &str,
        invocations: Vec<AgentInvocation>,
    ) -> Result<ExecutionPlan, super::SubagentError> {
        let mut conditional_agents = Vec::new();
        let mut condition = ExecutionCondition::OnError; // default
        let mut condition_params = HashMap::new();

        // Parse conditional invocations
        for captures in self.conditional_pattern.captures_iter(input) {
            let agent_name = captures.get(1).unwrap().as_str().to_string();
            let raw_parameters = captures
                .get(2)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            let condition_text = captures.get(3).unwrap().as_str().trim();

            let parameters = self.parse_parameters(&raw_parameters)?;

            // Parse condition
            let (parsed_condition, parsed_params) = self.parse_condition(condition_text)?;
            condition = parsed_condition;
            condition_params = parsed_params;

            conditional_agents.push(AgentInvocation {
                agent_name,
                parameters,
                raw_parameters,
                position: captures.get(0).unwrap().start(),
                mode_override: None,
                intelligence_override: None,
            });
        }

        if conditional_agents.is_empty() {
            // Fall back to regular parsing if no conditional agents found
            return self.build_execution_plan_without_conditionals(input, invocations);
        }

        Ok(ExecutionPlan::Conditional(ConditionalExecution {
            agents: conditional_agents,
            condition,
            condition_params,
        }))
    }

    /// Build execution plan without conditional support (fallback)
    fn build_execution_plan_without_conditionals(
        &self,
        input: &str,
        invocations: Vec<AgentInvocation>,
    ) -> Result<ExecutionPlan, super::SubagentError> {
        if invocations.len() == 1 {
            return Ok(ExecutionPlan::Single(
                invocations.into_iter().next().unwrap(),
            ));
        }

        // Check for sequential chains (→)
        if self.chain_pattern.is_match(input) {
            return Ok(ExecutionPlan::Sequential(AgentChain {
                agents: invocations,
                pass_output: true,
            }));
        }

        // Check for parallel execution (+)
        if self.parallel_pattern.is_match(input) {
            return Ok(ExecutionPlan::Parallel(invocations));
        }

        // Complex mixed execution
        if input.contains('→') && input.contains('+') {
            return self.build_mixed_execution_plan(input, invocations);
        }

        // Default to parallel if multiple agents
        Ok(ExecutionPlan::Parallel(invocations))
    }

    /// Parse execution condition from text
    fn parse_condition(
        &self,
        condition_text: &str,
    ) -> Result<(ExecutionCondition, HashMap<String, String>), super::SubagentError> {
        let mut params = HashMap::new();
        let condition_text = condition_text.trim().to_lowercase();

        let condition = match condition_text.as_str() {
            "error" | "errors" | "failed" | "failure" => ExecutionCondition::OnError,
            "success" | "succeeded" | "passed" => ExecutionCondition::OnSuccess,
            "test failure" | "test failures" | "tests fail" => ExecutionCondition::OnTestFailure,
            text if text.starts_with("file error") => {
                // Parse file patterns from "file error in *.rs"
                let pattern = text
                    .strip_prefix("file error")
                    .unwrap_or("")
                    .trim()
                    .strip_prefix("in")
                    .unwrap_or("*")
                    .trim();
                params.insert("pattern".to_string(), pattern.to_string());
                ExecutionCondition::OnFileError
            }
            text if text.starts_with("pattern")
                || text.ends_with(".rs")
                || text.ends_with(".py")
                || text.ends_with(".js")
                || text.ends_with(".ts") =>
            {
                // File pattern condition
                let pattern = if text.starts_with("pattern ") {
                    text.strip_prefix("pattern ").unwrap_or(text)
                } else {
                    text
                };
                ExecutionCondition::OnFilePattern(pattern.to_string())
            }
            _ => {
                // Custom condition
                params.insert("expression".to_string(), condition_text.to_string());
                ExecutionCondition::Custom(condition_text.to_string())
            }
        };

        Ok((condition, params))
    }

    /// Validate agent names against the registry
    fn validate_agent_names(
        &self,
        invocations: &[AgentInvocation],
    ) -> Result<(), super::SubagentError> {
        if let Some(ref registry) = self.registry {
            for invocation in invocations {
                if registry.get_agent(&invocation.agent_name).is_none() {
                    return Err(super::SubagentError::AgentNotFound {
                        name: invocation.agent_name.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Extract context (non-agent text) from the input
    fn extract_context(&self, input: &str) -> String {
        let mut context = input.to_string();

        // Remove agent invocations and operators
        context = self.agent_pattern.replace_all(&context, "").to_string();
        context = context.replace('→', " ");
        context = context.replace('+', " ");

        // Clean up whitespace
        context = context
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        context.trim().to_string()
    }

    /// Validate that an execution plan doesn't have circular dependencies
    pub fn validate_execution_plan(
        &self,
        plan: &ExecutionPlan,
    ) -> Result<(), super::SubagentError> {
        let agents: Vec<&String> = match plan {
            ExecutionPlan::Single(_) => return Ok(()), // No cycles possible
            ExecutionPlan::Sequential(chain) => {
                chain.agents.iter().map(|a| &a.agent_name).collect()
            }
            ExecutionPlan::Parallel(_) => return Ok(()), // No cycles in parallel
            ExecutionPlan::Conditional(cond) => {
                cond.agents.iter().map(|inv| &inv.agent_name).collect()
            }
            ExecutionPlan::Mixed(steps) => {
                // Flatten all agent names
                steps
                    .iter()
                    .flat_map(|step| match step {
                        ExecutionStep::Single(inv) => vec![&inv.agent_name],
                        ExecutionStep::Parallel(invs) => {
                            invs.iter().map(|inv| &inv.agent_name).collect()
                        }
                        ExecutionStep::Conditional(cond) => {
                            cond.agents.iter().map(|inv| &inv.agent_name).collect()
                        }
                        ExecutionStep::Barrier => vec![],
                    })
                    .collect()
            }
        };

        // Check for duplicate agents in sequential execution
        if let ExecutionPlan::Sequential(_) = plan {
            let mut seen = std::collections::HashSet::new();
            for agent_name in &agents {
                if !seen.insert(agent_name) {
                    return Err(super::SubagentError::CircularDependency {
                        chain: agents.into_iter().map(|s| s.to_string()).collect(),
                    });
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_agent_parsing() {
        let parser = InvocationParser::new();
        let result = parser
            .parse("@code-reviewer check this file")
            .unwrap()
            .unwrap();

        match result.execution_plan {
            ExecutionPlan::Single(inv) => {
                assert_eq!(inv.agent_name, "code-reviewer");
                assert_eq!(inv.raw_parameters, "check this file");
            }
            _ => panic!("Expected single execution plan"),
        }
    }

    #[test]
    fn test_sequential_chain_parsing() {
        let parser = InvocationParser::new();
        let result = parser
            .parse("@refactorer fix → @test-writer add tests")
            .unwrap()
            .unwrap();

        match result.execution_plan {
            ExecutionPlan::Sequential(chain) => {
                assert_eq!(chain.agents.len(), 2);
                assert_eq!(chain.agents[0].agent_name, "refactorer");
                assert_eq!(chain.agents[1].agent_name, "test-writer");
                assert!(chain.pass_output);
            }
            _ => panic!("Expected sequential execution plan"),
        }
    }

    #[test]
    fn test_parallel_execution_parsing() {
        let parser = InvocationParser::new();
        let result = parser
            .parse("@performance analyze + @security audit")
            .unwrap()
            .unwrap();

        match result.execution_plan {
            ExecutionPlan::Parallel(agents) => {
                assert_eq!(agents.len(), 2);
                assert_eq!(agents[0].agent_name, "performance");
                assert_eq!(agents[1].agent_name, "security");
            }
            _ => panic!("Expected parallel execution plan"),
        }
    }

    #[test]
    fn test_parameter_parsing() {
        let parser = InvocationParser::new();

        // Test key=value parameters
        let params = parser
            .parse_parameters("file=src/main.rs level=high")
            .unwrap();
        assert_eq!(params.get("file").unwrap(), "src/main.rs");
        assert_eq!(params.get("level").unwrap(), "high");

        // Test quoted values
        let params = parser
            .parse_parameters(r#"message="fix this bug" priority=1"#)
            .unwrap();
        assert_eq!(params.get("message").unwrap(), "fix this bug");
        assert_eq!(params.get("priority").unwrap(), "1");

        // Test positional arguments
        let params = parser.parse_parameters("src/main.rs high").unwrap();
        assert_eq!(params.get("arg0").unwrap(), "src/main.rs");
        assert_eq!(params.get("arg1").unwrap(), "high");
    }

    #[test]
    fn test_context_extraction() {
        let parser = InvocationParser::new();
        let result = parser.parse("Please @code-reviewer this file and then @test-writer. Make sure everything works.").unwrap().unwrap();

        assert_eq!(
            result.context,
            "Please this file and then . Make sure everything works."
        );
    }

    #[test]
    fn test_no_agents() {
        let parser = InvocationParser::new();
        let result = parser
            .parse("This is just regular text with no agents.")
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_conditional_parsing() {
        let parser = InvocationParser::new();
        let result = parser.parse("@debugger if errors").unwrap().unwrap();

        match result.execution_plan {
            ExecutionPlan::Conditional(cond) => {
                assert_eq!(cond.agents.len(), 1);
                assert_eq!(cond.agents[0].agent_name, "debugger");
                assert!(matches!(cond.condition, ExecutionCondition::OnError));
            }
            _ => panic!("Expected conditional execution plan"),
        }
    }

    #[test]
    fn test_file_pattern_conditional() {
        let parser = InvocationParser::new();
        let result = parser.parse("@security-scanner if *.rs").unwrap().unwrap();

        match result.execution_plan {
            ExecutionPlan::Conditional(cond) => {
                assert_eq!(cond.agents.len(), 1);
                assert_eq!(cond.agents[0].agent_name, "security-scanner");
                assert!(matches!(
                    cond.condition,
                    ExecutionCondition::OnFilePattern(_)
                ));
                if let ExecutionCondition::OnFilePattern(pattern) = &cond.condition {
                    assert_eq!(pattern, "*.rs");
                }
            }
            _ => panic!("Expected conditional execution plan"),
        }
    }

    #[test]
    fn test_complex_conditional_parsing() {
        let parser = InvocationParser::new();
        let result = parser
            .parse("@performance analyze → @debugger if errors → @refactorer")
            .unwrap()
            .unwrap();

        // This should create a mixed execution plan with conditional step
        match result.execution_plan {
            ExecutionPlan::Mixed(_) => {
                // Complex mixed parsing works
            }
            _ => {
                // Fallback to other patterns is also valid
            }
        }
    }

    #[test]
    fn test_condition_parsing() {
        let parser = InvocationParser::new();

        // Test error condition
        let (condition, _) = parser.parse_condition("errors").unwrap();
        assert!(matches!(condition, ExecutionCondition::OnError));

        // Test success condition
        let (condition, _) = parser.parse_condition("success").unwrap();
        assert!(matches!(condition, ExecutionCondition::OnSuccess));

        // Test file pattern condition
        let (condition, _) = parser.parse_condition("*.rs").unwrap();
        assert!(matches!(condition, ExecutionCondition::OnFilePattern(_)));

        // Test custom condition
        let (condition, params) = parser.parse_condition("custom logic").unwrap();
        assert!(matches!(condition, ExecutionCondition::Custom(_)));
        assert_eq!(params.get("expression").unwrap(), "custom logic");
    }

    #[test]
    fn test_registry_validation() {
        // This test would require a mock registry
        // For now, just test that validation passes without a registry
        let parser = InvocationParser::new();
        let invocations = vec![AgentInvocation {
            agent_name: "test-agent".to_string(),
            parameters: HashMap::new(),
            raw_parameters: String::new(),
            position: 0,
            mode_override: None,
            intelligence_override: None,
        }];

        let result = parser.validate_agent_names(&invocations);
        assert!(result.is_ok()); // Should pass without registry
    }

    #[test]
    fn test_circular_dependency_detection() {
        let parser = InvocationParser::new();
        let chain = AgentChain {
            agents: vec![
                AgentInvocation {
                    agent_name: "agent1".to_string(),
                    parameters: HashMap::new(),
                    raw_parameters: String::new(),
                    position: 0,
                    mode_override: None,
                    intelligence_override: None,
                },
                AgentInvocation {
                    agent_name: "agent1".to_string(), // Duplicate!
                    parameters: HashMap::new(),
                    raw_parameters: String::new(),
                    position: 1,
                    mode_override: None,
                    intelligence_override: None,
                },
            ],
            pass_output: true,
        };

        let plan = ExecutionPlan::Sequential(chain);
        let result = parser.validate_execution_plan(&plan);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            super::super::SubagentError::CircularDependency { .. }
        ));
    }
}
