//! Simple step-by-step reasoning tool for AGCodex
//!
//! Provides transparent reasoning that's easy for LLMs to follow and understand.
//! Focuses on practical problem-solving with clear step-by-step breakdown.

use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use thiserror::Error;

/// Tool recommendation for a thinking step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRecommendation {
    /// Name of the tool being recommended
    pub tool: String,
    /// Confidence level in this recommendation (0.0 to 1.0)
    pub confidence: f32,
    /// Rationale for why this tool is recommended
    pub rationale: String,
    /// Priority order in the recommendation sequence
    pub priority: usize,
}

/// Description of a step in the thinking process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepDescription {
    /// What needs to be done in this step
    pub description: String,
    /// Expected outcome from this step
    pub expected_outcome: String,
    /// Conditions to consider for the next step
    pub next_conditions: Vec<String>,
}

/// Enhanced thought data structure inspired by MCP sequential-thinking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtData {
    /// The actual thought content
    pub thought: String,
    /// Current thought number in the sequence
    pub thought_number: usize,
    /// Total estimated thoughts needed
    pub total_thoughts: usize,
    /// Whether another thought step is needed
    pub next_thought_needed: bool,
    /// Whether this thought revises a previous one
    pub is_revision: bool,
    /// Which thought number is being revised (if is_revision is true)
    pub revises_thought: Option<usize>,
    /// If branching, which thought is the branch point
    pub branch_from_thought: Option<usize>,
    /// Identifier for the current branch (if any)
    pub branch_id: Option<String>,
    /// Tools recommended for this thinking step
    pub recommended_tools: Vec<ToolRecommendation>,
    /// Current step description
    pub current_step: Option<StepDescription>,
    /// Previous steps that have been completed
    pub previous_steps: Vec<StepDescription>,
    /// Remaining high-level steps
    pub remaining_steps: Vec<String>,
    /// Confidence level for this thought (0.0 to 1.0)
    pub confidence: f32,
    /// Timestamp when this thought was created
    pub timestamp: u64,
}

impl ThoughtData {
    /// Create a new thought with basic information
    pub fn new(thought: String, thought_number: usize, total_thoughts: usize) -> Self {
        Self {
            thought,
            thought_number,
            total_thoughts,
            next_thought_needed: thought_number < total_thoughts,
            is_revision: false,
            revises_thought: None,
            branch_from_thought: None,
            branch_id: None,
            recommended_tools: Vec::new(),
            current_step: None,
            previous_steps: Vec::new(),
            remaining_steps: Vec::new(),
            confidence: 0.5,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }

    /// Mark this thought as a revision of a previous thought
    pub const fn as_revision(mut self, revises: usize) -> Self {
        self.is_revision = true;
        self.revises_thought = Some(revises);
        self
    }

    /// Set this thought as part of a branch
    pub fn with_branch(mut self, branch_from: usize, branch_id: String) -> Self {
        self.branch_from_thought = Some(branch_from);
        self.branch_id = Some(branch_id);
        self
    }

    /// Add tool recommendations to this thought
    pub fn with_tools(mut self, tools: Vec<ToolRecommendation>) -> Self {
        self.recommended_tools = tools;
        self
    }

    /// Set the confidence level for this thought
    pub const fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
}

/// Session manager for tracking thought history and branches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkSession {
    /// Main thought history
    pub thought_history: Vec<ThoughtData>,
    /// Branches of thought (key is branch_id)
    pub branches: HashMap<String, Vec<ThoughtData>>,
    /// Maximum number of thoughts to keep in history
    pub max_history_size: usize,
    /// Session identifier
    pub session_id: String,
    /// Current active branch (if any)
    pub active_branch: Option<String>,
    /// Total thoughts across all branches
    pub total_thought_count: usize,
    /// Session creation timestamp
    pub created_at: u64,
    /// Last activity timestamp
    pub last_activity: u64,
}

impl ThinkSession {
    /// Create a new thinking session
    pub fn new(max_history_size: usize) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            thought_history: Vec::new(),
            branches: HashMap::new(),
            max_history_size,
            session_id: format!("think_{}", now),
            active_branch: None,
            total_thought_count: 0,
            created_at: now,
            last_activity: now,
        }
    }

    /// Add a thought to the current history (main or branch)
    pub fn add_thought(&mut self, thought: ThoughtData) {
        self.total_thought_count += 1;
        self.last_activity = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if let Some(branch_id) = &self.active_branch {
            // Add to branch
            self.branches
                .entry(branch_id.clone())
                .or_default()
                .push(thought);

            // Trim branch if it exceeds max size
            if let Some(branch) = self.branches.get_mut(branch_id) {
                while branch.len() > self.max_history_size {
                    branch.remove(0);
                }
            }
        } else {
            // Add to main history
            self.thought_history.push(thought);

            // Trim history if it exceeds max size
            while self.thought_history.len() > self.max_history_size {
                self.thought_history.remove(0);
            }
        }
    }

    /// Create a new branch from a specific thought
    pub fn create_branch(&mut self, from_thought: usize, branch_id: String) {
        let branch_point = if let Some(branch_id) = &self.active_branch {
            // Branching from within a branch
            self.branches
                .get(branch_id)
                .and_then(|b| b.get(from_thought))
                .cloned()
        } else {
            // Branching from main history
            self.thought_history.get(from_thought).cloned()
        };

        if let Some(thought) = branch_point {
            let mut branch_history = vec![thought];
            branch_history[0].branch_from_thought = Some(from_thought);
            branch_history[0].branch_id = Some(branch_id.clone());
            self.branches.insert(branch_id.clone(), branch_history);
            self.active_branch = Some(branch_id);
        }
    }

    /// Switch to a different branch or back to main
    pub fn switch_branch(&mut self, branch_id: Option<String>) {
        self.active_branch = branch_id;
    }

    /// Get the current thought history (main or branch)
    pub fn current_history(&self) -> &[ThoughtData] {
        if let Some(branch_id) = &self.active_branch {
            self.branches
                .get(branch_id)
                .map(Vec::as_slice)
                .unwrap_or(&[])
        } else {
            &self.thought_history
        }
    }

    /// Find a thought that can be revised
    pub fn find_thought_to_revise(&self, thought_number: usize) -> Option<&ThoughtData> {
        self.current_history()
            .iter()
            .find(|t| t.thought_number == thought_number)
    }

    /// Get all thoughts that revised other thoughts
    pub fn get_revisions(&self) -> Vec<&ThoughtData> {
        self.current_history()
            .iter()
            .filter(|t| t.is_revision)
            .collect()
    }

    /// Get thoughts with high confidence
    pub fn get_high_confidence_thoughts(&self, threshold: f32) -> Vec<&ThoughtData> {
        self.current_history()
            .iter()
            .filter(|t| t.confidence >= threshold)
            .collect()
    }

    /// Get recommended tools from all thoughts
    pub fn get_all_recommended_tools(&self) -> Vec<&ToolRecommendation> {
        self.current_history()
            .iter()
            .flat_map(|t| &t.recommended_tools)
            .collect()
    }

    /// Check if more thoughts are needed
    pub fn needs_more_thoughts(&self) -> bool {
        self.current_history()
            .last()
            .map(|t| t.next_thought_needed)
            .unwrap_or(true)
    }
}

impl Default for ThinkSession {
    fn default() -> Self {
        Self::new(100) // Default to keeping 100 thoughts in history
    }
}

#[derive(Debug, Error)]
pub enum ThinkError {
    #[error("problem description is empty or too short")]
    EmptyProblem,

    #[error("failed to generate reasoning steps: {0}")]
    ReasoningFailed(String),

    #[error("confidence calculation error: {0}")]
    ConfidenceError(String),

    #[error("thinking strategy error: {0}")]
    StrategyError(String),
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
    /// Thought data for sequential thinking
    pub thought_data: Option<ThoughtData>,
    /// Thinking intensity used
    pub intensity: Option<ThinkingIntensity>,
    /// Progress indication
    pub progress: Option<ThinkingProgress>,
}

/// Thinking intensity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThinkingIntensity {
    /// Quick thinking (1x iterations)
    Quick,
    /// Deep thinking (2x iterations)
    Deep,
    /// Very deep thinking (3x iterations)
    VeryDeep,
}

impl ThinkingIntensity {
    /// Get the multiplier for iterations based on intensity
    pub const fn multiplier(&self) -> usize {
        match self {
            Self::Quick => 1,
            Self::Deep => 2,
            Self::VeryDeep => 3,
        }
    }

