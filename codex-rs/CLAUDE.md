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
- **Native tool integration**: fd-find and ripgrep as Rust libraries, not shell commands

## Current Implementation State

### What's Working
- **TUI Foundation**: Basic ChatWidget, onboarding flow, file search integration
- **Conversation Management**: UUID-based tracking, turn history, diff tracking
- **Client Architecture**: Dual API support (Responses/Chat), streaming, multi-provider
- **Sandboxing**: Platform-specific (Seatbelt/Landlock/seccomp) with approval workflows
- **MCP Protocol**: Client/server modes with tool discovery and invocation
- **Basic Agent Support**: Simple spawn_agent function in TUI

### Critical Gaps (Must Implement)
- **Error Handling**: 21 anyhow uses vs 4 thiserror (needs complete migration)
- **AST Intelligence**: Need full tree-sitter (50+ langs), ast-grep, AI Distiller-style compaction
- **Session Management**: No persistence at ~/.agcodex/history, missing smooth switching UX
- **Operating Modes**: No Plan/Build/Review modes, poor profile UX needs replacement
- **Mode Switching**: Missing Shift+Tab for instant mode cycling in TUI
- **Embeddings**: No configurable Light/Medium/Hard intelligence options
- **Location Awareness**: No precise file:line:column metadata in embeddings
- **Native Tools**: fd-find and ripgrep need native integration as internal tools
- **Defaults**: Need HIGH reasoning effort and verbosity as defaults
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
- **`file-search/`**: Enhanced with tree-sitter AST search and native fd-find/ripgrep
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
2. **Implement operating modes**: Plan/Build/Review with Shift+Tab switching
3. **Session persistence**: Create ~/.agcodex/history with Zstd compression
4. **Set HIGH defaults**: reasoning_effort=high, verbosity=high in config
5. **Complete anyhow→thiserror migration** (21 files affected)

### Phase 2: AST Intelligence (HIGH PRIORITY)
1. **Tree-sitter integration**: Add all 50+ language parsers
2. **AST-RAG engine**: Hierarchical retrieval (File→Class→Function)
3. **AI Distiller compaction**: 90%+ code compression
4. **Location-aware embeddings**: Precise file:line:column metadata
5. **Native tool integration**: fd-find and ripgrep as Rust libraries

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
```rust
// agcodex-core/src/modes.rs
pub struct ModeManager {
    current_mode: OperatingMode,
    mode_history: Vec<(OperatingMode, DateTime<Utc>)>,
    restrictions: ModeRestrictions,
}

pub struct ModeRestrictions {
    pub allow_file_write: bool,
    pub allow_command_exec: bool,
    pub allow_network_access: bool,
    pub allow_git_operations: bool,
    pub max_file_size: Option<usize>,
}

impl ModeManager {
    pub fn switch_mode(&mut self, new_mode: OperatingMode) {
        self.mode_history.push((self.current_mode, Utc::now()));
        self.current_mode = new_mode;
        self.restrictions = match new_mode {
            OperatingMode::Plan => ModeRestrictions {
                allow_file_write: false,
                allow_command_exec: false,
                allow_network_access: true,  // For research
                allow_git_operations: false,
                max_file_size: None,
            },
            OperatingMode::Build => ModeRestrictions {
                allow_file_write: true,
                allow_command_exec: true,
                allow_network_access: true,
                allow_git_operations: true,
                max_file_size: None,
            },
            OperatingMode::Review => ModeRestrictions {
                allow_file_write: true,  // Limited
                allow_command_exec: false,
                allow_network_access: true,
                allow_git_operations: false,
                max_file_size: Some(10_000),  // Small edits only
            },
        };
    }
    
    pub fn get_prompt(&self) -> &str {
        match self.current_mode {
            OperatingMode::Plan => r#"
<mode>PLAN MODE - Read Only</mode>
You are analyzing and planning. You CAN:
✓ Read any file
✓ Search code using ripgrep and fd-find
✓ Analyze AST structure with tree-sitter
✓ Create detailed implementation plans

You CANNOT:
✗ Edit or write files
✗ Execute commands that modify state
"#,
            OperatingMode::Build => r#"
<mode>BUILD MODE - Full Access</mode>
You have complete development capabilities:
✓ Read, write, edit files
✓ Execute commands
✓ Use all tools
✓ Full AST-based editing
"#,
            OperatingMode::Review => r#"
<mode>REVIEW MODE - Quality Focus</mode>
You are reviewing code quality. You CAN:
✓ Read and analyze code
✓ Suggest improvements
✓ Make small fixes (< 100 lines)
Focus on: bugs, performance, best practices, security
"#,
        }
    }
}
```

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
  - Ripgrep
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

