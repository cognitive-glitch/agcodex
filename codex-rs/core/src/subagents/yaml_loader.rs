//! YAML configuration loader for subagents
//!
//! This module provides functionality to load subagent configurations from YAML files,
//! supporting both global and project-specific agent directories.

use super::SubagentError;
use super::config::IntelligenceLevel;
use super::config::ParameterDefinition;
use super::config::SubagentConfig;
use super::config::ToolPermission;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use tracing::debug;
use tracing::info;
use tracing::warn;
use walkdir::WalkDir;

/// YAML representation of tool configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum YamlToolConfig {
    /// Simple permission (allow/deny)
    Simple(String),
    /// Complex configuration with permission and restrictions
    Complex {
        #[serde(rename = "permission")]
        permission: String,
        #[serde(default)]
        restrictions: HashMap<String, String>,
    },
}

/// YAML representation of a single tool entry
#[derive(Debug, Clone, Deserialize)]
struct YamlTool {
    /// Tool name
    name: String,
    /// Tool permission configuration
    #[serde(flatten)]
    config: YamlToolConfig,
}

/// YAML representation of subagent configuration
#[derive(Debug, Clone, Deserialize)]
struct YamlSubagentConfig {
    /// Agent name (must be unique)
    name: String,

    /// Human-readable description
    #[serde(default)]
    description: String,

    /// Override the operating mode when this agent is active
    #[serde(rename = "mode_override", default)]
    mode_override: Option<String>,

    /// Intelligence level for AST processing and embeddings
    #[serde(default)]
    intelligence: Option<String>,

    /// Tools and their permissions (array format)
    #[serde(default)]
    tools: Vec<YamlTool>,

    /// Custom prompt template for this agent
    #[serde(default)]
    prompt: String,

    /// Parameter definitions
    #[serde(default)]
    parameters: Vec<ParameterDefinition>,

    /// Template this agent inherits from (optional)
    #[serde(default)]
    template: Option<String>,

    /// Maximum execution time in seconds
    #[serde(default = "default_timeout")]
    timeout_seconds: u64,

    /// Whether this agent can be chained with others
    #[serde(default = "default_true")]
    chainable: bool,

    /// Whether this agent can run in parallel with others
    #[serde(default = "default_true")]
    parallelizable: bool,

    /// Custom metadata for the agent
    #[serde(default)]
    metadata: HashMap<String, serde_json::Value>,

    /// File patterns this agent is specialized for
    #[serde(default)]
    file_patterns: Vec<String>,

    /// Tags for categorizing agents
    #[serde(default)]
    tags: Vec<String>,
}

const fn default_timeout() -> u64 {
    300 // 5 minutes
}

const fn default_true() -> bool {
    true
}

impl YamlSubagentConfig {
    /// Convert from YAML representation to SubagentConfig
    fn into_config(self) -> Result<SubagentConfig, SubagentError> {
        // Parse mode override
        let mode_override = self
            .mode_override
            .map(|mode| match mode.to_lowercase().as_str() {
                "plan" => Ok(crate::modes::OperatingMode::Plan),
                "build" => Ok(crate::modes::OperatingMode::Build),
                "review" => Ok(crate::modes::OperatingMode::Review),
                _ => Err(SubagentError::InvalidConfig(format!(
                    "Invalid mode_override: {}. Must be 'plan', 'build', or 'review'",
                    mode
                ))),
            })
            .transpose()?;

        // Parse intelligence level
        let intelligence = self
            .intelligence
            .map(|level| match level.to_lowercase().as_str() {
                "light" => Ok(IntelligenceLevel::Light),
                "medium" => Ok(IntelligenceLevel::Medium),
                "hard" => Ok(IntelligenceLevel::Hard),
                _ => Err(SubagentError::InvalidConfig(format!(
                    "Invalid intelligence level: {}. Must be 'light', 'medium', or 'hard'",
                    level
                ))),
            })
            .transpose()?
            .unwrap_or_default();

        // Parse tools
        let mut tools = HashMap::new();
        for tool in self.tools {
            let permission = match tool.config {
                YamlToolConfig::Simple(perm) => match perm.to_lowercase().as_str() {
                    "allow" => ToolPermission::Allow,
                    "deny" => ToolPermission::Deny,
                    _ => {
                        return Err(SubagentError::InvalidConfig(format!(
                            "Invalid tool permission for '{}': {}. Must be 'allow' or 'deny'",
                            tool.name, perm
                        )));
                    }
                },
                YamlToolConfig::Complex {
                    permission,
                    restrictions,
                } => match permission.to_lowercase().as_str() {
                    "allow" => {
                        if restrictions.is_empty() {
                            ToolPermission::Allow
                        } else {
                            ToolPermission::Restricted(restrictions)
                        }
                    }
                    "deny" => ToolPermission::Deny,
                    "restricted" => ToolPermission::Restricted(restrictions),
                    _ => {
                        return Err(SubagentError::InvalidConfig(format!(
                            "Invalid tool permission for '{}': {}",
                            tool.name, permission
                        )));
                    }
                },
            };
            tools.insert(tool.name, permission);
        }

        let config = SubagentConfig {
            name: self.name,
            description: self.description,
            mode_override,
            intelligence,
            tools,
            prompt: self.prompt,
            parameters: self.parameters,
            template: self.template,
            timeout_seconds: self.timeout_seconds,
            chainable: self.chainable,
            parallelizable: self.parallelizable,
            metadata: self.metadata,
            file_patterns: self.file_patterns,
            tags: self.tags,
        };

        Ok(config)
    }
}

