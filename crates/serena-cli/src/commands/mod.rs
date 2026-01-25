//! Command implementations for Serena CLI
//!
//! This module contains the implementation of all CLI subcommands.

pub mod start;

use anyhow::Result;

/// Trait for command execution
pub trait Execute {
    /// Execute the command
    fn execute(&self) -> impl std::future::Future<Output = Result<()>> + Send;
}
