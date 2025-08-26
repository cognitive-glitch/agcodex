//! Benchmarks for AST compression at different levels.
//! Tests compression ratio, speed, and memory usage for Light/Medium/Hard modes.

use agcodex_core::context_engine::ast_compactor::AstCompactor;
use agcodex_core::context_engine::ast_compactor::CompactOptions;
use agcodex_core::context_engine::ast_compactor::CompressionLevel;
use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;
use criterion::criterion_group;
use criterion::criterion_main;
use std::hint::black_box;

/// Sample code snippets for benchmarking different sizes
fn get_test_samples() -> Vec<(&'static str, String)> {
    vec![
        (
            "small_function",
            r#"
fn calculate_factorial(n: u64) -> u64 {
    if n <= 1 {
        1
    } else {
        n * calculate_factorial(n - 1)
    }
}
"#
            .to_string(),
        ),
        (
            "medium_class",
            r#"
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct CacheManager<K, V> {
    cache: Arc<Mutex<HashMap<K, V>>>,
    max_size: usize,
    hits: Arc<Mutex<u64>>,
    misses: Arc<Mutex<u64>>,
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone> CacheManager<K, V> {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            max_size,
            hits: Arc::new(Mutex::new(0)),
            misses: Arc::new(Mutex::new(0)),
        }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let cache = self.cache.lock().unwrap();
        if let Some(value) = cache.get(key) {
            *self.hits.lock().unwrap() += 1;
            Some(value.clone())
        } else {
            *self.misses.lock().unwrap() += 1;
            None
        }
    }

    pub fn insert(&self, key: K, value: V) {
        let mut cache = self.cache.lock().unwrap();
        if cache.len() >= self.max_size {
            // Simple eviction: remove first item
            if let Some(first_key) = cache.keys().next().cloned() {
                cache.remove(&first_key);
            }
        }
        cache.insert(key, value);
    }

    pub fn hit_rate(&self) -> f64 {
        let hits = *self.hits.lock().unwrap();
        let misses = *self.misses.lock().unwrap();
        let total = hits + misses;
        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }
}
"#
            .to_string(),
        ),
        (
            "large_module",
            r#"
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub name: String,
    pub priority: u8,
    pub status: TaskStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub dependencies: Vec<Uuid>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[async_trait]
pub trait TaskExecutor: Send + Sync {
    async fn execute(&self, task: &Task) -> Result<TaskResult, TaskError>;
    async fn validate(&self, task: &Task) -> bool;
    async fn cleanup(&self, task: &Task);
}

#[derive(Debug)]
pub struct TaskResult {
    pub output: Vec<u8>,
    pub duration: Duration,
    pub metrics: HashMap<String, f64>,
}

#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    #[error("Task execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Task validation failed")]
    ValidationFailed,
    #[error("Task timeout")]
    Timeout,
    #[error("Task cancelled")]
    Cancelled,
}

pub struct TaskScheduler {
    tasks: Arc<RwLock<HashMap<Uuid, Task>>>,
    queue: Arc<Mutex<VecDeque<Uuid>>>,
    executors: Arc<RwLock<Vec<Box<dyn TaskExecutor>>>>,
    max_concurrent: usize,
    running_count: Arc<Mutex<usize>>,
}

impl TaskScheduler {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            queue: Arc::new(Mutex::new(VecDeque::new())),
            executors: Arc::new(RwLock::new(Vec::new())),
            max_concurrent,
            running_count: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn submit_task(&self, mut task: Task) -> Uuid {
        task.id = Uuid::new_v4();
        task.status = TaskStatus::Pending;
        task.created_at = chrono::Utc::now();
        task.updated_at = task.created_at;
        
        let id = task.id;
        self.tasks.write().await.insert(id, task);
        self.queue.lock().await.push_back(id);
        
        id
    }

    pub async fn add_executor(&self, executor: Box<dyn TaskExecutor>) {
        self.executors.write().await.push(executor);
    }

    pub async fn start(&self) {
        loop {
            let running = *self.running_count.lock().await;
            if running >= self.max_concurrent {
                sleep(Duration::from_millis(100)).await;
                continue;
            }

            let task_id = {
                let mut queue = self.queue.lock().await;
                queue.pop_front()
            };

            if let Some(id) = task_id {
                let scheduler = self.clone();
                tokio::spawn(async move {
                    scheduler.execute_task(id).await;
                });
            } else {
                sleep(Duration::from_millis(100)).await;
            }
        }
    }

    async fn execute_task(&self, task_id: Uuid) {
        *self.running_count.lock().await += 1;
        
        let task = {
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(&task_id) {
                task.status = TaskStatus::Running;
                task.updated_at = chrono::Utc::now();
                task.clone()
            } else {
                *self.running_count.lock().await -= 1;
                return;
            }
        };

        let executors = self.executors.read().await;
        for executor in executors.iter() {
            if executor.validate(&task).await {
                match executor.execute(&task).await {
                    Ok(_result) => {
                        let mut tasks = self.tasks.write().await;
                        if let Some(task) = tasks.get_mut(&task_id) {
                            task.status = TaskStatus::Completed;
                            task.updated_at = chrono::Utc::now();
                        }
                    }
                    Err(_err) => {
                        let mut tasks = self.tasks.write().await;
                        if let Some(task) = tasks.get_mut(&task_id) {
                            task.status = TaskStatus::Failed;
                            task.updated_at = chrono::Utc::now();
                        }
                    }
                }
                executor.cleanup(&task).await;
                break;
            }
        }

        *self.running_count.lock().await -= 1;
    }

    pub async fn get_task_status(&self, id: Uuid) -> Option<TaskStatus> {
        self.tasks.read().await.get(&id).map(|t| t.status)
    }

    pub async fn cancel_task(&self, id: Uuid) -> bool {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(&id) {
            if task.status == TaskStatus::Pending {
                task.status = TaskStatus::Cancelled;
                task.updated_at = chrono::Utc::now();
                return true;
            }
        }
        false
    }
}

impl Clone for TaskScheduler {
    fn clone(&self) -> Self {
        Self {
            tasks: Arc::clone(&self.tasks),
            queue: Arc::clone(&self.queue),
            executors: Arc::clone(&self.executors),
            max_concurrent: self.max_concurrent,
            running_count: Arc::clone(&self.running_count),
        }
    }
}
"#
            .to_string(),
        ),
    ]
}

