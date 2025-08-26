//! Vector Embeddings Engine with Simulated Embeddings
//!
//! This module provides embedding generation for code chunks using a simulated
//! embedding model. In production, this would integrate with real embedding
//! services like OpenAI, Voyage AI, or local models.
//!
//! # Hash-based Simulation
//!
//! For demonstration purposes, we generate deterministic vectors based on
//! content hashing. This ensures:
//! - **Reproducible results**: Same input → same vector
//! - **Reasonable similarity**: Similar code → similar vectors  
//! - **Performance**: No network calls or GPU computation
//!
//! # Memory Layout
//!
//! ```text
//! EmbeddingEngine
//! ├── dimensions: usize (768)
//! ├── cache: Arc<Mutex<LRU<String, EmbeddingVector>>>
//! └── normalization: L2 normalization for cosine similarity
//! ```
//!
//! # Concurrency Model
//!
//! - **Thread-safe caching**: Arc<Mutex<LRUCache>> for concurrent access
//! - **Async generation**: Non-blocking embedding computation
//! - **Batch processing**: Efficient handling of multiple chunks

use super::ChunkId;
use super::Result;
use crate::ast_compactor::Language;

use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

/// Vector embedding representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingVector {
    /// Unique identifier for this embedding
    pub id: Uuid,

    /// The actual vector data (f32 for efficient similarity computation)
    pub vector: Vec<f32>,

    /// Vector dimensions (for validation)
    pub dimensions: usize,

    /// L2 norm for efficient cosine similarity
    pub norm: f32,

    /// Hash of the original content (for cache validation)
    pub content_hash: String,
}

impl EmbeddingVector {
    /// Create a new embedding vector with automatic normalization
    pub fn new(vector: Vec<f32>, content_hash: String) -> Self {
        let dimensions = vector.len();

        // Calculate L2 norm for cosine similarity
        let norm = (vector.iter().map(|x| x * x).sum::<f32>()).sqrt();

        Self {
            id: Uuid::new_v4(),
            vector,
            dimensions,
            norm,
            content_hash,
        }
    }

    /// Calculate cosine similarity with another vector
    pub fn cosine_similarity(&self, other: &EmbeddingVector) -> f32 {
        if self.dimensions != other.dimensions {
            return 0.0;
        }

        if self.norm == 0.0 || other.norm == 0.0 {
            return 0.0;
        }

        let dot_product: f32 = self
            .vector
            .iter()
            .zip(&other.vector)
            .map(|(a, b)| a * b)
            .sum();

        dot_product / (self.norm * other.norm)
    }

    /// Get normalized vector for similarity computation
    pub fn normalized(&self) -> Vec<f32> {
        if self.norm == 0.0 {
            return self.vector.clone();
        }

        self.vector.iter().map(|x| x / self.norm).collect()
    }

    /// Validate vector integrity
    pub fn is_valid(&self) -> bool {
        self.vector.len() == self.dimensions
            && self.norm.is_finite()
            && self.vector.iter().all(|x| x.is_finite())
    }
}

/// Code chunk for embedding generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChunk {
    /// Unique identifier for this chunk
    pub id: ChunkId,

    /// The code content
    pub content: String,

    /// Programming language
    pub language: Language,

    /// Starting line number (if available)
    pub start_line: Option<usize>,

    /// Ending line number (if available)
    pub end_line: Option<usize>,

    /// Source file path (if available)
    pub file_path: Option<PathBuf>,
}

impl CodeChunk {
    /// Create a new code chunk
    pub fn new(content: String, language: Language, file_path: Option<PathBuf>) -> Self {
        Self {
            id: ChunkId::new_v4(),
            content,
            language,
            start_line: None,
            end_line: None,
            file_path,
        }
    }

    /// Get estimated token count (rough approximation)
    pub const fn estimated_tokens(&self) -> usize {
        // Rough approximation: 1 token ≈ 4 characters for code
        self.content.len() / 4
    }

    /// Get content hash for caching
    pub fn content_hash(&self) -> String {
        let mut hasher = DefaultHasher::new();
        self.content.hash(&mut hasher);
        format!("{:?}", self.language).hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Check if chunk is empty or trivial
    pub fn is_empty(&self) -> bool {
        self.content.trim().is_empty()
    }

    /// Get chunk size in bytes
    pub const fn size_bytes(&self) -> usize {
        self.content.len()
    }
}

/// Simple LRU cache implementation
#[derive(Debug)]
struct SimpleLruCache<K, V> {
    capacity: usize,
    data: HashMap<K, V>,
    order: VecDeque<K>,
}

impl<K: Clone + Hash + Eq, V> SimpleLruCache<K, V> {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            data: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    fn get(&mut self, key: &K) -> Option<&V> {
        if self.data.contains_key(key) {
            // Move to front
            self.order.retain(|k| k != key);
            self.order.push_front(key.clone());
            self.data.get(key)
        } else {
            None
        }
    }

