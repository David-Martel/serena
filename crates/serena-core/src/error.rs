use thiserror::Error;

/// Main error type for Serena operations
#[derive(Error, Debug)]
pub enum SerenaError {
    #[error("LSP error: {0}")]
    Lsp(#[from] LspError),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Tool execution error: {0}")]
    Tool(#[from] ToolError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Operation not supported: {0}")]
    NotSupported(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// LSP-specific errors
#[derive(Error, Debug)]
pub enum LspError {
    #[error("Language server not initialized")]
    NotInitialized,

    #[error("Language server failed to start: {0}")]
    StartupFailed(String),

    #[error("Language server crashed: {0}")]
    Crashed(String),

    #[error("Request timeout after {0}ms")]
    Timeout(u64),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),

    #[error("Language not supported: {0}")]
    LanguageNotSupported(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Initialization failed: {0}")]
    InitializationError(String),

    #[error("Shutdown failed: {0}")]
    ShutdownError(String),

    #[error("Request failed: {0}")]
    RequestFailed(String),

    #[error("Notification failed: {0}")]
    NotificationFailed(String),

    #[error("Invalid URI: {0}")]
    InvalidUri(String),
}

/// Configuration errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("Invalid configuration: {0}")]
    Invalid(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Parse error: {0}")]
    Parse(String),
}

/// Tool execution errors
#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Missing required parameter: {0}")]
    MissingParameter(String),

    #[error("Invalid parameter value: {0}")]
    InvalidParameterValue(String),

    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Resource conflict: {0}")]
    Conflict(String),
}
