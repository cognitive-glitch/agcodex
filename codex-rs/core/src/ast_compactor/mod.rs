//! AST Compactor Module - Comprehensive AST analysis and compaction
//!
//! This module provides advanced AST compaction capabilities that extract
//! function signatures, type definitions, trait definitions while removing
//! implementation bodies. Supports multiple programming languages through
//! tree-sitter parsers with zero-copy optimizations.
//!
//! # Features
//!
//! - Multi-language support (Rust, Python, JavaScript/TypeScript, Go)
//! - Function signature extraction with parameter types
//! - Type definition preservation (structs, enums, interfaces)
//! - Trait/interface definition extraction
//! - Documentation comment preservation
//! - Efficient zero-copy string handling with `Cow<str>`
//! - Comprehensive error handling with `thiserror`
//! - Performance-optimized with parser caching
//!
//! # Architecture
//!
//! ```text
//! ast_compactor/
//! ├── mod.rs         - Module entry and public API
//! ├── compactor.rs   - Core compaction logic and orchestration
//! ├── languages.rs   - Language-specific AST handlers
//! └── types.rs       - Type definitions and shared structures
//! ```
//!
//! # Usage Example
//!
//! ```rust
//! use agcodex_core::ast_compactor::{AstCompactor, CompactionOptions, Language};
//!
//! let compactor = AstCompactor::new();
//! let source_code = r#"
//!     pub struct User {
//!         pub id: u64,
//!         name: String,
//!     }
//!     
//!     impl User {
//!         pub fn new(id: u64, name: String) -> Self {
//!             // Implementation details...
//!             Self { id, name }
//!         }
//!     }
//! "#;
//!
//! let options = CompactionOptions::new()
//!     .with_language(Language::Rust)
//!     .preserve_docs(true)
//!     .preserve_signatures_only(true);
//!
//! let result = compactor.compact(source_code, &options)?;
//! println!("Compacted: {}", result.compacted_code);
//! ```
//!
//! # Performance Characteristics
//!
//! - **Parsing**: O(n) where n is source code length
//! - **Extraction**: O(m) where m is number of AST nodes
//! - **Memory**: Zero-copy string handling with `Cow<str>`
//! - **Caching**: O(1) parser reuse with LRU cache
//!
//! # Error Handling
//!
//! All errors are handled through the `CompactionError` enum which provides
//! detailed context about parsing failures, unsupported language constructs,
//! and I/O issues.

pub mod compactor;
pub mod languages;
pub mod types;

// Simple demo module for testing and examples
#[cfg(feature = "demo")]
pub mod simple_demo;

// Re-export core types for convenience
pub use compactor::AstCompactor;
pub use languages::LanguageHandler;
pub use types::CompactionError;
pub use types::CompactionOptions;
pub use types::CompactionResult;
pub use types::ExtractedElement;
pub use types::FunctionSignature;
pub use types::Language;
pub use types::TypeDefinition;

// Re-export commonly used Result type
pub type Result<T> = std::result::Result<T, CompactionError>;

/// Default compaction options for common use cases
impl Default for CompactionOptions {
    fn default() -> Self {
        Self::new()
            .preserve_docs(true)
            .preserve_signatures_only(false)
            .include_private(false)
            .zero_copy(true)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Test that all main types are properly exported
        let _compactor: AstCompactor = AstCompactor::new();
        let _options: CompactionOptions = CompactionOptions::new();
        let _language: Language = Language::Rust;

        // Test that Result type alias works
        fn test_result() -> Result<()> {
            Ok(())
        }

        assert!(test_result().is_ok());
    }

    #[test]
    fn test_default_options() {
        let options = CompactionOptions::default();
        assert_eq!(options.language, None);
        assert!(options.preserve_docs);
        assert!(!options.preserve_signatures_only);
        assert!(!options.include_private);
        assert!(options.zero_copy);
    }

    #[test]
    fn test_basic_functionality() {
        let mut compactor = AstCompactor::new();
        let source = "fn hello() { println!(\"Hello, world!\"); }";
        let options = CompactionOptions::new().with_language(Language::Rust);

        let result = compactor.compact(source, &options);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.language, Language::Rust);
        assert!(!result.compacted_code.is_empty());
    }
}