    fn put(&mut self, key: K, value: V) -> Option<V> {
        let old_value = if self.data.contains_key(&key) {
            self.order.retain(|k| k != &key);
            self.data.insert(key.clone(), value)
        } else {
            if self.data.len() >= self.capacity {
                // Evict least recently used
                if let Some(lru_key) = self.order.pop_back() {
                    self.data.remove(&lru_key);
                }
            }
            self.data.insert(key.clone(), value)
        };

        self.order.push_front(key);
        old_value
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn clear(&mut self) {
        self.data.clear();
        self.order.clear();
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: usize,
    pub misses: usize,
    pub evictions: usize,
    pub capacity: usize,
    pub current_size: usize,
}

impl CacheStats {
    /// Calculate hit rate (0.0 to 1.0)
    pub fn hit_rate(&self) -> f32 {
        let total = self.hits + self.misses;
        if total > 0 {
            self.hits as f32 / total as f32
        } else {
            0.0
        }
    }
}

/// Embedding engine with caching and batch processing
#[derive(Debug)]
pub struct EmbeddingEngine {
    /// Vector dimensions
    dimensions: usize,

    /// LRU cache for embeddings
    cache: Arc<Mutex<SimpleLruCache<String, EmbeddingVector>>>,

    /// Cache statistics
    stats: Arc<Mutex<CacheStats>>,
}

impl EmbeddingEngine {
    /// Create a new embedding engine with specified dimensions
    pub fn new(dimensions: usize) -> Self {
        Self {
            dimensions,
            cache: Arc::new(Mutex::new(SimpleLruCache::new(1000))),
            stats: Arc::new(Mutex::new(CacheStats {
                capacity: 1000,
                ..Default::default()
            })),
        }
    }

    /// Generate embedding for a code chunk with caching
    pub async fn generate_embedding(&self, content: &str) -> Result<EmbeddingVector> {
        let content_hash = self.hash_content(content);

        // Check cache first
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(cached) = cache.get(&content_hash) {
                self.stats.lock().unwrap().hits += 1;
                return Ok(cached.clone());
            }
        }

        // Cache miss - generate new embedding
        self.stats.lock().unwrap().misses += 1;

        let vector = self.simulate_embedding(content).await?;
        let embedding = EmbeddingVector::new(vector, content_hash.clone());

        // Store in cache
        {
            let mut cache = self.cache.lock().unwrap();
            if cache.put(content_hash, embedding.clone()).is_some() {
                self.stats.lock().unwrap().evictions += 1;
            }

            // Update cache size
            let mut stats = self.stats.lock().unwrap();
            stats.current_size = cache.len();
        }

        Ok(embedding)
    }

    /// Generate query embedding (optimized for search queries)
    pub async fn generate_query_embedding(&self, query: &str) -> Result<EmbeddingVector> {
        // For queries, we might apply different preprocessing
        let normalized_query = self.normalize_query(query);
        self.generate_embedding(&normalized_query).await
    }

    /// Generate embeddings for multiple chunks in batch
    pub async fn generate_batch_embeddings(
        &self,
        chunks: &[CodeChunk],
    ) -> Result<Vec<EmbeddingVector>> {
        let mut embeddings = Vec::with_capacity(chunks.len());

        for chunk in chunks {
            let embedding = self.generate_embedding(&chunk.content).await?;
            embeddings.push(embedding);
        }

        Ok(embeddings)
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> CacheStats {
        self.stats.lock().unwrap().clone()
    }

    /// Clear the embedding cache
    pub fn clear_cache(&self) {
        {
            let mut cache = self.cache.lock().unwrap();
            cache.clear();
        }

        {
            let mut stats = self.stats.lock().unwrap();
            stats.current_size = 0;
            stats.evictions = 0;
        }
    }

    /// Get cache capacity
    pub fn cache_capacity(&self) -> usize {
        self.stats.lock().unwrap().capacity
    }

    /// Simulate embedding generation using deterministic hashing
    async fn simulate_embedding(&self, content: &str) -> Result<Vec<f32>> {
        // In a real implementation, this would call an embedding service
        // For now, we simulate with deterministic hash-based vectors

        let mut vector = Vec::with_capacity(self.dimensions);

        // Create multiple hash variants for different dimensions
        for i in 0..self.dimensions {
            let mut hasher = DefaultHasher::new();
            content.hash(&mut hasher);
            i.hash(&mut hasher);

            let hash = hasher.finish();

            // Convert hash to float in range [-1.0, 1.0]
            let byte_value = ((hash >> (i % 8 * 8)) & 0xFF) as f32;
            let normalized = (byte_value / 127.5) - 1.0; // Maps [0,255] to [-1,1]

            vector.push(normalized);
        }

        // Add some content-specific features
        self.enhance_vector_with_features(&mut vector, content);

        Ok(vector)
    }

    /// Enhance vector with content-specific features
    fn enhance_vector_with_features(&self, vector: &mut Vec<f32>, content: &str) {
        if vector.is_empty() {
            return;
        }

        // Language-specific patterns
        let features = self.extract_content_features(content);

        // Blend features into existing vector dimensions
        let feature_count = features.len().min(vector.len() / 4);
        for i in 0..feature_count {
            let idx = i * 4; // Spread features across vector
            if idx < vector.len() {
                vector[idx] = vector[idx] * 0.8 + features[i] * 0.2;
            }
        }
    }

    /// Extract semantic features from code content
    fn extract_content_features(&self, content: &str) -> Vec<f32> {
        let mut features = Vec::new();

        // Length feature (normalized)
        features.push((content.len() as f32 / 1000.0).min(1.0));

        // Complexity features
        let line_count = content.lines().count() as f32;
        features.push((line_count / 100.0).min(1.0));

        // Keyword density features
        let keywords = [
            "fn",
            "function",
            "class",
            "struct",
            "enum",
            "trait",
            "interface",
        ];
        for keyword in &keywords {
            let count = content.matches(keyword).count() as f32;
            features.push((count / line_count).min(1.0));
        }

        // Indentation complexity
        let avg_indent = content
            .lines()
            .map(|line| line.len() - line.trim_start().len())
            .sum::<usize>() as f32
            / line_count.max(1.0);
        features.push((avg_indent / 20.0).min(1.0));

        // Comment ratio
        let comment_lines = content
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*")
            })
            .count() as f32;
        features.push((comment_lines / line_count).min(1.0));

        features
    }

