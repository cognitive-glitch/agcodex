use agcodex_core::semantic_index::IndexingOptions;
use agcodex_core::semantic_index::SearchQuery;
use agcodex_core::semantic_index::SemanticIndexConfig;
use agcodex_core::semantic_index::SemanticIndexer;
use tempfile::TempDir;

#[tokio::test]
async fn test_semantic_indexing_pipeline() {
    let temp_dir = TempDir::new().unwrap();
    let index_path = temp_dir.path().to_path_buf();

    let config = agcodex_core::semantic_index::SemanticIndexConfig {
        max_chunk_size: 512,
        cache_size: 100,
        enable_persistence: false,
        storage_path: Some(index_path.clone()),
        ..SemanticIndexConfig::default()
    };

    let indexer = SemanticIndexer::new(config).unwrap();

    // Create test files
    let test_file = index_path.join("test.rs");
    std::fs::write(
        &test_file,
        r#"
pub fn authenticate_user(username: &str, password: &str) -> bool {
    // Authentication logic here
    validate_credentials(username, password)
}

fn validate_credentials(user: &str, pass: &str) -> bool {
    // Validation logic
    true
}

pub struct UserSession {
    pub username: String,
    pub token: String,
}

impl UserSession {
    pub fn new(username: String) -> Self {
        Self {
            username,
            token: generate_token(),
        }
    }
}

fn generate_token() -> String {
    // Token generation
    "token123".to_string()
}
"#,
    )
    .unwrap();

    // Index the file
    indexer.index_file(&test_file).await.unwrap();

    // Search for authentication-related code
    let query = SearchQuery::new("user authentication").with_limit(5);

    let results = indexer.search(query).await.unwrap();

    assert!(!results.is_empty());
    assert!(results[0].content.contains("authenticate_user"));
    assert!(results[0].ranking_score() > 0.5);
}

#[tokio::test]
async fn test_incremental_indexing() {
    let temp_dir = TempDir::new().unwrap();
    let index_path = temp_dir.path().to_path_buf();

    let config = agcodex_core::semantic_index::SemanticIndexConfig {
        enable_persistence: false,
        storage_path: Some(index_path.clone()),
        ..SemanticIndexConfig::default()
    };

    let indexer = SemanticIndexer::new(config).unwrap();

    // Initial file
    let file1 = index_path.join("file1.rs");
    std::fs::write(&file1, "pub fn initial_function() {}").unwrap();
    indexer.index_file(&file1).await.unwrap();

    // Check that indexing worked
    let query = SearchQuery::new("initial_function");
    let results = indexer.search(query).await.unwrap();
    assert!(!results.is_empty());

    // Add second file
    let file2 = index_path.join("file2.rs");
    std::fs::write(&file2, "pub fn another_function() {}").unwrap();
    indexer.index_file(&file2).await.unwrap();

    // Check that both files are indexed
    let query = SearchQuery::new("function");
    let results = indexer.search(query).await.unwrap();
    assert!(!results.is_empty());

    // Update first file
    std::fs::write(&file1, "pub fn updated_function() {}").unwrap();
    indexer.index_file(&file1).await.unwrap();

    // Search should find updated content
    let query = SearchQuery::new("updated_function");
    let results = indexer.search(query).await.unwrap();
    assert!(!results.is_empty());
}

#[tokio::test]
async fn test_multi_language_indexing() {
    let temp_dir = TempDir::new().unwrap();
    let index_path = temp_dir.path().to_path_buf();

    let config = agcodex_core::semantic_index::SemanticIndexConfig {
        enable_persistence: false,
        storage_path: Some(index_path.clone()),
        ..SemanticIndexConfig::default()
    };

    let indexer = SemanticIndexer::new(config).unwrap();

    // Rust file
    let rust_file = index_path.join("code.rs");
    std::fs::write(
        &rust_file,
        "pub fn rust_function() -> String { String::new() }",
    )
    .unwrap();

    // Python file
    let python_file = index_path.join("code.py");
    std::fs::write(&python_file, "def python_function(): return 'hello'").unwrap();

    // JavaScript file
    let js_file = index_path.join("code.js");
    std::fs::write(&js_file, "function jsFunction() { return 'world'; }").unwrap();

    // Index all files
    let options = IndexingOptions::default();
    indexer
        .index_directory(&index_path, &options)
        .await
        .unwrap();

    // Check that all files are indexed
    let query = SearchQuery::new("function");
    let results = indexer.search(query).await.unwrap();
    assert!(!results.is_empty());

    // Search across languages
    let query = SearchQuery::new("function");
    let results = indexer.search(query).await.unwrap();
    assert_eq!(results.len(), 3);
}

