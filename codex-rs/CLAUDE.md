# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is the Rust implementation of Codex - a TUI-first coding agent from OpenAI that runs locally. The project prioritizes the Terminal User Interface (TUI) as the primary interaction method, with all features accessible through keyboard shortcuts and visual panels. The project is structured as a Cargo workspace with the TUI crate as the main user-facing component.

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
cargo run --bin codex

# Launch TUI with initial prompt
cargo run --bin codex -- "explain this codebase to me"

# TUI with specific model preference (can be changed in TUI)
cargo run --bin codex -- --model o3

# Secondary modes (not primary workflow):
cargo run --bin codex exec -- "your task here"  # Headless mode
cargo run --bin codex mcp                        # MCP server mode
```

### TUI Controls Once Launched
- **`/`** - Command palette (search for any action)
- **`Ctrl+N`** - New conversation
- **`Ctrl+S`** - Session management
- **`Ctrl+A`** - Agent panel
- **`Ctrl+H`** - History browser
- **`Ctrl+J`** - Jump to message
- **`Ctrl+?`** - Show all keybindings
- **`Esc`** - Close panel/cancel operation
- **`Tab`** - Cycle through UI elements

## Architecture and Code Organization

### Workspace Structure
The codebase is organized as a Cargo workspace with the following key crates:

- **`tui/`**: PRIMARY INTERFACE - Terminal UI implementation using Ratatui (first-party)
- **`core/`**: Business logic and main functionality. The heart of Codex operations.
- **`cli/`**: Command-line interface entry point (mainly launches TUI)
- **`exec/`**: Headless/non-interactive execution mode (secondary)
- **`mcp-client/`**, **`mcp-server/`**, **`mcp-types/`**: Model Context Protocol implementation
- **`file-search/`**: File discovery and fuzzy search functionality
- **`apply-patch/`**: Code modification and patching functionality
- **`protocol/`**: Communication protocol definitions
- **`execpolicy/`**: Sandboxing and execution policy enforcement
- **`linux-sandbox/`**: Linux-specific sandboxing using Landlock/seccomp
- **`chatgpt/`**: ChatGPT-specific authentication and session management
- **`ollama/`**: Ollama integration for local LLM support

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
- `config.rs`: Main configuration loading and management
- `config_types.rs`: Type definitions for configuration
- `config_profile.rs`: Profile-based configuration support
- TOML-based configuration with model provider definitions

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

## Critical Refactoring Requirements

### 1. Complete Migration from anyhow to thiserror
**MANDATORY**: Replace all uses of `anyhow` with idiomatic `thiserror` patterns throughout the codebase.

**Implementation Strategy**:
```rust
// Replace anyhow::Result with domain-specific error types
use thiserror::Error;

// Define granular error types for each domain
#[derive(Error, Debug)]
pub enum FileSearchError {
    #[error("pattern not found: {pattern}")]
    PatternNotFound { pattern: String },
    
    #[error("AST parsing failed for {file}")]
    AstParseError { 
        file: PathBuf,
        #[source] source: tree_sitter::Error 
    },
    
    #[error("invalid search query: {reason}")]
    InvalidQuery { reason: String },
}

// Use specific error types in function signatures
fn search_ast(query: &str) -> Result<Vec<Match>, FileSearchError> {
    // Implementation
}
```

**Migration Steps**:
1. Create domain-specific error types in each crate
2. Replace `anyhow::Result` with specific `Result<T, DomainError>`
3. Use `#[from]` for automatic error conversion
4. Add contextual information to error variants
5. Remove anyhow dependency from Cargo.toml files

### 2. Enhanced AST-Based Code Intelligence

**Current State**: Basic fuzzy file search using `nucleo-matcher` without semantic understanding.

**Required Enhancements**:

#### A. Tree-sitter Integration
```toml
# Add to file-search/Cargo.toml
[dependencies]
tree-sitter = "0.24"
tree-sitter-rust = "0.23"
tree-sitter-python = "0.23"
tree-sitter-javascript = "0.23"
tree-sitter-typescript = "0.23"
tree-sitter-go = "0.23"
tree-sitter-cpp = "0.23"
tree-sitter-java = "0.23"
```

#### B. AST-grep Integration
```toml
# Add to core/Cargo.toml
[dependencies]
ast-grep-core = "0.29"
ast-grep-language = "0.29"
```

