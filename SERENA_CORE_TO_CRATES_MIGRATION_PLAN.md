# Serena Core to Crates Migration Analysis

## Executive Summary

This document provides a detailed analysis of what needs to be migrated from `serena_core/` to `crates/` and identifies gaps, mismatches, and required implementations.

**Project Location:** T:\projects\serena-source

---

## 1. Memory Tools Migration

### Current State in serena_core

**Location:** `serena_core/src/mcp/tools/memory_tools.rs` (224 lines)

**Implemented Tools (5):**
1. `write_memory` - Write memory to markdown file
2. `read_memory` - Read memory file with optional truncation
3. `list_memories` - List all available memories
4. `delete_memory` - Delete a memory file
5. `edit_memory` - Edit memory using literal or regex replacement

**Service Layer:** `serena_core/src/mcp/tools/services.rs`
- `MemoryService` struct (lines 183-242)
  - `new(project_root)` - Creates .serena/memories directory
  - `write(name, content)` - Async file write
  - `read(name)` - Async file read
  - `list()` - Async list memories
  - `delete(name)` - Async delete memory

### Current State in crates/

**Location:** `crates/serena-memory/src/manager.rs` (419 lines)

**MemoryManager Implementation:**
- ✅ Implements `MemoryStorage` trait from serena-core
- ✅ Dual storage: Markdown files + SQLite database
- ✅ Methods: `save_memory`, `load_memory`, `list_memories`, `delete_memory`
- ✅ Advanced features: `search`, `replace_content`, `sync`, `list_memories_with_metadata`
- ✅ Comprehensive test coverage (18 tests)

**API Signature Comparison:**

| serena_core Service | crates/serena-memory Manager | Match? |
|---------------------|------------------------------|--------|
| `write(name, content)` | `save_memory(name, content)` | ⚠️ Different name |
| `read(name)` | `load_memory(name)` | ⚠️ Different name |
| `list()` | `list_memories()` | ✅ Match |
| `delete(name)` | `delete_memory(name)` | ⚠️ Different name |
| N/A | `replace_content(name, needle, repl, mode)` | ➕ New feature |

### Migration Strategy

**Step 1: Create MCP Tool Wrappers** (NEW FILE)
```rust
// Location: crates/serena-tools/src/memory/mod.rs

use async_trait::async_trait;
use serena_core::{Tool, ToolResult, SerenaError};
use serena_memory::{MemoryManager, ReplaceMode};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct WriteMemoryTool {
    manager: MemoryManager,
}

#[async_trait]
impl Tool for WriteMemoryTool {
    fn name(&self) -> &str { "write_memory" }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let p: WriteMemoryParams = serde_json::from_value(params)?;
        let result = self.manager.save_memory(&p.memory_file_name, &p.content).await?;
        Ok(ToolResult::success(serde_json::json!({ "message": result })))
    }
}

// Similar for: ReadMemoryTool, ListMemoriesTool, DeleteMemoryTool, EditMemoryTool
```

**Step 2: Adapt Method Names**
- Wrapper tools should call `save_memory` but expose as `write_memory` to maintain MCP API
- Internal usage should use crates/serena-memory directly with new names

**Step 3: Leverage New Features**
- Edit memory tool can use `manager.replace_content()` instead of manual regex
- Add metadata support to list_memories (optional enhancement)

**Step 4: Testing Requirements**
- Port existing integration tests from serena_core
- Add tests for SQLite metadata features
- Test error handling for file not found cases

---

## 2. Config Tools Migration

### Current State in serena_core

**Location:** `serena_core/src/mcp/tools/config_tools.rs` (396 lines)

**Implemented Tools (7):**
1. `get_current_config` - Returns active project, tools, context, mode, version
2. `initial_instructions` - Returns Serena Instructions Manual (hardcoded string)
3. `think` - Reflection prompt about collected information
4. `think_more` - Deep thinking prompt about edge cases
5. `think_different` - Alternative approach generation prompt
6. `onboarding` - Project onboarding instructions
7. `check_onboarding_performed` - Checks if memories exist for project

