//! Subagent registry for loading and managing agent configurations
//!
//! The registry loads agent configurations from TOML files and provides
//! hot-reload capabilities for dynamic agent management.

use super::config::SubagentConfig;
use super::config::SubagentTemplate;
use super::yaml_loader;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::SystemTime;
use thiserror::Error;
use walkdir::WalkDir;

/// Result type for registry operations
pub type RegistryResult<T> = std::result::Result<T, SubagentRegistryError>;

/// Errors specific to the subagent registry
#[derive(Error, Debug)]
pub enum SubagentRegistryError {
    #[error("agent configuration file not found: {path}")]
    ConfigNotFound { path: PathBuf },

    #[error("template not found: {name}")]
    TemplateNotFound { name: String },

    #[error("invalid agent directory: {path}")]
    InvalidDirectory { path: PathBuf },

    #[error("failed to load configuration: {path}: {error}")]
    LoadError { path: PathBuf, error: String },

    #[error("agent name conflict: {name} (paths: {path1}, {path2})")]
    NameConflict {
        name: String,
        path1: PathBuf,
        path2: PathBuf,
    },

    #[error("template inheritance loop detected: {chain:?}")]
    InheritanceLoop { chain: Vec<String> },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("configuration error: {0}")]
    Config(#[from] super::SubagentError),
}

/// Information about a loaded agent configuration
#[derive(Debug, Clone)]
pub struct AgentInfo {
    /// The agent configuration
    pub config: SubagentConfig,

    /// Path to the configuration file
    pub config_path: PathBuf,

    /// Last modification time of the configuration file
    pub last_modified: SystemTime,

    /// Whether this is a global or project-specific agent
    pub is_global: bool,
}

/// Information about a loaded template
#[derive(Debug, Clone)]
pub struct TemplateInfo {
    /// The template configuration
    pub template: SubagentTemplate,

    /// Path to the template file
    pub template_path: PathBuf,

    /// Last modification time of the template file
    pub last_modified: SystemTime,
}

/// Registry for managing subagent configurations
#[derive(Debug)]
pub struct SubagentRegistry {
    /// Global agents directory (~/.agcodex/agents/global/)
    global_agents_dir: PathBuf,

    /// Project-specific agents directory (./.agcodex/agents/)
    project_agents_dir: Option<PathBuf>,

    /// Templates directory (~/.agcodex/agents/templates/)
    templates_dir: PathBuf,

    /// Loaded agent configurations
    agents: Arc<Mutex<HashMap<String, AgentInfo>>>,

    /// Executable agents (for built-in and programmatic agents)
    executable_agents: Arc<Mutex<HashMap<String, Arc<dyn super::agents::Subagent>>>>,

    /// Loaded templates
    templates: Arc<Mutex<HashMap<String, TemplateInfo>>>,

    /// Whether to watch for file changes
    _watch_enabled: bool,

    /// Last full scan time
    last_scan: Arc<Mutex<SystemTime>>,
}

impl SubagentRegistry {
    
    /// Register an executable agent
    pub fn register_executable_agent(
        &self,
        name: String,
        agent: Arc<dyn super::agents::Subagent>,
    ) -> RegistryResult<()> {
        let mut agents = self.executable_agents.lock().map_err(|e| {
            SubagentRegistryError::LoadError {
                path: PathBuf::from("memory"),
                error: format!("Poison error: {}", e),
            }
        })?;
        agents.insert(name, agent);
        Ok(())
    }
    
    /// Get an agent configuration by name
    pub fn get_agent_config(&self, name: &str) -> Option<SubagentConfig> {
        self.agents
            .lock()
            .ok()
            .and_then(|agents| agents.get(name).map(|info| info.config.clone()))
    }
    
