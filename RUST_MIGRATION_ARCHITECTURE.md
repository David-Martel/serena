# Pure Rust Serena Architecture

**Version:** 1.0
**Date:** 2025-12-21
**Status:** Design Proposal

## Executive Summary

This document outlines the complete architecture for migrating Serena from Python to pure Rust, eliminating all Python dependencies, PyO3 bindings, and Docker requirements. The design prioritizes:

- **Incremental migration** - Build functionality piece by piece
- **Cross-platform support** - Windows, Linux, macOS from day one
- **High performance** - Leverage Rust's async/parallel capabilities
- **Maintainability** - Clean module boundaries and trait abstractions
- **Practical deployment** - Single binary distribution

## Table of Contents

1. [Workspace Structure](#workspace-structure)
2. [Module Architecture](#module-architecture)
3. [Core Trait Definitions](#core-trait-definitions)
4. [Technology Stack](#technology-stack)
5. [Migration Phases](#migration-phases)
6. [API Specifications](#api-specifications)
7. [Build and Distribution](#build-and-distribution)
8. [Testing Strategy](#testing-strategy)

---

## Workspace Structure

### Multi-Crate Workspace Layout

```
serena/
├── Cargo.toml                    # Workspace root
├── Makefile.toml                 # cargo-make build automation
├── .cargo/
│   └── config.toml              # Shared compiler configuration
├── crates/
│   ├── serena-core/             # Core types and traits
│   ├── serena-lsp/              # LSP client implementation
│   ├── serena-tools/            # Tool implementations
│   ├── serena-mcp/              # MCP server protocol
│   ├── serena-memory/           # Knowledge persistence
│   ├── serena-config/           # Configuration management
│   ├── serena-web/              # Web dashboard (optional)
│   ├── serena-cli/              # CLI interface
│   └── serena/                  # Main binary crate
├── tests/                       # Integration tests
├── benches/                     # Performance benchmarks
└── docs/                        # Documentation

```

### Workspace Cargo.toml

```toml
[workspace]
members = [
    "crates/serena-core",
    "crates/serena-lsp",
    "crates/serena-tools",
    "crates/serena-mcp",
    "crates/serena-memory",
    "crates/serena-config",
    "crates/serena-web",
    "crates/serena-cli",
    "crates/serena",
]
resolver = "2"

[workspace.package]
version = "0.2.0"
edition = "2021"
rust-version = "1.75"
license = "MIT"
repository = "https://github.com/oraios/serena"

[workspace.dependencies]
# Async runtime
tokio = { version = "1.41", features = ["full"] }
tokio-util = { version = "0.7", features = ["codec", "io"] }
futures = "0.3"
async-trait = "0.1"

# LSP and protocol
lsp-types = "0.98"
tower-lsp = "0.20"
lsp-server = "0.7"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
toml = "0.8"

# Web server (optional)
axum = { version = "0.7", optional = true }
tower = { version = "0.4", features = ["util", "timeout", "limit"] }
tower-http = { version = "0.5", features = ["fs", "trace", "cors"] }

# HTTP client
reqwest = { version = "0.12", features = ["json", "stream"] }

# File system
ignore = "0.4"
walkdir = "2.5"
notify = "6.1"

# Text processing
regex = "1.11"
ropey = "1.6"  # Text rope for efficient editing
tree-sitter = "0.22"

# Parallelism
rayon = "1.10"
dashmap = "6.1"
parking_lot = "0.12"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-appender = "0.2"

# CLI
clap = { version = "4.5", features = ["derive", "env", "color"] }

# Config
config = "0.14"
directories = "5.0"

# Database (for memory storage)
rusqlite = { version = "0.32", features = ["bundled"] }
# OR: use sled for embedded key-value store
sled = "0.34"

# Compression
flate2 = "1.0"
tar = "0.4"
zip = "2.2"

# Utilities
chrono = "0.4"
uuid = { version = "1.11", features = ["v4", "serde"] }
once_cell = "1.20"
lazy_static = "1.5"

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = true

[profile.release-optimized]
inherits = "release"
lto = "fat"
codegen-units = 1
panic = "abort"
```

---

## Module Architecture

### 1. serena-core

**Purpose:** Core types, traits, and error handling shared across all crates.

**Structure:**
```
serena-core/
├── src/
│   ├── lib.rs
│   ├── error.rs          # Error types
│   ├── types.rs          # Common types
│   ├── traits/
│   │   ├── mod.rs
│   │   ├── tool.rs       # Tool trait
│   │   ├── lsp.rs        # LSP abstraction
│   │   ├── storage.rs    # Storage trait
│   │   └── config.rs     # Config provider
│   └── utils/
│       ├── mod.rs
│       ├── path.rs       # Path utilities
│       └── text.rs       # Text processing
└── Cargo.toml
```

**Key Types:**

```rust
// src/types.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectPath {
    pub root: PathBuf,
    pub relative: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
    pub children: Vec<SymbolInfo>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SymbolKind {
    File, Module, Namespace, Package, Class, Method,
    Property, Field, Constructor, Enum, Interface,
    Function, Variable, Constant, String, Number,
    Boolean, Array, Object, Key, Null, EnumMember,
    Struct, Event, Operator, TypeParameter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}
```

**Error Types:**

```rust
// src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SerenaError {
    #[error("LSP error: {0}")]
    Lsp(#[from] LspError),

    #[error("Tool error: {0}")]
    Tool(String),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("Language server not available: {0}")]
    LanguageServerUnavailable(String),
}

#[derive(Error, Debug)]
pub enum LspError {
    #[error("Language server crashed")]
    Crashed,

    #[error("Request timeout")]
    Timeout,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Communication error: {0}")]
    Communication(String),
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Invalid configuration: {0}")]
    Invalid(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Parse error: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, SerenaError>;
```

### 2. serena-lsp

**Purpose:** LSP client implementation for all supported languages.

**Structure:**
```
serena-lsp/
├── src/
│   ├── lib.rs
│   ├── client.rs          # Generic LSP client
│   ├── manager.rs         # Language server lifecycle
│   ├── cache.rs           # Response caching
│   ├── languages/
│   │   ├── mod.rs
│   │   ├── python.rs
│   │   ├── rust.rs
│   │   ├── typescript.rs
│   │   ├── go.rs
│   │   ├── java.rs
│   │   └── ... (30+ languages)
│   └── utils/
│       ├── mod.rs
│       ├── stdio.rs       # Stdio communication
│       └── download.rs    # Language server downloads
└── Cargo.toml
```

**Key Traits:**

```rust
// src/lib.rs
use async_trait::async_trait;
use lsp_types::*;
use serena_core::{Result, SymbolInfo, Location};
use std::path::Path;

#[async_trait]
pub trait LanguageServer: Send + Sync {
    /// Initialize the language server
    async fn initialize(&mut self, root_uri: Url) -> Result<()>;

    /// Shutdown the language server
    async fn shutdown(&mut self) -> Result<()>;

    /// Get document symbols
    async fn document_symbols(&self, uri: &Url) -> Result<Vec<SymbolInfo>>;

    /// Find symbol references
    async fn find_references(&self, uri: &Url, position: Position) -> Result<Vec<Location>>;

    /// Rename symbol
    async fn rename(&self, uri: &Url, position: Position, new_name: String) -> Result<WorkspaceEdit>;

    /// Get symbol definition
    async fn goto_definition(&self, uri: &Url, position: Position) -> Result<Vec<Location>>;

    /// Open document
    async fn did_open(&self, uri: &Url, text: String, language_id: String) -> Result<()>;

    /// Close document
    async fn did_close(&self, uri: &Url) -> Result<()>;

    /// Notify of document changes
    async fn did_change(&self, uri: &Url, text: String) -> Result<()>;
}

pub struct LanguageServerManager {
    servers: DashMap<Language, Box<dyn LanguageServer>>,
    cache: Arc<LspCache>,
}

impl LanguageServerManager {
    pub async fn start_server(&self, language: Language, root: &Path) -> Result<()>;
    pub async fn stop_server(&self, language: Language) -> Result<()>;
    pub async fn restart_server(&self, language: Language) -> Result<()>;
    pub fn get_server(&self, language: Language) -> Option<Arc<dyn LanguageServer>>;
}
```

**LSP Client Implementation:**

```rust
// src/client.rs
use tokio::process::{Child, Command};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use lsp_server::{Connection, Message, Request, Response};

pub struct LspClient {
    process: Child,
    connection: Connection,
    request_id: AtomicU64,
    pending_requests: DashMap<u64, oneshot::Sender<Response>>,
}

impl LspClient {
    pub async fn spawn(command: &str, args: &[&str]) -> Result<Self> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        let connection = Connection::stdio_from_streams(stdin, stdout);

        Ok(Self {
            process: child,
            connection,
            request_id: AtomicU64::new(0),
            pending_requests: DashMap::new(),
        })
    }

    pub async fn send_request<P, R>(&self, method: &str, params: P) -> Result<R>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();

        self.pending_requests.insert(id, tx);

        let request = Request::new(id.into(), method.to_string(), params);
        self.connection.sender.send(Message::Request(request))?;

        let response = tokio::time::timeout(
            Duration::from_secs(30),
            rx
        ).await??;

        if let Some(error) = response.error {
            return Err(LspError::InvalidResponse(error.message).into());
        }

        Ok(serde_json::from_value(response.result.unwrap())?)
    }

    async fn handle_messages(self: Arc<Self>) {
        while let Ok(msg) = self.connection.receiver.recv() {
            match msg {
                Message::Response(response) => {
                    if let Some((_, tx)) = self.pending_requests.remove(&response.id) {
                        let _ = tx.send(response);
                    }
                }
                Message::Notification(notif) => {
                    self.handle_notification(notif).await;
                }
                _ => {}
            }
        }
    }
}
```

### 3. serena-tools

**Purpose:** Implementation of all Serena tools (file, symbol, memory operations).

**Structure:**
```
serena-tools/
├── src/
│   ├── lib.rs
│   ├── registry.rs        # Tool registration and discovery
│   ├── base.rs           # Base Tool trait
│   ├── file/
│   │   ├── mod.rs
│   │   ├── read.rs
│   │   ├── write.rs
│   │   ├── search.rs
│   │   └── replace.rs
│   ├── symbol/
│   │   ├── mod.rs
│   │   ├── find.rs
│   │   ├── rename.rs
│   │   ├── references.rs
│   │   └── edit.rs
│   ├── memory/
│   │   ├── mod.rs
│   │   ├── write.rs
│   │   └── read.rs
│   └── config/
│       ├── mod.rs
│       ├── activate.rs
│       └── modes.rs
└── Cargo.toml
```

**Tool Trait:**

```rust
// src/base.rs
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serena_core::Result;
use std::any::Any;

#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name (used for MCP protocol)
    fn name(&self) -> &str;

    /// Tool description (for AI agent)
    fn description(&self) -> &str;

    /// Parameter schema (JSON Schema)
    fn parameters_schema(&self) -> serde_json::Value;

    /// Execute the tool with parameters
    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult>;

    /// Whether this tool can edit files
    fn can_edit(&self) -> bool {
        false
    }

    /// Whether this tool requires an active project
    fn requires_project(&self) -> bool {
        true
    }

    /// Tool markers (for categorization)
    fn markers(&self) -> Vec<ToolMarker> {
        vec![]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub status: ToolStatus,
    pub data: serde_json::Value,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ToolStatus {
    Success,
    Error,
    Warning,
}

#[derive(Debug, Clone, Copy)]
pub enum ToolMarker {
    CanEdit,
    SymbolicRead,
    SymbolicEdit,
    Optional,
    NoProject,
}
```

**Example Tool Implementation:**

```rust
// src/file/read.rs
use super::*;

pub struct ReadFileTool {
    project: Arc<Project>,
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Reads a file from the project. Returns the full text of the file."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "relative_path": {
                    "type": "string",
                    "description": "Path relative to project root"
                },
                "start_line": {
                    "type": "integer",
                    "description": "Optional start line (0-based)"
                },
                "end_line": {
                    "type": "integer",
                    "description": "Optional end line (inclusive)"
                }
            },
            "required": ["relative_path"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult> {
        let relative_path: String = params["relative_path"]
            .as_str()
            .ok_or_else(|| SerenaError::Tool("Missing relative_path".into()))?
            .to_string();

        let abs_path = self.project.root().join(&relative_path);
        let content = tokio::fs::read_to_string(&abs_path).await?;

        let start = params["start_line"].as_u64().unwrap_or(0) as usize;
        let end = params["end_line"].as_u64().map(|l| l as usize);

        let lines: Vec<&str> = content.lines().collect();
        let slice = if let Some(end) = end {
            &lines[start..=end.min(lines.len() - 1)]
        } else {
            &lines[start..]
        };

        Ok(ToolResult {
            status: ToolStatus::Success,
            data: json!({
                "result": slice.join("\n"),
                "lines": slice.len(),
            }),
            error: None,
        })
    }
}
```

**Tool Registry:**

```rust
// src/registry.rs
use std::collections::HashMap;

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
    markers: HashMap<ToolMarker, Vec<String>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            markers: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        let name = tool.name().to_string();

        for marker in tool.markers() {
            self.markers.entry(marker)
                .or_insert_with(Vec::new)
                .push(name.clone());
        }

        self.tools.insert(name, tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|b| b.as_ref())
    }

    pub fn list_by_marker(&self, marker: ToolMarker) -> Vec<&str> {
        self.markers.get(&marker)
            .map(|v| v.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    pub fn all_names(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }
}
```

### 4. serena-mcp

**Purpose:** MCP (Model Context Protocol) server implementation.

**Structure:**
```
serena-mcp/
├── src/
│   ├── lib.rs
│   ├── server.rs          # MCP server
│   ├── protocol.rs        # MCP protocol types
│   ├── handlers.rs        # Request handlers
│   └── transport/
│       ├── mod.rs
│       ├── stdio.rs       # Stdio transport
│       └── http.rs        # HTTP transport (optional)
└── Cargo.toml
```

**MCP Server:**

```rust
// src/server.rs
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use serena_tools::ToolRegistry;
use serde_json::Value;

pub struct McpServer {
    tools: Arc<ToolRegistry>,
    project: Arc<RwLock<Option<Project>>>,
    config: Arc<SerenaConfig>,
}

impl McpServer {
    pub async fn run_stdio(self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let reader = BufReader::new(stdin);
        let mut writer = stdout;
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await? {
            let request: McpRequest = serde_json::from_str(&line)?;

            let response = self.handle_request(request).await;
            let response_json = serde_json::to_string(&response)?;

            writer.write_all(response_json.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await?;
        }

        Ok(())
    }

    async fn handle_request(&self, request: McpRequest) -> McpResponse {
        match request.method.as_str() {
            "tools/list" => self.handle_list_tools().await,
            "tools/call" => self.handle_call_tool(request.params).await,
            "initialize" => self.handle_initialize(request.params).await,
            _ => McpResponse::error("Unknown method"),
        }
    }

    async fn handle_call_tool(&self, params: Value) -> McpResponse {
        let tool_name = params["name"].as_str().unwrap();
        let tool_params = params["arguments"].clone();

        if let Some(tool) = self.tools.get(tool_name) {
            match tool.execute(tool_params).await {
                Ok(result) => McpResponse::success(result.data),
                Err(e) => McpResponse::error(&e.to_string()),
            }
        } else {
            McpResponse::error(&format!("Tool not found: {}", tool_name))
        }
    }
}
```

### 5. serena-memory

**Purpose:** Knowledge persistence and retrieval system.

**Structure:**
```
serena-memory/
├── src/
│   ├── lib.rs
│   ├── storage.rs         # Storage backend (SQLite/Sled)
│   ├── markdown.rs        # Markdown file storage
│   ├── search.rs          # Full-text search
│   └── retrieval.rs       # Context-aware retrieval
└── Cargo.toml
```

**Storage Trait:**

```rust
// src/lib.rs
use async_trait::async_trait;

#[async_trait]
pub trait MemoryStorage: Send + Sync {
    async fn write(&self, key: &str, content: &str) -> Result<()>;
    async fn read(&self, key: &str) -> Result<Option<String>>;
    async fn list(&self) -> Result<Vec<String>>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn search(&self, query: &str) -> Result<Vec<MemoryEntry>>;
}

#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub key: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// SQLite-based implementation
pub struct SqliteStorage {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl SqliteStorage {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = rusqlite::Connection::open(db_path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS memories (
                key TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Full-text search index
        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts
             USING fts5(key, content)",
            [],
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

#[async_trait]
impl MemoryStorage for SqliteStorage {
    async fn write(&self, key: &str, content: &str) -> Result<()> {
        let conn = self.conn.lock();
        let now = Utc::now().timestamp();

        conn.execute(
            "INSERT OR REPLACE INTO memories (key, content, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![key, content, now, now],
        )?;

        conn.execute(
            "INSERT OR REPLACE INTO memories_fts (key, content) VALUES (?1, ?2)",
            params![key, content],
        )?;

        Ok(())
    }

    async fn search(&self, query: &str) -> Result<Vec<MemoryEntry>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT m.key, m.content, m.created_at, m.updated_at
             FROM memories m
             JOIN memories_fts fts ON m.key = fts.key
             WHERE memories_fts MATCH ?1
             ORDER BY rank"
        )?;

        let entries = stmt.query_map([query], |row| {
            Ok(MemoryEntry {
                key: row.get(0)?,
                content: row.get(1)?,
                created_at: Utc.timestamp(row.get(2)?, 0),
                updated_at: Utc.timestamp(row.get(3)?, 0),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(entries)
    }
}
```

### 6. serena-config

**Purpose:** Configuration loading and management.

**Structure:**
```
serena-config/
├── src/
│   ├── lib.rs
│   ├── loader.rs          # Config file loading
│   ├── project.rs         # Project configuration
│   ├── context.rs         # Agent contexts
│   └── mode.rs           # Agent modes
└── Cargo.toml
```

**Configuration Types:**

```rust
// src/project.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub root: PathBuf,
    pub languages: Vec<Language>,
    pub encoding: String,
    pub read_only: bool,
    pub included_tools: Vec<String>,
    pub excluded_tools: Vec<String>,
    pub ignore_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerenaConfig {
    pub log_level: String,
    pub projects: Vec<ProjectConfig>,
    pub default_context: String,
    pub default_modes: Vec<String>,
    pub tool_timeout: Option<u64>,
    pub web_dashboard: bool,
    pub web_dashboard_port: u16,
}

impl SerenaConfig {
    pub fn load() -> Result<Self> {
        let config_dir = directories::ProjectDirs::from("dev", "oraios", "serena")
            .ok_or_else(|| ConfigError::Invalid("Cannot determine config dir".into()))?;

        let config_path = config_dir.config_dir().join("serena.yaml");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            Ok(serde_yaml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn get_project(&self, name_or_path: &str) -> Option<&ProjectConfig> {
        self.projects.iter().find(|p| {
            p.name == name_or_path || p.root.to_str() == Some(name_or_path)
        })
    }
}
```

### 7. serena-web (Optional)

**Purpose:** Web dashboard for monitoring and interaction.

**Structure:**
```
serena-web/
├── src/
│   ├── lib.rs
│   ├── server.rs          # Axum web server
│   ├── handlers.rs        # HTTP handlers
│   ├── websocket.rs       # WebSocket for live updates
│   └── static/           # Frontend assets
└── Cargo.toml
```

**Web Server:**

```rust
// src/server.rs
use axum::{
    Router,
    routing::{get, post},
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
};
use tower_http::services::ServeDir;

pub struct WebServer {
    agent: Arc<SerenaAgent>,
}

impl WebServer {
    pub async fn run(self, port: u16) -> Result<()> {
        let app = Router::new()
            .route("/api/projects", get(list_projects))
            .route("/api/tools", get(list_tools))
            .route("/api/logs", get(get_logs))
            .route("/ws", get(websocket_handler))
            .nest_service("/", ServeDir::new("static"))
            .with_state(Arc::new(self.agent));

        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await?;

        Ok(())
    }
}

async fn list_projects(
    State(agent): State<Arc<SerenaAgent>>
) -> impl IntoResponse {
    let projects = agent.list_projects();
    Json(projects)
}
```

### 8. serena-cli

**Purpose:** Command-line interface.

**Structure:**
```
serena-cli/
├── src/
│   ├── lib.rs
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── start.rs       # Start MCP server
│   │   ├── index.rs       # Index project
│   │   └── config.rs      # Config management
│   └── utils.rs
└── Cargo.toml
```

**CLI Definition:**

```rust
// src/lib.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "serena")]
#[command(about = "Serena AI Code Agent", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Config file path
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start MCP server
    Start {
        /// Project path or name
        #[arg(short, long)]
        project: Option<String>,

        /// MCP transport (stdio, http)
        #[arg(short, long, default_value = "stdio")]
        transport: String,

        /// HTTP port (if transport is http)
        #[arg(long, default_value = "3000")]
        port: u16,
    },

    /// Index a project
    Index {
        /// Project path
        path: PathBuf,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current configuration
    Show,

    /// Add a project
    AddProject {
        name: String,
        path: PathBuf,
    },

    /// List projects
    ListProjects,
}
```

### 9. serena (Main Binary)

**Purpose:** Main executable that ties everything together.

**Structure:**
```
serena/
├── src/
│   ├── main.rs
│   └── agent.rs           # SerenaAgent orchestrator
└── Cargo.toml
```

**Main Agent:**

```rust
// src/agent.rs
use serena_core::*;
use serena_lsp::LanguageServerManager;
use serena_tools::ToolRegistry;
use serena_config::SerenaConfig;
use serena_memory::MemoryStorage;

pub struct SerenaAgent {
    config: Arc<SerenaConfig>,
    project: Arc<RwLock<Option<Project>>>,
    lsp_manager: Arc<LanguageServerManager>,
    tools: Arc<ToolRegistry>,
    memory: Arc<dyn MemoryStorage>,
}

impl SerenaAgent {
    pub async fn new(config: SerenaConfig) -> Result<Self> {
        let lsp_manager = Arc::new(LanguageServerManager::new());
        let tools = Arc::new(Self::create_tool_registry());

        let memory_path = config.memory_path();
        let memory = Arc::new(SqliteStorage::new(&memory_path)?);

        Ok(Self {
            config: Arc::new(config),
            project: Arc::new(RwLock::new(None)),
            lsp_manager,
            tools,
            memory,
        })
    }

    pub async fn activate_project(&self, name_or_path: &str) -> Result<()> {
        let project_config = self.config.get_project(name_or_path)
            .ok_or_else(|| SerenaError::ProjectNotFound(name_or_path.into()))?;

        let project = Project::load(project_config.clone()).await?;

        // Start language servers for configured languages
        for language in &project_config.languages {
            self.lsp_manager.start_server(*language, &project_config.root).await?;
        }

        *self.project.write().await = Some(project);

        Ok(())
    }

    fn create_tool_registry() -> ToolRegistry {
        let mut registry = ToolRegistry::new();

        // Register all tools
        registry.register(Box::new(ReadFileTool::new()));
        registry.register(Box::new(WriteFileTool::new()));
        registry.register(Box::new(SearchFilesTool::new()));
        registry.register(Box::new(FindSymbolTool::new()));
        registry.register(Box::new(RenameSymbolTool::new()));
        // ... register all tools

        registry
    }

    pub async fn execute_tool(&self, name: &str, params: serde_json::Value) -> Result<ToolResult> {
        let tool = self.tools.get(name)
            .ok_or_else(|| SerenaError::Tool(format!("Tool not found: {}", name)))?;

        tool.execute(params).await
    }
}
```

---

## Core Trait Definitions

### Tool Trait

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult>;
    fn can_edit(&self) -> bool { false }
    fn requires_project(&self) -> bool { true }
    fn markers(&self) -> Vec<ToolMarker> { vec![] }
}
```

### Language Server Trait

```rust
#[async_trait]
pub trait LanguageServer: Send + Sync {
    async fn initialize(&mut self, root_uri: Url) -> Result<()>;
    async fn shutdown(&mut self) -> Result<()>;
    async fn document_symbols(&self, uri: &Url) -> Result<Vec<SymbolInfo>>;
    async fn find_references(&self, uri: &Url, position: Position) -> Result<Vec<Location>>;
    async fn rename(&self, uri: &Url, position: Position, new_name: String) -> Result<WorkspaceEdit>;
    async fn goto_definition(&self, uri: &Url, position: Position) -> Result<Vec<Location>>;
    async fn did_open(&self, uri: &Url, text: String, language_id: String) -> Result<()>;
    async fn did_close(&self, uri: &Url) -> Result<()>;
    async fn did_change(&self, uri: &Url, text: String) -> Result<()>;
}
```

### Memory Storage Trait

```rust
#[async_trait]
pub trait MemoryStorage: Send + Sync {
    async fn write(&self, key: &str, content: &str) -> Result<()>;
    async fn read(&self, key: &str) -> Result<Option<String>>;
    async fn list(&self) -> Result<Vec<String>>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn search(&self, query: &str) -> Result<Vec<MemoryEntry>>;
}
```

---

## Technology Stack

### Core Technologies

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| Async Runtime | tokio | Industry standard, full-featured |
| LSP Protocol | lsp-types, lsp-server | Official LSP types, proven server impl |
| Web Framework | axum | Fast, ergonomic, built on tokio |
| Serialization | serde, serde_json, serde_yaml | De facto standard |
| CLI | clap v4 | Best-in-class CLI framework |
| Error Handling | thiserror, anyhow | Library vs application errors |
| Logging | tracing | Async-aware, structured logging |
| Parallelism | rayon, dashmap | Data parallelism, concurrent maps |
| Text Editing | ropey | Efficient rope data structure |
| Database | rusqlite (bundled) | Zero-config, full-text search |
| Config | config crate | Hierarchical configuration |

### Optional Features

| Feature | Technology | Flag |
|---------|-----------|------|
| Web Dashboard | axum, tower-http | `web` |
| Advanced Search | tantivy | `search` |
| Tree-sitter | tree-sitter | `tree-sitter` |

---

## Migration Phases

### Phase 1: Foundation (Weeks 1-2)

**Goal:** Core infrastructure and CLI.

**Deliverables:**
- ✅ Workspace structure
- ✅ serena-core with error types and traits
- ✅ serena-config with YAML loading
- ✅ serena-cli with basic commands
- ✅ Build automation (Makefile.toml)

**Validation:**
- `serena config show` works
- Load and parse existing Python config files
- Cross-platform build verification

### Phase 2: LSP Client (Weeks 3-5)

**Goal:** Language server communication for 3-5 core languages.

**Deliverables:**
- ✅ Generic LSP client
- ✅ Language server lifecycle management
- ✅ Python, Rust, TypeScript server implementations
- ✅ Stdio communication
- ✅ Response caching

**Validation:**
- Start/stop language servers
- Get document symbols
- Find references
- Rename symbols

### Phase 3: File Tools (Week 6)

**Goal:** File operation tools.

**Deliverables:**
- ✅ ReadFileTool
- ✅ WriteFileTool
- ✅ SearchFilesTool
- ✅ ReplaceContentTool
- ✅ Tool registry

**Validation:**
- Read files with line ranges
- Write files preserving encoding
- Search with regex
- Replace with literal/regex

### Phase 4: Symbol Tools (Weeks 7-8)

**Goal:** Language-aware symbol operations.

**Deliverables:**
- ✅ FindSymbolTool
- ✅ RenameSymbolTool
- ✅ FindReferencesTool
- ✅ ReplaceSymbolBodyTool

**Validation:**
- Find symbols by name path
- Rename across files
- Find all references
- Replace method bodies

### Phase 5: MCP Server (Week 9)

**Goal:** Model Context Protocol server.

**Deliverables:**
- ✅ MCP protocol implementation
- ✅ Stdio transport
- ✅ Tool exposure via MCP
- ✅ Request/response handling

**Validation:**
- MCP client can list tools
- Call tools via MCP
- Error handling
- Timeout handling

### Phase 6: Memory System (Week 10)

**Goal:** Knowledge persistence.

**Deliverables:**
- ✅ SQLite storage backend
- ✅ Full-text search
- ✅ Memory tools (read, write, search)
- ✅ Markdown export/import

**Validation:**
- Write and retrieve memories
- Full-text search
- List all memories

### Phase 7: Web Dashboard (Week 11-12)

**Goal:** Optional web interface.

**Deliverables:**
- ✅ Axum web server
- ✅ WebSocket for live updates
- ✅ Log viewer
- ✅ Project browser

**Validation:**
- Dashboard loads
- View logs in real-time
- Browse project structure

### Phase 8: Multi-Language Support (Weeks 13-16)

**Goal:** Support 30+ languages.

**Deliverables:**
- ✅ Language server implementations for:
  - Go, Java, TypeScript, Vue
  - PHP, Perl, C#, Elixir
  - Bash, Ruby, Swift, Terraform
  - + 20 more languages

**Validation:**
- Each language server starts
- Symbol operations work
- Tests pass for each language

### Phase 9: Polish & Optimization (Week 17-18)

**Goal:** Production readiness.

**Deliverables:**
- ✅ Performance benchmarks
- ✅ Memory optimization
- ✅ Binary size reduction
- ✅ Comprehensive documentation
- ✅ Release automation

---

## API Specifications

### MCP Protocol

**List Tools:**
```json
Request:
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/list",
  "params": {}
}

Response:
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "tools": [
      {
        "name": "read_file",
        "description": "Reads a file from the project",
        "inputSchema": {
          "type": "object",
          "properties": {
            "relative_path": {"type": "string"}
          },
          "required": ["relative_path"]
        }
      }
    ]
  }
}
```

**Call Tool:**
```json
Request:
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "read_file",
    "arguments": {
      "relative_path": "src/main.rs"
    }
  }
}

Response:
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "fn main() {\n    println!(\"Hello, world!\");\n}"
      }
    ]
  }
}
```

### CLI API

**Start MCP Server:**
```bash
serena start --project /path/to/project --transport stdio
serena start --project my-project --transport http --port 3000
```

**Index Project:**
```bash
serena index /path/to/project
```

**Manage Config:**
```bash
serena config show
serena config add-project my-project /path/to/project
serena config list-projects
```

---

## Build and Distribution

### Build Automation (Makefile.toml)

```toml
[tasks.build]
description = "Build all crates"
command = "cargo"
args = ["build", "--workspace", "--release"]

