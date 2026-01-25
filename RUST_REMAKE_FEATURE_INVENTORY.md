# Serena Python → Rust Feature Inventory
## Complete Feature List for 1:1 Parity

**Version:** Based on serena-agent 0.1.4
**Date:** 2025-01-XX
**Purpose:** Comprehensive feature checklist for Rust remake to achieve complete parity with Python implementation

---

## 1. Tool System (`src/serena/tools/`)

### 1.1 File Tools (`file_tools.py`)

#### ReadFileTool
- **Name:** `read_file`
- **Parameters:**
  - `relative_path: str` - Path to file relative to project root
  - `start_line: int = 0` - 0-based index of first line to retrieve
  - `end_line: int | None = None` - 0-based index of last line (inclusive)
  - `max_answer_chars: int = -1` - Max characters to return (-1 for default)
- **Features:**
  - Read full file or specific line range
  - Path validation against project ignored paths
  - Character limit enforcement
  - Encoding support from project config

#### CreateTextFileTool
- **Name:** `create_text_file`
- **Parameters:**
  - `relative_path: str`
  - `content: str`
- **Features:**
  - Create new files
  - Overwrite existing files (with notification)
  - Automatic parent directory creation
  - Path validation (must be within project root)
  - Encoding support

#### ListDirTool
- **Name:** `list_dir`
- **Parameters:**
  - `relative_path: str`
  - `recursive: bool`
  - `skip_ignored_files: bool = False`
  - `max_answer_chars: int = -1`
- **Features:**
  - List files and directories
  - Recursive directory scanning
  - Gitignore integration
  - JSON output format: `{"dirs": [...], "files": [...]}`
  - Relative path resolution
  - Error handling for non-existent paths

#### FindFileTool
- **Name:** `find_file`
- **Parameters:**
  - `file_mask: str` - Wildcards (* or ?)
  - `relative_path: str`
- **Features:**
  - Filename pattern matching (fnmatch)
  - Recursive search
  - Gitignore filtering
  - JSON output: `{"files": [...]}`

#### ReplaceContentTool
- **Name:** `replace_content`
- **Parameters:**
  - `relative_path: str`
  - `needle: str` - Search pattern
  - `repl: str` - Replacement string
  - `mode: Literal["literal", "regex"]`
  - `allow_multiple_occurrences: bool = False`
- **Features:**
  - Literal string replacement
  - Regex replacement with DOTALL and MULTILINE flags
  - Backreference support: `$!1`, `$!2`, etc.
  - Ambiguity detection for multi-line regex matches
  - Single occurrence enforcement (optional)
  - Atomic file updates via EditedFileContext
  - Error messages with occurrence counts

#### DeleteLinesTool (Optional)
- **Name:** `delete_lines`
- **Markers:** `ToolMarkerCanEdit`, `ToolMarkerOptional`
- **Parameters:**
  - `relative_path: str`
  - `start_line: int` - 0-based
  - `end_line: int` - 0-based
- **Features:**
  - Range-based line deletion
  - Requires prior read operation verification
  - Works with both LanguageServer and JetBrains backends

#### ReplaceLinesTool (Optional)
- **Name:** `replace_lines`
- **Markers:** `ToolMarkerCanEdit`, `ToolMarkerOptional`
- **Parameters:**
  - `relative_path: str`
  - `start_line: int`
  - `end_line: int`
  - `content: str`
- **Features:**
  - Combines delete + insert operations
  - Automatic newline handling
  - Requires prior read verification

#### InsertAtLineTool (Optional)
- **Name:** `insert_at_line`
- **Markers:** `ToolMarkerCanEdit`, `ToolMarkerOptional`
- **Parameters:**
  - `relative_path: str`
  - `line: int` - 0-based
  - `content: str`
- **Features:**
  - Insert content at specific line
  - Automatic newline appending
  - Pushes existing content down

#### SearchForPatternTool
- **Name:** `search_for_pattern`
- **Parameters:**
  - `substring_pattern: str` - Regex pattern
  - `context_lines_before: int = 0`
  - `context_lines_after: int = 0`
  - `paths_include_glob: str = ""` - e.g., "*.py", "src/**/*.ts"
  - `paths_exclude_glob: str = ""` - Takes precedence over include
  - `relative_path: str = ""` - Restrict search scope
  - `restrict_search_to_code_files: bool = False`
  - `max_answer_chars: int = -1`
- **Features:**
  - Regex search with DOTALL flag
  - Context line inclusion
  - Glob pattern filtering (supports `**`, `*`, `?`, `[seq]`, brace expansion `{a,b,c}`)
  - Code-only vs all-file search modes
  - File and directory path restrictions
  - JSON output: `{file_path: [match_strings, ...]}`

### 1.2 Symbol Tools (`symbol_tools.py`)

#### RestartLanguageServerTool (Optional)
- **Name:** `restart_language_server`
- **Markers:** `ToolMarkerOptional`
- **Features:**
  - Manual language server restart
  - Only on explicit user request

#### GetSymbolsOverviewTool
- **Name:** `get_symbols_overview`
- **Markers:** `ToolMarkerSymbolicRead`
- **Parameters:**
  - `relative_path: str` - Must be a file, not directory
  - `depth: int = 0` - Symbol tree depth
  - `max_answer_chars: int = -1`
- **Features:**
  - Top-level symbol retrieval
  - Hierarchical symbol tree with depth control
  - JSON output with sanitized symbol dictionaries
  - File existence and type validation

#### FindSymbolTool
- **Name:** `find_symbol`
- **Markers:** `ToolMarkerSymbolicRead`
- **Parameters:**
  - `name_path_pattern: str` - e.g., "MyClass/my_method", "/absolute/path", "method[0]"
  - `depth: int = 0`
  - `relative_path: str = ""` - File or directory restriction
  - `include_body: bool = False`
  - `include_kinds: list[int] = []` - LSP SymbolKind integers
  - `exclude_kinds: list[int] = []` - Takes precedence
  - `substring_matching: bool = False`
  - `max_answer_chars: int = -1`
- **Features:**
  - Name path matching (simple, relative, absolute)
  - Overload disambiguation with `[index]` syntax
  - Symbol kind filtering (26 LSP kinds supported)
  - Substring matching for last path segment
  - Scope restriction by file/directory
  - Symbol body inclusion
  - Depth-based child retrieval
  - JSON output with sanitized symbols

**LSP Symbol Kinds (26 types):**
```
1=file, 2=module, 3=namespace, 4=package, 5=class, 6=method, 7=property,
8=field, 9=constructor, 10=enum, 11=interface, 12=function, 13=variable,
14=constant, 15=string, 16=number, 17=boolean, 18=array, 19=object,
20=key, 21=null, 22=enum member, 23=struct, 24=event, 25=operator,
26=type parameter
```

#### FindReferencingSymbolsTool
- **Name:** `find_referencing_symbols`
- **Markers:** `ToolMarkerSymbolicRead`
- **Parameters:**
  - `name_path: str`
  - `relative_path: str` - Must be file, not directory
  - `include_kinds: list[int] = []`
  - `exclude_kinds: list[int] = []`
  - `max_answer_chars: int = -1`
- **Features:**
  - Find all references to a symbol
  - Symbol kind filtering
  - Code snippet context (±1 line around reference)
  - JSON output with reference locations and context
  - Never includes full body of referencing symbols

#### ReplaceSymbolBodyTool
- **Name:** `replace_symbol_body`
- **Markers:** `ToolMarkerSymbolicEdit`
- **Parameters:**
  - `name_path: str`
  - `relative_path: str`
  - `body: str` - New symbol body (no docstrings/imports)
- **Features:**
  - Replace complete symbol definition
  - Automatic whitespace stripping
  - Works with CodeEditor abstraction (LS or JetBrains)

#### InsertAfterSymbolTool
- **Name:** `insert_after_symbol`
- **Markers:** `ToolMarkerSymbolicEdit`
- **Parameters:**
  - `name_path: str`
  - `relative_path: str`
  - `body: str`
