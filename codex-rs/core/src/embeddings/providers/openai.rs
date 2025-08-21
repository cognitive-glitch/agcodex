//! OpenAI embeddings provider - completely separate from chat models
//!
//! Supports:
//! - text-embedding-3-small (256-1536 dimensions)
//! - text-embedding-3-large (256-3072 dimensions)
//! - Batch processing (up to 2048 inputs)
//! - Uses OPENAI_EMBEDDING_KEY (not OPENAI_API_KEY)

use super::super::{EmbeddingError, EmbeddingProvider, EmbeddingVector};
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// OpenAI embedding provider
pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    model: String,
    dimensions: Option<usize>,
    api_endpoint: Option<String>,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider
    pub fn new(
        api_key: String,
        model: String,
        dimensions: Option<usize>,
        api_endpoint: Option<String>,
    ) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            dimensions,
            api_endpoint,
        }
    }
}

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    input: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    encoding_format: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    data: Vec<OpenAIEmbedding>,
    model: String,
    usage: OpenAIUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbedding {
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: usize,
    total_tokens: usize,
}

#[derive(Debug, Deserialize)]
struct OpenAIError {
    error: OpenAIErrorDetail,
}

#[derive(Debug, Deserialize)]
struct OpenAIErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
    code: Option<String>,
}

#[async_trait::async_trait]
impl EmbeddingProvider for OpenAIProvider {
    fn model_id(&self) -> String {
        format!("openai:{}", self.model)
    }

    fn dimensions(&self) -> usize {
        // Return configured dimensions or model defaults
        self.dimensions.unwrap_or_else(|| {
            match self.model.as_str() {
                "text-embedding-3-small" => 1536,
                "text-embedding-3-large" => 3072,
                "text-embedding-ada-002" => 1536,
                _ => 1536,
            }
        })
    }

    async fn embed(&self, text: &str) -> Result<EmbeddingVector, EmbeddingError> {
        self.embed_batch(&[text.to_string()])
            .await
            .map(|mut vecs| vecs.pop().unwrap_or_default())
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<EmbeddingVector>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        // OpenAI has a limit of 2048 inputs per batch
        const MAX_BATCH_SIZE: usize = 2048;
        if texts.len() > MAX_BATCH_SIZE {
            // Process in chunks
            let mut all_embeddings = Vec::with_capacity(texts.len());
            for chunk in texts.chunks(MAX_BATCH_SIZE) {
                let chunk_embeddings = self.embed_batch_internal(chunk).await?;
                all_embeddings.extend(chunk_embeddings);
            }
            return Ok(all_embeddings);
        }

        self.embed_batch_internal(texts).await
    }

    fn is_available(&self) -> bool {
        !self.api_key.is_empty()
    }
}

impl OpenAIProvider {
    async fn embed_batch_internal(&self, texts: &[String]) -> Result<Vec<EmbeddingVector>, EmbeddingError> {
        let endpoint = self.api_endpoint.as_deref()
            .unwrap_or("https://api.openai.com/v1/embeddings");

        let request = OpenAIRequest {
            model: self.model.clone(),
            input: texts.to_vec(),
            dimensions: self.dimensions,
            encoding_format: Some("float".to_string()),
        };

        let response = self
            .client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| EmbeddingError::ApiError(format!("Request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            
            // Try to parse OpenAI error format
            if let Ok(error) = serde_json::from_str::<OpenAIError>(&error_text) {
                return Err(EmbeddingError::ApiError(format!(
                    "OpenAI API error ({}): {} - {}",
                    status,
                    error.error.error_type,
                    error.error.message
                )));
            }
            
            return Err(EmbeddingError::ApiError(format!(
                "OpenAI API error ({}): {}",
                status,
                error_text
            )));
        }

        let openai_response: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| EmbeddingError::ApiError(format!("Failed to parse response: {}", e)))?;

        // Sort by index to ensure correct order
        let mut embeddings = openai_response.data;
        embeddings.sort_by_key(|e| e.index);

        // Validate dimensions
        let expected_dims = self.dimensions();
        for embedding in &embeddings {
            if embedding.embedding.len() != expected_dims {
                return Err(EmbeddingError::DimensionMismatch {
                    expected: expected_dims,
                    actual: embedding.embedding.len(),
                });
            }
        }

        Ok(embeddings.into_iter().map(|e| e.embedding).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_id() {
        let provider = OpenAIProvider::new(
            "test-key".to_string(),
            "text-embedding-3-small".to_string(),
            Some(256),
            None,
        );
        assert_eq!(provider.model_id(), "openai:text-embedding-3-small");
    }

    #[test]
    fn test_dimensions() {
        let provider = OpenAIProvider::new(
            "test-key".to_string(),
            "text-embedding-3-small".to_string(),
            Some(256),
            None,
        );
        assert_eq!(provider.dimensions(), 256);

        let provider_default = OpenAIProvider::new(
            "test-key".to_string(),
            "text-embedding-3-large".to_string(),
            None,
            None,
        );
        assert_eq!(provider_default.dimensions(), 3072);
    }

    #[test]
    fn test_is_available() {
        let provider = OpenAIProvider::new(
            "test-key".to_string(),
            "text-embedding-3-small".to_string(),
            None,
            None,
        );
        assert!(provider.is_available());

        let provider_empty = OpenAIProvider::new(
            String::new(),
            "text-embedding-3-small".to_string(),
            None,
            None,
        );
        assert!(!provider_empty.is_available());
    }
}