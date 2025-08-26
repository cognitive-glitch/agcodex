//! Type-safe builder for configuration with validation and defaults.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::num::NonZeroUsize;

use std::time::Duration;

use serde::Deserialize;
use serde::Serialize;

use super::BuilderError;
use super::BuilderResult;
use super::BuilderState;
use super::Init;
use super::Ready;
use super::Validated;
use crate::types::FilePath;

/// Model configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelConfig {
    pub name: String,
    pub provider: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub max_tokens: Option<NonZeroUsize>,
    pub temperature: Option<f32>,
    pub timeout: Duration,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            name: "gpt-4".to_string(),
            provider: "openai".to_string(),
            api_key: None,
            base_url: None,
            max_tokens: NonZeroUsize::new(4096),
            temperature: Some(0.7),
            timeout: Duration::from_secs(30),
        }
    }
}

/// Security policy configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub sandbox_enabled: bool,
    pub allowed_paths: Vec<FilePath>,
    pub blocked_commands: Vec<String>,
    pub network_access: bool,
    pub file_size_limit: NonZeroUsize,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            sandbox_enabled: true,
            allowed_paths: vec![],
            blocked_commands: vec!["rm".to_string(), "sudo".to_string()],
            network_access: false,
            file_size_limit: NonZeroUsize::new(10 * 1024 * 1024).unwrap(), // 10MB
        }
    }
}

/// Performance tuning configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub max_concurrent_requests: NonZeroUsize,
    pub cache_size: NonZeroUsize,
    pub compression_level: u8,
    pub batch_size: NonZeroUsize,
    pub timeout_multiplier: f32,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_concurrent_requests: NonZeroUsize::new(4).unwrap(),
            cache_size: NonZeroUsize::new(1000).unwrap(),
            compression_level: 3,
            batch_size: NonZeroUsize::new(10).unwrap(),
            timeout_multiplier: 1.0,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogConfig {
    pub level: LogLevel,
    pub file_path: Option<FilePath>,
    pub console_output: bool,
    pub structured: bool,
    pub max_file_size: NonZeroUsize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            file_path: None,
            console_output: true,
            structured: false,
            max_file_size: NonZeroUsize::new(100 * 1024 * 1024).unwrap(), // 100MB
        }
    }
}

/// Final application configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    pub model: ModelConfig,
    pub security: SecurityConfig,
    pub performance: PerformanceConfig,
    pub logging: LogConfig,
    pub custom_settings: HashMap<String, String>,
}

impl AppConfig {
    /// Create a new builder
    pub fn builder() -> ConfigBuilder<Init> {
        ConfigBuilder::new()
    }

    /// Validate the entire configuration
    pub fn validate(&self) -> BuilderResult<()> {
        // Validate model config
        if self.model.name.is_empty() {
            return Err(BuilderError::Configuration(
                "Model name cannot be empty".to_string(),
            ));
        }

        if let Some(temp) = self.model.temperature
            && !(0.0..=2.0).contains(&temp) {
                return Err(BuilderError::Configuration(format!(
                    "Temperature {} not in range [0.0, 2.0]",
                    temp
                )));
            }

        // Validate security config
        if self.security.file_size_limit.get() > 1_000_000_000 {
            // 1GB
            return Err(BuilderError::Configuration(
                "File size limit too large (max 1GB)".to_string(),
            ));
        }

        // Validate performance config
        if self.performance.compression_level > 9 {
            return Err(BuilderError::Configuration(
                "Compression level must be 0-9".to_string(),
            ));
        }

        Ok(())
    }

    /// Get a custom setting
    pub fn get_custom(&self, key: &str) -> Option<&str> {
        self.custom_settings.get(key).map(|s| s.as_str())
    }

    /// Check if security is enabled
    pub const fn is_secure(&self) -> bool {
        self.security.sandbox_enabled
    }
}

/// Type-safe configuration builder
#[derive(Debug)]
pub struct ConfigBuilder<S: BuilderState> {
    model: ModelConfig,
    security: SecurityConfig,
    performance: PerformanceConfig,
    logging: LogConfig,
    custom_settings: HashMap<String, String>,
    _state: PhantomData<S>,
}

impl ConfigBuilder<Init> {
    /// Create a new builder with defaults
    pub fn new() -> Self {
        Self {
            model: ModelConfig::default(),
            security: SecurityConfig::default(),
            performance: PerformanceConfig::default(),
            logging: LogConfig::default(),
            custom_settings: HashMap::new(),
            _state: PhantomData,
        }
    }

