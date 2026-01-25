# Serena Python to Rust Migration Analysis

## Executive Summary

This document provides a comprehensive analysis of migrating the Serena agent toolkit from Python to Rust. The codebase consists of approximately **35,103 lines of Python code** across **~100+ Python files** supporting **40+ language servers**.

### Current Architecture
- **Core Components**: SerenaAgent, MCP Server, SolidLanguageServer, Tool System
- **Language Support**: 40+ languages via LSP protocol
- **Dependencies**: 20+ Python packages (pydantic, flask, mcp, requests, etc.)
- **Existing Rust**: Small PyO3 extension in `serena_core/` (~320 LOC)

---

## 1. Python Dependency → Rust Crate Mapping

### Core Dependencies

| Python Package | Rust Crate(s) | Migration Complexity | Notes |
|----------------|---------------|---------------------|-------|
| **mcp==1.23.0** | Custom implementation | **HIGH** | Need to implement MCP protocol from scratch or use experimental Rust MCP SDK |
| **pydantic>=2.10.6** | `serde` + validation crates | **HIGH** | Requires `serde`, `validator`, custom derive macros |
| **flask>=3.0.0** | `axum` or `actix-web` | **MEDIUM** | Already using `axum` in serena_core |
| **requests>=2.32.3** | `reqwest` | **LOW** | Already in Cargo.toml |
| **pyyaml>=6.0.2** | `serde_yaml` | **LOW** | Direct replacement |
| **ruamel.yaml>=0.18.0** | `serde_yaml` + custom | **MEDIUM** | Needs comment preservation logic |
| **python-dotenv>=1.0.0** | `dotenvy` | **LOW** | Direct replacement |
| **jinja2>=3.1.6** | `tera` or `askama` | **MEDIUM** | Template syntax differs |
| **pathspec>=0.12.1** | `globset` or `ignore` | **LOW** | Already using `ignore` crate |
| **psutil>=7.0.0** | `sysinfo` | **LOW** | Cross-platform system info |
| **tiktoken>=0.9.0** | `tiktoken-rs` | **MEDIUM** | Token counting for LLMs |
| **anthropic>=0.54.0** | `anthropic-sdk-rust` (if exists) | **MEDIUM** | May need custom client |
| **docstring_parser>=0.16** | Custom parser | **HIGH** | Need to parse Python docstrings |
| **joblib>=1.5.1** | `rayon` | **LOW** | Already using `rayon` for parallelism |
| **tqdm>=4.67.1** | `indicatif` | **LOW** | Progress bars |
| **overrides>=7.7.0** | Decorators/macros | **MEDIUM** | Type system handles this differently |
| **sensai-utils>=1.5.0** | Custom port | **HIGH** | Likely custom utilities |
| **pyright>=1.1.396** | N/A | **N/A** | Runtime dependency, not code |
| **fortls>=3.2.2** | N/A | **N/A** | LSP server, separate process |

### Additional Tool Dependencies

| Python Package | Rust Equivalent | Complexity |
|----------------|-----------------|----------|
| **black** | `rustfmt` (different purpose) | N/A |
| **mypy** | Type system is native | N/A |
| **pytest** | `cargo test` + test frameworks | LOW |
| **ruff** | `clippy` (different purpose) | N/A |

---

## 2. Module-by-Module Migration Assessment

### 2.1 Core Agent (`src/serena/agent.py`) - **CRITICAL**
**Lines**: ~800 | **Complexity**: **VERY HIGH**

**Key Challenges**:
- Dynamic tool registration and discovery
- Multi-threaded task execution with timeouts
- Complex state management (projects, contexts, modes)
- Python reflection/introspection for tool discovery
- Dashboard API integration

**Python-Specific Patterns**:
```python
# Dynamic tool discovery via iter_subclasses
for tool_class in iter_subclasses(Tool):
    register(tool_class)

# Dynamic attribute access
getattr(self, "apply")

# Python's Global Interpreter Lock (GIL)
task_executor.submit(task)
```