    /// Create a new subagent registry
    pub fn new() -> RegistryResult<Self> {
        let home_dir = dirs::home_dir().ok_or_else(|| SubagentRegistryError::InvalidDirectory {
            path: PathBuf::from("~"),
        })?;

        let global_agents_dir = home_dir.join(".agcodex").join("agents").join("global");
        let templates_dir = home_dir.join(".agcodex").join("agents").join("templates");

        // Try to find project-specific agents directory
        let project_agents_dir = Self::find_project_agents_dir()?;

        let registry = Self {
            global_agents_dir,
            project_agents_dir,
            templates_dir,
            agents: Arc::new(Mutex::new(HashMap::new())),
            executable_agents: Arc::new(Mutex::new(HashMap::new())),
            templates: Arc::new(Mutex::new(HashMap::new())),
            _watch_enabled: true,
            last_scan: Arc::new(Mutex::new(SystemTime::UNIX_EPOCH)),
        };

        // Create directories if they don't exist
        registry.ensure_directories()?;

        Ok(registry)
    }

    /// Create a new subagent registry with explicit paths (for testing)
    #[cfg(test)]
    pub fn new_with_paths(
        global_agents_dir: PathBuf,
        project_agents_dir: Option<PathBuf>,
        templates_dir: PathBuf,
    ) -> RegistryResult<Self> {
        let registry = Self {
            global_agents_dir,
            project_agents_dir,
            templates_dir,
            agents: Arc::new(Mutex::new(HashMap::new())),
            executable_agents: Arc::new(Mutex::new(HashMap::new())),
            templates: Arc::new(Mutex::new(HashMap::new())),
            _watch_enabled: false, // Disable watching for tests
            last_scan: Arc::new(Mutex::new(SystemTime::UNIX_EPOCH)),
        };

        // Create directories if they don't exist
        registry.ensure_directories()?;

        Ok(registry)
    }

    /// Find the project-specific agents directory by walking up from current directory
    fn find_project_agents_dir() -> RegistryResult<Option<PathBuf>> {
        let current_dir = std::env::current_dir()?;

        for ancestor in current_dir.ancestors() {
            let agents_dir = ancestor.join(".agcodex").join("agents");
            if agents_dir.exists() && agents_dir.is_dir() {
                return Ok(Some(agents_dir));
            }
        }

        Ok(None)
    }

    /// Ensure that all required directories exist
    fn ensure_directories(&self) -> RegistryResult<()> {
        std::fs::create_dir_all(&self.global_agents_dir)?;
        std::fs::create_dir_all(&self.templates_dir)?;

        if let Some(ref project_dir) = self.project_agents_dir {
            std::fs::create_dir_all(project_dir)?;
        }

        Ok(())
    }

    /// Load all agent configurations and templates
    pub fn load_all(&self) -> RegistryResult<()> {
        self.load_templates()?;
        self.load_agents()?;

        *self
            .last_scan
            .lock()
            .map_err(|e| SubagentRegistryError::LoadError {
                path: PathBuf::from("memory"),
                error: format!("Poison error on last_scan mutex: {}", e),
            })? = SystemTime::now();

        Ok(())
    }
    
    /// Reload all agent configurations and templates
    pub fn reload(&self) -> RegistryResult<()> {
        // Clear existing agents (but keep executable agents as they're programmatically registered)
        self.agents
            .lock()
            .map_err(|e| SubagentRegistryError::LoadError {
                path: PathBuf::from("memory"),
                error: format!("Poison error on agents mutex: {}", e),
            })?
            .clear();
        
        // Clear templates
        self.templates
            .lock()
            .map_err(|e| SubagentRegistryError::LoadError {
                path: PathBuf::from("memory"),
                error: format!("Poison error on templates mutex: {}", e),
            })?
            .clear();
        
        // Reload everything
        self.load_all()
    }

