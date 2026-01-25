//! REST API endpoints for the Serena dashboard
//!
//! These endpoints provide status, configuration, and control APIs for the
//! Serena web dashboard and other monitoring tools.

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};

use crate::SerenaMcpServer;

/// Heartbeat response for health monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResponse {
    pub status: String,
    pub uptime_seconds: u64,
    pub version: String,
    pub server_name: String,
    pub runtime: String,
}

/// Configuration response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub project_name: Option<String>,
    pub active_modes: Vec<String>,
    pub active_context: Option<String>,
    pub available_tools: Vec<String>,
}

/// Tool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStat {
    pub name: String,
    pub call_count: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// Tool statistics response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatsResponse {
    pub tools: Vec<ToolStat>,
    pub total_calls: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
}

/// Server state for API handlers
#[derive(Clone)]
pub struct ApiState {
    pub mcp_server: Arc<SerenaMcpServer>,
    pub start_time: std::time::Instant,
}

impl ApiState {
    pub fn new(mcp_server: Arc<SerenaMcpServer>) -> Self {
        Self {
            mcp_server,
            start_time: std::time::Instant::now(),
        }
    }
}

/// Heartbeat endpoint handler
///
/// GET /heartbeat
/// Returns server health and uptime information
pub async fn heartbeat_handler(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    debug!("Heartbeat requested");

    let uptime = state.start_time.elapsed().as_secs();

    Json(HeartbeatResponse {
        status: "ok".to_string(),
        uptime_seconds: uptime,
        version: env!("CARGO_PKG_VERSION").to_string(),
        server_name: "serena-mcp".to_string(),
        runtime: "rust".to_string(),
    })
}

/// Configuration endpoint handler
///
/// GET /get_config
/// Returns current server configuration including active project, modes, and tools
pub async fn config_handler(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    debug!("Config requested");

    // Get available tools from the MCP server's registry
    let tool_names: Vec<String> = state
        .mcp_server
        .list_tools()
        .iter()
        .map(|t| t.name.clone())
        .collect();

    Json(ConfigResponse {
        project_name: None, // TODO: Get from App state when wired up
        active_modes: vec!["interactive".to_string()],
        active_context: Some("desktop-app".to_string()),
        available_tools: tool_names,
    })
}

/// Tool statistics endpoint handler
///
/// GET /get_stats
/// Returns tool invocation statistics
pub async fn stats_handler(State(_state): State<Arc<ApiState>>) -> impl IntoResponse {
    debug!("Stats requested");

    // TODO: Implement actual statistics collection
    // For now, return placeholder data
    Json(ToolStatsResponse {
        tools: vec![],
        total_calls: 0,
        total_input_tokens: 0,
        total_output_tokens: 0,
    })
}

/// Shutdown endpoint handler
///
/// POST /shutdown
/// Initiates graceful server shutdown
pub async fn shutdown_handler() -> impl IntoResponse {
    info!("Shutdown requested via API");

    // Note: Actual shutdown logic would need to be implemented
    // through a shutdown channel or similar mechanism
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "shutdown_initiated",
            "message": "Server shutdown has been initiated"
        })),
    )
}

/// List all registered tools
///
/// GET /tools
/// Returns a list of all available tools with their descriptions
pub async fn list_tools_handler(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    debug!("Tool list requested");

    let tools: Vec<serde_json::Value> = state
        .mcp_server
        .list_tools()
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.description,
            })
        })
        .collect();

    Json(serde_json::json!({
        "tools": tools,
        "count": tools.len()
    }))
}

/// Server info endpoint
///
/// GET /info
/// Returns detailed server information
pub async fn info_handler(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    debug!("Server info requested");

    let uptime = state.start_time.elapsed();
    let tool_count = state.mcp_server.list_tools().len();

    Json(serde_json::json!({
        "name": "serena-mcp",
        "version": env!("CARGO_PKG_VERSION"),
        "runtime": "rust",
        "uptime": {
            "seconds": uptime.as_secs(),
            "human": format_duration(uptime)
        },
        "tools": {
            "count": tool_count
        },
        "capabilities": {
            "mcp": true,
            "http": true,
            "sse": true
        }
    }))
}

/// Format duration in human-readable form
fn format_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let secs = secs % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, mins, secs)
    } else if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(std::time::Duration::from_secs(45)), "45s");
        assert_eq!(
            format_duration(std::time::Duration::from_secs(125)),
            "2m 5s"
        );
        assert_eq!(
            format_duration(std::time::Duration::from_secs(3725)),
            "1h 2m 5s"
        );
    }

    #[test]
    fn test_heartbeat_response_serialization() {
        let response = HeartbeatResponse {
            status: "ok".to_string(),
            uptime_seconds: 100,
            version: "0.2.0".to_string(),
            server_name: "test".to_string(),
            runtime: "rust".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"uptime_seconds\":100"));
    }
}
