//! File operation tools for Serena MCP server

use regex::RegexBuilder;
use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::mcp_2025_06_18::schema_utils::CallToolError;
use rust_mcp_sdk::schema::{CallToolResult, TextContent};
use std::path::Path;

use super::services::FileService;
use super::utils::truncate_output;

// ============================================================================
// read_file
// ============================================================================

#[mcp_tool(
    name = "read_file",
    description = "Read the complete contents of a file from the file system. Returns the full text of the file at the given relative path.",
    destructive_hint = false,
    idempotent_hint = true,
    read_only_hint = true
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct ReadFileTool {
    /// The relative path to the file to read.
    pub relative_path: String,

    /// The 0-based index of the first line to retrieve (optional).
    #[serde(default)]
    pub start_line: Option<u64>,

    /// The number of lines to read (optional). If None, read to end.
    #[serde(default)]
    pub limit: Option<u64>,

    /// Maximum characters to return. -1 for unlimited. Default: -1
    #[serde(default = "default_max_chars")]
    pub max_answer_chars: i32,
}

fn default_max_chars() -> i32 {
    -1
}

impl ReadFileTool {
    pub async fn run_tool(self, service: &FileService) -> Result<CallToolResult, CallToolError> {
        let content = service
            .read_file(Path::new(&self.relative_path))
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        // Apply line limits if specified
        let result = if self.start_line.is_some() || self.limit.is_some() {
            let lines: Vec<&str> = content.lines().collect();
            let start = self.start_line.unwrap_or(0) as usize;
            let end = self
                .limit
                .map(|l| start + l as usize)
                .unwrap_or(lines.len());

            lines
                .get(start..end.min(lines.len()))
                .map(|slice| slice.join("\n"))
                .unwrap_or_default()
        } else {
            content
        };

        // Apply character limit truncation
        let final_result = truncate_output(result, self.max_answer_chars);

        Ok(CallToolResult::text_content(vec![TextContent::from(
            final_result,
        )]))
    }
}

// ============================================================================
// create_text_file
// ============================================================================

#[mcp_tool(
    name = "create_text_file",
    description = "Write a new file or overwrite an existing file with the given content.",
    destructive_hint = true,
    idempotent_hint = false,
    read_only_hint = false
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct CreateTextFileTool {
    /// The relative path to the file to create.
    pub relative_path: String,

    /// The content to write to the file.
    pub content: String,
}

impl CreateTextFileTool {
    pub async fn run_tool(self, service: &FileService) -> Result<CallToolResult, CallToolError> {
        service
            .write_file(Path::new(&self.relative_path), &self.content)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            format!(
                "Successfully wrote {} bytes to {}",
                self.content.len(),
                self.relative_path
            ),
        )]))
    }
}

// ============================================================================
// list_dir
// ============================================================================

#[mcp_tool(
    name = "list_dir",
    description = "List files and directories in the given directory. Returns JSON with 'dirs' and 'files' arrays.",
    destructive_hint = false,
    idempotent_hint = true,
    read_only_hint = true
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct ListDirTool {
    /// The relative path to the directory to list. Use "." for project root.
    pub relative_path: String,

    /// Whether to scan subdirectories recursively.
    #[serde(default)]
    pub recursive: bool,

    /// Maximum characters to return. -1 for unlimited. Default: -1
    #[serde(default = "default_max_chars")]
    pub max_answer_chars: i32,
}

impl ListDirTool {
    pub async fn run_tool(self, service: &FileService) -> Result<CallToolResult, CallToolError> {
        let (dirs, files) = service
            .list_dir(Path::new(&self.relative_path), self.recursive)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let result = serde_json::json!({
            "dirs": dirs,
            "files": files
        });

        let json_string = result.to_string();
        let final_result = truncate_output(json_string, self.max_answer_chars);

        Ok(CallToolResult::text_content(vec![TextContent::from(
            final_result,
        )]))
    }
}

// ============================================================================
// find_file
// ============================================================================

#[mcp_tool(
    name = "find_file",
    description = "Find files matching a glob pattern within the project directory.",
    destructive_hint = false,
    idempotent_hint = true,
    read_only_hint = true
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct FindFileTool {
    /// The glob pattern to match (e.g., "*.rs", "**/*.py").
    pub file_mask: String,

    /// The relative path to search in. Use "." for project root.
    #[serde(default = "default_search_path")]
    pub relative_path: String,
}

fn default_search_path() -> String {
    ".".to_string()
}

impl FindFileTool {
    pub async fn run_tool(self, service: &FileService) -> Result<CallToolResult, CallToolError> {
        let search_path = service.project_root().join(&self.relative_path);
        let pattern = search_path
            .join(&self.file_mask)
            .to_string_lossy()
            .to_string();

        let mut matches = Vec::new();
        for entry in glob::glob(&pattern).map_err(|e| CallToolError::from_message(e.to_string()))? {
            if let Ok(path) = entry {
                let rel_path = path
                    .strip_prefix(service.project_root())
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                matches.push(rel_path);
            }
        }

        let result = serde_json::json!({ "files": matches });
        Ok(CallToolResult::text_content(vec![TextContent::from(
            result.to_string(),
        )]))
    }
}

