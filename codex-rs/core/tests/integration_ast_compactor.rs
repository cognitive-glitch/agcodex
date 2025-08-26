use agcodex_core::ast_compactor::AstCompactor;
use agcodex_core::ast_compactor::CompactionOptions;

#[tokio::test]
async fn test_ast_compactor_rust_code() {
    let rust_code = r#"
/// Documentation for MyStruct
#[derive(Debug, Clone)]
pub struct MyStruct {
    pub field1: String,
    field2: i32,
}

impl MyStruct {
    pub fn new(field1: String) -> Self {
        Self {
            field1,
            field2: 0,
        }
    }
    
    fn private_method(&self) -> i32 {
        self.field2 * 2
    }
}

pub trait MyTrait {
    fn required_method(&self) -> String;
    
    fn default_method(&self) {
        println!("Default implementation");
    }
}
"#;

    let mut compactor = AstCompactor::new();
    let options = CompactionOptions::new()
        .preserve_signatures_only(true)
        .preserve_docs(true);

    let result = compactor.compact(rust_code, &options).unwrap();

    assert!(result.compressed_size < rust_code.len());
    assert!(result.compression_ratio > 0.5);
    assert!(result.compacted_code.contains("pub struct MyStruct"));
    assert!(result.compacted_code.contains("pub fn new"));
    assert!(!result.compacted_code.contains("self.field2 * 2"));
}

#[tokio::test]
async fn test_ast_compactor_multi_language() {
    let mut compactor = AstCompactor::new();
    let options = CompactionOptions::default();

    // Test Python
    let python_code = r#"
class MyClass:
    def __init__(self, name):
        self.name = name
    
    def get_name(self):
        return self.name
"#;

    let result = compactor.compact(python_code, &options).unwrap();
    assert!(result.compacted_code.contains("class MyClass"));
    assert!(result.compressed_size < python_code.len());

    // Test JavaScript
    let js_code = r#"
function myFunction(a, b) {
    return a + b;
}

class MyClass {
    constructor(name) {
        this.name = name;
    }
}
"#;

    let result = compactor.compact(js_code, &options).unwrap();
    assert!(result.compacted_code.contains("function myFunction"));
    assert!(result.compacted_code.contains("class MyClass"));
}

#[tokio::test]
async fn test_compaction_levels() {
    let rust_code = r#"
/// Important documentation
pub fn complex_function(x: i32, y: i32) -> Result<i32, String> {
    if x > 0 {
        Ok(x + y)
    } else {
        Err("Invalid input".to_string())
    }
}
"#;

    let mut compactor = AstCompactor::new();

    // Test SignatureOnly - highest compression
    let options = CompactionOptions::new()
        .preserve_signatures_only(true)
        .preserve_docs(false);
    let result = compactor.compact(rust_code, &options).unwrap();
    assert!(result.compression_ratio > 0.7);
    assert!(!result.compacted_code.contains("Invalid input"));

    // Test Balanced - medium compression
    let options = CompactionOptions::new()
        .preserve_signatures_only(false)
        .preserve_docs(true);
    let result = compactor.compact(rust_code, &options).unwrap();
    assert!(result.compression_ratio > 0.3);
    assert!(result.compacted_code.contains("Important documentation"));

    // Test Full - lowest compression
    let options = CompactionOptions::new()
        .preserve_signatures_only(false)
        .preserve_docs(true);
    let result = compactor.compact(rust_code, &options).unwrap();
    assert!(result.compression_ratio < 0.3);
    assert!(result.compacted_code.contains("Invalid input"));
}

#[tokio::test]
async fn test_cache_performance() {
    let mut compactor = AstCompactor::new();
    let code = "pub fn test() -> i32 { 42 }";
    let options = CompactionOptions::default();

    // First call - cache miss
    let start = std::time::Instant::now();
    compactor.compact(code, &options).unwrap();
    let first_duration = start.elapsed();

    // Second call - cache hit
    let start = std::time::Instant::now();
    compactor.compact(code, &options).unwrap();
    let second_duration = start.elapsed();

    // Cache hit should be significantly faster
    assert!(second_duration < first_duration / 2);
}

#[tokio::test]
async fn test_concurrent_compaction() {
    let options = CompactionOptions::new();

    let mut handles = vec![];
    for i in 0..10 {
        let options = options.clone();
        let handle = tokio::spawn(async move {
            // Each task gets its own compactor instance for true concurrency
            let mut compactor = AstCompactor::new();
            let code = format!("pub fn test{}() -> i32 {{ {} }}", i, i);
            compactor
                .compact(&code, &options)
                .map(|r| r.compression_ratio)
        });
        handles.push(handle);
    }

    for handle in handles {
        let compression_ratio = handle.await.unwrap().unwrap();
        assert!(compression_ratio > 0.0);
    }
}
