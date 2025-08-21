//! Embedding index management with strict separation by model and repository.
//!
//! CRITICAL: Never mix embeddings from different models or repositories.
//! Each combination of (repo, model, dimensions) gets its own isolated index.

use super::EmbeddingVector;
use dashmap::DashMap;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IndexError {
    #[error("Index not found for key: {0:?}")]
    IndexNotFound(IndexKey),

    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Key for identifying a unique embedding index.
/// Each combination of repository, model, and dimensions gets a separate index.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct IndexKey {
    /// Canonical path to the repository root
    pub repo_root: PathBuf,
    /// Model identifier (e.g., "openai:text-embedding-3-small")
    pub model_id: String,
    /// Embedding dimensions (e.g., 256, 768, 1536, 3072)
    pub dimensions: usize,
}

impl IndexKey {
    pub fn new(repo: &Path, model: &str, dimensions: usize) -> Result<Self, IndexError> {
        let repo_root = repo.canonicalize()?;
        Ok(Self {
            repo_root,
            model_id: model.to_string(),
            dimensions,
        })
    }

    /// Generate a stable hash for filesystem storage
    pub fn storage_hash(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

/// A vector index for storing and searching embeddings
pub struct VectorIndex {
    dimensions: usize,
    vectors: Vec<(Vec<f32>, ChunkMetadata)>,
    // In production, this would use a proper vector database like LanceDB
}

impl VectorIndex {
    pub const fn new(dimensions: usize) -> Self {
        Self {
            dimensions,
            vectors: Vec::new(),
        }
    }

    pub fn insert(&mut self, vector: Vec<f32>, metadata: ChunkMetadata) -> Result<(), IndexError> {
        if vector.len() != self.dimensions {
            return Err(IndexError::DimensionMismatch {
                expected: self.dimensions,
                actual: vector.len(),
            });
        }
        self.vectors.push((vector, metadata));
        Ok(())
    }

    pub fn search(&self, query: &[f32], limit: usize) -> Result<Vec<SearchResult>, IndexError> {
        if query.len() != self.dimensions {
            return Err(IndexError::DimensionMismatch {
                expected: self.dimensions,
                actual: query.len(),
            });
        }

        // Simple cosine similarity search (would use HNSW or similar in production)
        let mut results: Vec<_> = self
            .vectors
            .iter()
            .map(|(vec, meta)| {
                let similarity = cosine_similarity(query, vec);
                SearchResult {
                    similarity,
                    metadata: meta.clone(),
                }
            })
            .collect();

        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
        results.truncate(limit);

        Ok(results)
    }

    pub async fn save_to_disk(&self, path: &Path) -> Result<(), IndexError> {
        let data = bincode::encode_to_vec(&self.vectors, bincode::config::standard())
            .map_err(|e| IndexError::Serialization(e.to_string()))?;
        tokio::fs::write(path, data).await?;
        Ok(())
    }

    pub async fn load_from_disk(path: &Path) -> Result<Self, IndexError> {
        let data = tokio::fs::read(path).await?;
        let vectors: Vec<(EmbeddingVector, ChunkMetadata)> =
            bincode::decode_from_slice(&data, bincode::config::standard())
                .map_err(|e| IndexError::Serialization(e.to_string()))?
                .0; // Extract the decoded value from the tuple

        // Infer dimensions from first vector
        let dimensions = vectors.first().map(|(v, _)| v.len()).unwrap_or(1536);

        Ok(Self {
            dimensions,
            vectors,
        })
    }
}

/// Metadata for an embedded chunk
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, bincode::Encode, bincode::Decode)]
pub struct ChunkMetadata {
    pub file_path: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
    pub content_hash: u64,
    pub chunk_type: String,
    pub symbols: Vec<String>,
}

/// Search result with similarity score
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub similarity: f32,
    pub metadata: ChunkMetadata,
}

/// Manager for multiple embedding indexes with strict separation
pub struct EmbeddingIndexManager {
    /// Map from IndexKey to VectorIndex
    indexes: DashMap<IndexKey, Arc<VectorIndex>>,
    /// Base directory for persistent storage
    storage_dir: PathBuf,
}

impl EmbeddingIndexManager {
    pub fn new(storage_dir: PathBuf) -> Self {
        Self {
            indexes: DashMap::new(),
            storage_dir,
        }
    }

