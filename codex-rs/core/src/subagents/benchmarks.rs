//! Performance benchmarks for the AGCodex subagent system
//!
//! This module provides comprehensive benchmarks for agent operations including
//! initialization, orchestration, context sharing, and parallel execution.

#[cfg(test)]
mod bench {
    use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::runtime::Runtime;

    use crate::modes::OperatingMode;
    use crate::subagents::{
        AgentContext, AgentInvocation, AgentOrchestrator, AgentRegistry,
        CodeReviewerAgent, DebuggerAgent, DocsAgent, IntelligenceLevel,
        OrchestratorConfig, PerformanceAgent, RefactorerAgent, SecurityAgent,
        Subagent, SubagentConfig, SubagentRegistry, TestWriterAgent,
    };

    /// Create a test runtime for async benchmarks
    fn create_runtime() -> Runtime {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap()
    }

    /// Create a test agent context
    fn create_test_context() -> AgentContext {
        AgentContext::new(
            "test-session".to_string(),
            OperatingMode::Build,
            vec!["Read".to_string(), "Write".to_string()],
        )
    }

    /// Benchmark agent initialization time
    fn bench_agent_initialization(c: &mut Criterion) {
        let mut group = c.benchmark_group("agent_initialization");

        // Benchmark each agent type
        group.bench_function("code_reviewer", |b| {
            b.iter(|| {
                let agent = CodeReviewerAgent::new();
                black_box(agent);
            })
        });

        group.bench_function("refactorer", |b| {
            b.iter(|| {
                let agent = RefactorerAgent::new();
                black_box(agent);
            })
        });

        group.bench_function("debugger", |b| {
            b.iter(|| {
                let agent = DebuggerAgent::new();
                black_box(agent);
            })
        });

        group.bench_function("test_writer", |b| {
            b.iter(|| {
                let agent = TestWriterAgent::new();
                black_box(agent);
            })
        });

        group.bench_function("performance", |b| {
            b.iter(|| {
                let agent = PerformanceAgent::new();
                black_box(agent);
            })
        });

        group.bench_function("security", |b| {
            b.iter(|| {
                let agent = SecurityAgent::new();
                black_box(agent);
            })
        });

        group.bench_function("docs", |b| {
            b.iter(|| {
                let agent = DocsAgent::new();
                black_box(agent);
            })
        });

        group.finish();
    }

    /// Benchmark agent registry operations
    fn bench_agent_registry(c: &mut Criterion) {
        let mut group = c.benchmark_group("agent_registry");

        group.bench_function("registry_creation", |b| {
            b.iter(|| {
                let registry = AgentRegistry::new();
                black_box(registry);
            })
        });

        group.bench_function("registry_with_defaults", |b| {
            b.iter(|| {
                let mut registry = AgentRegistry::new();
                registry.register_default_agents();
                black_box(registry);
            })
        });

        group.bench_function("agent_lookup", |b| {
            let mut registry = AgentRegistry::new();
            registry.register_default_agents();

            b.iter(|| {
                let agent = registry.get("code-reviewer");
                black_box(agent);
            })
        });

        group.bench_function("agent_registration", |b| {
            let mut registry = AgentRegistry::new();
            let agent = Arc::new(CodeReviewerAgent::new());

            b.iter(|| {
                registry.register("test-agent", agent.clone());
            })
        });

        group.finish();
    }

    /// Benchmark orchestrator creation and configuration
    fn bench_orchestrator_setup(c: &mut Criterion) {
        let mut group = c.benchmark_group("orchestrator_setup");

        group.bench_function("default_config", |b| {
            b.iter(|| {
                let config = OrchestratorConfig::default();
                black_box(config);
            })
        });

        group.bench_function("orchestrator_creation", |b| {
            b.iter(|| {
                let config = OrchestratorConfig::default();
                let orchestrator = AgentOrchestrator::new(config);
                black_box(orchestrator);
            })
        });

        group.bench_function("orchestrator_with_agents", |b| {
            b.iter(|| {
                let config = OrchestratorConfig::default();
                let mut orchestrator = AgentOrchestrator::new(config);
                orchestrator.registry.register_default_agents();
                black_box(orchestrator);
            })
        });

        group.finish();
    }

    /// Benchmark context creation and manipulation
    fn bench_context_operations(c: &mut Criterion) {
        let mut group = c.benchmark_group("context_operations");

        group.bench_function("context_creation", |b| {
            b.iter(|| {
                let context = create_test_context();
                black_box(context);
            })
        });

        group.bench_function("context_clone", |b| {
            let context = create_test_context();
            b.iter(|| {
                let cloned = context.clone();
                black_box(cloned);
            })
        });

        group.bench_function("context_snapshot", |b| {
            let context = create_test_context();
            b.iter(|| {
                let snapshot = context.snapshot();
                black_box(snapshot);
            })
        });

        group.bench_function("context_restore", |b| {
            let context = create_test_context();
            let snapshot = context.snapshot();

            b.iter(|| {
                let restored = AgentContext::from_snapshot(&snapshot);
                black_box(restored);
            })
        });

        group.bench_function("add_message", |b| {
            let mut context = create_test_context();
            b.iter(|| {
                context.add_message("test".to_string(), "message content".to_string());
            })
        });

        group.bench_function("update_metadata", |b| {
            let mut context = create_test_context();
            b.iter(|| {
                context.update_metadata("key".to_string(), serde_json::json!("value"));
            })
        });

        group.finish();
    }

