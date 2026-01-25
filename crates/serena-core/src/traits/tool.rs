use async_trait::async_trait;
use serde_json::Value;

use crate::{SerenaError, ToolResult};

/// Trait for tool implementations
#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns the name of the tool
    fn name(&self) -> &str;

    /// Returns a description of what the tool does
    fn description(&self) -> &str;

    /// Returns the JSON schema for the tool's parameters
    fn parameters_schema(&self) -> Value;

    /// Executes the tool with the given parameters
    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError>;

    /// Returns whether this tool can edit files/code
    fn can_edit(&self) -> bool {
        false
    }

    /// Returns whether this tool requires an active project
    fn requires_project(&self) -> bool {
        true
    }

    /// Returns whether this tool is available in the current context
    fn is_available(&self) -> bool {
        true
    }

    /// Returns tags/categories for the tool (for organization/filtering)
    fn tags(&self) -> Vec<String> {
        Vec::new()
    }
}