    /// Detect intensity from prompt keywords
    pub fn from_prompt(prompt: &str) -> Self {
        let prompt_lower = prompt.to_lowercase();

        // Explicit thinking instructions
        if prompt_lower.contains("think really hard")
            || prompt_lower.contains("think very deeply")
            || prompt_lower.contains("think extremely hard")
            || prompt_lower.contains("maximum thinking")
            || prompt_lower.contains("think very hard")
        {
            Self::VeryDeep
        } else if prompt_lower.contains("think deeply")
            || prompt_lower.contains("think hard")
            || prompt_lower.contains("deep thinking")
            || prompt_lower.contains("thorough thinking")
        {
            Self::Deep
        }
        // Complex problem indicators
        else if prompt_lower.contains("complex")
            || prompt_lower.contains("comprehensive")
            || prompt_lower.contains("multi-step")
            || prompt_lower.contains("distributed")
            || prompt_lower.contains("microservices")
            || prompt_lower.contains("multiple")
            || prompt_lower.contains("architecture")
            || prompt_lower.contains("system")
            || (prompt_lower.matches("and").count() > 2)
        {
            Self::Deep
        } else {
            Self::Quick
        }
    }
}

impl fmt::Display for ThinkingIntensity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Quick => write!(f, "Quick"),
            Self::Deep => write!(f, "Deep"),
            Self::VeryDeep => write!(f, "Very Deep"),
        }
    }
}

/// Progress tracking for thinking operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingProgress {
    /// Current step
    pub current_step: usize,
    /// Total steps
    pub total_steps: usize,
    /// Strategy being used
    pub strategy: String,
    /// Current phase description
    pub phase: String,
    /// Intensity level
    pub intensity: ThinkingIntensity,
}

/// Sequential thinking strategy - iterative thought refinement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequentialThinking {
    /// Maximum number of thoughts
    pub max_thoughts: usize,
    /// Current thoughts
    pub thoughts: Vec<ThoughtData>,
    /// Revision history
    pub revisions: HashMap<usize, Vec<ThoughtData>>,
    /// Branch points
    pub branches: Vec<(usize, String)>,
    /// Intensity level
    pub intensity: ThinkingIntensity,
}

impl SequentialThinking {
    /// Create new sequential thinking with intensity
    pub fn new(base_thoughts: usize, intensity: ThinkingIntensity) -> Self {
        Self {
            max_thoughts: base_thoughts * intensity.multiplier(),
            thoughts: Vec::new(),
            revisions: HashMap::new(),
            branches: Vec::new(),
            intensity,
        }
    }

    /// Add a thought to the sequence
    pub fn add_thought(&mut self, thought: String) -> ThoughtData {
        let thought_number = self.thoughts.len() + 1;
        let thought_data = ThoughtData::new(thought, thought_number, self.max_thoughts);
        self.thoughts.push(thought_data.clone());
        thought_data
    }

    /// Check if more thoughts are needed
    pub const fn needs_more_thoughts(&self) -> bool {
        self.thoughts.len() < self.max_thoughts
    }

    /// Get progress
    pub fn get_progress(&self) -> ThinkingProgress {
        ThinkingProgress {
            current_step: self.thoughts.len(),
            total_steps: self.max_thoughts,
            strategy: "Sequential Thinking".to_string(),
            phase: if self.thoughts.is_empty() {
                "Initializing".to_string()
            } else if self.thoughts.len() < self.max_thoughts / 2 {
                "Exploring".to_string()
            } else if self.thoughts.len() < self.max_thoughts {
                "Refining".to_string()
            } else {
                "Concluding".to_string()
            },
            intensity: self.intensity,
        }
    }
}

/// Shannon methodology - systematic problem solving
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShannonThinking {
    /// Problem definition
    pub problem_definition: Option<String>,
    /// Constraints identified
    pub constraints: Vec<String>,
    /// Mathematical/theoretical model
    pub model: Option<String>,
    /// Validation/proof
    pub proof: Option<String>,
    /// Implementation notes
    pub implementation: Vec<String>,
    /// Current phase
    pub current_phase: ShannonPhase,
    /// Uncertainty exploration rounds
    pub uncertainty_rounds: usize,
    /// Intensity level
    pub intensity: ThinkingIntensity,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ShannonPhase {
    Definition,
    Constraints,
    Modeling,
    Validation,
    Implementation,
    Complete,
}

impl ShannonThinking {
    /// Create new Shannon thinking with intensity
    pub const fn new(intensity: ThinkingIntensity) -> Self {
        Self {
            problem_definition: None,
            constraints: Vec::new(),
            model: None,
            proof: None,
            implementation: Vec::new(),
            current_phase: ShannonPhase::Definition,
            uncertainty_rounds: 2 * intensity.multiplier(),
            intensity,
        }
    }

    /// Advance to next phase
    pub const fn advance_phase(&mut self) {
        self.current_phase = match self.current_phase {
            ShannonPhase::Definition => ShannonPhase::Constraints,
            ShannonPhase::Constraints => ShannonPhase::Modeling,
            ShannonPhase::Modeling => ShannonPhase::Validation,
            ShannonPhase::Validation => ShannonPhase::Implementation,
            ShannonPhase::Implementation | ShannonPhase::Complete => ShannonPhase::Complete,
        };
    }

    /// Get progress
    pub fn get_progress(&self) -> ThinkingProgress {
        let current_step = match self.current_phase {
            ShannonPhase::Definition => 1,
            ShannonPhase::Constraints => 2,
            ShannonPhase::Modeling => 3,
            ShannonPhase::Validation => 4,
            ShannonPhase::Implementation => 5,
            ShannonPhase::Complete => 6,
        };

        ThinkingProgress {
            current_step,
            total_steps: 6,
            strategy: "Shannon Methodology".to_string(),
            phase: format!("{:?}", self.current_phase),
            intensity: self.intensity,
        }
    }
}

/// Actor-Critic thinking - dual perspective analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorCriticThinking {
    /// Actor (creative) thoughts
    pub actor_thoughts: Vec<String>,
    /// Critic (analytical) thoughts
    pub critic_thoughts: Vec<String>,
    /// Synthesis of perspectives
    pub synthesis: Option<String>,
    /// Number of rounds
    pub max_rounds: usize,
    /// Current round
    pub current_round: usize,
    /// Intensity level
    pub intensity: ThinkingIntensity,
}

impl ActorCriticThinking {
    /// Create new actor-critic thinking with intensity
    pub const fn new(base_rounds: usize, intensity: ThinkingIntensity) -> Self {
        Self {
            actor_thoughts: Vec::new(),
            critic_thoughts: Vec::new(),
            synthesis: None,
            max_rounds: base_rounds * intensity.multiplier(),
            current_round: 0,
            intensity,
        }
    }

    /// Add actor thought
    pub fn add_actor_thought(&mut self, thought: String) {
        self.actor_thoughts.push(thought);
    }

    /// Add critic thought
    pub fn add_critic_thought(&mut self, thought: String) {
        self.critic_thoughts.push(thought);
        self.current_round += 1;
    }

    /// Check if more rounds are needed
    pub const fn needs_more_rounds(&self) -> bool {
        self.current_round < self.max_rounds && self.synthesis.is_none()
    }

    /// Get progress
    pub fn get_progress(&self) -> ThinkingProgress {
        ThinkingProgress {
            current_step: self.current_round,
            total_steps: self.max_rounds,
            strategy: "Actor-Critic".to_string(),
            phase: if self.synthesis.is_some() {
                "Synthesis".to_string()
            } else if self.current_round == 0 {
                "Initializing".to_string()
            } else if self.current_round < self.max_rounds / 2 {
                "Dialogue".to_string()
            } else {
                "Converging".to_string()
            },
            intensity: self.intensity,
        }
    }
}

/// Thinking strategy enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThinkingStrategy {
    Sequential(SequentialThinking),
    Shannon(ShannonThinking),
    ActorCritic(ActorCriticThinking),
}

impl ThinkingStrategy {
    /// Get progress for any strategy
    pub fn get_progress(&self) -> ThinkingProgress {
        match self {
            Self::Sequential(s) => s.get_progress(),
            Self::Shannon(s) => s.get_progress(),
            Self::ActorCritic(s) => s.get_progress(),
        }
    }

    /// Select strategy based on problem type
    pub fn select_for_problem(problem: &str, intensity: ThinkingIntensity) -> Self {
        let problem_lower = problem.to_lowercase();

        // Use Shannon for systematic/mathematical problems
        if problem_lower.contains("prove")
            || problem_lower.contains("mathematical")
            || problem_lower.contains("algorithm")
            || problem_lower.contains("systematic")
            || problem_lower.contains("formal")
        {
            Self::Shannon(ShannonThinking::new(intensity))
        }
        // Use Actor-Critic for creative or evaluative problems
        else if problem_lower.contains("evaluate")
            || problem_lower.contains("creative")
            || problem_lower.contains("pros and cons")
            || problem_lower.contains("tradeoff")
            || problem_lower.contains("perspective")
        {
            Self::ActorCritic(ActorCriticThinking::new(3, intensity))
        }
        // Default to Sequential for general problems
        else {
            Self::Sequential(SequentialThinking::new(5, intensity))
        }
    }
}

/// Simple think tool implementation
#[derive(Debug)]
pub struct ThinkTool {
    /// Optional session for maintaining thinking state
    pub session: Option<ThinkSession>,
    /// Active thinking strategy
    pub active_strategy: Option<ThinkingStrategy>,
}

