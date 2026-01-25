# Serena Python to Rust Migration Project Context

**Date:** 2025-12-25
**Status:** Consolidation Complete, Tool Parity at 92%
**Project Root:** `T:/projects/serena-source`

---

## Executive Summary

This document captures the complete state of the Serena MCP server migration from Python to Rust. The consolidation phase is complete with 13 deprecated files archived. The modern Rust implementation uses a modular workspace architecture with 11 crates, achieving 92% tool parity (35/38 tools).

---

## Project Goals

1. **Primary:** Migrate Serena MCP server from Python to Rust
2. **Secondary:** Achieve 100% tool parity with Python implementation
3. **Tertiary:** Modern modular architecture with clean separation of concerns
4. **Quality:** Eliminate `unwrap()` calls, comprehensive error handling

---

## Technology Stack

| Component | Technology | Version/Notes |
|-----------|------------|---------------|
| Language | Rust | 2021 edition |
| Async Runtime | tokio | Full features |
| MCP Protocol | Custom (serena-mcp) | JSON-RPC based |
| LSP Types | lsp-types crate | Standard LSP protocol |
| Web Framework | Axum | HTTP/SSE transport |
| Concurrency | DashMap | Thread-safe maps |
| Serialization | serde + serde_json | JSON handling |
| CLI | clap | Argument parsing |

---

## Architecture Overview

### Workspace Structure

```
crates/
├── serena/           # Main binary entry point
├── serena-cli/       # CLI argument parsing, commands
├── serena-config/    # Configuration management + config tools
├── serena-core/      # Core traits: Tool, ToolRegistry, SerenaError
├── serena-commands/  # Shell execution tools
├── serena-dashboard/ # WASM-based dashboard UI
├── serena-lsp/       # LSP client, manager, language tools
├── serena-mcp/       # MCP protocol implementation + server
├── serena-memory/    # Memory manager + memory tools
├── serena-symbol/    # Symbol manipulation tools (7 tools)
├── serena-tools/     # File/editor/workflow tools + ToolFactory
└── serena-web/       # HTTP/SSE transport + REST API
```

### Key Design Patterns

1. **Tool Trait:**
   ```rust
   #[async_trait]
   pub trait Tool: Send + Sync {
       fn name(&self) -> &str;
       fn description(&self) -> &str;
       fn input_schema(&self) -> Value;
       async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError>;
   }
   ```

2. **Factory Pattern:**
   ```rust
   let factory = ToolFactory::new(project_root);
   let tools = factory.core_tools();  // Returns Vec<Arc<dyn Tool>>
   ```

3. **Registry Pattern:**
   ```rust
   let registry = ToolRegistryBuilder::new()
       .add_tools(tools)
       .build();
   ```

4. **Arc Wrapping:**
   All tools stored as `Arc<dyn Tool>` for thread-safety and shared ownership.

---

## Tool Implementation Status

### Completed Tools (35 total)

#### Memory Tools (6) - `serena-memory`
| Tool | Status | Notes |
|------|--------|-------|
| write_memory | Complete | Persists markdown memories |
| read_memory | Complete | Retrieves by filename |
| list_memories | Complete | Lists all memories |
| delete_memory | Complete | Removes memory file |
| edit_memory | Complete | Updates existing memory |
| search_memories | Complete | Full-text search |

#### Config Tools (6) - `serena-config`
| Tool | Status | Notes |
|------|--------|-------|
| activate_project | Complete | Project activation |
| get_current_config | Complete | Returns active config |
| switch_modes | Complete | Mode switching |
| list_projects | Complete | Lists available projects |
| get_active_tools | Complete | Returns enabled tools |
| remove_project | Complete | Deactivates project |

#### LSP Tools (4) - `serena-lsp`
| Tool | Status | Notes |
|------|--------|-------|
| restart_language_server | Complete | Restarts LSP |
| list_language_servers | Complete | Lists running servers |
| stop_language_server | Complete | Stops specific server |
| clear_lsp_cache | Complete | Clears LSP cache |

#### Symbol Tools (7) - `serena-symbol`
| Tool | Status | Notes |
|------|--------|-------|
| get_symbols_overview | Complete | Project symbol map |
| find_symbol | Complete | Locate symbol by name |
| find_referencing_symbols | Complete | Find references |
| replace_symbol_body | Complete | Replace implementation |
| rename_symbol | Complete | LSP-based rename |
| insert_after_symbol | Complete | Insert code after |
| insert_before_symbol | Complete | Insert code before |

#### File Tools (6) - `serena-tools/file`
| Tool | Status | Notes |
|------|--------|-------|
| read_file | Complete | Read with encoding |
| create_text_file | Complete | Create new files |
| list_directory | Complete | Directory listing |
| find_file | Complete | Glob-based search |
| search_files | Complete | Regex content search |
| replace_content | Complete | Regex replacement |

#### Editor Tools (3) - `serena-tools/file`
| Tool | Status | Notes |
|------|--------|-------|
| delete_lines | Complete | Line deletion |
| insert_at_line | Complete | Insert at line |
| replace_lines | Complete | Line replacement |

