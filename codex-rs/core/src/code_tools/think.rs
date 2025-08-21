//! Internal reasoning think tool for AGCodex
//!
//! This module provides three complementary reasoning strategies:
//! - Sequential: Step-by-step thinking with revision capabilities
//! - Shannon: Systematic problem-solving methodology inspired by Claude Shannon
//! - Actor-Critic: Dual perspective analysis for balanced evaluation
//!
//! The tool automatically selects the most appropriate strategy based on problem type
//! and provides comprehensive context trails and confidence scoring.

use crate::code_tools::CodeTool;
use crate::code_tools::ToolError;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ThinkError {
    #[error("reasoning strategy not supported: {0}")]
    UnsupportedStrategy(String),

    #[error("invalid confidence score: {0} (must be 0.0-1.0)")]
    InvalidConfidence(f32),

    #[error("thought not found: {0}")]
    ThoughtNotFound(usize),

    #[error("cannot revise non-existent thought: {0}")]
    InvalidRevision(usize),

    #[error("circular dependency in thought chain: {0:?}")]
    CircularDependency(Vec<usize>),

    #[error("reasoning session not initialized")]
    SessionNotInitialized,

    #[error("concurrent modification detected")]
    ConcurrentModification,

    #[error(transparent)]
    Tool(#[from] ToolError),
}

/// Problem types for auto-strategy selection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProblemType {
    /// Multi-step logical reasoning problems
    Sequential,
    /// Systematic engineering or mathematical problems
    Systematic,
    /// Creative problems requiring multiple perspectives
    Creative,
    /// Performance analysis and evaluation
    Evaluation,
    /// Complex problems requiring mixed approaches
    Hybrid,
}

/// Confidence levels for thoughts and reasoning
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    Low,      // 0.0-0.4
    Medium,   // 0.4-0.7
    High,     // 0.7-0.9
    VeryHigh, // 0.9-1.0
}

impl From<f32> for ConfidenceLevel {
    fn from(score: f32) -> Self {
        match score {
            s if s < 0.4 => ConfidenceLevel::Low,
            s if s < 0.7 => ConfidenceLevel::Medium,
            s if s < 0.9 => ConfidenceLevel::High,
            _ => ConfidenceLevel::VeryHigh,
        }
    }
}

/// Context information for each thought
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// Timestamp when thought was created
    pub timestamp: u64,
    /// Related file paths or code references
    pub references: Vec<String>,
    /// Key variables or concepts being considered
    pub variables: HashMap<String, String>,
    /// External dependencies or assumptions
    pub assumptions: Vec<String>,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            references: Vec::new(),
            variables: HashMap::new(),
            assumptions: Vec::new(),
        }
    }
}

/// Alternative approach considered but not taken
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alternative {
    /// Description of the alternative approach
    pub approach: String,
    /// Reason why it wasn't chosen
    pub reason_rejected: String,
    /// Confidence that this was the right choice
    pub confidence: f32,
}

/// Revision information for tracking thought changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Revision {
    /// Original thought content before revision
    pub original_content: String,
    /// Reason for the revision
    pub reason: String,
    /// Timestamp of revision
    pub timestamp: u64,
    /// Version number (starts at 1)
    pub version: u32,
}

/// Branch in thought process for exploring alternatives
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtBranch {
    /// Unique identifier for the branch
    pub id: Uuid,
    /// Point where branch diverged from main line
    pub branch_point: usize,
    /// Thoughts in this branch
    pub thoughts: Vec<usize>,
    /// Branch description
    pub description: String,
    /// Whether this branch was merged back
    pub merged: bool,
}

/// Individual thought in reasoning process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thought {
    /// Step number in sequence
    pub step: usize,
    /// Thought content
    pub content: String,
    /// Confidence level (0.0-1.0)
    pub confidence: f32,
    /// Context and metadata
    pub context: Context,
    /// Alternative approaches considered
    pub alternatives_considered: Vec<Alternative>,
    /// Dependencies on other thoughts
    pub dependencies: Vec<usize>,
    /// Revision history
    pub revisions: Vec<Revision>,
    /// Whether this thought needs further consideration
    pub needs_revision: bool,
}

