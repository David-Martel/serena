use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serena_core::{SerenaError, Tool, ToolError, ToolResult};
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, warn};

/// Tool for reading file contents
pub struct ReadFileTool {
    project_root: PathBuf,
}

#[derive(Debug, Deserialize)]
struct ReadFileParams {
    relative_path: String,
    #[serde(default)]
    start_line: Option<usize>,
    #[serde(default)]
    end_line: Option<usize>,
    #[serde(default)]
    max_answer_chars: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReadFileOutput {
    path: String,
    content: String,
    total_lines: usize,
    lines_read: usize,
    truncated: bool,
}

impl ReadFileTool {
    /// Create a new ReadFileTool with the given project root
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }

    /// Read file with optional line slicing
    async fn read_file_impl(&self, params: ReadFileParams) -> Result<ReadFileOutput, SerenaError> {
        // Validate and construct full path
        let full_path = self.project_root.join(&params.relative_path);

        // Security check: ensure path is within project root
        let canonical_root = self
            .project_root
            .canonicalize()
            .map_err(|e| SerenaError::InvalidParameter(format!("Invalid project root: {}", e)))?;

        let canonical_path = full_path.canonicalize().map_err(|_e| {
            SerenaError::NotFound(format!("File not found: {}", params.relative_path))
        })?;

        if !canonical_path.starts_with(&canonical_root) {
            return Err(SerenaError::Tool(ToolError::PermissionDenied(format!(
                "Path '{}' is outside project root",
                params.relative_path
            ))));
        }

        // Read file content
        debug!("Reading file: {:?}", canonical_path);
        let content = fs::read_to_string(&canonical_path)
            .await
            .map_err(SerenaError::Io)?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        // Apply line slicing if requested
        let (selected_lines, lines_read) =
            if params.start_line.is_some() || params.end_line.is_some() {
                let start = params.start_line.unwrap_or(0);
                // end_line is documented as inclusive, so add 1 for exclusive slice
                // If not provided, default to total_lines (meaning all remaining lines)
                let end = match params.end_line {
                    Some(e) => e.saturating_add(1), // Convert inclusive to exclusive
                    None => total_lines,
                };

                if start > total_lines {
                    warn!("start_line {} exceeds total lines {}", start, total_lines);
                    return Ok(ReadFileOutput {
                        path: params.relative_path,
                        content: String::new(),
                        total_lines,
                        lines_read: 0,
                        truncated: false,
                    });
                }

                // Cap at total_lines to prevent out-of-bounds
                let end = end.min(total_lines);
                let slice = &lines[start..end];
                let count = slice.len();
                (slice.join("\n"), count)
            } else {
                (content, total_lines)
            };

        // Apply character limit if specified
        let (final_content, truncated) = if let Some(max_chars) = params.max_answer_chars {
            if selected_lines.len() > max_chars {
                warn!(
                    "Content truncated from {} to {} chars",
                    selected_lines.len(),
                    max_chars
                );
                (
                    selected_lines.chars().take(max_chars).collect::<String>(),
                    true,
                )
            } else {
                (selected_lines, false)
            }
        } else {
            (selected_lines, false)
        };

        Ok(ReadFileOutput {
            path: params.relative_path,
            content: final_content,
            total_lines,
            lines_read,
            truncated,
        })
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Reads a file from the local filesystem. You can access any file directly by using this tool. \
        Supports optional line slicing and character limits."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "relative_path": {
                    "type": "string",
                    "description": "The relative path to the file to read"
                },
                "start_line": {
                    "type": "integer",
                    "description": "The 0-based index of the first line to be retrieved (optional)"
                },
                "end_line": {
                    "type": "integer",
                    "description": "The 0-based index of the last line to be retrieved, inclusive (optional)"
                },
                "max_answer_chars": {
                    "type": "integer",
                    "description": "Maximum characters to return. If exceeded, content is truncated (optional)"
                }
            },
            "required": ["relative_path"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: ReadFileParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        let output = self.read_file_impl(params).await?;

        Ok(ToolResult::success(serde_json::to_value(output)?))
    }

    fn can_edit(&self) -> bool {
        false
    }

    fn requires_project(&self) -> bool {
        true
    }

    fn tags(&self) -> Vec<String> {
        vec!["file".to_string(), "read".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs::write;

    async fn setup_test_env() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        write(&test_file, "Line 0\nLine 1\nLine 2\nLine 3\nLine 4")
            .await
            .unwrap();
        (temp_dir, test_file)
    }

    #[tokio::test]
    async fn test_read_full_file() {
        let (temp_dir, _) = setup_test_env().await;
        let tool = ReadFileTool::new(temp_dir.path());

        let params = json!({
            "relative_path": "test.txt"
        });

        let result = tool.execute(params).await.unwrap();
        assert_eq!(result.status, serena_core::ToolStatus::Success);

        let data = result.data.unwrap();
        let output: ReadFileOutput = serde_json::from_value(data).unwrap();
        assert_eq!(output.total_lines, 5);
        assert_eq!(output.lines_read, 5);
        assert!(!output.truncated);
    }

    #[tokio::test]
    async fn test_read_with_line_slice() {
        let (temp_dir, _) = setup_test_env().await;
        let tool = ReadFileTool::new(temp_dir.path());

        // end_line is inclusive per the schema documentation
        // start_line=1, end_line=3 means lines 1, 2, and 3 (0-based indices)
        let params = json!({
            "relative_path": "test.txt",
            "start_line": 1,
            "end_line": 3
        });

        let result = tool.execute(params).await.unwrap();
        let data = result.data.unwrap();
        let output: ReadFileOutput = serde_json::from_value(data).unwrap();

        // Should include lines 1, 2, and 3 (3 lines total, inclusive)
        assert_eq!(output.lines_read, 3);
        assert!(output.content.contains("Line 1"));
        assert!(output.content.contains("Line 2"));
        assert!(output.content.contains("Line 3"));
        assert!(!output.content.contains("Line 4"));
    }

    #[tokio::test]
    async fn test_path_traversal_prevention() {
        let temp_dir = TempDir::new().unwrap();
        let tool = ReadFileTool::new(temp_dir.path());

        let params = json!({
            "relative_path": "../../../etc/passwd"
        });

        let result = tool.execute(params).await;
        assert!(result.is_err());
    }
}