    /// Set model name (transitions to Validated state)
    pub fn model_name(
        mut self,
        name: impl Into<String>,
    ) -> BuilderResult<ConfigBuilder<Validated>> {
        let name = name.into();
        if name.is_empty() {
            return Err(BuilderError::InvalidField {
                field: "model_name",
                value: "empty".to_string(),
            });
        }

        self.model.name = name;

        Ok(ConfigBuilder {
            model: self.model,
            security: self.security,
            performance: self.performance,
            logging: self.logging,
            custom_settings: self.custom_settings,
            _state: PhantomData,
        })
    }
}

impl<S: BuilderState> ConfigBuilder<S> {
    /// Set model provider
    pub fn model_provider(mut self, provider: impl Into<String>) -> Self {
        self.model.provider = provider.into();
        self
    }

    /// Set API key
    pub fn api_key(mut self, key: Option<String>) -> Self {
        self.model.api_key = key;
        self
    }

    /// Set base URL
    pub fn base_url(mut self, url: Option<String>) -> Self {
        self.model.base_url = url;
        self
    }

    /// Set max tokens
    pub const fn max_tokens(mut self, tokens: Option<NonZeroUsize>) -> Self {
        self.model.max_tokens = tokens;
        self
    }

    /// Set temperature
    pub fn temperature(mut self, temp: Option<f32>) -> BuilderResult<Self> {
        if let Some(t) = temp
            && !(0.0..=2.0).contains(&t) {
                return Err(BuilderError::InvalidField {
                    field: "temperature",
                    value: t.to_string(),
                });
            }
        self.model.temperature = temp;
        Ok(self)
    }

    /// Set request timeout
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.model.timeout = timeout;
        self
    }

    /// Enable/disable sandbox
    pub const fn sandbox_enabled(mut self, enabled: bool) -> Self {
        self.security.sandbox_enabled = enabled;
        self
    }

    /// Add allowed path
    pub fn add_allowed_path(mut self, path: FilePath) -> Self {
        self.security.allowed_paths.push(path);
        self
    }

    /// Set allowed paths
    pub fn allowed_paths(mut self, paths: Vec<FilePath>) -> Self {
        self.security.allowed_paths = paths;
        self
    }

    /// Add blocked command
    pub fn add_blocked_command(mut self, command: impl Into<String>) -> Self {
        self.security.blocked_commands.push(command.into());
        self
    }

    /// Enable/disable network access
    pub const fn network_access(mut self, enabled: bool) -> Self {
        self.security.network_access = enabled;
        self
    }

    /// Set file size limit
    pub const fn file_size_limit(mut self, limit: NonZeroUsize) -> Self {
        self.security.file_size_limit = limit;
        self
    }

    /// Set max concurrent requests
    pub const fn max_concurrent_requests(mut self, max: NonZeroUsize) -> Self {
        self.performance.max_concurrent_requests = max;
        self
    }

    /// Set cache size
    pub const fn cache_size(mut self, size: NonZeroUsize) -> Self {
        self.performance.cache_size = size;
        self
    }

    /// Set compression level (0-9)
    pub fn compression_level(mut self, level: u8) -> BuilderResult<Self> {
        if level > 9 {
            return Err(BuilderError::InvalidField {
                field: "compression_level",
                value: level.to_string(),
            });
        }
        self.performance.compression_level = level;
        Ok(self)
    }

    /// Set log level
    pub const fn log_level(mut self, level: LogLevel) -> Self {
        self.logging.level = level;
        self
    }

    /// Set log file path
    pub fn log_file(mut self, path: Option<FilePath>) -> Self {
        self.logging.file_path = path;
        self
    }

    /// Enable/disable console output
    pub const fn console_output(mut self, enabled: bool) -> Self {
        self.logging.console_output = enabled;
        self
    }

    /// Enable/disable structured logging
    pub const fn structured_logs(mut self, structured: bool) -> Self {
        self.logging.structured = structured;
        self
    }

    /// Add custom setting
    pub fn custom_setting(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_settings.insert(key.into(), value.into());
        self
    }
}

impl ConfigBuilder<Validated> {
    /// Finalize configuration (transitions to Ready state)
    pub fn finalize(self) -> ConfigBuilder<Ready> {
        ConfigBuilder {
            model: self.model,
            security: self.security,
            performance: self.performance,
            logging: self.logging,
            custom_settings: self.custom_settings,
            _state: PhantomData,
        }
    }
}

