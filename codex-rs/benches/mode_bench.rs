//! Benchmarks for operating mode switching latency.
//! Tests Plan/Build/Review mode transitions and enforcement overhead.

use agcodex_core::modes::OperatingMode;
use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use std::hint::black_box;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

/// Benchmark basic mode switching
fn bench_mode_switching(c: &mut Criterion) {
    let mut group = c.benchmark_group("mode_switching");

    // Benchmark single mode switch
    group.bench_function("single_switch", |b| {
        let manager = MockModeManager::new(OperatingMode::Build);
        b.iter(|| {
            manager.switch_mode(OperatingMode::Plan);
            let mode = manager.current_mode();
            black_box(mode)
        });
    });

    // Benchmark cycling through all modes
    group.bench_function("cycle_all_modes", |b| {
        let manager = MockModeManager::new(OperatingMode::Build);
        b.iter(|| {
            manager.switch_mode(OperatingMode::Plan);
            manager.switch_mode(OperatingMode::Build);
            manager.switch_mode(OperatingMode::Review);
            let mode = manager.current_mode();
            black_box(mode)
        });
    });

    // Benchmark mode switch with Shift+Tab simulation
    group.bench_function("shift_tab_cycle", |b| {
        let manager = MockModeManager::new(OperatingMode::Build);
        b.iter(|| {
            // Simulate Shift+Tab cycling
            let current = manager.current_mode();
            let next = match current {
                OperatingMode::Plan => OperatingMode::Build,
                OperatingMode::Build => OperatingMode::Review,
                OperatingMode::Review => OperatingMode::Plan,
            };
            manager.switch_mode(next);
            black_box(next)
        });
    });

    // Benchmark concurrent mode switches (thread safety)
    group.bench_function("concurrent_switches", |b| {
        let manager = Arc::new(MockModeManager::new(OperatingMode::Build));
        b.iter(|| {
            let mgr1 = Arc::clone(&manager);
            let mgr2 = Arc::clone(&manager);

            let handle1 = std::thread::spawn(move || {
                mgr1.switch_mode(OperatingMode::Plan);
            });

            let handle2 = std::thread::spawn(move || {
                mgr2.switch_mode(OperatingMode::Review);
            });

            handle1.join().unwrap();
            handle2.join().unwrap();

            let final_mode = manager.current_mode();
            black_box(final_mode)
        });
    });

    group.finish();
}

/// Benchmark mode validation and enforcement
fn bench_mode_enforcement(c: &mut Criterion) {
    let mut group = c.benchmark_group("mode_enforcement");

    // Plan mode restrictions
    group.bench_function("plan_mode_validation", |b| {
        let manager = MockModeManager::new(OperatingMode::Plan);
        b.iter(|| {
            // Validate various operations
            let can_read = manager.validate_operation("read", None);
            let can_write = manager.validate_operation("write", None);
            let can_execute = manager.validate_operation("execute", None);
            black_box((can_read, can_write, can_execute))
        });
    });

    // Build mode validation (should be fastest - everything allowed)
    group.bench_function("build_mode_validation", |b| {
        let manager = MockModeManager::new(OperatingMode::Build);
        b.iter(|| {
            let can_read = manager.validate_operation("read", None);
            let can_write = manager.validate_operation("write", None);
            let can_execute = manager.validate_operation("execute", None);
            black_box((can_read, can_write, can_execute))
        });
    });

    // Review mode with size restrictions
    group.bench_function("review_mode_validation", |b| {
        let manager = MockModeManager::new(OperatingMode::Review);
        b.iter(|| {
            // Test with different file sizes
            let small_file = manager.validate_operation("write", Some(1024)); // 1KB
            let medium_file = manager.validate_operation("write", Some(5120)); // 5KB
            let large_file = manager.validate_operation("write", Some(20480)); // 20KB (>10KB limit)
            black_box((small_file, medium_file, large_file))
        });
    });

    // Benchmark validation with path checking
    group.bench_function("path_validation", |b| {
        let manager = MockModeManager::new(OperatingMode::Plan);
        let test_paths = vec![
            PathBuf::from("/home/user/project/src/main.rs"),
            PathBuf::from("/tmp/test.txt"),
            PathBuf::from("/etc/passwd"), // System file
        ];

        b.iter(|| {
            let mut results = Vec::new();
            for path in &test_paths {
                let valid = manager.validate_path_access(path, "read");
                results.push(valid);
            }
            black_box(results)
        });
    });

    group.finish();
}

