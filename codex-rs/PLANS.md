# AGCodex Architecture Plans

## Table of Contents
1. [Executive Summary](#executive-summary)
2. [Core Philosophy](#core-philosophy)
3. [Completed Phases](#completed-phases)
4. [Phase 3: AGCodex Subagent System](#phase-3-agcodex-subagent-system)
5. [Phase 9: Independent Embeddings System](#phase-9-independent-embeddings-system)
6. [Success Metrics](#success-metrics)

## Executive Summary

AGCodex is an independent AI coding assistant with comprehensive language support and AST-based intelligence.

### Completed Features
- ✅ **Rebranding**: Complete transformation to AGCodex
- ✅ **Operating Modes**: Plan/Build/Review with scaffolding (needs TUI wiring)
- ✅ **Tree-sitter**: 27 languages with AST intelligence
- ✅ **Internal Tools**: 10 tools with context-aware outputs
- ✅ **Session Persistence**: Zstd compression with efficient storage
- ✅ **Tests**: All 858 tests passing (100% success rate)

### Remaining Work
- 🚧 **TUI Mode Switching**: Wire Shift+Tab handler and ModeIndicator widget
- 📄 **Subagent System**: Implement @agent-name invocation
- 📄 **Independent Embeddings**: Optional multi-provider support

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

## Completed Phases

### Phase 1: Rebranding ✅ Complete
- All crates renamed to agcodex-*
- Binary renamed to agcodex
- Directory structure at ~/.agcodex/
- High defaults for reasoning and verbosity

### Phase 2: Operating Modes ✅ Scaffolded (needs TUI wiring)
- Three modes: Plan (read-only), Build (full access), Review (quality focus)
- ModeManager with restrictions implemented in core/src/modes.rs
- **TODO**: Wire Shift+Tab handler and ModeIndicator widget in TUI

### Phase 4-8: Core Infrastructure ✅ Complete
- **Tree-sitter**: 27 languages with LanguageRegistry
- **Internal Tools**: search, edit, think, plan, glob, tree, grep, bash, index, patch
- **AST-RAG**: Multi-layer search with Tantivy indexing
- **Edit Tools**: AST-aware transformations
- **Session Persistence**: Zstd compression at ~/.agcodex/history

### Phase 10: Configuration ✅ Complete
- High defaults for reasoning_effort and verbosity
- Intelligence modes: Light/Medium/Hard
- TOML-based configuration at ~/.agcodex/config.toml

## Phase 3: AGCodex Subagent System 📄 Ready to Implement

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
  - AST-Search
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

## Phase 9: Independent Embeddings System 📄 Ready to Implement

### 9.1 Core Architecture (Completely Separate from Chat)

```rust
// agcodex-core/src/embeddings/mod.rs
pub struct EmbeddingsManager {
    config: Option<EmbeddingsConfig>,  // None = disabled
    providers: HashMap<String, Box<dyn EmbeddingProvider>>,
}

impl EmbeddingsManager {
    pub fn new(config: Option<EmbeddingsConfig>) -> Self {
        if config.is_none() {
            return Self::disabled();  // Zero overhead
        }
        // Initialize only if explicitly enabled
    }
    
    pub fn is_enabled(&self) -> bool {
        self.config.is_some() && 
        self.config.as_ref().unwrap().enabled
    }
    
    pub async fn embed(&self, text: &str) -> Result<Option<Vec<f32>>> {
        if !self.is_enabled() {
            return Ok(None);  // Gracefully handle disabled state
        }
        // Embedding logic...
    }
}
```

### 9.2 Configuration (Disabled by Default)

```toml
# ~/.agcodex/config.toml

# Default state - embeddings completely disabled
# No [embeddings] section = no embeddings overhead

# To enable embeddings:
[embeddings]
enabled = true  # Must explicitly enable
provider = "auto"  # auto, openai, gemini, voyage

[embeddings.openai]
model = "text-embedding-3-small"
dimensions = 1536
# Uses OPENAI_EMBEDDING_KEY env var

[embeddings.gemini]  
model = "gemini-embedding-001"
dimensions = 768
# Uses GEMINI_API_KEY env var

[embeddings.voyage]
model = "voyage-3.5"
input_type = "document"
# Uses VOYAGE_API_KEY env var
```

### 9.3 Independent Authentication

```rust
// agcodex-core/src/embeddings/auth.rs
pub struct EmbeddingsAuth {
    // Completely separate from chat authentication
    openai_key: Option<String>,
    gemini_key: Option<String>,
    voyage_key: Option<String>,
}

impl EmbeddingsAuth {
    pub fn load() -> Self {
        Self {
            // Check env vars (no AGAGCODEX_ prefix)
            openai_key: env::var("OPENAI_EMBEDDING_KEY").ok(),
            gemini_key: env::var("GEMINI_API_KEY").ok(),
            voyage_key: env::var("VOYAGE_API_KEY").ok(),
        }
    }
}
```

### 9.4 Provider Implementations

```rust
// agcodex-core/src/embeddings/providers/openai.rs
pub struct OpenAIEmbeddings {
    client: reqwest::Client,
    api_key: String,  // From OPENAI_EMBEDDING_KEY
    model: String,
    dimensions: Option<usize>,
}

// Completely independent from OpenAI chat client
// No shared state, no dependencies on chat models
```

### 9.5 Intelligence Mode Mapping (If Embeddings Enabled)

```rust
pub fn select_model_for_intelligence(
    mode: IntelligenceMode,
    provider: &str,
) -> EmbeddingModel {
    match (mode, provider) {
        (IntelligenceMode::Light, "openai") => {
            EmbeddingModel::OpenAI {
                model: "text-embedding-3-small".into(),
                dimensions: Some(256),
            }
        }
        (IntelligenceMode::Medium, "openai") => {
            EmbeddingModel::OpenAI {
                model: "text-embedding-3-small".into(),
                dimensions: Some(1536),
            }
        }
        (IntelligenceMode::Hard, "openai") => {
            EmbeddingModel::OpenAI {
                model: "text-embedding-3-large".into(),
                dimensions: Some(3072),
            }
        }
        // Similar for Gemini and Voyage...
    }
}
```

### 9.6 Context Engine Integration

```rust
// agcodex-core/src/context_engine/mod.rs
pub struct ContextEngine {
    ast_engine: ASTEngine,  // Always available
    embeddings: Option<EmbeddingsManager>,  // Optional
}

impl ContextEngine {
    pub fn search(&self, query: &str) -> SearchResults {
        // AST search always works
        let ast_results = self.ast_engine.search(query);
        
        // Enhance with embeddings if available
        if let Some(embeddings) = &self.embeddings {
            if embeddings.is_enabled() {
                // Hybrid search
                return self.hybrid_search(query, ast_results);
            }
        }
        
        // AST-only search (still very powerful)
        ast_results
    }
}
```

### 9.7 Zero-Overhead When Disabled

```rust
impl EmbeddingsManager {
    pub fn disabled() -> Self {
        Self {
            config: None,
            providers: HashMap::new(),  // Empty, no allocations
        }
    }
    
    // All methods return immediately when disabled
    pub async fn embed(&self, _text: &str) -> Result<Option<Vec<f32>>> {
        if self.config.is_none() {
            return Ok(None);  // Fast path, no work done
        }
        // ...
    }
}
```

### 9.8 Module Structure

```
agcodex-core/src/embeddings/
├── mod.rs                 # Public API
├── config.rs              # Configuration types
├── auth.rs                # Separate authentication
├── providers/
│   ├── mod.rs
│   ├── openai.rs         # OpenAI embeddings
│   ├── gemini.rs         # Gemini embeddings
│   └── voyage.rs         # Voyage AI embeddings
├── cache.rs              # Optional caching
└── manager.rs            # Provider selection
```

## Implementation Priorities

### Priority 1: TUI Mode Switching
- Wire Shift+Tab handler in `tui/src/input.rs`
- Create ModeIndicator widget
- Add visual mode indicators and status colors
- Enforce mode restrictions in tool execution

### Priority 2: Subagent System  
- Implement SubAgentManager with @agent-name invocation
- Create agent configuration loading
- Add isolated context management
- Wire into conversation manager

### Priority 3: Independent Embeddings
- Create embeddings module (disabled by default)
- Implement multi-provider support (OpenAI, Gemini, Voyage)
- Add separate authentication handling
- Optional integration with context engine

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
- AST-based agent tools built-in
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

### 10. Independent Embeddings System
- Completely separate from chat/LLM models
- Disabled by default (zero overhead)
- Multi-provider support (OpenAI, Gemini, Voyage)
- Independent API keys for each provider
- Graceful fallback to AST-only search

## Risk Mitigation

1. **Backward Compatibility**: Keep API compatibility during migration
2. **Feature Flags**: Gradual rollout of new features
3. **Extensive Testing**: Each phase thoroughly tested
4. **Performance Monitoring**: Continuous benchmarking
5. **User Documentation**: Clear migration guides
6. **Rollback Plan**: Git tags at each milestone