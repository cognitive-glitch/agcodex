use crate::parser::ParseError;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, PatchError>;

// Legacy alias for migration
pub type ApplyPatchError = PatchError;

#[derive(Error, Debug)]
pub enum PatchError {
    #[error("no files were modified")]
    NoFilesModified,

    #[error("failed to extract patch from response: {0}")]
    ExtractFailed(String),

    #[error("invalid patch format: {0}")]
    InvalidPatch(String),

    #[error("file operation failed for {path}: {error}")]
    FileOperation { path: PathBuf, error: String },

    #[error("patch application failed: {0}")]
    ApplicationFailed(String),

    #[error("git operation failed: {0}")]
    GitOperation(String),

    #[error(transparent)]
    ParseError(#[from] ParseError),

    #[error("{context}: {source}")]
    IoError {
        context: String,
        #[source]
        source: io::Error,
    },

    #[error("{0}")]
    ComputeReplacements(String),

    #[error("{0}")]
    General(String),
}

// Manual PartialEq implementation since io::Error doesn't implement PartialEq
impl PartialEq for PatchError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (PatchError::NoFilesModified, PatchError::NoFilesModified) => true,
            (PatchError::ExtractFailed(a), PatchError::ExtractFailed(b)) => a == b,
            (PatchError::InvalidPatch(a), PatchError::InvalidPatch(b)) => a == b,
            (
                PatchError::FileOperation {
                    path: p1,
                    error: e1,
                },
                PatchError::FileOperation {
                    path: p2,
                    error: e2,
                },
            ) => p1 == p2 && e1 == e2,
            (PatchError::ApplicationFailed(a), PatchError::ApplicationFailed(b)) => a == b,
            (PatchError::GitOperation(a), PatchError::GitOperation(b)) => a == b,
            (PatchError::ParseError(a), PatchError::ParseError(b)) => a == b,
            (
                PatchError::IoError {
                    context: c1,
                    source: s1,
                },
                PatchError::IoError {
                    context: c2,
                    source: s2,
                },
            ) => c1 == c2 && s1.to_string() == s2.to_string(),
            (PatchError::ComputeReplacements(a), PatchError::ComputeReplacements(b)) => a == b,
            (PatchError::General(a), PatchError::General(b)) => a == b,
            _ => false,
        }
    }
}
