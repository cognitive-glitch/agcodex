# AGCodex Agent Guide

## Quick Start

AGCodex agents are specialized AI assistants that perform specific development tasks with mode-aware execution and intelligent tool selection.

### Basic Agent Invocation

```bash
# In TUI (primary interface)
@agent-code-reviewer      # Invoke code review agent
@agent-refactorer         # Invoke refactoring agent
@agent-test-writer        # Invoke test generation agent

# With parameters
@agent-security --deep    # Deep security scan
@agent-performance --profile=cpu  # CPU profiling focus
```

### TUI Shortcuts

- **`Ctrl+Shift+A`** - Open agent spawn dialog
- **`Ctrl+A`** - Toggle agent panel (shows active agents)
- **`@`** - Quick agent invocation in chat
- **`Shift+Tab`** - Cycle operating modes (affects agent behavior)

## Agent Types

### Core Development Agents

| Agent | Purpose | Default Mode | Key Tools |
|-------|---------|--------------|-----------|
| `@agent-code-reviewer` | Code quality analysis | Review | AST analysis, security scan |
| `@agent-refactorer` | Code restructuring | Build | AST transform, comby |
| `@agent-debugger` | Bug investigation | Review | Debugger, trace analyzer |
| `@agent-test-writer` | Test generation | Build | Coverage analyzer, test frameworks |
| `@agent-performance` | Performance optimization | Review → Build | Profiler, benchmarks |
| `@agent-security` | Security audit | Review | SAST, dependency scan |
| `@agent-docs` | Documentation | Build | AST search, diagram tools |
| `@agent-architect` | System design | Plan | Diagram generators, analyzers |

### Specialized Agents

| Agent | Purpose | Unique Features |
|-------|---------|-----------------|
| `@agent-migration` | Framework/version migration | Incremental changes, validation |
| `@agent-accessibility` | A11y compliance | WCAG checkers, screen reader sim |
| `@agent-i18n` | Internationalization | String extraction, locale generation |
| `@agent-api-designer` | API design and OpenAPI | Schema generation, validation |
| `@agent-dependency` | Dependency management | Update analysis, vulnerability check |
| `@agent-config` | Configuration audit | Best practices, security checks |

## Agent Invocation Syntax

### Basic Invocation
```
@agent-name [options] [target]
```

### With Options
```
@agent-code-reviewer --severity=high --focus=security src/
@agent-test-writer --framework=jest --coverage=90 lib/
@agent-performance --baseline=main --threshold=10ms
```

### With Context
```
# Provide specific context
@agent-refactorer "Convert callbacks to async/await" src/api/

# With file selection
@agent-security --files="*.rs,*.toml"

# With exclusions
@agent-code-reviewer --exclude="tests/,vendor/"
```

## Agent Parameters

### Common Parameters

| Parameter | Description | Example |
|-----------|-------------|---------|
| `--mode` | Override operating mode | `--mode=review` |
| `--intelligence` | AST analysis level | `--intelligence=hard` |
| `--output` | Output format | `--output=markdown` |
| `--worktree` | Use git worktree | `--worktree=feature-xyz` |
| `--parallel` | Enable parallel execution | `--parallel=4` |
| `--cache` | Use cached AST | `--cache=true` |
| `--verbose` | Detailed output | `--verbose` |

### Agent-Specific Parameters

#### Code Reviewer
```
--severity     # Minimum severity (low|medium|high|critical)
--focus        # Focus area (security|performance|maintainability|all)
--standards    # Coding standards (pep8|airbnb|google|custom)
--metrics      # Include metrics (complexity|coverage|duplication)
```

#### Refactorer
```
--pattern      # Refactoring pattern to apply
--incremental  # Apply changes incrementally
--validation   # Validation command after each change
--preserve     # Preserve specific patterns
```

#### Test Writer
```
--framework    # Test framework (jest|pytest|rspec|junit)
--coverage     # Target coverage percentage
--types        # Test types (unit|integration|e2e|property)
--mocking      # Mocking strategy (auto|manual|none)
```

