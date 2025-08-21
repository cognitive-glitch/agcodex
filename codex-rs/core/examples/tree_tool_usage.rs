//! TreeTool Usage Examples
//!
//! This file demonstrates various use cases for the TreeTool with different
//! programming languages and operations.

use agcodex_core::tools::IntelligenceLevel;
use agcodex_core::tools::InternalTool;
use agcodex_core::tools::SupportedLanguage;
use agcodex_core::tools::TreeInput;
use agcodex_core::tools::TreeOutput;
use agcodex_core::tools::TreeTool;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the tree tool with medium intelligence
    let tree_tool = TreeTool::new(IntelligenceLevel::Medium)?;

    println!("🌳 TreeTool Usage Examples\n");

    // Example 1: Parse Rust code
    println!("1. Parsing Rust Code");
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
        
        fn main() {
            let mut calc = Calculator::new();
            println!("Result: {}", calc.add(5.0));
        }
    "#;

    let parse_input = TreeInput::Parse {
        code: rust_code.to_string(),
        language: Some(SupportedLanguage::Rust),
        file_path: None,
    };

    let result = tree_tool.execute(parse_input).await?;
    match &result.result {
        TreeOutput::Parsed {
            language,
            node_count,
            has_errors,
            error_count,
            parse_time_ms,
        } => {
            println!("  ✅ Language: {:?}", language);
            println!("  ✅ Nodes: {}", node_count);
            println!("  ✅ Errors: {} ({})", has_errors, error_count);
            println!("  ✅ Parse time: {}ms", parse_time_ms);
        }
        _ => println!("  ❌ Unexpected output type"),
    }
    println!();

    // Example 2: Query for function definitions
    println!("2. Query for Function Definitions");
    let query_pattern = "(function_item name: (identifier) @func_name)";

    let query_input = TreeInput::Query {
        code: rust_code.to_string(),
        language: Some(SupportedLanguage::Rust),
        pattern: query_pattern.to_string(),
        file_path: None,
    };

    let result = tree_tool.execute(query_input).await?;
    match &result.result {
        TreeOutput::QueryResults {
            matches,
            total_matches,
            query_time_ms,
        } => {
            println!(
                "  ✅ Found {} functions in {}ms:",
                total_matches, query_time_ms
            );
            for query_match in matches {
                for capture in &query_match.captures {
                    if capture.name == "func_name" {
                        println!(
                            "    - {} at line {}",
                            capture.text,
                            capture.start_point.row + 1
                        );
                    }
                }
            }
        }
        _ => println!("  ❌ Unexpected output type"),
    }
    println!();

    // Example 3: Parse Python code
    println!("3. Parsing Python Code");
    let python_code = r#"
class DataProcessor:
    def __init__(self, data):
        self.data = data
        self.processed = False
    
    def process(self):
        """Process the data and return results."""
        if not self.processed:
            self.data = [x * 2 for x in self.data]
            self.processed = True
        return self.data
    
    def reset(self):
        self.processed = False

def main():
    processor = DataProcessor([1, 2, 3, 4, 5])
    result = processor.process()
    print(f"Processed data: {result}")

if __name__ == "__main__":
    main()
    "#;

    let parse_input = TreeInput::Parse {
        code: python_code.to_string(),
        language: Some(SupportedLanguage::Python),
        file_path: None,
    };

    let result = tree_tool.execute(parse_input).await?;
    match &result.result {
        TreeOutput::Parsed {
            language,
            node_count,
            has_errors,
            error_count,
            parse_time_ms,
        } => {
            println!("  ✅ Language: {:?}", language);
            println!("  ✅ Nodes: {}", node_count);
            println!("  ✅ Errors: {} ({})", has_errors, error_count);
            println!("  ✅ Parse time: {}ms", parse_time_ms);
        }
        _ => println!("  ❌ Unexpected output type"),
    }
    println!();

    // Example 4: Query Python class definitions
    println!("4. Query Python Classes");
    let python_query = "(class_definition name: (identifier) @class_name)";

    let query_input = TreeInput::Query {
        code: python_code.to_string(),
        language: Some(SupportedLanguage::Python),
        pattern: python_query.to_string(),
        file_path: None,
    };

    let result = tree_tool.execute(query_input).await?;
    match &result.result {
        TreeOutput::QueryResults {
            matches,
            total_matches,
            query_time_ms,
        } => {
            println!(
                "  ✅ Found {} classes in {}ms:",
                total_matches, query_time_ms
            );
            for query_match in matches {
                for capture in &query_match.captures {
                    if capture.name == "class_name" {
                        println!(
                            "    - {} at line {}",
                            capture.text,
                            capture.start_point.row + 1
                        );
                    }
                }
            }
        }
        _ => println!("  ❌ Unexpected output type"),
    }
    println!();

    // Example 5: JavaScript/TypeScript parsing
    println!("5. Parsing TypeScript Code");
    let typescript_code = r#"
