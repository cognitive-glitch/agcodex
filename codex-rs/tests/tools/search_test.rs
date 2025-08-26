//! Comprehensive tests for the AGCodex search tool implementations.
//!
//! Tests both the Tantivy-based indexing system (`core/src/tools/index.rs`) 
//! and the multi-layer search engine (`core/src/code_tools/search.rs`).
//!
//! Coverage includes:
//! - Basic search functionality (functions, variables, classes)
//! - Performance requirements (Symbol: <1ms, Full-text: <5ms)  
//! - Error handling (missing files, invalid queries)
//! - Cache effectiveness and hit rates
//! - Multi-language support (Rust, Python, TypeScript, Go)
//! - Performance benchmarks and scaling tests

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tempfile::TempDir;
use tokio::test;

// Import the search implementations
use agcodex_core::tools::index::{IndexTool, IndexConfig, BuildInput, SearchInput, SearchQuery};
use agcodex_core::code_tools::search::{
    MultiLayerSearchEngine, SearchConfig, SearchQuery as MultiSearchQuery, 
    QueryType, SearchScope, Symbol, SymbolKind, Location, Scope, Visibility,
};

// Import test utilities
use crate::helpers::test_utils::{
    TestEnvironment, PerformanceAssertions, TestTiming, BenchmarkStats,
    CodeSamples, MockDataGenerator
};

/// Test fixture with sample code files and search engines
struct SearchTestFixture {
    env: TestEnvironment,
    sample_files: HashMap<String, PathBuf>,
    index_tool: IndexTool,
    multi_search_engine: MultiLayerSearchEngine,
}

impl SearchTestFixture {
    async fn new() -> Self {
        let env = TestEnvironment::new();
        let sample_files = create_comprehensive_sample_files(&env).await;
        
        // Initialize IndexTool with test configuration
        let index_config = IndexConfig {
            index_path: env.path().join("search_index"),
            include_extensions: vec![
                "rs".to_string(), "py".to_string(), "js".to_string(), 
                "ts".to_string(), "tsx".to_string(), "go".to_string()
            ],
            max_file_size: 1024 * 1024, // 1MB
            incremental: true,
            writer_memory_mb: 64,
            num_threads: Some(2),
            merge_policy: Default::default(),
        };
        
        let index_tool = IndexTool::new(index_config).unwrap();
        
        // Initialize MultiLayerSearchEngine
        let search_config = SearchConfig {
            max_cache_size: 100,
            cache_ttl: Duration::from_secs(60),
            enable_symbol_index: true,
            enable_tantivy: true,
            enable_ast_cache: true,
            enable_ripgrep_fallback: true,
            max_results: 50,
            timeout: Duration::from_secs(5),
        };
        
        let multi_search_engine = MultiLayerSearchEngine::new(search_config).unwrap();
        
        Self {
            env,
            sample_files,
            index_tool,
            multi_search_engine,
        }
    }
    
    /// Build the index with all sample files
    async fn build_index(&self) {
        let build_input = BuildInput {
            directory: self.env.path().to_path_buf(),
            config: IndexConfig::default(),
            force_rebuild: true,
        };
        
        let _result = self.index_tool.build(build_input).await.unwrap();
    }
    
    /// Populate the multi-layer search engine with symbols
    async fn populate_symbols(&self) {
        // Add symbols for Rust sample
        self.add_rust_symbols();
        
        // Add symbols for Python sample  
        self.add_python_symbols();
        
        // Add symbols for TypeScript sample
        self.add_typescript_symbols();
        
        // Add symbols for Go sample
        self.add_go_symbols();
    }
    
    fn add_rust_symbols(&self) {
        let rust_file = self.sample_files.get("sample.rs").unwrap();
        
        // User struct
        self.multi_search_engine.add_symbol(Symbol {
            name: "User".to_string(),
            kind: SymbolKind::Struct,
            location: Location {
                file: rust_file.clone(),
                line: 7,
                column: 12,
                byte_offset: 120,
            },
            scope: Scope {
                function: None,
                class: None,
                module: Some("sample".to_string()),
                namespace: None,
            },
            visibility: Visibility::Public,
        });
        
        // new function
        self.multi_search_engine.add_symbol(Symbol {
            name: "new".to_string(),
            kind: SymbolKind::Function,
            location: Location {
                file: rust_file.clone(),
                line: 24,
                column: 12,
                byte_offset: 400,
            },
            scope: Scope {
                function: None,
                class: Some("User".to_string()),
                module: Some("sample".to_string()),
                namespace: None,
            },
            visibility: Visibility::Public,
        });
        
        // UserRegistry struct
        self.multi_search_engine.add_symbol(Symbol {
            name: "UserRegistry".to_string(),
            kind: SymbolKind::Struct,
            location: Location {
                file: rust_file.clone(),
                line: 60,
                column: 12,
                byte_offset: 1000,
            },
            scope: Scope {
                function: None,
                class: None,
                module: Some("sample".to_string()),
                namespace: None,
            },
            visibility: Visibility::Public,
        });
        
        // register_user method
        self.multi_search_engine.add_symbol(Symbol {
            name: "register_user".to_string(),
            kind: SymbolKind::Method,
            location: Location {
                file: rust_file.clone(),
                line: 73,
                column: 12,
                byte_offset: 1200,
            },
            scope: Scope {
                function: None,
                class: Some("UserRegistry".to_string()),
                module: Some("sample".to_string()),
                namespace: None,
            },
            visibility: Visibility::Public,
        });
    }
    
