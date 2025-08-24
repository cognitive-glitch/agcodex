//! Main embeddings manager - coordinates providers and indexes.
//!
//! This is completely independent from chat/LLM models and has zero overhead when disabled.

use super::EmbeddingError;
use super::EmbeddingProvider;
use super::EmbeddingVector;
use super::config::EmbeddingsConfig;
use super::config::IntelligenceMode;
use super::config::ProviderSelection;
use super::index_manager::EmbeddingIndexManager;
use super::index_manager::SearchResult;
use super::providers::GeminiProvider;
use super::providers::OpenAIProvider;
use super::providers::VoyageProvider;
use super::providers::voyage::VoyageInputType;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::debug;
use tracing::info;
use tracing::warn;

/// Main embeddings manager
pub struct EmbeddingsManager {
    /// Configuration (None = disabled)
    config: Option<EmbeddingsConfig>,

    /// Available providers
    providers: HashMap<String, Box<dyn EmbeddingProvider>>,

    /// Current active provider
    active_provider: Option<String>,

    /// Index manager for vector storage
    index_manager: Option<Arc<EmbeddingIndexManager>>,

    /// Current repository path
    current_repo: Option<PathBuf>,

    /// Intelligence mode
    intelligence_mode: IntelligenceMode,
}

impl EmbeddingsManager {
    /// Create a new embeddings manager
    pub fn new(config: Option<EmbeddingsConfig>) -> Self {
        if config.is_none() {
            info!("Embeddings disabled - zero overhead mode");
            return Self::disabled();
        }

        let config = config.expect("Config should be Some when calling this code path");
        if !config.enabled {
            info!("Embeddings explicitly disabled in config");
            return Self::disabled();
        }

        info!("Initializing embeddings manager");

        // Initialize providers based on available API keys
        let mut providers = HashMap::new();
        let mut active_provider = None;

        // Check OpenAI
        if let Some(api_key) = super::config::get_embedding_api_key("openai") {
            debug!("OpenAI embedding API key found");
            if let Some(openai_config) = &config.openai {
                providers.insert(
                    "openai".to_string(),
                    Box::new(OpenAIProvider::new(
                        api_key,
                        openai_config.model.clone(),
                        openai_config.dimensions,
                        None, // api_endpoint
                    )) as Box<dyn EmbeddingProvider>,
                );
                if active_provider.is_none() {
                    active_provider = Some("openai".to_string());
                }
            }
        }

        // Check Gemini
        if let Some(api_key) = super::config::get_embedding_api_key("gemini") {
            debug!("Gemini embedding API key found");
            if let Some(gemini_config) = &config.gemini {
                providers.insert(
                    "gemini".to_string(),
                    Box::new(GeminiProvider::new(api_key, gemini_config.model.clone()))
                        as Box<dyn EmbeddingProvider>,
                );
                if active_provider.is_none() {
                    active_provider = Some("gemini".to_string());
                }
            }
        }

        // Check Voyage
        if let Some(api_key) = super::config::get_embedding_api_key("voyage") {
            debug!("Voyage embedding API key found");
            if let Some(voyage_config) = &config.voyage {
                let input_type = match voyage_config.input_type.as_str() {
                    "query" => VoyageInputType::Query,
                    _ => VoyageInputType::Document,
                };
                providers.insert(
                    "voyage".to_string(),
                    Box::new(VoyageProvider::new(
                        api_key,
                        voyage_config.model.clone(),
                        input_type,
                        None, // api_endpoint
                    )) as Box<dyn EmbeddingProvider>,
                );
                if active_provider.is_none() {
                    active_provider = Some("voyage".to_string());
                }
            }
        }

        // Select provider based on config
        if let ProviderSelection::Auto = config.provider {
            // Auto mode - use first available
            info!("Auto-selecting embedding provider: {:?}", active_provider);
        } else {
            // Specific provider requested
            let requested = match config.provider {
                ProviderSelection::OpenAI => "openai",
                ProviderSelection::Gemini => "gemini",
                ProviderSelection::Voyage => "voyage",
                _ => "openai",
            };

            if providers.contains_key(requested) {
                active_provider = Some(requested.to_string());
                info!("Using requested embedding provider: {}", requested);
            } else {
                warn!(
                    "Requested provider {} not available, using: {:?}",
                    requested, active_provider
                );
            }
        }

        // Initialize index manager
        let storage_dir = dirs::home_dir()
            .unwrap_or_default()
            .join(".agcodex")
            .join("embeddings");

        let index_manager = if !providers.is_empty() {
            Some(Arc::new(EmbeddingIndexManager::new(storage_dir)))
        } else {
            None
        };

        Self {
            config: Some(config),
            providers,
            active_provider,
            index_manager,
            current_repo: None,
            intelligence_mode: IntelligenceMode::Medium,
        }
    }

