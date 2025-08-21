//! Voyage AI embeddings provider

use super::super::{EmbeddingModel, EmbeddingProvider, EmbeddingRequest, EmbeddingResponse};
use crate::error::{CoreError, CoreResult};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Voyage AI embedding provider
pub struct VoyageProvider {
    client: Client,
    api_key: String,
    model: EmbeddingModel,
}

impl VoyageProvider {
    /// Create a new Voyage provider
    pub fn new(api_key: String, model: EmbeddingModel) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
        }
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
}

#[derive(Debug, Deserialize)]
struct VoyageEmbedding {
    embedding: Vec<f32>,
}

#[async_trait]
impl EmbeddingProvider for VoyageProvider {
    async fn embed(&self, request: EmbeddingRequest) -> CoreResult<EmbeddingResponse> {
        let voyage_request = VoyageRequest {
            model: self.model.to_string(),
            input: request.texts,
            input_type: "document".to_string(), // Could be configurable: document, query, etc.
        };

        let response = self
            .client
            .post("https://api.voyageai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&voyage_request)
            .send()
            .await
            .map_err(|e| CoreError::EmbeddingError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(CoreError::EmbeddingError(format!(
                "Voyage API error: {}",
                error_text
            )));
        }

        let voyage_response: VoyageResponse = response
            .json()
            .await
            .map_err(|e| CoreError::EmbeddingError(e.to_string()))?;

        let embeddings = voyage_response
            .data
            .into_iter()
            .map(|e| e.embedding)
            .collect();

        Ok(EmbeddingResponse {
            embeddings,
            model: self.model.clone(),
            dimensions: self.model.dimensions().unwrap_or(1024),
        })
    }

    fn name(&self) -> &str {
        "Voyage"
    }

    fn model(&self) -> &EmbeddingModel {
        &self.model
    }
}