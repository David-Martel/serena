# Serena Rust MCP Server Implementation - Project Context

**Created:** 2025-12-21
**Version:** 1.0.0
**Status:** Implementation Complete - Testing Phase
**Context Type:** Completed Implementation Details

---

## 1. Project Overview

### Project Definition
- **Project:** Serena - A semantic coding toolkit with MCP server interface
- **Goal:** Pure Rust MCP server implementation in serena_core crate
- **Technology:** rust-mcp-sdk v0.7, tokio async runtime, lsp-types
- **Location:** `T:\projects\serena-source\serena_core\`

### Architecture Summary
- Single-crate implementation within existing serena_core workspace member
- Service layer pattern separating MCP protocol from business logic
- LSP client integration for symbol-aware operations
- Markdown-based memory persistence for knowledge storage

---

## 2. Current State - COMPLETED IMPLEMENTATION

### Build Status
- **Binary:** Successfully built (3.9MB release)
- **Path:** `T:\projects\serena-source\target\release\serena-mcp-server.exe`
- **Compile Status:** All clean, no warnings

### Implemented Tools (16 Total)

#### File Tools (6)
| Tool | Description | Status |
|------|-------------|--------|
| `read_file` | Read file contents with optional line range | Complete |
| `create_text_file` | Create or overwrite text files | Complete |
| `list_dir` | List directory contents | Complete |
| `find_file` | Search files by glob pattern | Complete |
| `replace_content` | Regex-based content replacement | Complete |
| `search_for_pattern` | Search files with regex pattern | Complete |

#### Symbol Tools (5)
| Tool | Description | Status |
|------|-------------|--------|
| `get_symbols_overview` | Get document symbol tree via LSP | Complete |
| `find_symbol` | Find symbol definition by name | Complete |
| `find_referencing_symbols` | Find all references to a symbol | Complete |
| `replace_symbol_body` | Replace function/method body | Complete |
| `rename_symbol` | Rename symbol across workspace | Complete |

#### Memory Tools (5)
| Tool | Description | Status |
|------|-------------|--------|
| `write_memory` | Persist knowledge to markdown | Complete |
| `read_memory` | Retrieve stored knowledge | Complete |
| `list_memories` | List all stored memories | Complete |
| `delete_memory` | Remove stored memory | Complete |
| `edit_memory` | Modify existing memory content | Complete |

---

## 3. Design Decisions

### SDK Selection: rust-mcp-sdk v0.7

**Selected:** rust-mcp-sdk v0.7
**NOT:** rmcp v0.9.0 (originally planned)

**Rationale:**
- Official MCP SDK with `#[mcp_tool]` macro for tool definitions
- Clean async handler pattern with `Arc<dyn McpServer>`
- Well-documented schema generation via schemars
- Stable v0.7 API (mcp_2025_06_18 schema version)

### Service Layer Architecture

```
MCP Protocol Layer (handler.rs)
         |
         v
   SerenaTools Enum (tools/mod.rs)
         |
         v
Service Layer (services.rs)
  - FileService
  - SymbolService
  - MemoryService
         |
         v
Backend Systems
  - std::fs for files
  - LspClient for symbols
  - Markdown files for memory
```

### LSP Integration Pattern

The LspClient uses a generic `send_request<R: Request>()` method:

```rust
// Generic LSP request pattern
pub async fn send_request<R: lsp_types::request::Request>(
    &self,
    params: R::Params,
) -> Result<R::Result, Error>;

// Usage example for document symbols
let result = lsp_client
    .send_request::<lsp_types::request::DocumentSymbolRequest>(params)
    .await?;
```

---

## 4. Code Patterns & Key Files

### Module Structure

```
serena_core/src/
  mcp/
    mod.rs           # Module exports
    handler.rs       # ServerHandler trait impl
    server.rs        # Server lifecycle, InitializeResult
    tools/
      mod.rs         # SerenaTools enum, TryFrom impl
      file_tools.rs  # 6 file operation tools
      symbol_tools.rs # 5 LSP-backed symbol tools
      memory_tools.rs # 5 memory persistence tools
      services.rs    # FileService, SymbolService, MemoryService
  bin/
    mcp_server.rs    # Binary entry point
```