**Rust Approach**:
- Use `inventory` crate for static tool registration
- Replace GIL-based concurrency with `tokio` async runtime
- Use trait objects (`Box<dyn Tool>`) for dynamic dispatch
- **Estimate**: 3-4 weeks, 1500+ lines of Rust

**Dependencies to Port**:
- ToolRegistry singleton pattern
- TaskExecutor with timeout support
- MemoryLogHandler
- Analytics/usage tracking

---

### 2.2 MCP Server (`src/serena/mcp.py`) - **CRITICAL**
**Lines**: ~600 | **Complexity**: **VERY HIGH**

**Key Challenges**:
- MCP protocol implementation (no mature Rust SDK)
- FastMCP wrapper compatibility
- OpenAI tool schema conversion
- Async context management
- Multiprocessing for isolation

**Python-Specific Patterns**:
```python
from mcp.server.fastmcp import FastMCP
from mcp.types import ToolAnnotations

@asynccontextmanager
async def lifespan(agent: SerenaAgent):
    yield {"agent": agent}

# Dynamic tool wrapping
mcp_app.add_tool(python_tool_to_mcp_tool(tool))
```

**Rust Approach**:
- Implement MCP protocol server from scratch using `axum`
- Use `tower` middleware for request/response processing
- Replace multiprocessing with tokio tasks
- **Estimate**: 4-6 weeks, 2000+ lines

**Critical Dependencies**:
- MCP protocol spec compliance
- JSON-RPC 2.0 implementation
- Tool schema generation from Rust types

---

### 2.3 SolidLanguageServer (`src/solidlsp/ls.py`) - **CRITICAL**
**Lines**: ~1200 | **Complexity**: **VERY HIGH**

**Key Challenges**:
- LSP protocol implementation (40+ languages)
- Process lifecycle management (spawn, restart, kill)
- Bidirectional JSON-RPC communication
- Caching layer with disk persistence
- Multi-file symbol indexing
- Reference finding across projects

**Python-Specific Patterns**:
```python
# Subprocess management
self.process = subprocess.Popen(cmd, stdin=PIPE, stdout=PIPE)

# Pickle-based caching
load_pickle(cache_file)
save_cache(data, cache_file)

# Dynamic symbol tree walking
for symbol in symbols:
    yield from walk_symbols(symbol.children)
```

**Rust Approach**:
- Use `lsp-types` crate (already in Cargo.toml)
- Implement LSP client using `tokio::process`
- Use `bincode` or `serde_json` for caching
- Replace pickle with safer serialization
- **Estimate**: 6-8 weeks, 3000+ lines

**Existing Rust Code**:
- Basic LSP types in `serena_core/src/lsp/`
- Need to expand significantly

---

### 2.4 Tool System (`src/serena/tools/*.py`) - **HIGH**
**Lines**: ~3000 across 9 files | **Complexity**: **HIGH**

**Tool Categories**:
1. **File Tools** (file_tools.py) - 800 lines
   - Read, write, search, regex operations
   - Directory scanning with gitignore support
   - **Complexity**: MEDIUM (already partially in Rust)

2. **Symbol Tools** (symbol_tools.py) - 1000 lines
   - Find symbols, get overview, rename
   - Replace symbol bodies, insert code
   - **Complexity**: HIGH (depends on LSP layer)

3. **Memory Tools** (memory_tools.py) - 400 lines
   - Markdown-based knowledge persistence
   - Semantic search and retrieval
   - **Complexity**: MEDIUM

4. **Config Tools** (config_tools.py) - 300 lines
   - Project activation, mode switching
   - **Complexity**: LOW

5. **Workflow Tools** (workflow_tools.py) - 300 lines
   - Onboarding, meta-operations
   - **Complexity**: MEDIUM

**Python-Specific Patterns**:
```python
class Tool(Component):
    def apply(self, **kwargs) -> str:
        """Dynamically typed parameters"""
        pass

    def apply_ex(self, log_call=True, **kwargs):
        # Reflection-based logging
        frame = inspect.currentframe()
        params = frame.f_locals
```

