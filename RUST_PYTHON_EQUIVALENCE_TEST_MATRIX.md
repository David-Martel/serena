# Rust vs Python Tool Equivalence Test Matrix

**Generated**: 2025-12-25
**Purpose**: Test matrix for validating Rust tool behavior matches Python implementation

---

## Test Categories

1. [Parameter Validation](#parameter-validation)
2. [Return Value Format](#return-value-format)
3. [Error Message Format](#error-message-format)
4. [Edge Cases](#edge-cases)
5. [Security Validation](#security-validation)
6. [Performance Benchmarks](#performance-benchmarks)

---

## Parameter Validation

### Test Matrix Template
```
Tool: <tool_name>
Python Class: <class_name>
Rust Struct: <struct_name>

| Parameter | Python Type | Rust Type | Required? | Default | Notes |
|-----------|-------------|-----------|-----------|---------|-------|
| ...       | ...         | ...       | ...       | ...     | ...   |
```

### File Tools

#### read_file
Python Class: `ReadFileTool`
Rust Struct: `ReadFileTool`

| Parameter | Python Type | Rust Type | Required? | Default | Notes |
|-----------|-------------|-----------|-----------|---------|-------|
| relative_path | str | String | Yes | - | ✓ Match |
| start_line | int | Option<usize> | No | None | ⚠️ Verify: Python 0 or 1 based? |
| end_line | int | Option<usize> | No | None | ⚠️ Verify: Python inclusive? |
| max_answer_chars | int | i32 | No | -1 | ✓ Match |

**Test Cases**:
```python
# TC1: Basic read
read_file(relative_path="test.txt")

# TC2: Line range
read_file(relative_path="test.txt", start_line=0, end_line=10)

# TC3: Character limit
read_file(relative_path="test.txt", max_answer_chars=1000)

# TC4: Combined
read_file(relative_path="test.txt", start_line=5, end_line=15, max_answer_chars=500)

# TC5: Edge - Empty file
read_file(relative_path="empty.txt")

# TC6: Edge - start_line beyond EOF
read_file(relative_path="small.txt", start_line=1000)
```

**Expected Output Format**:
```json
{
  "path": "test.txt",
  "content": "file contents...",
  "total_lines": 100,
  "lines_read": 11,
  "truncated": false
}
```

---

#### create_text_file
Python Class: `CreateTextFileTool`
Rust Struct: `CreateTextFileTool`

| Parameter | Python Type | Rust Type | Required? | Default | Notes |
|-----------|-------------|-----------|-----------|---------|-------|
| relative_path | str | String | Yes | - | ✓ Match |
| content | str | String | Yes | - | ✓ Match |

**Test Cases**:
```python
# TC1: Create new file
create_text_file(relative_path="new.txt", content="Hello")

# TC2: Overwrite existing
create_text_file(relative_path="existing.txt", content="New content")

# TC3: Create with parent dirs
create_text_file(relative_path="deep/path/file.txt", content="Test")

# TC4: Empty content
create_text_file(relative_path="empty.txt", content="")

# TC5: Multiline content
create_text_file(relative_path="multi.txt", content="Line1\nLine2\nLine3")

# TC6: UTF-8 content
create_text_file(relative_path="utf8.txt", content="Hello 世界 🌍")
```

**Expected Output**:
```json
{
  "path": "new.txt",
  "bytes_written": 5,
  "created": true
}
```

---

#### list_directory
Python Class: `ListDirectoryTool`
Rust Struct: `ListDirectoryTool`

| Parameter | Python Type | Rust Type | Required? | Default | Notes |
|-----------|-------------|-----------|-----------|---------|-------|
| relative_path | str | String | Yes | - | ✓ Match |
| recursive | bool | bool | No | false | ✓ Match |
| max_answer_chars | int | i32 | No | -1 | ✓ Match |

**Test Cases**:
```python
# TC1: Non-recursive
list_directory(relative_path="src")

# TC2: Recursive
list_directory(relative_path="src", recursive=True)

# TC3: Root directory
list_directory(relative_path=".")

# TC4: With character limit
list_directory(relative_path="large_dir", max_answer_chars=5000)

# TC5: Empty directory
list_directory(relative_path="empty_dir")
```

**Expected Output**:
```json
{
  "path": "src",
  "entries": [
    {
      "name": "main.rs",
      "path": "src/main.rs",
      "is_file": true,
      "is_dir": false,
      "size": 1234
    }
  ],
  "total_files": 10,
  "total_dirs": 3
}
```

---

#### find_file
Python Class: `FindFileTool`
Rust Struct: `FindFileTool`

| Parameter | Python Type | Rust Type | Required? | Default | Notes |
|-----------|-------------|-----------|-----------|---------|-------|
| file_mask | str | String | Yes | - | ✓ Match |
| relative_path | str | String | No | "." | ✓ Match |
| max_results | int | usize | No | 1000 | ✓ Match |

**Test Cases**:
```python
# TC1: Simple glob
find_file(file_mask="*.rs")

# TC2: Recursive glob
find_file(file_mask="**/*.py")

# TC3: Multiple extensions
find_file(file_mask="**/*.{rs,toml}")

# TC4: In subdirectory
find_file(file_mask="*.txt", relative_path="docs")

# TC5: With max results
find_file(file_mask="**/*.rs", max_results=10)

# TC6: No matches
find_file(file_mask="*.nonexistent")
```

**Expected Output**:
```json
{
  "files": ["src/main.rs", "src/lib.rs"],
  "total_found": 2,
  "truncated": false
}
```

---

#### search_files
Python Class: `SearchFilesTool`
Rust Struct: `SearchFilesTool`

| Parameter | Python Type | Rust Type | Required? | Default | Notes |
|-----------|-------------|-----------|-----------|---------|-------|
| pattern | str | String | Yes | - | ✓ Match |
| path | str | Option<String> | No | None | ✓ Match |
| include_glob | str | Option<String> | No | None | ✓ Match |
| exclude_glob | str | Option<String> | No | None | ✓ Match |
| case_insensitive | bool | bool | No | false | ✓ Match |
| max_results | int | usize | No | 1000 | ✓ Match |
| context_lines | int | usize | No | 0 | ⚠️ Verify Python has this |

**Test Cases**:
```python
# TC1: Simple pattern
search_files(pattern="TODO")

# TC2: Regex pattern
search_files(pattern=r"fn \w+\(")

# TC3: With file filter
search_files(pattern="import", include_glob="*.py")

# TC4: Case insensitive
search_files(pattern="ERROR", case_insensitive=True)

# TC5: With context
search_files(pattern="panic!", context_lines=2)

# TC6: In subdirectory
search_files(pattern="test", path="src/tests")

# TC7: Exclude files
search_files(pattern="debug", exclude_glob="*.min.js")
```

**Expected Output**:
```json
{
  "matches": [
    {
      "path": "src/main.rs",
      "line_number": 42,
      "line": "    // TODO: implement this",
      "context_before": ["fn main() {", "    let x = 5;"],
      "context_after": ["    println!(\"Done\");", "}"]
    }
  ],
  "total_matches": 1,
  "truncated": false
}
```

---

#### replace_content
Python Class: `ReplaceContentTool` (?)
Rust Struct: `ReplaceContentTool`

| Parameter | Python Type | Rust Type | Required? | Default | Notes |
|-----------|-------------|-----------|-----------|---------|-------|
| relative_path | str | String | Yes | - | ✓ |
| needle | str | String | Yes | - | ✓ |
| repl | str | String | Yes | - | ✓ |
| mode | str | String | Yes | - | "literal" or "regex" |
| allow_multiple_occurrences | bool | bool | No | false | ⚠️ Verify Python has this |

**Test Cases**:
```python
# TC1: Literal replacement
replace_content(
    relative_path="test.txt",
    needle="old text",
    repl="new text",
    mode="literal"
)

# TC2: Regex replacement
replace_content(
    relative_path="test.txt",
    needle=r"v\d+\.\d+\.\d+",
    repl="v2.0.0",
    mode="regex"
)

# TC3: Multiple occurrences allowed
replace_content(
    relative_path="test.txt",
    needle="TODO",
    repl="DONE",
    mode="literal",
    allow_multiple_occurrences=True
)

# TC4: Pattern not found (should error)
replace_content(
    relative_path="test.txt",
    needle="nonexistent",
    repl="new",
    mode="literal"
)

# TC5: Multiple matches without allow flag (should error)
replace_content(
    relative_path="test.txt",
    needle="common",
    repl="rare",
    mode="literal",
    allow_multiple_occurrences=False
)
```

**Expected Output**:
```json
{
  "path": "test.txt",
  "replacements_made": 1,
  "original_size": 100,
  "new_size": 102
}
```

---

### Editor Tools

#### delete_lines
Rust Struct: `DeleteLinesTool`
⚠️ **Verify Python equivalent exists**

| Parameter | Rust Type | Required? | Notes |
|-----------|-----------|-----------|-------|
| relative_path | String | Yes | - |
| start_line | usize | Yes | 1-based |
| end_line | usize | Yes | 1-based, inclusive |

**Test Cases**:
```python
# TC1: Delete single line
delete_lines(relative_path="test.txt", start_line=5, end_line=5)

# TC2: Delete range
delete_lines(relative_path="test.txt", start_line=10, end_line=20)

# TC3: Delete first line
delete_lines(relative_path="test.txt", start_line=1, end_line=1)

# TC4: Delete last line
delete_lines(relative_path="test.txt", start_line=100, end_line=100)

# TC5: Invalid range (start > end)
delete_lines(relative_path="test.txt", start_line=20, end_line=10)  # Error

# TC6: Line 0 (should error - 1-based)
delete_lines(relative_path="test.txt", start_line=0, end_line=5)  # Error
```

---

#### insert_at_line
Rust Struct: `InsertAtLineTool`
⚠️ **Verify Python equivalent exists**

**Test Cases**:
```python
# TC1: Insert at beginning
insert_at_line(relative_path="test.txt", line=1, content="First line")

# TC2: Insert in middle
insert_at_line(relative_path="test.txt", line=10, content="New line")

# TC3: Insert at end
insert_at_line(relative_path="test.txt", line=100, content="Last line")

# TC4: Insert multiline
insert_at_line(relative_path="test.txt", line=5, content="Line1\nLine2\nLine3")

# TC5: Line 0 (should error)
insert_at_line(relative_path="test.txt", line=0, content="Bad")  # Error
```

---

#### replace_lines
Rust Struct: `ReplaceLinesTool`
⚠️ **Verify Python equivalent exists**

**Test Cases**:
```python
# TC1: Replace single line
replace_lines(relative_path="test.txt", start_line=5, end_line=5, content="New line")

# TC2: Replace range with single line
replace_lines(relative_path="test.txt", start_line=10, end_line=15, content="One line")

# TC3: Replace single line with multiple
replace_lines(relative_path="test.txt", start_line=5, end_line=5, content="Line1\nLine2\nLine3")

# TC4: Replace range with range
replace_lines(relative_path="test.txt", start_line=10, end_line=12, content="A\nB\nC")
```

---

### Symbol Tools

#### get_symbols_overview
Python Class: `GetSymbolsOverviewTool`
Rust Struct: `GetSymbolsOverviewTool`

| Parameter | Python Type | Rust Type | Required? | Default | Notes |
|-----------|-------------|-----------|-----------|---------|-------|
| relative_path | str | String | Yes | - | ✓ |
| depth | int | usize | No | 0 | ✓ |
| max_answer_chars | int | i32 | No | -1 | ✓ |

**Test Cases**:
```python
# TC1: Top-level only
get_symbols_overview(relative_path="main.rs")

# TC2: With depth
get_symbols_overview(relative_path="main.rs", depth=2)

# TC3: With char limit
get_symbols_overview(relative_path="main.rs", max_answer_chars=5000)

# TC4: Empty file
get_symbols_overview(relative_path="empty.rs")

# TC5: Non-code file
get_symbols_overview(relative_path="README.md")
```

**Expected Output Format** (string):
```
Function main [1:0-10:0]
  Variable x [2:4-2:5]
Class MyClass [12:0-25:0]
  Method __init__ [13:4-15:8]
```

---

#### find_symbol
Python Class: `FindSymbolTool`
Rust Struct: `FindSymbolTool`

| Parameter | Python Type | Rust Type | Required? | Default | Notes |
|-----------|-------------|-----------|-----------|---------|-------|
| name_path_pattern | str | String | Yes | - | ✓ |
| relative_path | str | Option<String> | No | None | ✓ |
| depth | int | usize | No | 0 | ✓ |
| include_body | bool | bool | No | false | ✓ |
| substring_matching | bool | bool | No | false | ✓ |
| max_answer_chars | int | i32 | No | -1 | ✓ |

**Test Cases**:
```python
# TC1: Exact match
find_symbol(name_path_pattern="main")

# TC2: Path match
find_symbol(name_path_pattern="MyClass/method")

# TC3: Absolute path
find_symbol(name_path_pattern="/MyClass/method")

# TC4: Substring matching
find_symbol(name_path_pattern="test", substring_matching=True)

# TC5: With body
find_symbol(name_path_pattern="main", include_body=True)

# TC6: In specific file
find_symbol(name_path_pattern="method", relative_path="src/lib.rs")
```

---

#### rename_symbol
Python Class: `RenameSymbolTool`
Rust Struct: `RenameSymbolTool`

| Parameter | Python Type | Rust Type | Required? | Default | Notes |
|-----------|-------------|-----------|-----------|---------|-------|
| name_path | str | String | Yes | - | ✓ |
| relative_path | str | String | Yes | - | ✓ |
| new_name | str | String | Yes | - | ✓ |

**Test Cases**:
```python
# TC1: Simple rename
rename_symbol(
    name_path="old_function",
    relative_path="src/lib.rs",
    new_name="new_function"
)

# TC2: Class method rename
rename_symbol(
    name_path="MyClass/method",
    relative_path="src/lib.rs",
    new_name="renamed_method"
)

# TC3: Variable rename
rename_symbol(
    name_path="old_var",
    relative_path="src/main.rs",
    new_name="new_var"
)
```

**Expected Output**:
```json
{
  "message": "Renamed 'old_function' to 'new_function' in 5 files",
  "files_changed": 5
}
```

---

### Memory Tools

All memory tools appear to match between Python and Rust.

**Test Priority**: Medium (functionality straightforward)

---

### Config Tools

All config tools appear to match between Python and Rust.

**Test Priority**: Medium (state management critical but less complex)

---

### Command Tools

#### execute_shell_command
Python Class: `ExecuteShellCommandTool` (?)
Rust Struct: `ExecuteShellCommandTool`

| Parameter | Python Type | Rust Type | Required? | Default | Notes |
|-----------|-------------|-----------|-----------|---------|-------|
| command | str | String | Yes | - | ✓ |
| cwd | str | Option<String> | No | None | ✓ |
| capture_stderr | bool | bool | No | true | ✓ |
| max_answer_chars | int | i32 | No | -1 | ✓ |
| timeout_secs | int | u64 | No | 60 | ⚠️ Verify Python default |

**Security Test Cases** (CRITICAL):
```python
# TC1: Safe command
execute_shell_command(command="echo hello")

# TC2: Dangerous - rm -rf /
execute_shell_command(command="rm -rf /")  # Should ERROR

# TC3: Dangerous - fork bomb
execute_shell_command(command=":(){:|:&};:")  # Should ERROR

# TC4: Custom working dir
execute_shell_command(command="ls", cwd="src")

# TC5: Timeout
execute_shell_command(command="sleep 100", timeout_secs=1)  # Should timeout

# TC6: Stderr capture
execute_shell_command(command="ls nonexistent", capture_stderr=True)

# TC7: Platform-specific
# Windows: execute_shell_command(command="dir")
# Unix: execute_shell_command(command="ls")
```

**Expected Output**:
```json
{
  "stdout": "hello\n",
  "stderr": "",
  "exit_code": 0,
  "working_directory": "/project/root",
  "command": "echo hello"
}
```

---

## Error Message Format

### File Not Found

**Rust**:
```json
{
  "status": "error",
  "error": "File not found: test.txt"
}
```

**Python** (⚠️ Verify):
```python
"Error: File not found: test.txt"
```

**Action**: Ensure format matches exactly.

---

### Invalid Parameters

**Rust**:
```json
{
  "status": "error",
  "error": "Invalid parameter: start_line (5) must be <= end_line (3)"
}
```

**Python** (⚠️ Verify):
```python
"Error: Invalid parameter: ..."
```

---

### Permission Denied

**Rust**:
```json
{
  "status": "error",
  "error": "Permission denied: Path '/etc/passwd' is outside project root"
}
```

**Python** (⚠️ Verify):
Similar format needed.

---

## Edge Cases Test Matrix

| Tool | Edge Case | Expected Behavior |
|------|-----------|-------------------|
| read_file | Empty file | Return empty content, total_lines=0 |
| read_file | Line range beyond EOF | Return what's available, don't error |
| read_file | start_line > total_lines | Error or empty? ⚠️ |
| create_text_file | Path with parent dirs | Create parent dirs |
| create_text_file | Path traversal (..) | Error: outside project |
| list_directory | Empty directory | Return empty entries list |
| list_directory | Symlink loop | Handle gracefully |
| find_file | No matches | Return empty files list |
| search_files | Binary file | Skip or error? ⚠️ |
| search_files | Very large file | Respect max_results |
| replace_content | Pattern not found | Error |
| replace_content | Multiple matches, allow=false | Error |
| delete_lines | Line 0 | Error (1-based) |
| insert_at_line | Line beyond EOF | Append? ⚠️ |
| get_symbols_overview | No LSP available | Error with helpful message |
| find_symbol | Symbol not found | Empty matches |
| rename_symbol | Symbol in use | LSP handles, apply all |

---

## Security Validation Matrix

| Tool | Attack Vector | Expected Behavior |
|------|---------------|-------------------|
| read_file | ../../../etc/passwd | Error: outside project |
| read_file | Absolute path outside | Error: outside project |
| create_text_file | Path traversal | Error before creation |
| list_directory | Symlink outside project | Follow or error? ⚠️ |
| execute_shell_command | rm -rf / | Error: dangerous pattern |
| execute_shell_command | Fork bomb | Error: dangerous pattern |
| execute_shell_command | Command injection | Use proper escaping |
| replace_content | Regex catastrophic backtracking | Timeout? ⚠️ |

---

## Performance Benchmarks

### search_files Performance

**Test**: Search for pattern in large codebase (e.g., Linux kernel)

| Implementation | Time | Notes |
|----------------|------|-------|
| Python | ⚠️ Measure | Sequential or parallel? |
| Rust | ⚠️ Measure | Rayon parallel |

**Expected**: Rust should be 5-10x faster for parallel search.

---

### file operations

**Test**: Read/write 10,000 small files

| Operation | Python Time | Rust Time | Notes |
|-----------|-------------|-----------|-------|
| read_file | ⚠️ | ⚠️ | Sequential |
| create_text_file | ⚠️ | ⚠️ | Sequential |

**Expected**: Similar performance (both IO-bound).

---

### Symbol Operations

**Test**: get_symbols_overview on large file

| Implementation | Time | Notes |
|----------------|------|-------|
| Python | ⚠️ | With LSP caching |
| Rust | ⚠️ | With LSP caching |

**Expected**: Similar (LSP is bottleneck).

---

## Character Encoding Tests

| Test | Input | Expected Output |
|------|-------|-----------------|
| UTF-8 | "Hello 世界" | Same |
| Emoji | "🌍🚀💻" | Same |
| Newlines | "Line1\nLine2\r\nLine3" | Preserve or normalize? ⚠️ |
| BOM | UTF-8 BOM file | Handle correctly |

---

## Test Execution Plan

### Phase 1: Critical Security (Week 1)
- [ ] Path traversal tests (all file tools)
- [ ] Dangerous command blocking
- [ ] Project boundary enforcement

### Phase 2: Core Functionality (Week 2)
- [ ] Parameter validation (all tools)
- [ ] Return value format matching
- [ ] Error message format matching

### Phase 3: Edge Cases (Week 3)
- [ ] Empty files
- [ ] Line number boundaries
- [ ] Character encoding
- [ ] Binary files

### Phase 4: Performance (Week 4)
- [ ] Benchmark search_files
- [ ] Benchmark symbol operations
- [ ] Memory usage comparison

### Phase 5: Integration (Week 5)
- [ ] Test with real MCP clients
- [ ] Cross-platform validation (Windows, macOS, Linux)
- [ ] LSP integration with all languages

---

## Automated Test Framework

Recommended structure:
```
tests/
  equivalence/
    test_file_tools.py     # Compare Python vs Rust
    test_symbol_tools.py
    test_memory_tools.py
    test_config_tools.py
    test_command_tools.py
  security/
    test_path_traversal.py
    test_command_injection.py
  performance/
    benchmark_search.py
    benchmark_symbols.py
```

Each test should:
1. Call Python implementation
2. Call Rust implementation (via MCP or direct)
3. Compare outputs (JSON deep compare)
4. Assert error messages match
5. Verify performance within acceptable bounds

---

## Critical Verification Items

### ⚠️ Items Needing Python Verification

1. **Line number convention**:
   - `read_file`: Is Python 0-based or 1-based?
   - Editor tools: Does Python have equivalent?

2. **Parameter defaults**:
   - `execute_shell_command`: Python timeout default?
   - `search_files`: Python has context_lines?

3. **Error format**:
   - Does Python return JSON or string errors?
   - What's the exact error message format?

4. **Edge case behavior**:
   - `insert_at_line`: Line beyond EOF behavior?
   - `search_files`: Binary file handling?
   - `list_directory`: Symlink handling?

5. **Missing tools**:
   - Are there Python tools not in Rust inventory?
   - JetBrains tools documented separately?

---

## Test Data Requirements

Create test repository with:
- Various file sizes (empty, small, large)
- Different languages (Python, Rust, JavaScript, etc.)
- Edge cases (special chars in filenames, deeply nested dirs)
- UTF-8 and other encodings
- Binary files
- Symlinks
- Files with no newline at EOF

---

End of Test Matrix

**Next Steps**:
1. Verify Python tool parameters
2. Set up test environment
3. Implement automated comparison tests
4. Execute test plan phases
5. Document findings and differences
