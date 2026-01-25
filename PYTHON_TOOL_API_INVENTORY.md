# Python Tool API Inventory - Complete Reference for Rust Equivalence Testing

**Generated**: 2025-12-25
**Purpose**: Complete API contract documentation for comparing Python and Rust tool implementations

## Table of Contents

1. [Tool Base Architecture](#tool-base-architecture)
2. [File Tools](#file-tools)
3. [Symbol Tools](#symbol-tools)
4. [Memory Tools](#memory-tools)
5. [Config Tools](#config-tools)
6. [Command Tools](#command-tools)
7. [Workflow Tools](#workflow-tools)
8. [JetBrains Tools](#jetbrains-tools)
9. [Common Patterns](#common-patterns)

---

## Tool Base Architecture

### Base Class: `Tool`

**Location**: `src/serena/tools/tools_base.py`

**Core Methods**:
- `apply(**kwargs) -> str` - Must be implemented by subclasses
- `apply_ex(log_call: bool = True, catch_exceptions: bool = True, **kwargs) -> str` - Wrapper with logging/error handling
- `get_name() -> str` - Returns snake_case tool name (e.g., "ReadFileTool" -> "read_file")
- `is_active() -> bool` - Checks if tool is enabled in current context
- `_limit_length(result: str, max_answer_chars: int) -> str` - Truncates long responses
- `_to_json(x: Any) -> str` - JSON serialization helper

**Tool Markers** (mixins for classification):
- `ToolMarkerCanEdit` - Tool can modify files
- `ToolMarkerSymbolicRead` - Tool reads symbols via LSP
- `ToolMarkerSymbolicEdit` - Tool edits symbols via LSP
- `ToolMarkerDoesNotRequireActiveProject` - Tool works without active project
- `ToolMarkerOptional` - Tool disabled by default

**Common Behaviors**:
1. **Timeout**: All tools have configurable timeout (default from `serena_config.tool_timeout`)
2. **Active Project Check**: Most tools require active project (except those with `ToolMarkerDoesNotRequireActiveProject`)
3. **Response Length Limiting**: `max_answer_chars` parameter (default -1 uses config value)
4. **LSP Server Recovery**: Automatic restart on `SolidLSPException` if language server terminates
5. **Tool Usage Recording**: All executions logged via `agent.record_tool_usage()`
6. **Cache Saving**: LSP caches saved after every tool execution

**Success Result**: Constant `SUCCESS_RESULT = "OK"` returned on successful edits

---

## File Tools

**Location**: `src/serena/tools/file_tools.py`

### 1. ReadFileTool

**Tool Name**: `read_file`
**Markers**: None
**Description**: Reads a file within the project directory.

**Parameters**:
```python
relative_path: str           # Required - validated via project.validate_relative_path()
start_line: int = 0          # 0-based index of first line
end_line: int | None = None  # 0-based index of last line (inclusive), None = until EOF
max_answer_chars: int = -1   # Response length limit
```

**Return**: `str` - File content (optionally sliced by line range)

**Validation**:
- Path must exist and not be ignored: `project.validate_relative_path(relative_path, require_not_ignored=True)`
- Line slicing: `result_lines[start_line:end_line+1]`

**Error Handling**:
- Raises exception if path invalid or ignored
- Returns truncated message if exceeds `max_answer_chars`

---

### 2. CreateTextFileTool

**Tool Name**: `create_text_file`
**Markers**: `ToolMarkerCanEdit`
**Description**: Creates/overwrites a file in the project directory.

**Parameters**:
```python
relative_path: str  # Required - must be within project root
content: str        # Required - file content to write
```

**Return**: `str` - Success message (e.g., "File created: path/to/file. Overwrote existing file.")

**Validation**:
- If file exists: `project.validate_relative_path(relative_path, require_not_ignored=True)`
- If new file: Must be relative to project root via `abs_path.is_relative_to(project_root)`
- Creates parent directories if needed: `abs_path.parent.mkdir(parents=True, exist_ok=True)`

**Business Logic**:
1. Resolve absolute path
2. Check if file exists (determines "overwrote" message)
3. Validate path
4. Create parent directories
5. Write content with project encoding
6. Return success message

---

### 3. ListDirTool

**Tool Name**: `list_dir`
**Markers**: None
**Description**: Lists files and directories in the given directory (optionally with recursion).

**Parameters**:
```python
relative_path: str              # Required - "." for project root
recursive: bool                 # Required
skip_ignored_files: bool = False
max_answer_chars: int = -1
```

**Return**: `str` - JSON: `{"dirs": [...], "files": [...]}`

**Validation**:
- Path existence check before validation: `project.relative_path_exists(relative_path)`
- If not exists, returns JSON error: `{"error": "Directory not found: ...", "project_root": "...", "hint": "..."}`
- Path validation: `project.validate_relative_path(relative_path, require_not_ignored=skip_ignored_files)`

**Business Logic**:
- Uses `scan_directory()` with optional ignore filters
- Returns relative paths from project root
- Results grouped into directories and files

---

### 4. FindFileTool

**Tool Name**: `find_file`
**Markers**: None
**Description**: Finds files in the given relative paths.

**Parameters**:
```python
file_mask: str       # Required - wildcards * and ? supported (fnmatch)
relative_path: str   # Required - "." for project root
```

**Return**: `str` - JSON: `{"files": [...]}`

**Validation**:
- Path validation: `project.validate_relative_path(relative_path, require_not_ignored=True)`
- Filename matching via `fnmatch(filename, file_mask)`

**Business Logic**:
- Recursively scans directory
- Filters by file mask (ignoring non-matches)
- Respects project ignore patterns

---

### 5. ReplaceContentTool

**Tool Name**: `replace_content`
**Markers**: `ToolMarkerCanEdit`
**Description**: Replaces content in a file (optionally using regular expressions).

**Parameters**:
```python
relative_path: str                            # Required
needle: str                                   # Required - literal string or regex
repl: str                                     # Required - replacement (supports $!1, $!2 backreferences in regex mode)
mode: Literal["literal", "regex"]             # Required
allow_multiple_occurrences: bool = False
```

**Return**: `str` - `SUCCESS_RESULT` on success

**Validation**:
- Path validation: `project.validate_relative_path(relative_path, require_not_ignored=True)`
- Mode must be "literal" or "regex"
- Match count validation:
  - If 0 matches: `ValueError("No matches of search expression found")`
  - If >1 matches and not allowed: `ValueError("Expression matches N occurrences...")`
- **Ambiguity detection**: For multi-line matches, checks if pattern matches again within matched text (prevents unintended replacements)

**Business Logic**:
1. Read file via `EditedFileContext`
2. Compile pattern with `re.DOTALL | re.MULTILINE`
3. Perform replacement via custom function that:
   - Validates ambiguity for multi-line matches
   - Expands backreferences ($!1, $!2, etc.)
4. Validate match count
5. Write updated content

**Special Features**:
- **Backreference syntax**: `$!1`, `$!2` instead of standard `\1`, `\2`
- **Ambiguity prevention**: Detects overlapping matches in multi-line contexts
- **Internal variant**: `replace_content()` method with `require_not_ignored` parameter for internal use

---

### 6. DeleteLinesTool

**Tool Name**: `delete_lines`
**Markers**: `ToolMarkerCanEdit`, `ToolMarkerOptional`
**Description**: Deletes a range of lines within a file.

**Parameters**:
```python
relative_path: str  # Required
start_line: int     # Required - 0-based index
end_line: int       # Required - 0-based index (inclusive)
```

**Return**: `str` - `SUCCESS_RESULT` on success

**Validation**:
- Requires prior read of same line range (enforced by CodeEditor)

**Business Logic**:
- Delegates to `code_editor.delete_lines()`
- CodeEditor validates line range was previously read

---

### 7. ReplaceLinesTool

**Tool Name**: `replace_lines`
**Markers**: `ToolMarkerCanEdit`, `ToolMarkerOptional`
**Description**: Replaces a range of lines within a file with new content.

**Parameters**:
```python
relative_path: str  # Required
start_line: int     # Required - 0-based index
end_line: int       # Required - 0-based index (inclusive)
content: str        # Required - replacement content
```

**Return**: `str` - `SUCCESS_RESULT` on success

**Validation**:
- Requires prior read of same line range
- Auto-appends newline if missing: `if not content.endswith("\n"): content += "\n"`

**Business Logic**:
1. Call `delete_lines(relative_path, start_line, end_line)`
2. If deletion fails, return error
3. Call `insert_at_line(relative_path, start_line, content)`
4. Return `SUCCESS_RESULT`

---

### 8. InsertAtLineTool

**Tool Name**: `insert_at_line`
**Markers**: `ToolMarkerCanEdit`, `ToolMarkerOptional`
**Description**: Inserts content at a given line in a file.

**Parameters**:
```python
relative_path: str  # Required
line: int           # Required - 0-based index (existing content pushed down)
content: str        # Required - content to insert
```

**Return**: `str` - `SUCCESS_RESULT` on success

**Validation**:
- Auto-appends newline if missing

**Business Logic**:
- Delegates to `code_editor.insert_at_line()`
- Content inserted before specified line, pushing existing content down

---

### 9. SearchForPatternTool

**Tool Name**: `search_for_pattern`
**Markers**: None
**Description**: Performs a search for a pattern in the project.

**Parameters**:
```python
substring_pattern: str                     # Required - regex pattern (DOTALL enabled)
context_lines_before: int = 0
context_lines_after: int = 0
paths_include_glob: str = ""               # Glob pattern (e.g., "*.py", "src/**/*.ts")
paths_exclude_glob: str = ""               # Takes precedence over include
relative_path: str = ""                    # Restrict to file/directory
restrict_search_to_code_files: bool = False
max_answer_chars: int = -1
```

**Return**: `str` - JSON: `{file_path: [match1, match2, ...]}`

**Validation**:
- Path existence: `if not os.path.exists(abs_path): raise FileNotFoundError`
- Glob patterns support standard wildcards (`*`, `?`, `[seq]`, `**`) and brace expansion (`{a,b,c}`)

**Business Logic**:
1. Determine search scope (code files only or all files)
2. If `restrict_search_to_code_files=True`: Use `project.search_source_files_for_pattern()`
3. If `restrict_search_to_code_files=False`:
   - Scan directory with ignore filters
   - Search files with glob filtering
4. Group matches by file
5. Return JSON mapping file paths to matched lines

**Pattern Matching**:
- Compiled with `re.DOTALL` (dot matches newlines)
- Multi-line matches include all matched lines
- Context lines included before/after each match

---

## Symbol Tools

**Location**: `src/serena/tools/symbol_tools.py`

**Common Requirements**: All symbol tools require language server to be active (via `create_language_server_symbol_retriever()`)

### 1. RestartLanguageServerTool

**Tool Name**: `restart_language_server`
**Markers**: `ToolMarkerOptional`
**Description**: Restarts the language server, may be necessary when edits not through Serena happen.

**Parameters**: None

**Return**: `str` - `SUCCESS_RESULT`

**Business Logic**:
- Calls `agent.reset_language_server_manager()`
- Use only on explicit user request or after confirmation

---

### 2. GetSymbolsOverviewTool

**Tool Name**: `get_symbols_overview`
**Markers**: `ToolMarkerSymbolicRead`
**Description**: Gets an overview of the top-level symbols defined in a given file.

**Parameters**:
```python
relative_path: str          # Required - must be a file (not directory)
depth: int = 0              # Depth of descendant retrieval (1 = immediate children)
max_answer_chars: int = -1
```

**Return**: `str` - JSON array of symbol objects (sanitized)

**Validation**:
- Path existence: `if not os.path.exists(file_path): raise FileNotFoundError`
- Directory check: `if os.path.isdir(file_path): raise ValueError("Expected a file path...")`

**Business Logic**:
1. Get symbol retriever
2. Call `symbol_retriever.get_symbol_overview(relative_path, depth=depth)`
3. Extract symbols for the file
4. Sanitize symbols (remove redundant fields)
5. Return JSON

**Symbol Sanitization**:
- Remove `location` field (replaced with `relative_path`)
- Remove `name` field (use `name_path` instead)

---

### 3. FindSymbolTool

**Tool Name**: `find_symbol`
**Markers**: `ToolMarkerSymbolicRead`
**Description**: Performs a global (or local) search using the language server backend.

**Parameters**:
```python
name_path_pattern: str                  # Required - name, relative path, or /absolute/path
depth: int = 0                          # Descendant depth
relative_path: str = ""                 # Restrict to file/directory
include_body: bool = False              # Include source code
include_kinds: list[int] = []           # LSP symbol kind integers to include
exclude_kinds: list[int] = []           # LSP symbol kind integers to exclude (takes precedence)
substring_matching: bool = False        # Match substrings in last element
max_answer_chars: int = -1
```

**Return**: `str` - JSON array of symbol objects

**Validation**:
- `include_kinds` and `exclude_kinds` converted to `SymbolKind` enum values
- If empty list, treated as None (no filtering)

**Name Path Patterns**:
- Simple name: `"method"` - matches any symbol with that name
- Relative path: `"class/method"` - matches name path suffix
- Absolute path: `"/class/method"` - exact match within source file
- Overload index: `"MyClass/my_method[1]"` - specific overload

**Symbol Kinds** (LSP standard):
```
1=file, 2=module, 3=namespace, 4=package, 5=class, 6=method, 7=property,
8=field, 9=constructor, 10=enum, 11=interface, 12=function, 13=variable,
14=constant, 15=string, 16=number, 17=boolean, 18=array, 19=object,
20=key, 21=null, 22=enum member, 23=struct, 24=event, 25=operator,
26=type parameter
```

**Business Logic**:
1. Parse kind filters to `SymbolKind` enums
2. Call `symbol_retriever.find()` with parameters
3. Convert symbols to dicts with specified depth and body inclusion
4. Sanitize symbols
5. Return JSON

---

### 4. FindReferencingSymbolsTool

**Tool Name**: `find_referencing_symbols`
**Markers**: `ToolMarkerSymbolicRead`
**Description**: Finds symbols that reference the given symbol using the language server backend.

**Parameters**:
```python
name_path: str                  # Required - same logic as find_symbol
relative_path: str              # Required - must be a file (not directory)
include_kinds: list[int] = []
exclude_kinds: list[int] = []
max_answer_chars: int = -1
```

**Return**: `str` - JSON array of reference objects with:
- Symbol metadata (sanitized)
- `content_around_reference` field with context lines

**Validation**:
- Same kind filtering as `find_symbol`
- `include_body` always `False` (bodies not useful for referencing symbols)

**Business Logic**:
1. Parse kind filters
2. Call `symbol_retriever.find_referencing_symbols()`
3. For each reference:
   - Sanitize symbol dict
   - Retrieve context lines (1 before, 1 after) around reference
   - Add `content_around_reference` field
4. Return JSON

---

### 5. ReplaceSymbolBodyTool

**Tool Name**: `replace_symbol_body`
**Markers**: `ToolMarkerSymbolicEdit`
**Description**: Replaces the full definition of a symbol using the language server backend.

**Parameters**:
```python
name_path: str       # Required - same logic as find_symbol
relative_path: str   # Required
body: str            # Required - new symbol body (includes signature, excludes docstrings/imports)
```

**Return**: `str` - `SUCCESS_RESULT`

**Validation**:
- Body must not include preceding docstrings/comments or imports
- Body includes the symbol's signature/definition line

**Business Logic**:
- Delegates to `code_editor.replace_body(name_path, relative_file_path, body)`
- CodeEditor uses LSP to locate symbol and perform replacement

**Important Notes**:
- Only use if you know exact symbol body structure
- Body is the definition (e.g., function signature + implementation)
- Does NOT include docstrings above the symbol

---

### 6. InsertAfterSymbolTool

**Tool Name**: `insert_after_symbol`
**Markers**: `ToolMarkerSymbolicEdit`
**Description**: Inserts content after the end of the definition of a given symbol.

**Parameters**:
```python
name_path: str       # Required
relative_path: str   # Required
body: str            # Required - content to insert
```

**Return**: `str` - `SUCCESS_RESULT`

**Use Cases**:
- Insert new class after existing class
- Insert new method after existing method
- Insert new field/variable after existing one

**Business Logic**:
- Delegates to `code_editor.insert_after_symbol()`
- Content inserted on next line after symbol's definition ends

---

### 7. InsertBeforeSymbolTool

**Tool Name**: `insert_before_symbol`
**Markers**: `ToolMarkerSymbolicEdit`
**Description**: Inserts content before the beginning of the definition of a given symbol.

**Parameters**:
```python
name_path: str       # Required
relative_path: str   # Required
body: str            # Required - content to insert
```

**Return**: `str` - `SUCCESS_RESULT`

**Use Cases**:
- Insert new class before existing class
- Insert import statement before first symbol in file
- Insert new method before existing method

**Business Logic**:
- Delegates to `code_editor.insert_before_symbol()`
- Content inserted on line before symbol's definition begins

---

### 8. RenameSymbolTool

**Tool Name**: `rename_symbol`
**Markers**: `ToolMarkerSymbolicEdit`
**Description**: Renames a symbol throughout the codebase using language server refactoring capabilities.

**Parameters**:
```python
name_path: str       # Required - may need signature for overloaded methods
relative_path: str   # Required
new_name: str        # Required
```

**Return**: `str` - Status message (success or failure details)

**Validation**:
- For overloaded methods (Java, etc.), name_path may need signature index: `"MyClass/method[0]"`

**Business Logic**:
- Delegates to `code_editor.rename_symbol()`
- Uses LSP rename capability to update all references codebase-wide
- Returns detailed status message from language server

---

## Memory Tools

**Location**: `src/serena/tools/memory_tools.py`

**Storage**: Markdown files in `.serena/memories/` directories (UTF-8 encoding)

### 1. WriteMemoryTool

**Tool Name**: `write_memory`
**Markers**: None
**Description**: Writes a named memory (for future reference) to Serena's project-specific memory store.

**Parameters**:
```python
memory_file_name: str      # Required - meaningful name (no extension needed)
content: str               # Required - markdown-formatted content
max_answer_chars: int = -1
```

**Return**: `str` - Success message from `MemoriesManager.save_memory()`

**Validation**:
- Content length must not exceed `max_answer_chars`
- If exceeded: `ValueError("Content for X is too long. Max length is Y...")`

**Business Logic**:
1. Validate content length
2. Call `memories_manager.save_memory(memory_file_name, content)`
3. Return success message

---

### 2. ReadMemoryTool

**Tool Name**: `read_memory`
**Markers**: None
**Description**: Reads the memory with the given name from Serena's project-specific memory store.

**Parameters**:
```python
memory_file_name: str      # Required
max_answer_chars: int = -1
```

**Return**: `str` - Memory content

**Business Logic**:
- Calls `memories_manager.load_memory(memory_file_name)`
- Returns content directly (no length limiting applied)

**Usage Notes**:
- Only read if relevant to current task (infer from file name)
- Don't read same memory multiple times in one conversation

---

### 3. ListMemoriesTool

**Tool Name**: `list_memories`
**Markers**: None
**Description**: Lists memories in Serena's project-specific memory store.

**Parameters**: None

**Return**: `str` - JSON array of memory file names

**Business Logic**:
- Calls `memories_manager.list_memories()`
- Returns JSON-encoded list

---

### 4. DeleteMemoryTool

**Tool Name**: `delete_memory`
**Markers**: None
**Description**: Deletes a memory from Serena's project-specific memory store.

**Parameters**:
```python
memory_file_name: str  # Required
```

**Return**: `str` - Success message from `MemoriesManager.delete_memory()`

**Validation**:
- Should only be used on explicit user request
- Use when information is outdated or no longer relevant

---

### 5. EditMemoryTool

**Tool Name**: `edit_memory`
**Markers**: None
**Description**: Replaces content matching a regular expression in a memory.

**Parameters**:
```python
memory_file_name: str                # Required
needle: str                          # Required - literal string or regex
repl: str                            # Required - replacement
mode: Literal["literal", "regex"]    # Required
```

**Return**: `str` - `SUCCESS_RESULT` on success

**Business Logic**:
1. Get memory file path relative to project root
2. Delegate to `ReplaceContentTool.replace_content()` with:
   - `require_not_ignored=False` (memory files are in ignored directory)
   - Same regex/literal logic as file replacement
3. Return result

**Note**: Internally uses `ReplaceContentTool` with special flag to bypass ignore validation

---

## Config Tools

**Location**: `src/serena/tools/config_tools.py`

### 1. ActivateProjectTool

**Tool Name**: `activate_project`
**Markers**: `ToolMarkerDoesNotRequireActiveProject`
**Description**: Activates a project based on the project name or path.

**Parameters**:
```python
project: str  # Required - project name (from config) or path to project directory
```

**Return**: `str` - Project activation message + instruction to read Serena manual

**Business Logic**:
1. Call `agent.activate_project_from_path_or_name(project)`
2. Get activation message from project
3. Append instruction to read manual if not already done
4. Return message

**Success Message Format**:
```
[Project activation details]
IMPORTANT: If you have not yet read the 'Serena Instructions Manual', do it now before continuing!
```

---

### 2. RemoveProjectTool

**Tool Name**: `remove_project`
**Markers**: `ToolMarkerDoesNotRequireActiveProject`, `ToolMarkerOptional`
**Description**: Removes a project from the Serena configuration.

**Parameters**:
```python
project_name: str  # Required - name of registered project
```

**Return**: `str` - Success message

**Business Logic**:
- Calls `agent.serena_config.remove_project(project_name)`
- Returns confirmation message

---

### 3. SwitchModesTool

**Tool Name**: `switch_modes`
**Markers**: `ToolMarkerOptional`
**Description**: Activates modes by providing a list of their names.

**Parameters**:
```python
modes: list[str]  # Required - mode names (e.g., ["editing", "interactive"])
```

**Return**: `str` - Multi-line message with:
- Activated mode names
- Prompts for each mode
- Currently active tool names

**Validation**:
- Mode names must be valid (loaded via `SerenaAgentMode.load()`)

**Business Logic**:
1. Load mode instances from names
2. Call `agent.set_modes(mode_instances)`
3. Build result string:
   ```
   Successfully activated modes: mode1, mode2
   [mode1 prompt]
   [mode2 prompt]
   Currently active tools: tool1, tool2, ...
   ```

**Common Modes**:
- `"planning"` - Planning mode (no edits)
- `"editing"` - Editing mode (full edit capabilities)
- `"interactive"` - Interactive mode
- `"one-shot"` - Single-turn mode

---

### 4. GetCurrentConfigTool

**Tool Name**: `get_current_config`
**Markers**: None
**Description**: Prints the current configuration of the agent.

**Parameters**: None

**Return**: `str` - Configuration overview with:
- Active project
- Available projects
- Active tools
- Active contexts
- Active modes

**Business Logic**:
- Calls `agent.get_current_config_overview()`
- Returns formatted overview

---

## Command Tools

**Location**: `src/serena/tools/cmd_tools.py`

### 1. ExecuteShellCommandTool

**Tool Name**: `execute_shell_command`
**Markers**: `ToolMarkerCanEdit`
**Description**: Executes a shell command.

**Parameters**:
```python
command: str                 # Required - shell command to execute
cwd: str | None = None       # Working directory (None = project root)
capture_stderr: bool = True  # Whether to capture stderr
max_answer_chars: int = -1
```

**Return**: `str` - JSON: `{"stdout": "...", "stderr": "...", "returncode": 0}`

**Validation**:
- If `cwd` is relative: Resolve relative to project root and check if directory exists
- If `cwd` is absolute: Use as-is
- If `cwd` is None: Use project root

**Business Logic**:
1. Resolve working directory
2. Call `execute_shell_command(command, cwd=_cwd, capture_stderr=capture_stderr)`
3. Convert result to JSON
4. Apply length limiting
5. Return JSON

**Important Restrictions**:
- **DO NOT** use for long-running processes (servers)
- **DO NOT** use for processes requiring user interaction
- **DO NOT** execute unsafe commands

**Safety Note**: Tool marked as `ToolMarkerCanEdit` because shell commands can modify files

---

## Workflow Tools

**Location**: `src/serena/tools/workflow_tools.py`

**Purpose**: Meta-tools for agent workflow management (onboarding, reflection, summarization)

### 1. CheckOnboardingPerformedTool

**Tool Name**: `check_onboarding_performed`
**Markers**: None
**Description**: Checks whether project onboarding was already performed.

**Parameters**: None

**Return**: `str` - Status message:
- If no memories: "Onboarding not performed yet... call `onboarding` tool..."
- If memories exist: "The onboarding was already performed, below is the list of available memories... [list]"

**Business Logic**:
1. Call `list_memories` tool
2. Parse JSON result
3. If empty: Return "not performed" message
4. If not empty: Return "already performed" message with memory list

**Usage**: Should be called after activating a project, before starting work

---

### 2. OnboardingTool

**Tool Name**: `onboarding`
**Markers**: None
**Description**: Performs onboarding (identifying the project structure and essential tasks).

**Parameters**: None

**Return**: `str` - Onboarding prompt with instructions

**Business Logic**:
1. Detect system: `platform.system()`
2. Call `prompt_factory.create_onboarding_prompt(system=system)`
3. Return prompt

**Usage**: Call at most once per conversation, only if onboarding not performed

---

### 3. ThinkAboutCollectedInformationTool

**Tool Name**: `think_about_collected_information`
**Markers**: None
**Description**: Thinking tool for pondering the completeness of collected information.

**Parameters**: None

**Return**: `str` - Prompt for reflection

**Business Logic**:
- Returns `prompt_factory.create_think_about_collected_information()`

**Usage**: Should ALWAYS be called after non-trivial search sequences (find_symbol, find_referencing_symbols, search_for_pattern, read_file, etc.)

---

### 4. ThinkAboutTaskAdherenceTool

**Tool Name**: `think_about_task_adherence`
**Markers**: None
**Description**: Thinking tool for determining whether the agent is still on track with the current task.

**Parameters**: None

**Return**: `str` - Prompt for task adherence check

**Business Logic**:
- Returns `prompt_factory.create_think_about_task_adherence()`

**Usage**: Should ALWAYS be called before inserting, replacing, or deleting code

---

### 5. ThinkAboutWhetherYouAreDoneTool

**Tool Name**: `think_about_whether_you_are_done`
**Markers**: None
**Description**: Thinking tool for determining whether the task is truly completed.

**Parameters**: None

**Return**: `str` - Prompt for completion check

**Business Logic**:
- Returns `prompt_factory.create_think_about_whether_you_are_done()`

**Usage**: Call whenever you feel the task is complete

---

### 6. SummarizeChangesTool

**Tool Name**: `summarize_changes`
**Markers**: `ToolMarkerOptional`
**Description**: Provides instructions for summarizing the changes made to the codebase.

**Parameters**: None

**Return**: `str` - Prompt for change summarization

**Business Logic**:
- Returns `prompt_factory.create_summarize_changes()`

**Usage**: Call after fully completing non-trivial coding task, but only after `think_about_whether_you_are_done`

---

### 7. PrepareForNewConversationTool

**Tool Name**: `prepare_for_new_conversation`
**Markers**: None
**Description**: Provides instructions for preparing for a new conversation.

**Parameters**: None

**Return**: `str` - Prompt for new conversation preparation

**Business Logic**:
- Returns `prompt_factory.create_prepare_for_new_conversation()`

**Usage**: Only call on explicit user request

---

### 8. InitialInstructionsTool

**Tool Name**: `initial_instructions`
**Markers**: `ToolMarkerDoesNotRequireActiveProject`
**Description**: Provides instructions on how to use the Serena toolbox.

**Parameters**: None

**Return**: `str` - Full system prompt ("Serena Instructions Manual")

**Business Logic**:
- Returns `agent.create_system_prompt()`

**Usage**:
- Call immediately after receiving task from user if manual not yet read
- Essential for MCP clients that don't read system prompt automatically (e.g., Claude Desktop)

---

## JetBrains Tools

**Location**: `src/serena/tools/jetbrains_tools.py`

**Purpose**: Alternative backend for symbol operations using JetBrains IDE plugin

**Markers**: All tools have `ToolMarkerOptional` (disabled by default)

### 1. JetBrainsFindSymbolTool

**Tool Name**: `jet_brains_find_symbol`
**Markers**: `ToolMarkerSymbolicRead`, `ToolMarkerOptional`
**Description**: Performs a global (or local) search for symbols using the JetBrains backend.

**Parameters**:
```python
name_path_pattern: str           # Required - same logic as find_symbol
depth: int = 0
relative_path: str | None = None
include_body: bool = False
search_deps: bool = False        # Also search in dependencies (unique to JetBrains)
max_answer_chars: int = -1
```

**Return**: `str` - JSON array of symbol objects

**Business Logic**:
1. Create JetBrains plugin client
2. Call `client.find_symbol()` via context manager
3. Convert response to JSON
4. Apply length limiting

**Unique Features**:
- `search_deps` parameter allows searching in project dependencies (libraries)

---

### 2. JetBrainsFindReferencingSymbolsTool

**Tool Name**: `jet_brains_find_referencing_symbols`
**Markers**: `ToolMarkerSymbolicRead`, `ToolMarkerOptional`
**Description**: Finds symbols that reference the given symbol using the JetBrains backend.

**Parameters**:
```python
name_path: str              # Required
relative_path: str          # Required - must be a file
max_answer_chars: int = -1
```

**Return**: `str` - JSON array of reference objects

**Business Logic**:
1. Create JetBrains plugin client
2. Call `client.find_references()`
3. Convert response to JSON
4. Apply length limiting

---

### 3. JetBrainsGetSymbolsOverviewTool

**Tool Name**: `jet_brains_get_symbols_overview`
**Markers**: `ToolMarkerSymbolicRead`, `ToolMarkerOptional`
**Description**: Retrieves an overview of the top-level symbols within a specified file using the JetBrains backend.

**Parameters**:
```python
relative_path: str          # Required - must be a file
max_answer_chars: int = -1
```

**Return**: `str` - JSON object containing symbols

**Business Logic**:
1. Create JetBrains plugin client
2. Call `client.get_symbols_overview()`
3. Convert response to JSON
4. Apply length limiting

---

## Common Patterns

### 1. Error Handling

**Standard Pattern**:
```python
try:
    # Tool logic
    return result
except SpecificException as e:
    # Handled in apply_ex wrapper
    raise
```

**Wrapper Handling** (`apply_ex` in `Tool` base class):
- Catches all exceptions if `catch_exceptions=True`
- Logs error with full traceback
- Returns error message: `"Error executing tool: ExceptionType - message"`
- LSP server termination: Auto-restart and retry once

### 2. Path Validation

**Standard Pattern**:
```python
self.project.validate_relative_path(relative_path, require_not_ignored=True)
```

**Validation Rules**:
- Path must be relative to project root
- Path must exist
- If `require_not_ignored=True`: Path must not match ignore patterns (.gitignore, etc.)

**Error Messages**:
- Invalid path: Exception raised with details
- Ignored path: Exception with guidance

### 3. Response Length Limiting

**Standard Pattern**:
```python
return self._limit_length(result, max_answer_chars)
```

**Logic**:
```python
def _limit_length(self, result: str, max_answer_chars: int) -> str:
    if max_answer_chars == -1:
        max_answer_chars = self.agent.serena_config.default_max_tool_answer_chars
    if max_answer_chars <= 0:
        raise ValueError(f"Must be positive or -1, got: {max_answer_chars}")
    if len(result) > max_answer_chars:
        result = f"The answer is too long ({len(result)} characters). " \
                 + "Please try a more specific tool query or raise max_answer_chars."
    return result
```

**Default Values**:
- `-1` = use config default (`serena_config.default_max_tool_answer_chars`)
- Positive integer = specific limit

### 4. JSON Serialization

**Standard Pattern**:
```python
return self._to_json(data_structure)
```

**Implementation**:
```python
@staticmethod
def _to_json(x: Any) -> str:
    return json.dumps(x, ensure_ascii=False)
```

**Usage**:
- All JSON responses use this method
- `ensure_ascii=False` preserves Unicode characters

### 5. File Encoding

**Standard Pattern**:
```python
with open(file_path, 'r', encoding=self.project.project_config.encoding) as f:
    content = f.read()
```

**Default Encoding**: Project-specific (typically UTF-8)

### 6. Success Results

**Standard Pattern**:
```python
# For edit operations
return SUCCESS_RESULT  # "OK"

# For read operations
return self._to_json(result_data)

# For operations with custom messages
return f"File created: {relative_path}. Overwrote existing file."
```

### 7. Context Managers for Editing

**Standard Pattern**:
```python
with EditedFileContext(relative_path, self.agent) as context:
    original_content = context.get_original_content()
    # ... modify content ...
    context.set_updated_content(updated_content)
# File automatically written on successful exit
```

**Benefits**:
- Atomic file updates
- Automatic rollback on exception
- Proper encoding handling

### 8. Language Server Symbol Retrieval

**Standard Pattern**:
```python
symbol_retriever = self.create_language_server_symbol_retriever()
symbols = symbol_retriever.find(name_path_pattern, ...)
symbol_dicts = [_sanitize_symbol_dict(s.to_dict(...)) for s in symbols]
return self._to_json(symbol_dicts)
```

**Symbol Sanitization**:
```python
def _sanitize_symbol_dict(symbol_dict: dict[str, Any]) -> dict[str, Any]:
    symbol_dict = copy(symbol_dict)
    # Replace location with just relative_path
    s_relative_path = symbol_dict.get("location", {}).get("relative_path")
    if s_relative_path is not None:
        symbol_dict["relative_path"] = s_relative_path
    symbol_dict.pop("location", None)
    # Remove redundant name field (use name_path)
    symbol_dict.pop("name", None)
    return symbol_dict
```

### 9. Tool Usage Recording

**Automatic Pattern** (in `apply_ex`):
```python
# After successful tool execution
self.agent.record_tool_usage(kwargs, result, self)
```

**Purpose**:
- Track tool usage statistics
- Enable user query answering about what was done
- Support debugging and optimization

### 10. Cache Management

**Automatic Pattern** (in `apply_ex`):
```python
try:
    ls_manager = self.agent.get_language_server_manager()
    if ls_manager is not None:
        ls_manager.save_all_caches()
except Exception as e:
    log.error(f"Error saving language server cache: {e}")
```

**Purpose**:
- Persist LSP caches after every tool execution
- Improve performance on subsequent operations
- Gracefully handle cache save failures

---

## Tool Summary Table

| Tool Name | Category | Markers | Parameters | Return Type | Validation |
|-----------|----------|---------|------------|-------------|------------|
| `read_file` | File | None | relative_path, start_line, end_line, max_answer_chars | str (file content) | Path validation, line range |
| `create_text_file` | File | CanEdit | relative_path, content | str (success msg) | Path validation, parent dir check |
| `list_dir` | File | None | relative_path, recursive, skip_ignored_files, max_answer_chars | JSON (dirs/files) | Path existence, validation |
| `find_file` | File | None | file_mask, relative_path | JSON (files) | Path validation, fnmatch |
| `replace_content` | File | CanEdit | relative_path, needle, repl, mode, allow_multiple_occurrences | str (SUCCESS_RESULT) | Path validation, mode check, match count, ambiguity |
| `delete_lines` | File | CanEdit, Optional | relative_path, start_line, end_line | str (SUCCESS_RESULT) | Prior read requirement |
| `replace_lines` | File | CanEdit, Optional | relative_path, start_line, end_line, content | str (SUCCESS_RESULT) | Prior read requirement |
| `insert_at_line` | File | CanEdit, Optional | relative_path, line, content | str (SUCCESS_RESULT) | None |
| `search_for_pattern` | File | None | substring_pattern, context_lines_before/after, paths_include/exclude_glob, relative_path, restrict_search_to_code_files, max_answer_chars | JSON (file->matches) | Path existence, glob patterns |
| `restart_language_server` | Symbol | Optional | None | str (SUCCESS_RESULT) | User confirmation |
| `get_symbols_overview` | Symbol | SymbolicRead | relative_path, depth, max_answer_chars | JSON (symbols) | Path existence, file check |
| `find_symbol` | Symbol | SymbolicRead | name_path_pattern, depth, relative_path, include_body, include/exclude_kinds, substring_matching, max_answer_chars | JSON (symbols) | Kind filtering |
| `find_referencing_symbols` | Symbol | SymbolicRead | name_path, relative_path, include/exclude_kinds, max_answer_chars | JSON (references) | Kind filtering |
| `replace_symbol_body` | Symbol | SymbolicEdit | name_path, relative_path, body | str (SUCCESS_RESULT) | Body structure |
| `insert_after_symbol` | Symbol | SymbolicEdit | name_path, relative_path, body | str (SUCCESS_RESULT) | None |
| `insert_before_symbol` | Symbol | SymbolicEdit | name_path, relative_path, body | str (SUCCESS_RESULT) | None |
| `rename_symbol` | Symbol | SymbolicEdit | name_path, relative_path, new_name | str (status msg) | None |
| `write_memory` | Memory | None | memory_file_name, content, max_answer_chars | str (success msg) | Content length |
| `read_memory` | Memory | None | memory_file_name, max_answer_chars | str (content) | None |
| `list_memories` | Memory | None | None | JSON (filenames) | None |
| `delete_memory` | Memory | None | memory_file_name | str (success msg) | User request |
| `edit_memory` | Memory | None | memory_file_name, needle, repl, mode | str (SUCCESS_RESULT) | Same as replace_content |
| `activate_project` | Config | DoesNotRequireActiveProject | project | str (activation msg) | Project existence |
| `remove_project` | Config | DoesNotRequireActiveProject, Optional | project_name | str (success msg) | Project existence |
| `switch_modes` | Config | Optional | modes | str (mode info + active tools) | Mode validity |
| `get_current_config` | Config | None | None | str (config overview) | None |
| `execute_shell_command` | Command | CanEdit | command, cwd, capture_stderr, max_answer_chars | JSON (stdout/stderr/returncode) | cwd resolution, safety |
| `check_onboarding_performed` | Workflow | None | None | str (status + memories) | None |
| `onboarding` | Workflow | None | None | str (prompt) | Once per conversation |
| `think_about_collected_information` | Workflow | None | None | str (prompt) | None |
| `think_about_task_adherence` | Workflow | None | None | str (prompt) | None |
| `think_about_whether_you_are_done` | Workflow | None | None | str (prompt) | None |
| `summarize_changes` | Workflow | Optional | None | str (prompt) | After think_about_done |
| `prepare_for_new_conversation` | Workflow | None | None | str (prompt) | User request only |
| `initial_instructions` | Workflow | DoesNotRequireActiveProject | None | str (system prompt) | None |
| `jet_brains_find_symbol` | JetBrains | SymbolicRead, Optional | name_path_pattern, depth, relative_path, include_body, search_deps, max_answer_chars | JSON (symbols) | None |
| `jet_brains_find_referencing_symbols` | JetBrains | SymbolicRead, Optional | name_path, relative_path, max_answer_chars | JSON (references) | None |
| `jet_brains_get_symbols_overview` | JetBrains | SymbolicRead, Optional | relative_path, max_answer_chars | JSON (symbols) | None |

---

## Testing Checklist for Rust Equivalence

### API Contract Matching
- [ ] Tool names match exactly (snake_case conversion)
- [ ] Parameter names match exactly
- [ ] Parameter types match (str, int, bool, list, dict, Literal)
- [ ] Default values match exactly
- [ ] Return types match (str, JSON strings)

### Validation Logic Matching
- [ ] Path validation identical (relative paths, existence, ignore patterns)
- [ ] Parameter validation identical (mode literals, kind filters, line ranges)
- [ ] Error messages identical (or functionally equivalent)
- [ ] Edge case handling identical (empty results, missing files, etc.)

### Business Logic Matching
- [ ] File operations identical (encoding, line slicing, newline handling)
- [ ] Regex operations identical (DOTALL, MULTILINE, backreference syntax $!N)
- [ ] Symbol operations identical (name path patterns, depth, sanitization)
- [ ] Memory operations identical (storage location, encoding)
- [ ] Shell command operations identical (cwd resolution, stderr capture)

### Common Patterns Matching
- [ ] Response length limiting identical
- [ ] JSON serialization identical (ensure_ascii=False)
- [ ] Success result constant identical ("OK")
- [ ] Context manager pattern for file edits
- [ ] Symbol sanitization logic identical

### Integration Points
- [ ] Language server integration compatible
- [ ] Code editor integration compatible
- [ ] Memory manager integration compatible
- [ ] Project/config system integration compatible

### Error Handling
- [ ] Exception types match or are functionally equivalent
- [ ] Error messages identical or functionally equivalent
- [ ] Recovery mechanisms identical (LSP restart, etc.)
- [ ] Logging behavior compatible

---

## Notes for Rust Implementation

### Key Differences to Handle

1. **Type System**:
   - Python's `str | None` → Rust's `Option<String>`
   - Python's `list[int]` → Rust's `Vec<i32>`
   - Python's `Literal["literal", "regex"]` → Rust's `enum Mode { Literal, Regex }`

2. **Default Values**:
   - Python's mutable default `[]` → Rust's `Option::None` or `vec![]`
   - Python's `-1` sentinel → Rust's `Option<i32>` or `-1i32`

3. **Error Handling**:
   - Python's exceptions → Rust's `Result<String, ToolError>`
   - Python's `raise ValueError` → Rust's `Err(ToolError::InvalidParameter)`

4. **JSON Serialization**:
   - Python's `json.dumps(x, ensure_ascii=False)` → Rust's `serde_json::to_string(x)`
   - Ensure Unicode handling matches

5. **Regex**:
   - Python's `re.DOTALL | re.MULTILINE` → Rust's `(?s)(?m)` inline flags or `RegexBuilder`
   - Backreference syntax `$!1` needs custom implementation (not standard in Rust regex)

6. **Context Managers**:
   - Python's `with` statement → Rust's `Drop` trait or explicit `close()` calls

7. **LSP Integration**:
   - Ensure symbol retrieval, editing, and caching work identically
   - Match symbol sanitization logic exactly

8. **Memory Management**:
   - Path resolution (`.serena/memories/` directory)
   - UTF-8 encoding enforcement
   - File naming conventions

### Testing Strategy

1. **Unit Tests**: Test each tool in isolation with same inputs as Python tests
2. **Integration Tests**: Test tool chains (e.g., find_symbol → replace_symbol_body)
3. **Snapshot Tests**: Compare JSON outputs byte-for-byte with Python outputs
4. **Error Path Tests**: Verify error messages and exception handling match
5. **Edge Case Tests**: Empty files, large files, Unicode, special characters, etc.

### Performance Considerations

- Rust should match or exceed Python performance
- LSP caching behavior should be identical
- File I/O should use same buffering strategies
- Regex compilation should be cached where Python caches

---

## Appendix: Full Parameter Reference

### Common Parameters Across Tools

| Parameter | Type | Default | Used By | Description |
|-----------|------|---------|---------|-------------|
| `relative_path` | str | Required | Most file/symbol tools | Path relative to project root |
| `max_answer_chars` | int | -1 | Most tools | Response length limit (-1 = config default) |
| `depth` | int | 0 | Symbol overview/find tools | Descendant retrieval depth |
| `include_body` | bool | False | Symbol find tools | Include symbol source code |
| `name_path` | str | Required | Symbol edit tools | Symbol identifier within file |
| `name_path_pattern` | str | Required | Symbol find tools | Pattern for symbol matching |
| `body` | str | Required | Symbol edit tools | Content to insert/replace |
| `content` | str | Required | File create/memory tools | Content to write |
| `mode` | Literal["literal", "regex"] | Required | Replace content/memory tools | Matching mode |
| `needle` | str | Required | Replace content/memory tools | Search pattern |
| `repl` | str | Required | Replace content/memory tools | Replacement text |

### Validation Patterns

| Pattern | Implementation | Used By |
|---------|----------------|---------|
| Path validation | `project.validate_relative_path(path, require_not_ignored=True)` | Most file/symbol tools |
| Path existence | `if not os.path.exists(abs_path): raise FileNotFoundError` | search_for_pattern |
| File vs directory | `if os.path.isdir(path): raise ValueError` | get_symbols_overview |
| Mode validation | `if mode not in ["literal", "regex"]: raise ValueError` | replace_content |
| Match count | `if n == 0: raise ValueError` / `if n > 1 and not allow_multiple: raise ValueError` | replace_content |
| Content length | `if len(content) > max_chars: raise ValueError` | write_memory |

---

**End of Document**
