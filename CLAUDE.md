# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 🚨 MANDATORY BUILD SYSTEM USAGE

**ALWAYS use the project build system. NEVER use raw cargo/python commands directly.**

### Build System Files (USE THESE)
- `build.ps1` - **Windows PowerShell** (recommended for diagnostics)
- `justfile` - **Cross-platform** (install: `cargo install just`)
- `Makefile.toml` - **cargo-make** (install: `cargo install cargo-make`)

### Pre-Build Diagnostics (RUN FIRST)
```powershell
# Windows - ALWAYS run before builds
.\build.ps1 diag

# Cross-platform
just diag
```

## Development Commands

### Rust Development (USE BUILD SYSTEM)
```powershell
# Windows PowerShell (RECOMMENDED)
.\build.ps1 diag              # Pre-build diagnostics
.\build.ps1 build             # Build workspace
.\build.ps1 build -Release    # Release build
.\build.ps1 test              # Run all tests
.\build.ps1 fix               # Auto-fix all issues
.\build.ps1 ci                # Full CI pipeline
.\build.ps1 lint              # Run clippy

# Cross-platform (just)
just diag                     # Diagnostics
just build-all                # Build all crates
just test                     # Run tests
just fix                      # Auto-fix issues
just check                    # Full quality checks

# cargo-make
cargo make dev                # Quick dev build
cargo make test               # Run tests
cargo make ci                 # CI pipeline
```

### Python Development (use uv)
**Essential Commands (use these exact commands):**
- `uv run poe format` - Format code (BLACK + RUFF) - ONLY allowed formatting command
- `uv run poe type-check` - Run mypy type checking - ONLY allowed type checking command
- `uv run poe test` - Run tests with default markers (excludes java/rust by default)
- `uv run poe test -m "python or go"` - Run specific language tests
- `uv run poe test -m vue` - Run Vue tests
- `uv run poe lint` - Check code style without fixing

**Test Markers:**
Available pytest markers for selective testing:
- `python`, `go`, `java`, `rust`, `typescript`, `vue`, `php`, `perl`, `powershell`, `csharp`, `elixir`, `terraform`, `clojure`, `swift`, `bash`, `ruby`, `ruby_solargraph`
- `snapshot` - for symbolic editing operation tests

**Project Management:**
- `uv run serena-mcp-server` - Start MCP server from project root
- `uv run index-project` - Index project for faster tool performance

### Recommended Workflow
```powershell
# 1. Run diagnostics FIRST (catches environment issues)
.\build.ps1 diag

# 2. Auto-fix any issues
.\build.ps1 fix

# 3. Build
.\build.ps1 build

# 4. Test
.\build.ps1 test

# 5. Full CI before commit
.\build.ps1 ci
```

**NEVER run raw `cargo build` or `cargo test` - ALWAYS use the build system.**

## Architecture Overview

Serena is a dual-layer coding agent toolkit:

### Core Components

**1. SerenaAgent (`src/serena/agent.py`)**
- Central orchestrator managing projects, tools, and user interactions
- Coordinates language servers, memory persistence, and MCP server interface
- Manages tool registry and context/mode configurations

**2. SolidLanguageServer (`src/solidlsp/ls.py`)**  
- Unified wrapper around Language Server Protocol (LSP) implementations
- Provides language-agnostic interface for symbol operations
- Handles caching, error recovery, and multiple language server lifecycle

**3. Tool System (`src/serena/tools/`)**
- **file_tools.py** - File system operations, search, regex replacements
- **symbol_tools.py** - Language-aware symbol finding, navigation, editing
- **memory_tools.py** - Project knowledge persistence and retrieval
- **config_tools.py** - Project activation, mode switching
- **workflow_tools.py** - Onboarding and meta-operations

**4. Configuration System (`src/serena/config/`)**
- **Contexts** - Define tool sets for different environments (desktop-app, agent, ide-assistant)
- **Modes** - Operational patterns (planning, editing, interactive, one-shot)
- **Projects** - Per-project settings and language server configs

### Language Support Architecture

Each supported language has:
1. **Language Server Implementation** in `src/solidlsp/language_servers/`
2. **Runtime Dependencies** - Automatic language server downloads when needed
3. **Test Repository** in `test/resources/repos/<language>/`
4. **Test Suite** in `test/solidlsp/<language>/`

### Memory & Knowledge System

- **Markdown-based storage** in `.serena/memories/` directories
- **Project-specific knowledge** persistence across sessions
- **Contextual retrieval** based on relevance
- **Onboarding support** for new projects

## Development Patterns

### Adding New Languages
1. Create language server class in `src/solidlsp/language_servers/`
2. Add to Language enum in `src/solidlsp/ls_config.py` 
3. Update factory method in `src/solidlsp/ls.py`
4. Create test repository in `test/resources/repos/<language>/`
5. Write test suite in `test/solidlsp/<language>/`
6. Add pytest marker to `pyproject.toml`

### Adding New Tools
1. Inherit from `Tool` base class in `src/serena/tools/tools_base.py`
2. Implement required methods and parameter validation
3. Register in appropriate tool registry
4. Add to context/mode configurations

### Testing Strategy
- Language-specific tests use pytest markers
- Symbolic editing operations have snapshot tests
- Integration tests in `test_serena_agent.py`
- Test repositories provide realistic symbol structures

### E2E Testing (Playwright)
End-to-end tests are located in `e2e/` directory:

```bash
# Install dependencies (first time)
just e2e-install

# Run all E2E tests
just e2e

# Run specific test categories
just e2e-frontend        # Dashboard UI tests
just e2e-integration     # Frontend-Backend integration
just e2e-performance     # Performance benchmarks
just e2e-stability       # Long-running stability tests

# Run with UI (interactive)
just e2e-ui

# View test report
just e2e-report
```

**Test Categories:**
- `@frontend` - WASM Dashboard UI tests (rendering, interactions, accessibility)
- `@integration` - MCP server integration (SSE, tool execution, error handling)
- `@performance` - Latency, throughput, memory usage benchmarks
- `@stability` - Long-running operations, error recovery, resource cleanup

## Configuration Hierarchy

Configuration is loaded from (in order of precedence):
1. Command-line arguments to `serena-mcp-server`
2. Project-specific `.serena/project.yml`
3. User config `~/.serena/serena_config.yml`
4. Active modes and contexts

## Key Implementation Notes

- **Symbol-based editing** - Uses LSP for precise code manipulation
- **Caching strategy** - Reduces language server overhead
- **Error recovery** - Automatic language server restart on crashes
- **Multi-language support** - 19 languages with LSP integration (including Vue)
- **MCP protocol** - Exposes tools to AI agents via Model Context Protocol
- **Async operation** - Non-blocking language server interactions

## Working with the Codebase

- Project uses Python 3.11 with `uv` for dependency management
- Strict typing with mypy, formatted with black + ruff
- Language servers run as separate processes with LSP communication
- Memory system enables persistent project knowledge
- Context/mode system allows workflow customization