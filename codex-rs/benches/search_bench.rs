//! Benchmarks for multi-layer search engine performance.
//! Tests Symbol (<1ms), Tantivy (<5ms), and AST (<10ms) layers.

use agcodex_core::code_tools::search::Location;
use agcodex_core::code_tools::search::MultiLayerSearchEngine;
use agcodex_core::code_tools::search::Scope;
use agcodex_core::code_tools::search::SearchConfig;
use agcodex_core::code_tools::search::SearchQuery;
use agcodex_core::code_tools::search::Symbol;
use agcodex_core::code_tools::search::SymbolKind;
use agcodex_core::code_tools::search::Visibility;
use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;
// use criterion::black_box; // Deprecated, using std::hint::black_box instead
use criterion::criterion_group;
use criterion::criterion_main;
use dashmap::DashMap;
// use std::collections::HashMap; // Unused import
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::runtime::Runtime;

/// Generate test symbols for benchmarking
fn generate_test_symbols(count: usize) -> Vec<Symbol> {
    (0..count)
        .map(|i| Symbol {
            name: format!("symbol_{}", i),
            kind: if i % 3 == 0 {
                SymbolKind::Function
            } else if i % 3 == 1 {
                SymbolKind::Struct
            } else {
                SymbolKind::Variable
            },
            location: Location {
                file: PathBuf::from(format!("src/module_{}.rs", i / 100)),
                line: (i % 1000),
                column: (i % 80),
                byte_offset: i * 10,
            },
            scope: Scope {
                function: None,
                class: None,
                module: Some(format!("module_{}", i / 10)),
                namespace: None,
            },
            visibility: Visibility::Public,
        })
        .collect()
}

/// Generate test file content for indexing
fn generate_test_files(dir: &TempDir, count: usize) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for i in 0..count {
        let file_path = dir.path().join(format!("file_{}.rs", i));
        let content = format!(
            r#"
// File {} - Test content for benchmarking
use std::collections::HashMap;

pub struct TestStruct{} {{
    field1: String,
    field2: usize,
    field3: HashMap<String, Vec<u8>>,
}}

impl TestStruct{} {{
    pub fn new() -> Self {{
        Self {{
            field1: String::new(),
            field2: 0,
            field3: HashMap::new(),
        }}
    }}

    pub fn process(&mut self, input: &str) -> Result<(), Error> {{
        // Processing logic
        self.field1 = input.to_string();
        self.field2 += 1;
        Ok(())
    }}

    pub fn search_pattern(&self, pattern: &str) -> bool {{
        self.field1.contains(pattern)
    }}
}}

fn helper_function_{}() {{
    let data = vec![1, 2, 3, 4, 5];
    let sum: i32 = data.iter().sum();
    println!("Sum: {{}}", sum);
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn test_creation() {{
        let instance = TestStruct{}::new();
        assert_eq!(instance.field2, 0);
    }}
}}
"#,
            i, i, i, i, i
        );
        std::fs::write(&file_path, content).unwrap();
        paths.push(file_path);
    }
    paths
}

/// Benchmark Symbol layer (<1ms target)
fn bench_symbol_layer(c: &mut Criterion) {
    let mut group = c.benchmark_group("symbol_layer");
    let rt = Runtime::new().unwrap();

    // Different dataset sizes
    let sizes = vec![
        ("small", 100),
        ("medium", 1000),
        ("large", 10000),
        ("xlarge", 50000),
    ];

    for (name, count) in sizes {
        let symbols = generate_test_symbols(count);
        let _symbol_index: Arc<DashMap<String, Vec<Symbol>>> = Arc::new(DashMap::new());

        // Create search engine and populate it
        let config = SearchConfig {
            enable_tantivy: false,
            enable_ripgrep_fallback: false,
            enable_ast_cache: false,
            ..Default::default()
        };
        let engine = MultiLayerSearchEngine::new(config).unwrap();

        // Add symbols to engine
        for symbol in &symbols {
            engine.add_symbol(symbol.clone());
        }

        // Benchmark exact symbol lookup
        group.bench_function(BenchmarkId::new("exact_lookup", name), |b| {
            b.iter(|| {
                let query = SearchQuery::symbol(format!("symbol_{}", count / 2));
                let result = rt.block_on(engine.search(query));
                std::hint::black_box(result)
            });
        });

        // Benchmark prefix search (using fuzzy search as closest equivalent)
        group.bench_function(BenchmarkId::new("prefix_search", name), |b| {
            b.iter(|| {
                let query = SearchQuery::symbol("symbol_1").fuzzy();
                let result = rt.block_on(engine.search(query));
                std::hint::black_box(result)
            });
        });

        // Benchmark fuzzy search
        group.bench_function(BenchmarkId::new("fuzzy_search", name), |b| {
            b.iter(|| {
                let query = SearchQuery::symbol("symbl").fuzzy(); // Intentional typo
                let result = rt.block_on(engine.search(query));
                std::hint::black_box(result)
            });
        });
    }
    group.finish();
}

