# AGCodex Agent System

## Overview

AGCodex agents are specialized AI assistants that operate with custom contexts, prompts, and tool permissions. Each agent is designed for specific tasks and can be invoked directly from the TUI or chained together for complex workflows.

## Quick Start

### Using Built-in Agents

In the AGCodex TUI, invoke agents with the `@` prefix:

```
@code-reviewer      # Review code for quality and security
@refactorer        # Refactor code with AST transformations
@debugger          # Debug issues with root cause analysis
@test-writer       # Generate comprehensive tests
@performance       # Optimize performance bottlenecks
@security          # Security vulnerability analysis
@docs              # Generate documentation
```

### Agent Invocation Examples

```bash
# Simple invocation
@code-reviewer

# With parameters
@code-reviewer severity=high focus_areas="security,performance"

# Chained agents (sequential)
@security → @code-reviewer → @docs

# Parallel agents
@test-writer + @docs

# Complex workflow
@debugger issue_description="memory leak" → @performance → @refactorer
```

## Agent Configuration Structure

Each agent is defined by a TOML configuration file with the following sections:

### 1. **Identity & Metadata**

```toml
name = "agent-name"              # Unique identifier
description = "What it does"     # Brief description
intelligence = "medium"          # light, medium, or hard
mode_override = "review"         # Optional: plan, build, or review
priority = 80                    # 0-100, higher runs first
```

### 2. **Tool Permissions**

Control what tools the agent can use:

```toml
[tools]
allow = ["Read", "AST-Search", "Write"]  # Allowed tools
deny = ["Delete", "Execute"]             # Explicitly denied
```

Available tools:
- **Read/Write Operations**: `Read`, `Write`, `Delete`
- **AST Operations**: `AST-Search`, `AST-Transform`, `Tree-sitter-analyze`
- **Git Operations**: `Git-diff`, `Git-log`, `Git-blame`, `Git-commit`
- **Analysis Tools**: `Profile-cpu`, `Profile-memory`, `Coverage-analyze`
- **Security Tools**: `Security-scan`, `Secret-scanner`, `SAST-analyzer`
- **Other**: `Execute`, `Test-runner`, `Benchmark-runner`

### 3. **Prompt Template**

Define the agent's behavior and approach:

```toml
[prompt]
template = """
You are a {role} specializing in {specialty}.

Context:
- Task: {task}
- Parameters: {param1}, {param2}

Your approach:
1. Step one
2. Step two
...
"""

system = "Optional system message prepended to all interactions"
```

### 4. **Parameters**

Define configurable parameters:

```toml
[parameters]
param_name = {
    type = "string",          # string, integer, boolean, float
    default = "value",        # Default value
    required = false,         # Must be provided?
    values = ["opt1", "opt2"], # Optional: restrict values
    min = 0,                  # For numbers: minimum
    max = 100,                # For numbers: maximum
    description = "What this parameter controls"
}
```

### 5. **Context Inheritance**

Control what context the agent inherits:

```toml
[context]
inherit_ast_index = true       # Parsed AST index
inherit_embeddings = false     # Vector embeddings
inherit_git_history = false    # Git history
inherit_test_results = false   # Test results
max_context_size = "100MB"     # Maximum context
exclude_patterns = ["*.log"]   # Patterns to exclude
```

## Creating Custom Agents

### Step 1: Copy the Template

```bash
cp agents/templates/base-agent.toml agents/my-custom-agent.toml
```

### Step 2: Customize the Configuration

Edit your agent configuration:

```toml
name = "my-custom-agent"
description = "Specialized agent for specific task"
intelligence = "hard"

[tools]
allow = ["Read", "Write", "AST-Search"]

[prompt]
template = """
You are an expert in {domain}.
Focus on {objective}.
Follow these steps:
1. Analyze the current situation
2. Apply specialized knowledge
3. Produce actionable output
"""

[parameters]
domain = { type = "string", default = "general" }
objective = { type = "string", required = true }
```

### Step 3: Test Your Agent

In AGCodex TUI:
```
@my-custom-agent objective="optimize database queries"
```

## Agent Intelligence Levels

### Light (Fast, Minimal Resources)
- Basic AST parsing
- Simple pattern matching
- Quick analysis
- 70% code compression
- Suitable for: Quick checks, simple refactors

### Medium (Balanced, Default)
- Full AST analysis
- Pattern recognition
- Moderate context window
- 85% code compression
- Suitable for: Most tasks, standard development

### Hard (Maximum Intelligence)
- Deep AST analysis
- Call graph generation
- Data flow analysis
- Maximum context
- 95% code compression
- Suitable for: Complex refactoring, security analysis, architecture

## Operating Mode Overrides

Agents can override the current operating mode:

- **Plan Mode**: Read-only analysis, no modifications
- **Build Mode**: Full access to all operations
- **Review Mode**: Quality checks with limited modifications

