# Serena Rust MCP Implementation & Dashboard Runtime Indicator - Project Context

**Created:** 2025-12-22
**Version:** 1.1.0
**Status:** Active Development - Dashboard Enhancement Complete
**Context Type:** Implementation Progress + UI Enhancement

---

## 1. Project Overview

### Project Definition
- **Project:** Serena - A semantic coding agent toolkit with MCP server interface
- **Goal:** Migrate performance-critical components to Rust while maintaining Python compatibility
- **Technology Stack:**
  - **Python:** MCP server, LSP integration, dashboard backend
  - **Rust:** serena_core with Axum web server, rust-mcp-sdk v0.7
  - **Frontend:** HTML/CSS/JavaScript with jQuery
- **Location:** `T:\projects\serena-source\`
- **Language Support:** 19 language servers supported
- **Protocol:** MCP (Model Context Protocol) for AI agent interaction

### Architecture Summary
```
T:\projects\serena-source\
  serena_core/                    # Rust crate
    src/
      mcp/                        # MCP protocol implementation (16 tools)
        handler.rs               # ServerHandler trait impl
        server.rs                # Server lifecycle
        tools/                   # Tool implementations
          file_tools.rs          # 6 file operation tools
          symbol_tools.rs        # 5 LSP-backed symbol tools
          memory_tools.rs        # 5 memory persistence tools
          services.rs            # Business logic layer
      web/mod.rs                 # Axum web server (dashboard backend)
      bin/mcp_server.rs          # Binary entry point
    Cargo.toml                   # Dependencies: rust-mcp-sdk 0.7, axum, tokio

  src/serena/                    # Python package
    resources/dashboard/         # Frontend assets
      index.html                # Dashboard HTML
      dashboard.css             # Styles (875+ lines)
      dashboard.js              # Client logic (500+ lines)
    agent.py                    # Central orchestrator
    tools/                      # Python tool implementations

  target/release/               # Build artifacts
    serena-mcp-server.exe       # Rust binary (3.9MB)
```

---

## 2. Current State - Just Completed

### Session Date: 2025-12-22

### Completed: Dashboard Runtime Indicator Badge

A visible indicator was added to the Serena dashboard showing when the Rust backend is running vs Python fallback.

#### Files Modified

| File | Changes | Lines |
|------|---------|-------|
| `serena_core/src/web/mod.rs` | Added version to heartbeat endpoint | L25-31 |
| `src/serena/resources/dashboard/index.html` | Added badge HTML structure | L32-36 |
| `src/serena/resources/dashboard/dashboard.css` | Added styling (~130 lines) | L744-875 |
| `src/serena/resources/dashboard/dashboard.js` | Added detection logic | L416-468 |

#### Runtime States Implemented

| State | Icon | Border Color | Condition |
|-------|------|--------------|-----------|
| Connecting | hourglass | gray | Initial load, pulse animation |
| Rust Active | gear | #F74C00 (orange) | `response.agent === 'serena-rust'` |
| Python Fallback | snake | #3776AB (blue) | `response.agent === 'serena-python'` or generic OK |
| Error/Offline | warning | #DC3545 (red) | Connection failed |

#### Heartbeat Response Format (Rust)

```json
{
  "status": "ok",
  "agent": "serena-rust",
  "version": "0.1.0"
}
```

### Previous Session (2025-12-21)

- Implemented 16 MCP tools in Rust (see Section 4)
- Built release binary: `target/release/serena-mcp-server.exe` (3.9MB)
- Updated MCP configs to use Rust binary
- Created test harness infrastructure

---

## 3. Design Decisions

### Visual Design - Runtime Badge

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| Rust Color | #F74C00 (signature orange) | Official Rust branding color |
| Python Color | #3776AB (Python blue) | Official Python branding color |
| Typography | JetBrains Mono, Fira Code, Consolas | Monospace for technical aesthetic |
| Shape | Pill-shaped (20px border-radius) | Modern, non-intrusive |
| Animation | 0.5s scale-in on connect | Provides visual feedback |
| Dark Theme | Subtle glow effects | Enhanced visibility without harshness |

### CSS Architecture

- **BEM Naming:** `runtime-badge`, `runtime-badge__icon`, `runtime-badge--rust`
- **CSS Variables:** Integrates with existing theme system (`--bg-secondary`, `--text-primary`)
- **Animations:** Keyframe-based for connecting pulse and connected scale-in
- **Dark Theme:** Separate selectors with `[data-theme="dark"]` for enhanced gradients

### SDK Selection (from 2025-12-21)

**Selected:** rust-mcp-sdk v0.7
**NOT:** rmcp v0.9.0 (originally planned)

**Rationale:**
- Official MCP SDK with `#[mcp_tool]` macro
- Clean async handler pattern with `Arc<dyn McpServer>`
- Well-documented schema generation via schemars
- Stable API (mcp_2025_06_18 schema version)

