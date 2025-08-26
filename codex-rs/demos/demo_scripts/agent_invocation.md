# AGCodex Multi-Agent Invocation Demo Script

## Overview
Demonstrates AGCodex's powerful @agent-name invocation system for specialized task delegation.

## Demo Duration: 4 minutes

---

## Part 1: Introduction (30 seconds)

```bash
# Launch AGCodex with agent panel
$ cargo run --bin agcodex
# Press Ctrl+A to open agent panel
```

**Narration:**
"AGCodex revolutionizes AI coding assistance with specialized agents. Instead of one AI trying to do everything, delegate to experts using simple @agent-name commands."

**Screen shows:**
```
╔═══════════════════════════════════════════════════════════════╗
║  AGCodex - Multi-Agent System Active                         ║
║  Available Agents: 8 specialists ready                       ║
║  Invocation: Use @agent-name in any message                  ║
╚═══════════════════════════════════════════════════════════════╝

Available Specialist Agents:
┌────────────────┬──────────────────────────────────────┐
│ @code-reviewer │ Code quality and best practices     │
│ @refactorer    │ Code restructuring and optimization │
│ @test-writer   │ Comprehensive test generation       │
│ @security      │ Vulnerability detection and fixes   │
│ @performance   │ Performance optimization             │
│ @architect     │ System design and architecture      │
│ @debugger      │ Bug detection and resolution         │
│ @docs          │ Documentation generation             │
└────────────────┴──────────────────────────────────────┘
```

---

## Part 2: Single Agent Invocation (60 seconds)

**User Input:**
```
> @security analyze this authentication module for vulnerabilities
```

**AGCodex Response:**
```
[Security Agent Activated] 🔐
Mode Override: REVIEW (restricted to security analysis)
Analyzing: auth_module.rs

═══════════════════════════════════════════════════════════════
Security Analysis Report - auth_module.rs
═══════════════════════════════════════════════════════════════

🔴 CRITICAL VULNERABILITIES (2)
────────────────────────────────
1. SQL Injection Risk [Line 45-52]
   let query = format!("SELECT * FROM users WHERE email = '{}'", email);
   
   Impact: Database compromise possible
   CVSS: 9.8 (Critical)
   
   Fix Applied:
   + use sqlx::query!("SELECT * FROM users WHERE email = $1")
   +     .bind(email)
   +     .fetch_optional(&pool)
   +     .await?;

2. Timing Attack in Password Comparison [Line 78]
   if password == stored_password { ... }
   
   Impact: Password enumeration possible
   CVSS: 5.3 (Medium)
   
   Fix Applied:
   + use constant_time_eq::constant_time_eq;
   + if constant_time_eq(password.as_bytes(), stored_password.as_bytes()) { ... }

⚠️ WARNINGS (3)
────────────────
1. Missing rate limiting on login attempts
2. No password complexity requirements  
3. Sessions don't expire

✅ SECURITY STRENGTHS
─────────────────────
• Passwords properly hashed with Argon2id
• HTTPS enforced
• CSRF tokens implemented
• Security headers configured

Compliance Status:
├─ OWASP Top 10: 8/10 covered ✓
├─ CWE/SANS Top 25: 22/25 addressed ✓
└─ PCI DSS: Partial compliance ⚠️

[Security Agent Task Complete]
```

**Narration:**
"The security agent specializes in vulnerability detection, applying OWASP standards and security best practices with laser focus."

---

## Part 3: Multi-Agent Collaboration (90 seconds)

**User Input:**
```
> I need to optimize this data pipeline. @performance analyze it,
  then @refactorer clean it up, and @test-writer add benchmarks
```

