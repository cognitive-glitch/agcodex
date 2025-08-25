//! Benchmarks for agent spawn overhead and orchestration performance.
//! Tests agent creation, context switching, and parallel execution.

use agcodex_core::modes::OperatingMode;
use agcodex_core::subagents::AgentInvocation;
use agcodex_core::subagents::ExecutionPlan;
use agcodex_core::subagents::ExecutionStep;
use agcodex_core::subagents::SubagentConfig;
use agcodex_core::subagents::SubagentContext;
use agcodex_core::subagents::SubagentRegistry;
use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use std::collections::HashMap;
use std::hint::black_box;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

/// Create a mock agent configuration
fn create_mock_agent_config(name: &str) -> SubagentConfig {
    SubagentConfig {
        name: name.to_string(),
        description: format!("Mock agent for {}", name),
        mode_override: None,
        intelligence: agcodex_core::subagents::IntelligenceLevel::Medium,
        tools: HashMap::new(),
        prompt: format!("You are a {} agent", name),
        parameters: vec![],
        template: None,
        timeout_seconds: 30,
        chainable: true,
        parallelizable: true,
        metadata: HashMap::new(),
        file_patterns: vec!["search".to_string(), "edit".to_string()],
        tags: vec![],
    }
}

/// Benchmark single agent spawn time
fn bench_single_agent_spawn(c: &mut Criterion) {
    let mut group = c.benchmark_group("agent_spawn");
    let rt = Runtime::new().unwrap();

    group.bench_function("create_agent", |b| {
        b.iter(|| {
            let _config = create_mock_agent_config("test_agent");
            let agent = SubagentContext::new(OperatingMode::Build, HashMap::new());
            black_box(agent)
        });
    });

    group.bench_function("spawn_with_context", |b| {
        let rt = Runtime::new().unwrap();
        b.iter(|| {
            rt.block_on(async {
                let _config = create_mock_agent_config("context_agent");
                let agent_context = SubagentContext::new(
                    OperatingMode::Build,
                    HashMap::from([
                        ("user_query".to_string(), "test query".to_string()),
                        ("mode".to_string(), "build".to_string()),
                    ]),
                );

                // Simulate spawning with channel setup
                let (tx, mut rx) = mpsc::channel::<String>(100);
                tokio::spawn(async move {
                    tx.send(format!("Agent {} ready", agent_context.execution_id))
                        .await
                        .ok();
                });

                // Wait for spawn confirmation
                let _ = rx.recv().await;
                black_box(agent_context)
            })
        });
    });

    group.bench_function("spawn_with_registry", |b| {
        let rt = Runtime::new().unwrap();
        b.iter(|| {
            rt.block_on(async {
                let registry = SubagentRegistry::new().unwrap();

                // Load agents from files (simulating registration)
                let _ = registry.load_all();

                // Try to get an agent config from registry
                let agent = registry.get_agent_config("agent_2");
                black_box(agent)
            })
        });
    });

    group.finish();
}

/// Benchmark parallel agent spawning
fn bench_parallel_agent_spawn(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_agent_spawn");
    let rt = Runtime::new().unwrap();

    let agent_counts = vec![2, 4, 8, 16];

    for count in agent_counts {
        group.bench_function(BenchmarkId::new("spawn_parallel", count), |b| {
            let rt = Runtime::new().unwrap();
            b.iter(|| {
                rt.block_on(async {
                    let mut handles = Vec::new();

                    for i in 0..count {
                        let handle = tokio::spawn(async move {
                            let _config = create_mock_agent_config(&format!("parallel_{}", i));

                            SubagentContext::new(OperatingMode::Build, HashMap::new())
                        });
                        handles.push(handle);
                    }

                    // Wait for all agents to spawn
                    let agents: Vec<_> = futures::future::join_all(handles).await;
                    black_box(agents)
                })
            });
        });
    }

    group.finish();
}