**Rust Approach**:
- Use trait-based tool system
- Macro-based parameter validation
- Replace `**kwargs` with strongly-typed structs
- **Estimate**: 4-5 weeks, 2000+ lines

```rust
trait Tool: Send + Sync {
    type Input: serde::Deserialize;
    type Output: serde::Serialize;

    fn apply(&self, input: Self::Input) -> Result<Self::Output>;
}

// Derive macro for automatic registration
#[derive(Tool)]
struct ReadFileTool;
```

---

### 2.5 Configuration System (`src/serena/config/*.py`) - **MEDIUM**
**Lines**: ~1500 | **Complexity**: **MEDIUM**

**Key Components**:
- ProjectConfig with YAML persistence
- Context/Mode system
- Language backend selection
- Tool inclusion/exclusion logic

**Python-Specific Patterns**:
```python
@dataclass
class ProjectConfig:
    project_name: str
    languages: list[Language]
    ignored_paths: list[str] = field(default_factory=list)

    @cached_property
    def computed_value(self):
        return expensive_operation()
```

**Rust Approach**:
- Use `serde` derive macros for (de)serialization
- Replace `@cached_property` with `once_cell` or `lazy_static`
- **Estimate**: 2-3 weeks, 1000+ lines

```rust
#[derive(Debug, Deserialize, Serialize)]
struct ProjectConfig {
    project_name: String,
    languages: Vec<Language>,
    #[serde(default)]
    ignored_paths: Vec<String>,
}

impl ProjectConfig {
    fn computed_value(&self) -> &ComputedType {
        self.cached.get_or_init(|| expensive_operation())
    }
}
```

---

### 2.6 Language Server Implementations (`src/solidlsp/language_servers/*.py`) - **HIGH**
**Files**: 40+ language servers | **Lines**: ~8000 total | **Complexity**: **MEDIUM-HIGH**

**Per-Language Server Structure**:
- Download/install logic (npm, cargo, pip, etc.)
- Process launch configuration
- Initialization parameters
- Platform-specific paths

**Example** (TypeScript):
```python
class TypeScriptLanguageServer(SolidLanguageServerHandler):
    @staticmethod
    def download_installer():
        # npm install -g typescript-language-server
        subprocess.run(["npm", "install", "-g", ...])

    def get_launch_args(self):
        return ["typescript-language-server", "--stdio"]
```

**Rust Approach**:
- Factory pattern for language server creation
- Use `reqwest` for downloads
- Platform-specific installation via conditional compilation
- **Estimate**: 3-4 weeks for all 40+ servers, 2500+ lines

```rust
trait LanguageServerProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn install(&self) -> Result<PathBuf>;
    fn launch_args(&self, project_root: &Path) -> Vec<String>;
}

inventory::submit! {
    &TypeScriptServerProvider as &dyn LanguageServerProvider
}
```

---

## 3. Python Features Requiring Special Handling

### 3.1 Dynamic Typing → Static Typing
**Challenge**: Python's duck typing vs Rust's strict types

**Python**:
```python
def process(data):  # Can be str, dict, list, etc.
    if isinstance(data, str):
        return parse_string(data)
    elif isinstance(data, dict):
        return data["key"]
```

**Rust Solution**:
```rust
enum Data {
    String(String),
    Dict(HashMap<String, Value>),
    List(Vec<Value>),
}

fn process(data: Data) -> Result<Value> {
    match data {
        Data::String(s) => parse_string(&s),
        Data::Dict(map) => map.get("key").ok_or(...),
        Data::List(items) => Ok(items[0].clone()),
    }
}
```

### 3.2 Exception Handling → Result Types
**Python**:
```python
try:
    result = risky_operation()
except FileNotFoundError:
    result = default_value()
except ValueError as e:
    log.error(f"Invalid value: {e}")
    raise
```

