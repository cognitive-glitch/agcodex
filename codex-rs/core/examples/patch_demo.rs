//! Demonstration of the AST-aware patch tool
//!
//! This example shows how to use the PatchTool for semantic code transformations
//! with comprehensive before/after analysis and impact assessment.

use agcodex_core::subagents::config::IntelligenceLevel;
use agcodex_core::tools::PatchInput;
use agcodex_core::tools::PatchOptions;
use agcodex_core::tools::PatchTool;
use agcodex_core::tools::RiskLevel;
use agcodex_core::tools::SemanticTransformation;
use agcodex_core::tools::TransformationCondition;
use agcodex_core::tools::TransformationType;
use agcodex_core::tools::patch::AstNodeKind;
use agcodex_core::tools::patch::CompressionLevel;
use ast::SourceLocation;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a sample Rust file for transformation
    let demo_code = r#"
// Legacy function with unclear naming
fn calc(x: i32, y: i32) -> i32 {
    let temp_result = x * 2;
    let final_result = temp_result + y;
    final_result
}

fn old_helper() -> String {
    "helper data".to_string()
}

fn main() {
    let result = calc(10, 5);
    let data = old_helper();
    println!("Result: {}, Data: {}", result, data);
}
"#;

    let dir = tempdir()?;
    let file_path = dir.path().join("demo.rs");
    fs::write(&file_path, demo_code)?;

    println!("🔧 **AGCodex AST-Aware Patch Tool Demo**");
    println!("=====================================\n");

    // Initialize patch tool with medium intelligence
    let mut patch_tool = PatchTool::new(CompressionLevel::Medium);

    // Create semantic transformations
    let transformations = vec![
        // Transform 1: Rename unclear function name
        SemanticTransformation {
            id: Uuid::new_v4().to_string(),
            transformation_type: TransformationType::RenameSymbol {
                old_name: "calc".to_string(),
                new_name: "calculate_with_multiplier".to_string(),
            },
            source_location: SourceLocation::new(
                file_path.to_string_lossy(),
                3,
                4,
                3,
                8, // Line 3, columns 4-8 for "calc"
                (30, 34),
            ),
            target_pattern: "calc".to_string(),
            replacement_pattern: "calculate_with_multiplier".to_string(),
            conditions: vec![
                TransformationCondition::NodeKind(AstNodeKind::Function),
                TransformationCondition::PreserveReferences,
            ],
            confidence: 0.95,
            risk_level: RiskLevel::Low,
            preserve_context: true,
        },
        // Transform 2: Rename helper function for clarity
        SemanticTransformation {
            id: Uuid::new_v4().to_string(),
            transformation_type: TransformationType::RenameSymbol {
                old_name: "old_helper".to_string(),
                new_name: "get_helper_data".to_string(),
            },
            source_location: SourceLocation::new(
                file_path.to_string_lossy(),
                8,
                4,
                8,
                14, // Line 8, columns 4-14 for "old_helper"
                (120, 130),
            ),
            target_pattern: "old_helper".to_string(),
            replacement_pattern: "get_helper_data".to_string(),
            conditions: vec![
                TransformationCondition::NodeKind(AstNodeKind::Function),
                TransformationCondition::InScope("global".to_string()),
            ],
            confidence: 0.95,
            risk_level: RiskLevel::Low,
            preserve_context: true,
        },
    ];

    // Configure patch options
    let options = PatchOptions {
        preserve_formatting: true,
        validate_semantics: true,
        generate_diff: true,
        intelligence_level: IntelligenceLevel::Medium,
        timeout_ms: 30_000,
        enable_rollback: true,
        confidence_threshold: 0.8,
        safety_checks: true,
        analyze_dependencies: true,
    };

    // Create patch input
    let patch_input = PatchInput {
        file_path: file_path.clone(),
        transformations: transformations.clone(),
        options,
    };

    // Apply the patch transformations
    println!("📋 **TRANSFORMATION PLAN**");
    println!("---");
    for (i, transform) in transformations.iter().enumerate() {
        match &transform.transformation_type {
            TransformationType::RenameSymbol { old_name, new_name } => {
                println!("{}. 🏷️  **Symbol Rename**", i + 1);
                println!(
                    "   • Location: {}",
                    transform.source_location.to_range_string()
                );
                println!("   • Change: `{}` → `{}`", old_name, new_name);
                println!(
                    "   • Conditions: {} safety checks",
                    transform.conditions.len()
                );
            }
            _ => println!("{}. 🔄 **Other Transformation**", i + 1),
        }
        println!();
    }

    println!("⚙️  **APPLYING TRANSFORMATIONS**");
    println!("---");

    match patch_tool
        .patch(&file_path, transformations, patch_input.options)
        .await
    {
        Ok(output) => {
            println!("✅ **Patch completed successfully!**\n");

            // Display transformation results
            println!("📊 **TRANSFORMATION RESULTS**");
            println!("---");
            println!(
                "• **Success**: {}",
                if output.success { "✅ Yes" } else { "❌ No" }
            );
            println!(
                "• **Transformations Applied**: {}",
                output.transformations_applied.len()
            );
            println!("• **Warnings**: {}", output.warnings.len());
            println!("• **Errors**: {}", output.errors.len());
            println!();

            // Show applied transformations
            for (i, applied) in output.transformations_applied.iter().enumerate() {
                println!(
                    "**Transformation {}**: {}",
                    i + 1,
                    if applied.success {
                        "✅ SUCCESS"
                    } else {
                        "❌ FAILED"
                    }
                );
                println!("• **Type**: {:?}", applied.transformation_type);
                println!("• **Location**: {}", applied.location.to_range_string());
                println!(
                    "• **Semantic Preserving**: {}",
                    if applied.semantic_preserving {
                        "✅ Yes"
                    } else {
                        "⚠️ No"
                    }
                );
                println!("• **Changes Made**: {}", applied.changes.len());

                for change in &applied.changes {
                    println!(
                        "  - {:?}: '{}' → '{}'",
                        change.change_type, change.old_content, change.new_content
                    );
                }
                println!();
            }

            // Show semantic impact analysis
            println!("🧠 **SEMANTIC IMPACT ANALYSIS**");
            println!("---");
            println!(
                "• **Preserves Semantics**: {}",
                if output.semantic_impact.preserves_semantics {
                    "✅ Yes"
                } else {
                    "⚠️ No"
                }
            );
            println!(
                "• **API Changes**: {}",
                output.semantic_impact.api_changes.len()
            );
            println!(
                "• **Behavioral Changes**: {}",
                output.semantic_impact.behavioral_changes.len()
            );
            println!(
                "• **Dependency Changes**: {}",
                output.semantic_impact.dependency_changes.len()
            );

            let perf = &output.semantic_impact.performance_impact;
            println!("• **Performance Impact**:");
            println!("  - Complexity Change: {:+}", perf.complexity_change);
            println!("  - Memory Impact: {}", perf.memory_impact);
            println!("  - Runtime Impact: {}", perf.runtime_impact);
            println!();

            // Display API changes
            if !output.semantic_impact.api_changes.is_empty() {
                println!("🔌 **API CHANGES**");
                println!("---");
                for change in &output.semantic_impact.api_changes {
                    println!("• **Symbol**: `{}`", change.symbol_name);
                    println!("  - Change: {}", change.change_type);
                    println!(
                        "  - Breaking: {}",
                        if change.breaking_change {
                            "⚠️ Yes"
                        } else {
                            "✅ No"
                        }
                    );
                    println!("  - Description: {}", change.description);
                    println!();
                }
            }

            // Display behavioral changes
            if !output.semantic_impact.behavioral_changes.is_empty() {
                println!("🎭 **BEHAVIORAL CHANGES**");
                println!("---");
                for change in &output.semantic_impact.behavioral_changes {
                    let risk_emoji = match change.risk_level {
                        RiskLevel::Low => "🟢",
                        RiskLevel::Medium => "🟡",
                        RiskLevel::High => "🟠",
                        RiskLevel::Critical => "🔴",
                    };
                    println!("• **Function**: `{}` {}", change.function_name, risk_emoji);
                    println!("  - Risk: {:?}", change.risk_level);
                    println!("  - Description: {}", change.change_description);
                    println!();
                }
            }

            // Show AST comparison
            println!("🌳 **BEFORE/AFTER AST COMPARISON**");
            println!("---");
            let comparison = &output.before_after_comparison;
            println!(
                "• **Semantic Equivalence Score**: {:.1}%",
                comparison.semantic_equivalence_score * 100.0
            );
            println!();

            println!("**Source AST Summary**:");
            let src = &comparison.source_ast_summary;
            println!("• Total Nodes: {}", src.total_nodes);
            println!("• Functions: {}", src.function_count);
            println!("• Classes: {}", src.class_count);
            println!("• Complexity Score: {}", src.complexity_score);
            println!();

            println!("**Target AST Summary**:");
            let tgt = &comparison.target_ast_summary;
            println!("• Total Nodes: {}", tgt.total_nodes);
            println!("• Functions: {}", tgt.function_count);
            println!("• Classes: {}", tgt.class_count);
            println!("• Complexity Score: {}", tgt.complexity_score);
            println!();

            // Show structural differences
            if !comparison.structural_differences.is_empty() {
                println!("🔍 **STRUCTURAL DIFFERENCES**");
                println!("---");
                for diff in &comparison.structural_differences {
                    let severity_emoji = match diff.severity {
                        RiskLevel::Low => "🟢",
                        RiskLevel::Medium => "🟡",
                        RiskLevel::High => "🟠",
                        RiskLevel::Critical => "🔴",
                    };
                    println!(
                        "• **{}** {} at {}",
                        diff.difference_type, severity_emoji, diff.location
                    );
                    println!("  - {}", diff.description);
                    println!();
                }
            }

            // Show warnings and errors
            if !output.warnings.is_empty() {
                println!("⚠️  **WARNINGS**");
                println!("---");
                for warning in &output.warnings {
                    println!("• {}", warning);
                }
                println!();
            }

            if !output.errors.is_empty() {
                println!("❌ **ERRORS**");
                println!("---");
                for error in &output.errors {
                    println!("• {}", error);
                }
                println!();
            }

            println!("🎉 **Transformation Complete!**");
            println!("The code has been successfully transformed with full semantic analysis.");
        }

        Err(e) => {
            println!("❌ **Patch failed**: {}", e);
            println!("\n🔍 **Debugging Information**:");
            println!("• Check that the file exists and is readable");
            println!("• Verify the source locations are correct");
            println!("• Ensure the transformations are valid for the target language");
        }
    }

    Ok(())
}

/// Helper function to display file contents with line numbers
#[allow(dead_code)]
fn display_file_with_lines(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    println!(
        "📄 **File: {}**",
        path.file_name().unwrap().to_string_lossy()
    );
    println!("---");
    for (i, line) in content.lines().enumerate() {
        println!("{:3} | {}", i + 1, line);
    }
    println!();
    Ok(())
}
