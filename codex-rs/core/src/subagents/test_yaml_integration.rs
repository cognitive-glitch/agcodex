//! Integration tests for YAML configuration loading

#[cfg(test)]
mod tests {
    use crate::modes::OperatingMode;
    use crate::subagents::config::IntelligenceLevel;
    use crate::subagents::registry::SubagentRegistry;
    use crate::subagents::yaml_loader;

    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_yaml_integration_with_registry() {
        let temp_dir = TempDir::new().unwrap();
        let global_dir = temp_dir.path().join("global");
        let project_dir = temp_dir.path().join("project");
        let templates_dir = temp_dir.path().join("templates");

        fs::create_dir_all(&global_dir).unwrap();
        fs::create_dir_all(&project_dir).unwrap();
        fs::create_dir_all(&templates_dir).unwrap();

        // Create a global YAML agent
        let global_yaml = r#"
name: code-reviewer
description: Reviews code for quality and security
mode_override: review
intelligence: hard
tools:
  - name: read
    permission: allow
  - name: search
    permission: allow
  - name: edit
    permission: deny
prompt: |
  You are a code reviewer focused on:
  - Security vulnerabilities
  - Performance issues
  - Code quality
tags:
  - review
  - security
  - quality
file_patterns:
  - "*.rs"
  - "*.py"
"#;

        fs::write(global_dir.join("code-reviewer.yaml"), global_yaml).unwrap();

        // Create a project-specific YAML agent
        let project_yaml = r#"
name: test-writer
description: Writes comprehensive tests
mode_override: build
intelligence: medium
tools:
  - name: read
    permission: allow
  - name: write
    permission: allow
  - name: execute
    permission: restricted
    restrictions:
      command_pattern: "cargo test*"
      max_duration: "300"
prompt: |
  You are a test writer that creates:
  - Unit tests
  - Integration tests
  - Property-based tests
parameters:
  - name: test_framework
    description: Testing framework to use
    required: false
    default: "pytest"
    valid_values:
      - pytest
      - unittest
      - cargo
tags:
  - testing
  - automation
"#;

        fs::write(project_dir.join("test-writer.yml"), project_yaml).unwrap();

        // Create registry and load configurations
        let registry = SubagentRegistry::new_with_paths(
            global_dir.clone(),
            Some(project_dir.clone()),
            templates_dir,
        )
        .unwrap();

        registry.load_all().unwrap();

        // Verify code-reviewer agent was loaded
        let code_reviewer = registry.get_agent("code-reviewer").unwrap();
        assert_eq!(code_reviewer.config.name, "code-reviewer");
        assert_eq!(
            code_reviewer.config.mode_override,
            Some(OperatingMode::Review)
        );
        assert_eq!(code_reviewer.config.intelligence, IntelligenceLevel::Hard);
        assert!(code_reviewer.config.is_tool_allowed("read"));
        assert!(code_reviewer.config.is_tool_allowed("search"));
        assert!(!code_reviewer.config.is_tool_allowed("edit"));
        assert!(code_reviewer.config.tags.contains(&"security".to_string()));
        assert!(
            code_reviewer
                .config
                .file_patterns
                .contains(&"*.rs".to_string())
        );
        assert!(code_reviewer.is_global);

        // Verify test-writer agent was loaded
        let test_writer = registry.get_agent("test-writer").unwrap();
        assert_eq!(test_writer.config.name, "test-writer");
        assert_eq!(test_writer.config.mode_override, Some(OperatingMode::Build));
        assert_eq!(test_writer.config.intelligence, IntelligenceLevel::Medium);
        assert!(test_writer.config.is_tool_allowed("read"));
        assert!(test_writer.config.is_tool_allowed("write"));
        assert!(test_writer.config.is_tool_allowed("execute"));

        // Check tool restrictions
        let restrictions = test_writer.config.get_tool_restrictions("execute").unwrap();
        assert_eq!(restrictions.get("command_pattern").unwrap(), "cargo test*");
        assert_eq!(restrictions.get("max_duration").unwrap(), "300");

        // Check parameters
        assert_eq!(test_writer.config.parameters.len(), 1);
        let param = &test_writer.config.parameters[0];
        assert_eq!(param.name, "test_framework");
        assert_eq!(param.default, Some("pytest".to_string()));

        assert!(!test_writer.is_global); // Project-specific
    }

