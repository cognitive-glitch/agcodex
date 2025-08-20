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

pub use ast_compactor::{AstCompactor, CompactOptions, CompactResult};
pub use cache::{Cache, InMemoryCache};
pub use embeddings::{EmbeddingError, EmbeddingModel, EmbeddingVector};
pub use retrieval::{ContextRetriever, RetrievalQuery, RetrievalResult};
pub use semantic_index::{FileId, SemanticIndex, SymbolInfo};
