# Codex-RS Domain Knowledge

## Project Overview

Codex-RS is a TUI-first coding agent from OpenAI that runs locally. The project prioritizes the Terminal User Interface (TUI) as the primary interaction method, with all features accessible through keyboard shortcuts and visual panels. It's structured as a Cargo workspace with the TUI crate as the main user-facing component.

## Workspace Architecture

The project consists of **19 specialized crates** organized as a Cargo workspace:

### Core Components
- **`core/`** - Business logic and main functionality (heart of Codex operations)
- **`tui/`** - PRIMARY INTERFACE: Terminal UI implementation using Ratatui (first-party)
- **`cli/`** - Command-line interface entry point (mainly launches TUI)
- **`exec/`** - Headless/non-interactive execution mode (secondary)

### Communication & Protocol
- **`protocol/`** - Communication protocol definitions
- **`protocol-ts/`** - TypeScript protocol bindings
- **`mcp-client/`** - MCP client implementation
- **`mcp-server/`** - MCP server implementation
- **`mcp-types/`** - MCP type definitions

### Security & Sandboxing
- **`execpolicy/`** - Execution policy enforcement
- **`linux-sandbox/`** - Linux-specific sandboxing (Landlock/seccomp)

### Utilities
- **`file-search/`** - File discovery and fuzzy search
- **`apply-patch/`** - Code modification and patching
- **`ansi-escape/`** - ANSI escape sequence handling
- **`common/`** - Shared utilities
- **`login/`** - Authentication management
- **`chatgpt/`** - ChatGPT-specific authentication
- **`ollama/`** - Ollama LLM integration
- **`arg0/`** - Argument handling utilities

## Core Architectural Patterns

### TUI-First Design
The application prioritizes the Terminal UI as the primary interface, with all features accessible through keyboard shortcuts and visual panels.

### Actor Model Architecture
- **`ConversationManager`**: Manages multiple conversation instances
- **`CodexConversation`**: Individual conversation actors
- **`Codex`**: Core agent implementation

### Event-Driven Architecture
- Event streaming between components using `tokio` channels
- Protocol-based communication using typed events (`Event`, `EventMsg`)
- Async/await patterns throughout

## Key Components Analysis

### 1. TUI Implementation (tui/)

**Current State:**
```rust
enum AppState<'a> {
    Onboarding { screen: OnboardingScreen },
    Chat { widget: Box<ChatWidget<'a>> }
}
```

**Key Components:**
- `ChatWidget`: Main conversation interface
- `FileSearchManager`: Integrated file search
- `OnboardingScreen`: First-run experience and authentication
- Custom widgets for Codex-specific functionality

**Event Loop Architecture:**
- Dedicated thread for crossterm event polling
- Event aggregation and debouncing (1ms window)
- Frame scheduling for animations
- Non-blocking UI updates

### 2. Client Architecture (core/src/client*.rs)

**Dual API Support:**
```rust
pub enum WireApi {
    Responses,  // OpenAI Responses API (experimental)
    Chat        // Standard Chat Completions API
}
```

**Features:**
- Streaming response handling with aggregation
- Multiple provider support through `ModelProviderInfo`
- Authentication via API keys or ChatGPT login
- Automatic retry with exponential backoff
- Token usage tracking

### 3. Conversation Management (core/src/conversation_*.rs)

**Architecture:**
```rust
pub struct ConversationManager {
    conversations: Arc<RwLock<HashMap<Uuid, Arc<CodexConversation>>>>
}
```

**Features:**
- UUID-based conversation tracking
- Turn diff tracking for code changes
- History preservation (`ConversationHistory` exists)
- Session configuration events
- Concurrent conversation support

### 4. Execution Environment & Sandboxing

**Multi-Platform Support:**
- **macOS**: Apple Seatbelt (`sandbox-exec`)
- **Linux**: Landlock (kernel 5.13+) + seccomp
- **Windows**: Limited support with SmartScreen

**Safety Layers:**
1. **Approval Policy**: `AskForApproval` enum controls command execution
2. **Sandbox Policy**: Restricts filesystem and network access
3. **Shell Environment Policy**: Controls environment variable exposure
4. **Execution Policy**: Rule-based command filtering

### 5. MCP (Model Context Protocol) Integration

**Three-Crate Architecture:**
- `mcp-types`: Shared type definitions
- `mcp-client`: Client for connecting to MCP servers
- `mcp-server`: Server mode for Codex as MCP provider

