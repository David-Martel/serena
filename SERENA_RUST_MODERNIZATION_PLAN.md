# Serena Complete Rust Modernization Plan

**Version:** 1.0
**Date:** 2025-12-21
**Status:** Final Plan - Ready for Implementation
**Objective:** 100% Python to Pure Rust Migration (No PyO3, No Docker)

---

## Executive Summary

This document presents a comprehensive plan to completely replace Serena's Python codebase with pure Rust, eliminating all Python, PyO3, and Docker dependencies. The result will be a single, high-performance binary that can be distributed across Windows, Linux, and macOS without runtime dependencies.

### Key Outcomes
- **Performance**: 5-10x faster startup, 5-10x faster operations
- **Memory**: 4x smaller footprint (~50MB vs ~200MB)
- **Distribution**: Single binary, no Python/Node.js runtime required
- **Maintenance**: Strong type safety, compile-time error detection

### Scope Summary
| Metric | Current Python | Target Rust |
|--------|---------------|-------------|
| Lines of Code | ~35,103 | ~22,000-25,000 |
| Language Servers | 40+ | 40+ (feature parity) |
| Binary Size | N/A (interpreter) | <50MB (stripped) |
| Startup Time | 2-4 seconds | 0.2-0.5 seconds |
| Memory Usage | 200-500MB | 50-100MB |

---

## Table of Contents