#### Performance
```
--profile      # Profile type (cpu|memory|io|all)
--baseline     # Baseline for comparison
--threshold    # Performance threshold
--iterations   # Benchmark iterations
```

## Chaining Patterns

### Sequential Execution (`→`)

Execute agents one after another, passing context forward:

```
@agent-architect → @agent-refactorer → @agent-test-writer
```

**Example: Feature Development Pipeline**
```
@agent-requirement-analyzer "Add user authentication"
    → @agent-architect
    → @agent-implementer
    → @agent-test-writer
    → @agent-code-reviewer
```

### Parallel Execution (`+`)

Execute multiple agents simultaneously:

```
@agent-security + @agent-performance + @agent-code-reviewer
```

**Example: Comprehensive Audit**
```
@agent-security --deep
    + @agent-performance --profile=all
    + @agent-accessibility --wcag=AA
    + @agent-docs --check-completeness
```

### Mixed Patterns

Combine sequential and parallel execution:

```
(@agent-analyzer → @agent-planner) 
    → (@agent-impl-1 + @agent-impl-2 + @agent-impl-3)
    → @agent-integrator
    → (@agent-test-writer + @agent-doc-writer)
    → @agent-reviewer
```

### Conditional Chaining

Execute based on conditions:

```
@agent-test-writer 
    → [if coverage < 80%] @agent-test-enhancer
    → [if tests pass] @agent-merger
    → [else] @agent-debugger
```

## Best Practices

### 1. Mode Selection

- **Use Plan mode** for analysis and design without modifications
- **Use Review mode** for quality checks and validation
- **Use Build mode** only when modifications are required
- **Let agents override modes** when they have specific requirements

### 2. Context Management

```
# Good: Provide clear context
@agent-refactorer "Extract authentication logic into middleware" src/routes/

# Bad: Vague instructions
@agent-refactorer "clean up code"
```

### 3. Incremental Changes

```
# Good: Incremental with validation
@agent-refactorer --incremental --validation="cargo test"

# Bad: Big bang changes
@agent-refactorer --all --no-validation
```

### 4. Worktree Isolation

```
# Good: Use worktrees for experiments
@agent-performance --worktree=perf-test --aggressive

# Bad: Experiment on main branch
@agent-performance --aggressive
```

### 5. Parallel Execution

```
# Good: Parallelize independent tasks
@agent-security + @agent-performance + @agent-docs

# Bad: Parallelize dependent tasks
@agent-refactorer + @agent-test-writer  # Tests depend on refactored code
```

### 6. Resource Management

```
# Good: Limit parallel agents based on system resources
@agent-chain --parallel=4 --max-memory=8G

# Bad: Unlimited parallel execution
@agent-chain --parallel=unlimited
```

## Troubleshooting

### Common Issues

#### Agent Not Found
```
Error: Unknown agent 'agent-xyz'
Solution: Check available agents with `@agent-list`
```

#### Mode Restriction
```
Error: Agent requires Build mode but current mode is Plan
Solution: Switch modes with Shift+Tab or use --mode override
```

#### Insufficient Context
```
Error: Agent needs more context to proceed
Solution: Provide specific files or directories as targets
```

#### Tool Missing
```
Error: Required tool 'ast-grep' not available
Solution: Ensure tool dependencies are installed
```

#### Worktree Conflict
```
Error: Worktree already exists
Solution: Delete existing worktree or use different name
```

### Debug Commands

```bash
# Show agent details
@agent-info agent-name

# List available agents
@agent-list

# Show agent execution log
@agent-log agent-name

# Show agent resource usage
@agent-stats

# Kill running agent
@agent-kill agent-id
```

### Performance Debugging

```bash
# Profile agent execution
@agent-profile agent-name

# Show agent cache status
@agent-cache-status

# Clear agent cache
@agent-cache-clear

# Show AST parsing stats
@agent-ast-stats
```

## Performance Tips

### 1. Use Caching