    /// Create a disabled manager (zero overhead)
    pub fn disabled() -> Self {
        Self {
            config: None,
            providers: HashMap::new(),
            active_provider: None,
            index_manager: None,
            current_repo: None,
            intelligence_mode: IntelligenceMode::Medium,
        }
    }

    /// Check if embeddings are enabled
    pub fn is_enabled(&self) -> bool {
        self.config.as_ref().map(|c| c.enabled).unwrap_or(false)
    }

    /// Set the current repository
    pub fn set_repository(&mut self, repo: PathBuf) {
        self.current_repo = Some(repo);
    }

    /// Set intelligence mode
    pub const fn set_intelligence_mode(&mut self, mode: IntelligenceMode) {
        self.intelligence_mode = mode;
        // TODO: Update provider configurations based on mode
    }

    /// Get current model ID
    pub fn current_model_id(&self) -> Option<String> {
        self.active_provider
            .as_ref()
            .and_then(|name| self.providers.get(name).map(|p| p.model_id()))
    }

    /// Get current dimensions
    pub fn current_dimensions(&self) -> Option<usize> {
        self.active_provider
            .as_ref()
            .and_then(|name| self.providers.get(name).map(|p| p.dimensions()))
    }

    /// Embed a single text (returns None if disabled)
    pub async fn embed(&self, text: &str) -> Result<Option<EmbeddingVector>, EmbeddingError> {
        if !self.is_enabled() {
            return Ok(None); // Fast path - no work done
        }

        let provider_name = self
            .active_provider
            .as_ref()
            .ok_or(EmbeddingError::NotEnabled)?;

        let provider = self
            .providers
            .get(provider_name)
            .ok_or_else(|| EmbeddingError::ProviderNotAvailable(provider_name.clone()))?;

        let vector = provider.embed(text).await?;
        Ok(Some(vector))
    }

    /// Embed multiple texts in batch
    pub async fn embed_batch(
        &self,
        texts: &[String],
    ) -> Result<Option<Vec<EmbeddingVector>>, EmbeddingError> {
        if !self.is_enabled() {
            return Ok(None);
        }

        let provider_name = self
            .active_provider
            .as_ref()
            .ok_or(EmbeddingError::NotEnabled)?;

        let provider = self
            .providers
            .get(provider_name)
            .ok_or_else(|| EmbeddingError::ProviderNotAvailable(provider_name.clone()))?;

        let vectors = provider.embed_batch(texts).await?;
        Ok(Some(vectors))
    }

    /// Search in the appropriate index for current repo/model
    pub async fn search_in_index(
        &self,
        repo: &Path,
        model_id: &str,
        dimensions: usize,
        query: &str,
    ) -> Result<Vec<SearchResult>, EmbeddingError> {
        let index_manager = self
            .index_manager
            .as_ref()
            .ok_or(EmbeddingError::NotEnabled)?;

        // Embed the query
        let query_vector = self.embed(query).await?.ok_or(EmbeddingError::NotEnabled)?;

        // Search in the correct index
        let results = index_manager.search(
            repo,
            model_id,
            dimensions,
            &query_vector,
            10, // Top 10 results
        )?;

        Ok(results)
    }

    /// Get statistics about the embeddings system
    pub fn stats(&self) -> EmbeddingsStats {
        EmbeddingsStats {
            enabled: self.is_enabled(),
            active_provider: self.active_provider.clone(),
            available_providers: self.providers.keys().cloned().collect(),
            current_repo: self.current_repo.clone(),
            intelligence_mode: self.intelligence_mode,
            index_stats: self.index_manager.as_ref().map(|m| m.stats()),
        }
    }
}

/// Statistics about the embeddings system
#[derive(Debug)]
pub struct EmbeddingsStats {
    pub enabled: bool,
    pub active_provider: Option<String>,
    pub available_providers: Vec<String>,
    pub current_repo: Option<PathBuf>,
    pub intelligence_mode: IntelligenceMode,
    pub index_stats: Option<super::index_manager::IndexManagerStats>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disabled_manager_has_zero_overhead() {
        let manager = EmbeddingsManager::disabled();
        assert!(!manager.is_enabled());
        assert!(manager.providers.is_empty());
        assert!(manager.index_manager.is_none());
    }

    #[tokio::test]
    async fn test_disabled_embed_returns_none() {
        let manager = EmbeddingsManager::disabled();
        let result = manager.embed("test").await.unwrap();
        assert!(result.is_none());
    }
}
