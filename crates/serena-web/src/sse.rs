//! Server-Sent Events (SSE) transport for MCP
//!
//! Provides a unidirectional server-to-client communication channel using SSE,
//! allowing the server to push updates and responses to connected clients.

use crate::McpResponse;
use anyhow::{Context, Result};
use axum::{
    response::{sse::Event, Sse},
    Extension,
};
use futures::stream::Stream;
use std::convert::Infallible;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, trace};

/// SSE transport for sending MCP responses to clients
#[derive(Clone)]
pub struct SseTransport {
    tx: mpsc::UnboundedSender<McpResponse>,
}

impl SseTransport {
    /// Create a new SSE transport channel
    ///
    /// Returns a tuple of (SseTransport, receiver stream)
    pub fn new() -> (Self, SseStream) {
        let (tx, rx) = mpsc::unbounded_channel();
        let stream = SseStream { rx };
        (Self { tx }, stream)
    }

    /// Send an MCP response through the SSE channel
    pub fn send(&self, response: McpResponse) -> Result<()> {
        self.tx
            .send(response)
            .context("Failed to send response through SSE channel")?;
        trace!("Sent response through SSE");
        Ok(())
    }

    /// Check if the channel is closed
    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }
}

impl Default for SseTransport {
    fn default() -> Self {
        Self::new().0
    }
}

/// Stream for SSE events containing MCP responses
pub struct SseStream {
    rx: mpsc::UnboundedReceiver<McpResponse>,
}

type SseResponseStream = Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>>;

impl SseStream {
    /// Convert this stream into an Axum SSE response
    pub fn into_sse_response(mut self) -> Sse<SseResponseStream> {
        let stream = async_stream::stream! {
            while let Some(response) = self.rx.recv().await {
                match serde_json::to_string(&response) {
                    Ok(json) => {
                        debug!("SSE sending response: id={:?}", response.id);
                        yield Ok(Event::default().data(json));
                    }
                    Err(e) => {
                        error!("Failed to serialize response: {}", e);
                    }
                }
            }
            debug!("SSE stream ended");
        };

        Sse::new(Box::pin(stream))
    }
}

/// Handler for SSE endpoint
///
/// This creates a long-lived connection that pushes MCP responses to the client.
pub async fn sse_handler(
    Extension(_transport): Extension<Arc<SseTransport>>,
) -> Sse<Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>>> {
    debug!("New SSE connection established");
    let (_new_transport, stream) = SseTransport::new();

    // Note: In a production implementation, you would want to:
    // 1. Store the transport in a connection registry
    // 2. Associate it with a session ID
    // 3. Clean up when the connection closes

    stream.into_sse_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::McpResponse;

    #[tokio::test]
    async fn test_sse_transport_send() {
        let (transport, mut stream) = SseTransport::new();

        let response = McpResponse::success(Some(1), serde_json::json!({"test": "data"}));
        transport.send(response.clone()).unwrap();

        // Receive the response
        let received = stream.rx.recv().await;
        assert!(received.is_some());
        let received = received.unwrap();
        assert_eq!(received.id, Some(1));
    }

    #[test]
    fn test_sse_transport_closed() {
        let (transport, stream) = SseTransport::new();
        assert!(!transport.is_closed());

        // Drop the receiver
        drop(stream);

        // Channel should be closed
        assert!(transport.is_closed());
    }
}
