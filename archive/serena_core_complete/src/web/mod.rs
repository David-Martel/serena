use axum::{
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

pub async fn start_server(port: u16, dashboard_path: String) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/heartbeat", get(heartbeat))
        .route("/get_log_messages", post(get_log_messages))
        .nest_service("/dashboard", ServeDir::new(dashboard_path));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("Listening on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn heartbeat() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "agent": "serena-rust",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn get_log_messages(Json(_payload): Json<Value>) -> Json<Value> {
    // TODO: Connect to actual log store
    Json(json!({
        "messages": [],
        "last_index": 0
    }))
}
