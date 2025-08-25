# Changelog

All notable changes to AGCodex will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-01-25

### Added
- **Operating Modes System**: Plan/Build/Review modes with Shift+Tab switching
- **AST-RAG Intelligence Engine**: Multi-layer search with 70-95% code compression
- **Subagent Architecture**: 8 specialized agents with @agent-name invocation
- **Tree-sitter Integration**: Support for 27+ programming languages
- **Session Persistence**: Zstd-compressed storage at ~/.agcodex/history
- **Context Visualization**: Real-time token usage monitoring (Ctrl+T)
- **Message Navigation**: Jump to any message with context restoration (Ctrl+J)
- **History Browser**: Timeline visualization with search capabilities (Ctrl+H)
- **Agent Panel**: Multi-agent coordination interface (Ctrl+A)
- **Platform Sandboxing**: Seatbelt (macOS) and Landlock (Linux) support
- **Comprehensive Benchmarks**: 5 benchmark suites for performance tracking
- **User Documentation**: Complete user guide and configuration examples
- **Custom Agent Templates**: YAML configurations for specialized agents

### Changed
- Complete rebranding from Codex to AGCodex
- Redesigned as TUI-first application (CLI secondary)
- Moved from JSON to TOML configuration format
- Changed persistence location from ~/.codex/ to ~/.agcodex/
- Simplified tool names (search, edit, think, plan) with sophisticated internals
- GPT-5 optimized prompts with high reasoning/verbosity defaults

### Fixed
- 131 Clippy warnings resolved
- All test failures fixed (685 tests passing)
- Compression ratio enforcement (Light < Medium < Hard)
- Thread-safe mode switching with thread-local storage
- Proper error handling with thiserror (removed anyhow)

### Performance
- Mode switching: <50ms
- Symbol search: <1ms
- Tantivy search: <5ms
- AST parsing: <10ms per file
- Session save/load: <500ms
- Subagent spawn: <100ms
- File search: <100ms for 10k files
- Code search: <200ms for 1GB codebase

### Security
- Platform-specific sandboxing for command execution
- Approval workflows for filesystem modifications
- Secure credential storage using platform keychains
- No logging of sensitive data (tokens, keys)

## [0.0.1] - 2024-12-01

### Added
- Initial Codex prototype
- Basic chat interface
- Simple file operations
- OpenAI API integration

[Unreleased]: https://github.com/your-org/agcodex/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/your-org/agcodex/compare/v0.0.1...v0.1.0
[0.0.1]: https://github.com/your-org/agcodex/releases/tag/v0.0.1