/// Sequential thinking strategy with revision support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequentialThinking {
    /// All thoughts in order
    pub thoughts: Vec<Thought>,
    /// Revision mapping for quick lookup
    pub revisions: HashMap<usize, Vec<Revision>>,
    /// Active branches for exploring alternatives
    pub branches: Vec<ThoughtBranch>,
    /// Minimum confidence threshold for proceeding
    pub confidence_threshold: f32,
    /// Current active branch (None = main line)
    pub current_branch: Option<Uuid>,
}

impl SequentialThinking {
    pub fn new() -> Self {
        Self {
            thoughts: Vec::new(),
            revisions: HashMap::new(),
            branches: Vec::new(),
            confidence_threshold: 0.7,
            current_branch: None,
        }
    }

    /// Add a new thought to the sequence
    pub fn add_thought(&mut self, content: String, confidence: f32) -> Result<usize, ThinkError> {
        if !(0.0..=1.0).contains(&confidence) {
            return Err(ThinkError::InvalidConfidence(confidence));
        }

        let step = self.thoughts.len();
        let thought = Thought {
            step,
            content,
            confidence,
            context: Context::default(),
            alternatives_considered: Vec::new(),
            dependencies: Vec::new(),
            revisions: Vec::new(),
            needs_revision: confidence < self.confidence_threshold,
        };

        self.thoughts.push(thought);
        Ok(step)
    }

    /// Revise an existing thought
    pub fn revise_thought(
        &mut self,
        step: usize,
        new_content: String,
        reason: String,
    ) -> Result<(), ThinkError> {
        let thought = self
            .thoughts
            .get_mut(step)
            .ok_or(ThinkError::ThoughtNotFound(step))?;

        let revision = Revision {
            original_content: thought.content.clone(),
            reason,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            version: thought.revisions.len() as u32 + 1,
        };

        thought.revisions.push(revision.clone());
        thought.content = new_content;
        thought.needs_revision = false;

        // Update revisions map
        self.revisions.entry(step).or_default().push(revision);

        Ok(())
    }

    /// Create a branch from a specific thought
    pub fn create_branch(
        &mut self,
        branch_point: usize,
        description: String,
    ) -> Result<Uuid, ThinkError> {
        if branch_point >= self.thoughts.len() {
            return Err(ThinkError::ThoughtNotFound(branch_point));
        }

        let branch = ThoughtBranch {
            id: Uuid::new_v4(),
            branch_point,
            thoughts: Vec::new(),
            description,
            merged: false,
        };

        let branch_id = branch.id;
        self.branches.push(branch);
        Ok(branch_id)
    }

    /// Get thoughts that need revision
    pub fn needs_revision(&self) -> Vec<&Thought> {
        self.thoughts.iter().filter(|t| t.needs_revision).collect()
    }

    /// Check for circular dependencies
    pub fn check_dependencies(&self) -> Result<(), ThinkError> {
        for thought in &self.thoughts {
            let mut visited = Vec::new();
            if self.has_circular_dependency(thought.step, &mut visited)? {
                return Err(ThinkError::CircularDependency(visited));
            }
        }
        Ok(())
    }

    fn has_circular_dependency(
        &self,
        step: usize,
        visited: &mut Vec<usize>,
    ) -> Result<bool, ThinkError> {
        if visited.contains(&step) {
            return Ok(true);
        }

        visited.push(step);

        let thought = self
            .thoughts
            .get(step)
            .ok_or(ThinkError::ThoughtNotFound(step))?;

        for &dep in &thought.dependencies {
            if self.has_circular_dependency(dep, visited)? {
                return Ok(true);
            }
        }

        visited.pop();
        Ok(false)
    }
}

