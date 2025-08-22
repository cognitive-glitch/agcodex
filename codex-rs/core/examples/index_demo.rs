//! Demonstration of the Tantivy IndexTool
//!
//! This example shows how to use the IndexTool to:
//! 1. Build an index for a directory
//! 2. Search the index for code
//! 3. Update the index with changes
//! 4. Get statistics about the index

use agcodex_core::tools::BuildInput;
use agcodex_core::tools::IndexConfig;
use agcodex_core::tools::IndexTool;
use agcodex_core::tools::SearchInput;
use agcodex_core::tools::SearchQuery;
use agcodex_core::tools::UpdateInput;
use std::fs;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (optional - add tracing_subscriber as dev-dependency if needed)
    // tracing_subscriber::fmt::init();

    // Create a temporary directory with some sample code files
    let temp_dir = TempDir::new()?;
    let source_dir = temp_dir.path().join("source");
    fs::create_dir_all(&source_dir)?;

    // Create sample Rust file
    let rust_file = source_dir.join("main.rs");
    fs::write(
        &rust_file,
        r#"
fn main() {
    println!("Hello, world!");
    let numbers = vec![1, 2, 3, 4, 5];
    let sum = calculate_sum(&numbers);
    println!("Sum: {}", sum);
}

fn calculate_sum(numbers: &[i32]) -> i32 {
    numbers.iter().sum()
}

struct Person {
    name: String,
    age: u32,
}

impl Person {
    fn new(name: String, age: u32) -> Self {
        Person { name, age }
    }
    
    fn greet(&self) {
        println!("Hello, my name is {}", self.name);
    }
}
"#,
    )?;

    // Create sample Python file
    let python_file = source_dir.join("utils.py");
    fs::write(
        &python_file,
        r#"
def fibonacci(n):
    """Calculate the nth Fibonacci number."""
    if n <= 1:
        return n
    return fibonacci(n-1) + fibonacci(n-2)

class Calculator:
    """A simple calculator class."""
    
    def __init__(self):
        self.history = []
    
    def add(self, a, b):
        """Add two numbers."""
        result = a + b
        self.history.append(f"{a} + {b} = {result}")
        return result
    
    def multiply(self, a, b):
        """Multiply two numbers."""
        result = a * b
        self.history.append(f"{a} * {b} = {result}")
        return result
"#,
    )?;

    // Create sample JavaScript file
    let js_file = source_dir.join("script.js");
    fs::write(
        &js_file,
        r#"
function processArray(arr) {
    return arr
        .filter(x => x > 0)
        .map(x => x * 2)
        .reduce((sum, x) => sum + x, 0);
}

class EventEmitter {
    constructor() {
        this.events = {};
    }
    
    on(event, callback) {
        if (!this.events[event]) {
            this.events[event] = [];
        }
        this.events[event].push(callback);
    }
    
    emit(event, ...args) {
        if (this.events[event]) {
            this.events[event].forEach(callback => callback(...args));
        }
    }
}

const emitter = new EventEmitter();
emitter.on('data', data => console.log('Received:', data));
"#,
    )?;

    // Configure the index
    let index_dir = temp_dir.path().join("index");
    let config = IndexConfig {
        index_path: index_dir,
        include_extensions: vec!["rs".to_string(), "py".to_string(), "js".to_string()],
        max_file_size: 10 * 1024 * 1024, // 10MB
        incremental: true,
        writer_memory_mb: 64, // Small for demo
        num_threads: Some(2),
        ..Default::default()
    };

    println!("🏗️  Building index for directory: {:?}", source_dir);

    // Create and initialize the IndexTool
    let index_tool = IndexTool::new(config.clone())?;

    // Build the index
    let build_input = BuildInput {
        directory: source_dir.clone(),
        config: config.clone(),
        force_rebuild: true,
    };

    let stats = index_tool.build(build_input).await?;
    println!("✅ Index built successfully!");
    println!("📊 Statistics:");
    println!("   - Documents: {}", stats.result.document_count);
    println!("   - Size: {} bytes", stats.result.size_bytes);
    println!("   - Segments: {}", stats.result.segment_count);
    println!("   - Languages: {:?}", stats.result.language_stats);

    // Demonstrate search functionality
    println!("\n🔍 Searching for 'function'...");

    let search_query = SearchQuery {
        query: "function".to_string(),
        language: None,
        path_filter: None,
        symbol_type: None,
        limit: Some(10),
        fuzzy: false,
        min_score: None,
    };

    let search_input = SearchInput {
        query: search_query,
        config: config.clone(),
    };

    let results = index_tool.search(search_input).await?;
    println!("Found {} results:", results.result.len());

    for (i, result) in results.result.iter().enumerate().take(3) {
        println!(
            "{}. {} (score: {:.2})",
            i + 1,
            result.document.path,
            result.score
        );
        println!("   Language: {}", result.document.language);
        println!("   Size: {} bytes", result.document.size);
        if !result.document.symbols.is_empty() {
            println!("   Symbols: {} found", result.document.symbols.len());
        }
    }

    // Demonstrate language-specific search
    println!("\n🐍 Searching for Python files with 'class'...");

    let python_search = SearchQuery {
        query: "class".to_string(),
        language: Some("python".to_string()),
        path_filter: None,
        symbol_type: None,
        limit: Some(5),
        fuzzy: false,
        min_score: None,
    };

    let python_search_input = SearchInput {
        query: python_search,
        config: config.clone(),
    };

    let python_results = index_tool.search(python_search_input).await?;
    println!("Found {} Python results:", python_results.result.len());

    for result in python_results.result {
        println!("- {} (score: {:.2})", result.document.path, result.score);
    }

    // Demonstrate incremental update
    println!("\n🔄 Adding a new file and updating index...");

    let new_file = source_dir.join("new_module.rs");
    fs::write(
        &new_file,
        r#"
/// A new module for demonstration
pub mod demo {
    use std::collections::HashMap;
    
    /// Configuration struct
    pub struct Config {
        pub name: String,
        pub settings: HashMap<String, String>,
    }
    
    impl Config {
        /// Create a new configuration
        pub fn new(name: String) -> Self {
            Self {
                name,
                settings: HashMap::new(),
            }
        }
        
        /// Add a setting
        pub fn add_setting(&mut self, key: String, value: String) {
            self.settings.insert(key, value);
        }
    }
    
    /// Process configuration
    pub fn process_config(config: &Config) -> String {
        format!("Processing config: {}", config.name)
    }
}
"#,
    )?;

    // Update the index with the new file
    let update_input = UpdateInput {
        files: vec![new_file.clone()],
        config: config.clone(),
    };

    let updated_stats = index_tool.update(update_input).await?;
    println!("✅ Index updated!");
    println!("📊 New statistics:");
    println!("   - Documents: {}", updated_stats.result.document_count);
    println!("   - Size: {} bytes", updated_stats.result.size_bytes);

    // Search for the new content
    println!("\n🔍 Searching for 'HashMap'...");

    let hashmap_search = SearchQuery {
        query: "HashMap".to_string(),
        language: None,
        path_filter: None,
        symbol_type: None,
        limit: Some(5),
        fuzzy: false,
        min_score: None,
    };

    let hashmap_search_input = SearchInput {
        query: hashmap_search,
        config: config.clone(),
    };

    let hashmap_results = index_tool.search(hashmap_search_input).await?;
    println!(
        "Found {} results containing 'HashMap':",
        hashmap_results.result.len()
    );

    for result in hashmap_results.result {
        println!("- {} (score: {:.2})", result.document.path, result.score);
    }

    // Demonstrate optimization
    println!("\n⚡ Optimizing index...");
    index_tool.optimize().await?;
    println!("✅ Index optimization completed!");

    // Final statistics
    let final_stats = index_tool.stats().await?;
    println!("\n📈 Final Index Statistics:");
    println!(
        "   - Total documents: {}",
        final_stats.result.document_count
    );
    println!("   - Total size: {} bytes", final_stats.result.size_bytes);
    println!("   - Segments: {}", final_stats.result.segment_count);
    println!(
        "   - Average document size: {:.2} bytes",
        final_stats.result.avg_document_size
    );

    if !final_stats.result.language_stats.is_empty() {
        println!("   - Language distribution:");
        for (lang, count) in &final_stats.result.language_stats {
            println!("     • {}: {} files", lang, count);
        }
    }

    println!("\n🎉 IndexTool demonstration completed successfully!");

    Ok(())
}
