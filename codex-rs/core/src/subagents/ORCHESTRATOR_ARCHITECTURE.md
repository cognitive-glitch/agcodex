# Agent Orchestrator Architecture

## Overview

The Agent Orchestrator is the core engine for managing and coordinating subagent execution in AGCodex. It provides sophisticated execution strategies, error handling, and resource management for complex agent workflows.

## Core Components

### 1. **AgentOrchestrator**
The main orchestration engine that manages:
- Agent lifecycle (spawning, monitoring, termination)
- Execution strategies (sequential, parallel, mixed)
- Resource allocation and concurrency limits
- Error handling and recovery
- Progress tracking and reporting

### 2. **SharedContext**
Thread-safe context for data sharing between agents:
- Key-value store for passing data
- Output accumulation from previous agents
- File modification tracking
- Error collection for partial results
- Snapshot and restore capabilities
- Context merging for parallel results

### 3. **ExecutionPlan**
Defines how agents should be executed:
- **Single**: Execute one agent
- **Sequential**: Execute agents one after another with optional output chaining
- **Parallel**: Execute multiple agents concurrently
- **Mixed**: Complex patterns combining sequential and parallel execution

### 4. **OrchestratorConfig**
Comprehensive configuration options:
```rust
pub struct OrchestratorConfig {
    pub max_concurrency: usize,              // Max concurrent agents (default: 8)
    pub agent_timeout: Duration,             // Per-agent timeout (default: 5 min)
    pub enable_retries: bool,                // Retry transient failures
    pub max_retries: u32,                    // Max retry attempts (default: 3)
    pub retry_backoff: Duration,             // Backoff between retries
    pub enable_circuit_breaker: bool,        // Circuit breaker pattern
    pub circuit_breaker_threshold: u32,      // Failures before opening
    pub circuit_breaker_reset: Duration,     // Reset duration
    pub monitor_memory: bool,                // Memory pressure monitoring
    pub memory_threshold_mb: usize,          // Memory limit
}
```

## Execution Strategies

### Sequential Execution
```
Agent A → Agent B → Agent C
```
- Agents execute one after another
- Output can be passed from one agent to the next
- Useful for dependent operations

### Parallel Execution
```
Agent A ─┐
Agent B ─┼→ Merge Results
Agent C ─┘
```
- Multiple agents execute simultaneously
- Results are collected and merged
- Ideal for independent operations

### Mixed Execution
```
Agent A → (Agent B + Agent C) → Agent D
```
- Combines sequential and parallel patterns
- Supports complex workflows with barriers
- Maximizes efficiency for complex dependencies

### Conditional Execution
```rust
orchestrator.execute_conditional(
    agent_invocation,
    &context,
    |ctx| Box::pin(async move {
        // Condition logic
        ctx.get("should_run").await == Some(true)
    })
)
```
- Execute agents based on runtime conditions
- Supports dynamic workflow adaptation

## Error Handling

### Retry Logic
- Automatic retry for transient failures
- Exponential backoff between attempts
- Configurable max retry attempts

### Circuit Breaker Pattern
- Prevents cascading failures
- Opens after threshold consecutive failures
- Auto-resets after cool-down period
- Per-agent circuit breakers

### Partial Results
- Continues execution despite individual failures
- Collects successful results
- Reports both success and partial success states

## Resource Management

### Concurrency Control
- Semaphore-based concurrency limiting
- Prevents resource exhaustion
- Fair scheduling across agents

### Memory Monitoring
- Tracks memory pressure
- Prevents out-of-memory conditions
- Graceful degradation under pressure

### Cancellation Support
- Propagates cancellation to all active agents
- Clean resource cleanup
- Preserves partial results

## Progress Tracking

### Real-time Updates
```rust
pub struct ProgressUpdate {
    pub execution_id: Uuid,
    pub agent_name: String,
    pub status: SubagentStatus,
    pub message: Option<String>,
    pub progress_percentage: Option<u8>,
    pub timestamp: SystemTime,
}
```

### Status Transitions
```
Pending → Running → Completed
                 ↘ Failed
                 ↘ Cancelled
```

## Context Management

