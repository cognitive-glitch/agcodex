use std::io;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, TuiError>;

#[derive(Error, Debug)]
pub enum TuiError {
    #[error("TUI initialization failed: {0}")]
    InitializationFailed(String),
    
    #[error("Failed to parse latest tag name '{0}'")]
    InvalidTagName(String),
    
    #[error("Update check failed: {0}")]
    UpdateCheckFailed(String),
    
    #[error(transparent)]
    Io(#[from] io::Error),
    
    #[error(transparent)]
    Core(#[from] codex_core::error::CodexErr),
    
    #[error("{0}")]
    General(String),
}