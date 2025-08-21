//! Configuration for embeddings - completely independent from chat model configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main embeddings configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmbeddingsConfig {
    /// Whether embeddings are enabled (default: false)
    #[serde(default)]
    pub enabled: bool,
    
    /// Provider selection strategy
    #[serde(default = "default_provider")]
    pub provider: ProviderSelection,
    
    /// OpenAI configuration (if using OpenAI)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openai: Option<OpenAIConfig>,
    
    /// Gemini configuration (if using Gemini)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gemini: Option<GeminiConfig>,
    
    /// Voyage configuration (if using Voyage)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voyage: Option<VoyageConfig>,
    
    /// Cache settings
    #[serde(default)]
    pub cache: CacheConfig,
}

impl Default for EmbeddingsConfig {
    fn default() -> Self {
        Self {
            enabled: false, // DISABLED by default - zero overhead
            provider: ProviderSelection::Auto,
            openai: None,
            gemini: None,
            voyage: None,
            cache: CacheConfig::default(),
        }
    }
}

/// Provider selection strategy
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderSelection {
    /// Automatically select based on available API keys
    Auto,
    /// Use OpenAI embeddings
    OpenAI,
    /// Use Gemini embeddings
    Gemini,
    /// Use Voyage AI embeddings
    Voyage,
}

fn default_provider() -> ProviderSelection {
    ProviderSelection::Auto
}

/// OpenAI embeddings configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OpenAIConfig {
    /// Model to use (e.g., "text-embedding-3-small", "text-embedding-3-large")
    #[serde(default = "default_openai_model")]
    pub model: String,
    
    /// Optional dimension override (for models that support it)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<usize>,
    
    /// API endpoint (for custom/proxy endpoints)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_endpoint: Option<String>,
}

fn default_openai_model() -> String {
    "text-embedding-3-small".to_string()
}

/// Gemini embeddings configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GeminiConfig {
    /// Model to use (e.g., "gemini-embedding-001")
    #[serde(default = "default_gemini_model")]
    pub model: String,
    
    /// Task type for embeddings
    #[serde(default = "default_task_type")]
    pub task_type: String,
}

fn default_gemini_model() -> String {
    "gemini-embedding-001".to_string()
}

fn default_task_type() -> String {
    "retrieval_document".to_string()
}

/// Voyage AI embeddings configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VoyageConfig {
    /// Model to use (e.g., "voyage-3.5", "voyage-3.5-large")
    #[serde(default = "default_voyage_model")]
    pub model: String,
    
    /// Input type (document, query)
    #[serde(default = "default_input_type")]
    pub input_type: String,
}

fn default_voyage_model() -> String {
    "voyage-3.5".to_string()
}

fn default_input_type() -> String {
    "document".to_string()
}

/// Cache configuration for embeddings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheConfig {
    /// Enable caching of embeddings
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// Maximum cache size in MB
    #[serde(default = "default_cache_size")]
    pub max_size_mb: usize,
    
    /// Cache TTL in seconds
    #[serde(default = "default_cache_ttl")]
    pub ttl_seconds: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size_mb: 500,
            ttl_seconds: 3600, // 1 hour
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_cache_size() -> usize {
    500
}

fn default_cache_ttl() -> u64 {
    3600
}

/// Intelligence mode configuration for embeddings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntelligenceMode {
    /// Fast, minimal resources (256-768 dimensions)
    Light,
    /// Balanced (1536 dimensions)
    Medium,
    /// Maximum intelligence (3072 dimensions)
    Hard,
}

impl IntelligenceMode {
    /// Get recommended model configuration for each provider
    pub fn model_config(&self, provider: &str) -> ProviderConfig {
        match (self, provider) {
            (Self::Light, "openai") => ProviderConfig {
                model: "text-embedding-3-small".to_string(),
                dimensions: Some(256),
            },
            (Self::Medium, "openai") => ProviderConfig {
                model: "text-embedding-3-small".to_string(),
                dimensions: Some(1536),
            },
            (Self::Hard, "openai") => ProviderConfig {
                model: "text-embedding-3-large".to_string(),
                dimensions: Some(3072),
            },
            (Self::Light, "gemini") => ProviderConfig {
                model: "gemini-embedding-001".to_string(),
                dimensions: Some(256),
            },
            (Self::Medium, "gemini") => ProviderConfig {
                model: "gemini-embedding-001".to_string(),
                dimensions: Some(768),
            },
            (Self::Hard, "gemini") => ProviderConfig {
                model: "gemini-embedding-exp-03-07".to_string(),
                dimensions: Some(1536),
            },
            (Self::Light, "voyage") => ProviderConfig {
                model: "voyage-3.5-lite".to_string(),
                dimensions: None, // Fixed dimensions
            },
            (Self::Medium, "voyage") => ProviderConfig {
                model: "voyage-3.5".to_string(),
                dimensions: None,
            },
            (Self::Hard, "voyage") => ProviderConfig {
                model: "voyage-3-large".to_string(),
                dimensions: None,
            },
            _ => ProviderConfig {
                model: "text-embedding-3-small".to_string(),
                dimensions: Some(1536),
            },
        }
    }
}

/// Provider configuration for a specific intelligence mode
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub model: String,
    pub dimensions: Option<usize>,
}

/// Load embeddings configuration from environment and config file
pub fn load_embeddings_config() -> EmbeddingsConfig {
    // Try to load from config file first
    if let Ok(config_str) = std::fs::read_to_string(
        dirs::home_dir()
            .unwrap_or_default()
            .join(".agcodex")
            .join("config.toml")
    ) {
        if let Ok(config) = toml::from_str::<HashMap<String, toml::Value>>(&config_str) {
            if let Some(embeddings) = config.get("embeddings") {
                if let Ok(config) = embeddings.clone().try_into::<EmbeddingsConfig>() {
                    return config;
                }
            }
        }
    }
    
    // Default: disabled
    EmbeddingsConfig::default()
}

/// Check if embeddings are available
pub fn embeddings_available() -> bool {
    let config = load_embeddings_config();
    config.enabled && has_any_embedding_key()
}

/// Check if any embedding API key is available
pub fn has_any_embedding_key() -> bool {
    std::env::var("OPENAI_EMBEDDING_KEY").is_ok()
        || std::env::var("GEMINI_API_KEY").is_ok()
        || std::env::var("VOYAGE_API_KEY").is_ok()
}

/// Get API key for a specific provider
pub fn get_embedding_api_key(provider: &str) -> Option<String> {
    match provider {
        "openai" => std::env::var("OPENAI_EMBEDDING_KEY").ok(),
        "gemini" => std::env::var("GEMINI_API_KEY").ok(),
        "voyage" => std::env::var("VOYAGE_API_KEY").ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config_is_disabled() {
        let config = EmbeddingsConfig::default();
        assert!(!config.enabled);
    }
    
    #[test]
    fn test_intelligence_mode_mapping() {
        let light_openai = IntelligenceMode::Light.model_config("openai");
        assert_eq!(light_openai.model, "text-embedding-3-small");
        assert_eq!(light_openai.dimensions, Some(256));
        
        let hard_openai = IntelligenceMode::Hard.model_config("openai");
        assert_eq!(hard_openai.model, "text-embedding-3-large");
        assert_eq!(hard_openai.dimensions, Some(3072));
        
        let medium_gemini = IntelligenceMode::Medium.model_config("gemini");
        assert_eq!(medium_gemini.model, "gemini-embedding-001");
        assert_eq!(medium_gemini.dimensions, Some(768));
    }
}