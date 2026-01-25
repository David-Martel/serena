# Rust MCP Server Architecture - Detailed Design Document

## Table of Contents
1. [Core Architecture](#core-architecture)
2. [Module Structure](#module-structure)
3. [Tool Implementation Patterns](#tool-implementation-patterns)
4. [Async Execution Model](#async-execution-model)
5. [Error Handling](#error-handling)
6. [Testing Strategy](#testing-strategy)
7. [Integration Points](#integration-points)

---

## Core Architecture

### 1.1 System Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Claude/IDE Client                         │
│                  (Claude Desktop, Cursor)                    │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  │ Stdio (JSON-RPC 2.0)
                  │
┌─────────────────▼───────────────────────────────────────────┐
│              rmcp::ServerHandler                             │
│        (Automatic JSON-RPC message handling)                 │
└─────────────────┬───────────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────────┐
│            SerenaMCPServer                                   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ Tool Registry (HashMap<String, Box<ToolHandler>>)   │   │
│  │  - file_tools                                        │   │
│  │  - symbol_tools                                      │   │
│  │  - config_tools                                      │   │
│  │  - memory_tools                                      │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                               │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ SerenaAgent (Arc<>)                                  │   │
│  │  - Project management                               │   │
│  │  - Language server interface                        │   │
│  │  - Configuration/modes                              │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────┬───────────────────────────────────────────┘
                  │
        ┌─────────┼─────────┬──────────┐
        │         │         │          │
        ▼         ▼         ▼          ▼
    Project  Language  File System Memory
    Config   Servers   Operations  Files
```

### 1.2 Data Flow

```
1. Client Request (Stdio)
   │
   ├─→ "{"jsonrpc":"2.0","method":"tools/call",
   │     "params":{"name":"read_file","arguments":{...}}}"
   │
2. rmcp deserialization
   │
   ├─→ Identifies tool: "read_file"
   ├─→ Validates arguments against schema
   │
3. Tool Lookup & Execution
   │
   ├─→ SerenaMCPServer.tools["read_file"].call(arguments)
   ├─→ FileTools::read_file(req).await
   │
4. Response Generation
   │
   ├─→ Result serialization
   ├─→ {"jsonrpc":"2.0","result":{...},"id":1}
   │
5. Stdio Output
   │
   └─→ Client receives result
```

---

## Module Structure

### 2.1 Directory Layout

```
serena_core/
├── Cargo.toml
├── src/
│   ├── lib.rs                           # Public API exports
│   ├── project.rs                       # (existing) Project management
│   ├── symbol.rs                        # (existing) Symbol operations
│   │
│   ├── mcp/                             # NEW: MCP Server
│   │   ├── mod.rs                       # Module exports
│   │   ├── server.rs                    # Main server orchestrator
│   │   ├── config.rs                    # Server configuration
│   │   ├── errors.rs                    # MCP-specific errors
│   │   ├── schema.rs                    # Schema helpers
│   │   │
│   │   ├── tools/                       # Tool implementations
│   │   │   ├── mod.rs                   # Tool registry
│   │   │   ├── trait.rs                 # ToolHandler trait
│   │   │   ├── file_tools.rs            # File operations
│   │   │   ├── symbol_tools.rs          # Symbol operations
│   │   │   ├── config_tools.rs          # Config operations
│   │   │   ├── memory_tools.rs          # Memory operations
│   │   │   └── python_bridge.rs         # Python subprocess wrapper
│   │   │
│   │   └── transports/                  # Transport implementations
│   │       ├── mod.rs
│   │       ├── stdio.rs                 # (from rmcp)
│   │       └── http.rs                  # (future)
│   │
│   └── bin/
│       └── serena-mcp-server/
│           └── main.rs                  # Server binary entry point
│
├── tests/
│   ├── integration_tests.rs
│   ├── tools/
│   │   ├── file_tools_tests.rs
│   │   ├── config_tools_tests.rs
│   │   └── symbol_tools_tests.rs
│   └── fixtures/                        # Test data
│
└── benches/
    ├── tool_performance.rs              # Benchmarks
    └── schema_generation.rs
```

### 2.2 Key Files with Estimated LOC

| File | Estimated LOC | Purpose |
|------|---------------|---------|
| server.rs | 150 | Main server, tool registry, lifecycle |
| tools/trait.rs | 50 | ToolHandler trait definition |
| tools/file_tools.rs | 250 | File operations (7 tools) |
| tools/symbol_tools.rs | 200 | Symbol ops (5 tools, LSP bridge) |
| tools/config_tools.rs | 80 | Config/modes (3 tools) |
| tools/memory_tools.rs | 60 | Memory operations (3 tools) |
| tools/python_bridge.rs | 100 | Python subprocess wrapper |
| schema.rs | 80 | Schema generation helpers |
| errors.rs | 60 | Error types |
| config.rs | 70 | Server configuration |
| **Total** | **~1,100** | **Estimated implementation** |

---

## Tool Implementation Patterns

### 3.1 Base Tool Trait

```rust
// serena_core/src/mcp/tools/trait.rs

use async_trait::async_trait;
use rmcp::types::{Tool, CallToolResult};
use serde_json::Value;
use std::sync::Arc;

/// Core trait for MCP tool implementations
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Execute the tool with the provided arguments
    async fn call(&self, arguments: Value) -> anyhow::Result<CallToolResult>;

    /// Get tool metadata (name, description, schema)
    fn info(&self) -> Tool;

    /// Optional: Validate arguments before execution
    fn validate_arguments(&self, arguments: &Value) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Builder for constructing ToolHandler implementations
pub struct ToolBuilder {
    name: String,
    description: String,
    // ... other metadata
}

impl ToolBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn build(self) -> anyhow::Result<Box<dyn ToolHandler>> {
        // Implementation-specific build logic
        Ok(Box::new(self))
    }
}
```

### 3.2 File Tools Implementation

```rust
// serena_core/src/mcp/tools/file_tools.rs

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use std::sync::Arc;
use regex::Regex;
use walkdir::WalkDir;
use tokio::fs;

use crate::project::Project;
use super::{ToolHandler, CallToolResult};

// =============================================================================
// REQUEST/RESPONSE TYPES (Serde + JSON Schema)
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(crate = "serde")]
pub struct ReadFileRequest {
    /// Path relative to project root
    pub relative_path: String,

    /// 0-based start line (default: 0)
    #[serde(default)]
    pub start_line: Option<usize>,

    /// 0-based end line inclusive (default: EOF)
    #[serde(default)]
    pub end_line: Option<usize>,

    /// Max characters to return (-1 = unlimited)
    #[serde(default)]
    pub max_answer_chars: Option<i32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(crate = "serde")]
pub struct CreateTextFileRequest {
    /// Path relative to project root
    pub relative_path: String,

    /// File content to write
    pub content: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(crate = "serde")]
pub struct SearchFilesRequest {
    /// Regex pattern to search for
    pub pattern: String,

    /// Path to search in (default: project root)
    #[serde(default)]
    pub path: Option<String>,

    /// Glob pattern to include (default: all)
    #[serde(default)]
    pub include_glob: Option<String>,

    /// Glob pattern to exclude (default: none)
    #[serde(default)]
    pub exclude_glob: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub files_found: usize,
    pub matches: Vec<String>,
}

// =============================================================================
// TOOL GROUP
// =============================================================================

pub struct FileTools {
    agent: Arc<SerenaAgent>,
    project: Arc<Project>,
}

impl FileTools {
    pub fn new(agent: Arc<SerenaAgent>, project: Arc<Project>) -> Self {
        Self { agent, project }
    }

    /// Get all tools in this group
    pub fn tools() -> Vec<(&'static str, Box<dyn Fn(Arc<SerenaAgent>, Arc<Project>) -> Box<dyn ToolHandler>>)> {
        vec![
            ("read_file", Box::new(|a, p| Box::new(ReadFileTool::new(a, p)))),
            ("create_text_file", Box::new(|a, p| Box::new(CreateTextFileTool::new(a, p)))),
            ("search_files_for_pattern", Box::new(|a, p| Box::new(SearchFilesTool::new(a, p)))),
            // ... more tools
        ]
    }
}

// =============================================================================
// INDIVIDUAL TOOL IMPLEMENTATIONS
// =============================================================================

pub struct ReadFileTool {
    agent: Arc<SerenaAgent>,
    project: Arc<Project>,
}

impl ReadFileTool {
    fn new(agent: Arc<SerenaAgent>, project: Arc<Project>) -> Self {
        Self { agent, project }
    }

    async fn read_file_impl(&self, req: ReadFileRequest) -> anyhow::Result<String> {
        // Validate relative path
        self.project.validate_relative_path(&req.relative_path)?;

        let abs_path = self.project.root().join(&req.relative_path);
        let content = fs::read_to_string(&abs_path).await?;

        // Apply line slicing
        let lines: Vec<&str> = content.lines().collect();
        let start = req.start_line.unwrap_or(0);
        let end = req
            .end_line
            .map(|e| (e + 1).min(lines.len()))
            .unwrap_or(lines.len());

        if start >= lines.len() {
            return Ok(String::new());
        }

        let result = lines[start..end].join("\n");

        // Limit response size
        if let Some(max) = req.max_answer_chars {
            if max > 0 && result.len() > max as usize {
                return Err(anyhow::anyhow!(
                    "Content exceeds limit of {} characters (got {})",
                    max,
                    result.len()
                ));
            }
        }

        Ok(result)
    }
}

#[async_trait]
impl ToolHandler for ReadFileTool {
    async fn call(&self, arguments: Value) -> anyhow::Result<CallToolResult> {
        let req: ReadFileRequest = serde_json::from_value(arguments)?;
        let text = self.read_file_impl(req).await?;

        Ok(CallToolResult {
            content: vec![rmcp::types::Content::text(text)],
            is_error: false,
        })
    }

    fn info(&self) -> rmcp::types::Tool {
        rmcp::types::Tool {
            name: "read_file".to_string(),
            description: Some(
                "Reads the given file or a chunk of it. Generally, symbolic operations \
                like find_symbol or find_referencing_symbols should be preferred if you \
                know which symbols you are looking for.".to_string()
            ),
            inputSchema: schemars::schema_for!(ReadFileRequest),
        }
    }

    fn validate_arguments(&self, arguments: &Value) -> anyhow::Result<()> {
        serde_json::from_value::<ReadFileRequest>(arguments.clone())?;
        Ok(())
    }
}

// Similar implementations for:
// - CreateTextFileTool
// - SearchFilesTool
// - etc.
```

### 3.3 Symbol Tools with Python Bridge

```rust
// serena_core/src/mcp/tools/python_bridge.rs

use tokio::process::Command;
use serde_json::Value;

/// Wrapper for calling Python tools via subprocess
pub struct PythonBridge {
    python_executable: String,
}

impl PythonBridge {
    pub fn new(python_executable: Option<String>) -> Self {
        Self {
            python_executable: python_executable.unwrap_or_else(|| "python".to_string()),
        }
    }

    /// Call a Python tool module and return JSON result
    pub async fn call_tool(
        &self,
        module: &str,
        tool_name: &str,
        arguments: &Value,
    ) -> anyhow::Result<Value> {
        // Build command: python -m serena.tools.<module> <tool> <json_args>
        let args_json = serde_json::to_string(arguments)?;

        let output = Command::new(&self.python_executable)
            .args(&[
                "-m",
                &format!("serena.tools.{}", module),
                tool_name,
            ])
            .arg(&args_json)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Python tool failed: {}", stderr));
        }

        let stdout = String::from_utf8(output.stdout)?;
        let result = serde_json::from_str(&stdout)?;
        Ok(result)
    }

    /// Get tool schema from Python module
    pub async fn get_tool_schema(
        &self,
        module: &str,
        tool_name: &str,
    ) -> anyhow::Result<Value> {
        let output = Command::new(&self.python_executable)
            .args(&[
                "-c",
                &format!(
                    "from serena.tools.{} import {}; import json; print(json.dumps({{}}.__schema__))",
                    module, tool_name
                ),
            ])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get schema"));
        }

        let stdout = String::from_utf8(output.stdout)?;
        let schema = serde_json::from_str(&stdout)?;
        Ok(schema)
    }
}

// Usage in symbol_tools.rs:
pub struct SymbolToolsBridge {
    bridge: PythonBridge,
}

impl SymbolToolsBridge {
    pub async fn find_symbol(&self, name_path: &str) -> anyhow::Result<String> {
        let args = serde_json::json!({
            "name_path_pattern": name_path
        });

        let result = self.bridge
            .call_tool("symbol_tools", "find_symbol", &args)
            .await?;

        Ok(result.to_string())
    }
}
```

### 3.4 Configuration Tools

```rust
// serena_core/src/mcp/tools/config_tools.rs

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use std::sync::Arc;

use super::ToolHandler;

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(crate = "serde")]
pub struct ActivateProjectRequest {
    /// Project name or path to project directory
    pub project: String,
}

pub struct ActivateProjectTool {
    agent: Arc<SerenaAgent>,
}

impl ActivateProjectTool {
    pub fn new(agent: Arc<SerenaAgent>) -> Self {
        Self { agent }
    }
}

#[async_trait]
impl ToolHandler for ActivateProjectTool {
    async fn call(&self, arguments: Value) -> anyhow::Result<CallToolResult> {
        let req: ActivateProjectRequest = serde_json::from_value(arguments)?;

        self.agent.activate_project(&req.project)?;

        Ok(CallToolResult {
            content: vec![rmcp::types::Content::text(
                format!("Project '{}' activated", req.project)
            )],
            is_error: false,
        })
    }

    fn info(&self) -> rmcp::types::Tool {
        rmcp::types::Tool {
            name: "activate_project".to_string(),
            description: Some(
                "Activate a project for code operations. Either provide an absolute path \
                to the project directory or a name of an already registered project.".to_string()
            ),
            inputSchema: schemars::schema_for!(ActivateProjectRequest),
        }
    }
}
```

---

## Async Execution Model

### 4.1 Tokio Runtime Integration

```rust
// serena_core/src/mcp/server.rs

use tokio::runtime::Runtime;
use rmcp::server::{ServerHandler, ServerInfo};
use std::sync::Arc;

pub struct SerenaMCPServer {
    runtime: Runtime,
    tools: Arc<ToolRegistry>,
    agent: Arc<SerenaAgent>,
}

impl SerenaMCPServer {
    /// Initialize the server with async runtime
    pub async fn new(config: MCPServerConfig) -> anyhow::Result<Self> {
        // Initialize agent
        let agent = SerenaAgent::initialize(
            config.project_root,
            config.modes,
            config.context,
        ).await?;

        let agent = Arc::new(agent);

        // Register tools
        let mut tools = ToolRegistry::new();
        tools.register_file_tools(Arc::clone(&agent))?;
        tools.register_config_tools(Arc::clone(&agent))?;
        tools.register_memory_tools(Arc::clone(&agent))?;
        tools.register_symbol_tools(Arc::clone(&agent))?;

        let tools = Arc::new(tools);

        Ok(Self {
            runtime: Runtime::new()?,
            tools,
            agent,
        })
    }

    /// Start server with stdio transport
    pub async fn serve_stdio(self) -> anyhow::Result<()> {
        let handler = SerenaMCPHandler {
            tools: self.tools,
        };

        // rmcp handles JSON-RPC stdio automatically
        handler.serve_stdio().await
    }
}

/// Handler implementing MCP server interface
pub struct SerenaMCPHandler {
    tools: Arc<ToolRegistry>,
}

#[async_trait]
impl ServerHandler for SerenaMCPHandler {
    async fn list_tools(&self) -> anyhow::Result<Vec<Tool>> {
        Ok(self.tools.list_all())
    }

    async fn call_tool(
        &self,
        name: String,
        arguments: Value,
    ) -> anyhow::Result<CallToolResult> {
        let tool = self.tools.get(&name)?;
        tool.call(arguments).await
    }

    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "serena".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}
```

### 4.2 Timeout Handling

```rust
// serena_core/src/mcp/server.rs

use tokio::time::timeout;
use std::time::Duration;

pub async fn call_tool_with_timeout(
    tool: &dyn ToolHandler,
    arguments: Value,
    timeout_ms: u64,
) -> anyhow::Result<CallToolResult> {
    let duration = Duration::from_millis(timeout_ms);

    match timeout(duration, tool.call(arguments)).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(anyhow::anyhow!(
            "Tool execution timed out after {}ms",
            timeout_ms
        )),
    }
}
```

### 4.3 Concurrent Tool Execution

```rust
// Multiple tools can be called concurrently
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = SerenaMCPServer::new(config).await?;

    // rmcp handles concurrent JSON-RPC requests automatically
    // Each incoming request spawns a new task
    server.serve_stdio().await
}

