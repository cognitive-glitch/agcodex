# AGCodex Integration Test Suite

This directory contains comprehensive integration tests for AGCodex that test the complete workflows using real AGCodex components while mocking external API calls.

## Test Structure

### Core Integration Tests

#### `mode_switching.rs`
Tests the operating mode management system:
- Mode transitions between Plan/Build/Review modes
- Mode restriction enforcement
- Visual indicators and state persistence
- Performance characteristics
- Thread safety and concurrent access
- Error handling and validation

**Key Features Tested:**
- ✅ Basic mode cycling (Plan → Build → Review → Plan)
- ✅ Mode restrictions (Plan: read-only, Build: full access, Review: limited)
- ✅ File write validation based on mode
- ✅ Command execution permissions
- ✅ Git operation restrictions
- ✅ Rapid mode switching performance
- ✅ Concurrent mode access safety
- ✅ Visual indicator consistency

#### `agent_invocation.rs`
Tests the subagent invocation and execution system:
- @agent-name pattern detection and parsing
- Agent registry and configuration loading
- Execution plan generation (single, sequential, parallel)
- Context isolation and parameter passing
- Tool permissions and mode overrides

**Key Features Tested:**
- ✅ Agent registry loading from TOML configuration
- ✅ Simple invocation parsing (@code-reviewer, @performance-analyzer)
- ✅ Sequential agent chains (@agent1 → @agent2)
- ✅ Parallel agent execution (@agent1 + @agent2)
- ✅ Complex mixed patterns
- ✅ Parameter extraction and validation
- ✅ Tool permission enforcement
- ✅ Mode override functionality
- ✅ File pattern matching
- ✅ Tag-based agent discovery
- ✅ Performance characteristics
- ✅ Concurrent registry access

#### `session_management.rs`
Tests the session persistence and management system:
- Session creation, loading, and persistence
- Auto-save functionality
- Checkpoint creation and restoration
- Message history management
- Session state persistence

**Key Features Tested:**
- ✅ Basic session creation and metadata
- ✅ Message handling and persistence
- ✅ Session persistence across application restarts
- ✅ Checkpoint creation and restoration
- ✅ Mode switching within sessions
- ✅ Session state management (cursor, scroll, etc.)
- ✅ Auto-save functionality
- ✅ Session listing and search
- ✅ Session deletion and cleanup
- ✅ Concurrent session handling
- ✅ Performance characteristics
- ✅ Memory usage optimization
- ✅ Error recovery
- ✅ Conversation branching simulation
- ✅ Session limits and cleanup

## Test Architecture

### Real Components Used
- **`agcodex_core::modes::ModeManager`** - Real mode management logic
- **`agcodex_core::subagents`** - Real agent invocation processing
- **`agcodex_persistence::SessionManager`** - Real session persistence
- **Tree-sitter parsers** - Real AST parsing capabilities
- **File system operations** - Real file I/O through controlled test environments

### Mocked Components
- **LLM API calls** - All external API requests are mocked
- **Network requests** - HTTP/HTTPS calls use mock responses
- **System commands** - Command execution is simulated
- **User input** - Keyboard/mouse events are programmatically generated

### Test Utilities

#### `helpers/test_utils.rs`
Comprehensive test utilities including:
- `TestEnvironment` - Isolated test directories and configuration
- `MockDataGenerator` - Realistic test data generation
- `PerformanceAssertions` - Performance validation helpers
- `TestValidation` - Result validation utilities
- `AsyncTestHelpers` - Async test coordination
- `TestTiming` - Operation timing and benchmarking
- `CodeSamples` - Multi-language code samples for testing

## Running Tests

### All Integration Tests
```bash
cargo test --test lib
```

### Specific Test Modules
```bash
# Mode switching tests
cargo test --test mode_switching

# Agent invocation tests
cargo test --test agent_invocation

# Session management tests
cargo test --test session_management
```

### Specific Test Functions
```bash
# Test basic mode cycling
cargo test --test mode_switching test_basic_mode_cycling

# Test agent registry loading
cargo test --test agent_invocation test_agent_registry_loading

# Test session persistence
cargo test --test session_management test_session_persistence
```

### Performance Tests
```bash
# Run only performance-focused tests
cargo test --test lib performance

# Run with release optimizations
cargo test --release --test lib
```

### Verbose Output
```bash
# Show test output
cargo test --test lib -- --nocapture

# Show test progress
cargo test --test lib -- --show-output
```

## Performance Targets

The integration tests validate performance against these targets:

### Mode Switching
- Mode cycle operation: < 1ms per switch
- 1000 rapid mode switches: < 100ms total
- Concurrent access: 95%+ consistency

### Agent Invocation
- Invocation parsing: < 1ms per parse
- Agent registry loading: < 500ms for 20 agents
- Concurrent registry access: 95%+ consistency

### Session Management
- Session creation: < 100ms per session
- Message addition: < 10ms per message
- Session save/load: < 200ms per session
- Auto-save overhead: < 2s for 100 rapid messages

## Test Data

Tests use realistic data including:
- Multi-language code samples (Rust, Python, TypeScript, Go)
- Realistic session titles and conversation content
- Proper UUID generation for all identifiers
- Accurate timestamp handling
- Appropriate file patterns and extensions

## Error Scenarios

Tests comprehensively cover error conditions:
- Non-existent agents/sessions
- Invalid parameters and configurations
- File system permission issues
- Concurrent access race conditions
- Resource limit violations
- Network timeout simulations
- Malformed input handling

## CI/CD Integration

These tests are designed to run in CI/CD environments:
- No external dependencies required
- Isolated test environments (no shared state)
- Deterministic results (no flaky tests)
- Reasonable execution time (< 2 minutes total)
- Comprehensive error reporting
- Performance regression detection

## Maintenance

### Adding New Tests
1. Follow existing patterns in test modules
2. Use `TestEnvironment` for isolation
3. Include performance assertions where appropriate
4. Add error condition coverage
5. Update this README with new test descriptions

### Debugging Test Failures
1. Run specific failing tests with `--nocapture`
2. Check test environment setup
3. Verify mock data generation
4. Review performance assertions (may need adjustment)
5. Check for race conditions in concurrent tests

### Performance Regression
If performance tests fail:
1. Run with `--release` flag for optimized builds
2. Check system load during test execution
3. Review performance targets (may need adjustment)
4. Profile specific slow operations
5. Consider hardware differences in CI vs local

The integration test suite provides comprehensive coverage of AGCodex workflows while maintaining fast, reliable execution suitable for development and CI/CD environments.
