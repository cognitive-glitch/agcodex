//! Example usage of the AgentOrchestrator
//!
//! This module demonstrates how to use the orchestrator for various
//! agent execution patterns.

#[cfg(test)]
mod examples {
    use crate::modes::OperatingMode;
    use crate::subagents::{
        AgentChain, AgentInvocation, AgentOrchestrator, ExecutionPlan, ExecutionStep,
        InvocationRequest, OrchestratorConfig, SharedContext, SubagentConfig, SubagentRegistry,
        SubagentStatus,
    };
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;
    use uuid::Uuid;
    
    /// Example: Single agent execution
    #[tokio::test]
    async fn example_single_agent() {
        // Create registry and register agents
        let registry = Arc::new(SubagentRegistry::new());
        registry.register(create_test_agent("code-reviewer")).await.unwrap();
        
        // Create orchestrator
        let orchestrator = AgentOrchestrator::new(
            registry,
            OrchestratorConfig::default(),
            OperatingMode::Review,
        );
        
        // Create invocation request
        let request = InvocationRequest {
            id: Uuid::new_v4(),
            original_input: "@code-reviewer check src/main.rs".to_string(),
            execution_plan: ExecutionPlan::Single(AgentInvocation {
                agent_name: "code-reviewer".to_string(),
                parameters: HashMap::from([
                    ("file".to_string(), "src/main.rs".to_string()),
                ]),
                raw_parameters: "check src/main.rs".to_string(),
                position: 0,
            }),
            context: "Check the main file for issues".to_string(),
        };
        
        // Execute
        let result = orchestrator.execute_plan(request).await.unwrap();
        
        assert!(result.success);
        assert_eq!(result.executions.len(), 1);
        println!("Single agent completed in {:?}", result.total_duration);
    }
    
    /// Example: Sequential chain execution with output passing
    #[tokio::test]
    async fn example_sequential_chain() {
        let registry = Arc::new(SubagentRegistry::new());
        registry.register(create_test_agent("refactorer")).await.unwrap();
        registry.register(create_test_agent("test-writer")).await.unwrap();
        registry.register(create_test_agent("docs")).await.unwrap();
        
        let orchestrator = AgentOrchestrator::new(
            registry,
            OrchestratorConfig::default(),
            OperatingMode::Build,
        );
        
        let request = InvocationRequest {
            id: Uuid::new_v4(),
            original_input: "@refactorer → @test-writer → @docs".to_string(),
            execution_plan: ExecutionPlan::Sequential(AgentChain {
                agents: vec![
                    create_invocation("refactorer", "improve code structure"),
                    create_invocation("test-writer", "add missing tests"),
                    create_invocation("docs", "update documentation"),
                ],
                pass_output: true, // Pass output from one agent to the next
            }),
            context: "Improve the codebase".to_string(),
        };
        
        let result = orchestrator.execute_plan(request).await.unwrap();
        
        assert_eq!(result.executions.len(), 3);
        
        // Check that outputs were passed through the chain
        let outputs = result.context.all_outputs().await;
        assert_eq!(outputs.len(), 3);
        
        println!("Sequential chain completed with {} agents", result.executions.len());
    }
    
    /// Example: Parallel execution for independent tasks
    #[tokio::test]
    async fn example_parallel_execution() {
        let registry = Arc::new(SubagentRegistry::new());
        registry.register(create_test_agent("performance")).await.unwrap();
        registry.register(create_test_agent("security")).await.unwrap();
        registry.register(create_test_agent("code-reviewer")).await.unwrap();
        
        let orchestrator = AgentOrchestrator::new(
            registry,
            OrchestratorConfig {
                max_concurrency: 3, // Allow 3 agents to run simultaneously
                ..Default::default()
            },
            OperatingMode::Review,
        );
        
        let request = InvocationRequest {
            id: Uuid::new_v4(),
            original_input: "@performance + @security + @code-reviewer".to_string(),
            execution_plan: ExecutionPlan::Parallel(vec![
                create_invocation("performance", "analyze performance"),
                create_invocation("security", "scan for vulnerabilities"),
                create_invocation("code-reviewer", "review code quality"),
            ]),
            context: "Comprehensive code analysis".to_string(),
        };
        
        let start = std::time::Instant::now();
        let result = orchestrator.execute_plan(request).await.unwrap();
        let elapsed = start.elapsed();
        
        assert_eq!(result.executions.len(), 3);
        println!(
            "Parallel execution of {} agents completed in {:?}",
            result.executions.len(),
            elapsed
        );
        
        // Parallel execution should be faster than sequential
        // (in real scenarios with actual work)
    }
    
