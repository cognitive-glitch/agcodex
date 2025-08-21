//! AGCodex Context Engine scaffolding.
//!
//! This module provides compile-safe placeholders for the AST-RAG system
//! described in PLANS.md (ast_compactor, semantic_index, retrieval,
//! embeddings, cache). Implementations are intentionally minimal and do not
//! pull heavy dependencies yet. They serve as stable integration points to
//! progressively wire in functionality.

pub mod ast_compactor;
pub mod cache;
pub mod embeddings;
pub mod retrieval;
pub mod semantic_index;

pub use ast_compactor::AstCompactor;
pub use ast_compactor::CompactOptions;
pub use ast_compactor::CompactResult;
pub use cache::Cache;
pub use cache::InMemoryCache;
pub use embeddings::EmbeddingError;
pub use embeddings::EmbeddingModel;
pub use embeddings::EmbeddingModelBridge;
pub use embeddings::EmbeddingVector;
pub use embeddings::NoOpEmbeddingModel;
pub use retrieval::ContextRetriever;
pub use retrieval::RetrievalQuery;
pub use retrieval::RetrievalResult;
pub use semantic_index::FileId;
pub use semantic_index::SemanticIndex;
pub use semantic_index::SymbolInfo;