#### C. Smart Context Retrieval Architecture
```rust
// New module structure: core/src/context_engine/
pub mod context_engine {
    pub mod ast_compactor;     // Compress code to signatures/structure
    pub mod semantic_index;    // Build semantic index of codebase
    pub mod retrieval;        // Query-based context retrieval
    pub mod embeddings;       // AST-based embeddings for similarity
    pub mod cache;           // Cache parsed ASTs and indexes
}

// AST Compactor example
pub struct AstCompactor {
    // Extracts essential structure: functions, classes, types
    // Removes implementation details for context efficiency
}

// Semantic Index example  
pub struct SemanticIndex {
    // Indexes symbols, dependencies, call graphs
    // Supports fast lookup and traversal
}
```

### 3. Robust Type System Enhancements

**Required Patterns**:

#### A. Newtype Pattern for Domain Types
```rust
// Strong typing for domain concepts
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FilePath(PathBuf);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineNumber(u32);

#[derive(Debug, Clone)]
pub struct AstNodeId(usize);

// Implement necessary traits
impl From<PathBuf> for FilePath {
    fn from(path: PathBuf) -> Self {
        FilePath(path)
    }
}
```

#### B. Builder Pattern for Complex Types
```rust
pub struct QueryBuilder {
    language: Option<Language>,
    pattern: Option<String>,
    scope: SearchScope,
    max_results: Option<usize>,
}

impl QueryBuilder {
    pub fn new() -> Self { /* ... */ }
    pub fn language(mut self, lang: Language) -> Self { /* ... */ }
    pub fn pattern(mut self, pattern: impl Into<String>) -> Self { /* ... */ }
    pub fn build(self) -> Result<Query, QueryBuildError> { /* ... */ }
}
```

#### C. Typestate Pattern for State Machines
```rust
// Compile-time state machine guarantees
pub struct Connection<S> {
    inner: TcpStream,
    _state: PhantomData<S>,
}

pub struct Disconnected;
pub struct Connected;
pub struct Authenticated;

impl Connection<Disconnected> {
    pub fn connect(self) -> Result<Connection<Connected>, Error> { /* ... */ }
}

impl Connection<Connected> {
    pub fn authenticate(self) -> Result<Connection<Authenticated>, Error> { /* ... */ }
}
```

### 4. Tool Integration Module

**Required Structure**:
```rust
// core/src/code_tools/
pub mod code_tools {
    pub mod ripgrep;      // rg wrapper for fast text search
    pub mod fd_find;      // fd wrapper for file discovery  
    pub mod ast_grep;     // ast-grep wrapper for structural search
    pub mod tree_sitter;  // Enhanced tree-sitter operations
    pub mod comby;        // Structural code transformations
    
    // Unified interface
    pub trait CodeTool {
        type Query;
        type Result;
        fn search(&self, query: Self::Query) -> Result<Self::Result, ToolError>;
    }
}
```

### 5. Session Management and Persistence

**Required Features**:

#### A. Save/Load Sessions
```rust
// core/src/session/
pub mod session {
    pub mod persistence;  // Save/load conversation state
    pub mod checkpoint;   // Automatic checkpointing
    pub mod recovery;     // Crash recovery from checkpoints
    
    pub struct SessionManager {
        autosave_interval: Duration,
        checkpoint_dir: PathBuf,
        max_checkpoints: usize,
    }
    
    impl SessionManager {
        pub fn save_session(&self, path: &Path) -> Result<(), SessionError>;
        pub fn load_session(&self, path: &Path) -> Result<Session, SessionError>;
        pub fn create_checkpoint(&self) -> Result<CheckpointId, SessionError>;
        pub fn restore_checkpoint(&self, id: CheckpointId) -> Result<(), SessionError>;
    }
}
```

#### B. Undo/Redo Support
```rust
// Conversation history with undo stack
pub struct ConversationHistory {
    turns: Vec<Turn>,
    undo_stack: Vec<ConversationState>,
    redo_stack: Vec<ConversationState>,
    context_snapshots: HashMap<TurnId, ContextSnapshot>,
}

impl ConversationHistory {
    pub fn undo_last_turn(&mut self) -> Result<(), HistoryError>;
    pub fn redo_turn(&mut self) -> Result<(), HistoryError>;
    pub fn checkpoint_state(&mut self);
    pub fn jump_to_turn(&mut self, turn_id: TurnId) -> Result<(), HistoryError>;
    pub fn get_context_at_turn(&self, turn_id: TurnId) -> Result<ContextSnapshot, HistoryError>;
}
```

