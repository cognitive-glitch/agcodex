//! fd-find native integration (scaffold).

use super::CodeTool;
use super::ToolError;

#[derive(Debug, Clone, Default)]
pub struct FdFind;

#[derive(Debug, Clone)]
pub struct FdQuery {
    pub base: String,
    pub glob: Option<String>,
    pub regex: Option<String>,
}

impl CodeTool for FdFind {
    type Query = FdQuery;
    type Output = Vec<String>; // file paths

    fn search(&self, _query: Self::Query) -> Result<Self::Output, ToolError> {
        Err(ToolError::NotImplemented("fd_find"))
    }
}