- **Features:**
  - Insert content after symbol definition
  - Automatic newline handling
  - Language-aware empty line insertion
  - Typical use: Add methods, fields, classes

#### InsertBeforeSymbolTool
- **Name:** `insert_before_symbol`
- **Markers:** `ToolMarkerSymbolicEdit`
- **Parameters:**
  - `name_path: str`
  - `relative_path: str`
  - `body: str`
- **Features:**
  - Insert content before symbol definition
  - Typical use: Add imports, new symbols

#### RenameSymbolTool
- **Name:** `rename_symbol`
- **Markers:** `ToolMarkerSymbolicEdit`
- **Parameters:**
  - `name_path: str` - May include overload index for disambiguation
  - `relative_path: str`
  - `new_name: str`
- **Features:**
  - Language server refactoring capabilities
  - Codebase-wide renaming
  - Workspace edit application
  - Status message reporting

### 1.3 Memory Tools (`memory_tools.py`)

#### WriteMemoryTool
- **Name:** `write_memory`
- **Parameters:**
  - `memory_file_name: str` - Markdown filename (auto .md extension)
  - `content: str` - UTF-8 encoded markdown
  - `max_answer_chars: int = -1`
- **Features:**
  - Project-specific memory storage in `.serena/memories/`
  - Automatic .md extension handling
  - Content length validation
  - UTF-8 encoding

#### ReadMemoryTool
- **Name:** `read_memory`
- **Parameters:**
  - `memory_file_name: str`
  - `max_answer_chars: int = -1`
- **Features:**
  - Read project-specific memories
  - Error message for non-existent memories

#### ListMemoriesTool
- **Name:** `list_memories`
- **Features:**
  - List all memory files
  - JSON output: `["memory1", "memory2", ...]`
  - Strip .md extensions from names

#### DeleteMemoryTool
- **Name:** `delete_memory`
- **Parameters:**
  - `memory_file_name: str`
- **Features:**
  - Delete memory files
  - Only on explicit user request

#### EditMemoryTool
- **Name:** `edit_memory`
- **Parameters:**
  - `memory_file_name: str`
  - `needle: str`
  - `repl: str`
  - `mode: Literal["literal", "regex"]`
- **Features:**
  - Uses ReplaceContentTool internally
  - Memory file path resolution
  - Bypasses ignore validation

### 1.4 Configuration Tools (`config_tools.py`)

#### ActivateProjectTool
- **Name:** `activate_project`
- **Markers:** `ToolMarkerDoesNotRequireActiveProject`
- **Parameters:**
  - `project: str` - Project name or path
- **Features:**
  - Project activation by name or path
  - Activation message with project details
  - Reminder to read Serena Instructions Manual

#### RemoveProjectTool (Optional)
- **Name:** `remove_project`
- **Markers:** `ToolMarkerDoesNotRequireActiveProject`, `ToolMarkerOptional`
- **Parameters:**
  - `project_name: str`
- **Features:**
  - Remove project from configuration
  - Update serena_config.yml

#### SwitchModesTool (Optional)
- **Name:** `switch_modes`
- **Markers:** `ToolMarkerOptional`
- **Parameters:**
  - `modes: list[str]` - e.g., ["editing", "interactive"]
- **Features:**
  - Dynamic mode switching
  - Mode validation and loading
  - Tool set updates based on modes
  - Status message with active modes and tools

#### GetCurrentConfigTool
- **Name:** `get_current_config`
- **Features:**
  - Display current configuration overview
  - Active project, tools, contexts, modes
  - Available projects and tools

### 1.5 Workflow Tools (`workflow_tools.py`)

#### CheckOnboardingPerformedTool
- **Name:** `check_onboarding_performed`
- **Features:**
  - Check for existing memories
  - Guide to perform onboarding if needed

#### OnboardingTool
- **Name:** `onboarding`
- **Features:**
  - Project structure identification
  - Platform-specific instructions (via prompt_factory)
  - One-time per conversation

#### ThinkAboutCollectedInformationTool
- **Name:** `think_about_collected_information`
- **Features:**
  - Metacognitive prompt for information completeness
  - Called after search sequences

#### ThinkAboutTaskAdherenceTool
- **Name:** `think_about_task_adherence`
- **Features:**
  - Metacognitive prompt for task alignment
  - Called before code modifications

#### ThinkAboutWhetherYouAreDoneTool
- **Name:** `think_about_whether_you_are_done`
- **Features:**
  - Completion verification prompt

#### SummarizeChangesTool (Optional)
- **Name:** `summarize_changes`
- **Markers:** `ToolMarkerOptional`
- **Features:**
  - Change summary instructions
  - Called after non-trivial tasks

#### PrepareForNewConversationTool
- **Name:** `prepare_for_new_conversation`
- **Features:**
  - Context continuation instructions
  - Only on explicit user request

#### InitialInstructionsTool
- **Name:** `initial_instructions`
- **Markers:** `ToolMarkerDoesNotRequireActiveProject`
- **Features:**
  - Serena Instructions Manual
  - System prompt generation
  - Critical for MCP clients that don't auto-read system prompts

### 1.6 Command Tools (`cmd_tools.py`)

#### ExecuteShellCommandTool
- **Name:** `execute_shell_command`
- **Markers:** `ToolMarkerCanEdit`
- **Parameters:**
  - `command: str`
  - `cwd: str | None = None` - Relative or absolute
  - `capture_stderr: bool = True`
  - `max_answer_chars: int = -1`
- **Features:**
  - Shell command execution
  - Working directory control (defaults to project root)
  - Stderr capture
  - JSON output: `{"stdout": "...", "stderr": "..."}`
  - Safety warnings (no long-running/interactive processes)

### 1.7 JetBrains Tools (`jetbrains_tools.py`) - All Optional

#### JetBrainsFindSymbolTool
- **Name:** `jetbrains_find_symbol`
- **Markers:** `ToolMarkerSymbolicRead`, `ToolMarkerOptional`
- **Parameters:** Similar to FindSymbolTool plus:
  - `search_deps: bool = False` - Search in project dependencies
- **Features:**
  - Uses JetBrainsPluginClient
  - Alternative to LSP-based symbol finding

#### JetBrainsFindReferencingSymbolsTool
- **Name:** `jetbrains_find_referencing_symbols`
- **Markers:** `ToolMarkerSymbolicRead`, `ToolMarkerOptional`

#### JetBrainsGetSymbolsOverviewTool
- **Name:** `jetbrains_get_symbols_overview`
- **Markers:** `ToolMarkerSymbolicRead`, `ToolMarkerOptional`

### 1.8 Tool Base Infrastructure (`tools_base.py`)

#### Core Classes

**Tool (Abstract Base)**
- Methods:
  - `get_name_from_cls() -> str` - Convert class name to snake_case
  - `get_name() -> str`
  - `get_apply_fn() -> ApplyMethodProtocol`
  - `can_edit() -> bool` - Check ToolMarkerCanEdit
  - `get_tool_description() -> str` - From class docstring
  - `get_apply_docstring() -> str` - From apply method docstring
  - `get_apply_fn_metadata() -> FuncMetadata` - For MCP tool generation
  - `apply_ex(**kwargs) -> str` - Main execution with logging/exception handling
  - `is_active() -> bool`
- Features:
  - Timeout enforcement (configurable, default 240s)
  - Language server crash recovery (automatic restart + retry)
  - Tool usage recording
  - Active project validation
  - Result length limiting
  - Cache saving after execution
  - TaskExecutor integration

**Component**
- Base class for tools
- Properties:
  - `get_project_root() -> str`
  - `prompt_factory: PromptFactory`
  - `memories_manager: MemoriesManager`
  - `project: Project`
- Methods:
  - `create_language_server_symbol_retriever()`
  - `create_code_editor()` - Factory for LS or JetBrains editors