/// Benchmark compression at different levels
fn bench_compression_levels(c: &mut Criterion) {
    let samples = get_test_samples();
    let mut group = c.benchmark_group("compression_levels");

    for (name, code) in samples.iter() {
        let code_size = code.len() as u64;
        group.throughput(Throughput::Bytes(code_size));

        // Benchmark Light compression (70% reduction)
        group.bench_with_input(BenchmarkId::new("light", name), code, |b, code| {
            let compactor = AstCompactor::new();
            let opts = CompactOptions {
                compression_level: CompressionLevel::Light,
                preserve_mappings: false,
                precision_high: false,
                include_weights: false,
            };
            b.iter(|| {
                let compressed = compactor.compact_source(black_box(code), &opts);
                black_box(compressed)
            });
        });

        // Benchmark Medium compression (85% reduction)
        group.bench_with_input(BenchmarkId::new("medium", name), code, |b, code| {
            let compactor = AstCompactor::new();
            let opts = CompactOptions {
                compression_level: CompressionLevel::Medium,
                preserve_mappings: false,
                precision_high: false,
                include_weights: false,
            };
            b.iter(|| {
                let compressed = compactor.compact_source(black_box(code), &opts);
                black_box(compressed)
            });
        });

        // Benchmark Hard compression (95% reduction)
        group.bench_with_input(BenchmarkId::new("hard", name), code, |b, code| {
            let compactor = AstCompactor::new();
            let opts = CompactOptions {
                compression_level: CompressionLevel::Hard,
                preserve_mappings: false,
                precision_high: false,
                include_weights: false,
            };
            b.iter(|| {
                let compressed = compactor.compact_source(black_box(code), &opts);
                black_box(compressed)
            });
        });
    }
    group.finish();
}

