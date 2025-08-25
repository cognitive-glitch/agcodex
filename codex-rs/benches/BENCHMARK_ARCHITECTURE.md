# AGCodex Benchmark Architecture

## Overview

This document describes the architecture and design principles of AGCodex's performance benchmarking suite.

## Benchmark Design Principles

### 1. Realistic Scenarios
Each benchmark tests real-world usage patterns:
- **Best Case**: Optimal conditions (hot cache, small data)
- **Average Case**: Typical usage patterns
- **Worst Case**: Stress conditions (cold cache, large data, contention)

### 2. Isolation and Reproducibility
- Use `black_box()` to prevent compiler optimizations from skewing results
- Create fresh test data for each iteration when testing mutations
- Use `TempDir` for filesystem operations to ensure clean state

### 3. Comprehensive Coverage
Each critical path has dedicated benchmarks covering:
- **Latency**: Single operation response time
- **Throughput**: Operations per second
- **Scalability**: Performance with increasing load
- **Concurrency**: Behavior under parallel access

## Benchmark Categories

### Compression Benchmarks
```rust
// Test matrix: 3 compression levels × 4 code sizes × 3 patterns
CompressionLevel::{Light, Medium, Hard}
CodeSize::{Small, Medium, Large, XLarge}
Pattern::{Simple, Complex, Repetitive}
```

**Key Insights:**
- Light compression targets 70% reduction at >100MB/s
- Medium compression targets 85% reduction at >50MB/s
- Hard compression targets 95% reduction at >25MB/s

### Search Engine Benchmarks
```rust
// Multi-layer architecture testing
Layer 1: Symbol Index (DashMap) - <1ms target
Layer 2: Tantivy Full-text - <5ms target
Layer 3: AST Semantic - <10ms target
Layer 4: Ripgrep Fallback - best effort
```

**Key Insights:**
- Symbol index must stay hot in memory
- Tantivy benefits from proper schema design
- AST parsing should be cached aggressively

### Agent Orchestration Benchmarks
```rust
// Concurrency patterns tested
Sequential: Agent1 → Agent2 → Agent3
Parallel: Agent1 || Agent2 || Agent3
Mixed: (Agent1 → Agent2) || Agent3
Complex: Diamond/Fork-Join patterns
```

**Key Insights:**
- Spawn overhead dominates for short-lived agents
- Context switching cost scales with data size
- Message passing is faster than shared state

### Session Persistence Benchmarks
```rust
// Storage patterns tested
Save Patterns: Incremental, Batch, Checkpoint
Load Patterns: Full, Metadata-only, Partial
Compression: Zstd levels 1-9
```

**Key Insights:**
- Incremental saves reduce I/O significantly
- Memory-mapped metadata speeds up listing
- Zstd level 3 offers best speed/ratio balance

### Mode Switching Benchmarks
```rust
// Validation complexity tested
Simple: Mode check only
Medium: Mode + size validation
Complex: Mode + path + permissions
```

**Key Insights:**
- Build mode (no restrictions) is fastest
- Review mode size checks add ~10μs overhead
- Callback chains should be kept minimal

## Performance Monitoring

### Continuous Integration
```yaml
benchmark:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v3
    - uses: bencherdev/bencher@main
    - run: cargo bench -- --output-format bencher | tee output.txt
    - run: bencher run --file output.txt --threshold 1.1
```

### Local Development
```bash
# Quick validation (10 samples)
cargo bench -- --sample-size 10

# Full run with baseline
cargo bench -- --save-baseline dev

# Compare after changes
cargo bench -- --baseline dev
```

## Optimization Workflow

### 1. Measure First
```bash
cargo bench --bench [target]_bench > before.txt
```

### 2. Profile Bottlenecks
```bash
# CPU profiling
cargo flamegraph --bench [target]_bench

# Memory profiling
heaptrack target/release/deps/[target]_bench-*
```

### 3. Implement Optimization
Focus on:
- Algorithm complexity reduction
- Cache locality improvements
- Lock contention reduction
- Allocation minimization

### 4. Validate Improvement
```bash
cargo bench --bench [target]_bench > after.txt
diff before.txt after.txt
```

### 5. Check for Regressions
```bash
cargo bench -- --baseline main
```

## Benchmark Interpretation

### Statistical Significance
- **R²**: Coefficient of determination (>0.95 is good)
- **Std Dev**: Standard deviation (<10% of mean is stable)
- **Outliers**: Mild outliers OK, severe outliers indicate issues

### Performance Classes
```
Excellent: Better than target by >20%
Good:      Within target range
Acceptable: Within acceptable range
Warning:   Approaching critical threshold
Critical:  Exceeds critical threshold
```

### Common Pitfalls

1. **Micro-optimization**: Don't optimize <1% of runtime
2. **Artificial scenarios**: Ensure benchmarks reflect real usage
3. **Ignoring variance**: High variance often indicates systemic issues
4. **Cache effects**: Warm up caches before measuring
5. **Allocation pressure**: Watch for GC pauses in results

## Benchmark Maintenance

### Adding New Benchmarks
1. Identify critical path to benchmark
2. Define performance targets
3. Create realistic test scenarios
4. Implement using criterion
5. Document in README
6. Add to CI pipeline

### Updating Existing Benchmarks
1. Keep backwards compatibility when possible
2. Document breaking changes
3. Re-baseline after significant changes
4. Update performance targets if needed

## Tools and Resources

### Profiling Tools
- **flamegraph**: CPU profiling visualization
- **heaptrack**: Heap memory profiling
- **perf**: Linux performance counters
- **valgrind**: Memory leak detection
- **cachegrind**: Cache performance analysis

### Analysis Tools
- **criterion**: Statistical benchmarking
- **cargo-bench**: Benchmark runner
- **bencher**: Continuous benchmarking
- **hyperfine**: Command-line benchmarking

### References
- [Criterion.rs Book](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Flame Graphs](https://www.brendangregg.com/flamegraphs.html)
- [Systems Performance](https://www.brendangregg.com/systems-performance-2nd-edition-book.html)