**EditedFileContext**
- Context manager for atomic file edits
- Methods:
  - `get_original_content() -> str`
  - `set_updated_content(content: str)`
- Features:
  - Automatic write on successful exit
  - No write on exception
  - Encoding from project config

**ToolMarker Classes**
- `ToolMarker` - Base
- `ToolMarkerCanEdit` - Editing operations
- `ToolMarkerDoesNotRequireActiveProject`
- `ToolMarkerOptional` - Disabled by default
- `ToolMarkerSymbolicRead` - Symbol read operations
- `ToolMarkerSymbolicEdit` - Symbol edit operations

**ToolRegistry (Singleton)**
- Methods:
  - `get_tool_class_by_name(tool_name: str)`
  - `get_all_tool_classes()`
  - `get_tool_classes_default_enabled()`
  - `get_tool_classes_optional()`
  - `get_tool_names_default_enabled()`
  - `get_tool_names_optional()`
  - `get_tool_names()`
  - `print_tool_overview()`
  - `is_valid_tool_name(tool_name: str)`
- Features:
  - Automatic discovery via `iter_subclasses(Tool)`
  - Module filtering: `serena.tools.*`
  - Duplicate name detection

---

## 2. Language Server Support (`src/solidlsp/`)

### 2.1 Supported Languages (48 total, 41 stable + 7 experimental)

**Stable Languages (41):**

1. **Python** (`Language.PYTHON`)
   - Server: Pyright (default)
   - Files: `*.py`, `*.pyi`
   - Priority: 2

2. **Java** (`Language.JAVA`)
   - Server: Eclipse JDTLS
   - Files: `*.java`
   - Priority: 2

3. **Kotlin** (`Language.KOTLIN`)
   - Server: Kotlin Language Server
   - Files: `*.kt`, `*.kts`
   - Priority: 2

4. **Rust** (`Language.RUST`)
   - Server: rust-analyzer
   - Files: `*.rs`
   - Priority: 2

5. **C#** (`Language.CSHARP`)
   - Server: csharp-ls (default)
   - Files: `*.cs`
   - Priority: 2

6. **TypeScript/JavaScript** (`Language.TYPESCRIPT`)
   - Server: TypeScript Language Server
   - Files: `*.ts`, `*.tsx`, `*.js`, `*.jsx`, `*.mts`, `*.mjs`, `*.cts`, `*.cjs`
   - Priority: 2

7. **Go** (`Language.GO`)
   - Server: gopls
   - Files: `*.go`
   - Priority: 2

8. **Ruby** (`Language.RUBY`)
   - Server: ruby-lsp (default)
   - Files: `*.rb`, `*.erb`
   - Priority: 2

9. **Dart** (`Language.DART`)
   - Server: Dart Language Server
   - Files: `*.dart`
   - Priority: 2

10. **C/C++** (`Language.CPP`)
    - Server: clangd
    - Files: `*.cpp`, `*.h`, `*.hpp`, `*.c`, `*.hxx`, `*.cc`, `*.cxx`
    - Priority: 2

11. **PHP** (`Language.PHP`)
    - Server: Intelephense
    - Files: `*.php`
    - Priority: 2

12. **R** (`Language.R`)
    - Server: R Language Server
    - Files: `*.R`, `*.r`, `*.Rmd`, `*.Rnw`
    - Priority: 2

13. **Perl** (`Language.PERL`)
    - Server: Perl Language Server
    - Files: `*.pl`, `*.pm`, `*.t`
    - Priority: 2

14. **Clojure** (`Language.CLOJURE`)
    - Server: clojure-lsp
    - Files: `*.clj`, `*.cljs`, `*.cljc`, `*.edn`
    - Priority: 2

15. **Elixir** (`Language.ELIXIR`)
    - Server: ElixirLS
    - Files: `*.ex`, `*.exs`
    - Priority: 2

16. **Elm** (`Language.ELM`)
    - Server: Elm Language Server
    - Files: `*.elm`
    - Priority: 2

17. **Terraform** (`Language.TERRAFORM`)
    - Server: terraform-ls
    - Files: `*.tf`, `*.tfvars`, `*.tfstate`
    - Priority: 2

18. **Swift** (`Language.SWIFT`)
    - Server: SourceKit-LSP
    - Files: `*.swift`
    - Priority: 2

19. **Bash** (`Language.BASH`)
    - Server: bash-language-server
    - Files: `*.sh`, `*.bash`
    - Priority: 2

20. **Zig** (`Language.ZIG`)
    - Server: zls
    - Files: `*.zig`, `*.zon`
    - Priority: 2

21. **Lua** (`Language.LUA`)
    - Server: lua-ls
    - Files: `*.lua`
    - Priority: 2

22. **Nix** (`Language.NIX`)
    - Server: nixd
    - Files: `*.nix`
    - Priority: 2

23. **Erlang** (`Language.ERLANG`)
    - Server: Erlang Language Server
    - Files: `*.erl`, `*.hrl`, `*.escript`, `*.config`, `*.app`, `*.app.src`
    - Priority: 2

24. **AL** (`Language.AL`)
    - Server: AL Language Server
    - Files: `*.al`, `*.dal`
    - Priority: 2

25. **F#** (`Language.FSHARP`)
    - Server: F# Language Server
    - Files: `*.fs`, `*.fsx`, `*.fsi`
    - Priority: 2

26. **Rego** (`Language.REGO`)
    - Server: Regal
    - Files: `*.rego`
    - Priority: 2

27. **Scala** (`Language.SCALA`)
    - Server: Metals
    - Files: `*.scala`, `*.sbt`
    - Priority: 2

28. **Julia** (`Language.JULIA`)
    - Server: Julia Language Server
    - Files: `*.jl`
    - Priority: 2

29. **Fortran** (`Language.FORTRAN`)
    - Server: fortls
    - Files: `*.f90`, `*.F90`, `*.f95`, `*.F95`, `*.f03`, `*.F03`, `*.f08`, `*.F08`, `*.f`, `*.F`, `*.for`, `*.FOR`, `*.fpp`, `*.FPP`
    - Priority: 2

30. **Haskell** (`Language.HASKELL`)
    - Server: Haskell Language Server
    - Files: `*.hs`, `*.lhs`
    - Priority: 2

31. **Vue** (`Language.VUE`)
    - Server: Vue Language Server
    - Files: `*.vue` + TypeScript/JavaScript files
    - Priority: 1 (lower due to superset nature)

32. **PowerShell** (`Language.POWERSHELL`)
    - Server: PowerShell Language Server
    - Files: `*.ps1`, `*.psm1`, `*.psd1`
    - Priority: 2

**Experimental Languages (7):**

33. **TypeScript VTS** (`Language.TYPESCRIPT_VTS`)
    - Server: vtsls
    - Experimental alternative to standard TypeScript LS

34. **Python Jedi** (`Language.PYTHON_JEDI`)
    - Server: Jedi
    - Experimental alternative to Pyright

35. **C# OmniSharp** (`Language.CSHARP_OMNISHARP`)
    - Server: OmniSharp
    - Experimental (less stable than csharp-ls)

36. **Ruby Solargraph** (`Language.RUBY_SOLARGRAPH`)
    - Server: Solargraph
    - Legacy experimental

37. **Markdown** (`Language.MARKDOWN`)
    - Server: Marksman
    - Files: `*.md`, `*.markdown`
    - Must be explicitly specified

38. **YAML** (`Language.YAML`)
    - Server: YAML Language Server
    - Files: `*.yaml`, `*.yml`
    - Must be explicitly specified

39. **TOML** (`Language.TOML`)
    - Server: Taplo
    - Files: `*.toml`
    - Must be explicitly specified

40. **Groovy** (`Language.GROOVY`)
    - Server: Groovy Language Server
    - Files: `*.groovy`, `*.gvy`
    - Experimental

### 2.2 Language Server Architecture

**SolidLanguageServer (Abstract Base)**

