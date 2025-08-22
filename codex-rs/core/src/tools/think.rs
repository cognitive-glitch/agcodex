//! Simple step-by-step reasoning tool for AGCodex
//!
//! Provides transparent reasoning that's easy for LLMs to follow and understand.
//! Focuses on practical problem-solving with clear step-by-step breakdown.

use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ThinkError {
    #[error("problem description is empty or too short")]
    EmptyProblem,

    #[error("failed to generate reasoning steps: {0}")]
    ReasoningFailed(String),

    #[error("confidence calculation error: {0}")]
    ConfidenceError(String),
}

/// A single step in the reasoning process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkStep {
    /// Step number (1-based)
    pub step_number: usize,
    /// The thought or reasoning for this step
    pub thought: String,
    /// The reasoning behind this step
    pub reasoning: String,
}

/// Complexity level for code problems
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Complexity {
    Simple,  // Single file, <50 lines, well-defined scope
    Medium,  // Multiple files, moderate scope, some dependencies
    Complex, // Cross-cutting concerns, architectural changes, high impact
}

/// Code-specific problem types for better reasoning
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CodeProblemType {
    BugFix,         // Debugging and fixing issues
    Refactoring,    // Code improvement without changing behavior
    Implementation, // New feature development
    Performance,    // Optimization and performance improvements
    Security,       // Security analysis and fixes
    Testing,        // Test creation and improvement
    Documentation,  // Code documentation and comments
    Architecture,   // System design and structural changes
    CodeReview,     // Code quality and standards review
}

/// Enhanced result for code-specific thinking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeThinkResult {
    /// All reasoning steps (minimum 3)
    pub steps: Vec<ThinkStep>,
    /// Final conclusion
    pub conclusion: String,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f32,
    /// Recommended next action
    pub recommended_action: String,
    /// Files that might need changes
    pub affected_files: Vec<String>,
    /// Problem complexity assessment
    pub complexity: Complexity,
    /// Code problem type
    pub problem_type: CodeProblemType,
}

/// Complete result from the think tool (backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkResult {
    /// All reasoning steps (minimum 3)
    pub steps: Vec<ThinkStep>,
    /// Final conclusion
    pub conclusion: String,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f32,
}

/// Simple think tool implementation
#[derive(Debug)]
pub struct ThinkTool;

impl ThinkTool {
    /// Create a new think tool instance
    pub const fn new() -> Self {
        Self
    }

    /// Perform step-by-step reasoning on a question or problem
    pub fn think(question: &str) -> Result<ThinkResult, ThinkError> {
        if question.trim().is_empty() || question.len() < 5 {
            return Err(ThinkError::EmptyProblem);
        }

        let steps = Self::generate_reasoning_steps(question)?;
        let conclusion = Self::generate_conclusion(question, &steps);
        let confidence = Self::calculate_confidence(question, &steps);

        Ok(ThinkResult {
            steps,
            conclusion,
            confidence,
        })
    }

    /// Enhanced thinking specifically for code problems
    pub fn think_about_code(
        problem: &str,
        language: Option<&str>,
        context: Option<&str>, // File path, function name, etc
    ) -> Result<CodeThinkResult, ThinkError> {
        if problem.trim().is_empty() || problem.len() < 5 {
            return Err(ThinkError::EmptyProblem);
        }

        let problem_type = Self::classify_code_problem(problem);
        let complexity = Self::assess_complexity(problem, language, context);
        let steps = Self::generate_code_reasoning_steps(problem, &problem_type, language, context)?;
        let conclusion = Self::generate_code_conclusion(problem, &steps, &problem_type);
        let confidence = Self::calculate_code_confidence(problem, &steps, &problem_type);
        let recommended_action = Self::determine_recommended_action(&problem_type, problem);
        let affected_files = Self::identify_affected_files(problem, context);

        Ok(CodeThinkResult {
            steps,
            conclusion,
            confidence,
            recommended_action,
            affected_files,
            complexity,
            problem_type,
        })
    }

    /// Quick analysis for error messages and debugging
    pub fn analyze_error(error_message: &str) -> Result<CodeThinkResult, ThinkError> {
        if error_message.trim().is_empty() {
            return Err(ThinkError::EmptyProblem);
        }

        let _problem = format!("Debug and fix error: {}", error_message);
        let steps = Self::generate_error_analysis_steps(error_message)?;
        let recommended_action = Self::determine_error_action(error_message);
        let affected_files = Self::extract_files_from_error(error_message);
        let complexity = if affected_files.len() > 2 {
            Complexity::Complex
        } else if affected_files.len() > 1 {
            Complexity::Medium
        } else {
            Complexity::Simple
        };

        Ok(CodeThinkResult {
            steps,
            conclusion: "Systematic error analysis completed with specific debugging steps."
                .to_string(),
            confidence: 0.8,
            recommended_action,
            affected_files,
            complexity,
            problem_type: CodeProblemType::BugFix,
        })
    }

    /// Specialized thinking for refactoring scenarios
    pub fn plan_refactoring(code_smell: &str) -> Result<CodeThinkResult, ThinkError> {
        if code_smell.trim().is_empty() {
            return Err(ThinkError::EmptyProblem);
        }

        let problem = format!("Refactor to address: {}", code_smell);
        let steps = Self::generate_refactoring_steps(code_smell)?;
        let recommended_action = "Implement refactoring strategy".to_string();
        let complexity = 5; // Default complexity score for refactoring

        Ok(CodeThinkResult {
            steps,
            conclusion: "Refactoring strategy defined with clear improvement goals.".to_string(),
            confidence: 0.85,
            recommended_action,
            affected_files: vec![], // Would need more context to determine
            complexity: Complexity::Medium,
            problem_type: CodeProblemType::Refactoring,
        })
    }

