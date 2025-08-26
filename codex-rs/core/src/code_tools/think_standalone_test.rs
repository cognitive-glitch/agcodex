//! Standalone verification of ThinkTool functionality
//! 
//! This demonstrates the think tool works correctly independent of other compilation issues.

#[cfg(test)]
mod standalone_tests {
    use super::super::think::*;

    /// Standalone test showing basic ThinkTool functionality
    #[test]
    fn standalone_think_tool_basic_usage() {
        // Test problem type detection
        let think_tool = ThinkTool::new();
        
        // Test auto-detection of problem types
        assert_eq!(
            think_tool.detect_problem_type("How to optimize this algorithm for better performance?"),
            ProblemType::Systematic
        );
        
        assert_eq!(
            think_tool.detect_problem_type("Evaluate the pros and cons of this approach"),
            ProblemType::Evaluation
        );
        
        assert_eq!(
            think_tool.detect_problem_type("What creative solutions can we brainstorm?"),
            ProblemType::Creative
        );
        
        assert_eq!(
            think_tool.detect_problem_type("Follow these steps to implement the feature"),
            ProblemType::Sequential
        );

        // Test strategy selection
        assert_eq!(think_tool.select_strategy(&ProblemType::Systematic), "shannon");
        assert_eq!(think_tool.select_strategy(&ProblemType::Sequential), "sequential");
        assert_eq!(think_tool.select_strategy(&ProblemType::Creative), "actor-critic");
        assert_eq!(think_tool.select_strategy(&ProblemType::Evaluation), "actor-critic");
    }

    #[test]
    fn standalone_sequential_thinking() {
        let mut sequential = SequentialThinking::new();
        
        // Test adding thoughts
        let step1 = sequential.add_thought("First analysis step".to_string(), 0.8).unwrap();
        let step2 = sequential.add_thought("Second analysis step".to_string(), 0.6).unwrap();
        let step3 = sequential.add_thought("Final conclusion".to_string(), 0.9).unwrap();
        
        assert_eq!(step1, 0);
        assert_eq!(step2, 1);
        assert_eq!(step3, 2);
        assert_eq!(sequential.thoughts.len(), 3);
        
        // Test confidence-based revision detection
        let needs_revision = sequential.needs_revision();
        assert_eq!(needs_revision.len(), 1); // Step 2 has confidence 0.6 < threshold 0.7
        assert_eq!(needs_revision[0].step, 1);
        
        // Test revision
        sequential.revise_thought(
            1, 
            "Improved second analysis step".to_string(), 
            "Added more detail for clarity".to_string()
        ).unwrap();
        
        assert_eq!(sequential.thoughts[1].content, "Improved second analysis step");
        assert_eq!(sequential.thoughts[1].revisions.len(), 1);
        assert_eq!(sequential.revisions.get(&1).unwrap().len(), 1);
        
        // Test branching
        let branch_id = sequential.create_branch(1, "Explore alternative approach".to_string()).unwrap();
        assert_eq!(sequential.branches.len(), 1);
        assert_eq!(sequential.branches[0].id, branch_id);
        assert_eq!(sequential.branches[0].branch_point, 1);
        
        // Test dependency validation (no cycles)
        assert!(sequential.check_dependencies().is_ok());
    }

    #[test]
    fn standalone_shannon_thinking() {
        let mut shannon = ShannonThinking::new();
        
        // Test initial state
        assert_eq!(shannon.current_phase, ShannonPhase::Definition);
        assert!(!shannon.ready_to_advance());
        
        // Test phase progression
        shannon.problem_definition = Some("Design an efficient caching system".to_string());
        assert!(shannon.ready_to_advance());
        assert!(shannon.advance_phase());
        assert_eq!(shannon.current_phase, ShannonPhase::Constraints);
        
        // Add constraints
        shannon.constraints.push("Memory limited to 1GB".to_string());
        shannon.constraints.push("Sub-100ms response time".to_string());
        assert!(shannon.ready_to_advance());
        assert!(shannon.advance_phase());
        assert_eq!(shannon.current_phase, ShannonPhase::Modeling);
        
        // Add model
        shannon.model = Some("LRU cache with bloom filter pre-screening".to_string());
        shannon.model_confidence = 0.85;
        assert!(shannon.ready_to_advance());
        assert!(shannon.advance_phase());
        assert_eq!(shannon.current_phase, ShannonPhase::Validation);
        
        // Add validation
        shannon.proof = Some("Theoretical analysis shows O(1) avg case, benchmarks confirm performance".to_string());
        assert!(shannon.ready_to_advance());
        assert!(shannon.advance_phase());
        assert_eq!(shannon.current_phase, ShannonPhase::Implementation);
        
        // Add implementation notes
        shannon.implementation.push("Use Rust HashMap with custom hasher".to_string());
        shannon.implementation.push("Implement background cleanup task".to_string());
        assert!(shannon.ready_to_advance());
        assert!(shannon.advance_phase());
        assert_eq!(shannon.current_phase, ShannonPhase::Complete);
        
        // Cannot advance further
        assert!(!shannon.ready_to_advance());
        assert!(!shannon.advance_phase());
    }

