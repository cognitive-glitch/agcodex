//! Voyage AI embeddings provider - completely separate from chat models
//!
//! Supports:
//! - voyage-3.5 (1024 dimensions by default)
//! - Batch processing (up to 128 inputs)
//! - Document vs Query input types for optimized embeddings
//! - Uses VOYAGE_API_KEY environment variable

use super::super::EmbeddingError;
use super::super::EmbeddingProvider;
use super::super::EmbeddingVector;
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;

/// Input type for Voyage AI embeddings
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoyageInputType {
    /// For documents being indexed/stored
    Document,
    /// For search queries
    Query,
}

impl ToString for VoyageInputType {
    fn to_string(&self) -> String {
        match self {
            VoyageInputType::Document => "document".to_string(),
            VoyageInputType::Query => "query".to_string(),
        }
    }
}

/// Voyage AI embedding provider
pub struct VoyageProvider {
    client: Client,
    api_key: String,
    model: String,
    input_type: VoyageInputType,
    api_endpoint: Option<String>,
}

impl VoyageProvider {
    /// Create a new Voyage provider
    pub fn new(
        api_key: String,
        model: String,
        input_type: VoyageInputType,
        api_endpoint: Option<String>,
    ) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            input_type,
            api_endpoint,
        }
    }

    /// Create a new Voyage provider for documents
    pub fn new_for_documents(api_key: String, model: String) -> Self {
        Self::new(api_key, model, VoyageInputType::Document, None)
    }

    /// Create a new Voyage provider for queries
    pub fn new_for_queries(api_key: String, model: String) -> Self {
        Self::new(api_key, model, VoyageInputType::Query, None)
    }

    /// Get the current input type
    pub const fn input_type(&self) -> &VoyageInputType {
        &self.input_type
    }

    /// Set the input type
    pub const fn set_input_type(&mut self, input_type: VoyageInputType) {
        self.input_type = input_type;
    }
}

#[derive(Debug, Serialize)]
struct VoyageRequest {
    model: String,
    input: Vec<String>,
    input_type: String,
}

#[derive(Debug, Deserialize)]
struct VoyageResponse {
    data: Vec<VoyageEmbedding>,
    _usage: VoyageUsage,
}

#[derive(Debug, Deserialize)]
struct VoyageEmbedding {
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Debug, Deserialize)]
struct VoyageUsage {
    _total_tokens: usize,
}

#[derive(Debug, Deserialize)]
struct VoyageError {
    error: VoyageErrorDetail,
}

#[derive(Debug, Deserialize)]
struct VoyageErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
    _code: Option<String>,
}

#[async_trait::async_trait]
impl EmbeddingProvider for VoyageProvider {
    fn model_id(&self) -> String {
        format!("voyage:{}:{}", self.model, self.input_type.to_string())
    }

    fn dimensions(&self) -> usize {
        // Return model-specific dimensions
        match self.model.as_str() {
            "voyage-3.5" => 1024,
            "voyage-3.5-lite" => 512,
            "voyage-3-large" => 1536,
            "voyage-3" => 1024,
            "voyage-2" => 1024,
            "voyage-large-2" => 1536,
            "voyage-code-2" => 1536,
            "voyage-multilingual-2" => 1024,
            _ => 1024, // Default fallback
        }
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

        // Voyage AI has a limit of 128 inputs per batch
        const MAX_BATCH_SIZE: usize = 128;
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

impl VoyageProvider {
    async fn embed_batch_internal(
        &self,
        texts: &[String],
    ) -> Result<Vec<EmbeddingVector>, EmbeddingError> {
        let endpoint = self
            .api_endpoint
            .as_deref()
            .unwrap_or("https://api.voyageai.com/v1/embeddings");

        let request = VoyageRequest {
            model: self.model.clone(),
            input: texts.to_vec(),
            input_type: self.input_type.to_string(),
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
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            // Try to parse Voyage error format
            if let Ok(error) = serde_json::from_str::<VoyageError>(&error_text) {
                return Err(EmbeddingError::ApiError(format!(
                    "Voyage API error ({}): {} - {}",
                    status, error.error.error_type, error.error.message
                )));
            }

            return Err(EmbeddingError::ApiError(format!(
                "Voyage API error ({}): {}",
                status, error_text
            )));
        }

        let voyage_response: VoyageResponse = response
            .json()
            .await
            .map_err(|e| EmbeddingError::ApiError(format!("Failed to parse response: {}", e)))?;

        // Sort by index to ensure correct order
        let mut embeddings = voyage_response.data;
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
        let provider = VoyageProvider::new(
            "test-key".to_string(),
            "voyage-3.5".to_string(),
            VoyageInputType::Document,
            None,
        );
        assert_eq!(provider.model_id(), "voyage:voyage-3.5:document");

        let provider_query = VoyageProvider::new(
            "test-key".to_string(),
            "voyage-3.5".to_string(),
            VoyageInputType::Query,
            None,
        );
        assert_eq!(provider_query.model_id(), "voyage:voyage-3.5:query");
    }

    #[test]
    fn test_dimensions() {
        let provider = VoyageProvider::new(
            "test-key".to_string(),
            "voyage-3.5".to_string(),
            VoyageInputType::Document,
            None,
        );
        assert_eq!(provider.dimensions(), 1024);

        let provider_lite = VoyageProvider::new(
            "test-key".to_string(),
            "voyage-3.5-lite".to_string(),
            VoyageInputType::Document,
            None,
        );
        assert_eq!(provider_lite.dimensions(), 512);

        let provider_3_large = VoyageProvider::new(
            "test-key".to_string(),
            "voyage-3-large".to_string(),
            VoyageInputType::Document,
            None,
        );
        assert_eq!(provider_3_large.dimensions(), 1536);

        let provider_large = VoyageProvider::new(
            "test-key".to_string(),
            "voyage-large-2".to_string(),
            VoyageInputType::Document,
            None,
        );
        assert_eq!(provider_large.dimensions(), 1536);
    }

    #[test]
    fn test_input_type() {
        assert_eq!(VoyageInputType::Document.to_string(), "document");
        assert_eq!(VoyageInputType::Query.to_string(), "query");
    }

    #[test]
    fn test_convenience_constructors() {
        let provider_doc =
            VoyageProvider::new_for_documents("test-key".to_string(), "voyage-3.5".to_string());
        assert_eq!(provider_doc.input_type(), &VoyageInputType::Document);

        let provider_query =
            VoyageProvider::new_for_queries("test-key".to_string(), "voyage-3.5".to_string());
        assert_eq!(provider_query.input_type(), &VoyageInputType::Query);
    }

    #[test]
    fn test_is_available() {
        let provider = VoyageProvider::new(
            "test-key".to_string(),
            "voyage-3.5".to_string(),
            VoyageInputType::Document,
            None,
        );
        assert!(provider.is_available());

        let provider_empty = VoyageProvider::new(
            String::new(),
            "voyage-3.5".to_string(),
            VoyageInputType::Document,
            None,
        );
        assert!(!provider_empty.is_available());
    }

    #[test]
    fn test_set_input_type() {
        let mut provider =
            VoyageProvider::new_for_documents("test-key".to_string(), "voyage-3.5".to_string());
        assert_eq!(provider.input_type(), &VoyageInputType::Document);

        provider.set_input_type(VoyageInputType::Query);
        assert_eq!(provider.input_type(), &VoyageInputType::Query);
    }
}
