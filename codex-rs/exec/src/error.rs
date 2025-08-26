use std::io;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, ExecError>;

#[derive(Error, Debug)]
pub enum ExecError {
    #[error("OSS setup failed: {0}")]
    OssSetupFailed(String),
    
    #[error("execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("invalid arguments: {0}")]
    InvalidArguments(String),
    
    #[error(transparent)]
    Io(#[from] io::Error),
    
    #[error(transparent)]
    Core(#[from] agcodex_core::error::CodexErr),
    
    #[error("{0}")]
    General(String),
}