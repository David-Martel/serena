//! Serena configuration management
//!
//! This crate provides configuration structures and loading logic for Serena,
//! including project configurations, language settings, contexts, and modes.

pub mod context_mode;
pub mod language;
pub mod loader;
pub mod project;
pub mod serena_config;
pub mod service;
pub mod tools;

pub use context_mode::{Context, Mode};
pub use language::Language;
pub use project::ProjectConfig;
pub use serena_config::SerenaConfig;
pub use service::ConfigService;

// Re-export tools
pub use tools::{
    create_config_tools, ActivateProjectTool, GetActiveToolsTool, GetCurrentConfigTool,
    ListProjectsTool, RemoveProjectTool, SwitchModesTool,
};

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("Invalid configuration: {0}")]
    Invalid(String),

    #[error("Config file not found at: {0}")]
    FileNotFound(String),
}

/// Result type for configuration operations
pub type Result<T> = std::result::Result<T, ConfigError>;