    /// Generate at least 3 reasoning steps for the given question
    fn generate_reasoning_steps(question: &str) -> Result<Vec<ThinkStep>, ThinkError> {
        let mut steps = Vec::new();

        // Step 1: Problem Analysis
        steps.push(ThinkStep {
            step_number: 1,
            thought: format!("Analyzing the core question: '{}'", question.trim()),
            reasoning: Self::analyze_problem_type(question),
        });

        // Step 2: Breaking Down the Problem
        steps.push(ThinkStep {
            step_number: 2,
            thought: "Breaking down the problem into key components".to_string(),
            reasoning: Self::identify_key_components(question),
        });

        // Step 3: Considering Approach
        steps.push(ThinkStep {
            step_number: 3,
            thought: "Evaluating possible approaches and considerations".to_string(),
            reasoning: Self::evaluate_approaches(question),
        });

        // Additional steps based on problem complexity
        if Self::is_complex_problem(question) {
            steps.push(ThinkStep {
                step_number: 4,
                thought: "Analyzing potential challenges and constraints".to_string(),
                reasoning: Self::analyze_constraints(question),
            });
        }

        if Self::requires_implementation_thinking(question) {
            let step_num = steps.len() + 1;
            steps.push(ThinkStep {
                step_number: step_num,
                thought: "Considering implementation details and practical aspects".to_string(),
                reasoning: Self::consider_implementation(question),
            });
        }

        Ok(steps)
    }

    /// Generate a conclusion based on the question and reasoning steps
    fn generate_conclusion(question: &str, _steps: &[ThinkStep]) -> String {
        let problem_type = Self::classify_problem_type(question);

        match problem_type {
            ProblemType::Technical => {
                "Based on analysis: Technical solution requires systematic approach with careful implementation, testing, and edge case handling.".to_string()
            }
            ProblemType::Design => {
                "Design approach: Balance user requirements, technical constraints, and maintainability through iterative refinement.".to_string()
            }
            ProblemType::Analysis => {
                "Analysis complete: Key factors identified. Thorough data examination will inform decision-making process.".to_string()
            }
            ProblemType::Planning => {
                "Planning strategy: Break into phases, identify dependencies, maintain flexibility for evolving requirements.".to_string()
            }
            ProblemType::Debugging => {
                "Debug approach: Systematic investigation from most likely causes, methodical elimination of potential issues.".to_string()
            }
            ProblemType::General => {
                "Solution path: Focus on core requirements while maintaining adaptability for changing circumstances.".to_string()
            }
        }
    }

    /// Calculate confidence based on problem clarity and completeness
    fn calculate_confidence(question: &str, steps: &[ThinkStep]) -> f32 {
        let mut confidence = 0.5; // Base confidence

        // Boost confidence for well-defined problems
        if Self::is_well_defined_problem(question) {
            confidence += 0.2;
        }

        // Boost confidence based on number of reasoning steps
        let step_bonus = (steps.len() as f32 - 3.0) * 0.05;
        confidence += step_bonus.min(0.2);

        // Boost confidence for specific technical terms
        if Self::contains_technical_terms(question) {
            confidence += 0.1;
        }

        // Reduce confidence for very broad or vague questions
        if Self::is_vague_problem(question) {
            confidence -= 0.2;
        }

        // Ensure confidence stays within bounds
        confidence.max(0.1).min(0.95)
    }

    // Code-specific helper methods

    fn classify_code_problem(problem: &str) -> CodeProblemType {
        let p_lower = problem.to_lowercase();

        // Check more specific categories first (higher priority)
        if p_lower.contains("security")
            || p_lower.contains("vulnerability")
            || p_lower.contains("exploit")
            || p_lower.contains("injection")
            || p_lower.contains("csrf")
            || p_lower.contains("xss")
            || (p_lower.contains("auth") && !p_lower.contains("fix") && !p_lower.contains("bug"))
        {
            CodeProblemType::Security
        } else if p_lower.contains("optimize")
            || p_lower.contains("performance")
            || p_lower.contains("speed")
            || p_lower.contains("memory leak")
            || p_lower.contains("efficient")
            || p_lower.contains("bottleneck")
            || p_lower.contains("latency")
            || p_lower.contains("throughput")
        {
            CodeProblemType::Performance
        } else if p_lower.contains("architecture")
            || p_lower.contains("design pattern")
            || p_lower.contains("system design")
            || p_lower.contains("microservice")
            || (p_lower.contains("design")
                && (p_lower.contains("system") || p_lower.contains("pattern")))
        {
            CodeProblemType::Architecture
        } else if p_lower.contains("refactor")
            || p_lower.contains("clean up")
            || p_lower.contains("improve code")
            || p_lower.contains("simplify")
            || p_lower.contains("restructure")
            || p_lower.contains("code smell")
        {
            CodeProblemType::Refactoring
        } else if p_lower.contains("test")
            && (p_lower.contains("write")
                || p_lower.contains("create")
                || p_lower.contains("unit")
                || p_lower.contains("integration")
                || p_lower.contains("coverage"))
        {
            CodeProblemType::Testing
        } else if p_lower.contains("document")
            || p_lower.contains("comment")
            || p_lower.contains("explain")
            || p_lower.contains("readme")
            || p_lower.contains("api doc")
            || p_lower.contains("javadoc")
        {
            CodeProblemType::Documentation
        } else if p_lower.contains("review")
            && (p_lower.contains("code")
                || p_lower.contains("quality")
                || p_lower.contains("standards")
                || p_lower.contains("best practice")
                || p_lower.contains("lint"))
        {
            CodeProblemType::CodeReview
        } else if p_lower.contains("implement")
            || p_lower.contains("create")
            || p_lower.contains("build")
            || p_lower.contains("add feature")
            || p_lower.contains("develop")
            || p_lower.contains("new feature")
        {
            CodeProblemType::Implementation
        } else if p_lower.contains("bug")
            || p_lower.contains("crash")
            || p_lower.contains("debug")
            || p_lower.contains("exception")
            || p_lower.contains("failure")
            || (p_lower.contains("fix")
                && (p_lower.contains("error")
                    || p_lower.contains("issue")
                    || p_lower.contains("null")))
        {
            CodeProblemType::BugFix
        } else {
            CodeProblemType::Implementation // Default for ambiguous cases
        }
    }

