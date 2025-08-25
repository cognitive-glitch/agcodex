//! Integration tests for the agent system
//!
//! This module tests end-to-end agent workflows and system integration.
//! Tests are designed to work with the current implementation state.

use agcodex_core::subagents::*;
use agcodex_core::modes::OperatingMode;
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::tempdir;
use std::io::Write;

// Integration test fixtures
mod fixtures {
    use super::*;
    
    pub fn create_test_project() -> tempfile::TempDir {
        let temp_dir = tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        
        // Create main.rs
        let main_content = r#"
use std::collections::HashMap;

pub struct Application {
    name: String,
    version: String,
    config: HashMap<String, String>,
}

impl Application {
    pub fn new(name: String, version: String) -> Self {
        Self {
            name,
            version,
            config: HashMap::new(),
        }
    }
    
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting {} v{}", self.name, self.version);
        Ok(())
    }
}

fn main() {
    let app = Application::new("TestApp".to_string(), "1.0.0".to_string());
    if let Err(e) = app.run() {
        eprintln!("Error: {}", e);
    }
}
"#;
        std::fs::write(src_dir.join("main.rs"), main_content).unwrap();
        
        // Create lib.rs
        let lib_content = r#"
pub struct Calculator {
    precision: u8,
}

impl Calculator {
    pub fn new(precision: u8) -> Self {
        Self { precision }
    }
    
    pub fn calculate(&self, values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        
        values.iter().sum::<f64>() / values.len() as f64
    }
}
"#;
        std::fs::write(src_dir.join("lib.rs"), lib_content).unwrap();
        
        temp_dir
    }
    
    pub fn create_test_registry_with_agents() -> (SubagentRegistry, tempfile::TempDir) {
        let temp_dir = tempdir().unwrap();
        let agents_dir = temp_dir.path().join("agents");
        let global_dir = agents_dir.join("global");
        let templates_dir = agents_dir.join("templates");
        
        std::fs::create_dir_all(&global_dir).unwrap();
        std::fs::create_dir_all(&templates_dir).unwrap();
        
        // Create registry
        let registry = SubagentRegistry {
            global_agents_dir: global_dir.clone(),
            project_agents_dir: None,
            templates_dir: templates_dir.clone(),
            agents: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            templates: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            watch_enabled: false,
            last_scan: std::sync::Arc::new(std::sync::Mutex::new(std::time::SystemTime::UNIX_EPOCH)),
        };
        
        // Create code reviewer agent
        let code_reviewer = SubagentConfig {
            name: "code-reviewer".to_string(),
            description: "Reviews code for quality, security, and maintainability".to_string(),
            mode_override: Some(OperatingMode::Review),
            intelligence: IntelligenceLevel::Hard,
            tools: {
                let mut tools = HashMap::new();
                tools.insert("ast_search".to_string(), ToolPermission::Read);
                tools.insert("file_read".to_string(), ToolPermission::Read);
                tools
            },
            prompt: "You are a senior code reviewer.".to_string(),
            parameters: vec![
                config::ParameterDefinition {
                    name: "files".to_string(),
                    description: "Files to review".to_string(),
                    required: false,
                    default: Some("**/*.rs".to_string()),
                    valid_values: None,
                },
            ],
            template: None,
            timeout_seconds: 120,
            chainable: true,
            parallelizable: true,
            metadata: HashMap::new(),
            file_patterns: vec!["*.rs".to_string()],
            tags: vec!["review".to_string(), "quality".to_string()],
        };
        
        // Create performance analyzer agent
        let performance_analyzer = SubagentConfig {
            name: "performance-analyzer".to_string(),
            description: "Analyzes code for performance bottlenecks".to_string(),
            mode_override: Some(OperatingMode::Review),
            intelligence: IntelligenceLevel::Hard,
            tools: {
                let mut tools = HashMap::new();
                tools.insert("ast_search".to_string(), ToolPermission::Read);
                tools
            },
            prompt: "You are a performance optimization expert.".to_string(),
            parameters: vec![],
            template: None,
            timeout_seconds: 90,
            chainable: true,
            parallelizable: true,
            metadata: HashMap::new(),
            file_patterns: vec!["*.rs".to_string()],
            tags: vec!["performance".to_string()],
        };
        
        // Save agent configurations
        code_reviewer.to_file(&global_dir.join("code-reviewer.toml")).unwrap();
        performance_analyzer.to_file(&global_dir.join("performance-analyzer.toml")).unwrap();
        
        // Load all agents
        registry.load_all().unwrap();
        
        (registry, temp_dir)
    }
}