// Example: Parallel tool calls (if needed)
pub async fn call_multiple_tools(
    tools: &ToolRegistry,
    requests: Vec<(String, Value)>,
) -> Vec<anyhow::Result<CallToolResult>> {
    let futures = requests
        .into_iter()
        .map(|(name, args)| {
            let tool = tools.get(&name).cloned();
            async move {
                match tool {
                    Ok(t) => t.call(args).await,
                    Err(e) => Err(e),
                }
            }
        });

    futures::future::join_all(futures).await
}
```

---

## Error Handling

### 5.1 Error Type Hierarchy

```rust
// serena_core/src/mcp/errors.rs

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MCPError {
    #[error("Project error: {0}")]
    ProjectError(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Invalid file encoding")]
    EncodingError,

    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),

    #[error("Language server error: {0}")]
    LSPError(String),

    #[error("Timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Invalid tool arguments: {0}")]
    InvalidArguments(String),

    #[error("Python subprocess error: {0}")]
    PythonError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<MCPError> for rmcp::Error {
    fn from(err: MCPError) -> Self {
        rmcp::Error::Internal(err.to_string())
    }
}

impl From<std::io::Error> for MCPError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => {
                MCPError::FileNotFound(err.to_string())
            }
            _ => MCPError::Internal(err.to_string()),
        }
    }
}

