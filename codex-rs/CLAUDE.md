# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**AGCodex** is a complete overhaul of the original Codex project - an independent, TUI-first AI coding assistant that runs locally. This is NOT a migration but a completely rebranded and redesigned system with enhanced AST-based intelligence, configurable operating modes, and comprehensive language support via tree-sitter.

### Key Transformation Goals
- **Complete rebranding**: codex → agcodex across all crates and repositories
- **Three operating modes**: Plan (read-only), Build (full access), Review (quality focus)
- **50+ language support**: Comprehensive tree-sitter integration out of the box
- **AST-RAG intelligence**: Hierarchical retrieval with 90%+ code compression
- **Session persistence**: Efficient storage at ~/.agcodex/history with Zstd compression
- **GPT-5 best practices**: Structured XML-like prompts, high reasoning/verbosity defaults
- **Internal agent tools**: AST-powered code analysis and transformation tools

## Current Implementation State

### What's Working
- **TUI Foundation**: Basic ChatWidget, onboarding flow, file search integration
- **Conversation Management**: UUID-based tracking, turn history, diff tracking
- **Client Architecture**: Dual API support (Responses/Chat), streaming, multi-provider
- **Sandboxing**: Platform-specific (Seatbelt/Landlock/seccomp) with approval workflows
- **MCP Protocol**: Client/server modes with tool discovery and invocation
- **Basic Agent Support**: Simple spawn_agent function in TUI
- **Operating Modes**: Basic ModeManager implementation with Plan/Build/Review (2025-08-21)
- **Error Handling**: Domain-specific error types with thiserror (2025-08-21)
- **Workspace Dependencies**: Consolidated ~80 dependencies in root Cargo.toml (2025-08-21)

### Critical Gaps (Must Implement)
- ~~**Error Handling**: 21 anyhow uses vs 4 thiserror~~ ✅ **COMPLETE** (2025-08-21)
- **AST Intelligence**: Need full tree-sitter (50+ langs), ast-grep, AI Distiller-style compaction
  - **Plan created**: Comprehensive implementation strategy ready
- **Session Management**: No persistence at ~/.agcodex/history, missing smooth switching UX
- ~~**Operating Modes**: No Plan/Build/Review modes~~ ✅ **SCAFFOLDED** (needs TUI integration)
- **Mode Switching**: Missing Shift+Tab for instant mode cycling in TUI
  - **Plan created**: ModeIndicator widget and integration strategy ready
- **Embeddings**: No configurable Light/Medium/Hard intelligence options
  - **Plan created**: Multi-provider support with complete separation from chat models
- **Location Awareness**: No precise file:line:column metadata in embeddings
  - **Plan created**: SourceLocation type design complete
- **Internal Agent Tools**: AST-based analysis, search, and transformation tools
  - **Scaffolded**: Module structure in place
- ~~**Defaults**: Need HIGH reasoning effort and verbosity~~ ✅ **COMPLETE** (2025-08-21)
- **Multi-Agent**: No orchestrator, worktree management, or coordination
- **Type Safety**: Minimal newtype/builder/typestate patterns
- **TUI Features**: Missing Ctrl+J, Ctrl+H, Ctrl+S, Ctrl+A, Ctrl+Z/Y functionality

## Build and Development Commands

### Building the Project

### Running Tests
```bash
# Run full tests without being interrupted by failuresf in the workspace
cargo test --no-fail-fast

# Run tests for a specific crate
cargo test -p codex-core

# Run a specific test
cargo test test_name

# Run tests with output displayed
cargo test -- --nocapture

# Run tests with specific features
cargo test --all-features --no-fail-fast
```

### Code Quality Checks
```bash
# Check for compilation errors without building
cargo check --all-features --all-targets --workspace --tests

# Format code (REQUIRED before committing)
cargo fmt --all

# Lint autofix
cargo clippy --all-features --all-targets --workspace --tests --fix --allow-dirty

# Run clippy linter (REQUIRED before committing)
cargo clippy --all-features --all-targets --workspace --tests -- -D warnings


# Check individual crates to ensure proper feature specifications
```bash
# Use fd to locate Cargo.toml files at depth 2 (crate directories) and run cargo check for each
fd --type file --min-depth 2 --max-depth 2 -g 'Cargo.toml' -x cargo check --manifest-path {}
```

### Running the Application
```bash
# Launch TUI (primary interface)
cargo run --bin agcodex

# Launch TUI with initial prompt
cargo run --bin agcodex -- "explain this codebase to me"

# TUI with specific model preference (can be changed in TUI)
cargo run --bin agcodex -- --model o3

