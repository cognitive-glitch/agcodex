use std::io;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, ProtocolTsError>;

#[derive(Error, Debug)]
pub enum ProtocolTsError {
    #[error("TypeScript generation failed: {0}")]
    GenerationFailed(String),
    
    #[error("Invalid TypeScript definition: {0}")]
    InvalidDefinition(String),
    
    #[error(transparent)]
    Io(#[from] io::Error),
    
    #[error("{0}")]
    General(String),
}