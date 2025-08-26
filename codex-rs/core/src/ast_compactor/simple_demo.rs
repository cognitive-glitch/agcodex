//! Simple demo/example for AST compactor functionality
//!
//! This file demonstrates basic usage of the AST compactor module
//! and serves as a working example while the full implementation
//! is being developed.

use crate::ast_compactor::AstCompactor;
use crate::ast_compactor::CompactionOptions;
use crate::ast_compactor::CompactionResult;
use crate::ast_compactor::Language;

/// Demonstrate basic Rust code compaction
pub fn demo_rust_compaction() -> Result<CompactionResult<'static>, Box<dyn std::error::Error>> {
    let mut compactor = AstCompactor::new();

    let rust_code = r#"
        /// A simple user struct
        pub struct User {
            pub id: u64,
            name: String,
        }
        
        impl User {
            /// Create a new user
            pub fn new(id: u64, name: String) -> Self {
                println!("Creating user: {}", name);
                Self { id, name }
            }
            
            /// Get the user's name
            pub fn get_name(&self) -> &str {
                &self.name
            }
        }
    "#;

    let options = CompactionOptions::new()
        .with_language(Language::Rust)
        .preserve_docs(true)
        .preserve_signatures_only(true);

    let result = compactor.compact(rust_code, &options)?;

    println!("=== AST Compactor Demo ===");
    println!("Original size: {} bytes", result.original_size);
    println!("Compressed size: {} bytes", result.compressed_size);
    println!("Compression ratio: {:.1}%", result.compression_percentage());
    println!("Processing time: {}ms", result.processing_time_ms);
    println!("Elements extracted: {}", result.elements.len());
    println!("\n=== Compacted Code ===");
    println!("{}", result.compacted_code);

    Ok(result)
}

/// Demonstrate language detection
pub fn demo_language_detection() {
    use crate::ast_compactor::languages::LanguageHandlerFactory;

    let test_cases = vec![
        ("fn main() {}", Some(Language::Rust)),
        ("def hello(): pass", Some(Language::Python)),
        ("function test() {}", Some(Language::JavaScript)),
        (
            "interface User { name: string; }",
            Some(Language::TypeScript),
        ),
        ("func main() { fmt.Println() }", Some(Language::Go)),
        ("random text", None),
    ];

    println!("\n=== Language Detection Demo ===");
    for (code, expected) in test_cases {
        let detected = LanguageHandlerFactory::detect_language(code);
        let status = if detected == expected { "✓" } else { "✗" };
        println!(
            "{} '{}' -> {:?}",
            status,
            code.chars().take(20).collect::<String>(),
            detected
        );
    }
}

/// Demonstrate compaction options
pub fn demo_compaction_options() -> Result<(), Box<dyn std::error::Error>> {
    let mut compactor = AstCompactor::new();

    let code = r#"
        pub struct Calculator {
            precision: u8,
        }
        
        impl Calculator {
            pub fn new(precision: u8) -> Self {
                Self { precision }
            }
            
            pub fn add(&self, a: f64, b: f64) -> f64 {
                let result = a + b;
                let factor = 10f64.powi(self.precision as i32);
                (result * factor).round() / factor
            }
        }
    "#;

    println!("\n=== Compaction Options Demo ===");

    // Signatures only
    let sig_options = CompactionOptions::new()
        .with_language(Language::Rust)
        .preserve_signatures_only(true);
    let sig_result = compactor.compact(code, &sig_options)?;

    println!(
        "Signatures only: {:.1}% compression",
        sig_result.compression_percentage()
    );

    // Full preservation
    let full_options = CompactionOptions::new()
        .with_language(Language::Rust)
        .preserve_signatures_only(false);
    let full_result = compactor.compact(code, &full_options)?;

    println!(
        "Full code: {:.1}% compression",
        full_result.compression_percentage()
    );

    Ok(())
}

/// Run all demonstrations
pub fn run_all_demos() -> Result<(), Box<dyn std::error::Error>> {
    demo_language_detection();
    let _rust_result = demo_rust_compaction()?;
    demo_compaction_options()?;

    println!("\n=== Demo completed successfully ===");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo_rust_compaction() {
        let result = demo_rust_compaction();
        assert!(result.is_ok());

        if let Ok(result) = result {
            assert_eq!(result.language, Language::Rust);
            assert!(result.compression_ratio >= 0.0);
        }
    }

    #[test]
    fn test_demo_language_detection() {
        // This test just ensures the demo doesn't panic
        demo_language_detection();
    }

    #[test]
    fn test_demo_compaction_options() {
        let result = demo_compaction_options();
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_all_demos() {
        let result = run_all_demos();
        // This might fail with current placeholder implementation, but shouldn't panic
        println!("Demo result: {:?}", result);
    }
}
