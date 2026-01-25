# Serena Python to Rust Migration - Project Context

**Date:** 2025-12-25
**Version:** 3.0
**Status:** Active Migration - Consolidation Phase
**Last Updated:** 2025-12-25

---

## 1. Project Overview

### 1.1 Project Goals
- **Primary Goal:** Complete Python to Rust migration achieving 100% tool parity (38 tools)
- **Performance Target:** 10x faster startup, 50% memory reduction
- **Architecture:** Modern workspace with 12 crates in `crates/` directory
- **Current Parity:** ~71% (25/35 core tools implemented in `crates/`)

### 1.2 Key Architectural Decision
**Decision:** Modern workspace architecture in `crates/` vs legacy monolith in `serena_core/`

The project has TWO Rust implementations:
1. **`crates/` workspace** (PREFERRED) - Modern 12-crate workspace, clean architecture
2. **`serena_core/`** (LEGACY) - Monolithic implementation using rust-mcp-sdk v0.7

Migration strategy: Build on `crates/`, archive `serena_core/` after feature parity

### 1.3 Technology Stack
- **Language:** Rust 2021 edition
- **Async Runtime:** tokio 1.x
- **Serialization:** serde + serde_json
- **LSP Integration:** lsp-types, tower-lsp
- **HTTP/SSE:** axum, tokio-tungstenite
- **Database:** rusqlite (for memory persistence)
- **Pattern Matching:** globset, regex
- **Parallelism:** rayon

---

## 2. Current State

### 2.1 Workspace Structure (12 Crates)
```
crates/
  serena/              # Main application entry point
  serena-cli/          # CLI parsing and command execution
  serena-commands/     # Shell command execution tool
  serena-config/       # Configuration management
  serena-core/         # Core traits, types, error handling
  serena-dashboard/    # WASM dashboard (Yew framework)
  serena-lsp/          # Language Server Protocol client
  serena-mcp/          # MCP server protocol implementation
  serena-memory/       # Memory persistence (SQLite + markdown)
  serena-symbol/       # Symbol tools with LSP integration
  serena-tools/        # File tools, workflow tools, editor tools
  serena-web/          # HTTP/SSE transport layer
```

### 2.2 Implemented Tools by Crate

**serena-tools (17 tools):**
- File tools (6): ReadFileTool, CreateTextFileTool, ListDirectoryTool, FindFileTool, ReplaceContentTool, SearchFilesTool
- Editor tools (3): DeleteLinesTool, InsertAtLineTool, ReplaceLinesTool
- Workflow tools (8): CheckOnboardingPerformedTool, OnboardingTool, ThinkAboutCollectedInformationTool, ThinkAboutTaskAdherenceTool, ThinkAboutWhetherYouAreDoneTool, SummarizeChangesTool, PrepareForNewConversationTool, InitialInstructionsTool

**serena-symbol (7 tools):**
- GetSymbolsOverviewTool, FindSymbolTool, FindReferencingSymbolsTool, ReplaceSymbolBodyTool, RenameSymbolTool, InsertAfterSymbolTool, InsertBeforeSymbolTool

**serena-commands (1 tool):**
- ExecuteShellCommandTool

**Total Implemented:** 25 tools

### 2.3 Missing Tools (Need Implementation)

**Memory Tools (5) - Infrastructure exists in serena-memory/:**
- WriteMemoryTool
- ReadMemoryTool
- ListMemoriesTool
- DeleteMemoryTool
- EditMemoryTool

**Config Tools (4) - Service methods exist, need Tool wrappers:**
- ActivateProjectTool
- RemoveProjectTool
- SwitchModesTool
- GetCurrentConfigTool

**Symbol Tools (1):**
- RestartLanguageServerTool

**Total Missing:** 10 tools

### 2.4 Known Issues
1. **251 unwrap() calls** across 18 files in `crates/` - need error handling
2. **Empty tool registry** in `crates/serena/src/app.rs` - tools not connected
3. **Dual implementation** creates confusion (crates/ vs serena_core/)
4. **Memory tools** have MemoryManager but no Tool wrappers
5. **Config tools** have ConfigService but no Tool wrappers