    #[test]
    fn test_yaml_override_behavior() {
        let temp_dir = TempDir::new().unwrap();
        let global_dir = temp_dir.path().join("global");
        let project_dir = temp_dir.path().join("project");
        let templates_dir = temp_dir.path().join("templates");

        fs::create_dir_all(&global_dir).unwrap();
        fs::create_dir_all(&project_dir).unwrap();
        fs::create_dir_all(&templates_dir).unwrap();

        // Create a global agent
        let global_yaml = r#"
name: analyzer
description: Global analyzer
intelligence: light
prompt: Global prompt
tags:
  - global
"#;

        fs::write(global_dir.join("analyzer.yaml"), global_yaml).unwrap();

        // Create a project agent with the same name (should override)
        let project_yaml = r#"
name: analyzer
description: Project-specific analyzer
intelligence: hard
prompt: Project-specific prompt
tags:
  - project
  - override
"#;

        fs::write(project_dir.join("analyzer.yaml"), project_yaml).unwrap();

        // Create registry and load configurations
        let registry =
            SubagentRegistry::new_with_paths(global_dir, Some(project_dir), templates_dir).unwrap();

        registry.load_all().unwrap();

        // Verify project agent overrides global agent
        let analyzer = registry.get_agent("analyzer").unwrap();
        assert_eq!(analyzer.config.description, "Project-specific analyzer");
        assert_eq!(analyzer.config.intelligence, IntelligenceLevel::Hard);
        assert!(analyzer.config.tags.contains(&"project".to_string()));
        assert!(analyzer.config.tags.contains(&"override".to_string()));
        assert!(!analyzer.config.tags.contains(&"global".to_string()));
        assert!(!analyzer.is_global); // Should be marked as project-specific
    }

    #[test]
    fn test_mixed_toml_yaml_loading() {
        let temp_dir = TempDir::new().unwrap();
        let global_dir = temp_dir.path().join("global");
        let templates_dir = temp_dir.path().join("templates");

        fs::create_dir_all(&global_dir).unwrap();
        fs::create_dir_all(&templates_dir).unwrap();

        // Create a YAML agent
        let yaml_content = r#"
name: yaml-agent
description: Agent from YAML
prompt: YAML prompt
"#;

        fs::write(global_dir.join("yaml-agent.yaml"), yaml_content).unwrap();

        // Create a TOML agent
        let toml_content = r#"
name = "toml-agent"
description = "Agent from TOML"
prompt = "TOML prompt"
"#;

        fs::write(global_dir.join("toml-agent.toml"), toml_content).unwrap();

        // Create registry and load configurations
        let registry = SubagentRegistry::new_with_paths(global_dir, None, templates_dir).unwrap();

        registry.load_all().unwrap();

        // Verify both agents were loaded
        let yaml_agent = registry.get_agent("yaml-agent").unwrap();
        assert_eq!(yaml_agent.config.name, "yaml-agent");
        assert_eq!(yaml_agent.config.description, "Agent from YAML");

        let toml_agent = registry.get_agent("toml-agent").unwrap();
        assert_eq!(toml_agent.config.name, "toml-agent");
        assert_eq!(toml_agent.config.description, "Agent from TOML");

        // Verify we have exactly 2 agents
        let all_agents = registry.get_all_agents();
        assert_eq!(all_agents.len(), 2);
    }

    #[test]
    fn test_yaml_validation() {
        let temp_dir = TempDir::new().unwrap();

        // Test invalid YAML (missing required name)
        let invalid_yaml = r#"
description: Missing name field
prompt: Test prompt
"#;

        let invalid_path = temp_dir.path().join("invalid.yaml");
        fs::write(&invalid_path, invalid_yaml).unwrap();

        let result = yaml_loader::load_yaml_config(&invalid_path);
        assert!(result.is_err());

        // Test invalid mode_override
        let invalid_mode_yaml = r#"
name: test-agent
description: Test
mode_override: invalid_mode
prompt: Test prompt
"#;

        let invalid_mode_path = temp_dir.path().join("invalid_mode.yaml");
        fs::write(&invalid_mode_path, invalid_mode_yaml).unwrap();

        let result = yaml_loader::load_yaml_config(&invalid_mode_path);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid mode_override")
        );

        // Test invalid intelligence level
        let invalid_intel_yaml = r#"
name: test-agent
description: Test
intelligence: super_hard
prompt: Test prompt
"#;

        let invalid_intel_path = temp_dir.path().join("invalid_intel.yaml");
        fs::write(&invalid_intel_path, invalid_intel_yaml).unwrap();

        let result = yaml_loader::load_yaml_config(&invalid_intel_path);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid intelligence level")
        );
    }
}
