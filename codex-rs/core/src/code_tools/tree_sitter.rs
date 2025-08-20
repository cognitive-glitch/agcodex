//! Tree-sitter primary structural tool (scaffold).

use super::{CodeTool, ToolError};

#[derive(Debug, Clone, Default)]
pub struct TreeSitterTool;

#[derive(Debug, Clone)]
pub struct TsQuery {
    pub language: Option<String>,
    pub pattern: String,
}

#[derive(Debug, Clone, Default)]
pub struct TsMatch {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub context: Option<String>,
}

impl CodeTool for TreeSitterTool {
    type Query = TsQuery;
    type Output = Vec<TsMatch>;

    fn search(&self, _query: Self::Query) -> Result<Self::Output, ToolError> {
        Err(ToolError::NotImplemented("tree_sitter"))
    }
}
