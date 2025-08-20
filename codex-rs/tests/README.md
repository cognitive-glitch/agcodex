# AGCodex Test Suite

Comprehensive test infrastructure for AGCodex Phase 6, covering all critical functionality including mode switching, AST parsing, session persistence, and rebranding verification.

## Test Structure

```
tests/
├── unit/                     # Unit tests for individual components
│   ├── modes_test.rs        # Operating mode switching (Plan/Build/Review)
│   ├── ast_test.rs          # AST parsing and compression tests
│   ├── persistence_test.rs  # Session persistence with Zstd
│   └── rebranding_test.rs   # Codex→AGCodex migration verification
├── integration/             # Integration tests for complete workflows
│   ├── tui_mode_switching.rs # TUI mode switching with Shift+Tab
│   ├── session_management.rs # Complete session lifecycle
│   └── ast_search.rs        # AST-based code search functionality
├── benchmarks/             # Performance benchmarks
│   ├── ast_parsing.rs      # AST parsing performance targets
│   ├── compression.rs      # Compression ratio benchmarks
│   └── search_performance.rs # Search speed benchmarks
├── fixtures/               # Test data and sample files
│   ├── sample_code/        # Sample source files for testing
│   │   ├── rust_sample.rs  # Rust code sample
│   │   ├── python_sample.py # Python code sample
│   │   ├── typescript_sample.tsx # TypeScript React sample
│   │   └── go_sample.go    # Go code sample
│   ├── test_sessions/      # Sample session data
│   └── compression_data/   # Data for compression testing
├── helpers/                # Test utilities and common functionality
│   ├── mod.rs             # Module exports
│   └── test_utils.rs      # Test helpers and utilities
└── README.md              # This file
```

## Test Categories

### 1. Unit Tests (`tests/unit/`)

#### Mode Switching Tests (`modes_test.rs`)
- **Operating Mode Management**: Tests for Plan/Build/Review mode cycling
- **Restriction Enforcement**: Verifies mode-specific capability restrictions
- **State Persistence**: Ensures mode history is maintained correctly
- **Visual Indicators**: Tests mode indicator updates (📋 PLAN, 🔨 BUILD, 🔍 REVIEW)

**Key Test Cases:**
- `test_shift_tab_mode_cycling()` - Shift+Tab cycles through all modes
- `test_mode_restrictions_enforcement()` - Restrictions are properly enforced
- `test_mode_switching_with_history()` - Mode history is tracked
- `test_visual_indicator_updates()` - Visual indicators update correctly

#### AST Parsing Tests (`ast_test.rs`)
- **Multi-Language Support**: Tests parsing for 50+ programming languages
- **Compression Targets**: Verifies 70%/85%/95% compression ratios
- **Performance Benchmarks**: Ensures <10ms parse time per file
- **Cache Effectiveness**: Tests >90% cache hit rate target

**Key Test Cases:**
- `test_basic_compression_rust()` - 70% compression for Rust code
- `test_medium_compression_typescript()` - 85% compression for TypeScript
- `test_maximum_compression_python()` - 95% compression for Python
- `test_language_detection_accuracy()` - 100% accuracy for file extension detection
- `test_compression_performance()` - <10ms parsing target

#### Session Persistence Tests (`persistence_test.rs`)
- **Zstd Compression**: Tests session compression and decompression
- **Auto-Save Functionality**: Verifies 5-minute auto-save intervals
- **Data Integrity**: Ensures sessions load correctly after save
- **Performance**: Tests <500ms save/load target

**Key Test Cases:**
- `test_session_save_and_load_cycle()` - Complete persistence workflow
- `test_compression_and_decompression()` - Zstd compression integrity
- `test_auto_save_functionality()` - Auto-save triggers correctly
- `test_checkpoint_creation_and_restore()` - Checkpoint system works

#### Rebranding Tests (`rebranding_test.rs`)
- **Complete Migration**: Verifies codex→agcodex transformation
- **Import Updates**: Tests crate name changes
- **Configuration Paths**: Ensures ~/.agcodex instead of ~/.codex
- **Binary Names**: Verifies agcodex binary name

**Key Test Cases:**
- `test_source_code_references()` - No old "codex" references remain
- `test_binary_names_rebranded()` - Binary is named "agcodex"
- `test_config_directory_references()` - Uses ~/.agcodex path
- `test_rebranding_completion_percentage()` - >90% completion target

### 2. Integration Tests (`tests/integration/`)

#### TUI Mode Switching (`tui_mode_switching.rs`)
- **Complete TUI Workflow**: Tests entire mode switching user experience
- **Keyboard Shortcuts**: Verifies Shift+Tab, Ctrl+S, Ctrl+H, etc.
- **State Management**: Tests mode persistence across TUI operations
- **Performance**: Ensures <50ms mode switch time

**Key Test Cases:**
- `test_shift_tab_mode_cycling()` - Full mode cycling in TUI
- `test_app_mode_switching()` - Session manager, history browser activation
- `test_mode_persistence_across_app_modes()` - Operating mode persists
- `test_rapid_mode_switching()` - Handles rapid user input

#### Session Management (`session_management.rs`)
- **Complete Lifecycle**: Tests session creation, saving, loading, deletion
- **Multi-Session Support**: Verifies switching between multiple sessions
- **File Context Management**: Tests adding/removing file contexts
- **Auto-Save Integration**: Tests background auto-save functionality

**Key Test Cases:**
- `test_create_session()` - Session creation workflow
- `test_session_switching()` - Switching between sessions
- `test_multi_session_development()` - Multiple project workflows
- `test_typical_workflow()` - Real-world usage scenarios

### 3. Performance Benchmarks (`tests/benchmarks/`)