**Tool List in get_current_config:**
```rust
available_tools: vec![
    // File: read_file, create_text_file, list_dir, find_file, replace_content, search_for_pattern
    // Symbol: get_symbols_overview, find_symbol, find_referencing_symbols,
    //         replace_symbol_body, rename_symbol, insert_after_symbol, insert_before_symbol
    // Memory: write_memory, read_memory, list_memories, delete_memory, edit_memory
    // Command: execute_shell_command
    // Config: get_current_config, initial_instructions, think, think_more, think_different,
    //         onboarding, check_onboarding_performed
]
```

**Dependencies:**
- Uses `FileService` for project_root()
- Uses `MemoryService` for check_onboarding_performed

### Current State in crates/

**Location:** `crates/serena-config/src/service.rs` (480 lines)

**ConfigService Implementation:**
- ✅ Project activation/deactivation
- ✅ Context switching (desktop-app, agent, ide-assistant)
- ✅ Mode management (interactive, planning, editing, one-shot)
- ✅ Project listing and removal
- ✅ Active tool calculation based on context

**API Comparison:**

| Feature | serena_core | crates/serena-config | Gap |
|---------|-------------|----------------------|-----|
| Get config state | ❌ Returns JSON manually | ✅ `get_config()` | Method exists |
| Active project | ❌ From FileService | ✅ `get_active_project()` | ✅ Available |
| Context | ❌ Hardcoded "default" | ✅ `get_active_context()` | ✅ Available |
| Mode | ❌ Hardcoded "default" | ✅ `get_active_modes()` | ✅ Available |
| Available tools | ❌ Hardcoded list | ✅ `get_active_tools()` | ✅ Available |
| Instructions | ✅ Hardcoded string | ❌ Missing | Need to add |
| Think prompts | ✅ 3 thinking tools | ❌ Missing | Need to migrate |
| Onboarding | ✅ 2 tools | ⚠️ Partial in workflow | Need to enhance |

### Migration Strategy

**Step 1: Create Config MCP Tool** (NEW FILE)
```rust
// Location: crates/serena-tools/src/config/mod.rs

#[derive(Debug, Serialize, Deserialize)]
pub struct GetCurrentConfigTool {
    config_service: Arc<ConfigService>,
    tool_registry: Arc<ToolRegistry>,
}

#[async_trait]
impl Tool for GetCurrentConfigTool {
    async fn execute(&self, _params: Value) -> Result<ToolResult, SerenaError> {
        let project = self.config_service.get_active_project()?;
        let context = self.config_service.get_active_context()?;
        let modes = self.config_service.get_active_modes()?;
        let tools = self.tool_registry.list();

        let config = json!({
            "active_project": {
                "path": project.root_path.display().to_string(),
                "name": project.name
            },
            "available_tools": tools,
            "context": context,
            "modes": modes,
            "version": env!("CARGO_PKG_VERSION"),
            "server": "serena-mcp-server"
        });

        Ok(ToolResult::success(config))
    }
}
```

**Step 2: Migrate Thinking Tools**
- Already exists in `crates/serena-tools/src/workflow/`
- ✅ `ThinkAboutCollectedInformationTool` (equivalent to `think`)
- ✅ `ThinkAboutTaskAdherenceTool` (similar to `think_more`)
- ⚠️ Missing `think_different` - needs to be added

**Step 3: Migrate Onboarding Tools**
- ✅ `OnboardingTool` exists in workflow
- ✅ `CheckOnboardingPerformedTool` exists in workflow
- Need to integrate with MemoryManager to check memories

**Step 4: Add Instructions Tool**
```rust
// Location: crates/serena-tools/src/workflow/instructions.rs
// Migrate SERENA_INSTRUCTIONS constant and InitialInstructionsTool
```

---

## 3. Symbol Tools Migration

### Current State in serena_core

**Location:** `serena_core/src/mcp/tools/symbol_tools.rs` (716 lines)

**Implemented Tools (5):**
1. `get_symbols_overview` - Document symbols with depth parameter
2. `find_symbol` - Workspace symbol search with path filtering
3. `find_referencing_symbols` - Find all references to a symbol
4. `replace_symbol_body` - Replace entire symbol body
5. `rename_symbol` - LSP rename with workspace edit application

**MISSING from Implementation but Listed in Config:**
- ❌ `insert_after_symbol` - Mentioned in get_current_config but NOT implemented
- ❌ `insert_before_symbol` - Mentioned in get_current_config but NOT implemented

