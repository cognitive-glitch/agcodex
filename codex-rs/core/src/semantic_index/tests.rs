//! Comprehensive Integration Tests for Semantic Indexing System
//!
//! These tests verify the complete semantic indexing pipeline including:
//! - AST compaction integration
//! - Embedding generation and caching
//! - Vector similarity search
//! - Storage backends (memory and persistent)
//! - Real-time retrieval with sub-100ms latency
//! - RAG context assembly
//!
//! # Test Architecture
//!
//! ```text
//! Integration Tests
//! ├── Unit Tests (per module)
//! ├── Integration Tests (cross-module)
//! ├── Performance Tests (latency, throughput)
//! ├── Stress Tests (large datasets, concurrent access)
//! └── End-to-End Tests (full pipeline)
//! ```

use crate::ast_compactor::Language;
use crate::semantic_index::*;
use std::time::Duration;
use std::time::Instant;
use tempfile::TempDir;
use tempfile::tempdir;
use tokio::fs;

/// Helper struct for test setup
struct TestSetup {
    temp_dir: TempDir,
    indexer: SemanticIndexer,
    config: SemanticIndexConfig,
}

impl TestSetup {
    async fn new() -> Result<Self> {
        let temp_dir = tempdir().unwrap();
        let config = SemanticIndexConfig {
            max_documents: 1000,
            cache_size: 100,
            embedding_dimensions: 128, // Smaller for tests
            max_chunk_size: 512,
            chunk_overlap: 64,
            max_concurrent_indexing: 2,
            max_concurrent_queries: 4,
            enable_persistence: false,
            storage_path: Some(temp_dir.path().to_path_buf()),
            similarity_threshold: 0.6, // Lower for tests
            max_results: 20,
        };

        let indexer = SemanticIndexer::new(config.clone())?;

        Ok(Self {
            temp_dir,
            indexer,
            config,
        })
    }

    async fn create_test_files(&self) -> Result<Vec<std::path::PathBuf>> {
        let mut files = Vec::new();

        // Create Rust test file
        let rust_file = self.temp_dir.path().join("test.rs");
        let rust_content = r#"
//! Test Rust module for semantic indexing

use std::collections::HashMap;

/// User data structure
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: Option<String>,
}

impl User {
    /// Create a new user
    pub fn new(id: u64, name: String) -> Self {
        Self {
            id,
            name,
            email: None,
        }
    }
    
    /// Set user email
    pub fn set_email(&mut self, email: String) {
        self.email = Some(email);
    }
    
