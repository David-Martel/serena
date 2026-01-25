use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serena_core::{SerenaError, Tool, ToolError, ToolResult};
use std::path::PathBuf;
use tokio::fs;
use tracing::debug;

/// Tool for creating or overwriting files
pub struct CreateTextFileTool {
    project_root: PathBuf,
}

#[derive(Debug, Deserialize)]
struct CreateTextFileParams {
    relative_path: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateTextFileOutput {
    path: String,
    bytes_written: usize,
    created: bool, // true if new file, false if overwritten
}

impl CreateTextFileTool {
    /// Create a new CreateTextFileTool with the given project root
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }

    async fn write_file_impl(
        &self,
        params: CreateTextFileParams,
    ) -> Result<CreateTextFileOutput, SerenaError> {
        // Validate and construct full path
        let full_path = self.project_root.join(&params.relative_path);

        // Security check: ensure path is within project root
        let canonical_root = self
            .project_root
            .canonicalize()
            .map_err(|e| SerenaError::InvalidParameter(format!("Invalid project root: {}", e)))?;

        // Get parent directory for validation
        let parent = full_path.parent().ok_or_else(|| {
            SerenaError::InvalidParameter("Invalid file path: no parent directory".to_string())
        })?;

        // Check if file exists before writing
        let file_exists = full_path.exists();

        // Create parent directories if needed
        if !parent.exists() {
            debug!("Creating parent directories: {:?}", parent);
            fs::create_dir_all(parent)
                .await
                .map_err(SerenaError::Io)?;
        }

        // After creating directories, validate the path is still within project root
        let canonical_parent = parent.canonicalize().map_err(|e| {
            SerenaError::InvalidParameter(format!("Cannot canonicalize parent directory: {}", e))
        })?;

        if !canonical_parent.starts_with(&canonical_root) {
            return Err(SerenaError::Tool(ToolError::PermissionDenied(format!(
                "Path '{}' is outside project root",
                params.relative_path
            ))));
        }

        // Write file content
        debug!(
            "Writing file: {:?} ({} bytes)",
            full_path,
            params.content.len()
        );
        fs::write(&full_path, &params.content)
            .await
            .map_err(SerenaError::Io)?;

        Ok(CreateTextFileOutput {
            path: params.relative_path,
            bytes_written: params.content.len(),
            created: !file_exists,
        })
    }
}

#[async_trait]
impl Tool for CreateTextFileTool {
    fn name(&self) -> &str {
        "create_text_file"
    }

    fn description(&self) -> &str {
        "Writes a file to the local filesystem. Creates parent directories if needed. \
        Overwrites existing files."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "relative_path": {
                    "type": "string",
                    "description": "The relative path to the file to create or overwrite"
                },
                "content": {
                    "type": "string",
                    "description": "The content to write to the file"
                }
            },
            "required": ["relative_path", "content"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: CreateTextFileParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        let output = self.write_file_impl(params).await?;

        let message = if output.created {
            format!(
                "Created file '{}' ({} bytes)",
                output.path, output.bytes_written
            )
        } else {
            format!(
                "Overwrote file '{}' ({} bytes)",
                output.path, output.bytes_written
            )
        };

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
        vec!["file".to_string(), "write".to_string(), "edit".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let tool = CreateTextFileTool::new(temp_dir.path());

        let params = json!({
            "relative_path": "new_file.txt",
            "content": "Hello, World!"
        });

        let result = tool.execute(params).await.unwrap();
        assert_eq!(result.status, serena_core::ToolStatus::Success);

        let data = result.data.unwrap();
        let output: CreateTextFileOutput = serde_json::from_value(data).unwrap();
        assert!(output.created);
        assert_eq!(output.bytes_written, 13);

        // Verify file was created
        let file_path = temp_dir.path().join("new_file.txt");
        assert!(file_path.exists());
        let content = fs::read_to_string(file_path).await.unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_overwrite_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("existing.txt");
        fs::write(&file_path, "Old content").await.unwrap();

        let tool = CreateTextFileTool::new(temp_dir.path());

        let params = json!({
            "relative_path": "existing.txt",
            "content": "New content"
        });

        let result = tool.execute(params).await.unwrap();
        let data = result.data.unwrap();
        let output: CreateTextFileOutput = serde_json::from_value(data).unwrap();
        assert!(!output.created);

        // Verify file was overwritten
        let content = fs::read_to_string(file_path).await.unwrap();
        assert_eq!(content, "New content");
    }

    #[tokio::test]
    async fn test_create_with_nested_directories() {
        let temp_dir = TempDir::new().unwrap();
        let tool = CreateTextFileTool::new(temp_dir.path());

        let params = json!({
            "relative_path": "nested/deep/file.txt",
            "content": "Nested file"
        });

        let result = tool.execute(params).await.unwrap();
        assert_eq!(result.status, serena_core::ToolStatus::Success);

        // Verify directories were created
        let file_path = temp_dir.path().join("nested/deep/file.txt");
        assert!(file_path.exists());
    }

    #[tokio::test]
    async fn test_path_traversal_prevention() {
        let temp_dir = TempDir::new().unwrap();
        let tool = CreateTextFileTool::new(temp_dir.path());

        let params = json!({
            "relative_path": "../../../etc/passwd",
            "content": "malicious"
        });

        let result = tool.execute(params).await;
        assert!(result.is_err());
    }
}