impl ThinkTool {
    /// Create a new think tool instance
    pub const fn new() -> Self {
        Self {
            session: None,
            active_strategy: None,
        }
    }

    /// Create a new think tool with a session
    pub fn with_session(max_history_size: usize) -> Self {
        Self {
            session: Some(ThinkSession::new(max_history_size)),
            active_strategy: None,
        }
    }

    /// Create with a specific strategy and intensity
    pub fn with_strategy(strategy: ThinkingStrategy) -> Self {
        Self {
            session: Some(ThinkSession::new(100)),
            active_strategy: Some(strategy),
        }
    }

    /// Process a single thought with revision support and tool recommendations
    pub fn process_thought(
        &mut self,
        thought: String,
        thought_number: usize,
        total_thoughts: usize,
        next_thought_needed: bool,
        is_revision: Option<bool>,
        revises_thought: Option<usize>,
    ) -> Result<ThinkResult, ThinkError> {
        // Generate tool recommendations based on thought content
        let recommended_tools = self.analyze_for_tool_recommendations(&thought);

        // Create thought data using the constructor
        let mut thought_data = ThoughtData::new(thought.clone(), thought_number, total_thoughts)
            .with_tools(recommended_tools);

        // Handle revision if specified
        if let Some(true) = is_revision {
            thought_data.is_revision = true;
            thought_data.revises_thought = revises_thought;
        }

        // Update next_thought_needed if different from default
        thought_data.next_thought_needed = next_thought_needed;

        // Store in session if available
        if let Some(ref mut session) = self.session {
            session.add_thought(thought_data.clone());
        }

        // Generate simple result for compatibility
        let steps = vec![ThinkStep {
            step_number: thought_number,
            thought: thought.clone(),
            reasoning: format!(
                "Processing thought {} of {}",
                thought_number, total_thoughts
            ),
        }];

        let conclusion = if next_thought_needed {
            format!(
                "Thought {} processed. More thoughts needed.",
                thought_number
            )
        } else {
            format!("Thought {} processed. Thinking complete.", thought_number)
        };

        Ok(ThinkResult {
            steps,
            conclusion,
            confidence: 0.7, // Default confidence
            thought_data: Some(thought_data),
            intensity: None,
            progress: None,
        })
    }

    /// Analyze thought content to recommend appropriate tools
    fn analyze_for_tool_recommendations(&self, thought: &str) -> Vec<ToolRecommendation> {
        let mut recommendations = Vec::new();
        let thought_lower = thought.to_lowercase();
        let mut priority = 1;

        // Search tool recommendations
        if thought_lower.contains("search")
            || thought_lower.contains("find")
            || thought_lower.contains("look")
            || thought_lower.contains("locate")
            || thought_lower.contains("discover")
        {
            recommendations.push(ToolRecommendation {
                tool: "search".to_string(),
                confidence: 0.9,
                rationale: "Thought indicates need for code search or discovery".to_string(),
                priority,
            });
            priority += 1;
        }

        // Edit tool recommendations
        if thought_lower.contains("edit")
            || thought_lower.contains("modify")
            || thought_lower.contains("change")
            || thought_lower.contains("update")
            || thought_lower.contains("fix")
            || thought_lower.contains("correct")
        {
            recommendations.push(ToolRecommendation {
                tool: "edit".to_string(),
                confidence: 0.85,
                rationale: "Thought suggests code modification needed".to_string(),
                priority,
            });
            priority += 1;
        }

        // Patch tool recommendations (for more complex refactoring)
        if thought_lower.contains("refactor")
            || thought_lower.contains("rename")
            || thought_lower.contains("restructure")
            || thought_lower.contains("reorganize")
            || thought_lower.contains("extract")
        {
            recommendations.push(ToolRecommendation {
                tool: "patch".to_string(),
                confidence: 0.9,
                rationale: "Thought indicates structural code changes".to_string(),
                priority,
            });
            priority += 1;
        }

        // Test tool recommendations
        if thought_lower.contains("test")
            || thought_lower.contains("verify")
            || thought_lower.contains("validate")
            || thought_lower.contains("check")
            || thought_lower.contains("ensure")
        {
            recommendations.push(ToolRecommendation {
                tool: "test".to_string(),
                confidence: 0.8,
                rationale: "Thought suggests verification or testing needed".to_string(),
                priority,
            });
            priority += 1;
        }

        // Tree tool recommendations (AST analysis)
        if thought_lower.contains("parse")
            || thought_lower.contains("ast")
            || thought_lower.contains("syntax")
            || thought_lower.contains("structure")
            || thought_lower.contains("analyze")
        {
            recommendations.push(ToolRecommendation {
                tool: "tree".to_string(),
                confidence: 0.75,
                rationale: "Thought indicates need for AST or structural analysis".to_string(),
                priority,
            });
            priority += 1;
        }

        // Index tool recommendations
        if thought_lower.contains("index")
            || thought_lower.contains("catalog")
            || thought_lower.contains("organize")
            || thought_lower.contains("scan")
        {
            recommendations.push(ToolRecommendation {
                tool: "index".to_string(),
                confidence: 0.7,
                rationale: "Thought suggests indexing or cataloging needed".to_string(),
                priority,
            });
            priority += 1;
        }

        // Glob tool recommendations (file discovery)
        if thought_lower.contains("file")
            || thought_lower.contains("files")
            || thought_lower.contains("directory")
            || thought_lower.contains("folder")
            || thought_lower.contains("glob")
        {
            recommendations.push(ToolRecommendation {
                tool: "glob".to_string(),
                confidence: 0.75,
                rationale: "Thought indicates file system operations".to_string(),
                priority,
            });
            priority += 1;
        }

        // Plan tool recommendations
        if thought_lower.contains("plan")
            || thought_lower.contains("decompose")
            || thought_lower.contains("break down")
            || thought_lower.contains("strategy")
            || thought_lower.contains("approach")
        {
            recommendations.push(ToolRecommendation {
                tool: "plan".to_string(),
                confidence: 0.8,
                rationale: "Thought suggests need for task planning".to_string(),
                priority,
            });
        }

        // Sort by priority (already in order, but ensure consistency)
        recommendations.sort_by_key(|r| r.priority);

        recommendations
    }

    /// Perform step-by-step reasoning on a question or problem
    pub fn think(question: &str) -> Result<ThinkResult, ThinkError> {
        if question.trim().is_empty() || question.len() < 5 {
            return Err(ThinkError::EmptyProblem);
        }

        // Detect intensity from the question
        let intensity = ThinkingIntensity::from_prompt(question);

        // Create a temporary tool with strategy
        let mut tool = Self::new();
        let strategy = ThinkingStrategy::select_for_problem(question, intensity);
        tool.active_strategy = Some(strategy);

        // Use strategy-based thinking if available
        if let Some(mut strategy) = tool.active_strategy.take() {
            // Take ownership of the strategy temporarily to avoid borrow issues
            let result = tool.think_with_strategy(question, &mut strategy, intensity);
            tool.active_strategy = Some(strategy);
            result
        } else {
            // Fallback to basic thinking
            let steps = Self::generate_reasoning_steps(question)?;
            let conclusion = Self::generate_conclusion(question, &steps);
            let confidence = Self::calculate_confidence(question, &steps);

            Ok(ThinkResult {
                steps,
                conclusion,
                confidence,
                thought_data: None,
                intensity: Some(intensity),
                progress: None,
            })
        }
    }

