//! HTTP JSON-RPC transport for MCP
//!
//! Provides bidirectional request-response communication using HTTP POST requests
//! with JSON-RPC payloads.

use crate::{McpRequest, McpResponse};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serena_mcp::HttpTransport;
use std::sync::Arc;
use tracing::{debug, error};

/// Axum handler for HTTP JSON-RPC requests
///
/// Processes incoming MCP requests through the HTTP transport and returns
/// the response as JSON.
pub async fn http_handler(
    State(transport): State<Arc<HttpTransport>>,
    Json(request): Json<McpRequest>,
) -> Response {
    debug!(
        "Received HTTP JSON-RPC request: method={}, id={:?}",
        request.method, request.id
    );

    let response = transport.handle_request(request).await;

    debug!(
        "Sending HTTP JSON-RPC response: id={:?}, error={:?}",
        response.id,
        response.error.as_ref().map(|e| &e.message)
    );

    Json(response).into_response()
}

/// Batch request handler for multiple JSON-RPC requests
///
/// Processes multiple MCP requests in a single HTTP request and returns
/// all responses as a JSON array.
pub async fn http_batch_handler(
    State(transport): State<Arc<HttpTransport>>,
    Json(requests): Json<Vec<McpRequest>>,
) -> Response {
    debug!(
        "Received HTTP JSON-RPC batch request: {} items",
        requests.len()
    );

    if requests.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(McpResponse::error(
                None,
                -32600,
                "Invalid Request: empty batch",
            )),
        )
            .into_response();
    }

    // Process all requests concurrently
    let futures: Vec<_> = requests
        .into_iter()
        .map(|req| {
            let transport = Arc::clone(&transport);
            async move { transport.handle_request(req).await }
        })
        .collect();

    let responses = futures::future::join_all(futures).await;

    debug!(
        "Sending HTTP JSON-RPC batch response: {} items",
        responses.len()
    );

    Json(responses).into_response()
}

/// Error response for invalid JSON
pub fn invalid_json_error() -> Response {
    error!("Invalid JSON in request");
    (
        StatusCode::BAD_REQUEST,
        Json(McpResponse::error(None, -32700, "Parse error")),
    )
        .into_response()
}

/// Error response for internal errors
pub fn internal_error(message: impl Into<String>) -> Response {
    let msg = message.into();
    error!("Internal error: {}", msg);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(McpResponse::error(None, -32603, msg)),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serena_core::ToolRegistry;
    use serena_mcp::{HttpTransport as McpHttpTransport, SerenaMcpServer};

    #[tokio::test]
    async fn test_http_handler_with_mcp_server() {
        let registry = ToolRegistry::new();
        let mcp_server = Arc::new(SerenaMcpServer::new(registry));
        let transport = Arc::new(McpHttpTransport::new(mcp_server));

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
    async fn test_batch_handler_concurrent_processing() {
        let registry = ToolRegistry::new();
        let mcp_server = Arc::new(SerenaMcpServer::new(registry));
        let transport = Arc::new(McpHttpTransport::new(mcp_server));

        let requests = vec![
            McpRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(1),
                method: "ping".to_string(),
                params: None,
            },
            McpRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(2),
                method: "initialize".to_string(),
                params: None,
            },
        ];

        let futures: Vec<_> = requests
            .into_iter()
            .map(|req| {
                let transport = Arc::clone(&transport);
                async move { transport.handle_request(req).await }
            })
            .collect();

        let responses = futures::future::join_all(futures).await;

        assert_eq!(responses.len(), 2);
        assert_eq!(responses[0].id, Some(1));
        assert_eq!(responses[1].id, Some(2));
    }
}