**Service Layer:** `serena_core/src/mcp/tools/services.rs`
- `SymbolService` struct (lines 130-180)
  - `new(project_root)` - Creates service with LSP client holder
  - `start_lsp(command, args)` - Starts language server
  - `get_client()` - Returns Arc&lt;LspClient&gt;
  - Uses RwLock&lt;Option&lt;Arc&lt;LspClient&gt;&gt;&gt; for lazy initialization

### Current State in crates/

**Location:** `crates/serena-lsp/src/manager.rs` (276 lines)

**LanguageServerManager Implementation:**
- ✅ Multi-language server management with DashMap
- ✅ `start_server(language)` - Start server for specific language
- ✅ `stop_server(language)` - Stop specific server
- ✅ `get_server(language)` - Get client for language
- ✅ `get_or_start_server(language)` - Convenience method
- ✅ **`restart_server(language)`** - ⭐ NEW: Restart and clear cache
- ✅ `stop_all_servers()` - Shutdown all servers
- ✅ Cache management: `clear_cache()`, `prune_cache()`
- ✅ Status queries: `is_server_running()`, `list_running_servers()`

**LSP Trait:** `crates/serena-core/src/traits/lsp.rs`
```rust
#[async_trait]
pub trait LanguageServer: Send + Sync {
    async fn initialize(&mut self, params: InitializeParams) -> Result<ServerCapabilities, LspError>;
    async fn shutdown(&mut self) -> Result<(), LspError>;
    async fn document_symbols(&self, document: TextDocumentIdentifier) -> Result<Vec<SymbolInfo>, LspError>;
    async fn find_references(&self, params: TextDocumentPositionParams) -> Result<Vec<Location>, LspError>;
    async fn rename(&self, params: RenameParams) -> Result<WorkspaceEdit, LspError>;
    async fn goto_definition(&self, params: TextDocumentPositionParams) -> Result<GotoDefinitionResponse, LspError>;
    async fn restart(&mut self) -> Result<(), LspError>; // ⭐ DEFAULT IMPLEMENTATION
}
```

### API Comparison

| Feature | serena_core SymbolService | crates/serena-lsp Manager | Gap |
|---------|---------------------------|---------------------------|-----|
| Single LSP client | ✅ RwLock&lt;Option&lt;LspClient&gt;&gt; | ❌ Multi-language DashMap | Architecture diff |
| Start server | ✅ `start_lsp(cmd, args)` | ✅ `start_server(language)` | Language enum required |
| Get client | ✅ `get_client()` | ✅ `get_server(language)` | Need language param |
| Restart | ❌ Not implemented | ✅ `restart_server(language)` | ⭐ NEW FEATURE |
| Cache control | ❌ Not implemented | ✅ `clear_cache()`, `prune_cache()` | ⭐ NEW FEATURE |
| Multi-language | ❌ Single server only | ✅ Multiple servers | Major improvement |

### Migration Strategy

**Step 1: Decide on Service Architecture**

**Option A: Keep Single-Language Service (Compatible)**
```rust
// Wrapper around LanguageServerManager for single-language projects
pub struct SymbolService {
    manager: LanguageServerManager,
    primary_language: Language,
}

impl SymbolService {
    pub async fn start_lsp(&self, _command: &str, _args: Vec<String>) -> Result<()> {
        // Ignore command/args, use language config from manager
        self.manager.start_server(self.primary_language).await
    }

    pub async fn get_client(&self) -> Result<Arc<LspClient>> {
        self.manager.get_or_start_server(self.primary_language).await
    }
}
```

**Option B: Migrate to Multi-Language (Recommended)**
```rust
// Tools accept language parameter
#[derive(Deserialize)]
pub struct GetSymbolsOverviewParams {
    pub relative_path: String,
    pub language: Option<Language>, // Auto-detect from file extension if None
    pub depth: u64,
}

impl GetSymbolsOverviewTool {
    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let p: GetSymbolsOverviewParams = serde_json::from_value(params)?;

        // Auto-detect language from file extension
        let language = p.language.unwrap_or_else(|| detect_language(&p.relative_path));

        let client = self.manager.get_or_start_server(language).await?;
        // ... rest of implementation
    }
}
```

**Step 2: Implement Missing Insert Tools**

