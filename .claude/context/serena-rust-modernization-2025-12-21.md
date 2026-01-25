# Serena Pure Rust Modernization - Project Context

**Created:** 2025-12-21
**Version:** 1.0.0
**Status:** Active Development
**Context Type:** Comprehensive Project State

---

## 1. Project Overview

### Project Goals
- Complete 100% migration of Serena from Python (~35,103 LOC) to pure Rust
- Eliminate all PyO3 bindings - no Python/Rust FFI
- Remove all Docker dependencies
- Single binary distribution with no runtime dependencies

### Key Architectural Decisions
- **9-crate workspace structure:**
  - `serena-core` - Core types, traits, utilities
  - `serena-lsp` - Language server protocol client
  - `serena-tools` - Tool implementations
  - `serena-mcp` - MCP server implementation
  - `serena-memory` - Knowledge persistence
  - `serena-config` - Configuration management
  - `serena-web` - Dashboard web server
  - `serena-cli` - Command-line interface
  - `serena` - Main binary crate

- **MCP SDK:** Use rmcp v0.9.0 as the MCP protocol SDK (official Anthropic SDK)
- **Database:** Embedded SQLite for memory/knowledge persistence (no external DB)
- **Language Servers:** Native process spawning (no Docker isolation)
- **Design Pattern:** Trait-based tool system with async/await throughout

### Technology Stack
```toml
[dependencies]
# Async Runtime
tokio = { version = "1.41", features = ["full"] }

# MCP Protocol (Official SDK)
rmcp = { version = "0.9.0", features = ["server", "macros", "transport-stdio"] }

# LSP Protocol
lsp-types = "0.98"

# Web Framework
axum = "0.7"

# Database
rusqlite = { version = "0.32", features = ["bundled"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"

# CLI
clap = { version = "4.5", features = ["derive"] }

# Schema Generation
schemars = "0.8"
```

**Rust Version:** 1.75+ (for async trait stability)

---

## 2. Current State

### Recently Completed
- Comprehensive codebase analysis with 4 specialized agents
- PyO3 removal strategy documented
- MCP SDK selection (rmcp v0.9.0 recommended)
- Multi-crate workspace architecture designed
- 18-week phased migration roadmap created
- Docker elimination strategy documented

### Documentation Created
| Document | Purpose |
|----------|---------|
| `SERENA_RUST_MODERNIZATION_PLAN.md` | Main comprehensive plan |
| `RUST_MCP_RESEARCH.md` | MCP SDK comparison and patterns |
| `RUST_MIGRATION_ARCHITECTURE.md` | Workspace design and code patterns |
| `RUST_MIGRATION_ANALYSIS.md` | Python to Rust dependency mapping |
| `RUST_MCP_ARCHITECTURE.md` | MCP implementation architecture |
| `RUST_MCP_QUICK_START.md` | Quick start guide |
| `RUST_ARCHITECTURE_DIAGRAMS.md` | Visual architecture diagrams |
| `RUST_MIGRATION_PATTERNS.md` | Code migration patterns |
| `RUST_BUILD_CONFIG_EXAMPLE.md` | Build configuration examples |
| `RUST_CODE_EXAMPLES.md` | Rust code examples |
| `RUST_MIGRATION_ROADMAP.md` | Detailed timeline |
| `RUST_MIGRATION_SUMMARY.md` | Executive summary |
| `RUST_MIGRATION_DETAILED_ANALYSIS.md` | Deep dive analysis |

### Existing Rust Code (serena_core)
| File | Reusability | Notes |
|------|-------------|-------|
| `serena_core/src/lsp/client.rs` | 95% | Async LSP client, minor refactoring needed |
| `serena_core/src/lsp/mod.rs` | 100% | LSP module structure |
| `serena_core/src/lsp/resources.rs` | 100% | Resource management |
| `serena_core/src/symbol_graph/mod.rs` | 100% | Symbol indexing system |
| `serena_core/src/web/mod.rs` | 100% | Axum dashboard |
| `serena_core/src/project_host.rs` | 60% | Requires PyO3 removal |
| `serena_core/src/lib.rs` | 70% | Requires PyO3 removal |

---

## 3. Design Decisions

### MCP Protocol Implementation

**Selected SDK:** rmcp v0.9.0
**Rationale:**
- Official Anthropic SDK
- Macro-based tool definitions reduce boilerplate
- Multiple transport support (stdio, HTTP, WebSocket)
- Active maintenance and community

**Rejected Alternatives:**
- `mcp-protocol-sdk` - Less mature, fewer features
- `rust-mcp-sdk` - Not official, breaking changes

### Tool System Pattern
```rust
use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug)]
pub struct ToolResult {
    pub content: String,
    pub is_error: bool,
    pub metadata: Option<Value>,
}

#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name for MCP registration
    fn name(&self) -> &str;

    /// Human-readable description
    fn description(&self) -> &str;

    /// JSON Schema for parameters
    fn parameters_schema(&self) -> serde_json::Value;

    /// Execute the tool with given parameters
    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError>;
}
```