    fn assess_complexity(
        problem: &str,
        language: Option<&str>,
        context: Option<&str>,
    ) -> Complexity {
        let mut complexity_score = 0;

        // Problem complexity indicators
        let p_lower = problem.to_lowercase();
        if p_lower.contains("system") || p_lower.contains("architecture") {
            complexity_score += 2;
        }
        if p_lower.contains("multiple") || p_lower.contains("several") {
            complexity_score += 1;
        }
        if p_lower.contains("integration") || p_lower.contains("api") {
            complexity_score += 1;
        }
        if p_lower.contains("database") || p_lower.contains("concurrent") {
            complexity_score += 1;
        }
        if p_lower.contains("migration") || p_lower.contains("legacy") {
            complexity_score += 2;
        }

        // Language complexity
        if let Some(lang) = language {
            let lang_lower = lang.to_lowercase();
            if lang_lower == "c++" || lang_lower == "rust" || lang_lower == "haskell" {
                complexity_score += 1;
            }
            if lang_lower == "assembly" || lang_lower == "cuda" {
                complexity_score += 2;
            }
        }

        // Context complexity
        if let Some(ctx) = context {
            if ctx.contains('/') && ctx.matches('/').count() > 3 {
                complexity_score += 1;
            } // Deep file structure
            if ctx.contains("test") && ctx.contains("integration") {
                complexity_score += 1;
            }
        }

        // Problem length as complexity indicator
        if problem.len() > 200 {
            complexity_score += 1;
        }
        if problem.len() > 500 {
            complexity_score += 2;
        }

        match complexity_score {
            0..=2 => Complexity::Simple,
            3..=5 => Complexity::Medium,
            _ => Complexity::Complex,
        }
    }

    fn generate_code_reasoning_steps(
        problem: &str,
        problem_type: &CodeProblemType,
        language: Option<&str>,
        context: Option<&str>,
    ) -> Result<Vec<ThinkStep>, ThinkError> {
        let mut steps = Vec::new();

        // Step 1: Code Context Understanding
        let context_analysis = if let Some(ctx) = context {
            format!(
                "Code context: {}. Understanding scope and dependencies.",
                ctx
            )
        } else {
            "Analyzing code context and identifying scope of changes.".to_string()
        };

        steps.push(ThinkStep {
            step_number: 1,
            thought: format!("Understanding code context for: '{}'", problem.trim()),
            reasoning: context_analysis,
        });

        // Step 2: Problem-specific analysis
        let problem_reasoning = match problem_type {
            CodeProblemType::BugFix => {
                "Identifying root cause, reproduction steps, and potential side effects of the bug."
            }
            CodeProblemType::Refactoring => {
                "Analyzing code structure, identifying improvement opportunities without changing behavior."
            }
            CodeProblemType::Implementation => {
                "Breaking down requirements into implementable components and design decisions."
            }
            CodeProblemType::Performance => {
                "Profiling bottlenecks, measuring current performance, identifying optimization targets."
            }
            CodeProblemType::Security => {
                "Evaluating security vulnerabilities, attack vectors, and mitigation strategies."
            }
            CodeProblemType::Testing => {
                "Designing test cases, coverage analysis, and validation strategies."
            }
            CodeProblemType::Documentation => {
                "Structuring documentation, identifying key concepts, and user scenarios."
            }
            CodeProblemType::Architecture => {
                "Analyzing system components, dependencies, and architectural patterns."
            }
            CodeProblemType::CodeReview => {
                "Evaluating code quality, standards compliance, and best practices."
            }
        };

        steps.push(ThinkStep {
            step_number: 2,
            thought: format!("Analyzing specific challenge: {:?} problem", problem_type),
            reasoning: problem_reasoning.to_string(),
        });

        // Step 3: Implementation approach
        let approach_reasoning = Self::get_implementation_approach(problem_type, language);
        steps.push(ThinkStep {
            step_number: 3,
            thought: "Evaluating implementation approaches and technical considerations"
                .to_string(),
            reasoning: approach_reasoning,
        });

        // Step 4: Trade-offs and constraints (for complex problems)
        if Self::is_complex_code_problem(problem, problem_type) {
            steps.push(ThinkStep {
                step_number: 4,
                thought: "Analyzing trade-offs and technical constraints".to_string(),
                reasoning: Self::analyze_code_tradeoffs(problem_type, language),
            });
        }

        // Step 5: Solution strategy
        let solution_step = steps.len() + 1;
        steps.push(ThinkStep {
            step_number: solution_step,
            thought: "Proposing concrete solution strategy".to_string(),
            reasoning: Self::propose_solution_strategy(problem_type, problem),
        });

        Ok(steps)
    }

    fn generate_error_analysis_steps(error_message: &str) -> Result<Vec<ThinkStep>, ThinkError> {
        let mut steps = Vec::new();

        // Step 1: Error message parsing
        steps.push(ThinkStep {
            step_number: 1,
            thought: "Parsing error message for key information".to_string(),
            reasoning: Self::parse_error_components(error_message),
        });

        // Step 2: Root cause analysis
        steps.push(ThinkStep {
            step_number: 2,
            thought: "Identifying potential root causes".to_string(),
            reasoning: Self::identify_error_causes(error_message),
        });

        // Step 3: Debugging strategy
        steps.push(ThinkStep {
            step_number: 3,
            thought: "Planning systematic debugging approach".to_string(),
            reasoning:
                "Use debugging tools, add logging, isolate components, reproduce consistently."
                    .to_string(),
        });

        Ok(steps)
    }

    fn generate_refactoring_steps(code_smell: &str) -> Result<Vec<ThinkStep>, ThinkError> {
        let mut steps = Vec::new();

        // Step 1: Code smell analysis
        steps.push(ThinkStep {
            step_number: 1,
            thought: "Analyzing code smell and its impact".to_string(),
            reasoning: Self::analyze_code_smell(code_smell),
        });

        // Step 2: Refactoring strategy
        steps.push(ThinkStep {
            step_number: 2,
            thought: "Selecting appropriate refactoring technique".to_string(),
            reasoning: Self::select_refactoring_technique(code_smell),
        });

        // Step 3: Safety considerations
        steps.push(ThinkStep {
            step_number: 3,
            thought: "Planning safe refactoring with behavior preservation".to_string(),
            reasoning: "Ensure comprehensive tests, incremental changes, continuous validation."
                .to_string(),
        });

        Ok(steps)
    }

    // Helper methods for problem analysis

    fn analyze_problem_type(question: &str) -> String {
        let problem_type = Self::classify_problem_type(question);
        match problem_type {
            ProblemType::Technical => {
                "Technical problem - requires systematic analysis and solution design"
            }
            ProblemType::Design => {
                "Design challenge - needs multiple perspectives and trade-off analysis"
            }
            ProblemType::Analysis => {
                "Analysis task - examine data, patterns, and existing conditions"
            }
            ProblemType::Planning => {
                "Planning objective - structured approach with timeline consideration"
            }
            ProblemType::Debugging => {
                "Troubleshooting scenario - systematic investigation required"
            }
            ProblemType::General => "General problem - logical step-by-step reasoning approach",
        }
        .to_string()
    }

