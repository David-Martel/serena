use thiserror::Error;

/// Errors that can occur during memory operations
#[derive(Error, Debug)]
pub enum MemoryError {
    /// Memory not found
    #[error("Memory not found: {0}")]
    NotFound(String),

    /// Memory already exists
    #[error("Memory already exists: {0}")]
    AlreadyExists(String),

    /// Invalid memory name
    #[error("Invalid memory name: {0}")]
    InvalidName(String),

    /// Content too large
    #[error("Content too large: {size} bytes exceeds maximum of {max} bytes")]
    ContentTooLarge { size: usize, max: usize },

    /// Database error
    #[error("Database error: {0}")]
    Database(String),

    /// File system error
    #[error("File system error: {0}")]
    FileSystem(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Invalid regex pattern
    #[error("Invalid regex pattern: {0}")]
    InvalidRegex(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

impl From<std::io::Error> for MemoryError {
    fn from(err: std::io::Error) -> Self {
        MemoryError::FileSystem(err.to_string())
    }
}

impl From<rusqlite::Error> for MemoryError {
    fn from(err: rusqlite::Error) -> Self {
        MemoryError::Database(err.to_string())
    }
}

impl From<serde_json::Error> for MemoryError {
    fn from(err: serde_json::Error) -> Self {
        MemoryError::Serialization(err.to_string())
    }
}

impl From<regex::Error> for MemoryError {
    fn from(err: regex::Error) -> Self {
        MemoryError::InvalidRegex(err.to_string())
    }
}