#### AST Parsing Benchmarks (`ast_parsing.rs`)
- **Single Language Parsing**: Benchmarks for Rust, Python, Go, TypeScript
- **Cold vs Warm Parsing**: Cache performance comparison
- **Batch Processing**: Multi-file parsing efficiency
- **Memory Usage**: Memory scaling with code size

**Performance Targets:**
- AST parsing: <10ms per file (cached)
- Cache hit rate: >90%
- Language detection: <1ms with 100% accuracy
- Batch processing: <100ms for 10k files

#### Compression Benchmarks
- **Compression Ratios**: Measures actual compression achieved
- **Compression Speed**: Time to compress code samples
- **Memory Efficiency**: Memory usage during compression

#### Search Performance Benchmarks
- **Code Search**: Time to search large codebases
- **AST Query**: Performance of structural queries
- **Index Building**: Time to build search indices

### 4. Test Helpers (`tests/helpers/`)

#### Test Utilities (`test_utils.rs`)
- **TestEnvironment**: Sets up isolated test environments
- **MockDataGenerator**: Creates realistic test data
- **PerformanceAssertions**: Validates performance targets
- **TestValidation**: Common validation helpers
- **AsyncTestHelpers**: Utilities for async test scenarios

**Key Features:**
- Temporary directory management
- Mock session and message generation
- Performance assertion helpers
- Async condition waiting
- Benchmark statistics tracking

## Running Tests

### All Tests
```bash
# Run complete test suite
cargo test --all-features --workspace --no-fail-fast

# Run with coverage
cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info
```

### Specific Test Categories
```bash
# Unit tests only
cargo test --lib --all-features

# Integration tests only
cargo test --test "*" --all-features

# Specific test file
cargo test --test modes_test
cargo test --test tui_mode_switching

# Benchmarks
cargo bench --bench ast_parsing
```

### Performance Tests
```bash
# Run performance-sensitive tests
cargo test performance --all-features

# Check specific performance targets
cargo test test_mode_switching_performance
cargo test test_ast_parsing_performance
cargo test test_session_save_performance
```

## CI/CD Integration

### GitHub Actions Workflow (`.github/workflows/test.yml`)

**Jobs:**
- **test**: Core functionality tests on stable/beta/nightly Rust
- **coverage**: Code coverage reporting with 80% minimum threshold
- **benchmarks**: Performance regression detection
- **rebranding_check**: Automated verification of codex→agcodex migration
- **mode_switching_tests**: TUI mode switching validation
- **session_persistence_tests**: Session management validation
- **security_audit**: Dependency and unsafe code auditing
- **cross_platform**: Tests on Ubuntu, Windows, macOS

**Performance Monitoring:**
- AST parsing regression detection (>10ms fails)
- Cache hit rate monitoring (>90% target)
- Session save/load performance (<500ms target)
- Mode switching speed (<50ms target)

### Quality Gates
- **Code Coverage**: Minimum 80% for new code
- **Performance**: All benchmarks must meet targets
- **Rebranding**: >90% completion required
- **Security**: No high-severity vulnerabilities
- **Cross-Platform**: Tests pass on all supported platforms

## Test Data and Fixtures

### Sample Code Files
- **Rust**: Complex struct/impl with generics and traits
- **Python**: Async/await patterns with dataclasses
- **TypeScript**: React components with hooks and interfaces
- **Go**: HTTP server with concurrent request handling

### Test Sessions
- Sample session data for persistence testing
- Various compression scenarios
- Multi-conversation workflows

## Performance Targets Summary

| Metric | Target | Test Location |
|--------|---------|---------------|
| Mode switching | <50ms | `tui_mode_switching.rs` |
| AST parsing (cached) | <10ms | `ast_parsing.rs` |
| Session save/load | <500ms | `session_management.rs` |
| Cache hit rate | >90% | `ast_test.rs` |
| Code compression | 70-95% | `ast_test.rs` |
| Language detection | 100% accuracy | `ast_test.rs` |
| Code coverage | >80% | CI workflow |
| Rebranding completion | >90% | `rebranding_test.rs` |

## Test Development Guidelines

### Adding New Tests
1. **Use helpers**: Leverage `test_utils.rs` for common functionality
2. **Performance assertions**: Include timing and resource checks
3. **Error scenarios**: Test failure modes and edge cases
4. **Async patterns**: Use `AsyncTestHelpers` for async code
5. **Mock data**: Use `MockDataGenerator` for consistent test data

### Test Structure
```rust
#[test]
fn test_feature_with_expected_behavior() {
    // Arrange: Set up test environment
    let env = TestEnvironment::new();
    let mock_data = MockDataGenerator::session_id();
    
    // Act: Execute the operation
    let (result, duration) = TestTiming::time_operation(|| {
        perform_operation(mock_data)
    });
    
    // Assert: Verify results and performance
    assert!(result.is_ok());
    PerformanceAssertions::assert_duration_under(duration, 100, "operation");
}
```

### Performance Test Patterns
```rust
#[test]
fn test_performance_target() {
    let stats = TestTiming::benchmark_operation(|| {
        expensive_operation()
    }, 100); // 100 iterations
    
    stats.assert_average_under(Duration::from_millis(10), "expensive_operation");
    assert!(stats.min < Duration::from_millis(5));
}
```

## Integration with AGCodex Development

This test suite integrates with the AGCodex Phase 6 development workflow:

1. **Pre-commit**: Run unit tests and formatting checks
2. **CI/CD**: Comprehensive testing on all changes
3. **Performance monitoring**: Continuous benchmark regression detection
4. **Quality gates**: Coverage and performance thresholds enforced
5. **Rebranding verification**: Automated checking of migration progress

The test suite ensures that AGCodex meets all Phase 6 requirements while maintaining high code quality and performance standards.