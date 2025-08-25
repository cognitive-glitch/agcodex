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

    #[error("Notification system error: {0}")]
    NotificationError(String),

    #[error("Mutex poisoned: {0}")]
    PoisonedMutex(String),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Core(#[from] agcodex_core::error::CodexErr),

    #[error("{0}")]
    General(String),
}
