use std::io;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, FileSearchError>;

#[derive(Error, Debug)]
pub enum FileSearchError {
    #[error("pattern not found: {0}")]
    PatternNotFound(String),
    
    #[error("invalid search query: {0}")]
    InvalidQuery(String),
    
    #[error("search failed: {0}")]
    SearchFailed(String),
    
    #[error(transparent)]
    Io(#[from] io::Error),
    
    #[error("{0}")]
    General(String),
}