//! ripgrep native integration (scaffold).

use super::CodeTool;
use super::ToolError;

#[derive(Debug, Clone, Default)]
pub struct RipGrep;

#[derive(Debug, Clone)]
pub struct RgQuery {
    pub pattern: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct RgMatch {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub line_text: Option<String>,
}

impl CodeTool for RipGrep {
    type Query = RgQuery;
    type Output = Vec<RgMatch>;

    fn search(&self, _query: Self::Query) -> Result<Self::Output, ToolError> {
        Err(ToolError::NotImplemented("ripgrep"))
    }
}