impl From<serde_json::Error> for MCPError {
    fn from(err: serde_json::Error) -> Self {
        MCPError::InvalidArguments(err.to_string())
    }
}
```

### 5.2 Error Response Handling

```rust
// In tool implementations
#[async_trait]
impl ToolHandler for MyTool {
    async fn call(&self, arguments: Value) -> anyhow::Result<CallToolResult> {
        match self.execute(arguments).await {
            Ok(content) => {
                Ok(CallToolResult {
                    content: vec![rmcp::types::Content::text(content)],
                    is_error: false,
                })
            }
            Err(e) => {
                // Return error as content with is_error flag
                Ok(CallToolResult {
                    content: vec![rmcp::types::Content::text(e.to_string())],
                    is_error: true,
                })
            }
        }
    }
}
```

---

## Testing Strategy

### 6.1 Unit Test Template

```rust
// serena_core/tests/tools/file_tools_tests.rs

#[cfg(test)]
mod file_tools_tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_read_file_success() {
        // Setup
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "hello world").unwrap();

        let project = MockProject::new(temp_dir.path());
        let tool = ReadFileTool::new(Arc::new(project));

        // Execute
        let req = ReadFileRequest {
            relative_path: "test.txt".to_string(),
            start_line: None,
            end_line: None,
            max_answer_chars: None,
        };

        let result = tool
            .call(serde_json::to_value(req).unwrap())
            .await
            .unwrap();

        // Assert
        assert!(!result.is_error);
        assert!(result.content[0].text().contains("hello world"));
    }

    #[tokio::test]
    async fn test_read_file_line_range() {
        // Test line slicing functionality
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("multiline.txt");
        std::fs::write(&file_path, "line1\nline2\nline3\nline4").unwrap();

        let project = MockProject::new(temp_dir.path());
        let tool = ReadFileTool::new(Arc::new(project));

        let req = ReadFileRequest {
            relative_path: "multiline.txt".to_string(),
            start_line: Some(1),
            end_line: Some(2),
            max_answer_chars: None,
        };

        let result = tool
            .call(serde_json::to_value(req).unwrap())
            .await
            .unwrap();

        let content = result.content[0].text();
        assert!(content.contains("line2"));
        assert!(content.contains("line3"));
        assert!(!content.contains("line1"));
    }

    #[tokio::test]
    async fn test_read_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let project = MockProject::new(temp_dir.path());
        let tool = ReadFileTool::new(Arc::new(project));

        let req = ReadFileRequest {
            relative_path: "nonexistent.txt".to_string(),
            start_line: None,
            end_line: None,
            max_answer_chars: None,
        };

        let result = tool
            .call(serde_json::to_value(req).unwrap())
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_file_size_limit() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("large.txt");
        let large_content = "x".repeat(1000);
        std::fs::write(&file_path, large_content).unwrap();

        let project = MockProject::new(temp_dir.path());
        let tool = ReadFileTool::new(Arc::new(project));

        let req = ReadFileRequest {
            relative_path: "large.txt".to_string(),
            start_line: None,
            end_line: None,
            max_answer_chars: Some(100),
        };

        let result = tool
            .call(serde_json::to_value(req).unwrap())
            .await;

        assert!(result.is_err()); // Should exceed size limit
    }
}
```

### 6.2 Integration Test

```rust
// serena_core/tests/integration_tests.rs

