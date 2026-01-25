# Python to Rust Type Mappings - Tool API Contract Reference

**Generated**: 2025-12-25
**Purpose**: Exact type mappings and data structure definitions for Rust tool implementation

## Type Mapping Reference

### Primitive Types

| Python Type | Rust Type | Notes |
|-------------|-----------|-------|
| `str` | `String` | Owned string |
| `str` (param) | `&str` or `String` | Use `String` for owned, `&str` for borrowed |
| `int` | `i32` | Standard integer |
| `bool` | `bool` | Boolean |
| `None` | `()` | Unit type for no return value |

### Optional Types

| Python Type | Rust Type | Example |
|-------------|-----------|---------|
| `str \| None` | `Option<String>` | `None` → `None`, `"value"` → `Some("value".to_string())` |
| `int \| None` | `Option<i32>` | `None` → `None`, `5` → `Some(5)` |

### Collection Types

| Python Type | Rust Type | Example |
|-------------|-----------|---------|
| `list[str]` | `Vec<String>` | `["a", "b"]` → `vec!["a".to_string(), "b".to_string()]` |
| `list[int]` | `Vec<i32>` | `[1, 2, 3]` → `vec![1, 2, 3]` |
| `dict[str, Any]` | `HashMap<String, serde_json::Value>` | For JSON objects |
| `dict[str, list[str]]` | `HashMap<String, Vec<String>>` | For specific structured data |

### Literal Types

| Python Type | Rust Type | Definition |
|-------------|-----------|------------|
| `Literal["literal", "regex"]` | `enum Mode` | `enum Mode { Literal, Regex }` |

### Complex Return Types

| Python Return | Rust Return | Notes |
|---------------|-------------|-------|
| `str` (JSON) | `Result<String, ToolError>` | JSON serialized as string |
| `str` (message) | `Result<String, ToolError>` | Plain text message |
| `str` (constant) | `Result<&'static str, ToolError>` | For `SUCCESS_RESULT` |

---

## Enum Definitions for Rust

### Mode Enum (for replace_content/edit_memory)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReplaceMode {
    Literal,
    Regex,
}

impl ReplaceMode {
    pub fn from_str(s: &str) -> Result<Self, ToolError> {
        match s {
            "literal" => Ok(ReplaceMode::Literal),
            "regex" => Ok(ReplaceMode::Regex),
            _ => Err(ToolError::InvalidParameter(
                format!("Invalid mode: '{}', expected 'literal' or 'regex'", s)
            )),
        }
    }
}
```

### SymbolKind Enum (LSP standard)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum SymbolKind {
    File = 1,
    Module = 2,
    Namespace = 3,
    Package = 4,
    Class = 5,
    Method = 6,
    Property = 7,
    Field = 8,
    Constructor = 9,
    Enum = 10,
    Interface = 11,
    Function = 12,
    Variable = 13,
    Constant = 14,
    String = 15,
    Number = 16,
    Boolean = 17,
    Array = 18,
    Object = 19,
    Key = 20,
    Null = 21,
    EnumMember = 22,
    Struct = 23,
    Event = 24,
    Operator = 25,
    TypeParameter = 26,
}

impl SymbolKind {
    pub fn from_i32(value: i32) -> Result<Self, ToolError> {
        match value {
            1 => Ok(SymbolKind::File),
            2 => Ok(SymbolKind::Module),
            // ... all other values
            _ => Err(ToolError::InvalidParameter(
                format!("Invalid symbol kind: {}", value)
            )),
        }
    }
}
```

---

## Parameter Struct Definitions

### ReadFileParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadFileParams {
    pub relative_path: String,
    #[serde(default)]
    pub start_line: i32,  // 0-based
    #[serde(default)]
    pub end_line: Option<i32>,  // 0-based, inclusive
    #[serde(default = "default_max_answer_chars")]
    pub max_answer_chars: i32,
}

fn default_max_answer_chars() -> i32 {
    -1  // Sentinel for "use config default"
}
```

### CreateTextFileParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTextFileParams {
    pub relative_path: String,
    pub content: String,
}
```

### ListDirParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListDirParams {
    pub relative_path: String,
    pub recursive: bool,
    #[serde(default)]
    pub skip_ignored_files: bool,
    #[serde(default = "default_max_answer_chars")]
    pub max_answer_chars: i32,
}
```

### FindFileParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindFileParams {
    pub file_mask: String,
    pub relative_path: String,
}
```

### ReplaceContentParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaceContentParams {
    pub relative_path: String,
    pub needle: String,
    pub repl: String,
    pub mode: ReplaceMode,
    #[serde(default)]
    pub allow_multiple_occurrences: bool,
}
```

### DeleteLinesParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteLinesParams {
    pub relative_path: String,
    pub start_line: i32,  // 0-based
    pub end_line: i32,    // 0-based, inclusive
}
```

### ReplaceLinesParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaceLinesParams {
    pub relative_path: String,
    pub start_line: i32,
    pub end_line: i32,
    pub content: String,
}
```

### InsertAtLineParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertAtLineParams {
    pub relative_path: String,
    pub line: i32,  // 0-based
    pub content: String,
}
```

### SearchForPatternParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchForPatternParams {
    pub substring_pattern: String,
    #[serde(default)]
    pub context_lines_before: i32,
    #[serde(default)]
    pub context_lines_after: i32,
    #[serde(default)]
    pub paths_include_glob: String,
    #[serde(default)]
    pub paths_exclude_glob: String,
    #[serde(default)]
    pub relative_path: String,
    #[serde(default)]
    pub restrict_search_to_code_files: bool,
    #[serde(default = "default_max_answer_chars")]
    pub max_answer_chars: i32,
}
```

### GetSymbolsOverviewParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetSymbolsOverviewParams {
    pub relative_path: String,
    #[serde(default)]
    pub depth: i32,
    #[serde(default = "default_max_answer_chars")]
    pub max_answer_chars: i32,
}
```

### FindSymbolParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindSymbolParams {
    pub name_path_pattern: String,
    #[serde(default)]
    pub depth: i32,
    #[serde(default)]
    pub relative_path: String,
    #[serde(default)]
    pub include_body: bool,
    #[serde(default)]
    pub include_kinds: Vec<i32>,
    #[serde(default)]
    pub exclude_kinds: Vec<i32>,
    #[serde(default)]
    pub substring_matching: bool,
    #[serde(default = "default_max_answer_chars")]
    pub max_answer_chars: i32,
}
```

### FindReferencingSymbolsParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindReferencingSymbolsParams {
    pub name_path: String,
    pub relative_path: String,
    #[serde(default)]
    pub include_kinds: Vec<i32>,
    #[serde(default)]
    pub exclude_kinds: Vec<i32>,
    #[serde(default = "default_max_answer_chars")]
    pub max_answer_chars: i32,
}
```

### ReplaceSymbolBodyParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaceSymbolBodyParams {
    pub name_path: String,
    pub relative_path: String,
    pub body: String,
}
```

### InsertAfterSymbolParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertAfterSymbolParams {
    pub name_path: String,
    pub relative_path: String,
    pub body: String,
}
```

### InsertBeforeSymbolParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertBeforeSymbolParams {
    pub name_path: String,
    pub relative_path: String,
    pub body: String,
}
```

### RenameSymbolParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameSymbolParams {
    pub name_path: String,
    pub relative_path: String,
    pub new_name: String,
}
```

### WriteMemoryParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteMemoryParams {
    pub memory_file_name: String,
    pub content: String,
    #[serde(default = "default_max_answer_chars")]
    pub max_answer_chars: i32,
}
```

### ReadMemoryParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadMemoryParams {
    pub memory_file_name: String,
    #[serde(default = "default_max_answer_chars")]
    pub max_answer_chars: i32,
}
```

### DeleteMemoryParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteMemoryParams {
    pub memory_file_name: String,
}
```

### EditMemoryParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditMemoryParams {
    pub memory_file_name: String,
    pub needle: String,
    pub repl: String,
    pub mode: ReplaceMode,
}
```

### ActivateProjectParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivateProjectParams {
    pub project: String,
}
```

### RemoveProjectParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveProjectParams {
    pub project_name: String,
}
```

### SwitchModesParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchModesParams {
    pub modes: Vec<String>,
}
```

### ExecuteShellCommandParams

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteShellCommandParams {
    pub command: String,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default = "default_capture_stderr")]
    pub capture_stderr: bool,
    #[serde(default = "default_max_answer_chars")]
    pub max_answer_chars: i32,
}

fn default_capture_stderr() -> bool {
    true
}
```