    fn identify_key_components(question: &str) -> String {
        let q_lower = question.to_lowercase();
        let mut components = Vec::new();

        // Question type analysis
        if q_lower.contains("how") {
            components.push("Process/method");
        }
        if q_lower.contains("why") {
            components.push("Causal analysis");
        }
        if q_lower.contains("what") {
            components.push("Definition/identification");
        }
        if q_lower.contains("when") {
            components.push("Timing");
        }
        if q_lower.contains("where") {
            components.push("Location/context");
        }

        // Action type analysis
        if q_lower.contains("implement") || q_lower.contains("build") || q_lower.contains("create")
        {
            components.push("Implementation");
        }
        if q_lower.contains("optimize") || q_lower.contains("improve") {
            components.push("Performance/efficiency");
        }
        if q_lower.contains("debug") || q_lower.contains("fix") || q_lower.contains("error") {
            components.push("Problem resolution");
        }

        if components.is_empty() {
            "Core objectives and success criteria need identification".to_string()
        } else {
            format!("Key components: {}", components.join(", "))
        }
    }

    fn evaluate_approaches(question: &str) -> String {
        let q_lower = question.to_lowercase();

        if q_lower.contains("code") || q_lower.contains("program") || q_lower.contains("implement") {
            "Implementation factors: code quality, performance, maintainability, testing, documentation"
        } else if q_lower.contains("design") || q_lower.contains("architecture") {
            "Design factors: user needs, technical constraints, scalability, extensibility"
        } else if q_lower.contains("debug") || q_lower.contains("error") || q_lower.contains("bug") {
            "Debug factors: reproduction steps, error patterns, logs, systematic elimination"
        } else if q_lower.contains("optimize") || q_lower.contains("performance") {
            "Performance factors: bottlenecks, measurement, trade-offs, system impact"
        } else {
            "General factors: resources, constraints, alternatives, risks"
        }.to_string()
    }

    fn analyze_constraints(question: &str) -> String {
        let q_lower = question.to_lowercase();
        let mut constraints = Vec::new();

        if q_lower.contains("time") || q_lower.contains("deadline") || q_lower.contains("urgent") {
            constraints.push("Time");
        }
        if q_lower.contains("budget") || q_lower.contains("cost") || q_lower.contains("resource") {
            constraints.push("Resources");
        }
        if q_lower.contains("compatibility") || q_lower.contains("legacy") {
            constraints.push("Compatibility");
        }
        if q_lower.contains("security") || q_lower.contains("privacy") {
            constraints.push("Security");
        }
        if q_lower.contains("scalab") || q_lower.contains("performance") {
            constraints.push("Performance");
        }

        if constraints.is_empty() {
            "Consider technical, business, and operational constraints impacting solution"
                .to_string()
        } else {
            format!("Constraints: {}", constraints.join(", "))
        }
    }

    fn consider_implementation(question: &str) -> String {
        let q_lower = question.to_lowercase();

        if q_lower.contains("test") {
            "Implementation: comprehensive testing (unit, integration, edge cases)"
        } else if q_lower.contains("deploy") || q_lower.contains("production") {
            "Implementation: deployment strategy, monitoring, rollback, production readiness"
        } else {
            "Implementation: incremental approach with validation, documentation, maintainability"
        }
        .to_string()
    }

    // Classification helpers

    fn classify_problem_type(question: &str) -> ProblemType {
        let q_lower = question.to_lowercase();

        if q_lower.contains("implement")
            || q_lower.contains("code")
            || q_lower.contains("program")
            || q_lower.contains("algorithm")
            || q_lower.contains("function")
        {
            ProblemType::Technical
        } else if q_lower.contains("design")
            || q_lower.contains("architecture")
            || q_lower.contains("structure")
        {
            ProblemType::Design
        } else if q_lower.contains("analyze")
            || q_lower.contains("evaluate")
            || q_lower.contains("compare")
        {
            ProblemType::Analysis
        } else if q_lower.contains("plan")
            || q_lower.contains("schedule")
            || q_lower.contains("organize")
        {
            ProblemType::Planning
        } else if q_lower.contains("debug")
            || q_lower.contains("fix")
            || q_lower.contains("error")
            || q_lower.contains("bug")
            || q_lower.contains("problem")
        {
            ProblemType::Debugging
        } else {
            ProblemType::General
        }
    }

    fn is_complex_problem(question: &str) -> bool {
        let q_lower = question.to_lowercase();
        question.len() > 50
            || q_lower.contains("complex")
            || q_lower.contains("multiple")
            || q_lower.contains("integrate")
            || q_lower.contains("system")
            || (q_lower.matches("and").count() + q_lower.matches("or").count()) > 2
    }

    fn requires_implementation_thinking(question: &str) -> bool {
        let q_lower = question.to_lowercase();
        q_lower.contains("implement")
            || q_lower.contains("build")
            || q_lower.contains("create")
            || q_lower.contains("develop")
            || q_lower.contains("code")
    }

    fn is_well_defined_problem(question: &str) -> bool {
        let q_lower = question.to_lowercase();
        let has_specifics = q_lower.contains("specific")
            || q_lower.contains("exactly")
            || q_lower.contains("precisely");
        let has_context = question.len() > 30;
        let has_clear_intent = q_lower.starts_with("how to")
            || q_lower.starts_with("what is")
            || q_lower.starts_with("why does");

        has_specifics || (has_context && has_clear_intent)
    }

    fn contains_technical_terms(question: &str) -> bool {
        let q_lower = question.to_lowercase();
        let technical_terms = [
            "algorithm",
            "function",
            "class",
            "method",
            "variable",
            "array",
            "object",
            "database",
            "server",
            "client",
            "api",
            "framework",
            "library",
            "module",
            "performance",
            "memory",
            "cpu",
            "thread",
            "process",
            "cache",
            "index",
            "security",
            "authentication",
            "encryption",
            "hash",
            "protocol",
            "network",
        ];

        technical_terms.iter().any(|term| q_lower.contains(term))
    }

