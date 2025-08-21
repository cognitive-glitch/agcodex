//! Gemini embeddings provider

use super::super::{EmbeddingModel, EmbeddingProvider, EmbeddingRequest, EmbeddingResponse};
use crate::error::{CoreError, CoreResult};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Gemini embedding provider
pub struct GeminiProvider {
    client: Client,
    api_key: String,
    model: EmbeddingModel,
}

impl GeminiProvider {
    /// Create a new Gemini provider
    pub fn new(api_key: String, model: EmbeddingModel) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
        }
    }
}

#[derive(Debug, Serialize)]
struct GeminiRequest {
    requests: Vec<GeminiContent>,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    model: String,
    content: GeminiTextContent,
}

#[derive(Debug, Serialize)]
struct GeminiTextContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    embeddings: Vec<GeminiEmbedding>,
}

#[derive(Debug, Deserialize)]
struct GeminiEmbedding {
    values: Vec<f32>,
}

#[async_trait]
impl EmbeddingProvider for GeminiProvider {
    async fn embed(&self, request: EmbeddingRequest) -> CoreResult<EmbeddingResponse> {
        let gemini_requests: Vec<GeminiContent> = request
            .texts
            .into_iter()
            .map(|text| GeminiContent {
                model: format!("models/{}", self.model.to_string()),
                content: GeminiTextContent {
                    parts: vec![GeminiPart { text }],
                },
            })
            .collect();

        let gemini_request = GeminiRequest {
            requests: gemini_requests,
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:batchEmbedContents?key={}",
            self.model.to_string(),
            self.api_key
        );

        let response = self
            .client
            .post(&url)
            .json(&gemini_request)
            .send()
            .await
            .map_err(|e| CoreError::EmbeddingError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(CoreError::EmbeddingError(format!(
                "Gemini API error: {}",
                error_text
            )));
        }

        let gemini_response: GeminiResponse = response
            .json()
            .await
            .map_err(|e| CoreError::EmbeddingError(e.to_string()))?;

        let embeddings = gemini_response
            .embeddings
            .into_iter()
            .map(|e| e.values)
            .collect();

        Ok(EmbeddingResponse {
            embeddings,
            model: self.model.clone(),
            dimensions: self.model.dimensions().unwrap_or(768),
        })
    }

    fn name(&self) -> &str {
        "Gemini"
    }

    fn model(&self) -> &EmbeddingModel {
        &self.model
    }
}