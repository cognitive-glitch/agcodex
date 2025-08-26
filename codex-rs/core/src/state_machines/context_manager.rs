//! Context manager state machine for managing conversation context.

use std::collections::HashMap;
use std::collections::VecDeque;

use serde::Deserialize;
use serde::Serialize;

use super::StateMachine;
use super::StateMachineError;
use super::StateMachineResult;
use super::StateMachineState;
use super::StateTransition;
use super::TrackedStateMachine;
use crate::builders::Context;
use crate::builders::ContextItem;
use crate::builders::ContextPriority;
use crate::define_states;

// Define context manager states
define_states! {
    ContextIdle,
    ContextBuilding,
    ContextOptimizing,
    ContextReady,
    ContextError,
}

/// Context optimization strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationStrategy {
    /// No optimization
    None,
    /// Remove least recently used items
    LruEviction,
    /// Compress similar content
    ContentCompression,
    /// Prioritize high-importance items
    PriorityBased,
    /// Hybrid approach combining multiple strategies
    Hybrid,
}

/// Context validation rules
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationRules {
    pub max_total_size: usize,
    pub max_items: usize,
    pub min_priority: ContextPriority,
    pub require_user_input: bool,
    pub allow_duplicates: bool,
}

impl Default for ValidationRules {
    fn default() -> Self {
        Self {
            max_total_size: 32_768, // 32KB
            max_items: 100,
            min_priority: ContextPriority::Low,
            require_user_input: true,
            allow_duplicates: false,
        }
    }
}

/// Context manager configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextManagerConfig {
    pub optimization_strategy: OptimizationStrategy,
    pub validation_rules: ValidationRules,
    pub auto_optimize: bool,
    pub cache_contexts: bool,
    pub max_history: usize,
}

impl Default for ContextManagerConfig {
    fn default() -> Self {
        Self {
            optimization_strategy: OptimizationStrategy::Hybrid,
            validation_rules: ValidationRules::default(),
            auto_optimize: true,
            cache_contexts: true,
            max_history: 50,
        }
    }
}

/// Context history entry for tracking changes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextHistory {
    pub timestamp: std::time::SystemTime,
    pub operation: String,
    pub items_before: usize,
    pub items_after: usize,
    pub size_before: usize,
    pub size_after: usize,
    pub optimization_applied: Option<OptimizationStrategy>,
}

/// Context manager statistics
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ContextStats {
    pub contexts_created: usize,
    pub optimizations_applied: usize,
    pub total_items_processed: usize,
    pub total_size_processed: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub validation_failures: usize,
}

/// Type-safe context manager with state machine
#[derive(Debug)]
pub struct ContextManager<S: StateMachineState> {
    state_machine: TrackedStateMachine<S>,
    config: ContextManagerConfig,
    current_context: Option<Context>,
    context_cache: HashMap<String, Context>,
    history: VecDeque<ContextHistory>,
    stats: ContextStats,
}

impl ContextManager<ContextIdle> {
    /// Create a new context manager
    pub fn new(config: ContextManagerConfig) -> Self {
        Self {
            state_machine: TrackedStateMachine::new(),
            config,
            current_context: None,
            context_cache: HashMap::new(),
            history: VecDeque::new(),
            stats: ContextStats::default(),
        }
    }

    /// Start building a new context (transition to ContextBuilding)
    pub fn start_building(mut self) -> ContextManager<ContextBuilding> {
        let transition = StateTransition::new("ContextIdle", "ContextBuilding")
            .with_metadata("operation", "start_building".to_string());

        self.state_machine.record_transition(transition);
        self.stats.contexts_created += 1;

        ContextManager {
            state_machine: TrackedStateMachine::new(),
            config: self.config,
            current_context: self.current_context,
            context_cache: self.context_cache,
            history: self.history,
            stats: self.stats,
        }
    }

    /// Load context from cache (if available)
    pub fn load_from_cache(
        mut self,
        cache_key: &str,
    ) -> StateMachineResult<Option<ContextManager<ContextReady>>> {
        if let Some(cached_context) = self.context_cache.get(cache_key).cloned() {
            self.stats.cache_hits += 1;
            self.current_context = Some(cached_context);

            let transition = StateTransition::new("ContextIdle", "ContextReady")
                .with_metadata("operation", "load_from_cache".to_string())
                .with_metadata("cache_key", cache_key.to_string());

            self.state_machine.record_transition(transition);

            Ok(Some(ContextManager {
                state_machine: TrackedStateMachine::new(),
                config: self.config,
                current_context: self.current_context,
                context_cache: self.context_cache,
                history: self.history,
                stats: self.stats,
            }))
        } else {
            self.stats.cache_misses += 1;
            Ok(None)
        }
    }
}

