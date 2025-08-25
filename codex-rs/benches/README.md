# AGCodex Performance Benchmarks

This directory contains comprehensive performance benchmarks for AGCodex's critical paths.

## Benchmark Categories

### 1. AST Compression (`compression_bench.rs`)
Tests the performance of AST compression at different levels (Light/Medium/Hard).

**Key Metrics:**
- Compression speed (MB/s)
- Compression ratio (70-95% reduction)
- Memory usage during compression
- Incremental compression performance

**Scenarios:**
- Small functions (<100 lines)
- Medium classes (100-500 lines)
- Large modules (500+ lines)
- Worst-case: repetitive vs complex code

### 2. Search Engine (`search_bench.rs`)
Benchmarks the multi-layer search architecture performance.

**Key Metrics:**
- Symbol layer: <1ms target
- Tantivy layer: <5ms target
- AST layer: <10ms target
- Cache hit/miss performance

**Scenarios:**
- Exact symbol lookups
- Fuzzy matching
- Full-text search
- Pattern matching in AST
- Multi-layer cascade fallback

### 3. Agent Orchestration (`agent_bench.rs`)
Measures subagent spawn overhead and coordination performance.

**Key Metrics:**
- Single agent spawn: <100ms target
- Parallel agent spawn scaling
- Context switching overhead
- Message passing latency

**Scenarios:**
- Sequential execution
- Parallel execution (2-16 agents)
- Complex dependency graphs
- Resource contention

### 4. Session Persistence (`session_bench.rs`)
Tests session save/load operations with Zstd compression.

**Key Metrics:**
- Save time: <500ms for typical sessions
- Load time: <200ms for metadata
- Checkpoint creation/restoration
- Compression effectiveness

**Scenarios:**
- Small sessions (10 messages)
- Medium sessions (50 messages)
- Large sessions (100+ messages)
- Concurrent access patterns

### 5. Mode Switching (`mode_bench.rs`)
Benchmarks Plan/Build/Review mode transitions.

**Key Metrics:**
- Mode switch: <50ms target
- Validation overhead
- Callback execution time
- State persistence

**Scenarios:**
- Single mode switches
- Rapid cycling
- Concurrent switches
- Complex validation chains

## Running Benchmarks

### Run All Benchmarks
```bash
cargo bench
```

### Run Specific Benchmark
```bash
cargo bench --bench compression_bench
cargo bench --bench search_bench
cargo bench --bench agent_bench
cargo bench --bench session_bench
cargo bench --bench mode_bench
```

### Run with Baseline Comparison
```bash
# Save baseline
cargo bench -- --save-baseline main

# Compare against baseline
cargo bench -- --baseline main
```

### Generate HTML Reports
```bash
cargo bench -- --verbose
# Reports available in target/criterion/
```

## Performance Targets

| Component | Target | Acceptable | Critical |
|-----------|--------|------------|----------|
| Mode Switch | <50ms | <100ms | >200ms |
| Symbol Search | <1ms | <5ms | >10ms |
| Tantivy Search | <5ms | <10ms | >20ms |
| AST Search | <10ms | <20ms | >50ms |
| Agent Spawn | <100ms | <200ms | >500ms |
| Session Save | <500ms | <1s | >2s |
| Session Load | <200ms | <500ms | >1s |
| Compression (Light) | >100MB/s | >50MB/s | <20MB/s |
| Compression (Medium) | >50MB/s | >25MB/s | <10MB/s |
| Compression (Hard) | >25MB/s | >10MB/s | <5MB/s |

## Profiling

### CPU Profiling
```bash
cargo bench --bench compression_bench -- --profile-time=10
```

### Memory Profiling
```bash
CARGO_PROFILE_BENCH_DEBUG=true cargo bench --bench session_bench
valgrind --tool=massif target/release/deps/session_bench-*
```

### Flamegraph Generation
```bash
cargo flamegraph --bench compression_bench -- --bench
```

## Optimization Tips

1. **Compression**: Balance between speed and ratio based on use case
2. **Search**: Ensure symbol index is hot in cache for <1ms lookups
3. **Agents**: Reuse agent contexts when possible to avoid spawn overhead
4. **Sessions**: Use incremental saves for better performance
5. **Modes**: Cache validation results for repeated operations

## Continuous Benchmarking

Add to CI pipeline:
```yaml
- name: Run Benchmarks
  run: |
    cargo bench -- --output-format bencher | tee output.txt
    # Check for regressions
    cargo bench -- --baseline main
```

## Interpreting Results

- **time**: Lower is better (measured in ns/μs/ms)
- **throughput**: Higher is better (measured in MB/s or ops/s)
- **R²**: Closer to 1.0 means more consistent results
- **std dev**: Lower means more predictable performance

## Best Practices

1. Run benchmarks on a quiet system
2. Disable CPU frequency scaling during benchmarks
3. Run multiple iterations for statistical significance
4. Compare against baseline after optimizations
5. Profile before optimizing to identify bottlenecks