#### C. Message Navigation and Context Restoration
```rust
// Jump to any previous message with full context restoration
pub struct MessageNavigator {
    history: ConversationHistory,
    context_manager: ContextManager,
}

impl MessageNavigator {
    // Jump to a specific message and restore its context
    pub fn jump_to_message(&mut self, message_id: MessageId) -> Result<(), NavError>;
    
    // Get the context that existed before a specific message
    pub fn get_pre_message_context(&self, message_id: MessageId) -> Result<Context, NavError>;
    
    // Create a new conversation branch from a previous point
    pub fn branch_from_message(&mut self, message_id: MessageId) -> Result<BranchId, NavError>;
    
    // Navigate through conversation history
    pub fn prev_message(&mut self) -> Result<MessageId, NavError>;
    pub fn next_message(&mut self) -> Result<MessageId, NavError>;
    pub fn go_to_turn(&mut self, turn: usize) -> Result<(), NavError>;
}

// Context snapshot for restoring state
#[derive(Clone, Serialize, Deserialize)]
pub struct ContextSnapshot {
    messages: Vec<Message>,
    system_prompt: String,
    tool_states: HashMap<ToolId, ToolState>,
    file_states: HashMap<PathBuf, FileSnapshot>,
    variables: HashMap<String, Value>,
    timestamp: SystemTime,
}
```

### 6. Multi-Agent Architecture

**Required Components**:

#### A. Agent Spawning and Coordination
```rust
// core/src/agents/
pub mod agents {
    pub mod orchestrator;    // Main agent coordinator
    pub mod sub_agent;      // Sub-agent implementation
    pub mod git_worktree;   // Git worktree-aware agents
    pub mod communication;  // Inter-agent messaging
    
    pub struct AgentOrchestrator {
        agents: HashMap<AgentId, Agent>,
        worktrees: HashMap<WorktreeId, GitWorktree>,
        message_bus: MessageBus,
    }
    
    impl AgentOrchestrator {
        pub async fn spawn_agent(&mut self, config: AgentConfig) -> Result<AgentId, AgentError>;
        pub async fn spawn_worktree_agent(&mut self, worktree: &Path) -> Result<AgentId, AgentError>;
        pub async fn coordinate_agents(&mut self, task: Task) -> Result<TaskResult, AgentError>;
    }
    
    // Sub-agent with specialized capabilities
    pub struct SubAgent {
        id: AgentId,
        capabilities: Vec<Capability>,
        worktree: Option<GitWorktree>,
    }
}
```

#### B. Git Worktree Integration
```rust
// Git worktree management for multi-agent operations
pub struct GitWorktreeManager {
    main_worktree: PathBuf,
    agent_worktrees: HashMap<AgentId, PathBuf>,
}

impl GitWorktreeManager {
    pub fn create_agent_worktree(&mut self, agent_id: AgentId) -> Result<PathBuf, GitError>;
    pub fn sync_worktrees(&self) -> Result<(), GitError>;
    pub fn merge_agent_changes(&self, agent_id: AgentId) -> Result<(), GitError>;
}
```

### 7. Notification System

**Terminal Bell and Desktop Notifications**:
```rust
// core/src/notifications/
pub mod notifications {
    pub mod terminal_bell;  // Terminal bell notifications
    pub mod desktop;       // Desktop notifications
    pub mod hooks;         // Custom notification hooks
    
    pub enum NotificationLevel {
        Info,
        Warning,
        Error,
        TaskComplete,
    }
    
    pub struct NotificationManager {
        terminal_bell_enabled: bool,
        desktop_enabled: bool,
        custom_hooks: Vec<NotificationHook>,
    }
    
    impl NotificationManager {
        pub fn notify(&self, level: NotificationLevel, message: &str);
        pub fn ring_terminal_bell(&self);
        pub fn send_desktop_notification(&self, title: &str, body: &str);
    }
}
```

## Development Guidelines

