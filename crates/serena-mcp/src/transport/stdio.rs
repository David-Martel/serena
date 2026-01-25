use crate::protocol::{McpRequest, McpResponse};
use anyhow::{Context, Result};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::time::timeout;
use tracing::{debug, trace, warn};

/// Configuration for the stdio transport
#[derive(Debug, Clone)]
pub struct StdioTransportConfig {
    /// Read timeout for receiving messages (prevents infinite hangs)
    pub read_timeout: Duration,
    /// Maximum allowed message size in bytes (prevents DoS)
    pub max_message_size: usize,
}

impl Default for StdioTransportConfig {
    fn default() -> Self {
        Self {
            read_timeout: Duration::from_secs(300), // 5 minutes
            max_message_size: 10 * 1024 * 1024,      // 10 MB
        }
    }
}

pub struct StdioTransport {
    stdin: tokio::sync::Mutex<BufReader<tokio::io::Stdin>>,
    stdout: tokio::sync::Mutex<tokio::io::Stdout>,
    #[allow(dead_code)]
    config: StdioTransportConfig,
}

impl StdioTransport {
    pub fn new() -> Self {
        Self::with_config(StdioTransportConfig::default())
    }

    pub fn with_config(config: StdioTransportConfig) -> Self {
        Self {
            stdin: tokio::sync::Mutex::new(BufReader::new(tokio::io::stdin())),
            stdout: tokio::sync::Mutex::new(tokio::io::stdout()),
            config,
        }
    }

    pub async fn receive(&self) -> Result<Option<McpRequest>> {
        let mut stdin = self.stdin.lock().await;

        // Read headers (LSP-style with Content-Length) with timeout
        let mut content_length: Option<usize> = None;
        let mut line = String::new();

        loop {
            line.clear();

            // Apply timeout to header reading to prevent infinite hangs
            let bytes_read = match timeout(self.config.read_timeout, stdin.read_line(&mut line)).await {
                Ok(Ok(n)) => n,
                Ok(Err(e)) => {
                    return Err(e).context("Failed to read header line");
                }
                Err(_) => {
                    warn!("Read timeout after {:?} while reading headers - client may have disconnected", self.config.read_timeout);
                    anyhow::bail!("Read timeout: client may have crashed or disconnected");
                }
            };

            if bytes_read == 0 {
                // EOF - client disconnected gracefully
                debug!("Client disconnected (EOF on stdin)");
                return Ok(None);
            }

            let trimmed = line.trim();

            if trimmed.is_empty() {
                // Empty line indicates end of headers
                break;
            }

            if let Some(length_str) = trimmed.strip_prefix("Content-Length:") {
                content_length = Some(
                    length_str
                        .trim()
                        .parse()
                        .context("Invalid Content-Length header")?,
                );
            }
        }

        let content_length = content_length.context("Missing Content-Length header")?;

        // Validate Content-Length to prevent DoS attacks
        if content_length == 0 {
            anyhow::bail!("Content-Length cannot be zero");
        }

        if content_length > self.config.max_message_size {
            anyhow::bail!(
                "Content-Length {} exceeds maximum allowed size of {} bytes",
                content_length,
                self.config.max_message_size
            );
        }

        trace!("Reading message with Content-Length: {}", content_length);

        // Read the JSON content with timeout
        let mut buffer = vec![0u8; content_length];

        match timeout(self.config.read_timeout, stdin.read_exact(&mut buffer)).await {
            Ok(Ok(_)) => {},
            Ok(Err(e)) => {
                return Err(e).context("Failed to read message body");
            }
            Err(_) => {
                warn!("Read timeout after {:?} while reading message body - client may have disconnected", self.config.read_timeout);
                anyhow::bail!("Read timeout while reading message body: client may have crashed or disconnected");
            }
        }

        let content = String::from_utf8(buffer).context("Message body is not valid UTF-8")?;

        trace!("Received message: {}", content);

        let request: McpRequest =
            serde_json::from_str(&content).context("Failed to parse JSON-RPC request")?;

        debug!("Parsed request: method={}", request.method);

        Ok(Some(request))
    }

    pub async fn send(&self, response: &McpResponse) -> Result<()> {
        let json = serde_json::to_string(response).context("Failed to serialize response")?;

        trace!("Sending response: {}", json);

        let content_length = json.len();
        let message = format!("Content-Length: {}\r\n\r\n{}", content_length, json);

        let mut stdout = self.stdout.lock().await;

        // Apply timeout to write operations as well
        match timeout(self.config.read_timeout, stdout.write_all(message.as_bytes())).await {
            Ok(Ok(_)) => {},
            Ok(Err(e)) => {
                return Err(e).context("Failed to write response");
            }
            Err(_) => {
                warn!("Write timeout after {:?} - client may have disconnected", self.config.read_timeout);
                anyhow::bail!("Write timeout: client may have crashed or disconnected");
            }
        }

        match timeout(self.config.read_timeout, stdout.flush()).await {
            Ok(Ok(_)) => {},
            Ok(Err(e)) => {
                return Err(e).context("Failed to flush stdout");
            }
            Err(_) => {
                warn!("Flush timeout after {:?} - client may have disconnected", self.config.read_timeout);
                anyhow::bail!("Flush timeout: client may have crashed or disconnected");
            }
        }

        debug!("Sent response: id={:?}", response.id);

        Ok(())
    }

    /// Get the current configuration
    pub fn config(&self) -> &StdioTransportConfig {
        &self.config
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::McpRequest;

    #[test]
    fn test_default_config() {
        let config = StdioTransportConfig::default();
        assert_eq!(config.read_timeout, Duration::from_secs(300));
        assert_eq!(config.max_message_size, 10 * 1024 * 1024);
    }

    #[test]
    fn test_custom_config() {
        let config = StdioTransportConfig {
            read_timeout: Duration::from_secs(60),
            max_message_size: 1024 * 1024,
        };

        let transport = StdioTransport::with_config(config.clone());
        assert_eq!(transport.config().read_timeout, Duration::from_secs(60));
        assert_eq!(transport.config().max_message_size, 1024 * 1024);
    }

    #[tokio::test]
    async fn test_parse_request() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let request: McpRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, Some(1));
        assert_eq!(request.method, "initialize");
    }

    #[tokio::test]
    async fn test_serialize_response() {
        let response = McpResponse::success(Some(1), serde_json::json!({"status": "ok"}));
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"result\""));
    }

    #[test]
    fn test_transport_default() {
        let transport = StdioTransport::default();
        assert_eq!(transport.config().read_timeout, Duration::from_secs(300));
        assert_eq!(transport.config().max_message_size, 10 * 1024 * 1024);
    }

    // Note: Integration tests for timeout behavior should be in integration tests
    // as they require mocking stdin/stdout or creating actual processes
}
