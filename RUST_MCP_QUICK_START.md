# Rust MCP Server - Quick Start & Decision Guide

## Executive Quick Reference

### Decision Matrix: Which Rust MCP SDK?

```
RECOMMENDATION FOR SERENA:
╔════════════════════════════════════════════════════════════════╗
║  USE: rmcp (Official Rust MCP SDK)                             ║
║  Version: 0.9.0+                                               ║
║  Repository: github.com/modelcontextprotocol/rust-sdk         ║
║  Crates.io: crates.io/crates/rmcp                            ║
╚════════════════════════════════════════════════════════════════╝
```

**Why rmcp**:
- ✅ Official Anthropic-backed implementation
- ✅ Macro-based tool definition (minimal boilerplate)
- ✅ Multiple transports (stdio, HTTP/SSE, WebSocket)
- ✅ Already use tokio in serena_core
- ✅ Production-ready and actively maintained
- ✅ Smallest binary footprint
- ✅ Fastest startup time

---

## Quick Comparison Table

| Feature | rmcp | mcp-protocol-sdk | rust-mcp-sdk | jsonrpc-stdio |
|---------|------|------------------|--------------|----------------|
| **Official** | ✅ | ❌ | ❌ | ❌ |
| **Version** | 0.9.0 | 2025-06-18 | Latest | Deprecated |
| **Macros** | ✅ | ❌ | ⚠️ | N/A |
| **Boilerplate** | Minimal | High | Medium | High |
| **Stdio** | ✅ | ✅ | ✅ | ✅ |
| **HTTP/SSE** | ✅ | ✅ | ✅ | ❌ |
| **WebSocket** | ✅ | ✅ | ⚠️ | ❌ |
| **Binary Size** | Small | Large | Medium | Medium |
| **Learning Curve** | Easy | Hard | Medium | Medium |
| **Tokio Support** | ✅ | ✅ | ✅ | ✅ |
| **Type Safety** | High | High | High | Medium |
| **Maintenance** | Active | Active | Moderate | Unmaintained |

**WINNER**: `rmcp` for Serena MVP

---

## Implementation Estimate

| Component | Effort | Time | Dependencies |
|-----------|--------|------|--------------|
| **Foundation** | Low | 3-4 days | rmcp, schemars |
| **File Tools** | Medium | 4-5 days | walkdir, regex |
| **Config Tools** | Low | 1-2 days | None new |
| **Memory Tools** | Low | 1-2 days | tokio::fs |
| **Symbol Bridge** | Medium | 2-3 days | Python subprocess |
| **Testing** | Medium | 3-4 days | tokio-test, tempfile |
| **Integration** | Medium | 2-3 days | SerenaAgent bindings |
| **Total** | Medium | 3-4 weeks | |

---

## Current State Analysis

### Python MCP Implementation Footprint

```
src/serena/mcp.py
├── FastMCP Server (HTTP)
├── Tool Converter (Pydantic→OpenAI)
├── SerenaAgent (in-process)
└── Language Servers (LSP)

Total: ~350 lines of server glue code
Supporting: ~1,930 lines of Python tools
```

### Proposed Rust Architecture

```
serena_core/src/mcp/
├── server.rs (rmcp handler) - 150 lines
├── tools/
│   ├── file_tools.rs - 250 lines
│   ├── symbol_tools.rs - 200 lines
│   ├── config_tools.rs - 80 lines
│   ├── memory_tools.rs - 60 lines
│   └── python_bridge.rs - 100 lines
├── errors.rs - 60 lines
└── schema.rs - 80 lines

Total: ~1,000 lines of Rust (but 4-5x performance)
```

---

## Dependency Changes Required

### Add to serena_core/Cargo.toml

```toml
# MCP Protocol Implementation
rmcp = { version = "0.9", features = ["server", "macros", "transport-stdio"] }

# JSON Schema generation for tool parameters
schemars = { version = "0.8", features = ["preserve_order"] }

# Already present - good!
# tokio = "1.36" ✅
# serde = "1.0" ✅
# regex = "1.11" ✅
# walkdir = "2.4" ✅
# anyhow = "1.0" ✅
```

### Optional (for future phases)

```toml
# Phase 2: HTTP transport
axum = "0.7"

# Phase 3: Native LSP instead of Python bridge
tower-lsp = "0.20"

# Testing dependencies
[dev-dependencies]
tokio-test = "0.4"
tempfile = "3.8"
```

---

## Tool Implementation Examples

### Example 1: Simple Tool (read_file)

