# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**AGCodex** is a complete overhaul transforming the original Codex project into an independent, TUI-first AI coding assistant that runs locally. This is a completely rebranded and redesigned system with enhanced AST-based intelligence, configurable operating modes, and comprehensive language support via tree-sitter.

### Core Architecture Philosophy
- **Three simple operating modes**: Plan (read-only), Build (full access), Review (quality focus) with Shift+Tab switching
- **Comprehensive language support**: Tree-sitter integration for 27+ languages (extensible to 50+)
- **AST-RAG intelligence**: Hierarchical retrieval with multi-layer search and 70-95% code compression
- **Efficient session persistence**: Storage at ~/.agcodex/history with Zstd compression
- **GPT-5 optimized**: Structured XML-like prompts, high reasoning/verbosity defaults
- **Internal agent tools**: Simple names (search, edit, think, plan) hiding sophisticated implementations

## Current Architecture

### Core Systems
- **TUI**: Chat interface with streaming, file search, keyboard shortcuts
- **Conversation**: UUID tracking, turn history, diff tracking
- **Client**: Multi-provider support (OpenAI, Anthropic, Ollama), streaming
- **Security**: Platform sandboxing (Seatbelt/Landlock), approval workflows

#### Internal Tools Architecture
```rust
// Simple external names, sophisticated internal implementations
pub enum Tool {
    Search,  // Multi-layer Tantivy engine
    Edit,    // Basic patch-based editing  
    Think,   // Internal reasoning strategies
    Plan,    // Double-planning decomposition
    Glob,    // File discovery (fd_find.rs)
    Tree,    // Tree-sitter parser (27 languages)
    Grep,    // AST-grep pattern matching
    Bash,    // Enhanced safety validation
    Index,   // Tantivy indexing
    Patch,   // AST transformations (planned)
}
```

#### Search Engine Architecture
```rust
pub struct SearchEngine {
    symbol_index: Arc<DashMap<String, Vec<Symbol>>>,  // Layer 1: <1ms
    tantivy: Arc<TantivyIndex>,                       // Layer 2: <5ms
    ast_cache: Arc<DashMap<PathBuf, ParsedAST>>,     // Layer 3: <10ms
    ripgrep: RipgrepFallback,                        // Layer 4: fallback
}
```

#### AST Intelligence
- **Tree-sitter**: 27 language parsers integrated
- **Language registry**: Auto-detection from file extensions
- **Code compression**: 70-95% reduction (3 levels)
- **Location tracking**: Precise file:line:column metadata

### Key Features to Implement
- **Operating Modes**: Plan/Build/Review with Shift+Tab switching (see modes.rs)
- **Subagent System**: @agent-name invocation with isolated contexts
- **Enhanced TUI**: Ctrl+J/H/S/O/A/Z/Y for navigation and management
- **Embeddings**: Optional multi-provider system (disabled by default)

## Commands

### Testing
```bash
cargo test --no-fail-fast           # Run all tests
cargo test -p agcodex-core          # Test specific crate
cargo test -- --nocapture           # Show output
```

### Quality Checks
```bash
cargo +nightly fmt --all             # Format (required)
cargo clippy --all-features --all-targets --workspace --tests -- -D warnings  # Lint
```

### Running
```bash
cargo run --bin agcodex              # Launch TUI
cargo run --bin agcodex -- --mode plan/build/review  # Specific mode
cargo run --bin agcodex exec -- "task"  # Headless mode
```

### TUI Controls Once Launched
- **`Shift+Tab`** - **MODE SWITCHING**: Cycle between Plan/Build/Review modes
- **`/`** - Command palette (search for any action)
- **`Ctrl+N`** - New conversation
- **`Ctrl+S`** - Session management (save/load from ~/.agcodex/history)
- **`Ctrl+A`** - Agent panel
- **`Ctrl+H`** - History browser with timeline visualization
- **`Ctrl+J`** - Jump to message with context restoration
- **`Ctrl+Z/Y`** - Undo/redo conversation turns
- **`Ctrl+B`** - Branch conversation from current point
- **`Ctrl+?`** - Show all keybindings
- **`Esc`** - Close panel/cancel operation
- **`Tab`** - Cycle through UI elements
- **`F5`** - Create checkpoint
- **`F6`** - Restore checkpoint