---

## Response Data Structures

### ListDirResponse

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListDirResponse {
    pub dirs: Vec<String>,
    pub files: Vec<String>,
}

// Error variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListDirError {
    pub error: String,
    pub project_root: String,
    pub hint: String,
}
```

### FindFileResponse

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindFileResponse {
    pub files: Vec<String>,
}
```

### SearchForPatternResponse

```rust
// Type alias for clarity
pub type SearchForPatternResponse = HashMap<String, Vec<String>>;

// Where:
// - Key: relative file path
// - Value: Vec of matched line groups (as display strings)
```

### ExecuteShellCommandResponse

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteShellCommandResponse {
    pub stdout: String,
    pub stderr: String,
    pub returncode: i32,
}
```

### SymbolDict (Sanitized)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolDict {
    pub name_path: String,
    pub relative_path: String,
    pub kind: i32,  // SymbolKind as integer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_location: Option<BodyLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<SymbolDict>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyLocation {
    pub start_line: i32,
    pub end_line: i32,
    // May include other fields
}
```

### ReferenceDict

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceDict {
    pub name_path: String,
    pub relative_path: String,
    pub kind: i32,
    pub content_around_reference: String,  // Display string with context
}
```

---

## Error Type Definition

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolError {
    InvalidParameter(String),
    FileNotFound(String),
    DirectoryNotFound(String),
    PathValidationFailed(String),
    NoActiveProject(String),
    ToolNotActive(String),
    NoMatches(String),
    MultipleMatches { count: usize, message: String },
    AmbiguousMatch(String),
    ContentTooLong { actual: usize, max: usize },
    IoError(String),
    RegexError(String),
    LanguageServerError(String),
    ExecutionTimeout(String),
    Other(String),
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolError::InvalidParameter(msg) => write!(f, "Error: {}", msg),
            ToolError::FileNotFound(path) => write!(f, "File not found: {}", path),
            ToolError::DirectoryNotFound(path) => write!(f, "Directory not found: {}", path),
            ToolError::PathValidationFailed(msg) => write!(f, "Path validation failed: {}", msg),
            ToolError::NoActiveProject(msg) => write!(f, "Error: No active project. {}", msg),
            ToolError::ToolNotActive(msg) => write!(f, "Error: Tool not active. {}", msg),
            ToolError::NoMatches(msg) => write!(f, "Error: No matches found. {}", msg),
            ToolError::MultipleMatches { count, message } => {
                write!(f, "Expression matches {} occurrences. {}", count, message)
            }
            ToolError::AmbiguousMatch(msg) => write!(f, "Match is ambiguous: {}", msg),
            ToolError::ContentTooLong { actual, max } => {
                write!(f, "The answer is too long ({} characters). Please try a more specific tool query or raise the max_answer_chars parameter.", actual)
            }
            ToolError::IoError(msg) => write!(f, "IO error: {}", msg),
            ToolError::RegexError(msg) => write!(f, "Regex error: {}", msg),
            ToolError::LanguageServerError(msg) => write!(f, "Language server error: {}", msg),
            ToolError::ExecutionTimeout(msg) => write!(f, "Execution timeout: {}", msg),
            ToolError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for ToolError {}
```

---

## Constant Definitions

```rust
pub const SUCCESS_RESULT: &str = "OK";

pub const DEFAULT_MAX_ANSWER_CHARS: i32 = -1;  // Sentinel value
pub const DEFAULT_START_LINE: i32 = 0;
pub const DEFAULT_DEPTH: i32 = 0;
pub const DEFAULT_CONTEXT_LINES_BEFORE: i32 = 0;
pub const DEFAULT_CONTEXT_LINES_AFTER: i32 = 0;
pub const DEFAULT_CAPTURE_STDERR: bool = true;

// Regex flags
pub const REGEX_FLAGS: &str = "(?s)(?m)";  // DOTALL | MULTILINE
```

---

## Validation Functions

### Path Validation

