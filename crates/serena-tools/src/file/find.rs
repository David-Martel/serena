use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serena_core::{SerenaError, Tool, ToolResult};
use std::path::PathBuf;
use tracing::debug;

/// Tool for finding files matching a glob pattern
pub struct FindFileTool {
    project_root: PathBuf,
}

#[derive(Debug, Deserialize)]
struct FindFileParams {
    /// The glob pattern to match (e.g., "*.rs", "**/*.py")
    file_mask: String,
    /// The relative path to search in. Use "." for project root
    #[serde(default = "default_search_path")]
    relative_path: String,
    /// Maximum number of results to return (default: 1000)
    #[serde(default = "default_max_results")]
    max_results: usize,
}

fn default_search_path() -> String {
    ".".to_string()
}

fn default_max_results() -> usize {
    1000
}

#[derive(Debug, Serialize, Deserialize)]
struct FindFileOutput {
    files: Vec<String>,
    total_found: usize,
    truncated: bool,
}

impl FindFileTool {
    /// Create a new FindFileTool with the given project root
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }

    async fn find_files_impl(&self, params: FindFileParams) -> Result<FindFileOutput, SerenaError> {
        let search_path = self.project_root.join(&params.relative_path);

        // Construct the glob pattern
        let pattern = search_path
            .join(&params.file_mask)
            .to_string_lossy()
            .to_string();

        debug!(
            "Finding files matching pattern: {} in {:?}",
            params.file_mask, search_path
        );

        let mut matches = Vec::new();
        let mut total_found = 0;

        // Use glob to find matching files
        for entry in glob::glob(&pattern)
            .map_err(|e| SerenaError::InvalidParameter(format!("Invalid glob pattern: {}", e)))?
        {
            match entry {
                Ok(path) => {
                    total_found += 1;
                    if matches.len() < params.max_results {
                        let relative_path = path
                            .strip_prefix(&self.project_root)
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .replace('\\', "/");
                        matches.push(relative_path);
                    }
                }
                Err(e) => {
                    debug!("Glob error for entry: {}", e);
                }
            }
        }

        Ok(FindFileOutput {
            files: matches,
            total_found,
            truncated: total_found > params.max_results,
        })
    }
}

#[async_trait]
impl Tool for FindFileTool {
    fn name(&self) -> &str {
        "find_file"
    }

    fn description(&self) -> &str {
        "Find files matching a glob pattern within the project directory. \
        Supports patterns like '*.rs', '**/*.py', 'src/**/*.ts'."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_mask": {
                    "type": "string",
                    "description": "The glob pattern to match (e.g., '*.rs', '**/*.py')"
                },
                "relative_path": {
                    "type": "string",
                    "description": "The relative path to search in. Use '.' for project root (default: '.')"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 1000)"
                }
            },
            "required": ["file_mask"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: FindFileParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        let output = self.find_files_impl(params).await?;

        let message = if output.truncated {
            format!(
                "Found {} files (showing first {})",
                output.total_found,
                output.files.len()
            )
        } else {
            format!("Found {} files", output.total_found)
        };

        Ok(ToolResult::success_with_message(
            serde_json::to_value(output)?,
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
        vec!["file".to_string(), "find".to_string(), "search".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs::{create_dir_all, write};

    async fn setup_test_tree() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create directory structure
        create_dir_all(root.join("src")).await.unwrap();
        create_dir_all(root.join("src/nested")).await.unwrap();
        create_dir_all(root.join("tests")).await.unwrap();

        // Create files with different extensions
        write(root.join("main.rs"), "fn main() {}").await.unwrap();
        write(root.join("src/lib.rs"), "// lib").await.unwrap();
        write(root.join("src/utils.rs"), "// utils").await.unwrap();
        write(root.join("src/nested/deep.rs"), "// deep")
            .await
            .unwrap();
        write(root.join("tests/test.rs"), "// test").await.unwrap();
        write(root.join("readme.md"), "# Readme").await.unwrap();
        write(root.join("config.toml"), "[config]").await.unwrap();

        temp_dir
    }

    #[tokio::test]
    async fn test_find_all_rust_files() {
        let temp_dir = setup_test_tree().await;
        let tool = FindFileTool::new(temp_dir.path());

        let params = json!({
            "file_mask": "**/*.rs"
        });

        let result = tool.execute(params).await.unwrap();
        let data = result.data.unwrap();
        let output: FindFileOutput = serde_json::from_value(data).unwrap();

        assert_eq!(output.total_found, 5);
        assert!(!output.truncated);
    }

    #[tokio::test]
    async fn test_find_in_subdirectory() {
        let temp_dir = setup_test_tree().await;
        let tool = FindFileTool::new(temp_dir.path());

        let params = json!({
            "file_mask": "*.rs",
            "relative_path": "src"
        });

        let result = tool.execute(params).await.unwrap();
        let data = result.data.unwrap();
        let output: FindFileOutput = serde_json::from_value(data).unwrap();

        assert_eq!(output.total_found, 2);
    }

    #[tokio::test]
    async fn test_find_with_max_results() {
        let temp_dir = setup_test_tree().await;
        let tool = FindFileTool::new(temp_dir.path());

        let params = json!({
            "file_mask": "**/*.rs",
            "max_results": 2
        });

        let result = tool.execute(params).await.unwrap();
        let data = result.data.unwrap();
        let output: FindFileOutput = serde_json::from_value(data).unwrap();

        assert_eq!(output.files.len(), 2);
        assert_eq!(output.total_found, 5);
        assert!(output.truncated);
    }

    #[tokio::test]
    async fn test_find_specific_file() {
        let temp_dir = setup_test_tree().await;
        let tool = FindFileTool::new(temp_dir.path());

        let params = json!({
            "file_mask": "*.md"
        });

        let result = tool.execute(params).await.unwrap();
        let data = result.data.unwrap();
        let output: FindFileOutput = serde_json::from_value(data).unwrap();

        assert_eq!(output.total_found, 1);
        assert!(output.files.iter().any(|f| f.contains("readme.md")));
    }
}
