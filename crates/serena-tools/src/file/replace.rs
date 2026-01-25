use async_trait::async_trait;
use regex::RegexBuilder;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serena_core::{SerenaError, Tool, ToolError, ToolResult};
use std::path::PathBuf;
use tokio::fs;
use tracing::debug;

/// Tool for replacing content in files using literal or regex patterns
pub struct ReplaceContentTool {
    project_root: PathBuf,
}

#[derive(Debug, Deserialize)]
struct ReplaceContentParams {
    /// The relative path to the file
    relative_path: String,
    /// The string or regex pattern to search for
    needle: String,
    /// The replacement string
    repl: String,
    /// Either "literal" or "regex"
    mode: String,
    /// If true, replace all occurrences. Otherwise, error if multiple matches.
    #[serde(default)]
    allow_multiple_occurrences: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReplaceContentOutput {
    path: String,
    replacements_made: usize,
    original_size: usize,
    new_size: usize,
}

impl ReplaceContentTool {
    /// Create a new ReplaceContentTool with the given project root
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }

    async fn replace_impl(
        &self,
        params: ReplaceContentParams,
    ) -> Result<ReplaceContentOutput, SerenaError> {
        // Validate mode
        if params.mode != "literal" && params.mode != "regex" {
            return Err(SerenaError::InvalidParameter(format!(
                "Mode must be 'literal' or 'regex', got: {}",
                params.mode
            )));
        }

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
        debug!("Reading file for replacement: {:?}", canonical_path);
        let content = fs::read_to_string(&canonical_path)
            .await
            .map_err(SerenaError::Io)?;

        let original_size = content.len();

        // Perform replacement based on mode
        let (new_content, match_count) = if params.mode == "regex" {
            let re = RegexBuilder::new(&params.needle)
                .dot_matches_new_line(true)
                .multi_line(true)
                .build()
                .map_err(|e| {
                    SerenaError::InvalidParameter(format!("Invalid regex pattern: {}", e))
                })?;

            let match_count = re.find_iter(&content).count();

            if match_count == 0 {
                return Err(SerenaError::Tool(ToolError::ExecutionFailed(
                    "Pattern not found".to_string(),
                )));
            }

            if match_count > 1 && !params.allow_multiple_occurrences {
                return Err(SerenaError::Tool(ToolError::ExecutionFailed(format!(
                    "Pattern matches {} times. Set allow_multiple_occurrences=true or use a more specific pattern.",
                    match_count
                ))));
            }

            (re.replace_all(&content, &params.repl).to_string(), match_count)
        } else {
            // Literal mode
            let match_count = content.matches(&params.needle).count();

            if match_count == 0 {
                return Err(SerenaError::Tool(ToolError::ExecutionFailed(
                    "Pattern not found".to_string(),
                )));
            }

            if match_count > 1 && !params.allow_multiple_occurrences {
                return Err(SerenaError::Tool(ToolError::ExecutionFailed(format!(
                    "Pattern matches {} times. Set allow_multiple_occurrences=true or use a more specific pattern.",
                    match_count
                ))));
            }

            (content.replace(&params.needle, &params.repl), match_count)
        };

        // Write the modified content back
        debug!(
            "Writing {} bytes to {:?}",
            new_content.len(),
            canonical_path
        );
        fs::write(&canonical_path, &new_content)
            .await
            .map_err(SerenaError::Io)?;

        Ok(ReplaceContentOutput {
            path: params.relative_path,
            replacements_made: match_count,
            original_size,
            new_size: new_content.len(),
        })
    }
}

#[async_trait]
impl Tool for ReplaceContentTool {
    fn name(&self) -> &str {
        "replace_content"
    }

