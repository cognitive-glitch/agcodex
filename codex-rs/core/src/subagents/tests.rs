//! Basic tests for the subagent system
//!
//! This module provides foundational tests for the agent system including registry loading,
//! invocation parsing, basic orchestration, and error handling.

use super::*;
use crate::modes::OperatingMode;
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;
use uuid::Uuid;

// Test fixtures and helpers
mod fixtures {
    use super::*;
    
    pub fn create_test_config() -> SubagentConfig {
        SubagentConfig {
            name: "test-agent".to_string(),
            description: "Test agent for unit testing".to_string(),
            mode_override: Some(OperatingMode::Review),
            intelligence: IntelligenceLevel::Medium,
            tools: {
                let mut tools = HashMap::new();
                tools.insert("ast_search".to_string(), ToolPermission::Read);
                tools.insert("file_read".to_string(), ToolPermission::Read);
                tools
            },
            prompt: "You are a test agent.".to_string(),
            parameters: vec![
                config::ParameterDefinition {
                    name: "file".to_string(),
                    description: "File to analyze".to_string(),
                    required: false,
                    default: Some("**/*.rs".to_string()),
                    valid_values: None,
                },
            ],
            template: None,
            timeout_seconds: 30,
            chainable: true,
            parallelizable: true,
            metadata: HashMap::new(),
            file_patterns: vec!["*.rs".to_string()],
            tags: vec!["test".to_string()],
        }
    }
    
    pub fn create_test_registry() -> (SubagentRegistry, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let global_dir = temp_dir.path().join("global");
        let templates_dir = temp_dir.path().join("templates");
        
        std::fs::create_dir_all(&global_dir).unwrap();
        std::fs::create_dir_all(&templates_dir).unwrap();
        
        let registry = SubagentRegistry {
            global_agents_dir: global_dir,
            project_agents_dir: None,
            templates_dir,
            agents: Arc::new(std::sync::Mutex::new(HashMap::new())),
            templates: Arc::new(std::sync::Mutex::new(HashMap::new())),
            watch_enabled: false,
            last_scan: Arc::new(std::sync::Mutex::new(std::time::SystemTime::UNIX_EPOCH)),
        };
        
        (registry, temp_dir)
    }
}

// Registry tests
#[cfg(test)]
mod registry_tests {
    use super::*;
    use super::fixtures::*;

    #[test]
    fn test_registry_creation() {
        let (registry, _temp_dir) = create_test_registry();
        assert!(registry.global_agents_dir.exists());
        assert!(registry.templates_dir.exists());
        assert_eq!(registry.get_all_agents().len(), 0);
        assert_eq!(registry.get_all_templates().len(), 0);
    }

    #[test]
    fn test_agent_loading() {
        let (registry, _temp_dir) = create_test_registry();
        
        // Create test agent configuration
        let config = create_test_config();
        let config_path = registry.global_agents_dir.join("test-agent.toml");
        config.to_file(&config_path).unwrap();
        
        // Load agents
        registry.load_all().unwrap();
        
        // Verify agent was loaded
        let loaded_agent = registry.get_agent("test-agent").unwrap();
        assert_eq!(loaded_agent.config.name, "test-agent");
        assert_eq!(loaded_agent.config.description, "Test agent for unit testing");
        assert_eq!(loaded_agent.config.mode_override, Some(OperatingMode::Review));
        assert!(loaded_agent.is_global);
        assert!(loaded_agent.config.tools.contains_key("ast_search"));
    }

