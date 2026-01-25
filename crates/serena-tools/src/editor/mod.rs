//! Line-level editing tools for file manipulation
//!
//! Provides tools for precise line-based operations on files, complementing
//! the pattern-based replace_content tool.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serena_core::{SerenaError, Tool, ToolError, ToolResult};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::debug;

// ============================================================================
// DeleteLinesTool
// ============================================================================

/// Tool for deleting specific lines from a file
pub struct DeleteLinesTool {
    project_root: PathBuf,
}

#[derive(Debug, Deserialize)]
struct DeleteLinesParams {
    /// The relative path to the file
    relative_path: String,
    /// The 1-based start line number (inclusive)
    start_line: usize,
    /// The 1-based end line number (inclusive)
    end_line: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct DeleteLinesOutput {
    path: String,
    lines_deleted: usize,
    new_total_lines: usize,
}

impl DeleteLinesTool {
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }

    async fn delete_impl(&self, params: DeleteLinesParams) -> Result<DeleteLinesOutput, SerenaError> {
        // Validate line numbers
        if params.start_line == 0 || params.end_line == 0 {
            return Err(SerenaError::InvalidParameter(
                "Line numbers must be 1-based (start from 1)".to_string(),
            ));
        }
        if params.start_line > params.end_line {
            return Err(SerenaError::InvalidParameter(format!(
                "start_line ({}) must be <= end_line ({})",
                params.start_line, params.end_line
            )));
        }

        // Validate and construct full path
        let full_path = self.project_root.join(&params.relative_path);
        let canonical_path = validate_path(&full_path, &self.project_root, &params.relative_path)?;

        // Read file content
        let content = fs::read_to_string(&canonical_path)
            .await
            .map_err(SerenaError::Io)?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        // Validate line range
        if params.start_line > total_lines {
            return Err(SerenaError::InvalidParameter(format!(
                "start_line ({}) exceeds file length ({})",
                params.start_line, total_lines
            )));
        }

        // Convert to 0-based indices
        let start = params.start_line - 1;
        let end = params.end_line.min(total_lines);

        // Build new content excluding deleted lines
        let new_lines: Vec<&str> = lines
            .iter()
            .enumerate()
            .filter(|(i, _)| *i < start || *i >= end)
            .map(|(_, line)| *line)
            .collect();

        let lines_deleted = end - start;
        let new_total = new_lines.len();
        let new_content = new_lines.join("\n");

        // Add trailing newline if original had one
        let new_content = if content.ends_with('\n') {
            format!("{}\n", new_content)
        } else {
            new_content
        };

        debug!(
            "Deleting lines {}-{} from {:?}",
            params.start_line, end, canonical_path
        );

        fs::write(&canonical_path, &new_content)
            .await
            .map_err(SerenaError::Io)?;

        Ok(DeleteLinesOutput {
            path: params.relative_path,
            lines_deleted,
            new_total_lines: new_total,
        })
    }
}

#[async_trait]
impl Tool for DeleteLinesTool {
    fn name(&self) -> &str {
        "delete_lines"
    }

    fn description(&self) -> &str {
        "Delete a range of lines from a file. Line numbers are 1-based and inclusive."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "relative_path": {
                    "type": "string",
                    "description": "The relative path to the file"
                },
                "start_line": {
                    "type": "integer",
                    "description": "The 1-based start line number (inclusive)",
                    "minimum": 1
                },
                "end_line": {
                    "type": "integer",
                    "description": "The 1-based end line number (inclusive)",
                    "minimum": 1
                }
            },
            "required": ["relative_path", "start_line", "end_line"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: DeleteLinesParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        let output = self.delete_impl(params).await?;

        let message = format!(
            "Deleted {} lines from '{}' (now {} lines)",
            output.lines_deleted, output.path, output.new_total_lines
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
        vec!["file".to_string(), "edit".to_string(), "lines".to_string()]
    }
}

// ============================================================================
// InsertAtLineTool
// ============================================================================

/// Tool for inserting content at a specific line
pub struct InsertAtLineTool {
    project_root: PathBuf,
}

#[derive(Debug, Deserialize)]
struct InsertAtLineParams {
    /// The relative path to the file
    relative_path: String,
    /// The 1-based line number to insert at (content will appear at this line)
    line: usize,
    /// The content to insert
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct InsertAtLineOutput {
    path: String,
    lines_inserted: usize,
    new_total_lines: usize,
}

impl InsertAtLineTool {
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }

    async fn insert_impl(&self, params: InsertAtLineParams) -> Result<InsertAtLineOutput, SerenaError> {
        // Validate line number
        if params.line == 0 {
            return Err(SerenaError::InvalidParameter(
                "Line number must be 1-based (start from 1)".to_string(),
            ));
        }

        // Validate and construct full path
        let full_path = self.project_root.join(&params.relative_path);
        let canonical_path = validate_path(&full_path, &self.project_root, &params.relative_path)?;

        // Read file content
        let content = fs::read_to_string(&canonical_path)
            .await
            .map_err(SerenaError::Io)?;

        let mut lines: Vec<&str> = content.lines().collect();
        let insert_lines: Vec<&str> = params.content.lines().collect();
        let lines_to_insert = insert_lines.len();

        // Convert to 0-based index
        let insert_at = (params.line - 1).min(lines.len());

        // Insert the new content
        for (i, insert_line) in insert_lines.iter().enumerate() {
            lines.insert(insert_at + i, insert_line);
        }

        let new_total = lines.len();
        let new_content = lines.join("\n");

        // Add trailing newline if original had one
        let new_content = if content.ends_with('\n') {
            format!("{}\n", new_content)
        } else {
            new_content
        };

        debug!(
            "Inserting {} lines at line {} in {:?}",
            lines_to_insert, params.line, canonical_path
        );

        fs::write(&canonical_path, &new_content)
            .await
            .map_err(SerenaError::Io)?;

        Ok(InsertAtLineOutput {
            path: params.relative_path,
            lines_inserted: lines_to_insert,
            new_total_lines: new_total,
        })
    }
}

#[async_trait]
impl Tool for InsertAtLineTool {
    fn name(&self) -> &str {
        "insert_at_line"
    }

    fn description(&self) -> &str {
        "Insert content at a specific line in a file. Line number is 1-based. \
        Existing content at that line and below will be shifted down."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "relative_path": {
                    "type": "string",
                    "description": "The relative path to the file"
                },
                "line": {
                    "type": "integer",
                    "description": "The 1-based line number to insert at",
                    "minimum": 1
                },
                "content": {
                    "type": "string",
                    "description": "The content to insert (can be multiple lines)"
                }
            },
            "required": ["relative_path", "line", "content"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: InsertAtLineParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        let output = self.insert_impl(params).await?;

        let message = format!(
            "Inserted {} lines at line {} in '{}' (now {} lines)",
            output.lines_inserted,
            output.new_total_lines - output.lines_inserted + 1,
            output.path,
            output.new_total_lines
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
        vec!["file".to_string(), "edit".to_string(), "lines".to_string()]
    }
}

// ============================================================================
// ReplaceLinesTool
// ============================================================================

/// Tool for replacing a range of lines with new content
pub struct ReplaceLinesTool {
    project_root: PathBuf,
}

#[derive(Debug, Deserialize)]
struct ReplaceLinesParams {
    /// The relative path to the file
    relative_path: String,
    /// The 1-based start line number (inclusive)
    start_line: usize,
    /// The 1-based end line number (inclusive)
    end_line: usize,
    /// The replacement content
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReplaceLinesOutput {
    path: String,
    lines_replaced: usize,
    lines_inserted: usize,
    new_total_lines: usize,
}

impl ReplaceLinesTool {
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }

    async fn replace_impl(&self, params: ReplaceLinesParams) -> Result<ReplaceLinesOutput, SerenaError> {
        // Validate line numbers
        if params.start_line == 0 || params.end_line == 0 {
            return Err(SerenaError::InvalidParameter(
                "Line numbers must be 1-based (start from 1)".to_string(),
            ));
        }
        if params.start_line > params.end_line {
            return Err(SerenaError::InvalidParameter(format!(
                "start_line ({}) must be <= end_line ({})",
                params.start_line, params.end_line
            )));
        }

        // Validate and construct full path
        let full_path = self.project_root.join(&params.relative_path);
        let canonical_path = validate_path(&full_path, &self.project_root, &params.relative_path)?;

        // Read file content
        let content = fs::read_to_string(&canonical_path)
            .await
            .map_err(SerenaError::Io)?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        // Validate line range
        if params.start_line > total_lines {
            return Err(SerenaError::InvalidParameter(format!(
                "start_line ({}) exceeds file length ({})",
                params.start_line, total_lines
            )));
        }

        // Convert to 0-based indices
        let start = params.start_line - 1;
        let end = params.end_line.min(total_lines);
        let lines_replaced = end - start;

        // Split content into before, replacement, and after
        let before: Vec<&str> = lines[..start].to_vec();
        let after: Vec<&str> = lines[end..].to_vec();
        let replacement: Vec<&str> = params.content.lines().collect();
        let lines_inserted = replacement.len();

        // Build new content
        let mut new_lines = Vec::with_capacity(before.len() + replacement.len() + after.len());
        new_lines.extend(before);
        new_lines.extend(replacement);
        new_lines.extend(after);

        let new_total = new_lines.len();
        let new_content = new_lines.join("\n");

        // Add trailing newline if original had one
        let new_content = if content.ends_with('\n') {
            format!("{}\n", new_content)
        } else {
            new_content
        };

        debug!(
            "Replacing lines {}-{} with {} lines in {:?}",
            params.start_line, end, lines_inserted, canonical_path
        );

        fs::write(&canonical_path, &new_content)
            .await
            .map_err(SerenaError::Io)?;

        Ok(ReplaceLinesOutput {
            path: params.relative_path,
            lines_replaced,
            lines_inserted,
            new_total_lines: new_total,
        })
    }
}

#[async_trait]
impl Tool for ReplaceLinesTool {
    fn name(&self) -> &str {
        "replace_lines"
    }

    fn description(&self) -> &str {
        "Replace a range of lines in a file with new content. Line numbers are 1-based and inclusive."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "relative_path": {
                    "type": "string",
                    "description": "The relative path to the file"
                },
                "start_line": {
                    "type": "integer",
                    "description": "The 1-based start line number (inclusive)",
                    "minimum": 1
                },
                "end_line": {
                    "type": "integer",
                    "description": "The 1-based end line number (inclusive)",
                    "minimum": 1
                },
                "content": {
                    "type": "string",
                    "description": "The replacement content (can be multiple lines)"
                }
            },
            "required": ["relative_path", "start_line", "end_line", "content"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: ReplaceLinesParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        let output = self.replace_impl(params).await?;

        let message = format!(
            "Replaced {} lines with {} lines in '{}' (now {} lines)",
            output.lines_replaced, output.lines_inserted, output.path, output.new_total_lines
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
        vec!["file".to_string(), "edit".to_string(), "lines".to_string()]
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Validate that a path is within the project root
fn validate_path(
    full_path: &Path,
    project_root: &Path,
    relative_path: &str,
) -> Result<PathBuf, SerenaError> {
    let canonical_root = project_root
        .canonicalize()
        .map_err(|e| SerenaError::InvalidParameter(format!("Invalid project root: {}", e)))?;

    let canonical_path = full_path
        .canonicalize()
        .map_err(|_| SerenaError::NotFound(format!("File not found: {}", relative_path)))?;

    if !canonical_path.starts_with(&canonical_root) {
        return Err(SerenaError::Tool(ToolError::PermissionDenied(format!(
            "Path '{}' is outside project root",
            relative_path
        ))));
    }

    Ok(canonical_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_delete_lines() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Line 1\nLine 2\nLine 3\nLine 4\nLine 5")
            .await
            .unwrap();

        let tool = DeleteLinesTool::new(temp_dir.path());

        let params = json!({
            "relative_path": "test.txt",
            "start_line": 2,
            "end_line": 3
        });

        let result = tool.execute(params).await.unwrap();
        let data = result.data.unwrap();
        let output: DeleteLinesOutput = serde_json::from_value(data).unwrap();

        assert_eq!(output.lines_deleted, 2);
        assert_eq!(output.new_total_lines, 3);

        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content.trim(), "Line 1\nLine 4\nLine 5");
    }

    #[tokio::test]
    async fn test_insert_at_line() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Line 1\nLine 3")
            .await
            .unwrap();

        let tool = InsertAtLineTool::new(temp_dir.path());

        let params = json!({
            "relative_path": "test.txt",
            "line": 2,
            "content": "Line 2"
        });

        let result = tool.execute(params).await.unwrap();
        let data = result.data.unwrap();
        let output: InsertAtLineOutput = serde_json::from_value(data).unwrap();

        assert_eq!(output.lines_inserted, 1);
        assert_eq!(output.new_total_lines, 3);

        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content.trim(), "Line 1\nLine 2\nLine 3");
    }

