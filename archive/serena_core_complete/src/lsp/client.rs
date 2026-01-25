use anyhow::{Context, Result};
use lsp_types::{
    notification::Notification, request::Request, ClientCapabilities,
    DocumentSymbolClientCapabilities, InitializeParams, InitializeResult, InitializedParams,
    TextDocumentClientCapabilities, TraceValue, Url,
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
    process::Command,
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use tracing::{debug, error, warn};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Value,
    id: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonRpcNotification {
    jsonrpc: String,
    method: String,
    params: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<i64>,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonRpcError {
    code: i64,
    message: String,
    data: Option<Value>,
}

pub struct LspClient {
    child: Option<CommandChild>, // Option to allow taking it out if needed, or just for Drop
    request_id: AtomicI64,
    sender: mpsc::Sender<String>,
    pending_requests: Arc<dashmap::DashMap<i64, oneshot::Sender<Result<Value>>>>,
    _listener_task: JoinHandle<()>,
}

use tokio::process::Child as CommandChild;

impl Drop for LspClient {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();
        }
    }
}

impl LspClient {
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

        // Writer Task
        let mut writer_stdin = stdin;
        tokio::spawn(async move {
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

        // Stderr Logger Task
        let reader_stderr = BufReader::new(stderr);
        tokio::spawn(async move {
            let mut lines = reader_stderr.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                warn!("[LSP STDERR] {}", line);
            }
        });

        let pending_requests: Arc<dashmap::DashMap<i64, oneshot::Sender<Result<Value>>>> =
            Arc::new(dashmap::DashMap::new());
        let pending_requests_clone = pending_requests.clone();

        // Reader Task (Main Event Loop)
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
                        // It might be a notification or a request from the server (client-side)
                        // TODO: Handle server-to-client requests/notifications
                    }
                } else {
                    // Try parsing as notification
                    if let Ok(notif) = serde_json::from_str::<JsonRpcNotification>(&body_str) {
                        debug!("Received notification: {}", notif.method);
                        // TODO: Handle specific notifications (publishDiagnostics, etc.)
                    }
                }
            }
        });

        Ok(Self {
            child: Some(child),
            request_id: AtomicI64::new(1),
            sender: tx,
            pending_requests,
            _listener_task: listener_task,
        })
    }

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
            .context("Failed to send request channel")?;

        let response_value = rx.await.context("Sender dropped before response")??;

        let result = serde_json::from_value(response_value)?;
        Ok(result)
    }

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
            .context("Failed to send notification channel")?;
        Ok(())
    }

    pub async fn initialize(&self, root_uri: Url) -> Result<InitializeResult> {
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

        let params = InitializeParams {
            process_id: Some(std::process::id()),
            root_uri: Some(root_uri.clone()),
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
}