```rust
pub fn validate_relative_path(
    project: &Project,
    relative_path: &str,
    require_not_ignored: bool,
) -> Result<(), ToolError> {
    // 1. Check if path is relative
    if Path::new(relative_path).is_absolute() {
        return Err(ToolError::PathValidationFailed(
            format!("Path must be relative, got: {}", relative_path)
        ));
    }

    // 2. Resolve to absolute path
    let abs_path = project.project_root.join(relative_path);

    // 3. Check existence
    if !abs_path.exists() {
        return Err(ToolError::PathValidationFailed(
            format!("Path does not exist: {}", relative_path)
        ));
    }

    // 4. Check if ignored (if required)
    if require_not_ignored {
        if project.is_ignored_path(&abs_path) {
            return Err(ToolError::PathValidationFailed(
                format!("Path is ignored: {}", relative_path)
            ));
        }
    }

    Ok(())
}
```

### Mode Validation

```rust
pub fn validate_mode(mode: &str) -> Result<ReplaceMode, ToolError> {
    ReplaceMode::from_str(mode)
}
```

### Match Count Validation

```rust
pub fn validate_match_count(
    count: usize,
    allow_multiple: bool,
    relative_path: &str,
) -> Result<(), ToolError> {
    if count == 0 {
        return Err(ToolError::NoMatches(
            format!("No matches of search expression found in file '{}'.", relative_path)
        ));
    }
    if !allow_multiple && count > 1 {
        return Err(ToolError::MultipleMatches {
            count,
            message: format!(
                "in file '{}'. Please revise the expression to be more specific or enable allow_multiple_occurrences if this is expected.",
                relative_path
            ),
        });
    }
    Ok(())
}
```

### Content Length Validation

```rust
pub fn limit_length(
    result: String,
    max_answer_chars: i32,
    config_default: i32,
) -> Result<String, ToolError> {
    let max_chars = if max_answer_chars == -1 {
        config_default
    } else {
        max_answer_chars
    };

    if max_chars <= 0 {
        return Err(ToolError::InvalidParameter(
            format!("max_answer_chars must be positive or -1, got: {}", max_chars)
        ));
    }

    let actual_len = result.len();
    if actual_len > max_chars as usize {
        Err(ToolError::ContentTooLong {
            actual: actual_len,
            max: max_chars as usize,
        })
    } else {
        Ok(result)
    }
}
```

---

## JSON Serialization Helpers

```rust
use serde_json;

pub fn to_json<T: Serialize>(value: &T) -> Result<String, ToolError> {
    serde_json::to_string(value).map_err(|e| {
        ToolError::Other(format!("JSON serialization error: {}", e))
    })
}

pub fn to_json_pretty<T: Serialize>(value: &T) -> Result<String, ToolError> {
    serde_json::to_string_pretty(value).map_err(|e| {
        ToolError::Other(format!("JSON serialization error: {}", e))
    })
}
```

---

## Regex Helpers

### Backreference Expansion

Python uses custom backreference syntax `$!1`, `$!2` instead of standard `\1`, `\2`.

```rust
use regex::Regex;

pub fn expand_backreferences(
    template: &str,
    captures: &regex::Captures,
) -> Result<String, ToolError> {
    let backreference_re = Regex::new(r"\$!(\d+)").unwrap();

    let mut result = template.to_string();
    for cap in backreference_re.captures_iter(template) {
        let group_num: usize = cap[1].parse().map_err(|e| {
            ToolError::RegexError(format!("Invalid backreference group number: {}", e))
        })?;

        let group_value = captures.get(group_num)
            .map(|m| m.as_str())
            .unwrap_or("");

        let placeholder = &cap[0];
        result = result.replace(placeholder, group_value);
    }

    Ok(result)
}
```

### Ambiguity Detection

```rust
pub fn check_ambiguity(
    matched_text: &str,
    pattern: &Regex,
) -> Result<(), ToolError> {
    // For multi-line matches, check if pattern matches again within matched text
    if matched_text.contains('\n') {
        // Skip first character and search again
        if let Some(rest) = matched_text.get(1..) {
            if pattern.is_match(rest) {
                return Err(ToolError::AmbiguousMatch(
                    "the search pattern matches multiple overlapping occurrences. \
                     Please revise the search pattern to be more specific to avoid ambiguity."
                        .to_string()
                ));
            }
        }
    }
    Ok(())
}
```

---

## File I/O Helpers

### Read File with Encoding