#[tokio::test]
async fn test_relevance_ranking() {
    let temp_dir = TempDir::new().unwrap();
    let index_path = temp_dir.path().to_path_buf();

    let config = agcodex_core::semantic_index::SemanticIndexConfig {
        enable_persistence: false,
        storage_path: Some(index_path.clone()),
        ..SemanticIndexConfig::default()
    };

    let indexer = SemanticIndexer::new(config).unwrap();

    // Create files with varying relevance
    let file1 = index_path.join("exact.rs");
    std::fs::write(&file1, "pub fn database_connection() -> Connection {}").unwrap();

    let file2 = index_path.join("related.rs");
    std::fs::write(&file2, "pub fn db_connect() -> DbConn {}").unwrap();

    let file3 = index_path.join("distant.rs");
    std::fs::write(&file3, "pub fn unrelated_function() -> String {}").unwrap();

    let options = IndexingOptions::default();
    indexer
        .index_directory(&index_path, &options)
        .await
        .unwrap();

    // Search with relevance ranking
    let query = SearchQuery::new("database connection");

    let results = indexer.search(query).await.unwrap();

    assert!(results.len() >= 2);
    assert!(results[0].relevance_score.final_score > results[1].relevance_score.final_score);
    assert!(results[0].content.contains("database_connection"));
}

#[tokio::test]
async fn test_context_window_assembly() {
    let temp_dir = TempDir::new().unwrap();
    let index_path = temp_dir.path().to_path_buf();

    let config = SemanticIndexConfig {
        max_chunk_size: 100,
        storage_path: Some(index_path.clone()),
        ..SemanticIndexConfig::default()
    };

    let indexer = SemanticIndexer::new(config).unwrap();

    let file = index_path.join("large.rs");
    let large_content = (0..50)
        .map(|i| format!("pub fn function_{}() {{\n    // Implementation\n}}\n", i))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&file, large_content).unwrap();

    indexer.index_file(&file).await.unwrap();

    // Search with context window limit
    let query = SearchQuery::new("function")
        .with_context_window(500)
        .with_limit(10);

    let results = indexer.search(query).await.unwrap();

    let total_size: usize = results.iter().map(|r| r.content.len()).sum();
    assert!(total_size <= 600); // Some buffer for metadata
}

#[tokio::test]
async fn test_search_performance() {
    let temp_dir = TempDir::new().unwrap();
    let index_path = temp_dir.path().to_path_buf();

    let config = agcodex_core::semantic_index::SemanticIndexConfig {
        enable_persistence: false,
        storage_path: Some(index_path.clone()),
        ..SemanticIndexConfig::default()
    };

    let indexer = SemanticIndexer::new(config).unwrap();

    // Create 100 files
    for i in 0..100 {
        let file = index_path.join(format!("file_{}.rs", i));
        let content = format!("pub fn function_{}() {{ /* code */ }}", i);
        std::fs::write(&file, content).unwrap();
    }

    let options = IndexingOptions::default();
    indexer
        .index_directory(&index_path, &options)
        .await
        .unwrap();

    // Measure search performance
    let start = std::time::Instant::now();
    let query = SearchQuery::new("function");
    let results = indexer.search(query).await.unwrap();
    let duration = start.elapsed();

    assert!(!results.is_empty());
    assert!(duration.as_millis() < 100); // Sub-100ms requirement
}

#[tokio::test]
async fn test_concurrent_search() {
    let temp_dir = TempDir::new().unwrap();
    let index_path = temp_dir.path().to_path_buf();

    let config = SemanticIndexConfig {
        storage_path: Some(index_path.clone()),
        ..SemanticIndexConfig::default()
    };

    let indexer = SemanticIndexer::new(config).unwrap();

    // Create temporary file and index it
    let test_file = index_path.join("test_content.rs");
    std::fs::write(&test_file, "pub fn test() {}").unwrap();
    indexer.index_file(&test_file).await.unwrap();

    let indexer = std::sync::Arc::new(tokio::sync::Mutex::new(indexer));

    // Concurrent searches
    let mut handles = vec![];
    for i in 0..10 {
        let indexer = indexer.clone();
        let handle = tokio::spawn(async move {
            let query = SearchQuery::new(format!("test{}", i % 2));
            let indexer = indexer.lock().await;
            indexer.search(query).await
        });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}