/// Benchmark compression ratio effectiveness
fn bench_compression_ratio(c: &mut Criterion) {
    let samples = get_test_samples();
    let mut group = c.benchmark_group("compression_ratio");

    for (name, code) in samples.iter() {
        group.bench_function(BenchmarkId::new("ratio_analysis", name), |b| {
            b.iter(|| {
                let compactor = AstCompactor::new();

                let light_opts = CompactOptions {
                    compression_level: CompressionLevel::Light,
                    preserve_mappings: false,
                    precision_high: false,
                    include_weights: false,
                };
                let light_result = compactor.compact_source(code, &light_opts);
                let light_ratio = light_result.compression_ratio as f64;

                let medium_opts = CompactOptions {
                    compression_level: CompressionLevel::Medium,
                    preserve_mappings: false,
                    precision_high: false,
                    include_weights: false,
                };
                let medium_result = compactor.compact_source(code, &medium_opts);
                let medium_ratio = medium_result.compression_ratio as f64;

                let hard_opts = CompactOptions {
                    compression_level: CompressionLevel::Hard,
                    preserve_mappings: false,
                    precision_high: false,
                    include_weights: false,
                };
                let hard_result = compactor.compact_source(code, &hard_opts);
                let hard_ratio = hard_result.compression_ratio as f64;

                black_box((light_ratio, medium_ratio, hard_ratio))
            });
        });
    }
    group.finish();
}

/// Benchmark incremental compression (simulating real-world editing)
fn bench_incremental_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_compression");

    // Simulate editing a file - start with base code and apply modifications
    let base_code = r#"
pub struct DataProcessor {
    buffer: Vec<u8>,
    processed: usize,
}

impl DataProcessor {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            processed: 0,
        }
    }

    pub fn process(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
        self.processed += data.len();
    }
}
"#;

    let modifications = vec![
        // Add a method
        (
            "add_method",
            r#"
pub struct DataProcessor {
    buffer: Vec<u8>,
    processed: usize,
}

impl DataProcessor {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            processed: 0,
        }
    }

    pub fn process(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
        self.processed += data.len();
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.processed = 0;
    }
}
"#,
        ),
        // Add field
        (
            "add_field",
            r#"
pub struct DataProcessor {
    buffer: Vec<u8>,
    processed: usize,
    errors: usize,
}

impl DataProcessor {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            processed: 0,
            errors: 0,
        }
    }

    pub fn process(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
        self.processed += data.len();
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.processed = 0;
        self.errors = 0;
    }
}
"#,
        ),
    ];

    for (mod_name, modified_code) in modifications.iter() {
        group.bench_function(BenchmarkId::new("incremental", mod_name), |b| {
            let compactor = AstCompactor::new();
            let opts = CompactOptions {
                compression_level: CompressionLevel::Medium,
                preserve_mappings: false,
                precision_high: false,
                include_weights: false,
            };
            b.iter(|| {
                // First compress base
                let _base_compressed = compactor.compact_source(black_box(base_code), &opts);
                // Then compress modified version
                let modified_compressed = compactor.compact_source(black_box(modified_code), &opts);
                black_box(modified_compressed)
            });
        });
    }
    group.finish();
}

/// Benchmark worst-case scenarios
fn bench_worst_case_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("worst_case_compression");

    // Highly repetitive code (should compress well)
    let repetitive_code = (0..100)
        .map(|i| format!("let var_{} = {};\n", i, i))
        .collect::<String>();

    // Random-like code (harder to compress)
    let complex_code = r#"
#[macro_export]
macro_rules! impl_display {
    ($($t:ty),*) => {
        $(impl std::fmt::Display for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{:?}", self)
            }
        })*
    };
}

type ComplexType<'a, T> = Box<dyn Fn(&'a T) -> Result<Vec<&'a str>, Box<dyn std::error::Error>> + Send + Sync>;

const LOOKUP_TABLE: [u8; 256] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
    // ... more data
];
"#;

    group.bench_function("repetitive", |b| {
        let compactor = AstCompactor::new();
        let opts = CompactOptions {
            compression_level: CompressionLevel::Hard,
            preserve_mappings: false,
            precision_high: false,
            include_weights: false,
        };
        b.iter(|| {
            let compressed = compactor.compact_source(black_box(&repetitive_code), &opts);
            black_box(compressed)
        });
    });

    group.bench_function("complex", |b| {
        let compactor = AstCompactor::new();
        let opts = CompactOptions {
            compression_level: CompressionLevel::Hard,
            preserve_mappings: false,
            precision_high: false,
            include_weights: false,
        };
        b.iter(|| {
            let compressed = compactor.compact_source(black_box(complex_code), &opts);
            black_box(compressed)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_compression_levels,
    bench_compression_ratio,
    bench_incremental_compression,
    bench_worst_case_compression
);
criterion_main!(benches);
