use std::io;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Arg0Error>;

#[derive(Error, Debug)]
pub enum Arg0Error {
    #[error("dispatch failed: {0}")]
    DispatchFailed(String),
    
    #[error(transparent)]
    Io(#[from] io::Error),
    
    #[error("{0}")]
    General(String),
}