# Serena Rust Tool Implementation - Complete Inventory

**Generated**: 2025-12-25
**Purpose**: Complete documentation of all Rust tool implementations for equivalence testing against Python

---

## Table of Contents

1. [File Tools](#file-tools) (serena-tools/src/file/)
2. [Editor Tools](#editor-tools) (serena-tools/src/editor/)
3. [Symbol Tools](#symbol-tools) (serena-symbol/src/tools.rs)
4. [Memory Tools](#memory-tools) (serena-memory/src/tools.rs)
5. [Config Tools](#config-tools) (serena-config/src/tools.rs)
6. [Command Tools](#command-tools) (serena-commands/src/tools.rs)
7. [LSP Tools](#lsp-tools) (serena-lsp/src/tools.rs)
8. [Workflow Tools](#workflow-tools) (serena-tools/src/workflow/)
9. [Python vs Rust Gaps](#python-vs-rust-gaps)
10. [Parameter Schema Reference](#parameter-schema-reference)

---

## File Tools
**Location**: `crates/serena-tools/src/file/*.rs`

### 1. read_file
**File**: `read.rs`
**Struct**: `ReadFileTool`

**Description**: Reads a file from the local filesystem with optional line slicing and character limits.

**Parameters**:
```json
{
  "relative_path": {
    "type": "string",
    "required": true,
    "description": "The relative path to the file to read"
  },
  "start_line": {
    "type": "integer",
    "required": false,
    "description": "The 0-based index of the first line to be retrieved"
  },
  "end_line": {
    "type": "integer",
    "required": false,
    "description": "The 0-based index of the last line to be retrieved, inclusive"
  },
  "max_answer_chars": {
    "type": "integer",
    "required": false,
    "default": -1,
    "description": "Maximum characters to return. -1 for unlimited"
  }
}
```

**Returns**:
```rust
struct ReadFileOutput {
    path: String,
    content: String,
    total_lines: usize,
    lines_read: usize,
    truncated: bool,
}
```

**Security Features**:
- Path canonicalization
- Project root boundary enforcement
- Prevents directory traversal attacks

**Tags**: `file`, `read`

---

### 2. create_text_file
**File**: `write.rs`
**Struct**: `CreateTextFileTool`

**Description**: Creates or overwrites a text file. Creates parent directories if needed.

**Parameters**:
```json
{
  "relative_path": {
    "type": "string",
    "required": true,
    "description": "The relative path to the file to create or overwrite"
  },
  "content": {
    "type": "string",
    "required": true,
    "description": "The content to write to the file"
  }
}
```

**Returns**:
```rust
struct CreateTextFileOutput {
    path: String,
    bytes_written: usize,
    created: bool,  // true if new file, false if overwritten
}
```

**Security Features**:
- Path validation before and after directory creation
- Project root boundary enforcement
- Parent directory creation with validation

**Tags**: `file`, `write`, `edit`

---

### 3. list_directory
**File**: `list.rs`
**Struct**: `ListDirectoryTool`

**Description**: Lists directory contents with recursive option. Sorts directories first, then alphabetically.

**Parameters**:
```json
{
  "relative_path": {
    "type": "string",
    "required": true,
    "description": "The relative path to the directory to list"
  },
  "recursive": {
    "type": "boolean",
    "required": false,
    "default": false,
    "description": "Whether to recursively list subdirectories"
  },
  "max_answer_chars": {
    "type": "integer",
    "required": false,
    "default": -1,
    "description": "Maximum characters to return. -1 for unlimited"
  }
}
```

**Returns**:
```rust
struct DirectoryEntry {
    name: String,
    path: String,
    is_file: bool,
    is_dir: bool,
    size: Option<u64>,
}

struct ListDirectoryOutput {
    path: String,
    entries: Vec<DirectoryEntry>,
    total_files: usize,
    total_dirs: usize,
}
```

**Implementation Details**:
- Uses `walkdir` for recursive traversal
- Follows symlinks
- Handles errors gracefully (continues on permission errors)

**Tags**: `file`, `directory`, `list`

---

### 4. find_file
**File**: `find.rs`
**Struct**: `FindFileTool`

**Description**: Finds files matching a glob pattern (e.g., `*.rs`, `**/*.py`).

**Parameters**:
```json
{
  "file_mask": {
    "type": "string",
    "required": true,
    "description": "The glob pattern to match (e.g., '*.rs', '**/*.py')"
  },
  "relative_path": {
    "type": "string",
    "required": false,
    "default": ".",
    "description": "The relative path to search in"
  },
  "max_results": {
    "type": "integer",
    "required": false,
    "default": 1000,
    "description": "Maximum number of results to return"
  }
}
```

**Returns**:
```rust
struct FindFileOutput {
    files: Vec<String>,
    total_found: usize,
    truncated: bool,
}
```

**Implementation**: Uses `glob` crate with path normalization.

**Tags**: `file`, `find`, `search`

---

### 5. search_files
**File**: `search.rs`
**Struct**: `SearchFilesTool`

**Description**: Searches files for regex patterns with .gitignore support and parallel processing.

**Parameters**:
```json
{
  "pattern": {
    "type": "string",
    "required": true,
    "description": "Regex pattern to search for"
  },
  "path": {
    "type": "string",
    "required": false,
    "description": "Optional relative path to search within"
  },
  "include_glob": {
    "type": "string",
    "required": false,
    "description": "Optional glob pattern to include files (e.g., '*.rs')"
  },
  "exclude_glob": {
    "type": "string",
    "required": false,
    "description": "Optional glob pattern to exclude files"
  },
  "case_insensitive": {
    "type": "boolean",
    "required": false,
    "default": false
  },
  "max_results": {
    "type": "integer",
    "required": false,
    "default": 1000
  },
  "context_lines": {
    "type": "integer",
    "required": false,
    "default": 0,
    "description": "Number of context lines before/after match"
  }
}
```

**Returns**:
```rust
struct FileMatch {
    path: String,
    line_number: usize,
    line: String,
    context_before: Option<Vec<String>>,
    context_after: Option<Vec<String>>,
}

struct SearchFilesOutput {
    matches: Vec<FileMatch>,
    total_matches: usize,
    truncated: bool,
}
```

**Performance Optimizations**:
- Pre-compiled glob matchers (50-100x faster)
- Parallel file processing with Rayon
- Early termination when max results reached
- Path computation optimization (once per file)

**Tags**: `file`, `search`

---

### 6. replace_content
**File**: `replace.rs`
**Struct**: `ReplaceContentTool`

**Description**: Replaces content in files using literal or regex mode. Prevents accidental multiple replacements.

**Parameters**:
```json
{
  "relative_path": {
    "type": "string",
    "required": true,
    "description": "The relative path to the file"
  },
  "needle": {
    "type": "string",
    "required": true,
    "description": "The string or regex pattern to search for"
  },
  "repl": {
    "type": "string",
    "required": true,
    "description": "The replacement string"
  },
  "mode": {
    "type": "string",
    "enum": ["literal", "regex"],
    "required": true,
    "description": "Match mode"
  },
  "allow_multiple_occurrences": {
    "type": "boolean",
    "required": false,
    "default": false,
    "description": "If true, replace all occurrences. If false, error on multiple matches"
  }
}
```

**Returns**:
```rust
struct ReplaceContentOutput {
    path: String,
    replacements_made: usize,
    original_size: usize,
    new_size: usize,
}
```

**Safety Features**:
- Errors if pattern not found
- Errors if multiple matches unless explicitly allowed
- Supports multiline regex with dot-matches-newline
- Path traversal prevention

**Tags**: `file`, `replace`, `edit`

---

## Editor Tools
**Location**: `crates/serena-tools/src/editor/mod.rs`

### 7. delete_lines
**Struct**: `DeleteLinesTool`

**Description**: Deletes a range of lines from a file (1-based, inclusive).

**Parameters**:
```json
{
  "relative_path": {
    "type": "string",
    "required": true
  },
  "start_line": {
    "type": "integer",
    "minimum": 1,
    "required": true,
    "description": "1-based start line (inclusive)"
  },
  "end_line": {
    "type": "integer",
    "minimum": 1,
    "required": true,
    "description": "1-based end line (inclusive)"
  }
}
```

**Returns**:
```rust
struct DeleteLinesOutput {
    path: String,
    lines_deleted: usize,
    new_total_lines: usize,
}
```

**Validation**:
- Line numbers must be >= 1
- start_line must be <= end_line
- start_line must not exceed file length
- Preserves trailing newline behavior

**Tags**: `file`, `edit`, `lines`

---

### 8. insert_at_line
**Struct**: `InsertAtLineTool`

**Description**: Inserts content at a specific line. Existing content shifts down.

**Parameters**:
```json
{
  "relative_path": {
    "type": "string",
    "required": true
  },
  "line": {
    "type": "integer",
    "minimum": 1,
    "required": true,
    "description": "1-based line number to insert at"
  },
  "content": {
    "type": "string",
    "required": true,
    "description": "Content to insert (can be multiple lines)"
  }
}
```

**Returns**:
```rust
struct InsertAtLineOutput {
    path: String,
    lines_inserted: usize,
    new_total_lines: usize,
}
```

**Behavior**:
- Inserting at line 1 prepends to file
- Inserting beyond end of file appends
- Multiline content supported
- Preserves trailing newline

**Tags**: `file`, `edit`, `lines`

---

### 9. replace_lines
**Struct**: `ReplaceLinesTool`

**Description**: Replaces a range of lines with new content (1-based, inclusive).

**Parameters**:
```json
{
  "relative_path": {
    "type": "string",
    "required": true
  },
  "start_line": {
    "type": "integer",
    "minimum": 1,
    "required": true
  },
  "end_line": {
    "type": "integer",
    "minimum": 1,
    "required": true
  },
  "content": {
    "type": "string",
    "required": true,
    "description": "Replacement content (can be multiple lines)"
  }
}
```

**Returns**:
```rust
struct ReplaceLinesOutput {
    path: String,
    lines_replaced: usize,
    lines_inserted: usize,
    new_total_lines: usize,
}
```

**Use Cases**:
- Replace 3 lines with 1 line
- Replace 1 line with 5 lines
- Complete line range replacement

**Tags**: `file`, `edit`, `lines`

---

## Symbol Tools
**Location**: `crates/serena-symbol/src/tools.rs`

### 10. get_symbols_overview
**Struct**: `GetSymbolsOverviewTool`

**Description**: Gets high-level symbol overview (classes, functions, etc.) with optional depth.

**Parameters**:
```json
{
  "relative_path": {
    "type": "string",
    "required": true,
    "description": "File to analyze"
  },
  "depth": {
    "type": "integer",
    "required": false,
    "default": 0,
    "description": "Depth of descendants (0 = top-level only)"
  },
  "max_answer_chars": {
    "type": "integer",
    "required": false,
    "default": -1
  }
}
```

**Returns**: Formatted string of symbols with kind, name, and line range.

**Output Format**:
```
Function main [1:0-10:0]
  Variable x [2:4-2:5]
  Variable y [3:4-3:5]
Class MyClass [12:0-25:0]
  Method __init__ [13:4-15:8]
  Method process [17:4-24:8]
```

**Tags**: `symbol`, `read`, `lsp`

---

### 11. find_symbol
**Struct**: `FindSymbolTool`

**Description**: Finds symbols matching a name path pattern with optional body inclusion.

**Parameters**:
```json
{
  "name_path_pattern": {
    "type": "string",
    "required": true,
    "description": "Name path (e.g., 'method', 'Class/method', '/Class/method')"
  },
  "relative_path": {
    "type": "string",
    "required": false,
    "description": "File or directory to restrict search"
  },
  "depth": {
    "type": "integer",
    "required": false,
    "default": 0
  },
  "include_body": {
    "type": "boolean",
    "required": false,
    "default": false,
    "description": "Include symbol's source code body"
  },
  "substring_matching": {
    "type": "boolean",
    "required": false,
    "default": false
  },
  "max_answer_chars": {
    "type": "integer",
    "required": false,
    "default": -1
  }
}
```

**Returns**:
```json
{
  "matches": [
    {
      "name": "method_name",
      "kind": "Function",
      "line": 42,
      "character": 4,
      "body": "def method_name():\n    pass",  // if include_body=true
      "path": "src/file.py"
    }
  ],
  "count": 1
}
```

**Tags**: `symbol`, `search`, `lsp`

---

### 12. find_referencing_symbols
**Struct**: `FindReferencingSymbolsTool`

**Description**: Finds all references to a symbol using LSP.

**Parameters**:
```json
{
  "name_path": {
    "type": "string",
    "required": true,
    "description": "Name path of symbol to find references for"
  },
  "relative_path": {
    "type": "string",
    "required": true,
    "description": "File containing the symbol"
  },
  "max_answer_chars": {
    "type": "integer",
    "required": false,
    "default": -1
  }
}
```

**Returns**:
```json
{
  "references": [
    {
      "path": "src/another.py",
      "line": 15,
      "character": 8,
      "context": "14: ...\n15: call_to_symbol()\n16: ..."
    }
  ],
  "count": 1
}
```

**Context**: Provides 1 line before and after each reference.

**Tags**: `symbol`, `references`, `lsp`

---

### 13. replace_symbol_body
**Struct**: `ReplaceSymbolBodyTool`

**Description**: Replaces the entire body of a symbol with new content.

**Parameters**:
```json
{
  "name_path": {
    "type": "string",
    "required": true,
    "description": "Name path of symbol to replace"
  },
  "relative_path": {
    "type": "string",
    "required": true
  },
  "body": {
    "type": "string",
    "required": true,
    "description": "New body content"
  }
}
```

**Behavior**:
- Finds symbol using LSP document symbols
- Replaces entire symbol range (from start.line to end.line)
- Preserves file structure around symbol

**Tags**: `symbol`, `edit`, `lsp`

---

### 14. rename_symbol
**Struct**: `RenameSymbolTool`

**Description**: Renames a symbol across the entire codebase using LSP rename.

**Parameters**:
```json
{
  "name_path": {
    "type": "string",
    "required": true
  },
  "relative_path": {
    "type": "string",
    "required": true
  },
  "new_name": {
    "type": "string",
    "required": true,
    "description": "The new name for the symbol"
  }
}
```

**Returns**:
```json
{
  "message": "Renamed 'old_name' to 'new_name' in 5 files",
  "files_changed": 5
}
```

**Implementation**:
- Uses LSP rename capability
- Applies WorkspaceEdit across all files
- Uses optimized `apply_text_edits` with O(1) offset calculation
- Handles reverse order application to preserve positions

**Tags**: `symbol`, `rename`, `edit`, `lsp`

---

### 15. insert_after_symbol
**Struct**: `InsertAfterSymbolTool`

**Description**: Inserts content after a symbol's end line.

**Parameters**:
```json
{
  "name_path": {
    "type": "string",
    "required": true
  },
  "relative_path": {
    "type": "string",
    "required": true
  },
  "content": {
    "type": "string",
    "required": true,
    "description": "Content to insert after the symbol"
  }
}
```

**Behavior**:
- Adds blank line before content for separation
- Ensures trailing newline
- Inserts at line after symbol's end

**Tags**: `symbol`, `edit`, `insert`, `lsp`

---

### 16. insert_before_symbol
**Struct**: `InsertBeforeSymbolTool`

**Description**: Inserts content before a symbol's start line.

**Parameters**:
```json
{
  "name_path": {
    "type": "string",
    "required": true
  },
  "relative_path": {
    "type": "string",
    "required": true
  },
  "content": {
    "type": "string",
    "required": true
  }
}
```

**Behavior**:
- Adds blank line after content for separation
- Ensures trailing newline
- Inserts at symbol's start line

**Tags**: `symbol`, `edit`, `insert`, `lsp`

---

## Memory Tools
**Location**: `crates/serena-memory/src/tools.rs`

### 17. write_memory
**Struct**: `WriteMemoryTool`

**Description**: Writes content to a named memory file (markdown). Creates or overwrites.

**Parameters**:
```json
{
  "memory_name": {
    "type": "string",
    "required": true,
    "description": "Memory name (without .md extension)"
  },
  "content": {
    "type": "string",
    "required": true,
    "description": "Markdown content to write"
  }
}
```

**Storage**: `.serena/memories/<memory_name>.md`
**Indexing**: SQLite database for fast searching

**Tags**: `memory`, `write`

---

### 18. read_memory
**Struct**: `ReadMemoryTool`

**Description**: Reads a named memory file.

**Parameters**:
```json
{
  "memory_name": {
    "type": "string",
    "required": true
  }
}
```

**Returns**:
```rust
struct ReadMemoryOutput {
    memory_name: String,
    content: String,
    exists: bool,
}
```

**Tags**: `memory`, `read`

---

### 19. list_memories
**Struct**: `ListMemoriesTool`

**Description**: Lists all available memory files, sorted alphabetically.

**Parameters**: None

**Returns**:
```rust
struct ListMemoriesOutput {
    memories: Vec<String>,
    count: usize,
}
```

**Tags**: `memory`, `list`

---

### 20. delete_memory
**Struct**: `DeleteMemoryTool`

**Description**: Deletes a memory file. Removes both markdown file and database entry.

**Parameters**:
```json
{
  "memory_name": {
    "type": "string",
    "required": true
  }
}
```

**Warning**: Operation cannot be undone.

**Tags**: `memory`, `delete`

---

### 21. edit_memory
**Struct**: `EditMemoryTool`

**Description**: Edits memory content using find/replace (literal or regex).

**Parameters**:
```json
{
  "memory_name": {
    "type": "string",
    "required": true
  },
  "needle": {
    "type": "string",
    "required": true,
    "description": "Text or regex pattern to find"
  },
  "replacement": {
    "type": "string",
    "required": true
  },
  "use_regex": {
    "type": "boolean",
    "required": false,
    "default": false
  }
}
```

**Tags**: `memory`, `edit`

---

### 22. search_memories
**Struct**: `SearchMemoriesTool`

**Description**: Searches across all memories for matching content.

**Parameters**:
```json
{
  "query": {
    "type": "string",
    "required": true,
    "description": "Search query"
  }
}
```

**Returns**:
```rust
struct SearchResult {
    name: String,
    preview: String,  // First 200 chars
}

struct SearchMemoriesOutput {
    results: Vec<SearchResult>,
    count: usize,
}
```

**Tags**: `memory`, `search`

---

## Config Tools
**Location**: `crates/serena-config/src/tools.rs`

### 23. activate_project
**Struct**: `ActivateProjectTool`

**Description**: Activates a project by name or path. Auto-detects languages if new.

**Parameters**:
```json
{
  "name_or_path": {
    "type": "string",
    "required": true,
    "description": "Project name or directory path"
  }
}
```

**Returns**:
```rust
struct ActivateProjectOutput {
    project_name: String,
    project_root: String,
    languages: Vec<String>,
    message: String,
}
```

**Tags**: `config`, `project`

---

### 24. get_current_config
**Struct**: `GetCurrentConfigTool`

**Description**: Gets current configuration state.

**Parameters**: None

**Returns**:
```rust
struct ProjectInfo {
    name: String,
    root: String,
    languages: Vec<String>,
}

struct GetConfigOutput {
    active_project: Option<ProjectInfo>,
    active_context: String,
    active_modes: Vec<String>,
    project_count: usize,
    available_contexts: Vec<String>,
    available_modes: Vec<String>,
}
```

**Tags**: `config`, `info`

---

### 25. switch_modes
**Struct**: `SwitchModesTool`

**Description**: Switches active modes (interactive, planning, editing, one-shot).

**Parameters**:
```json
{
  "modes": {
    "type": "array",
    "items": { "type": "string" },
    "required": true,
    "description": "List of mode names to activate"
  }
}
```

**Tags**: `config`, `mode`

---

### 26. list_projects
**Struct**: `ListProjectsTool`

**Description**: Lists all registered projects with details.

**Parameters**: None

**Returns**:
```rust
struct ListProjectsOutput {
    projects: Vec<ProjectInfo>,
    count: usize,
    active_project: Option<String>,
}
```

**Tags**: `config`, `project`, `list`

---

### 27. get_active_tools
**Struct**: `GetActiveToolsTool`

**Description**: Lists currently active tools based on context and project.

**Parameters**: None

**Returns**:
```rust
struct GetActiveToolsOutput {
    tools: Vec<String>,
    count: usize,
    context: String,
}
```

**Tags**: `config`, `tools`

---

### 28. remove_project
**Struct**: `RemoveProjectTool`

**Description**: Removes a project from configuration. Must not be active.

**Parameters**:
```json
{
  "name": {
    "type": "string",
    "required": true
  }
}
```

**Tags**: `config`, `project`

---

## Command Tools
**Location**: `crates/serena-commands/src/tools.rs`

### 29. execute_shell_command
**Struct**: `ExecuteShellCommandTool`

**Description**: Executes shell commands with security validation and timeout.

**Parameters**:
```json
{
  "command": {
    "type": "string",
    "required": true,
    "description": "Shell command to execute"
  },
  "cwd": {
    "type": "string",
    "required": false,
    "description": "Working directory (defaults to project root)"
  },
  "capture_stderr": {
    "type": "boolean",
    "required": false,
    "default": true
  },
  "max_answer_chars": {
    "type": "integer",
    "required": false,
    "default": -1
  },
  "timeout_secs": {
    "type": "integer",
    "required": false,
    "default": 60,
    "description": "Command timeout in seconds"
  }
}
```

**Returns**:
```json
{
  "stdout": "...",
  "stderr": "...",
  "exit_code": 0,
  "working_directory": "/path/to/dir",
  "command": "echo hello"
}
```

**Security Validation**:
- Blocks dangerous patterns:
  - `rm -rf /`
  - `del /f /s /q c:\`
  - `format c:`
  - `mkfs`
  - `dd if=/dev/zero`
  - `:(){:|:&};:` (fork bomb)
- Validates non-empty commands
- Validates working directory exists

**Platform Support**:
- Windows: `cmd.exe /C`
- Unix: `sh -c`

**Tags**: `command`, `shell`, `execute`

---

## LSP Tools
**Location**: `crates/serena-lsp/src/tools.rs`

### 30. restart_language_server
**Struct**: `RestartLanguageServerTool`

**Description**: Restarts language server for a programming language.

**Parameters**:
```json
{
  "language": {
    "type": "string",
    "required": true,
    "description": "Programming language (e.g., 'rust', 'python', 'typescript')"
  }
}
```

**Supported Languages**:
- rust, python, javascript/js, typescript/ts
- go/golang, java, ruby, php
- csharp/c#, swift, kotlin, scala
- elixir, perl, bash/shell, terraform/tf
- vue, clojure, powershell
- cpp/c++, c, haskell, lua, dart

**Tags**: `lsp`, `language-server`

---

### 31. list_language_servers
**Struct**: `ListLanguageServersTool`

**Description**: Lists currently running language servers.

**Parameters**: None

**Returns**:
```rust
struct ListServersOutput {
    servers: Vec<String>,
    count: usize,
}
```

**Tags**: `lsp`, `language-server`, `list`

---

### 32. stop_language_server
**Struct**: `StopLanguageServerTool`

**Description**: Stops language server for a language.

**Parameters**:
```json
{
  "language": {
    "type": "string",
    "required": true
  }
}
```

**Tags**: `lsp`, `language-server`

---

### 33. clear_lsp_cache
**Struct**: `ClearLspCacheTool`

**Description**: Clears LSP response cache, forcing fresh requests.

**Parameters**: None

**Use Case**: When files modified outside editor.

**Tags**: `lsp`, `cache`

---

## Workflow Tools
**Location**: `crates/serena-tools/src/workflow/mod.rs`

### 34. check_onboarding_performed
**Struct**: `CheckOnboardingPerformedTool`

**Description**: Checks if project onboarding was already performed.

**Parameters**: None

**Returns**:
```json
{
  "onboarded": true,
  "message": "Onboarding was already performed. Project memories are available..."
}
```

**Tags**: `workflow`, `onboarding`

---

### 35. onboarding
**Struct**: `OnboardingTool`

**Description**: Provides onboarding instructions for new projects.

**Parameters**: None

**Returns**: Platform-specific onboarding prompt.

**Tags**: `workflow`, `onboarding`

---

### 36. think_about_collected_information
**Struct**: `ThinkAboutCollectedInformationTool`

**Description**: Prompt for reflecting on collected information completeness.

**Parameters**: None

**Tags**: `workflow`, `thinking`

---

### 37. think_about_task_adherence
**Struct**: `ThinkAboutTaskAdherenceTool`

**Description**: Prompt for checking if still on track with task.

**Parameters**: None

**Usage**: Call before inserting/replacing/deleting code.

**Tags**: `workflow`, `thinking`

---

### 38. think_about_whether_you_are_done
**Struct**: `ThinkAboutWhetherYouAreDoneTool`

**Description**: Prompt for determining task completion.

**Parameters**: None

**Tags**: `workflow`, `thinking`

---

### 39. summarize_changes
**Struct**: `SummarizeChangesTool`

**Description**: Instructions for summarizing codebase changes.

**Parameters**: None

**Tags**: `workflow`, `summary`

---

### 40. prepare_for_new_conversation
**Struct**: `PrepareForNewConversationTool`

**Description**: Instructions for preparing context for new session.

**Parameters**: None

**Usage**: When context running low and task needs continuation.

**Tags**: `workflow`, `context`

---

### 41. initial_instructions
**Struct**: `InitialInstructionsTool`

**Description**: Provides Serena Instructions Manual.

**Parameters**: None

**Does not require project**: Can be called without active project.

**Tags**: `workflow`, `documentation`

---

## Python vs Rust Gaps

### Tools Present in Python but Missing in Rust

Based on Python tool files, these tools are NOT yet implemented in Rust:

#### From file_tools.py:
- **None** - All core file tools are implemented

#### From symbol_tools.py:
- **Additional symbol operations** may exist

#### From jetbrains_tools.py:
- **JetBrains IDE integration tools**:
  - `open_file_in_editor`
  - `navigate_to_symbol`
  - `show_diff`
  - Other IDE-specific operations

#### From cmd_tools.py:
- **Run terminal command** (if different from execute_shell_command)
- **Install packages/dependencies**

#### From workflow_tools.py:
- All appear to be implemented

#### From memory_tools.py:
- All core operations implemented

#### From config_tools.py:
- All core operations implemented

### Parameter Signature Differences

#### Line Number Indexing:
- **Python**: May use 1-based line numbers
- **Rust**:
  - `read_file`: 0-based line numbers
  - Editor tools: 1-based line numbers
  - Symbol tools: Line numbers from LSP (0-based internally)

**Recommendation**: Verify Python uses same convention for consistency.

#### Default Values:
- **max_answer_chars**: Both default to -1 (unlimited)
- **Timeout**:
  - Rust `execute_shell_command`: 60 seconds
  - Python: Verify timeout value

#### Path Handling:
- **Rust**: All paths validated with canonicalization
- **Python**: Verify same security measures

### Missing Functionality

1. **HTTP Transport**:
   - Rust has `serena-mcp/src/transport/http.rs` (not yet tools)
   - Python may have HTTP-related tools

2. **Resource Management**:
   - Rust has LSP resources in separate module
   - Python may expose as tools

3. **Prompt Resources**:
   - Rust has `serena-core/src/prompts.rs`
   - May not be exposed as tools yet

4. **JetBrains Integration**:
   - Python has full integration
   - Rust appears to focus on LSP only

---

## Parameter Schema Reference

### Common Parameter Patterns

#### Relative Path
```json
{
  "relative_path": {
    "type": "string",
    "required": true,
    "description": "Path relative to project root"
  }
}
```

#### Line Range (1-based)
```json
{
  "start_line": {
    "type": "integer",
    "minimum": 1,
    "required": true
  },
  "end_line": {
    "type": "integer",
    "minimum": 1,
    "required": true
  }
}
```

#### Max Characters
```json
{
  "max_answer_chars": {
    "type": "integer",
    "required": false,
    "default": -1,
    "description": "-1 for unlimited"
  }
}
```

#### Search Pattern
```json
{
  "pattern": {
    "type": "string",
    "required": true,
    "description": "Regex pattern"
  }
}
```

---

## Tool Tags Summary

| Tag | Count | Tools |
|-----|-------|-------|
| `file` | 9 | read_file, create_text_file, list_directory, find_file, search_files, replace_content, delete_lines, insert_at_line, replace_lines |
| `edit` | 9 | create_text_file, replace_content, delete_lines, insert_at_line, replace_lines, replace_symbol_body, rename_symbol, insert_after_symbol, insert_before_symbol |
| `symbol` | 7 | get_symbols_overview, find_symbol, find_referencing_symbols, replace_symbol_body, rename_symbol, insert_after_symbol, insert_before_symbol |
| `lsp` | 11 | All symbol tools + 4 LSP management tools |
| `memory` | 6 | write_memory, read_memory, list_memories, delete_memory, edit_memory, search_memories |
| `config` | 6 | activate_project, get_current_config, switch_modes, list_projects, get_active_tools, remove_project |
| `command` | 1 | execute_shell_command |
| `workflow` | 8 | All workflow tools |
| `search` | 3 | find_file, search_files, search_memories, find_symbol |
| `read` | 3 | read_file, read_memory, get_symbols_overview |
| `write` | 2 | create_text_file, write_memory |

---

## Implementation Statistics

- **Total Tools**: 41
- **File Operations**: 9
- **Editor Operations**: 3
- **Symbol Operations**: 7
- **Memory Operations**: 6
- **Config Operations**: 6
- **Command Operations**: 1
- **LSP Management**: 4
- **Workflow/Meta**: 8

---

## Testing Recommendations

### Critical Tests for Equivalence

1. **Security**:
   - Path traversal prevention
   - Dangerous command blocking
   - Project boundary enforcement

2. **Line Number Handling**:
   - Verify 0-based vs 1-based consistency
   - Edge cases (line 0, beyond EOF)
   - Empty files

3. **Content Truncation**:
   - max_answer_chars behavior
   - Truncation message format

4. **Error Messages**:
   - Compare Python vs Rust error formats
   - Ensure same information provided

5. **Symbol Operations**:
   - LSP integration correctness
   - Name path resolution
   - Workspace edit application

6. **Parallel Processing**:
   - search_files performance
   - Race condition handling

7. **Character Encoding**:
   - UTF-8 handling
   - Line ending preservation (CRLF vs LF)

---

## Version Information

- **Document Version**: 1.0
- **Date**: 2025-12-25
- **Rust Tooling**: As of serena-source main branch
- **Comparison Base**: Python implementation in src/serena/tools/

---

## Notes for Implementation Team

1. **JetBrains Integration**: Decide if Rust should implement or remain Python-only
2. **HTTP Tools**: Complete HTTP transport tool exposure
3. **Line Number Convention**: Standardize 0-based or 1-based across all tools
4. **Error Format**: Ensure Rust errors match Python format for client compatibility
5. **Performance**: Document performance differences (parallel search, caching)
6. **Platform Support**: Test all tools on Windows, macOS, Linux
7. **LSP Compatibility**: Verify all LSP tools work with all supported language servers

---

End of Document
