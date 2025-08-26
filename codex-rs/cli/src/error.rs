use std::io;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CliError>;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum CliError {
    #[error("protocol mode expects stdin to be a pipe, not a terminal")]
    ProtocolModeRequiresPipe,

    #[error("debug sandbox command failed: {0}")]
    DebugSandboxFailed(String),

    #[error("invalid command: {0}")]
    InvalidCommand(String),

    #[error("OSS setup failed: {0}")]
    OssSetupFailed(String),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Core(#[from] agcodex_core::error::CodexErr),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),

    #[error("{0}")]
    General(String),
}
