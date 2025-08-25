# AGCodex User Guide

Welcome to AGCodex - an independent, TUI-first AI coding assistant that runs locally with AST-based intelligence, configurable operating modes, and comprehensive language support.

## Table of Contents

1. [Quick Start Guide](#quick-start-guide)
2. [Operating Modes](#operating-modes)
3. [TUI Features](#tui-features)
4. [Agent System](#agent-system)
5. [Configuration](#configuration)
6. [Advanced Features](#advanced-features)
7. [Common Workflows](#common-workflows)
8. [Troubleshooting](#troubleshooting)

---

## Quick Start Guide

### Installation

AGCodex can be installed through multiple methods:

#### Via npm (Recommended)
```bash
npm install -g agcodex
```

#### Via Cargo
```bash
cargo install agcodex
```

#### From Source
```bash
git clone https://github.com/your-org/agcodex
cd agcodex/codex-rs
cargo build --release
sudo cp target/release/agcodex /usr/local/bin/
```

#### Pre-built Binaries
Download the latest release from [GitHub Releases](https://github.com/your-org/agcodex/releases) for your platform:
- macOS: `agcodex-macos-arm64` or `agcodex-macos-x64`
- Linux: `agcodex-linux-x64`
- Windows: `agcodex-windows-x64.exe`

### First Run

1. Launch AGCodex:
```bash
agcodex
```

2. On first launch, you'll be guided through:
   - API key configuration
   - Trust directory approval
   - Model selection

3. The TUI will open in Build mode (full access) by default.

### Setting Up API Keys

Create or edit `~/.agcodex/config.toml`:

```toml
[models]
default_provider = "openai"  # or "anthropic", "ollama"

[providers.openai]
api_key = "sk-..."
base_url = "https://api.openai.com/v1"  # Optional

[providers.anthropic]
api_key = "sk-ant-..."

[providers.ollama]
base_url = "http://localhost:11434"  # Local Ollama instance
```

For security, you can also use environment variables:
```bash
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
```

---

## Operating Modes

AGCodex features three distinct operating modes, each optimized for different workflows. Switch between modes instantly with **Shift+Tab**.

### 📋 Plan Mode (Read-Only Analysis)

**Purpose**: Safe exploration and analysis without any file modifications.

**Features**:
- Read and analyze any file or codebase
- Generate architectural diagrams
- Search and understand code patterns
- Perfect for code reviews and learning

**Restrictions**:
- No file writes or modifications
- No command execution
- No git operations

**Example Use Cases**:
```bash
# Launch in Plan mode
agcodex --mode plan

# Or switch to Plan mode with Shift+Tab
```

Common Plan mode workflows:
- Analyzing unfamiliar codebases
- Understanding system architecture
- Preparing refactoring strategies
- Code review preparation

### 🔨 Build Mode (Full Access)

**Purpose**: Active development with complete system access.

**Features**:
- Full file system read/write
- Command execution (with sandboxing)
- Git operations
- Package management
- All tools available

**Example Use Cases**:
```bash
# Default mode when launching
agcodex

# Or explicitly
agcodex --mode build
```

Common Build mode workflows:
- Feature implementation
- Bug fixes
- Refactoring
- Test writing
- Documentation generation

### 🔍 Review Mode (Quality Focus)

**Purpose**: Code quality improvements with controlled modifications.

**Features**:
- Limited file edits (max 10KB per file)
- Focus on quality improvements
- Linting and formatting
- Small bug fixes
- Documentation updates

**Restrictions**:
- Large refactoring blocked
- New feature creation limited
- Structural changes restricted

**Example Use Cases**:
```bash
agcodex --mode review
```

Common Review mode workflows:
- Code style improvements
- Adding missing documentation
- Fixing linter warnings
- Small bug fixes
- Test improvements

### Mode Switching

Press **Shift+Tab** at any time to cycle through modes:
```
Plan → Build → Review → Plan
```

The current mode is always displayed in the top-right corner of the TUI with color coding:
- 📋 Plan (Blue)
- 🔨 Build (Green)
- 🔍 Review (Yellow)

---

## TUI Features

The Terminal User Interface provides powerful keyboard-driven navigation and control.

### Essential Keyboard Shortcuts

| Shortcut | Action | Description |
|----------|--------|-------------|
| **Shift+Tab** | Mode Switch | Cycle between Plan/Build/Review modes |
| **/** | Command Palette | Search and execute any command |
| **Ctrl+N** | New Conversation | Start fresh conversation |
| **Ctrl+S** | Session Manager | Save/load conversation sessions |
| **Ctrl+A** | Agent Panel | View and manage active agents |
| **Ctrl+H** | History Browser | Navigate conversation history |
| **Ctrl+J** | Jump to Message | Navigate to specific message |
| **Ctrl+T** | Context Window | Visualize token usage and context |
| **Ctrl+Z** | Undo | Undo last conversation turn |
| **Ctrl+Y** | Redo | Redo undone turn |
| **Ctrl+B** | Branch | Create conversation branch |
| **Esc** | Close/Cancel | Close panel or cancel operation |
| **Tab** | Focus Next | Cycle through UI elements |
| **F5** | Checkpoint | Create session checkpoint |
| **F6** | Restore | Restore from checkpoint |
| **Ctrl+?** | Help | Show all keybindings |

### Session Management

Sessions are automatically saved to `~/.agcodex/history` with Zstd compression.

#### Saving Sessions
Press **Ctrl+S** to open the session manager:
```
┌─ Save Session ─────────────────────────┐
│ Name: feature-auth-implementation      │
│ Tags: auth, jwt, security              │
│ Description: JWT auth implementation   │
│                                        │
│ [Enter] Save  [Esc] Cancel            │
└────────────────────────────────────────┘
```

#### Loading Sessions
Press **Ctrl+S** then select "Load":
```
┌─ Load Session ─────────────────────────┐
│ ▶ feature-auth-implementation         │
│   2024-01-15 14:30 | 45 messages      │
│                                        │
│ ▶ bug-fix-memory-leak                 │
│   2024-01-14 09:15 | 23 messages      │
│                                        │
│ [↑↓] Navigate  [Enter] Load          │
└────────────────────────────────────────┘
```

### History Navigation

Press **Ctrl+H** to open the history browser with timeline visualization:

```
┌─ History Browser ──────────────────────┐
│ ━━━━━━━━●━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│         ↑ Message 15 of 45            │
│                                        │
│ [14:30] You: Implement JWT auth       │
│ [14:31] Assistant: I'll help you...   │
│ [14:35] You: Add refresh tokens       │
│                                        │
│ [←→] Navigate  [Enter] Jump to        │
└────────────────────────────────────────┘
```

### Context Window Visualization

Press **Ctrl+T** to visualize token usage and context window:

```
┌─ Context Window ───────────────────────┐
│ Model: gpt-4 (128k context)           │
│                                        │
│ ████████████████░░░░░░░░ 65% (83k)    │
│                                        │
│ System: 2.5k tokens                    │
│ History: 45k tokens (last 20 msgs)    │
│ Current: 35.5k tokens                  │
│                                        │
│ Available: 45k tokens                  │
│                                        │
│ [Esc] Close                            │
└────────────────────────────────────────┘
```

### Agent Panel

Press **Ctrl+A** to view active agents:

```
┌─ Active Agents ────────────────────────┐
│ ● code-reviewer    [Plan mode]        │
│   Analyzing: src/auth.rs               │
│                                        │
│ ● test-writer      [Build mode]       │
│   Writing: tests/auth_test.rs         │
│                                        │
│ ○ refactorer       [Idle]             │
│                                        │
│ [↑↓] Select  [Enter] View  [d] Stop  │
└────────────────────────────────────────┘
```

---

## Agent System

AGCodex includes a powerful multi-agent system for specialized tasks.

### Using Agents

Invoke agents using the `@agent-name` pattern in your prompts:

```
@code-reviewer please review the authentication module

@test-writer create comprehensive tests for the User class

@refactorer optimize the database query functions
```

### Available Built-in Agents

| Agent | Purpose | Example Usage |
|-------|---------|---------------|
| **code-reviewer** | Code quality analysis | `@code-reviewer check src/` |
| **refactorer** | Code refactoring | `@refactorer simplify this function` |
| **debugger** | Bug investigation | `@debugger find the memory leak` |
| **test-writer** | Test generation | `@test-writer create unit tests` |
| **performance** | Performance optimization | `@performance optimize database queries` |
| **security** | Security analysis | `@security audit authentication` |
| **docs** | Documentation generation | `@docs generate API documentation` |
| **architect** | System design | `@architect design microservice structure` |

### Custom Agent Configuration

Create custom agents in `~/.agcodex/agents/` (user-level) or `.agcodex/agents/` (project-level):

#### Example: Custom React Agent
Create `~/.agcodex/agents/react-specialist.yaml`:

```yaml
name: react-specialist
description: React and Next.js development specialist
mode_override: build  # Force Build mode for this agent
tools:
  - search
  - edit
  - tree
  - grep
  - index
system_prompt: |
  You are a React and Next.js specialist with expertise in:
  - Modern React patterns (hooks, context, suspense)
  - Next.js App Router and Server Components
  - Performance optimization
  - State management (Redux, Zustand, Jotai)
  - Testing with React Testing Library
  
  Follow React best practices and ensure components are:
  - Properly typed with TypeScript
  - Optimized for performance
  - Accessible (WCAG 2.1 AA)
  - Well-tested

context:
  include_patterns:
    - "*.tsx"
    - "*.ts"
    - "*.jsx"
    - "*.js"
  exclude_patterns:
    - "node_modules/**"
    - "dist/**"
```

#### Agent Configuration Options

```yaml
# Required fields
name: agent-name
description: Brief description

# Optional fields
mode_override: plan|build|review  # Force specific mode
max_iterations: 10                 # Limit iterations
parallel: true                     # Enable parallel execution

# Tool restrictions
tools:
  - search
  - edit
  - think

# Custom prompting
system_prompt: |
  Detailed instructions for the agent...

# Context configuration
context:
  include_patterns:
    - "*.py"
    - "src/**/*.js"
  exclude_patterns:
    - "test/**"
    - "*.test.js"
  max_file_size: 100000  # bytes
  
# Integration with other agents
delegates_to:
  - test-writer  # Can invoke test-writer
  - debugger     # Can invoke debugger
```

### Agent Invocation Examples

#### Simple Invocation
```
@code-reviewer analyze the user authentication module
```

#### Multiple Agents
```
@architect design a caching layer then @test-writer create tests for it
```

#### Agent with Specific Context
```
@performance optimize the search function in src/search.rs
Focus on reducing memory allocations and improving cache usage.
```

#### Delegated Workflow
```
@refactorer clean up the database module and have @test-writer ensure all tests still pass
```

---

## Configuration

AGCodex uses TOML configuration files for flexible customization.

### Configuration File Location

Main configuration: `~/.agcodex/config.toml`

### Basic Configuration

```toml
# ~/.agcodex/config.toml

[general]
default_mode = "build"  # plan, build, or review
auto_save = true
history_limit = 1000

[ui]
theme = "dark"  # dark or light
show_line_numbers = true
syntax_highlighting = true
wrap_long_lines = false

[models]
default_provider = "openai"
default_model = "gpt-4"
temperature = 0.7
max_tokens = 4000

[providers.openai]
api_key = "${OPENAI_API_KEY}"  # Use environment variable
base_url = "https://api.openai.com/v1"
models = ["gpt-4", "gpt-4-turbo", "gpt-3.5-turbo"]

[providers.anthropic]
api_key = "${ANTHROPIC_API_KEY}"
models = ["claude-3-opus", "claude-3-sonnet", "claude-3-haiku"]

[providers.ollama]
base_url = "http://localhost:11434"
models = ["llama3", "codellama", "mistral"]
```

### Model Provider Configuration

#### OpenAI Configuration
```toml
[providers.openai]
api_key = "sk-..."
organization = "org-..."  # Optional
base_url = "https://api.openai.com/v1"
timeout = 120  # seconds
max_retries = 3

[providers.openai.models.gpt-4]
max_tokens = 8192
temperature = 0.7
top_p = 0.95
frequency_penalty = 0.0
presence_penalty = 0.0
```

#### Anthropic Configuration
```toml
[providers.anthropic]
api_key = "sk-ant-..."
base_url = "https://api.anthropic.com"
max_tokens_to_sample = 4000
```

#### Local Ollama Configuration
```toml
[providers.ollama]
base_url = "http://localhost:11434"
timeout = 300  # Longer timeout for local models
models = ["llama3:latest", "codellama:13b"]

[providers.ollama.models.codellama]
temperature = 0.3  # Lower temperature for code
num_predict = 2048
num_ctx = 4096
```

### Embeddings Configuration (Optional)

Embeddings are disabled by default but can be enabled for enhanced semantic search:

```toml
[embeddings]
enabled = true
provider = "openai"  # openai, gemini, or voyage
model = "text-embedding-3-small"
dimension = 1536
batch_size = 100

[embeddings.providers.openai]
api_key = "${OPENAI_API_KEY}"

[embeddings.providers.gemini]
api_key = "${GEMINI_API_KEY}"
model = "models/embedding-001"

[embeddings.providers.voyage]
api_key = "${VOYAGE_API_KEY}"
model = "voyage-2"
```

### Intelligence Levels

Configure AST-RAG intelligence levels:

```toml
[intelligence]
mode = "medium"  # light, medium, or hard

[intelligence.light]
compression = 0.70
chunk_size = 256
indexing = "on_demand"

[intelligence.medium]  # Default
compression = 0.85
chunk_size = 512
indexing = "background"

[intelligence.hard]
compression = 0.95
chunk_size = 1024
indexing = "aggressive"
include_call_graph = true
include_data_flow = true
```

### Sandbox Configuration

Control command execution safety:

```toml
[sandbox]
mode = "strict"  # off, normal, or strict
require_approval = true
allowed_commands = ["git", "npm", "cargo", "python"]
blocked_commands = ["rm -rf", "sudo", "chmod 777"]

[sandbox.macos]
use_seatbelt = true
profile = "default"

[sandbox.linux]
use_landlock = true
use_seccomp = true
allowed_paths = ["/tmp", "./"]
```

### MCP Server Configuration

Configure Model Context Protocol servers:

```toml
[mcp_servers.github]
command = "npx"
args = ["@modelcontextprotocol/server-github"]
env = { GITHUB_TOKEN = "${GITHUB_TOKEN}" }

[mcp_servers.postgres]
command = "npx"
args = ["@modelcontextprotocol/server-postgres", "postgresql://localhost/mydb"]
```

---

## Advanced Features

### MCP Integration

AGCodex supports the Model Context Protocol for tool extensibility:

#### As MCP Client
Connect to MCP servers for additional capabilities:

```toml
[mcp_servers.my_server]
command = "my-mcp-server"
args = ["--port", "3000"]
```

#### As MCP Server
Run AGCodex as an MCP server:

```bash
agcodex mcp
```

Debug with MCP Inspector:
```bash
npx @modelcontextprotocol/inspector agcodex mcp
```

### AST-RAG Intelligence System

AGCodex uses a sophisticated multi-layer retrieval system:

#### Search Layers
1. **Symbol Index** (<1ms): Direct symbol lookup
2. **Tantivy Index** (<5ms): Full-text search
3. **AST Cache** (<10ms): Structural analysis
4. **Ripgrep Fallback**: Comprehensive search

#### Code Compression
- **Light Mode**: 70% compression for quick analysis
- **Medium Mode**: 85% compression with balanced detail
- **Hard Mode**: 95% compression with call graphs and data flow

#### Example: Search with AST-RAG
```
Show me all React components that use the useAuth hook
```

AGCodex will:
1. Parse AST to find React components
2. Analyze hook usage patterns
3. Track component dependencies
4. Return relevant code with context

### Session Persistence

Sessions are automatically saved with intelligent compression:

#### Storage Location
```
~/.agcodex/history/
├── metadata.idx      # Fast lookup index
├── sessions/
│   ├── session-001.bin  # Zstd compressed
│   ├── session-002.bin
│   └── ...
└── checkpoints/
    ├── checkpoint-001.bin
    └── ...
```

#### Compression Statistics
- Average compression ratio: 10:1
- Fast loading: <500ms for 1000-message session
- Incremental saves: Only changes are written

### Git Worktree Integration

Agents can work in isolated git worktrees:

```yaml
# Agent configuration
name: feature-developer
git:
  use_worktree: true
  auto_commit: true
  branch_prefix: "feature/"
```

This enables:
- Parallel development without conflicts
- Automatic branching for each agent
- Clean separation of concerns
- Easy rollback if needed

---

## Common Workflows

### 1. Code Review Workflow

```bash
# Start in Plan mode for safety
agcodex --mode plan

# In the TUI:
@code-reviewer analyze the entire src/ directory for:
- Code quality issues
- Security vulnerabilities  
- Performance bottlenecks
- Missing tests

# Review results, then switch to Review mode (Shift+Tab)
# Make small fixes based on recommendations
```

### 2. Feature Implementation Workflow

```bash
# Start in Build mode
agcodex

# In the TUI:
I need to implement user authentication with JWT tokens.
Requirements:
- Login/logout endpoints
- Token refresh mechanism
- Role-based access control
- Secure password hashing

# AGCodex will:
# 1. Analyze existing code structure
# 2. Create necessary files
# 3. Implement authentication logic
# 4. Add tests
# 5. Update documentation
```

### 3. Bug Investigation Workflow

```bash
# Start in Plan mode to investigate
agcodex --mode plan

# In the TUI:
@debugger investigate why the application crashes when processing large files.
Error: "JavaScript heap out of memory"

# After finding the issue, switch to Build mode
# Apply the fix
```

### 4. Refactoring Workflow

```bash
agcodex

# In the TUI:
@refactorer please refactor the database module to:
- Use connection pooling
- Implement retry logic
- Add proper error handling
- Improve query performance

Then @test-writer ensure all tests still pass and add new ones for the retry logic
```

### 5. Documentation Generation Workflow

```bash
agcodex --mode build

# In the TUI:
@docs generate comprehensive API documentation for all public endpoints in src/api/
Include:
- Request/response examples
- Authentication requirements
- Error codes
- Rate limiting info
```

### 6. Performance Optimization Workflow

```bash
agcodex

# Create checkpoint before optimization
# Press F5 to create checkpoint

@performance analyze and optimize the search functionality
Focus on:
- Reducing memory allocations
- Improving cache hit rates
- Optimizing database queries

# If optimization causes issues, press F6 to restore checkpoint
```

### 7. Multi-Agent Collaboration Workflow

```bash
agcodex

# Complex task with multiple agents
@architect design a real-time notification system

Once the design is ready:
@code-reviewer verify the architecture follows best practices
@security check for potential vulnerabilities
@test-writer create a comprehensive test plan

Then we'll implement it together.
```

---

## Troubleshooting

### Common Issues and Solutions

#### API Key Issues
```
Error: Invalid API key
```
**Solution**: Verify your API key in `~/.agcodex/config.toml` or environment variables.

#### Mode Restrictions
```
Error: Operation not permitted in Plan mode
```
**Solution**: Switch to Build or Review mode with Shift+Tab.

#### Large File Handling
```
Error: File too large for context window
```
**Solution**: AGCodex automatically compresses code. For very large files, use:
```
Analyze only the authentication functions in src/large_file.js
```

#### Session Recovery
If AGCodex crashes, your session is auto-saved:
```bash
agcodex --recover-last
```

#### Performance Issues
For large codebases, optimize with:
```toml
[performance]
max_file_size = 1048576  # 1MB
exclude_patterns = ["node_modules", "dist", "build"]
cache_size = 1000  # MB
parallel_indexing = true
```

### Debug Commands

```bash
# Check configuration
agcodex config validate

# Test sandbox
agcodex debug sandbox ls -la

# View AST for a file
agcodex debug ast src/main.rs

# Check index status
agcodex debug index status

# Clear cache
agcodex cache clear
```

### Getting Help

- **In TUI**: Press `Ctrl+?` for keybinding help
- **Command line**: `agcodex --help`
- **Documentation**: Visit [docs.agcodex.dev](https://docs.agcodex.dev)
- **Community**: Join our Discord at [discord.gg/agcodex](https://discord.gg/agcodex)

---

## Best Practices

### 1. Mode Selection
- Start in **Plan mode** when exploring unfamiliar code
- Use **Build mode** for active development
- Switch to **Review mode** for quality improvements

### 2. Agent Usage
- Use specialized agents for specific tasks
- Combine agents for complex workflows
- Create custom agents for repetitive tasks

### 3. Session Management
- Create checkpoints before major changes (F5)
- Name sessions descriptively
- Use tags for easy searching

### 4. Performance
- Exclude unnecessary files in configuration
- Use intelligence levels appropriate to your task
- Enable caching for large codebases

### 5. Security
- Never commit API keys to version control
- Use environment variables for sensitive data
- Enable sandbox mode for untrusted code

---

## Appendix

### File Type Support

AGCodex supports 27+ languages via tree-sitter:

| Language | Extensions | Full AST Support |
|----------|------------|------------------|
| Rust | .rs | ✅ |
| Python | .py | ✅ |
| JavaScript | .js, .mjs | ✅ |
| TypeScript | .ts, .tsx | ✅ |
| Go | .go | ✅ |
| C/C++ | .c, .cpp, .h | ✅ |
| Java | .java | ✅ |
| Ruby | .rb | ✅ |
| PHP | .php | ✅ |
| Swift | .swift | ✅ |
| Kotlin | .kt | ✅ |
| Scala | .scala | ✅ |
| Haskell | .hs | ✅ |
| Elixir | .ex, .exs | ✅ |
| And 13+ more... | | ✅ |

### Keyboard Shortcuts Reference Card

```
┌─────────────────────────────────────────┐
│         AGCodex Quick Reference         │
├─────────────────────────────────────────┤
│ Mode Switching                          │
│   Shift+Tab    Cycle modes              │
│                                         │
│ Navigation                              │
│   /           Command palette           │
│   Tab         Next element              │
│   Esc         Close/Cancel              │
│                                         │
│ Conversation                            │
│   Ctrl+N      New conversation          │
│   Ctrl+Z/Y    Undo/Redo                │
│   Ctrl+B      Branch                    │
│                                         │
│ Session                                 │
│   Ctrl+S      Session manager           │
│   F5          Create checkpoint         │
│   F6          Restore checkpoint        │
│                                         │
│ Advanced                                │
│   Ctrl+A      Agent panel               │
│   Ctrl+H      History browser           │
│   Ctrl+J      Jump to message           │
│   Ctrl+T      Context window            │
│   Ctrl+?      Help                      │
└─────────────────────────────────────────┘
```

### Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `AGCODEX_CONFIG` | Config file path | `~/.agcodex/config.toml` |
| `AGCODEX_MODE` | Default mode | `plan`, `build`, `review` |
| `OPENAI_API_KEY` | OpenAI API key | `sk-...` |
| `ANTHROPIC_API_KEY` | Anthropic API key | `sk-ant-...` |
| `AGCODEX_LOG_LEVEL` | Logging level | `debug`, `info`, `warn` |
| `AGCODEX_CACHE_DIR` | Cache directory | `~/.agcodex/cache` |

---

*AGCodex - Intelligent AI Coding Assistant*

Version 1.0.0 | [GitHub](https://github.com/your-org/agcodex) | [Documentation](https://docs.agcodex.dev)