```
# Enable AST caching for repeated operations
@agent-code-reviewer --cache=true

# Preload cache before heavy operations
@agent-cache-warm src/
```

### 2. Optimize Scope

```
# Good: Target specific directories
@agent-security src/api/ src/auth/

# Bad: Scan entire monorepo
@agent-security /
```

### 3. Incremental Processing

```
# Process changes since last run
@agent-code-reviewer --since=last-run

# Process only modified files
@agent-test-writer --changed-only
```

### 4. Parallel Strategies

```
# Split large tasks
@agent-refactorer --split-by=module --parallel=4

# Use work stealing for better load balancing
@agent-chain --scheduler=work-stealing
```

### 5. Resource Limits

```
# Set memory limits
@agent-performance --max-memory=4G

# Set time limits
@agent-security --timeout=10m

# Limit CPU cores
@agent-chain --max-cores=8
```

### 6. Selective Intelligence

```
# Use light intelligence for quick scans
@agent-code-reviewer --intelligence=light --quick

# Use hard intelligence only when needed
@agent-security --intelligence=hard --critical-only
```

## Advanced Features

### Custom Agent Configuration

Create custom agents in `~/.agcodex/agents/`:

```yaml
# ~/.agcodex/agents/my-custom-agent.yaml
name: my-custom-agent
description: Custom agent for specific workflow
mode_override: review
intelligence: hard
tools:
  - ast-search
  - custom-tool
parameters:
  - name: depth
    type: integer
    default: 3
prompt: |
  You are a specialized agent that...
```

### Agent Composition

Create composite agents that orchestrate others:

```yaml
# ~/.agcodex/agents/feature-developer.yaml
name: feature-developer
type: composite
agents:
  - architect: 
      mode: plan
  - implementer:
      mode: build
      parallel: true
  - tester:
      mode: build
  - reviewer:
      mode: review
flow: |
  architect → implementer → tester → reviewer
```

### Agent Scripting

Script complex workflows:

```typescript
// ~/.agcodex/scripts/release.ts
import { Agent, Workspace } from '@agcodex/api';

export async function release(ws: Workspace) {
  // Run tests
  await Agent.spawn('test-runner', { 
    coverage: 95,
    strict: true 
  });
  
  // Security audit
  const security = await Agent.spawn('security', {
    deep: true,
    autoFix: true
  });
  
  if (security.criticalIssues > 0) {
    throw new Error('Critical security issues found');
  }
  
  // Performance validation
  await Agent.spawn('performance', {
    baseline: 'v1.0.0',
    threshold: '10%'
  });
  
  // Generate changelog
  await Agent.spawn('changelog', {
    since: 'last-release'
  });
  
  // Create release
  await Agent.spawn('release', {
    version: 'minor',
    sign: true
  });
}
```

## Integration with External Tools

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
      - uses: agcodex/action@v1
        with:
          agents: |
            @agent-code-reviewer --severity=medium
            @agent-security --quick
            @agent-test-writer --changed-only
```

### Git Hooks

```bash
# .git/hooks/pre-commit
#!/bin/bash
agcodex exec "@agent-code-reviewer --changed-only --quick"
```

### IDE Integration

```json
// .vscode/settings.json
{
  "agcodex.agents.onSave": [
    "@agent-formatter",
    "@agent-linter --fix"
  ],
  "agcodex.agents.onTest": [
    "@agent-test-writer --missing-only"
  ]
}
```

## Monitoring and Observability

### Agent Metrics

```bash
# View real-time metrics
@agent-metrics --live

# Export metrics
@agent-metrics --export=prometheus

# Set up alerts
@agent-alert --threshold="duration>5m" --notify=slack
```

### Execution Traces

```bash
# Enable tracing
@agent-trace --enable

# View trace
@agent-trace --view=latest

# Export trace
@agent-trace --export=jaeger
```

### Audit Logs

```bash
# View audit log
@agent-audit --since=1h

# Search audit log
@agent-audit --filter="mode=build"

# Export audit log
@agent-audit --export=json > audit.json
```