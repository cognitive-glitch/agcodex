# AGCodex Agent Cookbook

A collection of practical recipes for common development workflows using AGCodex agents.

## Table of Contents

1. [Full Codebase Audit](#recipe-full-codebase-audit)
2. [Legacy Code Modernization](#recipe-legacy-code-modernization)
3. [Test Coverage Improvement](#recipe-test-coverage-improvement)
4. [Security Hardening](#recipe-security-hardening)
5. [Performance Tuning](#recipe-performance-tuning)
6. [Documentation Sprint](#recipe-documentation-sprint)
7. [Bug Hunt Workflow](#recipe-bug-hunt-workflow)
8. [API Redesign](#recipe-api-redesign)
9. [Microservices Extraction](#recipe-microservices-extraction)
10. [Technical Debt Reduction](#recipe-technical-debt-reduction)

---

## Recipe: Full Codebase Audit

**Goal**: Comprehensive analysis of code quality, security, performance, and maintainability.

**Time Estimate**: 30-60 minutes for medium codebase

**Mode Requirements**: Review (read-only analysis)

### Steps

```bash
# 1. Start with high-level architecture analysis
@agent-architect --analyze --output=diagrams/

# 2. Run parallel comprehensive analysis
@agent-code-reviewer --deep --metrics=all \
  + @agent-security --owasp --cwe --dependencies \
  + @agent-performance --profile=all --bottlenecks \
  + @agent-docs --coverage --completeness

# 3. Check technical debt
@agent-debt-analyzer --include-refactoring-cost

# 4. Generate consolidated report
@agent-reporter --aggregate --format=html --output=audit-report.html
```

### Expected Output

```
📊 Codebase Audit Summary
├── Code Quality: B+ (82/100)
│   ├── Complexity: 8.2 avg (✓ Good)
│   ├── Duplication: 3.1% (✓ Excellent)
│   ├── Test Coverage: 67% (⚠ Needs improvement)
│   └── Documentation: 45% (⚠ Below target)
├── Security: A- (0 critical, 2 high, 8 medium)
├── Performance: B (3 bottlenecks identified)
└── Technical Debt: 120 hours estimated

Generated: audit-report.html (interactive dashboard)
```

### Tips

- Run during off-hours for large codebases
- Use `--cache=true` for faster subsequent runs
- Set up weekly automated audits in CI/CD
- Focus on critical paths with `--critical-only` for quick audits

---

## Recipe: Legacy Code Modernization

**Goal**: Systematically modernize legacy code while maintaining functionality.

**Time Estimate**: 2-4 hours per module

**Mode Requirements**: Plan → Build → Review

### Steps

```bash
# 1. Analyze current state (Plan mode)
Shift+Tab # Switch to Plan mode
@agent-legacy-analyzer --language-version --patterns --dependencies

# 2. Create modernization plan
@agent-modernization-planner \
  --target-version="latest" \
  --risk-assessment \
  --incremental

# 3. Set up safety net (Review mode)
@agent-test-writer --characterization --golden-master
@agent-snapshot --create-baseline

# 4. Apply modernizations incrementally (Build mode)
Shift+Tab # Switch to Build mode
@agent-modernizer \
  --step-by-step \
  --validation="npm test" \
  --rollback-on-failure

# 5. Specific modernizations
@agent-refactorer --pattern="callback-to-promise" --incremental
@agent-refactorer --pattern="class-to-hooks" --preserve-behavior
@agent-dependency-updater --strategy=conservative

# 6. Validate each step (Review mode)
Shift+Tab # Switch to Review mode
@agent-regression-tester --compare-baseline
@agent-performance --compare-baseline
```

### Example: Modernizing JavaScript Code

```javascript
// Before (Legacy)
var that = this;
function getData(callback) {
  $.ajax({
    url: '/api/data',
    success: function(data) {
      that.processData(data, callback);
    }
  });
}

// After (Modernized by agents)
async function getData() {
  const response = await fetch('/api/data');
  const data = await response.json();
  return this.processData(data);
}
```

### Tips

- Always create characterization tests first
- Use git worktrees for safe experimentation
- Modernize in small, verifiable increments
- Keep performance benchmarks throughout

---

## Recipe: Test Coverage Improvement

**Goal**: Increase test coverage to 80%+ with meaningful tests.

**Time Estimate**: 1-2 hours per module

**Mode Requirements**: Review → Build

### Steps

```bash
# 1. Analyze current coverage (Review mode)
@agent-coverage-analyzer \
  --detailed \
  --identify-critical-gaps \
  --output=coverage-report.html

# 2. Prioritize untested code
@agent-test-prioritizer \
  --risk-based \
  --complexity-weighted \
  --usage-frequency

# 3. Generate tests for critical paths (Build mode)
Shift+Tab # Switch to Build mode
@agent-test-writer \
  --target-coverage=80 \
  --focus="critical-paths" \
  --types="unit,integration" \
  --framework=auto

# 4. Generate edge case tests
@agent-test-writer \
  --edge-cases \
  --boundary-values \
  --error-conditions \
  --property-based

# 5. Add missing integration tests
@agent-test-writer \
  --integration \
  --api-contracts \
  --database-transactions

# 6. Validate test quality
@agent-test-validator \
  --mutation-testing \
  --determinism-check \
  --speed-check
```

### Coverage Improvement Tracking

```
Initial Coverage: 45%
├── Unit Tests: 52%
├── Integration: 20%
└── E2E: 15%

After Phase 1 (+2 hours):
├── Unit Tests: 75% (+23%)
├── Integration: 20%
└── E2E: 15%

After Phase 2 (+1 hour):
├── Unit Tests: 75%
├── Integration: 65% (+45%)
└── E2E: 15%

Final Coverage: 82%
├── Unit Tests: 85%
├── Integration: 78%
└── E2E: 40%
```

### Tips

- Focus on business-critical code first
- Use mutation testing to validate test effectiveness
- Don't aim for 100% - focus on meaningful coverage
- Maintain test execution speed under 30 seconds

---

## Recipe: Security Hardening

**Goal**: Identify and fix security vulnerabilities, implement best practices.

**Time Estimate**: 2-3 hours

**Mode Requirements**: Review → Build

### Steps

```bash
# 1. Deep security scan (Review mode)
@agent-security \
  --deep \
  --owasp-top-10 \
  --cwe-top-25 \
  --known-vulnerabilities

# 2. Scan for secrets and credentials
@agent-secrets-scanner \
  --history \
  --patterns="custom-patterns.yaml"

# 3. Dependency vulnerability check
@agent-dependency-scanner \
  --include-transitive \
  --severity=medium

# 4. Apply automated fixes (Build mode)
Shift+Tab # Switch to Build mode
@agent-security-fixer \
  --auto-patch \
  --validate-fixes \
  --test-after-patch

# 5. Implement security headers
@agent-security-headers \
  --framework=auto \
  --strict

# 6. Add input validation
@agent-validator \
  --add-sanitization \
  --sql-injection \
  --xss \
  --command-injection

# 7. Implement authentication hardening
@agent-auth-hardener \
  --mfa \
  --session-management \
  --password-policy
```

### Security Checklist Output

```
✅ Security Hardening Complete
├── Vulnerabilities Fixed: 15
│   ├── Critical: 2 (SQL injection, RCE)
│   ├── High: 5 (XSS, path traversal)
│   └── Medium: 8 (information disclosure)
├── Dependencies Updated: 23
├── Security Headers: 12/12 implemented
├── Input Validation: 100% coverage
├── Authentication: MFA enabled
└── Secrets: 0 exposed (3 removed from history)
```

### Tips

- Always review security fixes before applying
- Test thoroughly after security patches
- Set up continuous security monitoring
- Document security decisions in ADRs

---

## Recipe: Performance Tuning

**Goal**: Identify and resolve performance bottlenecks, optimize critical paths.

**Time Estimate**: 3-4 hours

**Mode Requirements**: Review → Build → Review

### Steps

```bash
# 1. Baseline performance (Review mode)
@agent-performance \
  --profile=all \
  --duration=5m \
  --save-baseline

# 2. Identify bottlenecks
@agent-bottleneck-finder \
  --cpu \
  --memory \
  --io \
  --network

# 3. Analyze algorithmic complexity
@agent-complexity-analyzer \
  --identify-n-squared \
  --recursive-depth \
  --loop-analysis

# 4. Generate optimization strategies (Plan mode)
Shift+Tab # Switch to Plan mode
@agent-optimization-strategist \
  --cost-benefit \
  --risk-assessment \
  --estimated-improvement

# 5. Apply optimizations (Build mode)
Shift+Tab # Switch to Build mode

# 5a. Algorithm improvements
@agent-algorithm-optimizer \
  --replace-inefficient \
  --parallel-opportunities

# 5b. Caching implementation
@agent-cache-implementer \
  --redis \
  --memory \
  --http-cache

# 5c. Database optimization
@agent-db-optimizer \
  --indexes \
  --query-optimization \
  --connection-pooling

# 5d. Code optimization
@agent-code-optimizer \
  --lazy-loading \
  --memoization \
  --debouncing

# 6. Validate improvements (Review mode)
Shift+Tab # Switch to Review mode
@agent-performance \
  --compare-baseline \
  --regression-check
```

### Performance Results Example

```
🚀 Performance Optimization Results
├── Response Time: -65% (450ms → 157ms)
├── Memory Usage: -40% (512MB → 307MB)
├── CPU Usage: -30% (45% → 31%)
└── Throughput: +180% (1000 req/s → 2800 req/s)

Optimizations Applied:
✓ Replaced O(n²) sort with O(n log n)
✓ Implemented Redis caching (95% hit rate)
✓ Added database indexes (3 new indexes)
✓ Enabled connection pooling (50 connections)
✓ Lazy loaded 15 components
✓ Debounced 8 event handlers
```

### Tips

- Always measure before and after
- Focus on the critical path first
- Consider trade-offs (memory vs CPU)
- Document performance SLAs

---

## Recipe: Documentation Sprint

**Goal**: Generate comprehensive, up-to-date documentation.

**Time Estimate**: 2-3 hours

**Mode Requirements**: Plan → Build

### Steps

```bash
# 1. Analyze documentation gaps (Plan mode)
@agent-docs-analyzer \
  --coverage \
  --outdated \
  --missing-examples

# 2. Generate API documentation (Build mode)
Shift+Tab # Switch to Build mode
@agent-api-documenter \
  --openapi \
  --examples \
  --playground

# 3. Create architecture documentation
@agent-architecture-documenter \
  --c4-model \
  --sequence-diagrams \
  --data-flow

# 4. Generate user guides
@agent-guide-writer \
  --getting-started \
  --tutorials \
  --faq

# 5. Create developer documentation
@agent-dev-docs \
  --setup-guide \
  --contributing \
  --code-style \
  --architecture-decisions

# 6. Generate code examples
@agent-example-generator \
  --common-use-cases \
  --edge-cases \
  --best-practices

# 7. Create README
@agent-readme-generator \
  --badges \
  --quick-start \
  --features \
  --installation
```

### Documentation Structure Output

```
📚 Documentation Generated
docs/
├── README.md (2.5k words)
├── API.md (15k words, 50 endpoints)
├── ARCHITECTURE.md (8k words, 12 diagrams)
├── guides/
│   ├── getting-started.md
│   ├── authentication.md
│   ├── deployment.md
│   └── troubleshooting.md
├── examples/
│   ├── basic-usage/
│   ├── advanced-patterns/
│   └── integrations/
├── api/
│   ├── openapi.yaml
│   └── postman-collection.json
└── decisions/
    ├── ADR-001-architecture.md
    ├── ADR-002-database.md
    └── ADR-003-authentication.md
```

### Tips

- Keep examples runnable and tested
- Use diagrams liberally
- Document the "why" not just the "what"
- Set up automatic documentation generation in CI

---

## Recipe: Bug Hunt Workflow

**Goal**: Systematically find and fix bugs in the codebase.

**Time Estimate**: 2-4 hours

**Mode Requirements**: Review → Build

### Steps

```bash
# 1. Static analysis (Review mode)
@agent-bug-hunter \
  --static-analysis \
  --pattern-matching \
  --common-bugs

# 2. Find logical errors
@agent-logic-analyzer \
  --off-by-one \
  --null-checks \
  --race-conditions \
  --deadlocks

# 3. Memory and resource issues
@agent-memory-analyzer \
  --leaks \
  --buffer-overflows \
  --use-after-free \
  --resource-cleanup

# 4. Concurrency issues
@agent-concurrency-analyzer \
  --race-conditions \
  --deadlocks \
  --livelocks \
  --thread-safety

# 5. Edge case detection
@agent-edge-case-finder \
  --boundary-values \
  --empty-collections \
  --type-confusion \
  --integer-overflow

# 6. Generate bug fixes (Build mode)
Shift+Tab # Switch to Build mode
@agent-bug-fixer \
  --auto-fix \
  --add-tests \
  --validate-fixes

# 7. Regression prevention
@agent-test-writer \
  --regression-tests \
  --from-bug-reports
```

### Bug Hunt Results

```
🐛 Bug Hunt Summary
Found: 47 issues
├── Critical: 3
│   ├── SQL Injection in user search
│   ├── Race condition in payment processing
│   └── Memory leak in file upload
├── High: 12
│   ├── 5 null pointer dereferences
│   ├── 4 off-by-one errors
│   └── 3 resource leaks
├── Medium: 32
│   └── Various edge cases and validations

Fixed: 44/47 (94%)
├── Auto-fixed: 38
├── Manual review required: 6
└── Tests added: 44
```

### Tips

- Prioritize bugs by user impact
- Add regression tests for every bug fixed
- Look for patterns in bugs
- Set up static analysis in CI

---

## Recipe: API Redesign

**Goal**: Modernize and improve API design for better DX.

**Time Estimate**: 3-5 hours

**Mode Requirements**: Plan → Build

### Steps

```bash
# 1. Analyze current API (Plan mode)
@agent-api-analyzer \
  --rest-principles \
  --consistency \
  --versioning \
  --performance

# 2. Design new API
@agent-api-designer \
  --openapi-first \
  --restful \
  --graphql-option \
  --versioning-strategy

# 3. Generate OpenAPI spec
@agent-openapi-generator \
  --from-code \
  --examples \
  --schemas

# 4. Implement changes (Build mode)
Shift+Tab # Switch to Build mode
@agent-api-implementer \
  --backward-compatible \
  --deprecation-notices \
  --migration-endpoints

# 5. Add validation and security
@agent-api-validator \
  --request-validation \
  --response-validation \
  --rate-limiting \
  --authentication

# 6. Generate client libraries
@agent-client-generator \
  --typescript \
  --python \
  --go \
  --documentation

# 7. Create migration guide
@agent-migration-guide \
  --breaking-changes \
  --deprecations \
  --examples
```

### API Redesign Output

```
API Redesign Complete
├── Endpoints: 45 → 52 (consolidated and expanded)
├── Consistency: 95% (was 60%)
├── Performance: +40% average response time
├── Documentation: 100% coverage
├── Client Libraries: 4 languages
└── Migration Guide: 15 pages with examples

Breaking Changes: 8 (all documented)
Deprecations: 12 endpoints (6-month timeline)
New Features: 15 endpoints added
```

---

## Recipe: Microservices Extraction

**Goal**: Extract monolith components into microservices.

**Time Estimate**: 4-6 hours per service

**Mode Requirements**: Plan → Build → Review

### Steps

```bash
# 1. Analyze monolith (Plan mode)
@agent-monolith-analyzer \
  --identify-boundaries \
  --coupling-analysis \
  --data-dependencies

# 2. Design service boundaries
@agent-ddd-analyzer \
  --bounded-contexts \
  --aggregates \
  --events

# 3. Plan extraction strategy
@agent-extraction-planner \
  --strangler-fig \
  --branch-by-abstraction \
  --parallel-run

# 4. Extract service (Build mode)
Shift+Tab # Switch to Build mode
@agent-service-extractor \
  --service="user-service" \
  --include-tests \
  --api-gateway \
  --event-bus

# 5. Implement service communication
@agent-service-mesh \
  --rest \
  --grpc \
  --event-driven \
  --circuit-breakers

# 6. Add observability
@agent-observability \
  --tracing \
  --metrics \
  --logging \
  --service-mesh

# 7. Validate extraction (Review mode)
Shift+Tab # Switch to Review mode
@agent-integration-tester \
  --end-to-end \
  --contract-testing \
  --performance
```

### Extraction Results

```
Microservice Extraction: user-service
├── Lines of Code: 15,000 extracted
├── API Endpoints: 12
├── Database: Separate PostgreSQL instance
├── Dependencies: 3 other services
├── Communication: gRPC + Events
├── Tests: 85% coverage
└── Deployment: Kubernetes ready

Performance Impact:
├── Latency: +5ms (acceptable)
├── Throughput: +200% (improved)
└── Scalability: Independent scaling enabled
```

---

## Recipe: Technical Debt Reduction

**Goal**: Systematically identify and reduce technical debt.

**Time Estimate**: 4-6 hours

**Mode Requirements**: Plan → Build → Review

### Steps

```bash
# 1. Debt assessment (Plan mode)
@agent-debt-analyzer \
  --code-smells \
  --outdated-dependencies \
  --documentation-debt \
  --test-debt \
  --architecture-debt

# 2. Prioritize debt items
@agent-debt-prioritizer \
  --impact-analysis \
  --cost-benefit \
  --risk-assessment

# 3. Create debt reduction plan
@agent-debt-planner \
  --quick-wins \
  --long-term \
  --roadmap

# 4. Fix code smells (Build mode)
Shift+Tab # Switch to Build mode
@agent-smell-fixer \
  --duplicated-code \
  --long-methods \
  --large-classes \
  --feature-envy

# 5. Update dependencies
@agent-dependency-updater \
  --security-first \
  --breaking-changes \
  --compatibility-check

# 6. Refactor architecture
@agent-architecture-refactorer \
  --patterns \
  --solid-principles \
  --clean-architecture

# 7. Add missing tests
@agent-test-writer \
  --debt-areas \
  --critical-paths

# 8. Update documentation
@agent-docs \
  --update-outdated \
  --fill-gaps

# 9. Measure improvement (Review mode)
Shift+Tab # Switch to Review mode
@agent-debt-analyzer \
  --compare-baseline
```

### Debt Reduction Results

```
Technical Debt Scorecard
Before: D (68 points)
After: B+ (42 points) [-38%]

Improvements:
├── Code Quality: C → B+
│   ├── Duplication: 8% → 2%
│   ├── Complexity: 15 → 8
│   └── Test Coverage: 45% → 75%
├── Dependencies: F → B
│   ├── Outdated: 45 → 5
│   ├── Vulnerable: 12 → 0
│   └── Unused: 8 → 0
├── Documentation: D → B
│   ├── Coverage: 30% → 70%
│   └── Up-to-date: 40% → 90%
└── Architecture: C → B
    ├── Coupling: High → Medium
    └── Cohesion: Low → High

Time Invested: 6 hours
Estimated Time Saved: 200+ hours/year
```

---

## Advanced Recipes

### Multi-Stage Release Pipeline

```bash
# Complete release pipeline with quality gates
@agent-release-manager \
  --stages="test,security,performance,docs,deploy" \
  --quality-gates \
  --rollback-on-failure \
  --notifications
```

### Cross-Platform Compatibility

```bash
# Ensure code works across platforms
@agent-compatibility \
  --platforms="windows,mac,linux" \
  --browsers="chrome,firefox,safari" \
  --node-versions="16,18,20" \
  --python-versions="3.8,3.9,3.10,3.11"
```

### Compliance and Audit

```bash
# Ensure regulatory compliance
@agent-compliance \
  --gdpr \
  --hipaa \
  --pci-dss \
  --sox \
  --generate-evidence
```

### Disaster Recovery Testing

```bash
# Test disaster recovery procedures
@agent-chaos \
  --kill-services \
  --network-partition \
  --data-corruption \
  --recovery-validation
```

## Tips for Creating Custom Recipes

1. **Start with Plan mode** for analysis
2. **Use worktrees** for experimental changes
3. **Chain agents** for complex workflows
4. **Validate incrementally** after each step
5. **Measure success** with clear metrics
6. **Document recipes** for team sharing
7. **Automate recipes** in CI/CD
8. **Version recipes** with the codebase

## Recipe Templates

### Basic Recipe Template

```bash
# Recipe: [Name]
# Goal: [What this accomplishes]
# Time: [Estimated duration]
# Mode: [Required modes]

# Step 1: Analyze
@agent-analyzer --options

# Step 2: Plan
@agent-planner --options

# Step 3: Execute
@agent-executor --options

# Step 4: Validate
@agent-validator --options
```

### Complex Recipe Template

```yaml
# ~/.agcodex/recipes/custom-recipe.yaml
name: custom-recipe
description: Complex multi-stage recipe
estimated_time: 4h
modes:
  - plan
  - build
  - review
  
stages:
  - name: analysis
    mode: plan
    agents:
      - analyzer:
          parallel: true
          options:
            deep: true
            
  - name: implementation
    mode: build
    agents:
      - implementer:
          incremental: true
          validation: true
          
  - name: validation
    mode: review
    agents:
      - validator:
          comprehensive: true
          
success_criteria:
  - coverage: ">80%"
  - performance: "<100ms"
  - security: "no-critical"
```

## Troubleshooting Recipes

### When Recipes Fail

1. **Check mode requirements** - Ensure correct mode is active
2. **Verify prerequisites** - Some agents need specific tools
3. **Review agent logs** - Use `@agent-log agent-name`
4. **Validate incrementally** - Break complex recipes into steps
5. **Use debug mode** - Add `--debug` to any agent invocation

### Common Issues

| Issue | Solution |
|-------|----------|
| Agent timeout | Increase timeout with `--timeout=30m` |
| Out of memory | Limit scope or use `--max-memory=4G` |
| Conflicting changes | Use worktrees for isolation |
| Test failures | Run `@agent-debugger` on failures |
| Slow execution | Enable caching with `--cache=true` |

## Contributing Recipes

Share your recipes with the community:

1. Test recipe thoroughly
2. Document expected outcomes
3. Include error handling
4. Submit PR to `docs/recipes/`
5. Tag with categories and complexity

---

*Last updated: 2025-08-21*
*AGCodex version: 2.0.0*