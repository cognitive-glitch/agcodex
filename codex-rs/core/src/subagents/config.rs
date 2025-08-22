//! Configuration structures for subagents
//!
//! This module defines how subagents are configured through TOML files.
//! Each agent has its own configuration file that defines its capabilities,
//! permissions, and behavior.

use crate::modes::OperatingMode;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// Intelligence level for subagent operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IntelligenceLevel {
    /// Fast, minimal resources (70% compression)
    Light,
    /// Balanced, default (85% compression)
    Medium,
    /// Maximum intelligence (95% compression)
    Hard,
}

impl Default for IntelligenceLevel {
    fn default() -> Self {
        Self::Medium
    }
}

impl IntelligenceLevel {
    /// Get the compression level as a percentage
    pub const fn compression_percentage(self) -> u8 {
        match self {
            Self::Light => 70,
            Self::Medium => 85,
            Self::Hard => 95,
        }
    }

    /// Get the maximum chunk size for this intelligence level
    pub const fn chunk_size(self) -> usize {
        match self {
            Self::Light => 256,
            Self::Medium => 512,
            Self::Hard => 1024,
        }
    }

    /// Get the maximum number of chunks for this intelligence level
    pub const fn max_chunks(self) -> usize {
        match self {
            Self::Light => 1_000,
            Self::Medium => 10_000,
            Self::Hard => 100_000,
        }
    }
}

/// Tool permission for subagents
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolPermission {
    /// Tool is allowed
    Allow,
    /// Tool is denied
    Deny,
    /// Tool is allowed with restrictions
    Restricted(HashMap<String, String>),
}

impl Default for ToolPermission {
    fn default() -> Self {
        Self::Allow
    }
}

/// Parameter definition for subagent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParameterDefinition {
    /// Parameter name
    pub name: String,
    /// Parameter description
    pub description: String,
    /// Whether the parameter is required
    #[serde(default)]
    pub required: bool,
    /// Default value if not provided
    pub default: Option<String>,
    /// Valid values (if restricted)
    pub valid_values: Option<Vec<String>>,
}

/// Complete configuration for a subagent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubagentConfig {
    /// Agent name (must be unique)
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// Override the operating mode when this agent is active
    pub mode_override: Option<OperatingMode>,

    /// Intelligence level for AST processing and embeddings
    #[serde(default)]
    pub intelligence: IntelligenceLevel,

    /// Tools and their permissions
    #[serde(default)]
    pub tools: HashMap<String, ToolPermission>,

    /// Custom prompt template for this agent
    pub prompt: String,

    /// Parameter definitions
    #[serde(default)]
    pub parameters: Vec<ParameterDefinition>,

    /// Template this agent inherits from (optional)
    pub template: Option<String>,

    /// Maximum execution time in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Whether this agent can be chained with others
    #[serde(default = "default_true")]
    pub chainable: bool,

    /// Whether this agent can run in parallel with others
    #[serde(default = "default_true")]
    pub parallelizable: bool,

    /// Custom metadata for the agent
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,

    /// File patterns this agent is specialized for
    #[serde(default)]
    pub file_patterns: Vec<String>,

    /// Tags for categorizing agents
    #[serde(default)]
    pub tags: Vec<String>,
}

const fn default_timeout() -> u64 {
    300 // 5 minutes
}

const fn default_true() -> bool {
    true
}

impl SubagentConfig {
    /// Load a subagent configuration from a TOML file
    pub fn from_file(path: &PathBuf) -> Result<Self, super::SubagentError> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Save this configuration to a TOML file
    pub fn to_file(&self, path: &PathBuf) -> Result<(), super::SubagentError> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| super::SubagentError::InvalidConfig(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), super::SubagentError> {
        if self.name.is_empty() {
            return Err(super::SubagentError::InvalidConfig(
                "agent name cannot be empty".to_string(),
            ));
        }

        if self.description.is_empty() {
            return Err(super::SubagentError::InvalidConfig(
                "agent description cannot be empty".to_string(),
            ));
        }

        if self.prompt.is_empty() {
            return Err(super::SubagentError::InvalidConfig(
                "agent prompt cannot be empty".to_string(),
            ));
        }

        if self.timeout_seconds == 0 {
            return Err(super::SubagentError::InvalidConfig(
                "timeout must be greater than 0".to_string(),
            ));
        }

        // Validate parameter names are unique
        let mut param_names = std::collections::HashSet::new();
        for param in &self.parameters {
            if !param_names.insert(&param.name) {
                return Err(super::SubagentError::InvalidConfig(format!(
                    "duplicate parameter name: {}",
                    param.name
                )));
            }
        }

        Ok(())
    }

    /// Check if a tool is allowed for this agent
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        match self.tools.get(tool_name) {
            Some(ToolPermission::Allow) => true,
            Some(ToolPermission::Deny) => false,
            Some(ToolPermission::Restricted(_)) => true, // Allowed with restrictions
            None => true,                                // Default allow
        }
    }

    /// Get tool restrictions for a specific tool
    pub fn get_tool_restrictions(&self, tool_name: &str) -> Option<&HashMap<String, String>> {
        match self.tools.get(tool_name) {
            Some(ToolPermission::Restricted(restrictions)) => Some(restrictions),
            _ => None,
        }
    }

    /// Get the effective operating mode (considering override)
    pub fn effective_mode(&self, current_mode: OperatingMode) -> OperatingMode {
        self.mode_override.unwrap_or(current_mode)
    }

    /// Check if this agent matches the given file patterns
    pub fn matches_file(&self, file_path: &std::path::Path) -> bool {
        if self.file_patterns.is_empty() {
            return true; // No restrictions
        }

        let path_str = file_path.to_string_lossy();
        self.file_patterns.iter().any(|pattern| {
            // Simple glob matching - could be enhanced with a proper glob library
            if pattern.contains('*') {
                // Basic wildcard matching
                let pattern = pattern.replace('*', ".*");
                regex_lite::Regex::new(&pattern)
                    .map(|re| re.is_match(&path_str))
                    .unwrap_or(false)
            } else {
                path_str.contains(pattern)
            }
        })
    }
}

