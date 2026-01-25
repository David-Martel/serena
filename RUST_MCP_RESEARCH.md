# Rust MCP Server Implementation Research & Architecture Design

**Date**: December 21, 2025
**Scope**: Research on porting Serena's MCP server from Python to pure Rust
**Status**: Research Complete, Architecture Proposed

---

## Executive Summary

A mature Rust ecosystem for MCP exists with **three production-ready options**:

1. **Official SDK (`rmcp`)** - Recommended for Serena
2. **`mcp-protocol-sdk`** - Advanced features, full 2025-06-18 spec compliance
3. **`rust-mcp-sdk`** - High-performance with built-in web server

**Key Finding**: The official Rust SDK (`rmcp@0.9.0`) is now mature (previously at 0.1), providing macro-based tool definition and multiple transports (stdio, HTTP/SSE, WebSocket).

---

## Part 1: Existing Rust MCP Implementations

### 1.1 Official Rust SDK - `rmcp` ⭐ RECOMMENDED

**Repository**: [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk)
**Current Version**: 0.9.0
**Crates.io**: [rmcp](https://crates.io/crates/rmcp)
**Documentation**: [docs.rs/rmcp](https://docs.rs/rmcp/latest/rmcp/)

**Why It's Recommended for Serena**:
- **Official implementation** - Backed by Anthropic's Model Context Protocol team
- **Zero boilerplate** - Macro system (`#[tool]`, `#[tool_box]`) eliminates protocol code
- **Tokio async** - Already in Serena's Cargo.toml dependencies
- **Multiple transports** - Stdio (LSP-like), HTTP/SSE, WebSocket support
- **Type-safe** - Pydantic-like validation via `serde` + `schemars`
- **Active maintenance** - Evolved from community fork (4t145/rmcp) to official

**Dependencies**:
```toml
rmcp = { version = "0.9.0", features = ["server", "macros", "transport-stdio"] }
tokio = "1.36"
serde = { version = "1.0", features = ["derive"] }
schemars = "0.8"  # JSON Schema generation for tool parameters
serde_json = "1.0"
anyhow = "1.0"
```

**Tool Definition Pattern**:
```rust
use rmcp::tool::{tool, Tool};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Deserialize, JsonSchema)]
struct ReadFileRequest {
    relative_path: String,
    #[schemars(description = "0-based start line")]
    start_line: Option<usize>,
}

#[tool_box]
impl FileTools {
    #[tool(description = "Read a file from the project directory")]
    pub async fn read_file(
        &self,
        #[tool(description = "Path relative to project root")]
        request: ReadFileRequest,
    ) -> anyhow::Result<String> {
        // Implementation
    }
}
```

**Server Initialization**:
```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let tools = FileTools::new(project_root);
    tools
        .serve_stdio()  // Or serve_http(addr)
        .await
}
```

---

### 1.2 Alternative: `mcp-protocol-sdk`

**Repository**: [docs.rs/mcp-protocol-sdk](https://docs.rs/mcp-protocol-sdk)
**Version**: Latest (2025-06-18 spec compliant)
**Use Case**: Advanced features like audio content, tool annotations

**Advantages Over Official SDK**:
- Full 2025-06-18 MCP specification support
- Audio/multimodal support
- Tool and content annotations
- Argument autocompletion
- Complete type coverage

**Disadvantages**:
- More verbose API (less macros)
- Larger generated binaries
- Steeper learning curve

---

### 1.3 Alternative: `rust-mcp-sdk`

**Repository**: [rust-mcp-stack/rust-mcp-sdk](https://github.com/rust-mcp-stack/rust-mcp-sdk)
**Crates.io**: [rust-mcp-sdk](https://crates.io/crates/rust-mcp-sdk)

**Advantages**:
- Built-in Axum web server
- Multiple transport layers
- Low-latency design
- Good for distributed deployments

**Disadvantages**:
- Less active than official SDK
- More overhead if you only need stdio

---

## Part 2: Current Python MCP Implementation Analysis

### 2.1 Python Implementation Structure (src/serena/mcp.py)

**Key Classes**:
- `SerenaMCPFactory` - Abstract factory for MCP server creation
- `SerenaMCPFactorySingleProcess` - Concrete implementation with in-process SerenaAgent
- Tool conversion via `make_mcp_tool()` - Wraps Serena `Tool` instances as MCP tools

**Current Tool Wrapping Logic**:
```python
def make_mcp_tool(tool: Tool, openai_tool_compatible: bool = True) -> MCPTool:
    # 1. Extract tool metadata (name, docstring, parameters)
    # 2. Generate JSON Schema from Pydantic models
    # 3. Sanitize for OpenAI compatibility (integer→number conversion)
    # 4. Parse docstring for parameter descriptions
    # 5. Create MCP tool with execution wrapper
```

**Server Lifespan**:
- Uses FastMCP's lifespan context manager
- Tools registered dynamically in startup
- Single-process execution to avoid asyncio contamination

---

### 2.2 Tool Inventory (What Must Be Exposed)

**Total Python Tool Code**: 1,930 lines across 10 files

| File | Lines | Category | Key Tools |
|------|-------|----------|-----------|
| file_tools.py | 483 | File Operations | read_file, create_text_file, edit_file, search_files, list_directory |
| symbol_tools.py | 311 | Symbol Navigation | find_symbol, get_symbols_overview, find_referencing_symbols, rename_symbol |
| tools_base.py | 429 | Base Classes | Tool, Component, ToolMarker* |
| config_tools.py | 67 | Configuration | activate_project, switch_modes |
| memory_tools.py | 90 | Knowledge | read_memory, write_memory, list_memories |
| workflow_tools.py | 138 | Onboarding | onboarding, verify_status |
| jetbrains_tools.py | 121 | IDE Integration | (JetBrains plugin specific) |
| cmd_tools.py | 52 | CLI | (Internal CLI commands) |

**Category Breakdown**:
- **File Operations** (483 lines) - 25% of codebase
- **Symbol/LSP Operations** (311 lines) - 16%
- **Base Infrastructure** (429 lines) - 22%
- **Project Management** (90 + 67 = 157 lines) - 8%
- **Workflows/IDE** (138 + 121 + 52 = 311 lines) - 16%

---

## Part 3: Tool Categories & Rust Implementation Strategy

### 3.1 File Operations (High Priority)

**Tools**:
- `read_file(relative_path, start_line?, end_line?, max_answer_chars?)`
- `create_text_file(relative_path, content)`
- `edit_file_with_regex(relative_path, pattern, replacement, mode)`
- `search_files_for_pattern(pattern, path?, glob?)`
- `list_directory(relative_path, recursive?)`
- `find_file(file_mask, relative_path)`

**Rust Implementation**:
```rust
// Leverage existing serena_core crate
use serena_core::project::Project;

#[tool_box]
impl FileTools {
    #[tool(description = "Read file content")]
    pub async fn read_file(
        &self,
        relative_path: String,
        start_line: Option<usize>,
        end_line: Option<usize>,
    ) -> anyhow::Result<String> {
        let content = self.project.read_file(&relative_path)?;
        // Apply line slicing
        Ok(content)
    }

    #[tool(description = "Search for files by pattern")]
    pub async fn find_file(
        &self,
        file_mask: String,
        relative_path: Option<String>,
    ) -> anyhow::Result<Vec<String>> {
        // Use glob patterns via walkdir
        Ok(Vec::new())
    }
}
```

**Crates Required**:
- `walkdir` - Directory traversal (already in Cargo.toml)
- `glob` - Pattern matching
- `regex` - Text search (already in Cargo.toml)
- `ignore` - Gitignore-aware filtering (already in Cargo.toml)

---

### 3.2 Symbol/LSP Operations (Medium Priority)

**Tools**:
- `get_symbols_overview(relative_path, depth?)`
- `find_symbol(name_path_pattern, relative_path?, depth?)`
- `find_referencing_symbols(name_path, relative_path, include_kinds?)`
- `rename_symbol(name_path, relative_path, new_name)`
- `replace_symbol_body(name_path, relative_path, body)`

**Challenge**: Requires integration with SolidLanguageServer (currently Python/LSP).

**Solution**:
- **Option A**: Wrap Python LSP via subprocess (keeps Python layer)
- **Option B**: Port LSP client to Rust (long-term, post-MVP)
- **Option C**: Use lsp-types Rust crate with direct LSP communication

**Recommended for MVP**: Option A (subprocess call to Python LSP layer)

```rust
#[tool_box]
impl SymbolTools {
    pub async fn find_symbol(
        &self,
        name_path_pattern: String,
        relative_path: Option<String>,
    ) -> anyhow::Result<String> {
        // Call Python LSP via tokio::process
        let output = tokio::process::Command::new("python")
            .args(&["-m", "serena.lsp", "find_symbol"])
            .arg(&name_path_pattern)
            .output()
            .await?;

        Ok(String::from_utf8(output.stdout)?)
    }
}
```

**Crates for Long-term LSP Integration**:
- `tower-lsp` - Language Server Protocol implementation
- `lsp-types` - MCP/LSP type definitions (already in Cargo.toml)

---

### 3.3 Configuration & Memory Tools (Low Priority)

**Configuration Tools**:
- `activate_project(project_name)`
- `switch_modes(modes: Vec<String>)`

**Memory Tools**:
- `read_memory(memory_file_name)`
- `write_memory(memory_file_name, content)`
- `list_memories()`

**Rust Implementation**:
```rust
#[tool_box]
impl ConfigTools {
    #[tool(description = "Activate a project for operations")]
    pub async fn activate_project(&self, project: String) -> anyhow::Result<String> {
        self.agent.activate_project(&project)?;
        Ok(format!("Project {} activated", project))
    }
}

#[tool_box]
impl MemoryTools {
    #[tool(description = "Read a memory file")]
    pub async fn read_memory(&self, memory_file_name: String) -> anyhow::Result<String> {
        let path = self.project.memories_dir().join(&memory_file_name);
        let content = tokio::fs::read_to_string(&path).await?;
        Ok(content)
    }
}
```

---

## Part 4: Proposed Rust MCP Server Architecture

### 4.1 Project Structure

```
serena_core/
├── Cargo.toml                        # Updated with rmcp, tokio, schemars
├── src/
│   ├── lib.rs                        # Re-export MCP modules
│   ├── mcp/
│   │   ├── mod.rs                    # MCP server orchestrator
│   │   ├── server.rs                 # Main MCP server
│   │   ├── tools/
│   │   │   ├── mod.rs                # Tool registry
│   │   │   ├── file_tools.rs         # File operations (~100 lines)
│   │   │   ├── symbol_tools.rs       # Symbol operations (~80 lines)
│   │   │   ├── config_tools.rs       # Config operations (~50 lines)
│   │   │   └── memory_tools.rs       # Memory operations (~40 lines)
│   │   ├── schema.rs                 # Tool parameter schemas
│   │   └── errors.rs                 # MCP-specific errors
│   └── project.rs                    # (existing) project management
└── bin/
    └── serena-mcp-server.rs          # Server binary entry point
```

### 4.2 Core MCP Server Architecture

**Design Pattern**: Async trait-based tool registry with macro derivation

```rust
// serena_core/src/mcp/server.rs
use rmcp::server::{ServerHandler, ServerInfo, CallToolResult};
use rmcp::types::{Tool, TextContent};
use serde_json::{json, Value};
use std::collections::HashMap;

pub struct SerenaMCPServer {
    tools: HashMap<String, Box<dyn ToolHandler>>,
    agent: Arc<SerenaAgent>,
    config: MCPServerConfig,
}

impl SerenaMCPServer {
    pub async fn new(
        project_root: String,
        modes: Vec<String>,
        context: String,
    ) -> anyhow::Result<Self> {
        let agent = SerenaAgent::initialize(project_root, modes, context).await?;
        let agent = Arc::new(agent);

        let mut tools: HashMap<String, Box<dyn ToolHandler>> = HashMap::new();

        // Register tool groups
        tools.extend(FileTools::register_tools(Arc::clone(&agent)));
        tools.extend(SymbolTools::register_tools(Arc::clone(&agent)));
        tools.extend(ConfigTools::register_tools(Arc::clone(&agent)));
        tools.extend(MemoryTools::register_tools(Arc::clone(&agent)));

        Ok(Self {
            tools,
            agent,
            config: MCPServerConfig::default(),
        })
    }

    pub async fn serve_stdio(self) -> anyhow::Result<()> {
        self.into_handler().serve_stdio().await
    }

    pub async fn serve_http(self, addr: String) -> anyhow::Result<()> {
        self.into_handler().serve_http(&addr.parse()?).await
    }
}

// Trait for tool implementations
#[async_trait]
pub trait ToolHandler: Send + Sync {
    async fn call(&self, arguments: Value) -> anyhow::Result<CallToolResult>;
    fn info(&self) -> Tool;
}

pub struct MCPServerConfig {
    pub encoding: String,
    pub tool_timeout: u64,
    pub max_file_size: usize,
}
```

### 4.3 Tool Implementation Pattern

**Using `rmcp` macros** (Recommended):

```rust
// serena_core/src/mcp/tools/file_tools.rs
use rmcp::tool::{tool, tool_box};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use std::sync::Arc;

#[derive(Deserialize, JsonSchema)]
struct ReadFileRequest {
    relative_path: String,
    #[schemars(description = "0-based start line (default: 0)")]
    start_line: Option<usize>,
    #[schemars(description = "0-based end line inclusive (default: EOF)")]
    end_line: Option<usize>,
    #[schemars(description = "Max characters to return (default: -1 for unlimited)")]
    max_answer_chars: Option<i32>,
}

#[tool_box]
pub struct FileTools {
    agent: Arc<SerenaAgent>,
}

impl FileTools {
    pub fn new(agent: Arc<SerenaAgent>) -> Self {
        Self { agent }
    }

    #[tool(
        description = "Read a file or chunk of it from the project directory. \
                       Use symbolic operations if you know what symbols you're looking for."
    )]
    pub async fn read_file(&self, req: ReadFileRequest) -> anyhow::Result<String> {
        let project = self.agent.get_active_project_or_raise()?;
        project.validate_relative_path(&req.relative_path)?;

        let content = tokio::fs::read_to_string(
            project.root().join(&req.relative_path)
        ).await?;

        let lines: Vec<&str> = content.lines().collect();
        let start = req.start_line.unwrap_or(0);
        let end = req.end_line.map(|e| e + 1).unwrap_or(lines.len());

        let result = lines[start..end].join("\n");

        if let Some(max_chars) = req.max_answer_chars {
            if max_chars > 0 && result.len() > max_chars as usize {
                return Err(anyhow::anyhow!(
                    "File content exceeds {max_chars} characters"
                ));
            }
        }

        Ok(result)
    }

    #[tool(description = "Create or overwrite a file in the project directory")]
    pub async fn create_text_file(
        &self,
        relative_path: String,
        content: String,
    ) -> anyhow::Result<String> {
        let project = self.agent.get_active_project_or_raise()?;
        let abs_path = project.root().join(&relative_path);

        // Create parent directories
        tokio::fs::create_dir_all(abs_path.parent().ok_or_else(||
            anyhow::anyhow!("Invalid path")
        )?).await?;

        // Write file
        tokio::fs::write(&abs_path, &content).await?;

        Ok(format!("File created: {relative_path}"))
    }

    #[tool(description = "Search for files matching a pattern")]
    pub async fn search_files(
        &self,
        pattern: String,
        include_glob: Option<String>,
        exclude_glob: Option<String>,
    ) -> anyhow::Result<Vec<String>> {
        let project = self.agent.get_active_project_or_raise()?;

        use walkdir::WalkDir;
        let mut results = Vec::new();

        for entry in WalkDir::new(project.root())
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let rel_path = path.strip_prefix(project.root())?;

            // Apply glob filters
            if let Some(ref exc) = exclude_glob {
                if glob::Pattern::new(exc)?.matches_path(rel_path) {
                    continue;
                }
            }
            if let Some(ref inc) = include_glob {
                if !glob::Pattern::new(inc)?.matches_path(rel_path) {
                    continue;
                }
            }

            // Search file content
            if let Ok(content) = tokio::fs::read_to_string(path).await {
                if regex::Regex::new(&pattern)?.is_match(&content) {
                    results.push(rel_path.to_string_lossy().to_string());
                }
            }
        }

        Ok(results)
    }
}
```

### 4.4 JSON-RPC Transport Layer

**`rmcp` handles JSON-RPC 2.0 automatically**, but here's the underlying pattern:

```rust
// Message Flow:
// 1. Stdio receives: {"jsonrpc":"2.0","method":"tools/call","params":{"name":"read_file",...}}
// 2. rmcp::Handler deserializes JSON-RPC message
// 3. Matches to tool by name
// 4. Calls tool.call(params)
// 5. Serializes result as: {"jsonrpc":"2.0","result":{...},"id":1}
// 6. Writes to stdout

// This is handled automatically by rmcp's ServerHandler trait
```

**If implementing custom JSON-RPC** (not recommended):

```rust
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

async fn handle_jsonrpc_message(
    line: String,
    tools: &ToolRegistry,
) -> anyhow::Result<()> {
    let msg: Value = serde_json::from_str(&line)?;

    if msg["method"] == "tools/call" {
        let tool_name = &msg["params"]["name"];
        let args = &msg["params"]["arguments"];

        let result = tools.call(tool_name.as_str()?, args).await?;

        let response = json!({
            "jsonrpc": "2.0",
            "result": result,
            "id": msg["id"]
        });

        println!("{}", response.to_string());
    }

    Ok(())
}
```

---

## Part 5: Implementation Roadmap

### Phase 1: Foundation (Week 1)
- [ ] Create `serena_core/src/mcp/` module structure
- [ ] Update `Cargo.toml` with rmcp dependencies
- [ ] Implement `SerenaMCPServer` orchestrator
- [ ] Set up tool registry pattern

**Output**: Minimal working Rust MCP server (no tools yet)

### Phase 2: File Tools (Week 2)
- [ ] Port file_tools.py → file_tools.rs
- [ ] Implement read_file, create_text_file, search_files
- [ ] Add directory listing and pattern matching
- [ ] Integration test with stdio transport

**Output**: File operation tools working end-to-end

### Phase 3: Configuration Tools (Week 3)
- [ ] Port config_tools.py → config_tools.rs
- [ ] Port memory_tools.py → memory_tools.rs
- [ ] Implement activate_project, switch_modes
- [ ] Memory file operations (read/write/list)

**Output**: Project management tools complete

### Phase 4: Symbol Tools Bridge (Week 4)
- [ ] Create subprocess wrapper for Python LSP
- [ ] Port symbol_tools.py interface to Rust
- [ ] Implement find_symbol, get_symbols_overview
- [ ] Error handling for LSP communication

**Output**: Symbol tools working via Python subprocess

### Phase 5: Testing & Optimization (Week 5)
- [ ] Unit tests for each tool
- [ ] Integration tests with SerenaAgent
- [ ] Performance benchmarking
- [ ] Binary size optimization

**Output**: Production-ready Rust MCP server

### Phase 6: Long-term: Native LSP (Future)
- [ ] Port SolidLanguageServer to Rust
- [ ] Eliminate Python subprocess dependency
- [ ] Direct LSP protocol implementation
- [ ] Performance improvements (10x+ faster symbol ops)

---

## Part 6: Key Architecture Decisions

### 6.1 Why `rmcp` Over Alternatives

| Criteria | rmcp | mcp-protocol-sdk | rust-mcp-sdk |
|----------|------|------------------|--------------|
| Official | ✅ Yes | ❌ No | ❌ No |
| Macros | ✅ Yes | ❌ No | ⚠️ Partial |
| Boilerplate | Minimal | High | Medium |
| Stdio Support | ✅ | ✅ | ✅ |
| Maintenance | Active | Active | Moderate |
| Learning Curve | Easy | Medium | Medium |
| Binary Size | Small | Large | Medium |
| Serialization Overhead | Low | Medium | Low |

**Winner**: `rmcp` - Best balance of simplicity and power

### 6.2 Transport Strategy

**Primary**: **Stdio** (same as LSP servers)
- Justification: SerenaAgent is already LSP-based; consistent architecture
- Performance: <1ms latency for local communication
- Security: Subprocess isolation, no exposed network

**Secondary**: **HTTP/SSE** (future)
- Enable: Remote MCP server access
- Use case: Running Serena in Docker/cloud
- Implementation: Trivial with rmcp's multi-transport design

### 6.3 Async Runtime

**Tokio 1.36+** (already in Cargo.toml)
- Native support in rmcp
- Integrated with SerenaAgent
- Consistent with existing Rust code

### 6.4 Error Handling Strategy

```rust
// Custom error type
#[derive(thiserror::Error, Debug)]
pub enum MCPError {
    #[error("Project error: {0}")]
    ProjectError(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("LSP error: {0}")]
    LSPError(String),

    #[error("Timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
}

impl From<MCPError> for rmcp::Error {
    fn from(err: MCPError) -> Self {
        rmcp::Error::Internal(err.to_string())
    }
}
```

### 6.5 LSP Integration Approach

**MVP**: Subprocess wrapper to Python LSP
```rust
async fn call_python_lsp(method: &str, args: Vec<String>) -> anyhow::Result<String> {
    let output = tokio::process::Command::new("python")
        .args(&["-m", "serena.lsp", method])
        .args(args)
        .output()
        .await?;

    Ok(String::from_utf8(output.stdout)?)
}
```

**Long-term**: Tower-LSP based Rust implementation
- Eliminate Python dependency
- 10x+ performance improvement
- Full type safety

---

## Part 7: Dependencies Summary

### 7.1 Required Crates

```toml
[dependencies]
# MCP Protocol
rmcp = { version = "0.9.0", features = ["server", "macros", "transport-stdio"] }

# Async Runtime (already in Cargo.toml)
tokio = { version = "1.36", features = ["full"] }
futures = "0.3"

# Serialization & Schema Generation
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
schemars = "0.8"  # NEW: JSON Schema for tool parameters

# File System & Search (already in Cargo.toml)
walkdir = "2.4"
regex = "1.11"
ignore = "0.4"
glob = "0.3"  # NEW: Pattern matching

# Error Handling (already in Cargo.toml)
anyhow = "1.0"
thiserror = "1.0"

# Logging (already in Cargo.toml)
tracing = "0.1"
tracing-subscriber = "0.3"

# Concurrency (already in Cargo.toml)
parking_lot = "0.12"
dashmap = "5.5"

[dev-dependencies]
# Testing
tokio-test = "0.4"
tempfile = "3.8"
```

### 7.2 What's Already Available

From existing `serena_core/Cargo.toml`:
- ✅ tokio (async runtime)
- ✅ serde (serialization)
- ✅ serde_json (JSON)
- ✅ regex (text search)
- ✅ walkdir (directory traversal)
- ✅ ignore (gitignore support)
- ✅ anyhow (error handling)
- ✅ tracing (logging)

**To Add**:
- 🆕 rmcp (MCP protocol)
- 🆕 schemars (JSON Schema generation)
- 🆕 glob (pattern matching - optional, can use regex)

---

## Part 8: Comparison: Python vs Rust MCP

### 8.1 Current Python Architecture

```
FastMCP Server (HTTP)
    ↓
Tool Converter (Pydantic→OpenAI Schema)
    ↓
SerenaAgent (in-process)
    ↓
Language Servers (LSP)
    ↓
Project Files
```

### 8.2 Proposed Rust Architecture

```
rmcp Server (Stdio)
    ↓
Tool Registry (rmcp macros)
    ↓
SerenaAgent Bindings (via PyO3 or subprocess)
    ↓
Language Servers (LSP)
    ↓
Project Files
```

### 8.3 Performance Gains (Estimated)

| Operation | Python | Rust | Improvement |
|-----------|--------|------|------------|
| Server startup | 2-3s | 0.3-0.5s | **5-10x faster** |
| Tool invocation | 50-100ms | 5-10ms | **5-10x faster** |
| File search (1MB) | 200ms | 20ms | **10x faster** |
| Symbol lookup | 150-300ms | 50-100ms | **3-5x faster** |
| Memory usage | ~200MB | ~50MB | **4x smaller** |

---

## Part 9: Migration Path (Python→Rust)

### Option A: Full Rewrite (Breaking)
- Rewrite all 1,930 lines of tool code in Rust
- Timeline: 4-6 weeks
- Risk: High (complete rewrite)
- Benefit: Best performance, cleanest code

### Option B: Hybrid Bridge (Recommended)
- Keep Python tools, wrap with Rust MCP server
- Timeline: 1-2 weeks for Rust layer
- Risk: Low (Rust layer isolated)
- Benefit: Quick MVP, gradual migration

**Bridge Implementation**:
```rust
// Python tool → Rust MCP bridge
pub async fn call_python_tool(
    tool_name: &str,
    arguments: Value,
) -> anyhow::Result<String> {
    let process = tokio::process::Command::new("python")
        .args(&["-c", &format!(
            "from serena.tools import {}; print({}.apply({:?}))",
            tool_name, tool_name, arguments
        )])
        .output()
        .await?;

    Ok(String::from_utf8(output.stdout)?)
}
```

### Option C: Gradual Porting (Safe)
- Port one tool category per week (file → symbol → config → memory)
- Python tools still available as fallback
- Timeline: 6-8 weeks
- Risk: Very low (always has fallback)

**Recommended**: **Option B** (Hybrid) for MVP, then Option C (Gradual) for full migration

---

## Part 10: Risks & Mitigations

### 10.1 Identified Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|-----------|
| rmcp API instability | Low | High | Lock version, upstream monitoring |
| Rust LSP complexity | High | High | Use subprocess bridge initially |
| Binary size bloat | Medium | Medium | Strip symbols, LTO optimization |
| Tokio async issues | Low | Medium | Extensive integration testing |
| Python subprocess overhead | Medium | Low | Plan Phase 6 native LSP |

### 10.2 Mitigation Strategies

1. **API Stability**: Pin rmcp version, maintain compatibility layer
2. **LSP Integration**: Phase subprocess bridge before native implementation
3. **Binary Size**: Use `strip` and LTO in release profile
4. **Testing**: Comprehensive integration test suite for Python↔Rust boundary
5. **Performance**: Benchmark each tool, optimize hot paths

---

## Conclusion & Recommendations

### Recommended Path Forward

1. **Immediate (Week 1-2)**:
   - Add `rmcp`, `schemars` to `Cargo.toml`
   - Implement `SerenaMCPServer` orchestrator
   - Create file_tools module with read_file, create_text_file
   - Test with basic stdio transport

2. **Short-term (Week 3-4)**:
   - Complete file operations
   - Add config/memory tools
   - Create subprocess bridge for symbol tools
   - Integration tests with SerenaAgent

3. **Medium-term (Week 5+)**:
   - Performance benchmarking
   - Optional: Gradual porting of remaining tools
   - HTTP/SSE transport support for distributed deployments

### Specific Crate Recommendations

```toml
# MUST HAVE
rmcp = { version = "0.9", features = ["server", "macros", "transport-stdio"] }
schemars = "0.8"

# NICE TO HAVE (Future)
tower-lsp = "0.20"  # For Phase 6 native LSP
axum = "0.7"        # For HTTP transport

# AVAILABLE (Use existing)
tokio = "1.36"
serde = "1.0"
walkdir = "2.4"
regex = "1.11"
```

### Success Metrics

- ✅ Rust MCP server handles all current Python tool categories
- ✅ <1s startup time vs 2-3s in Python
- ✅ <50MB binary size (stripped)
- ✅ 100% compatible with Claude Desktop, Cursor IDE
- ✅ No functionality loss from Python implementation

---

## References & Sources

### Primary Sources
- [Official Rust MCP SDK - modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk)
- [rmcp on crates.io](https://crates.io/crates/rmcp)
- [rmcp Documentation - docs.rs](https://docs.rs/rmcp/latest/rmcp/)
- [Build MCP Servers in Rust - MCPcat Guide](https://mcpcat.io/guides/building-mcp-server-rust/)

### Alternative Implementations
- [mcp-protocol-sdk - docs.rs](https://docs.rs/mcp-protocol-sdk)
- [rust-mcp-sdk - GitHub](https://github.com/rust-mcp-stack/rust-mcp-sdk)
- [async-mcp - GitHub](https://github.com/v3g42/async-mcp)
- [mcp-framework - GitHub](https://github.com/koki7o/mcp-framework)

### Educational Resources
- [Build MCP Weather Server - Paul's Blog](https://paulyu.dev/article/rust-mcp-server-weather-tutorial/)
- [Getting Started with Rust SDK - DeepWiki](https://deepwiki.com/modelcontextprotocol/rust-sdk/1.1-getting-started)
- [MCP in Rust Practical Guide - HackMD](https://hackmd.io/@Hamze/SytKkZP01l)

### JSON-RPC & Transport
- [Parity Technologies jsonrpc - GitHub](https://github.com/paritytech/jsonrpc)
- [jsonrpc-stdio-server - docs.rs](https://docs.rs/jsonrpc-stdio-server)
- [Shuttle: Rust stdio MCP Server - Blog](https://www.shuttle.dev/blog/2025/07/18/how-to-build-a-stdio-mcp-server-in-rust)

### Protocol Specification
- [Model Context Protocol Official Spec](https://spec.modelcontextprotocol.io/)

---

**Document Version**: 1.0
**Research Completed**: December 21, 2025
**Ready for Implementation**: Yes ✅
