# Serena Project Guide

## Project Overview

**Serena** is a powerful, open-source coding agent toolkit designed to empower LLMs with IDE-like capabilities. Unlike file-based agents that rely on grep and regex, Serena uses the **Language Server Protocol (LSP)** to understand code semantically. This allows for precise symbol navigation, retrieval, and editing across over 30 programming languages.

The project is a Python-based application that exposes its capabilities primarily through the **Model Context Protocol (MCP)**, making it compatible with clients like Claude Desktop, Gemini CLI, and various IDEs.

## Technical Architecture

*   **Language**: Python 3.11+
*   **Dependency Manager**: `uv`
*   **Build System**: `hatchling`
*   **Core Components**:
    *   **SerenaAgent** (`src/serena/agent.py`): The central orchestrator.
    *   **SolidLSP** (`src/solidlsp/`): A robust wrapper around LSP servers.
    *   **Tools** (`src/serena/tools/`): The suite of tools exposed to agents (File, Symbol, Memory, etc.).
    *   **MCP Server**: The interface allowing external agents to use Serena's tools.

## Development Environment Setup

This project uses `uv` for strict dependency management and `poethepoet` for task automation.

### Prerequisites
*   Python 3.11 or higher
*   `uv` (Universal Python Package Installer)

### Installation
1.  **Clone the repository.**
2.  **Sync dependencies**:
    ```bash
    uv sync --all-extras
    ```

## Key Development Commands

All commands are run via `uv run poe` to ensure the correct environment and tools are used.

| Task | Command | Description |
| :--- | :--- | :--- |
| **Format** | `uv run poe format` | Formats code using `black` and `ruff`. **Required before committing.** |
| **Lint** | `uv run poe lint` | Checks code style and quality without modifying files. |
| **Type Check** | `uv run poe type-check` | Runs `mypy` static type analysis. **Required.** |
| **Test (Fast)**| `uv run poe test` | Runs the default test suite (skips network-heavy tests). |
| **Test (Full)**| `uv run poe test-full` | Runs the complete test suite. |
| **Build Core** | `uv run poe build-core` | Builds the Rust extensions (requires Rust toolchain). |

### Running Tests for Specific Languages
Serena's tests are marked by language. To run tests for a specific language support:
```bash
uv run poe test -m "python"
uv run poe test -m "go"
uv run poe test -m "rust"
# Combine markers
uv run poe test -m "python or typescript"
```

## Running the Application

### Start the MCP Server
To run Serena as an MCP server (e.g., for testing with an MCP inspector or connecting a local client):
```bash
uv run serena-mcp-server
```

### Run Tools Directly (No LLM)
For debugging tool logic without an agent:
```bash
uv run python scripts/demo_run_tools.py
```

## Contribution Guidelines

1.  **Strict Formatting**: The CI pipeline enforces `black` and `ruff` formatting. Always run `uv run poe format` before pushing.
2.  **Type Safety**: `mypy` checks are strict. Ensure all new code is properly typed.
3.  **Adding Languages**:
    *   Add the Language Server implementation in `src/solidlsp/language_servers/`.
    *   Register it in `src/solidlsp/ls_config.py`.
    *   Add a test repo in `test/resources/repos/<lang>/` and tests in `test/solidlsp/<lang>/`.
4.  **Adding Tools**: Inherit from `serena.agent.Tool` and register the new tool in the appropriate registry.

## Project Structure

*   `src/serena`: Main application logic (Agent, Tools, Config).
*   `src/solidlsp`: LSP integration layer.
*   `serena_core`: Rust-based core extensions.
*   `test`: Comprehensive test suite (unit and integration).
*   `resources`: Static resources (logos, etc.).
*   `docs`: Sphinx documentation.
*   `.serena`: Default configuration and memory storage.