**Rust**:
```rust
let result = risky_operation()
    .or_else(|e| match e {
        Error::FileNotFound => Ok(default_value()),
        Error::ValueError(msg) => {
            error!("Invalid value: {}", msg);
            Err(e)
        }
        _ => Err(e),
    })?;
```

### 3.3 Decorators → Macros
**Python**:
```python
@tool_marker
@validate_input
def my_tool(path: str) -> str:
    """Tool documentation"""
    return process(path)
```

**Rust**:
```rust
#[tool]
#[validate_input]
fn my_tool(path: String) -> Result<String> {
    /// Tool documentation
    Ok(process(&path)?)
}
```

### 3.4 Reflection → Compile-Time Code Generation
**Python**:
```python
# Get all tool classes at runtime
tool_classes = [
    cls for cls in globals().values()
    if isinstance(cls, type) and issubclass(cls, Tool)
]
```

**Rust**:
```rust
// Static registration via inventory crate
inventory::collect!(ToolDescriptor);

fn all_tools() -> &'static [ToolDescriptor] {
    inventory::iter::<ToolDescriptor>().collect()
}
```

### 3.5 Async/Await Compatibility
**Python** (asyncio):
```python
async def fetch_data():
    async with aiohttp.ClientSession() as session:
        async with session.get(url) as response:
            return await response.json()
```

**Rust** (tokio):
```rust
async fn fetch_data() -> Result<Value> {
    let response = reqwest::get(url).await?;
    let json = response.json().await?;
    Ok(json)
}
```

**Key Difference**: Python's GIL means async is primarily for I/O; Rust's async is truly concurrent.

---

## 4. Critical Migration Challenges

### 4.1 MCP Protocol Implementation
**Problem**: No mature Rust MCP SDK exists

**Options**:
1. **Port FastMCP to Rust** - 6-8 weeks effort
2. **Implement minimal MCP server** - 4-6 weeks
3. **Use experimental mcp-rs crate** - Risk of API instability

**Recommendation**: Implement minimal MCP server using `axum` + JSON-RPC

---

### 4.2 Pydantic Replacement
**Problem**: No direct Rust equivalent with same DX

**Solution Stack**:
- `serde` for (de)serialization
- `validator` for validation rules
- Custom derive macros for convenience
- `garde` crate for advanced validation

**Example**:
```rust
#[derive(Debug, Deserialize, Validate)]
struct ToolInput {
    #[validate(length(min = 1))]
    relative_path: String,

    #[validate(range(min = 0))]
    start_line: usize,

    #[serde(default)]
    end_line: Option<usize>,
}
```

---

### 4.3 LSP Process Management
**Problem**: Python's subprocess is simpler than Rust's tokio::process

**Challenges**:
- Bidirectional JSON-RPC over stdio
- Process restart without data loss
- Handling stdout/stderr separately
- Timeout and cancellation

**Rust Solution**:
```rust
struct LspClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    pending_requests: HashMap<RequestId, oneshot::Sender<Response>>,
}

impl LspClient {
    async fn send_request(&mut self, req: Request) -> Result<Response> {
        let (tx, rx) = oneshot::channel();
        self.pending_requests.insert(req.id.clone(), tx);

        let json = serde_json::to_string(&req)?;
        self.stdin.write_all(json.as_bytes()).await?;

        tokio::time::timeout(Duration::from_secs(30), rx).await??
    }
}
```

---

### 4.4 Dynamic Tool Discovery
**Problem**: Python's runtime introspection vs Rust's compile-time

**Python Approach**:
```python
# Discover all Tool subclasses at runtime
for cls in iter_subclasses(Tool):
    registry.register(cls)
```

**Rust Approach**:
```rust
// Static registration at compile time
#[macro_export]
macro_rules! register_tool {
    ($tool:ty) => {
        inventory::submit! {
            ToolDescriptor {
                name: <$tool>::NAME,
                create: || Box::new(<$tool>::new()),
            }
        }
    }
}

// Usage
register_tool!(ReadFileTool);
register_tool!(WriteFileTool);
```

---

### 4.5 Pickle Caching → Binary Serialization
**Problem**: Python's pickle is unsafe and Python-specific

