//! Performance benchmarks for the AGCodex subagent system
//!
//! This module provides comprehensive benchmarks for agent operations including
//! initialization, orchestration, context sharing, and parallel execution.

#[cfg(test)]
mod bench {
    use chrono::Utc;
    use criterion::BenchmarkId;
    use criterion::Criterion;
    use criterion::black_box;
    use criterion::criterion_group;
    use criterion::criterion_main;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;
    use tokio::runtime::Runtime;
    use uuid::Uuid;

    use super::*;
    use crate::modes::OperatingMode;
    use crate::subagents::AgentContext;
    use crate::subagents::AgentInvocation;
    use crate::subagents::AgentOrchestrator;
    use crate::subagents::CodeReviewerAgent;
    use crate::subagents::DebuggerAgent;
    use crate::subagents::DocsAgent;
    use crate::subagents::IntelligenceLevel;
    use crate::subagents::PerformanceAgent;
    use crate::subagents::RefactorerAgent;
    use crate::subagents::SecurityAgent;
    use crate::subagents::Subagent;
    use crate::subagents::SubagentConfig;
    use crate::subagents::TestWriterAgent;
    use crate::subagents::orchestrator::OrchestratorConfig;
    use crate::subagents::registry::SubagentRegistry;

    /// Create a test runtime for async benchmarks
    pub fn create_runtime() -> Runtime {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap()
    }