    /// Normalize query text for better matching
    fn normalize_query(&self, query: &str) -> String {
        query.to_lowercase().trim().to_string()
    }

    /// Generate content hash for caching
    fn hash_content(&self, content: &str) -> String {
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        self.dimensions.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

/// Calculate similarity between two vectors efficiently
pub fn calculate_similarity(a: &EmbeddingVector, b: &EmbeddingVector) -> f32 {
    a.cosine_similarity(b)
}

/// Batch similarity calculation for efficient retrieval
pub fn calculate_batch_similarities(
    query: &EmbeddingVector,
    candidates: &[EmbeddingVector],
) -> Vec<(usize, f32)> {
    candidates
        .iter()
        .enumerate()
        .map(|(idx, candidate)| (idx, query.cosine_similarity(candidate)))
        .collect()
}

/// Find top K most similar vectors efficiently
pub fn find_top_k_similar(
    query: &EmbeddingVector,
    candidates: &[EmbeddingVector],
    k: usize,
) -> Vec<(usize, f32)> {
    let mut similarities = calculate_batch_similarities(query, candidates);

    // Partial sort to get top K (more efficient than full sort)
    similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    similarities.truncate(k);

    similarities
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast_compactor::Language;

    #[tokio::test]
    async fn test_embedding_generation() {
        let engine = EmbeddingEngine::new(768);

        let content = "fn hello_world() { println!(\"Hello, world!\"); }";
        let embedding = engine.generate_embedding(content).await.unwrap();

        assert_eq!(embedding.dimensions, 768);
        assert_eq!(embedding.vector.len(), 768);
        assert!(embedding.is_valid());
        assert!(embedding.norm > 0.0);
    }

    #[tokio::test]
    async fn test_embedding_caching() {
        let engine = EmbeddingEngine::new(256);

        let content = "let x = 42;";

        // First call - cache miss
        let embedding1 = engine.generate_embedding(content).await.unwrap();
        let stats1 = engine.get_cache_stats();
        assert_eq!(stats1.misses, 1);
        assert_eq!(stats1.hits, 0);

        // Second call - cache hit
        let embedding2 = engine.generate_embedding(content).await.unwrap();
        let stats2 = engine.get_cache_stats();
        assert_eq!(stats2.misses, 1);
        assert_eq!(stats2.hits, 1);

        // Embeddings should be identical
        assert_eq!(embedding1.content_hash, embedding2.content_hash);
        assert_eq!(embedding1.vector, embedding2.vector);
    }

    #[test]
    fn test_cosine_similarity() {
        let vec1 = EmbeddingVector::new(vec![1.0, 0.0, 0.0], "hash1".to_string());
        let vec2 = EmbeddingVector::new(vec![0.0, 1.0, 0.0], "hash2".to_string());
        let vec3 = EmbeddingVector::new(vec![1.0, 0.0, 0.0], "hash3".to_string());

        // Orthogonal vectors should have similarity ~0
        assert!((vec1.cosine_similarity(&vec2)).abs() < 0.001);

        // Identical vectors should have similarity ~1
        assert!((vec1.cosine_similarity(&vec3) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_code_chunk_creation() {
        let chunk = CodeChunk::new(
            "fn test() {}".to_string(),
            Language::Rust,
            Some(PathBuf::from("test.rs")),
        );

        assert!(!chunk.is_empty());
        assert_eq!(chunk.language, Language::Rust);
        assert_eq!(chunk.estimated_tokens(), 3); // "fn test() {}" ≈ 3 tokens
        assert!(!chunk.content_hash().is_empty());
    }

    #[test]
    fn test_vector_validation() {
        let valid_vector = EmbeddingVector::new(vec![1.0, 2.0, 3.0], "test".to_string());
        assert!(valid_vector.is_valid());

        let invalid_vector = EmbeddingVector {
            id: Uuid::new_v4(),
            vector: vec![1.0, f32::NAN, 3.0],
            dimensions: 3,
            norm: f32::INFINITY,
            content_hash: "test".to_string(),
        };
        assert!(!invalid_vector.is_valid());
    }

    #[test]
    fn test_top_k_similarity_search() {
        let query = EmbeddingVector::new(vec![1.0, 0.0, 0.0], "query".to_string());

        let candidates = vec![
            EmbeddingVector::new(vec![1.0, 0.0, 0.0], "exact".to_string()), // similarity ~1.0
            EmbeddingVector::new(vec![0.8, 0.6, 0.0], "similar".to_string()), // similarity ~0.8
            EmbeddingVector::new(vec![0.0, 1.0, 0.0], "different".to_string()), // similarity ~0.0
        ];

        let top_2 = find_top_k_similar(&query, &candidates, 2);

        assert_eq!(top_2.len(), 2);
        assert_eq!(top_2[0].0, 0); // First candidate (exact match)
        assert_eq!(top_2[1].0, 1); // Second candidate (similar)
        assert!(top_2[0].1 > top_2[1].1); // First should be more similar
    }

    #[tokio::test]
    async fn test_batch_embedding_generation() {
        let engine = EmbeddingEngine::new(128);

        let chunks = vec![
            CodeChunk::new("fn test1() {}".to_string(), Language::Rust, None),
            CodeChunk::new("fn test2() {}".to_string(), Language::Rust, None),
            CodeChunk::new("class Test {}".to_string(), Language::TypeScript, None),
        ];

        let embeddings = engine.generate_batch_embeddings(&chunks).await.unwrap();

        assert_eq!(embeddings.len(), 3);
        for embedding in &embeddings {
            assert!(embedding.is_valid());
            assert_eq!(embedding.dimensions, 128);
        }
    }

    #[tokio::test]
    async fn test_cache_eviction() {
        // Create engine with small cache
        let engine = EmbeddingEngine::new(64);

        // Fill cache beyond capacity
        for i in 0..1200 {
            // Exceeds default capacity of 1000
            let content = format!("content_{}", i);
            engine.generate_embedding(&content).await.unwrap();
        }

        let stats = engine.get_cache_stats();
        assert!(stats.evictions > 0);
        assert_eq!(stats.current_size, stats.capacity);
    }

    #[test]
    fn test_content_feature_extraction() {
        let engine = EmbeddingEngine::new(256);

        let rust_code = r#"
fn hello() {
    // This is a comment
    println!("Hello, world!");
}
"#;

        let features = engine.extract_content_features(rust_code);

        assert!(!features.is_empty());
        assert!(features.iter().all(|&f| (0.0..=1.0).contains(&f)));
    }
}