### 2.5 Recent Accomplishments
- Line editing tools implemented (DeleteLines, InsertAtLine, ReplaceLines)
- Symbol insertion tools implemented (InsertAfterSymbol, InsertBeforeSymbol)
- LSP lifecycle management (stop_all_servers, shutdown)
- Performance optimizations: globset, rayon, pre-computed offsets
- ToolRegistry with Arc<dyn Tool> for thread safety
- ToolRegistryBuilder for fluent API
- Dashboard runtime indicator badge

---

## 3. Design Decisions

### 3.1 Tool Trait Pattern
```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, SerenaError>;
    fn tags(&self) -> Vec<String> { vec![] }
    fn can_edit(&self) -> bool { false }
}
```

### 3.2 Tool Registry Pattern
```rust
// Thread-safe shared ownership
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

// Builder pattern
let registry = ToolRegistryBuilder::new()
    .add_tool(Arc::new(ReadFileTool::new(root.clone())))
    .add_tool(Arc::new(FindSymbolTool::new(root.clone(), lsp_client.clone())))
    .build();
```

### 3.3 LSP Client Pattern
```rust
// Shared LSP client with RwLock
let lsp_client: Arc<RwLock<Box<dyn LanguageServer>>> = ...;

// Symbol tools take LSP client reference
pub struct FindSymbolTool {
    project_root: PathBuf,
    lsp_client: Arc<RwLock<Box<dyn LanguageServer>>>,
}
```

### 3.4 Memory Storage (Dual Strategy)
- **Markdown files:** Human-readable, git-friendly, in `.serena/memories/`
- **SQLite database:** Fast queries, indexing, in `.serena/memories.db`

### 3.5 Performance Optimizations Applied
1. **globset** for pattern matching (50-100x faster than glob)
2. **rayon** parallel file processing
3. **AtomicBool** for early termination in search
4. **Relaxed atomic ordering** for counters
5. **Pre-computed line offsets** for text edits
6. **.next_back()** instead of .last() (O(1) vs O(n))
7. **Iterator optimizations** (avoid collect when possible)

---

## 4. Code Patterns

### 4.1 File Tool Pattern
```rust
pub struct ReadFileTool {
    project_root: PathBuf,
}

impl ReadFileTool {
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self { project_root: project_root.into() }
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str { "read_file" }
    // ... implementation
}
```

### 4.2 Error Handling Pattern
```rust
// Preferred - specific error types
use serena_core::{SerenaError, ToolError};

// Map IO errors
.map_err(SerenaError::Io)?;

// Map tool-specific errors
.map_err(|e| SerenaError::Tool(ToolError::ExecutionFailed(e.to_string())))?;

// NotFound for missing resources
SerenaError::NotFound(format!("Symbol not found: {}", name))
```

### 4.3 Result Pattern
```rust
// Success with JSON
Ok(ToolResult::success(json!({
    "content": file_content,
    "lines": line_count
})))

// Error with message
Err(SerenaError::InvalidParameter("Missing required field: path".to_string()))
```

---

## 5. Agent Coordination History

### 5.1 Effective Agent Combinations
| Task Type | Recommended Agents |
|-----------|-------------------|
| Rust implementation | rust-pro |
| LSP integration | debugger + rust-pro |
| Performance optimization | performance-engineer |
| Architecture review | backend-architect |
| Frontend/Dashboard | frontend-design |
| Tool migration | rust-pro + code-reviewer |

### 5.2 Previous Sessions
1. **2025-12-21**: Initial Rust MCP implementation (16 tools in serena_core/)
2. **2025-12-22**: Dashboard runtime indicator, badge styling
3. **2025-12-23**: Symbol insertion tools, LSP lifecycle
4. **2025-12-24**: Performance optimizations, unwrap elimination analysis
5. **2025-12-25**: Context capture, migration planning

---

## 6. Future Roadmap

### 6.1 Immediate Priority (This Week)
1. **Create Memory Tool wrappers** - 5 tools wrapping MemoryManager
2. **Create Config Tool wrappers** - 4 tools wrapping ConfigService
3. **Add RestartLanguageServerTool** - 1 tool for LSP restart
4. **Wire up tool registry** in app.rs - Connect all tools to MCP server