// Basic integration tests
#[cfg(test)]
mod basic_integration {
    use super::*;
    use super::fixtures::*;

    #[test]
    fn test_agent_registry_integration() {
        let (registry, _temp_dir) = create_test_registry_with_agents();
        
        // Verify agents were loaded
        let agents = registry.get_all_agents();
        assert_eq!(agents.len(), 2);
        
        let agent_names: Vec<_> = agents.keys().collect();
        assert!(agent_names.contains(&&"code-reviewer".to_string()));
        assert!(agent_names.contains(&&"performance-analyzer".to_string()));
        
        // Test agent retrieval
        let code_reviewer = registry.get_agent("code-reviewer").unwrap();
        assert_eq!(code_reviewer.config.mode_override, Some(OperatingMode::Review));
        assert!(code_reviewer.config.tools.contains_key("ast_search"));
        
        // Test file pattern matching
        let rust_file = PathBuf::from("src/main.rs");
        let matching_agents = registry.get_agents_for_file(&rust_file);
        assert_eq!(matching_agents.len(), 2); // Both agents should match *.rs files
        
        // Test tag filtering
        let review_agents = registry.get_agents_with_tags(&vec!["review".to_string()]);
        assert_eq!(review_agents.len(), 1);
        assert_eq!(review_agents[0].config.name, "code-reviewer");
        
        let performance_agents = registry.get_agents_with_tags(&vec!["performance".to_string()]);
        assert_eq!(performance_agents.len(), 1);
        assert_eq!(performance_agents[0].config.name, "performance-analyzer");
    }

    #[test]
    fn test_invocation_parsing_integration() {
        let parser = InvocationParser::new();
        
        // Test complex real-world invocations
        let test_cases = vec![
            ("@code-reviewer files=src/**/*.rs", "single"),
            ("@performance-analyzer", "single"),
            ("@code-reviewer → @performance-analyzer", "sequential"),
            ("@code-reviewer + @performance-analyzer", "parallel"),
        ];
        
        for (input, expected_type) in test_cases {
            let result = parser.parse(input).unwrap();
            assert!(result.is_some(), "Failed to parse: {}", input);
            
            let invocation = result.unwrap();
            
            match expected_type {
                "single" => {
                    assert!(matches!(invocation.execution_plan, ExecutionPlan::Single(_)));
                }
                "sequential" => {
                    assert!(matches!(invocation.execution_plan, ExecutionPlan::Sequential(_)));
                }
                "parallel" => {
                    assert!(matches!(invocation.execution_plan, ExecutionPlan::Parallel(_)));
                }
                _ => panic!("Unknown expected type: {}", expected_type),
            }
        }
    }
}

// Workflow tests
#[cfg(test)]
mod workflow_tests {
    use super::*;
    use super::fixtures::*;

    #[test]
    fn test_agent_execution_planning() {
        let (registry, _temp_dir) = create_test_registry_with_agents();
        let _project_dir = create_test_project();
        
        // Parse invocation
        let parser = InvocationParser::new();
        let invocation = parser.parse("@code-reviewer files=src/*.rs").unwrap().unwrap();
        
        // Execute single agent planning
        match invocation.execution_plan {
            ExecutionPlan::Single(agent_inv) => {
                assert_eq!(agent_inv.agent_name, "code-reviewer");
                
                // Get agent configuration
                let agent_info = registry.get_agent(&agent_inv.agent_name).unwrap();
                
                // Create execution context
                let context = SubagentContext {
                    execution_id: uuid::Uuid::new_v4(),
                    mode: agent_info.config.mode_override.unwrap_or(OperatingMode::Build),
                    available_tools: agent_info.config.tools.keys().cloned().collect(),
                    conversation_context: String::new(),
                    working_directory: std::env::current_dir().unwrap(),
                    parameters: agent_inv.parameters,
                    metadata: HashMap::new(),
                };
                
                // Verify context creation
                assert_eq!(context.mode, OperatingMode::Review);
                assert!(context.available_tools.contains(&"ast_search".to_string()));
                
                // Create execution tracker
                let mut execution = SubagentExecution::new(agent_inv.agent_name.clone());
                assert_eq!(execution.status, SubagentStatus::Pending);
                
                execution.start();
                assert_eq!(execution.status, SubagentStatus::Running);
                
                // Simulate completion
                execution.complete(
                    "Code review planning complete".to_string(),
                    vec![],
                );
                
                assert_eq!(execution.status, SubagentStatus::Completed);
                assert!(execution.output.is_some());
                assert!(execution.duration().is_some());
            }
            _ => panic!("Expected single execution plan"),
        }
    }