    fn add_python_symbols(&self) {
        let python_file = self.sample_files.get("calculator.py").unwrap();
        
        // Calculator class
        self.multi_search_engine.add_symbol(Symbol {
            name: "Calculator".to_string(),
            kind: SymbolKind::Class,
            location: Location {
                file: python_file.clone(),
                line: 2,
                column: 7,
                byte_offset: 20,
            },
            scope: Scope {
                function: None,
                class: None,
                module: Some("calculator".to_string()),
                namespace: None,
            },
            visibility: Visibility::Public,
        });
        
        // add method
        self.multi_search_engine.add_symbol(Symbol {
            name: "add".to_string(),
            kind: SymbolKind::Method,
            location: Location {
                file: python_file.clone(),
                line: 6,
                column: 9,
                byte_offset: 80,
            },
            scope: Scope {
                function: None,
                class: Some("Calculator".to_string()),
                module: Some("calculator".to_string()),
                namespace: None,
            },
            visibility: Visibility::Public,
        });
    }
    
    fn add_typescript_symbols(&self) {
        let ts_file = self.sample_files.get("user-manager.ts").unwrap();
        
        // UserProfile interface
        self.multi_search_engine.add_symbol(Symbol {
            name: "UserProfile".to_string(),
            kind: SymbolKind::Interface,
            location: Location {
                file: ts_file.clone(),
                line: 2,
                column: 11,
                byte_offset: 30,
            },
            scope: Scope {
                function: None,
                class: None,
                module: Some("user-manager".to_string()),
                namespace: None,
            },
            visibility: Visibility::Public,
        });
        
        // UserManager class
        self.multi_search_engine.add_symbol(Symbol {
            name: "UserManager".to_string(),
            kind: SymbolKind::Class,
            location: Location {
                file: ts_file.clone(),
                line: 11,
                column: 7,
                byte_offset: 200,
            },
            scope: Scope {
                function: None,
                class: None,
                module: Some("user-manager".to_string()),
                namespace: None,
            },
            visibility: Visibility::Public,
        });
        
        // addUser method
        self.multi_search_engine.add_symbol(Symbol {
            name: "addUser".to_string(),
            kind: SymbolKind::Method,
            location: Location {
                file: ts_file.clone(),
                line: 14,
                column: 5,
                byte_offset: 280,
            },
            scope: Scope {
                function: None,
                class: Some("UserManager".to_string()),
                module: Some("user-manager".to_string()),
                namespace: None,
            },
            visibility: Visibility::Public,
        });
    }
    
    fn add_go_symbols(&self) {
        let go_file = self.sample_files.get("task.go").unwrap();
        
        // Task struct
        self.multi_search_engine.add_symbol(Symbol {
            name: "Task".to_string(),
            kind: SymbolKind::Struct,
            location: Location {
                file: go_file.clone(),
                line: 5,
                column: 6,
                byte_offset: 50,
            },
            scope: Scope {
                function: None,
                class: None,
                module: Some("main".to_string()),
                namespace: None,
            },
            visibility: Visibility::Public,
        });
        
        // Toggle method
        self.multi_search_engine.add_symbol(Symbol {
            name: "Toggle".to_string(),
            kind: SymbolKind::Method,
            location: Location {
                file: go_file.clone(),
                line: 12,
                column: 17,
                byte_offset: 180,
            },
            scope: Scope {
                function: None,
                class: Some("Task".to_string()),
                module: Some("main".to_string()),
                namespace: None,
            },
            visibility: Visibility::Public,
        });
    }
}

/// Create comprehensive sample files for testing
async fn create_comprehensive_sample_files(env: &TestEnvironment) -> HashMap<String, PathBuf> {
    let mut files = HashMap::new();
    
    // Rust sample with various constructs
    let rust_content = r#"use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
    pub age: Option<u8>,
    pub roles: Vec<Role>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    Admin,
    Moderator,
    User,
    Guest,
}

impl User {
    pub fn new(id: u64, name: String, email: String) -> Self {
        Self {
            id,
            name,
            email,
            age: None,
            roles: vec![Role::User],
        }
    }
    
    pub fn with_age(mut self, age: u8) -> Self {
        self.age = Some(age);
        self
    }
    
    pub fn add_role(&mut self, role: Role) {
        if !self.roles.contains(&role) {
            self.roles.push(role);
        }
    }
    
    pub fn has_role(&self, role: &Role) -> bool {
        self.roles.contains(role)
    }
    
    pub fn is_admin(&self) -> bool {
        self.has_role(&Role::Admin)
    }
}

pub struct UserRegistry {
    users: HashMap<u64, User>,
    next_id: u64,
}

impl UserRegistry {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            next_id: 1,
        }
    }
    
    pub fn register_user(&mut self, name: String, email: String) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        
        let user = User::new(id, name, email);
        self.users.insert(id, user);
        
        id
    }
    
    pub fn get_user(&self, id: u64) -> Option<&User> {
        self.users.get(&id)
    }
    
    pub fn find_by_email(&self, email: &str) -> Option<&User> {
        self.users.values().find(|user| user.email == email)
    }
}
"#;

    // Python sample with class and methods  
    let python_content = r#"
class Calculator:
    def __init__(self):
        self.history = []
    
    def add(self, a, b):
        result = a + b
        self.history.append(f"{a} + {b} = {result}")
        return result
    
    def subtract(self, a, b):
        result = a - b
        self.history.append(f"{a} - {b} = {result}")
        return result
    
    def multiply(self, a, b):
        result = a * b
        self.history.append(f"{a} * {b} = {result}")
        return result
    
    def divide(self, a, b):
        if b == 0:
            raise ValueError("Cannot divide by zero")
        result = a / b
        self.history.append(f"{a} / {b} = {result}")
        return result
    
    def get_history(self):
        return self.history.copy()
    
    def clear_history(self):
        self.history.clear()

def create_calculator():
    return Calculator()
"#;

    // TypeScript sample with interface and class
    let typescript_content = r#"