### 6.2 Short-term (Next 2 Weeks)
1. **Eliminate 251 unwrap() calls** - Proper error handling
2. **Integration tests** - Tool execution, LSP operations
3. **Archive serena_core/** - After feature parity achieved
4. **Binary release** - Windows/Linux cross-compilation

### 6.3 Medium-term (Next Month)
1. **JetBrains tools** - Optional IDE integration
2. **Performance benchmarks** - Startup time, memory usage
3. **Documentation** - API docs, migration guide
4. **CI/CD pipeline** - Automated testing and releases

---

## 7. Risk Assessment

### 7.1 Technical Risks
| Risk | Severity | Mitigation |
|------|----------|------------|
| LSP compatibility | Medium | Extensive testing with multiple language servers |
| Memory/Config tool bugs | Low | Port directly from Python logic |
| Unwrap panics | High | Systematic error handling pass |
| Tool registry wiring | Medium | Unit tests for each tool registration |

### 7.2 Migration Risks
| Risk | Severity | Mitigation |
|------|----------|------------|
| Breaking Python compatibility | High | Keep Python implementation functional |
| Dual-implementation confusion | Medium | Clear documentation, archive legacy |
| Performance regression | Low | Benchmarks before/after |

---

## 8. Recovery Instructions

### 8.1 Quick Session Start
```bash
# Read this context file
# Check current git status
cd T:\projects\serena-source
git status

# Build and test
cargo build --release --workspace
cargo test --workspace

# Python validation
uv run poe test
uv run poe format
uv run poe type-check
```

### 8.2 Tool Parity Check
```bash
# Count Python tools
grep -r "class.*Tool.*Tool" src/serena/tools/*.py | wc -l

# Count Rust tools
grep -r "pub struct.*Tool" crates/*/src/**/*.rs | wc -l
```

### 8.3 Key Files to Review
```
crates/serena/src/app.rs           # Main app - empty tool registry (NEEDS WORK)
crates/serena-tools/src/registry.rs # ToolRegistry implementation
crates/serena-symbol/src/tools.rs   # Symbol tools (7 tools)
crates/serena-memory/src/manager.rs # MemoryManager (needs Tool wrappers)
crates/serena-config/src/service.rs # ConfigService (needs Tool wrappers)
```

---

## 9. Quick Reference

### 9.1 Essential Commands
```bash
# Build
cargo build --release --workspace
cargo build --release -p serena

# Test
cargo test --workspace
cargo test -p serena-symbol

# Run
cargo run -p serena -- --help

# Python
uv run poe test
uv run poe format
uv run poe type-check
```

### 9.2 Critical Paths
```
T:\projects\serena-source\crates\                    # Rust workspace
T:\projects\serena-source\src\serena\tools\          # Python tools (reference)
T:\projects\serena-source\.claude\context\            # Context files
T:\projects\serena-source\target\release\             # Compiled binaries
```

### 9.3 Tool Parity Summary
| Category | Python | Rust (crates/) | Missing |
|----------|--------|----------------|---------|
| File | 6 | 6 | 0 |
| Editor | 3 | 3 | 0 |
| Workflow | 8 | 8 | 0 |
| Symbol | 8 | 7 | 1 (RestartLS) |
| Memory | 5 | 0 | 5 |
| Config | 4 | 0 | 4 |
| Command | 1 | 1 | 0 |
| **Total** | **35** | **25** | **10** |

### 9.4 Plan File
`C:\Users\david\.claude\plans\shiny-finding-clover.md`

---

## 10. Session Handoff Notes

### 10.1 What Works Well
- Symbol tools with LSP integration
- File operations with parallelism
- ToolRegistry with Arc<dyn Tool>
- Dashboard with runtime indicator

### 10.2 What Needs Attention
- Memory tools: MemoryManager exists but no Tool wrappers
- Config tools: ConfigService exists but no Tool wrappers
- App.rs: Empty tool registry - nothing connected
- 251 unwrap() calls - potential panics

### 10.3 Next Actions
1. Create 5 memory Tool structs in serena-memory
2. Create 4 config Tool structs in serena-config
3. Create RestartLanguageServerTool in serena-symbol
4. Update app.rs to register all tools
5. Run integration tests

---

*Context Manager: Claude AI*
*Last Updated: 2025-12-25*
*Session ID: serena-migration-consolidation*