### Language Server Pattern
```rust
use async_trait::async_trait;
use lsp_types::{Position, Location, Url};

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub range: Range,
    pub children: Vec<SymbolInfo>,
}

#[async_trait]
pub trait LanguageServer: Send + Sync {
    /// Initialize the language server with workspace root
    async fn initialize(&mut self, root_uri: Url) -> Result<ServerCapabilities>;

    /// Get document symbols
    async fn document_symbols(&self, uri: &Url) -> Result<Vec<SymbolInfo>>;

    /// Find all references to symbol at position
    async fn find_references(&self, uri: &Url, pos: Position) -> Result<Vec<Location>>;

    /// Go to definition
    async fn goto_definition(&self, uri: &Url, pos: Position) -> Result<Option<Location>>;

    /// Rename symbol across workspace
    async fn rename(&self, uri: &Url, pos: Position, new_name: &str) -> Result<WorkspaceEdit>;

    /// Shutdown gracefully
    async fn shutdown(&mut self) -> Result<()>;
}
```

### Configuration Pattern
```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SerenaConfig {
    pub projects: Vec<ProjectConfig>,
    pub active_project: Option<String>,
    pub global_settings: GlobalSettings,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProjectConfig {
    pub name: String,
    pub path: PathBuf,
    pub language_servers: Vec<LanguageServerConfig>,
    pub tools_enabled: Vec<String>,
}

// Cross-platform config paths using `directories` crate
pub fn config_path() -> PathBuf {
    directories::ProjectDirs::from("", "", "serena")
        .map(|dirs| dirs.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".serena"))
}
```

### Memory/Knowledge Persistence
```rust
use rusqlite::{Connection, params};

pub struct MemoryStore {
    conn: Connection,
}

impl MemoryStore {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS memories (
                id INTEGER PRIMARY KEY,
                project TEXT NOT NULL,
                key TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(project, key)
            );
            CREATE INDEX IF NOT EXISTS idx_memories_project ON memories(project);
        "#)?;
        Ok(Self { conn })
    }

    pub fn write(&self, project: &str, key: &str, content: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO memories (project, key, content, updated_at)
             VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)",
            params![project, key, content],
        )?;
        Ok(())
    }

    pub fn read(&self, project: &str, key: &str) -> Result<Option<String>> {
        self.conn.query_row(
            "SELECT content FROM memories WHERE project = ?1 AND key = ?2",
            params![project, key],
            |row| row.get(0),
        ).optional()
    }
}
```

---

## 4. Agent Coordination History

### Agents Used in Analysis Phase

| Agent | Session ID | Focus Area |
|-------|------------|------------|
| rust-pro | a688b81 | Rust expansion strategy, PyO3 removal patterns |
| python-pro | aa0c169 | Python dependency analysis, migration complexity |
| search-specialist | a5d33a1 | MCP Rust implementation research |
| backend-architect | ad4f245 | Pure Rust architecture design |

### Successful Coordination Patterns
1. **Parallel Execution:** Run 4 agents simultaneously for comprehensive analysis
2. **Domain Focus:** Each agent focused on specific expertise area
3. **Result Synthesis:** Combined findings into unified plan document
4. **Iterative Refinement:** Multiple passes to validate decisions

### Agent Invocation Examples
```markdown
# For Rust-specific work
Use rust-pro agent for:
- Workspace configuration
- Async patterns
- Error handling idioms
- Performance optimization

# For Architecture decisions
Use backend-architect agent for:
- Crate organization
- API design
- Integration patterns

# For Research tasks
Use search-specialist agent for:
- Finding crate alternatives
- Best practices research
- Community patterns
```

---

## 5. Migration Roadmap

### 18-Week Timeline

| Phase | Weeks | Focus | Deliverables |
|-------|-------|-------|--------------|
| **1: Foundation** | 1-2 | Workspace setup, PyO3 removal | Clean Cargo workspace, CI/CD |
| **2: File Tools** | 3 | File operations | read_file, write_file, search, replace |
| **3: LSP Client** | 4-6 | LSP implementation | Async client, process spawning |
| **4: Symbol Tools** | 7-8 | Symbol operations | find_symbol, references, rename |
| **5: MCP Server** | 9 | rmcp integration | Tool registration, transports |
| **6: Memory** | 10 | SQLite persistence | Memory CRUD, search |
| **7: Web Dashboard** | 11-12 | Axum UI | Status, config, monitoring |
| **8: Languages** | 13-16 | 40+ language servers | All supported languages |
| **9: Polish** | 17-18 | Optimization | Binary size, performance tuning |

### Week-by-Week Breakdown

#### Weeks 1-2: Foundation
- [ ] Create clean Cargo workspace
- [ ] Remove all PyO3 dependencies
- [ ] Set up CI/CD with cross-compilation
- [ ] Define core traits and types
- [ ] Establish error handling patterns

#### Week 3: File Tools
- [ ] Implement `read_file` tool
- [ ] Implement `write_file` tool
- [ ] Implement `search_for_pattern` (regex)
- [ ] Implement `replace_content`
- [ ] Implement `list_dir`

