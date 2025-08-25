use std::io;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, ChatGptError>;

#[derive(Error, Debug)]
pub enum ChatGptError {
    #[error("ChatGPT token not available")]
    TokenNotAvailable,
    
    #[error("ChatGPT account ID not available, please re-run `codex login`")]
    AccountIdNotAvailable,
    
    #[error("Request failed with status {status}: {body}")]
    RequestFailed {
        status: reqwest::StatusCode,
        body: String,
    },
    
    #[error("No diff turn found")]
    NoDiffTurnFound,
    
    #[error("No PR output item found")]
    NoPrOutputItemFound,
    
    #[error("apply must be run from a git repository")]
    NotInGitRepository,
    
    #[error("git operation failed: {0}")]
    GitOperationFailed(String),
    
    #[error(transparent)]
    Io(#[from] io::Error),
    
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    
    #[error("{0}")]
    General(String),
}