/// Test module demonstrating type-safe patterns implementation
///
/// This module validates that our newtype wrappers, builders, and state machines
/// compile and provide the intended compile-time safety guarantees.

#[cfg(test)]
mod tests {
    
    
    use crate::builders::*;
    
    
    use crate::types::*;
    

    #[test]
    fn test_filepath_validation() {
        // Valid file paths should work
        let valid_path = FilePath::try_from("/tmp/test.txt").unwrap();
        assert_eq!(valid_path.as_path_buf().to_str().unwrap(), "/tmp/test.txt");

        // Invalid paths should fail
        assert!(FilePath::try_from("../../../etc/passwd").is_err());
        assert!(FilePath::try_from("/tmp/test\0.txt").is_err());
        assert!(FilePath::try_from("").is_err());
    }

    #[test]
    fn test_ast_node_id_nonzero() {
        // Should enforce NonZeroUsize constraint
        let node_id = AstNodeId::try_from(42).unwrap();
        assert_eq!(node_id.get(), 42);

        // Zero should fail
        assert!(AstNodeId::try_from(0).is_err());
    }

    #[test]
    fn test_query_pattern_validation() {
        // Valid regex patterns should work
        let pattern = QueryPattern::try_from(r"\d+").unwrap();
        assert!(pattern.is_regex());

        // Simple string patterns should work
        let simple = QueryPattern::try_from("hello world").unwrap();
        assert!(!simple.is_regex());

        // Invalid regex should fail
        assert!(QueryPattern::try_from(r"[invalid").is_err());
    }

    #[test]
    fn test_context_window_size_limits() {
        let small_context = ContextWindow::new("Small content".to_string()).unwrap();
        assert!(small_context.estimated_tokens() > 0);

        // Test size limits
        let large_content = "x".repeat(2_000_000); // 2MB
        assert!(ContextWindow::new(large_content).is_err());
    }

    #[test]
    fn test_fixed_buffer_const_generics() {
        let mut buffer: FixedBuffer<1024> = FixedBuffer::new();
        let data = b"Hello, world!";

        // Push bytes one by one
        for &byte in data {
            buffer.push(byte).unwrap();
        }
        assert_eq!(buffer.len(), data.len());
        assert_eq!(&buffer.as_slice()[..data.len()], data);
    }

    #[test]
    fn test_search_query_builder_typestate() {
        // Builder should enforce proper state transitions
        let builder = SearchQueryBuilder::new();

        // Can't build without setting required fields (this should fail to compile)
        // let invalid_query = builder.build(); // ← This would be a compile error

        // Proper usage with state transitions
        let query = builder
            .pattern("*.rs")
            .unwrap()
            .scope_current_dir()
            .build()
            .unwrap();

        // The query should be properly constructed
        assert!(query.pattern().as_str().contains("*.rs"));
    }

    #[test]
    fn test_search_engine_state_machine() {
        use crate::state_machines::search_engine::*;

        // State machine should enforce proper state transitions
        let engine = SearchEngine::new(SearchConfig::default());

        // Test state transitions
        let files = vec![];
        let indexing_engine = engine.start_indexing(files).unwrap();
        let ready_engine = indexing_engine.complete_indexing();

        // Should be able to execute searches in ready state
        // (Implementation would be more complex in real usage)
    }

    #[test]
    fn test_ast_query_builder_language_safety() {
        let builder = AstQueryBuilder::new();

        let file = FilePath::new("test.rs").unwrap();
        let query = builder
            .language(AstLanguage::Rust)
            .add_file(file)
            .select_node_type("function_item")
            .build()
            .unwrap();

        assert_eq!(query.language(), &AstLanguage::Rust);
    }

    #[test]
    fn test_config_builder_with_validation() {
        let builder = ConfigBuilder::new();

        let config = builder
            .model_name("test-model")
            .unwrap()
            .max_concurrent_requests(std::num::NonZeroUsize::new(4).unwrap())
            .cache_size(std::num::NonZeroUsize::new(100).unwrap())
            .timeout(std::time::Duration::from_secs(30))
            .finalize()
            .build()
            .unwrap();

        assert_eq!(config.performance.max_concurrent_requests.get(), 4);
        assert_eq!(config.performance.cache_size.get(), 100);
        assert_eq!(config.model.timeout, std::time::Duration::from_secs(30));
    }

    #[test]
    fn test_context_builder_priority_system() {
        let builder = ContextBuilder::new();

        let context = builder
            .max_size(4096)
            .unwrap()
            .compression_strategy(CompressionStrategy::PriorityBased)
            .add_item(
                "test content",
                ContextPriority::Critical,
                ContextSource::UserInput,
            )
            .unwrap()
            .add_user_input("more content")
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(context.config().max_size, 4096);
        assert_eq!(
            context.config().compression_strategy,
            CompressionStrategy::PriorityBased
        );
    }
}