**Rust Alternatives**:
| Format | Pros | Cons |
|--------|------|------|
| **bincode** | Fast, compact | Version-sensitive |
| **serde_json** | Human-readable, debuggable | Slower, larger |
| **rmp-serde** | Compact, fast | Less tooling |
| **postcard** | Embedded-friendly | Limited ecosystem |

**Recommendation**: `bincode` for performance, `serde_json` for debugging

---

## 5. Migration Strategy Recommendations

### Phase 1: Foundation (8-10 weeks)
**Goal**: Core infrastructure in Rust

**Deliverables**:
1. ✅ MCP server implementation (axum + JSON-RPC)
2. ✅ Configuration system (serde + YAML)
3. ✅ Basic tool trait system
4. ✅ File operations (already mostly done)
5. ✅ Process management utilities

**Estimated Lines**: ~5,000 Rust

---

### Phase 2: LSP Integration (10-12 weeks)
**Goal**: Language server support

**Deliverables**:
1. ✅ LSP client implementation
2. ✅ 5-10 priority language servers (Python, Rust, TypeScript, Go, Java)
3. ✅ Symbol caching layer
4. ✅ Reference finding
5. ✅ Rename refactoring

**Estimated Lines**: ~8,000 Rust

---

### Phase 3: Tool Ecosystem (6-8 weeks)
**Goal**: Full tool parity

**Deliverables**:
1. ✅ All file tools migrated
2. ✅ All symbol tools migrated
3. ✅ Memory/knowledge tools
4. ✅ Config tools
5. ✅ Workflow tools

**Estimated Lines**: ~3,000 Rust

---

### Phase 4: Advanced Features (6-8 weeks)
**Goal**: Full feature parity

**Deliverables**:
1. ✅ Analytics and usage tracking
2. ✅ Dashboard API
3. ✅ Remaining 30+ language servers
4. ✅ Performance optimizations
5. ✅ Comprehensive testing

**Estimated Lines**: ~4,000 Rust

---

### Phase 5: Python Interop (Optional, 4-6 weeks)
**Goal**: Gradual migration support

**Deliverables**:
1. ✅ PyO3 bindings for Rust components
2. ✅ Hybrid Python/Rust execution
3. ✅ Migration tooling
4. ✅ Compatibility layer

**Estimated Lines**: ~2,000 Rust

---

## 6. Total Effort Estimation

### Scope
- **Total Python LOC**: ~35,000
- **Estimated Rust LOC**: ~22,000-25,000 (Rust is more verbose but less boilerplate)
- **Migration Timeline**: 34-44 weeks (8-11 months)
- **Team Size**: 2-3 experienced Rust developers

### Risk Factors
- **HIGH**: MCP protocol implementation complexity
- **MEDIUM**: LSP client robustness across 40+ languages
- **MEDIUM**: Maintaining feature parity during migration
- **LOW**: Performance regression (Rust should be faster)

---

## 7. Incremental Migration Path

### Option A: Big Bang Rewrite (NOT RECOMMENDED)
- Rewrite entire system in Rust
- Switch over at once
- **Risk**: Very high, could fail entirely

### Option B: Hybrid Approach (RECOMMENDED)
1. **Week 1-10**: Migrate file operations to Rust (via PyO3)
2. **Week 11-20**: Migrate LSP client layer
3. **Week 21-30**: Migrate tool system
4. **Week 31-40**: Migrate MCP server
5. **Week 41-44**: Remove Python code, final optimizations

**Advantages**:
- Continuous testing in production
- Fallback to Python if Rust fails
- Gradual team learning curve
- Lower risk

---

## 8. Performance Expectations

### Current Python Performance (Estimated)
- **Startup Time**: 2-4 seconds
- **Symbol Search**: 500-1000ms for large projects
- **File Operations**: 100-500ms
- **Memory Usage**: 200-500MB