/// Benchmark agent orchestration overhead
fn bench_orchestration(c: &mut Criterion) {
    let mut group = c.benchmark_group("orchestration");
    let rt = Runtime::new().unwrap();

    // Benchmark orchestrator initialization
    group.bench_function("orchestrator_init", |b| {
        b.iter(|| {
            let config = agcodex_core::subagents::OrchestratorConfig {
                max_concurrency: 8,
                agent_timeout: Duration::from_secs(30),
                enable_retries: true,
                max_retries: 3,
                retry_backoff: Duration::from_secs(2),
                enable_circuit_breaker: true,
                circuit_breaker_threshold: 5,
                circuit_breaker_reset: Duration::from_secs(60),
                monitor_memory: true,
                memory_threshold_mb: 2048,
            };
            // Note: AgentOrchestrator::new requires registry parameter
            // For benchmarking purposes, we just measure config creation
            black_box(config)
        });
    });

    // Benchmark execution plan creation
    group.bench_function("create_execution_plan", |b| {
        b.iter(|| {
            let mut invocations = Vec::new();

            // Create sequential invocations
            for i in 0..5 {
                invocations.push(AgentInvocation {
                    agent_name: format!("agent_{}", i),
                    parameters: HashMap::new(),
                    raw_parameters: String::new(),
                    position: i,
                    intelligence_override: None,
                    mode_override: None,
                });
            }

            // Create parallel invocations
            for i in 5..10 {
                invocations.push(AgentInvocation {
                    agent_name: format!("parallel_agent_{}", i),
                    parameters: HashMap::new(),
                    raw_parameters: String::new(),
                    position: i,
                    intelligence_override: None,
                    mode_override: None,
                });
            }

            let plan = ExecutionPlan::Parallel(invocations);
            black_box(plan)
        });
    });

    // Benchmark message passing between agents
    group.bench_function("agent_communication", |b| {
        let rt = Runtime::new().unwrap();
        b.iter(|| {
            rt.block_on(async {
                let (tx1, mut rx1) = mpsc::channel::<String>(100);
                let (tx2, mut rx2) = mpsc::channel::<String>(100);

                // Agent 1 sends to Agent 2
                let handle1 = tokio::spawn(async move {
                    for i in 0..10 {
                        tx1.send(format!("Message {}", i)).await.ok();
                    }
                });

                // Agent 2 receives and responds
                let handle2 = tokio::spawn(async move {
                    let mut count = 0;
                    while let Some(msg) = rx1.recv().await {
                        tx2.send(format!("Received: {}", msg)).await.ok();
                        count += 1;
                        if count >= 10 {
                            break;
                        }
                    }
                });

                // Collector
                let handle3 = tokio::spawn(async move {
                    let mut results = Vec::new();
                    while let Some(msg) = rx2.recv().await {
                        results.push(msg);
                        if results.len() >= 10 {
                            break;
                        }
                    }
                    results
                });

                let (_, _, results) = tokio::join!(handle1, handle2, handle3);
                black_box(results)
            })
        });
    });

    group.finish();
}

/// Benchmark context switching between agents
fn bench_context_switching(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_switching");
    let rt = Runtime::new().unwrap();

    group.bench_function("context_save", |b| {
        let rt = Runtime::new().unwrap();
        b.iter(|| {
            rt.block_on(async {
                let context = SubagentContext::new(
                    OperatingMode::Build,
                    HashMap::from([
                        ("key1".to_string(), "value1".to_string()),
                        ("key2".to_string(), "value2".to_string()),
                        ("large_data".to_string(), "x".repeat(1000)),
                    ]),
                );

                // Simulate saving context
                let serialized = serde_json::to_string(&context.parameters).unwrap();
                black_box(serialized)
            })
        });
    });

    group.bench_function("context_restore", |b| {
        let serialized = serde_json::to_string(&HashMap::from([
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2".to_string()),
            ("large_data".to_string(), "x".repeat(1000)),
        ]))
        .unwrap();

        b.iter(|| {
            let restored: HashMap<String, String> = serde_json::from_str(&serialized).unwrap();
            let context = SubagentContext::new(OperatingMode::Build, restored);
            black_box(context)
        });
    });

    group.bench_function("context_merge", |b| {
        b.iter(|| {
            let mut base_context = HashMap::from([
                ("base1".to_string(), "value1".to_string()),
                ("base2".to_string(), "value2".to_string()),
            ]);

            let agent_context = HashMap::from([
                ("agent1".to_string(), "value3".to_string()),
                ("agent2".to_string(), "value4".to_string()),
                ("base1".to_string(), "override".to_string()), // Override
            ]);

            // Merge contexts
            for (k, v) in agent_context {
                base_context.insert(k, v);
            }

            black_box(base_context)
        });
    });

    group.finish();
}