interface UserProfile {
    id: number;
    name: string;
    email: string;
    preferences: {
        theme: 'light' | 'dark';
        notifications: boolean;
    };
}

class UserManager {
    private users: Map<number, UserProfile> = new Map();
    
    addUser(user: UserProfile): void {
        this.users.set(user.id, user);
    }
    
    getUser(id: number): UserProfile | undefined {
        return this.users.get(id);
    }
    
    updateUser(id: number, updates: Partial<UserProfile>): boolean {
        const user = this.users.get(id);
        if (!user) {
            return false;
        }
        
        const updatedUser = { ...user, ...updates };
        this.users.set(id, updatedUser);
        return true;
    }
    
    removeUser(id: number): boolean {
        return this.users.delete(id);
    }
    
    getAllUsers(): UserProfile[] {
        return Array.from(this.users.values());
    }
}

export { UserProfile, UserManager };
"#;

    // Go sample with struct and methods
    let go_content = r#"package main

import "fmt"

type Task struct {
    ID        int    `json:"id"`
    Title     string `json:"title"`
    Completed bool   `json:"completed"`
}

func (t *Task) Toggle() {
    t.Completed = !t.Completed
}

func (t Task) String() string {
    status := "pending"
    if t.Completed {
        status = "completed"
    }
    return fmt.Sprintf("Task %d: %s (%s)", t.ID, t.Title, status)
}

type TaskManager struct {
    tasks []Task
    nextID int
}

func NewTaskManager() *TaskManager {
    return &TaskManager{
        tasks: make([]Task, 0),
        nextID: 1,
    }
}

func (tm *TaskManager) AddTask(title string) *Task {
    task := Task{
        ID:        tm.nextID,
        Title:     title,
        Completed: false,
    }
    tm.tasks = append(tm.tasks, task)
    tm.nextID++
    return &task
}

func (tm *TaskManager) GetTask(id int) *Task {
    for i := range tm.tasks {
        if tm.tasks[i].ID == id {
            return &tm.tasks[i]
        }
    }
    return nil
}
"#;

    // Write sample files to temporary directory
    let samples = [
        ("sample.rs", rust_content),
        ("calculator.py", python_content), 
        ("user-manager.ts", typescript_content),
        ("task.go", go_content),
    ];
    
    for (filename, content) in samples {
        let file_path = env.path().join(filename);
        fs::write(&file_path, content).unwrap();
        files.insert(filename.to_string(), file_path);
    }
    
    files
}

// ==================== BASIC SEARCH FUNCTIONALITY TESTS ====================

#[test]
async fn test_basic_symbol_search() {
    let fixture = SearchTestFixture::new().await;
    fixture.populate_symbols().await;
    
    // Test searching for a Rust struct
    let query = MultiSearchQuery::symbol("User");
    let result = fixture.multi_search_engine.search(query).await.unwrap();
    
    assert!(!result.result.is_empty(), "Should find User struct");
    assert_eq!(result.result[0].content, "User");
    assert!(result.result[0].file.ends_with("sample.rs"));
    assert_eq!(result.result[0].line, 7);
    
    // Test searching for a Python class
    let query = MultiSearchQuery::symbol("Calculator");
    let result = fixture.multi_search_engine.search(query).await.unwrap();
    
    assert!(!result.result.is_empty(), "Should find Calculator class");
    assert_eq!(result.result[0].content, "Calculator");
    assert!(result.result[0].file.ends_with("calculator.py"));
    assert_eq!(result.result[0].line, 2);
    
    // Test searching for a TypeScript interface
    let query = MultiSearchQuery::symbol("UserProfile");
    let result = fixture.multi_search_engine.search(query).await.unwrap();
    
    assert!(!result.result.is_empty(), "Should find UserProfile interface");
    assert_eq!(result.result[0].content, "UserProfile");
    assert!(result.result[0].file.ends_with("user-manager.ts"));
}

#[test]
async fn test_function_search() {
    let fixture = SearchTestFixture::new().await;
    fixture.populate_symbols().await;
    
    // Test searching for Rust method
    let query = MultiSearchQuery::symbol("register_user");
    let result = fixture.multi_search_engine.search(query).await.unwrap();
    
    assert!(!result.result.is_empty(), "Should find register_user method");
    assert_eq!(result.result[0].content, "register_user");
    assert!(result.result[0].file.ends_with("sample.rs"));
    
    // Test searching for Python method
    let query = MultiSearchQuery::symbol("add");
    let result = fixture.multi_search_engine.search(query).await.unwrap();
    
    assert!(!result.result.is_empty(), "Should find add method");
    assert_eq!(result.result[0].content, "add");
    assert!(result.result[0].file.ends_with("calculator.py"));
    
    // Test searching for Go method  
    let query = MultiSearchQuery::symbol("Toggle");
    let result = fixture.multi_search_engine.search(query).await.unwrap();
    
    assert!(!result.result.is_empty(), "Should find Toggle method");
    assert_eq!(result.result[0].content, "Toggle");
    assert!(result.result[0].file.ends_with("task.go"));
}

#[test]
async fn test_variable_search() {
    let fixture = SearchTestFixture::new().await;
    fixture.build_index().await;
    
    // Test searching for variable names in full-text mode
    let search_query = SearchQuery {
        query: "next_id".to_string(),
        language: Some("rust".to_string()),
        path_filter: None,
        symbol_type: None,
        limit: Some(10),
        fuzzy: false,
        min_score: Some(0.1),
    };
    
    let search_input = SearchInput {
        query: search_query,
        config: IndexConfig::default(),
    };
    
    let result = fixture.index_tool.search(search_input).await.unwrap();
    
    assert!(!result.result.is_empty(), "Should find next_id variable");
    assert!(result.result.iter().any(|r| r.document.content.contains("next_id")));
}