#[tokio::test]
async fn test_mcp_server_lifecycle() {
    let config = MCPServerConfig {
        project_root: "/tmp/test_project".to_string(),
        modes: vec!["default".to_string()],
        context: "agent".to_string(),
        tool_timeout: 5000,
    };

    // Initialize server
    let server = SerenaMCPServer::new(config).await.unwrap();

    // Verify tools are registered
    assert!(server.tools.get("read_file").is_ok());
    assert!(server.tools.get("activate_project").is_ok());
    assert!(server.tools.get("read_memory").is_ok());
}

#[tokio::test]
async fn test_tool_registry_lookup() {
    let registry = ToolRegistry::new();

    // Should find registered tool
    let tool = registry.get("read_file");
    assert!(tool.is_ok());

    // Should fail for unregistered tool
    let tool = registry.get("nonexistent_tool");
    assert!(tool.is_err());
}
```

---

## Integration Points

### 7.1 SerenaAgent Integration

```rust
// serena_core/src/mcp/server.rs

/// Integration point with SerenaAgent
pub struct SerenaMCPServer {
    agent: Arc<SerenaAgent>,
    // ... other fields
}

// Tool groups access agent capabilities
pub trait AgentCapabilities: Send + Sync {
    fn get_active_project(&self) -> anyhow::Result<Arc<Project>>;
    fn get_language_server_manager(&self) -> anyhow::Result<Arc<LanguageServerManager>>;
    fn get_context(&self) -> Arc<SerenaContext>;
    fn get_modes(&self) -> Vec<String>;
}