**Features:**
- Tool discovery and invocation
- Bidirectional communication
- Message processor pattern
- Approval workflows for tool execution

## Current State Analysis

### Error Handling
- **anyhow usage**: 21 occurrences across crates
  - protocol-ts: 4 uses
  - mcp-client: 4 uses
  - apply-patch: 1 use
  - Multiple other crates
- **thiserror usage**: 4 occurrences (partial migration started)
  - login/src/token_data.rs
  - apply-patch/src/lib.rs
  - apply-patch/src/parser.rs
  - core/src/error.rs

### AST/Code Intelligence
- **Current Implementation:**
  - Basic tree-sitter in apply-patch and core/src/bash.rs
  - `nucleo-matcher` for fuzzy file search
  - No AST-based semantic search
  - No ast-grep integration
  - No code compaction/distillation

### Type System Patterns
- **Current State:**
  - Minimal newtype patterns (only 1 found: `DirGuard`)
  - No builder patterns detected
  - No typestate patterns
  - Mostly plain structs without strong typing

### Session Management
- **Existing Components:**
  - `ConversationHistory` struct exists
  - Basic turn tracking
- **Missing Components:**
  - No `SessionManager` implementation
  - No checkpoint/recovery system
  - No undo/redo support
  - No message jump functionality
  - No branch management

### Multi-Agent Architecture
- **Current State:**
  - Basic `spawn_agent` function in tui/src/chatwidget/agent.rs
- **Missing Components:**
  - No `AgentOrchestrator`
  - No git worktree management
  - No inter-agent communication bus
  - No agent coordination framework

### TUI Features Status
- **Implemented:**
  - Basic ChatWidget
  - Onboarding flow
  - File search integration
- **Not Implemented:**
  - Session management UI (Ctrl+S)
  - History browser (Ctrl+H)
  - Message jump (Ctrl+J)
  - Agent panel (Ctrl+A)
  - Undo/redo (Ctrl+Z/Y)
  - Branch management (Ctrl+B)
  - Checkpoint indicators
  - Notification system

## Refactoring Requirements (from CLAUDE.md)

### 1. Complete Migration from anyhow to thiserror
**Priority: HIGH**
- Replace all 21 uses of `anyhow` with domain-specific error types
- Define granular error types for each domain
- Use `#[from]` for automatic error conversion
- Add contextual information to error variants

### 2. Enhanced AST-Based Code Intelligence
**Priority: HIGH**
- Add tree-sitter grammars for multiple languages
- Integrate ast-grep-core for structural search
- Implement smart context retrieval architecture:
  - AST compactor for code signatures
  - Semantic index for symbols/dependencies
  - Retrieval system for query-based context
  - Embeddings for similarity search
  - Cache layer for parsed ASTs

### 3. Robust Type System Enhancements
**Priority: MEDIUM**
- Implement newtype pattern for domain types
- Add builder pattern for complex types
- Use typestate pattern for state machines
- Create strongly-typed wrappers for paths, IDs, etc.

### 4. Tool Integration Module
**Priority: MEDIUM**
- Create unified interface for code tools:
  - ripgrep wrapper
  - fd wrapper
  - ast-grep wrapper
  - tree-sitter operations
  - comby transformations

### 5. Session Management and Persistence
**Priority: HIGH**
- Implement SessionManager with:
  - Save/load conversation state
  - Automatic checkpointing
  - Crash recovery
  - Undo/redo support
- Add MessageNavigator for:
  - Jump to any message (Ctrl+J)
  - Context restoration
  - Conversation branching
  - Timeline navigation

### 6. Multi-Agent Architecture
**Priority: HIGH**
- Create AgentOrchestrator for:
  - Agent spawning and coordination
  - Git worktree management
  - Inter-agent messaging
  - Task distribution
- Implement SubAgent system:
  - Specialized capabilities
  - Worktree isolation
  - Result merging

### 7. Notification System
**Priority: LOW**
- Terminal bell notifications
- Desktop notifications
- Custom notification hooks
- Visual indicators in TUI

## TUI-First Architecture Requirements