/// Load a single subagent configuration from a YAML file
pub fn load_yaml_config(path: &Path) -> Result<SubagentConfig, SubagentError> {
    debug!("Loading YAML config from: {}", path.display());

    let content = std::fs::read_to_string(path)?;
    let yaml_config: YamlSubagentConfig = serde_yaml::from_str(&content).map_err(|e| {
        SubagentError::InvalidConfig(format!(
            "Failed to parse YAML from {}: {}",
            path.display(),
            e
        ))
    })?;

    let config = yaml_config.into_config()?;
    config.validate()?;

    info!(
        "Successfully loaded YAML agent '{}' from {}",
        config.name,
        path.display()
    );

    Ok(config)
}

/// Information about a loaded YAML configuration
#[derive(Debug, Clone)]
pub struct YamlConfigInfo {
    /// The loaded configuration
    pub config: SubagentConfig,
    /// Path to the source file
    pub source_path: PathBuf,
    /// Whether this is from the global directory
    pub is_global: bool,
}

/// Load all YAML configurations from a directory
pub fn load_yaml_configs_from_directory(
    dir: &Path,
    is_global: bool,
) -> Result<Vec<YamlConfigInfo>, SubagentError> {
    let mut configs = Vec::new();

    if !dir.exists() {
        debug!("Directory does not exist: {}", dir.display());
        return Ok(configs);
    }

    info!(
        "Scanning {} for YAML configs",
        if is_global {
            "global directory"
        } else {
            "project directory"
        }
    );

    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "yaml" || ext == "yml")
                .unwrap_or(false)
        })
    {
        let path = entry.path();
        match load_yaml_config(path) {
            Ok(config) => {
                configs.push(YamlConfigInfo {
                    config,
                    source_path: path.to_path_buf(),
                    is_global,
                });
            }
            Err(e) => {
                warn!("Failed to load YAML config from {}: {}", path.display(), e);
            }
        }
    }

    info!(
        "Loaded {} YAML configs from {}",
        configs.len(),
        dir.display()
    );

    Ok(configs)
}

/// Load all YAML configurations from both global and project directories
pub fn load_all_yaml_configs() -> Result<HashMap<String, YamlConfigInfo>, SubagentError> {
    let mut all_configs = HashMap::new();

    // Find global agents directory
    let home_dir = dirs::home_dir().ok_or_else(|| {
        SubagentError::InvalidConfig("Could not determine home directory".to_string())
    })?;
    let global_agents_dir = home_dir.join(".agcodex").join("agents");

    // Load global agents
    let global_configs = load_yaml_configs_from_directory(&global_agents_dir, true)?;
    for config_info in global_configs {
        let name = config_info.config.name.clone();
        all_configs.insert(name, config_info);
    }

    // Find project-specific agents directory
    if let Ok(current_dir) = std::env::current_dir() {
        for ancestor in current_dir.ancestors() {
            let project_agents_dir = ancestor.join(".agcodex").join("agents");
            if project_agents_dir.exists() && project_agents_dir.is_dir() {
                // Load project agents (overrides global agents with same name)
                let project_configs = load_yaml_configs_from_directory(&project_agents_dir, false)?;
                for config_info in project_configs {
                    let name = config_info.config.name.clone();
                    if all_configs.contains_key(&name) {
                        info!("Project agent '{}' overrides global agent", name);
                    }
                    all_configs.insert(name, config_info);
                }
                break;
            }
        }
    }

    Ok(all_configs)
}