    #[test]
    fn test_sequential_agent_planning() {
        let (registry, _temp_dir) = create_test_registry_with_agents();
        
        // Parse sequential invocation
        let parser = InvocationParser::new();
        let invocation = parser.parse("@code-reviewer → @performance-analyzer").unwrap().unwrap();
        
        match invocation.execution_plan {
            ExecutionPlan::Sequential(chain) => {
                assert_eq!(chain.agents.len(), 2);
                assert!(chain.pass_output);
                
                let mut execution_plan = Vec::new();
                
                // Plan execution for each agent
                for agent_inv in &chain.agents {
                    let agent_info = registry.get_agent(&agent_inv.agent_name).unwrap();
                    
                    let context = SubagentContext {
                        execution_id: uuid::Uuid::new_v4(),
                        mode: agent_info.config.mode_override.unwrap_or(OperatingMode::Build),
                        available_tools: agent_info.config.tools.keys().cloned().collect(),
                        conversation_context: String::new(),
                        working_directory: std::env::current_dir().unwrap(),
                        parameters: agent_inv.parameters.clone(),
                        metadata: HashMap::new(),
                    };
                    
                    execution_plan.push((agent_inv.agent_name.clone(), context));
                }
                
                // Verify execution plan
                assert_eq!(execution_plan.len(), 2);
                assert_eq!(execution_plan[0].0, "code-reviewer");
                assert_eq!(execution_plan[1].0, "performance-analyzer");
                
                // Both should be in Review mode
                assert_eq!(execution_plan[0].1.mode, OperatingMode::Review);
                assert_eq!(execution_plan[1].1.mode, OperatingMode::Review);
            }
            _ => panic!("Expected sequential execution plan"),
        }
    }

    #[test]
    fn test_parallel_agent_planning() {
        let (registry, _temp_dir) = create_test_registry_with_agents();
        
        // Parse parallel invocation
        let parser = InvocationParser::new();
        let invocation = parser.parse("@code-reviewer + @performance-analyzer").unwrap().unwrap();
        
        match invocation.execution_plan {
            ExecutionPlan::Parallel(agents) => {
                assert_eq!(agents.len(), 2);
                
                // Plan parallel execution
                let mut parallel_executions = Vec::new();
                
                for agent_inv in agents {
                    let agent_info = registry.get_agent(&agent_inv.agent_name).unwrap();
                    
                    let context = SubagentContext {
                        execution_id: uuid::Uuid::new_v4(),
                        mode: agent_info.config.mode_override.unwrap_or(OperatingMode::Build),
                        available_tools: agent_info.config.tools.keys().cloned().collect(),
                        conversation_context: String::new(),
                        working_directory: std::env::current_dir().unwrap(),
                        parameters: agent_inv.parameters,
                        metadata: HashMap::new(),
                    };
                    
                    let execution = SubagentExecution::new(agent_inv.agent_name.clone());
                    parallel_executions.push((execution, context));
                }
                
                // Verify parallel planning
                assert_eq!(parallel_executions.len(), 2);
                
                let names: Vec<_> = parallel_executions.iter()
                    .map(|(exec, _)| &exec.agent_name)
                    .collect();
                
                assert!(names.contains(&&"code-reviewer".to_string()));
                assert!(names.contains(&&"performance-analyzer".to_string()));
            }
            _ => panic!("Expected parallel execution plan"),
        }
    }
}

// Error handling and edge cases
#[cfg(test)]
mod error_handling_tests {
    use super::*;
    use super::fixtures::*;

