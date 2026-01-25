//! Integration tests for the MCP stdio transport
//!
//! These tests verify the end-to-end behavior of the MCP protocol
//! using simulated stdin/stdout.

use serena_core::{SerenaError, Tool, ToolRegistryBuilder, ToolResult};
use serena_mcp::protocol::{McpRequest, McpResponse};
use serena_mcp::SerenaMcpServer;
use serde_json::json;
use std::sync::Arc;

/// A simple test tool for integration tests
#[derive(Clone)]
struct EchoTool;

#[async_trait::async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Echoes the input message"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "Message to echo"
                }
            },
            "required": ["message"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, SerenaError> {
        let message = params
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("no message");
        Ok(ToolResult::success(json!({ "echoed": message })))
    }
}

fn create_test_server() -> SerenaMcpServer {
    let registry = ToolRegistryBuilder::new()
        .add_tool(Arc::new(EchoTool) as Arc<dyn Tool>)
        .build();
    SerenaMcpServer::new(registry)
}

#[tokio::test]
async fn test_initialize_request() {
    let server = create_test_server();

    let request = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(1),
        method: "initialize".to_string(),
        params: Some(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        })),
    };

    let response = server.handle_request(request).await;

    assert_eq!(response.jsonrpc, "2.0");
    assert_eq!(response.id, Some(1));
    assert!(response.error.is_none());
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert!(result["serverInfo"]["name"].as_str().is_some());
    assert!(result["serverInfo"]["version"].as_str().is_some());
}

#[tokio::test]
async fn test_tools_list_request() {
    let server = create_test_server();

    let request = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(2),
        method: "tools/list".to_string(),
        params: None,
    };

    let response = server.handle_request(request).await;

    assert_eq!(response.jsonrpc, "2.0");
    assert_eq!(response.id, Some(2));
    assert!(response.error.is_none());
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    let tools = result["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "echo");
}

#[tokio::test]
async fn test_tool_call_request() {
    let server = create_test_server();

    let request = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(3),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "echo",
            "arguments": {
                "message": "Hello, World!"
            }
        })),
    };

    let response = server.handle_request(request).await;

    assert_eq!(response.jsonrpc, "2.0");
    assert_eq!(response.id, Some(3));
    assert!(response.error.is_none());
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert_eq!(result["isError"], json!(false));

    // Parse the content
    let content = result["content"].as_array().unwrap();
    assert_eq!(content.len(), 1);
    assert_eq!(content[0]["type"], "text");

    // The text is the serialized ToolResult which has a data field
    let text = content[0]["text"].as_str().unwrap();
    let tool_result: serde_json::Value = serde_json::from_str(text).unwrap();

    // Verify the ToolResult structure
    assert_eq!(tool_result["status"], "success");

    // The actual data is in the "data" field
    let data = &tool_result["data"];
    assert_eq!(data["echoed"], "Hello, World!");
}

#[tokio::test]
async fn test_ping_request() {
    let server = create_test_server();

    let request = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(4),
        method: "ping".to_string(),
        params: None,
    };

    let response = server.handle_request(request).await;

    assert_eq!(response.jsonrpc, "2.0");
    assert_eq!(response.id, Some(4));
    assert!(response.error.is_none());
    assert!(response.result.is_some());
}

#[tokio::test]
async fn test_unknown_method_error() {
    let server = create_test_server();

    let request = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(5),
        method: "unknown/method".to_string(),
        params: None,
    };

    let response = server.handle_request(request).await;

    assert_eq!(response.jsonrpc, "2.0");
    assert_eq!(response.id, Some(5));
    assert!(response.error.is_some());
    assert!(response.result.is_none());

    let error = response.error.unwrap();
    assert_eq!(error.code, -32601); // Method not found
}

#[tokio::test]
async fn test_tool_not_found_error() {
    let server = create_test_server();

    let request = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(6),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "nonexistent_tool",
            "arguments": {}
        })),
    };

    let response = server.handle_request(request).await;

    assert_eq!(response.jsonrpc, "2.0");
    assert_eq!(response.id, Some(6));
    assert!(response.error.is_some());
    assert!(response.result.is_none());

    let error = response.error.unwrap();
    assert_eq!(error.code, -32602); // Invalid params (tool not found)
}

#[tokio::test]
async fn test_protocol_response_serialization() {
    // Verify that responses serialize to valid JSON-RPC format
    let response = McpResponse::success(Some(1), json!({"key": "value"}));
    let serialized = serde_json::to_string(&response).unwrap();

    assert!(serialized.contains("\"jsonrpc\":\"2.0\""));
    assert!(serialized.contains("\"id\":1"));
    assert!(serialized.contains("\"result\""));
    assert!(!serialized.contains("\"error\"")); // null fields are skipped
}

#[tokio::test]
async fn test_protocol_error_serialization() {
    let response = McpResponse::error(Some(1), -32600, "Invalid request");
    let serialized = serde_json::to_string(&response).unwrap();

    assert!(serialized.contains("\"jsonrpc\":\"2.0\""));
    assert!(serialized.contains("\"id\":1"));
    assert!(serialized.contains("\"error\""));
    assert!(serialized.contains("\"-32600\"") || serialized.contains("-32600"));
    assert!(!serialized.contains("\"result\"")); // null fields are skipped
}