/// Validate a YAML file without fully loading it
pub fn validate_yaml_file(path: &Path) -> Result<(), SubagentError> {
    let content = std::fs::read_to_string(path)?;
    let yaml_config: YamlSubagentConfig = serde_yaml::from_str(&content)
        .map_err(|e| SubagentError::InvalidConfig(format!("Invalid YAML structure: {}", e)))?;

    let config = yaml_config.into_config()?;
    config.validate()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_simple_yaml_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test-agent.yaml");

        let yaml_content = r#"
name: test-agent
description: A test agent
mode_override: review
intelligence: hard
tools:
  - name: read
    permission: allow
  - name: write
    permission: deny
prompt: |
  You are a test agent.
  This is a multi-line prompt.
tags:
  - test
  - example
"#;

        fs::write(&config_path, yaml_content).unwrap();

        let config = load_yaml_config(&config_path).unwrap();
        assert_eq!(config.name, "test-agent");
        assert_eq!(config.description, "A test agent");
        assert_eq!(
            config.mode_override,
            Some(crate::modes::OperatingMode::Review)
        );
        assert_eq!(config.intelligence, IntelligenceLevel::Hard);
        assert!(config.is_tool_allowed("read"));
        assert!(!config.is_tool_allowed("write"));
        assert!(config.prompt.contains("multi-line"));
        assert_eq!(config.tags, vec!["test", "example"]);
    }

    #[test]
    fn test_load_yaml_with_restricted_tools() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("restricted-agent.yaml");

        let yaml_content = r#"
name: restricted-agent
description: Agent with restricted tools
tools:
  - name: search
    permission: restricted
    restrictions:
      max_files: "10"
      file_types: "*.rs,*.py"
prompt: Test prompt
"#;

        fs::write(&config_path, yaml_content).unwrap();

        let config = load_yaml_config(&config_path).unwrap();
        assert_eq!(config.name, "restricted-agent");
        assert!(config.is_tool_allowed("search"));

        let restrictions = config.get_tool_restrictions("search").unwrap();
        assert_eq!(restrictions.get("max_files").unwrap(), "10");
        assert_eq!(restrictions.get("file_types").unwrap(), "*.rs,*.py");
    }

    #[test]
    fn test_invalid_yaml_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid.yaml");

        // Missing required fields
        let yaml_content = r#"
description: Missing name field
prompt: Test
"#;

        fs::write(&config_path, yaml_content).unwrap();
        let result = load_yaml_config(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_directory_with_multiple_configs() {
        let temp_dir = TempDir::new().unwrap();

        // Create multiple YAML files
        let yaml1 = r#"
name: agent1
description: First agent
prompt: Agent 1 prompt
"#;

        let yaml2 = r#"
name: agent2
description: Second agent
prompt: Agent 2 prompt
"#;

        let yaml3 = r#"
name: agent3
description: Third agent
prompt: Agent 3 prompt
"#;

        fs::write(temp_dir.path().join("agent1.yaml"), yaml1).unwrap();
        fs::write(temp_dir.path().join("agent2.yml"), yaml2).unwrap();
        fs::write(temp_dir.path().join("agent3.yaml"), yaml3).unwrap();

        // Also create a non-YAML file that should be ignored
        fs::write(temp_dir.path().join("not-an-agent.txt"), "ignored").unwrap();

        let configs = load_yaml_configs_from_directory(temp_dir.path(), false).unwrap();
        assert_eq!(configs.len(), 3);

        let names: Vec<String> = configs.iter().map(|c| c.config.name.clone()).collect();
        assert!(names.contains(&"agent1".to_string()));
        assert!(names.contains(&"agent2".to_string()));
        assert!(names.contains(&"agent3".to_string()));
    }

    #[test]
    fn test_yaml_with_parameters() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("param-agent.yaml");

        let yaml_content = r#"
name: param-agent
description: Agent with parameters
prompt: Test prompt
parameters:
  - name: target_file
    description: File to analyze
    required: true
  - name: output_format
    description: Output format
    required: false
    default: markdown
    valid_values:
      - markdown
      - json
      - plain
"#;

        fs::write(&config_path, yaml_content).unwrap();

        let config = load_yaml_config(&config_path).unwrap();
        assert_eq!(config.parameters.len(), 2);

        let param1 = &config.parameters[0];
        assert_eq!(param1.name, "target_file");
        assert!(param1.required);

        let param2 = &config.parameters[1];
        assert_eq!(param2.name, "output_format");
        assert!(!param2.required);
        assert_eq!(param2.default, Some("markdown".to_string()));
        assert_eq!(param2.valid_values.as_ref().unwrap().len(), 3);
    }
}
