//! HTTP transport adapter for MCP
//!
//! Provides integration between the MCP server and HTTP-based transports.
//! This module wraps the SerenaMcpServer to provide an async-compatible
//! handler for use with serena-web's HTTP server.

use crate::protocol::{McpRequest, McpResponse};
use crate::server::SerenaMcpServer;
use std::sync::Arc;
use tracing::{debug, trace};

/// HTTP transport wrapper for SerenaMcpServer
///
/// This adapter allows the MCP server to be used with HTTP-based transports
/// by providing an async-compatible request handler.
#[derive(Clone)]
pub struct HttpTransport {
    mcp_server: Arc<SerenaMcpServer>,
}

impl HttpTransport {
    /// Create a new HTTP transport wrapping an MCP server
    pub fn new(mcp_server: Arc<SerenaMcpServer>) -> Self {
        debug!("Creating HTTP transport");
        Self { mcp_server }
    }

    /// Handle an incoming MCP request asynchronously
    ///
    /// This method processes the request through the MCP server and returns
    /// the appropriate response.
    pub async fn handle_request(&self, request: McpRequest) -> McpResponse {
        trace!(
            "HTTP transport handling request: method={}, id={:?}",
            request.method,
            request.id
        );

        self.mcp_server.handle_request(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serena_core::ToolRegistry;

    #[tokio::test]
    async fn test_http_transport_initialize() {
        let registry = ToolRegistry::new();
        let mcp_server = Arc::new(SerenaMcpServer::new(registry));
        let transport = HttpTransport::new(mcp_server);

        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(1),
            method: "initialize".to_string(),
            params: None,
        };

        let response = transport.handle_request(request).await;

        assert_eq!(response.id, Some(1));
        assert!(response.error.is_none());
        assert!(response.result.is_some());
    }

    #[tokio::test]
    async fn test_http_transport_ping() {
        let registry = ToolRegistry::new();
        let mcp_server = Arc::new(SerenaMcpServer::new(registry));
        let transport = HttpTransport::new(mcp_server);

        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(1),
            method: "ping".to_string(),
            params: None,
        };

        let response = transport.handle_request(request).await;

        assert_eq!(response.id, Some(1));
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_http_transport_unknown_method() {
        let registry = ToolRegistry::new();
        let mcp_server = Arc::new(SerenaMcpServer::new(registry));
        let transport = HttpTransport::new(mcp_server);

        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(1),
            method: "unknown_method".to_string(),
            params: None,
        };

        let response = transport.handle_request(request).await;

        assert_eq!(response.id, Some(1));
        assert!(response.result.is_none());
        assert!(response.error.is_some());

        let error = response.error.unwrap();
        assert_eq!(error.code, -32601);
    }
}