#### Mode-Aware Subagents
```rust
pub struct SubAgent {
    name: String,
    mode_preference: Option<OperatingMode>,
    intelligence_override: Option<IntelligenceMode>,
    ast_requirements: Vec<Language>,
    tool_whitelist: Vec<Tool>,
}
```

#### Subagent Chaining
```
# Sequential execution
@agent-architect -> @agent-code-generator -> @agent-test-writer

# Parallel analysis
@agent-security + @agent-performance + @agent-code-reviewer
```

#### Context Inheritance
- Subagents inherit AST indices from parent context
- Location-aware embeddings preserved across subagent calls
- Session history accessible but isolated

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

#### A. Complete Tree-sitter Integration (50+ Languages)
```toml
# agcodex-ast/Cargo.toml
[dependencies]
tree-sitter = "0.24"

# Core Languages (Most Used)
tree-sitter-rust = "0.23"
tree-sitter-python = "0.23"
tree-sitter-javascript = "0.23"
tree-sitter-typescript = "0.23"
tree-sitter-go = "0.23"
tree-sitter-java = "0.23"
tree-sitter-cpp = "0.23"
tree-sitter-c = "0.20"
tree-sitter-c-sharp = "0.23"

# Web Languages
tree-sitter-html = "0.23"
tree-sitter-css = "0.23"
tree-sitter-json = "0.24"
tree-sitter-yaml = "0.6"
tree-sitter-toml = "0.6"
tree-sitter-xml = "0.7"

# Scripting Languages
tree-sitter-bash = "0.23"
tree-sitter-lua = "0.2"
tree-sitter-ruby = "0.23"
tree-sitter-php = "0.23"
tree-sitter-perl = "0.6"

# Functional Languages
tree-sitter-haskell = "0.23"
tree-sitter-ocaml = "0.23"
tree-sitter-elixir = "0.3"
tree-sitter-erlang = "0.8"
tree-sitter-clojure = "0.2"
tree-sitter-scala = "0.23"

# Systems Languages
tree-sitter-zig = "0.23"
tree-sitter-nim = "0.2"
tree-sitter-swift = "0.5"
tree-sitter-kotlin = "0.3"
tree-sitter-objective-c = "3.0"

# Config/Data Languages
tree-sitter-dockerfile = "0.2"
tree-sitter-sql = "0.3"
tree-sitter-graphql = "0.1"
tree-sitter-protobuf = "0.1"

# Documentation
tree-sitter-markdown = "0.3"
tree-sitter-rst = "0.4"
tree-sitter-latex = "0.4"

# Infrastructure
tree-sitter-hcl = "0.1"  # Terraform
tree-sitter-nix = "0.1"
tree-sitter-make = "0.1"
tree-sitter-cmake = "0.5"

# Other Popular Languages
tree-sitter-r = "0.2"
tree-sitter-julia = "0.23"
tree-sitter-dart = "0.2"
tree-sitter-vue = "0.2"
tree-sitter-svelte = "0.11"
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

### 4. Native Tool Integration

#### fd-find Integration
```rust
// agcodex-tools/src/fd_find.rs
use ignore::WalkBuilder;
use regex::Regex;

pub struct FdFind {
    base_path: PathBuf,
    walker: WalkBuilder,
    filters: FdFilters,
}

impl FdFind {
    pub fn search_parallel(&self) -> Vec<PathBuf> {
        use rayon::prelude::*;
        let results = Mutex::new(Vec::new());
        
        self.walker.build_parallel().run(|| {
            Box::new(move |entry| {
                if let Ok(entry) = entry {
                    if self.matches_filters(entry.path()) {
                        results.lock().unwrap().push(entry.path().to_path_buf());
                    }
                }
                ignore::WalkState::Continue
            })
        });
        
        results.into_inner().unwrap()
    }
}
```

#### ripgrep Integration
```rust
// agcodex-tools/src/ripgrep.rs
use grep_regex::{RegexMatcher, RegexMatcherBuilder};
use grep_searcher::{BinaryDetection, SearcherBuilder};

pub struct RipGrep {
    matcher: RegexMatcher,
    searcher: SearcherBuilder,
    config: RgConfig,
}