Core capabilities:
- LSP lifecycle management (initialize, shutdown)
- Document synchronization (textDocument/didOpen, didChange, didClose)
- Symbol operations:
  - `textDocument/documentSymbol` - Get symbol tree
  - `textDocument/definition` - Go to definition
  - `textDocument/references` - Find references
  - `textDocument/rename` - Rename symbol
- Workspace edits
- File buffer management (LSPFileBuffer)
- Caching system (2-tier):
  - Raw document symbols cache (pickle, versioned)
  - High-level document symbols cache
- Cache invalidation on file changes
- Automatic language server restart on crash
- Process management (stdio, TCP, independent process)
- Configuration via LanguageServerConfig

**Key Methods:**
- `get_document_symbols(uri: str) -> DocumentSymbols`
- `rename_symbol(uri: str, position: Position, new_name: str) -> WorkspaceEdit`
- `get_definition(uri: str, position: Position) -> Definition`
- `get_references(uri: str, position: Position) -> list[Location]`
- `open_file(file_path: str, content: str) -> LSPFileBuffer`
- `close_file(uri: str)`
- `save_all_caches()`
- `restart_language_server()`

**Caching Strategy:**
- Cache folder: `.serena/cache/`
- Raw symbols: `raw_document_symbols.pkl` (versioned)
- High-level symbols: `document_symbols.pkl`
- Content hash-based invalidation
- Per-language server versioning

**Language Detection:**
- Automatic based on file extensions
- Priority system (2 = regular, 1 = superset, 0 = experimental)
- Multi-language project support
- Explicit override in project.yml

### 2.3 LSP Protocol Handler

