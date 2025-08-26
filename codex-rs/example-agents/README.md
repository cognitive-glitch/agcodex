# Example Agent Configurations

This directory contains example YAML configurations for AGCodex subagents. These examples demonstrate the variety of configurations possible and serve as templates for creating custom agents.

## Available Examples

### 📋 code-reviewer.yaml
**Purpose**: Reviews code for quality, security, and performance issues  
**Mode**: Review (read-only with analysis capabilities)  
**Intelligence**: Hard (maximum analysis depth)  
**Use Cases**:
- Security vulnerability detection
- Performance bottleneck identification
- Code quality assessment
- OWASP Top 10 compliance checks

### 🧪 test-writer.yaml
**Purpose**: Generates comprehensive test suites for existing code  
**Mode**: Build (full file creation/editing access)  
**Intelligence**: Medium (balanced performance)  
**Use Cases**:
- Unit test generation
- Integration test creation
- Property-based testing
- Test coverage improvement

### 🔧 refactorer.yaml
**Purpose**: Performs intelligent code refactoring with AST awareness  
**Mode**: Inherits current mode (no override)  
**Intelligence**: Hard (complex refactoring requires deep analysis)  
**Use Cases**:
- Extract method/class refactoring
- Design pattern application
- Code deduplication
- Complexity reduction

## Installation

To use these example agents:

1. **Global Installation** (available to all projects):
   ```bash
   mkdir -p ~/.agcodex/agents/
   cp *.yaml ~/.agcodex/agents/
   ```

2. **Project-Specific Installation** (only for current project):
   ```bash
   mkdir -p ./.agcodex/agents/
   cp *.yaml ./.agcodex/agents/
   ```

## Creating Custom Agents

Use these examples as templates for your own agents:

1. Copy an example that's closest to your needs
2. Modify the `name`, `description`, and `prompt` fields
3. Adjust tool permissions based on required capabilities
4. Configure parameters for customization
5. Set appropriate intelligence level and mode override
6. Save to `~/.agcodex/agents/` or `./.agcodex/agents/`

## Key Configuration Options

### Operating Modes
- **plan**: Read-only analysis mode
- **build**: Full access mode (default)
- **review**: Quality focus mode with limited edits

### Intelligence Levels
- **light**: Fast, 70% compression, basic analysis
- **medium**: Balanced, 85% compression (default)
- **hard**: Maximum intelligence, 95% compression

### Tool Permissions
- **allow**: Full access to the tool
- **deny**: Tool is completely blocked
- **restricted**: Tool access with specific limitations

## Testing Your Configuration

Use the provided verification script:
```bash
python3 ../verify_yaml_config.py
```

This will validate your YAML syntax and check for required fields.

## Invoking Agents

Once installed, invoke agents using the `@agent-name` pattern:

```
@agent-code-reviewer - Analyze this file for security issues
@agent-test-writer - Generate unit tests for the Calculator class
@agent-refactorer - Extract this method into a separate function
```

## Tips

1. **Start Simple**: Begin with basic configurations and add complexity as needed
2. **Use Templates**: The `template` field allows inheriting from base configurations
3. **Test Restrictions**: Use `restricted` permissions to limit tool capabilities safely
4. **Document Parameters**: Clear parameter descriptions help users understand options
5. **Tag Appropriately**: Tags help organize and discover agents

## Contributing

To contribute new example agents:
1. Create a well-documented YAML configuration
2. Test it thoroughly in real scenarios
3. Include clear use cases in comments
4. Submit a pull request with your example

## Support

For questions or issues with agent configurations:
- Check the main AGCodex documentation
- Review the YAML_LOADER_IMPLEMENTATION.md file
- Open an issue on the GitHub repository