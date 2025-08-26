//! Builder patterns for type-safe construction of complex objects.
//!
//! This module provides builder patterns with compile-time state tracking
//! to ensure correct construction order and validation.

pub mod ast_query_builder;
pub mod config_builder;
pub mod context_builder;
pub mod search_query_builder;

pub use ast_query_builder::*;
pub use config_builder::*;
pub use context_builder::*;
pub use search_query_builder::*;

/// Trait for builder pattern state transitions
pub trait BuilderState {}

/// Marker for initial builder state
#[derive(Debug, Clone, Copy)]
pub struct Init;

/// Marker for validated builder state
#[derive(Debug, Clone, Copy)]
pub struct Validated;

/// Marker for ready-to-build state
#[derive(Debug, Clone, Copy)]
pub struct Ready;

impl BuilderState for Init {}
impl BuilderState for Validated {}
impl BuilderState for Ready {}

/// Common builder error types
#[derive(Debug, thiserror::Error)]
pub enum BuilderError {
    #[error("Missing required field: {field}")]
    MissingField { field: &'static str },

    #[error("Invalid field value: {field} = {value}")]
    InvalidField { field: &'static str, value: String },

    #[error("Builder not ready: missing validation")]
    NotValidated,

    #[error("Builder not ready: {reason}")]
    NotReady { reason: String },

    #[error("Type validation error: {0}")]
    TypeValidation(#[from] crate::types::TypeValidationError),

    #[error("Configuration error: {0}")]
    Configuration(String),
}

/// Result type for builder operations
pub type BuilderResult<T> = Result<T, BuilderError>;
