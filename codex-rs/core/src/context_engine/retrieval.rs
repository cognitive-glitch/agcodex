//! Retrieval scaffolding for the Context Engine.

use super::embeddings::EmbeddingVector;
use super::semantic_index::FileId;
use super::semantic_index::SymbolInfo;

#[derive(Debug, Clone, Default)]
pub struct RetrievalQuery {
    pub text: String,
    pub max_results: usize,
}

#[derive(Debug, Clone, Default)]
pub struct RetrievalResult {
    pub files: Vec<FileId>,
    pub symbols: Vec<SymbolInfo>,
    pub neighbors: Vec<(usize, f32)>, // index into results with similarity scores
    pub query_embedding: Option<EmbeddingVector>,
}

#[derive(Debug, Default)]
pub struct ContextRetriever;

impl ContextRetriever {
    pub const fn new() -> Self {
        Self
    }

    pub fn retrieve(&self, _query: &RetrievalQuery) -> RetrievalResult {
        RetrievalResult::default()
    }
}