### Key File Responsibilities

| File | Purpose |
|------|---------|
| `mcp/mod.rs` | Public exports: `SerenaHandler`, `SerenaServer` |
| `mcp/handler.rs` | Implements `ServerHandler` trait for MCP protocol |
| `mcp/server.rs` | Server startup, `InitializeResult` with tool list |
| `mcp/tools/mod.rs` | `SerenaTools` enum unifying all 16 tools |
| `mcp/tools/services.rs` | Business logic separated from MCP layer |
| `bin/mcp_server.rs` | `main()` with tokio runtime and stdio transport |

---

## 5. Critical API Patterns (rust-mcp-sdk v0.7)

### Tool Definition Pattern

```rust
use rust_mcp_sdk::macros::mcp_tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ReadFileParams {
    pub path: String,
    pub start_line: Option<u64>,  // Use u64, NOT usize
    pub end_line: Option<u64>,
}

#[mcp_tool(
    name = "read_file",
    description = "Read the contents of a file"
)]
pub fn read_file_tool() -> Tool {
    // Macro generates Tool definition
}
```

### Handler Signature Pattern

```rust
use rust_mcp_sdk::server::ServerHandler;

#[async_trait]
impl ServerHandler for SerenaHandler {
    async fn handle_call_tool(
        &self,
        params: CallToolRequestParams,
        _server: Arc<dyn McpServer>,
    ) -> Result<CallToolResult, CallToolError> {
        // Implementation
    }
}
```

### Result Construction Pattern

```rust
use rust_mcp_sdk::schema::mcp_2025_06_18::TextContent;

// SUCCESS: Create text content result
CallToolResult::text_content(vec![
    TextContent::from(result_string)
])

// ERROR: Create error from message
CallToolError::from_message("Error description".to_string())
```

### Parameter Deserialization Pattern

```rust
// Tool parameters come as Option<Map<String, Value>>
// Must wrap with Value::Object for from_value

let args = params.arguments.unwrap_or_default();
let tool_params: MyParams = serde_json::from_value(
    serde_json::Value::Object(args)
)?;
```

### Implementation Struct Pattern

```rust
use rust_mcp_sdk::schema::Implementation;

// InitializeResult requires Implementation with title field
Implementation {
    name: "serena-mcp-server".to_string(),
    version: "0.1.0".to_string(),
    title: Some("Serena MCP Server".to_string()),
}
```

---

## 6. Issues Resolved During Implementation

### Issue 1: CallToolResult API Change
**Problem:** Documentation showed `CallToolResult::new(String, Option<...>)`
**Solution:** Use `CallToolResult::text_content(vec![TextContent::from(s)])`

### Issue 2: CallToolError Import Path
**Problem:** Error type not at expected path
**Solution:** Import from `rust_mcp_sdk::schema::mcp_2025_06_18::schema_utils::CallToolError`

### Issue 3: JsonSchema usize Incompatibility
**Problem:** `usize` type not supported by schemars JsonSchema derive
**Solution:** Use `u64` instead of `usize` for all schema-derived structs

### Issue 4: LSP Request Pattern
**Problem:** No helper methods like `document_symbols()` on LspClient
**Solution:** Use generic `send_request::<R: Request>()` with explicit type parameter

### Issue 5: Parameter Deserialization
**Problem:** `from_value(params.arguments)` failed with `Option<Map>`
**Solution:** Wrap with `Value::Object(args.unwrap_or_default())`

---

## 7. Dependencies Configuration

### Cargo.toml Additions

```toml
[dependencies]
# MCP Protocol - ACTUAL SDK USED
rust-mcp-sdk = { version = "0.7", features = ["server", "macros"] }

# Schema generation
schemars = "0.8"

# Async runtime
tokio = { version = "1.41", features = ["full"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# LSP types
lsp-types = "0.98"

# File operations
glob = "0.3"
regex = "1.10"

# Async traits
async-trait = "0.1"

# Error handling
thiserror = "1.0"
```

