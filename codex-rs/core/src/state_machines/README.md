# State Machine Architecture Documentation

## Overview

This module provides typestate patterns that enforce correct state transitions at compile time, preventing invalid operations and ensuring system integrity.

## Architecture Diagrams

### State Transition Flow
```mermaid
graph TB
    subgraph "Builder State Flow"
        A[Init] -->|pattern() - Set search pattern| B[Validated]
        B -->|scope_*() - Set search scope| C[Ready] 
        C -->|build() - Create final query| D[Final Object]
    end
    
    subgraph "State Machine Flow"
        S1[Uninitialized] -->|initialize()| S2[Initializing]
        S2 -->|ready()| S3[Ready]
        S3 -->|process()| S4[Processing]
        S4 -->|complete()| S5[Completed]
        S4 -->|fail()| S6[Failed]
        S3 -->|suspend()| S7[Suspended]
        S7 -->|resume()| S3
    end

    style A fill:#ffcccc
    style B fill:#ffffcc  
    style C fill:#ccffcc
    style D fill:#ccccff
```

### Memory Layout & Ownership
```mermaid
graph LR
    subgraph "Zero-Cost State Tracking"
        M1[PhantomData&lt;Init&gt;<br/>Size: 0 bytes<br/>Cost: Zero]
        M2[PhantomData&lt;Validated&gt;<br/>Size: 0 bytes<br/>Cost: Zero]
        M3[PhantomData&lt;Ready&gt;<br/>Size: 0 bytes<br/>Cost: Zero]
    end
    
    M1 -->|pattern() - State Transition| M2
    M2 -->|scope_*() - State Transition| M3
    M3 -->|build() - Consume Self| QueryObject[Final Query Object<br/>Heap Allocated<br/>Ready for Use]

    style M1 fill:#ffeeee
    style M2 fill:#ffffee  
    style M3 fill:#eeffee
    style QueryObject fill:#eeeeff
```

## State Machine Types

### 1. Builder States
These are used in query builders to enforce correct construction order:

- **Init**: Initial state - only pattern setting allowed
- **Validated**: Pattern is set - scope setting allowed  
- **Ready**: All required fields set - building allowed

### 2. Runtime States
These are used for runtime state machine management:

- **Uninitialized**: Not yet initialized
- **Initializing**: In process of initialization
- **Ready**: Initialized and ready for operations
- **Processing**: Currently processing operations
- **Completed**: Successfully completed operations
- **Failed**: Operation failed
- **Suspended**: Temporarily suspended
- **Terminating**: In process of termination

## Implementation Examples

### Example 1: Type-Safe Query Builder

```rust
use crate::builders::{SearchQuery, SearchQueryBuilder};
use crate::types::QueryPattern;

fn example_correct_usage() -> Result<SearchQuery, BuilderError> {
    // This compiles - correct state transitions
    let query = SearchQuery::builder()
        .pattern("function")?           // Init -> Validated
        .scope_current_dir()           // Validated -> Ready
        .case_sensitive(true)          // Ready -> Ready (config)
        .build()?;                     // Ready -> SearchQuery
    
    Ok(query)
}

fn example_incorrect_usage() {
    // These won't compile due to type safety:
    
    // ❌ Can't set scope before pattern
    // let query = SearchQuery::builder()
    //     .scope_current_dir()        // ERROR: Init doesn't have scope_* methods
    //     .pattern("function");
    
    // ❌ Can't build without scope
    // let query = SearchQuery::builder()
    //     .pattern("function")?       // Init -> Validated
    //     .build();                   // ERROR: Validated doesn't have build() method
}
```

### Example 2: Runtime State Machine

```rust
use crate::state_machines::{TrackedStateMachine, StateTransition, Ready, Processing};

fn example_runtime_state_machine() {
    let mut machine = TrackedStateMachine::<Ready>::new();
    
    // Record state transitions
    let transition = StateTransition::new("Ready", "Processing")
        .with_metadata("operation", "file_processing")
        .with_metadata("user", "system");
    
    machine.record_transition(transition);
    
    // Query transition history
    println!("Transitions: {}", machine.transition_history().len());
    println!("Last transition: {:?}", machine.last_transition());
    
    // Check uptime
    if let Ok(uptime) = machine.uptime() {
        println!("Machine uptime: {:?}", uptime);
    }
}
```

### Example 3: AST Query Builder

```rust
use crate::builders::{AstQuery, AstLanguage};
use crate::types::FilePath;

fn example_ast_query() -> Result<AstQuery, BuilderError> {
    let file = FilePath::new("src/main.rs")?;
    
    let query = AstQuery::builder()
        .language(AstLanguage::Rust)    // Init -> Validated
        .add_file(file)                 // Validated -> Validated (accumulate files)
        .select_node_type("function_item") // Validated -> Ready
        .max_depth(Some(10))            // Ready -> Ready (config)
        .include_metadata(true)         // Ready -> Ready (config)
        .build()?;                      // Ready -> AstQuery
    
    Ok(query)
}
```

