use std::io;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, McpClientError>;

#[derive(Error, Debug)]
pub enum McpClientError {
    #[error("MCP client initialization failed: {0}")]
    InitializationFailed(String),
    
    #[error("MCP protocol error: {0}")]
    ProtocolError(String),
    
    #[error("MCP connection error: {0}")]
    ConnectionError(String),
    
    #[error("MCP request timeout")]
    Timeout,
    
    #[error("MCP server error: {0}")]
    ServerError(String),
    
    #[error(transparent)]
    Io(#[from] io::Error),
    
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    
    #[error("{0}")]
    General(String),
}