```rust
use std::fs;
use std::path::Path;

pub fn read_file_with_encoding(
    path: &Path,
    encoding: &str,
) -> Result<String, ToolError> {
    // For simplicity, assume UTF-8 (Rust default)
    // In production, use encoding_rs crate for other encodings
    if encoding != "utf-8" && encoding != "UTF-8" {
        return Err(ToolError::Other(
            format!("Unsupported encoding: {}", encoding)
        ));
    }

    fs::read_to_string(path).map_err(|e| {
        ToolError::IoError(format!("Failed to read file {:?}: {}", path, e))
    })
}
```

### Write File with Encoding

```rust
pub fn write_file_with_encoding(
    path: &Path,
    content: &str,
    encoding: &str,
) -> Result<(), ToolError> {
    // For simplicity, assume UTF-8 (Rust default)
    if encoding != "utf-8" && encoding != "UTF-8" {
        return Err(ToolError::Other(
            format!("Unsupported encoding: {}", encoding)
        ));
    }

    // Create parent directories if needed
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            ToolError::IoError(format!("Failed to create parent directories: {}", e))
        })?;
    }

    fs::write(path, content).map_err(|e| {
        ToolError::IoError(format!("Failed to write file {:?}: {}", path, e))
    })
}
```

### Ensure Newline

```rust
pub fn ensure_trailing_newline(content: &str) -> String {
    if content.ends_with('\n') {
        content.to_string()
    } else {
        format!("{}\n", content)
    }
}
```

---

## Test Data Examples

### ReadFile - Basic

```json
{
  "input": {
    "relative_path": "src/main.py",
    "start_line": 0,
    "end_line": null,
    "max_answer_chars": -1
  },
  "output": "import os\nimport sys\n\ndef main():\n    print('Hello, world!')\n",
  "error": null
}
```

### ReadFile - Line Range

```json
{
  "input": {
    "relative_path": "src/main.py",
    "start_line": 2,
    "end_line": 4,
    "max_answer_chars": -1
  },
  "output": "\ndef main():\n    print('Hello, world!')",
  "error": null
}
```

### CreateTextFile - New File

```json
{
  "input": {
    "relative_path": "src/new_file.py",
    "content": "# New file\n"
  },
  "output": "File created: src/new_file.py.",
  "error": null
}
```

### CreateTextFile - Overwrite

```json
{
  "input": {
    "relative_path": "src/existing.py",
    "content": "# Updated content\n"
  },
  "output": "File created: src/existing.py. Overwrote existing file.",
  "error": null
}
```

### ListDir - Success

```json
{
  "input": {
    "relative_path": "src",
    "recursive": true,
    "skip_ignored_files": false,
    "max_answer_chars": -1
  },
  "output": "{\"dirs\": [\"src/utils\", \"src/tests\"], \"files\": [\"src/main.py\", \"src/utils/__init__.py\"]}",
  "error": null
}
```

### ListDir - Not Found Error

```json
{
  "input": {
    "relative_path": "nonexistent",
    "recursive": false,
    "skip_ignored_files": false,
    "max_answer_chars": -1
  },
  "output": "{\"error\": \"Directory not found: nonexistent\", \"project_root\": \"/path/to/project\", \"hint\": \"Check if the path is correct relative to the project root\"}",
  "error": null
}
```

### FindFile - Success

```json
{
  "input": {
    "file_mask": "*.py",
    "relative_path": "src"
  },
  "output": "{\"files\": [\"src/main.py\", \"src/utils.py\"]}",
  "error": null
}
```

### ReplaceContent - Literal Mode

```json
{
  "input": {
    "relative_path": "src/main.py",
    "needle": "Hello, world!",
    "repl": "Hello, Rust!",
    "mode": "literal",
    "allow_multiple_occurrences": false
  },
  "output": "OK",
  "error": null
}
```

### ReplaceContent - Regex Mode with Backreferences

```json
{
  "input": {
    "relative_path": "src/main.py",
    "needle": "def (\\w+)\\(\\):",
    "repl": "async def $!1():",
    "mode": "regex",
    "allow_multiple_occurrences": true
  },
  "output": "OK",
  "error": null
}
```

### ReplaceContent - No Matches Error

```json
{
  "input": {
    "relative_path": "src/main.py",
    "needle": "nonexistent_pattern",
    "repl": "replacement",
    "mode": "literal",
    "allow_multiple_occurrences": false
  },
  "output": "Error: No matches of search expression found in file 'src/main.py'.",
  "error": "NoMatches"
}
```

