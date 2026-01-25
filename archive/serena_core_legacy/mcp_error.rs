//! Error types for the Serena MCP server

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SerenaError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("LSP error: {0}")]
    Lsp(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Path not allowed: {0}")]
    PathNotAllowed(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Project not active")]
    NoActiveProject,

    #[error("Language server not started")]
    LspNotStarted,

    #[error("Database error: {0}")]
    Database(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("{0}")]
    Other(String),
}

pub type SerenaResult<T> = Result<T, SerenaError>;

impl From<anyhow::Error> for SerenaError {
    fn from(err: anyhow::Error) -> Self {
        SerenaError::Other(err.to_string())
    }
}

impl From<rusqlite::Error> for SerenaError {
    fn from(err: rusqlite::Error) -> Self {
        SerenaError::Database(err.to_string())
    }
}