impl ConfigBuilder<Ready> {
    /// Build the final configuration
    pub fn build(self) -> BuilderResult<AppConfig> {
        let config = AppConfig {
            model: self.model,
            security: self.security,
            performance: self.performance,
            logging: self.logging,
            custom_settings: self.custom_settings,
        };

        // Validate the built configuration
        config.validate()?;

        Ok(config)
    }

    /// Build with custom validation
    pub fn build_with_validation<F>(self, validator: F) -> BuilderResult<AppConfig>
    where
        F: FnOnce(&AppConfig) -> BuilderResult<()>,
    {
        let config = self.build()?;
        validator(&config)?;
        Ok(config)
    }
}

impl Default for ConfigBuilder<Init> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Create a development configuration
pub fn dev_config() -> BuilderResult<AppConfig> {
    AppConfig::builder()
        .model_name("gpt-4")?
        .sandbox_enabled(false)
        .log_level(LogLevel::Debug)
        .console_output(true)
        .finalize()
        .build()
}

/// Create a production configuration
pub fn prod_config(api_key: String) -> BuilderResult<AppConfig> {
    AppConfig::builder()
        .model_name("gpt-4")?
        .api_key(Some(api_key))
        .sandbox_enabled(true)
        .network_access(false)
        .log_level(LogLevel::Info)
        .structured_logs(true)
        .max_concurrent_requests(NonZeroUsize::new(2).unwrap())
        .finalize()
        .build()
}

/// Create a secure configuration for sensitive environments
pub fn secure_config(api_key: String, allowed_paths: Vec<FilePath>) -> BuilderResult<AppConfig> {
    AppConfig::builder()
        .model_name("gpt-4")?
        .api_key(Some(api_key))
        .sandbox_enabled(true)
        .allowed_paths(allowed_paths)
        .network_access(false)
        .file_size_limit(NonZeroUsize::new(1024 * 1024).unwrap()) // 1MB
        .add_blocked_command("curl")
        .add_blocked_command("wget")
        .add_blocked_command("ssh")
        .log_level(LogLevel::Warn)
        .finalize()
        .build()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder_flow() {
        let config = AppConfig::builder()
            .model_name("gpt-4")
            .unwrap()
            .api_key(Some("test-key".to_string()))
            .sandbox_enabled(true)
            .log_level(LogLevel::Debug)
            .finalize()
            .build()
            .unwrap();

        assert_eq!(config.model.name, "gpt-4");
        assert_eq!(config.model.api_key, Some("test-key".to_string()));
        assert!(config.security.sandbox_enabled);
        assert_eq!(config.logging.level, LogLevel::Debug);
    }

    #[test]
    fn test_temperature_validation() {
        let result = AppConfig::builder()
            .model_name("gpt-4")
            .unwrap()
            .temperature(Some(3.0)); // Invalid temperature

        assert!(result.is_err());
    }

    #[test]
    fn test_compression_level_validation() {
        let result = AppConfig::builder()
            .model_name("gpt-4")
            .unwrap()
            .compression_level(15); // Invalid level

        assert!(result.is_err());
    }

    #[test]
    fn test_custom_settings() {
        let config = AppConfig::builder()
            .model_name("gpt-4")
            .unwrap()
            .custom_setting("debug_mode", "true")
            .custom_setting("theme", "dark")
            .finalize()
            .build()
            .unwrap();

        assert_eq!(config.get_custom("debug_mode"), Some("true"));
        assert_eq!(config.get_custom("theme"), Some("dark"));
        assert_eq!(config.get_custom("nonexistent"), None);
    }

    #[test]
    fn test_convenience_functions() {
        let dev = dev_config().unwrap();
        assert!(!dev.security.sandbox_enabled);
        assert_eq!(dev.logging.level, LogLevel::Debug);

        let prod = prod_config("api-key".to_string()).unwrap();
        assert!(prod.security.sandbox_enabled);
        assert_eq!(prod.logging.level, LogLevel::Info);
    }

    #[test]
    fn test_config_validation() {
        // Test that empty model name fails validation
        let result = AppConfig::builder().model_name("").unwrap_err();

        if let BuilderError::InvalidField { field, .. } = result {
            assert_eq!(field, "model_name");
        } else {
            panic!("Expected InvalidField error");
        }
    }
}