    #[test]
    fn test_agent_filtering() {
        let (registry, _temp_dir) = create_test_registry();
        
        // Create multiple test agents with different patterns and tags
        let configs = vec![
            {
                let mut config = create_test_config();
                config.name = "rust-agent".to_string();
                config.file_patterns = vec!["*.rs".to_string()];
                config.tags = vec!["rust".to_string(), "backend".to_string()];
                config
            },
            {
                let mut config = create_test_config();
                config.name = "js-agent".to_string();
                config.file_patterns = vec!["*.js".to_string(), "*.ts".to_string()];
                config.tags = vec!["javascript".to_string(), "frontend".to_string()];
                config
            },
        ];
        
        for config in configs {
            let config_path = registry.global_agents_dir.join(format!("{}.toml", config.name));
            config.to_file(&config_path).unwrap();
        }
        
        registry.load_all().unwrap();
        
        // Test file pattern filtering
        let rust_file = PathBuf::from("src/main.rs");
        let rust_agents = registry.get_agents_for_file(&rust_file);
        let rust_names: Vec<_> = rust_agents.iter().map(|a| &a.config.name).collect();
        assert!(rust_names.contains(&&"rust-agent".to_string()));
        
        // Test tag filtering
        let backend_agents = registry.get_agents_with_tags(&vec!["backend".to_string()]);
        assert_eq!(backend_agents.len(), 1);
        assert_eq!(backend_agents[0].config.name, "rust-agent");
    }

    #[test]
    fn test_agent_name_conflict_detection() {
        let (registry, _temp_dir) = create_test_registry();
        
        // Create two agents with the same name
        let config1 = create_test_config();
        let config2 = create_test_config();
        
        let config_path1 = registry.global_agents_dir.join("test-agent-1.toml");
        let config_path2 = registry.global_agents_dir.join("test-agent-2.toml");
        
        config1.to_file(&config_path1).unwrap();
        config2.to_file(&config_path2).unwrap();
        
        // Loading should fail due to name conflict
        let result = registry.load_all();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            registry::SubagentRegistryError::NameConflict { .. }
        ));
    }
}

// Invocation parser tests
#[cfg(test)]
mod invocation_tests {
    use super::*;
    use super::invocation::*;

    #[test]
    fn test_single_agent_parsing() {
        let parser = InvocationParser::new();
        
        // Simple invocation
        let result = parser.parse("@code-reviewer").unwrap().unwrap();
        match result.execution_plan {
            ExecutionPlan::Single(inv) => {
                assert_eq!(inv.agent_name, "code-reviewer");
                assert_eq!(inv.raw_parameters, "");
            }
            _ => panic!("Expected single execution plan"),
        }
        
        // With parameters
        let result = parser.parse("@code-reviewer check this file").unwrap().unwrap();
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
        
        // Simple chain
        let result = parser.parse("@refactorer → @test-writer").unwrap().unwrap();
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
        
        // Simple parallel
        let result = parser.parse("@performance + @security").unwrap().unwrap();
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
        let params = parser.parse_parameters("file=src/main.rs level=high").unwrap();
        assert_eq!(params.get("file").unwrap(), "src/main.rs");
        assert_eq!(params.get("level").unwrap(), "high");
        
        // Test positional arguments
        let params = parser.parse_parameters("src/main.rs high").unwrap();
        assert_eq!(params.get("arg0").unwrap(), "src/main.rs");
        assert_eq!(params.get("arg1").unwrap(), "high");
    }

    #[test]
    fn test_context_extraction() {
        let parser = InvocationParser::new();
        let result = parser.parse("Please @code-reviewer this file and make sure it's secure.").unwrap().unwrap();
        
        // Context should have agent invocations removed
        assert!(result.context.contains("Please"));
        assert!(result.context.contains("secure"));
        assert!(!result.context.contains("@code-reviewer"));
    }

    #[test]
    fn test_no_agents() {
        let parser = InvocationParser::new();
        let result = parser.parse("This is just regular text with no agents.").unwrap();
        assert!(result.is_none());
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
                },
                AgentInvocation {
                    agent_name: "agent1".to_string(), // Duplicate!
                    parameters: HashMap::new(),
                    raw_parameters: String::new(),
                    position: 1,
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

// Error handling tests
#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn test_subagent_error_types() {
        // Test all error variants can be created and display properly
        let errors = vec![
            SubagentError::AgentNotFound {
                name: "missing-agent".to_string(),
            },
            SubagentError::InvalidConfig("Bad config".to_string()),
            SubagentError::ExecutionFailed("Agent crashed".to_string()),
            SubagentError::CircularDependency {
                chain: vec!["a".to_string(), "b".to_string(), "a".to_string()],
            },
            SubagentError::Timeout {
                name: "slow-agent".to_string(),
            },
            SubagentError::ToolPermissionDenied {
                tool: "write_file".to_string(),
                agent: "read-only-agent".to_string(),
            },
            SubagentError::ModeRestriction {
                mode: OperatingMode::Plan,
                operation: "file_write".to_string(),
            },
        ];
        
        for error in errors {
            let error_string = error.to_string();
            assert!(!error_string.is_empty());
            
            // Test that error implements required traits
            let _: Box<dyn std::error::Error> = Box::new(error);
        }
    }