These tools are mentioned in config but NOT implemented in serena_core:

```rust
// Location: crates/serena-tools/src/editor/insert.rs

#[derive(Debug, Serialize, Deserialize)]
pub struct InsertAfterSymbolTool {
    manager: Arc<LanguageServerManager>,
    project_root: PathBuf,
}

#[derive(Debug, Deserialize)]
struct InsertAfterSymbolParams {
    name_path: String,
    relative_path: String,
    content: String,
    language: Option<Language>,
}

#[async_trait]
impl Tool for InsertAfterSymbolTool {
    fn name(&self) -> &str { "insert_after_symbol" }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let p: InsertAfterSymbolParams = serde_json::from_value(params)?;

        // 1. Auto-detect language
        let language = p.language.unwrap_or_else(|| detect_language(&p.relative_path));

        // 2. Get LSP client
        let client = self.manager.get_or_start_server(language).await?;

        // 3. Find symbol range via document_symbols
        let file_path = self.project_root.join(&p.relative_path);
        let uri = lsp_types::Url::from_file_path(&file_path)?;
        let symbols = client.document_symbols(uri).await?;

        // 4. Find target symbol
        let target_name = p.name_path.split('/').last().unwrap_or(&p.name_path);
        let range = find_symbol_range(&symbols, target_name)?;

        // 5. Read file
        let content = tokio::fs::read_to_string(&file_path).await?;
        let lines: Vec<&str> = content.lines().collect();

        // 6. Insert after symbol's end line
        let mut new_content = String::new();
        for (i, line) in lines.iter().enumerate() {
            new_content.push_str(line);
            new_content.push('\n');

            if i == range.end.line as usize {
                // Insert content after symbol
                new_content.push_str(&p.content);
                if !p.content.ends_with('\n') {
                    new_content.push('\n');
                }
            }
        }

        // 7. Write back
        tokio::fs::write(&file_path, &new_content).await?;

        Ok(ToolResult::success(json!({
            "message": format!("Inserted content after symbol '{}'", p.name_path)
        })))
    }
}

// Similar implementation for InsertBeforeSymbolTool
```

**Step 3: Add RestartLanguageServerTool**

NEW tool to expose restart functionality:

```rust
// Location: crates/serena-tools/src/lsp/restart.rs

#[derive(Debug, Serialize, Deserialize)]
pub struct RestartLanguageServerTool {
    manager: Arc<LanguageServerManager>,
}

#[derive(Debug, Deserialize)]
struct RestartParams {
    language: Option<Language>, // If None, restart all
}

#[async_trait]
impl Tool for RestartLanguageServerTool {
    fn name(&self) -> &str { "restart_language_server" }

    fn description(&self) -> &str {
        "Restart language server(s). Useful when the server becomes unresponsive or after configuration changes."
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let p: RestartParams = serde_json::from_value(params)?;

        match p.language {
            Some(lang) => {
                self.manager.restart_server(lang).await?;
                Ok(ToolResult::success(json!({
                    "message": format!("Restarted {:?} language server", lang)
                })))
            }
            None => {
                // Restart all running servers
                let running = self.manager.list_running_servers();
                for lang in running {
                    self.manager.restart_server(lang).await?;
                }
                Ok(ToolResult::success(json!({
                    "message": "Restarted all language servers"
                })))
            }
        }
    }
}
```

**Step 4: Port Existing Symbol Tools**

For each of the 5 existing tools (get_symbols_overview, find_symbol, etc.):
1. Copy implementation logic from serena_core
2. Replace SymbolService with LanguageServerManager
3. Add language detection/parameter
4. Use manager.get_or_start_server(language) instead of service.get_client()

---

## 4. File Tools Migration

### Current State in serena_core

**Location:** `serena_core/src/mcp/tools/file_tools.rs` (449 lines)

**Implemented Tools (6):**
1. `read_file` - Read file with path validation and truncation
2. `create_text_file` - Create/write file with directory creation
3. `list_dir` - List directory with recursive option
4. `find_file` - Find files by name pattern
5. `replace_content` - Regex/literal text replacement in file
6. `search_for_pattern` - Search files for pattern with context