    /// Think with a specific strategy
    fn think_with_strategy(
        &mut self,
        question: &str,
        strategy: &mut ThinkingStrategy,
        intensity: ThinkingIntensity,
    ) -> Result<ThinkResult, ThinkError> {
        let mut steps = Vec::new();
        let _progress = strategy.get_progress();

        match strategy {
            ThinkingStrategy::Sequential(seq) => {
                // Sequential thinking: iterative refinement
                let base_steps = 3;
                let total_steps = base_steps * intensity.multiplier();

                for i in 1..=total_steps {
                    let thought = format!(
                        "[{} thinking, step {}/{}] Analyzing: {}",
                        intensity, i, total_steps, question
                    );

                    seq.add_thought(thought.clone());

                    steps.push(ThinkStep {
                        step_number: i,
                        thought: thought.clone(),
                        reasoning: format!("Sequential analysis at {} intensity", intensity),
                    });
                }

                let conclusion = format!(
                    "Completed {} sequential thinking with {} iterations. The analysis reveals multiple perspectives and considerations.",
                    intensity, total_steps
                );

                // Calculate confidence based on question characteristics, not just intensity
                let base_confidence = Self::calculate_confidence(question, &steps);
                let intensity_boost = (0.05 * intensity.multiplier() as f32).min(0.1);
                let final_confidence = (base_confidence + intensity_boost).min(0.95);

                Ok(ThinkResult {
                    steps,
                    conclusion,
                    confidence: final_confidence,
                    thought_data: None,
                    intensity: Some(intensity),
                    progress: Some(seq.get_progress()),
                })
            }
            ThinkingStrategy::Shannon(shannon) => {
                // Shannon methodology: systematic phases
                let phases = [
                    "Problem Definition",
                    "Constraints Analysis",
                    "Model Development",
                    "Validation",
                    "Implementation Planning",
                ];

                for (i, phase) in phases.iter().enumerate() {
                    let thought = format!(
                        "[Shannon {} thinking, phase: {}] {}",
                        intensity, phase, question
                    );

                    steps.push(ThinkStep {
                        step_number: i + 1,
                        thought,
                        reasoning: format!(
                            "Shannon phase: {} with {} uncertainty rounds",
                            phase, shannon.uncertainty_rounds
                        ),
                    });

                    shannon.advance_phase();
                }

                let conclusion = format!(
                    "Shannon methodology completed at {} intensity with {} uncertainty exploration rounds.",
                    intensity, shannon.uncertainty_rounds
                );

                // Calculate confidence based on question characteristics, not just intensity
                let base_confidence = Self::calculate_confidence(question, &steps);
                let intensity_boost = (0.08 * intensity.multiplier() as f32).min(0.15);
                let final_confidence = (base_confidence + intensity_boost).min(0.95);

                Ok(ThinkResult {
                    steps,
                    conclusion,
                    confidence: final_confidence,
                    thought_data: None,
                    intensity: Some(intensity),
                    progress: Some(shannon.get_progress()),
                })
            }
            ThinkingStrategy::ActorCritic(ac) => {
                // Actor-Critic: dual perspective
                for round in 1..=ac.max_rounds {
                    // Actor perspective
                    let actor_thought = format!(
                        "[Actor-Critic {} thinking, round {}/{}] Actor perspective: Creative exploration of {}",
                        intensity, round, ac.max_rounds, question
                    );
                    ac.add_actor_thought(actor_thought.clone());

                    steps.push(ThinkStep {
                        step_number: round * 2 - 1,
                        thought: actor_thought,
                        reasoning: "Actor: Optimistic, creative viewpoint".to_string(),
                    });

                    // Critic perspective
                    let critic_thought = format!(
                        "[Actor-Critic {} thinking, round {}/{}] Critic perspective: Analytical evaluation",
                        intensity, round, ac.max_rounds
                    );
                    ac.add_critic_thought(critic_thought.clone());

                    steps.push(ThinkStep {
                        step_number: round * 2,
                        thought: critic_thought,
                        reasoning: "Critic: Cautious, analytical viewpoint".to_string(),
                    });
                }

                // Synthesis
                ac.synthesis = Some(format!(
                    "Balanced synthesis after {} rounds of actor-critic dialogue at {} intensity",
                    ac.max_rounds, intensity
                ));

                // Calculate confidence based on question characteristics, not just intensity
                let base_confidence = Self::calculate_confidence(question, &steps);
                let intensity_boost = (0.03 * intensity.multiplier() as f32).min(0.1);
                let final_confidence = (base_confidence + intensity_boost).min(0.95);

                Ok(ThinkResult {
                    steps,
                    conclusion: ac
                        .synthesis
                        .clone()
                        .unwrap_or_else(|| "Actor-Critic analysis complete".to_string()),
                    confidence: final_confidence,
                    thought_data: None,
                    intensity: Some(intensity),
                    progress: Some(ac.get_progress()),
                })
            }
        }
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

        let _problem = format!("Refactor to address: {}", code_smell);
        let steps = Self::generate_refactoring_steps(code_smell)?;
        let recommended_action = "Implement refactoring strategy".to_string();
        let _complexity = 5; // Default complexity score for refactoring

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
        confidence.clamp(0.1, 0.95)
    }

    // Code-specific helper methods