## Architecture and Code Organization

### Workspace Structure (20 Crates)
The codebase is organized as a Cargo workspace with the following crates:

#### Core Components
- **`tui/`**: PRIMARY INTERFACE - Terminal UI with mode switching (Plan/Build/Review)
- **`core/`**: Business logic with AST-RAG engine and tree-sitter integration
- **`cli/`**: Command-line interface entry point (mainly launches TUI)
- **`exec/`**: Headless/non-interactive execution mode (secondary)
- **`persistence/`**: Session management with ~/.agcodex/history storage (NEW)

#### Communication & Protocol
- **`protocol/`**: Communication protocol definitions
- **`protocol-ts/`**: TypeScript protocol bindings
- **`mcp-client/`**: MCP client for connecting to servers
- **`mcp-server/`**: MCP server mode for AGCodex
- **`mcp-types/`**: Shared MCP type definitions

#### Security & Sandboxing
- **`execpolicy/`**: Sandboxing and execution policy enforcement
- **`linux-sandbox/`**: Linux-specific sandboxing using Landlock/seccomp

#### Utilities & Integration
- **`file-search/`**: Enhanced with tree-sitter AST search and fd-find integration
- **`apply-patch/`**: AST-based patching with precise location metadata
- **`ansi-escape/`**: ANSI escape sequence handling
- **`common/`**: Shared utilities across crates
- **`login/`**: Authentication management
- **`chatgpt/`**: ChatGPT-specific authentication and session management
- **`ollama/`**: Ollama integration for local LLM support
- **`arg0/`**: Argument handling utilities

### Key Architectural Components

#### 1. Client Architecture (`core/src/client*.rs`)
- `client.rs`: Main client implementation with streaming support
- `client_common.rs`: Shared client utilities and types
- Handles API communication with OpenAI/compatible providers
- Supports both Chat Completions and Responses APIs
- Manages authentication (ChatGPT login and API keys)

#### 2. Conversation Management (`core/src/conversation_*.rs`)
- `conversation_manager.rs`: Main conversation state management
- `conversation_history.rs`: Turn-based history tracking
- `agcodex_conversation.rs`: AGCodex-specific conversation logic
- Handles turn-based interactions with diff tracking

#### 3. Execution Environment (`core/src/exec*.rs`)
- `exec.rs`: Main command execution interface
- `exec_env.rs`: Environment configuration for sandboxed execution
- Platform-specific sandbox implementations (Seatbelt on macOS, Landlock on Linux)
- Safety checks and approval workflows

#### 4. Configuration System (`core/src/config*.rs`)
- `config.rs`: Main configuration loading from ~/.agcodex/config.toml
- `config_types.rs`: Type definitions with HIGH reasoning/verbosity defaults
- `config_profile.rs`: Profile-based configuration support
- `config_modes.rs`: Plan/Build/Review mode configurations (NEW)
- TOML-based configuration with model provider definitions
- Configurable embedding options: Light/Medium/Hard intelligence levels

#### 5. MCP Integration (`core/src/mcp_*.rs`)
- `mcp_connection_manager.rs`: MCP server connection management
- `mcp_tool_call.rs`: Tool invocation handling
- Supports both client and server modes

#### 6. TUI Components (`tui/src/*.rs`) - PRIMARY INTERFACE
- `app.rs`: Main TUI application state and event loop
- `ui.rs`: UI rendering and layout management
- `input.rs`: Keyboard and mouse input handling
- `session_ui.rs`: Session management interface
- `agent_ui.rs`: Multi-agent coordination interface
- `history_browser.rs`: Conversation history navigation
- `notification.rs`: In-TUI notification system
- `widgets/`: Custom Ratatui widgets for AGCodex

## Key Implementation Details

### Type System Patterns
- **Newtype**: Strong typing for domain concepts
- **Builder**: Fluent API construction
- **Typestate**: Compile-time state machines

## Operating Modes (core/src/modes.rs)