```rust
use rmcp::types::{Tool, CallToolResult, Content};
use serde_json::Value;
use async_trait::async_trait;

pub struct ReadFileTool {
    // Dependencies
}

#[async_trait]
impl ToolHandler for ReadFileTool {
    async fn call(&self, arguments: Value) -> anyhow::Result<CallToolResult> {
        // Deserialize arguments
        let req: ReadFileRequest = serde_json::from_value(arguments)?;

        // Execute
        let content = tokio::fs::read_to_string(
            self.project.root().join(&req.relative_path)
        ).await?;

        // Respond
        Ok(CallToolResult {
            content: vec![Content::text(content)],
            is_error: false,
        })
    }

    fn info(&self) -> Tool {
        Tool {
            name: "read_file".to_string(),
            description: Some("Read a file from the project".to_string()),
            inputSchema: schemars::schema_for!(ReadFileRequest),
        }
    }
}
```

### Example 2: Tool with Parameters (search_files)

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, JsonSchema)]
pub struct SearchFilesRequest {
    /// Pattern to search for (regex)
    pub pattern: String,

    /// Path to search in (default: project root)
    #[serde(default)]
    pub path: Option<String>,
}

#[async_trait]
impl ToolHandler for SearchFilesTool {
    async fn call(&self, arguments: Value) -> anyhow::Result<CallToolResult> {
        let req: SearchFilesRequest = serde_json::from_value(arguments)?;

        // Search logic using regex + walkdir
        let results = self.search_impl(&req).await?;

        Ok(CallToolResult {
            content: vec![Content::text(
                serde_json::to_string_pretty(&results)?
            )],
            is_error: false,
        })
    }

    fn info(&self) -> Tool {
        Tool {
            name: "search_files".to_string(),
            description: Some("Search for files matching a pattern".to_string()),
            inputSchema: schemars::schema_for!(SearchFilesRequest),
        }
    }
}
```

---

## Testing Template

### Unit Test

```rust
#[tokio::test]
async fn test_read_file() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, "hello world").unwrap();

    let tool = ReadFileTool::new(MockProject::new(temp_dir.path()));

    let result = tool.call(serde_json::json!({
        "relative_path": "test.txt"
    })).await.unwrap();

    assert!(!result.is_error);
    assert!(result.content[0].text().contains("hello world"));
}
```

### Integration Test

```rust
#[tokio::test]
async fn test_mcp_server_with_real_agent() {
    let config = MCPServerConfig {
        project_root: "/tmp/test".to_string(),
        ..Default::default()
    };

    let server = SerenaMCPServer::new(config).await.unwrap();

    // Verify tools are registered
    assert!(server.tools.get("read_file").is_ok());
    assert!(server.tools.get("search_files").is_ok());
    assert!(server.tools.get("activate_project").is_ok());
}
```

---

## Performance Expectations

### Startup Time
- **Python MCP**: 2-3 seconds
- **Rust MCP**: 0.3-0.5 seconds
- **Improvement**: 5-10x faster

### Tool Execution (per operation)
| Tool | Python | Rust | Speedup |
|------|--------|------|---------|
| read_file | 50ms | 5ms | 10x |
| search_files (1MB) | 200ms | 20ms | 10x |
| activate_project | 100ms | 10ms | 10x |
| list_directory | 80ms | 8ms | 10x |

### Memory Usage
- **Python MCP**: ~200MB resident
- **Rust MCP**: ~50MB resident
- **Improvement**: 4x smaller

---

## Transport Options

### Option 1: Stdio (Recommended for MVP)
- Same pattern as LSP servers
- Subprocess communication
- No exposed ports
- Perfect for IDE integration

**Implementation**:
```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = SerenaMCPServer::new(config).await?;
    server.serve_stdio().await
}
```

### Option 2: HTTP/SSE (Future)
- Remote server access
- Cloud deployments
- Web-based clients

**Implementation** (future):
```rust
let server = SerenaMCPServer::new(config).await?;
server.serve_http("127.0.0.1:8080").await
```

### Option 3: WebSocket (Long-term)
- Full-duplex communication
- Real-time tool updates
- Browser-based IDE

---

## Migration Path Options

### Quick Win Path (Weeks 1-2)
```
Week 1: Foundation + File Tools (Rust)
  ├─ Create serena_core/src/mcp/ module
  ├─ Implement read_file, create_text_file
  └─ Pass basic integration tests

Week 2: Config + Memory Tools (Rust)
  ├─ Port config_tools.rs
  ├─ Port memory_tools.rs
  └─ Create subprocess bridge for symbol tools
```

### Gradual Migration Path (Weeks 3-4)
```
Week 3: Python Bridge + Testing
  ├─ Create Python subprocess wrapper
  ├─ Port symbol_tools interface
  └─ Comprehensive integration tests

Week 4: Optimization + Docs
  ├─ Performance tuning
  ├─ Binary optimization
  └─ Documentation