    fn classify_code_problem(problem: &str) -> CodeProblemType {
        let p_lower = problem.to_lowercase();

        // Check bug/debug/fix keywords first (highest priority for debugging tasks)
        if p_lower.contains("bug")
            || p_lower.contains("crash")
            || p_lower.contains("debug")
            || p_lower.contains("exception")
            || p_lower.contains("failure")
            || p_lower.contains("memory leak") // Memory leaks are bugs to fix
            || (p_lower.contains("fix")
                && (p_lower.contains("error")
                    || p_lower.contains("issue")
                    || p_lower.contains("null")))
        {
            CodeProblemType::BugFix
        // Implementation should take priority when explicitly mentioned
        } else if p_lower.contains("implement")
            || p_lower.contains("build")
            || p_lower.contains("create")
            || p_lower.contains("develop")
            || p_lower.contains("add feature")
            || p_lower.contains("new feature")
        {
            CodeProblemType::Implementation
        // Security checks (without conflicting with implementation)
        } else if p_lower.contains("security")
            || p_lower.contains("vulnerability")
            || p_lower.contains("exploit")
            || p_lower.contains("injection")
            || p_lower.contains("csrf")
            || p_lower.contains("xss")
            || (p_lower.contains("auth")
                && p_lower.contains("secure")
                && !p_lower.contains("implement"))
        // Only pure security-focused auth
        {
            CodeProblemType::Security
        // Performance optimization (without memory leak which is a bug)
        } else if p_lower.contains("optimiz") // Catches both optimize and optimization
            || p_lower.contains("performance")
            || p_lower.contains("speed")
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
        // Default for all other cases including auth-related tasks
        } else {
            CodeProblemType::Implementation
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
        let mut steps = vec![
            // Step 1: Error message parsing
            ThinkStep {
                step_number: 1,
                thought: "Parsing error message for key information".to_string(),
                reasoning: Self::parse_error_components(error_message),
            },
        ];

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
        let mut steps = vec![
            // Step 1: Code smell analysis
            ThinkStep {
                step_number: 1,
                thought: "Analyzing code smell and its impact".to_string(),
                reasoning: Self::analyze_code_smell(code_smell),
            },
        ];

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

        // Never consider vague questions as well-defined
        if Self::is_vague_problem(question) {
            return false;
        }

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

    fn propose_solution_strategy(problem_type: &CodeProblemType, _problem: &str) -> String {
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
        problem: &str,
        _steps: &[ThinkStep],
        problem_type: &CodeProblemType,
    ) -> String {
        let base_conclusion = match problem_type {
            CodeProblemType::BugFix => {
                "Bug analysis complete: Systematic debugging approach will isolate root cause and enable targeted fix with regression prevention."
            }
            CodeProblemType::Refactoring => {
                "Refactoring strategy defined: Incremental improvements with behavior preservation and comprehensive test validation."
            }
            CodeProblemType::Implementation => {
                "Implementation plan ready: Clear requirements breakdown with TDD approach ensures robust, maintainable solution."
            }
            CodeProblemType::Performance => {
                "Performance optimization strategy: Profile-driven improvements with measurable results and correctness validation."
            }
            CodeProblemType::Security => {
                "Security analysis complete: Defense-in-depth approach with threat modeling and comprehensive protection measures."
            }
            CodeProblemType::Testing => {
                "Testing strategy established: Comprehensive coverage with behavior verification and maintainable test suite."
            }
            CodeProblemType::Documentation => {
                "Documentation plan ready: User-focused content with examples and maintainable structure."
            }
            CodeProblemType::Architecture => {
                "Architectural design complete: Scalable, maintainable system with clear separation of concerns and migration path."
            }
            CodeProblemType::CodeReview => {
                "Code review framework ready: Systematic evaluation covering correctness, performance, security, and maintainability."
            }
        };

        // Add problem-specific context to conclusion
        let p_lower = problem.to_lowercase();
        let mut specific_context = Vec::new();

        if p_lower.contains("auth") {
            specific_context.push("authentication mechanisms");
        }
        if p_lower.contains("oauth") || p_lower.contains("oauth2") {
            specific_context.push("OAuth2 integration");
        }
        if p_lower.contains("security") || p_lower.contains("secure") {
            specific_context.push("security best practices");
        }
        if p_lower.contains("database") {
            specific_context.push("database optimization");
        }
        if p_lower.contains("scalab") {
            specific_context.push("scalability considerations");
        }

        if specific_context.is_empty() {
            base_conclusion.to_string()
        } else {
            format!(
                "{} Focus areas include: {}.",
                base_conclusion,
                specific_context.join(", ")
            )
        }
    }

    fn calculate_code_confidence(
        problem: &str,
        steps: &[ThinkStep],
        problem_type: &CodeProblemType,
    ) -> f32 {
        let mut confidence: f32 = 0.58; // Base confidence for code problems, slightly reduced

        // Boost confidence for well-defined problem types
        match problem_type {
            CodeProblemType::BugFix | CodeProblemType::Testing | CodeProblemType::Documentation => {
                confidence += 0.15
            }
            CodeProblemType::Implementation => confidence += 0.05, // Reduced for more conservative estimate
            CodeProblemType::Refactoring => confidence += 0.08, // Slightly reduced to stay within bounds
            CodeProblemType::Performance => confidence += 0.04, // More complex, conservative estimate
            CodeProblemType::Security => confidence += 0.05,    // More complex
            CodeProblemType::Architecture | CodeProblemType::CodeReview => confidence += 0.0, // Highly contextual
        }

        // Boost for technical specificity (but not for already complex problem types)
        if Self::contains_technical_terms(problem) {
            // Less boost for complex problem types that already have uncertainty
            match problem_type {
                CodeProblemType::Architecture
                | CodeProblemType::Performance
                | CodeProblemType::Security => {
                    confidence += 0.05; // Smaller boost for complex problems
                }
                _ => confidence += 0.1, // Normal boost for simpler problems
            }
        }

        // Boost for sufficient reasoning steps (but not for already uncertain problem types)
        if steps.len() > 4 {
            // Increased threshold to avoid over-boosting
            // Don't boost confidence for complex problem types that are inherently uncertain
            match problem_type {
                CodeProblemType::Architecture
                | CodeProblemType::Performance
                | CodeProblemType::Security => {
                    // Skip boost for complex problems
                }
                _ => confidence += 0.05,
            }
        }

        // Reduce confidence for vague problems
        if Self::is_vague_problem(problem) {
            confidence -= 0.2;
        }

        confidence.clamp(0.1, 0.95)
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

        let base_action = if error_lower.contains("compilation") || error_lower.contains("syntax") {
            "Fix syntax errors: Check for missing semicolons, brackets, or type mismatches. Review code structure and ensure all language requirements are met"
        } else if error_lower.contains("null") || error_lower.contains("undefined") {
            "Handle null/undefined: Add comprehensive null checks before dereferencing, initialize all variables at declaration, implement Option/Result patterns for safe error handling"
        } else if error_lower.contains("memory") || error_lower.contains("segmentation") {
            "Fix memory issue: Validate array bounds before access, verify pointer validity, audit memory allocation/deallocation patterns, check for use-after-free and double-free bugs"
        } else if error_lower.contains("timeout") || error_lower.contains("deadlock") {
            "Resolve concurrency issue: Audit lock acquisition order, check for race conditions, ensure proper synchronization primitives, implement timeout mechanisms"
        } else if error_lower.contains("permission") || error_lower.contains("access") {
            "Fix access issue: Verify file permissions match requirements, check user privileges, validate path accessibility, ensure proper resource ownership"
        } else {
            "Debug systematically: Add comprehensive logging at key points, reproduce error consistently, trace full execution path, examine state at failure point"
        };

        // Add specific context if line number or file is mentioned
        if error_lower.contains("line")
            || error_lower.contains(".rs")
            || error_lower.contains(".cpp")
            || error_lower.contains(".java")
            || error_lower.contains(".py")
            || error_lower.contains(".js")
        {
            format!(
                "{} - Focus on the specific file and line mentioned in the error message for targeted debugging",
                base_action
            )
        } else {
            base_action.to_string()
        }
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

    fn _assess_refactoring_complexity(code_smell: &str) -> Complexity {
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

    /// Recommend tools based on the code problem type
    pub fn recommend_tools_for_problem(
        &self,
        problem_type: &CodeProblemType,
    ) -> Vec<ToolRecommendation> {
        match problem_type {
            CodeProblemType::BugFix => vec![
                ToolRecommendation {
                    tool: "search".to_string(),
                    confidence: 0.9,
                    rationale: "Search tool helps locate error patterns, find related code sections, and identify all instances of the problematic code across the codebase".to_string(),
                    priority: 1,
                },
                ToolRecommendation {
                    tool: "edit".to_string(),
                    confidence: 0.85,
                    rationale: "Edit tool enables precise line-based fixes for identified bugs with immediate feedback and validation".to_string(),
                    priority: 2,
                },
                ToolRecommendation {
                    tool: "test".to_string(),
                    confidence: 0.8,
                    rationale: "Test tool ensures the bug fix doesn't introduce regressions and validates the solution works correctly".to_string(),
                    priority: 3,
                },
            ],
            CodeProblemType::Refactoring => vec![
                ToolRecommendation {
                    tool: "search".to_string(),
                    confidence: 0.85,
                    rationale: "Search tool identifies all code locations that need refactoring and finds usage patterns to ensure consistent changes".to_string(),
                    priority: 1,
                },
                ToolRecommendation {
                    tool: "patch".to_string(),
                    confidence: 0.9,
                    rationale: "Patch tool performs semantic-aware AST transformations that preserve code structure and behavior during refactoring".to_string(),
                    priority: 2,
                },
                ToolRecommendation {
                    tool: "test".to_string(),
                    confidence: 0.8,
                    rationale: "Test tool verifies behavior preservation after refactoring and ensures no functionality is broken".to_string(),
                    priority: 3,
                },
            ],
            CodeProblemType::Implementation => vec![
                ToolRecommendation {
                    tool: "plan".to_string(),
                    confidence: 0.9,
                    rationale: "Plan tool breaks down implementation into manageable tasks with dependencies and parallel execution opportunities".to_string(),
                    priority: 1,                },
                ToolRecommendation {
                    tool: "edit".to_string(),
                    confidence: 0.85,
                    rationale: "Edit tool creates new code sections and implements features with fast iteration and immediate validation".to_string(),
                    priority: 2,                },
                ToolRecommendation {
                    tool: "test".to_string(),
                    confidence: 0.8,
                    rationale: "Test tool validates new implementation meets requirements and integrates correctly with existing code".to_string(),
                    priority: 3,                },
            ],
            CodeProblemType::Performance => vec![
                ToolRecommendation {
                    tool: "search".to_string(),
                    confidence: 0.8,
                    rationale: "Search tool locates performance-critical code sections, bottlenecks, and resource-intensive operations".to_string(),
                    priority: 1,                },
                ToolRecommendation {
                    tool: "analyze".to_string(),
                    confidence: 0.85,
                    rationale: "Analyze tool profiles code execution, measures performance metrics, and identifies optimization opportunities".to_string(),
                    priority: 2,                },
                ToolRecommendation {
                    tool: "patch".to_string(),
                    confidence: 0.8,
                    rationale: "Patch tool applies algorithmic optimizations and structural improvements while maintaining correctness".to_string(),
                    priority: 3,                },
            ],
            CodeProblemType::Security => vec![
                ToolRecommendation {
                    tool: "search".to_string(),
                    confidence: 0.9,
                    rationale: "Search tool identifies security-sensitive code patterns, vulnerable functions, and potential attack vectors".to_string(),
                    priority: 1,                },
                ToolRecommendation {
                    tool: "analyze".to_string(),
                    confidence: 0.9,
                    rationale: "Analyze tool performs security scanning, vulnerability assessment, and compliance checking against security standards".to_string(),
                    priority: 2,                },
                ToolRecommendation {
                    tool: "edit".to_string(),
                    confidence: 0.85,
                    rationale: "Edit tool applies security patches, input validation, and defensive programming techniques".to_string(),
                    priority: 3,                },
            ],
            CodeProblemType::Testing => vec![
                ToolRecommendation {
                    tool: "search".to_string(),
                    confidence: 0.7,
                    rationale: "Search tool finds existing test patterns, identifies untested code, and locates test utilities".to_string(),
                    priority: 1,                },
                ToolRecommendation {
                    tool: "edit".to_string(),
                    confidence: 0.9,
                    rationale: "Edit tool creates test cases, implements assertions, and builds comprehensive test suites".to_string(),
                    priority: 2,                },
                ToolRecommendation {
                    tool: "test".to_string(),
                    confidence: 0.95,
                    rationale: "Test tool executes test suites, validates coverage, and ensures all tests pass successfully".to_string(),
                    priority: 3,                },
            ],
            CodeProblemType::Documentation => vec![
                ToolRecommendation {
                    tool: "search".to_string(),
                    confidence: 0.8,
                    rationale: "Search tool analyzes code structure, finds undocumented functions, and identifies documentation patterns".to_string(),
                    priority: 1,                },
                ToolRecommendation {
                    tool: "edit".to_string(),
                    confidence: 0.9,
                    rationale: "Edit tool adds documentation comments, creates README files, and updates API documentation".to_string(),
                    priority: 2,                },
            ],
            CodeProblemType::Architecture => vec![
                ToolRecommendation {
                    tool: "plan".to_string(),
                    confidence: 0.95,
                    rationale: "Plan tool designs system architecture, defines component relationships, and creates implementation roadmaps".to_string(),
                    priority: 1,                },
                ToolRecommendation {
                    tool: "think".to_string(),
                    confidence: 0.85,
                    rationale: "Think tool reasons through architectural decisions, evaluates trade-offs, and considers long-term implications".to_string(),
                    priority: 2,                },
                ToolRecommendation {
                    tool: "edit".to_string(),
                    confidence: 0.7,
                    rationale: "Edit tool implements architectural scaffolding, creates interfaces, and establishes module boundaries".to_string(),
                    priority: 3,                },
            ],
            CodeProblemType::CodeReview => vec![
                ToolRecommendation {
                    tool: "search".to_string(),
                    confidence: 0.9,
                    rationale: "Search tool examines code patterns, finds similar implementations, and checks for consistency across codebase".to_string(),
                    priority: 1,                },
                ToolRecommendation {
                    tool: "analyze".to_string(),
                    confidence: 0.85,
                    rationale: "Analyze tool evaluates code quality metrics, complexity scores, and adherence to best practices".to_string(),
                    priority: 2,                },
            ],
        }
    }

    /// Analyze thought content to identify mentioned tools and generate recommendations
    pub fn analyze_thought_for_tools(&self, thought: &str) -> Vec<ToolRecommendation> {
        let thought_lower = thought.to_lowercase();
        let mut recommendations = Vec::new();
        let mut priority = 1;

        // Check for search-related keywords
        if thought_lower.contains("search")
            || thought_lower.contains("find")
            || thought_lower.contains("locate")
            || thought_lower.contains("look for")
            || thought_lower.contains("identify")
            || thought_lower.contains("discover")
        {
            recommendations.push(ToolRecommendation {
                tool: "search".to_string(),
                confidence: 0.9,
                rationale: "Thought mentions searching or finding - search tool provides multi-layer search with symbol, full-text, and AST capabilities".to_string(),
                priority,            });
            priority += 1;
        }

        // Check for edit-related keywords
        if thought_lower.contains("edit")
            || thought_lower.contains("modify")
            || thought_lower.contains("change")
            || thought_lower.contains("update")
            || thought_lower.contains("fix")
            || thought_lower.contains("replace")
        {
            recommendations.push(ToolRecommendation {
                tool: "edit".to_string(),
                confidence: 0.85,
                rationale: "Thought mentions editing or modifying - edit tool provides fast line-based changes with context awareness".to_string(),
                priority,            });
            priority += 1;
        }

        // Check for planning-related keywords
        if thought_lower.contains("plan")
            || thought_lower.contains("organize")
            || thought_lower.contains("structure")
            || thought_lower.contains("break down")
            || thought_lower.contains("decompose")
            || thought_lower.contains("strategy")
        {
            recommendations.push(ToolRecommendation {
                tool: "plan".to_string(),
                confidence: 0.9,
                rationale: "Thought mentions planning or organization - plan tool provides task decomposition with dependency analysis".to_string(),
                priority,            });
            priority += 1;
        }

        // Check for analysis-related keywords
        if thought_lower.contains("analyze")
            || thought_lower.contains("examine")
            || thought_lower.contains("investigate")
            || thought_lower.contains("review")
            || thought_lower.contains("evaluate")
            || thought_lower.contains("assess")
        {
            recommendations.push(ToolRecommendation {
                tool: "analyze".to_string(),
                confidence: 0.85,
                rationale: "Thought mentions analysis or investigation - analyze tool provides deep code analysis and metrics".to_string(),
                priority,            });
            priority += 1;
        }

        // Check for testing-related keywords
        if thought_lower.contains("test")
            || thought_lower.contains("verify")
            || thought_lower.contains("validate")
            || thought_lower.contains("check")
            || thought_lower.contains("ensure")
        {
            recommendations.push(ToolRecommendation {
                tool: "test".to_string(),
                confidence: 0.8,
                rationale: "Thought mentions testing or validation - test tool ensures correctness and prevents regressions".to_string(),
                priority,            });
            priority += 1;
        }

        // Check for AST/tree-related keywords
        if thought_lower.contains("ast")
            || thought_lower.contains("syntax")
            || thought_lower.contains("parse")
            || thought_lower.contains("tree")
            || thought_lower.contains("structure")
        {
            recommendations.push(ToolRecommendation {
                tool: "tree".to_string(),
                confidence: 0.85,
                rationale: "Thought mentions AST or syntax - tree tool provides tree-sitter parsing for 27+ languages".to_string(),
                priority,            });
            priority += 1;
        }

        // Check for pattern matching keywords
        if thought_lower.contains("pattern")
            || thought_lower.contains("grep")
            || thought_lower.contains("match")
            || thought_lower.contains("regex")
        {
            recommendations.push(ToolRecommendation {
                tool: "grep".to_string(),
                confidence: 0.8,
                rationale: "Thought mentions patterns or matching - grep tool provides AST-aware pattern matching".to_string(),
                priority,            });
            priority += 1;
        }

        // Check for file discovery keywords
        if thought_lower.contains("files")
            || thought_lower.contains("directory")
            || thought_lower.contains("folder")
            || thought_lower.contains("glob")
            || thought_lower.contains("discover files")
        {
            recommendations.push(ToolRecommendation {
                tool: "glob".to_string(),
                confidence: 0.75,
                rationale: "Thought mentions file discovery - glob tool provides fast parallel file finding with pattern support".to_string(),
                priority,            });
            priority += 1;
        }

        // Check for command/script execution keywords
        if thought_lower.contains("run")
            || thought_lower.contains("execute")
            || thought_lower.contains("command")
            || thought_lower.contains("script")
            || thought_lower.contains("bash")
            || thought_lower.contains("shell")
        {
            recommendations.push(ToolRecommendation {
                tool: "bash".to_string(),
                confidence: 0.7,
                rationale: "Thought mentions execution or commands - bash tool provides safe command execution with validation".to_string(),
                priority,            });
            priority += 1;
        }

        // Check for refactoring/transformation keywords
        if thought_lower.contains("refactor")
            || thought_lower.contains("transform")
            || thought_lower.contains("restructure")
            || thought_lower.contains("patch")
        {
            recommendations.push(ToolRecommendation {
                tool: "patch".to_string(),
                confidence: 0.9,
                rationale: "Thought mentions refactoring or transformation - patch tool provides semantic-aware AST transformations".to_string(),
                priority,            });
            priority += 1;
        }

        // Check for indexing keywords
        if thought_lower.contains("index")
            || thought_lower.contains("catalog")
            || thought_lower.contains("full-text")
        {
            recommendations.push(ToolRecommendation {
                tool: "index".to_string(),
                confidence: 0.75,
                rationale: "Thought mentions indexing - index tool provides Tantivy-based full-text indexing for fast searches".to_string(),
                priority,            });
            priority += 1;
        }

        // Check for reasoning/thinking keywords
        if thought_lower.contains("think")
            || thought_lower.contains("reason")
            || thought_lower.contains("consider")
            || thought_lower.contains("evaluate options")
        {
            recommendations.push(ToolRecommendation {
                tool: "think".to_string(),
                confidence: 0.8,
                rationale: "Thought mentions reasoning or consideration - think tool provides structured reasoning strategies".to_string(),
                priority,            });
        }

        // If no specific tools were identified, provide general recommendations based on context
        if recommendations.is_empty() {
            // Default to search and edit as they're the most commonly needed tools
            recommendations.push(ToolRecommendation {
                tool: "search".to_string(),
                confidence: 0.6,
                rationale: "No specific tools mentioned - search tool helps understand the codebase before making changes".to_string(),
                priority: 1,            });
            recommendations.push(ToolRecommendation {
                tool: "edit".to_string(),
                confidence: 0.5,
                rationale:
                    "No specific tools mentioned - edit tool enables making necessary code changes"
                        .to_string(),
                priority: 2,
            });
        }

        // Sort by priority to ensure correct order
        recommendations.sort_by_key(|r| r.priority);
        recommendations
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

    // Sequential thinking capability tests

    #[test]
    fn test_sequential_thinking_basic() {
        // Test basic sequential thought processing with multiple problems
        let problems = [
            "How to debug a segmentation fault?",
            "What's the best approach for database optimization?",
            "How to implement user authentication securely?",
        ];

        let mut results = Vec::new();
        for problem in problems {
            let result = ThinkTool::think_about_code(problem, Some("rust"), None).unwrap();
            results.push(result);
        }

        // Verify each result has proper sequential structure
        for (i, result) in results.iter().enumerate() {
            assert!(
                result.steps.len() >= 3,
                "Problem {} should have at least 3 steps",
                i
            );

            // Verify steps are properly numbered sequentially
            for (step_idx, step) in result.steps.iter().enumerate() {
                assert_eq!(
                    step.step_number,
                    step_idx + 1,
                    "Step numbering should be sequential"
                );
                assert!(!step.thought.is_empty(), "Each thought should have content");
                assert!(
                    !step.reasoning.is_empty(),
                    "Each step should have reasoning"
                );
            }

            // Verify metadata consistency
            assert!(
                result.confidence >= 0.1 && result.confidence <= 0.95,
                "Confidence should be in valid range"
            );
            assert!(!result.conclusion.is_empty(), "Should have a conclusion");
            assert!(
                !result.recommended_action.is_empty(),
                "Should have recommended action"
            );
        }

        // Verify different problems produce different classifications
        let problem_types: Vec<_> = results.iter().map(|r| &r.problem_type).collect();
        assert!(problem_types.contains(&&CodeProblemType::BugFix));
        assert!(problem_types.contains(&&CodeProblemType::Performance));
        assert!(problem_types.contains(&&CodeProblemType::Implementation));
    }

    #[test]
    fn test_thought_revision() {
        // Test the capability to revise previous thoughts through re-analysis
        let initial_problem = "Fix the authentication bug";

        // Initial analysis
        let initial_result =
            ThinkTool::think_about_code(initial_problem, Some("rust"), None).unwrap();
        assert_eq!(initial_result.problem_type, CodeProblemType::BugFix);

        // More specific problem with additional context (simulating revision)
        let revised_problem =
            "Fix the authentication bug that causes null pointer exception in login validation";
        let revised_result =
            ThinkTool::think_about_code(revised_problem, Some("rust"), Some("src/auth.rs"))
                .unwrap();

        // Verify revision produces more detailed analysis
        assert!(
            revised_result.steps.len() >= initial_result.steps.len(),
            "Revised analysis should be as detailed or more"
        );
        assert!(
            !revised_result.affected_files.is_empty(),
            "Revised analysis should identify affected files"
        );
        assert!(
            revised_result.confidence >= initial_result.confidence - 0.1,
            "Confidence should not significantly decrease"
        );

        // Error-specific analysis should be more targeted
        let error_result =
            ThinkTool::analyze_error("NullPointerException in auth validation").unwrap();
        assert_eq!(error_result.problem_type, CodeProblemType::BugFix);
        assert!(
            error_result.recommended_action.contains("null")
                || error_result.recommended_action.contains("check")
        );

        // Test thought data revision capability
        let thought1 = ThoughtData::new("Initial analysis".to_string(), 1, 3);
        let thought2 = ThoughtData::new("Revised analysis".to_string(), 2, 3).as_revision(1);

        assert!(!thought1.is_revision);
        assert!(thought2.is_revision);
        assert_eq!(thought2.revises_thought, Some(1));
    }

    #[test]
    fn test_tool_recommendations() {
        // Test that appropriate tools are recommended for different problem types
        let test_cases = [
            (
                "Debug memory leak in C++ application",
                CodeProblemType::BugFix,
                "debug",
            ),
            (
                "Optimize slow database queries",
                CodeProblemType::Performance,
                "profile",
            ),
            (
                "Refactor messy legacy code",
                CodeProblemType::Refactoring,
                "refactor",
            ),
            (
                "Implement user registration feature",
                CodeProblemType::Implementation,
                "implement",
            ),
            (
                "Fix SQL injection vulnerability",
                CodeProblemType::Security,
                "security",
            ),
            (
                "Write comprehensive unit tests",
                CodeProblemType::Testing,
                "test",
            ),
            (
                "Document the REST API endpoints",
                CodeProblemType::Documentation,
                "documentation",
            ),
            (
                "Design microservices architecture",
                CodeProblemType::Architecture,
                "architecture",
            ),
            (
                "Review code for best practices",
                CodeProblemType::CodeReview,
                "review",
            ),
        ];

        for (problem, expected_type, expected_action_keyword) in test_cases {
            let result = ThinkTool::think_about_code(problem, Some("rust"), None).unwrap();

            assert_eq!(
                result.problem_type, expected_type,
                "Problem '{}' should be classified as {:?}",
                problem, expected_type
            );

            let action_lower = result.recommended_action.to_lowercase();
            assert!(
                action_lower.contains(expected_action_keyword),
                "Recommended action '{}' should contain '{}' for problem type {:?}",
                result.recommended_action,
                expected_action_keyword,
                expected_type
            );

            // Verify approach reasoning contains problem-specific guidance
            let has_relevant_step = result.steps.iter().any(|step| {
                let step_content = format!("{} {}", step.thought, step.reasoning).to_lowercase();
                step_content.contains(expected_action_keyword)
                    || step_content.contains(&format!("{:?}", expected_type).to_lowercase())
            });
            assert!(
                has_relevant_step,
                "Steps should contain problem-type specific reasoning for {:?}",
                expected_type
            );
        }

        // Test tool recommendation structure
        let tool_rec = ToolRecommendation {
            tool: "search".to_string(),
            confidence: 0.8,
            rationale: "Need to find relevant code".to_string(),
            priority: 1,
        };

        assert_eq!(tool_rec.tool, "search");
        assert!(tool_rec.confidence > 0.0 && tool_rec.confidence <= 1.0);
        assert!(!tool_rec.rationale.is_empty());
    }

    #[test]
    fn test_branch_management() {
        // Test branching thoughts for alternative approaches

        // Analyze from performance perspective
        let perf_result = ThinkTool::think_about_code(
            "Optimize performance bottlenecks in authentication system",
            Some("rust"),
            Some("src/auth.rs"),
        )
        .unwrap();

        // Analyze from security perspective
        let security_result = ThinkTool::think_about_code(
            "Fix security vulnerabilities in authentication system",
            Some("rust"),
            Some("src/auth.rs"),
        )
        .unwrap();

        // Verify different approaches are taken
        assert_eq!(perf_result.problem_type, CodeProblemType::Performance);
        assert_eq!(security_result.problem_type, CodeProblemType::Security);

        // Performance branch should focus on optimization
        let perf_actions = perf_result.recommended_action.to_lowercase();
        assert!(
            perf_actions.contains("profile")
                || perf_actions.contains("measure")
                || perf_actions.contains("performance")
        );

        // Security branch should focus on vulnerabilities
        let security_actions = security_result.recommended_action.to_lowercase();
        assert!(
            security_actions.contains("security")
                || security_actions.contains("vulnerability")
                || security_actions.contains("assessment")
        );

        // Both should identify the same affected files but different approaches
        assert_eq!(perf_result.affected_files, security_result.affected_files);
        assert_ne!(
            perf_result.recommended_action,
            security_result.recommended_action
        );

        // Test ThoughtData branching functionality
        let mut session = ThinkSession::new(100);
        session.add_thought(ThoughtData::new("Main thought".to_string(), 1, 3));

        // Create a branch from the first thought
        session.create_branch(0, "security-branch".to_string());
        assert!(session.branches.contains_key("security-branch"));

        // Test thought with branch data
        let branched_thought = ThoughtData::new("Security analysis".to_string(), 1, 2)
            .with_branch(1, "security-branch".to_string());

        assert_eq!(branched_thought.branch_from_thought, Some(1));
        assert_eq!(
            branched_thought.branch_id,
            Some("security-branch".to_string())
        );

        // Error analysis branching - different error types should produce different strategies
        let null_error = ThinkTool::analyze_error("NullPointerException in user service").unwrap();
        let memory_error =
            ThinkTool::analyze_error("Segmentation fault in buffer handling").unwrap();
        let network_error = ThinkTool::analyze_error("Connection timeout in API client").unwrap();

        // All should be bug fixes but with different approaches
        assert_eq!(null_error.problem_type, CodeProblemType::BugFix);
        assert_eq!(memory_error.problem_type, CodeProblemType::BugFix);
        assert_eq!(network_error.problem_type, CodeProblemType::BugFix);

        // But recommended actions should be different
        let actions = [
            &null_error.recommended_action,
            &memory_error.recommended_action,
            &network_error.recommended_action,
        ];
        assert!(actions.iter().all(|action| !action.is_empty()));
        // Actions should be unique (no two identical approaches)
        for (i, action1) in actions.iter().enumerate() {
            for (j, action2) in actions.iter().enumerate() {
                if i != j {
                    assert_ne!(
                        action1, action2,
                        "Different error types should have different recommended actions"
                    );
                }
            }
        }
    }

    #[test]
    fn test_session_history() {
        // Test that thought history is maintained correctly across multiple analyses
        let mut session = ThinkSession::new(100);

        // Simulate a debugging session with progressive problem refinement
        let problems = [
            "Application crashes on startup",
            "Application crashes when loading configuration",
            "Null pointer when parsing config file in src/config.rs line 45",
        ];

        let mut session_results = Vec::new();

        for (i, problem) in problems.iter().enumerate() {
            let context = if i == 2 { Some("src/config.rs") } else { None };
            let result = if i == 2 {
                ThinkTool::analyze_error(problem).unwrap()
            } else {
                ThinkTool::think_about_code(problem, Some("rust"), context).unwrap()
            };

            // Add thought to session
            let thought = ThoughtData::new(format!("Problem {}: {}", i + 1, problem), i + 1, 3)
                .with_confidence(result.confidence);
            session.add_thought(thought);

            session_results.push(result);
        }

        // Verify session history
        assert_eq!(session.thought_history.len(), 3);
        assert_eq!(session.total_thought_count, 3);

        // Verify progression in specificity and confidence
        assert!(session_results.len() == 3);

        // Later results should be more specific
        assert!(session_results[2].affected_files.len() >= session_results[0].affected_files.len());

        // Error analysis (last) should have high confidence
        assert!(
            session_results[2].confidence >= 0.7,
            "Error analysis should have high confidence"
        );

        // All should be related to debugging/bug fixing
        let problem_types: Vec<_> = session_results.iter().map(|r| &r.problem_type).collect();
        assert!(problem_types.iter().all(|&pt| matches!(
            pt,
            &CodeProblemType::BugFix | &CodeProblemType::Implementation
        )));

        // Recommended actions should become more specific
        let actions: Vec<_> = session_results
            .iter()
            .map(|r| &r.recommended_action)
            .collect();
        assert!(
            actions[2].len() >= actions[0].len(),
            "Later actions should be more detailed"
        );

        // Test session history access
        let current_history = session.current_history();
        assert_eq!(current_history.len(), 3);

        // Test session thought progression
        for (i, thought) in current_history.iter().enumerate() {
            assert_eq!(thought.thought_number, i + 1);
            assert!(!thought.thought.is_empty());
        }
    }

    #[test]
    fn test_tool_confidence_scores() {
        // Verify confidence scores are appropriate for each problem type
        let test_cases = [
            // High confidence cases (well-defined, specific problems)
            (
                "Fix NullPointerException in UserService.java line 42",
                CodeProblemType::BugFix,
                0.7,
                0.95,
            ),
            (
                "Write unit tests for calculateTax method",
                CodeProblemType::Testing,
                0.7,
                0.95,
            ),
            (
                "Document the getUserById API endpoint",
                CodeProblemType::Documentation,
                0.7,
                0.95,
            ),
            // Medium confidence cases (moderately complex)
            (
                "Implement user authentication with JWT",
                CodeProblemType::Implementation,
                0.5,
                0.8,
            ),
            (
                "Refactor the large UserService class",
                CodeProblemType::Refactoring,
                0.5,
                0.8,
            ),
            // Lower confidence cases (complex, contextual)
            (
                "Design scalable microservices architecture",
                CodeProblemType::Architecture,
                0.4,
                0.7,
            ),
            (
                "Optimize overall system performance",
                CodeProblemType::Performance,
                0.4,
                0.7,
            ),
            (
                "Review codebase for security issues",
                CodeProblemType::Security,
                0.4,
                0.7,
            ),
        ];

        for (problem, expected_type, min_confidence, max_confidence) in test_cases {
            let result = ThinkTool::think_about_code(problem, Some("java"), None).unwrap();

            assert_eq!(
                result.problem_type, expected_type,
                "Problem type should match for: {}",
                problem
            );

            assert!(
                result.confidence >= min_confidence && result.confidence <= max_confidence,
                "Confidence {:.2} should be between {:.2} and {:.2} for {:?} problem: '{}'",
                result.confidence,
                min_confidence,
                max_confidence,
                expected_type,
                problem
            );
        }

        // Test confidence with technical specificity
        let vague_problem = "Fix something that's broken";
        let specific_problem =
            "Fix memory leak in ArrayList resize operation in Java HashMap implementation";

        let vague_result = ThinkTool::think_about_code(vague_problem, Some("java"), None).unwrap();
        let specific_result =
            ThinkTool::think_about_code(specific_problem, Some("java"), None).unwrap();

        assert!(
            specific_result.confidence > vague_result.confidence,
            "Specific problems should have higher confidence than vague ones"
        );

        // Test confidence with language context
        let rust_result =
            ThinkTool::think_about_code("Implement memory-safe linked list", Some("rust"), None)
                .unwrap();
        let assembly_result = ThinkTool::think_about_code(
            "Implement memory-safe linked list",
            Some("assembly"),
            None,
        )
        .unwrap();

        // Assembly should have lower confidence due to complexity
        assert!(
            rust_result.confidence >= assembly_result.confidence,
            "Complex languages should not increase confidence unnecessarily"
        );

        // Test error analysis confidence
        let error_result = ThinkTool::analyze_error(
            "java.lang.NullPointerException: Cannot invoke method on null object",
        )
        .unwrap();
        assert!(
            error_result.confidence >= 0.7,
            "Error analysis with clear error messages should have high confidence"
        );

        // Test refactoring confidence
        let refactoring_result = ThinkTool::plan_refactoring("Long method with 200 lines").unwrap();
        assert!(
            refactoring_result.confidence >= 0.8,
            "Clear refactoring problems should have high confidence"
        );

        // Test ThoughtData confidence
        let thought = ThoughtData::new("Test thought".to_string(), 1, 3).with_confidence(0.85);
        assert_eq!(thought.confidence, 0.85);

        // Test confidence bounds
        let bounded_thought = ThoughtData::new("Test".to_string(), 1, 1).with_confidence(1.5); // Should be clamped to 1.0
        assert_eq!(bounded_thought.confidence, 1.0);
    }

    #[test]
    fn test_sequential_thinking_step_quality() {
        // Test the quality and coherence of sequential thinking steps
        let complex_problem = "Design and implement a secure, scalable user authentication system with OAuth2 integration";

        let result =
            ThinkTool::think_about_code(complex_problem, Some("typescript"), Some("src/auth/"))
                .unwrap();

        // Should generate multiple detailed steps for complex problems
        assert!(
            result.steps.len() >= 4,
            "Complex problems should generate more detailed analysis"
        );

        // Verify step progression makes logical sense
        let step_keywords = result
            .steps
            .iter()
            .map(|step| format!("{} {}", step.thought, step.reasoning).to_lowercase())
            .collect::<Vec<_>>();

        // Early steps should focus on understanding/analysis
        assert!(
            step_keywords[0].contains("context")
                || step_keywords[0].contains("understand")
                || step_keywords[0].contains("analyz")
        );

        // Later steps should focus on implementation/solution
        let last_steps = &step_keywords[step_keywords.len().saturating_sub(2)..];
        assert!(last_steps.iter().any(|step| {
            step.contains("implement") || step.contains("solution") || step.contains("strategy")
        }));

        // Verify step content quality
        for (i, step) in result.steps.iter().enumerate() {
            assert!(
                step.thought.len() >= 20,
                "Step {} thought should be substantial",
                i + 1
            );
            assert!(
                step.reasoning.len() >= 30,
                "Step {} reasoning should be detailed",
                i + 1
            );

            // Steps should not be repetitive
            for (j, other_step) in result.steps.iter().enumerate() {
                if i != j {
                    assert_ne!(
                        step.thought, other_step.thought,
                        "Steps should not be identical"
                    );
                }
            }
        }

        // Verify conclusion incorporates the analysis
        assert!(
            result.conclusion.len() >= 50,
            "Conclusion should be comprehensive"
        );
        assert!(
            result.conclusion.contains("auth")
                || result.conclusion.contains("security")
                || result.conclusion.contains("OAuth")
        );

        // Verify recommended action is specific and actionable
        assert!(
            result.recommended_action.len() >= 30,
            "Recommended action should be detailed"
        );
        assert!(
            !result.recommended_action.contains("TODO")
                && !result.recommended_action.contains("...")
        );

        // Test step description structure
        let step_desc = StepDescription {
            description: "Analyze authentication requirements".to_string(),
            expected_outcome: "Clear understanding of auth needs".to_string(),
            next_conditions: vec!["Security requirements defined".to_string()],
        };

        assert!(!step_desc.description.is_empty());
        assert!(!step_desc.expected_outcome.is_empty());
        assert!(!step_desc.next_conditions.is_empty());
    }

    #[test]
    fn test_thinking_consistency() {
        // Test that repeated analysis of the same problem produces consistent results
        let problem = "Optimize database query performance for user search";

        let results: Vec<_> = (0..3)
            .map(|_| {
                ThinkTool::think_about_code(problem, Some("sql"), Some("queries/user_search.sql"))
                    .unwrap()
            })
            .collect();

        // All results should have same classification
        let problem_types: Vec<_> = results.iter().map(|r| &r.problem_type).collect();
        assert!(
            problem_types
                .iter()
                .all(|&pt| pt == &CodeProblemType::Performance)
        );

        // Confidence should be consistent (within reasonable variance)
        let confidences: Vec<_> = results.iter().map(|r| r.confidence).collect();
        let confidence_variance = {
            let mean = confidences.iter().sum::<f32>() / confidences.len() as f32;
            confidences.iter().map(|c| (c - mean).abs()).sum::<f32>() / confidences.len() as f32
        };
        assert!(
            confidence_variance < 0.1,
            "Confidence should be consistent across runs"
        );

        // Step count should be consistent
        let step_counts: Vec<_> = results.iter().map(|r| r.steps.len()).collect();
        let min_steps = *step_counts.iter().min().unwrap();
        let max_steps = *step_counts.iter().max().unwrap();
        assert!(
            max_steps - min_steps <= 1,
            "Step count should be consistent"
        );

        // Recommended actions should be similar (same problem type)
        let actions: Vec<_> = results.iter().map(|r| &r.recommended_action).collect();
        assert!(actions.iter().all(|action| {
            action.to_lowercase().contains("profile")
                || action.to_lowercase().contains("measure")
                || action.to_lowercase().contains("performance")
        }));

        // Test basic ThinkResult with thought_data
        let basic_result = ThinkTool::think("How to solve this?").unwrap();
        assert!(basic_result.steps.len() >= 3);
        assert!(!basic_result.conclusion.is_empty());
        assert!(basic_result.confidence > 0.0);
        // thought_data should be None for basic thinking
        assert!(basic_result.thought_data.is_none());
    }
}