    /// Example: Mixed execution pattern (combine sequential and parallel)
    #[tokio::test]
    async fn example_mixed_execution() {
        let registry = Arc::new(SubagentRegistry::new());
        registry.register(create_test_agent("analyzer")).await.unwrap();
        registry.register(create_test_agent("refactorer")).await.unwrap();
        registry.register(create_test_agent("optimizer")).await.unwrap();
        registry.register(create_test_agent("test-writer")).await.unwrap();
        registry.register(create_test_agent("docs")).await.unwrap();
        
        let orchestrator = AgentOrchestrator::new(
            registry,
            OrchestratorConfig::default(),
            OperatingMode::Build,
        );
        
        // Pattern: analyzer → (refactorer + optimizer) → (test-writer + docs)
        let request = InvocationRequest {
            id: Uuid::new_v4(),
            original_input: "@analyzer → @refactorer + @optimizer → @test-writer + @docs".to_string(),
            execution_plan: ExecutionPlan::Mixed(vec![
                // Step 1: Analyze code
                ExecutionStep::Single(create_invocation("analyzer", "analyze codebase")),
                ExecutionStep::Barrier, // Wait for analysis to complete
                
                // Step 2: Refactor and optimize in parallel
                ExecutionStep::Parallel(vec![
                    create_invocation("refactorer", "refactor based on analysis"),
                    create_invocation("optimizer", "optimize performance"),
                ]),
                ExecutionStep::Barrier, // Wait for both to complete
                
                // Step 3: Update tests and docs in parallel
                ExecutionStep::Parallel(vec![
                    create_invocation("test-writer", "update tests"),
                    create_invocation("docs", "update documentation"),
                ]),
            ]),
            context: "Complex workflow with dependencies".to_string(),
        };
        
        let result = orchestrator.execute_plan(request).await.unwrap();
        
        assert_eq!(result.executions.len(), 5);
        println!("Mixed execution pattern completed with {} agents", result.executions.len());
    }
    
    /// Example: Using shared context for data passing
    #[tokio::test]
    async fn example_shared_context() {
        let registry = Arc::new(SubagentRegistry::new());
        registry.register(create_test_agent("scanner")).await.unwrap();
        registry.register(create_test_agent("fixer")).await.unwrap();
        
        let orchestrator = AgentOrchestrator::new(
            registry,
            OrchestratorConfig::default(),
            OperatingMode::Build,
        );
        
        // Create shared context with initial data
        let context = SharedContext::new();
        context.set("target_files".to_string(), serde_json::json!([
            "src/main.rs",
            "src/lib.rs"
        ])).await;
        
        // First agent: scan for issues
        let scan_result = orchestrator.execute_single(
            create_invocation("scanner", "scan for issues"),
            &context,
        ).await.unwrap();
        
        // Store scan results in context
        context.set("issues_found".to_string(), serde_json::json!({
            "count": 5,
            "severity": "medium"
        })).await;
        
        // Second agent: fix issues based on scan results
        let fix_result = orchestrator.execute_single(
            create_invocation("fixer", "fix identified issues"),
            &context,
        ).await.unwrap();
        
        // Check shared data
        let issues = context.get("issues_found").await.unwrap();
        println!("Shared context: {:?}", issues);
        
        assert_eq!(scan_result.status, SubagentStatus::Completed);
        assert_eq!(fix_result.status, SubagentStatus::Completed);
    }
    
    /// Example: Error handling and retries
    #[tokio::test]
    async fn example_error_handling() {
        let registry = Arc::new(SubagentRegistry::new());
        registry.register(create_test_agent("unreliable-agent")).await.unwrap();
        
        let orchestrator = AgentOrchestrator::new(
            registry,
            OrchestratorConfig {
                enable_retries: true,
                max_retries: 3,
                retry_backoff: Duration::from_millis(100),
                enable_circuit_breaker: true,
                circuit_breaker_threshold: 5,
                ..Default::default()
            },
            OperatingMode::Build,
        );
        
        let request = InvocationRequest {
            id: Uuid::new_v4(),
            original_input: "@unreliable-agent process data".to_string(),
            execution_plan: ExecutionPlan::Single(
                create_invocation("unreliable-agent", "process data")
            ),
            context: "Test error handling".to_string(),
        };
        
        // Execute with automatic retries
        let result = orchestrator.execute_plan(request).await;
        
        match result {
            Ok(res) => {
                println!("Execution succeeded after retries");
                assert!(res.success || res.partial_success);
            }
            Err(e) => {
                println!("Execution failed after retries: {}", e);
                // Circuit breaker should be open after repeated failures
            }
        }
    }
    