    /// Get or create an index for the given repository, model, and dimensions
    pub fn get_or_create_index(
        &self,
        repo: &Path,
        model: &str,
        dimensions: usize,
    ) -> Result<Arc<VectorIndex>, IndexError> {
        let key = IndexKey::new(repo, model, dimensions)?;

        // Check if index already exists in memory
        if let Some(index) = self.indexes.get(&key) {
            return Ok(index.clone());
        }

        // Try to load from disk
        let storage_path = self.index_storage_path(&key);
        if storage_path.exists() {
            let runtime = tokio::runtime::Runtime::new()?;
            let index = runtime.block_on(VectorIndex::load_from_disk(&storage_path))?;
            let index = Arc::new(index);
            self.indexes.insert(key, index.clone());
            return Ok(index);
        }

        // Create new index
        let index = Arc::new(VectorIndex::new(dimensions));
        self.indexes.insert(key, index.clone());
        Ok(index)
    }

    /// Search within a specific index
    pub fn search(
        &self,
        repo: &Path,
        model: &str,
        dimensions: usize,
        query_vector: &[f32],
        limit: usize,
    ) -> Result<Vec<SearchResult>, IndexError> {
        let index = self.get_or_create_index(repo, model, dimensions)?;
        index.search(query_vector, limit)
    }

    /// Get storage path for an index
    fn index_storage_path(&self, key: &IndexKey) -> PathBuf {
        self.storage_dir
            .join(key.storage_hash())
            .join(&key.model_id)
            .join(format!("dim_{}", key.dimensions))
            .join("index.bincode")
    }

    /// Save all indexes to disk
    pub async fn save_all(&self) -> Result<(), IndexError> {
        for entry in self.indexes.iter() {
            let key = entry.key();
            let index = entry.value();
            let path = self.index_storage_path(key);

            // Create directory structure
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            index.save_to_disk(&path).await?;
        }
        Ok(())
    }

    /// Get statistics about loaded indexes
    pub fn stats(&self) -> IndexManagerStats {
        let mut stats = IndexManagerStats::default();

        for entry in self.indexes.iter() {
            stats.total_indexes += 1;

            // Count by model
            let model = &entry.key().model_id;
            *stats.indexes_by_model.entry(model.clone()).or_insert(0) += 1;

            // Count by dimensions
            let dims = entry.key().dimensions;
            *stats.indexes_by_dimensions.entry(dims).or_insert(0) += 1;
        }

        stats
    }
}

/// Statistics about the index manager
#[derive(Debug, Default)]
pub struct IndexManagerStats {
    pub total_indexes: usize,
    pub indexes_by_model: std::collections::HashMap<String, usize>,
    pub indexes_by_dimensions: std::collections::HashMap<usize, usize>,
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a * magnitude_b == 0.0 {
        0.0
    } else {
        dot_product / (magnitude_a * magnitude_b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_index_key_separation() {
        let key1 = IndexKey::new(Path::new("/repo1"), "model1", 1536).unwrap();
        let key2 = IndexKey::new(Path::new("/repo1"), "model2", 1536).unwrap();
        let key3 = IndexKey::new(Path::new("/repo2"), "model1", 1536).unwrap();
        let key4 = IndexKey::new(Path::new("/repo1"), "model1", 768).unwrap();

        // All keys should be different
        assert_ne!(key1, key2); // Different model
        assert_ne!(key1, key3); // Different repo
        assert_ne!(key1, key4); // Different dimensions
    }

    #[test]
    fn test_dimension_mismatch_protection() {
        let mut index = VectorIndex::new(1536);

        // Should succeed with correct dimensions
        let vec_1536 = vec![0.1; 1536];
        let metadata = ChunkMetadata {
            file_path: PathBuf::from("test.rs"),
            start_line: 1,
            end_line: 10,
            content_hash: 12345,
            chunk_type: "function".to_string(),
            symbols: vec!["test_fn".to_string()],
        };
        assert!(index.insert(vec_1536, metadata.clone()).is_ok());

        // Should fail with wrong dimensions
        let vec_768 = vec![0.1; 768];
        assert!(matches!(
            index.insert(vec_768, metadata),
            Err(IndexError::DimensionMismatch { .. })
        ));
    }

    #[test]
    fn test_index_manager_separation() {
        let dir = tempdir().unwrap();
        let manager = EmbeddingIndexManager::new(dir.path().to_path_buf());

        // Create indexes for different combinations
        let index1 = manager
            .get_or_create_index(Path::new("/repo1"), "openai:text-embedding-3-small", 1536)
            .unwrap();

        let index2 = manager
            .get_or_create_index(Path::new("/repo1"), "gemini:embedding-001", 768)
            .unwrap();

        let index3 = manager
            .get_or_create_index(Path::new("/repo2"), "openai:text-embedding-3-small", 1536)
            .unwrap();

        // All indexes should be separate instances
        assert!(!Arc::ptr_eq(&index1, &index2));
        assert!(!Arc::ptr_eq(&index1, &index3));
        assert!(!Arc::ptr_eq(&index2, &index3));

        // Stats should show 3 indexes
        let stats = manager.stats();
        assert_eq!(stats.total_indexes, 3);
    }
}