# Launch in specific mode (Plan/Build/Review)
cargo run --bin agcodex --mode plan    # Read-only analysis mode
cargo run --bin agcodex --mode build   # Full access mode (default)
cargo run --bin agcodex --mode review  # Quality review mode

# Secondary modes (not primary workflow):
cargo run --bin agcodex exec -- "your task here"  # Headless mode
cargo run --bin agcodex mcp                        # MCP server mode
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
- `codex_conversation.rs`: Codex-specific conversation logic
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
- `widgets/`: Custom Ratatui widgets for Codex

## Refactoring Priority Roadmap

### Phase 1: Foundation & Rebranding (IMMEDIATE)
1. **Complete rebranding**: codex → agcodex across all 19+ crates
   - **Status**: Plan created with automated script for 8,773 occurrences
2. **Implement operating modes**: Plan/Build/Review with Shift+Tab switching
   - **Status**: ModeManager scaffolded, TUI integration plan ready
3. **Session persistence**: Create ~/.agcodex/history with Zstd compression
   - **Status**: Design complete, implementation pending
4. ~~**Set HIGH defaults**: reasoning_effort=high, verbosity=high~~ ✅ **COMPLETE**
5. ~~**Complete anyhow→thiserror migration**~~ ✅ **COMPLETE** (2025-08-21)

### Phase 2: AST Intelligence (HIGH PRIORITY)
1. **Tree-sitter integration**: Add all 50+ language parsers
2. **AST-RAG engine**: Hierarchical retrieval (File→Class→Function)
3. **AI Distiller compaction**: 90%+ code compression
4. **Location-aware embeddings**: Precise file:line:column metadata
5. **Internal agent tools**: AST-based search, analysis, and transformation

### Phase 3: Core TUI Features (HIGH PRIORITY)
1. **Message Navigation** (Ctrl+J jump with context restoration)
2. **History Browser** (Ctrl+H with timeline visualization)
3. **Session switching**: Smooth UX for switching between sessions
4. **Multi-Agent Orchestrator** with git worktree support
5. **Notification system** (terminal bell, desktop notifications)

### Phase 4: Enhancement (MEDIUM PRIORITY)
1. **Type system improvements** (newtype, builder, typestate patterns)
2. **Configurable intelligence**: Light/Medium/Hard embedding options
3. **GPT-5 prompt optimization**: XML-structured prompts
4. **AST-based edit tools**: Precise patches with location metadata

## AGCodex Operating Modes

### Plan Mode (Read-Only)
```rust
// Activated by: Shift+Tab or --mode plan
pub struct PlanMode {
    capabilities: vec![
        Capability::ReadFiles,
        Capability::SearchCode,
        Capability::AnalyzeAST,
        Capability::GenerateDiagrams,
        Capability::ProposePlans,
    ],
    restrictions: vec![
        Restriction::NoFileWrites,
        Restriction::NoExecutions,
        Restriction::NoExternalAPIs,
    ],
    visual_indicator: "📋 PLAN",
    status_color: Color::Blue,
}
```

### Build Mode (Full Access)
```rust
// Activated by: Shift+Tab or --mode build (default)
pub struct BuildMode {
    capabilities: vec![
        Capability::All,  // Full access to all operations
    ],
    visual_indicator: "🔨 BUILD",
    status_color: Color::Green,
}
```

### Review Mode (Quality Focus)
```rust
// Activated by: Shift+Tab or --mode review
pub struct ReviewMode {
    capabilities: vec![
        Capability::ReadFiles,
        Capability::RunTests,
        Capability::Lint,
        Capability::SecurityScan,
        Capability::GenerateReports,
    ],
    restrictions: vec![
        Restriction::NoDestructiveOps,
    ],
    visual_indicator: "🔍 REVIEW",
    status_color: Color::Yellow,
}
```

### Mode Manager Implementation
ModeManager tracks current mode, history, and enforces restrictions:
- **Plan Mode**: Read-only access, no file writes or command execution
- **Build Mode**: Full access to all operations
- **Review Mode**: Quality-focused with limited edit capabilities (<10KB)
- Each mode provides specific prompts and capability restrictions

## AGCodex Subagent System

### Overview
AGCodex features a sophisticated subagent system that enables specialized AI assistants for task-specific workflows. Each subagent operates with its own context, custom prompts, and tool permissions.

### Invoking Subagents
```
@agent-code-reviewer - Proactive code quality analysis
@agent-refactorer - Systematic code restructuring
@agent-debugger - Deep debugging and root cause analysis
@agent-test-writer - Comprehensive test generation
@agent-performance - Performance optimization specialist
@agent-security - Security vulnerability analysis
@agent-docs - Documentation generation
@agent-architect - System design and architecture
```