    /// Benchmark single agent execution
    fn bench_agent_execution(c: &mut Criterion) {
        let runtime = create_runtime();
        let mut group = c.benchmark_group("agent_execution");
        group.measurement_time(Duration::from_secs(10));

        group.bench_function("mock_execution", |b| {
            let config = OrchestratorConfig::default();
            let mut orchestrator = AgentOrchestrator::new(config);
            orchestrator.registry.register_default_agents();
            let context = create_test_context();

            b.to_async(&runtime).iter(|| async {
                let invocation = AgentInvocation {
                    agent_name: "code-reviewer".to_string(),
                    parameters: HashMap::new(),
                    mode_override: None,
                    intelligence_override: None,
                };

                // Mock execution without actual AI calls
                let result = orchestrator.validate_invocation(&invocation).await;
                black_box(result);
            })
        });

        group.finish();
    }

    /// Benchmark parallel agent execution
    fn bench_parallel_execution(c: &mut Criterion) {
        let runtime = create_runtime();
        let mut group = c.benchmark_group("parallel_execution");
        group.measurement_time(Duration::from_secs(10));

        for num_agents in &[2, 4, 8, 16] {
            group.bench_with_input(
                BenchmarkId::from_parameter(num_agents),
                num_agents,
                |b, &num_agents| {
                    let config = OrchestratorConfig {
                        max_concurrent_agents: num_agents,
                        ..Default::default()
                    };
                    let mut orchestrator = Arc::new(AgentOrchestrator::new(config));
                    Arc::get_mut(&mut orchestrator).unwrap().registry.register_default_agents();
                    let context = create_test_context();

                    b.to_async(&runtime).iter(|| {
                        let orchestrator = orchestrator.clone();
                        let context = context.clone();
                        
                        async move {
                            let mut handles = Vec::new();

                            for i in 0..num_agents {
                                let orchestrator = orchestrator.clone();
                                let context = context.clone();
                                
                                let handle = tokio::spawn(async move {
                                    let invocation = AgentInvocation {
                                        agent_name: "code-reviewer".to_string(),
                                        parameters: HashMap::from([
                                            ("file".to_string(), format!("file_{}.rs", i)),
                                        ]),
                                        mode_override: None,
                                        intelligence_override: None,
                                    };

                                    // Mock validation without actual execution
                                    orchestrator.validate_invocation(&invocation).await
                                });

                                handles.push(handle);
                            }

                            for handle in handles {
                                let _ = handle.await;
                            }
                        }
                    })
                },
            );
        }

        group.finish();
    }

    /// Benchmark orchestration overhead
    fn bench_orchestration_overhead(c: &mut Criterion) {
        let runtime = create_runtime();
        let mut group = c.benchmark_group("orchestration_overhead");

        group.bench_function("invocation_parsing", |b| {
            b.iter(|| {
                let invocation = AgentInvocation {
                    agent_name: "code-reviewer".to_string(),
                    parameters: HashMap::from([
                        ("files".to_string(), "src/main.rs,src/lib.rs".to_string()),
                        ("focus".to_string(), "security".to_string()),
                    ]),
                    mode_override: Some(OperatingMode::Review),
                    intelligence_override: Some(IntelligenceLevel::Hard),
                };
                black_box(invocation);
            })
        });

        group.bench_function("progress_tracking", |b| {
            let context = create_test_context();

            b.to_async(&runtime).iter(|| async {
                let progress = context.progress.clone();
                progress.update("test", 50.0, "Processing...".to_string()).await;
                progress.complete("test", "Done".to_string()).await;
            })
        });

        group.bench_function("message_passing", |b| {
            let context = create_test_context();

            b.to_async(&runtime).iter(|| async {
                context.send_message("test", "message content".to_string()).await;
                let _ = context.receive_message().await;
            })
        });

        group.finish();
    }