#### Workflow Tools (8) - `serena-tools/workflow`
| Tool | Status | Notes |
|------|--------|-------|
| check_onboarding_performed | Complete | Onboarding check |
| onboarding | Complete | Project onboarding |
| think | Complete | Reasoning tool |
| think_more | Complete | Extended reasoning |
| think_harder | Complete | Deep reasoning |
| summarize_changes | Complete | Change summary |
| prepare_for_new_conversation | Complete | Context prep |
| initial_instructions | Complete | Setup instructions |

#### Command Tools (1) - `serena-commands`
| Tool | Status | Notes |
|------|--------|-------|
| execute_shell_command | Complete | Shell execution |

### Missing/Pending Tools (3)

| Tool | Category | Notes |
|------|----------|-------|
| search_memories | Memory | Implemented but not in factory |
| (varies) | Python-specific | Some Python tools may not apply |

---

## REST API Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| /heartbeat | GET | Health check |
| /get_config | GET | Current configuration |
| /get_stats | GET | Server statistics |
| /shutdown | POST | Graceful shutdown |
| /tools | GET | List available tools |
| /info | GET | Server information |

---

## Archive Summary

### Files Archived (2025-12-25)

Moved 13 deprecated files from `serena_core/` to `archive/serena_core_legacy/`:

```
archive/serena_core_legacy/
├── mcp/
│   └── tools/
│       ├── config_tools.rs      # Replaced by serena-config
│       ├── file_tools.rs        # Replaced by serena-tools
│       ├── memory_tools.rs      # Replaced by serena-memory
│       ├── mod.rs               # Legacy dispatcher
│       ├── services.rs          # Replaced by crate services
│       └── symbol_tools.rs      # Replaced by serena-symbol
├── lsp/
│   ├── client.rs               # Duplicate of serena-lsp
│   └── mod.rs                  # Legacy LSP module
├── project_host.rs             # Unused project host
├── test_utils.rs               # Legacy test utilities
├── web/
│   └── mod.rs                  # Replaced by serena-web
└── lib.rs                      # Legacy entry point
```

### Files Retained in serena_core (Valuable)

| File | Purpose | Migration Plan |
|------|---------|----------------|
| `lsp/resources.rs` | Tool download manager | Migrate to serena-lsp |
| `symbol_graph/mod.rs` | DashMap symbol cache | Consider for serena-symbol |
| `lib.rs` (partial) | Parallel regex utilities | Extract to serena-tools |

---

## Test Coverage

| Crate | Tests | Status |
|-------|-------|--------|
| serena-web | 9 | Pass |
| serena-memory | 7 | Pass |
| serena-config | 4 | Pass |
| serena-lsp | 4 | Pass |
| serena-tools | 12+ | Pass |
| serena-symbol | 8+ | Pass |
| **Total** | **54+** | **All Pass** |

---

## Known Issues / Technical Debt

1. **Flaky Test:** One timeout test in legacy serena_core occasionally fails
2. **Unwrap Calls:** Some `unwrap()` calls remain in codebase (elimination planned)
3. **Duplicate LSP:** serena_core/lsp vs serena-lsp (legacy can be removed)
4. **Factory Gap:** search_memories not registered in ToolFactory

---

## Next Steps (Priority Order)

### High Priority
1. Register `search_memories` in ToolFactory
2. Eliminate remaining `unwrap()` calls
3. Remove serena_core legacy code after confirming no dependencies

### Medium Priority
4. Migrate `lsp/resources.rs` to serena-lsp for auto-download
5. Consider migrating symbol_graph cache for performance
6. Complete dashboard WASM integration

### Low Priority
7. Performance benchmarking vs Python implementation
8. Documentation updates
9. Release preparation

---

## Key File Locations

| Purpose | Path |
|---------|------|
| Main binary | `crates/serena/src/main.rs` |
| App orchestration | `crates/serena/src/app.rs` |
| Tool factory | `crates/serena-tools/src/factory.rs` |
| Tool trait | `crates/serena-core/src/traits/tool.rs` |
| MCP server | `crates/serena-mcp/src/server.rs` |
| HTTP transport | `crates/serena-web/src/http.rs` |
| Memory manager | `crates/serena-memory/src/manager.rs` |
| LSP manager | `crates/serena-lsp/src/manager.rs` |
| Symbol tools | `crates/serena-symbol/src/tools/` |
| Config tools | `crates/serena-config/src/tools/` |

---

## Build Commands

```bash
# Development build
cargo build --workspace

# Release build
cargo build --release --workspace

# Run tests
cargo test --workspace

# Run specific crate tests
cargo test -p serena-memory
cargo test -p serena-config

# Run binary
cargo run -p serena

# Check for warnings
cargo clippy --workspace --all-targets
```

---

## Context for Future Sessions

When resuming work on this project:

1. **Check git status** for any uncommitted changes
2. **Run tests** to verify current state: `cargo test --workspace`
3. **Review this context file** for current priorities
4. **Check archive** if looking for old implementations
5. **Use ToolFactory** for adding new tools to the system

---

*Last Updated: 2025-12-25*
*Context Version: 1.0*
*Migration Phase: Consolidation Complete*