**LSPTypes Supported:**
- DocumentSymbol (hierarchical)
- SymbolInformation (flat)
- UnifiedSymbolInformation (Serena's abstraction)
- Position, Range, Location, LocationLink
- TextEdit, WorkspaceEdit
- SymbolKind (26 types)
- Definition, DefinitionParams
- RenameParams

**Server Communication:**
- JSON-RPC 2.0
- stdio transport (default)
- TCP transport (optional)
- Independent process mode
- Request/response tracking
- Error handling (LSPError)
- Timeout support

---

## 3. Configuration System (`src/serena/config/`)

### 3.1 Contexts (`context_mode.py`)

**SerenaAgentContext**
- Fields:
  - `name: str`
  - `prompt: str` - Jinja2 template
  - `description: str`
  - `tool_description_overrides: dict[str, str]`
  - `single_project: bool = False`
  - `excluded_tools: Iterable[str]`
  - `included_optional_tools: Iterable[str]`
- Methods:
  - `from_yaml(yaml_path: str)`
  - `from_name(name: str)`
  - `load(name_or_path: str)`
  - `list_registered_context_names()`
  - `list_custom_context_names()`
  - `load_default()` - Returns "agent" context
- Features:
  - User contexts override built-in contexts
  - Context path: `~/.serena/contexts/` (user), `src/serena/contexts/` (built-in)
  - Jinja2 prompt templating
  - Tool customization per context
  - Legacy name mapping: `ide-assistant` → `claude-code`

**Built-in Contexts:**
- `agent` (default) - Full-featured MCP context
- `claude-code` - Optimized for Claude Code integration
- `desktop-app` - GUI application context

### 3.2 Modes (`context_mode.py`)

**SerenaAgentMode**
- Fields:
  - `name: str`
  - `prompt: str` - Jinja2 template
  - `description: str`
  - `excluded_tools: Iterable[str]`
  - `included_optional_tools: Iterable[str]`
- Methods:
  - `from_yaml(yaml_path: str)`
  - `from_name(name: str)`
  - `from_name_internal(name: str)` - Internal modes
  - `load(name_or_path: str)`
  - `list_registered_mode_names()`
  - `list_custom_mode_names()`
  - `load_default_modes()` - Returns ["interactive", "editing"]
- Features:
  - Multiple simultaneous modes
  - Dynamic mode switching via SwitchModesTool
  - User modes override built-in modes
  - Mode path: `~/.serena/modes/` (user), `src/serena/modes/` (built-in)
  - Internal modes for special purposes

**Built-in Modes:**
- `editing` (default) - Enable editing tools
- `interactive` (default) - Interactive workflow
- `planning` - Planning-focused, limited editing
- `one-shot` - Single-task execution

### 3.3 Project Configuration (`serena_config.py`)

**ProjectConfig**
- Fields:
  - `project_name: str`
  - `languages: list[Language]`
  - `ignored_paths: list[str] = []`
  - `read_only: bool = False`
  - `ignore_all_files_in_gitignore: bool = True`
  - `initial_prompt: str = ""`
  - `encoding: str = "utf-8"`
  - `excluded_tools: Iterable[str] = ()`
  - `included_optional_tools: Iterable[str] = ()`
- Methods:
  - `autogenerate(project_root, project_name, languages)`
  - `load(project_root, autogenerate=True)`
  - `load_commented_map(file_path)` - Preserve YAML comments
  - `save(file_path)`
  - `to_yaml_dict()`
  - `rel_path_to_project_yml() -> str` - Default: ".serena/project.yml"
- Features:
  - Automatic language detection via file analysis
  - Template-based project creation
  - Comment preservation on save (ruamel.yaml)
  - Per-project tool customization
  - Gitignore integration
  - Custom encoding support

**File Location Options:**
1. `.serena/project.yml` (default, recommended)
2. `project.yml` (project root)

### 3.4 Serena Configuration (`serena_config.py`)

**SerenaConfig**
- Fields:
  - `projects: dict[str, ProjectConfig]` - Registered projects
  - `log_level: int = logging.INFO`
  - `default_max_tool_answer_chars: int = 100000`
  - `tool_timeout: float = 240.0` - seconds
  - `gui_log_window_enabled: bool = False`
  - `language_backend: LanguageBackend = LanguageBackend.LSP`
  - `default_context: str = "agent"`
  - `default_modes: list[str] = ["interactive", "editing"]`
  - `enable_token_usage_analytics: bool = False`
  - `token_count_estimator: RegisteredTokenCountEstimator = CHAR_COUNT`
- Methods:
  - `from_config_file(config_path=None)` - Load from `~/.serena/serena_config.yml`
  - `create_default_config_file()`
  - `add_project(project_config: ProjectConfig)`
  - `remove_project(project_name: str)`
  - `save()`
  - `get_project_config(project_name: str) -> ProjectConfig`
- Features:
  - User home directory: `~/.serena/` (or `$SERENA_HOME`)
  - Template-based config creation
  - Project registration
  - Global default settings

**SerenaPaths (Singleton)**
- Properties:
  - `serena_user_home_dir: str` - `~/.serena/` or `$SERENA_HOME`
  - `user_prompt_templates_dir: str` - Custom prompts
  - `user_contexts_dir: str` - Custom contexts
  - `user_modes_dir: str` - Custom modes
- Methods:
  - `get_next_log_file_path(prefix: str) -> str` - Dated log files

### 3.5 Tool Inclusion System

**ToolInclusionDefinition**
- Fields:
  - `excluded_tools: Iterable[str] = ()`
  - `included_optional_tools: Iterable[str] = ()`
- Used by: ProjectConfig, SerenaAgentMode, SerenaAgentContext
- Features:
  - Hierarchical tool filtering
  - Legacy tool name mapping

**ToolSet**
- Methods:
  - `default()` - All default-enabled tools
  - `apply(*ToolInclusionDefinition) -> ToolSet`
  - `without_editing_tools() -> ToolSet`
  - `get_tool_names() -> set[str]`
  - `includes_name(tool_name: str) -> bool`
- Features:
  - Tool name validation
  - Read-only mode support
  - Context/mode/project tool customization

---

## 4. Memory & Knowledge System (`src/serena/project.py`)

### 4.1 MemoriesManager

**Storage:**
- Location: `.serena/memories/` (per-project)
- Format: Markdown (`.md`)
- Encoding: UTF-8

**Methods:**
- `load_memory(name: str) -> str`
- `save_memory(name: str, content: str) -> str`
- `list_memories() -> list[str]`
- `delete_memory(name: str) -> str`
- `get_memory_file_path(name: str) -> Path`

**Features:**
- Automatic `.md` extension handling
- Error messages for missing memories
- Integration with memory tools

### 4.2 Project

**Core Responsibilities:**
1. Project configuration management
2. File path validation
3. Gitignore integration
4. Language server lifecycle
5. Memory management
6. File I/O with encoding

**Key Methods:**

**Lifecycle:**
- `load(project_root, autogenerate=True) -> Project`
- `save_config()`

**Path Operations:**
- `validate_relative_path(relative_path, require_not_ignored=True)`
- `relative_path_exists(relative_path: str) -> bool`
- `is_ignored_path(abs_path: str) -> bool`
- `path_to_serena_data_folder() -> str`
- `path_to_project_yml() -> str`

**File Operations:**
- `read_file(relative_path: str) -> str`
- `retrieve_content_around_line(relative_file_path, line, context_lines_before, context_lines_after)`
- `search_source_files_for_pattern(pattern, relative_path, ...)`

**Metadata:**
- `get_activation_message() -> str` - Languages, encoding, memories, initial prompt
- `project_name: str` - Property
- `project_root: str`
- `project_config: ProjectConfig`
- `memories_manager: MemoriesManager`
- `language_server_manager: LanguageServerManager | None`

**Ignore System:**
- PathSpec-based matching (gitignore-compatible)
- Per-project ignore cache
- Explicit patterns + gitignore integration
- Source file filtering by language

**Features:**
- Automatic `.gitignore` creation in `.serena/`
- New project detection flag
- Encoding from config
- Rust core acceleration (optional, via `serena_core`)

---

## 5. MCP Server Interface (`src/serena/mcp.py`)

### 5.1 SerenaMCPFactory

**Initialization:**
- Parameters:
  - `context: str = "agent"` - Context name or path
  - `project: str | None = None` - Project path or name

**MCP Server Creation:**
- Based on FastMCP (mcp v1.23.0)
- Tool registration from SerenaAgent
- Settings class integration
- OpenAI tool schema compatibility mode

**Features:**
- Automatic tool discovery
- Docstring-based tool descriptions
- Pydantic schema generation
- JSON schema sanitization for OpenAI
- Request context with agent instance
- Async/await support

### 5.2 Tool Registration

**Process:**
1. Collect active tools from agent
2. Extract apply method metadata:
   - Docstring parsing (docstring_parser)
   - Parameter types (Pydantic validation)
   - Return type
3. Convert to MCP tool format
4. Apply context-specific overrides
5. Register with FastMCP

**Schema Sanitization:**
- `integer` → `number` (+ multipleOf: 1)
- Remove `null` from union types
- Simplify oneOf/anyOf
- Coerce integer-only enums
- OpenAI compatibility mode (optional)

### 5.3 Request Handling

**Execution Flow:**
1. MCP client sends tool call request
2. FastMCP routes to tool
3. Tool.apply_ex() execution:
   - Parameter validation
   - Logging
   - Exception handling
   - Timeout enforcement
   - Language server crash recovery
4. Result serialization
5. Response to client

**Context Management:**
- SerenaMCPRequestContext with agent
- Project activation state
- Language server lifecycle
- Memory log handler

### 5.4 CLI Integration (`src/serena/cli.py`)

**Commands:**

**serena-mcp-server:**
- Starts MCP server
- Options:
  - `--context` / `-c` - Context name/path
  - `--project` / `-p` - Project path/name
  - `--modes` / `-m` - Mode names (repeatable)
  - `--openai-compatible` - OpenAI schema mode

**serena:**
- Main CLI entry point
- Subcommands:
  - `project` - Project management
  - `config` - Configuration management
  - `tools` - List available tools

**index-project (deprecated):**
- Pre-index project for faster tool performance
- Builds language server caches

---

## 6. Code Editor System (`src/serena/code_editor.py`)

### 6.1 CodeEditor (Abstract)

**Backends:**
1. **LanguageServerCodeEditor** - LSP-based (default)
2. **JetBrainsCodeEditor** - JetBrains plugin integration

**Core Operations:**

**Symbol-based Editing:**
- `replace_body(name_path, relative_file_path, body)`
- `insert_after_symbol(name_path, relative_file_path, body)`
- `insert_before_symbol(name_path, relative_file_path, body)`
- `rename_symbol(name_path, relative_file_path, new_name)`

**Line-based Editing:**
- `delete_lines(relative_path, start_line, end_line)`
- `insert_at_line(relative_path, line, content)`

**Context Managers:**
- `_open_file_context(relative_path) -> EditedFile`
- `_edited_file_context(relative_path) -> EditedFile`

**EditedFile Interface:**
- `get_contents() -> str`
- `delete_text_between_positions(start_pos, end_pos)`
- `insert_text_at_position(pos, text)`

### 6.2 LanguageServerCodeEditor

**Implementation:**
- Uses LanguageServerSymbolRetriever
- LSPFileBuffer management
- Text edit application
- Workspace edit handling
- Automatic file synchronization

**Newline Handling:**
- Leading newline count preservation
- Trailing newline enforcement
- Language-specific empty line rules

**Symbol Operations:**
- Name path resolution
- Overload disambiguation
- Position calculation
- Body boundary detection

### 6.3 JetBrainsCodeEditor

**Implementation:**
- Uses JetBrainsPluginClient
- HTTP communication with IDE plugin
- Real-time IDE integration
- Dependency search support

**Plugin Communication:**
- REST API (Flask server in plugin)
- Request/response cycle
- Timeout handling
- Connection management

### 6.4 Symbol System (`src/serena/symbol.py`)

**Symbol Abstractions:**

**LanguageServerSymbol:**
- Fields:
  - `name: str`
  - `name_path: str` - Hierarchical path
  - `kind: SymbolKind`
  - `location: LanguageServerSymbolLocation`
  - `body_location: BodyLocation | None`
  - `children: list[LanguageServerSymbol]`
  - `body: str | None`
- Methods:
  - `to_dict(kind, location, depth, include_body)`
  - `get_body_start_position() -> PositionInFile | None`
  - `get_body_end_position() -> PositionInFile | None`
  - `is_neighbouring_definition_separated_by_empty_line() -> bool`

**JetBrainsSymbol:**
- Similar interface to LanguageServerSymbol
- Different internal representation

**NamePathMatcher:**
- Pattern types:
  - Simple name: `"method"`
  - Relative path: `"class/method"`
  - Absolute path: `"/class/method"`
  - Overload index: `"class/method[0]"`
- Methods:
  - `matches(name_path: str, overload_idx: int | None) -> bool`
  - `substring_match(name_path: str) -> bool`

**LanguageServerSymbolRetriever:**
- Methods:
  - `get_symbol_overview(relative_path, depth) -> dict[str, list[dict]]`
  - `find(name_path_pattern, include_kinds, exclude_kinds, substring_matching, within_relative_path)`
  - `find_referencing_symbols(name_path, relative_file_path, include_body, include_kinds, exclude_kinds)`
  - `find_unique_symbol_in_file(name_path, relative_file_path)`
- Features:
  - Multi-file search
  - Kind filtering
  - Substring matching
  - Reference tracking
  - Cache utilization

---

## 7. Language Server Manager (`src/serena/ls_manager.py`)

### 7.1 LanguageServerManager

**Responsibilities:**
1. Multi-language server lifecycle
2. File-to-language mapping
3. Cache coordination
4. Crash recovery

**Key Methods:**
- `get_language_server(language: Language) -> SolidLanguageServer`
- `get_language_for_file(file_path: str) -> Language | None`
- `restart_language_server(language: Language)`
- `close_all()`
- `save_all_caches()`
- `get_symbol_overview(relative_path, depth) -> dict[str, list]`

**File Mapping Strategy:**
- Extension-based detection
- Priority-based disambiguation
- Multi-language project support
- Cache by file path

**Cache Management:**
- Per-language caching
- Coordinated save operations
- Invalidation on file changes
- Version-aware loading

### 7.2 LanguageServerFactory

**Responsibilities:**
- Language server instantiation
- Configuration passing
- Settings propagation

**Methods:**
- `create_language_server(language: Language, project_root: str, config: LanguageServerConfig) -> SolidLanguageServer`

**Settings:**
- Trace LSP communication
- Independent process mode
- Ignored paths
- Encoding

---

## 8. Analytics & Dashboard (`src/serena/analytics.py`, `src/serena/dashboard.py`)

### 8.1 Tool Usage Analytics

**ToolUsageStats:**
- Methods:
  - `record_usage(tool_name: str, input_tokens: int, output_tokens: int)`
  - `get_stats() -> dict[str, dict[str, int]]`
  - `get_total_tokens() -> int`
  - `reset()`
- Features:
  - Per-tool token counts
  - Total token tracking
  - Thread-safe

**Token Estimators:**

**TiktokenCountEstimator:**
- Model: gpt-4o (default)
- Method: tiktoken library
- Accuracy: High

**AnthropicTokenCount:**
- Model: claude-sonnet-4-20250514 (default)
- Method: Anthropic API
- Accuracy: Exact
- Requires: API key, rate limited

**CharCountEstimator:**
- Method: Character count / 4
- Accuracy: Low but fast

**RegisteredTokenCountEstimator (Enum):**
- `TIKTOKEN_GPT4O`
- `ANTHROPIC_CLAUDE_SONNET_4`
- `CHAR_COUNT`

### 8.2 Dashboard API (Flask)

**SerenaDashboardAPI:**
- Port: Auto-selected available port
- Features:
  - Real-time log streaming
  - Tool usage statistics
  - Configuration management
  - Project activation
  - Memory CRUD operations
  - Language management
  - Task execution monitoring

**Endpoints:**

**GET Endpoints:**
- `/` - Dashboard UI (static files)
- `/api/logs` - Real-time log messages (polling)
- `/api/tool-names` - Active tool names
- `/api/tool-stats` - Usage statistics
- `/api/config-overview` - Complete config
- `/api/available-languages` - Supported languages
- `/api/get-memory` - Single memory content
- `/api/get-serena-config` - Global config YAML

**POST Endpoints:**
- `/api/activate-project` - Project activation
- `/api/add-language` - Add language to project
- `/api/remove-language` - Remove language
- `/api/save-memory` - Save memory content
- `/api/delete-memory` - Delete memory
- `/api/save-serena-config` - Update global config
- `/api/cancel-task` - Cancel running task

**Dashboard UI:**
- Location: `src/serena/dashboard/` (static files)
- Framework: HTML/CSS/JavaScript
- Features:
  - Configuration visualization
  - Log viewer
  - Memory editor
  - Project switcher
  - Tool statistics

---

## 9. Utility Systems (`src/serena/util/`)

### 9.1 File System (`file_system.py`)

**GitignoreParser:**
- Methods:
  - `get_ignore_specs() -> list[IgnoreSpec]`
  - Parse .gitignore files recursively
- Features:
  - Multiple .gitignore support
  - Pathspec integration

**Functions:**
- `scan_directory(path, relative_to, recursive, is_ignored_dir, is_ignored_file)`
- `match_path(path, pattern)` - Glob matching

### 9.2 Shell (`shell.py`)

**execute_shell_command:**
- Parameters:
  - `command: str`
  - `cwd: str`
  - `capture_stderr: bool = True`
- Returns: JSON `{"stdout": "...", "stderr": "..."}`
- Features:
  - Subprocess management
  - Error handling
  - Encoding

### 9.3 Logging (`logging.py`)

**MemoryLogHandler:**
- Methods:
  - `get_messages(start_idx=0) -> list[str]`
  - `get_max_idx() -> int`
  - Thread-safe log collection
- Features:
  - In-memory log buffering
  - Dashboard integration
  - Level filtering

**Log Format:**
```
SERENA_LOG_FORMAT = "%(asctime)s - %(name)s - %(levelname)s - %(message)s"
```

### 9.4 Inspection (`inspection.py`)

**Functions:**
- `iter_subclasses(cls) -> Iterator[type]` - Recursive subclass discovery
- `determine_programming_language_composition(project_root) -> dict[Language, int]`
  - File counting by language
  - Language detection for autogeneration

### 9.5 Git (`git.py`)

**Functions:**
- Git operations support (if needed)

### 9.6 Exception (`exception.py`)

**Functions:**
- `show_fatal_exception_safe(exception: Exception)`
- Safe exception display

### 9.7 GUI (`gui.py`)

**Functions:**
- `system_has_usable_display() -> bool`
- Display detection (Linux, macOS, Windows)

### 9.8 CLI Util (`cli_util.py`)

**Functions:**
- `ask_yes_no(question: str) -> bool`
- Interactive prompts

### 9.9 Thread (`thread.py`)

**Functions:**
- Thread utility functions

---

## 10. Task Execution (`src/serena/task_executor.py`)

### 10.1 TaskExecutor

**Features:**
- Concurrent.futures thread pool
- Timeout enforcement
- Task cancellation
- Result retrieval
- Exception propagation

**Methods:**
- `submit(fn: Callable, name: str) -> TaskExecution`
- `shutdown(wait: bool = True)`

**TaskExecution:**
- Methods:
  - `result(timeout: float) -> T`
  - `cancel() -> bool`
  - `is_running() -> bool`
- Properties:
  - `task_id: int`
  - `name: str`

---

## 11. Prompt System (`src/serena/prompt_factory.py`)

### 11.1 PromptFactory

**Jinja2 Template Engine:**
- Template locations:
  1. User templates: `~/.serena/prompt_templates/`
  2. Built-in templates: `src/serena/prompt_templates/`
- User templates override built-in

**Prompt Methods:**
- `create_onboarding_prompt(system: str) -> str`
- `create_think_about_collected_information() -> str`
- `create_think_about_task_adherence() -> str`
- `create_think_about_whether_you_are_done() -> str`
- `create_summarize_changes() -> str`
- `create_prepare_for_new_conversation() -> str`
- `create_system_prompt() -> str` - Main system prompt

**Template Variables:**
- `system` - OS platform
- `project_name`
- `languages`
- `context`
- `modes`
- Custom context/mode variables

---

## 12. Agent System (`src/serena/agent.py`)

### 12.1 SerenaAgent

**Initialization:**
- Parameters:
  - `project: str | None` - Project path or name
  - `project_activation_callback: Callable | None`
  - `serena_config: SerenaConfig | None`
  - `context: SerenaAgentContext | None`
  - `modes: list[SerenaAgentMode] | None`
  - `memory_log_handler: MemoryLogHandler | None`

**Core Responsibilities:**
1. Tool registry and availability management
2. Project lifecycle (activation, deactivation)
3. Language server coordination
4. Configuration application (contexts, modes)
5. System prompt generation
6. Tool execution orchestration
7. Analytics collection

**Key Methods:**

**Project Management:**
- `activate_project_from_path_or_name(project: str) -> Project`
- `get_active_project() -> Project | None`
- `get_active_project_or_raise() -> Project`
- `get_project_root() -> str`

**Tool Management:**
- `get_tool(tool_class: type[TTool]) -> TTool`
- `get_active_tool_names() -> list[str]`
- `tool_is_active(tool_class: type[Tool]) -> bool`
- `record_tool_usage(inputs: dict, result: str, tool: Tool)`

**Language Server:**
- `get_language_server_manager() -> LanguageServerManager | None`
- `get_language_server_manager_or_raise() -> LanguageServerManager`
- `is_using_language_server() -> bool`
- `reset_language_server_manager()`

**Configuration:**
- `set_modes(modes: list[SerenaAgentMode])`
- `get_current_config_overview() -> str`
- `create_system_prompt() -> str`

**Task Execution:**
- `issue_task(fn: Callable, name: str) -> TaskExecution`

**Dashboard:**
- `start_dashboard() -> SerenaDashboardAPI | None`

### 12.2 AvailableTools

**Features:**
- Tool instance collection
- Tool name tracking
- Marker name tracking (for capability detection)

**Methods:**
- `__len__() -> int`

### 12.3 ToolSet

**Features:**
- Tool inclusion/exclusion logic
- Default tool set
- Context/mode/project filtering
- Read-only mode (remove editing tools)
- Legacy tool name mapping

**Methods:**
- `default() -> ToolSet`
- `apply(*ToolInclusionDefinition) -> ToolSet`
- `without_editing_tools() -> ToolSet`
- `get_tool_names() -> set[str]`
- `includes_name(tool_name: str) -> bool`

---

## 13. Text Utilities (`src/serena/text_utils.py`)

### 13.1 MatchedConsecutiveLines

**Fields:**
- `source_file_path: str | None`
- `lines: list[str]`
- `first_line_number: int`

**Methods:**
- `to_display_string() -> str`
- Format: `"[lines {first}-{last}] {joined_lines}"`

### 13.2 search_files

**Parameters:**
- `relative_paths: list[str]`
- `pattern: str` - Regex
- `file_reader: Callable[[str], str]`
- `root_path: str`
- `paths_include_glob: str`
- `paths_exclude_glob: str`

**Features:**
- Regex search across files
- Glob-based filtering
- Context line support
- Match grouping by file

---

## 14. Performance & Profiling (`src/serena/perf.py`)

### 14.1 Performance Monitoring

**Features:**
- Execution time tracking
- Memory profiling (optional via memray)
- CPU profiling (optional via pyinstrument)

---

## 15. GUI Log Viewer (`src/serena/gui_log_viewer.py`)

### 15.1 GuiLogViewer

**Features:**
- Real-time log display
- Tkinter-based UI
- Thread-safe updates
- Configurable via `gui_log_window_enabled`

**Platform Support:**
- macOS: Requires TkAgg backend
- Linux: X11 display check
- Windows: Native support

---

## 16. TUI (Terminal UI) - Optional (`src/serena/tui/`)

### 16.1 Textual-based TUI

**Command:** `serena-tui`

**Features:**
- Terminal-based dashboard
- Project management
- Log viewing
- Memory editing
- Configuration editor

**Dependency:**
- Optional: `pip install serena-agent[tui]`
- Textual >=0.50, <2

---

## 17. Optional Rust Core Acceleration

### 17.1 serena_core (PyO3)

**Location:** `src/serena_core/` (Rust)

**Features:**
- Accelerated file operations
- Pattern matching optimization
- Cross-platform path handling

**Build:**
- `maturin` build system
- Optional dependency
- Fallback to pure Python if unavailable

**Detection:**
```python
try:
    import serena_core
except Exception:
    serena_core = None  # Use pure Python
```

---

## 18. Testing Infrastructure

### 18.1 Test Organization

**Structure:**
```
test/
├── resources/
│   └── repos/
│       ├── python/
│       ├── java/
│       ├── typescript/
│       ├── rust/
│       └── ... (per language)
├── solidlsp/
│   ├── python/
│   ├── java/
│   └── ... (per language)
└── test_serena_agent.py
```

### 18.2 Test Markers

**Pytest Markers:**
- Language-specific: `python`, `go`, `java`, `rust`, `typescript`, `vue`, `php`, `perl`, `powershell`, `csharp`, `elixir`, `terraform`, `clojure`, `swift`, `bash`, `ruby`, `ruby_solargraph`
- Feature: `snapshot` - Symbolic editing operations

**Default Test Command:**
```bash
uv run poe test  # Excludes java/rust by default
```

**Selective Testing:**
```bash
uv run poe test -m "python or go"
uv run poe test -m vue
```

### 18.3 Test Repositories

**Per-Language Test Repos:**
- Small, representative codebases
- Symbol structure for testing
- File organization examples
- Edge cases

### 18.4 Snapshot Testing

**Tool:** syrupy
**Purpose:** Symbolic editing operation validation
**Usage:** Verify LSP text edits produce expected results

---

## 19. Development Tools & Quality

### 19.1 Code Quality Tools

**Formatting:**
- Black (Python, line-length: 100)
- Ruff (linting + auto-fix)

**Type Checking:**
- mypy (strict mode)
- Pyright (LSP integration)

**Testing:**
- pytest
- pytest-xdist (parallel)
- pytest-cov (coverage)
- pytest-timeout
- pytest-benchmark
- pytest-asyncio
- hypothesis (property-based)

**Poe Tasks:**
```bash
uv run poe format       # Black + Ruff
uv run poe type-check   # mypy
uv run poe test         # pytest with defaults
uv run poe lint         # Ruff check only
```

### 19.2 Documentation

**Tools:**
- Sphinx (API docs)
- sphinx_rtd_theme
- jupyter-book
- nbsphinx

**Build:**
```bash
uv run sphinx-build -b html docs docs/_build
```

---

## 20. Deployment & Distribution

### 20.1 Package Distribution

**PyPI:**
- Package name: `serena-agent`
- Version: 0.1.4
- License: MIT

**Scripts:**
- `serena` - CLI
- `serena-mcp-server` - MCP server
- `serena-tui` - Terminal UI (optional)
- `index-project` - Pre-indexing (deprecated)

### 20.2 Requirements

**Python:**
- Version: >=3.11, <3.12
- Required: Exact match for compatibility

**Core Dependencies:**
- mcp==1.23.0
- sensai-utils>=1.5.0
- pydantic>=2.10.6
- flask>=3.0.0
- pyright>=1.1.396
- pathspec>=0.12.1
- ruamel.yaml>=0.18.0
- jinja2>=3.1.6
- anthropic>=0.54.0
- tiktoken>=0.9.0

**Optional:**
- `[tui]`: textual>=0.50
- `[agno]`: agno>=2.2.1, sqlalchemy>=2.0.40
- `[google]`: google-genai>=1.8.0

---

## 21. Configuration Files Summary

### 21.1 User Configuration Files

**~/.serena/**
```
serena_config.yml          # Global Serena settings
contexts/
  custom_context.yml       # User-defined contexts
modes/
  custom_mode.yml          # User-defined modes
prompt_templates/
  custom_prompt.jinja2     # Custom prompt templates
logs/
  YYYY-MM-DD/
    serena_TIMESTAMP.txt   # Dated logs
```

### 21.2 Project Configuration Files

**.serena/**
```
project.yml                # Project configuration
memories/
  memory1.md              # Project memories
  memory2.md
cache/
  language_server_name/
    raw_document_symbols.pkl
    document_symbols.pkl
.gitignore                # Cache folder ignored
```

---

## 22. Error Handling & Recovery

### 22.1 Tool Execution Errors

**Handled by Tool.apply_ex():**
- TimeoutError (default 240s)
- SolidLSPException (language server crashes)
- ValueError (invalid parameters)
- FileNotFoundError
- Generic exceptions

**Recovery Strategies:**
1. Language server crash → Automatic restart + retry
2. Timeout → Return error message
3. Invalid path → Validation error
4. JSON serialization → Error message

### 22.2 Language Server Recovery

**Crash Detection:**
- SolidLSPException.is_language_server_terminated()

**Recovery:**
1. Detect affected language
2. Log restart message
3. `restart_language_server(language)`
4. Retry original operation

---

## 23. Internationalization & Encoding

### 23.1 Encoding Support

**File Encoding:**
- Default: UTF-8
- Configurable per project in `project.yml`
- Applied to:
  - Source file reading
  - Memory files
  - Configuration files

**Text Processing:**
- Newline normalization (CRLF → LF)
- Unicode support throughout

---

## 24. Security & Safety

### 24.1 Path Validation

**Checks:**
- Project root restriction (no parent directory traversal)
- Ignored path enforcement
- Gitignore integration
- Absolute path resolution

### 24.2 Shell Command Safety

**ExecuteShellCommandTool:**
- Warning: No long-running processes
- Warning: No interactive processes
- No automatic shell expansion prevention (user responsibility)

### 24.3 Credential Management

**Best Practices:**
- Environment variables for API keys
- python-dotenv integration
- No hardcoded credentials

---

## 25. Platform Support

### 25.1 Supported Platforms

**Operating Systems:**
- Linux (tested)
- macOS (tested)
- Windows (tested)

**Platform-Specific:**
- macOS: TkAgg backend for GUI
- Linux: X11 display check
- Windows: Native GUI support

### 25.2 Platform Detection

**Functions:**
- `platform.system()` - OS detection
- `system_has_usable_display()` - GUI availability

---

## 26. Extension Points

### 26.1 Custom Tools

**Process:**
1. Create subclass of `Tool`
2. Implement `apply()` method
3. Add to `src/serena/tools/`
4. Auto-discovered by ToolRegistry

### 26.2 Custom Language Servers

**Process:**
1. Create subclass of `SolidLanguageServer`
2. Implement abstract methods
3. Add to `Language` enum
4. Add to `get_ls_class()` match statement

### 26.3 Custom Contexts/Modes

**Process:**
1. Create YAML file in `~/.serena/contexts/` or `~/.serena/modes/`
2. Define prompt, description, tool inclusions/exclusions
3. Auto-discovered on next run

---

## Appendix A: Complete Tool Checklist

**File Tools (8):**
- [x] read_file
- [x] create_text_file
- [x] list_dir
- [x] find_file
- [x] replace_content
- [x] delete_lines (optional)
- [x] replace_lines (optional)
- [x] insert_at_line (optional)
- [x] search_for_pattern

**Symbol Tools (7):**
- [x] restart_language_server (optional)
- [x] get_symbols_overview
- [x] find_symbol
- [x] find_referencing_symbols
- [x] replace_symbol_body
- [x] insert_after_symbol
- [x] insert_before_symbol
- [x] rename_symbol

**Memory Tools (5):**
- [x] write_memory
- [x] read_memory
- [x] list_memories
- [x] delete_memory
- [x] edit_memory

**Config Tools (4):**
- [x] activate_project
- [x] remove_project (optional)
- [x] switch_modes (optional)
- [x] get_current_config

**Workflow Tools (7):**
- [x] check_onboarding_performed
- [x] onboarding
- [x] think_about_collected_information
- [x] think_about_task_adherence
- [x] think_about_whether_you_are_done
- [x] summarize_changes (optional)
- [x] prepare_for_new_conversation
- [x] initial_instructions

**Command Tools (1):**
- [x] execute_shell_command

**JetBrains Tools (3) - All optional:**
- [x] jetbrains_find_symbol
- [x] jetbrains_find_referencing_symbols
- [x] jetbrains_get_symbols_overview

**Total Tools:** 35 (27 default-enabled, 8 optional)

---

## Appendix B: Language Server Checklist

**Stable Languages (41):**
- [x] Python (Pyright)
- [x] Java (Eclipse JDTLS)
- [x] Kotlin
- [x] Rust (rust-analyzer)
- [x] C# (csharp-ls)
- [x] TypeScript/JavaScript
- [x] Go (gopls)
- [x] Ruby (ruby-lsp)
- [x] Dart
- [x] C/C++ (clangd)
- [x] PHP (Intelephense)
- [x] R
- [x] Perl
- [x] Clojure
- [x] Elixir
- [x] Elm
- [x] Terraform
- [x] Swift (SourceKit-LSP)
- [x] Bash
- [x] Zig
- [x] Lua
- [x] Nix
- [x] Erlang
- [x] AL
- [x] F#
- [x] Rego
- [x] Scala (Metals)
- [x] Julia
- [x] Fortran
- [x] Haskell
- [x] Vue
- [x] PowerShell

**Experimental (7):**
- [x] TypeScript VTS
- [x] Python Jedi
- [x] C# OmniSharp
- [x] Ruby Solargraph
- [x] Markdown (Marksman)
- [x] YAML
- [x] TOML (Taplo)
- [x] Groovy

---

## Appendix C: Configuration Schema Reference

### ProjectConfig Schema
```yaml
project_name: "my-project"
languages: [python, typescript]
ignored_paths:
  - "*.tmp"
  - "build/"
read_only: false
ignore_all_files_in_gitignore: true
initial_prompt: "Custom instructions..."
encoding: "utf-8"
excluded_tools: []
included_optional_tools: []
```

### SerenaConfig Schema
```yaml
projects:
  my-project:
    project_name: "my-project"
    languages: [python]
log_level: 20  # INFO
default_max_tool_answer_chars: 100000
tool_timeout: 240.0
gui_log_window_enabled: false
language_backend: "LSP"  # or "JetBrains"
default_context: "agent"
default_modes: [interactive, editing]
enable_token_usage_analytics: false
token_count_estimator: "CHAR_COUNT"
```

### Context Schema
```yaml
name: "custom-context"
prompt: "Jinja2 template..."
description: "Custom context description"
tool_description_overrides:
  find_symbol: "Custom description..."
single_project: false
excluded_tools: []
included_optional_tools: []
```

### Mode Schema
```yaml
name: "custom-mode"
prompt: "Jinja2 template..."
description: "Custom mode description"
excluded_tools: []
included_optional_tools: []
```

---

## Appendix D: MCP Protocol Mapping

### Tool Call Flow

**MCP Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "find_symbol",
    "arguments": {
      "name_path_pattern": "MyClass/my_method",
      "relative_path": "src/",
      "depth": 1
    }
  }
}
```

**Internal Execution:**
1. FastMCP routes to FindSymbolTool
2. Tool.apply_ex() invoked
3. FindSymbolTool.apply() executes
4. Result serialized to JSON string

**MCP Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "content": [
      {
        "type": "text",
        "text": "[{\"name_path\": \"MyClass/my_method\", ...}]"
      }
    ]
  }
}
```

---

## Appendix E: LSP Protocol Operations

### Implemented LSP Methods

**Lifecycle:**
- `initialize`
- `initialized`
- `shutdown`
- `exit`

**Document Synchronization:**
- `textDocument/didOpen`
- `textDocument/didChange`
- `textDocument/didClose`

**Language Features:**
- `textDocument/documentSymbol`
- `textDocument/definition`
- `textDocument/references`
- `textDocument/rename`

**Workspace:**
- `workspace/applyEdit`

---

## Appendix F: Rust Remake Priorities

### Critical Path (Must Have for 1:1 Parity)

**P0 - Core Infrastructure:**
1. Tool base system (Tool, ToolRegistry, markers)
2. Configuration system (ProjectConfig, SerenaConfig, contexts, modes)
3. Project management (Project, MemoriesManager)
4. MCP server interface
5. File tools (all 8)
6. Symbol tools (core 6, skip optional)
7. Memory tools (all 5)
8. Config tools (core 2)
9. Workflow tools (core 6)

**P1 - Language Server Support:**
1. SolidLanguageServer abstraction
2. LSP protocol handler
3. Stable language servers (top 10-15)
4. Symbol system (LanguageServerSymbol, NamePathMatcher)
5. LanguageServerManager
6. Code editor (LanguageServerCodeEditor)
7. Caching system (2-tier)

**P2 - Advanced Features:**
1. Remaining language servers
2. JetBrains backend
3. Dashboard/TUI
4. Analytics
5. GUI log viewer
6. Rust core acceleration

**P3 - Nice to Have:**
1. Experimental language servers
2. Performance profiling
3. Advanced error recovery
4. Custom language server extensions

---

**End of Feature Inventory**

This document should serve as the complete reference for achieving 1:1 parity between the Python and Rust implementations of Serena.