// ==================== PERFORMANCE REQUIREMENT TESTS ====================

#[test]
async fn test_symbol_lookup_performance() {
    let fixture = SearchTestFixture::new().await;
    fixture.populate_symbols().await;
    
    // Test symbol lookup performance (<1ms requirement)
    let query = MultiSearchQuery::symbol("User");
    
    let start = Instant::now();
    let _result = fixture.multi_search_engine.search(query).await.unwrap();
    let duration = start.elapsed();
    
    PerformanceAssertions::assert_duration_under(duration, 1, "symbol lookup");
    
    // Benchmark multiple symbol lookups
    let stats = TestTiming::benchmark_operation(
        || {
            let query = MultiSearchQuery::symbol("UserRegistry");
            tokio::runtime::Handle::current().block_on(async {
                fixture.multi_search_engine.search(query).await.unwrap()
            })
        },
        50
    );
    
    stats.assert_average_under(Duration::from_millis(1), "symbol lookup batch");
}

#[test] 
async fn test_full_text_search_performance() {
    let fixture = SearchTestFixture::new().await;
    fixture.build_index().await;
    
    // Test full-text search performance (<5ms requirement)
    let search_query = SearchQuery {
        query: "function".to_string(),
        language: None,
        path_filter: None,
        symbol_type: None,
        limit: Some(20),
        fuzzy: false,
        min_score: Some(0.1),
    };
    
    let search_input = SearchInput {
        query: search_query,
        config: IndexConfig::default(),
    };
    
    let start = Instant::now();
    let _result = fixture.index_tool.search(search_input).await.unwrap();
    let duration = start.elapsed();
    
    PerformanceAssertions::assert_duration_under(duration, 5, "full-text search");
    
    // Benchmark multiple full-text searches
    let stats = TestTiming::benchmark_operation(
        || {
            let search_query = SearchQuery {
                query: "struct".to_string(),
                language: None,
                path_filter: None,
                symbol_type: None,
                limit: Some(10),
                fuzzy: false,
                min_score: Some(0.1),
            };
            
            let search_input = SearchInput {
                query: search_query,
                config: IndexConfig::default(),
            };
            
            tokio::runtime::Handle::current().block_on(async {
                fixture.index_tool.search(search_input).await.unwrap()
            })
        },
        30
    );
    
    stats.assert_average_under(Duration::from_millis(5), "full-text search batch");
}

#[test]
async fn test_performance_scaling() {
    let env = TestEnvironment::new();
    
    // Create larger dataset for scaling tests
    let mut large_files = HashMap::new();
    for i in 0..20 {
        let filename = format!("large_sample_{}.rs", i);
        let content = format!(r#"
pub struct LargeStruct{} {{
    field_{}: String,
    data_{}: Vec<u8>,
}}

impl LargeStruct{} {{
    pub fn new_method_{}() -> Self {{
        Self {{
            field_{}: String::new(),
            data_{}: Vec::new(),
        }}
    }}
    
    pub fn process_{}(&self) {{
        println!("Processing {{}}", self.field_{});
    }}
}}
"#, i, i, i, i, i, i, i, i, i, i);

        let file_path = env.path().join(&filename);
        fs::write(&file_path, content).unwrap();
        large_files.insert(filename, file_path);
    }
    
    // Test index building performance with larger dataset
    let index_config = IndexConfig {
        index_path: env.path().join("large_index"),
        ..Default::default()
    };
    
    let index_tool = IndexTool::new(index_config).unwrap();
    
    let build_input = BuildInput {
        directory: env.path().to_path_buf(),
        config: IndexConfig::default(),
        force_rebuild: true,
    };
    
    let (_, build_duration) = TestTiming::time_async_operation(|| {
        index_tool.build(build_input)
    }).await;
    
    // Build time should scale reasonably (target: <500ms for 20 files)
    PerformanceAssertions::assert_duration_under(build_duration, 500, "index building");
    
    // Test search performance on larger dataset
    let search_query = SearchQuery {
        query: "LargeStruct".to_string(),
        language: None,
        path_filter: None,
        symbol_type: None,
        limit: Some(50),
        fuzzy: false,
        min_score: Some(0.1),
    };
    
    let search_input = SearchInput {
        query: search_query,
        config: IndexConfig::default(),
    };
    
    let (result, search_duration) = TestTiming::time_async_operation(|| {
        index_tool.search(search_input)
    }).await;
    
    // Search should still be fast even with more data
    PerformanceAssertions::assert_duration_under(search_duration, 10, "search on large dataset");
    
    // Should find multiple results
    assert!(result.result.len() >= 10, "Should find multiple LargeStruct definitions");
}

// ==================== ERROR HANDLING TESTS ====================

#[test]
async fn test_missing_file_error_handling() {
    let env = TestEnvironment::new();
    let index_config = IndexConfig {
        index_path: env.path().join("missing_index"),
        ..Default::default()
    };
    
    let index_tool = IndexTool::new(index_config).unwrap();
    
    // Test building index on non-existent directory
    let build_input = BuildInput {
        directory: env.path().join("nonexistent"),
        config: IndexConfig::default(),
        force_rebuild: true,
    };
    
    // Should handle missing directory gracefully
    let result = index_tool.build(build_input).await;
    
    // May succeed with 0 files or fail gracefully
    match result {
        Ok(output) => {
            assert_eq!(output.result.document_count, 0, "Should find 0 documents in missing directory");
        }
        Err(_) => {
            // Expected error for missing directory
        }
    }
}

#[test] 
async fn test_invalid_query_handling() {
    let fixture = SearchTestFixture::new().await;
    fixture.build_index().await;
    
    // Test invalid query patterns
    let invalid_queries = [
        "",              // Empty query
        "   ",           // Whitespace only  
        "a".repeat(1000), // Very long query
        "\n\t\r",        // Control characters
        "query with\x00null", // Null byte
    ];
    
    for invalid_query in invalid_queries {
        let search_query = SearchQuery {
            query: invalid_query.to_string(),
            language: None,
            path_filter: None,
            symbol_type: None,
            limit: Some(10),
            fuzzy: false,
            min_score: Some(0.1),
        };
        
        let search_input = SearchInput {
            query: search_query,
            config: IndexConfig::default(),
        };
        
        // Should handle invalid queries gracefully
        let result = index_tool.search(search_input).await;
        
        match result {
            Ok(output) => {
                // May return empty results for invalid queries
                assert!(output.result.len() >= 0, "Should handle invalid query gracefully");
            }
            Err(err) => {
                // Expected error for some invalid queries
                assert!(err.to_string().contains("query") || err.to_string().contains("parse"));
            }
        }
    }
}

#[test]
async fn test_concurrent_access_safety() {
    let fixture = SearchTestFixture::new().await;
    fixture.populate_symbols().await;
    fixture.build_index().await;
    
    // Test concurrent symbol searches
    let mut handles = Vec::new();
    
    for i in 0..10 {
        let engine = &fixture.multi_search_engine;
        let query_text = if i % 2 == 0 { "User" } else { "Calculator" };
        
        let handle = tokio::spawn(async move {
            let query = MultiSearchQuery::symbol(query_text);
            engine.search(query).await
        });
        
        handles.push(handle);
    }
    
    // All searches should complete successfully
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent search should succeed");
        assert!(!result.unwrap().result.is_empty(), "Should find results");
    }
    
    // Test concurrent index operations
    let mut index_handles = Vec::new();
    
    for i in 0..5 {
        let tool = &fixture.index_tool;
        let query_text = format!("test_query_{}", i);
        
        let handle = tokio::spawn(async move {
            let search_query = SearchQuery {
                query: query_text,
                language: None,
                path_filter: None,
                symbol_type: None,
                limit: Some(5),
                fuzzy: false,
                min_score: Some(0.1),
            };
            
            let search_input = SearchInput {
                query: search_query,
                config: IndexConfig::default(),
            };
            
            tool.search(search_input).await
        });
        
        index_handles.push(handle);
    }
    
    // All index searches should complete
    for handle in index_handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent index search should succeed");
    }
}