impl AgentCapabilities for SerenaAgent {
    fn get_active_project(&self) -> anyhow::Result<Arc<Project>> {
        // Forward to agent
        self.get_active_project_or_raise().map(Arc::new)
    }

    fn get_language_server_manager(&self) -> anyhow::Result<Arc<LanguageServerManager>> {
        self.get_language_server_manager_or_raise().map(Arc::new)
    }

    // ... implement others
}
```

### 7.2 Language Server Integration

```rust
// serena_core/src/mcp/tools/symbol_tools.rs

/// Bridge to Python LSP layer
pub struct LSPBridge {
    lsp_manager: Arc<LanguageServerManager>,
}

impl LSPBridge {
    pub async fn find_symbol(
        &self,
        relative_path: &str,
        name_pattern: &str,
    ) -> anyhow::Result<Vec<SymbolInfo>> {
        // Call Python LSP via JSON-RPC
        let request = serde_json::json!({
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": format!("file://{}", relative_path)
                }
            }
        });

        let response = self.lsp_manager.send_request(request).await?;
        // Parse response...
        Ok(Vec::new())
    }
}
```

### 7.3 Project Management Integration

```rust
// serena_core/src/mcp/tools/config_tools.rs

impl ConfigTools {
    pub async fn activate_project(&self, project: &str) -> anyhow::Result<()> {
        // Leverage agent's project management
        self.agent.activate_project(project)?;
        Ok(())
    }

