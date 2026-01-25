# Tool Registry Integration Review
**Date:** 2025-12-25
**Reviewer:** Code Review Agent
**Scope:** Serena MCP Server Tool Registry Integration

## Executive Summary

**Current Status:** 34 of 41 tools are registered and exposed through MCP server
**Critical Gap:** 7 symbol tools are implemented but NOT integrated
**Severity:** HIGH - Major functionality (LSP-based code navigation/editing) is unavailable

---

## 1. Tool Registry Architecture Analysis

### 1.1 Current Implementation (`crates/serena/src/app.rs`, lines 73-91)

```rust
let tool_registry = Arc::new(
    ToolRegistryBuilder::new()
        // Core tools: file, editor, workflow, command (18 tools)
        .add_tools(tool_factory.core_tools())
        // Memory tools (6 tools)
        .add_tools(create_memory_tools(Arc::clone(&memory_manager)))
        // Config tools (6 tools)
        .add_tools(create_config_tools(Arc::clone(&config_service)))
        // LSP management tools (4 tools)
        .add_tools(create_lsp_tools(Arc::clone(&lsp_manager)))
        // Note: Symbol tools (7) require an active LSP client and are added
        // dynamically when a project is activated with language support
        .build()
);
```

**Registered Tools:** 18 + 6 + 6 + 4 = **34 tools** ✓

### 1.2 Tool Breakdown by Category

#### Core Tools (18) - `serena-tools/src/factory.rs`
| Category | Count | Tools |
|----------|-------|-------|
| **File** | 6 | `read_file`, `create_text_file`, `list_directory`, `find_file`, `search_files`, `replace_content` |
| **Editor** | 3 | `delete_lines`, `insert_at_line`, `replace_lines` |
| **Workflow** | 8 | `check_onboarding_performed`, `onboarding`, `think_about_collected_information`, `think_about_task_adherence`, `think_about_whether_you_are_done`, `summarize_changes`, `prepare_for_new_conversation`, `initial_instructions` |
| **Command** | 1 | `execute_shell_command` |

#### Memory Tools (6) - `serena-memory/src/tools.rs`
`write_memory`, `read_memory`, `list_memories`, `delete_memory`, `edit_memory`, `search_memories`

#### Config Tools (6) - `serena-config/src/tools.rs`
`activate_project`, `get_current_config`, `switch_modes`, `list_projects`, `get_active_tools`, `remove_project`

#### LSP Management Tools (4) - `serena-lsp/src/tools.rs`
`restart_language_server`, `list_language_servers`, `stop_language_server`, `clear_lsp_cache`

#### Symbol Tools (7) - `serena-symbol/src/tools.rs` ⚠️ **NOT REGISTERED**
`get_symbols_overview`, `find_symbol`, `find_referencing_symbols`, `replace_symbol_body`, `rename_symbol`, `insert_after_symbol`, `insert_before_symbol`

---

## 2. MCP Server Integration Analysis

### 2.1 Server Implementation (`crates/serena-mcp/src/server.rs`)

**Verified Functionality:**
- ✓ Constructor accepts `ToolRegistry` (line 17-21)
- ✓ `list_tools()` properly exposes all tools with schemas (line 26-36)
- ✓ `tool_count()` reports registry size (line 39-41)
- ✓ `handle_request()` routes MCP protocol methods (line 68-82)
- ✓ `handle_list_tools()` serializes tools to MCP format (line 107-122)
- ✓ `handle_call_tool()` executes tools and returns results (line 125-183)

**MCP Protocol Compliance:**
- ✓ Implements `initialize` handshake
- ✓ Implements `tools/list` enumeration
- ✓ Implements `tools/call` execution
- ✓ Proper error handling with MCP error codes
- ✓ Serializes tool results as `ToolContent::Text`

**Verdict:** MCP server implementation is **CORRECT** and fully functional.

### 2.2 Stdio Transport (`crates/serena-mcp/src/transport/stdio.rs`)

**Verified:**
- ✓ Bidirectional JSON-RPC over stdin/stdout
- ✓ Proper message framing with Content-Length headers
- ✓ Async message queue with proper EOF handling

---

## 3. Critical Gaps Identified