    #[tokio::test]
    async fn test_insert_multiline() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Line 1\nLine 4")
            .await
            .unwrap();

        let tool = InsertAtLineTool::new(temp_dir.path());

        let params = json!({
            "relative_path": "test.txt",
            "line": 2,
            "content": "Line 2\nLine 3"
        });

        let result = tool.execute(params).await.unwrap();
        let data = result.data.unwrap();
        let output: InsertAtLineOutput = serde_json::from_value(data).unwrap();

        assert_eq!(output.lines_inserted, 2);
        assert_eq!(output.new_total_lines, 4);

        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content.trim(), "Line 1\nLine 2\nLine 3\nLine 4");
    }

    #[tokio::test]
    async fn test_replace_lines() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Line 1\nLine 2\nLine 3\nLine 4")
            .await
            .unwrap();

        let tool = ReplaceLinesTool::new(temp_dir.path());

        let params = json!({
            "relative_path": "test.txt",
            "start_line": 2,
            "end_line": 3,
            "content": "New Line 2"
        });

        let result = tool.execute(params).await.unwrap();
        let data = result.data.unwrap();
        let output: ReplaceLinesOutput = serde_json::from_value(data).unwrap();

        assert_eq!(output.lines_replaced, 2);
        assert_eq!(output.lines_inserted, 1);
        assert_eq!(output.new_total_lines, 3);

        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content.trim(), "Line 1\nNew Line 2\nLine 4");
    }

    #[tokio::test]
    async fn test_replace_with_more_lines() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Line 1\nLine 2\nLine 3")
            .await
            .unwrap();

        let tool = ReplaceLinesTool::new(temp_dir.path());

        let params = json!({
            "relative_path": "test.txt",
            "start_line": 2,
            "end_line": 2,
            "content": "New Line 2a\nNew Line 2b\nNew Line 2c"
        });

        let result = tool.execute(params).await.unwrap();
        let data = result.data.unwrap();
        let output: ReplaceLinesOutput = serde_json::from_value(data).unwrap();

        assert_eq!(output.lines_replaced, 1);
        assert_eq!(output.lines_inserted, 3);
        assert_eq!(output.new_total_lines, 5);

        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(
            content.trim(),
            "Line 1\nNew Line 2a\nNew Line 2b\nNew Line 2c\nLine 3"
        );
    }

    #[tokio::test]
    async fn test_delete_invalid_range() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Line 1\nLine 2")
            .await
            .unwrap();

        let tool = DeleteLinesTool::new(temp_dir.path());

        // Start line > end line
        let params = json!({
            "relative_path": "test.txt",
            "start_line": 3,
            "end_line": 1
        });

        let result = tool.execute(params).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_insert_at_zero_fails() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Line 1")
            .await
            .unwrap();

        let tool = InsertAtLineTool::new(temp_dir.path());

        let params = json!({
            "relative_path": "test.txt",
            "line": 0,
            "content": "New Line"
        });

        let result = tool.execute(params).await;
        assert!(result.is_err());
    }
}