**Service Layer:** `serena_core/src/mcp/tools/services.rs`
- `FileService` struct (lines 11-128)
  - `new(project_root)` - Creates service with path validation
  - `validate_path(path)` - Checks if path is within allowed directories
  - `read_file(path)` - Async file read
  - `write_file(path, content)` - Async file write with dir creation
  - `list_dir(path, recursive)` - Async directory listing

### Current State in crates/

**Location:** `crates/serena-tools/src/file/` (6 files, ~60 lines each)

**Implemented Tools (6):**
1. ✅ `ReadFileTool` - read.rs
2. ✅ `CreateTextFileTool` - write.rs
3. ✅ `ListDirectoryTool` - list.rs
4. ✅ `FindFileTool` - find.rs
5. ✅ `ReplaceContentTool` - replace.rs
6. ✅ `SearchFilesTool` - search.rs

**Architecture:**
- Each tool is self-contained (no FileService layer)
- Tools directly implement `serena_core::Tool` trait
- Use `PathBuf` passed to constructor for project root
- Path validation is built into each tool

### API Comparison

| Feature | serena_core | crates/serena-tools | Match? |
|---------|-------------|---------------------|--------|
| Read file | ✅ read_file | ✅ ReadFileTool | ✅ |
| Write file | ✅ create_text_file | ✅ CreateTextFileTool | ✅ |
| List dir | ✅ list_dir | ✅ ListDirectoryTool | ✅ |
| Find file | ✅ find_file | ✅ FindFileTool | ✅ |
| Replace content | ✅ replace_content | ✅ ReplaceContentTool | ✅ |
| Search pattern | ✅ search_for_pattern | ✅ SearchFilesTool | ✅ |

### Migration Strategy

**Status: ✅ COMPLETE - No migration needed**

File tools are already fully implemented in crates/ with equivalent functionality. The serena_core FileService layer is redundant and can be removed.

**Action Items:**
1. ✅ Verify parameter compatibility between versions
2. ✅ Compare error handling approaches
3. ✅ Ensure truncation behavior matches (max_answer_chars parameter)
4. ⚠️ Add integration tests comparing behavior

---

## 5. Command Tools Migration

### Current State in serena_core

**Location:** `serena_core/src/mcp/tools/cmd_tools.rs` (234 lines)

**Implemented Tools (1):**
1. `execute_shell_command` - Execute shell commands with timeout and output capture

**Features:**
- Platform-specific shell selection (bash/sh/cmd)
- Working directory parameter
- Output truncation
- Destructive/idempotent/read-only hints
- Timeout support

### Current State in crates/

**Status:** ❌ NOT IMPLEMENTED

**Required Location:** `crates/serena-tools/src/command/` (new directory)

### Migration Strategy

**Step 1: Create Command Tools Module**
```rust
// Location: crates/serena-tools/src/command/mod.rs
mod execute;
pub use execute::ExecuteShellCommandTool;
```

**Step 2: Port Implementation**
```rust
// Location: crates/serena-tools/src/command/execute.rs
// Copy implementation from serena_core/src/mcp/tools/cmd_tools.rs
// Adapt to use serena_core::Tool trait
```

**Step 3: Add to lib.rs**
```rust
// Location: crates/serena-tools/src/lib.rs
pub mod command;
pub use command::ExecuteShellCommandTool;
```

---

## 6. Services Layer - Architecture Decision

### Current Pattern in serena_core

```rust
// serena_core/src/mcp/tools/services.rs

pub struct FileService { /* ... */ }
pub struct SymbolService { /* ... */ }
pub struct MemoryService { /* ... */ }

// Tools depend on services:
impl ReadFileTool {
    pub async fn run_tool(self, service: &FileService) -> Result<CallToolResult> {
        service.read_file(&self.relative_path).await?
    }
}
```

### Proposed Pattern in crates/

**Option A: No Service Layer (Current Approach)**
```rust
// Tools are self-contained with dependencies injected via constructor

pub struct ReadFileTool {
    project_root: PathBuf,
}

#[async_trait]
impl Tool for ReadFileTool {
    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        // Direct file system operations
    }
}
```

**Option B: Shared Context Object**
```rust
// All tools share a context with common services

pub struct ToolContext {
    pub project_root: PathBuf,
    pub lsp_manager: Arc<LanguageServerManager>,
    pub memory_manager: Arc<MemoryManager>,
    pub config_service: Arc<ConfigService>,
}

pub struct ReadFileTool {
    context: Arc<ToolContext>,
}
```

