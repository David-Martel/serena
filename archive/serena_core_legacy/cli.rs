//! CLI arguments for the Serena MCP server

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(name = "serena-mcp-server")]
#[command(author, version, about = "Serena semantic coding agent MCP server", long_about = None)]
pub struct CommandArguments {
    /// Project directory path (defaults to current directory)
    #[arg(short, long)]
    pub project_path: Option<PathBuf>,

    /// Language server command to use (e.g., "pyright-langserver", "rust-analyzer")
    #[arg(short, long)]
    pub language_server: Option<String>,

    /// Language server arguments
    #[arg(long)]
    pub lsp_args: Vec<String>,

    /// Enable debug logging
    #[arg(short, long, default_value = "false")]
    pub debug: bool,

    /// Memory database path (defaults to .serena/memories.db)
    #[arg(long)]
    pub memory_db: Option<PathBuf>,

    /// Allowed directories for file operations (defaults to project path)
    #[arg(long)]
    pub allowed_dirs: Vec<PathBuf>,
}

impl Default for CommandArguments {
    fn default() -> Self {
        Self {
            project_path: None,
            language_server: None,
            lsp_args: Vec::new(),
            debug: false,
            memory_db: None,
            allowed_dirs: Vec::new(),
        }
    }
}
