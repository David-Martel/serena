use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use serena_config::{
    create_config_tools, loader::ConfigLoader, ConfigService, Language, ProjectConfig, SerenaConfig,
};
use serena_core::{LanguageServer, ToolRegistry, ToolRegistryBuilder};
use serena_lsp::{create_lsp_tools, LanguageServerManager, LspClientAdapter};
use serena_mcp::SerenaMcpServer;
use serena_memory::{create_memory_tools, MemoryManager};
use serena_symbol::create_symbol_tools;
use serena_tools::ToolFactory;

/// Main application structure that manages the Serena lifecycle
pub struct App {
    /// MCP server instance (consumed when starting)
    mcp_server: Option<SerenaMcpServer>,

    /// LSP manager for language servers
    lsp_manager: Arc<LanguageServerManager>,

    /// Memory manager for project knowledge persistence
    memory_manager: Arc<MemoryManager>,

    /// Configuration service
    config_service: Arc<ConfigService>,

    /// Tool registry
    tool_registry: Arc<ToolRegistry>,

    /// Application configuration
    config: Arc<RwLock<SerenaConfig>>,

    /// Active project configuration
    project_config: Arc<RwLock<Option<ProjectConfig>>>,

    /// Configuration loader
    config_loader: ConfigLoader,
}

impl App {
    // ==================== Accessors ====================
    // These make the stored managers accessible for external use

    /// Get a reference to the memory manager
    pub fn memory_manager(&self) -> &Arc<MemoryManager> {
        &self.memory_manager
    }

    /// Get a reference to the config service
    pub fn config_service(&self) -> &Arc<ConfigService> {
        &self.config_service
    }

    /// Get a reference to the config loader
    pub fn config_loader(&self) -> &ConfigLoader {
        &self.config_loader
    }

    /// Get a reference to the LSP manager
    pub fn lsp_manager(&self) -> &Arc<LanguageServerManager> {
        &self.lsp_manager
    }

    /// Get a reference to the tool registry
    pub fn tool_registry(&self) -> &Arc<ToolRegistry> {
        &self.tool_registry
    }
}

impl App {
    /// Create a new App instance
    pub async fn new(config_path: Option<PathBuf>, project_path: Option<PathBuf>) -> Result<Self> {
        info!("Initializing Serena application");

        // Initialize configuration loader
        let config_loader = ConfigLoader::new();

        // Load configuration
        let config = Self::load_config(&config_loader, config_path).await?;
        let config = Arc::new(RwLock::new(config));

        // Determine root path for LSP manager and tools
        let root_path = project_path
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        // Initialize managers
        debug!("Initializing managers with project root: {}", root_path.display());

        // LSP manager for language servers
        let lsp_manager = Arc::new(LanguageServerManager::new(root_path.clone()));

        // Memory manager for project knowledge persistence
        let memory_manager = Arc::new(
            MemoryManager::new(&root_path)
                .context("Failed to initialize memory manager")?
        );

        // Configuration service
        let config_service = Arc::new(ConfigService::new());

        // Build comprehensive tool registry
        debug!("Building tool registry with all tool factories");
        let tool_factory = ToolFactory::new(&root_path);

        let tool_registry = Arc::new(
            ToolRegistryBuilder::new()
                // Core tools: file, editor, workflow, command (18 tools)
                .add_tools(tool_factory.core_tools())
                // Memory tools (6 tools)
                .add_tools(create_memory_tools(Arc::clone(&memory_manager)))
                // Config tools (6 tools)
                .add_tools(create_config_tools(Arc::clone(&config_service)))
                // LSP management tools (4 tools)
                .add_tools(create_lsp_tools(Arc::clone(&lsp_manager)))
                // Note: Symbol tools (7) require an active LSP client and are added
                // dynamically when a project is activated with language support
                .build()
        );
        info!("Registered {} tools in registry", tool_registry.len());

        // Initialize MCP server with tool registry
        debug!("Initializing MCP server");
        let mcp_server = Some(SerenaMcpServer::new((*tool_registry).clone()));

        // Load project if specified
        let project_config = if let Some(project_path) = project_path {
            info!("Loading project from: {}", project_path.display());
            let proj_config = Self::load_project_config(&project_path)?;
            Arc::new(RwLock::new(Some(proj_config)))
        } else {
            Arc::new(RwLock::new(None))
        };

        info!("Serena application initialized successfully");

        Ok(Self {
            mcp_server,
            lsp_manager,
            memory_manager,
            config_service,
            tool_registry,
            config,
            project_config,
            config_loader,
        })
    }

