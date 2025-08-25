//! Example demonstrating the enhanced think tool with variable thinking intensities
//!
//! Run with: cargo run --example think_intensity_demo

use agcodex_core::tools::think::ThinkTool;
use agcodex_core::tools::think::ThinkingIntensity;

fn main() {
    println!("=== AGCodex Enhanced Think Tool Demo ===\n");

    // Example 1: Quick thinking (default)
    println!("1. Quick Thinking Example:");
    println!("   Question: 'How to implement a cache?'");

    match ThinkTool::think("How to implement a cache?") {
        Ok(result) => {
            println!("   Intensity: {:?}", result.intensity);
            println!("   Steps: {}", result.steps.len());
            println!("   Confidence: {:.2}", result.confidence);
            if let Some(progress) = result.progress {
                println!("   Strategy: {}", progress.strategy);
                println!(
                    "   Progress: {}/{} steps",
                    progress.current_step, progress.total_steps
                );
            }
        }
        Err(e) => println!("   Error: {}", e),
    }

    println!("\n2. Deep Thinking Example:");
    println!("   Question: 'Think deeply about the security implications of this design'");

    match ThinkTool::think("Think deeply about the security implications of this design") {
        Ok(result) => {
            println!("   Intensity: {:?}", result.intensity);
            println!("   Steps: {}", result.steps.len());
            println!("   Confidence: {:.2}", result.confidence);
            if let Some(progress) = result.progress {
                println!("   Strategy: {}", progress.strategy);
                println!("   Phase: {}", progress.phase);
                println!(
                    "   Progress: {}/{} steps",
                    progress.current_step, progress.total_steps
                );
            }
            println!("\n   First few steps:");
            for step in result.steps.iter().take(3) {
                println!(
                    "     Step {}: {}",
                    step.step_number,
                    &step.thought[..step.thought.len().min(80)]
                );
            }
        }
        Err(e) => println!("   Error: {}", e),
    }

    println!("\n3. Very Deep Thinking Example:");
    println!("   Question: 'Think really hard about optimizing this algorithm for performance'");

    match ThinkTool::think("Think really hard about optimizing this algorithm for performance") {
        Ok(result) => {
            println!("   Intensity: {:?}", result.intensity);
            println!("   Steps: {}", result.steps.len());
            println!("   Confidence: {:.2}", result.confidence);
            if let Some(progress) = result.progress {
                println!("   Strategy: {}", progress.strategy);
                println!("   Phase: {}", progress.phase);
                println!(
                    "   Progress: {}/{} steps",
                    progress.current_step, progress.total_steps
                );
            }
            println!("\n   Strategy reasoning:");
            for (i, step) in result.steps.iter().enumerate().take(2) {
                println!("     {}: {}", i + 1, step.reasoning);
            }
        }
        Err(e) => println!("   Error: {}", e),
    }

    println!("\n4. Mathematical Problem (Shannon Strategy):");
    println!("   Question: 'Think hard to prove this algorithm is O(n log n)'");

    match ThinkTool::think("Think hard to prove this algorithm is O(n log n)") {
        Ok(result) => {
            println!("   Intensity: {:?}", result.intensity);
            println!("   Strategy: Shannon Methodology");
            println!("   Steps: {}", result.steps.len());
            println!("   Confidence: {:.2}", result.confidence);
            if let Some(progress) = result.progress {
                println!("   Current Phase: {}", progress.phase);
            }
        }
        Err(e) => println!("   Error: {}", e),
    }

    println!("\n5. Evaluative Problem (Actor-Critic Strategy):");
    println!("   Question: 'Think deeply about the pros and cons of microservices'");

    match ThinkTool::think("Think deeply about the pros and cons of microservices") {
        Ok(result) => {
            println!("   Intensity: {:?}", result.intensity);
            println!("   Strategy: Actor-Critic");
            println!("   Steps: {}", result.steps.len());
            println!("   Confidence: {:.2}", result.confidence);
            if let Some(progress) = result.progress {
                println!(
                    "   Dialog Rounds: {}/{}",
                    progress.current_step, progress.total_steps
                );
            }
        }
        Err(e) => println!("   Error: {}", e),
    }

    println!("\n=== Intensity Detection Examples ===");

    let test_prompts = vec![
        ("Simple question", ThinkingIntensity::Quick),
        ("Think hard about this", ThinkingIntensity::Deep),
        ("Think deeply about architecture", ThinkingIntensity::Deep),
        ("Think really hard about this", ThinkingIntensity::VeryDeep),
        ("Maximum thinking required", ThinkingIntensity::VeryDeep),
    ];

    for (prompt, expected) in test_prompts {
        let detected = ThinkingIntensity::from_prompt(prompt);
        println!(
            "   '{}' -> {:?} ({})",
            prompt,
            detected,
            if detected == expected { "✓" } else { "✗" }
        );
    }

    println!("\n=== Demo Complete ===");
}
