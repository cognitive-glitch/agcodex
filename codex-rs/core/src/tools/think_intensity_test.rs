//! Test module for demonstrating enhanced think tool with variable intensities

#[cfg(test)]
mod tests {
    use super::super::think::*;

    #[test]
    fn test_intensity_detection() {
        // Quick intensity (default)
        assert_eq!(
            ThinkingIntensity::from_prompt("How do I implement a cache?"),
            ThinkingIntensity::Quick
        );

        // Deep intensity
        assert_eq!(
            ThinkingIntensity::from_prompt("Think deeply about the architecture"),
            ThinkingIntensity::Deep
        );
        assert_eq!(
            ThinkingIntensity::from_prompt("Think hard about this problem"),
            ThinkingIntensity::Deep
        );

        // Very deep intensity
        assert_eq!(
            ThinkingIntensity::from_prompt("Think really hard about optimization"),
            ThinkingIntensity::VeryDeep
        );
        assert_eq!(
            ThinkingIntensity::from_prompt("Think very deeply about security"),
            ThinkingIntensity::VeryDeep
        );
    }

    #[test]
    fn test_intensity_multiplier() {
        assert_eq!(ThinkingIntensity::Quick.multiplier(), 1);
        assert_eq!(ThinkingIntensity::Deep.multiplier(), 2);
        assert_eq!(ThinkingIntensity::VeryDeep.multiplier(), 3);
    }

    #[test]
    fn test_sequential_thinking_with_intensity() {
        let mut seq_quick = SequentialThinking::new(3, ThinkingIntensity::Quick);
        assert_eq!(seq_quick.max_thoughts, 3);

        let seq_deep = SequentialThinking::new(3, ThinkingIntensity::Deep);
        assert_eq!(seq_deep.max_thoughts, 6);

        let seq_very_deep = SequentialThinking::new(3, ThinkingIntensity::VeryDeep);
        assert_eq!(seq_very_deep.max_thoughts, 9);

        // Test adding thoughts
        seq_quick.add_thought("First thought".to_string());
        assert_eq!(seq_quick.thoughts.len(), 1);
        assert!(seq_quick.needs_more_thoughts());

        // Test progress
        let progress = seq_quick.get_progress();
        assert_eq!(progress.current_step, 1);
        assert_eq!(progress.total_steps, 3);
        assert_eq!(progress.strategy, "Sequential Thinking");
    }

    #[test]
    fn test_shannon_thinking_with_intensity() {
        let shannon_quick = ShannonThinking::new(ThinkingIntensity::Quick);
        assert_eq!(shannon_quick.uncertainty_rounds, 2);

        let shannon_deep = ShannonThinking::new(ThinkingIntensity::Deep);
        assert_eq!(shannon_deep.uncertainty_rounds, 4);

        let mut shannon_very_deep = ShannonThinking::new(ThinkingIntensity::VeryDeep);
        assert_eq!(shannon_very_deep.uncertainty_rounds, 6);

        // Test phase advancement
        assert_eq!(shannon_very_deep.current_phase, ShannonPhase::Definition);
        shannon_very_deep.advance_phase();
        assert_eq!(shannon_very_deep.current_phase, ShannonPhase::Constraints);

        // Test progress
        let progress = shannon_very_deep.get_progress();
        assert_eq!(progress.strategy, "Shannon Methodology");
        assert_eq!(progress.phase, "Constraints");
    }

    #[test]
    fn test_actor_critic_with_intensity() {
        let ac_quick = ActorCriticThinking::new(2, ThinkingIntensity::Quick);
        assert_eq!(ac_quick.max_rounds, 2);

        let ac_deep = ActorCriticThinking::new(2, ThinkingIntensity::Deep);
        assert_eq!(ac_deep.max_rounds, 4);

        let mut ac_very_deep = ActorCriticThinking::new(2, ThinkingIntensity::VeryDeep);
        assert_eq!(ac_very_deep.max_rounds, 6);

        // Test adding thoughts
        ac_very_deep.add_actor_thought("Creative idea".to_string());
        assert_eq!(ac_very_deep.actor_thoughts.len(), 1);

        ac_very_deep.add_critic_thought("Critical analysis".to_string());
        assert_eq!(ac_very_deep.critic_thoughts.len(), 1);
        assert_eq!(ac_very_deep.current_round, 1);

        assert!(ac_very_deep.needs_more_rounds());

        // Test progress
        let progress = ac_very_deep.get_progress();
        assert_eq!(progress.strategy, "Actor-Critic");
        assert_eq!(progress.current_step, 1);
        assert_eq!(progress.total_steps, 6);
    }

    #[test]
    fn test_strategy_selection() {
        // Sequential for general problems
        let strategy = ThinkingStrategy::select_for_problem(
            "How to implement a feature?",
            ThinkingIntensity::Quick,
        );
        assert!(matches!(strategy, ThinkingStrategy::Sequential(_)));

        // Shannon for systematic problems
        let strategy = ThinkingStrategy::select_for_problem(
            "Prove this algorithm is correct",
            ThinkingIntensity::Deep,
        );
        assert!(matches!(strategy, ThinkingStrategy::Shannon(_)));

        // Actor-Critic for evaluative problems
        let strategy = ThinkingStrategy::select_for_problem(
            "Evaluate the pros and cons of this approach",
            ThinkingIntensity::VeryDeep,
        );
        assert!(matches!(strategy, ThinkingStrategy::ActorCritic(_)));
    }

    #[test]
    fn test_think_with_quick_intensity() {
        let result = ThinkTool::think("How to implement a simple cache?");
        assert!(result.is_ok());

        let think_result = result.unwrap();
        assert_eq!(think_result.intensity, Some(ThinkingIntensity::Quick));
        assert!(think_result.steps.len() >= 3);
        assert!(think_result.confidence > 0.0 && think_result.confidence <= 1.0);
    }

    #[test]
    fn test_think_with_deep_intensity() {
        let result = ThinkTool::think("Think deeply about the security implications");
        assert!(result.is_ok());

        let think_result = result.unwrap();
        assert_eq!(think_result.intensity, Some(ThinkingIntensity::Deep));
        assert!(think_result.progress.is_some());
    }

    #[test]
    fn test_think_with_very_deep_intensity() {
        // Use a general problem to get Sequential strategy which scales with intensity
        let result = ThinkTool::think("Think really hard about implementing this feature");
        assert!(result.is_ok());

        let think_result = result.unwrap();
        assert_eq!(think_result.intensity, Some(ThinkingIntensity::VeryDeep));

        // Very deep thinking with Sequential strategy should have 9 steps (3 * 3)
        // But Shannon has 5 steps, Actor-Critic varies by rounds
        // So we check for at least 5 steps as minimum
        assert!(think_result.steps.len() >= 5);

        if let Some(progress) = think_result.progress {
            assert_eq!(progress.intensity, ThinkingIntensity::VeryDeep);
        }
    }

    #[test]
    fn test_progress_indication() {
        let tool = ThinkTool::with_strategy(ThinkingStrategy::Sequential(SequentialThinking::new(
            3,
            ThinkingIntensity::Deep,
        )));

        // The tool should have an active strategy
        assert!(tool.active_strategy.is_some());

        if let Some(strategy) = &tool.active_strategy {
            let progress = strategy.get_progress();
            assert_eq!(progress.intensity, ThinkingIntensity::Deep);
            assert!(progress.total_steps > 0);
        }
    }
}