/// Benchmark mode state management
fn bench_mode_state(c: &mut Criterion) {
    let mut group = c.benchmark_group("mode_state");

    // Benchmark state persistence
    group.bench_function("state_save", |b| {
        let manager = MockModeManager::new(OperatingMode::Build);
        b.iter(|| {
            let state = manager.save_state();
            black_box(state)
        });
    });

    // Benchmark state restoration
    group.bench_function("state_restore", |b| {
        let manager = MockModeManager::new(OperatingMode::Build);
        let saved_state = manager.save_state();

        b.iter(|| {
            manager.switch_mode(OperatingMode::Plan);
            manager.restore_state(&saved_state);
            let mode = manager.current_mode();
            black_box(mode)
        });
    });

    // Benchmark history tracking
    group.bench_function("history_tracking", |b| {
        let manager = MockModeManager::new(OperatingMode::Build);
        b.iter(|| {
            // Add to history
            manager.switch_mode(OperatingMode::Plan);
            manager.switch_mode(OperatingMode::Review);
            manager.switch_mode(OperatingMode::Build);

            let history = manager.get_mode_history();
            black_box(history)
        });
    });

    group.finish();
}

/// Benchmark mode-specific feature access
fn bench_mode_features(c: &mut Criterion) {
    let mut group = c.benchmark_group("mode_features");

    // Benchmark tool availability checks
    group.bench_function("tool_availability", |b| {
        let manager = MockModeManager::new(OperatingMode::Plan);
        let tools = vec!["search", "edit", "execute", "delete", "create"];

        b.iter(|| {
            let mut available = Vec::new();
            for tool in &tools {
                let can_use = manager.is_tool_available(tool);
                available.push(can_use);
            }
            black_box(available)
        });
    });

    // Benchmark mode-specific limits
    group.bench_function("operation_limits", |b| {
        b.iter(|| {
            let plan_limits = ModeLimits::for_mode(OperatingMode::Plan);
            let build_limits = ModeLimits::for_mode(OperatingMode::Build);
            let review_limits = ModeLimits::for_mode(OperatingMode::Review);
            black_box((plan_limits, build_limits, review_limits))
        });
    });

    // Benchmark mode descriptions and visuals
    group.bench_function("mode_visuals", |b| {
        b.iter(|| {
            let plan_visuals = OperatingMode::Plan.visuals();
            let build_visuals = OperatingMode::Build.visuals();
            let review_visuals = OperatingMode::Review.visuals();
            black_box((plan_visuals, build_visuals, review_visuals))
        });
    });

    group.finish();
}

/// Benchmark mode switching with callbacks
fn bench_mode_callbacks(c: &mut Criterion) {
    let mut group = c.benchmark_group("mode_callbacks");

    // Benchmark with no callbacks (baseline)
    group.bench_function("no_callbacks", |b| {
        let manager = MockModeManager::new(OperatingMode::Build);
        b.iter(|| {
            manager.switch_mode(OperatingMode::Plan);
            black_box(manager.current_mode())
        });
    });

    // Benchmark with single callback
    group.bench_function("single_callback", |b| {
        let manager = MockModeManager::new(OperatingMode::Build);
        let counter = Arc::new(Mutex::new(0));

        manager.on_mode_change({
            let counter = Arc::clone(&counter);
            move |_old_mode, _new_mode| {
                let mut count = counter.lock().unwrap();
                *count += 1;
            }
        });

        b.iter(|| {
            manager.switch_mode(OperatingMode::Plan);
            manager.switch_mode(OperatingMode::Build);
            black_box(manager.current_mode())
        });
    });

    // Benchmark with multiple callbacks
    group.bench_function("multiple_callbacks", |b| {
        let manager = MockModeManager::new(OperatingMode::Build);

        // Add multiple callbacks
        for i in 0..5 {
            manager.on_mode_change(move |_old_mode, _new_mode| {
                // Simulate some work
                let _ = format!("Callback {} triggered", i);
            });
        }

        b.iter(|| {
            manager.switch_mode(OperatingMode::Review);
            manager.switch_mode(OperatingMode::Build);
            black_box(manager.current_mode())
        });
    });

    group.finish();
}