    pub async fn switch_modes(&self, modes: Vec<String>) -> anyhow::Result<()> {
        self.agent.set_modes(modes)?;
        Ok(())
    }
}
```

---

## Configuration Management

### 8.1 Server Configuration Structure

```rust
// serena_core/src/mcp/config.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    /// Project root directory
    pub project_root: String,

    /// Initial modes to activate
    pub modes: Vec<String>,

    /// Context name or path
    pub context: String,

    /// Tool execution timeout in milliseconds
    pub tool_timeout: u64,

    /// Maximum file size to read (bytes)
    pub max_file_size: usize,

    /// Enable logging
    pub enable_logging: bool,

    /// Log level
    pub log_level: String,
}

impl Default for MCPServerConfig {
    fn default() -> Self {
        Self {
            project_root: std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_default(),
            modes: vec!["default".to_string()],
            context: "agent".to_string(),
            tool_timeout: 5000,
            max_file_size: 10 * 1024 * 1024, // 10 MB
            enable_logging: true,
            log_level: "info".to_string(),
        }
    }
}

impl MCPServerConfig {
    /// Load from environment variables
    pub fn from_env() -> anyhow::Result<Self> {
        let mut config = Self::default();

        if let Ok(root) = std::env::var("SERENA_PROJECT_ROOT") {
            config.project_root = root;
        }
        if let Ok(modes) = std::env::var("SERENA_MODES") {
            config.modes = modes.split(',').map(|s| s.to_string()).collect();
        }
        if let Ok(context) = std::env::var("SERENA_CONTEXT") {
            config.context = context;
        }

        Ok(config)
    }

    /// Load from YAML file
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}
```

---

## Summary: Key Implementation Points

| Component | Approach | Complexity | Est. LOC |
|-----------|----------|-----------|---------|
| Tool Trait | Async trait w/ rmcp | Low | 50 |
| File Tools | Direct impl, async I/O | Medium | 250 |
| Config Tools | Agent delegation | Low | 80 |
| Memory Tools | File system operations | Low | 60 |
| Symbol Tools | Python subprocess bridge | Medium | 200 |
| Server Orchestrator | rmcp handler impl | Medium | 150 |
| Error Handling | Custom error types | Low | 60 |
| Testing | tokio + tempfile | Medium | 300+ |
| **TOTAL** | | | **~1,150** |

This architecture prioritizes:
- ✅ **Type Safety** - Pydantic-like validation via serde + schemars
- ✅ **Async/Await** - Full tokio integration
- ✅ **Maintainability** - Clear trait-based abstraction
- ✅ **Performance** - Direct Rust operations where possible
- ✅ **Compatibility** - 100% MCP spec compliance via rmcp
- ✅ **Migration Path** - Python bridge enables gradual porting

---

**Document Version**: 1.0
**Architecture Status**: Ready for Implementation
**Recommended Start**: Phase 1 (Foundation + File Tools)
