//! AGCodex Agent Workflow Examples
//! 
//! This module demonstrates various agent workflow patterns for AGCodex,
//! including single-agent operations, sequential pipelines, parallel execution,
//! and complex multi-agent orchestration.

use agcodex_core::{
    agents::{Agent, AgentBuilder, AgentChain, AgentResult},
    modes::{BuildMode, PlanMode, ReviewMode},
    workspace::Workspace,
};
use std::path::PathBuf;

/// Example 1: Code Review Workflow
/// 
/// Performs comprehensive code review with security, performance, and quality checks.
/// Uses Review mode to ensure no destructive operations.
#[tokio::main]
async fn code_review_workflow(workspace: &Workspace) -> AgentResult<()> {
    println!("🔍 Starting comprehensive code review...");
    
    // Create code reviewer agent with Review mode enforcement
    let reviewer = AgentBuilder::new("code-reviewer")
        .mode_override(ReviewMode)
        .intelligence("hard")  // Maximum AST analysis
        .tools(vec![
            "ast-search",
            "tree-sitter-analyze",
            "security-scan",
            "complexity-analyzer",
        ])
        .prompt(r#"
            Perform comprehensive code review focusing on:
            1. Security vulnerabilities (OWASP Top 10)
            2. Performance bottlenecks (O(n²) or worse)
            3. Memory leaks and resource management
            4. Error handling completeness
            5. Code complexity (cyclomatic > 10)
            
            Generate actionable recommendations with severity levels.
        "#)
        .build()?;
    
    // Execute review on entire workspace
    let review_result = reviewer
        .analyze_workspace(workspace)
        .with_output_format("markdown")
        .with_cache(true)  // Use cached AST for performance
        .execute()
        .await?;
    
    // Generate review report
    review_result.save_report("review_report.md")?;
    println!("✅ Code review complete: {}", review_result.summary());
    
    Ok(())
}

/// Example 2: Refactoring Workflow
/// 
/// Systematic code refactoring with safety checks and incremental application.
/// Uses Build mode for write access with validation between steps.
async fn refactoring_workflow(
    workspace: &Workspace,
    target_pattern: &str,
) -> AgentResult<()> {
    println!("🔨 Starting refactoring workflow for pattern: {}", target_pattern);
    
    // Step 1: Analyze refactoring opportunities (Plan mode)
    let analyzer = AgentBuilder::new("refactor-analyzer")
        .mode_override(PlanMode)
        .intelligence("hard")
        .prompt(format!(
            "Identify refactoring opportunities for pattern: {}
             Focus on: code duplication, complex methods, poor naming, 
             tight coupling, missing abstractions",
            target_pattern
        ))
        .build()?;
    
    let opportunities = analyzer.analyze_workspace(workspace).await?;
    
    // Step 2: Generate refactoring plan
    let planner = AgentBuilder::new("refactor-planner")
        .mode_override(PlanMode)
        .context(opportunities.clone())
        .prompt("Create detailed refactoring plan with dependency order")
        .build()?;
    
    let plan = planner.generate_plan().await?;
    
    // Step 3: Apply refactorings incrementally (Build mode)
    let refactorer = AgentBuilder::new("refactorer")
        .mode_override(BuildMode)
        .tools(vec!["ast-transform", "comby", "ast-grep"])
        .incremental(true)  // Apply changes one at a time
        .validation(true)   // Validate after each change
        .build()?;
    
    for step in plan.steps() {
        println!("  Applying: {}", step.description);
        
        // Create git worktree for isolated changes
        let worktree = workspace.create_worktree(&step.branch_name)?;
        
        // Apply refactoring
        refactorer
            .apply_transformation(step)
            .in_worktree(&worktree)
            .with_validation(|ws| {
                // Run tests after each refactoring
                ws.run_tests().is_ok()
            })
            .execute()
            .await?;
        
        println!("  ✓ Step complete: {}", step.description);
    }
    
    println!("✅ Refactoring workflow complete");
    Ok(())
}

/// Example 3: Test Generation Workflow
/// 
/// Generates comprehensive test suites with coverage analysis.
/// Combines static analysis with LLM-powered test synthesis.
async fn test_generation_workflow(
    workspace: &Workspace,
    target_dir: PathBuf,
) -> AgentResult<()> {
    println!("🧪 Starting test generation for: {:?}", target_dir);
    
    // Step 1: Analyze existing code and tests
    let analyzer = AgentBuilder::new("test-analyzer")
        .mode_override(PlanMode)
        .tools(vec!["coverage-analyzer", "ast-search"])
        .prompt("Analyze code coverage and identify untested paths")
        .build()?;
    
    let coverage = analyzer
        .analyze_directory(&target_dir)
        .with_metrics(true)
        .await?;
    
    println!("  Current coverage: {:.1}%", coverage.percentage());
    
    // Step 2: Generate test cases
    let generator = AgentBuilder::new("test-writer")
        .mode_override(BuildMode)
        .intelligence("hard")
        .context(coverage.uncovered_paths())
        .prompt(r#"
            Generate comprehensive test cases for uncovered code:
            - Unit tests for all public functions
            - Edge cases and boundary conditions
            - Error handling scenarios
            - Property-based tests where applicable
            - Integration tests for complex interactions
        "#)
        .build()?;
    
    let test_suite = generator
        .generate_tests()
        .with_framework_detection()  // Auto-detect test framework
        .with_mocking(true)          // Generate mocks as needed
        .await?;
    
    // Step 3: Validate and refine tests
    let validator = AgentBuilder::new("test-validator")
        .mode_override(ReviewMode)
        .prompt("Validate test quality, remove redundancy, ensure determinism")
        .build()?;
    
    let refined_tests = validator
        .validate_tests(test_suite)
        .with_mutation_testing(true)  // Use mutation testing for quality
        .await?;
    
    // Step 4: Apply tests and measure improvement
    refined_tests.write_to_workspace(workspace)?;
    
    let new_coverage = workspace.measure_coverage()?;
    println!("✅ Test generation complete");
    println!("  Coverage improved: {:.1}% → {:.1}%", 
             coverage.percentage(), 
             new_coverage.percentage());
    
    Ok(())
}

/// Example 4: Security Audit Workflow
/// 
/// Comprehensive security analysis with vulnerability detection and remediation.
/// Combines static analysis, dependency scanning, and security best practices.
async fn security_audit_workflow(workspace: &Workspace) -> AgentResult<()> {
    println!("🔒 Starting security audit...");
    
    // Parallel security analysis with multiple specialized agents
    let agents = vec![
        // SAST agent for code vulnerabilities
        AgentBuilder::new("sast-scanner")
            .mode_override(ReviewMode)
            .tools(vec!["semgrep", "ast-security-analyzer"])
            .prompt("Scan for OWASP Top 10 and CWE vulnerabilities")
            .build()?,
        
        // Dependency scanner
        AgentBuilder::new("dependency-scanner")
            .mode_override(ReviewMode)
            .tools(vec!["cargo-audit", "npm-audit", "safety"])
            .prompt("Check for known vulnerabilities in dependencies")
            .build()?,
        
        // Secrets scanner
        AgentBuilder::new("secrets-scanner")
            .mode_override(ReviewMode)
            .tools(vec!["gitleaks", "trufflehog"])
            .prompt("Scan for exposed secrets, tokens, and credentials")
            .build()?,
        
        // Configuration auditor
        AgentBuilder::new("config-auditor")
            .mode_override(ReviewMode)
            .prompt("Audit security configurations and permissions")
            .build()?,
    ];
    
    // Execute all scanners in parallel
    let results = AgentChain::parallel(agents)
        .execute_on(workspace)
        .await?;
    
    // Aggregate and prioritize findings
    let aggregator = AgentBuilder::new("security-aggregator")
        .mode_override(ReviewMode)
        .context(results)
        .prompt(r#"
            Aggregate security findings and:
            1. Deduplicate similar issues
            2. Assign CVSS scores
            3. Prioritize by severity and exploitability
            4. Generate remediation recommendations
            5. Create fix templates where possible
        "#)
        .build()?;
    
    let audit_report = aggregator.generate_report().await?;
    
    // Generate remediation patches
    if audit_report.has_auto_fixable() {
        let remediator = AgentBuilder::new("security-remediator")
            .mode_override(BuildMode)
            .prompt("Generate security patches for auto-fixable issues")
            .build()?;
        
        let patches = remediator
            .generate_patches(audit_report.auto_fixable())
            .await?;
        
        patches.save_as("security_patches.patch")?;
        println!("  Generated {} security patches", patches.len());
    }
    
    audit_report.save_as("security_audit.md")?;
    println!("✅ Security audit complete: {} issues found", 
             audit_report.total_issues());
    
    Ok(())
}

/// Example 5: Performance Optimization Workflow
/// 
/// Identifies and resolves performance bottlenecks through profiling and optimization.
/// Uses incremental optimization with benchmarking between changes.
async fn performance_optimization_workflow(
    workspace: &Workspace,
    baseline_benchmark: Option<PathBuf>,
) -> AgentResult<()> {
    println!("⚡ Starting performance optimization...");
    
    // Step 1: Profile and identify bottlenecks
    let profiler = AgentBuilder::new("performance-profiler")
        .mode_override(ReviewMode)
        .tools(vec![
            "flamegraph",
            "perf",
            "complexity-analyzer",
            "memory-profiler",
        ])
        .prompt(r#"
            Profile the application and identify:
            1. CPU hotspots and bottlenecks
            2. Memory allocation patterns
            3. I/O blocking operations
            4. Algorithmic complexity issues
            5. Cache misses and data locality problems
        "#)
        .build()?;
    
    let profile = profiler
        .profile_workspace(workspace)
        .with_benchmark(baseline_benchmark)
        .await?;
    
    // Step 2: Generate optimization strategies
    let strategist = AgentBuilder::new("optimization-strategist")
        .mode_override(PlanMode)
        .context(profile.clone())
        .intelligence("hard")
        .prompt(r#"
            Generate optimization strategies for identified bottlenecks:
            - Algorithm replacements (O(n²) → O(n log n))
            - Data structure optimizations
            - Parallelization opportunities
            - Caching strategies
            - Memory layout optimizations
            - I/O batching and async patterns
        "#)
        .build()?;
    
    let strategies = strategist.generate_strategies().await?;
    
    // Step 3: Apply optimizations incrementally
    for strategy in strategies.prioritized() {
        println!("  Applying: {}", strategy.description);
        
        // Create isolated worktree for optimization
        let worktree = workspace.create_worktree(&strategy.branch_name)?;
        
        let optimizer = AgentBuilder::new("optimizer")
            .mode_override(BuildMode)
            .tools(vec!["ast-transform", "comby"])
            .prompt(format!("Apply optimization: {}", strategy.description))
            .build()?;
        
        // Apply optimization
        optimizer
            .apply_optimization(strategy)
            .in_worktree(&worktree)
            .await?;
        
        // Benchmark after optimization
        let benchmark = workspace
            .run_benchmark_in_worktree(&worktree)
            .await?;
        
        // Validate improvement
        if benchmark.shows_improvement(&profile.baseline) {
            println!("    ✓ Performance improved: {:.1}%", 
                     benchmark.improvement_percentage());
            worktree.merge_to_main()?;
        } else {
            println!("    ✗ No improvement, reverting");
            worktree.delete()?;
        }
    }
    
    // Step 4: Final validation and report
    let final_benchmark = workspace.run_benchmark().await?;
    let report = final_benchmark.compare_to(&profile.baseline);
    
    report.save_as("performance_report.md")?;
    println!("✅ Optimization complete: {:.1}% overall improvement",
             report.total_improvement());
    
    Ok(())
}

/// Example 6: Documentation Generation Workflow
/// 
/// Generates comprehensive documentation from code analysis.
/// Creates API docs, architecture diagrams, and usage examples.
async fn documentation_workflow(workspace: &Workspace) -> AgentResult<()> {
    println!("📚 Starting documentation generation...");
    
    // Parallel documentation generation
    let doc_agents = vec![
        // API documentation
        AgentBuilder::new("api-documenter")
            .mode_override(PlanMode)
            .tools(vec!["ast-search", "rustdoc", "jsdoc"])
            .prompt("Generate API documentation with examples")
            .build()?,
        
        // Architecture documentation
        AgentBuilder::new("architecture-documenter")
            .mode_override(PlanMode)
            .tools(vec!["mermaid", "plantuml"])
            .prompt("Create architecture diagrams and system design docs")
            .build()?,
        
        // Usage examples
        AgentBuilder::new("example-generator")
            .mode_override(PlanMode)
            .prompt("Generate practical usage examples and tutorials")
            .build()?,
        
        // README generator
        AgentBuilder::new("readme-generator")
            .mode_override(PlanMode)
            .prompt("Create comprehensive README with quick start guide")
            .build()?,
    ];
    
    // Generate all documentation in parallel
    let docs = AgentChain::parallel(doc_agents)
        .execute_on(workspace)
        .await?;
    
    // Consolidate and organize documentation
    let organizer = AgentBuilder::new("doc-organizer")
        .mode_override(BuildMode)
        .context(docs)
        .prompt(r#"
            Organize documentation into coherent structure:
            - README.md (overview and quick start)
            - docs/API.md (complete API reference)
            - docs/ARCHITECTURE.md (system design)
            - docs/EXAMPLES.md (usage examples)
            - docs/CONTRIBUTING.md (developer guide)
        "#)
        .build()?;
    
    organizer
        .organize_documentation()
        .with_cross_references(true)
        .with_search_index(true)
        .await?;
    
    println!("✅ Documentation generation complete");
    Ok(())
}

/// Example 7: Complex Multi-Agent Pipeline
/// 
/// Demonstrates a sophisticated pipeline combining multiple agents
/// in both sequential and parallel patterns for a complete development workflow.
async fn complex_multi_agent_pipeline(
    workspace: &Workspace,
    feature_spec: &str,
) -> AgentResult<()> {
    println!("🚀 Starting complex multi-agent pipeline for: {}", feature_spec);
    
    // Phase 1: Planning and Design (Sequential)
    let design_chain = AgentChain::sequential(vec![
        // Requirement analyzer
        AgentBuilder::new("requirement-analyzer")
            .mode_override(PlanMode)
            .prompt(format!("Analyze requirements: {}", feature_spec))
            .build()?,
        
        // Architecture designer
        AgentBuilder::new("architect")
            .mode_override(PlanMode)
            .intelligence("hard")
            .prompt("Design system architecture based on requirements")
            .build()?,
        
        // Task decomposer
        AgentBuilder::new("task-decomposer")
            .mode_override(PlanMode)
            .prompt("Break down into implementation tasks with dependencies")
            .build()?,
    ]);
    
    let design = design_chain.execute().await?;
    
    // Phase 2: Parallel Implementation
    // Create worktrees for parallel development
    let tasks = design.get_tasks();
    let mut implementation_agents = Vec::new();
    
    for task in tasks.independent_groups() {
        let worktree = workspace.create_worktree(&task.branch_name)?;
        
        let implementer = AgentBuilder::new(&format!("impl-{}", task.id))
            .mode_override(BuildMode)
            .context(task.specification.clone())
            .worktree(worktree)
            .tools(vec!["ast-transform", "comby", "copilot"])
            .prompt(format!("Implement: {}", task.description))
            .build()?;
        
        implementation_agents.push(implementer);
    }
    
    // Execute implementations in parallel
    let implementations = AgentChain::parallel(implementation_agents)
        .with_progress_tracking(true)
        .execute()
        .await?;
    
    // Phase 3: Integration and Testing (Sequential with validation)
    let integration_chain = AgentChain::sequential(vec![
        // Merge coordinator
        AgentBuilder::new("merge-coordinator")
            .mode_override(BuildMode)
            .context(implementations.clone())
            .prompt("Merge parallel implementations, resolve conflicts")
            .build()?,
        
        // Integration tester
        AgentBuilder::new("integration-tester")
            .mode_override(ReviewMode)
            .prompt("Create and run integration tests")
            .build()?,
        
        // Performance validator
        AgentBuilder::new("performance-validator")
            .mode_override(ReviewMode)
            .tools(vec!["benchmark", "profiler"])
            .prompt("Validate performance requirements are met")
            .build()?,
    ]);
    
    let integration_result = integration_chain
        .with_rollback_on_failure(true)
        .execute()
        .await?;
    
    // Phase 4: Quality Assurance (Parallel)
    let qa_agents = vec![
        AgentBuilder::new("security-auditor")
            .mode_override(ReviewMode)
            .build()?,
        
        AgentBuilder::new("code-reviewer")
            .mode_override(ReviewMode)
            .build()?,
        
        AgentBuilder::new("doc-reviewer")
            .mode_override(ReviewMode)
            .build()?,
    ];
    
    let qa_results = AgentChain::parallel(qa_agents)
        .execute_on(&integration_result.workspace)
        .await?;
    
    // Phase 5: Final Report Generation
    let reporter = AgentBuilder::new("pipeline-reporter")
        .mode_override(PlanMode)
        .context(vec![design, implementations, integration_result, qa_results])
        .prompt(r#"
            Generate comprehensive pipeline report:
            - Feature implementation summary
            - Test results and coverage
            - Performance metrics
            - Security findings
            - Documentation status
            - Deployment readiness checklist
        "#)
        .build()?;
    
    let report = reporter.generate_report().await?;
    report.save_as("pipeline_report.md")?;
    
    println!("✅ Complex pipeline complete: {}", report.summary());
    Ok(())
}

/// Helper function to run an example workflow
pub async fn run_example(example: &str, workspace: &Workspace) -> AgentResult<()> {
    match example {
        "review" => code_review_workflow(workspace).await,
        "refactor" => refactoring_workflow(workspace, "singleton").await,
        "test" => test_generation_workflow(workspace, PathBuf::from("src")).await,
        "security" => security_audit_workflow(workspace).await,
        "performance" => performance_optimization_workflow(workspace, None).await,
        "docs" => documentation_workflow(workspace).await,
        "complex" => complex_multi_agent_pipeline(workspace, "Add caching layer").await,
        _ => {
            println!("Unknown example: {}", example);
            println!("Available examples: review, refactor, test, security, performance, docs, complex");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_agent_builder() {
        let agent = AgentBuilder::new("test-agent")
            .mode_override(ReviewMode)
            .intelligence("medium")
            .tools(vec!["ast-search"])
            .prompt("Test prompt")
            .build();
        
        assert!(agent.is_ok());
    }
    
    #[tokio::test]
    async fn test_agent_chain_sequential() {
        let agents = vec![
            AgentBuilder::new("agent1").build().unwrap(),
            AgentBuilder::new("agent2").build().unwrap(),
        ];
        
        let chain = AgentChain::sequential(agents);
        assert_eq!(chain.len(), 2);
    }
    
    #[tokio::test]
    async fn test_agent_chain_parallel() {
        let agents = vec![
            AgentBuilder::new("agent1").build().unwrap(),
            AgentBuilder::new("agent2").build().unwrap(),
            AgentBuilder::new("agent3").build().unwrap(),
        ];
        
        let chain = AgentChain::parallel(agents);
        assert_eq!(chain.len(), 3);
    }
}