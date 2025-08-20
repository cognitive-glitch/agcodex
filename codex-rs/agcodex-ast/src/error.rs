//! Error types for AST operations

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AstError {
    #[error("Language not supported: {0}")]
    UnsupportedLanguage(String),

    #[error("Failed to detect language for file: {0}")]
    LanguageDetectionFailed(String),

    #[error("Parser error: {0}")]
    ParserError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Compression error: {0}")]
    CompressionError(String),

    #[error("Index error: {0}")]
    IndexError(String),

    #[error("Invalid AST node: {0}")]
    InvalidNode(String),

    #[error("Timeout parsing file: {0}")]
    ParseTimeout(String),

    #[error("File too large: {size} bytes exceeds maximum {max} bytes")]
    FileTooLarge { size: usize, max: usize },
}

pub type AstResult<T> = Result<T, AstError>;
