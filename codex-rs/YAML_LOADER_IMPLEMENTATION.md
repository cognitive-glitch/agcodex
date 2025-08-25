# YAML Configuration Loader for AGCodex Subagent System

## Implementation Summary

This document describes the YAML configuration loader that has been added to the AGCodex subagent system, enabling agent definitions to be loaded from YAML files in addition to the existing TOML support.

## Components Implemented

### 1. **Core YAML Loader Module** (`core/src/subagents/yaml_loader.rs`)
- **Purpose**: Provides functionality to parse and validate YAML agent configurations
- **Key Functions**:
  - `load_yaml_config()` - Load a single YAML file into SubagentConfig
  - `load_yaml_configs_from_directory()` - Scan and load all YAML files from a directory
  - `load_all_yaml_configs()` - Load from both global and project directories
  - `validate_yaml_file()` - Validate YAML structure without full loading

### 2. **Registry Integration** (`core/src/subagents/registry.rs`)
- **Updated Methods**:
  - `load_agent_from_file()` - Now supports both TOML and YAML based on file extension
  - `load_agents_from_directory()` - Scans for `.toml`, `.yaml`, and `.yml` files
  - `load_yaml_configs()` - New public method for explicit YAML loading
- **File Extension Support**: `.yaml`, `.yml`, and `.toml`

### 3. **Dependencies Added**
- Added `serde_yaml = "0.9"` to `core/Cargo.toml`

## YAML Configuration Format

### Structure
```yaml
name: agent-name                # Required: Unique agent identifier
description: Agent description   # Required: Human-readable description
mode_override: review           # Optional: plan/build/review
intelligence: hard              # Optional: light/medium/hard

# Tool permissions
tools:
  - name: tool-name
    permission: allow/deny/restricted
    restrictions:              # Only for 'restricted' permission
      key: value

# Custom prompt
prompt: |
  Multi-line prompt text
  for the agent

# Parameters
parameters:
  - name: param-name
    description: Parameter description
    required: true/false
    default: default-value
    valid_values:
      - option1
      - option2

# Additional settings
timeout_seconds: 600
chainable: true
parallelizable: true
file_patterns:
  - "*.rs"
  - "*.py"
tags:
  - tag1
  - tag2
metadata:
  custom_key: custom_value
```

## Directory Structure

```
~/.agcodex/agents/           # Global agents directory
├── code-reviewer.yaml       # Global agent configurations
├── test-writer.yml          # Supports both .yaml and .yml
└── ...

./.agcodex/agents/           # Project-specific agents
├── custom-analyzer.yaml    # Project agents override global
└── ...
```

## Key Features

### 1. **Flexible Tool Configuration**
- Simple format: `permission: allow/deny`
- Complex format with restrictions:
  ```yaml
  - name: bash
    permission: restricted
    restrictions:
      allowed_commands: "cargo test,pytest"
      max_duration: "300"
  ```

### 2. **Priority System**
- Project agents (`./.agcodex/agents/`) override global agents (`~/.agcodex/agents/`)
- YAML and TOML files can coexist in the same directory
- Name conflicts are detected and reported

### 3. **Validation**
- Required fields: `name`, `description`, `prompt` (unless using template)
- Mode validation: must be `plan`, `build`, or `review`
- Intelligence validation: must be `light`, `medium`, or `hard`
- Tool permission validation: must be `allow`, `deny`, or `restricted`

### 4. **Type Conversions**
- String-based enums in YAML are converted to proper Rust enums
- Case-insensitive parsing for enum values
- Comprehensive error messages for invalid values

## Example Agents Provided

### 1. **code-reviewer.yaml**
- Mode: Review (read-only)
- Intelligence: Hard (95% compression)
- Focus: Security, performance, quality analysis
- Restricted tools for safety

### 2. **test-writer.yaml**
- Mode: Build (full access)
- Intelligence: Medium (85% compression)
- Generates comprehensive test suites
- Parameterized for different test frameworks

### 3. **refactorer.yaml**
- Intelligence: Hard
- AST-aware refactoring
- Rich metadata for refactoring patterns
- Sequential execution only

## Testing

### Unit Tests
Located in `yaml_loader.rs`:
- `test_load_simple_yaml_config()`
- `test_load_yaml_with_restricted_tools()`
- `test_invalid_yaml_config()`
- `test_load_directory_with_multiple_configs()`
- `test_yaml_with_parameters()`

### Integration Tests
Located in `test_yaml_integration.rs`:
- `test_yaml_integration_with_registry()`
- `test_yaml_override_behavior()`
- `test_mixed_toml_yaml_loading()`
- `test_yaml_validation()`

### Verification Script
`verify_yaml_config.py` - Python script to validate YAML structure

## Usage

### Creating a New Agent
1. Create a YAML file in `~/.agcodex/agents/` or `./.agcodex/agents/`
2. Define required fields: `name`, `description`, `prompt`
3. Configure tools, parameters, and metadata as needed
4. The registry will automatically load it on startup

### Invoking an Agent
```bash
# In the application
@agent-code-reviewer - Review code for quality
@agent-test-writer - Generate tests
@agent-refactorer - Refactor code
```

## Benefits Over TOML

1. **More readable for complex prompts** - Multi-line strings with `|`
2. **Cleaner array syntax** - Natural list format for tools and parameters
3. **Flexible nesting** - Easier to express complex restrictions
4. **Familiar format** - Widely used in CI/CD and configuration

## Backward Compatibility

- Existing TOML configurations continue to work
- Both formats can be mixed in the same directory
- No changes required to existing agents
- Gradual migration path available

## Future Enhancements

1. **YAML Templates** - Support for YAML-based template inheritance
2. **Schema Validation** - JSON Schema for YAML validation
3. **Hot Reload** - Watch for YAML file changes
4. **Migration Tool** - TOML to YAML converter
5. **Web UI** - Visual YAML configuration editor

## Error Handling

The implementation provides detailed error messages:
- Missing required fields
- Invalid enum values (mode, intelligence, permission)
- File I/O errors
- YAML parsing errors
- Name conflicts between agents

All errors are properly typed using `thiserror` and propagated through the `SubagentError` type hierarchy.