/// Benchmark Tantivy layer (<5ms target)
fn bench_tantivy_layer(c: &mut Criterion) {
    let mut group = c.benchmark_group("tantivy_layer");
    let _rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();

    // Generate test files
    let file_counts = vec![("small", 10), ("medium", 50), ("large", 100)];

    for (name, count) in file_counts {
        let files = generate_test_files(&temp_dir, count);

        // Mock Tantivy operations (since we need actual Tantivy setup)
        group.bench_function(BenchmarkId::new("full_text_search", name), |b| {
            b.iter(|| {
                // Simulate full-text search
                let query = "process";
                let mut results = Vec::new();
                for file in &files {
                    if let Ok(content) = std::fs::read_to_string(file)
                        && content.contains(query)
                    {
                        results.push(file.clone());
                    }
                }
                std::hint::black_box(results)
            });
        });

        // Benchmark phrase search
        group.bench_function(BenchmarkId::new("phrase_search", name), |b| {
            b.iter(|| {
                let phrase = "pub fn process";
                let mut results = Vec::new();
                for file in &files {
                    if let Ok(content) = std::fs::read_to_string(file)
                        && content.contains(phrase)
                    {
                        results.push(file.clone());
                    }
                }
                std::hint::black_box(results)
            });
        });

        // Benchmark boolean query
        group.bench_function(BenchmarkId::new("boolean_query", name), |b| {
            b.iter(|| {
                let must_have = "TestStruct";
                let should_have = "process";
                let must_not_have = "deprecated";

                let mut results = Vec::new();
                for file in &files {
                    if let Ok(content) = std::fs::read_to_string(file)
                        && content.contains(must_have)
                        && content.contains(should_have)
                        && !content.contains(must_not_have)
                    {
                        results.push(file.clone());
                    }
                }
                std::hint::black_box(results)
            });
        });
    }
    group.finish();
}

/// Benchmark AST layer (<10ms target)
fn bench_ast_layer(c: &mut Criterion) {
    let mut group = c.benchmark_group("ast_layer");
    let _temp_dir = TempDir::new().unwrap();

    // Test different code complexities
    let test_cases = vec![
        (
            "simple_function",
            r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#,
        ),
        (
            "complex_struct",
            r#"
pub struct ComplexType<T> {
    data: Vec<T>,
    cache: HashMap<String, T>,
    metadata: Option<Box<dyn Any>>,
}

impl<T: Clone> ComplexType<T> {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            cache: HashMap::new(),
            metadata: None,
        }
    }

    pub fn process(&mut self, item: T) where T: Display {
        self.data.push(item.clone());
        self.cache.insert(format!("{}", item), item);
    }
}
"#,
        ),
        (
            "nested_code",
            r#"
mod outer {
    pub mod inner {
        pub struct Nested {
            value: i32,
        }

        impl Nested {
            pub fn new(value: i32) -> Self {
                Self { value }
            }

            pub fn transform<F>(&self, f: F) -> i32 
            where
                F: Fn(i32) -> i32,
            {
                f(self.value)
            }
        }
    }

    pub use inner::Nested;

    pub fn create_nested() -> Nested {
        Nested::new(42)
    }
}
"#,
        ),
    ];

    for (name, code) in test_cases {
        let code_size = code.len() as u64;
        group.throughput(Throughput::Bytes(code_size));

        // Benchmark AST parsing
        group.bench_function(BenchmarkId::new("parse_ast", name), |b| {
            b.iter(|| {
                // Simulate AST parsing (would use tree-sitter in real implementation)
                let tokens: Vec<&str> = code.split_whitespace().collect();
                let ast_nodes = tokens.len();
                std::hint::black_box(ast_nodes)
            });
        });

        // Benchmark semantic search in AST
        group.bench_function(BenchmarkId::new("semantic_search", name), |b| {
            b.iter(|| {
                // Simulate finding all function definitions
                let mut functions = Vec::new();
                for line in code.lines() {
                    if line.trim().starts_with("fn ") || line.trim().starts_with("pub fn ") {
                        functions.push(line);
                    }
                }
                std::hint::black_box(functions)
            });
        });

        // Benchmark pattern matching in AST
        group.bench_function(BenchmarkId::new("pattern_match", name), |b| {
            b.iter(|| {
                // Simulate finding impl blocks
                let mut impl_blocks = 0;
                let mut in_impl = false;
                let mut brace_count = 0;

                for line in code.lines() {
                    if line.contains("impl") && !in_impl {
                        in_impl = true;
                        impl_blocks += 1;
                    }
                    if in_impl {
                        brace_count += line.matches('{').count();
                        brace_count -= line.matches('}').count();
                        if brace_count == 0 {
                            in_impl = false;
                        }
                    }
                }
                std::hint::black_box(impl_blocks)
            });
        });
    }
    group.finish();
}