- **📋 PLAN**: Read-only analysis mode (no writes/execution)
- **🔨 BUILD**: Full access mode (default)
- **🔍 REVIEW**: Quality focus mode (limited edits <10KB)
- **Switching**: Shift+Tab cycles between modes
- **Enforcement**: ModeManager validates operations against restrictions

## Subagent System

### Invocation
- Use `@agent-name` pattern in prompts
- Available: code-reviewer, refactorer, debugger, test-writer, performance, security, docs, architect

### Configuration
- Location: `~/.agcodex/agents/` (user) or `.agcodex/agents/` (project)
- YAML format with name, tools, mode_override, and custom prompt
- Features: Mode override, tool restrictions, context isolation

## Design Requirements

### Error Handling Strategy
- **Use thiserror exclusively**: Domain-specific error types in each crate
- **No anyhow**: Replace all `anyhow::Result` with specific `Result<T, DomainError>`
- **Rich error context**: Include file locations, operation details in errors

### Type System Requirements
- **Newtype pattern**: Strong typing for FilePath, LineNumber, AstNodeId
- **Builder pattern**: Fluent APIs for complex configurations
- **Typestate pattern**: Compile-time guarantees for state machines

### Tool Design Principles
1. **Simple external names**: search, edit, think, plan, glob, tree, grep, bash, index, patch
2. **Sophisticated internals**: Multi-layer engines, caching, optimization
3. **Context-aware outputs**: Rich metadata for LLM consumption
4. **Performance tiers**: Fast (edit) → Smart (patch) → Comprehensive (search)
5. **No redundancy**: Each tool has unique, clear purpose

### 5. Session Management and Persistence

**Required Features**:
- **SessionManager**: Auto-save, checkpointing, and recovery
- **ConversationHistory**: Undo/redo support with context snapshots
- **MessageNavigator**: Jump to any message with full context restoration
- **Branching**: Create conversation branches from any point
- **ContextSnapshot**: Complete state preservation for navigation

### 6. Multi-Agent Architecture

**Components**:
- **AgentOrchestrator**: Spawn and coordinate multiple agents with message bus
- **Git Worktree Integration**: Isolated worktrees for parallel development
- **SubAgent**: Specialized agents with custom capabilities

### 7. Notification System

**Features**:
- Terminal bell notifications (`\x07`)
- Desktop notifications via system integration
- Custom notification hooks
- Multiple notification levels (Info, Warning, Error, TaskComplete)

## Internal Tools (10 Tools with Simple Names)

### Philosophy
- Simple external names, sophisticated internal implementations
- All tools provide context-aware outputs for LLM consumption

### Tools
1. **search** - Multi-layer engine (Symbol <1ms, Tantivy <5ms, AST <10ms)
2. **edit** - Basic patch-based editing (<1ms)
3. **think** - Reasoning strategies (Sequential, Shannon, Actor-Critic)
4. **plan** - Double-planning with parallelization analysis
5. **glob** - File discovery respecting .gitignore
6. **tree** - Tree-sitter parser for 27 languages
7. **grep** - AST pattern matching with YAML rules
8. **bash** - Safe command execution with validation
9. **index** - Tantivy indexing (integrated in search)
10. **patch** - AST transformations (planned)

### Tool Output Structure
All tools return context-aware outputs with:
- Core result with before/after states
- Surrounding context and scope information
- Change tracking with semantic impact
- Performance metrics and strategy rationale




## AST-RAG Architecture

### Intelligence Modes
- **Light**: 70% compression, on-demand indexing, 256 chunk size
- **Medium** (default): 85% compression, background indexing, 512 chunk size
- **Hard**: 95% compression, aggressive indexing, includes call graph & data flow

## Embeddings (Optional)

- **Disabled by default** with zero overhead
- **Multi-provider**: OpenAI, Gemini, Voyage AI
- **Independent auth**: Separate API keys from chat models
- **Config**: `~/.agcodex/config.toml` under `[embeddings]`
- **When disabled**: AST search works perfectly without embeddings

## Session Persistence

- **Location**: `~/.agcodex/history` with Zstd compression
- **Formats**: Bincode (metadata), MessagePack (messages)
- **Fast loading** via memory-mapped metadata


## Performance Targets

