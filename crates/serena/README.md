# Serena - Main Binary

This is the main binary crate for Serena AI coding assistant. It ties together all the component crates and provides the command-line interface.

## Architecture

The binary consists of:

### Main Entry Point (`src/main.rs`)
- CLI argument parsing using `clap`
- Tracing/logging setup
- Application initialization and lifecycle management
- Transport selection (stdio, HTTP, SSE)

### Application Manager (`src/app.rs`)
- Initializes and coordinates all components:
  - **Tool Registry** - Manages available tools
  - **LSP Manager** - Handles language server instances
  - **MCP Server** - Exposes tools via Model Context Protocol
  - **Configuration** - Loads and manages settings
  - **Project Config** - Project-specific settings and language detection

## Usage

### Basic Usage (stdio transport)
```bash
serena
```

### With Project
```bash
serena --project /path/to/project
```

### With Custom Config
```bash
serena --config /path/to/config.yml
```

### HTTP Transport
```bash
serena --transport http --port 3000
```

### Verbose Logging
```bash
serena --verbose --log-level debug
```

## Command-Line Options

- `-t, --transport <TRANSPORT>` - Transport mode: stdio, http, or sse (default: stdio)
- `-p, --port <PORT>` - HTTP server port for http/sse transports (default: 3000)
- `-P, --project <PROJECT>` - Project directory to activate
- `-c, --config <CONFIG>` - Configuration file path
- `-m, --mode <MODE>` - Operating mode (planning, editing, interactive, one-shot)
- `--context <CONTEXT>` - Context to use (desktop-app, agent, ide-assistant)
- `--log-level <LOG_LEVEL>` - Log level (default: info)
- `-v, --verbose` - Enable verbose logging
- `-h, --help` - Print help
- `-V, --version` - Print version

## Component Integration

The binary integrates the following crates:
- **serena-core** - Core types, traits, and tool registry
- **serena-config** - Configuration management
- **serena-tools** - Tool implementations
- **serena-mcp** - MCP server and protocol
- **serena-lsp** - LSP client management

## TODOs

- [ ] Implement HTTP transport
- [ ] Implement SSE transport  
- [ ] Add mode switching functionality
- [ ] Add context switching functionality
- [ ] Implement signal handling for graceful shutdown
- [ ] Add metrics and health check endpoints
- [ ] Integrate memory/knowledge persistence
- [ ] Add web dashboard support

## Building

```bash
# Development build
cargo build -p serena

# Release build
cargo build -p serena --release

# Check compilation
cargo check -p serena
```

## Binary Location

After building, the binary is located at:
- Debug: `target/debug/serena` (or `serena.exe` on Windows)
- Release: `target/release/serena` (or `serena.exe` on Windows)