---

## 4. Implemented MCP Tools (16 Total)

### File Tools (6)
| Tool | Description |
|------|-------------|
| `read_file` | Read file contents with optional line range |
| `create_text_file` | Create or overwrite text files |
| `list_dir` | List directory contents |
| `find_file` | Search files by glob pattern |
| `replace_content` | Regex-based content replacement |
| `search_for_pattern` | Search files with regex pattern |

### Symbol Tools (5)
| Tool | Description |
|------|-------------|
| `get_symbols_overview` | Get document symbol tree via LSP |
| `find_symbol` | Find symbol definition by name |
| `find_referencing_symbols` | Find all references to a symbol |
| `replace_symbol_body` | Replace function/method body |
| `rename_symbol` | Rename symbol across workspace |

### Memory Tools (5)
| Tool | Description |
|------|-------------|
| `write_memory` | Persist knowledge to markdown |
| `read_memory` | Retrieve stored knowledge |
| `list_memories` | List all stored memories |
| `delete_memory` | Remove stored memory |
| `edit_memory` | Modify existing memory content |

---

## 5. Key Files and Locations

### Rust Implementation
| Path | Purpose |
|------|---------|
| `serena_core/Cargo.toml` | Dependencies: rust-mcp-sdk 0.7, axum 0.7, tokio |
| `serena_core/src/mcp/mod.rs` | Module exports: `SerenaHandler`, `SerenaServer` |
| `serena_core/src/mcp/handler.rs` | Implements `ServerHandler` trait |
| `serena_core/src/mcp/server.rs` | Server lifecycle, `InitializeResult` |
| `serena_core/src/mcp/tools/mod.rs` | `SerenaTools` enum (all 16 tools) |
| `serena_core/src/mcp/tools/services.rs` | FileService, SymbolService, MemoryService |
| `serena_core/src/web/mod.rs` | Axum routes: `/heartbeat`, `/get_log_messages` |
| `serena_core/src/bin/mcp_server.rs` | Binary entry point |

### Dashboard Frontend
| Path | Purpose |
|------|---------|
| `src/serena/resources/dashboard/index.html` | Main HTML with runtime badge element |
| `src/serena/resources/dashboard/dashboard.css` | Full styles including runtime badge (875+ lines) |
| `src/serena/resources/dashboard/dashboard.js` | Client logic with runtime detection |

### Binary
| Path | Size |
|------|------|
| `target/release/serena-mcp-server.exe` | 3.9 MB |

### Test Infrastructure
| Path | Purpose |
|------|---------|
| `test/harness/` | MCP test harness utilities |
| `test/performance/` | Benchmark configurations |

### Context Documentation
| Path | Purpose |
|------|---------|
| `.claude/context/INDEX.md` | Context file index |
| `.claude/context/QUICK_LOAD.md` | Quick context summary |
| `.claude/context/serena-rust-mcp-implementation-2025-12-21.md` | Previous context |

---

## 6. Technical Patterns

### rust-mcp-sdk v0.7 Tool Definition