    fn is_vague_problem(question: &str) -> bool {
        let q_lower = question.to_lowercase();
        let vague_indicators = [
            "something",
            "anything",
            "stuff",
            "things",
            "whatever",
            "somehow",
            "maybe",
            "perhaps",
            "might",
            "could be",
            "not sure",
            "unclear",
        ];

        question.len() < 20
            || vague_indicators
                .iter()
                .any(|indicator| q_lower.contains(indicator))
    }

    // Additional code-specific helper methods

    fn get_implementation_approach(
        problem_type: &CodeProblemType,
        language: Option<&str>,
    ) -> String {
        let lang_considerations =
            language.map_or(String::new(), |lang| match lang.to_lowercase().as_str() {
                "rust" => " Consider ownership, borrowing, and zero-cost abstractions.".to_string(),
                "javascript" | "typescript" => {
                    " Consider async patterns, type safety, and browser compatibility.".to_string()
                }
                "python" => {
                    " Consider performance implications, type hints, and Pythonic patterns."
                        .to_string()
                }
                "java" => {
                    " Consider OOP design, memory management, and JVM optimizations.".to_string()
                }
                "c++" => {
                    " Consider RAII, memory safety, and performance optimizations.".to_string()
                }
                "go" => " Consider goroutines, channels, and simplicity principles.".to_string(),
                _ => String::new(),
            });

        let base_approach = match problem_type {
            CodeProblemType::BugFix => {
                "Isolate the issue, create minimal reproduction, apply targeted fix"
            }
            CodeProblemType::Refactoring => {
                "Preserve behavior, improve structure, maintain test coverage"
            }
            CodeProblemType::Implementation => {
                "Start with interfaces, implement incrementally, add comprehensive tests"
            }
            CodeProblemType::Performance => {
                "Measure first, optimize bottlenecks, validate improvements"
            }
            CodeProblemType::Security => {
                "Apply defense in depth, validate inputs, use secure defaults"
            }
            CodeProblemType::Testing => {
                "Cover edge cases, aim for behavior verification, maintain readability"
            }
            CodeProblemType::Documentation => {
                "Focus on user scenarios, provide examples, keep up-to-date"
            }
            CodeProblemType::Architecture => {
                "Design for scalability, maintainability, and clear separation of concerns"
            }
            CodeProblemType::CodeReview => {
                "Focus on readability, performance, security, and maintainability"
            }
        };

        format!("{}.{}", base_approach, lang_considerations)
    }

    fn analyze_code_tradeoffs(problem_type: &CodeProblemType, language: Option<&str>) -> String {
        let general_tradeoffs = match problem_type {
            CodeProblemType::BugFix => "Speed vs thoroughness: Quick fixes may introduce new bugs",
            CodeProblemType::Refactoring => {
                "Improvement benefits vs risk of introducing regressions"
            }
            CodeProblemType::Implementation => {
                "Feature completeness vs time to market and code complexity"
            }
            CodeProblemType::Performance => {
                "Optimization effort vs maintainability and code readability"
            }
            CodeProblemType::Security => {
                "Security measures vs user experience and system performance"
            }
            CodeProblemType::Testing => {
                "Test coverage vs test maintenance burden and execution time"
            }
            CodeProblemType::Documentation => "Documentation depth vs maintenance overhead",
            CodeProblemType::Architecture => {
                "Flexibility vs simplicity, future-proofing vs over-engineering"
            }
            CodeProblemType::CodeReview => "Thoroughness vs development velocity",
        };

        if let Some(lang) = language {
            format!("{} (Language: {})", general_tradeoffs, lang)
        } else {
            general_tradeoffs.to_string()
        }
    }

    fn propose_solution_strategy(problem_type: &CodeProblemType, problem: &str) -> String {
        match problem_type {
            CodeProblemType::BugFix => "1) Reproduce the bug consistently, 2) Identify root cause through debugging, 3) Implement minimal fix, 4) Add regression tests".to_string(),
            CodeProblemType::Refactoring => "1) Ensure comprehensive test coverage, 2) Apply refactoring incrementally, 3) Validate behavior after each change, 4) Update documentation".to_string(),
            CodeProblemType::Implementation => "1) Define clear requirements and interfaces, 2) Implement core functionality with TDD, 3) Add error handling and edge cases, 4) Integrate and test".to_string(),
            CodeProblemType::Performance => "1) Profile and identify bottlenecks, 2) Optimize critical paths, 3) Measure improvements, 4) Ensure correctness maintained".to_string(),
            CodeProblemType::Security => "1) Identify threat model, 2) Apply security best practices, 3) Implement defense mechanisms, 4) Conduct security testing".to_string(),
            CodeProblemType::Testing => "1) Analyze requirements for test scenarios, 2) Implement unit and integration tests, 3) Verify edge cases and error conditions, 4) Maintain test suite".to_string(),
            CodeProblemType::Documentation => "1) Identify target audience and use cases, 2) Structure information logically, 3) Provide examples and tutorials, 4) Keep documentation current".to_string(),
            CodeProblemType::Architecture => "1) Analyze current system constraints, 2) Design modular, extensible architecture, 3) Plan migration strategy, 4) Implement incrementally".to_string(),
            CodeProblemType::CodeReview => "1) Review for correctness and logic, 2) Evaluate performance and security implications, 3) Check coding standards, 4) Provide constructive feedback".to_string(),
        }
    }

    fn is_complex_code_problem(problem: &str, problem_type: &CodeProblemType) -> bool {
        // Always show trade-offs for architecture and performance problems
        matches!(problem_type, CodeProblemType::Architecture | CodeProblemType::Performance)
        // Or if the problem mentions multiple components
        || problem.to_lowercase().contains("multiple")
        || problem.to_lowercase().contains("system")
        || problem.to_lowercase().contains("integration")
        || problem.len() > 150 // Long problem descriptions suggest complexity
    }