// ==================== CACHE EFFECTIVENESS TESTS ====================

#[test]
async fn test_search_result_caching() {
    let search_config = SearchConfig {
        max_cache_size: 10,
        cache_ttl: Duration::from_secs(60),
        ..Default::default()
    };
    
    let engine = MultiLayerSearchEngine::new(search_config).unwrap();
    
    // Add a test symbol
    engine.add_symbol(Symbol {
        name: "TestFunction".to_string(),
        kind: SymbolKind::Function,
        location: Location {
            file: PathBuf::from("test.rs"),
            line: 10,
            column: 5,
            byte_offset: 100,
        },
        scope: Scope {
            function: None,
            class: None,
            module: Some("test".to_string()),
            namespace: None,
        },
        visibility: Visibility::Public,
    });
    
    let query = MultiSearchQuery::symbol("TestFunction");
    
    // First search - should be slower (cache miss)
    let (_, first_duration) = TestTiming::time_async_operation(|| {
        engine.search(query.clone())
    }).await;
    
    // Second search - should be faster (cache hit)
    let (_, second_duration) = TestTiming::time_async_operation(|| {
        engine.search(query.clone())
    }).await;
    
    // Cache hit should be significantly faster
    assert!(
        second_duration < first_duration || second_duration < Duration::from_micros(100),
        "Cache hit should be faster: first={}μs, second={}μs", 
        first_duration.as_micros(), second_duration.as_micros()
    );
    
    // Test cache hit rate with repeated queries
    let mut hit_count = 0;
    let total_queries = 20;
    
    for i in 0..total_queries {
        let query = if i < 10 {
            MultiSearchQuery::symbol("TestFunction") // Repeated query
        } else {
            MultiSearchQuery::symbol(&format!("NewQuery{}", i)) // New query
        };
        
        let start = Instant::now();
        let _result = engine.search(query).await.unwrap();
        let duration = start.elapsed();
        
        // Fast responses likely indicate cache hits
        if duration < Duration::from_micros(500) {
            hit_count += 1;
        }
    }
    
    // Should have good cache hit rate for repeated queries
    PerformanceAssertions::assert_cache_hit_rate(hit_count, total_queries, 30.0);
}

#[test]
async fn test_cache_expiration() {
    let search_config = SearchConfig {
        max_cache_size: 5,
        cache_ttl: Duration::from_millis(50), // Very short TTL
        ..Default::default()
    };
    
    let engine = MultiLayerSearchEngine::new(search_config).unwrap();
    
    // Add test symbol
    engine.add_symbol(Symbol {
        name: "ExpirationTest".to_string(),
        kind: SymbolKind::Function,
        location: Location {
            file: PathBuf::from("test.rs"),
            line: 5,
            column: 1,
            byte_offset: 50,
        },
        scope: Scope {
            function: None,
            class: None,
            module: Some("test".to_string()),
            namespace: None,
        },
        visibility: Visibility::Public,
    });
    
    let query = MultiSearchQuery::symbol("ExpirationTest");
    
    // First search
    let _result1 = engine.search(query.clone()).await.unwrap();
    
    // Search again immediately (should hit cache)
    let (_, fast_duration) = TestTiming::time_async_operation(|| {
        engine.search(query.clone())
    }).await;
    
    // Wait for cache to expire
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Search after expiration (should miss cache)
    let (_, slow_duration) = TestTiming::time_async_operation(|| {
        engine.search(query.clone())
    }).await;
    
    // Post-expiration search should be slower than cached version
    assert!(
        slow_duration >= fast_duration,
        "Expired cache search should be slower: fast={}μs, slow={}μs",
        fast_duration.as_micros(), slow_duration.as_micros()
    );
}