### Error Handling Best Practices
```rust
// ALWAYS use thiserror, NEVER use anyhow
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ModuleError {
    #[error("operation failed: {operation}")]
    OperationFailed { 
        operation: String,
        #[source] cause: Box<dyn std::error::Error + Send + Sync>,
    },
    
    #[error("invalid state transition from {from} to {to}")]
    InvalidTransition { from: String, to: String },
    
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

// Use Result aliases for clarity
pub type Result<T> = std::result::Result<T, ModuleError>;
```

### Testing Requirements
- Unit tests must be colocated with implementation
- Integration tests in `tests/` directory
- Use property-based testing for complex logic
- Mock external services with `wiremock` or `mockito`
- Minimum test coverage: 80% for new code

### Performance Considerations
- Use `Arc<str>` instead of `String` for shared immutable strings
- Prefer `SmallVec` for small collections
- Use `OnceCell`/`LazyLock` for lazy initialization
- Profile with `cargo flamegraph` before optimization
- Benchmark critical paths with `criterion`

## Platform-Specific Considerations

### macOS
- Sandbox: Apple Seatbelt (`sandbox-exec`)
- Test: `codex debug seatbelt [COMMAND]`
- Keychain integration for credential storage

### Linux
- Sandbox: Landlock (kernel 5.13+) and seccomp
- Test: `codex debug landlock [COMMAND]`
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

### Widget Development
```rust
// tui/src/widgets/
pub trait CodexWidget {
    fn render(&mut self, area: Rect, buf: &mut Buffer);
    fn handle_input(&mut self, key: KeyEvent) -> InputResult;
    fn handle_mouse(&mut self, mouse: MouseEvent) -> InputResult;
    fn get_help_text(&self) -> &str;
}

// Example: Message jump widget
pub struct MessageJumpWidget {
    search_input: String,
    filtered_messages: Vec<MessageInfo>,
    selected_index: usize,
    preview: Option<ContextSnapshot>,
}
```

### State Management in TUI
```rust
// tui/src/state.rs
pub struct AppState {
    mode: AppMode,
    conversation: ConversationState,
    agents: AgentOrchestratorState,
    session: SessionState,
    ui_state: UIState,
}

pub enum AppMode {
    Normal,           // Main chat interface
    SessionManager,   // Ctrl+S
    HistoryBrowser,  // Ctrl+H
    AgentPanel,      // Ctrl+A
    MessageJump,     // Ctrl+J
    CommandPalette,  // /
}
```

### Event Handling Architecture
```rust
// tui/src/events.rs
pub enum AppEvent {
    // User input
    KeyPress(KeyEvent),
    MouseEvent(MouseEvent),
    Resize(u16, u16),
    
    // Agent events
    AgentSpawned(AgentId),
    AgentProgress(AgentId, Progress),
    AgentCompleted(AgentId, Result<Output, Error>),
    
    // Session events
    CheckpointCreated(CheckpointId),
    SessionSaved(PathBuf),
    MessageJump(MessageId),
    
    // Notifications
    Notification(NotificationLevel, String),
}
```

## Common Development Workflows

### Adding TUI Feature
1. Create widget in `tui/src/widgets/`
2. Add state to `AppState` in `tui/src/state.rs`
3. Add keybinding in `tui/src/input.rs`
4. Handle events in `tui/src/events.rs`
5. Update help text in `tui/src/help.rs`
6. Test with mock data in `tui/tests/`

### Adding AST-based Feature
1. Add tree-sitter grammar to Cargo.toml
2. Create parser module in `core/src/ast/parsers/`
3. Implement `AstParser` trait
4. Add caching layer with `DashMap`
5. Write property-based tests
6. Benchmark performance impact

### Implementing New Tool Integration
1. Create module in `core/src/code_tools/`
2. Define tool-specific error types with thiserror
3. Implement `CodeTool` trait
4. Add configuration in `config_types.rs`
5. Write integration tests with mocked responses
6. Document tool capabilities and limitations

### Adding New Error Type
1. Define error enum with thiserror in module
2. Implement conversions with `#[from]`
3. Add error recovery logic at call sites
4. Update documentation with error conditions
5. Add tests for error paths

## Performance Profiling