**AGCodex Response:**
```
[Multi-Agent Task Detected]
Orchestrating: @performance → @refactorer → @test-writer
─────────────────────────────────────────────────────────

[Performance Agent Starting] ⚡
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Profiling: data_pipeline.py
• Instrumentation points: 47
• Sampling rate: 10,000 Hz
• Duration: 5.2 seconds

Performance Bottlenecks Found:
┌─────────────────────────────────────────────────────┐
│ Function          │ Time │ Calls │ Avg   │ Impact  │
├───────────────────┼──────┼───────┼───────┼─────────┤
│ parse_json()      │ 2.1s │ 10K   │ 210μs │ 40.4%   │
│ validate_schema() │ 1.8s │ 10K   │ 180μs │ 34.6%   │
│ transform_data()  │ 0.9s │ 10K   │ 90μs  │ 17.3%   │
│ write_output()    │ 0.4s │ 1     │ 400ms │ 7.7%    │
└─────────────────────────────────────────────────────┘

Memory Profile:
Peak: 847MB | Average: 423MB | Leaked: 0B

Optimization Opportunities:
1. Use ujson instead of json (3x faster parsing)
2. Implement schema validation caching
3. Batch transformations
4. Use memory-mapped file for output

[Passing context to @refactorer...]

─────────────────────────────────────────────────────────

[Refactorer Agent Starting] 🔧
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Applying performance optimizations...

Refactoring Summary:
📝 data_pipeline.py
├─ Replaced json with ujson (lines 12-45)
├─ Added @lru_cache for schema validation (line 67)
├─ Implemented batch processing (lines 89-124)
├─ Converted to memory-mapped output (lines 145-167)
└─ Extracted magic numbers to constants

Code Quality Improvements:
• Reduced cyclomatic complexity: 15 → 7
• Improved maintainability index: 65 → 89
• Eliminated code duplication: 3 blocks merged
• Added type hints: 100% coverage

Before → After Performance:
• Execution time: 5.2s → 1.3s (4x faster)
• Memory usage: 847MB → 234MB (72% reduction)
• CPU usage: 100% → 45% (better efficiency)

[Passing optimized code to @test-writer...]

─────────────────────────────────────────────────────────

[Test-Writer Agent Starting] 🧪
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Generating comprehensive test suite...

Created: test_data_pipeline_performance.py
```python
import pytest
import pytest_benchmark
from unittest.mock import Mock, patch
import numpy as np

class TestPipelinePerformance:
    """Performance regression test suite"""
    
    @pytest.mark.benchmark(group="parsing")
    def test_parse_json_performance(self, benchmark):
        """Ensure JSON parsing stays under 50μs per record"""
        data = generate_test_json(1000)
        result = benchmark(parse_json, data)
        assert benchmark.stats['mean'] < 0.00005  # 50μs
    
    @pytest.mark.benchmark(group="validation")  
    def test_schema_validation_caching(self, benchmark):
        """Verify schema cache improves performance"""
        schema = load_test_schema()
        data = generate_test_data(1000)
        
        # First run - cache miss
        first_run = benchmark(validate_schema, schema, data[0])
        
        # Subsequent runs - cache hits
        cached_run = benchmark(validate_schema, schema, data[1])
        assert cached_run < first_run * 0.1  # 10x faster
        
    @pytest.mark.memory
    def test_memory_usage(self):
        """Ensure pipeline doesn't exceed memory limits"""
        import tracemalloc
        tracemalloc.start()
        
        process_large_dataset(10000)
        current, peak = tracemalloc.get_traced_memory()
        tracemalloc.stop()
        
        assert peak < 250 * 1024 * 1024  # Under 250MB
```

Test Coverage Report:
├─ Performance benchmarks: 12 tests
├─ Memory leak detection: 3 tests  
├─ Regression guards: 8 tests
├─ Edge cases: 15 tests
└─ Load testing: 5 scenarios

Benchmark Baseline Established:
• Parse JSON: 42μs ± 3μs per record
• Validate: 8μs ± 1μs per record (cached)
• Transform: 31μs ± 5μs per record  
• Total pipeline: 127ms for 1000 records

[Multi-Agent Task Complete]
═══════════════════════════════════════════════════════════════
Total Execution Time: 8.3 seconds
Agents Collaborated: 3
Performance Gain: 4x
Code Quality: A+ (89/100)
═══════════════════════════════════════════════════════════════
```

**Narration:**
"Multiple agents work together seamlessly. Each agent focuses on their specialty while passing context to the next, creating a powerful development pipeline."

---

## Part 4: Custom Agent Creation (60 seconds)

**User Input:**
```
> Show me how to create a custom @api-designer agent
```

**AGCodex Response:**
```
[Agent Configuration Guide]

Creating Custom Agent: @api-designer
Location: ~/.agcodex/agents/api-designer.yaml

```yaml
# api-designer.yaml
name: api-designer
description: "REST API design specialist with OpenAPI focus"
mode_override: plan  # Operates in read-only by default