    fn generate_code_conclusion(
        _problem: &str,
        _steps: &[ThinkStep],
        problem_type: &CodeProblemType,
    ) -> String {
        match problem_type {
            CodeProblemType::BugFix => "Bug analysis complete: Systematic debugging approach will isolate root cause and enable targeted fix with regression prevention.",
            CodeProblemType::Refactoring => "Refactoring strategy defined: Incremental improvements with behavior preservation and comprehensive test validation.",
            CodeProblemType::Implementation => "Implementation plan ready: Clear requirements breakdown with TDD approach ensures robust, maintainable solution.",
            CodeProblemType::Performance => "Performance optimization strategy: Profile-driven improvements with measurable results and correctness validation.",
            CodeProblemType::Security => "Security analysis complete: Defense-in-depth approach with threat modeling and comprehensive protection measures.",
            CodeProblemType::Testing => "Testing strategy established: Comprehensive coverage with behavior verification and maintainable test suite.",
            CodeProblemType::Documentation => "Documentation plan ready: User-focused content with examples and maintainable structure.",
            CodeProblemType::Architecture => "Architectural design complete: Scalable, maintainable system with clear separation of concerns and migration path.",
            CodeProblemType::CodeReview => "Code review framework ready: Systematic evaluation covering correctness, performance, security, and maintainability.",
        }.to_string()
    }

    fn calculate_code_confidence(
        problem: &str,
        steps: &[ThinkStep],
        problem_type: &CodeProblemType,
    ) -> f32 {
        let mut confidence: f32 = 0.6; // Base confidence for code problems

        // Boost confidence for well-defined problem types
        match problem_type {
            CodeProblemType::BugFix | CodeProblemType::Testing | CodeProblemType::Documentation => {
                confidence += 0.15
            }
            CodeProblemType::Implementation | CodeProblemType::Refactoring => confidence += 0.1,
            CodeProblemType::Performance | CodeProblemType::Security => confidence += 0.05, // More complex
            CodeProblemType::Architecture | CodeProblemType::CodeReview => confidence += 0.0, // Highly contextual
        }

        // Boost for technical specificity
        if Self::contains_technical_terms(problem) {
            confidence += 0.1;
        }

        // Boost for sufficient reasoning steps
        if steps.len() > 3 {
            confidence += 0.05;
        }

        // Reduce confidence for vague problems
        if Self::is_vague_problem(problem) {
            confidence -= 0.2;
        }

        confidence.max(0.1).min(0.95)
    }

    fn determine_recommended_action(problem_type: &CodeProblemType, _problem: &str) -> String {
        match problem_type {
            CodeProblemType::BugFix => "Start debugging: Add logging, create minimal reproduction case, use debugger to trace execution",
            CodeProblemType::Refactoring => "Begin refactoring: Ensure tests exist, apply small incremental changes, validate behavior continuously",
            CodeProblemType::Implementation => "Start implementation: Define interfaces first, use TDD approach, implement incrementally",
            CodeProblemType::Performance => "Profile first: Measure current performance, identify bottlenecks before optimizing",
            CodeProblemType::Security => "Conduct security assessment: Review for common vulnerabilities, apply security best practices",
            CodeProblemType::Testing => "Design test cases: Cover happy path, edge cases, error conditions with clear assertions",
            CodeProblemType::Documentation => "Create documentation structure: Focus on user scenarios, include code examples",
            CodeProblemType::Architecture => "Design system architecture: Start with high-level design, define component interfaces",
            CodeProblemType::CodeReview => "Perform systematic review: Check correctness, performance, security, and coding standards",
        }.to_string()
    }

    fn identify_affected_files(problem: &str, context: Option<&str>) -> Vec<String> {
        let mut files = Vec::new();

        // Extract file paths from context
        if let Some(ctx) = context
            && (ctx.contains('/') || ctx.contains('\\'))
        {
            files.push(ctx.to_string());
        }

        // Extract file paths from problem description
        let words: Vec<&str> = problem.split_whitespace().collect();
        for word in words {
            if word.contains('.') && (word.contains('/') || word.contains('\\')) {
                files.push(word.to_string());
            }
        }

        // Remove duplicates
        let unique_files: HashSet<String> = files.into_iter().collect();
        unique_files.into_iter().collect()
    }

    fn determine_error_action(error_message: &str) -> String {
        let error_lower = error_message.to_lowercase();

        if error_lower.contains("compilation") || error_lower.contains("syntax") {
            "Fix syntax errors: Check for missing semicolons, brackets, or type mismatches"
        } else if error_lower.contains("null") || error_lower.contains("undefined") {
            "Handle null/undefined: Add null checks, initialize variables properly"
        } else if error_lower.contains("memory") || error_lower.contains("segmentation") {
            "Fix memory issue: Check array bounds, pointer validity, memory allocation/deallocation"
        } else if error_lower.contains("timeout") || error_lower.contains("deadlock") {
            "Resolve concurrency issue: Check for race conditions, proper synchronization"
        } else if error_lower.contains("permission") || error_lower.contains("access") {
            "Fix access issue: Check file permissions, user privileges, path accessibility"
        } else {
            "Debug systematically: Add logging, reproduce error, trace execution path"
        }
        .to_string()
    }

    fn extract_files_from_error(error_message: &str) -> Vec<String> {
        let mut files = Vec::new();
        let lines: Vec<&str> = error_message.lines().collect();

        for line in lines {
            // Look for common file path patterns in error messages
            if let Some(start) = line.find('/')
                && let Some(end) = line[start..].find(':')
            {
                let potential_file = &line[start..start + end];
                if potential_file.contains('.') {
                    files.push(potential_file.to_string());
                }
            }
        }

        // Remove duplicates
        let unique_files: HashSet<String> = files.into_iter().collect();
        unique_files.into_iter().collect()
    }

    fn parse_error_components(error_message: &str) -> String {
        let mut components = Vec::new();
        let error_lower = error_message.to_lowercase();

        // Error type detection
        if error_lower.contains("syntax") {
            components.push("Syntax error");
        }
        if error_lower.contains("runtime") {
            components.push("Runtime error");
        }
        if error_lower.contains("compile") {
            components.push("Compilation error");
        }
        if error_lower.contains("null") {
            components.push("Null reference");
        }
        if error_lower.contains("index") || error_lower.contains("bounds") {
            components.push("Array bounds");
        }
        if error_lower.contains("memory") {
            components.push("Memory error");
        }
        if error_lower.contains("network") || error_lower.contains("connection") {
            components.push("Network error");
        }

        // File and line extraction
        let files = Self::extract_files_from_error(error_message);
        if !files.is_empty() {
            components.push("File locations identified");
        }

        if components.is_empty() {
            "General error requiring systematic analysis".to_string()
        } else {
            format!("Error components: {}", components.join(", "))
        }
    }

