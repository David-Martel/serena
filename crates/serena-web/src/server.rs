//! Web server for MCP over HTTP/SSE
//!
//! Provides an Axum-based web server that exposes MCP functionality through
//! HTTP JSON-RPC and Server-Sent Events endpoints.

use crate::{api, http, sse, ApiState, HttpTransport, SerenaMcpServer, SseTransport};
use anyhow::{Context, Result};
use axum::{
    http::{header, Method, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{debug, info};

/// Configuration for the web server
#[derive(Debug, Clone)]
pub struct WebServerConfig {
    /// Address to bind the server to
    pub bind_addr: SocketAddr,
    /// Enable CORS for all origins
    pub enable_cors: bool,
    /// Maximum request body size in bytes
    pub max_body_size: usize,
}

impl Default for WebServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:3000"
                .parse()
                .expect("hardcoded bind address is valid"),
            enable_cors: true,
            max_body_size: 10 * 1024 * 1024, // 10MB
        }
    }
}

/// Web server for MCP protocol over HTTP/SSE
pub struct WebServer {
    config: WebServerConfig,
    mcp_server: Arc<SerenaMcpServer>,
}

impl WebServer {
    /// Create a new web server with default configuration
    pub fn new(mcp_server: Arc<SerenaMcpServer>) -> Self {
        Self {
            config: WebServerConfig::default(),
            mcp_server,
        }
    }

    /// Create a new web server with custom configuration
    pub fn with_config(mcp_server: Arc<SerenaMcpServer>, config: WebServerConfig) -> Self {
        Self { config, mcp_server }
    }

    /// Build the Axum router with all endpoints
    fn build_router(&self) -> Router {
        // Create HTTP transport wrapping the MCP server
        let http_transport = Arc::new(HttpTransport::new(Arc::clone(&self.mcp_server)));

        // Create SSE transport
        let sse_transport = Arc::new(SseTransport::new().0);

        // Create API state for dashboard endpoints
        let api_state = Arc::new(ApiState::new(Arc::clone(&self.mcp_server)));

        // Build CORS layer if enabled
        let cors_layer = if self.config.enable_cors {
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
                .max_age(std::time::Duration::from_secs(3600))
        } else {
            CorsLayer::permissive()
        };

        // Build the MCP protocol router
        let mcp_router = Router::new()
            .route("/mcp", post(http::http_handler))
            .route("/mcp/batch", post(http::http_batch_handler))
            .route("/mcp/events", get(sse::sse_handler))
            .with_state(http_transport)
            .layer(Extension(sse_transport));

        // Build the API router for dashboard
        let api_router = Router::new()
            .route("/heartbeat", get(api::heartbeat_handler))
            .route("/get_config", get(api::config_handler))
            .route("/get_stats", get(api::stats_handler))
            .route("/shutdown", post(api::shutdown_handler))
            .route("/tools", get(api::list_tools_handler))
            .route("/info", get(api::info_handler))
            .with_state(api_state);

        // Combine routers
        Router::new()
            // Health check endpoint
            .route("/health", get(health_handler))
            // Merge MCP and API routers
            .merge(mcp_router)
            .merge(api_router)
            // Add middleware - layers are applied bottom-up (last to first)
            .layer(tower_http::trace::TraceLayer::new_for_http())
            .layer(cors_layer)
    }

    /// Start the web server
    ///
    /// This will bind to the configured address and run until shutdown.
    pub async fn serve(self) -> Result<()> {
        let router = self.build_router();
        let listener = tokio::net::TcpListener::bind(&self.config.bind_addr)
            .await
            .context("Failed to bind server")?;

        info!(
            "MCP web server listening on http://{}",
            self.config.bind_addr
        );
        info!("MCP Protocol:");
        info!("  - HTTP JSON-RPC: POST /mcp");
        info!("  - Batch requests: POST /mcp/batch");
        info!("  - SSE events: GET /mcp/events");
        info!("Dashboard API:");
        info!("  - Heartbeat: GET /heartbeat");
        info!("  - Config: GET /get_config");
        info!("  - Stats: GET /get_stats");
        info!("  - Tools: GET /tools");
        info!("  - Info: GET /info");
        info!("  - Health: GET /health");
        info!("  - Shutdown: POST /shutdown");

        axum::serve(listener, router)
            .await
            .context("Server failed")?;

        Ok(())
    }

    /// Get the server's bind address
    pub fn bind_addr(&self) -> SocketAddr {
        self.config.bind_addr
    }
}

/// Health check endpoint handler
async fn health_handler() -> impl IntoResponse {
    debug!("Health check requested");
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "service": "serena-mcp",
            "version": env!("CARGO_PKG_VERSION"),
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_server_config_default() {
        let config = WebServerConfig::default();
        assert_eq!(config.bind_addr.port(), 3000);
        assert!(config.enable_cors);
        assert_eq!(config.max_body_size, 10 * 1024 * 1024);
    }

    #[test]
    fn test_web_server_creation() {
        use serena_core::ToolRegistry;

        // Create a mock MCP server with empty registry
        let registry = ToolRegistry::new();
        let mcp_server = Arc::new(SerenaMcpServer::new(registry));
        let server = WebServer::new(mcp_server);

        assert_eq!(server.bind_addr().port(), 3000);
    }

    #[test]
    fn test_web_server_with_custom_config() {
        use serena_core::ToolRegistry;

        let config = WebServerConfig {
            bind_addr: "0.0.0.0:8080".parse().unwrap(),
            enable_cors: false,
            max_body_size: 5 * 1024 * 1024,
        };

        let registry = ToolRegistry::new();
        let mcp_server = Arc::new(SerenaMcpServer::new(registry));
        let server = WebServer::with_config(mcp_server, config);

        assert_eq!(server.bind_addr().port(), 8080);
    }
}