/// Template for creating subagent configurations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubagentTemplate {
    /// Template name
    pub name: String,

    /// Template description
    pub description: String,

    /// Base configuration
    pub config: SubagentConfig,

    /// Placeholders that can be customized
    #[serde(default)]
    pub placeholders: Vec<String>,
}

impl SubagentTemplate {
    /// Load a template from a TOML file
    pub fn from_file(path: &PathBuf) -> Result<Self, super::SubagentError> {
        let content = std::fs::read_to_string(path)?;
        let template: Self = toml::from_str(&content)?;
        Ok(template)
    }

    /// Create a configuration from this template with substitutions
    pub fn instantiate(
        &self,
        name: String,
        substitutions: HashMap<String, String>,
    ) -> Result<SubagentConfig, super::SubagentError> {
        let mut config = self.config.clone();
        config.name = name;

        // Apply substitutions to the prompt
        let mut prompt = config.prompt.clone();
        for (placeholder, value) in substitutions {
            let placeholder_pattern = format!("{{{{{}}}}}", placeholder);
            prompt = prompt.replace(&placeholder_pattern, &value);
        }
        config.prompt = prompt;

        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intelligence_level_properties() {
        assert_eq!(IntelligenceLevel::Light.compression_percentage(), 70);
        assert_eq!(IntelligenceLevel::Medium.compression_percentage(), 85);
        assert_eq!(IntelligenceLevel::Hard.compression_percentage(), 95);

        assert_eq!(IntelligenceLevel::Light.chunk_size(), 256);
        assert_eq!(IntelligenceLevel::Medium.chunk_size(), 512);
        assert_eq!(IntelligenceLevel::Hard.chunk_size(), 1024);
    }

    #[test]
    fn test_subagent_config_validation() {
        let mut config = SubagentConfig {
            name: "test-agent".to_string(),
            description: "Test agent".to_string(),
            mode_override: None,
            intelligence: IntelligenceLevel::Medium,
            tools: HashMap::new(),
            prompt: "You are a test agent.".to_string(),
            parameters: vec![],
            template: None,
            timeout_seconds: 300,
            chainable: true,
            parallelizable: true,
            metadata: HashMap::new(),
            file_patterns: vec![],
            tags: vec![],
        };

        // Valid config should pass
        assert!(config.validate().is_ok());

        // Empty name should fail
        config.name = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_tool_permissions() {
        let mut tools = HashMap::new();
        tools.insert("allowed_tool".to_string(), ToolPermission::Allow);
        tools.insert("denied_tool".to_string(), ToolPermission::Deny);
        tools.insert(
            "restricted_tool".to_string(),
            ToolPermission::Restricted(HashMap::from([(
                "max_files".to_string(),
                "10".to_string(),
            )])),
        );

        let config = SubagentConfig {
            name: "test-agent".to_string(),
            description: "Test agent".to_string(),
            mode_override: None,
            intelligence: IntelligenceLevel::Medium,
            tools,
            prompt: "You are a test agent.".to_string(),
            parameters: vec![],
            template: None,
            timeout_seconds: 300,
            chainable: true,
            parallelizable: true,
            metadata: HashMap::new(),
            file_patterns: vec![],
            tags: vec![],
        };

        assert!(config.is_tool_allowed("allowed_tool"));
        assert!(!config.is_tool_allowed("denied_tool"));
        assert!(config.is_tool_allowed("restricted_tool"));
        assert!(config.is_tool_allowed("unknown_tool")); // Default allow

        assert!(config.get_tool_restrictions("restricted_tool").is_some());
        assert!(config.get_tool_restrictions("allowed_tool").is_none());
    }

    #[test]
    fn test_config_serialization() {
        let config = SubagentConfig {
            name: "test-agent".to_string(),
            description: "Test agent".to_string(),
            mode_override: Some(OperatingMode::Review),
            intelligence: IntelligenceLevel::Hard,
            tools: HashMap::new(),
            prompt: "You are a test agent.".to_string(),
            parameters: vec![ParameterDefinition {
                name: "target".to_string(),
                description: "Target file".to_string(),
                required: true,
                default: None,
                valid_values: None,
            }],
            template: None,
            timeout_seconds: 600,
            chainable: false,
            parallelizable: true,
            metadata: HashMap::new(),
            file_patterns: vec!["*.rs".to_string()],
            tags: vec!["rust".to_string(), "review".to_string()],
        };

        // Test TOML serialization
        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: SubagentConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_file_pattern_matching() {
        let config = SubagentConfig {
            name: "rust-agent".to_string(),
            description: "Rust-specific agent".to_string(),
            mode_override: None,
            intelligence: IntelligenceLevel::Medium,
            tools: HashMap::new(),
            prompt: "You are a Rust agent.".to_string(),
            parameters: vec![],
            template: None,
            timeout_seconds: 300,
            chainable: true,
            parallelizable: true,
            metadata: HashMap::new(),
            file_patterns: vec!["*.rs".to_string(), "Cargo.toml".to_string()],
            tags: vec![],
        };

        assert!(config.matches_file(&PathBuf::from("src/main.rs")));
        assert!(config.matches_file(&PathBuf::from("Cargo.toml")));
        assert!(!config.matches_file(&PathBuf::from("src/main.py")));
    }
}
