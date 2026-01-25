# Rust Migration Patterns: Python → Rust Code Examples

This document provides concrete code examples for migrating common Python patterns in the Serena codebase to Rust.

---

## Table of Contents
1. [Tool System Migration](#1-tool-system-migration)
2. [Configuration & Serialization](#2-configuration--serialization)
3. [LSP Client Implementation](#3-lsp-client-implementation)
4. [MCP Server Implementation](#4-mcp-server-implementation)
5. [File Operations](#5-file-operations)
6. [Error Handling Patterns](#6-error-handling-patterns)
7. [Async/Concurrency Patterns](#7-asyncconcurrency-patterns)
8. [Testing Patterns](#8-testing-patterns)

---

## 1. Tool System Migration

### Python: Dynamic Tool Registration

```python
# src/serena/tools/tools_base.py
class Tool(Component):
    @classmethod
    def get_name_from_cls(cls) -> str:
        name = cls.__name__
        if name.endswith("Tool"):
            name = name[:-4]
        return "".join(["_" + c.lower() if c.isupper() else c for c in name]).lstrip("_")

    def apply_ex(self, **kwargs) -> str:
        apply_fn = getattr(self, "apply")
        result = apply_fn(**kwargs)
        return result

@singleton
class ToolRegistry:
    def __init__(self):
        self._tool_dict = {}
        for cls in iter_subclasses(Tool):
            if not cls.__module__.startswith("serena.tools"):
                continue
            name = cls.get_name_from_cls()
            self._tool_dict[name] = cls
```

### Rust: Static Tool Registration with Inventory

```rust
// src/tools/base.rs
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::fmt::Debug;

/// Base trait for all tools
pub trait Tool: Send + Sync + Debug {
    /// Get the tool's name
    fn name(&self) -> &'static str;

    /// Get the tool's description
    fn description(&self) -> &'static str;

    /// Apply the tool with JSON input
    fn apply_json(&self, input: serde_json::Value) -> Result<String>;

    /// Whether this tool can edit files
    fn can_edit(&self) -> bool {
        false
    }

    /// Whether this tool requires an active project
    fn requires_project(&self) -> bool {
        true
    }
}

/// Tool descriptor for static registration
pub struct ToolDescriptor {
    pub name: &'static str,
    pub description: &'static str,
    pub can_edit: bool,
    pub requires_project: bool,
    pub create: fn() -> Box<dyn Tool>,
}

// Collect all registered tools
inventory::collect!(ToolDescriptor);

/// Tool registry (singleton)
pub struct ToolRegistry;

impl ToolRegistry {
    pub fn all_tools() -> impl Iterator<Item = &'static ToolDescriptor> {
        inventory::iter::<ToolDescriptor>
    }

    pub fn get_tool(name: &str) -> Option<Box<dyn Tool>> {
        Self::all_tools()
            .find(|desc| desc.name == name)
            .map(|desc| (desc.create)())
    }

    pub fn tool_names() -> Vec<&'static str> {
        Self::all_tools().map(|desc| desc.name).collect()
    }
}

/// Macro to register a tool
#[macro_export]
macro_rules! register_tool {
    ($tool:ty) => {
        inventory::submit! {
            $crate::tools::ToolDescriptor {
                name: <$tool>::NAME,
                description: <$tool>::DESCRIPTION,
                can_edit: <$tool>::CAN_EDIT,
                requires_project: <$tool>::REQUIRES_PROJECT,
                create: || Box::new(<$tool>::default()),
            }
        }
    };
}

// Example tool implementation
// src/tools/file.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Default)]
pub struct ReadFileTool;

impl ReadFileTool {
    pub const NAME: &'static str = "read_file";
    pub const DESCRIPTION: &'static str = "Reads a file from the project directory";
    pub const CAN_EDIT: bool = false;
    pub const REQUIRES_PROJECT: bool = true;
}

#[derive(Debug, Deserialize)]
struct ReadFileInput {
    relative_path: String,
    #[serde(default)]
    start_line: usize,
    end_line: Option<usize>,
}

impl Tool for ReadFileTool {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn description(&self) -> &'static str {
        Self::DESCRIPTION
    }

    fn apply_json(&self, input: serde_json::Value) -> Result<String> {
        let input: ReadFileInput = serde_json::from_value(input)?;

        // Implementation here
        let content = std::fs::read_to_string(&input.relative_path)?;
        let lines: Vec<&str> = content.lines().collect();

        let start = input.start_line;
        let end = input.end_line.unwrap_or(lines.len());

        let result = lines[start..end].join("\n");
        Ok(result)
    }

    fn can_edit(&self) -> bool {
        Self::CAN_EDIT
    }

    fn requires_project(&self) -> bool {
        Self::REQUIRES_PROJECT
    }
}

// Register the tool
register_tool!(ReadFileTool);
```

### Strongly-Typed Tool Inputs (Alternative Approach)

```rust
// src/tools/typed.rs
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Trait for tools with strongly-typed inputs/outputs
#[async_trait]
pub trait TypedTool: Send + Sync {
    type Input: for<'de> Deserialize<'de> + Send;
    type Output: Serialize + Send;

    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;

    async fn apply(&self, input: Self::Input, context: &ToolContext) -> Result<Self::Output>;
}

pub struct ToolContext {
    pub project_root: PathBuf,
    pub agent: Arc<dyn Agent>,
}

// Example: Strongly-typed symbol finding tool
#[derive(Debug, Deserialize)]
pub struct FindSymbolInput {
    name_path_pattern: String,
    #[serde(default)]
    depth: usize,
    #[serde(default)]
    relative_path: Option<String>,
    #[serde(default)]
    include_body: bool,
}

#[derive(Debug, Serialize)]
pub struct FindSymbolOutput {
    symbols: Vec<SymbolInfo>,
}

pub struct FindSymbolTool;

#[async_trait]
impl TypedTool for FindSymbolTool {
    type Input = FindSymbolInput;
    type Output = FindSymbolOutput;

    fn name(&self) -> &'static str {
        "find_symbol"
    }

    fn description(&self) -> &'static str {
        "Finds symbols matching a name path pattern"
    }

    async fn apply(&self, input: Self::Input, ctx: &ToolContext) -> Result<Self::Output> {
        let lsp_manager = ctx.agent.language_server_manager();
        let symbols = lsp_manager
            .find_symbols(&input.name_path_pattern, input.depth)
            .await?;

        Ok(FindSymbolOutput { symbols })
    }
}
```

---

## 2. Configuration & Serialization

### Python: Dataclass with YAML Serialization

```python
# src/serena/config/serena_config.py
from dataclasses import dataclass, field
from typing import Optional
import yaml

@dataclass
class ProjectConfig:
    project_name: str
    languages: list[Language]
    ignored_paths: list[str] = field(default_factory=list)
    read_only: bool = False
    encoding: str = "utf-8"

    @classmethod
    def load(cls, path: str) -> "ProjectConfig":
        with open(path, "r") as f:
            data = yaml.safe_load(f)
        return cls(**data)

    def save(self, path: str) -> None:
        with open(path, "w") as f:
            yaml.dump(self.__dict__, f)
```

### Rust: Serde with YAML

```rust
// src/config/project.rs
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProjectConfig {
    pub project_name: String,
    pub languages: Vec<Language>,

    #[serde(default)]
    pub ignored_paths: Vec<String>,

    #[serde(default)]
    pub read_only: bool,

    #[serde(default = "default_encoding")]
    pub encoding: String,
}

fn default_encoding() -> String {
    "utf-8".to_string()
}

impl ProjectConfig {
    /// Load configuration from YAML file
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .context("Failed to read config file")?;

        serde_yaml::from_str(&content)
            .context("Failed to parse YAML")
    }

    /// Save configuration to YAML file
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let yaml = serde_yaml::to_string(self)?;
        std::fs::write(path.as_ref(), yaml)
            .context("Failed to write config file")
    }

    /// Auto-generate config from project directory
    pub fn autogenerate(project_root: impl AsRef<Path>) -> Result<Self> {
        let project_name = project_root
            .as_ref()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed")
            .to_string();

        let languages = detect_languages(project_root.as_ref())?;

        Ok(Self {
            project_name,
            languages,
            ignored_paths: default_ignored_paths(),
            read_only: false,
            encoding: default_encoding(),
        })
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Python,
    Rust,
    TypeScript,
    Go,
    Java,
    // ... 40+ languages
}

fn detect_languages(root: &Path) -> Result<Vec<Language>> {
    // Implementation: scan directory for file extensions
    let mut languages = std::collections::HashSet::new();

    for entry in walkdir::WalkDir::new(root).max_depth(3) {
        let entry = entry?;
        if let Some(ext) = entry.path().extension() {
            match ext.to_str() {
                Some("py") => { languages.insert(Language::Python); }
                Some("rs") => { languages.insert(Language::Rust); }
                Some("ts") | Some("tsx") => { languages.insert(Language::TypeScript); }
                Some("go") => { languages.insert(Language::Go); }
                Some("java") => { languages.insert(Language::Java); }
                _ => {}
            }
        }
    }

    Ok(languages.into_iter().collect())
}

fn default_ignored_paths() -> Vec<String> {
    vec![
        "node_modules".to_string(),
        "target".to_string(),
        "__pycache__".to_string(),
        ".git".to_string(),
    ]
}
```

### Advanced: Preserving YAML Comments (ruamel.yaml equivalent)

```rust
// src/config/yaml_comments.rs
use serde_yaml::Value;
use std::collections::HashMap;

/// YAML configuration with comment preservation
pub struct CommentedConfig {
    pub value: Value,
    pub comments: HashMap<String, String>,
}

impl CommentedConfig {
    pub fn load(content: &str) -> Result<Self> {
        // Parse YAML
        let value: Value = serde_yaml::from_str(content)?;

        // Parse comments separately (basic implementation)
        let comments = extract_comments(content);

        Ok(Self { value, comments })
    }

    pub fn save(&self) -> String {
        let mut yaml = serde_yaml::to_string(&self.value).unwrap();

        // Re-insert comments
        for (key, comment) in &self.comments {
            // Find key in YAML and insert comment above it
            yaml = yaml.replace(
                &format!("{}:", key),
                &format!("# {}\n{}:", comment, key)
            );
        }

        yaml
    }
}

fn extract_comments(content: &str) -> HashMap<String, String> {
    let mut comments = HashMap::new();
    let mut last_comment = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            last_comment = Some(trimmed[1..].trim().to_string());
        } else if trimmed.contains(':') && last_comment.is_some() {
            let key = trimmed.split(':').next().unwrap().trim();
            comments.insert(key.to_string(), last_comment.take().unwrap());
        }
    }

    comments
}
```

---

## 3. LSP Client Implementation

### Python: Subprocess-based LSP Client

```python
# src/solidlsp/ls.py
class SolidLanguageServer:
    def __init__(self, config: LanguageServerConfig):
        self.config = config
        self.process = None
        self.stdin = None
        self.stdout = None

    def start(self):
        self.process = subprocess.Popen(
            self.config.command,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        self.stdin = self.process.stdin
        self.stdout = self.process.stdout

    def send_request(self, method: str, params: dict) -> dict:
        request = {
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": method,
            "params": params,
        }

        json_str = json.dumps(request)
        message = f"Content-Length: {len(json_str)}\r\n\r\n{json_str}"

        self.stdin.write(message.encode())
        self.stdin.flush()

        # Read response
        headers = self.read_headers()
        content_length = int(headers["Content-Length"])
        content = self.stdout.read(content_length)

        return json.loads(content)
```

### Rust: Tokio-based LSP Client

```rust
// src/lsp/client.rs
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};
use lsp_types::*;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::Mutex;

pub struct LspClient {
    child: Child,
    stdin: ChildStdin,
    stdout_task: tokio::task::JoinHandle<()>,
    pending_requests: Arc<Mutex<HashMap<i64, oneshot::Sender<Result<Value>>>>>,
    next_id: Arc<Mutex<i64>>,
}

impl LspClient {
    pub async fn spawn(command: Vec<String>, args: Vec<String>) -> Result<Self> {
        let mut child = Command::new(&command[0])
            .args(&command[1..])
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        let pending_requests = Arc::new(Mutex::new(HashMap::new()));
        let pending_clone = pending_requests.clone();

        // Spawn task to read stdout
        let stdout_task = tokio::spawn(async move {
            Self::read_stdout(stdout, pending_clone).await;
        });

        Ok(Self {
            child,
            stdin,
            stdout_task,
            pending_requests,
            next_id: Arc::new(Mutex::new(1)),
        })
    }

    async fn read_stdout(
        stdout: ChildStdout,
        pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Result<Value>>>>>,
    ) {
        let mut reader = BufReader::new(stdout);
        let mut headers = String::new();
        let mut content_length = 0;

        loop {
            headers.clear();

            // Read headers
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).await.unwrap() == 0 {
                    return; // EOF
                }

                if line == "\r\n" {
                    break;
                }

                if line.starts_with("Content-Length: ") {
                    content_length = line[16..].trim().parse().unwrap();
                }
            }

            // Read content
            let mut content = vec![0u8; content_length];
            reader.read_exact(&mut content).await.unwrap();

            // Parse JSON
            let response: Value = serde_json::from_slice(&content).unwrap();

            // Handle response
            if let Some(id) = response.get("id").and_then(|v| v.as_i64()) {
                let mut pending = pending.lock();
                if let Some(sender) = pending.remove(&id) {
                    let _ = sender.send(Ok(response));
                }
            }
        }
    }

    pub async fn send_request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = {
            let mut next = self.next_id.lock();
            let id = *next;
            *next += 1;
            id
        };

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let json = serde_json::to_string(&request)?;
        let message = format!("Content-Length: {}\r\n\r\n{}", json.len(), json);

        self.stdin.write_all(message.as_bytes()).await?;
        self.stdin.flush().await?;

        // Wait for response
        let (tx, rx) = oneshot::channel();
        self.pending_requests.lock().insert(id, tx);

        tokio::time::timeout(std::time::Duration::from_secs(30), rx)
            .await??
    }

    pub async fn initialize(&mut self, root_uri: Url) -> Result<InitializeResult> {
        let params = InitializeParams {
            process_id: Some(std::process::id() as i32),
            root_uri: Some(root_uri.clone()),
            capabilities: ClientCapabilities::default(),
            ..Default::default()
        };

        let response = self.send_request("initialize", serde_json::to_value(params)?).await?;
        let result: InitializeResult = serde_json::from_value(response["result"].clone())?;

        // Send initialized notification
        self.send_notification("initialized", serde_json::json!({})).await?;

        Ok(result)
    }

    pub async fn send_notification(&mut self, method: &str, params: Value) -> Result<()> {
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });

        let json = serde_json::to_string(&notification)?;
        let message = format!("Content-Length: {}\r\n\r\n{}", json.len(), json);

        self.stdin.write_all(message.as_bytes()).await?;
        self.stdin.flush().await?;

        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        self.send_request("shutdown", Value::Null).await?;
        self.send_notification("exit", Value::Null).await?;
        self.child.kill().await?;
        Ok(())
    }
}

/// Higher-level language server manager
pub struct LanguageServerManager {
    clients: Arc<Mutex<HashMap<Language, Arc<LspClient>>>>,
}

impl LanguageServerManager {
    pub async fn get_or_start(&self, language: Language) -> Result<Arc<LspClient>> {
        let mut clients = self.clients.lock();

        if let Some(client) = clients.get(&language) {
            return Ok(client.clone());
        }

        // Start new client
        let config = LanguageServerConfig::for_language(language);
        let client = LspClient::spawn(config.command, config.args).await?;
        let client = Arc::new(client);

        clients.insert(language, client.clone());
        Ok(client)
    }

    pub async fn find_symbols(&self, pattern: &str) -> Result<Vec<SymbolInformation>> {
        // Find in all active language servers
        let clients = self.clients.lock();
        let mut symbols = Vec::new();

        for client in clients.values() {
            let params = WorkspaceSymbolParams {
                query: pattern.to_string(),
                ..Default::default()
            };

            let response = client.send_request(
                "workspace/symbol",
                serde_json::to_value(params)?
            ).await?;

            let syms: Vec<SymbolInformation> = serde_json::from_value(response["result"].clone())?;
            symbols.extend(syms);
        }

        Ok(symbols)
    }
}
```

---

## 4. MCP Server Implementation

### Python: FastMCP Server

```python
# src/serena/mcp.py
from mcp.server.fastmcp import FastMCP
from mcp.types import ToolAnnotations

app = FastMCP("serena")

@app.tool()
async def read_file(relative_path: str) -> str:
    """Read a file from the project"""
    # Implementation
    pass

if __name__ == "__main__":
    app.run()
```

### Rust: Axum-based MCP Server

```rust
// src/mcp/server.rs
use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[derive(Debug, Deserialize)]
struct McpRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct McpResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpError>,
}

#[derive(Debug, Serialize)]
struct McpError {
    code: i32,
    message: String,
}

pub struct McpServer {
    agent: Arc<SerenaAgent>,
    tool_registry: Arc<ToolRegistry>,
}

impl McpServer {
    pub fn new(agent: Arc<SerenaAgent>) -> Self {
        Self {
            agent,
            tool_registry: Arc::new(ToolRegistry::new()),
        }
    }

    pub fn router(self) -> Router {
        let state = Arc::new(self);

        Router::new()
            .route("/mcp", post(handle_mcp_request))
            .route("/tools/list", post(list_tools))
            .route("/tools/call", post(call_tool))
            .layer(CorsLayer::permissive())
            .with_state(state)
    }

    async fn handle_request(&self, request: McpRequest) -> McpResponse {
        let result = match request.method.as_str() {
            "tools/list" => self.list_tools(),
            "tools/call" => self.call_tool(request.params.unwrap()).await,
            "initialize" => self.initialize(),
            _ => return self.error_response(request.id, -32601, "Method not found"),
        };

        match result {
            Ok(value) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(value),
                error: None,
            },
            Err(e) => self.error_response(request.id, -32603, &e.to_string()),
        }
    }

    fn list_tools(&self) -> Result<Value> {
        let tools: Vec<_> = ToolRegistry::all_tools()
            .map(|desc| {
                serde_json::json!({
                    "name": desc.name,
                    "description": desc.description,
                    "inputSchema": self.generate_schema(desc),
                })
            })
            .collect();

        Ok(serde_json::json!({ "tools": tools }))
    }

    async fn call_tool(&self, params: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct CallParams {
            name: String,
            arguments: Value,
        }

        let call: CallParams = serde_json::from_value(params)?;

        let tool = ToolRegistry::get_tool(&call.name)
            .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", call.name))?;

        let context = ToolContext {
            project_root: self.agent.project_root(),
            agent: self.agent.clone(),
        };

        let result = tool.apply_json(call.arguments)?;
        Ok(serde_json::json!({ "content": [{ "type": "text", "text": result }] }))
    }

    fn generate_schema(&self, desc: &ToolDescriptor) -> Value {
        // Generate JSON Schema from tool input types
        // This would use a derive macro in practice
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    fn error_response(&self, id: Option<Value>, code: i32, message: &str) -> McpResponse {
        McpResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(McpError {
                code,
                message: message.to_string(),
            }),
        }
    }
}

async fn handle_mcp_request(
    State(server): State<Arc<McpServer>>,
    Json(request): Json<McpRequest>,
) -> Json<McpResponse> {
    let response = server.handle_request(request).await;
    Json(response)
}

async fn list_tools(
    State(server): State<Arc<McpServer>>,
) -> Json<Value> {
    let tools = server.list_tools().unwrap_or_else(|_| serde_json::json!({"tools": []}));
    Json(tools)
}

async fn call_tool(
    State(server): State<Arc<McpServer>>,
    Json(params): Json<Value>,
) -> Json<Value> {
    let result = server.call_tool(params).await
        .unwrap_or_else(|e| serde_json::json!({"error": e.to_string()}));
    Json(result)
}

pub async fn start_mcp_server(port: u16) -> Result<()> {
    let agent = Arc::new(SerenaAgent::new()?);
    let server = McpServer::new(agent);
    let app = server.router();

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));

    tracing::info!("MCP server listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
```

---

## 5. File Operations

### Python: File Search with Context

```python
# src/serena/text_utils.py
def search_files(pattern: str, root: str, files: list[str]) -> dict:
    regex = re.compile(pattern, re.DOTALL)
    results = {}

    for file in files:
        path = os.path.join(root, file)
        with open(path, 'r') as f:
            content = f.read()

        matches = []
        for match in regex.finditer(content):
            lines = content[:match.start()].count('\n')
            matches.append({
                'line': lines + 1,
                'text': match.group(),
            })

        if matches:
            results[file] = matches

    return results
```

### Rust: Parallel File Search (Already Implemented)

```rust
// serena_core/src/lib.rs (already exists, showing for completeness)
use rayon::prelude::*;
use regex::RegexBuilder;

fn search_files_impl(
    pattern: &str,
    root: &str,
    relative_paths: Vec<String>,
    context_lines_before: usize,
    context_lines_after: usize,
) -> Result<Vec<(String, Vec<FileMatch>)>> {
    let re = RegexBuilder::new(pattern)
        .dot_matches_new_line(true)
        .build()?;

    let root_path = PathBuf::from(root);

    // Parallel search across files
    let results: Vec<(String, Vec<FileMatch>)> = relative_paths
        .into_par_iter()  // Rayon parallel iterator
        .filter_map(|rel_path| {
            let full_path = root_path.join(&rel_path);
            let content = fs::read_to_string(&full_path).ok()?;

            let file_matches = search_in_content(
                &content,
                &re,
                context_lines_before,
                context_lines_after,
            ).ok()?;

            if file_matches.is_empty() {
                None
            } else {
                Some((rel_path, file_matches))
            }
        })
        .collect();

    Ok(results)
}

#[pyfunction]
fn search_files(
    py: Python<'_>,
    pattern: &str,
    root: &str,
    relative_paths: Vec<String>,
    context_lines_before: usize,
    context_lines_after: usize,
) -> PyResult<Vec<PyObject>> {
    // Release GIL during CPU-intensive work
    let results = py.allow_threads(|| {
        search_files_impl(pattern, root, relative_paths, context_lines_before, context_lines_after)
    })?;

    // Convert to Python objects
    let mut out = Vec::new();
    for (path, matches) in results {
        for m in matches {
            let dict = PyDict::new_bound(py);
            dict.set_item("path", &path)?;
            dict.set_item("lines", convert_matches(py, m)?)?;
            out.push(dict.into_py(py));
        }
    }

    Ok(out)
}
```

---

## 6. Error Handling Patterns

### Python: Exception-based

```python
class LanguageServerError(Exception):
    pass

def find_symbol(name: str) -> Symbol:
    try:
        result = lsp_client.send_request("findSymbol", {"name": name})
        if "error" in result:
            raise LanguageServerError(result["error"])
        return Symbol.from_dict(result)
    except TimeoutError:
        raise LanguageServerError("LSP request timed out")
    except json.JSONDecodeError as e:
        raise LanguageServerError(f"Invalid JSON: {e}")
```

### Rust: Result-based with thiserror

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LspError {
    #[error("Language server request failed: {0}")]
    RequestFailed(String),

    #[error("Language server timed out after {0}s")]
    Timeout(u64),

    #[error("Invalid JSON response: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Language server not found for {0:?}")]
    ServerNotFound(Language),
}

pub type Result<T> = std::result::Result<T, LspError>;

async fn find_symbol(name: &str) -> Result<Symbol> {
    let response = lsp_client
        .send_request("findSymbol", json!({"name": name}))
        .await
        .map_err(|e| LspError::RequestFailed(e.to_string()))?;

    if let Some(error) = response.get("error") {
        return Err(LspError::RequestFailed(error.to_string()));
    }

    let symbol = serde_json::from_value(response["result"].clone())?;
    Ok(symbol)
}

// Usage with context
async fn handle_find_symbol(name: &str) -> anyhow::Result<Symbol> {
    find_symbol(name)
        .await
        .context("Failed to find symbol in workspace")
}
```

---

## 7. Async/Concurrency Patterns

### Python: asyncio + ThreadPoolExecutor

```python
import asyncio
from concurrent.futures import ThreadPoolExecutor

class TaskExecutor:
    def __init__(self):
        self.executor = ThreadPoolExecutor(max_workers=4)

    def submit(self, fn, *args):
        return self.executor.submit(fn, *args)

    async def run_async(self, fn, *args):
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(self.executor, fn, *args)
```

### Rust: Tokio Runtime

```rust
use tokio::task;
use std::time::Duration;

pub struct TaskExecutor {
    runtime: tokio::runtime::Runtime,
}

impl TaskExecutor {
    pub fn new() -> Self {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        Self { runtime }
    }

    /// Run a blocking task in a thread pool
    pub fn spawn_blocking<F, R>(&self, f: F) -> task::JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        task::spawn_blocking(f)
    }

    /// Run an async task
    pub fn spawn<F>(&self, future: F) -> task::JoinHandle<F::Output>
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        task::spawn(future)
    }

    /// Run with timeout
    pub async fn with_timeout<F>(
        &self,
        duration: Duration,
        future: F,
    ) -> Result<F::Output>
    where
        F: std::future::Future,
    {
        tokio::time::timeout(duration, future)
            .await
            .map_err(|_| anyhow::anyhow!("Task timed out"))
    }

    /// Run multiple tasks concurrently
    pub async fn join_all<F>(&self, futures: Vec<F>) -> Vec<F::Output>
    where
        F: std::future::Future,
    {
        futures::future::join_all(futures).await
    }
}

// Example usage
async fn process_tools_concurrently() -> Result<Vec<ToolResult>> {
    let executor = TaskExecutor::new();

    let tasks = vec![
        executor.spawn(process_tool("read_file")),
        executor.spawn(process_tool("write_file")),
        executor.spawn(process_tool("find_symbol")),
    ];

    let results = futures::future::join_all(tasks).await;

    results.into_iter()
        .map(|r| r.map_err(|e| anyhow::anyhow!("Task failed: {}", e))?)
        .collect()
}
```

---

## 8. Testing Patterns

### Python: pytest with fixtures

```python
# test/test_tools.py
import pytest

@pytest.fixture
def agent():
    return SerenaAgent.create_for_testing()

@pytest.fixture
def tool(agent):
    return ReadFileTool(agent)

def test_read_file(tool):
    result = tool.apply(relative_path="test.py")
    assert "def test_" in result

def test_read_file_with_lines(tool):
    result = tool.apply(relative_path="test.py", start_line=0, end_line=10)
    assert result.count('\n') <= 10
```

### Rust: cargo test with rstest

```rust
// src/tools/file.rs
#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use tempfile::TempDir;

    #[fixture]
    fn temp_project() -> TempDir {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("test.rs"), "fn main() {}").unwrap();
        dir
    }

    #[fixture]
    fn agent(temp_project: TempDir) -> SerenaAgent {
        SerenaAgent::new_for_testing(temp_project.path())
    }

    #[rstest]
    fn test_read_file(agent: SerenaAgent) {
        let tool = ReadFileTool::default();
        let ctx = ToolContext {
            project_root: agent.project_root(),
            agent: Arc::new(agent),
        };

        let input = ReadFileInput {
            relative_path: "test.rs".to_string(),
            start_line: 0,
            end_line: None,
        };

        let result = tool.apply(input, &ctx).unwrap();
        assert!(result.contains("fn main"));
    }

    #[rstest]
    #[case(0, Some(5))]
    #[case(5, Some(10))]
    fn test_read_file_with_lines(
        agent: SerenaAgent,
        #[case] start: usize,
        #[case] end: Option<usize>,
    ) {
        let tool = ReadFileTool::default();
        let ctx = ToolContext {
            project_root: agent.project_root(),
            agent: Arc::new(agent),
        };

        let input = ReadFileInput {
            relative_path: "test.rs".to_string(),
            start_line: start,
            end_line: end,
        };

        let result = tool.apply(input, &ctx).unwrap();
        let line_count = result.lines().count();

        if let Some(end_line) = end {
            assert!(line_count <= (end_line - start + 1));
        }
    }
}

// Integration tests
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_tool_workflow() {
        let agent = SerenaAgent::new_for_testing("test_project");
        let mcp_server = McpServer::new(Arc::new(agent));

        // Call tool via MCP
        let request = serde_json::json!({
            "name": "read_file",
            "arguments": {
                "relative_path": "src/main.rs"
            }
        });

        let response = mcp_server.call_tool(request).await.unwrap();
        assert!(response["content"][0]["text"].as_str().unwrap().contains("fn main"));
    }
}
```

### Snapshot Testing with Insta

```rust
use insta::assert_snapshot;

#[test]
fn test_symbol_output_format() {
    let symbol = Symbol {
        name: "MyClass".to_string(),
        kind: SymbolKind::Class,
        range: Range::default(),
        detail: Some("A test class".to_string()),
    };

    let json = serde_json::to_string_pretty(&symbol).unwrap();

    // Creates/validates snapshot in snapshots/ directory
    assert_snapshot!(json);
}
```

---

## Summary

This document provides concrete migration patterns for:
1. ✅ Tool system (dynamic → static registration)
2. ✅ Configuration (dataclass → serde)
3. ✅ LSP client (subprocess → tokio)
4. ✅ MCP server (FastMCP → axum)
5. ✅ File operations (sequential → parallel)
6. ✅ Error handling (exceptions → Result)
7. ✅ Concurrency (GIL → tokio)
8. ✅ Testing (pytest → cargo test)

All patterns follow Rust best practices:
- Type safety via trait system
- Error handling via Result/anyhow
- Async via tokio
- Serialization via serde
- Testing via cargo test + rstest + insta

---

*Generated: 2025-12-21*