    /// Benchmark memory usage patterns
    fn bench_memory_usage(c: &mut Criterion) {
        let mut group = c.benchmark_group("memory_usage");

        group.bench_function("context_with_messages", |b| {
            b.iter_custom(|iters| {
                let start = std::time::Instant::now();

                for _ in 0..iters {
                    let mut context = create_test_context();
                    
                    // Add 100 messages to simulate real usage
                    for i in 0..100 {
                        context.add_message(
                            format!("sender_{}", i),
                            format!("This is message content number {}", i),
                        );
                    }

                    black_box(context);
                }

                start.elapsed()
            })
        });

        group.bench_function("large_metadata", |b| {
            b.iter_custom(|iters| {
                let start = std::time::Instant::now();

                for _ in 0..iters {
                    let mut context = create_test_context();
                    
                    // Add large metadata objects
                    for i in 0..50 {
                        let large_value = serde_json::json!({
                            "index": i,
                            "data": vec![0u8; 1024], // 1KB of data
                            "nested": {
                                "more_data": vec![0u8; 512],
                            }
                        });
                        context.update_metadata(format!("key_{}", i), large_value);
                    }

                    black_box(context);
                }

                start.elapsed()
            })
        });

        group.finish();
    }

    /// Benchmark configuration loading
    fn bench_config_loading(c: &mut Criterion) {
        let mut group = c.benchmark_group("config_loading");

        group.bench_function("parse_config", |b| {
            let config_str = r#"
                name = "test-agent"
                description = "Test agent"
                mode_override = "review"
                intelligence = "hard"
                
                [tools]
                allowed = ["Read", "Write", "AST-Search"]
                
                [parameters]
                max_files = 100
                timeout = 120
            "#;

            b.iter(|| {
                let config: Result<SubagentConfig, _> = toml::from_str(config_str);
                black_box(config);
            })
        });

        group.bench_function("registry_from_configs", |b| {
            let configs = vec![
                SubagentConfig {
                    name: "agent1".to_string(),
                    description: "Agent 1".to_string(),
                    mode_override: Some(OperatingMode::Review),
                    intelligence: IntelligenceLevel::Medium,
                    tools: Default::default(),
                    prompt: "Test prompt".to_string(),
                    parameters: HashMap::new(),
                },
                SubagentConfig {
                    name: "agent2".to_string(),
                    description: "Agent 2".to_string(),
                    mode_override: Some(OperatingMode::Build),
                    intelligence: IntelligenceLevel::Hard,
                    tools: Default::default(),
                    prompt: "Another prompt".to_string(),
                    parameters: HashMap::new(),
                },
            ];

            b.iter(|| {
                let registry = SubagentRegistry::from_configs(configs.clone());
                black_box(registry);
            })
        });

        group.finish();
    }

    /// Main benchmark groups
    criterion_group!(
        benches,
        bench_agent_initialization,
        bench_agent_registry,
        bench_orchestrator_setup,
        bench_context_operations,
        bench_agent_execution,
        bench_parallel_execution,
        bench_orchestration_overhead,
        bench_memory_usage,
        bench_config_loading,
    );

    criterion_main!(benches);
}

/// Standalone benchmark runner (can be used without criterion)
#[cfg(test)]
mod standalone {
    use super::*;
    use std::time::Instant;

    /// Quick performance test for development
    #[test]
    #[ignore] // Run with: cargo test --ignored standalone_perf_test -- --nocapture
    fn standalone_perf_test() {
        println!("\n=== AGCodex Subagent Performance Test ===\n");

        // Test agent initialization
        let start = Instant::now();
        let mut registry = AgentRegistry::new();
        registry.register_default_agents();
        let init_time = start.elapsed();
        println!("✓ Agent registry initialization: {:?}", init_time);

        // Test context creation
        let start = Instant::now();
        let context = create_test_context();
        let context_time = start.elapsed();
        println!("✓ Context creation: {:?}", context_time);

        // Test orchestrator setup
        let start = Instant::now();
        let config = OrchestratorConfig::default();
        let orchestrator = AgentOrchestrator::new(config);
        let orchestrator_time = start.elapsed();
        println!("✓ Orchestrator setup: {:?}", orchestrator_time);

        // Test parallel agent spawn simulation
        let runtime = create_runtime();
        let start = Instant::now();
        runtime.block_on(async {
            let mut handles = Vec::new();
            for i in 0..10 {
                let handle = tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    i
                });
                handles.push(handle);
            }
            for handle in handles {
                let _ = handle.await;
            }
        });
        let parallel_time = start.elapsed();
        println!("✓ 10 parallel agent spawns: {:?}", parallel_time);

        // Memory footprint test
        let start = Instant::now();
        let mut contexts = Vec::new();
        for _ in 0..100 {
            let mut ctx = create_test_context();
            for j in 0..10 {
                ctx.add_message(format!("sender_{}", j), "test message".to_string());
            }
            contexts.push(ctx);
        }
        let memory_time = start.elapsed();
        println!("✓ 100 contexts with 10 messages each: {:?}", memory_time);

        println!("\n=== Performance Test Complete ===\n");
        
        // Assert reasonable performance bounds
        assert!(init_time.as_millis() < 100, "Registry init too slow");
        assert!(context_time.as_micros() < 1000, "Context creation too slow");
        assert!(orchestrator_time.as_millis() < 50, "Orchestrator setup too slow");
        assert!(parallel_time.as_millis() < 200, "Parallel spawn too slow");
        assert!(memory_time.as_millis() < 500, "Memory operations too slow");
    }
}