    #[test]
    fn standalone_actor_critic_thinking() {
        let mut actor_critic = ActorCriticThinking::new();
        
        // Test initial state
        assert_eq!(actor_critic.active_perspective, Perspective::Actor);
        assert!(!actor_critic.ready_for_synthesis());
        
        // Add actor thoughts
        actor_critic.add_actor_thought("We could implement real-time ML-based optimization".to_string());
        actor_critic.add_actor_thought("Add predictive prefetching based on user behavior".to_string());
        assert!(!actor_critic.ready_for_synthesis()); // Need critic thoughts too
        
        // Switch perspective and add critic thoughts
        actor_critic.switch_perspective();
        assert_eq!(actor_critic.active_perspective, Perspective::Critic);
        
        actor_critic.add_critic_thought("ML approach increases complexity and failure modes".to_string());
        actor_critic.add_critic_thought("Predictive prefetching may waste bandwidth on incorrect predictions".to_string());
        
        // Now ready for synthesis
        assert!(actor_critic.ready_for_synthesis());
        
        // Generate synthesis
        actor_critic.generate_synthesis(
            "Balanced approach: Use simple heuristics for prefetching with fallback to ML only for power users".to_string(),
            0.8
        );
        
        assert_eq!(actor_critic.active_perspective, Perspective::Synthesis);
        assert!(actor_critic.synthesis.is_some());
        assert_eq!(actor_critic.synthesis_confidence, 0.8);
        assert_eq!(actor_critic.actor_thoughts.len(), 2);
        assert_eq!(actor_critic.critic_thoughts.len(), 2);
    }

    #[test]
    fn standalone_context_handling() {
        let mut context = Context::default();
        
        // Test context building
        context.references.push("src/cache.rs:45-67".to_string());
        context.references.push("benchmarks/cache_performance.rs".to_string());
        context.variables.insert("cache_size".to_string(), "1GB".to_string());
        context.variables.insert("hit_ratio".to_string(), "85%".to_string());
        context.assumptions.push("Memory is not constrained".to_string());
        context.assumptions.push("Read-heavy workload".to_string());
        
        assert_eq!(context.references.len(), 2);
        assert_eq!(context.variables.len(), 2);
        assert_eq!(context.assumptions.len(), 2);
        assert!(context.timestamp > 0);
    }

    #[test]
    fn standalone_confidence_levels() {
        assert_eq!(ConfidenceLevel::from(0.2), ConfidenceLevel::Low);
        assert_eq!(ConfidenceLevel::from(0.5), ConfidenceLevel::Medium);
        assert_eq!(ConfidenceLevel::from(0.8), ConfidenceLevel::High);
        assert_eq!(ConfidenceLevel::from(0.95), ConfidenceLevel::VeryHigh);
    }

    #[test]
    fn standalone_alternative_tracking() {
        let alternative = Alternative {
            approach: "Use Redis instead of in-memory cache".to_string(),
            reason_rejected: "Adds network latency and external dependency".to_string(),
            confidence: 0.7,
        };
        
        assert_eq!(alternative.approach, "Use Redis instead of in-memory cache");
        assert_eq!(alternative.confidence, 0.7);
    }

    #[test]
    fn standalone_think_query_creation() {
        let mut context = Context::default();
        context.references.push("architecture.md".to_string());
        
        let query = ThinkQuery {
            problem: "How to implement distributed caching across microservices?".to_string(),
            problem_type: Some(ProblemType::Systematic),
            preferred_strategy: Some("shannon".to_string()),
            context: Some(context),
            confidence_threshold: Some(0.8),
        };
        
        assert_eq!(query.problem_type.unwrap(), ProblemType::Systematic);
        assert_eq!(query.preferred_strategy.unwrap(), "shannon");
        assert_eq!(query.confidence_threshold.unwrap(), 0.8);
        assert!(query.context.is_some());
    }