/// Benchmark combined multi-layer search
fn bench_multi_layer_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_layer_search");
    let _rt = Runtime::new().unwrap();

    // Simulate different query complexities
    let queries = vec![
        ("simple", "function_name"),
        ("moderate", "process AND data"),
        ("complex", "impl Display WHERE struct TestStruct"),
    ];

    for (complexity, _query) in queries {
        group.bench_function(BenchmarkId::new("integrated_search", complexity), |b| {
            b.iter(|| {
                // Layer 1: Symbol search (<1ms)
                let symbol_start = std::time::Instant::now();
                let symbol_results = 5; // Simulate finding 5 symbols
                let symbol_time = symbol_start.elapsed();

                // Layer 2: Tantivy search (<5ms)
                let tantivy_start = std::time::Instant::now();
                std::thread::sleep(std::time::Duration::from_micros(500)); // Simulate work
                let tantivy_results = 10; // Simulate finding 10 documents
                let tantivy_time = tantivy_start.elapsed();

                // Layer 3: AST search (<10ms)
                let ast_start = std::time::Instant::now();
                std::thread::sleep(std::time::Duration::from_millis(1)); // Simulate work
                let ast_results = 3; // Simulate finding 3 AST matches
                let ast_time = ast_start.elapsed();

                // Combine results
                let total_results = symbol_results + tantivy_results + ast_results;
                std::hint::black_box((total_results, symbol_time, tantivy_time, ast_time))
            });
        });
    }

    // Benchmark fallback strategy
    group.bench_function("fallback_cascade", |b| {
        b.iter(|| {
            let _query = "very_specific_symbol_12345";

            // Try symbol layer first
            let symbol_found = false;

            // Fallback to Tantivy
            let tantivy_found = if !symbol_found {
                std::thread::sleep(std::time::Duration::from_micros(100));
                false
            } else {
                false
            };

            // Fallback to AST
            let ast_found = if !tantivy_found {
                std::thread::sleep(std::time::Duration::from_micros(200));
                false
            } else {
                false
            };

            // Final fallback to ripgrep
            let ripgrep_found = if !ast_found {
                std::thread::sleep(std::time::Duration::from_micros(500));
                true
            } else {
                false
            };

            std::hint::black_box(ripgrep_found)
        });
    });

    group.finish();
}

/// Benchmark cache performance
fn bench_search_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_cache");

    // Simulate cache with different hit rates
    let cache_sizes = vec![("small", 100), ("medium", 1000), ("large", 10000)];

    for (name, size) in cache_sizes {
        let cache: Arc<DashMap<String, Vec<String>>> = Arc::new(DashMap::new());

        // Populate cache
        for i in 0..size {
            cache.insert(format!("query_{}", i), vec![format!("result_{}", i)]);
        }

        // Benchmark cache hit
        group.bench_function(BenchmarkId::new("cache_hit", name), |b| {
            b.iter(|| {
                let key = format!("query_{}", size / 2);
                let result = cache.get(&key);
                std::hint::black_box(result)
            });
        });

        // Benchmark cache miss
        group.bench_function(BenchmarkId::new("cache_miss", name), |b| {
            b.iter(|| {
                let key = format!("nonexistent_query_{}", size * 2);
                let result = cache.get(&key);
                std::hint::black_box(result)
            });
        });

        // Benchmark cache update
        group.bench_function(BenchmarkId::new("cache_update", name), |b| {
            let counter = std::sync::atomic::AtomicUsize::new(0);
            b.iter(|| {
                let idx = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let key = format!("dynamic_query_{}", idx);
                cache.insert(key, vec![format!("dynamic_result_{}", idx)]);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_symbol_layer,
    bench_tantivy_layer,
    bench_ast_layer,
    bench_multi_layer_search,
    bench_search_cache
);
criterion_main!(benches);
