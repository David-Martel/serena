use crate::protocol::{
    CallToolParams, CallToolResult, InitializeResult, McpRequest, McpResponse, ServerCapabilities,
    ServerInfo, ToolContent, ToolInfo, ToolsCapability,
};
use crate::transport::stdio::StdioTransport;
use anyhow::Result;
use serde_json::json;
use serena_core::ToolRegistry;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// Default timeout for tool execution (5 minutes)
const TOOL_EXECUTION_TIMEOUT: Duration = Duration::from_secs(300);

pub struct SerenaMcpServer {
    tools: Arc<ToolRegistry>,
}

impl SerenaMcpServer {
    pub fn new(tools: ToolRegistry) -> Self {
        Self {
            tools: Arc::new(tools),
        }
    }

    /// Returns a list of tool info for all registered tools
    ///
    /// Used by the dashboard API to display available tools
    pub fn list_tools(&self) -> Vec<ToolInfo> {
        self.tools
            .list_tools()
            .iter()
            .map(|tool| ToolInfo {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                input_schema: tool.parameters_schema(),
            })
            .collect()
    }

    /// Returns the number of registered tools
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    pub async fn serve_stdio(self) -> Result<()> {
        info!("Starting Serena MCP server on stdio");
        let transport = StdioTransport::new();

        loop {
            match transport.receive().await {
                Ok(Some(request)) => {
                    debug!("Received request: {:?}", request.method);
                    let response = self.handle_request(request).await;
                    transport.send(&response).await?;
                }
                Ok(None) => {
                    info!("Client disconnected");
                    break;
                }
                Err(e) => {
                    error!("Error receiving request: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    pub async fn handle_request(&self, request: McpRequest) -> McpResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request.id).await,
            "tools/list" => self.handle_list_tools(request.id).await,
            "tools/call" => {
                self.handle_call_tool(request.id, request.params.unwrap_or(json!({})))
                    .await
            }
            "ping" => McpResponse::success(request.id, json!({})),
            method => {
                warn!("Unknown method: {}", method);
                McpResponse::error(request.id, -32601, format!("Method not found: {}", method))
            }
        }
    }

    async fn handle_initialize(&self, id: Option<i64>) -> McpResponse {
        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: false,
                }),
            },
            server_info: ServerInfo {
                name: "serena-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        match serde_json::to_value(result) {
            Ok(value) => McpResponse::success(id, value),
            Err(e) => {
                error!("Failed to serialize initialize result: {}", e);
                McpResponse::error(id, -32603, "Internal error")
            }
        }
    }

    async fn handle_list_tools(&self, id: Option<i64>) -> McpResponse {
        let tools: Vec<ToolInfo> = self
            .tools
            .list_tools()
            .iter()
            .map(|tool| {
                let schema = tool.parameters_schema();
                ToolInfo {
                    name: tool.name().to_string(),
                    description: tool.description().to_string(),
                    input_schema: schema,
                }
            })
            .collect();

        McpResponse::success(id, json!({ "tools": tools }))
    }

    async fn handle_call_tool(&self, id: Option<i64>, params: serde_json::Value) -> McpResponse {
        let call_params: CallToolParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to parse call tool params: {}", e);
                return McpResponse::error(id, -32602, "Invalid params");
            }
        };

        debug!("Calling tool: {} (timeout: {:?})", call_params.name, TOOL_EXECUTION_TIMEOUT);

        match self.tools.get_tool(&call_params.name) {
            Some(tool) => {
                // Wrap tool execution with timeout to prevent hung tools
                let execution_result = timeout(
                    TOOL_EXECUTION_TIMEOUT,
                    tool.execute(call_params.arguments)
                ).await;

                match execution_result {
                    Ok(Ok(result)) => {
                        // Tool completed successfully within timeout
                        let result_str = match serde_json::to_string_pretty(&result) {
                            Ok(s) => s,
                            Err(e) => {
                                error!("Failed to serialize tool result: {}", e);
                                return McpResponse::error(id, -32603, "Internal error");
                            }
                        };

                        let call_result = CallToolResult {
                            content: vec![ToolContent::Text { text: result_str }],
                            is_error: Some(false),
                        };

                        match serde_json::to_value(call_result) {
                            Ok(value) => McpResponse::success(id, value),
                            Err(e) => {
                                error!("Failed to serialize call result: {}", e);
                                McpResponse::error(id, -32603, "Internal error")
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        // Tool execution failed
                        error!("Tool execution failed: {}", e);
                        let call_result = CallToolResult {
                            content: vec![ToolContent::Text {
                                text: format!("Error: {}", e),
                            }],
                            is_error: Some(true),
                        };

                        match serde_json::to_value(call_result) {
                            Ok(value) => McpResponse::success(id, value),
                            Err(e) => {
                                error!("Failed to serialize error result: {}", e);
                                McpResponse::error(id, -32603, "Internal error")
                            }
                        }
                    }
                    Err(_) => {
                        // Timeout elapsed
                        error!(
                            "Tool '{}' execution timed out after {:?}",
                            call_params.name, TOOL_EXECUTION_TIMEOUT
                        );
                        let call_result = CallToolResult {
                            content: vec![ToolContent::Text {
                                text: format!(
                                    "Error: Tool execution timed out after {} seconds",
                                    TOOL_EXECUTION_TIMEOUT.as_secs()
                                ),
                            }],
                            is_error: Some(true),
                        };

                        match serde_json::to_value(call_result) {
                            Ok(value) => McpResponse::success(id, value),
                            Err(e) => {
                                error!("Failed to serialize timeout result: {}", e);
                                McpResponse::error(id, -32603, "Internal error")
                            }
                        }
                    }
                }
            }
            None => {
                warn!("Tool not found: {}", call_params.name);
                McpResponse::error(id, -32602, format!("Tool not found: {}", call_params.name))
            }
        }
    }
}
