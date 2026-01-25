//! Serena LSP Client Library
//!
//! This crate provides a generic LSP client implementation and language server management
//! for the Serena project. It handles communication with language servers via the Language
//! Server Protocol (LSP).

pub mod cache;
pub mod client;
pub mod languages;
pub mod manager;
pub mod resources;
pub mod tools;

// Re-exports
pub use cache::LspCache;
pub use client::{
    JsonRpcError, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, LspClient, LspClientAdapter,
};
pub use languages::{get_config, LanguageServerConfig};
pub use manager::LanguageServerManager;

// Re-export tools
pub use tools::{
    create_lsp_tools, ClearLspCacheTool, ListLanguageServersTool, RestartLanguageServerTool,
    StopLanguageServerTool,
};

// Re-export resource manager
pub use resources::ResourceManager;

/// LSP-related errors
#[derive(Debug, thiserror::Error)]
pub enum LspError {
    #[error("Failed to spawn language server: {0}")]
    SpawnError(String),

    #[error("Language server not found for language: {0:?}")]
    ServerNotFound(serena_config::Language),

    #[error("Language server communication error: {0}")]
    CommunicationError(String),

    #[error("JSON-RPC error {code}: {message}")]
    JsonRpcError { code: i64, message: String },

    #[error("LSP initialization failed: {0}")]
    InitializationError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Result type for LSP operations
pub type Result<T> = std::result::Result<T, LspError>;