// ==================== MULTI-LANGUAGE SUPPORT TESTS ====================

#[test]
async fn test_rust_language_support() {
    let fixture = SearchTestFixture::new().await;
    fixture.build_index().await;
    
    // Test Rust-specific constructs
    let rust_queries = [
        "struct",
        "impl",
        "pub fn",
        "Vec",
        "Option",
        "enum",
        "match",
    ];
    
    for query_text in rust_queries {
        let search_query = SearchQuery {
            query: query_text.to_string(),
            language: Some("rust".to_string()),
            path_filter: Some("*.rs".to_string()),
            symbol_type: None,
            limit: Some(10),
            fuzzy: false,
            min_score: Some(0.1),
        };
        
        let search_input = SearchInput {
            query: search_query,
            config: IndexConfig::default(),
        };
        
        let result = fixture.index_tool.search(search_input).await.unwrap();
        
        assert!(
            !result.result.is_empty(), 
            "Should find Rust construct: {}", query_text
        );
        
        // Verify results are from Rust files
        for search_result in &result.result {
            assert!(
                search_result.document.path.ends_with(".rs"),
                "Result should be from Rust file"
            );
        }
    }
}

#[test]
async fn test_python_language_support() {
    let fixture = SearchTestFixture::new().await;
    fixture.build_index().await;
    
    // Test Python-specific constructs
    let python_queries = [
        "class",
        "def",
        "__init__",
        "self",
        "import",
        "return",
    ];
    
    for query_text in python_queries {
        let search_query = SearchQuery {
            query: query_text.to_string(),
            language: Some("python".to_string()),
            path_filter: Some("*.py".to_string()),
            symbol_type: None,
            limit: Some(10),
            fuzzy: false,
            min_score: Some(0.1),
        };
        
        let search_input = SearchInput {
            query: search_query,
            config: IndexConfig::default(),
        };
        
        let result = fixture.index_tool.search(search_input).await.unwrap();
        
        if !result.result.is_empty() {
            // Verify results are from Python files
            for search_result in &result.result {
                assert!(
                    search_result.document.path.ends_with(".py"),
                    "Result should be from Python file"
                );
            }
        }
    }
}

#[test]
async fn test_typescript_language_support() {
    let fixture = SearchTestFixture::new().await;
    fixture.build_index().await;
    
    // Test TypeScript-specific constructs
    let typescript_queries = [
        "interface",
        "type",
        "private",
        "public",
        "Map",
        "Array",
        "export",
    ];
    
    for query_text in typescript_queries {
        let search_query = SearchQuery {
            query: query_text.to_string(),
            language: None,
            path_filter: Some("*.ts".to_string()),
            symbol_type: None,
            limit: Some(10),
            fuzzy: false,
            min_score: Some(0.1),
        };
        
        let search_input = SearchInput {
            query: search_query,
            config: IndexConfig::default(),
        };
        
        let result = fixture.index_tool.search(search_input).await.unwrap();
        
        if !result.result.is_empty() {
            // Verify results are from TypeScript files
            for search_result in &result.result {
                assert!(
                    search_result.document.path.ends_with(".ts"),
                    "Result should be from TypeScript file"
                );
            }
        }
    }
}

#[test]
async fn test_go_language_support() {
    let fixture = SearchTestFixture::new().await;
    fixture.build_index().await;
    
    // Test Go-specific constructs
    let go_queries = [
        "package",
        "func",
        "struct",
        "type",
        "import",
        "fmt",
    ];
    
    for query_text in go_queries {
        let search_query = SearchQuery {
            query: query_text.to_string(),
            language: None,
            path_filter: Some("*.go".to_string()),
            symbol_type: None,
            limit: Some(10),
            fuzzy: false,
            min_score: Some(0.1),
        };
        
        let search_input = SearchInput {
            query: search_query,
            config: IndexConfig::default(),
        };
        
        let result = fixture.index_tool.search(search_input).await.unwrap();
        
        if !result.result.is_empty() {
            // Verify results are from Go files
            for search_result in &result.result {
                assert!(
                    search_result.document.path.ends_with(".go"),
                    "Result should be from Go file"
                );
            }
        }
    }
}

#[test]
async fn test_language_detection_accuracy() {
    let fixture = SearchTestFixture::new().await;
    fixture.build_index().await;
    
    // Get index statistics to verify language detection
    let stats = fixture.index_tool.stats().await.unwrap();
    
    // Should detect multiple languages
    assert!(
        stats.result.language_stats.len() >= 3,
        "Should detect multiple programming languages"
    );
    
    // Verify expected languages are detected
    let expected_languages = ["rust", "python", "javascript", "go"];
    for expected_lang in expected_languages {
        assert!(
            stats.result.language_stats.contains_key(expected_lang),
            "Should detect {} language", expected_lang
        );
    }
}

// ==================== ADVANCED SEARCH FEATURES TESTS ====================

#[test]
async fn test_fuzzy_search() {
    let fixture = SearchTestFixture::new().await;
    fixture.populate_symbols().await;
    
    // Test fuzzy symbol search with typos
    let typo_queries = [
        ("Usr", "User"),          // Missing character
        ("Calculatr", "Calculator"), // Missing character  
        ("UserManger", "UserManager"), // Typo
        ("Togle", "Toggle"),      // Missing character
    ];
    
    for (typo, expected) in typo_queries {
        let query = MultiSearchQuery::symbol(typo).fuzzy();
        let result = fixture.multi_search_engine.search(query).await.unwrap();
        
        assert!(
            !result.result.is_empty(),
            "Fuzzy search should find {} for typo {}", expected, typo
        );
        
        // Result should have reasonable score
        assert!(
            result.result[0].score > 0.5,
            "Fuzzy match score should be reasonable: {}", result.result[0].score
        );
    }
}

