//! Demo of the AST-grep based semantic search tool
//!
//! This example showcases the key features of the GrepTool:
//! - Pattern-based search with AST understanding
//! - YAML rule support for complex transformations
//! - Meta-variable bindings ($VAR, $$EXPR)
//! - Semantic context extraction
//! - Performance optimization with caching

use agcodex_core::tools::GrepQuery;
use agcodex_core::tools::GrepSupportedLanguage;
use agcodex_core::tools::GrepTool;
use agcodex_core::tools::RuleType;
use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a sample Rust file for demonstration
    let mut temp_file = NamedTempFile::new()?;
    writeln!(
        temp_file,
        r#"
pub struct User {{
    pub id: u64,
    name: String,
}}

impl User {{
    pub fn new(id: u64, name: String) -> Self {{
        User {{ id, name }}
    }}
    
    pub fn get_name(&self) -> &str {{
        &self.name
    }}
    
    fn calculate_score(&self) -> i32 {{
        42
    }}
}}

fn main() {{
    let user = User::new(1, "Alice".to_string());
    println!("User: {{}}", user.get_name());
}}
"#
    )?;

    let file_path = temp_file.path().to_path_buf();

    // Initialize the grep tool
    let grep_tool = GrepTool::new();

    println!("🔍 AST-Grep Tool Demo\n");

    // Demo 1: Simple pattern search
    println!("📌 Demo 1: Simple Pattern Search");
    println!("Searching for 'User' patterns...\n");

    let result = grep_tool.grep("User", vec![file_path.clone()])?;
    println!("Found {} matches", result.result.len());
    for (i, grep_match) in result.result.iter().enumerate() {
        println!(
            "  {}. {}:{}: {} (confidence: {:.2})",
            i + 1,
            grep_match.file.file_name().unwrap().to_string_lossy(),
            grep_match.line,
            grep_match.content.trim(),
            grep_match.confidence
        );
    }

    // Demo 2: Advanced query with builder pattern
    println!("\n📌 Demo 2: Advanced Query with Context");
    println!("Searching for function declarations with context...\n");

    let query = GrepQuery::new("fn")
        .paths(vec![file_path.clone()])
        .language(GrepSupportedLanguage::Rust)
        .with_context(true)
        .limit(3);

    let result = grep_tool.search_with_query(query)?;
    println!("Advanced search summary: {}", result.summary);

    for grep_match in &result.result {
        println!(
            "\n🎯 Match in {}:{}",
            grep_match.file.file_name().unwrap().to_string_lossy(),
            grep_match.line
        );
        println!("   Content: {}", grep_match.content.trim());
        println!("   Role: {:?}", grep_match.semantic_context.role);
        println!(
            "   Is Definition: {}",
            grep_match.semantic_context.is_definition
        );

        if !grep_match.metavar_bindings.is_empty() {
            println!("   Meta-variables:");
            for (key, value) in &grep_match.metavar_bindings {
                println!("     ${} = {}", key, value);
            }
        }

        if !grep_match.surrounding_context.before_lines.is_empty() {
            println!("   Context (before):");
            for line in &grep_match.surrounding_context.before_lines {
                println!("     {}: {}", line.number, line.content.trim());
            }
        }
    }

    // Demo 3: YAML Rule example (simplified)
    println!("\n📌 Demo 3: YAML Rule Support");
    println!("Note: This demonstrates the YAML rule interface (simplified example)\n");

    let yaml_rule = r#"
id: find-public-functions
message: Found public function
severity: info
language: rust
rule:
  pattern: pub fn $NAME($$$ARGS) { $$$BODY }
"#;

    // Note: This would require a more complete YAML rule parser
    // For demo purposes, we'll show how it would be called
    println!("YAML Rule:");
    println!("{}", yaml_rule);
    println!("This rule would find all public functions with meta-variable bindings for:");
    println!("  - $NAME: function name");
    println!("  - $ARGS: function arguments");
    println!("  - $BODY: function body");

    // Demo 4: Cache statistics
    println!("\n📌 Demo 4: Performance & Caching");
    let (cache_size, max_cache_size) = grep_tool.cache_stats();
    println!("Pattern cache: {}/{} entries", cache_size, max_cache_size);
    println!(
        "Cache utilization: {:.1}%",
        (cache_size as f32 / max_cache_size as f32) * 100.0
    );

    // Demo 5: Multi-language support
    println!("\n📌 Demo 5: Multi-Language Support");
    println!("Supported languages:");
    let languages = [
        "Rust",
        "Python",
        "JavaScript",
        "TypeScript",
        "Go",
        "Java",
        "C",
        "C++",
        "C#",
        "HTML",
        "CSS",
        "JSON",
        "YAML",
        "Bash",
        "PHP",
        "Ruby",
        "Swift",
        "Kotlin",
    ];

    for (i, lang) in languages.iter().enumerate() {
        if i % 6 == 0 && i > 0 {
            println!();
        }
        print!("  {:12}", lang);
    }
    println!("\n");

    println!("✅ Demo completed successfully!");
    println!("\nKey Features Demonstrated:");
    println!("  • AST-aware pattern matching");
    println!("  • Semantic context extraction");
    println!("  • Meta-variable bindings");
    println!("  • Confidence scoring");
    println!("  • Pattern caching for performance");
    println!("  • YAML rule support interface");
    println!("  • Multi-language detection");
    println!("  • Context-aware output for LLMs");

    Ok(())
}