interface User {
    id: number;
    name: string;
    email?: string;
}

class UserService {
    private users: User[] = [];
    
    constructor() {
        this.loadUsers();
    }
    
    async loadUsers(): Promise<void> {
        // Simulate API call
        this.users = [
            { id: 1, name: "Alice", email: "alice@example.com" },
            { id: 2, name: "Bob" }
        ];
    }
    
    findUser(id: number): User | undefined {
        return this.users.find(user => user.id === id);
    }
    
    addUser(user: Omit<User, 'id'>): User {
        const newUser: User = {
            id: Math.max(...this.users.map(u => u.id)) + 1,
            ...user
        };
        this.users.push(newUser);
        return newUser;
    }
}

const userService = new UserService();
    "#;

    let parse_input = TreeInput::Parse {
        code: typescript_code.to_string(),
        language: Some(SupportedLanguage::TypeScript),
        file_path: None,
    };

    let result = tree_tool.execute(parse_input).await?;
    match &result.result {
        TreeOutput::Parsed {
            language,
            node_count,
            has_errors,
            error_count,
            parse_time_ms,
        } => {
            println!("  ✅ Language: {:?}", language);
            println!("  ✅ Nodes: {}", node_count);
            println!("  ✅ Errors: {} ({})", has_errors, error_count);
            println!("  ✅ Parse time: {}ms", parse_time_ms);
        }
        _ => println!("  ❌ Unexpected output type"),
    }
    println!();

    // Example 6: Extract symbols
    println!("6. Symbol Extraction from Rust Code");
    let symbol_input = TreeInput::ExtractSymbols {
        code: rust_code.to_string(),
        language: Some(SupportedLanguage::Rust),
        file_path: None,
    };

    let result = tree_tool.execute(symbol_input).await?;
    match &result.result {
        TreeOutput::Symbols {
            symbols,
            total_symbols,
            extraction_time_ms,
        } => {
            println!(
                "  ✅ Extracted {} symbols in {}ms",
                total_symbols, extraction_time_ms
            );
            // Note: Symbol extraction would need implementation for each language
            if symbols.is_empty() {
                println!("    (Symbol extraction implementation pending for Rust)");
            }
        }
        _ => println!("  ❌ Unexpected output type"),
    }
    println!();

    // Example 7: Code diffing
    println!("7. Semantic Code Diff");
    let old_rust_code = r#"
        fn calculate(a: i32, b: i32) -> i32 {
            a + b
        }
    "#;

    let new_rust_code = r#"
        fn calculate(a: i32, b: i32, c: i32) -> i32 {
            a + b + c
        }
        
        fn multiply(x: i32, y: i32) -> i32 {
            x * y
        }
    "#;

    let diff_input = TreeInput::Diff {
        old_code: old_rust_code.to_string(),
        new_code: new_rust_code.to_string(),
        language: Some(SupportedLanguage::Rust),
    };

    let result = tree_tool.execute(diff_input).await?;
    match &result.result {
        TreeOutput::Diff { diff, diff_time_ms } => {
            println!("  ✅ Computed diff in {}ms", diff_time_ms);
            println!("  ✅ Similarity: {:.2}%", diff.similarity_score * 100.0);
            println!("  ✅ Added: {} nodes", diff.added.len());
            println!("  ✅ Removed: {} nodes", diff.removed.len());
            println!("  ✅ Modified: {} nodes", diff.modified.len());
        }
        _ => println!("  ❌ Unexpected output type"),
    }
    println!();

    // Example 8: Cache performance demonstration
    println!("8. Cache Performance Test");
    let start = std::time::Instant::now();

    // Parse the same code multiple times to demonstrate caching
    for i in 0..5 {
        let parse_input = TreeInput::Parse {
            code: rust_code.to_string(),
            language: Some(SupportedLanguage::Rust),
            file_path: None,
        };

        let result = tree_tool.execute(parse_input).await?;
        if let TreeOutput::Parsed { parse_time_ms, .. } = result.result {
            println!("  Parse #{}: {}ms", i + 1, parse_time_ms);
        }
    }

    let total_time = start.elapsed();
    println!("  ✅ Total time for 5 parses: {:?}", total_time);
    println!("  ✅ Average per parse: {:?}", total_time / 5);

    // Show cache statistics
    let cache_stats = tree_tool.cache_stats()?;
    println!("  📊 Cache Statistics:");
    for (key, value) in cache_stats {
        println!("    - {}: {:?}", key, value);
    }

    println!("\n🎉 All examples completed successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_multi_language_parsing() {
        let tool = TreeTool::new(IntelligenceLevel::Light).unwrap();

        let test_cases = vec![
            ("fn main() {}", SupportedLanguage::Rust),
            ("def main(): pass", SupportedLanguage::Python),
            ("function main() {}", SupportedLanguage::JavaScript),
            ("func main() {}", SupportedLanguage::Go),
            ("public class Main {}", SupportedLanguage::Java),
            ("int main() { return 0; }", SupportedLanguage::C),
        ];

        for (code, lang) in test_cases {
            let input = TreeInput::Parse {
                code: code.to_string(),
                language: Some(lang),
                file_path: None,
            };

            let result = tool.execute(input).await;
            assert!(
                result.is_ok(),
                "Failed to parse {} code: {}",
                lang.as_str(),
                code
            );

            if let Ok(output) = result {
                assert!(
                    output.success,
                    "Parse was not successful for {}",
                    lang.as_str()
                );
                match output.result {
                    TreeOutput::Parsed { has_errors, .. } => {
                        assert!(!has_errors, "Parse had errors for {}", lang.as_str());
                    }
                    _ => panic!("Expected Parsed output"),
                }
            }
        }
    }

    #[tokio::test]
    async fn test_query_functionality() {
        let tool = TreeTool::new(IntelligenceLevel::Medium).unwrap();

        let rust_code = r#"
            fn first() -> i32 { 1 }
            fn second() -> i32 { 2 }  
            fn third() -> i32 { 3 }
        "#;

        let input = TreeInput::Query {
            code: rust_code.to_string(),
            language: Some(SupportedLanguage::Rust),
            pattern: "(function_item name: (identifier) @func_name)".to_string(),
            file_path: None,
        };

        let result = tool.execute(input).await.unwrap();
        assert!(result.success);

        match result.result {
            TreeOutput::QueryResults { total_matches, .. } => {
                assert_eq!(total_matches, 3, "Should find exactly 3 functions");
            }
            _ => panic!("Expected QueryResults output"),
        }
    }

    #[tokio::test]
    async fn test_caching_behavior() {
        let tool = TreeTool::new(IntelligenceLevel::Light).unwrap();
        let code = "fn test() -> i32 { 42 }".to_string();

        // Parse the same code twice
        let input1 = TreeInput::Parse {
            code: code.clone(),
            language: Some(SupportedLanguage::Rust),
            file_path: None,
        };

        let input2 = TreeInput::Parse {
            code,
            language: Some(SupportedLanguage::Rust),
            file_path: None,
        };

        let _result1 = tool.execute(input1).await.unwrap();
        let _result2 = tool.execute(input2).await.unwrap();

        // Check cache statistics
        let stats = tool.cache_stats().unwrap();
        let cache_size = stats.get("ast_cache_size").unwrap().as_i64().unwrap();
        assert_eq!(cache_size, 1, "Should have 1 cached entry");
    }
}
