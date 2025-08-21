//! Independent embeddings system - completely separate from chat/LLM models.
//! 
//! This module provides optional embedding functionality with:
//! - Multiple provider support (OpenAI, Gemini, Voyage)
//! - Strict index separation by model and repository
//! - Zero overhead when disabled
//! - Independent authentication from chat models

pub mod config;
pub mod index_manager;
pub mod manager;
pub mod providers;

pub use config::{EmbeddingsConfig, ProviderConfig};
pub use index_manager::{EmbeddingIndexManager, IndexKey, SearchResult};
pub use manager::EmbeddingsManager;

use thiserror::Error;

pub type EmbeddingVector = Vec<f32>;

#[derive(Debug, Error)]
pub enum EmbeddingError {
    #[error("Embeddings not enabled")]
    NotEnabled,
    
    #[error("Provider not available: {0}")]
    ProviderNotAvailable(String),
    
    #[error("API error: {0}")]
    ApiError(String),
    
    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
    
    #[error("Index error: {0}")]
    IndexError(#[from] index_manager::IndexError),
    
    #[error(transparent)]
    Io(#[from] std::io::Error),
    
    #[error("Provider error: {0}")]
    Provider(String),
}

/// Trait for embedding providers
#[allow(async_fn_in_trait)]
pub trait EmbeddingProvider: Send + Sync {
    /// Get the unique model identifier
    fn model_id(&self) -> String;
    
    /// Get the actual dimensions for this model
    fn dimensions(&self) -> usize;
    
    /// Embed a single text
    async fn embed(&self, text: &str) -> Result<EmbeddingVector, EmbeddingError>;
    
    /// Embed multiple texts in batch
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<EmbeddingVector>, EmbeddingError>;
    
    /// Check if this provider is available (has valid API key)
    fn is_available(&self) -> bool;
}