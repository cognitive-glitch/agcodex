# AGCodex Complete Overhaul Plans

## Table of Contents
1. [Executive Summary](#executive-summary)
2. [Core Philosophy](#core-philosophy)
3. [Phase 1: Complete Rebranding](#phase-1-complete-rebranding)
4. [Phase 2: Operating Modes System](#phase-2-operating-modes-system)
5. [Phase 3: AGCodex Subagent System](#phase-3-agcodex-subagent-system)
6. [Phase 4: Comprehensive Tree-sitter Integration](#phase-4-comprehensive-tree-sitter-integration)
7. [Phase 5: Internal Tools Integration](#phase-5-internal-tools-integration)
8. [Phase 6: AST-RAG for Large Codebases](#phase-6-ast-rag-for-large-codebases)
9. [Phase 7: AST-Based Edit Tools](#phase-7-ast-based-edit-tools)
10. [Phase 8: Session Persistence](#phase-8-session-persistence)
11. [Phase 9: Configuration System](#phase-9-configuration-system)
12. [Implementation Timeline](#implementation-timeline)
13. [Success Metrics](#success-metrics)

## Executive Summary

AGCodex is a complete overhaul of the original Codex project, transforming it into an independent, powerful AI coding assistant with:
- **Three simple operating modes**: Plan (read-only), Build (full access), Review (quality focus)
- **Sophisticated subagent system**: Specialized AI assistants invoked via `@agent-name`
- **50+ language support** via comprehensive tree-sitter integration
- **AST-based intelligence** for precise code understanding and modification
- **Efficient session persistence** with 90%+ compression
- **GPT-5 best practices** with high reasoning and verbosity defaults
- **Location-aware embeddings** for precise agentic coding
- **Native tool integration**: fd-find and ripgrep as Rust libraries

## Core Philosophy

### Keep It Simple, Make It Powerful
- **Clear, structured prompts** using XML-like tags (GPT-5 best practice)
- **Smart code understanding** that reduces complexity by 90%+ (AI Distiller approach)
- **Simple modes** that users can easily understand
- **Visual feedback** for current state and available operations
- **Language-universal** support out of the box

### Design Principles
1. **Clarity over cleverness**: Simple names, clear functions
2. **Visual over textual**: Icons and colors show state
3. **Fast over perfect**: Cache and approximate when possible
4. **Guided over autonomous**: Help users, don't surprise them
5. **Precision over guessing**: Exact locations for all operations

## Phase 1: Complete Rebranding

### 1.1 Global Renaming Strategy
```bash
# All crate renamings
codex-core → agcodex-core
codex-tui → agcodex-tui
codex-cli → agcodex-cli
codex-protocol → agcodex-protocol
codex-protocol-ts → agcodex-protocol-ts
codex-mcp-client → agcodex-mcp-client
codex-mcp-server → agcodex-mcp-server
codex-mcp-types → agcodex-mcp-types
codex-file-search → agcodex-file-search
codex-apply-patch → agcodex-apply-patch
codex-execpolicy → agcodex-execpolicy
codex-linux-sandbox → agcodex-linux-sandbox
codex-ansi-escape → agcodex-ansi-escape
codex-common → agcodex-common
codex-login → agcodex-login
codex-chatgpt → agcodex-chatgpt
codex-ollama → agcodex-ollama
codex-arg0 → agcodex-arg0
codex-persistence → agcodex-persistence  # NEW

# Binary renaming
codex → agcodex

# Remove OpenAI-specific branding
# Keep API compatibility as generic LLM interface
```

### 1.2 New Directory Structure
```
~/.agcodex/
├── config.toml              # Main configuration with HIGH defaults
├── history/                 # Efficient session persistence
│   ├── index.bincode       # Fast binary index
│   ├── sessions/           # Date-organized sessions
│   │   ├── 2024-01-20/
│   │   │   ├── session_uuid1.zst
│   │   │   └── session_uuid2.zst
│   │   └── 2024-01-21/
│   │       └── session_uuid3.zst
│   ├── cache/              # Performance caches
│   │   ├── embeddings/     # LanceDB vector storage
│   │   │   └── index.lance
│   │   ├── ast/           # Cached AST data
│   │   │   └── *.bincode.zst
│   │   └── search/        # Search indices
│   │       └── tantivy/
│   └── checkpoints/       # Auto-save checkpoints
│       └── *.checkpoint.zst
├── modes/                  # Mode-specific configurations
│   ├── plan.toml
│   ├── execute.toml
│   ├── review.toml
│   └── debug.toml
├── tools/                  # Tool configurations
│   ├── prompts/           # Tool-specific prompts
│   └── configs/           # Tool settings
├── auth/                   # Authentication tokens
├── logs/                   # Application logs
└── models/                 # Model configurations
```

### 1.3 Configuration with High Defaults
```rust
// agcodex-core/src/config.rs
impl Default for AGCodexConfig {
    fn default() -> Self {
        Self {
            app_name: "AGCodex".to_string(),
            base_dir: expand_path("~/.agcodex"),
            reasoning_effort: ReasoningEffort::High,  // HIGH by default
            verbosity: Verbosity::High,               // HIGH by default
            default_mode: OperatingMode::Build,
            auto_save: true,
            intelligence_mode: IntelligenceMode::Medium,
            session_persistence: true,
            cache_enabled: true,
        }
    }
}
```

## Phase 2: Operating Modes System

### 2.1 Three Clear Modes with Shift+Tab Switching

```rust
// agcodex-core/src/modes.rs
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum OperatingMode {
    Plan,       // 📋 Read-only mode for planning and analysis
    Build,      // 🔨 Full execution mode (default)
    Review,     // 👁️ Code review mode with limited writes
}

impl OperatingMode {
    pub fn is_read_only(&self) -> bool {
        matches!(self, Self::Plan)
    }
    
    pub fn allows_write(&self) -> bool {
        !self.is_read_only()
    }
    
    pub fn allows_execution(&self) -> bool {
        matches!(self, Self::Build)
    }
    
    pub fn get_prompt(&self) -> &str {
        match self {
            Self::Plan => r#"
<mode>PLAN MODE - Read Only</mode>
You are analyzing and planning. You CAN:
✓ Read any file
✓ Search code using ripgrep and fd-find
✓ Analyze AST structure with tree-sitter
✓ Create detailed implementation plans
✓ Research and gather information

You CANNOT:
✗ Edit or write files
✗ Execute commands that modify state
✗ Make git commits or modifications
✗ Run build or test commands
✗ Install packages

Remind user: "Press Shift+Tab for Build mode when ready to implement"
"#,
            Self::Build => r#"
<mode>BUILD MODE - Full Access</mode>
You have complete development capabilities:
✓ Read, write, edit files
✓ Execute commands
✓ Use all tools
✓ Make commits
✓ Full AST-based editing
"#,
            Self::Review => r#"
<mode>REVIEW MODE - Quality Focus</mode>
You are reviewing code quality. You CAN:
✓ Read and analyze code
✓ Suggest improvements
✓ Make small fixes (< 100 lines)
✓ Use AST analysis tools

Focus on: bugs, performance, best practices, security
"#,
        }
    }
}
```

### 2.2 Mode Manager with Restrictions

```rust
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
        self.restrictions = Self::get_restrictions(new_mode);
    }
    
    fn get_restrictions(mode: OperatingMode) -> ModeRestrictions {
        match mode {
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
                max_file_size: Some(10_000),  // Only small edits
            },
        }
    }
}
```

### 2.3 TUI Mode Interface

```rust
// agcodex-tui/src/mode_switcher.rs
pub struct ModeSwitcher {
    modes: Vec<ModeInfo>,
    current_index: usize,
    transition: Transition,
}

// Key binding for mode switching
pub const MODE_SWITCH_KEY: KeyBinding = KeyBinding {
    key: "Shift+Tab",
    action: Action::CycleMode,
};

// Visual mode indicator at top of screen
impl Widget for ModeIndicator {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (text, style) = match self.mode {
            OperatingMode::Plan => (
                " 📋 PLAN MODE (Read-Only) ",
                Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            OperatingMode::Build => (
                " 🔨 BUILD MODE ",
                Style::default().bg(Color::Green).fg(Color::Black),
            ),
            OperatingMode::Review => (
                " 👁️ REVIEW MODE ",
                Style::default().bg(Color::Yellow).fg(Color::Black),
            ),
        };
        
        buf.set_string(area.x, area.y, text, style);
    }
}
```

## Phase 3: AGCodex Subagent System

### 3.1 Subagent Architecture

```rust
// agcodex-core/src/subagents/mod.rs
pub struct SubAgent {
    name: String,
    description: String,
    mode_preference: Option<OperatingMode>,
    intelligence_override: Option<IntelligenceMode>,
    ast_requirements: Vec<Language>,
    tool_whitelist: Vec<Tool>,
    custom_prompt: String,
}

pub struct SubAgentManager {
    agents: HashMap<String, SubAgent>,
    active_agents: Vec<AgentId>,
    context_isolation: bool,
}

impl SubAgentManager {
    pub fn invoke(&mut self, agent_name: &str) -> Result<AgentSession> {
        // Validate agent exists
        let agent = self.agents.get(agent_name)?;
        
        // Create isolated context
        let context = IsolatedContext::new();
        
        // Apply agent configuration
        context.set_mode(agent.mode_preference);
        context.set_intelligence(agent.intelligence_override);
        context.set_tools(agent.tool_whitelist.clone());
        
        // Start agent session
        Ok(AgentSession {
            agent: agent.clone(),
            context,
            start_time: Utc::now(),
        })
    }
}
```

### 3.2 Subagent Invocation

```rust
// User invocation patterns
pub enum InvocationPattern {
    Direct(String),        // @agent-code-reviewer
    Sequential(Vec<String>), // @agent-architect -> @agent-code-generator
    Parallel(Vec<String>),   // @agent-security + @agent-performance
    Conditional {           // @agent-debugger if errors
        agent: String,
        condition: Condition,
    },
}

// Parse user input for agent invocations
pub fn parse_agent_invocation(input: &str) -> Vec<InvocationPattern> {
    let mut patterns = Vec::new();
    
    // Direct invocation: @agent-name
    let direct_regex = Regex::new(r"@agent-([a-z-]+)").unwrap();
    for cap in direct_regex.captures_iter(input) {
        patterns.push(InvocationPattern::Direct(cap[1].to_string()));
    }
    
    // Sequential: @agent1 -> @agent2
    let seq_regex = Regex::new(r"@agent-([a-z-]+)\s*->\s*@agent-([a-z-]+)").unwrap();
    // ... parse sequential patterns
    
    // Parallel: @agent1 + @agent2
    let par_regex = Regex::new(r"@agent-([a-z-]+)\s*\+\s*@agent-([a-z-]+)").unwrap();
    // ... parse parallel patterns
    
    patterns
}
```

### 3.3 Built-in Subagents

```yaml
# ~/.agcodex/agents/code-reviewer.yaml
name: code-reviewer
description: Proactively reviews code for quality, security, and maintainability
mode_override: review
tools:
  - Read
  - AST-Search
  - Ripgrep
  - Tree-sitter-analyze
intelligence: hard
prompt: |
  You are a senior code reviewer with AST-based analysis.
  
  Review checklist:
  - Syntactic correctness via tree-sitter validation
  - Security vulnerabilities (OWASP Top 10)
  - Performance bottlenecks (O(n²) or worse)
  - Memory leaks and resource management
  - Error handling completeness
  - Code duplication detection
  - Type safety verification
  
  Provide feedback organized by:
  - Critical issues (must fix)
  - Warnings (should fix)
  - Suggestions (consider improving)
```

```yaml
# ~/.agcodex/agents/refactorer.yaml
name: refactorer
description: Systematic code restructuring with AST preservation
mode_override: build
tools:
  - Read
  - Edit
  - AST-Edit
  - Tree-sitter-refactor
  - Comby
intelligence: hard
prompt: |
  You are a refactoring specialist using AST-based transformations.
  
  Refactoring approach:
  1. Analyze current structure with tree-sitter
  2. Identify refactoring opportunities
  3. Create AST-preserving transformations
  4. Apply changes with location tracking
  5. Verify syntactic correctness
  
  Focus on:
  - Extract method/variable
  - Rename with full reference tracking
  - Move to appropriate modules
  - Remove dead code
  - Simplify complex expressions
```

```yaml
# ~/.agcodex/agents/test-writer.yaml
name: test-writer
description: Comprehensive test generation with coverage analysis
mode_override: build
tools:
  - Read
  - Write
  - AST-Analyze
  - Coverage-Check
intelligence: medium
prompt: |
  You are a test generation specialist.
  
  Test creation process:
  1. Analyze function signatures with AST
  2. Identify edge cases and boundaries
  3. Generate comprehensive test cases
  4. Include positive and negative tests
  5. Add property-based tests where applicable
  
  Test types:
  - Unit tests for individual functions
  - Integration tests for modules
  - Property-based tests for invariants
  - Fuzz tests for robustness
  - Performance benchmarks
```

### 3.4 Subagent Storage Structure

```
~/.agcodex/
├── agents/                    # User-level subagents
│   ├── global/               # Available everywhere
│   │   ├── code-reviewer.yaml
│   │   ├── refactorer.yaml
│   │   ├── debugger.yaml
│   │   ├── test-writer.yaml
│   │   ├── performance.yaml
│   │   ├── security.yaml
│   │   ├── docs.yaml
│   │   └── architect.yaml
│   └── templates/            # Reusable templates
│       ├── basic-reviewer.yaml
│       ├── basic-tester.yaml
│       └── basic-debugger.yaml
└── .agcodex/                 # Project root
    └── agents/               # Project-specific subagents
        ├── domain-expert.yaml
        ├── legacy-migrator.yaml
        └── custom-linter.yaml
```

### 3.5 Subagent Context Management

```rust
pub struct IsolatedContext {
    // Separate from main conversation
    messages: Vec<Message>,
    
    // Inherits AST indices but isolated changes
    ast_index: Arc<ASTIndex>,
    
    // Own embedding cache
    embeddings: HashMap<ChunkId, Embedding>,
    
    // Tool restrictions
    allowed_tools: HashSet<Tool>,
    
    // Mode enforcement
    operating_mode: OperatingMode,
}

impl IsolatedContext {
    pub fn inherit_from_parent(&mut self, parent: &Context) {
        // Inherit read-only data
        self.ast_index = parent.ast_index.clone();
        
        // Copy relevant embeddings
        for (id, embedding) in &parent.embeddings {
            if self.is_relevant(embedding) {
                self.embeddings.insert(*id, embedding.clone());
            }
        }
    }
    
    pub fn merge_back(&self, parent: &mut Context) -> Result<()> {
        // Selective merge of changes
        for message in &self.messages {
            if message.is_result() {
                parent.add_subagent_result(message.clone());
            }
        }
        
        Ok(())
    }
}
```

### 3.6 Subagent Performance Optimization

```rust
pub struct SubAgentCache {
    // Cache compiled prompts
    prompt_cache: HashMap<String, CompiledPrompt>,
    
    // Cache agent configurations
    config_cache: HashMap<String, SubAgentConfig>,
    
    // Preload frequently used agents
    preloaded: HashSet<String>,
}

impl SubAgentCache {
    pub async fn preload_common_agents(&mut self) {
        let common = vec![
            "code-reviewer",
            "refactorer",
            "debugger",
            "test-writer",
        ];
        
        for agent_name in common {
            let config = self.load_config(agent_name).await?;
            let prompt = self.compile_prompt(&config.prompt).await?;
            
            self.config_cache.insert(agent_name.to_string(), config);
            self.prompt_cache.insert(agent_name.to_string(), prompt);
            self.preloaded.insert(agent_name.to_string());
        }
    }
}
```

## Phase 4: Comprehensive Tree-sitter Integration

### 3.1 Full Language Support (50+ Languages)

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

### 3.2 Language Registry with Auto-Detection

```rust
// agcodex-ast/src/languages.rs
pub struct LanguageRegistry {
    languages: HashMap<String, LanguageInfo>,
}

pub struct LanguageInfo {
    name: String,
    extensions: Vec<String>,
    language: Language,
    display_name: String,
}

impl LanguageRegistry {
    pub fn new() -> Self {
        let mut languages = HashMap::new();
        
        // Register all 50+ languages
        languages.insert("rust".to_string(), LanguageInfo {
            name: "rust".to_string(),
            extensions: vec!["rs".to_string()],
            language: tree_sitter_rust::LANGUAGE.into(),
            display_name: "Rust".to_string(),
        });
        
        languages.insert("python".to_string(), LanguageInfo {
            name: "python".to_string(),
            extensions: vec!["py".to_string(), "pyw".to_string()],
            language: tree_sitter_python::LANGUAGE.into(),
            display_name: "Python".to_string(),
        });
        
        // ... All 50+ languages registered ...
        
        Self { languages }
    }
    
    pub fn detect_from_file(path: &Path) -> Option<&LanguageInfo> {
        let ext = path.extension()?.to_str()?;
        
        for (_, info) in &self.languages {
            if info.extensions.contains(&ext.to_string()) {
                return Some(info);
            }
        }
        None
    }
}
```

### 3.3 Universal Code Understanding

```rust
// agcodex-ast/src/understand.rs
pub struct CodeUnderstander {
    registry: LanguageRegistry,
    cache: HashMap<PathBuf, ParsedFile>,
}

impl CodeUnderstander {
    pub fn understand_file(&mut self, path: &Path) -> Result<FileUnderstanding> {
        // Auto-detect language
        let language = self.registry.detect_from_file(path)?;
        
        // Parse with appropriate parser
        let content = std::fs::read_to_string(path)?;
        let parser = self.registry.get_parser(&language.name)?;
        let tree = parser.parse(&content, None)?;
        
        // Extract universal concepts
        Ok(FileUnderstanding {
            language: language.display_name.clone(),
            functions: self.extract_functions(&tree, &content),
            classes: self.extract_classes(&tree, &content),
            imports: self.extract_imports(&tree, &content),
            exports: self.extract_exports(&tree, &content),
            complexity: self.calculate_complexity(&tree),
            line_count: content.lines().count(),
        })
    }
}
```

## Phase 5: Internal Tools Integration

### 4.1 Native fd-find Integration

```rust
// agcodex-tools/src/fd_find.rs
use ignore::WalkBuilder;
use regex::Regex;

pub struct FdFind {
    base_path: PathBuf,
    walker: WalkBuilder,
    filters: FdFilters,
}

pub struct FdFilters {
    pub extensions: Vec<String>,
    pub patterns: Vec<Regex>,
    pub file_type: FileType,
    pub max_depth: Option<usize>,
    pub hidden: bool,
    pub ignore_git: bool,
    pub follow_symlinks: bool,
}

impl FdFind {
    pub fn search_parallel(&self) -> Vec<PathBuf> {
        use rayon::prelude::*;
        use std::sync::Mutex;
        
        let results = Mutex::new(Vec::new());
        
        self.walker.build_parallel().run(|| {
            let results = results.clone();
            Box::new(move |entry| {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if self.matches_filters(path) {
                        results.lock().unwrap().push(path.to_path_buf());
                    }
                }
                ignore::WalkState::Continue
            })
        });
        
        results.into_inner().unwrap()
    }
}
```

### 4.2 Native ripgrep Integration

```rust
// agcodex-tools/src/ripgrep.rs
use grep_regex::{RegexMatcher, RegexMatcherBuilder};
use grep_searcher::{BinaryDetection, SearcherBuilder, Sink};

pub struct RipGrep {
    matcher: RegexMatcher,
    searcher: SearcherBuilder,
    config: RgConfig,
}

pub struct RgConfig {
    pub case_sensitive: bool,
    pub multiline: bool,
    pub word_boundary: bool,
    pub invert_match: bool,
    pub max_count: Option<usize>,
    pub max_depth: Option<usize>,
    pub file_types: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub context_lines: usize,
}

impl RipGrep {
    pub fn search_with_ast_context(&self, path: &Path, ast: &Tree) -> Result<Vec<Match>, Error> {
        // Enhanced search that uses AST information
        let basic_matches = self.search_file(path)?;
        
        // Enrich matches with AST context
        let enriched: Vec<Match> = basic_matches
            .into_iter()
            .map(|mut m| {
                // Find AST node at match location
                if let Some(node) = find_node_at_position(ast, m.line_number, m.column) {
                    // Add semantic information
                    m.score *= get_node_importance(&node);
                    m.context_before.push(format!("// In {}", node.kind()));
                }
                m
            })
            .collect();
        
        Ok(enriched)
    }
}
```

### 4.3 Enhanced AST Tools

```rust
// agcodex-tools/src/ast_tools.rs
pub struct ASTSearchTool {
    engine: Arc<ASTEngine>,
}

impl Tool for ASTSearchTool {
    fn name(&self) -> &str {
        "ast_search"
    }
    
    fn description(&self) -> &str {
        "Search code using AST patterns and queries"
    }
    
    fn execute(&self, params: ToolParams) -> Result<ToolResult> {
        let results = match params.search_type {
            SearchType::Query => {
                // Use tree-sitter query syntax
                self.engine.query(&params.tree, &params.query, &params.code)?
            }
            SearchType::Pattern => {
                // Use structural pattern matching
                self.engine.find_similar_code(&params.pattern, params.files)?
            }
            SearchType::Symbol => {
                // Find symbol definitions and references
                self.engine.find_symbol(&params.symbol, params.files)?
            }
        };
        
        Ok(ToolResult::ASTMatches(results))
    }
}
```

## Phase 6: AST-RAG for Large Codebases

### 5.1 Indexing Pipeline

```rust
// agcodex-rag/src/indexer.rs
pub struct ASTIndexer {
    parser_pool: ParserPool,        // Parallel parsers
    chunk_store: ChunkStore,        // Stores code chunks
    vector_db: VectorDatabase,      // Stores embeddings
    symbol_graph: SymbolGraph,      // Tracks relationships
}

impl ASTIndexer {
    pub async fn index_codebase(&self, root: &Path) -> Result<IndexStats> {
        // Step 1: Discover all code files
        let files = self.discover_files(root)?;
        println!("Found {} files to index", files.len());
        
        // Step 2: Parse files in parallel
        let parsed_files: Vec<ParsedFile> = files
            .par_iter()
            .filter_map(|path| self.parse_file(path).ok())
            .collect();
        
        // Step 3: Extract semantic chunks
        let mut all_chunks = Vec::new();
        for file in parsed_files {
            let chunks = self.extract_chunks(&file);
            all_chunks.extend(chunks);
        }
        
        // Step 4: Generate embeddings
        let embeddings = self.generate_embeddings(&all_chunks).await?;
        
        // Step 5: Store in vector database
        self.vector_db.insert_batch(embeddings).await?;
        
        Ok(IndexStats {
            files_indexed: files.len(),
            chunks_created: all_chunks.len(),
            total_size: calculate_size(&all_chunks),
        })
    }
}
```

### 5.2 Semantic Chunking Strategy

```rust
// agcodex-rag/src/chunker.rs
pub struct SemanticChunker {
    max_chunk_size: usize,  // e.g., 512 tokens
    overlap_size: usize,    // e.g., 64 tokens
}

impl SemanticChunker {
    pub fn chunk_file(&self, file: &ParsedFile) -> Vec<CodeChunk> {
        let mut chunks = Vec::new();
        
        // Level 1: File-level chunk (overview)
        chunks.push(CodeChunk {
            id: generate_id(),
            level: ChunkLevel::File,
            content: self.create_file_summary(file),
            metadata: ChunkMetadata {
                file_path: file.path.clone(),
                language: file.language.clone(),
                chunk_type: "file_summary",
                symbols: extract_all_symbols(file),
            },
        });
        
        // Level 2: Class/Module chunks
        for class in &file.classes {
            chunks.push(CodeChunk {
                id: generate_id(),
                level: ChunkLevel::Class,
                content: self.distill_class(class),
                metadata: ChunkMetadata {
                    file_path: file.path.clone(),
                    language: file.language.clone(),
                    chunk_type: "class",
                    symbols: vec![class.name.clone()],
                },
            });
        }
        
        // Level 3: Function chunks
        for function in &file.functions {
            if function.lines_of_code < 50 {
                chunks.push(self.create_function_chunk(function, true));
            } else {
                chunks.push(self.create_function_chunk(function, false));
                chunks.extend(self.chunk_function_body(function));
            }
        }
        
        chunks
    }
}
```

### 5.3 Vector Storage and Retrieval

```rust
// agcodex-rag/src/vector_db.rs
pub struct VectorDatabase {
    lance_db: lancedb::Database,
    indices: HashMap<ChunkLevel, HNSWIndex>,
}

impl VectorDatabase {
    pub async fn search(
        &self,
        query_embedding: &[f32],
        filters: SearchFilters,
        limit: usize,
    ) -> Vec<SearchResult> {
        // Multi-stage retrieval
        let mut results = Vec::new();
        
        // Stage 1: Coarse search (file level)
        let file_matches = self.search_level(
            ChunkLevel::File,
            query_embedding,
            limit * 2,
        ).await?;
        
        // Stage 2: Refined search within matched files
        for file_match in file_matches.iter().take(10) {
            let refined = self.search_within_file(
                &file_match.file_path,
                query_embedding,
                limit / 2,
            ).await?;
            results.extend(refined);
        }
        
        // Stage 3: Direct function search
        if filters.search_functions {
            let function_matches = self.search_level(
                ChunkLevel::Function,
                query_embedding,
                limit,
            ).await?;
            results.extend(function_matches);
        }
        
        self.rerank_results(results, query_embedding, limit)
    }
}
```

### 5.4 Context Assembly

```rust
// agcodex-rag/src/context.rs
pub struct ContextAssembler {
    max_context_size: usize,  // Token limit
    prioritizer: ChunkPrioritizer,
}

impl ContextAssembler {
    pub fn assemble_context(
        &self,
        retrieved_chunks: Vec<CodeChunk>,
        query: &str,
    ) -> String {
        let mut chunks = self.prioritizer.prioritize(retrieved_chunks, query);
        let mut context = String::new();
        let mut token_count = 0;
        
        // Always include file summaries first
        for chunk in &chunks {
            if chunk.level == ChunkLevel::File {
                let tokens = count_tokens(&chunk.content);
                if token_count + tokens < self.max_context_size {
                    context.push_str(&format!("=== {} ===\n{}\n\n", 
                        chunk.metadata.file_path.display(),
                        chunk.content
                    ));
                    token_count += tokens;
                }
            }
        }
        
        // Add detailed chunks for relevant files
        for chunk in chunks {
            if chunk.level != ChunkLevel::File {
                let tokens = count_tokens(&chunk.content);
                if token_count + tokens < self.max_context_size {
                    context.push_str(&chunk.content);
                    context.push_str("\n\n");
                    token_count += tokens;
                } else {
                    let compressed = self.compress_chunk(&chunk);
                    context.push_str(&compressed);
                    break;
                }
            }
        }
        
        context
    }
}
```

## Phase 7: AST-Based Edit Tools

### 6.1 Location-Aware Edit Tool

```rust
// agcodex-edit/src/ast_edit.rs
pub struct ASTEditTool {
    parsers: HashMap<Language, Parser>,
    location_map: LocationMap,
}

pub struct ASTPatch {
    // Precise location information
    location: SourceLocation,
    
    // What we're changing
    operation: EditOperation,
    
    // AST context
    target_node: NodeDescriptor,
    parent_context: Option<NodeDescriptor>,
    
    // The actual change
    old_content: String,
    new_content: String,
    
    // Metadata for tracking
    metadata: PatchMetadata,
}

pub struct SourceLocation {
    file_path: PathBuf,
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
    byte_range: Range<usize>,
}

pub enum EditOperation {
    Replace,       // Replace entire node
    Insert,        // Insert new node
    Delete,        // Remove node
    Rename,        // Rename identifier
    Refactor,      // Structural change
    WrapIn,        // Wrap in new structure
    ExtractTo,     // Extract to function/variable
}
```

### 6.2 Enhanced Embedding Metadata

```rust
// agcodex-rag/src/embedding_metadata.rs
pub struct EnhancedEmbedding {
    // The embedding vector
    vector: Vec<f32>,
    
    // Rich metadata for precise location
    metadata: EmbeddingMetadata,
    
    // Link to original AST
    ast_reference: ASTReference,
}

pub struct EmbeddingMetadata {
    // File location
    file_path: PathBuf,
    file_hash: String,
    
    // Precise position in original file
    line_range: Range<usize>,
    column_range: Range<usize>,
    byte_range: Range<usize>,
    
    // What this embedding represents
    content_type: ContentType,
    symbols: Vec<String>,
    
    // Relationships
    parent_chunk_id: Option<ChunkId>,
    child_chunk_ids: Vec<ChunkId>,
    references: Vec<Reference>,
    
    // Context for better understanding
    surrounding_context: SurroundingContext,
}

pub struct SurroundingContext {
    // Lines before and after for context
    before_lines: Vec<String>,
    after_lines: Vec<String>,
    
    // AST context
    parent_node_type: String,
    sibling_nodes: Vec<String>,
    
    // Semantic context
    imports_in_scope: Vec<String>,
    variables_in_scope: Vec<String>,
}
```

### 6.3 Location-Aware Compaction

```rust
// agcodex-rag/src/compactor.rs
pub struct LocationAwareCompactor {
    distill_level: DistillLevel,
    location_tracker: LocationTracker,
}

impl LocationAwareCompactor {
    pub fn compact_with_locations(
        &self,
        file: &ParsedFile,
    ) -> Vec<CompactedChunk> {
        let mut chunks = Vec::new();
        
        for function in &file.functions {
            let compacted = CompactedChunk {
                content: self.compact_function(function),
                original_location: SourceLocation {
                    file_path: file.path.clone(),
                    start_line: function.start_line,
                    start_column: function.start_column,
                    end_line: function.end_line,
                    end_column: function.end_column,
                    byte_range: function.byte_range.clone(),
                },
                compaction_map: self.create_compaction_map(function),
                metadata: ChunkMetadata {
                    original_size: function.content.len(),
                    compacted_size: compacted_content.len(),
                    compression_ratio: calculate_ratio(),
                    preserved_elements: vec![
                        "signature".to_string(),
                        "parameters".to_string(),
                        "return_type".to_string(),
                    ],
                },
            };
            
            chunks.push(compacted);
        }
        
        chunks
    }
}
```

### 6.4 Agent-Friendly Edit Interface

```rust
// agcodex-agent/src/edit_agent.rs
pub struct AgentEditInterface {
    ast_editor: ASTEditTool,
    retriever: LocationAwareRetriever,
    validator: EditValidator,
}

impl AgentEditInterface {
    pub async fn edit_with_context(
        &self,
        instruction: &str,
    ) -> Result<EditPlan> {
        // Step 1: Retrieve relevant context with locations
        let chunks = self.retriever.retrieve_with_locations(
            instruction,
            20,
        ).await?;
        
        // Step 2: Analyze what needs to be edited
        let edit_targets = self.analyze_edit_targets(instruction, &chunks);
        
        // Step 3: Create edit plan with precise locations
        let mut plan = EditPlan::new();
        
        for target in edit_targets {
            let edit = PlannedEdit {
                file: target.location.file_path.clone(),
                line_range: target.location.line_range.clone(),
                operation: self.infer_operation(instruction, &target),
                ast_query: target.edit_context.ast_path.join(" > "),
                confidence: target.relevance_score,
                preview: self.generate_preview(&target),
            };
            
            plan.add_edit(edit);
        }
        
        // Step 4: Validate plan
        self.validator.validate_plan(&plan)?;
        
        Ok(plan)
    }
}
```

## Phase 8: Session Persistence

### 7.1 Efficient Storage Formats

```rust
// agcodex-persistence/src/lib.rs
pub struct AGCodexSessionStore {
    base_dir: PathBuf,  // ~/.agcodex/history
    index: BincodeIndex,
    compressor: ZstdCompressor,
    cache: MemoryMappedCache,
}

pub enum StorageFormat {
    Bincode,      // Metadata, indices
    MessagePack,  // Messages
    Arrow,        // Tabular data
    LanceDB,      // Vector embeddings
}

impl SessionStore {
    pub async fn save_efficient(&self, session: &Session) -> Result<()> {
        // 1. Serialize metadata with bincode
        let meta_bytes = bincode::serialize(&session.metadata)?;
        
        // 2. Serialize messages with MessagePack
        let msg_bytes = rmp_serde::to_vec(&session.messages)?;
        
        // 3. Compress with Zstd
        let compressed_meta = zstd::encode_all(&meta_bytes[..], 3)?;
        let compressed_msgs = zstd::encode_all(&msg_bytes[..], 3)?;
        
        // 4. Write to disk
        let path = self.session_path(session.id);
        let mut file = File::create(path)?;
        
        // Write header
        file.write_all(b"AGCX")?;  // Magic bytes
        file.write_all(&VERSION.to_le_bytes())?;
        
        // Write compressed data with lengths
        file.write_all(&(compressed_meta.len() as u32).to_le_bytes())?;
        file.write_all(&compressed_meta)?;
        file.write_all(&(compressed_msgs.len() as u32).to_le_bytes())?;
        file.write_all(&compressed_msgs)?;
        
        Ok(())
    }
}
```

### 7.2 Fast Session Loading

```rust
// agcodex-persistence/src/loader.rs
pub struct FastSessionLoader {
    cache: Arc<MemoryMappedCache>,
    preload_queue: VecDeque<Uuid>,
}

impl FastSessionLoader {
    pub async fn load_session_lazy(&self, id: Uuid) -> Result<LazySession> {
        // 1. Load metadata from memory-mapped index (instant)
        let metadata = self.cache.get_metadata(id)?;
        
        // 2. Load recent messages only (last 20)
        let recent_messages = self.load_recent_messages(id, 20).await?;
        
        // 3. Return lazy session that loads more on demand
        Ok(LazySession {
            metadata,
            recent_messages,
            full_loader: Box::new(move || self.load_full_session(id)),
        })
    }
}
```

### 7.3 Session Management UI

```rust
// agcodex-tui/src/session_manager.rs
pub struct SessionManager {
    sessions: Vec<SessionInfo>,
    active: Option<Uuid>,
    loader: FastLoader,
    transition: TransitionState,
}

// Keyboard shortcuts
pub const SESSION_KEYS: &[KeyBinding] = &[
    KeyBinding { key: "Ctrl+Tab", action: Action::QuickSwitch },
    KeyBinding { key: "Ctrl+S", action: Action::SessionMenu },
    KeyBinding { key: "Ctrl+N", action: Action::NewSession },
    KeyBinding { key: "Alt+[1-9]", action: Action::SwitchToSession },
];
```

## Phase 9: Configuration System

### 8.1 Main Configuration

```toml
# ~/.agcodex/config.toml
[general]
app_name = "AGCodex"
version = "1.0.0"
default_mode = "build"
reasoning_effort = "high"      # ALWAYS HIGH
verbosity = "high"             # ALWAYS HIGH

[modes]
default = "build"
allow_switching = true
switch_key = "Shift+Tab"

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
ast.languages = "*"  # All languages

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
```

### 8.2 Mode-Specific Configurations

```toml
# ~/.agcodex/modes/plan.toml
[intelligence]
mode = "hard"  # Maximum intelligence for planning
cache_enabled = true
index_on_start = true

[tools]
enabled = ["file_read", "search", "ast_analysis", "doc_generation"]
disabled = ["file_write", "exec", "git"]

[prompt]
suffix = "You are in PLAN MODE. Create detailed plans but do not execute."

# ~/.agcodex/modes/build.toml
[intelligence]
mode = "medium"  # Balanced for execution
cache_enabled = true

[tools]
enabled = "*"  # All tools available

[prompt]
suffix = ""  # No restrictions

# ~/.agcodex/modes/review.toml
[intelligence]
mode = "hard"  # Deep analysis for review
focus = "code_quality"

[tools]
enabled = ["file_read", "search", "ast_analysis", "limited_edit"]
max_edit_size = 10000

[prompt]
suffix = "Focus on code quality, best practices, and potential issues."
```

## Implementation Timeline

### Week 1: Foundation & Core
**Day 1-2: Rebranding & Setup**
- Complete codex → agcodex renaming
- Create ~/.agcodex directory structure
- Set up new Cargo workspace
- Initialize subagent directories

**Day 3: Operating Modes**
- Implement Plan/Build/Review modes
- Add Shift+Tab switching
- Create mode restrictions
- Add mode manager with prompts

**Day 4: Subagent System**
- Implement SubAgentManager
- Create @agent-name parser
- Add context isolation
- Create built-in agents

**Day 5: Tree-sitter Integration**
- Add all 50+ language dependencies
- Implement LanguageRegistry
- Create auto-detection
- Add language-specific parsers

### Week 2: Intelligence Layer
**Day 1: Internal Tools**
- Native fd-find integration
- Native ripgrep integration
- Unified tool interface
- AST-enhanced search

**Day 2: AST-RAG System**
- Implement indexing pipeline
- Create semantic chunker
- Set up LanceDB vectors
- Add hierarchical retrieval

**Day 3: Location-Aware Embeddings**
- Add precise metadata tracking
- Implement compaction mapping
- Create retrieval system
- Add context assembly

**Day 4: AST Edit Tools**
- Build AST-based editor
- Add location tracking
- Create patch validation
- Implement batch edits

**Day 5: Session Persistence**
- Implement Zstd compression
- Add lazy loading
- Create fast switching
- Memory-mapped indices

### Week 3: Polish & Testing
**Day 1: Configuration System**
- Create complete config structure
- Add intelligence modes (light/medium/hard)
- Set HIGH defaults
- Mode-specific configs

**Day 2: TUI Enhancements**
- Add mode indicators
- Visual status bars
- Notification system
- Subagent UI

**Day 3: Error Migration**
- Complete anyhow → thiserror
- Add domain-specific errors
- Improve error recovery
- Add error context

**Day 4: Integration Testing**
- Subagent invocation tests
- Mode switching tests
- AST-RAG retrieval tests
- Session persistence tests

**Day 5: Performance & Optimization**
- Benchmark all components
- Optimize hot paths
- Cache optimization
- Final performance tuning

## Success Metrics

### Performance Targets
- **Mode switch**: <50ms instant switching
- **Language detection**: 100% accuracy
- **File search**: <100ms for 10k files
- **Code search**: <200ms for 1GB codebase
- **AST parsing**: <10ms per file with caching
- **Distillation**: 90%+ compression ratio
- **Session save/load**: <500ms
- **Edit precision**: 100% syntactically valid
- **Location accuracy**: Exact line/column positions
- **Embedding overhead**: <20% additional storage
- **Query latency**: <200ms for semantic search
- **Memory usage**: <2GB for 100k chunks
- **Initial indexing**: 2-5 minutes for 1M lines

### Quality Metrics
- **Error handling**: 0 anyhow uses remaining
- **Test coverage**: >80% for new code
- **Cache hit rate**: >90% for hot paths
- **Accuracy**: 85-95% relevant chunk retrieval

### User Experience
- **Clear mode indication**: Always visible
- **Simple controls**: Shift+Tab for modes
- **Fast feedback**: <16ms frame time
- **Smart defaults**: Works out of box
- **No surprises**: Predictable behavior

## Key Innovations

### 1. Simple Three-Mode System
- **Plan Mode**: Read-only analysis and planning
- **Build Mode**: Full development capabilities
- **Review Mode**: Code quality focus
- All switchable with Shift+Tab

### 2. Sophisticated Subagent System
- **@agent-name invocation**: Direct agent calling in prompts
- **Isolated contexts**: Each agent has separate context
- **Mode overrides**: Agents can enforce specific modes
- **Tool restrictions**: Granular permission control
- **Chaining support**: Sequential and parallel execution

### 3. Comprehensive Language Support
- 50+ languages out of the box
- Auto-detection from file extensions
- Universal code understanding
- Language-specific optimizations

### 4. AST-RAG Architecture
- Hierarchical search (File → Class → Function)
- Smart chunking at semantic boundaries
- Location-aware embeddings

### 5. Precise AST-Based Editing
- Every edit validated by AST
- Exact source location tracking
- Batch edit support

### 6. Efficient Session Persistence
- 90%+ compression with Zstd
- Lazy loading for fast switching
- Memory-mapped indices

### 7. Native Tool Integration
- fd-find and ripgrep built-in
- AST-enhanced search
- Parallel processing

### 8. GPT-5 Best Practices
- Structured prompts with XML tags
- High reasoning and verbosity defaults
- Clear mode-specific instructions

### 9. AI Distiller Approach
- 90-98% code compression
- Preserve essential structure
- Token-efficient context

## Risk Mitigation

1. **Backward Compatibility**: Keep API compatibility during migration
2. **Feature Flags**: Gradual rollout of new features
3. **Extensive Testing**: Each phase thoroughly tested
4. **Performance Monitoring**: Continuous benchmarking
5. **User Documentation**: Clear migration guides
6. **Rollback Plan**: Git tags at each milestone

## Conclusion

This comprehensive plan transforms Codex into AGCodex - a powerful, simple, and efficient AI coding assistant that:
- Supports all major programming languages via tree-sitter
- Features sophisticated subagent system with @agent-name invocation
- Provides precise, AST-based code understanding and editing
- Offers clear operating modes (Plan/Build/Review) for different tasks
- Maintains exact location information (file:line:column) for accurate edits
- Compresses code efficiently (90%+) for AI consumption
- Persists sessions with minimal overhead using Zstd compression
- Integrates native tools (fd-find, ripgrep) for maximum performance

The system is designed to be both powerful for advanced users and simple enough for beginners, with:
- Visual feedback through mode indicators and status bars
- Smart defaults (HIGH reasoning/verbosity for GPT-5)
- Predictable behavior with clear mode restrictions
- Intuitive subagent invocation via @agent-name
- Fast performance meeting all target metrics
- Comprehensive language support out of the box