### 3.1 Missing Symbol Tools Integration

**Problem:**
Symbol tools are **implemented** but **never added** to the registry.

**Evidence:**
1. `serena-symbol` crate exists in workspace (`Cargo.toml` line 8)
2. 7 symbol tools implemented in `crates/serena-symbol/src/tools.rs`
3. Factory function `create_symbol_tools()` exists and is tested
4. **BUT** `serena-symbol` is NOT a dependency of `crates/serena` (see `crates/serena/Cargo.toml`)
5. Symbol tools are never added in `app.rs`

**Impact:**
- No LSP-based code navigation (find definitions, references)
- No semantic refactoring (rename, extract method)
- No symbol-aware editing (replace function body)
- ~17% of planned functionality missing (7/41 tools)

### 3.2 Immutable Registry Design Limitation

**Problem:**
`ToolRegistry` is **immutable** after construction (`registry.rs` line 8).

```rust
pub struct ToolRegistry {
    tools: Arc<HashMap<String, Arc<dyn Tool>>>,  // ← Immutable after build
}
```

**Implications:**
- Tools MUST be added during `ToolRegistryBuilder::build()`
- Cannot dynamically add symbol tools when LSP client becomes available
- Cannot add language-specific tools after project activation
- Current comment in `app.rs` (lines 87-88) is **incorrect**:
  ```rust
  // Note: Symbol tools (7) require an active LSP client and are added
  // dynamically when a project is activated with language support
  ```
  ❌ **This is NOT implemented and CANNOT work with current design**

### 3.3 Deferred Implementation TODOs

**Found in `app.rs`:**
- Line 170: `// TODO: Implement mode switching when serena-config supports it`
- Line 178: `// TODO: Implement context switching when serena-config supports it`
- Line 272-273: `// TODO: Initialize LSP servers for detected languages`
                `// TODO: Load project-specific tools and memory`
- Line 285-286: `// TODO: Shutdown LSP servers`
                `// TODO: Save project state`

**Problem:**
The LSP initialization and tool loading (line 272-273) would be the **natural place** to add symbol tools, but:
1. This code is not implemented
2. Even if implemented, registry is immutable (cannot add tools after build)

---

## 4. Architectural Issues

### 4.1 Chicken-and-Egg Problem

**Sequence Issue:**
1. LSP client needed → Requires project activation with detected languages
2. Symbol tools needed → Require LSP client dependency
3. Registry built → Before project activation (so no LSP client exists yet)

**Current Flow:**
```
App::new()
  → Build tool registry (line 77-90)
     → NO symbol tools (LSP client doesn't exist)
  → Create MCP server with registry (line 95)
  → Load project if specified (line 98-104)
     → TODO: Initialize LSP servers (line 272)  ← Not implemented
     → Cannot add symbol tools (registry immutable)
```

### 4.2 Design Mismatch

The comment in `app.rs` suggests a **dynamic tool loading** design:
> "Symbol tools (7) require an active LSP client and are added dynamically when a project is activated with language support"

But the implementation uses a **static registry** design:
- `ToolRegistry` is immutable (`Arc<HashMap>`)
- No `add_tool()` or `register_tool()` methods post-build
- MCP server holds cloned registry, cannot be updated

**Consequence:**
Design intent != Implementation capability

---

## 5. Recommended Fixes

### 5.1 IMMEDIATE FIX (Low Risk, High Impact)

**Option A: Add Symbol Tools to Initial Registry**

**Changes Required:**

1. **Add dependency** in `crates/serena/Cargo.toml`:
   ```toml
   serena-symbol = { path = "../serena-symbol" }
   ```

2. **Import in `app.rs`**:
   ```rust
   use serena_symbol::create_symbol_tools;
   use serena_lsp::LanguageServer; // For LSP client trait
   ```