**Recommendation: Use Option B (Shared Context)**

Benefits:
- Consistent dependency injection
- Easy to share services (LSP, memory, config)
- Testable with mock context
- Matches pattern in crates/serena/src/app.rs

---

## 7. Missing Features Summary

### Features in serena_core but NOT in crates/

| Feature | Location | Priority | Complexity |
|---------|----------|----------|------------|
| ❌ `insert_after_symbol` tool | Symbol tools | High | Medium |
| ❌ `insert_before_symbol` tool | Symbol tools | High | Medium |
| ❌ `restart_language_server` tool | LSP tools | Medium | Low |
| ❌ `execute_shell_command` tool | Command tools | High | Low |
| ❌ `get_current_config` tool | Config tools | High | Low |
| ❌ `initial_instructions` tool | Config tools | Medium | Low |
| ❌ `think_different` tool | Workflow tools | Low | Low |
| ❌ Memory MCP tool wrappers | Memory tools | High | Low |

### Features in crates/ but NOT in serena_core

| Feature | Location | Benefit |
|---------|----------|---------|
| ✅ Multi-language LSP support | serena-lsp/manager.rs | Major |
| ✅ LSP cache with TTL | serena-lsp/cache.rs | Performance |
| ✅ `restart_server` method | serena-lsp/manager.rs | Reliability |
| ✅ SQLite memory metadata | serena-memory/store.rs | Search, metadata |
| ✅ Memory search | serena-memory/manager.rs | Discoverability |
| ✅ Config service with state | serena-config/service.rs | Management |
| ✅ Editor tools (delete/insert lines) | serena-tools/editor/ | Precision |

---

## 8. Migration Checklist

### Phase 1: Foundation (Week 1)

- [ ] **Create ToolContext struct** in crates/serena-core
  - [ ] Define shared context with all managers
  - [ ] Add constructor with project_root
  - [ ] Add builder pattern for testing

- [ ] **Create memory tool wrappers** in crates/serena-tools/src/memory/
  - [ ] WriteMemoryTool
  - [ ] ReadMemoryTool
  - [ ] ListMemoriesTool
  - [ ] DeleteMemoryTool
  - [ ] EditMemoryTool
  - [ ] Integration tests

- [ ] **Port command tools** to crates/serena-tools/src/command/
  - [ ] ExecuteShellCommandTool
  - [ ] Platform-specific shell selection
  - [ ] Timeout and output handling
  - [ ] Security considerations

### Phase 2: Configuration (Week 2)

- [ ] **Create config tool wrappers** in crates/serena-tools/src/config/
  - [ ] GetCurrentConfigTool (uses ConfigService + ToolRegistry)
  - [ ] InitialInstructionsTool (port SERENA_INSTRUCTIONS)
  - [ ] Integration with existing workflow tools

- [ ] **Enhance onboarding tools**
  - [ ] Update CheckOnboardingPerformedTool to use MemoryManager
  - [ ] Verify OnboardingTool instructions match serena_core

- [ ] **Add ThinkDifferentTool** to workflow/
  - [ ] Port from serena_core/config_tools.rs
  - [ ] Add to workflow module exports

### Phase 3: Symbol Tools (Week 3)

- [ ] **Design multi-language symbol tool API**
  - [ ] Decide: auto-detect language vs explicit parameter
  - [ ] Create helper for language detection from file path
  - [ ] Update all symbol tool parameter structs

- [ ] **Port existing symbol tools** to crates/serena-tools/src/symbol/
  - [ ] GetSymbolsOverviewTool
  - [ ] FindSymbolTool
  - [ ] FindReferencingSymbolsTool
  - [ ] ReplaceSymbolBodyTool
  - [ ] RenameSymbolTool

- [ ] **Implement missing insert tools**
  - [ ] InsertAfterSymbolTool (new implementation)
  - [ ] InsertBeforeSymbolTool (new implementation)
  - [ ] Helper: find_symbol_range function
  - [ ] Edge case handling (symbol not found, multiple matches)

- [ ] **Add LSP control tools** in crates/serena-tools/src/lsp/
  - [ ] RestartLanguageServerTool
  - [ ] ListRunningServersT tool (optional)
  - [ ] ClearLspCacheTool (optional)