### Key Bindings to Implement
```
Session Management:
- Ctrl+S: Open session manager
- Ctrl+Shift+S: Quick save
- Ctrl+O: Load session
- Ctrl+Z: Undo turn
- Ctrl+Y: Redo turn

Navigation:
- Ctrl+J: Jump to message
- Ctrl+H: History browser
- Alt+↑/↓: Navigate messages
- Ctrl+B: Branch from here

Agents:
- Ctrl+A: Agent panel
- Ctrl+Shift+A: Spawn agent
- Ctrl+W: Create worktree agent
- Tab: Switch agent
- Ctrl+M: Merge agent work

Other:
- F5: Create checkpoint
- F6: Restore checkpoint
- Ctrl+?: Show help
- /: Command palette
```

### TUI Layout Requirements
```
┌─────────────────────────────────────────────────────────┐
│ Main Chat Area                                          │
│                                                         │
│ [Messages with context]                                │
│                                                         │
├─────────────────────────────────────────────────────────┤
│ [Session: project-x] [●Rec] [CP: 5m] [Agents: 3] [?]  │
├─────────────────────────────────────────────────────────┤
│ [🔔] Task completed | Agent 2 finished                 │
└─────────────────────────────────────────────────────────┘

Optional Panels (toggled):
- Right: Agent Panel (Ctrl+A)
- Bottom: History Browser (Ctrl+H)
- Modal: Message Jump (Ctrl+J)
- Modal: Session Manager (Ctrl+S)
```

## Performance Considerations

### Priority Optimization Areas
1. AST parsing and caching (use `DashMap` for concurrent access)
2. Context retrieval (implement incremental indexing)
3. File watching (use `notify` crate efficiently)
4. Streaming response handling (minimize allocations)
5. Agent coordination overhead (use message passing, not shared state)
6. Session checkpointing (async background saves)
7. Worktree operations (batch git operations)

### Release Profile Optimizations
```toml
[profile.release]
lto = "fat"
codegen-units = 1
strip = true
```

## Configuration System

### Hierarchical Configuration
```
~/.agcodex/config.toml
├── Model configuration (provider, API keys)
├── TUI settings (layout, keybindings, themes)
├── MCP server configurations
├── Sandbox and approval policies
├── Shell environment policies
└── Notification settings
```

### Key Configuration Types
- `Config`: Main configuration struct
- `ConfigProfile`: Profile-based settings
- `ModelProviderInfo`: Provider-specific settings
- `McpServerConfig`: MCP server definitions

## Security Requirements

- NEVER log or expose sensitive data (tokens, keys)
- All external inputs must be validated
- Use constant-time comparisons for secrets
- Sandbox all command execution by default
- Require explicit user approval for filesystem writes outside workspace

## Development Guidelines

### Testing Requirements
- Unit tests colocated with implementation
- Integration tests in `tests/` directory
- Property-based testing for complex logic
- Mock external services with `wiremock` or `mockito`
- Minimum test coverage: 80% for new code

### Code Quality Standards
- Complete migration from `anyhow` to `thiserror`
- Strong typing with newtype patterns
- Builder patterns for complex initialization
- Typestate patterns for compile-time guarantees
- Use `Arc<str>` instead of `String` for shared immutable strings
- Prefer `SmallVec` for small collections
- Use `OnceCell`/`LazyLock` for lazy initialization

## Build and Development Commands

### Building
```bash
cargo build --all-features --workspace
cargo check --all-features --all-targets --workspace --tests
```

### Testing
```bash
cargo test --no-fail-fast
cargo test -p codex-core
cargo test -- --nocapture
cargo test --all-features --no-fail-fast
```

### Code Quality
```bash
cargo fmt --all
cargo clippy --all-features --all-targets --workspace --tests --fix --allow-dirty
cargo clippy --all-features --all-targets --workspace --tests -- -D warnings
```

### Running
```bash
# Launch TUI (primary interface)
cargo run --bin codex

# Launch with initial prompt
cargo run --bin codex -- "explain this codebase"

# Secondary modes
cargo run --bin codex exec -- "task"  # Headless
cargo run --bin codex mcp              # MCP server
```

## Summary

Codex-RS is a well-architected TUI-first coding agent with strong foundations but requires significant refactoring to meet the vision outlined in CLAUDE.md. The main gaps are:

1. **Error handling** needs complete migration to thiserror
2. **AST intelligence** needs tree-sitter/ast-grep integration
3. **Session management** needs persistence, checkpointing, and navigation
4. **Multi-agent** capabilities need orchestration framework
5. **TUI features** need many keyboard shortcuts and panels implemented

The architecture supports these enhancements with its modular workspace design, actor model, and event-driven patterns. The refactoring should maintain the TUI-first philosophy while adding the missing advanced features.