    /// Load configuration from file or use defaults
    async fn load_config(
        loader: &ConfigLoader,
        config_path: Option<PathBuf>,
    ) -> Result<SerenaConfig> {
        if let Some(path) = config_path {
            info!("Loading configuration from: {}", path.display());
            loader
                .load_from_file(&path)
                .context("Failed to load configuration file")
        } else {
            info!("Loading configuration from default locations");
            loader.load().context("Failed to load configuration")
        }
    }

    /// Load project configuration from a directory
    fn load_project_config(project_path: &Path) -> Result<ProjectConfig> {
        // Try to find and load project.yml in the project directory
        let loader = ConfigLoader::new();

        if let Some(config_file) = loader.find_project_config(project_path) {
            info!("Found project config: {}", config_file.display());
            let content = std::fs::read_to_string(&config_file)?;
            let mut config: ProjectConfig = serde_yaml::from_str(&content)?;
            config.root = project_path.to_path_buf();
            Ok(config)
        } else {
            // Create a default project config
            info!("No project config found, creating default");
            let project_name = project_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unnamed")
                .to_string();

            let mut config = ProjectConfig::new(project_name, project_path.to_path_buf());

            // Try to detect languages
            if let Err(e) = config.detect_languages() {
                warn!("Failed to detect languages: {}", e);
            }

            Ok(config)
        }
    }

    /// Set the operating mode
    pub fn set_mode(&mut self, mode: &str) -> Result<()> {
        info!("Setting mode to: {}", mode);
        self.config_service
            .switch_modes(vec![mode.to_string()])
            .context("Failed to switch mode")?;
        Ok(())
    }

    /// Set the context
    pub fn set_context(&mut self, context: &str) -> Result<()> {
        info!("Setting context to: {}", context);
        self.config_service
            .switch_context(context)
            .context("Failed to switch context")?;
        Ok(())
    }

    /// Run the MCP server using stdio transport
    pub async fn run_stdio(mut self) -> Result<()> {
        info!("Running MCP server on stdio transport");

        let server = self
            .mcp_server
            .take()
            .ok_or_else(|| anyhow::anyhow!("MCP server already consumed"))?;

        server.serve_stdio().await
    }

    /// Run the MCP server using HTTP transport
    pub async fn run_http(mut self, port: u16) -> Result<()> {
        use serena_web::{WebServer, WebServerConfig};
        use std::net::SocketAddr;

        info!("Running MCP server on HTTP transport (port {})", port);

        let server = self
            .mcp_server
            .take()
            .ok_or_else(|| anyhow::anyhow!("MCP server already consumed"))?;

        // Configure the web server
        let config = WebServerConfig {
            bind_addr: SocketAddr::from(([0, 0, 0, 0], port)),
            enable_cors: true,
            max_body_size: 10 * 1024 * 1024, // 10MB
        };

        let web_server = WebServer::with_config(Arc::new(server), config);

        info!("Starting HTTP MCP server on port {}", port);
        web_server.serve().await
    }

    /// Run the MCP server using SSE transport
    pub async fn run_sse(mut self, port: u16) -> Result<()> {
        use serena_web::{WebServer, WebServerConfig};
        use std::net::SocketAddr;

        info!("Running MCP server on SSE transport (port {})", port);

        let server = self
            .mcp_server
            .take()
            .ok_or_else(|| anyhow::anyhow!("MCP server already consumed"))?;

        // Configure the web server (SSE uses the same web server infrastructure)
        let config = WebServerConfig {
            bind_addr: SocketAddr::from(([0, 0, 0, 0], port)),
            enable_cors: true,
            max_body_size: 10 * 1024 * 1024, // 10MB
        };

        let web_server = WebServer::with_config(Arc::new(server), config);

        info!("Starting SSE MCP server on port {}", port);
        info!("SSE events available at http://0.0.0.0:{}/mcp/events", port);
        web_server.serve().await
    }

    /// Get the current configuration
    pub async fn get_config(&self) -> SerenaConfig {
        self.config.read().await.clone()
    }

    /// Update the configuration
    pub async fn update_config(&self, config: SerenaConfig) -> Result<()> {
        let mut cfg = self.config.write().await;
        *cfg = config;
        Ok(())
    }

