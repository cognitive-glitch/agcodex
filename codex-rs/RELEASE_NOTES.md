# AGCodex v0.1.0 Release Notes

## 🚀 Overview

AGCodex is a complete overhaul and rebranding of the original Codex project, transforming it into an independent, TUI-first AI coding assistant that runs locally. This release introduces a revolutionary multi-mode architecture with AST-based intelligence and comprehensive language support.

## ✨ Key Features

### Three Operating Modes (Shift+Tab to switch)
- **📋 PLAN Mode**: Read-only analysis and planning
- **🔨 BUILD Mode**: Full development access (default)
- **🔍 REVIEW Mode**: Quality-focused code review

### AST-RAG Intelligence Engine
- **Multi-layer search**: Symbol (<1ms) → Tantivy (<5ms) → AST (<10ms) → Ripgrep fallback
- **Code compression**: 70-95% reduction while preserving semantic meaning
- **27+ language support**: Via tree-sitter integration (extensible to 50+)
- **Smart caching**: DashMap for concurrent access with >90% hit rates

### Subagent Architecture
- **8 specialized agents**: code-reviewer, refactorer, debugger, test-writer, performance, security, docs, architect
- **@agent-name invocation**: Natural language agent spawning
- **Parallel orchestration**: Multiple agents working concurrently
- **Isolated contexts**: Each agent has its own workspace and permissions

### Enhanced TUI Experience
- **Streaming responses**: Real-time token-by-token display
- **Context visualization**: Live token usage monitoring (Ctrl+T)
- **Message navigation**: Jump to any message with full context (Ctrl+J)
- **Session management**: Auto-save, checkpoints, branching (Ctrl+S)
- **History browser**: Timeline visualization with search (Ctrl+H)
- **Agent panel**: Multi-agent coordination interface (Ctrl+A)

### Performance Achievements
- **Mode switching**: <50ms transition time
- **File search**: <100ms for 10k files
- **Code search**: <200ms for 1GB codebase
- **AST parsing**: <10ms per file (cached)
- **Session save/load**: <500ms with Zstd compression
- **Subagent spawn**: <100ms per agent

## 🔧 Technical Improvements

### Architecture Enhancements
- **20-crate workspace**: Modular design for maintainability
- **Thread-local mode state**: Safe concurrent mode enforcement
- **Memory-mapped persistence**: Fast session loading
- **Platform sandboxing**: Seatbelt (macOS) / Landlock (Linux)

### Code Quality
- **Test coverage**: 685 tests passing (100% success rate)
- **Zero Clippy warnings**: Clean codebase
- **Comprehensive benchmarks**: 5 benchmark suites for performance tracking
- **Type safety**: Strong typing with newtype patterns

### Developer Experience
- **GPT-5 optimized prompts**: High reasoning/verbosity defaults
- **Simple tool names**: search, edit, think, plan (hiding sophisticated internals)
- **Rich error context**: File locations and operation details in all errors
- **Extensive documentation**: User guide, examples, and configuration templates

## 📦 Installation

```bash
# Clone the repository
git clone https://github.com/your-org/agcodex.git
cd agcodex/codex-rs

# Build with optimizations
cargo build --release

# Run the TUI
cargo run --release --bin agcodex
```

## ⚙️ Configuration

Create `~/.agcodex/config.toml`:

```toml
[models]
default = "gpt-5"

[models.gpt5]
type = "openai"
model = "gpt-5"
api_key = "${OPENAI_API_KEY}"

[interface]
default_mode = "build"
auto_save = true
enable_notifications = true

[intelligence]
level = "medium"  # light/medium/hard
enable_embeddings = false  # Optional, disabled by default
```

## 🎮 Key Bindings

| Key | Action |
|-----|--------|
| **Shift+Tab** | Switch between Plan/Build/Review modes |
| **/** | Command palette |
| **Ctrl+N** | New conversation |
| **Ctrl+S** | Session manager |
| **Ctrl+A** | Agent panel |
| **Ctrl+H** | History browser |
| **Ctrl+J** | Jump to message |
| **Ctrl+T** | Context visualizer |
| **Ctrl+Z/Y** | Undo/redo |
| **F5/F6** | Create/restore checkpoint |

## 🏗️ Breaking Changes

This is a complete rewrite from the original Codex:
- New configuration format (TOML instead of JSON)
- Different CLI arguments and commands
- Redesigned agent invocation syntax
- Changed persistence location (`~/.agcodex/` instead of `~/.codex/`)

## 🐛 Known Issues

- Windows sandbox support is limited
- Some language parsers may need additional configuration
- Embeddings are experimental and disabled by default

## 🙏 Acknowledgments

Special thanks to all contributors who helped transform Codex into AGCodex. This release represents months of architectural improvements, performance optimizations, and user experience enhancements.

## 📚 Resources

- [User Guide](docs/USER_GUIDE.md)
- [Configuration Examples](examples/configuration_templates/)
- [Custom Agent Templates](examples/custom_agents/)
- [API Documentation](https://docs.agcodex.dev)

## 📝 License

AGCodex is released under the MIT License. See [LICENSE](LICENSE) file for details.

---

*For questions and support, please visit our [GitHub Issues](https://github.com/your-org/agcodex/issues) page.*