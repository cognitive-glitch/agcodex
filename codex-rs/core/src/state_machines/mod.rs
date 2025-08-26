//! State machine patterns with compile-time state validation.
//!
//! This module provides typestate patterns that enforce correct state transitions
//! at compile time, preventing invalid operations and ensuring system integrity.

pub mod context_manager;
pub mod file_processor;
pub mod search_engine;

#[cfg(test)]
pub mod examples;

pub use context_manager::*;
pub use file_processor::*;
pub use search_engine::*;

use serde::Deserialize;
use serde::Serialize;
use std::marker::PhantomData;

/// Trait for state machine states
pub trait StateMachineState: 'static {}

/// Common state machine error types
#[derive(Debug, thiserror::Error)]
pub enum StateMachineError {
    #[error("Invalid state transition from {from} to {to}")]
    InvalidTransition {
        from: &'static str,
        to: &'static str,
    },

    #[error("Operation not allowed in state {state}")]
    OperationNotAllowed { state: &'static str },

    #[error("State machine not initialized")]
    NotInitialized,

    #[error("State machine already finalized")]
    AlreadyFinalized,

    #[error("Precondition failed: {condition}")]
    PreconditionFailed { condition: String },

    #[error("Resource not available in current state")]
    ResourceUnavailable,

    #[error("State validation error: {0}")]
    ValidationError(#[from] crate::types::TypeValidationError),
}

/// Result type for state machine operations
pub type StateMachineResult<T> = Result<T, StateMachineError>;

/// Base state machine trait with type-safe transitions
pub trait StateMachine<S: StateMachineState> {
    type Error;

    /// Get current state name for debugging
    fn state_name(&self) -> &'static str;

    /// Validate current state integrity
    fn validate_state(&self) -> Result<(), Self::Error>;

    /// Check if a transition is valid (used in debug builds)
    fn can_transition_to(&self, target_state: &'static str) -> bool;
}

/// Macro for defining state machine states with automatic trait implementations
#[macro_export]
macro_rules! define_states {
    ($($state:ident),* $(,)?) => {
        $(
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
            pub struct $state;

            impl StateMachineState for $state {}

            impl $state {
                pub const fn name() -> &'static str {
                    stringify!($state)
                }
            }
        )*
    };
}

// Common states used across different state machines
define_states! {
    Uninitialized,
    Initializing,
    Ready,
    Processing,
    Completed,
    Failed,
    Suspended,
    Terminating,
}

/// State transition tracker for debugging and logging
#[derive(Debug, Clone)]
pub struct StateTransition {
    pub from_state: &'static str,
    pub to_state: &'static str,
    pub timestamp: std::time::SystemTime,
    pub metadata: std::collections::HashMap<String, String>,
}

impl StateTransition {
    pub fn new(from: &'static str, to: &'static str) -> Self {
        Self {
            from_state: from,
            to_state: to,
            timestamp: std::time::SystemTime::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// State machine with transition history and validation
#[derive(Debug)]
pub struct TrackedStateMachine<S: StateMachineState> {
    _state: PhantomData<S>,
    transitions: Vec<StateTransition>,
    created_at: std::time::SystemTime,
}

impl<S: StateMachineState> TrackedStateMachine<S> {
    pub fn new() -> Self {
        Self {
            _state: PhantomData,
            transitions: Vec::new(),
            created_at: std::time::SystemTime::now(),
        }
    }

    pub fn record_transition(&mut self, transition: StateTransition) {
        self.transitions.push(transition);
    }

    pub fn transition_history(&self) -> &[StateTransition] {
        &self.transitions
    }

    pub const fn created_at(&self) -> std::time::SystemTime {
        self.created_at
    }

    pub fn uptime(&self) -> Result<std::time::Duration, std::time::SystemTimeError> {
        std::time::SystemTime::now().duration_since(self.created_at)
    }

    /// Get the most recent transition
    pub fn last_transition(&self) -> Option<&StateTransition> {
        self.transitions.last()
    }

    /// Count transitions to a specific state
    pub fn count_transitions_to(&self, state: &str) -> usize {
        self.transitions
            .iter()
            .filter(|t| t.to_state == state)
            .count()
    }
}

impl<S: StateMachineState> Default for TrackedStateMachine<S> {
    fn default() -> Self {
        Self::new()
    }
}

/// Compile-time state validation utilities
pub struct StateValidator;

impl StateValidator {
    /// Validate that a transition is allowed at compile time
    pub const fn validate_transition<From: StateMachineState, To: StateMachineState>() -> bool {
        // This is a compile-time check - actual validation logic would be
        // implemented per state machine
        true
    }

    /// Create a transition record
    pub fn create_transition<From: StateMachineState, To: StateMachineState>() -> StateTransition {
        StateTransition::new(std::any::type_name::<From>(), std::any::type_name::<To>())
    }
}

/// Phantom data wrapper for zero-cost state tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct StatePhantom<S: StateMachineState> {
    _phantom: PhantomData<S>,
}

impl<S: StateMachineState> StatePhantom<S> {
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<S: StateMachineState> Default for StatePhantom<S> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Test states
    define_states!(TestInit, TestProcessing, TestDone);

    #[test]
    fn test_state_names() {
        assert_eq!(TestInit::name(), "TestInit");
        assert_eq!(TestProcessing::name(), "TestProcessing");
        assert_eq!(TestDone::name(), "TestDone");
    }

    #[test]
    fn test_state_transition() {
        let transition = StateTransition::new("TestInit", "TestProcessing")
            .with_metadata("reason", "user_initiated");

        assert_eq!(transition.from_state, "TestInit");
        assert_eq!(transition.to_state, "TestProcessing");
        assert_eq!(
            transition.metadata.get("reason"),
            Some(&"user_initiated".to_string())
        );
    }

    #[test]
    fn test_tracked_state_machine() {
        let mut machine = TrackedStateMachine::<TestInit>::new();

        let transition = StateTransition::new("TestInit", "TestProcessing");
        machine.record_transition(transition);

        assert_eq!(machine.transition_history().len(), 1);
        assert_eq!(machine.count_transitions_to("TestProcessing"), 1);

        let last = machine.last_transition().unwrap();
        assert_eq!(last.from_state, "TestInit");
        assert_eq!(last.to_state, "TestProcessing");
    }

    #[test]
    fn test_state_phantom_zero_size() {
        use std::mem;

        assert_eq!(mem::size_of::<StatePhantom<TestInit>>(), 0);
        assert_eq!(mem::size_of::<StatePhantom<TestProcessing>>(), 0);

        // Ensure the phantom data doesn't add runtime cost
        let phantom1 = StatePhantom::<TestInit>::new();
        let phantom2 = StatePhantom::<TestInit>::new();
        assert_eq!(phantom1, phantom2);
    }

    #[test]
    fn test_uptime_calculation() {
        let machine = TrackedStateMachine::<TestInit>::new();

        // Should be able to calculate uptime
        let uptime = machine.uptime();
        assert!(uptime.is_ok());
        assert!(uptime.unwrap().as_nanos() > 0);
    }
}