/// Shannon's systematic problem-solving approach
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShannonThinking {
    /// Problem definition and constraints
    pub problem_definition: Option<String>,
    /// System constraints and limitations
    pub constraints: Vec<String>,
    /// Mathematical or theoretical model
    pub model: Option<String>,
    /// Proof or validation approach
    pub proof: Option<String>,
    /// Implementation considerations
    pub implementation: Vec<String>,
    /// Current phase of Shannon methodology
    pub current_phase: ShannonPhase,
    /// Confidence in current model
    pub model_confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShannonPhase {
    Definition,
    Constraints,
    Modeling,
    Validation,
    Implementation,
    Complete,
}

impl ShannonThinking {
    pub fn new() -> Self {
        Self {
            problem_definition: None,
            constraints: Vec::new(),
            model: None,
            proof: None,
            implementation: Vec::new(),
            current_phase: ShannonPhase::Definition,
            model_confidence: 0.0,
        }
    }

    /// Advance to next phase if current phase is complete
    pub fn advance_phase(&mut self) -> bool {
        self.current_phase = match self.current_phase {
            ShannonPhase::Definition if self.problem_definition.is_some() => {
                ShannonPhase::Constraints
            }
            ShannonPhase::Constraints if !self.constraints.is_empty() => ShannonPhase::Modeling,
            ShannonPhase::Modeling if self.model.is_some() => ShannonPhase::Validation,
            ShannonPhase::Validation if self.proof.is_some() => ShannonPhase::Implementation,
            ShannonPhase::Implementation if !self.implementation.is_empty() => {
                ShannonPhase::Complete
            }
            _ => return false,
        };
        true
    }

    /// Check if ready to advance to next phase
    pub fn ready_to_advance(&self) -> bool {
        match self.current_phase {
            ShannonPhase::Definition => self.problem_definition.is_some(),
            ShannonPhase::Constraints => !self.constraints.is_empty(),
            ShannonPhase::Modeling => self.model.is_some(),
            ShannonPhase::Validation => self.proof.is_some(),
            ShannonPhase::Implementation => !self.implementation.is_empty(),
            ShannonPhase::Complete => false,
        }
    }
}

/// Actor-Critic thinking for balanced perspective analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorCriticThinking {
    /// Actor perspective (creative, optimistic)
    pub actor_thoughts: Vec<String>,
    /// Critic perspective (analytical, cautious)
    pub critic_thoughts: Vec<String>,
    /// Synthesis of both perspectives
    pub synthesis: Option<String>,
    /// Current active perspective
    pub active_perspective: Perspective,
    /// Confidence in synthesis
    pub synthesis_confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Perspective {
    Actor,
    Critic,
    Synthesis,
}

impl ActorCriticThinking {
    pub fn new() -> Self {
        Self {
            actor_thoughts: Vec::new(),
            critic_thoughts: Vec::new(),
            synthesis: None,
            active_perspective: Perspective::Actor,
            synthesis_confidence: 0.0,
        }
    }

    /// Add thought from actor perspective
    pub fn add_actor_thought(&mut self, thought: String) {
        self.actor_thoughts.push(thought);
    }

    /// Add thought from critic perspective
    pub fn add_critic_thought(&mut self, thought: String) {
        self.critic_thoughts.push(thought);
    }

    /// Switch active perspective
    pub fn switch_perspective(&mut self) {
        self.active_perspective = match self.active_perspective {
            Perspective::Actor => Perspective::Critic,
            Perspective::Critic => Perspective::Actor,
            Perspective::Synthesis => Perspective::Actor,
        };
    }

    /// Generate synthesis from both perspectives
    pub fn generate_synthesis(&mut self, synthesis: String, confidence: f32) {
        self.synthesis = Some(synthesis);
        self.synthesis_confidence = confidence;
        self.active_perspective = Perspective::Synthesis;
    }

    /// Check if ready for synthesis
    pub fn ready_for_synthesis(&self) -> bool {
        !self.actor_thoughts.is_empty() && !self.critic_thoughts.is_empty()
    }
}

/// Query for think tool operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkQuery {
    /// Problem description
    pub problem: String,
    /// Problem type for strategy selection
    pub problem_type: Option<ProblemType>,
    /// Preferred strategy (overrides auto-selection)
    pub preferred_strategy: Option<String>,
    /// Initial context
    pub context: Option<Context>,
    /// Confidence threshold
    pub confidence_threshold: Option<f32>,
}

/// Output from think tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkOutput {
    /// Session ID for continued reasoning
    pub session_id: Uuid,
    /// Selected strategy
    pub strategy: String,
    /// Auto-detected or specified problem type
    pub problem_type: ProblemType,
    /// Current reasoning state
    pub reasoning_state: ReasoningState,
    /// Summary of reasoning so far
    pub summary: String,
    /// Next recommended action
    pub next_action: Option<String>,
    /// Overall confidence
    pub confidence: f32,
    /// LLM-friendly reasoning trace
    pub reasoning_trace: String,
}