    /// Example: Progress tracking
    #[tokio::test]
    async fn example_progress_tracking() {
        let registry = Arc::new(SubagentRegistry::new());
        registry.register(create_test_agent("long-running")).await.unwrap();
        
        let orchestrator = AgentOrchestrator::new(
            registry,
            OrchestratorConfig::default(),
            OperatingMode::Build,
        );
        
        // Spawn a task to track progress
        let progress_task = tokio::spawn(async move {
            let mut rx = orchestrator.progress_receiver().await;
            
            while let Some(update) = rx.recv().await {
                println!(
                    "[{}] {}: {:?} - {:?}",
                    update.agent_name,
                    update.progress_percentage.unwrap_or(0),
                    update.status,
                    update.message
                );
            }
        });
        
        let request = InvocationRequest {
            id: Uuid::new_v4(),
            original_input: "@long-running heavy computation".to_string(),
            execution_plan: ExecutionPlan::Single(
                create_invocation("long-running", "heavy computation")
            ),
            context: "Track progress".to_string(),
        };
        
        let result = orchestrator.execute_plan(request).await.unwrap();
        assert!(result.success);
        
        // Clean up progress task
        progress_task.abort();
    }
    
    /// Example: Cancellation
    #[tokio::test]
    async fn example_cancellation() {
        let registry = Arc::new(SubagentRegistry::new());
        registry.register(create_test_agent("slow-agent")).await.unwrap();
        
        let orchestrator = Arc::new(AgentOrchestrator::new(
            registry,
            OrchestratorConfig {
                agent_timeout: Duration::from_secs(10),
                ..Default::default()
            },
            OperatingMode::Build,
        ));
        
        let orchestrator_clone = orchestrator.clone();
        
        // Start execution in background
        let execution_task = tokio::spawn(async move {
            let request = InvocationRequest {
                id: Uuid::new_v4(),
                original_input: "@slow-agent process large dataset".to_string(),
                execution_plan: ExecutionPlan::Single(
                    create_invocation("slow-agent", "process large dataset")
                ),
                context: "Test cancellation".to_string(),
            };
            
            orchestrator_clone.execute_plan(request).await
        });
        
        // Cancel after a short delay
        tokio::time::sleep(Duration::from_millis(50)).await;
        orchestrator.cancel();
        
        // Execution should fail due to cancellation
        let result = execution_task.await.unwrap();
        assert!(result.is_err());
        
        // Reset for future use
        orchestrator.reset_cancellation();
    }
    
    /// Example: Conditional execution
    #[tokio::test]
    async fn example_conditional_execution() {
        let registry = Arc::new(SubagentRegistry::new());
        registry.register(create_test_agent("validator")).await.unwrap();
        registry.register(create_test_agent("processor")).await.unwrap();
        
        let orchestrator = AgentOrchestrator::new(
            registry,
            OrchestratorConfig::default(),
            OperatingMode::Build,
        );
        
        let context = SharedContext::new();
        
        // First, validate
        let validation = orchestrator.execute_single(
            create_invocation("validator", "validate input"),
            &context,
        ).await.unwrap();
        
        // Store validation result
        context.set("is_valid".to_string(), serde_json::json!(true)).await;
        
        // Conditionally execute processor
        let processor_result = orchestrator.execute_conditional(
            create_invocation("processor", "process if valid"),
            &context,
            |ctx| Box::pin(async move {
                ctx.get("is_valid").await
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            }),
        ).await.unwrap();
        
        assert!(processor_result.is_some());
        println!("Conditional execution: processor ran because validation passed");
    }
    
    // Helper functions
    
    fn create_test_agent(name: &str) -> SubagentConfig {
        SubagentConfig {
            name: name.to_string(),
            description: format!("Test agent: {}", name),
            mode_override: None,
            tools: vec![],
            intelligence: crate::subagents::config::IntelligenceLevel::Medium,
            prompt: format!("You are the {} agent", name),
            timeout_seconds: Some(30),
            max_retries: Some(2),
        }
    }
    
    fn create_invocation(name: &str, params: &str) -> AgentInvocation {
        AgentInvocation {
            agent_name: name.to_string(),
            parameters: HashMap::from([
                ("task".to_string(), params.to_string()),
            ]),
            raw_parameters: params.to_string(),
            position: 0,
        }
    }
}