#### Weeks 4-6: LSP Client
- [ ] Refactor existing `lsp/client.rs`
- [ ] Native process spawning for language servers
- [ ] Connection management and lifecycle
- [ ] Request/response handling
- [ ] Error recovery and restart

#### Weeks 7-8: Symbol Tools
- [ ] `find_symbol` implementation
- [ ] `find_referencing_symbols`
- [ ] `get_symbols_overview`
- [ ] `replace_symbol_body`
- [ ] `rename_symbol`

#### Week 9: MCP Server
- [ ] rmcp integration
- [ ] Tool registry with macros
- [ ] Stdio transport
- [ ] Error handling
- [ ] Tool discovery

#### Week 10: Memory System
- [ ] SQLite schema design
- [ ] `write_memory`
- [ ] `read_memory`
- [ ] `list_memories`
- [ ] `search_memories`

#### Weeks 11-12: Web Dashboard
- [ ] Axum server setup
- [ ] Project status API
- [ ] Configuration UI
- [ ] Language server monitoring
- [ ] Health checks

#### Weeks 13-16: Language Support
Priority languages (Phase 1):
- Python (pyright/pylsp)
- TypeScript (typescript-language-server)
- Rust (rust-analyzer)
- Go (gopls)
- Java (Eclipse JDT)

Additional languages (Phase 2):
- Vue, PHP, Ruby, C#, Swift, Elixir, etc.

#### Weeks 17-18: Polish
- [ ] Binary size optimization
- [ ] Startup time optimization
- [ ] Memory usage profiling
- [ ] Documentation
- [ ] Release packaging

---

## 6. Expected Outcomes

### Performance Targets

| Metric | Python (Current) | Rust (Target) | Improvement |
|--------|-----------------|---------------|-------------|
| Startup | 2-4s | 0.2-0.5s | 5-10x faster |
| Operations | 500-1000ms | 50-100ms | 5-10x faster |
| Memory | 200-500MB | 50-100MB | 4x smaller |
| Binary | N/A (interpreted) | <50MB | Single file |
| Dependencies | Many Python packages | Zero runtime | Fully self-contained |

### Distribution Targets

| Platform | Binary | Notes |
|----------|--------|-------|
| Windows x64 | `serena.exe` | MSVC toolchain |
| macOS x64 | `serena` | Intel Macs |
| macOS ARM64 | `serena` | Apple Silicon |
| Linux x64 | `serena` | glibc/musl |
| Linux ARM64 | `serena` | Raspberry Pi, etc. |

### Cargo.toml Changes

**Dependencies to Add:**
```toml
[dependencies]
rmcp = { version = "0.9.0", features = ["server", "macros", "transport-stdio"] }
schemars = "0.8"
rusqlite = { version = "0.32", features = ["bundled"] }
async-trait = "0.1"
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
```

**Dependencies to Remove:**
```toml
# Remove all PyO3-related dependencies
pyo3 = { version = "0.21", features = ["extension-module"] }
pyo3-log = "0.10"
```

---

## 7. Risk Assessment

### Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| LSP compatibility | Medium | High | Extensive testing, fallback to Python |
| rmcp API changes | Low | Medium | Pin version, monitor releases |
| Cross-compilation issues | Medium | Medium | CI/CD testing, Docker builds |
| Performance regression | Low | High | Benchmarking at each phase |

### Migration Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Feature parity gaps | Medium | High | Comprehensive Python analysis |
| Timeline slippage | Medium | Medium | Buffer time in schedule |
| Breaking changes | Low | High | Version tagging, gradual rollout |

---

## 8. Context Recovery Instructions

### For New Sessions

1. **Read this context file first** to understand project state
2. **Check recent commits** in `merge-oraios-main-2025-12-18` branch
3. **Review documentation files** listed in Section 2
4. **Activate appropriate agents** based on task:
   - Rust work: rust-pro
   - Architecture: backend-architect
   - Research: search-specialist

### For Continuing Work

1. **Check current phase** in roadmap
2. **Review last session's changes** via `git log`
3. **Update this context** after significant progress
4. **Create phase completion checkpoints**

### Context File Maintenance

- Update this file after each major phase completion
- Create versioned snapshots before major decisions
- Archive completed phase contexts to `context/archive/`

---

## 9. Quick Reference

### Key Directories
```
T:/projects/serena-source/
  serena_core/          # Existing Rust code (to refactor)
  src/serena/           # Python source (reference only)
  src/solidlsp/         # Python LSP (reference only)
  .claude/context/      # Context files (this file)
  SERENA_*.md           # Planning documents
  RUST_*.md             # Rust-specific docs
```

### Essential Commands
```bash
# Build workspace
cargo build --workspace

# Run tests
cargo test --workspace

# Check all targets
cargo check --workspace --all-targets

# Format code
cargo fmt --all

# Lint
cargo clippy --workspace --all-targets
```

### Agent Quick Reference
```yaml
rust-pro: "Implement the LSP client refactoring"
backend-architect: "Design the tool registry pattern"
search-specialist: "Find rmcp usage examples"
debugger: "Debug cross-compilation issues"
```

---

*Last Updated: 2025-12-21*
*Next Review: After Phase 1 completion*
*Maintained by: Claude AI Context Manager*