/// Current state of reasoning session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningState {
    /// Sequential thinking state
    pub sequential: Option<SequentialThinking>,
    /// Shannon thinking state
    pub shannon: Option<ShannonThinking>,
    /// Actor-critic thinking state
    pub actor_critic: Option<ActorCriticThinking>,
    /// Session metadata
    pub metadata: HashMap<String, String>,
}

/// Main think tool implementation
#[derive(Debug)]
pub struct ThinkTool {
    /// Active reasoning sessions
    sessions: HashMap<Uuid, ReasoningState>,
    /// Default confidence threshold
    default_confidence_threshold: f32,
}

impl Default for ThinkTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ThinkTool {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            default_confidence_threshold: 0.7,
        }
    }

    /// Auto-select strategy based on problem type
    pub fn select_strategy(&self, problem_type: &ProblemType) -> &'static str {
        match problem_type {
            ProblemType::Sequential => "sequential",
            ProblemType::Systematic => "shannon",
            ProblemType::Creative => "actor-critic",
            ProblemType::Evaluation => "actor-critic",
            ProblemType::Hybrid => "sequential", // Start with sequential, can switch
        }
    }

    /// Analyze problem text to detect type
    pub fn detect_problem_type(&self, problem: &str) -> ProblemType {
        let problem_lower = problem.to_lowercase();

        // Look for systematic/engineering keywords
        if problem_lower.contains("algorithm")
            || problem_lower.contains("optimize")
            || problem_lower.contains("mathematical")
            || problem_lower.contains("design")
            || problem_lower.contains("architecture")
        {
            return ProblemType::Systematic;
        }

        // Look for evaluation keywords
        if problem_lower.contains("evaluate")
            || problem_lower.contains("review")
            || problem_lower.contains("compare")
            || problem_lower.contains("analyze")
        {
            return ProblemType::Evaluation;
        }

        // Look for creative keywords
        if problem_lower.contains("creative")
            || problem_lower.contains("brainstorm")
            || problem_lower.contains("innovative")
            || problem_lower.contains("alternative")
        {
            return ProblemType::Creative;
        }

        // Look for sequential keywords
        if problem_lower.contains("step")
            || problem_lower.contains("process")
            || problem_lower.contains("sequence")
            || problem_lower.contains("plan")
        {
            return ProblemType::Sequential;
        }

        // Default to sequential for general problems
        ProblemType::Sequential
    }

    /// Continue reasoning with a specific session
    pub fn continue_reasoning(
        &mut self,
        session_id: Uuid,
        input: String,
    ) -> Result<ThinkOutput, ThinkError> {
        let state = self
            .sessions
            .get_mut(&session_id)
            .ok_or(ThinkError::SessionNotInitialized)?;

        // Determine which strategy is active and continue
        if let Some(sequential) = &mut state.sequential {
            let confidence = 0.8; // Could be calculated based on input
            let step = sequential.add_thought(input, confidence)?;

            let summary = format!(
                "Sequential reasoning: {} thoughts, latest step {}",
                sequential.thoughts.len(),
                step
            );

            // Clone state for generating trace to avoid borrow conflicts
            let state_clone = state.clone();
            let reasoning_trace = Self::generate_reasoning_trace_static(&state_clone);

            Ok(ThinkOutput {
                session_id,
                strategy: "sequential".to_string(),
                problem_type: ProblemType::Sequential, // Could be stored in state
                reasoning_state: state.clone(),
                summary,
                next_action: Some("Continue with next logical step".to_string()),
                confidence,
                reasoning_trace,
            })
        } else if let Some(shannon) = &mut state.shannon {
            // Handle Shannon reasoning continuation
            match shannon.current_phase {
                ShannonPhase::Definition if shannon.problem_definition.is_none() => {
                    shannon.problem_definition = Some(input);
                }
                ShannonPhase::Constraints => {
                    shannon.constraints.push(input);
                }
                ShannonPhase::Modeling if shannon.model.is_none() => {
                    shannon.model = Some(input);
                    shannon.model_confidence = 0.8; // Could be calculated
                }
                ShannonPhase::Validation if shannon.proof.is_none() => {
                    shannon.proof = Some(input);
                }
                ShannonPhase::Implementation => {
                    shannon.implementation.push(input);
                }
                _ => {}
            }

            shannon.advance_phase();

            let summary = format!(
                "Shannon methodology: {:?} phase, model confidence: {:.2}",
                shannon.current_phase, shannon.model_confidence
            );

            let next_action = Self::get_shannon_next_action_static(shannon);
            let confidence = shannon.model_confidence;

            // Clone state for generating trace to avoid borrow conflicts
            let state_clone = state.clone();
            let reasoning_trace = Self::generate_reasoning_trace_static(&state_clone);

            Ok(ThinkOutput {
                session_id,
                strategy: "shannon".to_string(),
                problem_type: ProblemType::Systematic,
                reasoning_state: state.clone(),
                summary,
                next_action,
                confidence,
                reasoning_trace,
            })
        } else if let Some(actor_critic) = &mut state.actor_critic {
            // Handle Actor-Critic reasoning continuation
            match actor_critic.active_perspective {
                Perspective::Actor => {
                    actor_critic.add_actor_thought(input);
                    actor_critic.switch_perspective();
                }
                Perspective::Critic => {
                    actor_critic.add_critic_thought(input);
                    if actor_critic.ready_for_synthesis() {
                        actor_critic.active_perspective = Perspective::Synthesis;
                    } else {
                        actor_critic.switch_perspective();
                    }
                }
                Perspective::Synthesis => {
                    actor_critic.generate_synthesis(input, 0.85);
                }
            }

            let summary = format!(
                "Actor-Critic: {} actor thoughts, {} critic thoughts, synthesis: {}",
                actor_critic.actor_thoughts.len(),
                actor_critic.critic_thoughts.len(),
                actor_critic.synthesis.is_some()
            );

            let next_action = Self::get_actor_critic_next_action_static(actor_critic);
            let confidence = actor_critic.synthesis_confidence;

            // Clone state for generating trace to avoid borrow conflicts
            let state_clone = state.clone();
            let reasoning_trace = Self::generate_reasoning_trace_static(&state_clone);

            Ok(ThinkOutput {
                session_id,
                strategy: "actor-critic".to_string(),
                problem_type: ProblemType::Creative,
                reasoning_state: state.clone(),
                summary,
                next_action,
                confidence,
                reasoning_trace,
            })
        } else {
            Err(ThinkError::SessionNotInitialized)
        }
    }

    fn get_shannon_next_action_static(shannon: &ShannonThinking) -> Option<String> {
        match shannon.current_phase {
            ShannonPhase::Definition => Some("Define the problem clearly".to_string()),
            ShannonPhase::Constraints => Some("Identify constraints and limitations".to_string()),
            ShannonPhase::Modeling => Some("Build mathematical/theoretical model".to_string()),
            ShannonPhase::Validation => Some("Validate model with proof/testing".to_string()),
            ShannonPhase::Implementation => Some("Plan implementation considerations".to_string()),
            ShannonPhase::Complete => None,
        }
    }

    fn get_actor_critic_next_action_static(actor_critic: &ActorCriticThinking) -> Option<String> {
        match actor_critic.active_perspective {
            Perspective::Actor => Some("Provide creative, optimistic perspective".to_string()),
            Perspective::Critic => Some("Provide analytical, cautious perspective".to_string()),
            Perspective::Synthesis => {
                if actor_critic.synthesis.is_none() {
                    Some("Synthesize both perspectives into balanced view".to_string())
                } else {
                    None
                }
            }
        }
    }

    /// Generate LLM-friendly reasoning trace
    fn generate_reasoning_trace_static(state: &ReasoningState) -> String {
        Self::_generate_reasoning_trace(state)
    }

    /// Generate LLM-friendly reasoning trace (instance method)
    fn generate_reasoning_trace(&self, state: &ReasoningState) -> String {
        Self::_generate_reasoning_trace(state)
    }

    /// Internal implementation for reasoning trace generation
    fn _generate_reasoning_trace(state: &ReasoningState) -> String {
        let mut trace = String::new();

        if let Some(sequential) = &state.sequential {
            trace.push_str("## Sequential Reasoning\n\n");
            for (i, thought) in sequential.thoughts.iter().enumerate() {
                trace.push_str(&format!(
                    "**Step {}** (confidence: {:.2}): {}\n\n",
                    i + 1,
                    thought.confidence,
                    thought.content
                ));

                if !thought.alternatives_considered.is_empty() {
                    trace.push_str("*Alternatives considered:*\n");
                    for alt in &thought.alternatives_considered {
                        trace.push_str(&format!("- {}: {}\n", alt.approach, alt.reason_rejected));
                    }
                    trace.push('\n');
                }
            }
        }

        if let Some(shannon) = &state.shannon {
            trace.push_str("## Shannon Methodology\n\n");
            trace.push_str(&format!(
                "**Current Phase:** {:?}\n\n",
                shannon.current_phase
            ));

            if let Some(def) = &shannon.problem_definition {
                trace.push_str(&format!("**Problem Definition:** {}\n\n", def));
            }

            if !shannon.constraints.is_empty() {
                trace.push_str("**Constraints:**\n");
                for constraint in &shannon.constraints {
                    trace.push_str(&format!("- {}\n", constraint));
                }
                trace.push('\n');
            }

            if let Some(model) = &shannon.model {
                trace.push_str(&format!(
                    "**Model** (confidence: {:.2}): {}\n\n",
                    shannon.model_confidence, model
                ));
            }

            if let Some(proof) = &shannon.proof {
                trace.push_str(&format!("**Validation:** {}\n\n", proof));
            }

            if !shannon.implementation.is_empty() {
                trace.push_str("**Implementation Notes:**\n");
                for note in &shannon.implementation {
                    trace.push_str(&format!("- {}\n", note));
                }
                trace.push('\n');
            }
        }

        if let Some(actor_critic) = &state.actor_critic {
            trace.push_str("## Actor-Critic Analysis\n\n");

            if !actor_critic.actor_thoughts.is_empty() {
                trace.push_str("### Actor Perspective (Creative/Optimistic)\n");
                for thought in &actor_critic.actor_thoughts {
                    trace.push_str(&format!("- {}\n", thought));
                }
                trace.push('\n');
            }

            if !actor_critic.critic_thoughts.is_empty() {
                trace.push_str("### Critic Perspective (Analytical/Cautious)\n");
                for thought in &actor_critic.critic_thoughts {
                    trace.push_str(&format!("- {}\n", thought));
                }
                trace.push('\n');
            }

            if let Some(synthesis) = &actor_critic.synthesis {
                trace.push_str(&format!(
                    "### Synthesis (confidence: {:.2})\n{}\n\n",
                    actor_critic.synthesis_confidence, synthesis
                ));
            }
        }

        trace
    }
}