## Concurrency Model

### Thread Safety
All state machines are designed to be thread-safe when needed:

```rust
use std::sync::Arc;
use std::thread;

fn concurrent_state_machines() {
    let machine = Arc::new(TrackedStateMachine::<Ready>::new());
    
    // Spawn multiple threads that can safely access the machine
    let handles: Vec<_> = (0..4).map(|i| {
        let machine_clone = Arc::clone(&machine);
        thread::spawn(move || {
            // Each thread can safely read state
            println!("Thread {} - Created at: {:?}", i, machine_clone.created_at());
            
            // State transitions would be coordinated through channels or other sync primitives
        })
    }).collect();
    
    for handle in handles {
        handle.join().unwrap();
    }
}
```

## Performance Characteristics

### Zero-Cost State Tracking
- **PhantomData**: 0 bytes at runtime
- **State transitions**: Compile-time only
- **Type safety**: No runtime overhead

### Runtime State Tracking
- **Transition history**: O(n) space where n = number of transitions
- **State queries**: O(1) access time
- **Memory overhead**: ~24 bytes + transition history

## Best Practices

### 1. Use Typestate for Build-Time Safety
```rust
// ✅ Good: Enforce construction order at compile time
pub struct ConfigBuilder<S: BuilderState> {
    // fields...
    _state: PhantomData<S>,
}

// ❌ Bad: Runtime validation only
pub struct ConfigBuilder {
    is_valid: bool,  // Runtime check - can be forgotten
}
```

### 2. Meaningful State Names
```rust
// ✅ Good: Clear, descriptive states
pub struct ConnectionBuilderState;
pub struct AuthenticatedState;
pub struct ConfiguredState;

// ❌ Bad: Generic, unclear states
pub struct State1;
pub struct State2;
```

### 3. Document State Transitions
```rust
impl ConnectionBuilder<Unauthenticated> {
    /// Authenticate the connection (Unauthenticated -> Authenticated)
    /// 
    /// # Returns
    /// ConnectionBuilder in Authenticated state ready for configuration
    pub fn authenticate(self, credentials: Credentials) -> Result<ConnectionBuilder<Authenticated>, AuthError> {
        // implementation
    }
}
```

### 4. Provide Error Context
```rust
impl StateMachine<Processing> for FileProcessor {
    type Error = ProcessingError;
    
    fn validate_state(&self) -> Result<(), Self::Error> {
        if !self.has_input_files() {
            return Err(ProcessingError::NoInputFiles);
        }
        Ok(())
    }
}
```

## Testing State Machines

### Example Test Suite
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_transitions() {
        // Test valid transitions compile
        let builder = QueryBuilder::new()
            .pattern("test")
            .unwrap()
            .scope_current_dir()
            .build()
            .unwrap();
        
        assert!(builder.is_valid());
    }
    
    #[test]
    fn test_state_machine_tracking() {
        let mut machine = TrackedStateMachine::<Ready>::new();
        
        let transition = StateTransition::new("Ready", "Processing");
        machine.record_transition(transition);
        
        assert_eq!(machine.transition_history().len(), 1);
        assert_eq!(machine.count_transitions_to("Processing"), 1);
    }
    
    #[test]
    fn test_zero_cost_phantom_data() {
        use std::mem;
        
        // Verify zero runtime cost
        assert_eq!(mem::size_of::<StatePhantom<Ready>>(), 0);
        assert_eq!(mem::size_of::<StatePhantom<Processing>>(), 0);
    }
}
```

## Migration Guide

If you encounter state type mismatches, follow these steps:

### 1. Identify Current States
Check your builder implementations for the correct state names:
- Use `Init`, `Validated`, `Ready` for builders
- Use `Uninitialized`, `Ready`, `Processing`, etc. for runtime state machines

### 2. Fix State Transitions
Ensure methods return the correct state types:
```rust
// ✅ Correct
impl Builder<Init> {
    pub fn set_pattern(self) -> Builder<Validated> { /* */ }
}

impl Builder<Validated> {
    pub fn set_scope(self) -> Builder<Ready> { /* */ }
}
```

### 3. Update PhantomData Usage
Use consistent PhantomData patterns:
```rust
pub struct MyBuilder<S: BuilderState> {
    // your fields...
    _state: PhantomData<S>,
}
```

This architecture provides compile-time safety while maintaining zero runtime overhead for type states and minimal overhead for runtime state tracking.