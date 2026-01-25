//! MCP Tool wrappers for memory operations
//!
//! These tools provide a consistent interface for AI agents to interact
//! with the memory system via the MCP protocol.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serena_core::{SerenaError, Tool, ToolResult};
use tracing::debug;

use crate::manager::{MemoryManager, ReplaceMode};

// ============================================================================
// WriteMemoryTool
// ============================================================================

/// Tool for writing/saving memories
pub struct WriteMemoryTool {
    manager: Arc<MemoryManager>,
}

#[derive(Debug, Deserialize)]
struct WriteMemoryParams {
    memory_name: String,
    content: String,
}

impl WriteMemoryTool {
    pub fn new(manager: Arc<MemoryManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for WriteMemoryTool {
    fn name(&self) -> &str {
        "write_memory"
    }

    fn description(&self) -> &str {
        "Writes content to a named memory file. Creates the memory if it doesn't exist, \
        or overwrites it if it does. Memories are stored as markdown files and indexed \
        in a SQLite database for fast searching."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "memory_name": {
                    "type": "string",
                    "description": "The name of the memory to write (without .md extension)"
                },
                "content": {
                    "type": "string",
                    "description": "The content to write to the memory file (markdown supported)"
                }
            },
            "required": ["memory_name", "content"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: WriteMemoryParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        debug!("Writing memory: {}", params.memory_name);

        let result = self
            .manager
            .save_memory(&params.memory_name, &params.content)
            .await
            .map_err(|e| SerenaError::Internal(e.to_string()))?;

        Ok(ToolResult::success(json!({
            "message": result,
            "memory_name": params.memory_name
        })))
    }

    fn can_edit(&self) -> bool {
        true
    }

    fn requires_project(&self) -> bool {
        true
    }

    fn tags(&self) -> Vec<String> {
        vec!["memory".to_string(), "write".to_string()]
    }
}

// ============================================================================
// ReadMemoryTool
// ============================================================================

/// Tool for reading memories
pub struct ReadMemoryTool {
    manager: Arc<MemoryManager>,
}

#[derive(Debug, Deserialize)]
struct ReadMemoryParams {
    memory_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReadMemoryOutput {
    memory_name: String,
    content: String,
    exists: bool,
}

impl ReadMemoryTool {
    pub fn new(manager: Arc<MemoryManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for ReadMemoryTool {
    fn name(&self) -> &str {
        "read_memory"
    }

    fn description(&self) -> &str {
        "Reads the content of a named memory file. Returns the content if the memory exists, \
        or a helpful message if it doesn't exist suggesting to create it."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "memory_name": {
                    "type": "string",
                    "description": "The name of the memory to read (without .md extension)"
                }
            },
            "required": ["memory_name"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: ReadMemoryParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        debug!("Reading memory: {}", params.memory_name);

        let content = self
            .manager
            .load_memory(&params.memory_name)
            .await
            .map_err(|e| SerenaError::Internal(e.to_string()))?;

        let exists = !content.contains("not found");

        Ok(ToolResult::success(
            serde_json::to_value(ReadMemoryOutput {
                memory_name: params.memory_name,
                content,
                exists,
            })
            .map_err(|e| SerenaError::Internal(e.to_string()))?,
        ))
    }

    fn can_edit(&self) -> bool {
        false
    }

    fn requires_project(&self) -> bool {
        true
    }

    fn tags(&self) -> Vec<String> {
        vec!["memory".to_string(), "read".to_string()]
    }
}

// ============================================================================
// ListMemoriesTool
// ============================================================================

/// Tool for listing all available memories
pub struct ListMemoriesTool {
    manager: Arc<MemoryManager>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListMemoriesOutput {
    memories: Vec<String>,
    count: usize,
}

impl ListMemoriesTool {
    pub fn new(manager: Arc<MemoryManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for ListMemoriesTool {
    fn name(&self) -> &str {
        "list_memories"
    }

    fn description(&self) -> &str {
        "Lists all available memory files in the project. Returns the names of all memories \
        (without .md extension) sorted alphabetically."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _params: Value) -> Result<ToolResult, SerenaError> {
        debug!("Listing memories");

        let memories = self
            .manager
            .list_memories()
            .await
            .map_err(|e| SerenaError::Internal(e.to_string()))?;

        let count = memories.len();

        Ok(ToolResult::success(
            serde_json::to_value(ListMemoriesOutput { memories, count })
                .map_err(|e| SerenaError::Internal(e.to_string()))?,
        ))
    }

    fn can_edit(&self) -> bool {
        false
    }

    fn requires_project(&self) -> bool {
        true
    }

    fn tags(&self) -> Vec<String> {
        vec!["memory".to_string(), "list".to_string()]
    }
}

// ============================================================================
// DeleteMemoryTool
// ============================================================================

/// Tool for deleting memories
pub struct DeleteMemoryTool {
    manager: Arc<MemoryManager>,
}

#[derive(Debug, Deserialize)]
struct DeleteMemoryParams {
    memory_name: String,
}

impl DeleteMemoryTool {
    pub fn new(manager: Arc<MemoryManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for DeleteMemoryTool {
    fn name(&self) -> &str {
        "delete_memory"
    }

    fn description(&self) -> &str {
        "Deletes a named memory file. Removes both the markdown file and the database entry. \
        Use with caution as this operation cannot be undone."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "memory_name": {
                    "type": "string",
                    "description": "The name of the memory to delete (without .md extension)"
                }
            },
            "required": ["memory_name"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: DeleteMemoryParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        debug!("Deleting memory: {}", params.memory_name);

        let result = self
            .manager
            .delete_memory(&params.memory_name)
            .await
            .map_err(|e| SerenaError::Internal(e.to_string()))?;

        Ok(ToolResult::success(json!({
            "message": result,
            "memory_name": params.memory_name
        })))
    }

    fn can_edit(&self) -> bool {
        true
    }

    fn requires_project(&self) -> bool {
        true
    }

    fn tags(&self) -> Vec<String> {
        vec!["memory".to_string(), "delete".to_string()]
    }
}

// ============================================================================
// EditMemoryTool
// ============================================================================

/// Tool for editing memory content using find/replace
pub struct EditMemoryTool {
    manager: Arc<MemoryManager>,
}

#[derive(Debug, Deserialize)]
struct EditMemoryParams {
    memory_name: String,
    needle: String,
    replacement: String,
    #[serde(default)]
    use_regex: bool,
}

impl EditMemoryTool {
    pub fn new(manager: Arc<MemoryManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for EditMemoryTool {
    fn name(&self) -> &str {
        "edit_memory"
    }

    fn description(&self) -> &str {
        "Edits a memory file by replacing content. Supports both literal string replacement \
        and regex pattern matching. Returns the updated memory content."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "memory_name": {
                    "type": "string",
                    "description": "The name of the memory to edit (without .md extension)"
                },
                "needle": {
                    "type": "string",
                    "description": "The text or regex pattern to find"
                },
                "replacement": {
                    "type": "string",
                    "description": "The text to replace matches with"
                },
                "use_regex": {
                    "type": "boolean",
                    "description": "If true, treat needle as a regex pattern. Default: false",
                    "default": false
                }
            },
            "required": ["memory_name", "needle", "replacement"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: EditMemoryParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        debug!(
            "Editing memory: {} (regex: {})",
            params.memory_name, params.use_regex
        );

        let mode = if params.use_regex {
            ReplaceMode::Regex
        } else {
            ReplaceMode::Literal
        };

        let result = self
            .manager
            .replace_content(&params.memory_name, &params.needle, &params.replacement, mode)
            .await
            .map_err(|e| SerenaError::Internal(e.to_string()))?;

        Ok(ToolResult::success(json!({
            "message": result,
            "memory_name": params.memory_name
        })))
    }

    fn can_edit(&self) -> bool {
        true
    }

    fn requires_project(&self) -> bool {
        true
    }

    fn tags(&self) -> Vec<String> {
        vec!["memory".to_string(), "edit".to_string()]
    }
}

// ============================================================================
// SearchMemoriesTool
// ============================================================================

/// Tool for searching across all memories
pub struct SearchMemoriesTool {
    manager: Arc<MemoryManager>,
}

#[derive(Debug, Deserialize)]
struct SearchMemoriesParams {
    query: String,
}

#[derive(Debug, Serialize)]
struct SearchResult {
    name: String,
    preview: String,
}

#[derive(Debug, Serialize)]
struct SearchMemoriesOutput {
    results: Vec<SearchResult>,
    count: usize,
}

impl SearchMemoriesTool {
    pub fn new(manager: Arc<MemoryManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for SearchMemoriesTool {
    fn name(&self) -> &str {
        "search_memories"
    }

    fn description(&self) -> &str {
        "Searches across all memories for matching content. Returns a list of memory names \
        that contain the search query, with content previews."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query to find in memory content"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: SearchMemoriesParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        debug!("Searching memories for: {}", params.query);

        let metadata_results = self
            .manager
            .search(&params.query)
            .await
            .map_err(|e| SerenaError::Internal(e.to_string()))?;

        let mut results = Vec::new();
        for metadata in metadata_results {
            // Get a preview of the content
            let content = self
                .manager
                .load_memory(&metadata.name)
                .await
                .map_err(|e| SerenaError::Internal(e.to_string()))?;

            // Create a preview (first 200 chars or up to first newline after query match)
            let preview = content.chars().take(200).collect::<String>();

            results.push(SearchResult {
                name: metadata.name,
                preview,
            });
        }

        let count = results.len();

        Ok(ToolResult::success(
            serde_json::to_value(SearchMemoriesOutput { results, count })
                .map_err(|e| SerenaError::Internal(e.to_string()))?,
        ))
    }

    fn can_edit(&self) -> bool {
        false
    }

    fn requires_project(&self) -> bool {
        true
    }

    fn tags(&self) -> Vec<String> {
        vec!["memory".to_string(), "search".to_string()]
    }
}

// ============================================================================
// Factory functions
// ============================================================================

/// Create all memory tools with a shared MemoryManager
pub fn create_memory_tools(manager: Arc<MemoryManager>) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(WriteMemoryTool::new(Arc::clone(&manager))),
        Arc::new(ReadMemoryTool::new(Arc::clone(&manager))),
        Arc::new(ListMemoriesTool::new(Arc::clone(&manager))),
        Arc::new(DeleteMemoryTool::new(Arc::clone(&manager))),
        Arc::new(EditMemoryTool::new(Arc::clone(&manager))),
        Arc::new(SearchMemoriesTool::new(manager)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, TempDir};

    // Helper to create a test manager, keeping the TempDir alive
    fn create_test_manager() -> (TempDir, Arc<MemoryManager>) {
        let dir = tempdir().unwrap();
        let manager = Arc::new(MemoryManager::new(dir.path()).unwrap());
        (dir, manager)
    }

    #[tokio::test]
    async fn test_write_memory_tool() {
        let (_dir, manager) = create_test_manager();
        let tool = WriteMemoryTool::new(Arc::clone(&manager));

        assert_eq!(tool.name(), "write_memory");
        assert!(tool.can_edit());

        let result = tool
            .execute(json!({
                "memory_name": "test-note",
                "content": "# Test Note\n\nThis is a test."
            }))
            .await
            .unwrap();

        assert_eq!(result.status, serena_core::ToolStatus::Success);

        // Verify the memory was written
        let content = manager.load_memory("test-note").await.unwrap();
        assert!(content.contains("Test Note"));
    }

    #[tokio::test]
    async fn test_read_memory_tool() {
        let (_dir, manager) = create_test_manager();
        manager
            .save_memory("my-note", "Hello World")
            .await
            .unwrap();

        let tool = ReadMemoryTool::new(Arc::clone(&manager));

        assert_eq!(tool.name(), "read_memory");
        assert!(!tool.can_edit());

        let result = tool
            .execute(json!({ "memory_name": "my-note" }))
            .await
            .unwrap();

        assert_eq!(result.status, serena_core::ToolStatus::Success);

        let data = result.data.unwrap();
        let output: ReadMemoryOutput = serde_json::from_value(data).unwrap();
        assert!(output.exists);
        assert_eq!(output.content, "Hello World");
    }

    #[tokio::test]
    async fn test_list_memories_tool() {
        let (_dir, manager) = create_test_manager();
        manager.save_memory("note-1", "Content 1").await.unwrap();
        manager.save_memory("note-2", "Content 2").await.unwrap();

        let tool = ListMemoriesTool::new(manager);

        let result = tool.execute(json!({})).await.unwrap();

        assert_eq!(result.status, serena_core::ToolStatus::Success);

        let data = result.data.unwrap();
        let output: ListMemoriesOutput = serde_json::from_value(data).unwrap();
        assert_eq!(output.count, 2);
        assert!(output.memories.contains(&"note-1".to_string()));
        assert!(output.memories.contains(&"note-2".to_string()));
    }

    #[tokio::test]
    async fn test_delete_memory_tool() {
        let (_dir, manager) = create_test_manager();
        manager.save_memory("to-delete", "Bye").await.unwrap();
        assert!(manager.exists("to-delete").await);

        let tool = DeleteMemoryTool::new(Arc::clone(&manager));

        let result = tool
            .execute(json!({ "memory_name": "to-delete" }))
            .await
            .unwrap();

        assert_eq!(result.status, serena_core::ToolStatus::Success);
        assert!(!manager.exists("to-delete").await);
    }

    #[tokio::test]
    async fn test_edit_memory_tool() {
        let (_dir, manager) = create_test_manager();
        manager
            .save_memory("editable", "Hello World")
            .await
            .unwrap();

        let tool = EditMemoryTool::new(Arc::clone(&manager));

        let result = tool
            .execute(json!({
                "memory_name": "editable",
                "needle": "World",
                "replacement": "Rust"
            }))
            .await
            .unwrap();

        assert_eq!(result.status, serena_core::ToolStatus::Success);

        let content = manager.load_memory("editable").await.unwrap();
        assert_eq!(content, "Hello Rust");
    }

    #[tokio::test]
    async fn test_edit_memory_with_regex() {
        let (_dir, manager) = create_test_manager();
        manager
            .save_memory("regex-test", "Version 1.0.0")
            .await
            .unwrap();

        let tool = EditMemoryTool::new(Arc::clone(&manager));

        let result = tool
            .execute(json!({
                "memory_name": "regex-test",
                "needle": r"\d+\.\d+\.\d+",
                "replacement": "2.0.0",
                "use_regex": true
            }))
            .await
            .unwrap();

        assert_eq!(result.status, serena_core::ToolStatus::Success);

        let content = manager.load_memory("regex-test").await.unwrap();
        assert_eq!(content, "Version 2.0.0");
    }

    #[tokio::test]
    async fn test_create_memory_tools() {
        let (_dir, manager) = create_test_manager();
        let tools = create_memory_tools(manager);

        assert_eq!(tools.len(), 6);

        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"write_memory"));
        assert!(names.contains(&"read_memory"));
        assert!(names.contains(&"list_memories"));
        assert!(names.contains(&"delete_memory"));
        assert!(names.contains(&"edit_memory"));
        assert!(names.contains(&"search_memories"));
    }
}
