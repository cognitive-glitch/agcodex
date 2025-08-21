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

    #[error("parse error: {0}")]
    ParseError(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("unsupported language: {0}")]
    UnsupportedLanguage(String),
}

/// A generic interface that concrete tools may adopt.
pub trait CodeTool {
    type Query;
    type Output;
    fn search(&self, _query: Self::Query) -> Result<Self::Output, ToolError>;
}

pub mod fd_find;
pub mod tree_sitter;

/// Comprehensive tree-sitter query library for structural code analysis
pub mod queries;

/// AST-based agent tools for code analysis and transformation
pub mod ast_agent_tools;

/// Optional: AST-Grep internal tooling. Kept as a stub for now.
pub mod ast_grep;

/// Multi-layer search engine with Tantivy integration
pub mod search;

/// Internal reasoning think tool with multiple strategies
pub mod think;

// Intentionally no `comby` module: Comby is not used in AGCodex.

// Include comprehensive tests for AST agent tools
#[cfg(test)]
mod ast_agent_tools_test;
