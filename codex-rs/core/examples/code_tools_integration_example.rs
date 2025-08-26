//! Code tools integration example demonstrating unified search interface.
//!
//! This example shows how to use the enhanced code tools module with:
//! - Tool discovery and registration
//! - Unified search interface
//! - Builder patterns for complex queries
//! - Streaming results processing
//! - Error handling and cancellation

use agcodex_core::code_tools::RipgrepQueryBuilder;
use agcodex_core::code_tools::SrgnLanguage;
use agcodex_core::code_tools::SrgnQueryBuilder;
use agcodex_core::code_tools::ToolFactory;
use agcodex_core::code_tools::traits::*;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for better debugging (commented out - dependency not available)
    // tracing_subscriber::fmt::init();

    println!("🔍 Code Tools Integration Example");
    println!("==================================\n");

    // 1. Discover available tools
    println!("📋 Discovering available tools...");
    let registry = ToolFactory::discover_all_tools().await;
    let available_tools = registry.get_available_tools();

    if available_tools.is_empty() {
        println!("⚠️  No tools found. Please install ripgrep and/or srgn.");
        println!("   - Install ripgrep: https://github.com/BurntSushi/ripgrep");
        println!("   - Install srgn: https://github.com/alexpovel/srgn");
        return Ok(());
    }

    println!("✅ Available tools: {}", available_tools.join(", "));

    // Display tool metadata
    for tool_name in &available_tools {
        if let Some(metadata) = registry.get_tool_metadata(tool_name) {
            println!(
                "   🔧 {}: {} - {:?}",
                metadata.name, metadata.version, metadata.capabilities
            );
        }
    }

    println!();

    // 2. Demonstrate Ripgrep integration if available
    if available_tools.contains(&"ripgrep".to_string()) {
        println!("🦀 Testing Ripgrep integration...");

        match ToolFactory::create_ripgrep().await {
            Ok(rg) => {
                // Build a complex search query
                let query = RipgrepQueryBuilder::new("fn.*main")
                    .paths(["."])
                    .file_types(["rust"])
                    .context(2, 2)
                    .max_matches_per_file(5)
                    .build();

                // Configure search
                let config = SearchConfigBuilder::new()
                    .max_results(10)
                    .timeout_ms(5000)
                    .case_sensitive(false)
                    .build();

                let cancel_token = CancellationToken::new();

                println!("   📊 Searching for 'fn.*main' in Rust files...");

                // Execute streaming search
                match rg.search_streaming(query, config, cancel_token).await {
                    Ok(mut stream) => {
                        use futures::StreamExt;
                        let mut count = 0;

                        while let Some(result) = stream.next().await {
                            match result {
                                Ok(streaming_result) => {
                                    if let Some(search_result) = streaming_result.result {
                                        count += 1;
                                        println!(
                                            "     📁 {}: line {}",
                                            search_result.path.display(),
                                            search_result.line_number.unwrap_or(0)
                                        );

                                        if let Some(content) = &search_result.content {
                                            println!("        📝 {}", content.trim());
                                        }
                                    } else if !streaming_result.status.is_empty() {
                                        println!("     ℹ️  {}", streaming_result.status);
                                    }

                                    if streaming_result.is_final {
                                        break;
                                    }
                                }
                                Err(e) => {
                                    println!("     ❌ Error: {}", e);
                                    break;
                                }
                            }
                        }

                        println!("   ✅ Found {} matches\n", count);
                    }
                    Err(e) => {
                        println!("   ❌ Search failed: {}\n", e);
                    }
                }
            }
            Err(e) => {
                println!("   ❌ Failed to create ripgrep instance: {}\n", e);
            }
        }
    }

    // 3. Demonstrate SRGN integration if available
    if available_tools.contains(&"srgn".to_string()) {
        println!("🔄 Testing SRGN integration...");

        match ToolFactory::create_srgn().await {
            Ok(srgn) => {
                // Build an extract query to find function signatures
                let query = SrgnQueryBuilder::extract(r"fn\s+(\w+)\s*\(")
                    .language(SrgnLanguage::Rust)
                    .inputs(["."])
                    .include_pattern("*.rs")
                    .extract_format("Function: $1")
                    .dry_run() // Don't modify files
                    .build();

                let config = SearchConfigBuilder::new()
                    .max_results(5)
                    .timeout_ms(3000)
                    .build();

                let cancel_token = CancellationToken::new();

                println!("   📊 Extracting function signatures from Rust files...");

                match srgn.search_streaming(query, config, cancel_token).await {
                    Ok(mut stream) => {
                        use futures::StreamExt;
                        let mut count = 0;

                        while let Some(result) = stream.next().await {
                            match result {
                                Ok(streaming_result) => {
                                    if let Some(search_result) = streaming_result.result {
                                        count += 1;
                                        println!("     📁 {}", search_result.path.display());

                                        if let Some(content) = &search_result.content {
                                            println!("        📝 {}", content.trim());
                                        }
                                    } else if !streaming_result.status.is_empty() {
                                        println!("     ℹ️  {}", streaming_result.status);
                                    }

                                    if streaming_result.is_final {
                                        break;
                                    }
                                }
                                Err(e) => {
                                    println!("     ❌ Error: {}", e);
                                    break;
                                }
                            }
                        }

                        println!("   ✅ Processed {} files\n", count);
                    }
                    Err(e) => {
                        println!("   ❌ Extract failed: {}\n", e);
                    }
                }
            }
            Err(e) => {
                println!("   ❌ Failed to create srgn instance: {}\n", e);
            }
        }
    }

    // 4. System information
    println!("🔧 System Tool Information");
    println!("==========================");

    let system_info = agcodex_core::code_tools::utils::get_tool_system_info().await;
    for (key, value) in system_info {
        println!("{}: {}", key, value);
    }

    println!("\n✨ Example completed successfully!");
    Ok(())
}

/// Helper function to demonstrate tool validation
#[allow(dead_code)]
async fn validate_tools_example() -> Result<(), agcodex_core::code_tools::ToolError> {
    use agcodex_core::code_tools::utils::validate_required_tools;

    // Validate that required tools are available
    let required = ["rg", "fd"];

    match validate_required_tools(&required).await {
        Ok(_) => {
            println!("✅ All required tools are available");
        }
        Err(e) => {
            println!("❌ Missing tools: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

/// Helper function to demonstrate advanced configuration
#[allow(dead_code)]
fn advanced_config_example() {
    // Demonstrate complex search configuration
    let _config = SearchConfigBuilder::new()
        .max_results(100)
        .timeout_ms(10_000)
        .case_sensitive(true)
        .include_hidden(false)
        .follow_symlinks(true)
        .env_var("RUST_LOG".to_string(), "debug".to_string())
        .working_dir(PathBuf::from("/tmp"))
        .build();

    // Demonstrate ripgrep query building
    let _rg_query = RipgrepQueryBuilder::new(r"\bstruct\s+(\w+)")
        .paths(["src", "examples"])
        .file_types(["rust", "toml"])
        .include_glob("*.rs")
        .exclude_glob("*test*")
        .context(3, 3)
        .max_matches_per_file(20)
        .word_boundaries()
        .multiline()
        .flag("--color=never")
        .build();

    // Demonstrate SRGN query building
    let _srgn_query = SrgnQueryBuilder::replace(r"console\.log\(([^)]*)\)", "logger.info($1)")
        .language(SrgnLanguage::JavaScript)
        .inputs(["src/js"])
        .include_pattern("*.js")
        .exclude_pattern("*.min.js")
        .backup()
        .max_depth(5)
        .build();

    println!("📋 Advanced configurations prepared (examples only)");
}