[tasks.test]
description = "Run all tests"
command = "cargo"
args = ["test", "--workspace"]

[tasks.clippy]
description = "Run clippy"
command = "cargo"
args = ["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]

[tasks.fmt]
description = "Format code"
command = "cargo"
args = ["fmt", "--all"]

[tasks.check-all]
description = "Run all checks"
dependencies = ["fmt", "clippy", "test"]

[tasks.release]
description = "Build optimized release"
command = "cargo"
args = ["build", "--workspace", "--profile", "release-optimized"]

[tasks.cross-build-windows]
description = "Cross-compile for Windows"
command = "cross"
args = ["build", "--target", "x86_64-pc-windows-msvc", "--release"]

[tasks.cross-build-linux]
description = "Cross-compile for Linux"
command = "cross"
args = ["build", "--target", "x86_64-unknown-linux-gnu", "--release"]

[tasks.cross-build-macos]
description = "Cross-compile for macOS"
command = "cross"
args = ["build", "--target", "x86_64-apple-darwin", "--release"]

[tasks.package]
description = "Create distribution packages"
script = '''
#!/bin/bash
VERSION=$(cargo metadata --format-version=1 | jq -r '.packages[] | select(.name == "serena") | .version')

# Windows
zip -j serena-${VERSION}-windows-x64.zip target/x86_64-pc-windows-msvc/release/serena.exe

# Linux
tar czf serena-${VERSION}-linux-x64.tar.gz -C target/x86_64-unknown-linux-gnu/release serena

# macOS
tar czf serena-${VERSION}-macos-x64.tar.gz -C target/x86_64-apple-darwin/release serena
'''
```

### Cross-Platform Binary Distribution

**Targets:**
- Windows: `x86_64-pc-windows-msvc`
- Linux: `x86_64-unknown-linux-gnu`
- macOS: `x86_64-apple-darwin`, `aarch64-apple-darwin`

**Package Structure:**
```
serena-0.2.0-linux-x64/
├── serena                    # Binary
├── LICENSE
├── README.md
└── config/
    └── serena.yaml.example  # Example config
```

**GitHub Release Automation:**
```yaml
# .github/workflows/release.yml
name: Release
on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo build --release --workspace
      - run: cargo make package
      - uses: actions/upload-artifact@v3
        with:
          name: binaries-${{ matrix.os }}
          path: serena-*.{zip,tar.gz}
```

---

## Testing Strategy

### Unit Tests

**Per-crate tests:**
```rust
// crates/serena-lsp/tests/client_test.rs
#[tokio::test]
async fn test_python_language_server() {
    let client = PythonLanguageServer::new();
    client.initialize(Url::parse("file:///test").unwrap()).await.unwrap();

    let symbols = client.document_symbols(
        &Url::parse("file:///test/main.py").unwrap()
    ).await.unwrap();

    assert!(!symbols.is_empty());
}
```

### Integration Tests

**Cross-crate integration:**
```rust
// tests/integration/tool_execution.rs
#[tokio::test]
async fn test_find_symbol_via_mcp() {
    let agent = SerenaAgent::new(SerenaConfig::default()).await.unwrap();
    agent.activate_project("test-project").await.unwrap();

    let result = agent.execute_tool("find_symbol", json!({
        "name_path": "MyClass/my_method",
        "relative_path": "src/main.py"
    })).await.unwrap();

    assert_eq!(result.status, ToolStatus::Success);
}
```

### End-to-End Tests

**Full MCP workflow:**
```rust
#[tokio::test]
async fn test_mcp_stdio_workflow() {
    let mcp_server = spawn_mcp_server();

    send_request("tools/list");
    let tools = receive_response();
    assert!(tools.contains(&"read_file"));

    send_request("tools/call", json!({
        "name": "read_file",
        "arguments": {"relative_path": "test.txt"}
    }));
    let result = receive_response();
    assert!(result["content"].is_string());
}
```

### Benchmark Tests

**Performance benchmarks:**
```rust
// benches/lsp_benchmarks.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_document_symbols(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let client = runtime.block_on(async {
        PythonLanguageServer::new()
    });

    c.bench_function("document_symbols", |b| {
        b.iter(|| {
            runtime.block_on(async {
                client.document_symbols(black_box(uri)).await
            })
        })
    });
}

