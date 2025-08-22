//! Simple TreeTool demonstration
//!
//! This example shows basic tree-sitter parsing capabilities.

use agcodex_core::subagents::config::IntelligenceLevel;
use agcodex_core::tools::SupportedLanguage;
use agcodex_core::tools::TreeTool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the tree tool
    let tree_tool = TreeTool::new(IntelligenceLevel::Medium)?;

    println!("🌳 TreeTool Demo\n");

    // Example: Parse Rust code
    println!("Parsing Rust Code:");
    let rust_code = r#"
        pub struct Calculator {
            value: f64,
        }
        
        impl Calculator {
            pub fn new() -> Self {
                Self { value: 0.0 }
            }
            
            pub fn add(&mut self, n: f64) -> f64 {
                self.value += n;
                self.value
            }
        }
    "#;

    let result = tree_tool
        .parse(rust_code.to_string(), Some(SupportedLanguage::Rust), None)
        .await?;

    println!("✅ Language: {:?}", result.language);
    println!("✅ Nodes: {}", result.node_count);
    println!("✅ Has errors: {}", result.has_errors());
    println!("✅ Parse time: {:?}", result.parse_time);

    println!("\n📊 Parsing completed successfully!");
    println!("\n🎉 Demo completed successfully!");
    Ok(())
}