### Subagent Configuration
```yaml
# ~/.agcodex/agents/code-reviewer.yaml
name: code-reviewer
description: Proactively reviews code for quality, security, and maintainability
mode_override: review  # Forces Review mode when active
tools:
  - Read
  - AST-Search
  - AST-Search
  - Tree-sitter-analyze
intelligence: hard  # Maximum AST analysis
prompt: |
  You are a senior code reviewer with AST-based analysis.
  Focus on:
  - Syntactic correctness via tree-sitter validation
  - Security vulnerabilities (OWASP Top 10)
  - Performance bottlenecks (O(n²) or worse)
  - Memory leaks and resource management
  - Error handling completeness
```

### Subagent Storage
```
~/.agcodex/
├── agents/              # User-level subagents
│   ├── global/         # Available everywhere
│   └── templates/      # Reusable templates
└── .agcodex/
    └── agents/         # Project-specific subagents
```

### Advanced Subagent Features
- **Mode-Aware**: Subagents can override operating mode and intelligence level
- **Chaining**: Sequential (→) or parallel (+) execution
- **Context Inheritance**: AST indices, embeddings, and session history preserved

## Critical Refactoring Requirements

### 1. Complete Migration from anyhow to thiserror
**MANDATORY**: Replace all uses of `anyhow` with idiomatic `thiserror` patterns throughout the codebase.
- Create domain-specific error types in each crate
- Replace `anyhow::Result` with specific `Result<T, DomainError>`
- Use `#[from]` for automatic error conversion
- Add contextual information to error variants

### 2. Enhanced AST-Based Code Intelligence

**Current State**: Basic fuzzy file search using `nucleo-matcher` without semantic understanding.

**Required Enhancements**:

#### A. Complete Tree-sitter Integration (50+ Languages)
Add comprehensive tree-sitter language support for: Core (Rust, Python, JS/TS, Go, Java, C/C++, C#), Web (HTML, CSS, JSON, YAML), Scripting (Bash, Ruby, PHP), Functional (Haskell, Elixir, Scala), Systems (Zig, Swift, Kotlin), Config/Data (SQL, Dockerfile, HCL), and Documentation (Markdown, LaTeX).

#### B. AST-grep Integration
Add `ast-grep-core` and `ast-grep-language` for structural code search and transformation.

#### C. Smart Context Retrieval Architecture
Create modular context engine with: ast_compactor (code compression), semantic_index (symbol indexing), retrieval (query-based search), embeddings (similarity), and cache (parsed AST storage).

### 3. Robust Type System Enhancements

**Required Patterns**:
- **Newtype Pattern**: Strong typing for domain concepts (FilePath, LineNumber, AstNodeId)
- **Builder Pattern**: Fluent APIs for complex type construction
- **Typestate Pattern**: Compile-time state machine guarantees

### 4. Native Tool Integration

**fd-find Integration**: Native file discovery using `ignore::WalkBuilder` with parallel search support.

**AST Search Tools**: Tree-sitter powered search with semantic understanding and precise location tracking.

**Unified Tool Interface**: All tools implement `CodeTool` trait with search and parallel search methods.

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

## AST-RAG Implementation Details

### Indexing Pipeline
- **ASTIndexer**: Parallel parsing for 50+ languages
- **Hierarchical Chunking**: File → Class → Function → Block
- **Location-aware Embeddings**: Precise file:line:column metadata
- **Vector Storage**: LanceDB with symbol graph relationships
- **Target**: 90%+ compression ratio with AI Distiller compaction

### Intelligence Modes Configuration

#### Light Mode (Fast, Minimal Resources)
```toml
[intelligence.light]
chunk_size = 256
max_chunks = 1000
cache_size_mb = 100
indexing = "on_demand"
compression_level = "basic"  # 70% compression
# Embedding models (if enabled):
# OpenAI: text-embedding-3-small (256 dims)
# Gemini: gemini-embedding-001 (256 dims)
# Voyage: voyage-3.5-lite
```

#### Medium Mode (Balanced, Default)
```toml
[intelligence.medium]
chunk_size = 512
max_chunks = 10000
cache_size_mb = 500
indexing = "background"
compression_level = "standard"  # 85% compression
include_ast = true
# Embedding models (if enabled):
# OpenAI: text-embedding-3-small (1536 dims)
# Gemini: gemini-embedding-001 (768 dims)
# Voyage: voyage-3.5
```

#### Hard Mode (Maximum Intelligence)
```toml
[intelligence.hard]
chunk_size = 1024
max_chunks = 100000
cache_size_mb = 2000
indexing = "aggressive"
compression_level = "maximum"  # 95% compression
include_ast = true
include_call_graph = true
include_data_flow = true
# Embedding models (if enabled):
# OpenAI: text-embedding-3-large (3072 dims)
# Gemini: gemini-embedding-exp-03-07 (1536 dims)
# Voyage: voyage-3-large
```

## Embeddings System (Optional, Independent)

### Core Design
- **Complete Separation**: 100% independent from chat/LLM models
- **Disabled by Default**: Opt-in feature with zero overhead when disabled
- **Multi-Provider Support**: OpenAI, Gemini, and Voyage AI
- **Independent Authentication**: Separate API keys from chat models

### Configuration
```toml
# ~/.agcodex/config.toml

# Embeddings disabled by default (zero overhead)
[embeddings]
enabled = false  # Set to true to enable
provider = "auto"  # auto, openai, gemini, voyage

[embeddings.openai]
model = "text-embedding-3-small"
dimensions = 1536
# API key from OPENAI_EMBEDDING_KEY env var

[embeddings.gemini]
model = "gemini-embedding-001"
dimensions = 768
# API key from GEMINI_API_KEY env var

[embeddings.voyage]
model = "voyage-3.5"
input_type = "document"
# API key from VOYAGE_API_KEY env var
```

### Environment Variables
- `OPENAI_EMBEDDING_KEY` - Separate from `OPENAI_API_KEY` for chat
- `GEMINI_API_KEY` - Used for Gemini embeddings
- `VOYAGE_API_KEY` - Voyage AI embeddings only

### Separate Authentication File
```json
// ~/.agcodex/embeddings_auth.json
{
  "openai_embedding_key": "sk-...",
  "gemini_embedding_key": "...",
  "voyage_embedding_key": "..."
}
```

### When Embeddings Are Disabled
- AST-based search works perfectly
- Tree-sitter semantic analysis fully functional
- Symbol search and definition finding unaffected
- Zero performance or memory overhead

## Session Persistence Implementation

### Storage Architecture
- **Formats**: Bincode (metadata), MessagePack (messages), Zstd (compression)
- **Location**: ~/.agcodex/history
- **Fast Loading**: Memory-mapped metadata with lazy message loading
- **Version Header**: Magic bytes "AGCX" for format detection

## Implementation Timeline

### Week 1: Foundation & Core
- **Day 1-2**: Complete rebranding (codex → agcodex)
- **Day 3**: Implement Plan/Build/Review modes with Shift+Tab
- **Day 4**: Add all 50+ tree-sitter languages
- **Day 5**: Internal agent tools implementation

### Week 2: Intelligence Layer
- **Day 1-2**: AST-RAG indexing pipeline
- **Day 3**: Location-aware embeddings with metadata
- **Day 4**: AST-based edit tools
- **Day 5**: Session persistence with Zstd

### Week 3: Polish & Testing
- **Day 1**: Subagent system implementation
- **Day 2**: TUI enhancements and visual indicators
- **Day 3**: Complete anyhow → thiserror migration
- **Day 4-5**: Testing, benchmarks, optimization

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
- **CodexWidget trait**: render, handle_input, handle_mouse, get_help_text
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

## TUI Interface Implementation

### Key Features
- **Session Management**: Save/load (Ctrl+S/O), undo/redo (Ctrl+Z/Y), jump to message (Ctrl+J)
- **Agent Panel**: Spawn agents (Ctrl+Shift+A), progress bars, worktree management
- **History Browser**: Visual timeline with branches, context preview, mouse support
- **Notifications**: Terminal bell, visual bell, status bar notifications
- **Enhanced Layout**: Toggleable panels, status bar, quick actions menu (Ctrl+Space)

### Configuration Overview (~/.agcodex/config.toml)

**Key Settings**:
- `reasoning_effort = "high"` and `verbosity = "high"` (ALWAYS)
- Intelligence modes: light/medium/hard for AST processing
- Embeddings: Disabled by default, optional multi-provider support
- Session auto-save with Zstd compression
- Mode switching via Shift+Tab (Plan/Build/Review)
- Internal agent tools enabled (ast_search, ast_transform, ast_analyze)
- TUI with enhanced layout, notifications, and customizable keybindings

## TUI-First Architecture Principles

1. **All features accessible via TUI** - No feature should require dropping to CLI
2. **Keyboard-first, mouse-optional** - Everything reachable via keybindings
3. **Progressive disclosure** - Advanced features in panels/modals, not cluttering main view
4. **Visual feedback** - Progress bars for agents, status indicators for sessions
5. **Context preservation** - Navigation never loses user context
6. **Non-blocking operations** - Long operations run in background with progress indication
7. **Responsive design** - Adapts to terminal size, degrades gracefully
8. **Accessibility** - Visual bell option, high contrast themes, screen reader hints


## Summary: AGCodex Transformation Goals

This overhaul transforms Codex into **AGCodex** - a powerful, independent AI coding assistant with:

### Core Transformation Tasks
1. **Complete Rebranding**: codex → agcodex across all crates and binaries
2. **Operating Modes**: Plan/Build/Review with Shift+Tab switching
3. **Tree-sitter Integration**: 50+ languages with AST-RAG and AI Distiller compaction
4. **Session Persistence**: ~/.agcodex/history with efficient Zstd compression
5. **Internal Agent Tools**: AST-based code analysis and transformation
6. **High Defaults**: reasoning_effort=high, verbosity=high for GPT-5
7. **Location Awareness**: Precise file:line:column in all embeddings
8. **Error Handling**: Complete migration from `anyhow` to `thiserror`
9. **Type Safety**: Newtype, builder, and typestate patterns

### TUI-Exclusive Features (No CLI Commands Needed)
1. **Session Management**
   - Save/load sessions via `Ctrl+S` / `Ctrl+O`
   - Auto-checkpointing every 5 minutes
   - Visual checkpoint indicators in status bar

2. **Message Navigation & History**
   - **Jump to any previous message** with `Ctrl+J` (restores full context)
   - Undo/redo turns with `Ctrl+Z` / `Ctrl+Y`
   - Visual history browser with `Ctrl+H`
   - Timeline navigation with `Alt+↑` / `Alt+↓`
   - Branch conversations from any point with `Ctrl+B`

3. **Multi-Agent Orchestration**
   - Spawn agents via `Ctrl+Shift+A` (opens spawn dialog)
   - Agent panel with `Ctrl+A` showing progress bars
   - Git worktree agents for parallel development
   - Visual merge interface for combining agent work

4. **Notifications**
   - Terminal bell on task completion (`\x07`)
   - Visual bell for accessibility
   - In-TUI notification bar (bottom-right)
   - Desktop notifications via system integration

5. **Context Management**
   - Smart context retrieval with AST compaction
   - Real-time embeddings for similar code
   - Context preview in jump/history modes

### Key TUI Implementation Files
- `tui/src/app.rs` - Main application loop
- `tui/src/session_ui.rs` - Session management UI
- `tui/src/agent_ui.rs` - Agent orchestration UI
- `tui/src/history_browser.rs` - History navigation UI
- `tui/src/widgets/message_jump.rs` - Jump to message widget
- `tui/src/notifications.rs` - Notification system

## Risk Mitigation

1. **Backward Compatibility**: Maintain API compatibility during migration
2. **Feature Flags**: Gradual rollout with `--experimental` flags
3. **Incremental Migration**: Phase-by-phase implementation
4. **Performance Monitoring**: Continuous benchmarking against targets
5. **Rollback Strategy**: Git tags at each milestone
6. **Testing Coverage**: Minimum 80% for all new code

## Key Innovations

1. **Simple Three-Mode System**: Plan/Build/Review with Shift+Tab
2. **50+ Language Support**: Comprehensive tree-sitter out of the box
3. **AST-RAG Architecture**: Hierarchical retrieval with 90%+ compression
4. **Location-Aware Everything**: Precise file:line:column in all operations
5. **Internal Agent Tools**: AST-powered analysis and transformation
6. **Efficient Persistence**: Zstd compression with lazy loading
7. **Subagent System**: `@agent-name` invocation with isolated contexts
8. **GPT-5 Optimized**: XML-structured prompts, high defaults

### AGCodex Design Philosophy
- **Simple modes, powerful features**: Plan/Build/Review cover all use cases
- **TUI is primary**: All features accessible through TUI with Shift+Tab mode switching
- **Language-universal**: 50+ languages supported out of the box
- **Precision over guessing**: Exact location metadata for all operations
- **Fast over perfect**: 90%+ compression, caching, approximation when sensible
- **Visual feedback**: Mode indicators, progress bars, status colors
- **GPT-5 optimized**: Structured prompts, high reasoning/verbosity defaults
- **Independent project**: No migration from Codex, fresh ~/.agcodex structure
- **Context-aware**: Never lose user's place when navigating