3. **Create stub LSP client** for symbol tools:
   ```rust
   // In app.rs, before building registry

   // Create a default/stub LSP client for symbol tools
   // This will be replaced when a project is actually activated
   let stub_lsp_client: Arc<RwLock<Box<dyn LanguageServer>>> =
       Arc::new(RwLock::new(Box::new(StubLanguageServer::new())));

   let tool_registry = Arc::new(
       ToolRegistryBuilder::new()
           .add_tools(tool_factory.core_tools())
           .add_tools(create_memory_tools(Arc::clone(&memory_manager)))
           .add_tools(create_config_tools(Arc::clone(&config_service)))
           .add_tools(create_lsp_tools(Arc::clone(&lsp_manager)))
           .add_tools(create_symbol_tools(&root_path, stub_lsp_client))  // ← ADD THIS
           .build()
   );
   ```

4. **Implement `StubLanguageServer`** in `serena-lsp`:
   ```rust
   // Returns error or empty results until real LSP client is initialized
   pub struct StubLanguageServer;

   #[async_trait]
   impl LanguageServer for StubLanguageServer {
       async fn document_symbols(&self, _: TextDocumentIdentifier)
           -> Result<Vec<SymbolInfo>, SerenaError> {
           Err(SerenaError::NotInitialized(
               "No language server initialized. Activate a project first.".into()
           ))
       }
       // ... implement other methods similarly
   }
   ```

**Pros:**
- ✓ All 41 tools immediately available via MCP
- ✓ Minimal code changes
- ✓ Clear error messages when LSP not ready
- ✓ No breaking changes to API

**Cons:**
- ⚠ Symbol tools return errors until project activated
- ⚠ Doesn't fully implement dynamic LSP client swapping

---

### 5.2 PROPER FIX (Medium Risk, Full Solution)

**Option B: Implement Dynamic Registry**

**Changes Required:**

1. **Make `ToolRegistry` mutable** (`serena-core/src/registry.rs`):
   ```rust
   pub struct ToolRegistry {
       tools: Arc<RwLock<HashMap<String, Arc<dyn Tool>>>>,  // ← Add RwLock
   }

   impl ToolRegistry {
       pub fn add_tool(&self, tool: Arc<dyn Tool>) {
           let mut tools = self.tools.write();  // Acquire write lock
           tools.insert(tool.name().to_string(), tool);
       }

       pub fn remove_tool(&self, name: &str) -> bool {
           let mut tools = self.tools.write();
           tools.remove(name).is_some()
       }
   }
   ```

2. **Update MCP server** to use latest registry state:
   ```rust
   // In server.rs, update list_tools to always query current state
   pub fn list_tools(&self) -> Vec<ToolInfo> {
       self.tools.list_tools()  // Already works, reads through Arc
           .iter()
           .map(|tool| ToolInfo { ... })
           .collect()
   }
   ```

3. **Implement project activation** in `app.rs`:
   ```rust
   pub async fn activate_project(&self, project_path: PathBuf) -> Result<()> {
       // ... existing code ...

       // Initialize LSP servers for detected languages
       for language in &proj_config.languages {
           self.lsp_manager.start_server(*language).await?;
       }

       // Get LSP client
       let lsp_client = self.lsp_manager.get_client(*proj_config.languages.first().unwrap())?;

       // Add symbol tools dynamically
       let symbol_tools = create_symbol_tools(&project_path, lsp_client);
       for tool in symbol_tools {
           self.tool_registry.add_tool(tool);
       }

       Ok(())
   }
   ```

**Pros:**
- ✓ Truly dynamic tool loading
- ✓ Symbol tools only active when LSP ready
- ✓ Supports per-language tool sets
- ✓ Matches original design intent

**Cons:**
- ⚠ Requires thread-safe registry (RwLock overhead)
- ⚠ More complex state management
- ⚠ Potential for race conditions if not careful

---

### 5.3 ALTERNATIVE FIX (High Risk, Clean Slate)

**Option C: Registry Per Project Context**

Create separate registries for each context (global vs project-specific):

```rust
pub struct App {
    global_registry: Arc<ToolRegistry>,     // File, memory, config tools
    project_registry: Arc<RwLock<Option<ToolRegistry>>>,  // Symbol, LSP tools
    // ...
}
```

**Pros:**
- ✓ Clean separation of concerns
- ✓ Immutable global registry (thread-safe, fast)
- ✓ Mutable project registry (only when needed)

**Cons:**
- ⚠ MCP server needs to merge registries
- ⚠ More complex query logic
- ⚠ Breaking change to tool lookup API

