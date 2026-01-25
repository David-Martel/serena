//! Generic LSP client implementation
//!
//! Provides a robust LSP client that handles JSON-RPC communication with language servers
//! over stdio. Based on the existing serena_core LSP client implementation.

use anyhow::{Context, Result};
use dashmap::DashMap;
use lsp_types::{
    notification::Notification, request::Request, ClientCapabilities,
    DocumentSymbolClientCapabilities, InitializeParams, InitializeResult, InitializedParams,
    TextDocumentClientCapabilities, TraceValue, Uri, WorkspaceFolder,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    process::Stdio,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc,
    },
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use tracing::{debug, error, warn};

/// JSON-RPC request message
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Value,
    id: i64,
}

/// JSON-RPC notification message (no response expected)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonRpcNotification {
    jsonrpc: String,
    method: String,
    params: Value,
}

/// JSON-RPC response message
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<i64>,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

/// JSON-RPC error object
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

/// LSP client that communicates with a language server via stdio
pub struct LspClient {
    child: Option<Child>,
    request_id: AtomicI64,
    sender: mpsc::Sender<String>,
    pending_requests: Arc<DashMap<i64, oneshot::Sender<Result<Value>>>>,
    listener_task: JoinHandle<()>,
    writer_task: JoinHandle<()>,
    stderr_task: JoinHandle<()>,
}

impl Drop for LspClient {
    fn drop(&mut self) {
        // Abort all background tasks to prevent zombies
        self.writer_task.abort();
        self.stderr_task.abort();
        self.listener_task.abort();

        // Kill child process if still running
        if let Some(mut child) = self.child.take() {
            if let Err(e) = child.start_kill() {
                error!("Failed to kill LSP child process: {}", e);
            }
        }
    }
}