impl RipGrep {
    pub fn search_with_ast_context(&self, path: &Path, ast: &Tree) -> Vec<Match> {
        let basic_matches = self.search_file(path)?;
        
        // Enrich with AST context
        basic_matches.into_iter()
            .map(|mut m| {
                if let Some(node) = find_node_at_position(ast, m.line, m.column) {
                    m.score *= get_node_importance(&node);
                    m.context_before.push(format!("// In {}", node.kind()));
                }
                m
            })
            .collect()
    }
}
```

### 5. Tool Integration Module

**Unified Structure**:
```rust
// core/src/code_tools/
pub mod code_tools {
    pub mod ripgrep;      // Native ripgrep integration
    pub mod fd_find;      // Native fd integration
    pub mod ast_grep;     // AST-based structural search
    pub mod tree_sitter;  // 50+ language parsers
    pub mod comby;        // Structural transformations
    
    // Unified interface
    pub trait CodeTool {
        type Query;
        type Result;
        fn search(&self, query: Self::Query) -> Result<Self::Result, ToolError>;
        fn search_parallel(&self, queries: Vec<Self::Query>) -> Vec<Self::Result>;
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

## AST-RAG Implementation Details

### Indexing Pipeline
```rust
pub struct ASTIndexer {
    parser_pool: ParserPool,        // Parallel parsers for 50+ languages
    chunk_store: ChunkStore,        // Hierarchical chunk storage
    vector_db: LanceDB,            // Vector embeddings
    symbol_graph: SymbolGraph,     // Relationship tracking
}

impl ASTIndexer {
    pub async fn index_codebase(&self, root: &Path) -> IndexStats {
        // Parallel file discovery and parsing
        let files = self.discover_files(root)?;
        let parsed = files.par_iter()
            .filter_map(|p| self.parse_file(p).ok())
            .collect();
        
        // Hierarchical chunking: File -> Class -> Function
        let chunks = self.extract_semantic_chunks(&parsed);
        
        // Generate location-aware embeddings
        let embeddings = self.generate_embeddings(&chunks).await?;
        
        // Store with precise metadata
        self.vector_db.insert_batch(embeddings).await?;
        
        IndexStats {
            files_indexed: files.len(),
            chunks_created: chunks.len(),
            compression_ratio: 0.92,  // Target: 90%+
        }
    }
}
```

### Semantic Chunking Strategy
```rust
pub enum ChunkLevel {
    File,      // Overview and imports
    Class,     // Class/module signatures
    Function,  // Function bodies
    Block,     // Complex code blocks
}

pub struct CodeChunk {
    level: ChunkLevel,
    content: String,  // AI Distiller compacted
    location: SourceLocation {
        file_path: PathBuf,
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
        byte_range: Range<usize>,
    },
    metadata: ChunkMetadata {
        language: String,
        symbols: Vec<String>,
        imports: Vec<String>,
        complexity: f32,
        compressed_size: usize,
        original_size: usize,
    },
}
```

### Intelligence Modes Configuration

#### Light Mode (Fast, Minimal Resources)
```toml
[intelligence.light]
embedding_model = "nomic-embed-text-v1.5"
chunk_size = 256
max_chunks = 1000
cache_size_mb = 100
indexing = "on_demand"
compression_level = "basic"  # 70% compression
```

#### Medium Mode (Balanced, Default)
```toml
[intelligence.medium]
embedding_model = "all-MiniLM-L6-v2"
chunk_size = 512
max_chunks = 10000
cache_size_mb = 500
indexing = "background"
compression_level = "standard"  # 85% compression
include_ast = true
```

#### Hard Mode (Maximum Intelligence)
```toml
[intelligence.hard]
embedding_model = "codebert-base"
chunk_size = 1024
max_chunks = 100000
cache_size_mb = 2000
indexing = "aggressive"
compression_level = "maximum"  # 95% compression
include_ast = true
include_call_graph = true
include_data_flow = true
```

## Session Persistence Implementation

### Storage Format Architecture
```rust
pub enum StorageFormat {
    Bincode,      // Metadata and indices (fastest)
    MessagePack,  // Messages (compact)
    Arrow,        // Tabular data (analytics)
    LanceDB,      // Vector embeddings (similarity)
    Zstd,         // Compression wrapper
}

pub struct SessionStore {
    base_dir: PathBuf,  // ~/.agcodex/history
    formats: HashMap<DataType, StorageFormat>,
    compressor: ZstdCompressor,
}

impl SessionStore {
    pub async fn save_efficient(&self, session: &Session) -> Result<()> {
        // Serialize metadata with bincode
        let meta = bincode::serialize(&session.metadata)?;
        
        // Serialize messages with MessagePack
        let msgs = rmp_serde::to_vec(&session.messages)?;
        
        // Compress with Zstd level 3 (balanced)
        let compressed_meta = zstd::encode_all(&meta[..], 3)?;
        let compressed_msgs = zstd::encode_all(&msgs[..], 3)?;
        
        // Write with version header
        let mut file = File::create(self.session_path(session.id))?;
        file.write_all(b"AGCX")?;  // Magic bytes
        file.write_all(&VERSION.to_le_bytes())?;
        file.write_all(&compressed_meta)?;
        file.write_all(&compressed_msgs)?;
        
        Ok(())
    }
}
```

### Fast Session Loading
```rust
pub struct FastSessionLoader {
    cache: Arc<MemoryMappedCache>,
    preload_queue: VecDeque<Uuid>,
}

impl FastSessionLoader {
    pub async fn load_lazy(&self, id: Uuid) -> LazySession {
        // Memory-mapped metadata (instant)
        let metadata = self.cache.get_metadata(id)?;
        
        // Load only recent messages
        let recent = self.load_recent_messages(id, 20).await?;
        
        // Lazy-load rest on demand
        LazySession {
            metadata,
            recent_messages: recent,
            loader: Box::new(move || self.load_full(id)),
        }
    }
}
```

## Implementation Timeline

### Week 1: Foundation & Core
- **Day 1-2**: Complete rebranding (codex → agcodex)
- **Day 3**: Implement Plan/Build/Review modes with Shift+Tab
- **Day 4**: Add all 50+ tree-sitter languages
- **Day 5**: Native fd-find and ripgrep integration

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
cargo flamegraph --bin agcodex -- exec "test command"

# Memory profiling  
cargo install cargo-bloat
cargo bloat --release --bin agcodex

# Benchmark specific functions
cargo bench --bench context_engine

# AST performance
cargo bench --bench ast_indexer
cargo bench --bench tree_sitter_parsing

# Session performance
cargo bench --bench session_persistence
```

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

### Complete AGCodex Configuration
```toml
# ~/.agcodex/config.toml

[general]
app_name = "AGCodex"
version = "1.0.0"
default_mode = "build"
reasoning_effort = "high"      # ALWAYS HIGH
verbosity = "high"             # ALWAYS HIGH

[intelligence]
enabled = true
mode = "medium"  # light/medium/hard
cache_dir = "~/.agcodex/history/cache"

[intelligence.light]
embedding_model = "nomic-embed-text-v1.5"
chunk_size = 256
cache_size_mb = 100

[intelligence.medium]
embedding_model = "all-MiniLM-L6-v2"
chunk_size = 512
cache_size_mb = 500
include_ast = true

[intelligence.hard]
embedding_model = "codebert-base"
chunk_size = 1024
cache_size_mb = 2000
include_ast = true
include_call_graph = true

[sessions]
auto_save = true
auto_save_interval = "30s"
base_dir = "~/.agcodex/history"
compression = "zstd"
max_sessions = 100

[tools]
fd_find.enabled = true
fd_find.parallel = true
ripgrep.enabled = true
ripgrep.cache = true
ast.enabled = true
ast.languages = "*"  # All 50+ languages

[ast_edit]
enabled = true
validation = "strict"
backup_before_edit = true
max_batch_edits = 100

[embeddings]
include_locations = true
metadata_level = "full"
context_before = 3
context_after = 3

[compaction]
preserve_mappings = true
precision = "high"
default_level = "medium"

# Operating modes configuration
[modes]
default = "build"
allow_switching = true
switch_key = "Shift+Tab"

[modes.plan]
read_only = true
color = "blue"
icon = "📋"
prompt_suffix = "You are in PLAN MODE. Create detailed plans but do not execute."

[modes.build]
full_access = true
color = "green"
icon = "🔨"
prompt_suffix = ""  # No restrictions

[modes.review]
quality_focus = true
color = "yellow"
icon = "🔍"
max_edit_size = 10000
prompt_suffix = "Focus on code quality, best practices, and potential issues."

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
use agcodex_tui::test_utils::*;

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

## Summary: AGCodex Transformation Goals

This overhaul transforms Codex into **AGCodex** - a powerful, independent AI coding assistant with:

### Core Transformation Tasks
1. **Complete Rebranding**: codex → agcodex across all crates and binaries
2. **Operating Modes**: Plan/Build/Review with Shift+Tab switching
3. **Tree-sitter Integration**: 50+ languages with AST-RAG and AI Distiller compaction
4. **Session Persistence**: ~/.agcodex/history with efficient Zstd compression
5. **Native Tools**: fd-find and ripgrep as internal Rust libraries
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
5. **Native Tool Integration**: fd-find and ripgrep as Rust libraries
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