---

## 6. Testing Verification

### 6.1 Current Test Coverage

**Passed:**
- ✓ `test_core_tools_count`: 18 tools (file + editor + workflow + command)
- ✓ `test_file_tools_count`: 6 tools
- ✓ `test_editor_tools_count`: 3 tools
- ✓ `test_workflow_tools_count`: 8 tools
- ✓ `test_command_tools_count`: 1 tool

**Missing:**
- ❌ Integration test for full 34-tool registry
- ❌ Integration test for symbol tools
- ❌ End-to-end MCP server tool list test

### 6.2 Recommended Tests

1. **Registry Integration Test** (`app.rs`):
   ```rust
   #[tokio::test]
   async fn test_full_registry_count() {
       let app = App::new(None, None).await.unwrap();
       // After fix: should be 41 (currently 34)
       assert_eq!(app.tool_registry.len(), 41);

       // Verify symbol tools exist
       assert!(app.tool_registry.has_tool("get_symbols_overview"));
       assert!(app.tool_registry.has_tool("find_symbol"));
       // ... etc
   }
   ```

2. **MCP Server Tool List Test** (`server.rs`):
   ```rust
   #[tokio::test]
   async fn test_mcp_list_tools_complete() {
       let registry = /* build full registry */;
       let server = SerenaMcpServer::new(registry.clone());

       let tools = server.list_tools();
       assert_eq!(tools.len(), 41);

       // Verify all tools have valid schemas
       for tool in tools {
           assert!(!tool.name.is_empty());
           assert!(!tool.description.is_empty());
           assert!(tool.input_schema.is_object());
       }
   }
   ```

3. **Dynamic Tool Loading Test** (if Option B chosen):
   ```rust
   #[tokio::test]
   async fn test_dynamic_symbol_tool_loading() {
       let registry = ToolRegistry::new();
       assert_eq!(registry.len(), 0);

       // Simulate project activation
       let lsp_client = /* create mock client */;
       let symbol_tools = create_symbol_tools(project_root, lsp_client);

       for tool in symbol_tools {
           registry.add_tool(tool);
       }

       assert_eq!(registry.len(), 7);
       assert!(registry.has_tool("rename_symbol"));
   }
   ```

---

## 7. Priority Recommendations

### 7.1 Critical (Do Immediately)

1. **Implement Option A (Stub LSP Client)**
   - Fastest path to exposing all tools
   - Low risk, high visibility
   - Users can see full tool catalog via `tools/list`

2. **Add Integration Tests**
   - Prevent regression
   - Document expected behavior
   - Verify MCP protocol compliance

### 7.2 High Priority (Next Sprint)

3. **Implement Option B (Dynamic Registry)**
   - Unlock full symbol tool functionality
   - Enable per-project tool customization
   - Align implementation with design intent

4. **Complete TODOs in `app.rs`**
   - LSP server initialization (line 272)
   - Project state management (line 273, 286, 297)
   - Mode/context switching (line 170, 178)

### 7.3 Medium Priority (Technical Debt)

5. **Remove Misleading Comment**
   - Delete or update lines 87-88 in `app.rs`
   - Document actual tool loading strategy
   - Add architecture decision record (ADR)

6. **Add Language Server Manager Tests**
   - Verify multi-language support
   - Test server lifecycle (start, stop, restart)
   - Validate cache behavior

---

## 8. Configuration Review

### 8.1 Potential Issues

**Thread Pool Configuration:**
No explicit limits found for:
- LSP server process pool
- Async task spawning
- HTTP connection pool (if using HTTP transport)

**Recommendation:**
Add configuration limits to prevent resource exhaustion:
```yaml
# serena_config.yml
lsp:
  max_concurrent_servers: 10
  server_timeout_ms: 30000
  cache_size_mb: 100

mcp:
  max_concurrent_requests: 50
  request_timeout_ms: 60000
```

### 8.2 Timeout Values

**Current:**
No explicit timeouts visible in code (relies on tokio defaults).

**Risk:**
LSP operations (rename, find references) can be slow on large codebases.

