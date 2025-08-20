//! Unified code tools scaffolding for AGCodex.
//!
//! Policy per ISSUE: Do not use Comby; prefer Tree-sitter as the primary
//! structural engine. Offer AST-Grep as optional internal tooling.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("tool not implemented: {0}")]
    NotImplemented(&'static str),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("invalid query: {0}")]
    InvalidQuery(String),
}

/// A generic interface that concrete tools may adopt.
pub trait CodeTool {
    type Query;
    type Output;
    fn search(&self, _query: Self::Query) -> Result<Self::Output, ToolError>;
}

pub mod fd_find;
pub mod ripgrep;
pub mod tree_sitter;

/// Optional: AST-Grep internal tooling. Kept as a stub for now.
pub mod ast_grep;

// Intentionally no `comby` module: Comby is not used in AGCodex.