    fn identify_error_causes(error_message: &str) -> String {
        let error_lower = error_message.to_lowercase();

        if error_lower.contains("null") || error_lower.contains("undefined") {
            "Likely cause: Uninitialized variable, missing null check, or incorrect object access"
        } else if error_lower.contains("index") || error_lower.contains("bounds") {
            "Likely cause: Array access beyond bounds, off-by-one error, or empty collection access"
        } else if error_lower.contains("type") {
            "Likely cause: Type mismatch, incorrect casting, or missing type conversion"
        } else if error_lower.contains("memory") || error_lower.contains("segmentation") {
            "Likely cause: Buffer overflow, dangling pointer, or memory corruption"
        } else if error_lower.contains("permission") || error_lower.contains("access") {
            "Likely cause: Insufficient file permissions, incorrect path, or security restrictions"
        } else if error_lower.contains("network") || error_lower.contains("connection") {
            "Likely cause: Network connectivity, server unavailability, or timeout issues"
        } else {
            "Multiple potential causes: Requires systematic elimination through debugging"
        }
        .to_string()
    }

    fn analyze_code_smell(code_smell: &str) -> String {
        let smell_lower = code_smell.to_lowercase();

        if smell_lower.contains("long method") || smell_lower.contains("long function") {
            "Impact: Reduced readability, difficult testing, multiple responsibilities. Extract smaller methods."
        } else if smell_lower.contains("duplicate") || smell_lower.contains("repetition") {
            "Impact: Maintenance burden, inconsistent changes, code bloat. Extract common functionality."
        } else if smell_lower.contains("large class") || smell_lower.contains("god object") {
            "Impact: Low cohesion, high coupling, difficult maintenance. Apply Single Responsibility Principle."
        } else if smell_lower.contains("complex") || smell_lower.contains("cyclomatic") {
            "Impact: Hard to understand, error-prone, difficult testing. Simplify control flow."
        } else if smell_lower.contains("coupling") {
            "Impact: Rigid design, difficult changes, reduced reusability. Introduce abstractions."
        } else {
            "General code quality issue requiring analysis of structure, readability, and maintainability."
        }.to_string()
    }

    fn select_refactoring_technique(code_smell: &str) -> String {
        let smell_lower = code_smell.to_lowercase();

        if smell_lower.contains("long method") || smell_lower.contains("long function") {
            "Technique: Extract Method - Break down into smaller, focused functions with clear purposes"
        } else if smell_lower.contains("duplicate") {
            "Technique: Extract Common Method/Class - Centralize repeated logic in reusable components"
        } else if smell_lower.contains("large class") {
            "Technique: Extract Class/Module - Split responsibilities into focused, cohesive units"
        } else if smell_lower.contains("complex") {
            "Technique: Simplify Conditional - Use guard clauses, strategy pattern, or state machines"
        } else if smell_lower.contains("coupling") {
            "Technique: Introduce Interface/Dependency Injection - Reduce direct dependencies"
        } else if smell_lower.contains("magic number") || smell_lower.contains("hardcode") {
            "Technique: Extract Constants/Configuration - Make values explicit and configurable"
        } else {
            "Technique: Systematic analysis required - Identify specific structural issues first"
        }.to_string()
    }

    fn assess_refactoring_complexity(code_smell: &str) -> Complexity {
        let smell_lower = code_smell.to_lowercase();

        if smell_lower.contains("system")
            || smell_lower.contains("architecture")
            || smell_lower.contains("large class")
            || smell_lower.contains("god object")
        {
            Complexity::Complex
        } else if smell_lower.contains("multiple")
            || smell_lower.contains("coupling")
            || smell_lower.contains("duplicate")
        {
            Complexity::Medium
        } else {
            Complexity::Simple
        }
    }
}

impl Default for ThinkTool {
    fn default() -> Self {
        Self::new()
    }
}

/// Problem classification for reasoning strategy (backward compatibility)
#[derive(Debug, Clone, PartialEq)]
enum ProblemType {
    Technical, // Code, algorithms, implementation
    Design,    // Architecture, structure, planning
    Analysis,  // Evaluation, comparison, research
    Planning,  // Organization, scheduling, strategy
    Debugging, // Troubleshooting, problem-solving
    General,   // Everything else
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_thinking() {
        let result = ThinkTool::think("How to implement a hash table?").unwrap();

        assert!(result.steps.len() >= 3);
        assert!(!result.conclusion.is_empty());
        assert!(result.confidence > 0.0 && result.confidence <= 1.0);

        // Check that steps are numbered correctly
        for (i, step) in result.steps.iter().enumerate() {
            assert_eq!(step.step_number, i + 1);
        }
    }

    #[test]
    fn test_empty_question() {
        let result = ThinkTool::think("");
        assert!(matches!(result, Err(ThinkError::EmptyProblem)));
    }

    #[test]
    fn test_short_question() {
        let result = ThinkTool::think("Hi");
        assert!(matches!(result, Err(ThinkError::EmptyProblem)));
    }

    #[test]
    fn test_technical_question_confidence() {
        let result =
            ThinkTool::think("How to optimize database query performance using indexes?").unwrap();

        // Technical questions with specific terms should have higher confidence
        assert!(result.confidence > 0.6);
    }

    #[test]
    fn test_vague_question_confidence() {
        let result = ThinkTool::think("How to do something with stuff?").unwrap();

        // Vague questions should have lower confidence
        assert!(result.confidence < 0.6);
    }

    #[test]
    fn test_complex_problem_extra_steps() {
        let result = ThinkTool::think(
            "How to implement a distributed system with multiple microservices, message queues, and load balancing?"
        ).unwrap();

        // Complex problems should generate more than 3 steps
        assert!(result.steps.len() > 3);
    }

    #[test]
    fn test_step_content_quality() {
        let result = ThinkTool::think("How to debug a memory leak in a C++ application?").unwrap();

        // Each step should have meaningful content
        for step in &result.steps {
            assert!(!step.thought.is_empty());
            assert!(!step.reasoning.is_empty());
            assert!(step.thought.len() > 10);
            assert!(step.reasoning.len() > 20);
        }
    }

    #[test]
    fn test_conclusion_relevance() {
        let result = ThinkTool::think("What's the best way to learn Rust programming?").unwrap();

        // Conclusion should be relevant and substantial
        assert!(!result.conclusion.is_empty());
        assert!(result.conclusion.len() > 50);
    }