// ============================================================================
// replace_content
// ============================================================================

#[mcp_tool(
    name = "replace_content",
    description = "Replace occurrences of a pattern in a file with new content. Supports literal or regex mode.",
    destructive_hint = true,
    idempotent_hint = false,
    read_only_hint = false
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct ReplaceContentTool {
    /// The relative path to the file.
    pub relative_path: String,

    /// The string or regex pattern to search for.
    pub needle: String,

    /// The replacement string.
    pub repl: String,

    /// Either "literal" or "regex".
    pub mode: String,

    /// If true, replace all occurrences. Otherwise, error if multiple matches.
    #[serde(default)]
    pub allow_multiple_occurrences: bool,
}

impl ReplaceContentTool {
    pub async fn run_tool(self, service: &FileService) -> Result<CallToolResult, CallToolError> {
        let content = service
            .read_file(Path::new(&self.relative_path))
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let new_content = if self.mode == "regex" {
            let re = RegexBuilder::new(&self.needle)
                .dot_matches_new_line(true)
                .multi_line(true)
                .build()
                .map_err(|e| CallToolError::from_message(e.to_string()))?;

            let match_count = re.find_iter(&content).count();
            if match_count == 0 {
                return Err(CallToolError::from_message("Pattern not found"));
            }
            if match_count > 1 && !self.allow_multiple_occurrences {
                return Err(CallToolError::from_message(format!(
                    "Pattern matches {} times. Set allow_multiple_occurrences=true or use a more specific pattern.",
                    match_count
                )));
            }

            re.replace_all(&content, &self.repl).to_string()
        } else {
            let match_count = content.matches(&self.needle).count();
            if match_count == 0 {
                return Err(CallToolError::from_message("Pattern not found"));
            }
            if match_count > 1 && !self.allow_multiple_occurrences {
                return Err(CallToolError::from_message(format!(
                    "Pattern matches {} times. Set allow_multiple_occurrences=true or use a more specific pattern.",
                    match_count
                )));
            }

            content.replace(&self.needle, &self.repl)
        };

        service
            .write_file(Path::new(&self.relative_path), &new_content)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            "Replacement successful".to_string(),
        )]))
    }
}

// ============================================================================
// search_for_pattern
// ============================================================================

#[mcp_tool(
    name = "search_for_pattern",
    description = "Search for a regex pattern across files in the project. Returns matching lines with context.",
    destructive_hint = false,
    idempotent_hint = true,
    read_only_hint = true
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct SearchForPatternTool {
    /// The regex pattern to search for.
    pub substring_pattern: String,

    /// Optional path to restrict the search.
    #[serde(default)]
    pub relative_path: Option<String>,

    /// Number of context lines before each match.
    #[serde(default)]
    pub context_lines_before: u64,

    /// Number of context lines after each match.
    #[serde(default)]
    pub context_lines_after: u64,

    /// Maximum characters to return. -1 for unlimited. Default: -1
    #[serde(default = "default_max_chars")]
    pub max_answer_chars: i32,
}

impl SearchForPatternTool {
    pub async fn run_tool(self, service: &FileService) -> Result<CallToolResult, CallToolError> {
        let re = RegexBuilder::new(&self.substring_pattern)
            .dot_matches_new_line(true)
            .build()
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let search_root = match &self.relative_path {
            Some(p) if !p.is_empty() => service.project_root().join(p),
            _ => service.project_root().to_path_buf(),
        };

        let mut results: std::collections::HashMap<String, Vec<serde_json::Value>> =
            std::collections::HashMap::new();

        // Walk the directory tree
        for entry in walkdir::WalkDir::new(&search_root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                let lines: Vec<&str> = content.lines().collect();

                for (i, line) in lines.iter().enumerate() {
                    if re.is_match(line) {
                        let rel_path = entry
                            .path()
                            .strip_prefix(service.project_root())
                            .unwrap_or(entry.path())
                            .to_string_lossy()
                            .replace('\\', "/");

                        let start = i.saturating_sub(self.context_lines_before as usize);
                        let end = (i + 1 + self.context_lines_after as usize).min(lines.len());

                        let context: Vec<String> = lines[start..end]
                            .iter()
                            .enumerate()
                            .map(|(j, l)| format!("{}: {}", start + j + 1, l))
                            .collect();

                        results
                            .entry(rel_path)
                            .or_default()
                            .push(serde_json::json!({
                                "line": i + 1,
                                "context": context
                            }));
                    }
                }
            }
        }

        let json_result = serde_json::to_string(&results).map_err(|e| {
            CallToolError::from_message(format!("Failed to serialize search results: {}", e))
        })?;

        let final_result = truncate_output(json_result, self.max_answer_chars);

        Ok(CallToolResult::text_content(vec![TextContent::from(
            final_result,
        )]))
    }
}
