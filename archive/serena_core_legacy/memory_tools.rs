//! Memory tools for Serena MCP server
//!
//! These tools provide persistent knowledge storage for project-specific
//! information across sessions.

use regex::RegexBuilder;
use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::mcp_2025_06_18::schema_utils::CallToolError;
use rust_mcp_sdk::schema::{CallToolResult, TextContent};

use super::services::MemoryService;
use super::utils::truncate_output;

// ============================================================================
// write_memory
// ============================================================================

#[mcp_tool(
    name = "write_memory",
    description = "Write project knowledge to a memory file for future reference. Memories are stored as markdown files.",
    destructive_hint = false,
    idempotent_hint = false,
    read_only_hint = false
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct WriteMemoryTool {
    /// The name for the memory file (without .md extension).
    pub memory_file_name: String,

    /// The content to write to the memory.
    pub content: String,
}

fn default_max_chars() -> i32 {
    -1
}

impl WriteMemoryTool {
    pub async fn run_tool(self, service: &MemoryService) -> Result<CallToolResult, CallToolError> {
        let len = self.content.len();
        service
            .write(&self.memory_file_name, &self.content)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            format!(
                "Memory '{}' written successfully ({len} bytes)",
                self.memory_file_name
            ),
        )]))
    }
}

// ============================================================================
// read_memory
// ============================================================================

#[mcp_tool(
    name = "read_memory",
    description = "Read the content of a memory file. Only use if the information is relevant to the current task.",
    destructive_hint = false,
    idempotent_hint = true,
    read_only_hint = true
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct ReadMemoryTool {
    /// The name of the memory file to read (without .md extension).
    pub memory_file_name: String,

    /// Maximum characters to return. -1 for unlimited. Default: -1
    #[serde(default = "default_max_chars")]
    pub max_answer_chars: i32,
}

impl ReadMemoryTool {
    pub async fn run_tool(self, service: &MemoryService) -> Result<CallToolResult, CallToolError> {
        let content = service
            .read(&self.memory_file_name)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let final_result = truncate_output(content, self.max_answer_chars);

        Ok(CallToolResult::text_content(vec![TextContent::from(
            final_result,
        )]))
    }
}

// ============================================================================
// list_memories
// ============================================================================

#[mcp_tool(
    name = "list_memories",
    description = "List all available memory files for this project.",
    destructive_hint = false,
    idempotent_hint = true,
    read_only_hint = true
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct ListMemoriesTool {
    /// Maximum characters to return. -1 for unlimited. Default: -1
    #[serde(default = "default_max_chars")]
    pub max_answer_chars: i32,
}

impl ListMemoriesTool {
    pub async fn run_tool(self, service: &MemoryService) -> Result<CallToolResult, CallToolError> {
        let memories = service
            .list()
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let result = serde_json::json!({
            "memories": memories,
            "count": memories.len()
        });

        let result_str = result.to_string();
        let final_result = truncate_output(result_str, self.max_answer_chars);

        Ok(CallToolResult::text_content(vec![TextContent::from(
            final_result,
        )]))
    }
}

// ============================================================================
// delete_memory
// ============================================================================

#[mcp_tool(
    name = "delete_memory",
    description = "Delete a memory file. Should only be used when explicitly requested by the user.",
    destructive_hint = true,
    idempotent_hint = false,
    read_only_hint = false
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct DeleteMemoryTool {
    /// The name of the memory file to delete (without .md extension).
    pub memory_file_name: String,
}

impl DeleteMemoryTool {
    pub async fn run_tool(self, service: &MemoryService) -> Result<CallToolResult, CallToolError> {
        service
            .delete(&self.memory_file_name)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            format!("Memory '{}' deleted successfully", self.memory_file_name),
        )]))
    }
}

// ============================================================================
// edit_memory
// ============================================================================

#[mcp_tool(
    name = "edit_memory",
    description = "Edit a memory file using literal or regex replacement.",
    destructive_hint = true,
    idempotent_hint = false,
    read_only_hint = false
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct EditMemoryTool {
    /// The name of the memory file to edit.
    pub memory_file_name: String,

    /// The string or regex pattern to search for.
    pub needle: String,

    /// The replacement string.
    pub repl: String,

    /// Either "literal" or "regex".
    pub mode: String,
}

impl EditMemoryTool {
    pub async fn run_tool(self, service: &MemoryService) -> Result<CallToolResult, CallToolError> {
        let content = service
            .read(&self.memory_file_name)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let new_content = match self.mode.as_str() {
            "regex" => {
                let re = RegexBuilder::new(&self.needle)
                    .dot_matches_new_line(true)
                    .multi_line(true)
                    .build()
                    .map_err(|e| CallToolError::from_message(e.to_string()))?;

                if !re.is_match(&content) {
                    return Err(CallToolError::from_message("Pattern not found in memory"));
                }
                re.replace_all(&content, &self.repl).into_owned()
            }
            _ => {
                if !content.contains(&self.needle) {
                    return Err(CallToolError::from_message("Pattern not found in memory"));
                }
                content.replace(&self.needle, &self.repl)
            }
        };

        service
            .write(&self.memory_file_name, &new_content)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            format!("Memory '{}' edited successfully", self.memory_file_name),
        )]))
    }
}
