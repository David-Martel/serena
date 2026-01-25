//! API Client for Serena Backend
//!
//! Handles all HTTP communication with the Serena backend server.

use crate::HeartbeatResponse;
use gloo_net::http::Request;

/// Fetch heartbeat from the Serena backend
pub async fn fetch_heartbeat() -> Result<HeartbeatResponse, gloo_net::Error> {
    let response = Request::get("/heartbeat")
        .header("Accept", "application/json")
        .send()
        .await?;

    let heartbeat: HeartbeatResponse = response.json().await?;
    Ok(heartbeat)
}

/// Configuration response from the backend
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ConfigResponse {
    pub project_name: Option<String>,
    pub active_modes: Vec<String>,
    pub active_context: Option<String>,
    pub available_tools: Vec<String>,
}

/// Fetch current configuration
pub async fn fetch_config() -> Result<ConfigResponse, gloo_net::Error> {
    let response = Request::get("/get_config")
        .header("Accept", "application/json")
        .send()
        .await?;

    let config: ConfigResponse = response.json().await?;
    Ok(config)
}

/// Tool statistics from the backend
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ToolStatsResponse {
    pub tools: Vec<ToolStat>,
    pub total_calls: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ToolStat {
    pub name: String,
    pub call_count: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// Fetch tool statistics
pub async fn fetch_stats() -> Result<ToolStatsResponse, gloo_net::Error> {
    let response = Request::get("/get_stats")
        .header("Accept", "application/json")
        .send()
        .await?;

    let stats: ToolStatsResponse = response.json().await?;
    Ok(stats)
}

/// Shutdown the server
pub async fn shutdown_server() -> Result<(), gloo_net::Error> {
    Request::post("/shutdown")
        .header("Accept", "application/json")
        .send()
        .await?;
    Ok(())
}
