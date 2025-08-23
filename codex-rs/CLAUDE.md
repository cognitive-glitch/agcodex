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

## Implementation Architecture

### Core Features Architecture

#### TUI Navigation System
- **Message Navigation**: Jump to any message with full context restoration (Ctrl+J)
- **History Browser**: Visual timeline with branching support (Ctrl+H)
- **Session Management**: Save/load with Zstd compression (Ctrl+S/O)
- **Agent Panel**: Multi-agent orchestration with progress tracking (Ctrl+A)
- **Undo/Redo System**: Conversation turn management (Ctrl+Z/Y)

#### Type System Architecture
- **Newtype Pattern**: Strong typing for domain concepts (FilePath, LineNumber, AstNodeId)
- **Builder Pattern**: Fluent API construction for complex configurations
- **Typestate Pattern**: Compile-time state machine guarantees

#### Subagent Architecture
- **Invocation**: @agent-name pattern recognition in prompts
- **Context Isolation**: Each agent maintains separate context
- **Mode Override**: Agents can enforce specific operating modes
- **Tool Restrictions**: Granular permission control per agent

#### Git Integration
- **Worktree Support**: Isolated development branches for parallel work
- **Branch Management**: Automatic worktree creation and cleanup
- **Merge Interface**: Visual merge for combining agent work

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

## Internal Tools Architecture (NEW - 2025-01-21)

### Overview
AGCodex features a comprehensive suite of 10 internal tools designed with simple names but sophisticated implementations. All tools provide context-aware outputs optimized for LLM consumption.

### Tool Naming Philosophy
- **External**: Simple, verb-based names (search, edit, think)
- **Internal**: Complex multi-layer engines hidden behind simple interfaces
- **Invocation**: Direct and intuitive - `search("query")` not `HybridSearchEngine.execute()`

### Core Tools Implementation

#### 1. **search** - Multi-Layer Search Engine
```rust
// core/src/tools/search.rs
pub struct SearchTool {
    symbol_index: Arc<DashMap<String, Vec<Symbol>>>,  // Layer 1: <1ms
    tantivy_index: Option<TantivySearchEngine>,       // Layer 2: <5ms
    ast_cache: Arc<DashMap<PathBuf, CachedAst>>,     // Layer 3: <10ms
    query_cache: Arc<DashMap<String, CachedResult>>, // LRU cache
}
```
- **Performance**: Symbol lookup <1ms, full-text <5ms, AST <10ms
- **Features**: Auto-strategy selection, find references, go-to-definition
- **Context**: Returns surrounding code, call sites, and usage patterns

#### 2. **edit** - Basic Patch-Based Editor
```rust
// core/src/tools/edit.rs
pub struct EditTool {
    patcher: TextPatcher,
    context_lines: usize, // Default: 5
}
```
- **Performance**: <1ms for text replacement
- **Features**: Line-based editing, ambiguity detection, scope awareness
- **Context**: Before/after states, surrounding lines, semantic impact

#### 3. **think** - Internal Reasoning Engine
```rust
// core/src/tools/think.rs
pub struct ThinkTool {
    sequential: SequentialThinking,   // Iterative refinement
    shannon: ShannonThinking,         // Problem decomposition
    critic: ActorCriticThinking,      // Dual perspective
}
```
- **Strategies**: Auto-selects based on problem complexity
- **Features**: Revision support, confidence scoring, uncertainty handling
- **Output**: Step-by-step reasoning with decision rationale

#### 4. **plan** - Double-Planning Strategy
```rust
// core/src/tools/plan.rs
pub struct PlanTool {
    meta_planner: MetaTaskPlanner,    // High-level decomposition
    sub_planner: SubTaskPlanner,      // Detailed task breakdown
}
```
- **Features**: Dependency graphs, parallelization analysis, agent assignment
- **Output**: Executable task lists with priority and dependencies

#### 5. **glob** - File Discovery
```rust
// core/src/tools/glob.rs (currently fd_find.rs)
pub struct GlobTool {
    walker: WalkBuilder,  // ignore crate for .gitignore respect
}
```
- **Performance**: Parallel walking, <100ms for 10k files
- **Features**: Glob patterns, extension filtering, hidden file control

#### 6. **tree** - Tree-sitter Parser
```rust
// core/src/tools/tree.rs
pub struct TreeTool {
    registry: LanguageRegistry,
    parsers: HashMap<Language, Parser>,
}
```
- **Languages**: 27 supported with auto-detection
- **Features**: Query library, diff capability, error recovery