impl CodeTool for ThinkTool {
    type Query = ThinkQuery;
    type Output = ThinkOutput;

    fn search(&self, query: Self::Query) -> Result<Self::Output, ToolError> {
        let problem_type = query
            .problem_type
            .unwrap_or_else(|| self.detect_problem_type(&query.problem));

        let strategy = query
            .preferred_strategy
            .as_deref()
            .unwrap_or_else(|| self.select_strategy(&problem_type));

        let session_id = Uuid::new_v4();
        let confidence_threshold = query
            .confidence_threshold
            .unwrap_or(self.default_confidence_threshold);

        // Initialize appropriate reasoning strategy
        let mut reasoning_state = ReasoningState {
            sequential: None,
            shannon: None,
            actor_critic: None,
            metadata: HashMap::new(),
        };

        match strategy {
            "sequential" => {
                let mut sequential = SequentialThinking::new();
                sequential.confidence_threshold = confidence_threshold;

                // Add initial thought
                sequential
                    .add_thought(query.problem.clone(), 0.8)
                    .map_err(|e| ToolError::InvalidQuery(e.to_string()))?;

                reasoning_state.sequential = Some(sequential);
            }
            "shannon" => {
                let mut shannon = ShannonThinking::new();
                shannon.problem_definition = Some(query.problem.clone());
                reasoning_state.shannon = Some(shannon);
            }
            "actor-critic" => {
                let mut actor_critic = ActorCriticThinking::new();
                actor_critic.add_actor_thought(format!("Initial problem: {}", query.problem));
                reasoning_state.actor_critic = Some(actor_critic);
            }
            _ => {
                return Err(ToolError::InvalidQuery(format!(
                    "Unknown strategy: {}",
                    strategy
                )));
            }
        }

        // Store session (Note: in real implementation, would need mutable reference)
        // For now, we'll create output without storing
        let summary = format!(
            "Initialized {} reasoning for problem type: {:?}",
            strategy, problem_type
        );

        let reasoning_trace = format!(
            "# Reasoning Session Started\n\n**Problem:** {}\n**Strategy:** {}\n**Type:** {:?}\n\n{}",
            query.problem,
            strategy,
            problem_type,
            Self::_generate_reasoning_trace(&reasoning_state)
        );

        Ok(ThinkOutput {
            session_id,
            strategy: strategy.to_string(),
            problem_type,
            reasoning_state,
            summary,
            next_action: Some(match strategy {
                "sequential" => "Add next logical step".to_string(),
                "shannon" => "Define constraints and limitations".to_string(),
                "actor-critic" => "Provide critic perspective".to_string(),
                _ => "Continue reasoning".to_string(),
            }),
            confidence: 0.8,
            reasoning_trace,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequential_thinking() {
        let mut sequential = SequentialThinking::new();

        // Add thoughts
        let step1 = sequential
            .add_thought("First thought".to_string(), 0.8)
            .unwrap();
        let step2 = sequential
            .add_thought("Second thought".to_string(), 0.6)
            .unwrap();

        assert_eq!(step1, 0);
        assert_eq!(step2, 1);
        assert_eq!(sequential.thoughts.len(), 2);

        // Test revision
        sequential
            .revise_thought(
                0,
                "Revised first thought".to_string(),
                "Better clarity".to_string(),
            )
            .unwrap();

        assert_eq!(sequential.thoughts[0].content, "Revised first thought");
        assert_eq!(sequential.thoughts[0].revisions.len(), 1);
    }

    #[test]
    fn test_shannon_thinking() {
        let mut shannon = ShannonThinking::new();

        assert_eq!(shannon.current_phase, ShannonPhase::Definition);
        assert!(!shannon.ready_to_advance());

        shannon.problem_definition = Some("Test problem".to_string());
        assert!(shannon.ready_to_advance());
        assert!(shannon.advance_phase());
        assert_eq!(shannon.current_phase, ShannonPhase::Constraints);
    }

    #[test]
    fn test_actor_critic_thinking() {
        let mut actor_critic = ActorCriticThinking::new();

        actor_critic.add_actor_thought("Optimistic view".to_string());
        actor_critic.add_critic_thought("Cautious analysis".to_string());

        assert!(actor_critic.ready_for_synthesis());
        assert_eq!(actor_critic.actor_thoughts.len(), 1);
        assert_eq!(actor_critic.critic_thoughts.len(), 1);
    }

    #[test]
    fn test_problem_type_detection() {
        let tool = ThinkTool::new();

        assert_eq!(
            tool.detect_problem_type("How to optimize this algorithm?"),
            ProblemType::Systematic
        );

        assert_eq!(
            tool.detect_problem_type("Evaluate the effectiveness of this approach"),
            ProblemType::Evaluation
        );

        assert_eq!(
            tool.detect_problem_type("What creative solutions exist?"),
            ProblemType::Creative
        );

        assert_eq!(
            tool.detect_problem_type("Follow these steps to solve"),
            ProblemType::Sequential
        );
    }

    #[test]
    fn test_think_tool_basic_usage() {
        let tool = ThinkTool::new();

        let query = ThinkQuery {
            problem: "How to implement efficient caching?".to_string(),
            problem_type: Some(ProblemType::Systematic),
            preferred_strategy: None,
            context: None,
            confidence_threshold: None,
        };

        let output = tool.search(query).unwrap();

        assert_eq!(output.strategy, "shannon");
        assert_eq!(output.problem_type, ProblemType::Systematic);
        assert!(output.reasoning_trace.contains("shannon"));
    }
}
