//! Embeddings abstraction for the Context Engine.
//! This is a lightweight placeholder; actual implementations (OpenAI API,
//! local models) can be added later.

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
