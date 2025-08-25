//! Demonstration of the enhanced AST compactor with aggressive compression levels

use agcodex_core::context_engine::AstCompactor;
use agcodex_core::context_engine::CompactOptions;
use agcodex_core::context_engine::CompressionLevel;
use agcodex_core::models::ContentItem;
use agcodex_core::models::ResponseItem;

fn main() {
    println!("=== AGCodex AST Compactor Demo ===\n");

    // Sample Rust code for compression
    let rust_code = r#"
use std::collections::HashMap;
use std::sync::Arc;

/// Configuration for the application
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Database connection string
    pub database_url: String,
    /// Server port
    pub port: u16,
    /// Enable debug mode
    pub debug: bool,
    /// Cache settings
    cache: CacheConfig,
}

impl AppConfig {
    /// Create a new configuration with defaults
    pub fn new() -> Self {
        Self {
            database_url: "postgres://localhost/app".to_string(),
            port: 8080,
            debug: false,
            cache: CacheConfig::default(),
        }
    }
    
    /// Load configuration from environment
    pub fn from_env() -> Result<Self, String> {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL not set")?;
        
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .map_err(|_| "Invalid PORT")?;
            
        Ok(Self {
            database_url,
            port,
            debug: std::env::var("DEBUG").is_ok(),
            cache: CacheConfig::default(),
        })
    }
    
    /// Private helper method
    fn validate(&self) -> bool {
        !self.database_url.is_empty() && self.port > 0
    }
}

#[derive(Debug, Clone, Default)]
struct CacheConfig {
    size: usize,
    ttl: u64,
}
"#;

    let compactor = AstCompactor::new();

    println!("Original code size: {} bytes\n", rust_code.len());
    println!("Testing compression levels:\n");

    // Test different compression levels
    for level in [
        CompressionLevel::Light,
        CompressionLevel::Medium,
        CompressionLevel::Hard,
    ] {
        println!("--- {} ---", level.description());

        let opts = CompactOptions {
            compression_level: level,
            preserve_mappings: false,
            precision_high: false,
            include_weights: true,
        };

        let result = compactor.compact_source(rust_code, &opts);

        println!("Compressed size: {} bytes", result.compacted.len());
        println!(
            "Token reduction: {} → {} ({}% compression)",
            result.original_tokens,
            result.compressed_tokens,
            (result.compression_ratio * 100.0) as u32
        );

        if let Some(weights) = &result.semantic_weights {
            let high_weight_items: Vec<_> = weights.iter().filter(|(_, w)| **w >= 0.7).collect();

            if !high_weight_items.is_empty() {
                println!("High-weight preserved items:");
                for (name, weight) in high_weight_items.iter().take(5) {
                    println!("  - {}: {:.2}", name, weight);
                }
            }
        }

        println!("\nCompressed output preview (first 300 chars):");
        let preview = if result.compacted.len() > 300 {
            &result.compacted[..300]
        } else {
            &result.compacted
        };
        println!("{}", preview);

        if result.compacted.len() > 300 {
            println!("... [truncated]");
        }

        println!("\n");
    }

    // Test thread compression
    println!("=== Thread Compression Demo ===\n");

    let messages = vec![
        ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![ContentItem::InputText {
                text: format!(
                    "Here's my Rust code that needs review:\n```rust\n{}\n```\nCan you help optimize it?",
                    rust_code
                ),
            }],
        },
        ResponseItem::Message {
            id: None,
            role: "assistant".to_string(),
            content: vec![ContentItem::OutputText {
                text: "I'll analyze your code. Here are the key observations:\n\n\
                      1. The configuration structure looks good\n\
                      2. Consider using a builder pattern for complex configs\n\
                      3. The validation method could be more comprehensive\n\n\
                      Let me show you an optimized version:\n\n\
                      ```rust\n\
                      // Optimized implementation would go here\n\
                      pub struct AppConfigBuilder { /* ... */ }\n\
                      ```"
                .to_string(),
            }],
        },
    ];

    let (compressed_thread, metrics) =
        compactor.compress_thread(&messages, CompressionLevel::Medium);

    println!("Thread Compression Metrics:");
    println!("  Messages processed: {}", metrics.messages_processed);
    println!(
        "  Code blocks compressed: {}",
        metrics.code_blocks_compressed
    );
    println!(
        "  Token reduction: {} → {} tokens",
        metrics.original_tokens, metrics.compressed_tokens
    );
    println!(
        "  Compression ratio: {:.1}%",
        metrics.compression_ratio * 100.0
    );
    println!("  Processing time: {}ms", metrics.time_ms);

    // Show the difference in message content
    if let ResponseItem::Message { content, .. } = &compressed_thread[0]
        && let ContentItem::InputText { text } = &content[0]
    {
        let original_len = messages[0]
            .clone()
            .into_message()
            .map(|(_, _, c)| c[0].clone().into_text().unwrap_or_default().len())
            .unwrap_or(0);

        println!(
            "\nFirst message size change: {} → {} bytes ({:.1}% reduction)",
            original_len,
            text.len(),
            (1.0 - text.len() as f32 / original_len as f32) * 100.0
        );
    }

    println!("\n=== Compression Level Summary ===");
    println!("Light (70%): Best for code review - preserves comments and structure");
    println!("Medium (85%): Balanced for general use - keeps public APIs");
    println!("Hard (95%): Maximum compression - only critical types remain");
    println!("\nUse case recommendations:");
    println!("- Long conversation threads: Use Hard compression for context");
    println!("- Code analysis: Use Medium for good balance");
    println!("- Documentation generation: Use Light to preserve details");
}

// Helper trait to work with ResponseItem
trait ResponseItemExt {
    fn into_message(self) -> Option<(Option<String>, String, Vec<ContentItem>)>;
}

impl ResponseItemExt for ResponseItem {
    fn into_message(self) -> Option<(Option<String>, String, Vec<ContentItem>)> {
        match self {
            ResponseItem::Message { id, role, content } => Some((id, role, content)),
            _ => None,
        }
    }
}

// Helper trait for ContentItem
trait ContentItemExt {
    fn into_text(self) -> Option<String>;
}

impl ContentItemExt for ContentItem {
    fn into_text(self) -> Option<String> {
        match self {
            ContentItem::InputText { text } | ContentItem::OutputText { text } => Some(text),
            _ => None,
        }
    }
}