### Phase 4: Integration (Week 4)

- [ ] **Update serena MCP server** in crates/serena/
  - [ ] Replace serena_core tools with crates/serena-tools
  - [ ] Create ToolContext in App::new()
  - [ ] Register all tools in tool_registry
  - [ ] Remove dependencies on serena_core/mcp/tools

- [ ] **Testing**
  - [ ] Port integration tests from serena_core/tests/
  - [ ] Add new tests for multi-language support
  - [ ] Test memory manager integration
  - [ ] Test config service integration
  - [ ] Performance benchmarks (LSP cache effectiveness)

- [ ] **Documentation**
  - [ ] Update tool documentation
  - [ ] Document ToolContext pattern
  - [ ] Add migration guide from Python serena
  - [ ] Update CLAUDE.md with new tool names

### Phase 5: Cleanup (Week 5)

- [ ] **Remove deprecated code**
  - [ ] Delete serena_core/src/mcp/tools/ (except utils if needed)
  - [ ] Remove FileService, SymbolService, MemoryService
  - [ ] Clean up old tests

- [ ] **Verification**
  - [ ] All 20+ tools working in MCP server
  - [ ] Claude Desktop integration tested
  - [ ] Performance meets or exceeds serena_core
  - [ ] Documentation complete

---

## 9. Code Patterns to Follow

### Tool Implementation Pattern

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serena_core::{Tool, ToolResult, SerenaError};
use std::sync::Arc;

// 1. Tool struct with dependencies
#[derive(Clone)]
pub struct ExampleTool {
    context: Arc<ToolContext>,
}

// 2. Parameter struct for type-safe deserialization
#[derive(Debug, Deserialize)]
struct ExampleParams {
    required_field: String,
    #[serde(default)]
    optional_field: Option<String>,
}

// 3. Implement Tool trait
#[async_trait]
impl Tool for ExampleTool {
    fn name(&self) -> &str {
        "example_tool"
    }

    fn description(&self) -> &str {
        "Brief description of what this tool does"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "required_field": {
                    "type": "string",
                    "description": "Field description"
                },
                "optional_field": {
                    "type": "string",
                    "description": "Optional field"
                }
            },
            "required": ["required_field"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        // 4. Deserialize and validate parameters
        let params: ExampleParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidInput(e.to_string()))?;

        // 5. Execute tool logic
        let result = perform_operation(&params)?;

        // 6. Return success result
        Ok(ToolResult::success(serde_json::json!({
            "message": "Operation completed",
            "data": result
        })))
    }

    fn tags(&self) -> Vec<String> {
        vec!["category".to_string(), "subcategory".to_string()]
    }
}

// 7. Constructor
impl ExampleTool {
    pub fn new(context: Arc<ToolContext>) -> Self {
        Self { context }
    }
}

// 8. Helper functions
fn perform_operation(params: &ExampleParams) -> Result<String, SerenaError> {
    // Implementation
    Ok("result".to_string())
}
```

### Error Handling Pattern

```rust
use serena_core::SerenaError;

// Convert various error types to SerenaError
async fn example_operation() -> Result<String, SerenaError> {
    // I/O errors
    let content = tokio::fs::read_to_string("file.txt")
        .await
        .map_err(|e| SerenaError::FileNotFound(format!("Cannot read file: {}", e)))?;

    // JSON errors
    let parsed: Value = serde_json::from_str(&content)
        .map_err(|e| SerenaError::InvalidInput(format!("Invalid JSON: {}", e)))?;

    // LSP errors (custom variant)
    let client = lsp_manager.get_server(Language::Rust)
        .await
        .map_err(|e| SerenaError::Internal(format!("LSP error: {}", e)))?;

    // Path errors
    let path = PathBuf::from("relative/path");
    if !path.exists() {
        return Err(SerenaError::FileNotFound(path.display().to_string()));
    }

    Ok(content)
}
```

### Testing Pattern

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use serena_config::ConfigService;
    use serena_lsp::LanguageServerManager;
    use serena_memory::MemoryManager;

    fn create_test_context() -> Arc<ToolContext> {
        let temp = TempDir::new().unwrap();
        let project_root = temp.path().to_path_buf();

        Arc::new(ToolContext {
            project_root: project_root.clone(),
            lsp_manager: Arc::new(LanguageServerManager::new(project_root.clone())),
            memory_manager: Arc::new(MemoryManager::new(&project_root).unwrap()),
            config_service: Arc::new(ConfigService::new()),
        })
    }

    #[tokio::test]
    async fn test_tool_execution() {
        let context = create_test_context();
        let tool = ExampleTool::new(context);

        let params = serde_json::json!({
            "required_field": "test"
        });

        let result = tool.execute(params).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        assert_eq!(tool_result.status, ToolStatus::Success);
    }

    #[tokio::test]
    async fn test_tool_validation() {
        let context = create_test_context();
        let tool = ExampleTool::new(context);

        // Missing required field
        let params = serde_json::json!({});
        let result = tool.execute(params).await;
        assert!(result.is_err());
    }
}
```