```bash
# CPU profiling
cargo install flamegraph
cargo flamegraph --bin codex -- exec "test command"

# Memory profiling  
cargo install cargo-bloat
cargo bloat --release --bin codex

# Benchmark specific functions
cargo bench --bench context_engine
```

## MCP (Model Context Protocol) Notes

- Client config: `~/.codex/config.toml` under `[mcp_servers]`
- Server mode: `codex mcp`
- Debug with: `npx @modelcontextprotocol/inspector codex mcp`
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

## TUI Interface Implementation (First-Party)

### TUI Session Management
```rust
// tui/src/session_ui.rs
pub struct SessionUI {
    mode: SessionMode,
    checkpoint_indicator: CheckpointStatus,
    history_panel: HistoryPanel,
}

pub enum SessionMode {
    Normal,
    HistoryBrowse,    // Activated by Ctrl+H
    MessageJump,      // Activated by Ctrl+J
    SessionManager,   // Activated by Ctrl+S
}

// Key bindings for session management
pub const SESSION_KEYS: &[KeyBinding] = &[
    KeyBinding { key: "Ctrl+S", action: Action::OpenSessionManager },
    KeyBinding { key: "Ctrl+Shift+S", action: Action::QuickSave },
    KeyBinding { key: "Ctrl+O", action: Action::LoadSession },
    KeyBinding { key: "Ctrl+Z", action: Action::UndoTurn },
    KeyBinding { key: "Ctrl+Y", action: Action::RedoTurn },
    KeyBinding { key: "Ctrl+J", action: Action::JumpToMessage },
    KeyBinding { key: "Alt+↑", action: Action::PrevMessage },
    KeyBinding { key: "Alt+↓", action: Action::NextMessage },
    KeyBinding { key: "Ctrl+B", action: Action::BranchFromHere },
    KeyBinding { key: "F5", action: Action::CreateCheckpoint },
    KeyBinding { key: "F6", action: Action::RestoreCheckpoint },
];
```

### TUI Multi-Agent Interface
```rust
// tui/src/agent_ui.rs
pub struct AgentPanel {
    agents: Vec<AgentView>,
    orchestrator_view: OrchestratorView,
    worktree_panel: WorktreePanel,
}

// Agent management through TUI
impl AgentPanel {
    pub fn render_agent_list(&self, area: Rect, buf: &mut Buffer) {
        // Show active agents with status indicators
        // • Agent 1 [Refactoring] ████░░░░ 60%
        // • Agent 2 [Testing] ████████ 100% ✓
        // • Agent 3 [Reviewing] ██░░░░░░ 25%
    }
    
    pub fn render_spawn_dialog(&self) -> SpawnAgentDialog {
        // Modal dialog for spawning new agents
        // Shows: capabilities selector, worktree options, task assignment
    }
}

// Key bindings for agent management
pub const AGENT_KEYS: &[KeyBinding] = &[
    KeyBinding { key: "Ctrl+A", action: Action::OpenAgentPanel },
    KeyBinding { key: "Ctrl+Shift+A", action: Action::SpawnAgent },
    KeyBinding { key: "Ctrl+W", action: Action::CreateWorktreeAgent },
    KeyBinding { key: "Tab", action: Action::SwitchAgent },
    KeyBinding { key: "Ctrl+M", action: Action::MergeAgentWork },
];
```

### TUI Navigation & History Browser
```rust
// tui/src/history_browser.rs
pub struct HistoryBrowser {
    timeline_view: TimelineView,
    context_preview: ContextPreview,
    branch_visualization: BranchTree,
}

impl HistoryBrowser {
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Visual timeline with clickable messages
        // ──●──●──●──●──●──●── [Current]
        //      └──●──●── [Branch: feature-x]
        
        // Preview pane shows context at selected point
        // Jump button to restore that context
    }
    
    pub fn handle_mouse(&mut self, event: MouseEvent) {
        // Click on timeline point to preview
        // Double-click to jump to that message
    }
}
```

### TUI Notification Integration
```rust
// tui/src/notifications.rs
pub struct NotificationBar {
    notifications: VecDeque<Notification>,
    bell_enabled: bool,
    flash_enabled: bool,
}

impl NotificationBar {
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        // Status bar with notifications
        // [🔔] Task completed | Agent 2 finished refactoring
    }
    
    pub fn trigger_bell(&self) {
        // Terminal bell: \x07
        if self.bell_enabled {
            print!("\x07");
        }
    }
    
    pub fn flash_screen(&self) {
        // Visual bell for accessibility
    }
}
```