```

### Long-term Path (Future)
```
Phase 2: Native LSP (No Python dependency)
  ├─ Use tower-lsp crate
  ├─ Direct LSP protocol handling
  └─ 10x performance improvement for symbol ops

Phase 3: Distributed Deployment
  ├─ HTTP/SSE transport
  ├─ Cloud-ready architecture
  └─ Multiple concurrent clients
```

---

## Common Gotchas & Solutions

### Gotcha 1: Async Runtime Conflicts

**Problem**: Rust MCP server runs in tokio; SerenaAgent might use different runtime

**Solution**:
```rust
// Use Arc<SerenaAgent> shared across all async contexts
let agent = Arc::new(SerenaAgent::initialize(...).await?);

// All tools clone the Arc
let tools = FileTools::new(Arc::clone(&agent));
```

### Gotcha 2: JSON Schema Generation

**Problem**: Tool parameters need valid JSON Schema for OpenAI compatibility

**Solution**:
```rust
// Use schemars derive + JsonSchema trait
#[derive(Deserialize, JsonSchema)]
pub struct MyRequest {
    #[schemars(description = "User-friendly description")]
    pub field: String,
}

// rmcp automatically generates schema from JsonSchema impl
```

### Gotcha 3: Large File Handling

**Problem**: Reading 100MB+ files causes memory issues

**Solution**:
```rust
// Implement size limits in tool
if content.len() > self.max_file_size {
    return Err(anyhow::anyhow!("File too large"));
}

// Support streaming for large files
pub async fn read_file_streamed(
    &self,
    start_line: usize,
    end_line: usize,
) -> anyhow::Result<String> {
    // Only read requested range
}
```

### Gotcha 4: Python Subprocess Communication

**Problem**: Subprocess calls add latency and complexity

**Solution**:
```rust
// Cache Python subprocess in PhoneBox
pub struct PythonBridge {
    cached_python: Option<String>,
}

// For MVP, accept subprocess overhead
// Plan Phase 2 native LSP integration

// Timeout subprocess calls
let output = tokio::time::timeout(
    Duration::from_millis(5000),
    Command::new("python").output()
).await??;
```

---

## Validation Checklist

Before marking MVP complete, verify:

- [ ] Rust MCP server starts in <1s
- [ ] All file tools working (read, write, search, list)
- [ ] Configuration tools working (activate_project, switch_modes)
- [ ] Memory tools working (read/write/list memories)
- [ ] Symbol tools bridge to Python (with timeout)
- [ ] Error handling graceful (no panics)
- [ ] JSON-RPC protocol 100% compliant
- [ ] Compatible with Claude Desktop
- [ ] Compatible with Cursor IDE
- [ ] Binary <20MB (stripped)
- [ ] Unit tests passing (>80% coverage)
- [ ] Integration tests passing
- [ ] No memory leaks (valgrind/miri check)

---

## Crate Versions (Pinned)

```toml
[dependencies]
# Core MCP Protocol
rmcp = "0.9"                    # Official SDK
schemars = "0.8"                # JSON Schema generation

# Already in serena_core
tokio = "1.36"                  # Async runtime
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"              # JSON
regex = "1.11"                  # Pattern matching
walkdir = "2.4"                 # Directory traversal
ignore = "0.4"                  # Gitignore support
anyhow = "1.0"                  # Error handling
thiserror = "1.0"               # Error types
parking_lot = "0.12"            # Fast mutex
dashmap = "5.5"                 # Concurrent hashmap

[dev-dependencies]
tempfile = "3.8"                # Testing temp files
tokio-test = "0.4"              # Tokio testing utilities
```

---

## Next Steps

### Immediate (Weeks 1-2)
1. ✅ Research complete (you are here)
2. Create `serena_core/src/mcp/` module structure
3. Add rmcp + schemars to Cargo.toml
4. Implement SerenaMCPServer orchestrator
5. Port file_tools with read_file as MVP

### Short-term (Weeks 3-4)
6. Complete file tools (create, search, list)
7. Add config & memory tools
8. Create Python subprocess bridge
9. Comprehensive testing

### Medium-term (Weeks 5+)
10. Performance optimization
11. HTTP transport support
12. Begin gradual tool porting
13. Documentation

---

## Key Resources

- [Official Rust MCP SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [rmcp Documentation](https://docs.rs/rmcp/latest/rmcp/)
- [MCP Specification](https://spec.modelcontextprotocol.io/)
- [Tokio Async Programming](https://tokio.rs/)
- [Schemars JSON Schema](https://docs.rs/schemars/latest/schemars/)

---

**Status**: Ready for Implementation ✅
**Recommendation**: Start with Phase 1 (Foundation + File Tools)
**Estimated Effort**: 3-4 weeks for full MVP
**Expected Outcome**: 5-10x performance improvement, drop-in Python replacement