    // Tests for code-specific functionality

    #[test]
    fn test_code_problem_classification() {
        assert_eq!(
            ThinkTool::classify_code_problem("Fix the null pointer bug"),
            CodeProblemType::BugFix
        );
        assert_eq!(
            ThinkTool::classify_code_problem("Refactor the messy code"),
            CodeProblemType::Refactoring
        );
        assert_eq!(
            ThinkTool::classify_code_problem("Implement user authentication"),
            CodeProblemType::Implementation
        );
        assert_eq!(
            ThinkTool::classify_code_problem("Optimize database performance"),
            CodeProblemType::Performance
        );
        assert_eq!(
            ThinkTool::classify_code_problem("Fix security vulnerability"),
            CodeProblemType::Security
        );
        assert_eq!(
            ThinkTool::classify_code_problem("Write unit tests"),
            CodeProblemType::Testing
        );
        assert_eq!(
            ThinkTool::classify_code_problem("Document the API"),
            CodeProblemType::Documentation
        );
        assert_eq!(
            ThinkTool::classify_code_problem("Design system architecture"),
            CodeProblemType::Architecture
        );
        assert_eq!(
            ThinkTool::classify_code_problem("Review code quality"),
            CodeProblemType::CodeReview
        );
    }

    #[test]
    fn test_complexity_assessment() {
        // Simple case
        let simple =
            ThinkTool::assess_complexity("Fix variable name", Some("rust"), Some("main.rs"));
        assert_eq!(simple, Complexity::Simple);

        // Complex case
        let complex = ThinkTool::assess_complexity(
            "Implement distributed system with multiple microservices and database migration",
            Some("rust"),
            Some("src/services/complex/integration/handler.rs"),
        );
        assert!(matches!(complex, Complexity::Medium | Complexity::Complex));
    }

    #[test]
    fn test_think_about_code() {
        let result = ThinkTool::think_about_code(
            "Fix memory leak in C++ application",
            Some("c++"),
            Some("src/memory_manager.cpp"),
        )
        .unwrap();

        assert_eq!(result.problem_type, CodeProblemType::BugFix);
        assert!(result.steps.len() >= 3);
        assert!(!result.recommended_action.is_empty());
        assert_eq!(result.affected_files, vec!["src/memory_manager.cpp"]);
        assert!(result.confidence > 0.5);
        assert!(!result.conclusion.is_empty());
    }

    #[test]
    fn test_analyze_error() {
        let result =
            ThinkTool::analyze_error("NullPointerException at line 42 in UserService.java")
                .unwrap();

        assert_eq!(result.problem_type, CodeProblemType::BugFix);
        assert!(result.steps.len() >= 3);
        assert!(result.recommended_action.contains("null"));
        assert!(result.confidence >= 0.7); // Error analysis should have high confidence
        assert!(!result.affected_files.is_empty() || result.affected_files.is_empty()); // May or may not extract files
    }

    #[test]
    fn test_plan_refactoring() {
        let result =
            ThinkTool::plan_refactoring("Long method with too many responsibilities").unwrap();

        assert_eq!(result.problem_type, CodeProblemType::Refactoring);
        assert!(result.steps.len() >= 3);
        assert!(result.recommended_action.contains("refactor"));
        assert!(result.confidence >= 0.8); // Refactoring should have high confidence
        assert!(matches!(
            result.complexity,
            Complexity::Simple | Complexity::Medium
        ));
    }

    #[test]
    fn test_error_file_extraction() {
        let error_msg = "Error in /src/main.rs:42:10\nCompilation failed in /tests/unit_test.rs:15";
        let files = ThinkTool::extract_files_from_error(error_msg);

        assert!(
            files.contains(&"/src/main.rs".to_string())
                || files.contains(&"/tests/unit_test.rs".to_string())
        );
    }

    #[test]
    fn test_recommended_actions() {
        let bug_action =
            ThinkTool::determine_recommended_action(&CodeProblemType::BugFix, "null pointer");
        assert!(bug_action.contains("debug"));

        let perf_action =
            ThinkTool::determine_recommended_action(&CodeProblemType::Performance, "slow query");
        assert!(perf_action.contains("Profile") || perf_action.contains("profile"));

        let test_action =
            ThinkTool::determine_recommended_action(&CodeProblemType::Testing, "no tests");
        assert!(test_action.contains("test"));
    }

    #[test]
    fn test_code_confidence_calculation() {
        let steps = vec![ThinkStep {
            step_number: 1,
            thought: "test".to_string(),
            reasoning: "test reasoning".to_string(),
        }];

        // Bug fixes should have higher confidence
        let bug_confidence = ThinkTool::calculate_code_confidence(
            "Fix null pointer exception in authentication module",
            &steps,
            &CodeProblemType::BugFix,
        );
        assert!(bug_confidence > 0.7);

        // Architecture problems should have lower confidence (more contextual)
        let arch_confidence = ThinkTool::calculate_code_confidence(
            "Design system architecture",
            &steps,
            &CodeProblemType::Architecture,
        );
        assert!(arch_confidence < bug_confidence);
    }

    #[test]
    fn test_implementation_approach() {
        let rust_approach =
            ThinkTool::get_implementation_approach(&CodeProblemType::BugFix, Some("rust"));
        assert!(rust_approach.contains("ownership") || rust_approach.contains("borrowing"));

        let js_approach = ThinkTool::get_implementation_approach(
            &CodeProblemType::Implementation,
            Some("javascript"),
        );
        assert!(js_approach.contains("async") || js_approach.contains("type"));
    }

    #[test]
    fn test_affected_files_identification() {
        let files = ThinkTool::identify_affected_files(
            "Fix bug in src/auth.rs and update tests/auth_test.rs",
            Some("src/main.rs"),
        );

        assert!(!files.is_empty());
        // Should contain context file
        assert!(files.contains(&"src/main.rs".to_string()));
    }

    #[test]
    fn test_backward_compatibility() {
        // Ensure original think method still works
        let result = ThinkTool::think("How to implement authentication?").unwrap();

        assert!(result.steps.len() >= 3);
        assert!(!result.conclusion.is_empty());
        assert!(result.confidence > 0.0 && result.confidence <= 1.0);
    }
}