1. [Current State Analysis](#1-current-state-analysis)
2. [PyO3 Removal Strategy](#2-pyo3-removal-strategy)
3. [MCP Protocol Implementation](#3-mcp-protocol-implementation)
4. [Multi-Crate Workspace Architecture](#4-multi-crate-workspace-architecture)
5. [Docker Elimination Strategy](#5-docker-elimination-strategy)
6. [Component Migration Plan](#6-component-migration-plan)
7. [Phased Implementation Roadmap](#7-phased-implementation-roadmap)
8. [Testing Strategy](#8-testing-strategy)
9. [Build and Distribution](#9-build-and-distribution)
10. [Risk Mitigation](#10-risk-mitigation)

---

## 1. Current State Analysis

### 1.1 Existing Rust Code (serena_core)

The existing `serena_core` crate contains **reusable components** that form the foundation for the pure Rust implementation:

**Fully Reusable (100%)**:
- `serena_core/src/lsp/client.rs` - Async LSP client implementation
- `serena_core/src/symbol_graph/mod.rs` - Symbol indexing with DashMap
- `serena_core/src/web/mod.rs` - Axum-based dashboard server

**Requires Modification (Remove PyO3)**:
- `serena_core/src/lib.rs` - Remove `#[pyfunction]`, `#[pymodule]` decorators
- `serena_core/src/project_host.rs` - Remove `#[pyclass]`, `#[pymethods]`

**Current Cargo Dependencies (Keep)**:
```toml
# Already Available
tokio = { version = "1.41", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
lsp-types = "0.98"
regex = "1.11"
walkdir = "2.5"
ignore = "0.4"
dashmap = "6.1"
rayon = "1.10"
anyhow = "1.0"
tracing = "0.1"
axum = "0.7"
```

### 1.2 Python Codebase Structure

| Component | Python Lines | Migration Complexity |
|-----------|-------------|---------------------|
| SerenaAgent (agent.py) | ~800 | VERY HIGH |
| MCP Server (mcp.py) | ~600 | VERY HIGH |
| SolidLanguageServer (ls.py) | ~1,200 | VERY HIGH |
| Tool System (tools/*.py) | ~3,000 | HIGH |
| Config System (config/*.py) | ~1,500 | MEDIUM |
| Language Servers (40+ files) | ~8,000 | MEDIUM |
| Other utilities | ~20,000 | MEDIUM |

---

## 2. PyO3 Removal Strategy

### 2.1 Files Requiring PyO3 Removal

**serena_core/src/lib.rs** - Current PyO3 bindings:
```rust
// REMOVE ALL OF THESE:
#[pyfunction]
fn search_files(...) -> PyResult<Vec<PyObject>>

#[pyfunction]
fn walk_files_gitignored(...) -> PyResult<Vec<String>>

#[pyfunction]
fn find_symbol(...) -> PyResult<Vec<PyObject>>

#[pymodule]
fn serena_core(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()>
```

**serena_core/src/project_host.rs** - Current PyO3 class:
```rust
// REMOVE ALL OF THESE:
#[pyclass]
pub struct ProjectHost { ... }

#[pymethods]
impl ProjectHost { ... }
```

### 2.2 Conversion Strategy

Replace PyO3 with pure Rust traits and types:

```rust
// BEFORE (PyO3)
#[pyfunction]
fn search_files(py: Python<'_>, ...) -> PyResult<Vec<PyObject>>

// AFTER (Pure Rust)
pub async fn search_files(...) -> Result<Vec<FileMatch>>
```

```rust
// BEFORE (PyO3)
#[pyclass]
pub struct ProjectHost { ... }

// AFTER (Pure Rust)
pub struct ProjectHost { ... }

impl ProjectHost {
    pub async fn definition(&self, path: &str, line: u32, char: u32) -> Result<Vec<Location>>
}
```

### 2.3 PyO3 Dependencies to Remove

Update `serena_core/Cargo.toml`:
```toml
# REMOVE THESE:
[dependencies]
pyo3 = { version = "0.21", features = ["extension-module"] }

[lib]
crate-type = ["cdylib", "rlib"]  # Remove "cdylib"

# KEEP THESE:
[lib]
crate-type = ["rlib"]  # Pure Rust library
```

---

## 3. MCP Protocol Implementation

### 3.1 Recommended SDK: rmcp v0.9.0

The official Rust MCP SDK provides the best balance of features and maintainability:

```toml
[dependencies]
rmcp = { version = "0.9.0", features = ["server", "macros", "transport-stdio"] }
schemars = "0.8"  # JSON Schema generation for tool parameters
```

### 3.2 Tool Definition Pattern

```rust
use rmcp::tool::{tool, tool_box};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Deserialize, JsonSchema)]
struct ReadFileRequest {
    relative_path: String,
    #[schemars(description = "0-based start line")]
    start_line: Option<usize>,
    #[schemars(description = "0-based end line (inclusive)")]
    end_line: Option<usize>,
}

#[tool_box]
pub struct FileTools {
    agent: Arc<SerenaAgent>,
}

impl FileTools {
    #[tool(description = "Read file content from the project directory")]
    pub async fn read_file(&self, req: ReadFileRequest) -> anyhow::Result<String> {
        let project = self.agent.get_active_project()?;
        let content = tokio::fs::read_to_string(
            project.root().join(&req.relative_path)
        ).await?;

        // Apply line slicing
        let lines: Vec<&str> = content.lines().collect();
        let start = req.start_line.unwrap_or(0);
        let end = req.end_line.map(|e| e + 1).unwrap_or(lines.len());

        Ok(lines[start..end].join("\n"))
    }
}
```

### 3.3 MCP Server Architecture

```rust
// serena-mcp/src/server.rs
pub struct SerenaMCPServer {
    tools: Arc<ToolRegistry>,
    agent: Arc<SerenaAgent>,
}

impl SerenaMCPServer {
    pub async fn new(config: SerenaConfig) -> Result<Self> {
        let agent = Arc::new(SerenaAgent::new(config).await?);
        let tools = Arc::new(ToolRegistry::new(&agent));

        Ok(Self { tools, agent })
    }

    pub async fn serve_stdio(self) -> Result<()> {
        // rmcp handles JSON-RPC 2.0 automatically
        self.into_handler().serve_stdio().await
    }

    pub async fn serve_http(self, port: u16) -> Result<()> {
        self.into_handler().serve_http(&format!("0.0.0.0:{}", port).parse()?).await
    }
}
```

---

## 4. Multi-Crate Workspace Architecture

### 4.1 Workspace Layout

```
serena/
├── Cargo.toml                    # Workspace root
├── Makefile.toml                 # cargo-make automation
├── .cargo/
│   └── config.toml               # Shared compiler config
├── crates/
│   ├── serena-core/              # Core types, traits, errors
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs          # SerenaError, LspError, ConfigError
│   │       ├── types.rs          # SymbolInfo, Location, Range, Position
│   │       └── traits/
│   │           ├── mod.rs
│   │           ├── tool.rs       # Tool trait
│   │           ├── lsp.rs        # LanguageServer trait
│   │           └── storage.rs    # MemoryStorage trait
│   │
│   ├── serena-lsp/               # LSP client implementation
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs         # Generic LSP client
│   │       ├── manager.rs        # Language server lifecycle
│   │       ├── cache.rs          # Response caching
│   │       └── languages/        # 40+ language server configs
│   │
│   ├── serena-tools/             # Tool implementations
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── registry.rs       # Tool registration
│   │       ├── file/             # File operations
│   │       ├── symbol/           # Symbol operations
│   │       ├── memory/           # Knowledge persistence
│   │       └── config/           # Project management
│   │
│   ├── serena-mcp/               # MCP server protocol
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── server.rs         # MCP server
│   │       └── transport/        # Stdio, HTTP transports
│   │
│   ├── serena-memory/            # Knowledge persistence
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── storage.rs        # SQLite backend
│   │       └── search.rs         # Full-text search
│   │
│   ├── serena-config/            # Configuration management
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── loader.rs         # YAML loading
│   │       ├── project.rs        # Project config
│   │       └── mode.rs           # Context/mode system
│   │
│   ├── serena-web/               # Web dashboard (optional)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── server.rs         # Axum server
│   │       └── handlers.rs       # HTTP handlers
│   │
│   ├── serena-cli/               # CLI interface
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── commands/         # CLI commands
│   │
│   └── serena/                   # Main binary
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs           # Entry point
│           └── agent.rs          # SerenaAgent orchestrator
│
├── tests/                        # Integration tests
└── benches/                      # Performance benchmarks
```

### 4.2 Workspace Cargo.toml

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

# MCP Protocol
rmcp = { version = "0.9.0", features = ["server", "macros", "transport-stdio"] }

# LSP Protocol
lsp-types = "0.98"
lsp-server = "0.7"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
schemars = "0.8"

# Web server (optional)
axum = { version = "0.7", optional = true }
tower = { version = "0.4", features = ["util", "timeout"] }
tower-http = { version = "0.5", features = ["fs", "trace", "cors"] }

# File system
ignore = "0.4"
walkdir = "2.5"
glob = "0.3"

# Text processing
regex = "1.11"
ropey = "1.6"

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

# CLI
clap = { version = "4.5", features = ["derive", "env", "color"] }

# Database
rusqlite = { version = "0.32", features = ["bundled"] }

# Utilities
chrono = "0.4"
uuid = { version = "1.11", features = ["v4", "serde"] }
once_cell = "1.20"
directories = "5.0"

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = true

[profile.release-optimized]
inherits = "release"
lto = "fat"
panic = "abort"
```

### 4.3 Core Traits

```rust
// serena-core/src/traits/tool.rs
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    async fn execute(&self, params: Value) -> Result<ToolResult>;
    fn can_edit(&self) -> bool { false }
    fn requires_project(&self) -> bool { true }
}

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub status: ToolStatus,
    pub data: Value,
    pub error: Option<String>,
}
```

```rust
// serena-core/src/traits/lsp.rs
use async_trait::async_trait;
use lsp_types::*;

#[async_trait]
pub trait LanguageServer: Send + Sync {
    async fn initialize(&mut self, root_uri: Url) -> Result<()>;
    async fn shutdown(&mut self) -> Result<()>;
    async fn document_symbols(&self, uri: &Url) -> Result<Vec<SymbolInfo>>;
    async fn find_references(&self, uri: &Url, pos: Position) -> Result<Vec<Location>>;
    async fn rename(&self, uri: &Url, pos: Position, new_name: String) -> Result<WorkspaceEdit>;
    async fn goto_definition(&self, uri: &Url, pos: Position) -> Result<Vec<Location>>;
    async fn did_open(&self, uri: &Url, text: String, lang_id: String) -> Result<()>;
    async fn did_close(&self, uri: &Url) -> Result<()>;
    async fn did_change(&self, uri: &Url, text: String) -> Result<()>;
}
```

---

## 5. Docker Elimination Strategy

### 5.1 Current Docker Dependencies

The existing codebase may reference Docker for:
- Development environment
- CI/CD pipelines
- Language server isolation

### 5.2 Docker-Free Alternatives

| Use Case | Docker Approach | Pure Rust Approach |
|----------|----------------|-------------------|
| Language Server Isolation | Docker containers | Native process spawning with `tokio::process` |
| CI/CD Builds | Docker images | GitHub Actions with cross-compilation |
| Development Environment | docker-compose | `cargo-make` + native tooling |
| Database | Docker PostgreSQL | Embedded SQLite via `rusqlite` (bundled) |
| Testing | Docker test containers | Native test harness |

### 5.3 Native Process Management

Replace Docker container spawning with native process management:

```rust
// Native language server spawning (no Docker)
pub async fn spawn_language_server(
    command: &str,
    args: &[String],
    root_path: &Path,
) -> Result<LspClient> {
    let child = tokio::process::Command::new(command)
        .args(args)
        .current_dir(root_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    LspClient::from_child(child).await
}
```

### 5.4 Embedded Database

Use bundled SQLite instead of Docker PostgreSQL:

```toml
[dependencies]
rusqlite = { version = "0.32", features = ["bundled"] }
```

```rust
// Zero-config embedded database
let db_path = config_dir.join("serena.db");
let conn = Connection::open(&db_path)?;
```

---

## 6. Component Migration Plan

### 6.1 File Tools (serena-tools/src/file/)

**Python Source**: `src/serena/tools/file_tools.py` (~483 lines)

| Tool | Rust Implementation | Complexity |
|------|-------------------|------------|
| `read_file` | `tokio::fs::read_to_string` + line slicing | LOW |
| `create_text_file` | `tokio::fs::write` | LOW |
| `replace_content` | `regex::Regex` + file rewrite | MEDIUM |
| `search_for_pattern` | `ignore::Walk` + parallel regex | MEDIUM |
| `list_dir` | `walkdir::WalkDir` | LOW |
| `find_file` | `glob::glob` | LOW |

### 6.2 Symbol Tools (serena-tools/src/symbol/)

**Python Source**: `src/serena/tools/symbol_tools.py` (~311 lines)

| Tool | Dependencies | Complexity |
|------|-------------|------------|
| `get_symbols_overview` | LSP `documentSymbol` request | MEDIUM |
| `find_symbol` | LSP + name path matching | HIGH |
| `find_referencing_symbols` | LSP `references` request | MEDIUM |
| `rename_symbol` | LSP `rename` request | MEDIUM |
| `replace_symbol_body` | LSP location + file editing | HIGH |
| `insert_before_symbol` | LSP location + file editing | HIGH |
| `insert_after_symbol` | LSP location + file editing | HIGH |

### 6.3 Memory Tools (serena-memory/)

**Python Source**: `src/serena/tools/memory_tools.py` (~90 lines)

```rust
// SQLite-based memory storage
#[async_trait]
pub trait MemoryStorage: Send + Sync {
    async fn write(&self, key: &str, content: &str) -> Result<()>;
    async fn read(&self, key: &str) -> Result<Option<String>>;
    async fn list(&self) -> Result<Vec<String>>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn search(&self, query: &str) -> Result<Vec<MemoryEntry>>;
}

pub struct SqliteStorage {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteStorage {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
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

        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }
}
```

### 6.4 Config Tools (serena-config/)

**Python Source**: `src/serena/tools/config_tools.py` (~67 lines)

```rust
// Configuration loading
#[derive(Debug, Deserialize, Serialize)]
pub struct SerenaConfig {
    pub log_level: String,
    pub projects: Vec<ProjectConfig>,
    pub default_context: String,
    pub default_modes: Vec<String>,
}

impl SerenaConfig {
    pub fn load() -> Result<Self> {
        let dirs = directories::ProjectDirs::from("dev", "oraios", "serena")
            .ok_or_else(|| anyhow::anyhow!("Cannot determine config directory"))?;

        let config_path = dirs.config_dir().join("serena.yaml");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            Ok(serde_yaml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }
}
```

### 6.5 Language Server Implementations

Port all 40+ language server configurations:

```rust
// serena-lsp/src/languages/mod.rs
pub mod bash;
pub mod csharp;
pub mod elixir;
pub mod go;
pub mod java;
pub mod perl;
pub mod php;
pub mod python;
pub mod ruby;
pub mod rust;
pub mod swift;
pub mod terraform;
pub mod typescript;
pub mod vue;
// ... 26+ more

// Factory pattern
pub fn create_server(language: Language) -> Box<dyn LanguageServerProvider> {
    match language {
        Language::Python => Box::new(python::PythonServer),
        Language::Rust => Box::new(rust::RustServer),
        Language::TypeScript => Box::new(typescript::TypeScriptServer),
        Language::Go => Box::new(go::GoServer),
        // ... 36+ more
    }
}
```

---

## 7. Phased Implementation Roadmap

### Phase 1: Foundation (Weeks 1-2)
**Goal**: Core infrastructure and workspace setup

**Deliverables**:
- [ ] Create multi-crate workspace structure
- [ ] Implement `serena-core` with error types and traits
- [ ] Port `serena-config` from Python
- [ ] Remove PyO3 from existing Rust code
- [ ] Set up cargo-make build automation

**Validation**:
- `cargo build --workspace` succeeds
- Configuration loading works
- No PyO3 dependencies remain

### Phase 2: File Operations (Week 3)
**Goal**: Complete file tool implementation

**Deliverables**:
- [ ] `read_file` with line slicing
- [ ] `create_text_file` with directory creation
- [ ] `replace_content` with regex/literal modes
- [ ] `search_for_pattern` with parallel execution
- [ ] `list_dir` with recursive option
- [ ] `find_file` with glob patterns

**Validation**:
- All file tools pass unit tests
- Performance benchmarks vs Python baseline

### Phase 3: LSP Client (Weeks 4-6)
**Goal**: Language server communication

**Deliverables**:
- [ ] Generic LSP client implementation
- [ ] Process lifecycle management (spawn, restart, kill)
- [ ] Stdio communication with JSON-RPC 2.0
- [ ] Response caching layer
- [ ] Initial 5 language servers (Python, Rust, TypeScript, Go, Java)

**Validation**:
- Start/stop language servers
- Get document symbols
- Find references
- Rename symbols across files

### Phase 4: Symbol Tools (Weeks 7-8)
**Goal**: Language-aware symbol operations

**Deliverables**:
- [ ] `get_symbols_overview` with depth parameter
- [ ] `find_symbol` with name path matching
- [ ] `find_referencing_symbols`
- [ ] `rename_symbol`
- [ ] `replace_symbol_body`
- [ ] `insert_before_symbol` / `insert_after_symbol`

**Validation**:
- Symbol operations work across all 5 initial languages
- Snapshot tests for symbol editing

### Phase 5: MCP Server (Week 9)
**Goal**: Model Context Protocol server

**Deliverables**:
- [ ] MCP protocol implementation using `rmcp`
- [ ] Stdio transport
- [ ] HTTP transport (optional)
- [ ] Tool exposure via MCP
- [ ] Error handling and timeouts

**Validation**:
- Claude Desktop can connect and list tools
- Tool invocations return correct results
- Error messages are informative

### Phase 6: Memory System (Week 10)
**Goal**: Knowledge persistence

**Deliverables**:
- [ ] SQLite storage backend
- [ ] Full-text search with FTS5
- [ ] Markdown import/export
- [ ] Memory tools (read, write, list, search, delete)

**Validation**:
- Memories persist across restarts
- Full-text search returns relevant results

### Phase 7: Web Dashboard (Weeks 11-12)
**Goal**: Optional monitoring interface

**Deliverables**:
- [ ] Axum web server
- [ ] Project browser API
- [ ] Log viewer with WebSocket streaming
- [ ] Tool execution history

**Validation**:
- Dashboard loads in browser
- Real-time log streaming works

### Phase 8: Multi-Language Support (Weeks 13-16)
**Goal**: All 40+ language servers

**Deliverables**:
- [ ] Remaining 35+ language server implementations
- [ ] Language detection
- [ ] Per-language configuration
- [ ] Comprehensive integration tests

**Validation**:
- Each language server starts successfully
- Symbol operations work for each language

### Phase 9: Polish & Optimization (Weeks 17-18)
**Goal**: Production readiness

**Deliverables**:
- [ ] Performance benchmarking (criterion.rs)
- [ ] Memory profiling
- [ ] Binary size optimization (strip, LTO)
- [ ] Cross-platform testing (Windows, Linux, macOS)
- [ ] Documentation
- [ ] Release automation

**Validation**:
- All benchmarks meet targets
- Single binary <50MB
- Tests pass on all platforms

---

## 8. Testing Strategy

### 8.1 Unit Tests

```rust
// Per-crate tests
#[tokio::test]
async fn test_read_file_with_line_range() {
    let tools = FileTools::new_test();
    let result = tools.read_file(ReadFileRequest {
        relative_path: "test.txt".into(),
        start_line: Some(5),
        end_line: Some(10),
    }).await.unwrap();

    assert_eq!(result.lines().count(), 6);
}
```

### 8.2 Integration Tests

```rust
// Cross-crate integration
#[tokio::test]
async fn test_symbol_find_via_mcp() {
    let server = SerenaMCPServer::new_test().await.unwrap();

    let result = server.call_tool("find_symbol", json!({
        "name_path_pattern": "MyClass/my_method",
        "relative_path": "src/main.py"
    })).await.unwrap();

    assert!(result["symbols"].as_array().unwrap().len() > 0);
}
```

### 8.3 Language Server Tests

```rust
// Per-language tests with markers
#[tokio::test]
#[cfg(feature = "test-python")]
async fn test_python_language_server() {
    let server = PythonServer::spawn(&test_project_path()).await.unwrap();

    let symbols = server.document_symbols(
        &Url::parse("file:///test/main.py").unwrap()
    ).await.unwrap();

    assert!(!symbols.is_empty());
}
```

### 8.4 Benchmark Tests

```rust
// Performance benchmarks
use criterion::{black_box, criterion_group, Criterion};

fn bench_file_search(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let tools = FileTools::new(test_project());

    c.bench_function("search_files_large_project", |b| {
        b.iter(|| {
            runtime.block_on(async {
                tools.search_files(black_box("pattern"), None, None).await
            })
        })
    });
}

criterion_group!(benches, bench_file_search);
```

---

## 9. Build and Distribution

### 9.1 Build Automation (Makefile.toml)

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

[tasks.release]
description = "Build optimized release"
command = "cargo"
args = ["build", "--workspace", "--profile", "release-optimized"]

[tasks.package-windows]
description = "Create Windows package"
script = '''
cargo build --release --target x86_64-pc-windows-msvc
strip target/x86_64-pc-windows-msvc/release/serena.exe
'''

[tasks.package-linux]
description = "Create Linux package"
script = '''
cargo build --release --target x86_64-unknown-linux-gnu
strip target/x86_64-unknown-linux-gnu/release/serena
'''

[tasks.package-macos]
description = "Create macOS package"
script = '''
cargo build --release --target x86_64-apple-darwin
strip target/x86_64-apple-darwin/release/serena
'''

[tasks.package-all]
dependencies = ["package-windows", "package-linux", "package-macos"]
```

### 9.2 GitHub Actions Release

```yaml
name: Release
on:
  push:
    tags: ['v*']

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            ext: .exe
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            ext: ''
          - os: macos-latest
            target: x86_64-apple-darwin
            ext: ''

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - run: cargo build --release --target ${{ matrix.target }}

      - name: Strip binary
        run: strip target/${{ matrix.target }}/release/serena${{ matrix.ext }}
        if: matrix.os != 'windows-latest'

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: serena-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/serena${{ matrix.ext }}
```

### 9.3 Binary Distribution

**Package Structure**:
```
serena-0.2.0-linux-x64/
├── serena                    # Single binary (~30-50MB)
├── LICENSE
├── README.md
└── config/
    └── serena.yaml.example
```

**No Runtime Dependencies**:
- No Python interpreter
- No Node.js runtime
- No Docker
- No external language runtimes (servers are auto-downloaded)

---

## 10. Risk Mitigation

### 10.1 Risk Matrix

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|-----------|
| rmcp API instability | LOW | HIGH | Lock version, monitor upstream |
| LSP client complexity | MEDIUM | HIGH | Reuse existing `lsp/client.rs` |
| Language server coverage | MEDIUM | MEDIUM | Prioritize top 10, gradual expansion |
| Performance regression | LOW | MEDIUM | Continuous benchmarking |
| Cross-platform issues | MEDIUM | MEDIUM | CI testing on all platforms |
| Binary size bloat | LOW | LOW | LTO, strip symbols |

### 10.2 Mitigation Strategies

1. **API Stability**: Pin `rmcp = "=0.9.0"` in Cargo.toml
2. **LSP Robustness**: Extensive error handling, automatic server restart
3. **Language Coverage**: Focus on Python, Rust, TypeScript first
4. **Performance**: Benchmark every phase, optimize hot paths
5. **Cross-Platform**: Test Windows, Linux, macOS in CI
6. **Binary Size**: Use `strip = true`, `lto = "fat"`, `panic = "abort"`

### 10.3 Fallback Options

If `rmcp` proves unsuitable:
- **Alternative 1**: `mcp-protocol-sdk` (more verbose but complete)
- **Alternative 2**: Custom JSON-RPC server with `axum`

---

## Summary

This plan provides a complete roadmap for migrating Serena from Python to pure Rust:

| Aspect | Approach |
|--------|----------|
| **PyO3 Removal** | Remove all `#[py*]` decorators, convert to pure Rust traits |
| **MCP Protocol** | Use `rmcp` v0.9.0 with macro-based tool definitions |
| **Architecture** | 9-crate workspace for clean separation |
| **Docker Elimination** | Native process spawning, embedded SQLite |
| **Timeline** | 18 weeks for complete migration |
| **Distribution** | Single binary, no runtime dependencies |

**Expected Outcomes**:
- 5-10x faster startup
- 5-10x faster operations
- 4x smaller memory footprint
- Zero external runtime dependencies
- Single binary distribution

---

*Document Generated: 2025-12-21*
*Based on research from: rust-pro, python-pro, search-specialist, backend-architect agents*
*Ready for Implementation: Yes*