criterion_group!(benches, bench_document_symbols);
criterion_main!(benches);
```

### Test Matrix

| Language | Unit | Integration | E2E |
|----------|------|-------------|-----|
| Python   | ✅   | ✅          | ✅  |
| Rust     | ✅   | ✅          | ✅  |
| TypeScript | ✅ | ✅          | ✅  |
| Go       | ✅   | ✅          | ✅  |
| Java     | ✅   | ✅          | ✅  |
| ... (30+) | ✅  | ✅          | ✅  |

---

## Summary

This architecture provides:

1. **Clean Separation** - Each crate has a single responsibility
2. **Async by Default** - tokio throughout for I/O operations
3. **Trait-Based Design** - Easy to extend and test
4. **Cross-Platform** - Works on Windows, Linux, macOS
5. **Single Binary** - No Python runtime required
6. **Incremental Migration** - Build functionality piece by piece
7. **Performance** - Parallel operations, efficient caching
8. **Maintainable** - Clear module boundaries, comprehensive tests

**Next Steps:**
1. Create workspace structure (Phase 1)
2. Implement serena-core traits and types
3. Build LSP client for Python (proof of concept)
4. Implement file tools
5. Create MCP server
6. Expand language support
7. Add web dashboard
8. Optimize and release

**Timeline:** 18 weeks for complete migration with full feature parity.
