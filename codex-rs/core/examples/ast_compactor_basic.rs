//! Basic AST Compactor Example
//!
//! This example demonstrates the fundamental usage of the AST compactor
//! for extracting function signatures and type definitions from Rust code.

use agcodex_core::ast_compactor::AstCompactor;
use agcodex_core::ast_compactor::CompactionOptions;
use agcodex_core::ast_compactor::Language;
use agcodex_core::ast_compactor::types::ElementType;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 AST Compactor Basic Example");
    println!("===============================\n");

    // Create a new AST compactor instance
    let mut compactor = AstCompactor::new();

    // Example Rust code with various constructs
    let rust_source = r#"
        //! This is a module-level documentation comment
        
        use std::collections::HashMap;
        use std::fmt::{Debug, Display};

        /// A user in our system with rich metadata
        #[derive(Debug, Clone, Serialize)]
        pub struct User {
            /// Unique identifier for the user
            pub id: u64,
            /// Display name for the user
            pub name: String,
            /// Optional email address
            pub email: Option<String>,
            /// User preferences as key-value pairs
            preferences: HashMap<String, String>,
        }

        /// Status enumeration for user accounts
        #[derive(Debug, PartialEq)]
        pub enum UserStatus {
            /// Account is active and can be used
            Active,
            /// Account is temporarily suspended with a reason
            Suspended { reason: String, until: Option<chrono::DateTime<chrono::Utc>> },
            /// Account has been permanently deactivated
            Deactivated,
        }

        /// Trait for user repository operations
        pub trait UserRepository {
            type Error: Debug + Display;

            /// Find a user by their unique ID
            fn find_by_id(&self, id: u64) -> Result<Option<User>, Self::Error>;
            
            /// Save a user to the repository
            fn save(&mut self, user: User) -> Result<(), Self::Error>;
            
            /// Update user status
            fn update_status(&mut self, id: u64, status: UserStatus) -> Result<(), Self::Error>;
            
            /// List all users with pagination
            fn list_users(&self, offset: usize, limit: usize) -> Result<Vec<User>, Self::Error> {
                // Default implementation returns empty list
                Ok(Vec::new())
            }
        }

        impl User {
            /// Create a new user with required fields
            pub fn new(id: u64, name: String) -> Self {
                println!("Creating new user: {} with ID: {}", name, id);
                Self {
                    id,
                    name,
                    email: None,
                    preferences: HashMap::new(),
                }
            }

            /// Get the user's display name
            pub fn display_name(&self) -> &str {
                &self.name
            }

            /// Update user email address
            pub fn set_email(&mut self, email: Option<String>) {
                if let Some(ref email_addr) = email {
                    println!("Setting email for user {}: {}", self.id, email_addr);
                }
                self.email = email;
            }

            /// Add a preference for this user
            pub fn set_preference(&mut self, key: String, value: String) {
                println!("Setting preference {}={} for user {}", key, value, self.id);
                self.preferences.insert(key, value);
            }

            /// Get a preference value by key
            pub fn get_preference(&self, key: &str) -> Option<&String> {
                self.preferences.get(key)
            }

            /// Helper method to validate user data (private)
            fn validate_data(&self) -> bool {
                !self.name.trim().is_empty() && self.id > 0
            }

            /// Internal method to log user actions
            fn log_action(&self, action: &str) {
                eprintln!("User {} performed action: {}", self.id, action);
            }
        }

        impl Display for User {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "User(id={}, name='{}')", self.id, self.name)
            }
        }

        /// Convenience function to create a demo user
        pub fn create_demo_user() -> User {
            let mut user = User::new(1, "Demo User".to_string());
            user.set_email(Some("demo@example.com".to_string()));
            user.set_preference("theme".to_string(), "dark".to_string());
            user
        }

        /// Helper function to validate email format
        fn validate_email(email: &str) -> bool {
            email.contains('@') && email.contains('.')
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            fn test_user_creation() {
                let user = User::new(123, "Test User".to_string());
                assert_eq!(user.id, 123);
                assert_eq!(user.name, "Test User");
                assert!(user.email.is_none());
                assert!(user.preferences.is_empty());
            }

            #[test]
            fn test_user_preferences() {
                let mut user = create_demo_user();
                user.set_preference("language".to_string(), "en".to_string());
                assert_eq!(user.get_preference("theme"), Some(&"dark".to_string()));
                assert_eq!(user.get_preference("language"), Some(&"en".to_string()));
            }
        }
    "#;

    println!("📄 Original Source Code:");
    println!("Length: {} characters", rust_source.len());
    println!("Lines: {}", rust_source.lines().count());
    println!();

    // Configure compaction options for signature extraction
    let signature_options = CompactionOptions::new()
        .with_language(Language::Rust)
        .preserve_docs(true)
        .preserve_signatures_only(true)
        .include_private(false); // Only public APIs

    println!("🔄 Performing AST compaction (signatures only)...");
    let signature_result = compactor.compact(rust_source, &signature_options)?;

    println!("✅ Compaction Results:");
    println!("Language detected: {}", signature_result.language);
    println!("Original size: {} bytes", signature_result.original_size);
    println!(
        "Compressed size: {} bytes",
        signature_result.compressed_size
    );
    println!(
        "Compression ratio: {:.1}%",
        signature_result.compression_percentage()
    );
    println!("Processing time: {}ms", signature_result.processing_time_ms);
    println!("Elements extracted: {}", signature_result.elements.len());

    // Show element breakdown
    println!("\n📊 Element Breakdown:");
    let functions = signature_result.elements_of_type(ElementType::Function);
    let methods = signature_result.elements_of_type(ElementType::Method);
    let structs = signature_result.elements_of_type(ElementType::Struct);
    let enums = signature_result.elements_of_type(ElementType::Enum);
    let traits = signature_result.elements_of_type(ElementType::Trait);

    println!("  - Functions: {}", functions.len());
    println!("  - Methods: {}", methods.len());
    println!("  - Structs: {}", structs.len());
    println!("  - Enums: {}", enums.len());
    println!("  - Traits: {}", traits.len());

    println!("\n🎯 Compacted Code Output:");
    println!("{}", "=".repeat(60));
    println!("{}", signature_result.compacted_code);
    println!("{}", "=".repeat(60));

    // Test with full preservation
    let full_options = CompactionOptions::new()
        .with_language(Language::Rust)
        .preserve_docs(true)
        .preserve_signatures_only(false)
        .include_private(true);

    println!("\n🔄 Full code compaction (with private elements)...");
    let full_result = compactor.compact(rust_source, &full_options)?;

    println!("📈 Comparison:");
    println!(
        "Signature-only: {:.1}% compression",
        signature_result.compression_percentage()
    );
    println!(
        "Full code: {:.1}% compression",
        full_result.compression_percentage()
    );

    // Show cache performance
    println!("\n⚡ Performance Metrics:");
    println!("Cache hit rate: {:.1}%", compactor.cache_hit_rate());
    let (parsers, asts) = compactor.memory_info();
    println!("Cached parsers: {}", parsers);
    println!("Cached ASTs: {}", asts);

    Ok(())
}