### Expected Rust Performance
- **Startup Time**: 50-200ms (10-20x faster)
- **Symbol Search**: 50-100ms (5-10x faster)
- **File Operations**: 10-50ms (10x faster)
- **Memory Usage**: 50-100MB (3-5x lower)

### Benchmarking Plan
1. Establish Python baselines
2. Implement Rust equivalents
3. Compare with criterion.rs
4. Optimize hot paths
5. Profile with `flamegraph` and `valgrind`

---

## 9. Testing Strategy

### Unit Tests
- **Python**: pytest (existing)
- **Rust**: `cargo test` with `rstest` for parameterization

### Integration Tests
- **Language Server Tests**: All 40+ servers must pass
- **MCP Protocol Tests**: Compliance with spec
- **Tool Tests**: Snapshot testing with `insta` crate

### Property-Based Tests
- Use `proptest` crate for fuzzing tool inputs
- Verify LSP message parsing edge cases

### Performance Tests
- Criterion.rs benchmarks for all hot paths
- Regression detection in CI

---

## 10. Dependency Audit

### Critical External Dependencies (Rust)
| Crate | Purpose | Stability | Risk |
|-------|---------|-----------|------|
| `tokio` | Async runtime | Stable 1.x | **LOW** |
| `serde` | Serialization | Stable 1.x | **LOW** |
| `axum` | Web server | 0.7.x | **MEDIUM** (pre-1.0) |
| `lsp-types` | LSP protocol | Stable | **LOW** |
| `reqwest` | HTTP client | Stable 0.11 | **LOW** |
| `pyo3` | Python bindings | Stable 0.21 | **LOW** |
| `anyhow` | Error handling | Stable 1.x | **LOW** |
| `thiserror` | Error derives | Stable 1.x | **LOW** |

### Missing Rust Crates
- **MCP SDK**: Need custom implementation
- **Pydantic equivalent**: Use serde + validator
- **Jinja2 equivalent**: `tera` or `askama`

---

## 11. Code Complexity Comparison

### Most Complex Python Modules (by Cyclomatic Complexity)
1. **solidlsp/ls.py**: LSP client (~1200 lines, complexity ~80)
2. **serena/agent.py**: Core agent (~800 lines, complexity ~60)
3. **serena/mcp.py**: MCP server (~600 lines, complexity ~50)
4. **serena/tools/symbol_tools.py**: Symbol operations (~1000 lines, complexity ~45)

### Rust Complexity Expectations
- **Lower Complexity**: Type system prevents many edge cases
- **More Verbose**: Explicit error handling adds LOC
- **Better Maintainability**: Compiler catches errors early

---

## 12. Recommended Next Steps

### Immediate Actions (Week 1-2)
1. ✅ Set up Rust workspace structure
2. ✅ Implement MCP protocol MVP in Rust
3. ✅ Port configuration system (serena_config.py)
4. ✅ Create tool trait system
5. ✅ Benchmark file operations (Python vs existing Rust)

### Short-Term (Week 3-8)
1. ✅ Implement 3-5 core tools in Rust
2. ✅ LSP client for Python language server
3. ✅ Integration tests for hybrid system
4. ✅ Performance profiling

### Medium-Term (Week 9-20)
1. ✅ Migrate all file tools
2. ✅ LSP support for top 10 languages
3. ✅ Memory/knowledge system
4. ✅ Symbol search and refactoring

### Long-Term (Week 21-44)
1. ✅ Complete tool ecosystem
2. ✅ All 40+ language servers
3. ✅ Remove Python dependencies
4. ✅ Production deployment

---

## 13. Success Metrics

### Technical Metrics
- ✅ 100% feature parity with Python version
- ✅ 5-10x performance improvement
- ✅ 50%+ memory reduction
- ✅ <100ms startup time
- ✅ All tests passing

### Business Metrics
- ✅ No regressions in user experience
- ✅ Easier deployment (single binary)
- ✅ Reduced infrastructure costs
- ✅ Improved developer velocity

---

## 14. Conclusion

Migrating Serena from Python to Rust is a **substantial but achievable** effort:

