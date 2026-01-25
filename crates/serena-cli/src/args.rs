//! Command-line argument parsing for Serena CLI

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Serena - A language-agnostic coding agent toolkit
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Transport mechanism to use
    #[arg(short, long, value_enum, default_value = "stdio")]
    pub transport: Transport,

    /// Path to configuration file
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Path to project directory
    #[arg(short, long)]
    pub project: Option<PathBuf>,

    /// Logging level
    #[arg(short, long, value_enum, default_value = "info")]
    pub log_level: LogLevel,

    /// Port for HTTP/SSE transports
    #[arg(long, default_value = "3000")]
    pub port: u16,

    /// Host address for HTTP/SSE transports
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Transport mechanisms for MCP communication
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Transport {
    /// Standard input/output (default for MCP servers)
    Stdio,
    /// HTTP with JSON-RPC
    Http,
    /// Server-Sent Events
    Sse,
}

impl Transport {
    /// Returns true if this transport requires network configuration
    pub fn requires_network(&self) -> bool {
        matches!(self, Transport::Http | Transport::Sse)
    }

    /// Returns the string representation of the transport
    pub fn as_str(&self) -> &'static str {
        match self {
            Transport::Stdio => "stdio",
            Transport::Http => "http",
            Transport::Sse => "sse",
        }
    }
}

/// Logging levels
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum LogLevel {
    /// Only errors
    Error,
    /// Warnings and errors
    Warn,
    /// Info, warnings, and errors (default)
    Info,
    /// Debug information and above
    Debug,
    /// All log messages including trace
    Trace,
}

impl LogLevel {
    /// Returns the tracing filter string for this log level
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }
}

/// Available subcommands
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Start the Serena MCP server
    Start {
        /// Override transport for this command
        #[arg(short, long, value_enum)]
        transport: Option<Transport>,

        /// Override port for this command
        #[arg(short, long)]
        port: Option<u16>,

        /// Override host for this command
        #[arg(long)]
        host: Option<String>,
    },

    /// Index a project for faster tool performance
    Index {
        /// Project path to index (defaults to current directory)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },

    /// Show configuration information
    Config {
        /// Show effective configuration (merged from all sources)
        #[arg(short, long)]
        effective: bool,
    },

    /// Manage language servers
    Lsp {
        /// LSP subcommand
        #[command(subcommand)]
        command: LspCommand,
    },
}

/// Language server management commands
#[derive(Debug, Subcommand)]
pub enum LspCommand {
    /// List available language servers
    List,

    /// Install a language server
    Install {
        /// Language identifier
        language: String,
    },

    /// Check status of language servers
    Status,

    /// Update all language servers
    Update,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_requires_network() {
        assert!(!Transport::Stdio.requires_network());
        assert!(Transport::Http.requires_network());
        assert!(Transport::Sse.requires_network());
    }

    #[test]
    fn test_transport_as_str() {
        assert_eq!(Transport::Stdio.as_str(), "stdio");
        assert_eq!(Transport::Http.as_str(), "http");
        assert_eq!(Transport::Sse.as_str(), "sse");
    }

    #[test]
    fn test_log_level_as_str() {
        assert_eq!(LogLevel::Error.as_str(), "error");
        assert_eq!(LogLevel::Warn.as_str(), "warn");
        assert_eq!(LogLevel::Info.as_str(), "info");
        assert_eq!(LogLevel::Debug.as_str(), "debug");
        assert_eq!(LogLevel::Trace.as_str(), "trace");
    }
}
