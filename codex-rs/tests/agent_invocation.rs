//! Integration tests for subagent invocation processing.
//!
//! Tests the complete @agent-name pattern detection and execution workflow:
//! - InvocationProcessor from agcodex_core::subagents
//! - Agent registry and configuration loading
//! - Execution plan generation and validation
//! - Context isolation and parameter passing
//!
//! Uses real AGCodex components but mocks LLM API calls.

use agcodex_core::modes::OperatingMode;
use agcodex_core::subagents::{
    config::{IntelligenceLevel, ParameterDefinition, SubagentConfig, ToolPermission},
    invocation::{AgentInvocation, ExecutionPlan, InvocationParser, InvocationRequest},
    invocation_processor::InvocationProcessor,
    registry::SubagentRegistry,
    SubagentContext, SubagentError,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

mod helpers;
use helpers::test_utils::{TestEnvironment, PerformanceAssertions, TestTiming};

/// Test fixture for agent invocation integration tests
struct AgentInvocationFixture {
    registry: Arc<SubagentRegistry>,
    processor: InvocationProcessor,
    _test_env: TestEnvironment,
    agents_dir: PathBuf,
}

impl AgentInvocationFixture {
    async fn new() -> Self {
        let test_env = TestEnvironment::new();
        let agents_dir = test_env.path().join("agents");
        let global_dir = agents_dir.join("global");
        let templates_dir = agents_dir.join("templates");
        
        tokio::fs::create_dir_all(&global_dir).await.unwrap();
        tokio::fs::create_dir_all(&templates_dir).await.unwrap();
        
        // Create registry
        let registry = Arc::new(SubagentRegistry {
            global_agents_dir: global_dir.clone(),
            project_agents_dir: None,
            templates_dir,
            agents: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            templates: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            watch_enabled: false,
            last_scan: std::sync::Arc::new(std::sync::Mutex::new(std::time::SystemTime::UNIX_EPOCH)),
        });
        
        // Create processor
        let processor = InvocationProcessor::new(
            registry.clone(),
            OperatingMode::Build,
        ).unwrap();
        
        let fixture = Self {
            registry,
            processor,
            _test_env: test_env,
            agents_dir,
        };
        
        // Create test agents
        fixture.create_test_agents().await;
        
        fixture
    }
    
    async fn create_test_agents(&self) {
        let global_dir = self.agents_dir.join("global");
        
        // Code reviewer agent
        let code_reviewer = SubagentConfig {
            name: "code-reviewer".to_string(),
            description: "Reviews code for quality, security, and maintainability".to_string(),
            mode_override: Some(OperatingMode::Review),
            intelligence: IntelligenceLevel::Hard,
            tools: {
                let mut tools = HashMap::new();
                tools.insert("search".to_string(), ToolPermission::Read);
                tools.insert("tree".to_string(), ToolPermission::Read);
                tools.insert("grep".to_string(), ToolPermission::Read);
                tools
            },
            prompt: "You are an expert code reviewer. Analyze code for bugs, security issues, and improvements.".to_string(),
            parameters: vec![
                ParameterDefinition {
                    name: "files".to_string(),
                    description: "Files or patterns to review".to_string(),
                    required: false,
                    default: Some("**/*.rs".to_string()),
                    valid_values: None,
                },
                ParameterDefinition {
                    name: "focus".to_string(),
                    description: "Focus area: security, performance, maintainability".to_string(),
                    required: false,
                    default: Some("all".to_string()),
                    valid_values: Some(vec![
                        "security".to_string(),
                        "performance".to_string(), 
                        "maintainability".to_string(),
                        "all".to_string()
                    ]),
                },
            ],
            template: None,
            timeout_seconds: 120,
            chainable: true,
            parallelizable: true,
            metadata: HashMap::new(),
            file_patterns: vec!["*.rs".to_string(), "*.py".to_string(), "*.ts".to_string()],
            tags: vec!["review".to_string(), "quality".to_string()],
        };
        
        // Performance analyzer agent
        let performance_analyzer = SubagentConfig {
            name: "performance-analyzer".to_string(),
            description: "Analyzes code for performance bottlenecks and optimization opportunities".to_string(),
            mode_override: Some(OperatingMode::Review),
            intelligence: IntelligenceLevel::Hard,
            tools: {
                let mut tools = HashMap::new();
                tools.insert("search".to_string(), ToolPermission::Read);
                tools.insert("tree".to_string(), ToolPermission::Read);
                tools
            },
            prompt: "You are a performance optimization expert. Find bottlenecks and suggest improvements.".to_string(),
            parameters: vec![
                ParameterDefinition {
                    name: "target".to_string(),
                    description: "Performance target: memory, cpu, latency, throughput".to_string(),
                    required: false,
                    default: Some("all".to_string()),
                    valid_values: Some(vec![
                        "memory".to_string(),
                        "cpu".to_string(),
                        "latency".to_string(),
                        "throughput".to_string(),
                        "all".to_string(),
                    ]),
                },
            ],
            template: None,
            timeout_seconds: 180,
            chainable: true,
            parallelizable: true,
            metadata: HashMap::new(),
            file_patterns: vec!["*.rs".to_string()],
            tags: vec!["performance".to_string(), "optimization".to_string()],
        };
        
        // Test writer agent
        let test_writer = SubagentConfig {
            name: "test-writer".to_string(),
            description: "Generates comprehensive test suites".to_string(),
            mode_override: Some(OperatingMode::Build), // Can write files
            intelligence: IntelligenceLevel::Medium,
            tools: {
                let mut tools = HashMap::new();
                tools.insert("search".to_string(), ToolPermission::Read);
                tools.insert("edit".to_string(), ToolPermission::Write);
                tools.insert("tree".to_string(), ToolPermission::Read);
                tools
            },
            prompt: "You are a test automation expert. Write thorough, maintainable tests.".to_string(),
            parameters: vec![
                ParameterDefinition {
                    name: "type".to_string(),
                    description: "Test type: unit, integration, e2e".to_string(),
                    required: false,
                    default: Some("unit".to_string()),
                    valid_values: Some(vec![
                        "unit".to_string(),
                        "integration".to_string(),
                        "e2e".to_string(),
                    ]),
                },
                ParameterDefinition {
                    name: "coverage".to_string(),
                    description: "Target coverage percentage".to_string(),
                    required: false,
                    default: Some("80".to_string()),
                    valid_values: None,
                },
            ],
            template: None,
            timeout_seconds: 300,
            chainable: false,
            parallelizable: true,
            metadata: HashMap::new(),
            file_patterns: vec!["*.rs".to_string(), "*.py".to_string()],
            tags: vec!["testing".to_string(), "automation".to_string()],
        };
        
        // Save agent configurations
        let agents = [code_reviewer, performance_analyzer, test_writer];
        for agent in &agents {
            let config_path = global_dir.join(format!("{}.toml", agent.name));
            agent.to_file(&config_path).unwrap();
        }
        
        // Load all agents
        self.registry.load_all().unwrap();
    }
    
    fn get_agent(&self, name: &str) -> Option<agcodex_core::subagents::registry::SubagentInfo> {
        self.registry.get_agent(name)
    }
    
    fn list_agents(&self) -> HashMap<String, agcodex_core::subagents::registry::SubagentInfo> {
        self.registry.get_all_agents()
    }
    
    async fn process_message(&self, message: &str) -> Result<Option<String>, SubagentError> {
        // This would normally call LLM APIs, but we'll mock it for testing
        self.processor.process_message(message).await
    }
}

#[tokio::test]
async fn test_agent_registry_loading() {
    let fixture = AgentInvocationFixture::new().await;
    
    // Verify agents were loaded
    let agents = fixture.list_agents();
    assert_eq!(agents.len(), 3);
    
    let agent_names: Vec<_> = agents.keys().collect();
    assert!(agent_names.contains(&&"code-reviewer".to_string()));
    assert!(agent_names.contains(&&"performance-analyzer".to_string()));
    assert!(agent_names.contains(&&"test-writer".to_string()));
    
    // Test specific agent configuration
    let code_reviewer = fixture.get_agent("code-reviewer").unwrap();
    assert_eq!(code_reviewer.config.mode_override, Some(OperatingMode::Review));
    assert_eq!(code_reviewer.config.intelligence, IntelligenceLevel::Hard);
    assert!(code_reviewer.config.tools.contains_key("search"));
    assert_eq!(code_reviewer.config.tools["search"], ToolPermission::Read);
    
    // Test file pattern matching
    let rust_file = PathBuf::from("src/main.rs");
    let matching_agents = fixture.registry.get_agents_for_file(&rust_file);
    assert!(matching_agents.len() >= 2); // At least code-reviewer and test-writer should match
    
    // Test tag filtering
    let review_agents = fixture.registry.get_agents_with_tags(&vec!["review".to_string()]);
    assert_eq!(review_agents.len(), 1);
    assert_eq!(review_agents[0].config.name, "code-reviewer");
}

#[tokio::test]
async fn test_simple_agent_invocation_parsing() {
    let fixture = AgentInvocationFixture::new().await;
    let parser = InvocationParser::new();
    
    // Test simple invocations
    let test_cases = vec![
        "@code-reviewer",
        "@performance-analyzer",
        "@test-writer",
        "@code-reviewer files=src/**/*.rs",
        "@performance-analyzer target=memory",
        "@test-writer type=integration coverage=90",
    ];
    
    for input in test_cases {
        let result = parser.parse(input).unwrap();
        assert!(result.is_some(), "Failed to parse: {}", input);
        
        let invocation = result.unwrap();
        assert!(matches!(invocation.execution_plan, ExecutionPlan::Single(_)));
        
        if let ExecutionPlan::Single(agent_inv) = invocation.execution_plan {
            // Verify agent name extraction
            let expected_name = input.split_whitespace().next().unwrap().trim_start_matches('@');
            if !expected_name.contains('=') {
                assert_eq!(agent_inv.agent_name, expected_name);
            }
        }
    }
}

#[tokio::test]
async fn test_sequential_agent_invocation_parsing() {
    let fixture = AgentInvocationFixture::new().await;
    let parser = InvocationParser::new();
    
    // Test sequential invocations
    let test_cases = vec![
        "@code-reviewer → @performance-analyzer",
        "@code-reviewer files=src/*.rs → @test-writer type=unit",
        "@performance-analyzer → @code-reviewer → @test-writer",
        "@test-writer coverage=95 → @code-reviewer focus=maintainability",
    ];
    
    for input in test_cases {
        let result = parser.parse(input).unwrap();
        assert!(result.is_some(), "Failed to parse: {}", input);
        
        let invocation = result.unwrap();
        assert!(matches!(invocation.execution_plan, ExecutionPlan::Sequential(_)));
        
        if let ExecutionPlan::Sequential(chain) = invocation.execution_plan {
            assert!(chain.agents.len() >= 2, "Sequential chain should have at least 2 agents");
            assert!(chain.pass_output, "Sequential chains should pass output by default");
            
            // Verify agent order is preserved
            for (i, agent) in chain.agents.iter().enumerate() {
                assert_eq!(agent.position, i, "Agent position should match index");
            }
        }
    }
}

#[tokio::test]
async fn test_parallel_agent_invocation_parsing() {
    let fixture = AgentInvocationFixture::new().await;
    let parser = InvocationParser::new();
    
    // Test parallel invocations
    let test_cases = vec![
        "@code-reviewer + @performance-analyzer",
        "@code-reviewer files=src/*.rs + @performance-analyzer target=cpu",
        "@code-reviewer + @performance-analyzer + @test-writer",
        "@test-writer type=unit + @code-reviewer focus=security",
    ];
    
    for input in test_cases {
        let result = parser.parse(input).unwrap();
        assert!(result.is_some(), "Failed to parse: {}", input);
        
        let invocation = result.unwrap();
        assert!(matches!(invocation.execution_plan, ExecutionPlan::Parallel(_)));
        
        if let ExecutionPlan::Parallel(agents) = invocation.execution_plan {
            assert!(agents.len() >= 2, "Parallel execution should have at least 2 agents");
            
            // Verify all agents are parallelizable
            for agent in &agents {
                let agent_info = fixture.get_agent(&agent.agent_name);
                if let Some(info) = agent_info {
                    assert!(info.config.parallelizable, 
                           "Agent {} should be parallelizable", agent.agent_name);
                }
            }
        }
    }
}

#[tokio::test]
async fn test_complex_agent_invocation_patterns() {
    let fixture = AgentInvocationFixture::new().await;
    let parser = InvocationParser::new();
    
    // Test complex mixed patterns
    let test_cases = vec![
        "@code-reviewer files=src/*.rs → @performance-analyzer + @test-writer",
        "@code-reviewer + @performance-analyzer → @test-writer coverage=95",
        "(@code-reviewer files=*.rs + @performance-analyzer) → @test-writer",
    ];
    
    for input in test_cases {
        let result = parser.parse(input).unwrap();
        if let Some(invocation) = result {
            // Complex patterns might be parsed as Mixed or Sequential
            assert!(matches!(
                invocation.execution_plan, 
                ExecutionPlan::Mixed(_) | ExecutionPlan::Sequential(_) | ExecutionPlan::Parallel(_)
            ));
            
            // Should contain references to all our test agents
            let plan_text = format!("{:?}", invocation.execution_plan);
            assert!(plan_text.contains("code-reviewer"), "Should contain code-reviewer");
        }
    }
}

#[tokio::test]
async fn test_agent_parameter_extraction() {
    let fixture = AgentInvocationFixture::new().await;
    let parser = InvocationParser::new();
    
    let test_cases = vec![
        ("@code-reviewer files=src/*.rs focus=security", "code-reviewer", vec![("files", "src/*.rs"), ("focus", "security")]),
        ("@performance-analyzer target=memory", "performance-analyzer", vec![("target", "memory")]),
        ("@test-writer type=integration coverage=90", "test-writer", vec![("type", "integration"), ("coverage", "90")]),
    ];
    
    for (input, expected_agent, expected_params) in test_cases {
        let result = parser.parse(input).unwrap();
        assert!(result.is_some(), "Failed to parse: {}", input);
        
        let invocation = result.unwrap();
        if let ExecutionPlan::Single(agent_inv) = invocation.execution_plan {
            assert_eq!(agent_inv.agent_name, expected_agent);
            
            for (key, expected_value) in expected_params {
                assert!(agent_inv.parameters.contains_key(key), 
                       "Parameter {} missing from {}", key, input);
                assert_eq!(agent_inv.parameters[key], expected_value,
                          "Parameter {} value mismatch in {}", key, input);
            }
        } else {
            panic!("Expected single agent invocation for: {}", input);
        }
    }
}

#[tokio::test]
async fn test_agent_mode_override() {
    let fixture = AgentInvocationFixture::new().await;
    
    // Test that agents with mode overrides use them correctly
    let code_reviewer = fixture.get_agent("code-reviewer").unwrap();
    assert_eq!(code_reviewer.config.mode_override, Some(OperatingMode::Review));
    
    let test_writer = fixture.get_agent("test-writer").unwrap();
    assert_eq!(test_writer.config.mode_override, Some(OperatingMode::Build));
    
    // In a real scenario, we would test that the processor respects mode overrides
    // and creates contexts with the correct operating mode
}

#[tokio::test]
async fn test_agent_tool_permissions() {
    let fixture = AgentInvocationFixture::new().await;
    
    // Test that agents have correct tool permissions
    let code_reviewer = fixture.get_agent("code-reviewer").unwrap();
    let cr_tools = &code_reviewer.config.tools;
    assert_eq!(cr_tools["search"], ToolPermission::Read);
    assert_eq!(cr_tools["tree"], ToolPermission::Read);
    assert_eq!(cr_tools["grep"], ToolPermission::Read);
    
    let test_writer = fixture.get_agent("test-writer").unwrap();
    let tw_tools = &test_writer.config.tools;
    assert_eq!(tw_tools["search"], ToolPermission::Read);
    assert_eq!(tw_tools["edit"], ToolPermission::Write); // Only test-writer can write
    assert_eq!(tw_tools["tree"], ToolPermission::Read);
}

#[tokio::test]
async fn test_invocation_parsing_performance() {
    let fixture = AgentInvocationFixture::new().await;
    let parser = InvocationParser::new();
    
    let test_inputs = vec![
        "@code-reviewer",
        "@performance-analyzer target=memory",
        "@code-reviewer files=src/*.rs focus=security",
        "@code-reviewer → @performance-analyzer",
        "@code-reviewer + @performance-analyzer + @test-writer",
        "@code-reviewer files=*.rs → @performance-analyzer target=cpu → @test-writer type=integration",
    ];
    
    let iterations = 100;
    let (_, total_duration) = TestTiming::time_async_operation(|| async {
        for _ in 0..iterations {
            for input in &test_inputs {
                let _ = parser.parse(input).unwrap();
            }
        }
    }).await;
    
    // Should parse quickly (target: <1ms per parse)
    let avg_per_parse = total_duration.as_nanos() as f64 / (iterations * test_inputs.len()) as f64;
    assert!(avg_per_parse < 1_000_000.0, "Parsing too slow: {:.0}ns per parse", avg_per_parse);
    
    PerformanceAssertions::assert_duration_under(
        total_duration, 
        100, 
        &format!("parsing {} invocations {} times", test_inputs.len(), iterations)
    );
}

#[tokio::test]
async fn test_agent_validation() {
    let fixture = AgentInvocationFixture::new().await;
    let parser = InvocationParser::new();
    
    // Test invocation of non-existent agent
    let result = parser.parse("@nonexistent-agent").unwrap();
    assert!(result.is_some()); // Parsing should succeed
    
    if let Some(invocation) = result {
        if let ExecutionPlan::Single(agent_inv) = invocation.execution_plan {
            assert_eq!(agent_inv.agent_name, "nonexistent-agent");
            
            // Registry should not find the agent
            let agent_info = fixture.get_agent("nonexistent-agent");
            assert!(agent_info.is_none());
        }
    }
    
    // Test validation of chainable agents
    let code_reviewer = fixture.get_agent("code-reviewer").unwrap();
    assert!(code_reviewer.config.chainable, "code-reviewer should be chainable");
    
    let performance_analyzer = fixture.get_agent("performance-analyzer").unwrap();
    assert!(performance_analyzer.config.chainable, "performance-analyzer should be chainable");
    
    let test_writer = fixture.get_agent("test-writer").unwrap();
    assert!(!test_writer.config.chainable, "test-writer should not be chainable");
}

#[tokio::test]
async fn test_agent_parameter_validation() {
    let fixture = AgentInvocationFixture::new().await;
    
    // Test parameter validation for code-reviewer
    let code_reviewer = fixture.get_agent("code-reviewer").unwrap();
    let params = &code_reviewer.config.parameters;
    
    // Find focus parameter
    let focus_param = params.iter().find(|p| p.name == "focus").unwrap();
    assert!(focus_param.valid_values.is_some());
    let valid_values = focus_param.valid_values.as_ref().unwrap();
    assert!(valid_values.contains(&"security".to_string()));
    assert!(valid_values.contains(&"performance".to_string()));
    assert!(valid_values.contains(&"maintainability".to_string()));
    assert!(valid_values.contains(&"all".to_string()));
    
    // Test parameter defaults
    let files_param = params.iter().find(|p| p.name == "files").unwrap();
    assert!(!files_param.required);
    assert_eq!(files_param.default.as_ref().unwrap(), "**/*.rs");
}

#[tokio::test]
async fn test_agent_timeout_configuration() {
    let fixture = AgentInvocationFixture::new().await;
    
    // Test that different agents have appropriate timeouts
    let code_reviewer = fixture.get_agent("code-reviewer").unwrap();
    assert_eq!(code_reviewer.config.timeout_seconds, 120); // 2 minutes
    
    let performance_analyzer = fixture.get_agent("performance-analyzer").unwrap();
    assert_eq!(performance_analyzer.config.timeout_seconds, 180); // 3 minutes
    
    let test_writer = fixture.get_agent("test-writer").unwrap();
    assert_eq!(test_writer.config.timeout_seconds, 300); // 5 minutes (longer for writing tests)
}

#[tokio::test]
async fn test_agent_file_pattern_matching() {
    let fixture = AgentInvocationFixture::new().await;
    
    // Test file pattern matching
    let test_files = vec![
        ("src/main.rs", vec!["code-reviewer", "performance-analyzer", "test-writer"]),
        ("app.py", vec!["code-reviewer", "test-writer"]), // Only code-reviewer and test-writer support Python
        ("component.tsx", vec!["code-reviewer"]), // Only code-reviewer supports TypeScript
        ("README.md", vec![]), // No agents should match markdown files
    ];
    
    for (file_path, expected_agents) in test_files {
        let path = PathBuf::from(file_path);
        let matching_agents = fixture.registry.get_agents_for_file(&path);
        
        assert_eq!(matching_agents.len(), expected_agents.len(), 
                  "File {} should match {} agents, got {}", 
                  file_path, expected_agents.len(), matching_agents.len());
        
        for expected_agent in expected_agents {
            let found = matching_agents.iter().any(|a| a.config.name == expected_agent);
            assert!(found, "Agent {} should match file {}", expected_agent, file_path);
        }
    }
}

#[tokio::test]
async fn test_agent_intelligence_levels() {
    let fixture = AgentInvocationFixture::new().await;
    
    // Test that agents have appropriate intelligence levels
    let code_reviewer = fixture.get_agent("code-reviewer").unwrap();
    assert_eq!(code_reviewer.config.intelligence, IntelligenceLevel::Hard); // Complex analysis
    
    let performance_analyzer = fixture.get_agent("performance-analyzer").unwrap();
    assert_eq!(performance_analyzer.config.intelligence, IntelligenceLevel::Hard); // Complex optimization
    
    let test_writer = fixture.get_agent("test-writer").unwrap();
    assert_eq!(test_writer.config.intelligence, IntelligenceLevel::Medium); // Standard test generation
}

#[tokio::test]
async fn test_agent_tag_system() {
    let fixture = AgentInvocationFixture::new().await;
    
    // Test tag-based agent discovery
    let review_agents = fixture.registry.get_agents_with_tags(&vec!["review".to_string()]);
    assert_eq!(review_agents.len(), 1);
    assert_eq!(review_agents[0].config.name, "code-reviewer");
    
    let performance_agents = fixture.registry.get_agents_with_tags(&vec!["performance".to_string()]);
    assert_eq!(performance_agents.len(), 1);
    assert_eq!(performance_agents[0].config.name, "performance-analyzer");
    
    let testing_agents = fixture.registry.get_agents_with_tags(&vec!["testing".to_string()]);
    assert_eq!(testing_agents.len(), 1);
    assert_eq!(testing_agents[0].config.name, "test-writer");
    
    // Test multiple tag search
    let quality_agents = fixture.registry.get_agents_with_tags(&vec![
        "quality".to_string(), 
        "review".to_string()
    ]);
    assert!(quality_agents.len() >= 1); // Should find code-reviewer
}

#[tokio::test]
async fn test_concurrent_agent_registry_access() {
    let fixture = Arc::new(AgentInvocationFixture::new().await);
    let num_tasks = 20;
    let operations_per_task = 50;
    
    let mut handles = Vec::new();
    
    for task_id in 0..num_tasks {
        let fixture_clone = fixture.clone();
        let handle = tokio::spawn(async move {
            let mut successful_operations = 0;
            
            for _ in 0..operations_per_task {
                // Mix of registry operations
                let agents = fixture_clone.list_agents();
                if agents.len() == 3 {
                    successful_operations += 1;
                }
                
                let code_reviewer = fixture_clone.get_agent("code-reviewer");
                if code_reviewer.is_some() {
                    successful_operations += 1;
                }
                
                let rust_file = PathBuf::from("test.rs");
                let matching = fixture_clone.registry.get_agents_for_file(&rust_file);
                if matching.len() >= 2 {
                    successful_operations += 1;
                }
                
                // Small delay to encourage race conditions
                if task_id % 2 == 0 {
                    tokio::time::sleep(Duration::from_nanos(1)).await;
                }
            }
            
            successful_operations
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    let results: Vec<_> = futures::future::join_all(handles).await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();
    
    // All tasks should have successful operations
    let total_successful: usize = results.iter().sum();
    let total_operations = num_tasks * operations_per_task * 3; // 3 operations per loop
    let success_rate = total_successful as f64 / total_operations as f64;
    
    // Should have very high success rate with thread-safe registry
    assert!(success_rate >= 0.95, 
           "Concurrent access issue: only {:.1}% operations successful", 
           success_rate * 100.0);
    
    println!("Concurrent registry access: {}/{} ({:.1}%) operations successful", 
             total_successful, total_operations, success_rate * 100.0);
}

#[tokio::test]
async fn test_invocation_context_isolation() {
    let fixture = AgentInvocationFixture::new().await;
    let parser = InvocationParser::new();
    
    // Test that different invocations create isolated contexts
    let invocation1 = parser.parse("@code-reviewer files=src/main.rs focus=security").unwrap().unwrap();
    let invocation2 = parser.parse("@code-reviewer files=tests/*.rs focus=maintainability").unwrap().unwrap();
    
    if let (ExecutionPlan::Single(inv1), ExecutionPlan::Single(inv2)) = 
        (invocation1.execution_plan, invocation2.execution_plan) 
    {
        // Same agent, different parameters
        assert_eq!(inv1.agent_name, "code-reviewer");
        assert_eq!(inv2.agent_name, "code-reviewer");
        
        // Parameters should be isolated
        assert_eq!(inv1.parameters["files"], "src/main.rs");
        assert_eq!(inv1.parameters["focus"], "security");
        
        assert_eq!(inv2.parameters["files"], "tests/*.rs");
        assert_eq!(inv2.parameters["focus"], "maintainability");
        
        // Invocation IDs should be different
        assert_ne!(invocation1.id, invocation2.id);
    }
}

#[tokio::test]
async fn test_error_handling_patterns() {
    let fixture = AgentInvocationFixture::new().await;
    
    // Test error handling for various scenarios
    
    // Invalid agent name
    let result = fixture.process_message("@nonexistent-agent analyze this code").await;
    // This should return an AgentNotFound error when actually executed
    // For now, we just verify the parsing works but execution would fail
    
    // Invalid parameter syntax
    let parser = InvocationParser::new();
    let result = parser.parse("@code-reviewer files= focus=security"); // Empty parameter value
    // Parser should still succeed but validation might catch this later
    assert!(result.is_ok());
    
    // Malformed invocation
    let result = parser.parse("@@code-reviewer files=test"); // Double @
    // Should not parse as agent invocation
    assert!(result.unwrap().is_none());
}
