//! Serena CLI - Command-line interface for the Serena coding agent toolkit
//!
//! This crate provides the CLI interface for Serena, including:
//! - Argument parsing with clap
//! - Command execution
//! - Transport configuration (stdio, HTTP, SSE)
//! - Logging and tracing setup

pub mod args;
pub mod commands;

pub use args::{Args, LogLevel, Transport};
pub use commands::start::StartCommand;

use anyhow::Result;
use tracing_subscriber::EnvFilter;

/// Initialize logging with the specified log level
pub fn init_logging(level: LogLevel) -> Result<()> {
    let filter =
        EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new(level.as_str()))?;

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_conversion() {
        assert_eq!(LogLevel::Error.as_str(), "error");
        assert_eq!(LogLevel::Warn.as_str(), "warn");
        assert_eq!(LogLevel::Info.as_str(), "info");
        assert_eq!(LogLevel::Debug.as_str(), "debug");
        assert_eq!(LogLevel::Trace.as_str(), "trace");
    }
}
