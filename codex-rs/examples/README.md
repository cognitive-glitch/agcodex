# AGCodex Examples & Workflows

Welcome to the AGCodex examples directory! This comprehensive guide provides practical examples, workflows, and configuration templates to help you get the most out of AGCodex.

## 📚 Example Categories

### 1. [Basic Usage](basic_usage.md)
Learn the fundamentals of AGCodex with step-by-step tutorials covering:
- Starting AGCodex and navigating the TUI
- Mode switching (Plan/Build/Review) with Shift+Tab
- Basic code generation and editing
- Session management and persistence
- Understanding the search and edit tools

**Best for:** New users, quick reference, basic workflows

### 2. [Agent Workflows](agent_workflows.md)
Master the multi-agent system with practical examples:
- Code review with `@code-reviewer`
- Refactoring with `@refactorer`
- Performance optimization with `@performance`
- Security auditing with `@security`
- Chaining multiple agents for complex tasks
- Creating custom agent pipelines

**Best for:** Advanced users, team collaboration, automated workflows

### 3. [Advanced Features](advanced_features.md)
Explore powerful capabilities for sophisticated development:
- Custom agent configuration
- AST compression levels (Light/Medium/Hard)
- MCP server integration
- Git worktree workflows
- Session branching and timeline navigation
- Embedding configuration for semantic search

**Best for:** Power users, complex projects, custom integrations

### 4. [Configuration Templates](configuration_templates/)
Ready-to-use configuration files for different scenarios:
- **[minimal_config.toml](configuration_templates/minimal_config.toml)** - Quick start with essential settings
- **[full_featured_config.toml](configuration_templates/full_featured_config.toml)** - All features enabled with detailed options
- **[team_config.toml](configuration_templates/team_config.toml)** - Standardized team configuration
- **[local_llm_config.toml](configuration_templates/local_llm_config.toml)** - Ollama setup for local LLMs

**Best for:** Quick setup, team standardization, specific use cases

### 5. [Custom Agents](custom_agents/)
Pre-configured agent definitions for specialized tasks:
- **[react_specialist.yaml](custom_agents/react_specialist.yaml)** - React/TypeScript development
- **[database_expert.yaml](custom_agents/database_expert.yaml)** - Database design and optimization
- **[api_designer.yaml](custom_agents/api_designer.yaml)** - REST/GraphQL API development
- **[deployment_assistant.yaml](custom_agents/deployment_assistant.yaml)** - CI/CD and deployment workflows

**Best for:** Specialized development, domain-specific tasks, team workflows

## 🚀 Quick Start Guide

### First Time Setup
```bash
# 1. Install AGCodex
cargo install agcodex

# 2. Copy minimal configuration
cp examples/configuration_templates/minimal_config.toml ~/.agcodex/config.toml

# 3. Set your API key
export OPENAI_API_KEY="your-key-here"  # or use ChatGPT login

# 4. Launch AGCodex
agcodex
```

### Essential Keyboard Shortcuts

| Shortcut | Action | Mode |
|----------|--------|------|
| **Shift+Tab** | Switch modes (Plan→Build→Review) | All |
| **/** | Command palette | All |
| **Ctrl+N** | New conversation | All |
| **Ctrl+S** | Save/load session | All |
| **Ctrl+H** | History browser | All |
| **Ctrl+A** | Agent panel | All |
| **Ctrl+J** | Jump to message | All |
| **Ctrl+Z/Y** | Undo/redo | All |
| **Esc** | Close panel/cancel | All |

## 🎯 Common Use Cases

### 1. Code Review Workflow
```bash
# Start in Review mode (read-only analysis)
agcodex --mode review

# In chat:
@code-reviewer analyze src/ for security and performance issues
```

### 2. Feature Development
```bash
# Start in Build mode (full access)
agcodex --mode build

# In chat:
Create a REST API endpoint for user authentication with JWT
```

### 3. Refactoring Session
```bash
# Start in Plan mode (planning only)
agcodex --mode plan

# In chat:
@refactorer identify code duplication in the controllers/ directory
# Switch to Build mode with Shift+Tab
@refactorer apply the refactoring plan
```

### 4. Test Generation
```bash
# Use test-writer agent
agcodex

# In chat:
@test-writer generate comprehensive tests for src/auth.rs
```

## 📖 Learning Path

### Beginner (Day 1-3)
1. Read [Basic Usage](basic_usage.md)
2. Try the minimal configuration
3. Practice mode switching
4. Learn basic search and edit commands

### Intermediate (Week 1-2)
1. Explore [Agent Workflows](agent_workflows.md)
2. Set up team configuration
3. Create your first custom agent
4. Master session management

### Advanced (Week 2+)
1. Study [Advanced Features](advanced_features.md)
2. Configure MCP servers
3. Build complex agent pipelines
4. Optimize AST compression levels

## 🛠️ Troubleshooting

### Common Issues

**Issue:** "API key not found"
```bash
# Solution 1: Set environment variable
export OPENAI_API_KEY="sk-..."

# Solution 2: Use ChatGPT login
agcodex login
```

**Issue:** "Mode restrictions preventing edits"
```bash
# Check current mode (shown in status bar)
# Press Shift+Tab to switch to Build mode
```

**Issue:** "Agent not found"
```bash
# List available agents
ls ~/.agcodex/agents/

# Copy example agent
cp examples/custom_agents/react_specialist.yaml ~/.agcodex/agents/
```

## 📊 Performance Tips

### 1. AST Compression Levels
- **Light (70%)**: Fast, minimal indexing - good for small projects
- **Medium (85%)**: Balanced performance - default for most users
- **Hard (95%)**: Maximum compression - best for large codebases

### 2. Search Optimization
- Use glob patterns: `search "*.rs" for error handling`
- Enable caching: Set `cache_enabled = true` in config
- Limit scope: `search in src/controllers`

### 3. Agent Performance
- Run independent agents in parallel
- Use git worktrees for isolation
- Enable incremental mode for large changes

## 🤝 Contributing Examples

We welcome contributions! To add new examples:

1. Create a new markdown file in the appropriate directory
2. Follow the existing format with clear sections
3. Include practical, runnable examples
4. Add troubleshooting tips
5. Submit a PR with your examples

## 📚 Additional Resources

- [AGCodex Documentation](../README.md)
- [Configuration Guide](../docs/CONFIG.md)
- [Agent Development Guide](../docs/AGENT_GUIDE.md)
- [API Reference](../docs/API.md)

## 🎓 Example Code Repository

For executable code examples, see:
- [agent_workflows.rs](agent_workflows.rs) - Rust implementation examples
- [core/examples/](../core/examples/) - Core functionality demos
- [tests/](../tests/) - Integration test examples

---

**Need Help?** Join our [Discord](https://discord.gg/agcodex) or check [GitHub Issues](https://github.com/agcodex/agcodex/issues)