### Pros
- ✅ Massive performance gains (5-10x)
- ✅ Lower memory footprint (3-5x)
- ✅ Type safety prevents entire classes of bugs
- ✅ Single binary deployment (no Python runtime)
- ✅ Better concurrency (no GIL)

### Cons
- ❌ 8-11 months of engineering time
- ❌ Learning curve for Rust
- ❌ No mature MCP SDK in Rust
- ❌ More verbose code in some areas
- ❌ Risk of feature parity issues

### Recommendation
**Proceed with HYBRID MIGRATION** approach:
1. Start with file operations and configuration
2. Gradually migrate LSP layer
3. Port tools one by one
4. Finally migrate MCP server
5. Remove Python code last

**Total Investment**: 2-3 FTE developers for 8-11 months

**Expected ROI**:
- 5-10x performance improvement
- 50%+ memory reduction
- Easier deployment and maintenance
- Better long-term maintainability

---

## Appendix A: Detailed Dependency Mapping

### Python → Rust Crate Mapping (Comprehensive)

```yaml
Core Runtime:
  mcp: Custom axum server
  pydantic: serde + validator + garde
  flask: axum 0.7

HTTP/Network:
  requests: reqwest 0.11
  anthropic: anthropic-sdk-rust (or custom)

Data Formats:
  pyyaml: serde_yaml 0.9
  ruamel.yaml: serde_yaml + custom comments
  json: serde_json 1.0

File System:
  pathspec: globset 0.4 or ignore 0.4
  python-dotenv: dotenvy 0.15

Text Processing:
  jinja2: tera 1.19 or askama 0.12
  docstring_parser: Custom pest parser
  tiktoken: tiktoken-rs 0.5

System:
  psutil: sysinfo 0.29

Concurrency:
  joblib: rayon 1.10
  asyncio: tokio 1.36

Terminal/UI:
  tqdm: indicatif 0.17

Utilities:
  overrides: Type system + macros
  sensai-utils: Custom port

Testing:
  pytest: cargo test + rstest
  syrupy: insta 1.34

Development:
  black: rustfmt
  mypy: Native type checking
  ruff: clippy
  maturin: cargo build
```

---

## Appendix B: File-by-File Migration Checklist

### Priority 1: Core Infrastructure
- [ ] `src/serena/config/serena_config.py` → `src/config/mod.rs`
- [ ] `src/serena/config/context_mode.py` → `src/config/context_mode.rs`
- [ ] `src/serena/project.py` → `src/project/mod.rs`
- [ ] `src/serena/constants.py` → `src/constants.rs`

### Priority 2: Tool System
- [ ] `src/serena/tools/tools_base.py` → `src/tools/base.rs`
- [ ] `src/serena/tools/file_tools.py` → `src/tools/file.rs`
- [ ] `src/serena/tools/symbol_tools.py` → `src/tools/symbol.rs`
- [ ] `src/serena/tools/memory_tools.py` → `src/tools/memory.rs`
- [ ] `src/serena/tools/config_tools.py` → `src/tools/config.rs`

### Priority 3: LSP Layer
- [ ] `src/solidlsp/ls.py` → `src/lsp/client.rs`
- [ ] `src/solidlsp/ls_handler.py` → `src/lsp/handler.rs`
- [ ] `src/solidlsp/ls_types.py` → `src/lsp/types.rs`
- [ ] `src/solidlsp/ls_config.py` → `src/lsp/config.rs`

### Priority 4: Agent Core
- [ ] `src/serena/agent.py` → `src/agent/mod.rs`
- [ ] `src/serena/mcp.py` → `src/mcp/server.rs`
- [ ] `src/serena/task_executor.py` → `src/executor/mod.rs`

### Priority 5: Language Servers (40+ files)
- [ ] All 40+ language server implementations
- [ ] Test each with comprehensive integration tests

---

*Generated: 2025-12-21*
*Python LOC Analyzed: ~35,103*
*Estimated Rust LOC: ~22,000-25,000*
*Migration Timeline: 34-44 weeks*