tools:
  allowed:
    - search
    - tree
    - think
    - plan
  restricted:
    - edit  # Can't modify without explicit permission
    - bash  # No execution needed for design

capabilities:
  - OpenAPI 3.0 specification generation
  - RESTful best practices enforcement
  - API versioning strategies
  - Rate limiting design
  - Authentication/authorization patterns
  - Request/response schema validation
  - Error handling standards
  - HATEOAS compliance

custom_prompt: |
  You are an API design specialist focused on creating robust,
  scalable, and developer-friendly REST APIs.
  
  Your expertise includes:
  - RESTful principles and constraints
  - OpenAPI/Swagger specification
  - HTTP semantics and status codes
  - API versioning strategies
  - Security patterns (OAuth2, JWT, API keys)
  - Rate limiting and throttling
  - Pagination patterns
  - Error response formats
  - HATEOAS and JSON:API standards
  
  When designing APIs, always consider:
  1. Consistency across endpoints
  2. Intuitive resource naming
  3. Proper HTTP method usage
  4. Comprehensive error handling
  5. Future extensibility
  6. Developer experience
  
  Generate OpenAPI specs with examples and schemas.

context_injection:
  - type: file_pattern
    pattern: "**/*api*.{ts,js,py,rs,go}"
  - type: file_pattern
    pattern: "**/openapi.{yaml,yml,json}"
  - type: dependencies
    focus: ["express", "fastapi", "actix-web", "gin"]

validation_rules:
  - Ensure all endpoints have OpenAPI documentation
  - Verify consistent naming conventions
  - Check for proper status code usage
  - Validate request/response schemas
```

Usage Example:
```
> @api-designer create a REST API for a task management system

[API-Designer Agent Activated] 🌐
Generating OpenAPI Specification...

openapi: 3.0.0
info:
  title: Task Management API
  version: 1.0.0
  
paths:
  /api/v1/tasks:
    get:
      summary: List all tasks
      parameters:
        - $ref: '#/components/parameters/pagination'
        - $ref: '#/components/parameters/filtering'
      responses:
        200:
          description: Successful response
          content:
            application/json:
              schema:
                type: object
                properties:
                  data:
                    type: array
                    items:
                      $ref: '#/components/schemas/Task'
                  meta:
                    $ref: '#/components/schemas/PaginationMeta'
...
```
```

**Narration:**
"Create custom agents for your specific needs. Define their capabilities, tools, and behavior through simple YAML configuration."

---

## Part 5: Agent Coordination Dashboard (30 seconds)

**Action:** Press `Ctrl+A` to show agent panel

**Screen shows:**
```
╔═══════════════════════════════════════════════════════════════╗
║  AGCodex Agent Coordination Dashboard                        ║
╠═══════════════════════════════════════════════════════════════╣
║  Active Agents       │ Queue    │ Completed                  ║
║  ──────────────────  │ ──────── │ ───────────                ║
║  @performance  [▓▓▓] │ @docs    │ ✓ @security (2.3s)        ║
║  @refactorer   [▓░░] │ @test    │ ✓ @architect (1.8s)       ║
║                      │          │ ✓ @code-reviewer (3.1s)   ║
╠═══════════════════════════════════════════════════════════════╣
║  Agent Statistics                                             ║
║  ────────────────                                            ║
║  Total Invocations: 47    │ Success Rate: 98.2%             ║
║  Avg Response Time: 2.4s  │ Context Preserved: 100%        ║
║  Parallel Executions: 3   │ Memory Efficient: ✓            ║
╚═══════════════════════════════════════════════════════════════╝
```

---

## Key Takeaways

1. **Specialized Expertise**: Each agent masters specific domains
2. **Simple Invocation**: Just use @agent-name in any message
3. **Seamless Collaboration**: Agents pass context automatically
4. **Custom Agents**: Create your own specialists via YAML
5. **Mode Awareness**: Agents respect and can override modes
6. **Parallel Execution**: Multiple agents can work simultaneously

## Try It Yourself

```bash
# List available agents
$ agcodex agents list

# Create custom agent
$ agcodex agents create my-agent

# Invoke specific agent
$ agcodex exec "@security audit my-code"
```