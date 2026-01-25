# serena-config

Configuration management for the Serena coding agent toolkit.

## Overview

This crate provides comprehensive configuration structures and loading logic for Serena, including:

- **Project Configuration** - Per-project settings, language support, and tool management
- **Language Support** - 40+ programming languages with file extension detection
- **Contexts** - Environment-specific tool sets (desktop-app, ide-assistant, agent)
- **Modes** - Operational patterns (planning, editing, interactive, one-shot)
- **Configuration Loading** - Multi-location config file discovery and merging

## Features

- Cross-platform configuration file discovery
- YAML and JSON configuration formats
- Hierarchical configuration merging
- Project-specific tool filtering
- Language detection from project files
- Built-in default contexts and modes
- Comprehensive validation

## Usage

### Basic Configuration Loading

```rust
use serena_config::SerenaConfig;

// Load from default locations
let config = SerenaConfig::load()?;

// Load from specific file
let config = SerenaConfig::load_from("path/to/config.yml")?;
```

### Working with Projects

```rust
use serena_config::ProjectConfig;
use std::path::PathBuf;

// Create a new project configuration
let mut project = ProjectConfig::new("my-project", PathBuf::from("/path/to/project"));

// Detect languages automatically
project.detect_languages()?;

// Check if a tool is enabled
if project.is_tool_enabled("execute_shell_command") {
    // Execute command
}

// Check if a file should be ignored
if project.should_ignore("node_modules/package.json") {
    // Skip processing
}
```

### Using Contexts and Modes

```rust
use serena_config::context_mode::{Context, Mode};

// Get default contexts
let contexts = Context::defaults();

// Create custom context
let custom_context = Context::new("custom", "My custom context")
    .with_tool("read_file")
    .with_tool("write_file")
    .as_default();

// Get default modes
let modes = Mode::defaults();

// Create custom mode
let custom_mode = Mode::new("debug", "Debug mode")
    .with_behavior("verbose")
    .with_behavior("step-by-step");
```

### Language Detection

```rust
use serena_config::Language;

// Detect language from file extension
if let Some(lang) = Language::from_extension("rs") {
    println!("Language: {}", lang.display_name()); // "Rust"
}

// Get file extensions for a language
let extensions = Language::Python.extensions();
// ["py", "pyw", "pyi"]
```

## Supported Languages

The crate supports 40+ programming languages including:

- **Systems**: Rust, C, C++, Zig, Nim
- **Web**: TypeScript, JavaScript, Vue, HTML, CSS, SCSS
- **Backend**: Python, Go, Java, C#, Ruby, PHP, Elixir
- **Functional**: Haskell, Erlang, OCaml, F#, Elm, PureScript
- **Data**: R, Julia
- **Mobile**: Swift, Kotlin, Dart, Objective-C
- **Config**: YAML, TOML, JSON, Terraform
- **Scripting**: Bash, PowerShell, Perl, Lua, Groovy
- **Other**: Clojure, Scala, Crystal, ReasonML, Solidity

## Configuration File Format

### User Configuration (`~/.serena/serena_config.yml`)

```yaml
log_level: info
default_context: desktop-app
default_modes:
  - interactive
  - editing

web_dashboard: true
web_dashboard_port: 3000

tool_timeout: 30

projects:
  - name: my-project
    root: /path/to/project
    languages:
      - python
      - rust
    excluded_tools:
      - execute_shell_command
    ignore_patterns:
      - "*.pyc"
      - "node_modules/"
```

### Project Configuration (`.serena/project.yml`)

```yaml
name: my-project
root: .
languages:
  - python
  - typescript
encoding: utf-8
read_only: false

included_tools: []
excluded_tools:
  - execute_shell_command

ignore_patterns:
  - node_modules/
  - .git/
  - __pycache__/
  - "*.pyc"

max_file_size: 10485760  # 10 MB
enable_indexing: true
enable_memory: true

language_server_config:
  python:
    interpreter: /usr/bin/python3
  rust:
    clippy_preference: on
```

## API Documentation

### Core Types

- `SerenaConfig` - Main configuration structure
- `ProjectConfig` - Project-specific configuration
- `Language` - Programming language enumeration
- `Context` - Environment and tool set definition
- `Mode` - Operational behavior definition
- `ConfigLoader` - Configuration file loading and discovery

### Error Handling

All configuration operations return `Result<T, ConfigError>` where `ConfigError` provides detailed error information for:
- IO errors
- YAML/JSON parsing errors
- Validation errors
- Missing configuration files

## Testing

The crate includes comprehensive unit tests:

```bash
cargo test --package serena-config
```

All 23 tests pass successfully, covering:
- Configuration loading and saving
- Project management (add, update, remove)
- Context and mode creation
- Language detection
- Tool filtering
- File pattern matching
- Configuration merging
- Validation

## License

MIT

## Contributing

See the main Serena repository for contribution guidelines.
