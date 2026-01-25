use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serena_core::{SerenaError, Tool, ToolError, ToolResult};
use std::path::PathBuf;
use tokio::fs;
use tracing::debug;

/// Tool for listing directory contents
pub struct ListDirectoryTool {
    project_root: PathBuf,
}

#[derive(Debug, Deserialize)]
struct ListDirectoryParams {
    relative_path: String,
    #[serde(default)]
    recursive: bool,
    /// Maximum characters to return. -1 for unlimited. Default: -1
    #[serde(default = "default_max_chars")]
    max_answer_chars: i32,
}

fn default_max_chars() -> i32 {
    -1
}

#[derive(Debug, Serialize, Deserialize)]
struct DirectoryEntry {
    name: String,
    path: String,
    is_file: bool,
    is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListDirectoryOutput {
    path: String,
    entries: Vec<DirectoryEntry>,
    total_files: usize,
    total_dirs: usize,
}

impl ListDirectoryTool {
    /// Create a new ListDirectoryTool with the given project root
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }

    async fn list_dir_impl(
        &self,
        params: ListDirectoryParams,
    ) -> Result<ListDirectoryOutput, SerenaError> {
        // Validate and construct full path
        let full_path = self.project_root.join(&params.relative_path);

        // Security check: ensure path is within project root
        let canonical_root = self
            .project_root
            .canonicalize()
            .map_err(|e| SerenaError::InvalidParameter(format!("Invalid project root: {}", e)))?;

        let canonical_path = full_path.canonicalize().map_err(|_e| {
            SerenaError::NotFound(format!("Directory not found: {}", params.relative_path))
        })?;

        if !canonical_path.starts_with(&canonical_root) {
            return Err(SerenaError::Tool(ToolError::PermissionDenied(format!(
                "Path '{}' is outside project root",
                params.relative_path
            ))));
        }

        // Check if it's a directory
        let metadata = fs::metadata(&canonical_path)
            .await
            .map_err(SerenaError::Io)?;

        if !metadata.is_dir() {
            return Err(SerenaError::InvalidParameter(format!(
                "'{}' is not a directory",
                params.relative_path
            )));
        }

        debug!(
            "Listing directory: {:?} (recursive: {})",
            canonical_path, params.recursive
        );

        let mut entries = Vec::new();
        let mut total_files = 0;
        let mut total_dirs = 0;

        if params.recursive {
            self.walk_recursive(
                &canonical_path,
                &mut entries,
                &mut total_files,
                &mut total_dirs,
            )
            .await?;
        } else {
            self.list_single_level(
                &canonical_path,
                &mut entries,
                &mut total_files,
                &mut total_dirs,
            )
            .await?;
        }

        // Sort entries: directories first, then by name
        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

        Ok(ListDirectoryOutput {
            path: params.relative_path,
            entries,
            total_files,
            total_dirs,
        })
    }

