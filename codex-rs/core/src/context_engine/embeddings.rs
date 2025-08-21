//! Embeddings abstraction for the Context Engine.
//!
//! This module provides a bridge between the Context Engine and the independent
//! embeddings system. When embeddings are disabled, operations return NotImplemented
//! errors with zero overhead.

use crate::config::Config;
use crate::embeddings::EmbeddingsConfig;
use crate::embeddings::EmbeddingsManager;
use thiserror::Error;

pub type EmbeddingVector = Vec<f32>;

#[derive(Debug, Error)]
pub enum EmbeddingError {
    #[error("not implemented")]
    NotImplemented,

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("provider error: {0}")]
    Provider(String),

    #[error("embeddings error: {0}")]
    EmbeddingsError(#[from] crate::embeddings::EmbeddingError),
}

#[allow(async_fn_in_trait)]
pub trait EmbeddingModel {
    fn dimensions(&self) -> usize {
        1536 // common default placeholder
    }

    async fn embed(&self, _text: &str) -> Result<EmbeddingVector, EmbeddingError> {
        Err(EmbeddingError::NotImplemented)
    }

    async fn embed_batch(&self, _texts: &[String]) -> Result<Vec<EmbeddingVector>, EmbeddingError> {
        Err(EmbeddingError::NotImplemented)
    }
}

/// Bridge between the Context Engine and the EmbeddingsManager.
///
/// This struct provides zero overhead when embeddings are disabled and seamless
/// integration when they are enabled.
///
/// # Usage Examples
///
/// ```rust,ignore
/// // Create a disabled bridge (zero overhead)
/// let bridge = EmbeddingModelBridge::disabled();
///
/// // Create from embeddings config
/// let embeddings_config = Some(EmbeddingsConfig::default());
/// let bridge = EmbeddingModelBridge::from_embeddings_config(embeddings_config);
///
/// // Use the bridge (returns NotImplemented when disabled)
/// match bridge.embed("some text").await {
///     Ok(vector) => println!("Embedded: {} dimensions", vector.len()),
///     Err(EmbeddingError::NotImplemented) => println!("Embeddings disabled"),
///     Err(e) => println!("Error: {}", e),
/// }
/// ```
pub struct EmbeddingModelBridge {
    /// The embeddings manager (None when disabled)
    manager: Option<EmbeddingsManager>,
}

impl EmbeddingModelBridge {
    /// Create a new bridge from the embeddings manager
    pub const fn new(manager: Option<EmbeddingsManager>) -> Self {
        Self { manager }
    }

    /// Create a bridge from embeddings configuration
    ///
    /// This is the preferred way to create the bridge, as it handles
    /// the case where embeddings are disabled.
    pub fn from_embeddings_config(config: Option<EmbeddingsConfig>) -> Self {
        let manager = EmbeddingsManager::new(config);
        Self::new(Some(manager))
    }

    /// Create a bridge from the main Config (future extension point)
    ///
    /// For now, this creates a disabled bridge since the main Config
    /// doesn't yet have an embeddings field. When embeddings are added
    /// to the main Config, this method will extract the embeddings config.
    pub fn from_config(_config: &Config) -> Self {
        // TODO: When Config gets an embeddings field, use:
        // let embeddings_config = config.embeddings.clone();
        // Self::from_embeddings_config(embeddings_config)

        // For now, create a disabled bridge
        Self::new(None)
    }

    /// Create a disabled bridge (zero overhead)
    pub fn disabled() -> Self {
        Self::new(None)
    }

    /// Check if embeddings are enabled
    pub fn is_enabled(&self) -> bool {
        self.manager
            .as_ref()
            .map(|m| m.is_enabled())
            .unwrap_or(false)
    }
}

impl EmbeddingModel for EmbeddingModelBridge {
    fn dimensions(&self) -> usize {
        if let Some(manager) = &self.manager {
            manager.current_dimensions().unwrap_or(1536)
        } else {
            1536 // Default when disabled
        }
    }

    async fn embed(&self, text: &str) -> Result<EmbeddingVector, EmbeddingError> {
        match &self.manager {
            Some(manager) => {
                if !manager.is_enabled() {
                    return Err(EmbeddingError::NotImplemented);
                }

                let result = manager.embed(text).await?;
                match result {
                    Some(vector) => Ok(vector),
                    None => Err(EmbeddingError::NotImplemented),
                }
            }
            None => Err(EmbeddingError::NotImplemented),
        }
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<EmbeddingVector>, EmbeddingError> {
        match &self.manager {
            Some(manager) => {
                if !manager.is_enabled() {
                    return Err(EmbeddingError::NotImplemented);
                }

                let result = manager.embed_batch(texts).await?;
                match result {
                    Some(vectors) => Ok(vectors),
                    None => Err(EmbeddingError::NotImplemented),
                }
            }
            None => Err(EmbeddingError::NotImplemented),
        }
    }
}

/// Default implementation that always returns NotImplemented
///
/// This is useful for testing and as a fallback when no embeddings are configured.
pub struct NoOpEmbeddingModel;

impl EmbeddingModel for NoOpEmbeddingModel {
    fn dimensions(&self) -> usize {
        1536
    }

    async fn embed(&self, _text: &str) -> Result<EmbeddingVector, EmbeddingError> {
        Err(EmbeddingError::NotImplemented)
    }

    async fn embed_batch(&self, _texts: &[String]) -> Result<Vec<EmbeddingVector>, EmbeddingError> {
        Err(EmbeddingError::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disabled_bridge_has_zero_overhead() {
        let bridge = EmbeddingModelBridge::disabled();
        assert!(!bridge.is_enabled());
        assert_eq!(bridge.dimensions(), 1536);
    }

    #[tokio::test]
    async fn test_disabled_bridge_returns_not_implemented() {
        let bridge = EmbeddingModelBridge::disabled();

        let result = bridge.embed("test").await;
        assert!(matches!(result, Err(EmbeddingError::NotImplemented)));

        let batch_result = bridge.embed_batch(&["test".to_string()]).await;
        assert!(matches!(batch_result, Err(EmbeddingError::NotImplemented)));
    }

    // NOTE: Config construction test is skipped for now since Config has many fields
    // and doesn't yet have embeddings configuration. When embeddings are added to Config,
    // we can add a proper test for from_config()

    #[test]
    fn test_noop_embedding_model() {
        let model = NoOpEmbeddingModel;
        assert_eq!(model.dimensions(), 1536);
    }

    #[tokio::test]
    async fn test_noop_embedding_model_returns_not_implemented() {
        let model = NoOpEmbeddingModel;

        let result = model.embed("test").await;
        assert!(matches!(result, Err(EmbeddingError::NotImplemented)));

        let batch_result = model.embed_batch(&["test".to_string()]).await;
        assert!(matches!(batch_result, Err(EmbeddingError::NotImplemented)));
    }
}