    /// Demo showing complete reasoning workflow
    #[test]
    fn standalone_complete_workflow_demo() {
        // Initialize think tool
        let think_tool = ThinkTool::new();
        
        // Create a complex problem query
        let mut context = Context::default();
        context.references.push("src/performance_bottleneck.rs".to_string());
        context.variables.insert("current_latency".to_string(), "500ms".to_string());
        context.variables.insert("target_latency".to_string(), "50ms".to_string());
        context.assumptions.push("Database queries are the bottleneck".to_string());
        
        let query = ThinkQuery {
            problem: "Our API response time is 500ms, but we need it under 50ms. The bottleneck appears to be database queries. How should we systematically approach this optimization?".to_string(),
            problem_type: None, // Let it auto-detect
            preferred_strategy: None, // Let it auto-select
            context: Some(context),
            confidence_threshold: Some(0.8),
        };
        
        // Process the query
        let output = think_tool.search(query).unwrap();
        
        // Verify auto-selection worked correctly
        assert_eq!(output.strategy, "shannon"); // Should detect systematic problem
        assert_eq!(output.problem_type, ProblemType::Systematic);
        assert!(output.confidence > 0.0);
        assert!(output.reasoning_trace.contains("Reasoning Session Started"));
        assert!(output.reasoning_trace.contains("shannon"));
        assert!(output.next_action.is_some());
        
        // Verify reasoning state was created correctly
        assert!(output.reasoning_state.shannon.is_some());
        assert!(output.reasoning_state.sequential.is_none());
        assert!(output.reasoning_state.actor_critic.is_none());
        
        let shannon_state = output.reasoning_state.shannon.unwrap();
        assert_eq!(shannon_state.current_phase, ShannonPhase::Definition);
        assert!(shannon_state.problem_definition.is_some());
        
        println!("✅ Complete reasoning workflow test passed!");
        println!("Strategy selected: {}", output.strategy);
        println!("Problem type: {:?}", output.problem_type);
        println!("Confidence: {:.2}", output.confidence);
        println!("Session ID: {}", output.session_id);
    }
}

/// Integration demo showing how ThinkTool would be used in AGCodex
pub fn demo_agcodex_integration() {
    println!("🧠 AGCodex ThinkTool Integration Demo");
    println!("=====================================\n");
    
    let think_tool = ThinkTool::new();
    
    // Scenario 1: Code refactoring decision
    println!("📝 Scenario 1: Code Refactoring Decision");
    let refactor_query = ThinkQuery {
        problem: "This function has nested loops and high cyclomatic complexity. Should we refactor it now or optimize performance first?".to_string(),
        problem_type: Some(ProblemType::Evaluation),
        preferred_strategy: None,
        context: None,
        confidence_threshold: None,
    };
    
    match think_tool.search(refactor_query) {
        Ok(output) => {
            println!("   Strategy: {} (confidence: {:.2})", output.strategy, output.confidence);
            println!("   Next action: {}", output.next_action.unwrap_or_else(|| "Continue analysis".to_string()));
        }
        Err(e) => println!("   Error: {}", e),
    }
    
    // Scenario 2: Architecture design
    println!("\n🏗️ Scenario 2: Architecture Design");
    let arch_query = ThinkQuery {
        problem: "Design a real-time notification system that can handle 1M+ concurrent users with sub-second delivery".to_string(),
        problem_type: Some(ProblemType::Systematic),
        preferred_strategy: Some("shannon".to_string()),
        context: None,
        confidence_threshold: Some(0.85),
    };
    
    match think_tool.search(arch_query) {
        Ok(output) => {
            println!("   Strategy: {} (confidence: {:.2})", output.strategy, output.confidence);
            println!("   Problem type: {:?}", output.problem_type);
        }
        Err(e) => println!("   Error: {}", e),
    }
    
    // Scenario 3: Creative problem solving
    println!("\n🎨 Scenario 3: Creative Problem Solving");
    let creative_query = ThinkQuery {
        problem: "How can we make our CLI tool more intuitive for new developers while keeping power users happy?".to_string(),
        problem_type: Some(ProblemType::Creative),
        preferred_strategy: Some("actor-critic".to_string()),
        context: None,
        confidence_threshold: None,
    };
    
    match think_tool.search(creative_query) {
        Ok(output) => {
            println!("   Strategy: {} (confidence: {:.2})", output.strategy, output.confidence);
            println!("   Reasoning approach: Dual perspective analysis");
        }
        Err(e) => println!("   Error: {}", e),
    }
    
    println!("\n✅ All integration scenarios completed successfully!");
}

#[cfg(test)]
mod integration_tests {
    use super::demo_agcodex_integration;
    
    #[test]
    fn test_integration_demo() {
        // This test ensures the integration demo runs without panicking
        demo_agcodex_integration();
    }
}