# Advanced Features Guide

Unlock the full potential of AGCodex with advanced workflows, custom configurations, and powerful integrations.

## 📖 Table of Contents
1. [Custom Agent Configuration](#custom-agent-configuration)
2. [AST Compression Levels](#ast-compression-levels)
3. [MCP Server Integration](#mcp-server-integration)
4. [Git Worktree Workflows](#git-worktree-workflows)
5. [Session Branching & Timeline](#session-branching--timeline)
6. [Embedding Configuration](#embedding-configuration)
7. [Performance Optimization](#performance-optimization)
8. [Advanced Tool Usage](#advanced-tool-usage)

## Custom Agent Configuration

### Creating a Custom Agent

Custom agents allow you to create specialized AI assistants tailored to your specific needs.

**Basic Agent Definition** (`~/.agcodex/agents/frontend-expert.yaml`):
```yaml
name: frontend-expert
description: "Specialized agent for modern frontend development"
version: "1.0.0"

# Mode enforcement
mode_override: build  # Can be: plan, build, or review

# Intelligence level (affects AST analysis depth)
intelligence: hard  # Options: light, medium, hard

# Available tools
tools:
  - search      # Multi-layer search engine
  - edit        # Code editing
  - tree        # AST parsing
  - grep        # Pattern matching
  - index       # Tantivy indexing
  - patch       # AST transformations

# Custom prompts
prompts:
  system: |
    You are a frontend development expert specializing in:
    - React, Vue, and Angular frameworks
    - TypeScript and modern JavaScript
    - CSS-in-JS and responsive design
    - Performance optimization
    - Accessibility (WCAG compliance)
    - Testing with Jest and Cypress
    
  analysis: |
    When analyzing frontend code:
    1. Check for React hooks violations
    2. Identify performance bottlenecks
    3. Ensure accessibility compliance
    4. Validate TypeScript types
    5. Check for CSS conflicts

# Behavioral configuration
behavior:
  auto_format: true
  validate_syntax: true
  run_tests_after_edit: true
  
# Context preservation
context:
  max_history: 50
  preserve_workspace: true
  track_dependencies: true

# Restrictions (optional)
restrictions:
  max_file_size: 100000  # bytes
  allowed_extensions: [".ts", ".tsx", ".js", ".jsx", ".css", ".scss"]
  forbidden_paths: ["node_modules/", "dist/", "build/"]
```

**Advanced Agent with Capabilities** (`~/.agcodex/agents/api-architect.yaml`):
```yaml
name: api-architect
description: "API design and implementation expert"
version: "2.0.0"

# Advanced configuration
capabilities:
  - name: openapi-generation
    description: "Generate OpenAPI specifications"
    command: "generate-openapi"
    
  - name: graphql-schema
    description: "Design GraphQL schemas"
    command: "design-graphql"
    
  - name: rest-compliance
    description: "Validate RESTful principles"
    command: "validate-rest"

# Tool configurations
tool_configs:
  search:
    index_type: "semantic"  # Use embeddings
    chunk_size: 512
    overlap: 50
    
  tree:
    languages: ["typescript", "python", "rust"]
    max_depth: 10
    include_comments: false

# Workflow templates
workflows:
  api_design:
    steps:
      - analyze_requirements
      - design_endpoints
      - generate_openapi
      - create_tests
      - implement_handlers
      
  api_refactor:
    steps:
      - analyze_existing
      - identify_improvements
      - create_migration_plan
      - refactor_incrementally
      - validate_compatibility

# Integration with external tools
integrations:
  postman:
    enabled: true
    auto_sync: true
    
  swagger:
    enabled: true
    ui_port: 8080
```

### Agent Invocation with Options

```bash
# Basic invocation
> @frontend-expert analyze the React components

# With specific configuration
> @api-architect --workflow=api_design create user management API

# Override intelligence level
> @frontend-expert --intelligence=light quick review of App.tsx

# With context from another agent
> @code-reviewer analyze src/ | @frontend-expert focus on UI issues
```

## AST Compression Levels

AGCodex uses sophisticated AST (Abstract Syntax Tree) compression for efficient code analysis.

### Understanding Compression Levels

**Light (70% compression)**
```toml
[context_engine]
intelligence_mode = "light"
compression_level = 70
chunk_size = 256
indexing = "on_demand"
```

Best for:
- Small projects (<10k LOC)
- Quick iterations
- Real-time analysis
- Limited memory systems

Example output:
```
Function: calculate_total
  Parameters: items, tax_rate
  Returns: float
  Complexity: 5
  Lines: 15-28
```

**Medium (85% compression)** - Default
```toml
[context_engine]
intelligence_mode = "medium"
compression_level = 85
chunk_size = 512
indexing = "background"
```

Best for:
- Medium projects (10k-100k LOC)
- Balanced performance
- Most use cases
- Standard development

Example output:
```
Function: calculate_total
  Parameters: items: List[Item], tax_rate: float = 0.08
  Returns: float
  Complexity: 5
  Calls: [sum_items, apply_tax]
  Lines: 15-28
  Context: Part of OrderProcessor class
```

**Hard (95% compression)**
```toml
[context_engine]
intelligence_mode = "hard"
compression_level = 95
chunk_size = 1024
indexing = "aggressive"
include_call_graph = true
include_data_flow = true
```

Best for:
- Large projects (>100k LOC)
- Deep analysis
- Architecture understanding
- Complex refactoring

Example output:
```
Function: OrderProcessor.calculate_total
  Signature: (items: List[Item], tax_rate: float = 0.08) -> float
  Complexity: Cyclomatic=5, Cognitive=8
  Call Graph:
    → sum_items() [line 18]
    → apply_tax() [line 22]
    → round_currency() [line 25]
  Data Flow:
    items → filtered_items → subtotal → total
  Side Effects: None
  Test Coverage: 87%
  Performance: O(n) time, O(1) space
  Context: 
    Class: OrderProcessor
    Module: billing.processors
    Dependencies: [Item, TaxCalculator]
```

### Configuring Compression

```bash
# Check current compression
> show context config

Current configuration:
  Intelligence: medium
  Compression: 85%
  Chunks indexed: 45,231
  Cache size: 124 MB
  
# Change compression level
> set compression hard

Reindexing with 95% compression...
  ✓ Analyzed 1,247 files
  ✓ Created 67,891 chunks
  ✓ Built call graph
  ✓ Indexed relationships
  
New stats:
  Compression: 95%
  Index size: 89 MB (-28%)
  Query speed: 2.3ms (+0.8ms)
  Accuracy: 94% (+7%)
```

## MCP Server Integration

Model Context Protocol (MCP) allows AGCodex to connect with external tools and services.

### Setting Up MCP Servers

**Configuration** (`~/.agcodex/config.toml`):
```toml
[[mcp_servers]]
name = "github"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
env = { GITHUB_TOKEN = "${GITHUB_TOKEN}" }

[[mcp_servers]]
name = "postgres"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-postgres"]
env = { DATABASE_URL = "${DATABASE_URL}" }

[[mcp_servers]]
name = "filesystem"  
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/workspace"]
```

### Using MCP Tools

```bash
# List available MCP tools
> list mcp tools

Available MCP tools:
  github:
    - create_issue
    - search_repos
    - get_pr_diff
    
  postgres:
    - query
    - schema
    - explain
    
  filesystem:
    - read_file
    - write_file
    - list_directory

# Use MCP tool
> mcp github:create_issue title="Bug in auth" body="Details..."

Issue created: #234

# Chain MCP tools
> mcp postgres:schema users | analyze for optimization opportunities
```

### Creating Custom MCP Server

```javascript
// custom-mcp-server.js
import { Server } from '@modelcontextprotocol/sdk';

const server = new Server({
  name: 'custom-tool',
  version: '1.0.0',
});

server.setRequestHandler('tools/list', async () => ({
  tools: [{
    name: 'analyze_metrics',
    description: 'Analyze application metrics',
    inputSchema: {
      type: 'object',
      properties: {
        metric: { type: 'string' },
        timeRange: { type: 'string' }
      }
    }
  }]
}));

server.setRequestHandler('tools/call', async (request) => {
  // Implementation
  return { result: 'Analysis complete' };
});

server.start();
```

## Git Worktree Workflows

AGCodex integrates with Git worktrees for isolated development.

### Parallel Development

```bash
# Create worktree for feature
> @architect design payment system in worktree feature/payments

Creating worktree at .worktrees/feature-payments...
  ✓ Worktree created
  ✓ Switched context
  
Designing in isolated environment...
  [Architecture work happens here]
  
# Work on another feature in parallel
> @refactorer optimize database in worktree perf/db-optimization

Creating worktree at .worktrees/perf-db-optimization...
  ✓ Parallel development enabled

# List active worktrees
> show worktrees

Active worktrees:
  1. feature/payments (2 agents working)
  2. perf/db-optimization (1 agent working)
  3. main (base)
```

### Worktree Agent Coordination

```bash
# Assign agents to different worktrees
> parallel execution:
    @frontend-expert: worktree ui/redesign
    @api-architect: worktree api/v2
    @test-writer: worktree tests/integration

Orchestrating parallel development...

[Progress Dashboard]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
ui/redesign      ████████░░ 80%
api/v2           ██████░░░░ 60%  
tests/integration ████████░ 90%
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

# Merge worktrees when ready
> merge worktree ui/redesign to main

Merging changes...
  ✓ No conflicts
  ✓ Tests passing
  ✓ Merged successfully
```

## Session Branching & Timeline

Advanced session management for experimental changes and time travel.

### Session Branching

```bash
# Current session
> show session

Session: main-development
  Messages: 156
  Duration: 2h 34m
  Mode changes: 5
  
# Create branch for experiment
> branch session as experiment-1

Created session branch: experiment-1
  ✓ Full context preserved
  ✓ Independent timeline
  
# Try risky changes
> @refactorer aggressive optimization of entire codebase

[Experimental changes applied]

# If unhappy, switch back
> switch session main-development

Restored to main-development
All experimental changes isolated in experiment-1

# Or merge if successful
> merge session experiment-1

Merging experimental changes...
  ✓ 34 improvements applied
  ✓ Session timelines merged
```

### Timeline Navigation

```bash
# View session timeline
> show timeline

Session Timeline:
═══════════════════════════════════════
[09:00] Started session
[09:15] Switched to PLAN mode
[09:23] Message 1-10: Analysis phase
[09:45] Switched to BUILD mode
[10:02] Message 11-45: Implementation ← Current
[10:34] Created checkpoint
[10:45] Message 46-78: Testing
[11:15] Branch created: experiment-1
[11:30] Current position

# Jump to specific point
> jump to checkpoint 10:34

Restored to checkpoint:
  ✓ Context restored
  ✓ File states reverted
  ✓ Mode: BUILD
  
# Navigate by message
> jump to message 25

Jumped to message 25
Context: "Implementing user authentication"

# Replay from point
> replay from message 25 with different approach

Replaying with modifications...
  [Alternative implementation applied]
```

### Advanced Timeline Features

```bash
# Create named checkpoints
> checkpoint "before major refactor"

Checkpoint created: before-major-refactor

# Compare timelines
> compare timeline with experiment-1

Timeline Comparison:
  Main:        156 messages, 2h 34m
  Experiment:  178 messages, 2h 51m
  
  Divergence point: Message 115
  Common messages: 114
  Unique to main: 42
  Unique to experiment: 64

# Timeline search
> search timeline for "database migration"

Found in timeline:
  Message 45: Planning database migration
  Message 67: Implementing migration
  Message 89: Testing migration
  
# Export timeline
> export timeline as markdown

Timeline exported to: session-timeline-2024-01-15.md
```

## Embedding Configuration

Configure semantic search with various embedding providers.

### Multi-Provider Setup

```toml
# ~/.agcodex/config.toml

[embeddings]
enabled = true
default_provider = "openai"

[embeddings.providers.openai]
api_key = "${OPENAI_API_KEY}"
model = "text-embedding-3-large"
dimensions = 3072
batch_size = 100

[embeddings.providers.gemini]
api_key = "${GEMINI_API_KEY}"
model = "embedding-001"
dimensions = 768
batch_size = 50

[embeddings.providers.voyage]
api_key = "${VOYAGE_API_KEY}"
model = "voyage-code-2"
dimensions = 1536
batch_size = 128
```

### Semantic Search Examples

```bash
# Enable semantic search
> enable embeddings

Initializing embeddings...
  Provider: OpenAI
  Model: text-embedding-3-large
  Indexing codebase...
  ✓ 12,456 chunks embedded

# Semantic code search
> search similar to "user authentication flow"

Semantic search results (similarity score):
  1. auth/login_handler.rs (0.94)
  2. auth/session_manager.rs (0.91)
  3. middleware/auth_check.rs (0.89)
  4. auth/jwt_validator.rs (0.87)
  5. models/user.rs (0.82)

# Find similar implementations
> find code similar to this pattern:
```rust
async fn process_data(input: Vec<T>) -> Result<Output> {
    validate(input)?;
    transform(input).await?;
    save_to_db().await
}
```

Found similar patterns:
  1. handlers/order_processor.rs:45 (0.92)
  2. services/data_pipeline.rs:78 (0.88)
  3. workers/batch_job.rs:123 (0.85)
```

### Embedding Performance Tuning

```bash
# Check embedding stats
> show embedding stats

Embedding Statistics:
  Total chunks: 12,456
  Embedded: 12,456 (100%)
  Index size: 487 MB
  Average query time: 23ms
  Cache hit rate: 87%

# Optimize embeddings
> optimize embeddings

Optimization process:
  ✓ Removing duplicate chunks
  ✓ Recomputing stale embeddings
  ✓ Compacting index
  ✓ Building KNN graph
  
Results:
  Index size: 412 MB (-15%)
  Query time: 18ms (-22%)
  Accuracy maintained: 94%
```

## Performance Optimization

### Profiling and Benchmarking

```bash
# Profile AGCodex performance
> profile performance

Performance Profile:
═══════════════════════════════════
Component          Time    Memory
─────────────────────────────────
AST Parsing        45ms    124MB
Search Index       12ms    89MB
Embeddings        23ms    412MB
Agent Execution   230ms    67MB
UI Rendering       8ms     34MB
─────────────────────────────────
Total             318ms   726MB

Bottlenecks identified:
  1. Agent execution (72% of time)
  2. Embedding search (7% of time)

# Run benchmarks
> benchmark search performance

Running search benchmarks...
  Simple search: 2.3ms (10,000 ops)
  Regex search: 8.7ms (5,000 ops)
  AST search: 12.4ms (3,000 ops)
  Semantic search: 23.1ms (1,000 ops)
  
Performance grade: A- (87/100)
```

### Cache Configuration

```toml
[cache]
enabled = true
memory_limit = "2GB"
disk_limit = "10GB"
ttl = 3600  # seconds

[cache.strategies]
ast_cache = "lru"  # Least Recently Used
search_cache = "lfu"  # Least Frequently Used
embedding_cache = "arc"  # Adaptive Replacement Cache
```

### Parallel Processing

```bash
# Configure parallelism
> set parallelism 8

Parallelism set to 8 threads
  ✓ Agent execution: 4 threads
  ✓ Search indexing: 2 threads
  ✓ Background tasks: 2 threads

# Test parallel performance
> benchmark parallel vs sequential

Test: Process 1000 files
  Sequential: 45.2s
  Parallel (8): 8.7s
  Speedup: 5.2x
```

## Advanced Tool Usage

### Complex Search Patterns

```bash
# AST-based search with YAML rules
> search with rule:
```yaml
id: find-complex-functions
language: rust
rule:
  pattern: |
    fn $NAME($$$ARGS) -> $RET {
      $$$BODY
    }
  where:
    - inside:
        kind: impl_item
    - has:
        pattern: "async"
  filters:
    - complexity: ">10"
```

Found 23 complex async methods in impl blocks

# Multi-layer search
> search "error handling" using:
    layer1: symbol index
    layer2: tantivy search  
    layer3: AST analysis
    layer4: semantic similarity

Aggregated results from all layers:
  High confidence (all layers): 45 matches
  Medium confidence (3 layers): 67 matches
  Low confidence (2 layers): 123 matches
```

### Advanced Edit Operations

```bash
# AST-aware transformations
> transform all matches:
    pattern: "println!($MSG)"
    to: "log::debug!($MSG)"
    where: "in test modules"
    
Transforming...
  ✓ 67 transformations in 12 files
  ✓ AST validity maintained
  ✓ Imports added where needed

# Batch refactoring with validation
> refactor with validation:
    1. Extract method from lines 45-89
    2. Rename to "process_user_data"
    3. Add return type annotation
    4. Update all callers
    5. Run tests after each step
    
Executing refactoring pipeline...
  Step 1: ✓ Method extracted
  Step 2: ✓ Renamed
  Step 3: ✓ Type added
  Step 4: ✓ 8 callers updated
  Step 5: ✓ All tests passing
```

### Custom Tool Chains

```bash
# Create tool chain
> define toolchain "full-analysis":
    1. tree: parse all files
    2. grep: find patterns
    3. search: deep analysis
    4. think: reasoning
    5. plan: action items

# Execute toolchain
> run toolchain "full-analysis" on src/

Executing full-analysis toolchain...
  [1/5] Tree parsing: 1,247 files
  [2/5] Pattern matching: 3,456 matches
  [3/5] Deep analysis: 89 insights
  [4/5] Reasoning: 15 recommendations
  [5/5] Action plan: 8 priority items

Full analysis complete. Report saved.
```

## Integration Examples

### CI/CD Integration

```yaml
# .github/workflows/agcodex.yml
name: AGCodex Analysis

on: [pull_request]

jobs:
  analyze:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Run AGCodex Security Audit
        run: |
          agcodex exec "@security audit --fail-on=high"
          
      - name: Run AGCodex Code Review
        run: |
          agcodex exec "@code-reviewer analyze --min-score=7"
          
      - name: Check Test Coverage
        run: |
          agcodex exec "@test-writer coverage --min=80"
```

### IDE Integration

```json
// .vscode/settings.json
{
  "agcodex.enabled": true,
  "agcodex.mode": "review",
  "agcodex.autoSave": true,
  "agcodex.agents": {
    "onSave": ["@code-reviewer", "@test-writer"],
    "onCommit": ["@security", "@performance"]
  }
}
```

## Best Practices

### 1. Performance Optimization
- Use appropriate compression levels
- Enable caching for repeated operations
- Configure parallelism based on system resources
- Profile regularly to identify bottlenecks

### 2. Agent Configuration
- Start with built-in agents, customize as needed
- Use mode overrides wisely
- Configure intelligence levels based on task complexity
- Chain agents for complex workflows

### 3. Session Management
- Create checkpoints before major changes
- Use branching for experiments
- Name checkpoints descriptively
- Export important timelines

### 4. Search Optimization
- Use specific file patterns
- Leverage AST search for structural queries
- Enable embeddings for semantic search
- Combine search layers for best results

## Troubleshooting

### High Memory Usage
```bash
# Check memory usage
> show memory stats

# Reduce cache size
> set cache memory_limit 1GB

# Use lighter compression
> set compression light
```

### Slow Search Performance
```bash
# Rebuild search index
> rebuild index

# Check index statistics  
> show index stats

# Optimize for specific patterns
> optimize index for "*.rs"
```

### Agent Timeout Issues
```bash
# Increase timeout
> set agent timeout 300s

# Use incremental processing
> @agent --incremental --batch-size=10
```

## Next Steps

1. Review [Configuration Templates](configuration_templates/) for quick setup
2. Explore [Custom Agents](custom_agents/) for specialized workflows  
3. Check [Basic Usage](basic_usage.md) for fundamentals
4. Read [Agent Workflows](agent_workflows.md) for multi-agent patterns

---

**Advanced Command Reference:**
- `set <option> <value>` - Configure settings
- `show <component> stats` - View statistics
- `profile <component>` - Performance profiling
- `optimize <component>` - Run optimizations
- `export <data> as <format>` - Export data
- `benchmark <operation>` - Run benchmarks