### Data Sharing
```rust
// Set data
context.set("key", json!(value)).await;

// Get data
let value = context.get("key").await;

// Add output for chaining
context.add_output(output).await;

// Track modified files
context.add_modified_files(files).await;
```

### Context Snapshots
```rust
// Create snapshot
let snapshot = context.snapshot().await;

// Modify context
context.set("temp", json!(data)).await;

// Restore to snapshot
context.restore(snapshot).await;
```

### Context Merging
```rust
// Merge results from parallel agents
context.merge(&other_context).await;
```

## Usage Examples

### Simple Sequential Chain
```rust
let request = InvocationRequest {
    execution_plan: ExecutionPlan::Sequential(AgentChain {
        agents: vec![
            refactorer_agent,
            test_writer_agent,
            docs_agent,
        ],
        pass_output: true,
    }),
    // ...
};

let result = orchestrator.execute_plan(request).await?;
```

### Parallel Analysis
```rust
let request = InvocationRequest {
    execution_plan: ExecutionPlan::Parallel(vec![
        performance_agent,
        security_agent,
        code_review_agent,
    ]),
    // ...
};

let result = orchestrator.execute_plan(request).await?;
```

### Complex Workflow
```rust
let request = InvocationRequest {
    execution_plan: ExecutionPlan::Mixed(vec![
        ExecutionStep::Single(analyzer),
        ExecutionStep::Barrier,
        ExecutionStep::Parallel(vec![refactorer, optimizer]),
        ExecutionStep::Barrier,
        ExecutionStep::Parallel(vec![test_writer, docs]),
    ]),
    // ...
};

let result = orchestrator.execute_plan(request).await?;
```

## Performance Characteristics

### Concurrency
- Default: 8 concurrent agents
- Configurable based on system resources
- Fair scheduling prevents starvation

### Timeouts
- Default: 5 minutes per agent
- Configurable per-agent overrides
- Graceful timeout handling

### Memory Usage
- Shared context minimizes duplication
- Lazy loading of agent outputs
- Configurable memory thresholds

## Integration Points

### With TUI
- Progress updates displayed in real-time
- Visual representation of execution flow
- Interactive cancellation support

### With Operating Modes
- Respects mode restrictions (Plan/Build/Review)
- Agents can override operating mode
- Mode-aware resource allocation

### With MCP Protocol
- Agents can be MCP tools
- Supports remote agent execution
- Tool discovery and invocation

## Future Enhancements

### Planned Features
1. **Git Worktree Integration**: Isolated execution environments
2. **Agent Pools**: Pre-warmed agent instances
3. **Distributed Execution**: Multi-machine orchestration
4. **Smart Scheduling**: ML-based execution optimization
5. **Checkpoint/Resume**: Long-running workflow persistence
6. **Event Sourcing**: Complete execution history
7. **Metrics & Telemetry**: Performance monitoring
8. **Dynamic Agent Loading**: Hot-reload agent configurations

### Extension Points
- Custom execution strategies
- Plugin-based error handlers
- External resource managers
- Custom progress reporters

## Security Considerations

### Sandboxing
- Agents execute in sandboxed environments
- Resource limits enforced
- File system access control

### Authentication
- Per-agent credentials
- Secure context passing
- Audit logging

### Isolation
- Agent outputs sanitized
- Context validation
- Prevention of data leakage

## Debugging Support

### Logging
- Detailed execution traces
- Error context preservation
- Performance metrics

### Inspection
- Context snapshots for debugging
- Execution replay capability
- Step-through debugging support

## Best Practices

### Workflow Design
1. Keep individual agents focused and single-purpose
2. Use parallel execution for independent operations
3. Implement proper error handling at agent level
4. Design for idempotency where possible
5. Use context snapshots for checkpointing

### Performance Optimization
1. Batch similar operations
2. Use appropriate concurrency limits
3. Implement caching at agent level
4. Monitor memory usage
5. Profile and optimize hot paths

### Error Recovery
1. Use retry logic for transient failures
2. Implement circuit breakers for external dependencies
3. Design for partial results
4. Log detailed error context
5. Provide fallback strategies