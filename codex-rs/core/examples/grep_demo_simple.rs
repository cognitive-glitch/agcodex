//! Simple demo of the Grep tool for pattern-based search
//!
//! This example showcases basic pattern search functionality.

use agcodex_core::tools::GrepConfig;
use agcodex_core::tools::GrepQuery;
use agcodex_core::tools::GrepSupportedLanguage;
use agcodex_core::tools::GrepTool;
use std::io::Write;
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
}}

fn main() {{
    let user = User::new(1, "Alice".to_string());
    println!("User: {{}}", user.get_name());
}}
"#
    )?;

    let file_path = temp_file.path().to_path_buf();

    // Initialize the grep tool
    let grep_tool = GrepTool::new(GrepConfig::default());

    println!("🔍 Grep Tool Demo\n");

    // Simple pattern search
    println!("📌 Searching for 'User' patterns...");

    let query = GrepQuery {
        pattern: "User".to_string(),
        paths: vec![file_path.clone()],
        language: Some(GrepSupportedLanguage::Rust),
        ..Default::default()
    };

    let result = grep_tool.search_with_query(query)?;
    println!("Found {} matches", result.result.len());

    for (i, grep_match) in result.result.iter().enumerate().take(5) {
        println!("  {}. Line {}: Pattern matched", i + 1, grep_match.line);
    }

    println!("\n📊 Summary: {}", result.summary);
    println!("\n✅ Demo completed successfully!");

    Ok(())
}
