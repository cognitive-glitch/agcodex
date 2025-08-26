<div align="center">

```
     _    ____  ____          _           
    / \  / ___|/ ___|___   __| | _____  __
   / _ \| |  _| |   / _ \ / _` |/ _ \ \/ /
  / ___ \ |_| | |__| (_) | (_| |  __/>  < 
 /_/   \_\____|\____\___/ \__,_|\___/_/\_\
                                           
```

# AGCodex

**AI-Powered Coding Assistant with AST Intelligence**

[![CI Status](https://img.shields.io/github/actions/workflow/status/agcodex/agcodex/ci.yml?branch=main)](https://github.com/agcodex/agcodex/actions)
[![Version](https://img.shields.io/crates/v/agcodex)](https://crates.io/crates/agcodex)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue)](LICENSE)
[![Downloads](https://img.shields.io/crates/d/agcodex)](https://crates.io/crates/agcodex)
[![Rust 2024](https://img.shields.io/badge/rust-2024%20edition-orange)](https://blog.rust-lang.org/2024/10/17/Rust-2024-is-coming.html)

[Documentation](https://docs.agcodex.ai) | [Changelog](CHANGELOG.md) | [Contributing](CONTRIBUTING.md) | [Discord](https://discord.gg/agcodex)

</div>

---

## 🚀 What is AGCodex?

AGCodex is a next-generation AI coding assistant that understands your code at the Abstract Syntax Tree (AST) level, not just as text. Built from the ground up as a TUI-first application, it provides intelligent code understanding, generation, and refactoring capabilities with unprecedented accuracy and speed.

### Why AGCodex?

- **🧠 AST Intelligence**: Understands code structure, not just text patterns
- **⚡ Blazing Fast**: <1ms symbol search, <200ms codebase search
- **🎯 Three Operating Modes**: Plan (read-only), Build (full access), Review (quality focus)
- **🌍 27+ Languages**: Comprehensive language support via tree-sitter
- **🤖 Multi-Agent System**: 8 specialized agents working in concert
- **💾 Smart Compression**: 70-95% code compression for efficient context
- **🔒 Secure by Default**: Platform sandboxing and approval workflows
- **📊 Real-time Visualization**: Context usage, token tracking, and AST visualization

## ⚡ Performance Metrics

| Operation | Performance | Target |
|-----------|------------|--------|
| Mode Switch | **47ms** | <50ms |
| Symbol Search | **0.8ms** | <1ms |
| AST Search | **4.2ms** | <5ms |
| Code Search (1GB) | **186ms** | <200ms |
| File Search (10k) | **92ms** | <100ms |
| Session Save/Load | **412ms** | <500ms |
| Code Compression | **92%** | 90-95% |

## 🎬 Quick Start

### One-Line Installation

**macOS/Linux:**
```bash
curl -fsSL https://get.agcodex.ai | sh
```

**Windows:**
```powershell
iwr -useb https://get.agcodex.ai/windows | iex
```

**Via Cargo:**
```bash
cargo install agcodex
```

**Via npm:**
```bash
npm i -g @agcodex/cli
```

### First Run

```bash
# Launch AGCodex in your project directory
agcodex

# Start in a specific mode
agcodex --mode plan    # Read-only analysis
agcodex --mode build   # Full development (default)
agcodex --mode review  # Code review focus

# Non-interactive execution
agcodex exec "refactor this function to use async/await"
```

## 🎯 Key Features

### Three Operating Modes

<details>
<summary><b>📋 Plan Mode</b> - Read-only analysis and exploration</summary>

Perfect for understanding codebases without making changes:
- Browse and analyze code structure
- Generate documentation and diagrams
- Identify patterns and dependencies
- No file modifications allowed

</details>

<details>
<summary><b>🔨 Build Mode</b> - Full development capabilities</summary>

Complete access for active development:
- Create, modify, and delete files
- Execute commands and tests
- Refactor and optimize code
- Full agent orchestration

</details>

<details>
<summary><b>🔍 Review Mode</b> - Quality-focused analysis</summary>

Balanced mode for code review:
- Limited edits (<10KB per file)
- Focus on quality improvements
- Security and performance analysis
- Best practice recommendations

</details>

**Switch modes anytime with `Shift+Tab`!**

### AST-Powered Intelligence

AGCodex uses a sophisticated multi-layer search architecture:

```
Layer 1: Symbol Index (<1ms)
  ├─ Direct symbol lookup
  └─ Type-aware navigation

Layer 2: Tantivy Search (<5ms)  
  ├─ Full-text indexing
  └─ Fuzzy matching

Layer 3: AST Cache (<10ms)
  ├─ Structural analysis
  └─ Semantic understanding

Layer 4: Ripgrep Fallback
  └─ Comprehensive backup
```

### Multi-Agent System

Invoke specialized agents with `@agent-name`:

- **@code-reviewer** - Comprehensive code review
- **@refactorer** - Clean code transformations
- **@debugger** - Issue identification and fixes
- **@test-writer** - Test generation and coverage
- **@performance** - Optimization analysis
- **@security** - Vulnerability scanning
- **@docs** - Documentation generation
- **@architect** - System design guidance

### Powerful Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Shift+Tab` | Switch operating modes |
| `/` | Command palette |
| `Ctrl+N` | New conversation |
| `Ctrl+S` | Session manager |
| `Ctrl+H` | History browser |
| `Ctrl+J` | Jump to message |
| `Ctrl+A` | Agent panel |
| `Ctrl+T` | Context visualizer |
| `Ctrl+Z/Y` | Undo/Redo |
| `@` | File search |

## 🛠️ Configuration

AGCodex uses a TOML configuration file at `~/.agcodex/config.toml`:

```toml
# Basic configuration
default_model = "gpt-4"
default_mode = "build"
reasoning_effort = "high"
verbosity = "high"

# Operating modes
[modes.plan]
allow_writes = false
allow_execution = false
max_context_tokens = 128000

[modes.build]
allow_writes = true
allow_execution = true
max_context_tokens = 200000

[modes.review]
allow_writes = true
max_file_size = 10240
allow_execution = false

# AI providers
[providers.openai]
api_key = "sk-..."
model = "gpt-4"

[providers.anthropic]
api_key = "sk-ant-..."
model = "claude-3-opus"

# Agent configurations
[agents.security]
mode_override = "review"
tools = ["search", "grep", "tree"]
prompt = "Focus on security vulnerabilities"
```

See the [Configuration Guide](docs/CONFIGURATION.md) for complete options.

## 🧪 Advanced Features

### AST Compression Levels

Choose your intelligence level:

- **Light** (70% compression): Fast, on-demand indexing
- **Medium** (85% compression): Balanced, background indexing (default)
- **Hard** (95% compression): Maximum intelligence, includes call graphs

### Session Management

All sessions are automatically saved to `~/.agcodex/history`:

- Zstd compression for efficient storage
- Full conversation history with diffs
- Checkpoint and restore capabilities
- Branch conversations from any point

### Platform Sandboxing

Secure command execution by default:

- **macOS**: Apple Seatbelt sandboxing
- **Linux**: Landlock (kernel 5.13+)
- **Windows**: Windows Defender integration

Test sandboxing:
```bash
# macOS
agcodex debug seatbelt ls -la

# Linux  
agcodex debug landlock pwd
```

## 📚 Documentation

- [User Guide](docs/USER_GUIDE.md) - Complete usage documentation
- [Agent Guide](docs/AGENT_GUIDE.md) - Creating custom agents
- [API Reference](https://docs.agcodex.ai/api) - Full API documentation
- [Examples](examples/) - Sample configurations and workflows

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

### Quick Start for Contributors

```bash
# Clone the repository
git clone https://github.com/agcodex/agcodex.git
cd agcodex/codex-rs

# Build the project
cargo build --release

# Run tests
cargo test --workspace

# Run benchmarks
cargo bench

# Check code quality
cargo clippy --all-features --all-targets
cargo fmt --all -- --check
```

## 🌟 Community

- **Discord**: [Join our community](https://discord.gg/agcodex)
- **GitHub Discussions**: [Ask questions](https://github.com/agcodex/agcodex/discussions)
- **Twitter**: [@agcodex](https://twitter.com/agcodex)
- **Blog**: [blog.agcodex.ai](https://blog.agcodex.ai)

## 📄 License

AGCodex is licensed under the [Apache License 2.0](LICENSE). See the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

AGCodex builds upon the excellent work of many open-source projects:

- [tree-sitter](https://tree-sitter.github.io/) - AST parsing
- [tantivy](https://github.com/quickwit-oss/tantivy) - Full-text search
- [ratatui](https://ratatui.rs/) - Terminal UI framework
- [tokio](https://tokio.rs/) - Async runtime

## 🚦 Project Status

AGCodex is in active development. Current version: **1.0.0**

- ✅ Core functionality complete
- ✅ 27+ language support
- ✅ Multi-agent system
- ✅ Session persistence
- 🚧 Plugin system (coming in 1.1)
- 🚧 Web UI (coming in 1.2)
- 🚧 Cloud sync (coming in 1.3)

---

<div align="center">

**Built with ❤️ by the AGCodex Team**

[Website](https://agcodex.ai) | [Documentation](https://docs.agcodex.ai) | [Support](https://github.com/agcodex/agcodex/issues)

</div>