### Speed Metrics
- **Mode switch**: <50ms
- **Language detection**: <10ms with 100% accuracy
- **File search**: <100ms for 10k files
- **Code search**: <200ms for 1GB codebase
- **AST parsing**: <10ms per file (cached)
- **Session save/load**: <500ms
- **Subagent spawn**: <100ms

### Efficiency Metrics
- **Code compression**: 90-95% (AI Distiller)
- **Cache hit rate**: >90% for hot paths
- **Memory usage**: <2GB for 100k chunks
- **Initial indexing**: 2-5 min for 1M LOC
- **Incremental updates**: <1s per file change

### Quality Metrics
- **Location precision**: Exact line:column
- **Retrieval accuracy**: 85-95% relevant chunks
- **Edit validity**: 100% syntactically correct
- **Test coverage**: >80% for new code

## Development Guidelines

### Best Practices
- **Error Handling**: ALWAYS use thiserror, NEVER use anyhow
- **Testing**: 80% minimum coverage, colocated unit tests
- **Performance**: Use Arc<str> for shared strings, SmallVec for small collections
- **Profiling**: cargo flamegraph for CPU, criterion for benchmarks

## Platform-Specific Considerations

### macOS
- Sandbox: Apple Seatbelt (`sandbox-exec`)
- Test: `agcodex debug seatbelt [COMMAND]`
- Keychain integration for credential storage

### Linux
- Sandbox: Landlock (kernel 5.13+) and seccomp
- Test: `agcodex debug landlock [COMMAND]`
- May require adjustments in containers

### Windows
- Limited sandbox support
- Uses Windows Defender SmartScreen for trust
- Credential Manager for secure storage

## Security Requirements

- NEVER log or expose sensitive data (tokens, keys)
- All external inputs must be validated
- Use constant-time comparisons for secrets
- Sandbox all command execution by default
- Require explicit user approval for filesystem writes outside workspace

## TUI Development Patterns

### Core Components
- **AGCodexWidget trait**: render, handle_input, handle_mouse, get_help_text
- **AppState**: Manages mode, conversation, agents, session, UI state
- **AppMode**: Normal, SessionManager (Ctrl+S), HistoryBrowser (Ctrl+H), AgentPanel (Ctrl+A), MessageJump (Ctrl+J)
- **AppEvent**: Handles key/mouse input, agent events, session events, notifications

## Common Development Workflows

**Adding TUI Feature**: Create widget → Add state → Add keybinding → Handle events → Update help → Test

**Adding AST Feature**: Add grammar → Create parser → Implement trait → Add caching → Test → Benchmark

**New Tool Integration**: Create module → Define errors → Implement CodeTool → Add config → Test → Document

**New Error Type**: Define with thiserror → Add conversions → Recovery logic → Document → Test

## Performance Profiling

- **CPU**: `cargo flamegraph --bin agcodex`
- **Memory**: `cargo bloat --release --bin agcodex`
- **Benchmarks**: `cargo bench --bench [context_engine|ast_indexer|session_persistence]`

## MCP (Model Context Protocol) Notes

- Client config: `~/.agcodex/config.toml` under `[mcp_servers]`
- Server mode: `agcodex mcp`
- Debug with: `npx @modelcontextprotocol/inspector agcodex mcp`
- Supports tool discovery and invocation

## Critical Path Optimizations

Priority areas for optimization:
1. AST parsing and caching (use `DashMap` for concurrent access)
2. Context retrieval (implement incremental indexing)
3. File watching (use `notify` crate efficiently)
4. Streaming response handling (minimize allocations)
5. Agent coordination overhead (use message passing, not shared state)
6. Session checkpointing (async background saves)
7. Worktree operations (batch git operations)


## TUI Principles
- All features accessible via keyboard shortcuts
- Progressive disclosure with panels/modals
- Visual feedback for long operations
- Context preservation during navigation




## Key Design Principles
- **Three modes**: Plan/Build/Review with Shift+Tab switching
- **27 language support** via tree-sitter (extensible to 50+)
- **AST-RAG architecture** with 70-95% compression
- **Simple tool names** hiding sophisticated implementations
- **TUI-first** with all features accessible via keyboard
- **Context preservation** throughout navigation
- **GPT-5 optimized** with high reasoning/verbosity defaults