    /// Get the current project configuration
    pub async fn get_project_config(&self) -> Option<ProjectConfig> {
        self.project_config.read().await.clone()
    }

    /// Activate a project
    pub async fn activate_project(&self, project_path: PathBuf) -> Result<()> {
        info!("Activating project: {}", project_path.display());

        // Use ConfigService to activate project (handles registration and validation)
        let proj_config = self
            .config_service
            .activate_project(
                project_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid project path"))?,
            )
            .context("Failed to activate project in ConfigService")?;

        // Start LSP servers for detected languages and wire symbol tools
        for language in &proj_config.languages {
            match self.activate_language_support(&project_path, *language).await {
                Ok(tool_count) => {
                    info!(
                        "Activated {} with {} symbol tools",
                        language.display_name(),
                        tool_count
                    );
                }
                Err(e) => {
                    warn!(
                        "Failed to activate language support for {}: {}",
                        language.display_name(),
                        e
                    );
                }
            }
        }

        // Update App-level project reference
        let mut project = self.project_config.write().await;
        *project = Some(proj_config);

        Ok(())
    }

    /// Activate language support for a specific language
    ///
    /// This starts the language server and registers the symbol tools dynamically.
    async fn activate_language_support(
        &self,
        project_path: &Path,
        language: Language,
    ) -> Result<usize> {
        info!("Activating language support for: {:?}", language);

        // Start the language server
        self.lsp_manager.start_server(language).await?;

        // Get the LSP client (this is kept around for the manager to track)
        let _client = self
            .lsp_manager
            .get_server(language)
            .ok_or_else(|| anyhow::anyhow!("Failed to get LSP client for {:?}", language))?;

        // Create adapter that implements LanguageServer trait
        // Note: We create a new client instance because the adapter needs ownership
        // In practice, we'd want to refactor to share clients more efficiently
        let config = serena_lsp::get_config(language)?;
        let adapter = LspClientAdapter::new(
            serena_lsp::LspClient::new(config.command, config.args).await?,
            language.display_name().to_string(),
        );

        // Wrap in Arc<RwLock<Box<dyn LanguageServer>>> as expected by symbol tools
        // Symbol tools expect tokio::sync::RwLock
        let lsp_client: Arc<tokio::sync::RwLock<Box<dyn LanguageServer>>> =
            Arc::new(tokio::sync::RwLock::new(Box::new(adapter)));

        // Create symbol tools with the LSP client
        let symbol_tools = create_symbol_tools(project_path.to_path_buf(), lsp_client);

        // Register the symbol tools dynamically
        let tool_count = self.tool_registry.extend(symbol_tools);

        info!("Registered {} symbol tools for {:?}", tool_count, language);
        Ok(tool_count)
    }

    /// Deactivate the current project
    pub async fn deactivate_project(&self) -> Result<()> {
        info!("Deactivating current project");

        // Remove symbol tools (they have a common prefix pattern)
        let removed = self.tool_registry.remove_by_prefix("get_symbols_overview");
        let removed = removed + self.tool_registry.remove_by_prefix("find_symbol");
        let removed = removed + self.tool_registry.remove_by_prefix("find_referencing_symbols");
        let removed = removed + self.tool_registry.remove_by_prefix("replace_symbol_body");
        let removed = removed + self.tool_registry.remove_by_prefix("rename_symbol");
        let removed = removed + self.tool_registry.remove_by_prefix("insert_after_symbol");
        let removed = removed + self.tool_registry.remove_by_prefix("insert_before_symbol");

        if removed > 0 {
            info!("Removed {} symbol tools", removed);
        }

        // Stop all LSP servers
        self.lsp_manager.stop_all_servers().await;

        let mut project = self.project_config.write().await;
        *project = None;

        Ok(())
    }

    /// Shutdown the application gracefully
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down Serena application");

        // Save configuration including active project state
        if self.get_project_config().await.is_some() {
            if let Err(e) = self.config_service.save() {
                warn!("Failed to save project state on shutdown: {}", e);
            } else {
                info!("Saved project state successfully");
            }
        }

        // Shutdown all LSP servers
        self.lsp_manager.stop_all_servers().await;

        info!("Serena application shutdown complete");
        Ok(())
    }
}

impl Drop for App {
    fn drop(&mut self) {
        debug!("App instance dropped");
    }
}