/// Benchmark worst-case scenarios
fn bench_worst_case_modes(c: &mut Criterion) {
    let mut group = c.benchmark_group("worst_case_modes");

    // Rapid mode switching
    group.bench_function("rapid_switching", |b| {
        let manager = MockModeManager::new(OperatingMode::Build);
        b.iter(|| {
            for _ in 0..100 {
                manager.switch_mode(OperatingMode::Plan);
                manager.switch_mode(OperatingMode::Build);
                manager.switch_mode(OperatingMode::Review);
            }
            black_box(manager.current_mode())
        });
    });

    // Complex validation chains
    group.bench_function("complex_validation", |b| {
        let manager = MockModeManager::new(OperatingMode::Review);
        b.iter(|| {
            let mut results = Vec::new();

            // Simulate complex validation scenario
            for size in [100, 1000, 5000, 10000, 15000] {
                for operation in ["read", "write", "execute", "delete"] {
                    let valid = manager.validate_operation(operation, Some(size));
                    results.push(valid);
                }
            }

            black_box(results)
        });
    });

    // Mode switching under contention
    group.bench_function("contention", |b| {
        let manager = Arc::new(MockModeManager::new(OperatingMode::Build));
        b.iter(|| {
            let mut handles = Vec::new();

            // Spawn multiple threads trying to switch modes
            for i in 0..10 {
                let mgr = Arc::clone(&manager);
                let handle = std::thread::spawn(move || {
                    let mode = match i % 3 {
                        0 => OperatingMode::Plan,
                        1 => OperatingMode::Build,
                        _ => OperatingMode::Review,
                    };
                    mgr.switch_mode(mode);
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.join().unwrap();
            }

            black_box(manager.current_mode())
        });
    });

    group.finish();
}

/// Mock types for benchmarking - separate from the real implementation
struct MockModeManager {
    current: Arc<Mutex<OperatingMode>>,
    history: Arc<Mutex<Vec<OperatingMode>>>,
    callbacks: Arc<Mutex<Vec<Box<dyn Fn(OperatingMode, OperatingMode) + Send + Sync>>>>,
}

/// Helper implementations for benchmarking
/// Note: These are mock implementations for benchmarking purposes
impl MockModeManager {
    fn new(initial_mode: OperatingMode) -> Self {
        Self {
            current: Arc::new(Mutex::new(initial_mode)),
            history: Arc::new(Mutex::new(Vec::new())),
            callbacks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn current_mode(&self) -> OperatingMode {
        *self.current.lock().unwrap()
    }

    fn switch_mode(&self, new_mode: OperatingMode) {
        let old_mode = {
            let mut current = self.current.lock().unwrap();
            let old = *current;
            *current = new_mode;
            old
        };

        // Update history
        self.history.lock().unwrap().push(new_mode);

        // Trigger callbacks
        let callbacks = self.callbacks.lock().unwrap();
        for callback in callbacks.iter() {
            callback(old_mode, new_mode);
        }
    }

    fn validate_operation(&self, operation: &str, size: Option<usize>) -> bool {
        let mode = self.current_mode();
        match mode {
            OperatingMode::Plan => operation == "read",
            OperatingMode::Build => true,
            OperatingMode::Review => operation != "execute" && size.is_none_or(|s| s < 10240),
        }
    }

    fn validate_path_access(&self, path: &PathBuf, operation: &str) -> bool {
        let mode = self.current_mode();
        match mode {
            OperatingMode::Plan => operation == "read",
            OperatingMode::Build => true,
            OperatingMode::Review => !path.starts_with("/etc") && !path.starts_with("/sys"),
        }
    }

    fn is_tool_available(&self, tool: &str) -> bool {
        let mode = self.current_mode();
        match mode {
            OperatingMode::Plan => matches!(tool, "search" | "grep" | "tree"),
            OperatingMode::Build => true,
            OperatingMode::Review => !matches!(tool, "delete" | "execute"),
        }
    }

    fn save_state(&self) -> ModeState {
        ModeState {
            mode: self.current_mode(),
            timestamp: std::time::SystemTime::now(),
        }
    }

    fn restore_state(&self, state: &ModeState) {
        self.switch_mode(state.mode);
    }

    fn get_mode_history(&self) -> Vec<OperatingMode> {
        self.history.lock().unwrap().clone()
    }

    fn on_mode_change<F>(&self, callback: F)
    where
        F: Fn(OperatingMode, OperatingMode) + Send + Sync + 'static,
    {
        self.callbacks.lock().unwrap().push(Box::new(callback));
    }
}

/// Mock limits for benchmarking
struct ModeLimits {
    max_file_size: usize,
    max_operations: usize,
    allow_writes: bool,
    allow_execution: bool,
}

impl ModeLimits {
    fn for_mode(mode: OperatingMode) -> Self {
        match mode {
            OperatingMode::Plan => ModeLimits {
                max_file_size: 0,
                max_operations: 1000,
                allow_writes: false,
                allow_execution: false,
            },
            OperatingMode::Build => ModeLimits {
                max_file_size: usize::MAX,
                max_operations: usize::MAX,
                allow_writes: true,
                allow_execution: true,
            },
            OperatingMode::Review => ModeLimits {
                max_file_size: 10240,
                max_operations: 10000,
                allow_writes: true,
                allow_execution: false,
            },
        }
    }
}

struct ModeState {
    mode: OperatingMode,
    timestamp: std::time::SystemTime,
}

criterion_group!(
    benches,
    bench_mode_switching,
    bench_mode_enforcement,
    bench_mode_state,
    bench_mode_features,
    bench_mode_callbacks,
    bench_worst_case_modes
);
criterion_main!(benches);