    /// Load all templates from the templates directory
    fn load_templates(&self) -> RegistryResult<()> {
        let mut templates =
            self.templates
                .lock()
                .map_err(|e| SubagentRegistryError::LoadError {
                    path: PathBuf::from("memory"),
                    error: format!("Poison error on templates mutex: {}", e),
                })?;
        templates.clear();

        if !self.templates_dir.exists() {
            return Ok(());
        }

        for entry in WalkDir::new(&self.templates_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "toml" || ext == "yaml" || ext == "yml")
                    .unwrap_or(false)
            })
        {
            let path = entry.path();

            match self.load_template_from_file(path) {
                Ok(template_info) => {
                    let name = template_info.template.name.clone();
                    templates.insert(name, template_info);
                }
                Err(e) => {
                    tracing::warn!("Failed to load template from {}: {}", path.display(), e);
                }
            }
        }

        Ok(())
    }

    /// Load a single template from a file (supports both TOML and YAML)
    fn load_template_from_file(&self, path: &Path) -> RegistryResult<TemplateInfo> {
        let metadata = std::fs::metadata(path)?;
        let last_modified = metadata.modified()?;

        // Note: Currently templates are only supported in TOML format
        // YAML template support could be added in the future if needed
        let template = SubagentTemplate::from_file(&path.to_path_buf()).map_err(|e| {
            SubagentRegistryError::LoadError {
                path: path.to_path_buf(),
                error: e.to_string(),
            }
        })?;

        Ok(TemplateInfo {
            template,
            template_path: path.to_path_buf(),
            last_modified,
        })
    }

    /// Load all agent configurations
    fn load_agents(&self) -> RegistryResult<()> {
        let mut agents = self
            .agents
            .lock()
            .map_err(|_| SubagentRegistryError::LoadError {
                path: PathBuf::from("memory"),
                error: "Poison error on agents mutex".to_string(),
            })?;
        agents.clear();

        // Load global agents (both TOML and YAML)
        self.load_agents_from_directory(&self.global_agents_dir, true, &mut agents)?;

        // Load project-specific agents (both TOML and YAML)
        if let Some(ref project_dir) = self.project_agents_dir {
            self.load_agents_from_directory(project_dir, false, &mut agents)?;
        }

        // Resolve template inheritance
        self.resolve_template_inheritance(&mut agents)?;

        Ok(())
    }

    /// Load agents from a specific directory
    fn load_agents_from_directory(
        &self,
        dir: &Path,
        is_global: bool,
        agents: &mut HashMap<String, AgentInfo>,
    ) -> RegistryResult<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in WalkDir::new(dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "toml" || ext == "yaml" || ext == "yml")
                    .unwrap_or(false)
            })
        {
            let path = entry.path();

            match self.load_agent_from_file(path, is_global) {
                Ok(agent_info) => {
                    let name = agent_info.config.name.clone();

                    // Check for name conflicts
                    if let Some(existing) = agents.get(&name) {
                        // Allow project configs to override global configs
                        if is_global || !existing.is_global {
                            // Error if:
                            // 1. We're loading a global config and there's any conflict
                            // 2. We're loading a project config and the existing is also project (not global)
                            return Err(SubagentRegistryError::NameConflict {
                                name,
                                path1: existing.config_path.clone(),
                                path2: path.to_path_buf(),
                            });
                        }
                        // If we're loading a project config and existing is global, allow override
                        tracing::info!("Project agent '{}' overrides global agent", name);
                    }

                    agents.insert(name, agent_info);
                }
                Err(e) => {
                    tracing::warn!("Failed to load agent from {}: {}", path.display(), e);
                }
            }
        }

        Ok(())
    }

    /// Load a single agent configuration from a file (supports both TOML and YAML)
    fn load_agent_from_file(&self, path: &Path, is_global: bool) -> RegistryResult<AgentInfo> {
        let metadata = std::fs::metadata(path)?;
        let last_modified = metadata.modified()?;

        // Determine file type and load accordingly
        let config = match path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
            .as_deref()
        {
            Some("yaml") | Some("yml") => {
                // Load YAML configuration
                yaml_loader::load_yaml_config(path).map_err(|e| {
                    SubagentRegistryError::LoadError {
                        path: path.to_path_buf(),
                        error: e.to_string(),
                    }
                })?
            }
            Some("toml") | None => {
                // Load TOML configuration (default)
                SubagentConfig::from_file(&path.to_path_buf()).map_err(|e| {
                    SubagentRegistryError::LoadError {
                        path: path.to_path_buf(),
                        error: e.to_string(),
                    }
                })?
            }
            Some(ext) => {
                return Err(SubagentRegistryError::LoadError {
                    path: path.to_path_buf(),
                    error: format!("Unsupported file extension: {}", ext),
                });
            }
        };

        Ok(AgentInfo {
            config,
            config_path: path.to_path_buf(),
            last_modified,
            is_global,
        })
    }

    /// Resolve template inheritance for all agents
    fn resolve_template_inheritance(
        &self,
        agents: &mut HashMap<String, AgentInfo>,
    ) -> RegistryResult<()> {
        let templates = self
            .templates
            .lock()
            .map_err(|e| SubagentRegistryError::LoadError {
                path: PathBuf::from("memory"),
                error: format!("Poison error on templates mutex: {}", e),
            })?;

        for agent_info in agents.values_mut() {
            if let Some(ref template_name) = agent_info.config.template {
                let mut inheritance_chain = Vec::new();
                let mut current_template = template_name.clone();

                // Follow the inheritance chain
                loop {
                    if inheritance_chain.contains(&current_template) {
                        return Err(SubagentRegistryError::InheritanceLoop {
                            chain: inheritance_chain,
                        });
                    }

                    inheritance_chain.push(current_template.clone());

                    let template = templates.get(&current_template).ok_or_else(|| {
                        SubagentRegistryError::TemplateNotFound {
                            name: current_template.clone(),
                        }
                    })?;

                    // Apply template to the agent config
                    self.apply_template_to_config(&template.template, &mut agent_info.config)?;

                    // Check if this template inherits from another
                    if let Some(ref parent_template) = template.template.config.template {
                        current_template = parent_template.clone();
                    } else {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// Apply a template to an agent configuration
    fn apply_template_to_config(
        &self,
        template: &SubagentTemplate,
        config: &mut SubagentConfig,
    ) -> RegistryResult<()> {
        // Merge template configuration with agent configuration
        // Agent-specific values take precedence

        if config.description.is_empty() {
            config.description = template.config.description.clone();
        }

        if config.mode_override.is_none() {
            config.mode_override = template.config.mode_override;
        }

        if config.prompt.is_empty() {
            config.prompt = template.config.prompt.clone();
        }

        // Merge tools (agent tools override template tools)
        for (tool, permission) in &template.config.tools {
            config
                .tools
                .entry(tool.clone())
                .or_insert_with(|| permission.clone());
        }

        // Merge parameters (template parameters as defaults)
        for template_param in &template.config.parameters {
            if !config
                .parameters
                .iter()
                .any(|p| p.name == template_param.name)
            {
                config.parameters.push(template_param.clone());
            }
        }

        // Merge metadata
        for (key, value) in &template.config.metadata {
            config
                .metadata
                .entry(key.clone())
                .or_insert_with(|| value.clone());
        }

        // Merge file patterns
        for pattern in &template.config.file_patterns {
            if !config.file_patterns.contains(pattern) {
                config.file_patterns.push(pattern.clone());
            }
        }

        // Merge tags
        for tag in &template.config.tags {
            if !config.tags.contains(tag) {
                config.tags.push(tag.clone());
            }
        }

        Ok(())
    }

    /// Register an agent programmatically (for built-in agents)
    pub fn register(
        &self,
        name: &str,
        agent: Arc<dyn super::agents::Subagent>,
    ) -> RegistryResult<()> {
        // Create a minimal SubagentConfig for built-in agents
        let config = SubagentConfig {
            name: name.to_string(),
            description: format!("Built-in {} agent", name),
            mode_override: None,
            intelligence: super::config::IntelligenceLevel::Medium,
            tools: HashMap::new(),
            prompt: String::new(),
            parameters: vec![],
            template: None,
            timeout_seconds: 300,
            chainable: true,
            parallelizable: true,
            metadata: HashMap::new(),
            file_patterns: vec![],
            tags: vec![],
        };

        let info = AgentInfo {
            config,
            config_path: PathBuf::new(),
            last_modified: SystemTime::now(),
            is_global: false,
        };

        // Store both configuration and executable agent
        let mut agents = self
            .agents
            .lock()
            .map_err(|_| SubagentRegistryError::LoadError {
                path: PathBuf::from("memory"),
                error: "Poison error on agents mutex".to_string(),
            })?;
        agents.insert(name.to_string(), info);
        drop(agents);

        let mut executable_agents =
            self.executable_agents
                .lock()
                .map_err(|e| SubagentRegistryError::LoadError {
                    path: PathBuf::from("memory"),
                    error: format!("Poison error on executable_agents mutex: {}", e),
                })?;
        executable_agents.insert(name.to_string(), agent);
        Ok(())
    }

    /// Get an agent configuration by name
    pub fn get_agent(&self, name: &str) -> Option<AgentInfo> {
        self.agents.lock().ok()?.get(name).cloned()
    }

    /// Get an executable agent by name
    pub fn get_executable_agent(&self, name: &str) -> Option<Arc<dyn super::agents::Subagent>> {
        self.executable_agents.lock().ok()?.get(name).cloned()
    }

    /// Get all loaded agents
    pub fn get_all_agents(&self) -> HashMap<String, AgentInfo> {
        match self.agents.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// Get a template by name
    pub fn get_template(&self, name: &str) -> Option<TemplateInfo> {
        self.templates.lock().ok()?.get(name).cloned()
    }

    /// Get all loaded templates
    pub fn get_all_templates(&self) -> HashMap<String, TemplateInfo> {
        match self.templates.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// Check if any configurations have been modified and reload if necessary
    pub fn check_for_updates(&self) -> RegistryResult<bool> {
        let mut updated = false;

        // Check templates
        {
            let templates =
                self.templates
                    .lock()
                    .map_err(|_| SubagentRegistryError::LoadError {
                        path: PathBuf::from("memory"),
                        error: "Poison error on templates mutex".to_string(),
                    })?;
            for template_info in templates.values() {
                if let Ok(metadata) = std::fs::metadata(&template_info.template_path)
                    && let Ok(modified) = metadata.modified()
                    && modified > template_info.last_modified
                {
                    updated = true;
                    break;
                }
            }
        }

        // Check agents
        if !updated {
            let agents = self
                .agents
                .lock()
                .map_err(|_| SubagentRegistryError::LoadError {
                    path: PathBuf::from("memory"),
                    error: "Poison error on agents mutex".to_string(),
                })?;
            for agent_info in agents.values() {
                if let Ok(metadata) = std::fs::metadata(&agent_info.config_path)
                    && let Ok(modified) = metadata.modified()
                    && modified > agent_info.last_modified
                {
                    updated = true;
                    break;
                }
            }
        }

        // Reload if updates detected
        if updated {
            self.load_all()?;
        }

        Ok(updated)
    }

    /// Get agents that match a specific file pattern
    pub fn get_agents_for_file(&self, file_path: &Path) -> Vec<AgentInfo> {
        self.agents
            .lock()
            .ok()
            .map(|agents| {
                agents
                    .values()
                    .filter(|agent| agent.config.matches_file(file_path))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get agents with specific tags
    pub fn get_agents_with_tags(&self, tags: &[String]) -> Vec<AgentInfo> {
        self.agents
            .lock()
            .ok()
            .map(|agents| {
                agents
                    .values()
                    .filter(|agent| tags.iter().any(|tag| agent.config.tags.contains(tag)))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Load YAML configurations explicitly (useful for testing and debugging)
    pub fn load_yaml_configs(&self) -> RegistryResult<()> {
        let mut agents = self
            .agents
            .lock()
            .map_err(|_| SubagentRegistryError::LoadError {
                path: PathBuf::from("memory"),
                error: "Poison error on agents mutex".to_string(),
            })?;

        // Load YAML configs from global directory
        let yaml_configs =
            yaml_loader::load_yaml_configs_from_directory(&self.global_agents_dir, true).map_err(
                |e| SubagentRegistryError::LoadError {
                    path: self.global_agents_dir.clone(),
                    error: e.to_string(),
                },
            )?;

        for yaml_info in yaml_configs {
            let name = yaml_info.config.name.clone();

            // Check for name conflicts
            if let Some(existing) = agents.get(&name)
                && !yaml_info.source_path.eq(&existing.config_path)
            {
                return Err(SubagentRegistryError::NameConflict {
                    name,
                    path1: existing.config_path.clone(),
                    path2: yaml_info.source_path,
                });
            }

            let agent_info = AgentInfo {
                config: yaml_info.config,
                config_path: yaml_info.source_path.clone(),
                last_modified: std::fs::metadata(&yaml_info.source_path)
                    .and_then(|m| m.modified())
                    .unwrap_or_else(|_| SystemTime::now()),
                is_global: yaml_info.is_global,
            };

            agents.insert(name, agent_info);
        }

        // Load YAML configs from project directory if present
        if let Some(ref project_dir) = self.project_agents_dir {
            let project_yaml_configs =
                yaml_loader::load_yaml_configs_from_directory(project_dir, false).map_err(|e| {
                    SubagentRegistryError::LoadError {
                        path: project_dir.clone(),
                        error: e.to_string(),
                    }
                })?;

            for yaml_info in project_yaml_configs {
                let name = yaml_info.config.name.clone();

                // Project configs override global configs
                if agents.contains_key(&name) {
                    tracing::info!("Project YAML agent '{}' overrides global agent", name);
                }

                let agent_info = AgentInfo {
                    config: yaml_info.config,
                    config_path: yaml_info.source_path.clone(),
                    last_modified: std::fs::metadata(&yaml_info.source_path)
                        .and_then(|m| m.modified())
                        .unwrap_or_else(|_| SystemTime::now()),
                    is_global: yaml_info.is_global,
                };

                agents.insert(name, agent_info);
            }
        }

        drop(agents);

        // Resolve template inheritance after loading
        let mut agents = self
            .agents
            .lock()
            .map_err(|_| SubagentRegistryError::LoadError {
                path: PathBuf::from("memory"),
                error: "Poison error on agents mutex".to_string(),
            })?;
        self.resolve_template_inheritance(&mut agents)?;

        Ok(())
    }

    /// Create default agent configurations
    pub fn create_default_agents(&self) -> RegistryResult<()> {
        self.ensure_directories()?;

        // Create default code reviewer agent
        let code_reviewer = SubagentConfig {
            name: "code-reviewer".to_string(),
            description: "Proactive code quality analysis and security review".to_string(),
            mode_override: Some(crate::modes::OperatingMode::Review),
            intelligence: crate::subagents::IntelligenceLevel::Hard,
            tools: std::collections::HashMap::new(),
            prompt: r#"You are a senior code reviewer with AST-based analysis capabilities.

Focus on:
- Syntactic correctness via tree-sitter validation
- Security vulnerabilities (OWASP Top 10)
- Performance bottlenecks (O(n²) or worse)
- Memory leaks and resource management
- Error handling completeness
- Code quality and maintainability

Use AST-powered semantic search to understand code structure and relationships."#
                .to_string(),
            parameters: vec![
                super::config::ParameterDefinition {
                    name: "files".to_string(),
                    description: "Files or patterns to review".to_string(),
                    required: false,
                    default: Some("**/*.rs".to_string()),
                    valid_values: None,
                },
                super::config::ParameterDefinition {
                    name: "focus".to_string(),
                    description: "Focus area (security, performance, quality)".to_string(),
                    required: false,
                    default: Some("quality".to_string()),
                    valid_values: Some(vec![
                        "security".to_string(),
                        "performance".to_string(),
                        "quality".to_string(),
                    ]),
                },
            ],
            template: None,
            timeout_seconds: 600,
            chainable: true,
            parallelizable: true,
            metadata: std::collections::HashMap::new(),
            file_patterns: vec![
                "*.rs".to_string(),
                "*.py".to_string(),
                "*.js".to_string(),
                "*.ts".to_string(),
            ],
            tags: vec![
                "review".to_string(),
                "quality".to_string(),
                "security".to_string(),
            ],
        };

        let config_path = self.global_agents_dir.join("code-reviewer.toml");
        if !config_path.exists() {
            code_reviewer.to_file(&config_path)?;
        }

        Ok(())
    }
}

// Note: No Default implementation since new() can fail.
// Use SubagentRegistry::new() explicitly to handle potential errors.

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_registry() -> (SubagentRegistry, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let global_dir = temp_dir.path().join("global");
        let templates_dir = temp_dir.path().join("templates");

        std::fs::create_dir_all(&global_dir).unwrap();
        std::fs::create_dir_all(&templates_dir).unwrap();

        let registry = SubagentRegistry {
            global_agents_dir: global_dir,
            project_agents_dir: None,
            templates_dir,
            agents: Arc::new(Mutex::new(HashMap::new())),
            executable_agents: Arc::new(Mutex::new(HashMap::new())),
            templates: Arc::new(Mutex::new(HashMap::new())),
            _watch_enabled: false,
            last_scan: Arc::new(Mutex::new(SystemTime::UNIX_EPOCH)),
        };

        (registry, temp_dir)
    }

    #[test]
    fn test_registry_creation() {
        let (registry, _temp_dir) = create_test_registry();
        assert!(registry.global_agents_dir.exists());
        assert!(registry.templates_dir.exists());
    }

    #[test]
    fn test_agent_loading() {
        let (registry, _temp_dir) = create_test_registry();

        // Create a test agent configuration
        let config = SubagentConfig {
            name: "test-agent".to_string(),
            description: "Test agent".to_string(),
            mode_override: None,
            intelligence: crate::subagents::IntelligenceLevel::Medium,
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

        let config_path = registry.global_agents_dir.join("test-agent.toml");
        config.to_file(&config_path).unwrap();

        // Load agents
        registry.load_all().unwrap();

        // Verify agent was loaded
        let loaded_agent = registry.get_agent("test-agent").unwrap();
        assert_eq!(loaded_agent.config.name, "test-agent");
        assert!(loaded_agent.is_global);
    }

    #[test]
    fn test_template_inheritance() {
        let (registry, _temp_dir) = create_test_registry();

        // Create a template
        let template = SubagentTemplate {
            name: "base-reviewer".to_string(),
            description: "Base template for reviewers".to_string(),
            config: SubagentConfig {
                name: "template".to_string(),
                description: "Template description".to_string(),
                mode_override: Some(crate::modes::OperatingMode::Review),
                intelligence: crate::subagents::IntelligenceLevel::Hard,
                tools: HashMap::new(),
                prompt: "You are a reviewer.".to_string(),
                parameters: vec![],
                template: None,
                timeout_seconds: 300,
                chainable: true,
                parallelizable: true,
                metadata: HashMap::new(),
                file_patterns: vec!["*.rs".to_string()],
                tags: vec!["review".to_string()],
            },
            placeholders: vec![],
        };

        let template_path = registry.templates_dir.join("base-reviewer.toml");
        let template_content = toml::to_string(&template).unwrap();
        std::fs::write(&template_path, template_content).unwrap();

        // Create an agent that inherits from the template
        let agent_config = SubagentConfig {
            name: "code-reviewer".to_string(),
            description: "".to_string(), // Will inherit from template
            mode_override: None,
            intelligence: crate::subagents::IntelligenceLevel::Medium,
            tools: HashMap::new(),
            prompt: "".to_string(), // Will inherit from template
            parameters: vec![],
            template: Some("base-reviewer".to_string()),
            timeout_seconds: 300,
            chainable: true,
            parallelizable: true,
            metadata: HashMap::new(),
            file_patterns: vec![],
            tags: vec![],
        };

        let config_path = registry.global_agents_dir.join("code-reviewer.toml");
        agent_config.to_file(&config_path).unwrap();

        // Load all configurations
        registry.load_all().unwrap();

        // Verify inheritance was applied
        let loaded_agent = registry.get_agent("code-reviewer").unwrap();
        assert_eq!(loaded_agent.config.description, "Template description");
        assert_eq!(loaded_agent.config.prompt, "You are a reviewer.");
        assert_eq!(
            loaded_agent.config.mode_override,
            Some(crate::modes::OperatingMode::Review)
        );
        assert!(
            loaded_agent
                .config
                .file_patterns
                .contains(&"*.rs".to_string())
        );
        assert!(loaded_agent.config.tags.contains(&"review".to_string()));
    }
}
