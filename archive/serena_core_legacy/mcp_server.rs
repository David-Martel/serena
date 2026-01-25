//! Serena MCP Server - Core server setup and lifecycle

use rust_mcp_sdk::schema::{
    Implementation, InitializeResult, ServerCapabilities, ServerCapabilitiesTools,
    LATEST_PROTOCOL_VERSION,
};
use rust_mcp_sdk::McpServer;
use rust_mcp_sdk::{mcp_server::server_runtime, StdioTransport, TransportOptions};

use super::error::SerenaResult;
use super::handler::SerenaServerHandler;
use crate::mcp::tools::cli::CommandArguments;

/// Server metadata for MCP initialization
pub fn server_details() -> InitializeResult {
    InitializeResult {
        server_info: Implementation {
            name: "serena-mcp-server".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            title: Some("Serena MCP Server".to_string()),
        },
        capabilities: ServerCapabilities {
            experimental: None,
            logging: None,
            prompts: None,
            resources: None,
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            completions: None,
        },
        instructions: Some(get_server_instructions()),
        meta: None,
        protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
    }
}

/// Start the MCP server with stdio transport
pub async fn start_server(args: CommandArguments) -> SerenaResult<()> {
    let transport = StdioTransport::new(TransportOptions::default())
        .map_err(|e| super::error::SerenaError::Transport(e.to_string()))?;

    let handler = SerenaServerHandler::new(&args)?;
    let server = server_runtime::create_server(server_details(), transport, handler);

    server
        .start()
        .await
        .map_err(|e| super::error::SerenaError::Transport(e.to_string()))?;

    Ok(())
}

/// Get the initial instructions for the MCP server
fn get_server_instructions() -> String {
    r#"You are a professional coding agent using Serena's semantic coding tools.
You have access to semantic coding tools upon which you rely heavily for all your work.
You operate in a resource-efficient and intelligent manner, always keeping in mind to not read or generate
content that is not needed for the task at hand.

Available tool categories:
- File Tools: read_file, create_text_file, list_dir, find_file, replace_content, search_for_pattern
- Symbol Tools: get_symbols_overview, find_symbol, find_referencing_symbols, replace_symbol_body, rename_symbol
- Memory Tools: write_memory, read_memory, list_memories, delete_memory
- Config Tools: activate_project, get_current_config, switch_modes

Use symbolic reading tools to understand code structure before making changes.
Always prefer editing existing files over creating new ones."#.to_string()
}
