# Serena Core Integration Tests

This directory contains comprehensive integration tests for the Serena MCP server.

## Test Structure

- **mod.rs** - Module declaration for all integration tests
- **file_tools_tests.rs** - File system operation tests (read, write, search, etc.)
- **memory_tools_tests.rs** - Memory persistence system tests
- **symbol_tools_tests.rs** - Symbol navigation and LSP integration tests
- **mcp_protocol_tests.rs** - MCP protocol compliance tests

## Running Tests

### Run All Tests
```bash
cargo test --test integration_tests
```

### Run Specific Test Module
```bash
cargo test --test integration_tests file_tools_tests
cargo test --test integration_tests memory_tools_tests
cargo test --test integration_tests mcp_protocol_tests
```

### Run LSP-Dependent Tests
Many symbol tests require a running LSP server and are marked with `#[ignore]`:
```bash
cargo test --test integration_tests symbol_tools_tests -- --ignored
```

## Test Coverage

### File Tools Tests
- Basic file read/write operations
- Line-limited reading
- Directory listing (recursive and non-recursive)
- Glob pattern file finding
- Content replacement (literal and regex)
- Pattern searching with context lines

### Memory Tools Tests
- Complete CRUD workflow
- Memory listing
- Content editing (literal and regex)
- Error handling for non-existent memories
- Large content handling

### Symbol Tools Tests
- Symbol graph initialization
- Document symbol insertion
- Symbol search (exact and substring)
- LSP integration (requires language servers)

### MCP Protocol Tests
- Server metadata verification
- Tool listing
- Tool invocation with various parameters
- Error handling for invalid tools/arguments
- Sequential tool call workflows

## Test Utilities

All test suites use `tempfile::TempDir` for isolated test environments, ensuring:
- No cross-test interference
- Automatic cleanup
- Consistent test conditions

## Known Issues

- Some tests have deprecated field warnings for `lsp_types::DocumentSymbol::deprecated`
- LSP-dependent tests require language servers to be installed
- Symbol tools tests need `max_answer_chars` field added

## Contributing

When adding new tests:
1. Use helper functions for setup (`setup_file_service`, `setup_memory_service`, etc.)
2. Mark LSP-dependent tests with `#[ignore]`
3. Use descriptive test names following the pattern `test_<feature>_<scenario>`
4. Include both success and error cases
5. Clean up resources using `TempDir` pattern