    async fn list_single_level(
        &self,
        dir_path: &PathBuf,
        entries: &mut Vec<DirectoryEntry>,
        total_files: &mut usize,
        total_dirs: &mut usize,
    ) -> Result<(), SerenaError> {
        let mut read_dir = fs::read_dir(dir_path)
            .await
            .map_err(SerenaError::Io)?;

        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(SerenaError::Io)?
        {
            let path = entry.path();
            let metadata = entry.metadata().await.map_err(SerenaError::Io)?;

            let relative_path = path
                .strip_prefix(&self.project_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");

            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let is_file = metadata.is_file();
            let is_dir = metadata.is_dir();

            if is_file {
                *total_files += 1;
            } else if is_dir {
                *total_dirs += 1;
            }

            entries.push(DirectoryEntry {
                name,
                path: relative_path,
                is_file,
                is_dir,
                size: if is_file { Some(metadata.len()) } else { None },
            });
        }

        Ok(())
    }

    async fn walk_recursive(
        &self,
        dir_path: &PathBuf,
        entries: &mut Vec<DirectoryEntry>,
        total_files: &mut usize,
        total_dirs: &mut usize,
    ) -> Result<(), SerenaError> {
        use walkdir::WalkDir;

        for entry in WalkDir::new(dir_path).follow_links(true) {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();

            // Skip the root directory itself
            if path == dir_path {
                continue;
            }

            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let relative_path = path
                .strip_prefix(&self.project_root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");

            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let is_file = metadata.is_file();
            let is_dir = metadata.is_dir();

            if is_file {
                *total_files += 1;
            } else if is_dir {
                *total_dirs += 1;
            }

            entries.push(DirectoryEntry {
                name,
                path: relative_path,
                is_file,
                is_dir,
                size: if is_file { Some(metadata.len()) } else { None },
            });
        }

        Ok(())
    }
}

#[async_trait]
impl Tool for ListDirectoryTool {
    fn name(&self) -> &str {
        "list_directory"
    }

    fn description(&self) -> &str {
        "List contents of a directory. Supports recursive listing."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "relative_path": {
                    "type": "string",
                    "description": "The relative path to the directory to list"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "Whether to recursively list subdirectories (default: false)"
                }
            },
            "required": ["relative_path"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: ListDirectoryParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        let max_answer_chars = params.max_answer_chars;
        let output = self.list_dir_impl(params).await?;

        let message = format!(
            "Listed {} files and {} directories in '{}'",
            output.total_files, output.total_dirs, output.path
        );

        // Apply character limit truncation
        let json_output = serde_json::to_value(&output)?;
        let output_str = json_output.to_string();
        let final_output = if max_answer_chars < 0 {
            output_str
        } else {
            let max = max_answer_chars as usize;
            if output_str.len() > max {
                let truncated = output_str.chars().take(max).collect::<String>();
                format!(
                    "{}...
[Output truncated at {} chars. {} chars total.]",
                    truncated,
                    max,
                    output_str.len()
                )
            } else {
                output_str
            }
        };

        Ok(ToolResult::success_with_message(
            serde_json::from_str(&final_output).unwrap_or(json_output),
            message,
        ))
    }

    fn can_edit(&self) -> bool {
        false
    }

    fn requires_project(&self) -> bool {
        true
    }

    fn tags(&self) -> Vec<String> {
        vec![
            "file".to_string(),
            "directory".to_string(),
            "list".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs::{create_dir, write};

    async fn setup_test_tree() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create directory structure
        create_dir(root.join("dir1")).await.unwrap();
        create_dir(root.join("dir2")).await.unwrap();
        create_dir(root.join("dir1/subdir")).await.unwrap();

        // Create files
        write(root.join("file1.txt"), "content1").await.unwrap();
        write(root.join("file2.txt"), "content2").await.unwrap();
        write(root.join("dir1/file3.txt"), "content3")
            .await
            .unwrap();
        write(root.join("dir1/subdir/file4.txt"), "content4")
            .await
            .unwrap();

        temp_dir
    }

    #[tokio::test]
    async fn test_list_non_recursive() {
        let temp_dir = setup_test_tree().await;
        let tool = ListDirectoryTool::new(temp_dir.path());

        let params = json!({
            "relative_path": ".",
            "recursive": false
        });

        let result = tool.execute(params).await.unwrap();
        let data = result.data.unwrap();
        let output: ListDirectoryOutput = serde_json::from_value(data).unwrap();

        assert_eq!(output.total_files, 2);
        assert_eq!(output.total_dirs, 2);
        assert_eq!(output.entries.len(), 4);
    }

    #[tokio::test]
    async fn test_list_recursive() {
        let temp_dir = setup_test_tree().await;
        let tool = ListDirectoryTool::new(temp_dir.path());

        let params = json!({
            "relative_path": ".",
            "recursive": true
        });

        let result = tool.execute(params).await.unwrap();
        let data = result.data.unwrap();
        let output: ListDirectoryOutput = serde_json::from_value(data).unwrap();

        assert_eq!(output.total_files, 4);
        assert_eq!(output.total_dirs, 3);
    }

    #[tokio::test]
    async fn test_list_subdirectory() {
        let temp_dir = setup_test_tree().await;
        let tool = ListDirectoryTool::new(temp_dir.path());

        let params = json!({
            "relative_path": "dir1",
            "recursive": false
        });

        let result = tool.execute(params).await.unwrap();
        let data = result.data.unwrap();
        let output: ListDirectoryOutput = serde_json::from_value(data).unwrap();

        assert_eq!(output.total_files, 1);
        assert_eq!(output.total_dirs, 1);
    }

    #[tokio::test]
    async fn test_path_traversal_prevention() {
        let temp_dir = TempDir::new().unwrap();
        let tool = ListDirectoryTool::new(temp_dir.path());

        let params = json!({
            "relative_path": "../../../etc",
            "recursive": false
        });

        let result = tool.execute(params).await;
        assert!(result.is_err());
    }
}