### Binary Configuration

```toml
[[bin]]
name = "serena-mcp-server"
path = "src/bin/mcp_server.rs"
```

---

## 8. Testing & Integration

### Build Verification

```bash
# Build release binary
cargo build --release -p serena_core --bin serena-mcp-server

# Binary location
T:\projects\serena-source\target\release\serena-mcp-server.exe

# Binary size
3.9 MB (release build)
```

### Claude Desktop Integration (Pending)

Configuration for `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "serena": {
      "command": "T:\\projects\\serena-source\\target\\release\\serena-mcp-server.exe",
      "args": []
    }
  }
}
```

### Tool Discovery Test

```bash
# Start server and send initialize request
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | serena-mcp-server.exe
```

---

## 9. Future Roadmap

### Immediate Next Steps

1. **Test with Claude Desktop** - Verify tool discovery and execution
2. **Add Config Tools:**
   - `activate_project` - Switch active project context
   - `get_current_config` - Read current configuration
   - `switch_modes` - Change operational mode

### Medium-term Goals

3. **Transport Alternatives:**
   - UDS (Unix Domain Sockets) for Windows
   - WebSocket transport for browser integration

4. **Tool Expansion:**
   - Port remaining Python Serena tools to Rust
   - Add workflow tools (onboarding, etc.)

### Long-term Goals

5. **Performance Optimization:**
   - Reduce binary size (currently 3.9MB)
   - Optimize LSP client connection pooling

6. **Multi-project Support:**
   - Concurrent LSP clients for multiple projects
   - Project switching without restart

---

## 10. Quick Reference

### Build Commands

```bash
# Debug build
cargo build -p serena_core --bin serena-mcp-server

# Release build (recommended)
cargo build --release -p serena_core --bin serena-mcp-server

# Check without building
cargo check -p serena_core --bin serena-mcp-server

# Run with cargo
cargo run -p serena_core --bin serena-mcp-server
```

### Key Files Quick Access

```
serena_core/src/mcp/mod.rs           # Module structure
serena_core/src/mcp/handler.rs       # Protocol handler
serena_core/src/mcp/server.rs        # Server lifecycle
serena_core/src/mcp/tools/mod.rs     # Tool registry
serena_core/src/mcp/tools/services.rs # Business logic
serena_core/src/bin/mcp_server.rs    # Entry point
```

### Error Debugging

```rust
// Enable tracing for debugging
RUST_LOG=debug cargo run -p serena_core --bin serena-mcp-server
```

### Agent Recommendations

| Task | Agent |
|------|-------|
| LSP integration issues | rust-pro |
| MCP protocol debugging | debugger |
| Performance tuning | performance-engineer |
| Architecture changes | backend-architect |
| Tool expansion | rust-pro |

---

## 11. Context Recovery Instructions

### For New Sessions

1. **Read this file** to understand implementation state
2. **Check binary exists:** `target/release/serena-mcp-server.exe`
3. **Review tool list** in Section 2 for current capabilities
4. **Note SDK version:** rust-mcp-sdk v0.7 (NOT rmcp)

### For Continuing Work

1. **Check pending tasks** in Section 9 (Future Roadmap)
2. **Review issues resolved** in Section 6 before making changes
3. **Follow API patterns** in Section 5 for consistency
4. **Update this context** after significant progress

### Key Differences from Planning Phase

| Aspect | Planning | Implementation |
|--------|----------|----------------|
| MCP SDK | rmcp v0.9.0 | rust-mcp-sdk v0.7 |
| Architecture | 9-crate workspace | Single serena_core crate |
| Database | SQLite | Markdown files |
| Status | Planning complete | Implementation complete |

---

*Last Updated: 2025-12-21*
*Next Review: After Claude Desktop integration testing*
*Maintained by: Claude AI Context Manager*