    #[test]
    fn test_missing_agent_handling() {
        let (registry, _temp_dir) = create_test_registry_with_agents();
        
        // Parse invocation for non-existent agent
        let parser = InvocationParser::new();
        let invocation = parser.parse("@nonexistent-agent").unwrap().unwrap();
        
        match invocation.execution_plan {
            ExecutionPlan::Single(agent_inv) => {
                assert_eq!(agent_inv.agent_name, "nonexistent-agent");
                
                // Try to get agent configuration
                let agent_info = registry.get_agent(&agent_inv.agent_name);
                assert!(agent_info.is_none(), "Should not find nonexistent agent");
                
                // This would normally result in an AgentNotFound error
                // in the execution phase
            }
            _ => panic!("Expected single execution plan"),
        }
    }

    #[test]
    fn test_execution_timeout_simulation() {
        let (registry, _temp_dir) = create_test_registry_with_agents();
        
        // Get agent with short timeout
        let agent_info = registry.get_agent("performance-analyzer").unwrap();
        assert!(agent_info.config.timeout_seconds > 0);
        
        // Create execution that would timeout
        let mut execution = SubagentExecution::new("performance-analyzer".to_string());
        execution.start();
        
        // Simulate timeout by marking as failed
        execution.fail("Execution timed out".to_string());
        
        assert!(matches!(execution.status, SubagentStatus::Failed(_)));
        assert!(execution.error.is_some());
        assert!(execution.error.unwrap().contains("timed out"));
    }

    #[test]
    fn test_circular_dependency_prevention() {
        let parser = InvocationParser::new();
        
        // Create a chain with circular dependency
        let chain = AgentChain {
            agents: vec![
                AgentInvocation {
                    agent_name: "agent1".to_string(),
                    parameters: HashMap::new(),
                    raw_parameters: String::new(),
                    position: 0,
                },
                AgentInvocation {
                    agent_name: "agent2".to_string(),
                    parameters: HashMap::new(),
                    raw_parameters: String::new(),
                    position: 1,
                },
                AgentInvocation {
                    agent_name: "agent1".to_string(), // Circular!
                    parameters: HashMap::new(),
                    raw_parameters: String::new(),
                    position: 2,
                },
            ],
            pass_output: true,
        };
        
        let plan = ExecutionPlan::Sequential(chain);
        let result = parser.validate_execution_plan(&plan);
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SubagentError::CircularDependency { .. }));
    }
}

// Performance tests
#[cfg(test)]
mod performance_tests {
    use super::*;
    use super::fixtures::*;

    #[test]
    fn test_large_agent_registry_performance() {
        let temp_dir = tempdir().unwrap();
        let global_dir = temp_dir.path().join("global");
        let templates_dir = temp_dir.path().join("templates");
        
        std::fs::create_dir_all(&global_dir).unwrap();
        std::fs::create_dir_all(&templates_dir).unwrap();
        
        let registry = SubagentRegistry {
            global_agents_dir: global_dir.clone(),
            project_agents_dir: None,
            templates_dir,
            agents: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            templates: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            watch_enabled: false,
            last_scan: std::sync::Arc::new(std::sync::Mutex::new(std::time::SystemTime::UNIX_EPOCH)),
        };
        
        // Create many agent configurations
        let start = std::time::Instant::now();
        
        for i in 0..20 {
            let config = SubagentConfig {
                name: format!("agent-{}", i),
                description: format!("Test agent {}", i),
                mode_override: None,
                intelligence: IntelligenceLevel::Medium,
                tools: HashMap::new(),
                prompt: "Test prompt".to_string(),
                parameters: vec![],
                template: None,
                timeout_seconds: 60,
                chainable: true,
                parallelizable: true,
                metadata: HashMap::new(),
                file_patterns: vec!["*.rs".to_string()],
                tags: vec!["test".to_string()],
            };
            
            let config_path = global_dir.join(format!("agent-{}.toml", i));
            config.to_file(&config_path).unwrap();
        }
        
        // Load all agents
        let load_start = std::time::Instant::now();
        registry.load_all().unwrap();
        let load_duration = load_start.elapsed();
        
        // Should load 20 agents quickly
        assert!(load_duration < std::time::Duration::from_millis(500));
        
        // Verify all agents were loaded
        assert_eq!(registry.get_all_agents().len(), 20);
        
        let total_duration = start.elapsed();
        println!("Total time for 20 agents: {:?}", total_duration);
        println!("Load time: {:?}", load_duration);
    }

