//! Demonstration of the GlobTool for fast file discovery
//!
//! This example shows how to use the GlobTool to efficiently find files
//! in large codebases with gitignore support and parallel walking.

use agcodex_core::tools::FileType;
use agcodex_core::tools::GlobTool;
use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Get the current directory or use provided argument
    let search_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap());

    println!("🔍 GlobTool Demo - Searching in: {}", search_dir.display());
    println!();

    // Create glob tool with parallel processing enabled
    let glob_tool = GlobTool::new(search_dir.clone())
        .with_parallel(true)
        .with_max_results(Some(50)) // Limit for demo
        .with_include_hidden(false);

    // Demo 1: Find all Rust files
    println!("📋 Demo 1: Finding all Rust files (*.rs)");
    match glob_tool.find_type("rs") {
        Ok(result) => {
            println!(
                "✅ Found {} Rust files in {}ms",
                result.result.len(),
                result.metadata.search_duration.as_millis()
            );

            for file_match in result.result.iter().take(10) {
                let file_type_emoji = match file_match.file_type {
                    FileType::Source => "📄",
                    FileType::Test => "🧪",
                    FileType::Config => "⚙️",
                    FileType::Documentation => "📚",
                    FileType::Generated => "🔧",
                    FileType::Binary => "🔵",
                    FileType::Unknown => "❓",
                };

                println!(
                    "  {} {} ({} bytes)",
                    file_type_emoji,
                    file_match.relative_path.display(),
                    file_match.size
                );
            }

            if result.result.len() > 10 {
                println!("  ... and {} more files", result.result.len() - 10);
            }

            println!(
                "📊 Performance: {} threads, {}KB memory used",
                result.performance.threads_used, result.performance.memory_used_kb
            );
        }
        Err(e) => println!("❌ Error: {}", e),
    }

    println!();

    // Demo 2: Find configuration files
    println!("📋 Demo 2: Finding configuration files");
    match glob_tool.glob("*.{toml,yaml,yml,json}") {
        Ok(result) => {
            println!("✅ Found {} config files", result.result.len());

            for file_match in result.result.iter().take(5) {
                println!(
                    "  ⚙️  {} ({})",
                    file_match.relative_path.display(),
                    file_match.extension.as_ref().unwrap_or(&"?".to_string())
                );
            }

            if result.result.len() > 5 {
                println!("  ... and {} more files", result.result.len() - 5);
            }
        }
        Err(e) => println!("❌ Error: {}", e),
    }

    println!();

    // Demo 3: Find test files (smart classification)
    println!("📋 Demo 3: Finding test files (intelligent classification)");
    match glob_tool.glob("*") {
        Ok(result) => {
            let test_files: Vec<_> = result
                .result
                .iter()
                .filter(|f| f.file_type == FileType::Test)
                .collect();

            println!("✅ Found {} test files", test_files.len());

            for file_match in test_files.iter().take(5) {
                println!("  🧪 {}", file_match.relative_path.display());
            }

            if test_files.len() > 5 {
                println!("  ... and {} more test files", test_files.len() - 5);
            }
        }
        Err(e) => println!("❌ Error: {}", e),
    }

    println!();

    // Demo 4: Search in specific directory
    if let Ok(src_dir) = std::fs::read_dir(search_dir.join("src")) {
        println!("📋 Demo 4: Searching in src/ directory only");
        match glob_tool.find_in_directory(&search_dir.join("src"), "*") {
            Ok(result) => {
                println!("✅ Found {} files in src/", result.result.len());

                // Group by file type
                let mut type_counts = std::collections::HashMap::new();
                for file_match in &result.result {
                    *type_counts.entry(file_match.file_type.clone()).or_insert(0) += 1;
                }

                for (file_type, count) in type_counts {
                    let emoji = match file_type {
                        FileType::Source => "📄",
                        FileType::Test => "🧪",
                        FileType::Config => "⚙️",
                        FileType::Documentation => "📚",
                        FileType::Generated => "🔧",
                        FileType::Binary => "🔵",
                        FileType::Unknown => "❓",
                    };
                    println!("  {} {:?}: {} files", emoji, file_type, count);
                }
            }
            Err(e) => println!("❌ Error: {}", e),
        }
    } else {
        println!("📋 Demo 4: Skipped (no src/ directory found)");
    }

    println!();
    println!("🎉 GlobTool demo completed!");

    Ok(())
}