---

## 10. Architecture Decisions Record

### ADR-001: Service Layer Elimination

**Decision:** Eliminate FileService/SymbolService/MemoryService in favor of direct manager usage

**Rationale:**
- crates/ pattern uses direct manager injection (MemoryManager, LanguageServerManager)
- Service layer was thin wrapper adding no value
- Direct usage reduces indirection and improves testability

**Consequences:**
- Tools receive Arc&lt;Manager&gt; directly via ToolContext
- Simpler dependency graph
- Migration requires updating all tool constructors

### ADR-002: Multi-Language LSP Support

**Decision:** All symbol tools support multi-language via language parameter or auto-detection

**Rationale:**
- LanguageServerManager supports multiple languages
- Real projects often mix languages (Rust + TOML, TypeScript + JSON, etc.)
- serena_core's single-language assumption is limiting

**Consequences:**
- Symbol tool parameters get optional `language: Option<Language>` field
- Auto-detection from file extension when language not specified
- Need helper function: `detect_language(path: &str) -> Language`

### ADR-003: ToolContext Pattern

**Decision:** All tools receive Arc&lt;ToolContext&gt; with shared managers

**Rationale:**
- Consistent dependency injection across all tools
- Easy to mock for testing
- Supports sharing expensive resources (LSP clients, DB connections)
- Matches pattern in crates/serena/src/app.rs

**Consequences:**
- All tool constructors take `Arc<ToolContext>`
- ToolContext creation happens once in App::new()
- Tests create lightweight test context

---

## 11. Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| API breaking changes in rust-mcp-sdk | Medium | High | Pin version, monitor upstream |
| LSP server compatibility issues | Medium | Medium | Maintain compatibility layer |
| Performance regression from multi-language | Low | Medium | Benchmark before/after |
| Migration introduces bugs | High | High | Comprehensive test coverage |
| Incomplete tool coverage | Medium | High | Checklist-driven development |

---

## 12. Success Criteria

✅ **Must Have:**
- [ ] All 20+ tools from serena_core working in crates/
- [ ] No regressions in functionality
- [ ] All integration tests passing
- [ ] MCP protocol compliance verified

✅ **Should Have:**
- [ ] Performance equal to or better than serena_core
- [ ] insert_after_symbol and insert_before_symbol implemented
- [ ] restart_language_server tool working
- [ ] Multi-language LSP support functional

✅ **Nice to Have:**
- [ ] Enhanced memory search using SQLite
- [ ] LSP cache metrics/monitoring
- [ ] Automated migration script for Python configs

---

## 13. Timeline Estimate

| Phase | Duration | Dependencies |
|-------|----------|--------------|
| Phase 1: Foundation | 5 days | None |
| Phase 2: Configuration | 3 days | Phase 1 |
| Phase 3: Symbol Tools | 7 days | Phase 1 |
| Phase 4: Integration | 5 days | Phases 1-3 |
| Phase 5: Cleanup | 2 days | Phase 4 |
| **Total** | **22 days** | ~4-5 weeks |

---

## Next Steps

1. **Review this document** with team/maintainers
2. **Create GitHub issues** for each checklist item
3. **Set up project board** for tracking
4. **Start with Phase 1** - Foundation work is blocking

---

**Document Version:** 1.0
**Last Updated:** 2025-12-25
**Author:** Claude (Sonnet 4.5)
**Status:** Draft - Awaiting Review