    #[test]
    fn test_invocation_parsing_performance() {
        let parser = InvocationParser::new();
        
        let test_inputs = vec![
            "@simple-agent",
            "@agent with parameters param1=value1",
            "@agent1 → @agent2 → @agent3",
            "@agent1 + @agent2 + @agent3",
            "@complex param1=val1 param2=\"quoted value\" → @simple + @other param=x",
        ];
        
        let start = std::time::Instant::now();
        
        // Parse multiple times
        for _ in 0..100 {
            for input in &test_inputs {
                let _result = parser.parse(input).unwrap();
            }
        }
        
        let duration = start.elapsed();
        
        // Should parse quickly
        assert!(duration < std::time::Duration::from_millis(100));
        
        println!("Parsed {} invocations in {:?}", test_inputs.len() * 100, duration);
    }
}

// Real-world scenario tests
#[cfg(test)]
mod scenario_tests {
    use super::*;
    use super::fixtures::*;

    #[test]
    fn test_code_review_workflow_simulation() {
        let (registry, _temp_dir) = create_test_registry_with_agents();
        let project_dir = create_test_project();
        
        // Simulate a code review workflow
        let workflows = vec![
            // Simple review
            "@code-reviewer files=src/main.rs",
            // Review then performance analysis
            "@code-reviewer → @performance-analyzer",
            // Parallel analysis
            "@code-reviewer + @performance-analyzer",
        ];
        
        let parser = InvocationParser::new();
        
        for workflow in workflows {
            let invocation = parser.parse(workflow).unwrap().unwrap();
            
            // Verify the workflow can be planned
            match invocation.execution_plan {
                ExecutionPlan::Single(agent_inv) => {
                    let agent_info = registry.get_agent(&agent_inv.agent_name);
                    assert!(agent_info.is_some(), "Agent {} should exist", agent_inv.agent_name);
                }
                ExecutionPlan::Sequential(chain) => {
                    for agent_inv in &chain.agents {
                        let agent_info = registry.get_agent(&agent_inv.agent_name);
                        assert!(agent_info.is_some(), "Agent {} should exist", agent_inv.agent_name);
                    }
                }
                ExecutionPlan::Parallel(agents) => {
                    for agent_inv in &agents {
                        let agent_info = registry.get_agent(&agent_inv.agent_name);
                        assert!(agent_info.is_some(), "Agent {} should exist", agent_inv.agent_name);
                    }
                }
                ExecutionPlan::Mixed(_) => {
                    // Mixed workflows should also be verifiable
                }
            }
            
            println!("Successfully planned workflow: {}", workflow);
        }
        
        // Verify project structure was created
        assert!(project_dir.path().join("src/main.rs").exists());
        assert!(project_dir.path().join("src/lib.rs").exists());
    }

    #[test]
    fn test_agent_context_isolation() {
        let (registry, _temp_dir) = create_test_registry_with_agents();
        
        // Create multiple execution contexts for the same agent
        let agent_info = registry.get_agent("code-reviewer").unwrap();
        
        let contexts = (0..3).map(|i| {
            SubagentContext {
                execution_id: uuid::Uuid::new_v4(),
                mode: OperatingMode::Build,
                available_tools: agent_info.config.tools.keys().cloned().collect(),
                conversation_context: format!("Context {}", i),
                working_directory: std::env::current_dir().unwrap(),
                parameters: {
                    let mut params = HashMap::new();
                    params.insert("file".to_string(), format!("file{}.rs", i));
                    params
                },
                metadata: HashMap::new(),
            }
        }).collect::<Vec<_>>();
        
        // Verify contexts are isolated
        for (i, context) in contexts.iter().enumerate() {
            assert!(context.conversation_context.contains(&format!("Context {}", i)));
            assert_eq!(context.parameters.get("file").unwrap(), &format!("file{}.rs", i));
            
            // Each should have a unique execution ID
            for (j, other_context) in contexts.iter().enumerate() {
                if i != j {
                    assert_ne!(context.execution_id, other_context.execution_id);
                }
            }
        }
    }
}