    #[test]
    fn test_error_conversion() {
        // Test IO error conversion
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let subagent_error: SubagentError = io_error.into();
        assert!(matches!(subagent_error, SubagentError::Io(_)));
    }

    #[test] 
    fn test_result_types() {
        // Test that our result types work correctly
        let success: SubagentResult<String> = Ok("success".to_string());
        assert!(success.is_ok());
        assert_eq!(success.unwrap(), "success");
        
        let failure: SubagentResult<String> = Err(SubagentError::AgentNotFound {
            name: "test".to_string(),
        });
        assert!(failure.is_err());
        
        // Test registry result types  
        let reg_success: registry::RegistryResult<i32> = Ok(42);
        assert!(reg_success.is_ok());
        
        let reg_failure: registry::RegistryResult<i32> = Err(registry::SubagentRegistryError::ConfigNotFound {
            path: PathBuf::from("missing.toml"),
        });
        assert!(reg_failure.is_err());
    }
}

// Basic execution tests
#[cfg(test)]
mod execution_tests {
    use super::*;

    #[test]
    fn test_subagent_execution_lifecycle() {
        let mut execution = SubagentExecution::new("test-agent".to_string());
        
        assert_eq!(execution.status, SubagentStatus::Pending);
        assert_eq!(execution.agent_name, "test-agent");
        
        execution.start();
        assert_eq!(execution.status, SubagentStatus::Running);
        
        execution.complete("Success!".to_string(), vec![]);
        assert_eq!(execution.status, SubagentStatus::Completed);
        assert_eq!(execution.output.as_ref().unwrap(), "Success!");
        assert!(execution.duration().is_some());
    }
    
    #[test]
    fn test_subagent_execution_failure() {
        let mut execution = SubagentExecution::new("failing-agent".to_string());
        
        execution.start();
        execution.fail("Something went wrong".to_string());
        
        assert!(matches!(execution.status, SubagentStatus::Failed(_)));
        assert_eq!(execution.error.as_ref().unwrap(), "Something went wrong");
        assert!(execution.duration().is_some());
    }

    #[test]
    fn test_subagent_context_creation() {
        let context = SubagentContext {
            execution_id: Uuid::new_v4(),
            mode: OperatingMode::Build,
            available_tools: vec!["ast_search".to_string(), "file_read".to_string()],
            conversation_context: "Previous context".to_string(),
            working_directory: std::env::current_dir().unwrap(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("file".to_string(), "src/main.rs".to_string());
                params
            },
            metadata: HashMap::new(),
        };
        
        assert_eq!(context.mode, OperatingMode::Build);
        assert_eq!(context.available_tools.len(), 2);
        assert!(context.available_tools.contains(&"ast_search".to_string()));
        assert!(context.conversation_context.contains("Previous context"));
        assert_eq!(context.parameters.get("file").unwrap(), "src/main.rs");
    }
}

// Integration tests
#[cfg(test)]
mod integration_tests {
    use super::*;
    use super::fixtures::*;