    fn description(&self) -> &str {
        "Replace occurrences of a pattern in a file with new content. \
        Supports literal string matching or regex mode. \
        By default, errors if pattern matches multiple times (use allow_multiple_occurrences to override)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "relative_path": {
                    "type": "string",
                    "description": "The relative path to the file"
                },
                "needle": {
                    "type": "string",
                    "description": "The string or regex pattern to search for"
                },
                "repl": {
                    "type": "string",
                    "description": "The replacement string"
                },
                "mode": {
                    "type": "string",
                    "enum": ["literal", "regex"],
                    "description": "Match mode: 'literal' for exact string matching, 'regex' for regex patterns"
                },
                "allow_multiple_occurrences": {
                    "type": "boolean",
                    "description": "If true, replace all occurrences. If false (default), error if pattern matches more than once."
                }
            },
            "required": ["relative_path", "needle", "repl", "mode"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: ReplaceContentParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        let output = self.replace_impl(params).await?;

        let message = format!(
            "Made {} replacement(s) in '{}'",
            output.replacements_made, output.path
        );

        Ok(ToolResult::success_with_message(
            serde_json::to_value(output)?,
            message,
        ))
    }

    fn can_edit(&self) -> bool {
        true
    }

    fn requires_project(&self) -> bool {
        true
    }

    fn tags(&self) -> Vec<String> {
        vec![
            "file".to_string(),
            "replace".to_string(),
            "edit".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_literal_replacement() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello World").await.unwrap();

        let tool = ReplaceContentTool::new(temp_dir.path());

        let params = json!({
            "relative_path": "test.txt",
            "needle": "World",
            "repl": "Rust",
            "mode": "literal"
        });

        let result = tool.execute(params).await.unwrap();
        assert_eq!(result.status, serena_core::ToolStatus::Success);

        // Verify file was modified
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Hello Rust");
    }

    #[tokio::test]
    async fn test_regex_replacement() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Version: 1.0.0").await.unwrap();

        let tool = ReplaceContentTool::new(temp_dir.path());

        let params = json!({
            "relative_path": "test.txt",
            "needle": r"\d+\.\d+\.\d+",
            "repl": "2.0.0",
            "mode": "regex"
        });

        let result = tool.execute(params).await.unwrap();
        assert_eq!(result.status, serena_core::ToolStatus::Success);

        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Version: 2.0.0");
    }

    #[tokio::test]
    async fn test_multiple_occurrences_error() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "foo bar foo baz foo")
            .await
            .unwrap();

        let tool = ReplaceContentTool::new(temp_dir.path());

        let params = json!({
            "relative_path": "test.txt",
            "needle": "foo",
            "repl": "qux",
            "mode": "literal"
        });

        let result = tool.execute(params).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("matches 3 times"));
    }

    #[tokio::test]
    async fn test_multiple_occurrences_allowed() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "foo bar foo baz foo")
            .await
            .unwrap();

        let tool = ReplaceContentTool::new(temp_dir.path());

        let params = json!({
            "relative_path": "test.txt",
            "needle": "foo",
            "repl": "qux",
            "mode": "literal",
            "allow_multiple_occurrences": true
        });

        let result = tool.execute(params).await.unwrap();
        let data = result.data.unwrap();
        let output: ReplaceContentOutput = serde_json::from_value(data).unwrap();

        assert_eq!(output.replacements_made, 3);

        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "qux bar qux baz qux");
    }

    #[tokio::test]
    async fn test_pattern_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello World").await.unwrap();

        let tool = ReplaceContentTool::new(temp_dir.path());

        let params = json!({
            "relative_path": "test.txt",
            "needle": "notfound",
            "repl": "replacement",
            "mode": "literal"
        });

        let result = tool.execute(params).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_multiline_regex() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "start\nmiddle\nend").await.unwrap();

        let tool = ReplaceContentTool::new(temp_dir.path());

        let params = json!({
            "relative_path": "test.txt",
            "needle": "start.*end",
            "repl": "replaced",
            "mode": "regex"
        });

        let _result = tool.execute(params).await.unwrap();
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "replaced");
    }

    #[tokio::test]
    async fn test_invalid_mode() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "content").await.unwrap();

        let tool = ReplaceContentTool::new(temp_dir.path());

        let params = json!({
            "relative_path": "test.txt",
            "needle": "content",
            "repl": "new",
            "mode": "invalid"
        });

        let result = tool.execute(params).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_path_traversal_prevention() {
        let temp_dir = TempDir::new().unwrap();
        let tool = ReplaceContentTool::new(temp_dir.path());

        let params = json!({
            "relative_path": "../../../etc/passwd",
            "needle": "root",
            "repl": "hacked",
            "mode": "literal"
        });

        let result = tool.execute(params).await;
        assert!(result.is_err());
    }
}