    /// Get user display name
    pub fn display_name(&self) -> &str {
        &self.name
    }
}

/// User management service
pub struct UserService {
    users: HashMap<u64, User>,
}

impl UserService {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
        }
    }
    
    pub fn add_user(&mut self, user: User) -> Result<(), String> {
        if self.users.contains_key(&user.id) {
            return Err("User already exists".to_string());
        }
        self.users.insert(user.id, user);
        Ok(())
    }
    
    pub fn get_user(&self, id: u64) -> Option<&User> {
        self.users.get(&id)
    }
    
    pub fn remove_user(&mut self, id: u64) -> Option<User> {
        self.users.remove(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_user_creation() {
        let user = User::new(1, "Alice".to_string());
        assert_eq!(user.id, 1);
        assert_eq!(user.name, "Alice");
        assert!(user.email.is_none());
    }
    
    #[test]
    fn test_user_service() {
        let mut service = UserService::new();
        let user = User::new(1, "Bob".to_string());
        
        assert!(service.add_user(user).is_ok());
        assert!(service.get_user(1).is_some());
        assert_eq!(service.get_user(1).unwrap().name, "Bob");
    }
}
"#;
        fs::write(&rust_file, rust_content).await.unwrap();
        files.push(rust_file);

        // Create TypeScript test file
        let ts_file = self.temp_dir.path().join("test.ts");
        let ts_content = r#"
/**
 * User management TypeScript module
 */

interface User {
    id: number;
    name: string;
    email?: string;
}

interface UserRepository {
    findById(id: number): Promise<User | null>;
    save(user: User): Promise<void>;
    delete(id: number): Promise<boolean>;
}

class InMemoryUserRepository implements UserRepository {
    private users = new Map<number, User>();

    async findById(id: number): Promise<User | null> {
        return this.users.get(id) || null;
    }

    async save(user: User): Promise<void> {
        this.users.set(user.id, user);
    }

    async delete(id: number): Promise<boolean> {
        return this.users.delete(id);
    }

    async findByName(name: string): Promise<User[]> {
        const results: User[] = [];
        for (const user of this.users.values()) {
            if (user.name.includes(name)) {
                results.push(user);
            }
        }
        return results;
    }
}

class UserService {
    constructor(private repository: UserRepository) {}

    async createUser(name: string, email?: string): Promise<User> {
        const user: User = {
            id: Date.now(),
            name,
            email,
        };
        
        await this.repository.save(user);
        return user;
    }

    async getUser(id: number): Promise<User | null> {
        return this.repository.findById(id);
    }

    async updateUserEmail(id: number, email: string): Promise<boolean> {
        const user = await this.repository.findById(id);
        if (!user) return false;
        
        user.email = email;
        await this.repository.save(user);
        return true;
    }

    async deleteUser(id: number): Promise<boolean> {
        return this.repository.delete(id);
    }
}

export { User, UserRepository, InMemoryUserRepository, UserService };
"#;
        fs::write(&ts_file, ts_content).await.unwrap();
        files.push(ts_file);

        // Create Python test file
        let py_file = self.temp_dir.path().join("test.py");
        let py_content = r#"
"""
User management Python module with comprehensive functionality
"""

from typing import Optional, List, Dict
from dataclasses import dataclass
from abc import ABC, abstractmethod
import asyncio


@dataclass
class User:
    """User data class with validation"""
    id: int
    name: str
    email: Optional[str] = None

    def __post_init__(self):
        if not self.name:
            raise ValueError("User name cannot be empty")
        if self.email and "@" not in self.email:
            raise ValueError("Invalid email format")

    @property
    def display_name(self) -> str:
        """Get formatted display name"""
        return f"{self.name} <{self.email}>" if self.email else self.name

    def update_email(self, email: str) -> None:
        """Update user email with validation"""
        if "@" not in email:
            raise ValueError("Invalid email format")
        self.email = email


class UserRepository(ABC):
    """Abstract user repository interface"""
    
    @abstractmethod
    async def find_by_id(self, user_id: int) -> Optional[User]:
        pass
    
    @abstractmethod
    async def save(self, user: User) -> None:
        pass
    
    @abstractmethod
    async def delete(self, user_id: int) -> bool:
        pass
    
    @abstractmethod
    async def find_by_name(self, name: str) -> List[User]:
        pass


class InMemoryUserRepository(UserRepository):
    """In-memory implementation of user repository"""
    
    def __init__(self):
        self._users: Dict[int, User] = {}
    
    async def find_by_id(self, user_id: int) -> Optional[User]:
        return self._users.get(user_id)
    
    async def save(self, user: User) -> None:
        self._users[user.id] = user
    
    async def delete(self, user_id: int) -> bool:
        return self._users.pop(user_id, None) is not None
    
    async def find_by_name(self, name: str) -> List[User]:
        return [user for user in self._users.values() 
                if name.lower() in user.name.lower()]
    
    async def count(self) -> int:
        """Get total user count"""
        return len(self._users)
    
    async def find_all(self) -> List[User]:
        """Get all users"""
        return list(self._users.values())


class UserService:
    """User management service with business logic"""
    
    def __init__(self, repository: UserRepository):
        self.repository = repository
    
    async def create_user(self, name: str, email: Optional[str] = None) -> User:
        """Create a new user with validation"""
        user_id = len(await self.repository.find_all()) + 1
        user = User(id=user_id, name=name, email=email)
        await self.repository.save(user)
        return user
    
    async def get_user(self, user_id: int) -> Optional[User]:
        """Retrieve user by ID"""
        return await self.repository.find_by_id(user_id)
    
    async def update_user_email(self, user_id: int, email: str) -> bool:
        """Update user email"""
        user = await self.repository.find_by_id(user_id)
        if not user:
            return False
        
        try:
            user.update_email(email)
            await self.repository.save(user)
            return True
        except ValueError:
            return False
    
    async def delete_user(self, user_id: int) -> bool:
        """Delete user by ID"""
        return await self.repository.delete(user_id)
    
    async def search_users(self, query: str) -> List[User]:
        """Search users by name"""
        return await self.repository.find_by_name(query)


# Example usage and tests
async def main():
    """Example usage of the user management system"""
    repository = InMemoryUserRepository()
    service = UserService(repository)
    
    # Create users
    alice = await service.create_user("Alice Johnson", "alice@example.com")
    bob = await service.create_user("Bob Smith", "bob@example.com")
    charlie = await service.create_user("Charlie Brown")
    
    print(f"Created users: {alice}, {bob}, {charlie}")
    
    # Search and retrieve
    users = await service.search_users("alice")
    print(f"Search results for 'alice': {users}")
    
    # Update email
    success = await service.update_user_email(charlie.id, "charlie@example.com")
    print(f"Email update successful: {success}")
    
    # Delete user
    deleted = await service.delete_user(bob.id)
    print(f"User deleted: {deleted}")


if __name__ == "__main__":
    asyncio.run(main())
"#;
        fs::write(&py_file, py_content).await.unwrap();
        files.push(py_file);

        Ok(files)
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_indexing_pipeline() {
        let setup = TestSetup::new().await.unwrap();
        let files = setup.create_test_files().await.unwrap();

        // Index all test files
        let mut document_ids = Vec::new();
        for file in &files {
            let doc_id = setup.indexer.index_file(file).await.unwrap();
            document_ids.push(doc_id);
        }

        assert_eq!(document_ids.len(), 3);

        // Verify indexing metrics
        let metrics = setup.indexer.get_metrics();
        assert_eq!(metrics.documents_indexed, 3);
        assert!(metrics.chunks_created > 0);
        assert!(metrics.embeddings_generated > 0);
        assert!(metrics.avg_indexing_time_ms > 0.0);

        // Verify documents are retrievable
        for doc_id in &document_ids {
            let doc_info = setup.indexer.get_document_info(*doc_id);
            assert!(doc_info.is_some());
            assert!(!doc_info.unwrap().chunk_ids.is_empty());
        }
    }

    #[tokio::test]
    async fn test_semantic_search_functionality() {
        let setup = TestSetup::new().await.unwrap();
        let files = setup.create_test_files().await.unwrap();

        // Index files
        for file in &files {
            setup.indexer.index_file(file).await.unwrap();
        }

        // Test various search queries
        let test_queries = vec![
            ("user management", 3),    // Should match all files
            ("struct User", 1),        // Should match Rust file primarily
            ("class UserService", 1),  // Should match TypeScript file
            ("async def", 1),          // Should match Python file
            ("email validation", 2),   // Should match Python and possibly others
            ("repository pattern", 2), // Should match TypeScript and Python
        ];

        for (query_text, expected_min_results) in test_queries {
            let query = SearchQuery::new(query_text)
                .with_limit(10)
                .with_threshold(0.5)
                .with_scores();

            let results = setup.indexer.search(query).await.unwrap();

            assert!(
                results.len() >= expected_min_results,
                "Query '{}' returned {} results, expected at least {}",
                query_text,
                results.len(),
                expected_min_results
            );

            // Verify result quality
            for result in &results {
                assert!(!result.content.is_empty());
                assert!(result.similarity_score >= 0.5);
                assert!(result.relevance_score.final_score >= 0.5);
            }

            // Check results are properly ranked
            let scores: Vec<f32> = results.iter().map(|r| r.ranking_score()).collect();

            assert!(
                scores.windows(2).all(|w| w[0] >= w[1]),
                "Results not properly ranked for query '{}'",
                query_text
            );
        }
    }

    #[tokio::test]
    async fn test_language_specific_filtering() {
        let setup = TestSetup::new().await.unwrap();
        let files = setup.create_test_files().await.unwrap();

        // Index files
        for file in &files {
            setup.indexer.index_file(file).await.unwrap();
        }

        // Test language-specific queries
        let rust_query = SearchQuery::new("struct User")
            .with_languages(vec![Language::Rust])
            .with_threshold(0.4);

        let rust_results = setup.indexer.search(rust_query).await.unwrap();
        assert!(!rust_results.is_empty());

        // All results should be from Rust files
        for result in &rust_results {
            assert_eq!(result.language, Language::Rust);
        }

        // Test TypeScript specific query
        let ts_query = SearchQuery::new("interface User")
            .with_languages(vec![Language::TypeScript])
            .with_threshold(0.4);

        let ts_results = setup.indexer.search(ts_query).await.unwrap();
        assert!(!ts_results.is_empty());

        for result in &ts_results {
            assert_eq!(result.language, Language::TypeScript);
        }
    }

    #[tokio::test]
    async fn test_incremental_indexing() {
        let setup = TestSetup::new().await.unwrap();

        // Index initial file
        let rust_file = setup.temp_dir.path().join("incremental.rs");
        let initial_content = "fn hello() { println!(\"Hello\"); }";
        fs::write(&rust_file, initial_content).await.unwrap();

        let doc_id = setup.indexer.index_file(&rust_file).await.unwrap();

        let initial_metrics = setup.indexer.get_metrics();
        assert_eq!(initial_metrics.documents_indexed, 1);

        // Modify file and reindex
        let updated_content = r#"
fn hello() { 
    println!("Hello, World!"); 
}

fn goodbye() {
    println!("Goodbye, World!");
}
"#;
        fs::write(&rust_file, updated_content).await.unwrap();

        // Remove old document and reindex
        let removed = setup.indexer.remove_document(doc_id).await.unwrap();
        assert!(removed);

        let new_doc_id = setup.indexer.index_file(&rust_file).await.unwrap();
        assert_ne!(doc_id, new_doc_id); // Should be different document

        // Search for new function
        let query = SearchQuery::new("goodbye function").with_threshold(0.4);
        let results = setup.indexer.search(query).await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_directory_indexing() {
        let setup = TestSetup::new().await.unwrap();

        // Create subdirectory structure
        let src_dir = setup.temp_dir.path().join("src");
        let lib_dir = src_dir.join("lib");
        fs::create_dir_all(&lib_dir).await.unwrap();

        // Create multiple files
        let files = vec![
            (
                src_dir.join("main.rs"),
                "fn main() { println!(\"Hello\"); }",
            ),
            (
                src_dir.join("utils.rs"),
                "pub fn add(a: i32, b: i32) -> i32 { a + b }",
            ),
            (
                lib_dir.join("math.rs"),
                "pub fn multiply(x: f64, y: f64) -> f64 { x * y }",
            ),
        ];

        for (path, content) in &files {
            fs::write(path, content).await.unwrap();
        }

        // Index entire directory
        let options = IndexingOptions {
            include_patterns: vec!["**/*.rs".to_string()],
            parallel_workers: 2,
            ..Default::default()
        };

        let doc_ids = setup
            .indexer
            .index_directory(&src_dir, &options)
            .await
            .unwrap();
        assert_eq!(doc_ids.len(), 3);

        // Verify all files were indexed
        let metrics = setup.indexer.get_metrics();
        assert_eq!(metrics.documents_indexed, 3);

        // Test cross-file search
        let query = SearchQuery::new("function").with_threshold(0.4);
        let results = setup.indexer.search(query).await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_performance_requirements() {
        let setup = TestSetup::new().await.unwrap();
        let files = setup.create_test_files().await.unwrap();

        // Index files and measure time
        let start = Instant::now();
        for file in &files {
            setup.indexer.index_file(file).await.unwrap();
        }
        let indexing_time = start.elapsed();

        // Indexing should be reasonably fast
        assert!(
            indexing_time < Duration::from_secs(10),
            "Indexing took too long: {:?}",
            indexing_time
        );

        // Test query performance
        let query = SearchQuery::new("user management").with_limit(5);

        let start = Instant::now();
        let results = setup.indexer.search(query).await.unwrap();
        let query_time = start.elapsed();

        assert!(!results.is_empty());
        assert!(
            query_time < Duration::from_millis(100),
            "Query took too long: {:?}",
            query_time
        );

        // Test concurrent queries
        let concurrent_queries = 5;
        let mut handles = Vec::new();

        let start = Instant::now();
        for i in 0..concurrent_queries {
            let indexer = setup.indexer.clone();
            let query_text = format!("test query {}", i);

            let handle = tokio::spawn(async move {
                let query = SearchQuery::new(query_text).with_threshold(0.3);
                indexer.search(query).await
            });

            handles.push(handle);
        }

        // Wait for all queries to complete
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }

        let concurrent_time = start.elapsed();
        assert!(
            concurrent_time < Duration::from_millis(500),
            "Concurrent queries took too long: {:?}",
            concurrent_time
        );
    }

    #[tokio::test]
    async fn test_rag_context_assembly() {
        let setup = TestSetup::new().await.unwrap();
        let files = setup.create_test_files().await.unwrap();

        // Index files
        for file in &files {
            setup.indexer.index_file(file).await.unwrap();
        }

        // Test context window assembly
        let query = SearchQuery::new("user creation")
            .with_context_window(1000)
            .with_limit(3)
            .with_scores();

        let results = setup.indexer.search(query).await.unwrap();
        assert!(!results.is_empty());

        // Verify context is included
        for result in &results {
            assert!(result.context.is_some());
            let context = result.context.as_ref().unwrap();
            assert!(!context.is_empty());
            assert!(context.len() <= 1000);
        }

        // Test that context provides useful information
        let combined_context: String = results
            .iter()
            .filter_map(|r| r.context.as_ref())
            .cloned()
            .collect::<Vec<_>>()
            .join("\n\n");

        assert!(combined_context.contains("user"));
        assert!(combined_context.len() > 100); // Should have substantial content
    }

    #[tokio::test]
    async fn test_error_handling_and_recovery() {
        let setup = TestSetup::new().await.unwrap();

        // Test invalid file indexing
        let nonexistent_file = setup.temp_dir.path().join("nonexistent.rs");
        let result = setup.indexer.index_file(&nonexistent_file).await;
        assert!(result.is_err());

        // Test empty query
        let empty_query = SearchQuery::new("");
        let result = setup.indexer.search(empty_query).await;
        assert!(result.is_err());

        // Test query with invalid threshold
        let mut invalid_query = SearchQuery::new("test");
        invalid_query.threshold = 2.0; // Invalid threshold
        let result = invalid_query.validate();
        assert!(result.is_ok()); // Should be clamped, not error

        // Test very large file handling
        let large_file = setup.temp_dir.path().join("large.rs");
        let large_content = "fn test() { println!(\"test\"); }\n".repeat(10000);
        fs::write(&large_file, large_content).await.unwrap();

        let result = setup.indexer.index_file(&large_file).await;
        // Should either succeed or fail gracefully
        if result.is_err() {
            // Error should be informative
            let error = result.unwrap_err();
            assert!(!error.to_string().is_empty());
        }
    }

    #[tokio::test]
    async fn test_storage_backend_functionality() {
        let setup = TestSetup::new().await.unwrap();

        // Create test embedding
        let chunk_id = ChunkId::new_v4();
        let embedding = crate::semantic_index::embeddings::EmbeddingVector::new(
            vec![1.0, 2.0, 3.0, 4.0],
            "test_hash".to_string(),
        );

        // Test storage operations through indexer
        let test_file = setup.temp_dir.path().join("storage_test.rs");
        let content = "fn test_storage() { /* test content */ }";
        fs::write(&test_file, content).await.unwrap();

        let doc_id = setup.indexer.index_file(&test_file).await.unwrap();

        // Verify document was stored
        let doc_info = setup.indexer.get_document_info(doc_id);
        assert!(doc_info.is_some());
        assert!(!doc_info.unwrap().chunk_ids.is_empty());

        // Test search retrieval
        let query = SearchQuery::new("test storage").with_threshold(0.3);
        let results = setup.indexer.search(query).await.unwrap();
        assert!(!results.is_empty());

        // Test document removal
        let removed = setup.indexer.remove_document(doc_id).await.unwrap();
        assert!(removed);

        // Verify document is gone
        let doc_info_after = setup.indexer.get_document_info(doc_id);
        assert!(doc_info_after.is_none());
    }

    #[tokio::test]
    async fn test_caching_and_optimization() {
        let setup = TestSetup::new().await.unwrap();

        // Create file for testing
        let test_file = setup.temp_dir.path().join("cache_test.rs");
        let content = "fn cache_test() { println!(\"Testing cache\"); }";
        fs::write(&test_file, content).await.unwrap();

        // Index file
        setup.indexer.index_file(&test_file).await.unwrap();

        // Perform same search multiple times to test caching
        let query = SearchQuery::new("cache test").with_threshold(0.4);

        let mut query_times = Vec::new();
        for _ in 0..5 {
            let start = Instant::now();
            let results = setup.indexer.search(query.clone()).await.unwrap();
            let elapsed = start.elapsed();

            assert!(!results.is_empty());
            query_times.push(elapsed);
        }

        // Later queries should generally be faster (due to caching)
        let avg_early = query_times[0..2].iter().sum::<Duration>() / 2;
        let avg_later = query_times[3..5].iter().sum::<Duration>() / 2;

        // Allow some variance, but later queries should not be significantly slower
        assert!(
            avg_later <= avg_early * 2,
            "Caching doesn't appear to be working effectively"
        );

        // Check metrics
        let metrics = setup.indexer.get_metrics();
        assert!(metrics.avg_retrieval_time_ms > 0.0);
        assert!(metrics.efficiency_score() > 0.5);
    }

    #[tokio::test]
    async fn test_index_maintenance_and_cleanup() {
        let setup = TestSetup::new().await.unwrap();

        // Create and index multiple files
        for i in 0..5 {
            let test_file = setup.temp_dir.path().join(format!("test_{}.rs", i));
            let content = format!("fn test_{}() {{ println!(\"Test {}\"); }}", i, i);
            fs::write(&test_file, content).await.unwrap();
            setup.indexer.index_file(&test_file).await.unwrap();
        }

        // Verify all files are indexed
        let initial_metrics = setup.indexer.get_metrics();
        assert_eq!(initial_metrics.documents_indexed, 5);

        // List documents by language
        let rust_docs = setup.indexer.list_documents(Some(Language::Rust));
        assert_eq!(rust_docs.len(), 5);

        let all_docs = setup.indexer.list_documents(None);
        assert_eq!(all_docs.len(), 5);

        // Clear entire index
        setup.indexer.clear_index().await.unwrap();

        // Verify index is cleared
        let final_metrics = setup.indexer.get_metrics();
        assert_eq!(final_metrics.documents_indexed, 0);
        assert_eq!(final_metrics.chunks_created, 0);

        let docs_after_clear = setup.indexer.list_documents(None);
        assert!(docs_after_clear.is_empty());
    }
}

#[cfg(test)]
mod benchmark_tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Run with --ignored for benchmarks
    async fn benchmark_large_codebase_indexing() {
        let setup = TestSetup::new().await.unwrap();

        // Create larger test dataset
        let num_files = 100;
        let mut files = Vec::new();

        for i in 0..num_files {
            let file_path = setup.temp_dir.path().join(format!("bench_{}.rs", i));
            let content = format!(
                r#"
//! Module {} for benchmarking

use std::collections::HashMap;

pub struct BenchStruct{} {{
    pub id: u64,
    pub name: String,
    data: HashMap<String, i32>,
}}

impl BenchStruct{} {{
    pub fn new(id: u64, name: String) -> Self {{
        Self {{
            id,
            name,
            data: HashMap::new(),
        }}
    }}
    
    pub fn process_data(&mut self, key: String, value: i32) {{
        self.data.insert(key, value);
    }}
    
    pub fn get_data(&self, key: &str) -> Option<&i32> {{
        self.data.get(key)
    }}
}}

#[cfg(test)]
mod tests {{
    use super::*;
    
    #[test]
    fn test_bench_struct_{}() {{
        let mut s = BenchStruct{}::new({}, "test".to_string());
        s.process_data("key".to_string(), 42);
        assert_eq!(s.get_data("key"), Some(&42));
    }}
}}
"#,
                i, i, i, i, i, i
            );

            fs::write(&file_path, content).await.unwrap();
            files.push(file_path);
        }

        // Benchmark indexing
        let start = Instant::now();

        let options = IndexingOptions {
            parallel_workers: 4,
            ..Default::default()
        };

        let doc_ids = setup
            .indexer
            .index_directory(setup.temp_dir.path(), &options)
            .await
            .unwrap();
        let indexing_time = start.elapsed();

        assert_eq!(doc_ids.len(), num_files);

        println!("Indexed {} files in {:?}", num_files, indexing_time);
        println!(
            "Average time per file: {:?}",
            indexing_time / num_files as u32
        );

        // Benchmark search performance
        let search_queries = vec![
            "BenchStruct",
            "process_data",
            "HashMap",
            "test function",
            "get_data method",
        ];

        for query_text in search_queries {
            let start = Instant::now();
            let query = SearchQuery::new(query_text).with_limit(20);
            let results = setup.indexer.search(query).await.unwrap();
            let search_time = start.elapsed();

            println!(
                "Query '{}': {} results in {:?}",
                query_text,
                results.len(),
                search_time
            );

            assert!(
                search_time < Duration::from_millis(100),
                "Search for '{}' took {:?}, exceeding 100ms target",
                query_text,
                search_time
            );
        }

        // Final metrics
        let metrics = setup.indexer.get_metrics();
        println!("Final metrics: {:#?}", metrics);

        assert!(
            metrics.efficiency_score() > 0.7,
            "Efficiency score too low: {}",
            metrics.efficiency_score()
        );
    }
}