Example:
```toml
mode_override = "review"  # Forces review mode when agent runs
```

## Agent Chaining

### Sequential Execution (→)

Agents run one after another, passing context:

```
@security → @code-reviewer → @docs
```

### Parallel Execution (+)

Agents run simultaneously on separate worktrees:

```
@test-writer + @docs + @performance
```

### Complex Workflows

Combine sequential and parallel:

```
@debugger → (@refactorer + @test-writer) → @code-reviewer
```

## Advanced Features

### Resource Limits

```toml
[resources]
max_memory = "2GB"
max_cpu_percent = 80
timeout_seconds = 300
```

### Lifecycle Hooks

```toml
[hooks]
pre_execute = "prepare.sh"
post_execute = "cleanup.sh"
on_error = "rollback.sh"
```

### Environment Variables

```toml
[environment]
CUSTOM_VAR = "value"
API_ENDPOINT = "https://api.example.com"
```

## Agent Storage Locations

```
~/.agcodex/
├── agents/              # User-level agents
│   ├── global/         # Available in all projects
│   └── templates/      # Reusable templates
```

Project-specific agents:
```
.agcodex/
└── agents/             # Project-specific agents
    └── *.toml
```

## Best Practices

### 1. **Single Responsibility**
Each agent should focus on one specific task or domain.

### 2. **Clear Parameters**
Define parameters with clear descriptions and sensible defaults.

### 3. **Appropriate Intelligence**
Use the minimum intelligence level needed for the task.

### 4. **Tool Restrictions**
Only grant necessary tool permissions following principle of least privilege.

### 5. **Context Management**
Inherit only the context needed to avoid overwhelming the agent.

### 6. **Error Handling**
Include error scenarios in your prompt template.

### 7. **Documentation**
Document complex agents with examples and use cases.

## Debugging Agents

### View Agent Configuration
```bash
agcodex agent info @agent-name
```

### Test Agent Locally
```bash
agcodex agent test @agent-name --params "key=value"
```

### Agent Logs
```bash
tail -f ~/.agcodex/logs/agents/agent-name.log
```

## Common Agent Patterns

### 1. **Analyzer Pattern**
Read-only agent that analyzes code without modifications:
```toml
mode_override = "plan"
[tools]
allow = ["Read", "AST-Search"]
deny = ["Write", "Execute"]
```

### 2. **Transformer Pattern**
Agent that modifies code structure:
```toml
mode_override = "build"
[tools]
allow = ["Read", "Write", "AST-Transform"]
```

### 3. **Validator Pattern**
Agent that checks code quality:
```toml
mode_override = "review"
[tools]
allow = ["Read", "Test-runner", "Coverage-analyze"]
```

### 4. **Generator Pattern**
Agent that creates new content:
```toml
[tools]
allow = ["Read", "Write", "AST-Search"]
[parameters]
template_type = { type = "string", values = ["class", "function", "test"] }
```

## Troubleshooting

### Agent Not Found
- Check agent name spelling
- Ensure `.toml` file exists in correct location
- Verify file permissions

### Parameter Errors
- Check required parameters are provided
- Verify parameter types match configuration
- Ensure values are within allowed ranges

### Permission Denied
- Agent may lack necessary tool permissions
- Check mode_override compatibility
- Verify file system permissions

### Performance Issues
- Consider using lighter intelligence level
- Reduce max_context_size
- Enable caching in context settings

## Examples

### Custom Security Auditor

```toml
name = "custom-security-auditor"
description = "Project-specific security checks"
intelligence = "hard"
mode_override = "review"

[tools]
allow = ["Read", "AST-Search", "Security-scan"]
deny = ["Write", "Execute"]

[prompt]
template = """
Perform security audit focusing on:
- {security_focus}
- Project-specific patterns
- Custom validation rules
"""

[parameters]
security_focus = {
    type = "string",
    default = "authentication",
    values = ["authentication", "encryption", "validation"]
}
```

### Migration Assistant

```toml
name = "migration-assistant"
description = "Helps migrate code between versions"
intelligence = "medium"

[tools]
allow = ["Read", "Write", "AST-Transform", "Git-diff"]

[prompt]
template = """
Migrate code from {from_version} to {to_version}.
Apply transformation rules:
- Update deprecated APIs
- Modify import statements
- Adjust configuration formats
"""

[parameters]
from_version = { type = "string", required = true }
to_version = { type = "string", required = true }
```

## Contributing

To contribute a new agent to AGCodex:

1. Create agent configuration in `agents/community/`
2. Include comprehensive documentation
3. Add tests in `agents/tests/`
4. Submit PR with example usage

## Support

- Documentation: https://agcodex.dev/agents
- Issues: https://github.com/agcodex/codex-rs/issues
- Discord: https://discord.gg/agcodex