# TreeTool Test Suite

This directory contains comprehensive tests for the TreeTool component of AGCodex.

## Test Files Overview

### 1. `tree_test.rs` - Comprehensive Functional Tests

The main comprehensive test suite covering:

#### Multi-Language Parsing Tests
- **Rust parsing**: Complex user management system with structs, enums, implementations
- **Python parsing**: Task management with async support, dataclasses, type hints
- **JavaScript parsing**: Event-driven component with classes and async methods
- **TypeScript parsing**: Type-safe configuration management with generics
- **Go parsing**: Concurrent worker pool with channels and goroutines

#### Performance Testing (<10ms target)
- Parse time benchmarks for all supported languages
- Cache performance testing (first parse vs cached parse)
- Large file handling (10x repeated code samples)
- Query execution performance (<50ms target)

#### Query Functionality Tests
- Language-specific query patterns for function extraction
- Pattern matching across multiple languages
- Query compilation and execution validation
- Error handling for invalid query patterns

#### Error Recovery Tests
- **Syntax error handling**: Malformed Rust, Python, JavaScript code samples
- **Parse error reporting**: Error count and location tracking
- **Graceful degradation**: Parser continues despite syntax errors

#### Symbol Extraction Tests
- Function, class, method, and variable extraction
- Language-specific symbol recognition patterns
- Symbol metadata (name, type, location, visibility)

#### Semantic Diff Testing
- Code change detection between versions
- Addition, modification, deletion tracking
- Similarity scoring algorithms
- File-based diff operations

#### Cache Effectiveness Tests
- Cache hit rate validation (>80% target)
- TTL (Time-To-Live) functionality testing
- Cache clearing and statistics
- Memory usage optimization

#### Language Auto-Detection Tests
- File extension mapping for 27+ languages
- Extension uniqueness validation
- Common file type recognition

### 2. `tree_test_focused.rs` - Lightweight Tests

Focused tests that avoid parser initialization:
- SupportedLanguage enum testing
- Point structure validation
- Basic functionality without actual parsing
- Performance expectation validation

### 3. `tree_interface_test.rs` - Interface Tests

Tests for public types and enums without parser dependencies:
- TreeInput/TreeOutput variant testing  
- Language extension mapping
- Enum completeness validation
- Interface structure testing

## Test Coverage Areas

### ✅ Implemented Test Categories

1. **Language Support Testing**
   - All 27 supported languages validated
   - Extension mappings verified
   - Language detection accuracy

2. **Performance Benchmarking**
   - Parse time validation (<10ms per file)
   - Query execution speed (<50ms)
   - Cache lookup performance (<1ms)
   - Memory usage tracking

3. **Functional Testing**
   - AST parsing accuracy
   - Query pattern matching
   - Symbol extraction
   - Semantic diffing
   - Error recovery

4. **Cache Testing**
   - Hit rate optimization
   - TTL management
   - Statistics reporting
   - Memory efficiency

5. **Edge Case Handling**
   - Empty code handling
   - Invalid query patterns
   - Unsupported languages
   - Malformed syntax

### 🔧 Test Sample Code

The tests include realistic code samples in multiple languages:

- **Rust**: User management system (166 lines) with advanced features
- **Python**: Task manager with async/await (91 lines) 
- **JavaScript**: Event manager class (135 lines)
- **TypeScript**: Generic configuration manager (245 lines)
- **Go**: Worker pool with channels (139 lines)
- **Error samples**: Intentionally broken syntax for error recovery testing

## Current Status

### ⚠️ Known Issues

**Tree-sitter-latex Linking Problem**: 
Currently, tests cannot run due to undefined references in tree-sitter-latex:
```
undefined reference to `tree_sitter_latex_external_scanner_create'
undefined reference to `tree_sitter_latex_external_scanner_destroy'
...
```

This is a known compatibility issue mentioned in the tree tool implementation with TODO comments.

### 🏃 Running Tests (When Fixed)

Once the linking issues are resolved, tests can be run with:

```bash
# Run all tree tool tests
cargo test tree_test --no-fail-fast

# Run specific test categories
cargo test test_rust_parsing_and_performance
cargo test test_query_functionality_rust
cargo test test_cache_effectiveness
cargo test benchmark_parsing_performance

# Run interface tests (lightweight)
cargo test tree_interface_test
```

## Performance Targets

The tests validate these performance expectations:

| Operation | Target | Test Validation |
|-----------|--------|----------------|
| File parsing | <10ms | ✅ Per-language benchmarks |
| Query execution | <50ms | ✅ Query performance tests |
| Cache lookup | <1ms | ✅ Cache hit testing |
| Symbol extraction | <100ms | ✅ Extraction timing |
| Diff computation | <100ms | ✅ Diff performance |

## Intelligence Level Testing

Tests validate different intelligence levels:

- **Light**: 100 cache entries, 5min TTL
- **Medium**: 500 cache entries, 15min TTL  
- **Hard**: 2000 cache entries, 30min TTL

## Future Enhancements

Once core tests are working:

1. **Integration Tests**: File-based parsing with real project files
2. **Stress Testing**: Very large files (1MB+) and concurrent parsing
3. **Memory Leak Testing**: Long-running parsing sessions
4. **Query Library Testing**: Pre-built query patterns for common use cases
5. **Custom Grammar Testing**: User-defined tree-sitter grammars

## File Structure

```
tests/tools/
├── tree_test.rs              # Main comprehensive test suite
├── tree_test_focused.rs      # Lightweight focused tests  
├── tree_interface_test.rs    # Interface/type tests only
├── tree_test_README.md       # This documentation
└── mod.rs                    # Module declarations
```

## Contributing

When adding new tree tool tests:

1. Follow the existing test patterns (Arrange-Act-Assert)
2. Include performance assertions with clear targets
3. Test both success and error cases
4. Add realistic code samples for new languages
5. Validate cache behavior for new operations
6. Document any new test categories in this README

## Dependencies Resolution

To fix the current linking issues:

1. **Remove problematic languages**: Comment out or conditionally compile languages with linking issues
2. **Update Cargo.toml**: Ensure compatible tree-sitter crate versions
3. **Platform-specific fixes**: May require different approaches for Linux/macOS/Windows
4. **Alternative parsers**: Consider fallback parsers for problematic languages

The test suite is comprehensive and ready to validate all TreeTool functionality once the tree-sitter dependency issues are resolved.