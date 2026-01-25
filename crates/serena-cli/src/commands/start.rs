//! Start command - launches the Serena MCP server

use crate::args::Transport;
use crate::commands::Execute;
use anyhow::{Context, Result};
use serena_config::{create_config_tools, ConfigService};
use serena_core::ToolRegistryBuilder;
use serena_lsp::{create_lsp_tools, LanguageServerManager};
use serena_mcp::SerenaMcpServer;
use serena_memory::{create_memory_tools, MemoryManager};
use serena_tools::ToolFactory;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info};

#[cfg(feature = "web")]
use serena_web::{WebServer, WebServerConfig};

/// Configuration for the start command
#[derive(Debug, Clone)]
pub struct StartCommand {
    /// Transport mechanism to use
    pub transport: Transport,

    /// Host address (for network transports)
    pub host: String,

    /// Port number (for network transports)
    pub port: u16,

    /// Configuration file path
    pub config_path: Option<PathBuf>,

    /// Project directory path
    pub project_path: Option<PathBuf>,
}

impl StartCommand {
    /// Create a new start command with the given configuration
    pub fn new(
        transport: Transport,
        host: String,
        port: u16,
        config_path: Option<PathBuf>,
        project_path: Option<PathBuf>,
    ) -> Self {
        Self {
            transport,
            host,
            port,
            config_path,
            project_path,
        }
    }

    /// Validate the command configuration
    fn validate(&self) -> Result<()> {
        // Validate network configuration for network transports
        if self.transport.requires_network() {
            if self.port == 0 {
                anyhow::bail!("Port must be greater than 0 for network transports");
            }
            if self.host.is_empty() {
                anyhow::bail!("Host must not be empty for network transports");
            }
        }

        // Validate paths exist if provided
        if let Some(ref config) = self.config_path {
            if !config.exists() {
                anyhow::bail!("Configuration file does not exist: {}", config.display());
            }
        }

        if let Some(ref project) = self.project_path {
            if !project.exists() {
                anyhow::bail!("Project directory does not exist: {}", project.display());
            }
            if !project.is_dir() {
                anyhow::bail!("Project path is not a directory: {}", project.display());
            }
        }

        Ok(())
    }

    /// Build tool registry with all available tools
    fn build_tool_registry(&self) -> Result<serena_core::ToolRegistry> {
        let root_path = self
            .project_path
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        info!("Building tool registry with project root: {}", root_path.display());

        // Initialize managers
        let lsp_manager = Arc::new(LanguageServerManager::new(root_path.clone()));
        let memory_manager = Arc::new(
            MemoryManager::new(&root_path).context("Failed to initialize memory manager")?,
        );
        let config_service = Arc::new(ConfigService::new());

        // Build tool factory
        let tool_factory = ToolFactory::new(&root_path);

        // Build comprehensive tool registry
        let registry = ToolRegistryBuilder::new()
            // Core tools: file, editor, workflow, command (18 tools)
            .add_tools(tool_factory.core_tools())
            // Memory tools (6 tools)
            .add_tools(create_memory_tools(Arc::clone(&memory_manager)))
            // Config tools (6 tools)
            .add_tools(create_config_tools(Arc::clone(&config_service)))
            // LSP management tools (4 tools)
            .add_tools(create_lsp_tools(Arc::clone(&lsp_manager)))
            .build();

        info!("Registered {} tools in registry", registry.len());
        Ok(registry)
    }

    /// Start the MCP server with stdio transport
    async fn start_stdio(&self) -> Result<()> {
        info!("Starting Serena MCP server with stdio transport");

        // Build tool registry
        let registry = self.build_tool_registry()?;

        // Create MCP server with registered tools
        let mcp_server = SerenaMcpServer::new(registry);

        info!("MCP server initialization complete");
        info!("Listening on stdio for MCP requests");

        // Start serving - this blocks until client disconnects
        mcp_server.serve_stdio().await?;

        info!("Shutting down MCP server");
        Ok(())
    }

