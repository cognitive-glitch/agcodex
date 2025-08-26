//! Simple demonstration of the GlobTool for file discovery
//!
//! This example shows basic usage of the GlobTool to find files
//! in a codebase with gitignore support.

use agcodex_core::tools::GlobTool;
use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get the current directory or use provided argument
    let search_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap());

    println!("🔍 GlobTool Demo - Searching in: {}", search_dir.display());
    println!();

    // Create glob tool
    let glob_tool = GlobTool::new(search_dir.clone()).with_max_results(Some(20));

    // Demo: Find all Rust files
    println!("📋 Finding Rust files (*.rs)");
    match glob_tool.find_type("rs") {
        Ok(result) => {
            println!("✅ Found {} Rust files", result.result.len());

            for file_match in result.result.iter().take(10) {
                println!(
                    "  - {} ({} bytes)",
                    file_match.relative_path.display(),
                    file_match.size.unwrap_or(0)
                );
            }

            if result.result.len() > 10 {
                println!("  ... and {} more files", result.result.len() - 10);
            }

            println!("\n📊 Summary: {}", result.summary);
        }
        Err(e) => println!("❌ Error: {}", e),
    }

    println!("\n🎉 GlobTool demo completed!");
    Ok(())
}