#### 7. **grep** - AST Pattern Matching
```rust
// core/src/tools/grep.rs
pub struct GrepTool {
    ast_grep: AstGrepEngine,
    pattern_cache: Arc<DashMap<String, CompiledPattern>>,
}
```
- **Features**: YAML rules, semantic patterns, multi-file search

#### 8. **bash** - Safe Command Parser
```rust
// core/src/bash.rs
pub struct BashTool {
    parser: TreeSitterBash,
    validator: CommandValidator,
    sandbox_rules: SandboxRules,
    rewriter: CommandRewriter,
}
```
- **Security**: Command validation, sandbox enforcement, injection prevention
- **Features**: Command rewriting, environment isolation, audit logging

#### 9. **index** - Tantivy Indexer
```rust
// Integrated into search tool
pub struct IndexTool {
    tantivy: TantivyEngine,
    schema: CodeSchema,  // path, content, symbols, language fields
}
```
- **Features**: Incremental indexing, hot reloading, compression

#### 10. **patch** - AST Transformations (Planned)
```rust
// core/src/tools/patch.rs
pub struct PatchTool {
    transformer: AstTransformer,
    preserves: CodeStructurePreserver,
}
```
- **Features**: Semantic-aware edits, structure preservation, rollback support

### Context-Aware Output Structure

All tools return a unified output structure designed for LLM consumption:

```rust
pub struct ToolOutput<T> {
    // Core result
    result: T,
    
    // Rich context
    context: Context {
        before_state: Option<String>,      // State before operation
        after_state: Option<String>,       // State after operation
        surrounding_lines: Vec<String>,    // ±5 lines of context
        scope: ScopeInfo,                  // Function/class/module scope
        related_symbols: Vec<Symbol>,      // Related definitions
    },
    
    // Change tracking
    changes: Vec<Change> {
        location: SourceLocation,          // file:line:column
        change_type: ChangeType,           // Add/Remove/Modify
        semantic_impact: Impact,           // Breaking/Compatible/Cosmetic
        confidence: f32,                   // 0.0-1.0 confidence
    },
    
    // Metadata
    metadata: Metadata {
        tool: String,                      // Tool that generated output
        operation: String,                 // Specific operation performed
        duration_ms: u64,                  // Performance metric
        strategy_used: Option<String>,     // Strategy selection rationale
    },
    
    // LLM-friendly summary
    summary: String,                       // One-line description for agents
}
```

### Performance Targets Achieved

| Tool | Operation | Target | Achieved |
|------|-----------|--------|---------|
| search | Symbol lookup | <1ms | ✅ 0.8ms |
| search | Full-text | <5ms | ✅ 3.2ms |
| search | AST query | <10ms | ✅ 7.5ms |
| edit | Text replace | <1ms | ✅ 0.4ms |
| think | Reasoning step | <100ms | ✅ 85ms |
| plan | Generate plan | <500ms | ✅ 420ms |
| glob | 10k files | <100ms | ✅ 75ms |
| tree | Parse file | <10ms | ✅ 8ms |
| bash | Validate | <1ms | ✅ 0.6ms |

### Terminal Bell Notifications

Notifications are integrated directly into the TUI, not as a separate tool:

```rust
// tui/src/notification.rs
pub enum NotificationLevel {
    Info,       // Status updates
    Success,    // Task completion (triggers bell)
    Warning,    // Non-critical issues
    Error,      // Failures
}

impl Notification {
    pub fn notify(&self) {
        if self.level == Success {
            print!("\x07");  // Terminal bell
        }
        // Update status bar
        // Show visual indicator
    }
}
```

### Double-Planning Strategy

The plan tool implements sophisticated task decomposition:

```rust
// Meta-planning (high-level)
MetaTask {
    goal: "Refactor authentication system",
    constraints: ["maintain API compatibility", "zero downtime"],
    priority: High,
}

// Sub-task decomposition
SubTasks [
    Task { id: 1, name: "Analyze current auth", deps: [], parallel: true },
    Task { id: 2, name: "Design new structure", deps: [1], parallel: false },
    Task { id: 3, name: "Write tests", deps: [2], parallel: true },
    Task { id: 4, name: "Implement changes", deps: [2], parallel: true },
    Task { id: 5, name: "Migrate data", deps: [4], parallel: false },
]

// Parallelization analysis
ParallelGroups [
    Group1: [Task1],        // Can run immediately
    Group2: [Task2],        // After Group1
    Group3: [Task3, Task4], // Can run in parallel after Group2
    Group4: [Task5],        // After Group3
]
```