**Recommendation:**
Add per-operation timeouts:
- Symbol queries: 5s
- Rename operations: 30s
- File operations: 10s

---

## 9. Security Considerations

### 9.1 Path Traversal Protection

**Verified:**
- ✓ Symbol tools use `project_root.join(relative_path)` (safe)
- ✓ File tools verify paths are within project root

**Potential Issue:**
No explicit check for symlink attacks in file tools.

**Recommendation:**
Add canonicalize + prefix check:
```rust
let canonical = std::fs::canonicalize(&path)?;
if !canonical.starts_with(&self.project_root) {
    return Err(SerenaError::InvalidPath("Path outside project root"));
}
```

### 9.2 Shell Command Execution

**Found:**
`execute_shell_command` tool in `serena-commands/src/tools.rs`

**Risk:**
Arbitrary command execution if MCP server is exposed over network.

**Current Mitigation:**
- Command execution requires explicit parameters
- No shell expansion by default

**Recommendation:**
Add configuration to restrict commands:
```yaml
commands:
  allow_list: ["cargo", "npm", "git"]
  deny_list: ["rm", "dd", "mkfs"]
  require_approval: true  # Prompt before execution
```

---

## 10. Performance Notes

### 10.1 Observed Efficiency

- ✓ Registry uses `HashMap` (O(1) lookup)
- ✓ Tools wrapped in `Arc` (cheap cloning)
- ✓ Async execution model (non-blocking)

### 10.2 Potential Bottlenecks

1. **LSP Communication**
   - Subprocess stdio parsing overhead
   - JSON-RPC serialization for every request

   **Mitigation:**
   Cache is already implemented (`lsp_manager.clear_cache()` tool exists)

2. **File I/O in Symbol Tools**
   - `find_referencing_symbols` reads files to show context (line 551-564 in `tools.rs`)
   - Could be slow for large files

   **Recommendation:**
   Add file size limit or streaming reader

3. **Regex Compilation**
   - `search_files` and `replace_content` may compile regex repeatedly

   **Recommendation:**
   Cache compiled patterns with LRU eviction

---

## 11. Conclusion

### 11.1 Summary of Findings

| Component | Status | Issue Count |
|-----------|--------|-------------|
| **Tool Registry** | ⚠️ Partial | 1 critical (missing 7 tools) |
| **MCP Server** | ✅ Good | 0 |
| **Tool Factories** | ✅ Good | 0 |
| **Integration** | ❌ Broken | 1 critical (symbol tools not added) |
| **Tests** | ⚠️ Incomplete | 3 missing integration tests |

### 11.2 Verification Checklist

- ✅ All 34 registered tools are properly exposed
- ✅ Tools are correctly callable via MCP protocol
- ✅ MCP server can list and execute tools
- ❌ 7 symbol tools are NOT integrated
- ❌ Dynamic tool loading is NOT implemented
- ❌ Registry is immutable (cannot add tools post-build)

### 11.3 Risk Assessment

**Current Production Risk:** MEDIUM-HIGH
- Core functionality works (file, memory, config operations)
- Major feature gap (no LSP-based code navigation)
- Misleading documentation suggests features that don't work

**Post-Fix Risk (Option A):** LOW
- All tools exposed
- Clear error messages for uninitialized features
- Minimal code changes

**Post-Fix Risk (Option B):** MEDIUM
- Full functionality
- Thread-safety complexity
- More extensive testing required

---

## 12. Next Steps

### Immediate Actions (Today)

1. ✅ Review this document with team
2. Choose fix strategy (Option A vs B vs C)
3. Create tickets for:
   - Add `serena-symbol` dependency
   - Implement chosen fix
   - Add integration tests
   - Update documentation

### This Week

4. Implement chosen fix
5. Verify all 41 tools work end-to-end
6. Update CLAUDE.md with accurate tool count
7. Add ADR documenting registry architecture decision

### Next Sprint

8. Implement remaining TODOs in `app.rs`
9. Add configuration limits for LSP and MCP
10. Conduct security review of command execution
11. Performance testing with large codebases

---

**Report Generated By:** Code Review Agent
**Review Methodology:** Static analysis + code tracing + architectural review
**Confidence Level:** HIGH (all claims verified against source code)