```rust
use rust_mcp_sdk::macros::mcp_tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ReadFileParams {
    pub path: String,
    pub start_line: Option<u64>,  // Use u64, NOT usize (schemars limitation)
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

### Axum Heartbeat Endpoint

```rust
async fn heartbeat() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "agent": "serena-rust",
        "version": env!("CARGO_PKG_VERSION")
    }))
}
```

### Dashboard Runtime Detection (JavaScript/jQuery)

```javascript
detectRuntime() {
    $.ajax({
        url: '/heartbeat',
        type: 'GET',
        timeout: 5000,
        success: function (response) {
            if (response.agent === 'serena-rust') {
                self.setRuntimeState('rust', 'gear', 'Rust v' + response.version);
            } else if (response.agent === 'serena-python' || response.status === 'ok') {
                self.setRuntimeState('python', 'snake', 'Python');
            }
        },
        error: function () {
            self.setRuntimeState('error', 'warning', 'Offline');
        }
    });
}
```

### CSS BEM Pattern for Runtime Badge

```css
.runtime-badge { /* Base container */ }
.runtime-badge__icon { /* Icon span */ }
.runtime-badge__text { /* Text span */ }
.runtime-badge--connecting { /* State modifier */ }
.runtime-badge--rust { /* State modifier */ }
.runtime-badge--python { /* State modifier */ }
.runtime-badge--error { /* State modifier */ }
```

---

## 7. Dependencies

### Rust (from Cargo.toml)

```toml
[dependencies]
rust-mcp-sdk = { version = "0.7", features = ["server", "macros", "stdio", "2025_06_18"] }
tokio = { version = "1.36", features = ["full"] }
axum = "0.7"
tower-http = { version = "0.5", features = ["fs", "trace", "cors"] }
lsp-types = "0.95"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
schemars = "0.8"
glob = "0.3"
regex = "1.11"
rusqlite = { version = "0.32", features = ["bundled"] }
```

### Frontend

- jQuery (included in dashboard)
- No build system (vanilla JS/CSS)

---

## 8. Build Commands

```bash
# Debug build
cargo build -p serena_core --bin serena-mcp-server

# Release build (recommended)
cargo build --release -p serena_core --bin serena-mcp-server

# Run with cargo
cargo run -p serena_core --bin serena-mcp-server

# Run tests
uv run poe test
uv run poe format
uv run poe type-check
```

---

## 9. Future Roadmap

### Immediate (Next Session)

1. **Test Runtime Badge** - Verify badge displays correctly with live Rust server
2. **Add Config Tools** - `activate_project`, `get_current_config`, `switch_modes`
3. **Performance Benchmarks** - Set up criterion benchmarks in `benches/`

### Medium-term

4. **WASM Frontend Potential** - Mentioned in user request, explore Yew/Leptos
5. **LSP Client Migration** - Port Python LSP client to Rust
6. **Full Test Coverage** - Integration tests for all 16 tools

### Long-term

7. **Multi-project Support** - Concurrent LSP clients
8. **WebSocket Transport** - Alternative to stdio for browser integration
9. **Binary Size Optimization** - Currently 3.9MB, target < 2MB

---

## 10. Context Recovery Instructions

### For New Sessions

1. **Read this file** to understand current state
2. **Check binary exists:** `target/release/serena-mcp-server.exe`
3. **Note SDK version:** rust-mcp-sdk v0.7 (NOT rmcp)
4. **Review tool list** in Section 4 for capabilities

### For Continuing Dashboard Work

1. Dashboard files in `src/serena/resources/dashboard/`
2. Rust web server in `serena_core/src/web/mod.rs`
3. CSS uses BEM naming: `runtime-badge--*`
4. JavaScript uses jQuery: `$('#runtime-badge')`

### For Continuing MCP Work

1. Review `serena_core/src/mcp/` module structure
2. Follow patterns in Section 6 for new tools
3. Use service layer pattern for business logic

---

## 11. Quick Reference

### Git Status (as of session start)

- **Branch:** merge-oraios-main-2025-12-18
- **Main:** main
- **Recent commits:** Merged upstream oraios/main (2025-12-18)

### Key Rust Files

```
serena_core/src/mcp/mod.rs           # Module structure
serena_core/src/mcp/handler.rs       # Protocol handler
serena_core/src/mcp/server.rs        # Server lifecycle
serena_core/src/mcp/tools/mod.rs     # Tool registry
serena_core/src/mcp/tools/services.rs # Business logic
serena_core/src/web/mod.rs           # Dashboard backend
serena_core/src/bin/mcp_server.rs    # Entry point
```

### Agent Recommendations

| Task | Agent |
|------|-------|
| Rust implementation | rust-pro |
| LSP integration | debugger |
| Performance tuning | performance-engineer |
| Frontend CSS/JS | frontend-design |
| Architecture changes | backend-architect |

---

*Last Updated: 2025-12-22*
*Next Review: After runtime badge testing*
*Maintained by: Claude AI Context Manager*