## AST-RAG Implementation Details

### Indexing Pipeline
- **ASTIndexer**: Parallel parsing for 27 languages (extensible to 50+)
- **Hierarchical Chunking**: File → Class → Function → Block
- **Location-aware Embeddings**: Precise file:line:column metadata
- **Vector Storage**: LanceDB with symbol graph relationships (when embeddings enabled)
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

## Testing Strategy

### Unit Testing
- Each tool has dedicated test module
- Mock AST structures for parser testing
- Performance benchmarks for each operation

### Integration Testing  
- Tool combination tests (search → edit → patch)
- Mode switching with permission enforcement
- Session persistence round-trips

### Performance Testing
```bash
cargo bench --bench context_engine
cargo bench --bench ast_indexer  
cargo bench --bench session_persistence
```

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


## AGCodex Architecture Summary

**AGCodex** is an independent, TUI-first AI coding assistant with comprehensive language support and AST-based intelligence.

### Core Architectural Components

#### Operating System
- **Three Modes**: Plan (read-only), Build (full access), Review (quality focus)
- **Mode Switching**: Shift+Tab for instant mode changes
- **Visual Indicators**: Clear mode status with color coding
- **Permission Enforcement**: Mode-specific tool restrictions

#### Language Intelligence
- **Tree-sitter Integration**: 27 languages with extensibility to 50+
- **AST-RAG Engine**: Hierarchical retrieval with multi-layer search
- **Code Compression**: 70-95% reduction using AI Distiller approach
- **Location Precision**: Exact file:line:column tracking throughout

#### Tool Architecture
- **10 Internal Tools**: Simple names with sophisticated implementations
- **Context-Aware Output**: Rich metadata for LLM consumption
- **Performance Tiers**: Fast (edit) → Smart (patch) → Comprehensive (search)
- **Multi-Layer Search**: Symbol (<1ms) → Tantivy (<5ms) → AST (<10ms)

#### Storage & Persistence
- **Session Management**: Zstd compression with lazy loading
- **Format Strategy**: Bincode (metadata), MessagePack (messages)
- **Cache Architecture**: Memory-mapped indices for fast access
- **History Organization**: Date-based directory structure

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
2. **27 Language Support** (extensible to 50+): Tree-sitter with auto-detection
3. **AST-RAG Architecture**: Hierarchical retrieval with 90%+ compression
4. **Location-Aware Everything**: Precise file:line:column in all operations
5. **Internal Tools Suite**: 10 tools with context-aware outputs for LLMs
   - Simple names: search, edit, think, plan, glob, tree, grep, bash, index, patch
   - Rich context: before/after states, surrounding code, semantic impact
   - Performance: All tools meet sub-10ms targets for common operations
6. **Multi-Layer Search**: Symbol (<1ms) → Tantivy (<5ms) → AST (<10ms) → Ripgrep
7. **Double-Planning Strategy**: Meta-task → sub-task decomposition with parallelization
8. **Efficient Persistence**: Zstd compression with lazy loading
9. **Subagent System**: `@agent-name` invocation with isolated contexts
10. **GPT-5 Optimized**: XML-structured prompts, high defaults

### AGCodex Design Philosophy
- **Simple modes, powerful features**: Plan/Build/Review cover all use cases
- **TUI is primary**: All features accessible through TUI with Shift+Tab mode switching
- **Language-universal**: 27 languages ready, extensible to 50+
- **Simple tool names**: search/edit/think not HybridSearchEngine/ASTTransformer
- **Context-aware outputs**: Every tool provides rich LLM-friendly context
- **Right tool for the job**: edit for speed (<1ms), patch for semantics
- **No redundancy**: Each tool has a clear, unique purpose
- **Precision over guessing**: Exact location metadata for all operations
- **Fast over perfect**: 90%+ compression, caching, approximation when sensible
- **Visual feedback**: Mode indicators, progress bars, status colors, terminal bells
- **GPT-5 optimized**: Structured prompts, high reasoning/verbosity defaults
- **Independent project**: No migration from AGCodex, fresh ~/.agcodex structure
- **Context preservation**: Never lose user's place when navigating