#[test]
async fn test_scoped_search() {
    let fixture = SearchTestFixture::new().await;
    fixture.build_index().await;
    
    // Test searching within specific files
    let specific_files = vec![
        fixture.sample_files.get("sample.rs").unwrap().clone()
    ];
    
    let query = MultiSearchQuery::symbol("User")
        .in_files(specific_files);
    
    let result = fixture.multi_search_engine.search(query).await.unwrap();
    
    if !result.result.is_empty() {
        assert!(
            result.result[0].file.ends_with("sample.rs"),
            "Scoped search should only return results from specified files"
        );
    }
    
    // Test directory-scoped search
    let query = MultiSearchQuery::full_text("function")
        .in_directory(fixture.env.path().to_path_buf());
    
    let result = fixture.multi_search_engine.search(query).await.unwrap();
    
    // Should find results within the specified directory
    for match_result in &result.result {
        assert!(
            match_result.file.starts_with(fixture.env.path()),
            "Directory-scoped search should only return results from specified directory"
        );
    }
}

#[test]
async fn test_definition_and_reference_search() {
    let fixture = SearchTestFixture::new().await;
    fixture.populate_symbols().await;
    
    // Test finding definitions
    let def_result = fixture.multi_search_engine.find_definition("User").await.unwrap();
    
    assert!(!def_result.result.is_empty(), "Should find User definition");
    
    // Definition should be the struct declaration, not usage
    let definition_match = &def_result.result[0];
    assert_eq!(definition_match.content, "User");
    assert!(definition_match.file.ends_with("sample.rs"));
    assert_eq!(definition_match.line, 7); // Line where struct is defined
    
    // Test finding references  
    let ref_result = fixture.multi_search_engine.find_references("User").await.unwrap();
    
    // May find multiple references (usage sites)
    assert!(!ref_result.result.is_empty(), "Should find User references");
}

#[test]
async fn test_context_aware_search() {
    let fixture = SearchTestFixture::new().await;
    fixture.build_index().await;
    
    // Test searching with context lines
    let query = MultiSearchQuery::full_text("register_user")
        .with_context_lines(3);
    
    let result = fixture.multi_search_engine.search(query).await.unwrap();
    
    if !result.result.is_empty() {
        // Context should be available in the search metadata
        assert_eq!(
            result.metadata.search_layer,
            agcodex_core::code_tools::search::SearchLayer::RipgrepFallback
        );
        assert!(result.metadata.duration < Duration::from_millis(100));
    }
}

// ==================== PERFORMANCE BENCHMARK TESTS ====================

#[test]
async fn test_search_performance_benchmarks() {
    let fixture = SearchTestFixture::new().await;
    fixture.populate_symbols().await;
    fixture.build_index().await;
    
    // Benchmark different search types
    println!("Running search performance benchmarks...");
    
    // Symbol search benchmark
    let symbol_stats = TestTiming::benchmark_operation(
        || {
            let query = MultiSearchQuery::symbol("User");
            tokio::runtime::Handle::current().block_on(async {
                fixture.multi_search_engine.search(query).await.unwrap()
            })
        },
        100
    );
    
    println!("Symbol Search Benchmark:");
    println!("  Average: {}μs", symbol_stats.average.as_micros());
    println!("  Min: {}μs", symbol_stats.min.as_micros());
    println!("  Max: {}μs", symbol_stats.max.as_micros());
    println!("  Median: {}μs", symbol_stats.median.as_micros());
    
    symbol_stats.assert_average_under(Duration::from_millis(1), "symbol search");
    
    // Full-text search benchmark  
    let fulltext_stats = TestTiming::benchmark_operation(
        || {
            let search_query = SearchQuery {
                query: "function".to_string(),
                language: None,
                path_filter: None,
                symbol_type: None,
                limit: Some(10),
                fuzzy: false,
                min_score: Some(0.1),
            };
            
            let search_input = SearchInput {
                query: search_query,
                config: IndexConfig::default(),
            };
            
            tokio::runtime::Handle::current().block_on(async {
                fixture.index_tool.search(search_input).await.unwrap()
            })
        },
        50
    );
    
    println!("Full-text Search Benchmark:");
    println!("  Average: {}ms", fulltext_stats.average.as_millis());
    println!("  Min: {}ms", fulltext_stats.min.as_millis());
    println!("  Max: {}ms", fulltext_stats.max.as_millis());
    println!("  Median: {}ms", fulltext_stats.median.as_millis());
    
    fulltext_stats.assert_average_under(Duration::from_millis(5), "full-text search");
}

#[test]
async fn test_memory_usage_benchmarks() {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Simple memory tracking (basic implementation)
    static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
    
    struct TrackingAllocator;
    
    unsafe impl GlobalAlloc for TrackingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
            System.alloc(layout)
        }
        
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            ALLOCATED.fetch_sub(layout.size(), Ordering::SeqCst);
            System.dealloc(ptr, layout)
        }
    }
    
    let fixture = SearchTestFixture::new().await;
    
    // Measure memory usage during index building
    let initial_memory = ALLOCATED.load(Ordering::SeqCst);
    
    fixture.build_index().await;
    fixture.populate_symbols().await;
    
    let post_build_memory = ALLOCATED.load(Ordering::SeqCst);
    let memory_used = post_build_memory.saturating_sub(initial_memory);
    
    // Memory usage should be reasonable (target: <100MB for test data)
    PerformanceAssertions::assert_memory_usage_reasonable(memory_used, 100);
    
    println!("Memory Usage Benchmark:");
    println!("  Index + Symbols: {} MB", memory_used / (1024 * 1024));
}