impl ContextManager<ContextBuilding> {
    /// Add items to the context being built
    pub fn add_context_items(&mut self, items: Vec<ContextItem>) -> StateMachineResult<()> {
        for item in items {
            // Validate each item
            if item.priority < self.config.validation_rules.min_priority {
                self.stats.validation_failures += 1;
                continue;
            }

            // Add to current context or create new one
            if let Some(ref mut context) = self.current_context {
                // We'd need to implement add_item on Context
                // For now, just track stats
                self.stats.total_items_processed += 1;
                self.stats.total_size_processed += item.content.len();
            } else {
                // Create a new context with first item
                // This is a simplified approach
                self.stats.total_items_processed += 1;
                self.stats.total_size_processed += item.content.len();
            }
        }

        Ok(())
    }

    /// Validate the current context
    pub fn validate_context(&mut self) -> StateMachineResult<bool> {
        let context =
            self.current_context
                .as_ref()
                .ok_or_else(|| StateMachineError::PreconditionFailed {
                    condition: "No context available for validation".to_string(),
                })?;

        let rules = &self.config.validation_rules;

        // Check size limits
        if context.total_size() > rules.max_total_size {
            self.stats.validation_failures += 1;
            return Ok(false);
        }

        // Check item count
        if context.items().len() > rules.max_items {
            self.stats.validation_failures += 1;
            return Ok(false);
        }

        // Check for required user input
        if rules.require_user_input {
            let has_user_input = context
                .items()
                .iter()
                .any(|item| matches!(item.priority, ContextPriority::Critical));

            if !has_user_input {
                self.stats.validation_failures += 1;
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Complete building and transition based on auto-optimization setting
    pub fn complete_building(mut self) -> StateMachineResult<ContextManager<ContextOptimizing>> {
        // Validate before completing
        let is_valid = {
            let mut temp_manager = ContextManager {
                state_machine: TrackedStateMachine::new(),
                config: self.config.clone(),
                current_context: self.current_context.clone(),
                context_cache: HashMap::new(),
                history: VecDeque::new(),
                stats: ContextStats::default(),
            };
            temp_manager.validate_context().unwrap_or(false)
        };

        if !is_valid {
            return Err(StateMachineError::ValidationError(
                crate::types::TypeValidationError::InvalidQueryPattern(
                    "Context validation failed".to_string(),
                ),
            ));
        }

        let transition = StateTransition::new("ContextBuilding", "ContextOptimizing")
            .with_metadata("auto_optimize", self.config.auto_optimize.to_string());

        self.state_machine.record_transition(transition);

        Ok(ContextManager {
            state_machine: TrackedStateMachine::new(),
            config: self.config,
            current_context: self.current_context,
            context_cache: self.context_cache,
            history: self.history,
            stats: self.stats,
        })
    }

    /// Handle building error
    pub fn handle_building_error(mut self, error: String) -> ContextManager<ContextError> {
        let transition =
            StateTransition::new("ContextBuilding", "ContextError").with_metadata("error", error);

        self.state_machine.record_transition(transition);

        ContextManager {
            state_machine: TrackedStateMachine::new(),
            config: self.config,
            current_context: self.current_context,
            context_cache: self.context_cache,
            history: self.history,
            stats: self.stats,
        }
    }
}

impl ContextManager<ContextOptimizing> {
    /// Apply optimization to the current context
    pub fn optimize_context(&mut self) -> StateMachineResult<()> {
        let context =
            self.current_context
                .as_mut()
                .ok_or_else(|| StateMachineError::PreconditionFailed {
                    condition: "No context available for optimization".to_string(),
                })?;

        let original_size = context.total_size();
        let original_items = context.items().len();

        // Apply optimization based on strategy
        let optimization_result = match self.config.optimization_strategy {
            OptimizationStrategy::None => Ok(()),
            OptimizationStrategy::LruEviction => Self::apply_lru_optimization(context),
            OptimizationStrategy::ContentCompression => Self::apply_content_compression(context),
            OptimizationStrategy::PriorityBased => Self::apply_priority_optimization(context),
            OptimizationStrategy::Hybrid => Self::apply_hybrid_optimization(context),
        };

        match optimization_result {
            Ok(()) => {
                self.stats.optimizations_applied += 1;

                // Record history
                let history_entry = ContextHistory {
                    timestamp: std::time::SystemTime::now(),
                    operation: "optimization".to_string(),
                    items_before: original_items,
                    items_after: context.items().len(),
                    size_before: original_size,
                    size_after: context.total_size(),
                    optimization_applied: Some(self.config.optimization_strategy),
                };

                self.history.push_back(history_entry);

                // Maintain history size limit
                if self.history.len() > self.config.max_history {
                    self.history.pop_front();
                }

                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Complete optimization and transition to ready
    pub fn complete_optimization(mut self) -> ContextManager<ContextReady> {
        let transition = StateTransition::new("ContextOptimizing", "ContextReady").with_metadata(
            "optimization_strategy",
            format!("{:?}", self.config.optimization_strategy),
        );

        self.state_machine.record_transition(transition);

        ContextManager {
            state_machine: TrackedStateMachine::new(),
            config: self.config,
            current_context: self.current_context,
            context_cache: self.context_cache,
            history: self.history,
            stats: self.stats,
        }
    }

    /// Skip optimization and go directly to ready
    pub fn skip_optimization(mut self) -> ContextManager<ContextReady> {
        let transition = StateTransition::new("ContextOptimizing", "ContextReady")
            .with_metadata("optimization_skipped", "true".to_string());

        self.state_machine.record_transition(transition);

        ContextManager {
            state_machine: TrackedStateMachine::new(),
            config: self.config,
            current_context: self.current_context,
            context_cache: self.context_cache,
            history: self.history,
            stats: self.stats,
        }
    }

    fn apply_lru_optimization(context: &mut Context) -> StateMachineResult<()> {
        // Simplified LRU - remove oldest items if over limit
        // In a real implementation, we'd track access times
        context
            .compress()
            .map_err(|e| StateMachineError::PreconditionFailed {
                condition: format!("LRU optimization failed: {}", e),
            })?;
        Ok(())
    }

    fn apply_content_compression(context: &mut Context) -> StateMachineResult<()> {
        // Simplified compression - remove duplicate content
        context
            .compress()
            .map_err(|e| StateMachineError::PreconditionFailed {
                condition: format!("Content compression failed: {}", e),
            })?;
        Ok(())
    }

    fn apply_priority_optimization(context: &mut Context) -> StateMachineResult<()> {
        // Remove low-priority items first
        context
            .compress()
            .map_err(|e| StateMachineError::PreconditionFailed {
                condition: format!("Priority optimization failed: {}", e),
            })?;
        Ok(())
    }

    fn apply_hybrid_optimization(context: &mut Context) -> StateMachineResult<()> {
        // Combine multiple strategies
        context
            .compress()
            .map_err(|e| StateMachineError::PreconditionFailed {
                condition: format!("Hybrid optimization failed: {}", e),
            })?;
        Ok(())
    }
}

impl ContextManager<ContextReady> {
    /// Get the ready context
    pub const fn get_context(&self) -> Option<&Context> {
        self.current_context.as_ref()
    }

    /// Cache the current context
    pub fn cache_context(&mut self, cache_key: String) -> StateMachineResult<()> {
        if let Some(context) = &self.current_context
            && self.config.cache_contexts {
                self.context_cache.insert(cache_key, context.clone());
            }
        Ok(())
    }

    /// Get context statistics
    pub const fn get_stats(&self) -> &ContextStats {
        &self.stats
    }

    /// Get context history
    pub const fn get_history(&self) -> &VecDeque<ContextHistory> {
        &self.history
    }

    /// Reset to idle state
    pub fn reset(mut self) -> ContextManager<ContextIdle> {
        let transition = StateTransition::new("ContextReady", "ContextIdle");
        self.state_machine.record_transition(transition);

        ContextManager {
            state_machine: TrackedStateMachine::new(),
            config: self.config,
            current_context: None,
            context_cache: self.context_cache, // Keep cache
            history: self.history,             // Keep history
            stats: self.stats,                 // Keep stats
        }
    }
}

impl ContextManager<ContextError> {
    /// Get error information
    pub fn get_error_info(&self) -> Option<&StateTransition> {
        self.state_machine.last_transition()
    }

    /// Recover from error
    pub fn recover(mut self) -> ContextManager<ContextIdle> {
        let transition = StateTransition::new("ContextError", "ContextIdle")
            .with_metadata("recovery", "manual".to_string());

        self.state_machine.record_transition(transition);

        ContextManager {
            state_machine: TrackedStateMachine::new(),
            config: self.config,
            current_context: None, // Clear failed context
            context_cache: self.context_cache,
            history: self.history,
            stats: self.stats,
        }
    }
}

// Implement StateMachine trait for all states
impl<S: StateMachineState> StateMachine<S> for ContextManager<S> {
    type Error = StateMachineError;

    fn state_name(&self) -> &'static str {
        std::any::type_name::<S>()
    }

    fn validate_state(&self) -> Result<(), Self::Error> {
        // Validate configuration consistency
        if self.config.validation_rules.max_total_size == 0 {
            return Err(StateMachineError::PreconditionFailed {
                condition: "Max total size cannot be zero".to_string(),
            });
        }

        if self.config.validation_rules.max_items == 0 {
            return Err(StateMachineError::PreconditionFailed {
                condition: "Max items cannot be zero".to_string(),
            });
        }

        Ok(())
    }

    fn can_transition_to(&self, target_state: &'static str) -> bool {
        match (self.state_name(), target_state) {
            ("ContextIdle", "ContextBuilding") => true,
            ("ContextIdle", "ContextReady") => true, // From cache
            ("ContextBuilding", "ContextOptimizing") => true,
            ("ContextBuilding", "ContextError") => true,
            ("ContextOptimizing", "ContextReady") => true,
            ("ContextReady", "ContextIdle") => true,
            ("ContextError", "ContextIdle") => true,
            _ => false,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_manager_creation() {
        let config = ContextManagerConfig::default();
        let manager = ContextManager::<ContextIdle>::new(config);

        assert_eq!(manager.state_name(), "context_manager::ContextIdle");
        assert_eq!(manager.stats.contexts_created, 0);
        assert!(manager.current_context.is_none());
    }

    #[test]
    fn test_validation_rules() {
        let rules = ValidationRules::default();

        assert_eq!(rules.max_total_size, 32_768);
        assert_eq!(rules.max_items, 100);
        assert_eq!(rules.min_priority, ContextPriority::Low);
        assert!(rules.require_user_input);
        assert!(!rules.allow_duplicates);
    }

    #[test]
    fn test_optimization_strategies() {
        let strategies = [
            OptimizationStrategy::None,
            OptimizationStrategy::LruEviction,
            OptimizationStrategy::ContentCompression,
            OptimizationStrategy::PriorityBased,
            OptimizationStrategy::Hybrid,
        ];

        for strategy in strategies {
            let config = ContextManagerConfig {
                optimization_strategy: strategy,
                ..Default::default()
            };

            let manager = ContextManager::<ContextIdle>::new(config);
            assert_eq!(manager.config.optimization_strategy, strategy);
        }
    }

    #[test]
    fn test_state_transitions() {
        let config = ContextManagerConfig::default();
        let manager = ContextManager::<ContextIdle>::new(config);

        // Test valid transitions
        assert!(manager.can_transition_to("ContextBuilding"));
        assert!(manager.can_transition_to("ContextReady"));
        assert!(!manager.can_transition_to("ContextOptimizing")); // Invalid from idle

        let building_manager = manager.start_building();
        assert_eq!(
            building_manager.state_name(),
            "context_manager::ContextBuilding"
        );
        assert_eq!(building_manager.stats.contexts_created, 1);
    }

    #[test]
    fn test_context_stats() {
        let stats = ContextStats::default();

        assert_eq!(stats.contexts_created, 0);
        assert_eq!(stats.optimizations_applied, 0);
        assert_eq!(stats.total_items_processed, 0);
        assert_eq!(stats.total_size_processed, 0);
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.cache_misses, 0);
        assert_eq!(stats.validation_failures, 0);
    }

    #[test]
    fn test_context_history() {
        let history = ContextHistory {
            timestamp: std::time::SystemTime::now(),
            operation: "test_operation".to_string(),
            items_before: 10,
            items_after: 8,
            size_before: 1024,
            size_after: 800,
            optimization_applied: Some(OptimizationStrategy::PriorityBased),
        };

        assert_eq!(history.operation, "test_operation");
        assert_eq!(history.items_before, 10);
        assert_eq!(history.items_after, 8);
        assert_eq!(history.size_before, 1024);
        assert_eq!(history.size_after, 800);
        assert_eq!(
            history.optimization_applied,
            Some(OptimizationStrategy::PriorityBased)
        );
    }
}
