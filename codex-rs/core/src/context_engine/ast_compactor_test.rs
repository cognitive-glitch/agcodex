//! Tests for the enhanced AST compactor

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::models::ContentItem;
    use crate::models::ResponseItem;

    #[test]
    fn test_compression_levels_produce_different_results() {
        let compactor = AstCompactor::new();

        let code = r#"
        // This is a comment
        pub struct TestStruct {
            pub field1: String,
            private_field: u32,
        }
        
        impl TestStruct {
            pub fn new() -> Self {
                Self {
                    field1: String::new(),
                    private_field: 0,
                }
            }
            
            fn private_method(&self) {
                println!("Private");
            }
        }
        "#;

        let light_opts = CompactOptions {
            compression_level: CompressionLevel::Light,
            ..Default::default()
        };

        let medium_opts = CompactOptions {
            compression_level: CompressionLevel::Medium,
            ..Default::default()
        };

        let hard_opts = CompactOptions {
            compression_level: CompressionLevel::Hard,
            ..Default::default()
        };

        let light_result = compactor.compact_source(code, &light_opts);
        let medium_result = compactor.compact_source(code, &medium_opts);
        let hard_result = compactor.compact_source(code, &hard_opts);

        // Verify compression increases with level
        assert!(light_result.compression_ratio < medium_result.compression_ratio);
        assert!(medium_result.compression_ratio < hard_result.compression_ratio);

        // Verify token counts decrease with higher compression
        assert!(light_result.compressed_tokens > medium_result.compressed_tokens);
        assert!(medium_result.compressed_tokens > hard_result.compressed_tokens);

        // Verify all have some compression
        assert!(light_result.compression_ratio > 0.0);
        assert!(medium_result.compression_ratio > 0.0);
        assert!(hard_result.compression_ratio > 0.0);
    }

    #[test]
    fn test_semantic_weights_calculation() {
        let compactor = AstCompactor::new();

        let opts = CompactOptions {
            compression_level: CompressionLevel::Medium,
            include_weights: true,
            ..Default::default()
        };

        let code = "pub struct Test { pub field: String }";
        let result = compactor.compact_source(code, &opts);

        assert!(
            result.semantic_weights.is_some(),
            "Expected semantic_weights to be Some when include_weights is true"
        );

        if let Some(weights) = result.semantic_weights {
            // Debug: print available keys if test fails
            let keys: Vec<String> = weights.keys().cloned().collect();

            // Check for struct-related keys (struct_item exists in the implementation)
            let struct_weight = weights.get("struct_item").copied();
            assert!(
                struct_weight.is_some(),
                "Expected 'struct_item' key in weights. Available keys: {:?}",
                keys
            );
            assert!(
                struct_weight.unwrap() > 0.8,
                "struct_item weight should be > 0.8, got {}",
                struct_weight.unwrap()
            );

            // Check for comment-related keys (line_comment exists in the implementation)
            let comment_weight = weights.get("line_comment").copied();
            assert!(
                comment_weight.is_some(),
                "Expected 'line_comment' key in weights. Available keys: {:?}",
                keys
            );
            assert!(
                comment_weight.unwrap() < 0.3,
                "line_comment weight should be < 0.3, got {}",
                comment_weight.unwrap()
            );
        }
    }

    #[test]
    fn test_thread_compression() {
        let compactor = AstCompactor::new();

        let code_block = r#"
        fn example() {
            let x = 42;
            println!("The answer is {}", x);
            // This comment will be removed at medium compression
            let result = x * 2;
            return result;
        }
        "#;

        let messages = vec![
            ResponseItem::Message {
                id: None,
                role: "user".to_string(),
                content: vec![ContentItem::InputText {
                    text: format!("Check this code:\n```rust\n{}\n```", code_block),
                }],
            },
            ResponseItem::Message {
                id: None,
                role: "assistant".to_string(),
                content: vec![ContentItem::OutputText {
                    text: "I'll review your code.".to_string(),
                }],
            },
        ];

        let (compressed, metrics) = compactor.compress_thread(&messages, CompressionLevel::Medium);

        // Verify metrics
        assert_eq!(metrics.messages_processed, 2);
        assert_eq!(metrics.code_blocks_compressed, 1);
        assert!(metrics.compression_ratio > 0.0);
        assert!(metrics.compressed_tokens < metrics.original_tokens);

        // Verify messages are preserved
        assert_eq!(compressed.len(), 2);

        // Verify code block was compressed
        if let ResponseItem::Message { content, .. } = &compressed[0]
            && let ContentItem::InputText { text } = &content[0]
        {
            assert!(text.contains("```"));
            // Original text should be longer than compressed
            let original_text = match &messages[0] {
                ResponseItem::Message { content, .. } => match &content[0] {
                    ContentItem::InputText { text } => text,
                    _ => "",
                },
                _ => "",
            };
            assert!(original_text.len() >= text.len());
        }
    }

    #[test]
    fn test_language_detection() {
        let compactor = AstCompactor::new();

        let rust_code = "fn main() { println!(\"Hello\"); }";
        let python_code = "import sys\nfrom os import path\ndef main(): pass";
        let js_code = "const app = () => { console.log('test'); };";
        let ts_code = "interface User { name: string; age: number; }";
        let go_code = "package main\nfunc main() { fmt.Println(\"Hello\") }";

        // Test that different languages are detected
        let rust_result = compactor.detect_language_from_content(rust_code);
        let python_result = compactor.detect_language_from_content(python_code);
        let js_result = compactor.detect_language_from_content(js_code);
        let ts_result = compactor.detect_language_from_content(ts_code);
        let go_result = compactor.detect_language_from_content(go_code);

        assert!(rust_result.is_some());
        assert!(python_result.is_some());
        assert!(js_result.is_some());
        assert!(ts_result.is_some());
        assert!(go_result.is_some());
    }

    #[test]
    fn test_fallback_compression() {
        let compactor = AstCompactor::new();

        // Use text that won't parse as valid code
        let text = "This is not valid code but has some keywords class function";

        let opts = CompactOptions {
            compression_level: CompressionLevel::Hard,
            ..Default::default()
        };

        let result = compactor.compact_source(text, &opts);

        // Should still produce some result even if AST parsing fails
        assert!(!result.compacted.is_empty());
        assert!(result.original_tokens > 0);
        assert!(result.compressed_tokens > 0);
    }

    #[test]
    fn test_code_block_detection() {
        let compactor = AstCompactor::new();

        assert!(compactor.is_code_block("```rust\nfn main() {}\n```"));
        assert!(compactor.is_code_block("~~~python\nprint('hello')\n~~~"));
        assert!(!compactor.is_code_block("This is just text"));

        // Multi-line code-looking text
        let code_like =
            "function test() {\n  console.log('test');\n  return true;\n}\nconst x = 42;";
        assert!(compactor.is_code_block(code_like));
    }

    #[test]
    fn test_compression_target_ratios() {
        assert_eq!(CompressionLevel::Light.target_ratio(), 0.70);
        assert_eq!(CompressionLevel::Medium.target_ratio(), 0.85);
        assert_eq!(CompressionLevel::Hard.target_ratio(), 0.95);
    }

    #[test]
    fn test_thread_compression_metrics_accuracy() {
        let compactor = AstCompactor::new();

        let messages = vec![ResponseItem::Message {
            id: Some("msg1".to_string()),
            role: "user".to_string(),
            content: vec![
                ContentItem::InputText {
                    text: "Here's code:\n```rust\nfn test() { println!(\"test\"); }\n```"
                        .to_string(),
                },
                ContentItem::InputText {
                    text: "And more:\n```python\ndef test(): print('test')\n```".to_string(),
                },
            ],
        }];

        let (compressed, metrics) = compactor.compress_thread(&messages, CompressionLevel::Medium);

        assert_eq!(metrics.messages_processed, 1);
        assert_eq!(metrics.code_blocks_compressed, 2); // Two code blocks
        assert_eq!(metrics.compression_level, CompressionLevel::Medium);
        // time_ms is u64, always >= 0

        // Verify the message structure is preserved
        assert_eq!(compressed.len(), 1);
        if let ResponseItem::Message { id, role, content } = &compressed[0] {
            assert_eq!(id, &Some("msg1".to_string()));
            assert_eq!(role, "user");
            assert_eq!(content.len(), 2); // Two content items preserved
        }
    }
}