### TUI Layout with Advanced Features
```rust
// tui/src/layout.rs
pub struct EnhancedLayout {
    main_chat: ChatArea,
    agent_sidebar: Option<AgentPanel>,      // Toggle with Ctrl+A
    history_panel: Option<HistoryPanel>,    // Toggle with Ctrl+H
    session_status: SessionStatusBar,       // Always visible
    notification_bar: NotificationBar,      // Always visible
}

// Status bar shows:
// [Session: project-x] [●Recording] [Checkpoint: 5m ago] [Agents: 3 active] [Ctrl+? for help]
```

### TUI Quick Actions Menu
```rust
// Activated by Ctrl+Space or F1
pub struct QuickActionsMenu {
    actions: Vec<QuickAction>,
}

impl QuickActionsMenu {
    pub fn default_actions() -> Vec<QuickAction> {
        vec![
            QuickAction::JumpToMessage,
            QuickAction::SpawnAgent,
            QuickAction::SaveSession,
            QuickAction::CreateCheckpoint,
            QuickAction::UndoLastTurn,
            QuickAction::BranchConversation,
            QuickAction::MergeAgentWork,
        ]
    }
}
```

### TUI Configuration
```toml
# ~/.codex/config.toml
[tui]
default_layout = "enhanced"  # "classic" or "enhanced"
auto_checkpoint_interval = "5m"
show_agent_panel = false  # Start with agent panel hidden
show_history_browser = false
enable_mouse = true
theme = "dark"

[tui.notifications]
terminal_bell = true
visual_bell = false  # For accessibility
desktop_notify = true
notification_position = "bottom-right"

[tui.keybindings]
# Custom keybinding overrides
undo = "Ctrl+Z"
redo = "Ctrl+Shift+Z"
jump_to_message = "Ctrl+J"
agent_panel = "F3"
history_browser = "F4"

[tui.layout]
sidebar_width = 30
history_panel_height = 10
agent_list_position = "right"  # "left" or "right"
```

## TUI-First Architecture Principles

1. **All features accessible via TUI** - No feature should require dropping to CLI
2. **Keyboard-first, mouse-optional** - Everything reachable via keybindings
3. **Progressive disclosure** - Advanced features in panels/modals, not cluttering main view
4. **Visual feedback** - Progress bars for agents, status indicators for sessions
5. **Context preservation** - Navigation never loses user context
6. **Non-blocking operations** - Long operations run in background with progress indication
7. **Responsive design** - Adapts to terminal size, degrades gracefully
8. **Accessibility** - Visual bell option, high contrast themes, screen reader hints

## TUI Testing Strategy

```rust
// tui/tests/integration/
use codex_tui::test_utils::*;

#[tokio::test]
async fn test_message_jump() {
    let mut app = TestApp::new();
    app.load_fixture("conversation_with_history.json");
    
    // Simulate Ctrl+J
    app.send_key(KeyCode::Char('j'), KeyModifiers::CONTROL);
    assert_eq!(app.mode(), AppMode::MessageJump);
    
    // Type to search
    app.type_text("refactor");
    assert!(app.filtered_messages().len() > 0);
    
    // Select and jump
    app.send_key(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(app.current_message_id(), MessageId::from("msg_123"));
}
```

## Summary: TUI-First Refactoring Goals

This refactoring transforms Codex into a **TUI-first application** where all interactions happen through the terminal interface:

### Core Refactoring Tasks
1. **Error Handling**: Complete migration from `anyhow` to `thiserror` with domain-specific error types
2. **AST Intelligence**: Tree-sitter and ast-grep integration for semantic code understanding
3. **Type Safety**: Newtype, builder, and typestate patterns for compile-time guarantees

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

### Design Philosophy
- **TUI is primary**: All features must be accessible through TUI
- **No CLI fallback needed**: TUI provides complete functionality
- **Keyboard-driven**: Every action has a keybinding
- **Visual feedback**: Progress bars, status indicators, previews
- **Non-blocking**: Background operations with progress indication
- **Context-aware**: Never lose user's place when navigating