    /// Start the MCP server with HTTP transport
    #[cfg(feature = "web")]
    async fn start_http(&self) -> Result<()> {
        info!(
            "Starting Serena MCP server with HTTP transport on {}:{}",
            self.host, self.port
        );

        // Build tool registry
        let registry = self.build_tool_registry()?;

        // Create MCP server with registered tools
        let mcp_server = Arc::new(SerenaMcpServer::new(registry));

        // Configure web server
        let bind_addr = format!("{}:{}", self.host, self.port)
            .parse()
            .context("Invalid bind address")?;

        let config = WebServerConfig {
            bind_addr,
            enable_cors: true,
            max_body_size: 10 * 1024 * 1024, // 10MB
        };

        // Create and start web server
        let server = WebServer::with_config(mcp_server, config);

        info!("MCP server initialization complete");
        info!(
            "Listening on http://{}:{} for MCP requests",
            self.host, self.port
        );

        // Start serving - this blocks until shutdown
        server.serve().await?;

        info!("Shutting down MCP server");
        Ok(())
    }

    #[cfg(not(feature = "web"))]
    async fn start_http(&self) -> Result<()> {
        anyhow::bail!("HTTP transport requires 'web' feature to be enabled. Rebuild with --features web")
    }

    /// Start the MCP server with SSE transport
    #[cfg(feature = "web")]
    async fn start_sse(&self) -> Result<()> {
        info!(
            "Starting Serena MCP server with SSE transport on {}:{}",
            self.host, self.port
        );

        // SSE is implemented as part of the web server
        // The web server exposes both /mcp (HTTP) and /mcp/events (SSE) endpoints

        // Build tool registry
        let registry = self.build_tool_registry()?;

        // Create MCP server with registered tools
        let mcp_server = Arc::new(SerenaMcpServer::new(registry));

        // Configure web server
        let bind_addr = format!("{}:{}", self.host, self.port)
            .parse()
            .context("Invalid bind address")?;

        let config = WebServerConfig {
            bind_addr,
            enable_cors: true,
            max_body_size: 10 * 1024 * 1024, // 10MB
        };

        // Create and start web server (includes SSE endpoint)
        let server = WebServer::with_config(mcp_server, config);

        info!("MCP server initialization complete");
        info!(
            "Listening on http://{}:{} for SSE connections",
            self.host, self.port
        );
        info!(
            "SSE endpoint: http://{}:{}/mcp/events",
            self.host, self.port
        );

        // Start serving - this blocks until shutdown
        server.serve().await?;

        info!("Shutting down MCP server");
        Ok(())
    }

    #[cfg(not(feature = "web"))]
    async fn start_sse(&self) -> Result<()> {
        anyhow::bail!("SSE transport requires 'web' feature to be enabled. Rebuild with --features web")
    }
}

impl Execute for StartCommand {
    async fn execute(&self) -> Result<()> {
        // Validate configuration
        self.validate().context("Invalid configuration")?;

        // Log configuration
        info!("Transport: {}", self.transport.as_str());
        if let Some(ref config) = self.config_path {
            info!("Config file: {}", config.display());
        }
        if let Some(ref project) = self.project_path {
            info!("Project path: {}", project.display());
        }

        // Start the server based on transport type
        let result = match self.transport {
            Transport::Stdio => self.start_stdio().await,
            Transport::Http => self.start_http().await,
            Transport::Sse => self.start_sse().await,
        };

        if let Err(ref e) = result {
            error!("Failed to start MCP server: {:#}", e);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_command_creation() {
        let cmd = StartCommand::new(Transport::Stdio, "127.0.0.1".to_string(), 3000, None, None);

        assert_eq!(cmd.transport.as_str(), "stdio");
        assert_eq!(cmd.host, "127.0.0.1");
        assert_eq!(cmd.port, 3000);
    }

    #[test]
    fn test_validate_network_transport_requires_port() {
        let cmd = StartCommand::new(Transport::Http, "127.0.0.1".to_string(), 0, None, None);

        assert!(cmd.validate().is_err());
    }

    #[test]
    fn test_validate_network_transport_requires_host() {
        let cmd = StartCommand::new(Transport::Http, String::new(), 3000, None, None);

        assert!(cmd.validate().is_err());
    }

    #[test]
    fn test_validate_stdio_transport() {
        let cmd = StartCommand::new(Transport::Stdio, "127.0.0.1".to_string(), 3000, None, None);

        assert!(cmd.validate().is_ok());
    }
}