### ReplaceContent - Multiple Matches Error

```json
{
  "input": {
    "relative_path": "src/main.py",
    "needle": "import",
    "repl": "from",
    "mode": "literal",
    "allow_multiple_occurrences": false
  },
  "output": "Expression matches 3 occurrences in file 'src/main.py'. Please revise the expression to be more specific or enable allow_multiple_occurrences if this is expected.",
  "error": "MultipleMatches"
}
```

### SearchForPattern - Success

```json
{
  "input": {
    "substring_pattern": "def \\w+",
    "context_lines_before": 1,
    "context_lines_after": 1,
    "paths_include_glob": "*.py",
    "paths_exclude_glob": "*test*",
    "relative_path": "src",
    "restrict_search_to_code_files": true,
    "max_answer_chars": -1
  },
  "output": "{\"src/main.py\": [\"\\ndef main():\\n    print('Hello')\"], \"src/utils.py\": [\"\\ndef helper():\\n    pass\"]}",
  "error": null
}
```

### FindSymbol - Simple Name

```json
{
  "input": {
    "name_path_pattern": "main",
    "depth": 0,
    "relative_path": "",
    "include_body": false,
    "include_kinds": [],
    "exclude_kinds": [],
    "substring_matching": false,
    "max_answer_chars": -1
  },
  "output": "[{\"name_path\": \"main\", \"relative_path\": \"src/main.py\", \"kind\": 12, \"body_location\": {\"start_line\": 3, \"end_line\": 4}}]",
  "error": null
}
```

### FindSymbol - With Children

```json
{
  "input": {
    "name_path_pattern": "MyClass",
    "depth": 1,
    "relative_path": "src",
    "include_body": true,
    "include_kinds": [5],
    "exclude_kinds": [],
    "substring_matching": false,
    "max_answer_chars": -1
  },
  "output": "[{\"name_path\": \"MyClass\", \"relative_path\": \"src/classes.py\", \"kind\": 5, \"body_location\": {\"start_line\": 10, \"end_line\": 20}, \"body\": \"class MyClass:\\n    def __init__(self):\\n        pass\", \"children\": [{\"name_path\": \"MyClass/__init__\", \"kind\": 9, \"body_location\": {\"start_line\": 11, \"end_line\": 12}}]}]",
  "error": null
}
```

### ExecuteShellCommand - Success

```json
{
  "input": {
    "command": "echo 'Hello, Rust!'",
    "cwd": null,
    "capture_stderr": true,
    "max_answer_chars": -1
  },
  "output": "{\"stdout\": \"Hello, Rust!\\n\", \"stderr\": \"\", \"returncode\": 0}",
  "error": null
}
```

### ExecuteShellCommand - Error

```json
{
  "input": {
    "command": "nonexistent_command",
    "cwd": null,
    "capture_stderr": true,
    "max_answer_chars": -1
  },
  "output": "{\"stdout\": \"\", \"stderr\": \"nonexistent_command: command not found\\n\", \"returncode\": 127}",
  "error": null
}
```

---

## Testing Checklist

### Type Equivalence Tests
- [ ] All param structs serialize/deserialize identically to Python
- [ ] All response structs serialize/deserialize identically to Python
- [ ] Enum values match exactly (including integer representations)
- [ ] Optional fields handle None/null correctly
- [ ] Default values match Python defaults

### Validation Tests
- [ ] Path validation matches Python behavior exactly
- [ ] Mode validation matches Python behavior exactly
- [ ] Match count validation matches Python error messages
- [ ] Content length validation matches Python error messages
- [ ] Line range validation matches Python behavior

### Regex Tests
- [ ] Backreference expansion ($!1, $!2) works identically
- [ ] Ambiguity detection matches Python logic
- [ ] DOTALL and MULTILINE flags work identically
- [ ] Literal mode escapes patterns correctly

### File I/O Tests
- [ ] UTF-8 encoding/decoding works identically
- [ ] Newline handling matches Python (\\n endings)
- [ ] Parent directory creation works identically
- [ ] File overwrite behavior matches Python

### Error Message Tests
- [ ] All error messages match Python word-for-word
- [ ] Error types map correctly to ToolError variants
- [ ] Error context (file paths, line numbers) matches

---

**End of Document**