/// Benchmark agent lifecycle management
fn bench_agent_lifecycle(c: &mut Criterion) {
    let mut group = c.benchmark_group("agent_lifecycle");
    let rt = Runtime::new().unwrap();

    group.bench_function("full_lifecycle", |b| {
        let rt = Runtime::new().unwrap();
        b.iter(|| {
            rt.block_on(async {
                // 1. Create agent
                let _config = create_mock_agent_config("lifecycle_agent");
                let agent = SubagentContext::new(OperatingMode::Build, HashMap::new());

                // 2. Initialize resources
                let (tx, mut rx) = mpsc::channel::<String>(10);
                let agent_id = agent.execution_id;

                // 3. Run agent task
                let handle = tokio::spawn(async move {
                    // Simulate work
                    tokio::time::sleep(Duration::from_micros(10)).await;
                    tx.send(format!("Agent {} completed", agent_id)).await.ok();
                });

                // 4. Wait for completion
                let result = rx.recv().await;
                handle.await.ok();

                // 5. Cleanup (simulated)
                drop(rx);

                black_box(result)
            })
        });
    });

    group.bench_function("agent_cancellation", |b| {
        let rt = Runtime::new().unwrap();
        b.iter(|| {
            rt.block_on(async {
                let handle = tokio::spawn(async {
                    // Long-running task
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    "Should not complete"
                });

                // Cancel after short delay
                tokio::time::sleep(Duration::from_micros(1)).await;
                handle.abort();

                let result = handle.await;
                black_box(result.is_err()) // Should be cancelled
            })
        });
    });

    group.finish();
}

/// Benchmark worst-case scenarios
fn bench_worst_case_agents(c: &mut Criterion) {
    let mut group = c.benchmark_group("worst_case_agents");
    let rt = Runtime::new().unwrap();

    // Many agents with dependencies
    group.bench_function("complex_dependencies", |b| {
        b.iter(|| {
            let mut mixed_steps = Vec::new();

            // Create a complex execution plan with mixed steps
            for i in 0..20 {
                let invocation = AgentInvocation {
                    agent_name: format!("agent_{}", i),
                    parameters: HashMap::new(),
                    raw_parameters: String::new(),
                    position: i,
                    intelligence_override: None,
                    mode_override: None,
                };

                mixed_steps.push(if i % 3 == 0 {
                    ExecutionStep::Single(invocation)
                } else {
                    ExecutionStep::Parallel(vec![invocation])
                });
            }

            let plan = ExecutionPlan::Mixed(mixed_steps);
            black_box(plan)
        });
    });

    // Resource contention
    group.bench_function("resource_contention", |b| {
        let rt = Runtime::new().unwrap();
        b.iter(|| {
            rt.block_on(async {
                let shared_resource = Arc::new(Mutex::new(0));
                let mut handles = Vec::new();

                // Spawn many agents competing for same resource
                for _i in 0..10 {
                    let resource = Arc::clone(&shared_resource);
                    let handle = tokio::spawn(async move {
                        for _ in 0..10 {
                            let mut val = resource.lock().await;
                            *val += 1;
                            // Simulate work while holding lock
                            tokio::time::sleep(Duration::from_nanos(100)).await;
                        }
                    });
                    handles.push(handle);
                }

                futures::future::join_all(handles).await;
                let final_value = *shared_resource.lock().await;
                black_box(final_value)
            })
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_single_agent_spawn,
    bench_parallel_agent_spawn,
    bench_orchestration,
    bench_context_switching,
    bench_agent_lifecycle,
    bench_worst_case_agents
);
criterion_main!(benches);
