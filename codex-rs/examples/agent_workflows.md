# Agent Workflows Guide

Master AGCodex's powerful multi-agent system with practical examples and real-world workflows.

## 📖 Table of Contents
1. [Understanding Agents](#understanding-agents)
2. [Built-in Agents](#built-in-agents)
3. [Single Agent Examples](#single-agent-examples)
4. [Multi-Agent Workflows](#multi-agent-workflows)
5. [Custom Agent Pipelines](#custom-agent-pipelines)
6. [Best Practices](#best-practices)

## Understanding Agents

Agents are specialized AI assistants that excel at specific tasks. They can work independently or collaborate in complex pipelines.

### Agent Invocation Syntax
```bash
@agent-name [command] [options]
```

### Available Agents
```bash
> list agents

Available agents:
  @code-reviewer    - Code quality and security analysis
  @refactorer      - Code refactoring and optimization
  @test-writer     - Test generation and coverage
  @performance     - Performance analysis and optimization
  @security        - Security audit and vulnerability detection
  @debugger        - Bug detection and fix suggestions
  @docs            - Documentation generation
  @architect       - System design and architecture
```

## Built-in Agents

### @code-reviewer
**Purpose:** Comprehensive code review with actionable feedback
```bash
> @code-reviewer analyze src/auth.rs

📋 Code Review Report for src/auth.rs
═══════════════════════════════════════

Security Issues (2):
  🔴 Line 45: Potential SQL injection vulnerability
     - Risk: HIGH
     - Fix: Use parameterized queries
     
  🟡 Line 78: Hardcoded secret key
     - Risk: MEDIUM
     - Fix: Move to environment variables

Code Quality (3):
  🟡 Line 23-67: Function exceeds 40 lines
     - Complexity: 12 (threshold: 10)
     - Suggestion: Extract validation logic
     
  🟡 Line 89: Missing error handling
     - Impact: Potential panic
     - Fix: Add Result return type

Performance (1):
  🟡 Line 112: O(n²) algorithm detected
     - Current: Nested loops over users
     - Suggestion: Use HashMap for O(n) lookup

Best Practices (2):
  ℹ️ Missing documentation for public functions
  ℹ️ Consider adding unit tests

Summary: 8 issues found (1 high, 4 medium, 3 info)
```

### @refactorer
**Purpose:** Systematic code refactoring with safety checks
```bash
> @refactorer reduce duplication in controllers/

🔨 Refactoring Analysis for controllers/
═══════════════════════════════════════════

Duplication Found:
  📁 user_controller.rs & admin_controller.rs
     - Lines 45-72 (validation logic)
     - Similarity: 87%
     - Recommendation: Extract to validation module

  📁 api_controller.rs (internal)
     - Lines 23-35, 89-101, 156-168
     - Pattern: Error handling boilerplate
     - Recommendation: Create error handling macro

Refactoring Plan:
  1. Create shared/validation.rs module
  2. Extract common validation functions
  3. Create error_handler! macro
  4. Update controllers to use shared code
  5. Run tests after each step

Would you like me to:
  [1] Show detailed refactoring for validation
  [2] Apply all refactorings
  [3] Create step-by-step branch plan
```

### @test-writer
**Purpose:** Generate comprehensive test suites
```bash
> @test-writer generate tests for src/calculator.rs

🧪 Test Generation for calculator.rs
══════════════════════════════════════

Analyzing code structure...
  ✓ Found 6 public functions
  ✓ Found 3 private helpers
  ✓ Current coverage: 45%

Generated Test Suite:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Basic functionality tests
    #[test]
    fn test_add_positive_numbers() {
        assert_eq!(add(2, 3), 5);
        assert_eq!(add(0, 0), 0);
        assert_eq!(add(100, 200), 300);
    }

    #[test]
    fn test_add_negative_numbers() {
        assert_eq!(add(-5, -3), -8);
        assert_eq!(add(-10, 5), -5);
    }

    // Edge cases
    #[test]
    fn test_add_overflow() {
        let result = add(i32::MAX, 1);
        assert!(result.is_err());
    }

    // Property-based tests
    #[quickcheck]
    fn prop_add_commutative(a: i32, b: i32) -> bool {
        add(a, b) == add(b, a)
    }

    #[quickcheck]
    fn prop_add_associative(a: i32, b: i32, c: i32) -> bool {
        add(add(a, b), c) == add(a, add(b, c))
    }

    // Error scenarios
    #[test]
    #[should_panic(expected = "division by zero")]
    fn test_divide_by_zero() {
        divide(10, 0);
    }
}
```

Generated:
  ✓ 6 unit tests
  ✓ 3 edge case tests
  ✓ 2 property tests
  ✓ 2 error tests

Estimated coverage after: 92%
```

### @performance
**Purpose:** Performance profiling and optimization
```bash
> @performance analyze hot paths in src/

⚡ Performance Analysis Report
════════════════════════════════

Hotspots Identified:
  
  🔥 src/data_processor.rs::process_batch (45% CPU)
    - Current: O(n²) with nested iterations
    - Bottleneck: Line 234-267
    - Memory: 450MB allocations per call
    - Suggestion: Use HashMap for O(n) lookup
    - Potential improvement: 10-15x faster

  🟡 src/api/search.rs::fuzzy_search (23% CPU)
    - Current: Linear scan of all records
    - Bottleneck: No indexing
    - Suggestion: Implement trie or BK-tree
    - Potential improvement: 5-8x faster

  🟡 src/cache.rs::invalidate (12% CPU)
    - Current: Full cache scan
    - Issue: Lock contention
    - Suggestion: Sharded locks
    - Potential improvement: 3x faster

Memory Issues:
  📊 Excessive allocations in render_template
    - 10,000 allocations per request
    - Suggestion: Use object pool

Optimization Plan:
  1. Refactor process_batch to use HashMap
  2. Add indexing to search functionality  
  3. Implement sharded cache locks
  4. Add object pooling for templates

Run optimizations? [y/n]
```

### @security
**Purpose:** Security vulnerability detection and remediation
```bash
> @security audit full codebase

🔒 Security Audit Report
═════════════════════════

Critical Vulnerabilities (2):
  
  🚨 SQL Injection [CWE-89]
    File: src/db/queries.rs:45
    ```rust
    let query = format!("SELECT * FROM users WHERE id = {}", user_id);
    ```
    Fix: Use prepared statements
    
  🚨 Path Traversal [CWE-22]
    File: src/file_handler.rs:78
    ```rust
    let path = format!("./uploads/{}", user_input);
    ```
    Fix: Sanitize and validate paths

High Risk (3):
  
  ⚠️ Weak Cryptography [CWE-326]
    File: src/auth.rs:92
    Using MD5 for password hashing
    Fix: Use bcrypt or argon2
    
  ⚠️ Missing Authentication [CWE-306]
    File: src/api/admin.rs:23
    Admin endpoints lack auth checks
    Fix: Add authentication middleware

Medium Risk (5):
  
  ⚡ Unvalidated Input [CWE-20]
    Multiple locations lacking input validation
    
  ⚡ Information Disclosure [CWE-200]
    Detailed error messages exposed to users

Recommendations:
  1. Immediate: Fix critical SQL injection
  2. Urgent: Update password hashing
  3. Important: Add authentication checks
  4. Suggested: Implement input validation

Generate fixes? [y/n]
```

## Single Agent Examples

### Example 1: Comprehensive Code Review
```bash
> @code-reviewer analyze the entire API module with focus on security

< Starting comprehensive review of API module...

📊 Analysis Complete: 15 files, 3,847 lines

Security Review:
  ✅ Authentication properly implemented
  ✅ Rate limiting in place
  ⚠️ 2 endpoints missing authorization checks
  ⚠️ Input validation incomplete in 3 handlers
  🔴 Potential XSS vulnerability in error responses

Code Quality:
  ✅ Consistent error handling pattern
  ✅ Good separation of concerns
  ⚠️ 5 functions exceed complexity threshold
  ℹ️ Consider extracting business logic from handlers

Testing:
  Current coverage: 67%
  Missing tests for error scenarios
  No integration tests for auth flow

[Generating detailed report with line-by-line annotations...]
```

### Example 2: Intelligent Refactoring
```bash
> @refactorer extract common patterns from all service classes

< Analyzing service classes for common patterns...

Found 8 service classes with repeated patterns:

1. Logger initialization (8 occurrences)
2. Database connection setup (8 occurrences)  
3. Error handling wrapper (24 occurrences)
4. Retry logic (12 occurrences)
5. Transaction management (15 occurrences)

Refactoring proposal:
  Creating base_service.rs with:
  - BaseService trait
  - Common initialization
  - Error handling macros
  - Transaction wrapper
  - Retry decorator

[Showing refactored code structure...]

Apply refactoring? This will:
  - Create 1 new file
  - Modify 8 service files
  - Reduce code by ~340 lines
  - Improve maintainability score by 23%
```

### Example 3: Smart Test Generation
```bash
> @test-writer create integration tests for the payment flow

< Analyzing payment flow for test scenarios...

Identified test scenarios:
  1. Happy path: successful payment
  2. Insufficient funds
  3. Invalid card details
  4. Network timeout
  5. Partial refund
  6. Full refund
  7. Concurrent payments
  8. Payment with discounts

Generating integration test suite...

[Shows comprehensive test file with mocks, fixtures, and assertions]

Additional recommendations:
  - Add performance benchmarks for payment processing
  - Create chaos tests for network failures
  - Add property-based tests for calculation logic
```

## Multi-Agent Workflows

### Workflow 1: Feature Development Pipeline
```bash
> @architect design user notification system then @test-writer create tests then @code-reviewer validate

< Starting multi-agent pipeline...

[Agent 1: @architect]
Designing notification system architecture...
  ✓ Created component diagram
  ✓ Defined interfaces
  ✓ Specified data flow
  ✓ Documented API contracts

[Agent 2: @test-writer]  
Generating tests based on architecture...
  ✓ Created 24 unit tests
  ✓ Created 8 integration tests
  ✓ Added performance benchmarks
  ✓ Included edge cases

[Agent 3: @code-reviewer]
Validating design and tests...
  ✓ Architecture follows SOLID principles
  ✓ Tests cover critical paths
  ⚠️ Missing tests for error recovery
  ℹ️ Consider adding circuit breaker pattern

Pipeline complete. Results saved to feature/notifications branch.
```

### Workflow 2: Bug Fix Pipeline
```bash
> @debugger find root cause of memory leak then @refactorer fix it then @test-writer add regression tests

< Executing bug fix pipeline...

[Agent 1: @debugger]
Analyzing memory usage patterns...
  Found leak in websocket_handler.rs:
  - Connections not properly closed
  - Event listeners not removed
  - Growing HashMap without cleanup
  Root cause: Missing cleanup in disconnect handler

[Agent 2: @refactorer]
Applying fix...
  ✓ Added connection cleanup
  ✓ Implemented weak references
  ✓ Added periodic cleanup task
  ✓ Fixed event listener lifecycle

[Agent 3: @test-writer]
Creating regression tests...
  ✓ Memory leak detection test
  ✓ Connection limit test
  ✓ Cleanup verification test
  ✓ Performance regression test

Memory leak fixed. PR ready for review.
```

### Workflow 3: Performance Optimization Pipeline
```bash
> Run performance pipeline: @performance profile -> @refactorer optimize -> @test-writer benchmark

< Starting performance optimization pipeline...

[Stage 1: Profiling]
@performance: Identifying bottlenecks...
  - API response time: 450ms (target: 200ms)
  - Database queries: 65% of time
  - JSON serialization: 20% of time
  - Business logic: 15% of time

[Stage 2: Optimization]
@refactorer: Applying optimizations...
  ✓ Added database query caching
  ✓ Implemented query batching
  ✓ Optimized JSON serialization
  ✓ Parallelized independent operations

[Stage 3: Benchmarking]
@test-writer: Creating benchmark suite...
  ✓ Load test: 200 req/s → 850 req/s
  ✓ Response time: 450ms → 125ms
  ✓ Memory usage: -30%
  ✓ CPU usage: -45%

Performance improved by 3.6x. All tests passing.
```

## Custom Agent Pipelines

### Creating Complex Pipelines
```bash
> Create deployment pipeline: security audit -> test coverage -> performance check -> deploy

< Building custom deployment pipeline...

Pipeline Configuration:
```yaml
name: deployment-pipeline
stages:
  - name: security-audit
    agent: "@security"
    config:
      severity: "medium"
      fail-on: "critical"
    
  - name: test-coverage
    agent: "@test-writer"
    config:
      minimum-coverage: 80
      generate-missing: true
    
  - name: performance-check
    agent: "@performance"
    config:
      benchmarks: ["api", "database", "cache"]
      regression-threshold: 10
    
  - name: deployment
    agent: "@deploy"
    config:
      environment: "staging"
      rollback-on-failure: true
```

Execute pipeline? [y/n]: y

[Running Stage 1/4: Security Audit]
  ✓ No critical vulnerabilities
  ⚠️ 2 medium issues (acceptable)

[Running Stage 2/4: Test Coverage]
  ✓ Coverage: 84% (target: 80%)
  ✓ Generated 12 missing tests

[Running Stage 3/4: Performance Check]
  ✓ API: 125ms (baseline: 130ms)
  ✓ Database: 45ms (baseline: 50ms)
  ✓ Cache: 2ms (baseline: 2ms)

[Running Stage 4/4: Deployment]
  ✓ Deployed to staging
  ✓ Health checks passing
  ✓ Smoke tests complete

Pipeline successful! Deployment complete.
```

### Conditional Agent Execution
```bash
> If tests fail, run @debugger, else run @performance

< Evaluating conditional pipeline...

Running tests...
  ✗ 3 tests failing

Condition met: tests failing
Executing @debugger...

Found test failures:
  1. test_user_authentication: Mock not properly configured
  2. test_data_validation: Regex pattern incorrect
  3. test_api_response: Expected status 200, got 401

Suggested fixes:
  [Showing fixes for each test...]

Apply fixes and rerun? [y/n]
```

### Parallel Agent Execution
```bash
> Run in parallel: @code-reviewer on src/, @security on api/, @performance on database/

< Executing 3 agents in parallel...

[Parallel Execution Starting]
  Thread 1: @code-reviewer analyzing src/
  Thread 2: @security auditing api/
  Thread 3: @performance profiling database/

[Results arriving...]

Thread 2 complete (4.2s): @security
  ✓ No critical vulnerabilities
  ⚠️ 3 medium risks identified

Thread 3 complete (5.1s): @performance  
  ✓ Query optimization opportunities found
  ✓ Index recommendations generated

Thread 1 complete (6.3s): @code-reviewer
  ✓ Code quality score: 8.5/10
  ℹ️ 15 suggestions for improvement

All parallel tasks complete. Aggregating results...
```

## Best Practices

### 1. Agent Selection
Choose the right agent for the task:
- **@code-reviewer** for quality checks
- **@refactorer** for code improvements
- **@test-writer** for test generation
- **@performance** for optimization
- **@security** for vulnerability scanning
- **@debugger** for troubleshooting
- **@docs** for documentation
- **@architect** for system design

### 2. Pipeline Design Principles

**Sequential for Dependencies:**
```bash
# When order matters
> @architect design -> @code-reviewer validate -> implement
```

**Parallel for Independence:**
```bash
# When tasks don't depend on each other
> parallel: @security scan, @performance profile, @test-writer generate
```

**Conditional for Flexibility:**
```bash
# When you need branching logic
> if @test-writer coverage < 80% then @test-writer generate more
```

### 3. Agent Configuration

**Mode Override:**
```bash
# Force read-only for analysis
> @code-reviewer --mode=review analyze src/

# Allow writes for fixes
> @refactorer --mode=build apply fixes
```

**Intelligence Levels:**
```bash
# Light analysis for quick checks
> @code-reviewer --intelligence=light quick scan

# Deep analysis for thorough review
> @security --intelligence=hard full audit
```

### 4. Error Handling in Pipelines

```bash
> Create safe pipeline with rollback: 
  try: 
    @refactorer optimize
    @test-writer verify
    @deploy staging
  catch:
    @debugger analyze failure
    rollback changes
```

### 5. Agent Communication

Agents can pass context between each other:
```bash
> @architect design system | 
  @test-writer create tests based on design |
  @code-reviewer validate both
```

### 6. Custom Agent Creation

Create specialized agents for your workflow:
```yaml
# ~/.agcodex/agents/api-specialist.yaml
name: api-specialist
description: "Expert in REST API development"
mode_override: build
intelligence: hard
tools:
  - search
  - edit
  - tree
  - grep
prompts:
  system: "You are an expert in REST API design..."
  analysis: "Analyze APIs for RESTful compliance..."
capabilities:
  - OpenAPI spec generation
  - Route optimization
  - Middleware configuration
```

## Troubleshooting

### Agent Not Found
```bash
> @unknown-agent analyze

Error: Agent 'unknown-agent' not found

> list agents  # Show available agents
> agcodex agents install unknown-agent  # Install if available
```

### Pipeline Timeout
```bash
# Set timeout for long-running pipelines
> with timeout 5m: @performance deep profile

# Or configure in pipeline
timeout: 300  # seconds
```

### Agent Conflicts
```bash
# When agents try to modify same files
> sequential: @refactorer cleanup, @test-writer update
# Instead of parallel execution
```

### Memory Issues with Large Codebases
```bash
# Use incremental processing
> @code-reviewer analyze src/ --incremental --batch-size=10
```

## Advanced Examples

### Multi-Stage Refactoring
```bash
> Execute refactoring stages:
  Stage 1: @refactorer identify code smells
  Stage 2: @architect propose solutions  
  Stage 3: @refactorer apply approved changes
  Stage 4: @test-writer update tests
  Stage 5: @code-reviewer final validation
```

### Continuous Integration Pipeline
```bash
> CI pipeline for PR:
  - @security scan for vulnerabilities
  - @code-reviewer check style and quality
  - @test-writer ensure 90% coverage
  - @performance verify no regressions
  - @docs update if APIs changed
```

### AI-Driven Development
```bash
> Implement feature "user notifications":
  @architect: Design the system
  @test-writer: Create test specs (TDD)
  @developer: Implement based on tests
  @refactorer: Optimize implementation
  @security: Verify security
  @docs: Generate documentation
```

## Next Steps

1. Explore [Advanced Features](advanced_features.md) for complex workflows
2. Create [Custom Agents](custom_agents/) for your specific needs
3. Review [Configuration Templates](configuration_templates/) for optimization
4. Check the [API Reference](../docs/API.md) for programmatic access

---

**Quick Agent Reference:**
- `@agent-name` - Invoke agent
- `list agents` - Show available agents
- `@agent --help` - Agent-specific help
- `parallel:` - Run agents in parallel
- `sequential:` - Run agents in sequence
- `|` - Pipe output between agents