impl LspClient {
    /// Create a new LSP client by spawning a language server process
    ///
    /// # Arguments
    /// * `command` - The command to execute (e.g., "rust-analyzer")
    /// * `args` - Command-line arguments for the language server
    ///
    /// # Returns
    /// A new LSP client instance that's ready to communicate
    pub async fn new(command: String, args: Vec<String>) -> Result<Self> {
        let mut child = Command::new(&command)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn language server")?;

        let stdin = child.stdin.take().context("Failed to open stdin")?;
        let stdout = child.stdout.take().context("Failed to open stdout")?;
        let stderr = child.stderr.take().context("Failed to open stderr")?;

        // Channel to send raw strings to stdin writer task
        let (tx, mut rx) = mpsc::channel::<String>(32);

        // Writer Task - sends messages to language server stdin
        let mut writer_stdin = stdin;
        let writer_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let content_length = msg.len();
                let full_msg = format!("Content-Length: {}\r\n\r\n{}", content_length, msg);
                if let Err(e) = writer_stdin.write_all(full_msg.as_bytes()).await {
                    error!("Failed to write to LSP stdin: {}", e);
                    break;
                }
                if let Err(e) = writer_stdin.flush().await {
                    error!("Failed to flush LSP stdin: {}", e);
                    break;
                }
            }
        });

        // Stderr Logger Task - logs language server stderr output
        let reader_stderr = BufReader::new(stderr);
        let stderr_task = tokio::spawn(async move {
            let mut lines = reader_stderr.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                warn!("[LSP STDERR] {}", line);
            }
        });

        let pending_requests: Arc<DashMap<i64, oneshot::Sender<Result<Value>>>> =
            Arc::new(DashMap::new());
        let pending_requests_clone = pending_requests.clone();

        // Reader Task - receives messages from language server stdout
        let listener_task = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            loop {
                // Read headers
                let mut content_length = 0;
                loop {
                    let mut line = String::new();
                    match reader.read_line(&mut line).await {
                        Ok(0) => return, // EOF
                        Ok(_) => {
                            if line.trim().is_empty() {
                                // End of headers
                                break;
                            }
                            if line.starts_with("Content-Length: ") {
                                if let Ok(len) =
                                    line.trim()["Content-Length: ".len()..].parse::<usize>()
                                {
                                    content_length = len;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Error reading LSP header: {}", e);
                            return;
                        }
                    }
                }

                if content_length == 0 {
                    continue;
                }

                // Read body
                let mut body_buf = vec![0u8; content_length];
                if let Err(e) = reader.read_exact(&mut body_buf).await {
                    error!("Error reading LSP body: {}", e);
                    return;
                }

                let body_str = String::from_utf8_lossy(&body_buf);
                debug!("Received LSP message: {}", body_str);

                // Parse JSON
                if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(&body_str) {
                    if let Some(id) = resp.id {
                        if let Some((_, tx)) = pending_requests_clone.remove(&id) {
                            if let Some(error) = resp.error {
                                let _ = tx.send(Err(anyhow::anyhow!(
                                    "LSP Error {}: {}",
                                    error.code,
                                    error.message
                                )));
                            } else {
                                let _ = tx.send(Ok(resp.result.unwrap_or(Value::Null)));
                            }
                        }
                    } else {
                        // Server-to-client notification or request
                        debug!("Received server notification/request");
                    }
                } else if let Ok(notif) = serde_json::from_str::<JsonRpcNotification>(&body_str) {
                    debug!("Received notification: {}", notif.method);
                    // TODO: Handle specific notifications (publishDiagnostics, etc.)
                }
            }
        });

        Ok(Self {
            child: Some(child),
            request_id: AtomicI64::new(1),
            sender: tx,
            pending_requests,
            listener_task,
            writer_task,
            stderr_task,
        })
    }

    /// Send a typed LSP request and wait for the response
    ///
    /// # Type Parameters
    /// * `R` - The LSP request type (e.g., `GotoDefinition`, `DocumentSymbol`)
    ///
    /// # Arguments
    /// * `params` - Request parameters
    ///
    /// # Returns
    /// The typed response for the request
    pub async fn send_request<R>(&self, params: R::Params) -> Result<R::Result>
    where
        R: Request,
    {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        let params_json = serde_json::to_value(params)?;

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: R::METHOD.to_string(),
            params: params_json,
            id,
        };

        let req_str = serde_json::to_string(&req)?;

        let (tx, rx) = oneshot::channel();
        self.pending_requests.insert(id, tx);

        self.sender
            .send(req_str)
            .await
            .context("Failed to send request")?;

        let response_value = rx.await.context("Sender dropped before response")??;

        let result = serde_json::from_value(response_value)?;
        Ok(result)
    }

    /// Send a typed LSP notification (no response expected)
    ///
    /// # Type Parameters
    /// * `N` - The LSP notification type (e.g., `DidOpenTextDocument`)
    ///
    /// # Arguments
    /// * `params` - Notification parameters
    pub async fn send_notification<N>(&self, params: N::Params) -> Result<()>
    where
        N: Notification,
    {
        let params_json = serde_json::to_value(params)?;
        let notif = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: N::METHOD.to_string(),
            params: params_json,
        };
        let notif_str = serde_json::to_string(&notif)?;
        self.sender
            .send(notif_str)
            .await
            .context("Failed to send notification")?;
        Ok(())
    }

    /// Initialize the language server
    ///
    /// This must be called before sending any other requests. It sends the
    /// `initialize` request followed by the `initialized` notification.
    ///
    /// # Arguments
    /// * `root_uri` - The root URI of the workspace
    ///
    /// # Returns
    /// The server's initialization result containing capabilities
    pub async fn initialize(&self, root_uri: Uri) -> Result<InitializeResult> {
        let capabilities = ClientCapabilities {
            text_document: Some(TextDocumentClientCapabilities {
                document_symbol: Some(DocumentSymbolClientCapabilities {
                    hierarchical_document_symbol_support: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        // Use workspace_folders (modern) while maintaining root_uri for backward compatibility
        let workspace_folder = WorkspaceFolder {
            uri: root_uri.clone(),
            name: root_uri
                .path()
                .segments()
                .next_back()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "workspace".to_string()),
        };

        #[allow(deprecated)]
        let params = InitializeParams {
            process_id: Some(std::process::id()),
            // root_uri is deprecated but kept for compatibility with older LSP servers
            root_uri: Some(root_uri.clone()),
            // workspace_folders is the modern approach (LSP 3.6+)
            workspace_folders: Some(vec![workspace_folder]),
            capabilities,
            trace: Some(TraceValue::Off),
            ..Default::default()
        };

        let result = self
            .send_request::<lsp_types::request::Initialize>(params)
            .await?;

        self.send_notification::<lsp_types::notification::Initialized>(InitializedParams {})
            .await?;

        Ok(result)
    }

    /// Shutdown the language server gracefully
    ///
    /// Sends the `shutdown` request followed by the `exit` notification,
    /// then waits for all background tasks to complete.
    pub async fn shutdown(&mut self) -> Result<()> {
        // Send shutdown request
        let _: () = self
            .send_request::<lsp_types::request::Shutdown>(())
            .await?;

        // Send exit notification
        self.send_notification::<lsp_types::notification::Exit>(())
            .await?;

        // Kill the child process - this will cause stdin/stdout/stderr to close
        // which will cause the tasks to exit gracefully
        if let Some(mut child) = self.child.take() {
            child
                .kill()
                .await
                .context("Failed to kill LSP child process")?;
        }

        // Wait for tasks to complete gracefully (with timeout)
        let shutdown_timeout = tokio::time::Duration::from_secs(5);

        // Wait for writer task
        match tokio::time::timeout(shutdown_timeout, async {
            // Abort and ignore the result - we just want it to stop
            self.writer_task.abort();
            let _ = (&mut self.writer_task).await;
        })
        .await
        {
            Ok(_) => debug!("Writer task stopped gracefully"),
            Err(_) => warn!("Writer task shutdown timed out"),
        }

        // Wait for stderr task
        match tokio::time::timeout(shutdown_timeout, async {
            self.stderr_task.abort();
            let _ = (&mut self.stderr_task).await;
        })
        .await
        {
            Ok(_) => debug!("Stderr task stopped gracefully"),
            Err(_) => warn!("Stderr task shutdown timed out"),
        }

        // Wait for listener task
        match tokio::time::timeout(shutdown_timeout, async {
            self.listener_task.abort();
            let _ = (&mut self.listener_task).await;
        })
        .await
        {
            Ok(_) => debug!("Listener task stopped gracefully"),
            Err(_) => warn!("Listener task shutdown timed out"),
        }

        Ok(())
    }
}

// Implementation of LanguageServer trait for LspClient
use async_trait::async_trait;
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DocumentSymbolParams, DocumentSymbolResponse, GotoDefinitionParams, GotoDefinitionResponse,
    ReferenceContext, ReferenceParams, RenameParams, ServerCapabilities,
    TextDocumentContentChangeEvent, TextDocumentItem, VersionedTextDocumentIdentifier,
    WorkspaceEdit,
};
use serena_core::{LanguageServer, Location as CoreLocation, LspError, SymbolInfo};
use std::sync::atomic::AtomicBool;

/// LspClient extended with LanguageServer trait support
pub struct LspClientAdapter {
    client: LspClient,
    language_id: String,
    initialized: AtomicBool,
}

impl LspClientAdapter {
    /// Create a new adapter wrapping an LspClient
    pub fn new(client: LspClient, language_id: String) -> Self {
        Self {
            client,
            language_id,
            initialized: AtomicBool::new(false),
        }
    }

    /// Convert lsp_types::DocumentSymbolResponse to Vec<SymbolInfo>
    fn convert_symbols(response: DocumentSymbolResponse) -> Vec<SymbolInfo> {
        match response {
            DocumentSymbolResponse::Flat(symbols) => symbols
                .into_iter()
                .map(|s| SymbolInfo {
                    name: s.name,
                    kind: s.kind.into(), // Use From trait
                    location: s.location.into(), // Use From trait
                    detail: None,
                    children: vec![],
                    container_name: s.container_name,
                })
                .collect(),
            DocumentSymbolResponse::Nested(symbols) => Self::convert_nested_symbols(symbols, None),
        }
    }

    fn convert_nested_symbols(
        symbols: Vec<lsp_types::DocumentSymbol>,
        parent_uri: Option<&str>,
    ) -> Vec<SymbolInfo> {
        symbols
            .into_iter()
            .map(|s| {
                // For nested symbols, we don't have a full location, create a placeholder
                let location = CoreLocation {
                    uri: parent_uri.unwrap_or("file:///unknown").to_string(),
                    range: s.selection_range.into(),
                };
                SymbolInfo {
                    name: s.name,
                    kind: s.kind.into(),
                    location,
                    detail: s.detail,
                    children: s
                        .children
                        .map(|c| Self::convert_nested_symbols(c, parent_uri))
                        .unwrap_or_default(),
                    container_name: None,
                }
            })
            .collect()
    }

    /// Parse a URI string to lsp_types::Uri
    fn parse_uri(uri: &str) -> Result<Uri, LspError> {
        use std::str::FromStr;
        Uri::from_str(uri).map_err(|e| LspError::InvalidUri(e.to_string()))
    }
}

#[async_trait]
impl LanguageServer for LspClientAdapter {
    async fn initialize(
        &mut self,
        params: lsp_types::InitializeParams,
    ) -> Result<ServerCapabilities, LspError> {
        #[allow(deprecated)] // root_uri is deprecated but still widely used
        let root_uri = params
            .root_uri
            .ok_or_else(|| LspError::InitializationError("No root URI provided".into()))?;

        let result = self
            .client
            .initialize(root_uri)
            .await
            .map_err(|e| LspError::InitializationError(e.to_string()))?;

        self.initialized
            .store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(result.capabilities)
    }

    async fn shutdown(&mut self) -> Result<(), LspError> {
        self.client
            .shutdown()
            .await
            .map_err(|e| LspError::ShutdownError(e.to_string()))?;
        self.initialized
            .store(false, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    async fn document_symbols(
        &self,
        document: lsp_types::TextDocumentIdentifier,
    ) -> Result<Vec<SymbolInfo>, LspError> {
        let params = DocumentSymbolParams {
            text_document: document,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result: Option<DocumentSymbolResponse> = self
            .client
            .send_request::<lsp_types::request::DocumentSymbolRequest>(params)
            .await
            .map_err(|e| LspError::RequestFailed(e.to_string()))?;

        match result {
            Some(response) => Ok(Self::convert_symbols(response)),
            None => Ok(vec![]),
        }
    }

    async fn find_references(
        &self,
        params: lsp_types::TextDocumentPositionParams,
    ) -> Result<Vec<lsp_types::Location>, LspError> {
        let ref_params = ReferenceParams {
            text_document_position: params,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: ReferenceContext {
                include_declaration: true,
            },
        };

        let result: Option<Vec<lsp_types::Location>> = self
            .client
            .send_request::<lsp_types::request::References>(ref_params)
            .await
            .map_err(|e| LspError::RequestFailed(e.to_string()))?;

        Ok(result.unwrap_or_default())
    }

    async fn rename(&self, params: RenameParams) -> Result<WorkspaceEdit, LspError> {
        let result: Option<WorkspaceEdit> = self
            .client
            .send_request::<lsp_types::request::Rename>(params)
            .await
            .map_err(|e| LspError::RequestFailed(e.to_string()))?;

        Ok(result.unwrap_or_default())
    }

    async fn goto_definition(
        &self,
        params: lsp_types::TextDocumentPositionParams,
    ) -> Result<GotoDefinitionResponse, LspError> {
        let goto_params = GotoDefinitionParams {
            text_document_position_params: params,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result: Option<GotoDefinitionResponse> = self
            .client
            .send_request::<lsp_types::request::GotoDefinition>(goto_params)
            .await
            .map_err(|e| LspError::RequestFailed(e.to_string()))?;

        Ok(result.unwrap_or(GotoDefinitionResponse::Array(vec![])))
    }

    async fn did_open(
        &self,
        uri: String,
        language_id: String,
        text: String,
    ) -> Result<(), LspError> {
        let lsp_uri = Self::parse_uri(&uri)?;

        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: lsp_uri,
                language_id,
                version: 1,
                text,
            },
        };

        self.client
            .send_notification::<lsp_types::notification::DidOpenTextDocument>(params)
            .await
            .map_err(|e| LspError::NotificationFailed(e.to_string()))?;

        Ok(())
    }

    async fn did_close(&self, uri: String) -> Result<(), LspError> {
        let lsp_uri = Self::parse_uri(&uri)?;

        let params = DidCloseTextDocumentParams {
            text_document: lsp_types::TextDocumentIdentifier { uri: lsp_uri },
        };

        self.client
            .send_notification::<lsp_types::notification::DidCloseTextDocument>(params)
            .await
            .map_err(|e| LspError::NotificationFailed(e.to_string()))?;

        Ok(())
    }

    async fn did_change(&self, uri: String, text: String) -> Result<(), LspError> {
        let lsp_uri = Self::parse_uri(&uri)?;

        let params = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: lsp_uri,
                version: 1, // Version tracking would need to be improved
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text,
            }],
        };

        self.client
            .send_notification::<lsp_types::notification::DidChangeTextDocument>(params)
            .await
            .map_err(|e| LspError::NotificationFailed(e.to_string()))?;

        Ok(())
    }

    fn language_id(&self) -> &str {
        &self.language_id
    }

    fn is_initialized(&self) -> bool {
        self.initialized.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn is_running(&self) -> bool {
        self.client.child.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsonrpc_request_serialization() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "initialize".to_string(),
            params: serde_json::json!({}),
            id: 1,
        };

        let serialized = serde_json::to_string(&req).unwrap();
        assert!(serialized.contains("\"method\":\"initialize\""));
        assert!(serialized.contains("\"id\":1"));
    }

    #[test]
    fn test_jsonrpc_response_deserialization() {
        // Test with non-null result
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"success":true}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, Some(1));
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());

        // Test with null result
        let json_null = r#"{"jsonrpc":"2.0","id":2,"result":null}"#;
        let resp_null: JsonRpcResponse = serde_json::from_str(json_null).unwrap();
        assert_eq!(resp_null.id, Some(2));
        // Note: serde deserializes null as None for Option<Value>
        assert!(resp_null.result.is_none() || resp_null.result == Some(Value::Null));
        assert!(resp_null.error.is_none());
    }
}