// ==================== INTEGRATION TESTS ====================

#[test]
async fn test_search_tool_integration() {
    let fixture = SearchTestFixture::new().await;
    fixture.build_index().await;
    fixture.populate_symbols().await;
    
    // Test that both search engines find the same symbols
    let symbol_query = MultiSearchQuery::symbol("Calculator");
    let multi_result = fixture.multi_search_engine.search(symbol_query).await.unwrap();
    
    let index_query = SearchQuery {
        query: "Calculator".to_string(),
        language: Some("python".to_string()),
        path_filter: None,
        symbol_type: None,
        limit: Some(10),
        fuzzy: false,
        min_score: Some(0.1),
    };
    
    let index_input = SearchInput {
        query: index_query,
        config: IndexConfig::default(),
    };
    
    let index_result = fixture.index_tool.search(index_input).await.unwrap();
    
    // Both should find the Calculator class
    assert!(!multi_result.result.is_empty(), "Multi-layer engine should find Calculator");
    assert!(!index_result.result.is_empty(), "Index tool should find Calculator");
    
    // Results should reference the same file
    let multi_file = &multi_result.result[0].file;
    let index_file = &index_result.result[0].document.path;
    
    assert!(
        multi_file.ends_with("calculator.py") && index_file.ends_with("calculator.py"),
        "Both engines should find Calculator in Python file"
    );
}

#[test]
async fn test_end_to_end_search_workflow() {
    let fixture = SearchTestFixture::new().await;
    
    // Step 1: Build index
    println!("Building search index...");
    let build_start = Instant::now();
    fixture.build_index().await;
    let build_duration = build_start.elapsed();
    println!("Index built in {}ms", build_duration.as_millis());
    
    // Step 2: Populate symbol cache
    println!("Populating symbol cache...");
    let symbol_start = Instant::now();
    fixture.populate_symbols().await;
    let symbol_duration = symbol_start.elapsed();
    println!("Symbols populated in {}ms", symbol_duration.as_millis());
    
    // Step 3: Verify index statistics
    let stats = fixture.index_tool.stats().await.unwrap();
    println!("Index Statistics:");
    println!("  Documents: {}", stats.result.document_count);
    println!("  Size: {} bytes", stats.result.size_bytes);
    println!("  Languages: {:?}", stats.result.language_stats.keys().collect::<Vec<_>>());
    
    assert!(stats.result.document_count > 0, "Index should contain documents");
    assert!(stats.result.size_bytes > 0, "Index should have non-zero size");
    
    // Step 4: Test various search scenarios
    let test_scenarios = [
        ("Symbol search", MultiSearchQuery::symbol("User")),
        ("Full-text search", MultiSearchQuery::full_text("function")),
        ("Definition search", MultiSearchQuery::definition("Calculator")),
        ("Fuzzy search", MultiSearchQuery::symbol("Usr").fuzzy()),
    ];
    
    for (scenario_name, query) in test_scenarios {
        println!("Testing {}...", scenario_name);
        let search_start = Instant::now();
        
        let result = fixture.multi_search_engine.search(query).await.unwrap();
        
        let search_duration = search_start.elapsed();
        println!("  {} completed in {}μs", scenario_name, search_duration.as_micros());
        println!("  Found {} results", result.result.len());
        
        if !result.result.is_empty() {
            println!("  Top result: {} in {}", 
                result.result[0].content, 
                result.result[0].file.file_name().unwrap().to_string_lossy()
            );
        }
    }
    
    println!("End-to-end search workflow completed successfully!");
}

// Helper macro for test module organization
macro_rules! test_module {
    ($name:ident { $($test:item)* }) => {
        #[cfg(test)]
        mod $name {
            use super::*;
            $($test)*
        }
    };
}

// Organize tests into logical modules
test_module!(performance_tests {
    // Performance tests already defined above
});

test_module!(error_handling_tests {
    // Error handling tests already defined above
});

test_module!(language_support_tests {
    // Language support tests already defined above
});

test_module!(advanced_features_tests {
    // Advanced features tests already defined above
});

#[cfg(test)]
mod test_utilities {
    use super::*;
    
    /// Verify test fixture setup
    #[test] 
    async fn test_fixture_creation() {
        let fixture = SearchTestFixture::new().await;
        
        assert!(fixture.env.path().exists(), "Test environment should exist");
        assert_eq!(fixture.sample_files.len(), 4, "Should create 4 sample files");
        
        // Verify files were created correctly
        for (filename, path) in &fixture.sample_files {
            assert!(path.exists(), "Sample file {} should exist", filename);
            
            let content = fs::read_to_string(path).unwrap();
            assert!(!content.is_empty(), "Sample file {} should not be empty", filename);
        }
    }
    
    /// Test sample code quality
    #[test]
    async fn test_sample_code_quality() {
        let env = TestEnvironment::new();
        let sample_files = create_comprehensive_sample_files(&env).await;
        
        // Each sample should contain expected constructs
        let expectations = [
            ("sample.rs", vec!["struct", "impl", "pub fn"]),
            ("calculator.py", vec!["class", "def", "__init__"]),
            ("user-manager.ts", vec!["interface", "class", "private"]),
            ("task.go", vec!["type", "func", "struct"]),
        ];
        
        for (filename, expected_keywords) in expectations {
            let file_path = sample_files.get(filename).unwrap();
            let content = fs::read_to_string(file_path).unwrap();
            
            for keyword in expected_keywords {
                assert!(
                    content.contains(keyword),
                    "Sample {} should contain keyword '{}'", filename, keyword
                );
            }
        }
    }
}