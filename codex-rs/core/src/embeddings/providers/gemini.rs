//! Gemini embeddings provider - completely separate from chat models
//!
//! Supports:
//! - gemini-embedding-001 (768 dimensions)
//! - Batch processing
//! - Uses GEMINI_API_KEY environment variable

use super::super::{EmbeddingError, EmbeddingProvider, EmbeddingVector};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};

/// Gemini embedding provider
pub struct GeminiProvider {
    client: Client,
    api_key: String,
    model: String,
    api_endpoint: Option<String>,
}

impl GeminiProvider {
    /// Create a new Gemini provider
    pub fn new(
        api_key: String,
        model: String,
    ) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            api_endpoint: None,
        }
    }

    /// Create a new Gemini provider with custom endpoint
    pub fn new_with_endpoint(
        api_key: String,
        model: String,
        api_endpoint: String,
    ) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            api_endpoint: Some(api_endpoint),
        }
    }
}

#[derive(Debug, Serialize)]
struct GeminiRequest {
    content: GeminiContent,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    embedding: GeminiEmbedding,
}

#[derive(Debug, Deserialize)]
struct GeminiEmbedding {
    values: Vec<f32>,
}

#[derive(Debug, Deserialize)]
struct GeminiError {
    error: GeminiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct GeminiErrorDetail {
    message: String,
    code: Option<u32>,
    status: Option<String>,
}

#[async_trait::async_trait]
impl EmbeddingProvider for GeminiProvider {
    fn model_id(&self) -> String {
        format!("gemini:{}", self.model)
    }

    fn dimensions(&self) -> usize {
        // Return model-specific dimensions
        match self.model.as_str() {
            "gemini-embedding-001" => 768,
            "text-embedding-004" => 768,
            "embedding-001" => 768,
            "textembedding-gecko@001" => 768,
            "textembedding-gecko@003" => 768,
            _ => 768, // Default fallback
        }
    }

    async fn embed(&self, text: &str) -> Result<EmbeddingVector, EmbeddingError> {
        let request = GeminiRequest {
            content: GeminiContent {
                parts: vec![GeminiPart {
                    text: text.to_string(),
                }],
            },
        };

        let endpoint = self.api_endpoint.as_deref().unwrap_or(
            "https://generativelanguage.googleapis.com"
        );
        let url = format!(
            "{}/v1/models/{}:embedContent?key={}",
            endpoint,
            self.model,
            self.api_key
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| EmbeddingError::ApiError(format!("Request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            
            // Try to parse Gemini error format
            if let Ok(error) = serde_json::from_str::<GeminiError>(&error_text) {
                return Err(EmbeddingError::ApiError(format!(
                    "Gemini API error ({}): {} - {}",
                    status,
                    error.error.status.unwrap_or_else(|| "Unknown".to_string()),
                    error.error.message
                )));
            }
            
            return Err(EmbeddingError::ApiError(format!(
                "Gemini API error ({}): {}",
                status,
                error_text
            )));
        }

        let gemini_response: GeminiResponse = response
            .json()
            .await
            .map_err(|e| EmbeddingError::ApiError(format!("Failed to parse response: {}", e)))?;

        // Validate dimensions
        let expected_dims = self.dimensions();
        if gemini_response.embedding.values.len() != expected_dims {
            return Err(EmbeddingError::DimensionMismatch {
                expected: expected_dims,
                actual: gemini_response.embedding.values.len(),
            });
        }

        Ok(gemini_response.embedding.values)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<EmbeddingVector>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        // Gemini doesn't support batch embedding in a single call,
        // so we need to make multiple requests
        let mut embeddings = Vec::with_capacity(texts.len());
        
        for text in texts {
            let embedding = self.embed(text).await?;
            embeddings.push(embedding);
            
            // Add a small delay to respect rate limits
            sleep(Duration::from_millis(100)).await;
        }

        Ok(embeddings)
    }

    fn is_available(&self) -> bool {
        !self.api_key.is_empty()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_id() {
        let provider = GeminiProvider::new(
            "test-key".to_string(),
            "embedding-001".to_string(),
        );
        assert_eq!(provider.model_id(), "gemini:embedding-001");
    }

    #[test]
    fn test_dimensions() {
        let provider = GeminiProvider::new(
            "test-key".to_string(),
            "gemini-embedding-001".to_string(),
        );
        assert_eq!(provider.dimensions(), 768);

        let provider_old = GeminiProvider::new(
            "test-key".to_string(),
            "text-embedding-004".to_string(),
        );
        assert_eq!(provider_old.dimensions(), 768);

        let provider_default = GeminiProvider::new(
            "test-key".to_string(),
            "unknown-model".to_string(),
        );
        assert_eq!(provider_default.dimensions(), 768);
    }

    #[test]
    fn test_is_available() {
        let provider = GeminiProvider::new(
            "test-key".to_string(),
            "embedding-001".to_string(),
        );
        assert!(provider.is_available());

        let provider_empty = GeminiProvider::new(
            String::new(),
            "embedding-001".to_string(),
        );
        assert!(!provider_empty.is_available());
    }
}