    #[test]
    fn test_end_to_end_agent_discovery() {
        // This test simulates discovering and preparing to execute an agent
        let (registry, _temp_dir) = create_test_registry();
        
        // Create test agent
        let config = create_test_config();
        let config_path = registry.global_agents_dir.join("test-agent.toml");
        config.to_file(&config_path).unwrap();
        
        registry.load_all().unwrap();
        
        // Parse invocation
        let parser = InvocationParser::new();
        let invocation = parser.parse("@test-agent check src/main.rs").unwrap().unwrap();
        
        // Verify invocation
        match invocation.execution_plan {
            ExecutionPlan::Single(inv) => {
                assert_eq!(inv.agent_name, "test-agent");
                assert_eq!(inv.raw_parameters, "check src/main.rs");
                
                // Get agent configuration
                let agent_info = registry.get_agent(&inv.agent_name).unwrap();
                assert_eq!(agent_info.config.name, "test-agent");
                assert!(agent_info.config.tools.contains_key("ast_search"));
                
                // Create execution context
                let context = SubagentContext {
                    execution_id: Uuid::new_v4(),
                    mode: agent_info.config.mode_override.unwrap_or(OperatingMode::Build),
                    available_tools: agent_info.config.tools.keys().cloned().collect(),
                    conversation_context: String::new(),
                    working_directory: std::env::current_dir().unwrap(),
                    parameters: inv.parameters,
                    metadata: HashMap::new(),
                };
                
                assert_eq!(context.mode, OperatingMode::Review);
                assert!(context.available_tools.contains(&"ast_search".to_string()));
            }
            _ => panic!("Expected single execution plan"),
        }
    }

    #[test]
    fn test_multi_agent_planning() {
        let (registry, _temp_dir) = create_test_registry();
        
        // Create multiple agents
        let agents = vec!["code-reviewer", "refactorer", "test-writer"];
        for agent_name in &agents {
            let mut config = create_test_config();
            config.name = agent_name.to_string();
            
            let config_path = registry.global_agents_dir.join(format!("{}.toml", agent_name));
            config.to_file(&config_path).unwrap();
        }
        
        registry.load_all().unwrap();
        
        // Parse complex invocation
        let parser = InvocationParser::new();
        let invocation = parser.parse("@code-reviewer → @refactorer → @test-writer").unwrap().unwrap();
        
        match invocation.execution_plan {
            ExecutionPlan::Sequential(chain) => {
                assert_eq!(chain.agents.len(), 3);
                assert!(chain.pass_output);
                
                // Verify all agents exist in registry
                for agent_inv in &chain.agents {
                    let agent_info = registry.get_agent(&agent_inv.agent_name);
                    assert!(agent_info.is_some(), "Agent {} not found", agent_inv.agent_name);
                }
                
                // Verify execution order
                assert_eq!(chain.agents[0].agent_name, "code-reviewer");
                assert_eq!(chain.agents[1].agent_name, "refactorer");
                assert_eq!(chain.agents[2].agent_name, "test-writer");
            }
            _ => panic!("Expected sequential execution plan"),
        }
    }
}

// Performance tests
#[cfg(test)]
mod performance_tests {
    use super::*;
    use super::fixtures::*;

    #[test]
    fn test_large_registry_performance() {
        let (registry, _temp_dir) = create_test_registry();
        
        // Create many agent configurations
        let start = std::time::Instant::now();
        
        for i in 0..50 {
            let mut config = create_test_config();
            config.name = format!("agent-{}", i);
            
            let config_path = registry.global_agents_dir.join(format!("agent-{}.toml", i));
            config.to_file(&config_path).unwrap();
        }
        
        // Load all agents
        let load_start = std::time::Instant::now();
        registry.load_all().unwrap();
        let load_duration = load_start.elapsed();
        
        // Should load 50 agents reasonably quickly (under 1 second)
        assert!(load_duration < std::time::Duration::from_secs(1));
        
        // Verify all agents were loaded
        assert_eq!(registry.get_all_agents().len(), 50);
        
        let total_duration = start.elapsed();
        println!("Total time for 50 agents: {:?}", total_duration);
        println!("Load time: {:?}", load_duration);
    }

    #[test]
    fn test_complex_invocation_parsing_performance() {
        let parser = InvocationParser::new();
        
        // Create a complex invocation string
        let complex_input = "@agent1 param1=value1 → @agent2 + @agent3 file=test.rs → @agent4";
        
        let start = std::time::Instant::now();
        
        // Parse the same complex input many times
        for _ in 0..100 {
            let result = parser.parse(complex_input).unwrap();
            assert!(result.is_some());
        }
        
        let duration = start.elapsed();
        
        // Should parse 100 complex invocations quickly (under 50ms)
        assert!(duration < std::time::Duration::from_millis(50));
        println!("Parsed 100 complex invocations in: {:?}", duration);
    }
}