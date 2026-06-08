use thiserror::Error;
use std::io;
use std::path::PathBuf;

#[derive(Error, Debug)]
pub enum VfsError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("file not found: {0}")]
    NotFound(PathBuf),

    #[error("encoding error: {0}")]
    Encoding(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("mount error: {0}")]
    Mount(String),
}