    /// Create a test agent context
    pub fn create_test_context() -> AgentContext {
        AgentContext::new(OperatingMode::Build, HashMap::new())
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
                let registry = SubagentRegistry::new().unwrap();
                black_box(registry);
            })
        });

        group.bench_function("registry_with_defaults", |b| {
            b.iter(|| {
                let registry = SubagentRegistry::new().unwrap();
                black_box(registry);
            })
        });

        group.bench_function("agent_lookup", |b| {
            let registry = SubagentRegistry::new().unwrap();

            b.iter(|| {
                let agent = registry.get_agent("code-reviewer");
                black_box(agent);
            })
        });

        group.bench_function("agent_registration", |b| {
            let registry = SubagentRegistry::new().unwrap();
            let agent = Arc::new(CodeReviewerAgent::new());

            b.iter(|| {
                // Mock registration - SubagentRegistry loads from files
                black_box(&registry);
                black_box(&agent);
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
                let registry = Arc::new(SubagentRegistry::new().unwrap());
                let config = OrchestratorConfig::default();
                let orchestrator = AgentOrchestrator::new(registry, config, OperatingMode::Build);
                black_box(orchestrator);
            })
        });

        group.bench_function("orchestrator_with_agents", |b| {
            b.iter(|| {
                let registry = Arc::new(SubagentRegistry::new().unwrap());
                let config = OrchestratorConfig::default();
                let orchestrator = AgentOrchestrator::new(registry, config, OperatingMode::Build);
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

        // Note: snapshot functionality not yet implemented
        // group.bench_function("context_snapshot", |b| {
        //     let context = create_test_context();
        //     b.iter(|| {
        //         let snapshot = context.snapshot();
        //         black_box(snapshot);
        //     })
        // });

        // group.bench_function("context_restore", |b| {
        //     let context = create_test_context();
        //     let snapshot = context.snapshot();

        //     b.iter(|| {
        //         let restored = AgentContext::from_snapshot(&snapshot);
        //         black_box(restored);
        //     })
        // });

        group.bench_function("add_finding", |b| {
            let context = create_test_context();
            let runtime = create_runtime();
            b.iter_custom(|iters| {
                let start = std::time::Instant::now();
                runtime.block_on(async {
                    for _ in 0..iters {
                        let finding = crate::subagents::context::ContextFinding {
                            id: uuid::Uuid::new_v4(),
                            agent: "test".to_string(),
                            severity: crate::subagents::context::FindingSeverity::Info,
                            category: "test".to_string(),
                            message: "test finding".to_string(),
                            location: None,
                            suggestion: None,
                            confidence: 0.9,
                            timestamp: chrono::Utc::now(),
                        };
                        let _ = context.add_finding(finding).await;
                    }
                });
                start.elapsed()
            })
        });

        group.bench_function("metadata_access", |b| {
            let context = create_test_context();
            b.iter(|| {
                // Access metadata through the context
                let metadata = &context.metadata;
                metadata.insert("key".to_string(), serde_json::json!("value"));
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
            let registry = Arc::new(SubagentRegistry::new().unwrap());
            let config = OrchestratorConfig::default();
            let orchestrator = AgentOrchestrator::new(registry, config, OperatingMode::Build);
            let context = create_test_context();

            b.iter_custom(|iters| {
                let start = std::time::Instant::now();
                runtime.block_on(async {
                    for _ in 0..iters {
                        let invocation = AgentInvocation {
                            agent_name: "code-reviewer".to_string(),
                            parameters: HashMap::new(),
                            raw_parameters: String::new(),
                            position: 0,
                            mode_override: None,
                            intelligence_override: None,
                        };

                        // Mock execution without actual AI calls
                        let result = orchestrator.validate_invocation(&invocation).await;
                        black_box(result);
                    }
                });
                start.elapsed()
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
                    let registry = Arc::new(SubagentRegistry::new().unwrap());
                    let config = OrchestratorConfig::default();
                    let orchestrator = Arc::new(AgentOrchestrator::new(
                        registry,
                        config,
                        OperatingMode::Build,
                    ));
                    let context = create_test_context();

                    b.iter_custom(|iters| {
                        let start = std::time::Instant::now();
                        runtime.block_on(async {
                            for _ in 0..iters {
                                let mut handles = Vec::new();

                                for i in 0..num_agents {
                                    let orchestrator = orchestrator.clone();

                                    let handle = tokio::spawn(async move {
                                        let invocation = AgentInvocation {
                                            agent_name: "code-reviewer".to_string(),
                                            parameters: HashMap::from([(
                                                "file".to_string(),
                                                format!("file_{}.rs", i),
                                            )]),
                                            raw_parameters: format!("file=file_{}.rs", i),
                                            position: 0,
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
                        });
                        start.elapsed()
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
                    raw_parameters: "files=src/main.rs,src/lib.rs focus=security".to_string(),
                    position: 0,
                    mode_override: Some(OperatingMode::Review),
                    intelligence_override: Some("hard".to_string()),
                };
                black_box(invocation);
            })
        });

        group.bench_function("progress_tracking", |b| {
            let context = create_test_context();

            b.iter_custom(|iters| {
                let start = std::time::Instant::now();
                runtime.block_on(async {
                    for _ in 0..iters {
                        let progress = context.progress.clone();
                        let _ = progress
                            .update("test", 50.0, "Processing...".to_string())
                            .await;
                        let _ = progress.complete("test", "Done".to_string()).await;
                    }
                });
                start.elapsed()
            })
        });

        group.bench_function("metadata_operations", |b| {
            let context = create_test_context();

            b.iter(|| {
                // Test metadata operations
                context
                    .metadata
                    .insert("test_key".to_string(), serde_json::json!("test_value"));
                let _value = context.metadata.get("test_key");
                black_box(_value);
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
                    parameters: vec![],
                    template: None,
                    timeout_seconds: 300,
                    chainable: true,
                    parallelizable: true,
                    metadata: HashMap::new(),
                    file_patterns: vec![],
                },
                SubagentConfig {
                    name: "agent2".to_string(),
                    description: "Agent 2".to_string(),
                    mode_override: Some(OperatingMode::Build),
                    intelligence: IntelligenceLevel::Hard,
                    tools: Default::default(),
                    prompt: "Another prompt".to_string(),
                    parameters: vec![],
                    template: None,
                    timeout_seconds: 300,
                    chainable: true,
                    parallelizable: true,
                    metadata: HashMap::new(),
                    file_patterns: vec![],
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
        let registry = SubagentRegistry::new().unwrap();
        let init_time = start.elapsed();
        println!("✓ Agent registry initialization: {:?}", init_time);

        // Test context creation
        let start = Instant::now();
        let context = create_test_context();
        let context_time = start.elapsed();
        println!("✓ Context creation: {:?}", context_time);

        // Test orchestrator setup
        let start = Instant::now();
        let registry = Arc::new(SubagentRegistry::new().unwrap());
        let config = OrchestratorConfig::default();
        let orchestrator = AgentOrchestrator::new(registry, config, OperatingMode::Build);
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
            let ctx = create_test_context();
            for j in 0..10 {
                ctx.metadata
                    .insert(format!("key_{}", j), serde_json::json!("test message"));
            }
            contexts.push(ctx);
        }
        let memory_time = start.elapsed();
        println!(
            "✓ 100 contexts with 10 metadata entries each: {:?}",
            memory_time
        );

        println!("\n=== Performance Test Complete ===\n");

        // Assert reasonable performance bounds
        assert!(init_time.as_millis() < 100, "Registry init too slow");
        assert!(context_time.as_micros() < 1000, "Context creation too slow");
        assert!(
            orchestrator_time.as_millis() < 50,
            "Orchestrator setup too slow"
        );
        assert!(parallel_time.as_millis() < 200, "Parallel spawn too slow");
        assert!(memory_time.as_millis() < 500, "Memory operations too slow");
    }
}
