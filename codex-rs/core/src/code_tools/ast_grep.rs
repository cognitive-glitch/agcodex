//! Optional AST-Grep integration (scaffold).
//! Note: Tree-sitter is the primary structural engine; AST-Grep is offered as
//! internal optional tooling.

use super::{CodeTool, ToolError};

#[derive(Debug, Clone, Default)]
pub struct AstGrep;

#[derive(Debug, Clone)]
pub struct AgQuery {
    pub language: Option<String>,
    pub pattern: String,
}

#[derive(Debug, Clone, Default)]
pub struct AgMatch {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

impl CodeTool for AstGrep {
    type Query = AgQuery;
    type Output = Vec<AgMatch>;

    fn search(&self, _query: Self::Query) -> Result<Self::Output, ToolError> {
        Err(ToolError::NotImplemented("ast_grep"))
    }
}
