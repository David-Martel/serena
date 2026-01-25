//! MCP Tool wrappers for configuration operations
//!
//! These tools provide a consistent interface for AI agents to manage
//! project configuration via the MCP protocol.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serena_core::{SerenaError, Tool, ToolResult};
use tracing::debug;

use crate::service::ConfigService;
use crate::ProjectConfig;

// ============================================================================
// ActivateProjectTool
// ============================================================================

/// Tool for activating a project
pub struct ActivateProjectTool {
    service: Arc<ConfigService>,
}

#[derive(Debug, Deserialize)]
struct ActivateProjectParams {
    name_or_path: String,
}

#[derive(Debug, Serialize)]
struct ActivateProjectOutput {
    project_name: String,
    project_root: String,
    languages: Vec<String>,
    message: String,
}

impl ActivateProjectTool {
    pub fn new(service: Arc<ConfigService>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl Tool for ActivateProjectTool {
    fn name(&self) -> &str {
        "activate_project"
    }

    fn description(&self) -> &str {
        "Activates a project by name or path. If the name matches an existing project, \
        that project is activated. If it's a path to a directory, a new project is \
        created with auto-detected languages."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name_or_path": {
                    "type": "string",
                    "description": "The project name or path to the project directory"
                }
            },
            "required": ["name_or_path"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: ActivateProjectParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        debug!("Activating project: {}", params.name_or_path);

        let project = self
            .service
            .activate_project(&params.name_or_path)
            .map_err(|e| SerenaError::Internal(e.to_string()))?;

        let languages: Vec<String> = project.languages.iter().map(|l| format!("{:?}", l)).collect();

        Ok(ToolResult::success(
            serde_json::to_value(ActivateProjectOutput {
                project_name: project.name.clone(),
                project_root: project.root.display().to_string(),
                languages,
                message: format!("Activated project: {}", project.name),
            })
            .map_err(|e| SerenaError::Internal(e.to_string()))?,
        ))
    }

    fn can_edit(&self) -> bool {
        true
    }

    fn requires_project(&self) -> bool {
        false
    }

    fn tags(&self) -> Vec<String> {
        vec!["config".to_string(), "project".to_string()]
    }
}

// ============================================================================
// GetCurrentConfigTool
// ============================================================================

/// Tool for getting the current configuration state
pub struct GetCurrentConfigTool {
    service: Arc<ConfigService>,
}

#[derive(Debug, Serialize)]
struct GetConfigOutput {
    active_project: Option<ProjectInfo>,
    active_context: String,
    active_modes: Vec<String>,
    project_count: usize,
    available_contexts: Vec<String>,
    available_modes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectInfo {
    name: String,
    root: String,
    languages: Vec<String>,
}

impl From<ProjectConfig> for ProjectInfo {
    fn from(p: ProjectConfig) -> Self {
        Self {
            name: p.name,
            root: p.root.display().to_string(),
            languages: p.languages.iter().map(|l| format!("{:?}", l)).collect(),
        }
    }
}

impl GetCurrentConfigTool {
    pub fn new(service: Arc<ConfigService>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl Tool for GetCurrentConfigTool {
    fn name(&self) -> &str {
        "get_current_config"
    }

    fn description(&self) -> &str {
        "Gets the current configuration state including active project, context, modes, \
        and available options."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _params: Value) -> Result<ToolResult, SerenaError> {
        debug!("Getting current configuration");

        let active_project = self.service.get_active_project().map(ProjectInfo::from);

        let active_context = self
            .service
            .get_active_context()
            .map_err(|e| SerenaError::Internal(e.to_string()))?;

        let active_modes = self
            .service
            .get_active_modes()
            .map_err(|e| SerenaError::Internal(e.to_string()))?;

        let projects = self
            .service
            .list_projects()
            .map_err(|e| SerenaError::Internal(e.to_string()))?;

        let available_contexts = self
            .service
            .list_contexts()
            .map_err(|e| SerenaError::Internal(e.to_string()))?;

        let available_modes = self
            .service
            .list_modes()
            .map_err(|e| SerenaError::Internal(e.to_string()))?;

        Ok(ToolResult::success(
            serde_json::to_value(GetConfigOutput {
                active_project,
                active_context,
                active_modes,
                project_count: projects.len(),
                available_contexts,
                available_modes,
            })
            .map_err(|e| SerenaError::Internal(e.to_string()))?,
        ))
    }

    fn can_edit(&self) -> bool {
        false
    }

    fn requires_project(&self) -> bool {
        false
    }

    fn tags(&self) -> Vec<String> {
        vec!["config".to_string(), "info".to_string()]
    }
}

// ============================================================================
// SwitchModesTool
// ============================================================================

/// Tool for switching active modes
pub struct SwitchModesTool {
    service: Arc<ConfigService>,
}

#[derive(Debug, Deserialize)]
struct SwitchModesParams {
    modes: Vec<String>,
}

impl SwitchModesTool {
    pub fn new(service: Arc<ConfigService>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl Tool for SwitchModesTool {
    fn name(&self) -> &str {
        "switch_modes"
    }

    fn description(&self) -> &str {
        "Switches the active modes. Modes affect tool behavior and availability. \
        Common modes include 'interactive', 'planning', 'editing', and 'one-shot'."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "modes": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of mode names to activate"
                }
            },
            "required": ["modes"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: SwitchModesParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        debug!("Switching modes to: {:?}", params.modes);

        self.service
            .switch_modes(params.modes.clone())
            .map_err(|e| SerenaError::Internal(e.to_string()))?;

        Ok(ToolResult::success(json!({
            "message": format!("Switched to modes: {:?}", params.modes),
            "active_modes": params.modes
        })))
    }

    fn can_edit(&self) -> bool {
        true
    }

    fn requires_project(&self) -> bool {
        false
    }

    fn tags(&self) -> Vec<String> {
        vec!["config".to_string(), "mode".to_string()]
    }
}

// ============================================================================
// ListProjectsTool
// ============================================================================

/// Tool for listing all available projects
pub struct ListProjectsTool {
    service: Arc<ConfigService>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListProjectsOutput {
    projects: Vec<ProjectInfo>,
    count: usize,
    active_project: Option<String>,
}

impl ListProjectsTool {
    pub fn new(service: Arc<ConfigService>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl Tool for ListProjectsTool {
    fn name(&self) -> &str {
        "list_projects"
    }

    fn description(&self) -> &str {
        "Lists all registered projects with their details including name, root path, \
        and detected languages."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _params: Value) -> Result<ToolResult, SerenaError> {
        debug!("Listing projects");

        let projects = self
            .service
            .list_projects()
            .map_err(|e| SerenaError::Internal(e.to_string()))?;

        let count = projects.len();
        let projects: Vec<ProjectInfo> = projects.into_iter().map(ProjectInfo::from).collect();

        let active_project = self.service.get_active_project_name();

        Ok(ToolResult::success(
            serde_json::to_value(ListProjectsOutput {
                projects,
                count,
                active_project,
            })
            .map_err(|e| SerenaError::Internal(e.to_string()))?,
        ))
    }

    fn can_edit(&self) -> bool {
        false
    }

    fn requires_project(&self) -> bool {
        false
    }

    fn tags(&self) -> Vec<String> {
        vec!["config".to_string(), "project".to_string(), "list".to_string()]
    }
}

// ============================================================================
// GetActiveToolsTool
// ============================================================================

/// Tool for getting the list of currently active tools
pub struct GetActiveToolsTool {
    service: Arc<ConfigService>,
}

#[derive(Debug, Serialize)]
struct GetActiveToolsOutput {
    tools: Vec<String>,
    count: usize,
    context: String,
}

impl GetActiveToolsTool {
    pub fn new(service: Arc<ConfigService>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl Tool for GetActiveToolsTool {
    fn name(&self) -> &str {
        "get_active_tools"
    }

    fn description(&self) -> &str {
        "Gets the list of currently active tools based on the current context and project. \
        This shows which tools are available in the current configuration state."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _params: Value) -> Result<ToolResult, SerenaError> {
        debug!("Getting active tools");

        let tools = self
            .service
            .get_active_tools()
            .map_err(|e| SerenaError::Internal(e.to_string()))?;

        let context = self
            .service
            .get_active_context()
            .map_err(|e| SerenaError::Internal(e.to_string()))?;

        let count = tools.len();

        Ok(ToolResult::success(
            serde_json::to_value(GetActiveToolsOutput {
                tools,
                count,
                context,
            })
            .map_err(|e| SerenaError::Internal(e.to_string()))?,
        ))
    }

    fn can_edit(&self) -> bool {
        false
    }

    fn requires_project(&self) -> bool {
        false
    }

    fn tags(&self) -> Vec<String> {
        vec!["config".to_string(), "tools".to_string()]
    }
}

// ============================================================================
// RemoveProjectTool
// ============================================================================

/// Tool for removing a project from configuration
pub struct RemoveProjectTool {
    service: Arc<ConfigService>,
}

#[derive(Debug, Deserialize)]
struct RemoveProjectParams {
    name: String,
}

impl RemoveProjectTool {
    pub fn new(service: Arc<ConfigService>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl Tool for RemoveProjectTool {
    fn name(&self) -> &str {
        "remove_project"
    }

    fn description(&self) -> &str {
        "Removes a project from the configuration. The project must not be active. \
        Use deactivate_project first if needed."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "The name of the project to remove"
                }
            },
            "required": ["name"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: RemoveProjectParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        debug!("Removing project: {}", params.name);

        let removed = self
            .service
            .remove_project(&params.name)
            .map_err(|e| SerenaError::Internal(e.to_string()))?;

        if removed {
            Ok(ToolResult::success(json!({
                "message": format!("Project '{}' removed successfully", params.name),
                "removed": true
            })))
        } else {
            Ok(ToolResult::warning(format!(
                "Project '{}' was not found",
                params.name
            )))
        }
    }

    fn can_edit(&self) -> bool {
        true
    }

    fn requires_project(&self) -> bool {
        false
    }

    fn tags(&self) -> Vec<String> {
        vec!["config".to_string(), "project".to_string()]
    }
}

// ============================================================================
// Factory functions
// ============================================================================

/// Create all config tools with a shared ConfigService
pub fn create_config_tools(service: Arc<ConfigService>) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(ActivateProjectTool::new(Arc::clone(&service))),
        Arc::new(GetCurrentConfigTool::new(Arc::clone(&service))),
        Arc::new(SwitchModesTool::new(Arc::clone(&service))),
        Arc::new(ListProjectsTool::new(Arc::clone(&service))),
        Arc::new(GetActiveToolsTool::new(Arc::clone(&service))),
        Arc::new(RemoveProjectTool::new(service)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_service() -> Arc<ConfigService> {
        Arc::new(ConfigService::new())
    }

    #[tokio::test]
    async fn test_get_current_config_tool() {
        let service = create_test_service();
        let tool = GetCurrentConfigTool::new(service);

        assert_eq!(tool.name(), "get_current_config");
        assert!(!tool.can_edit());

        let result = tool.execute(json!({})).await.unwrap();
        assert_eq!(result.status, serena_core::ToolStatus::Success);
    }

    #[tokio::test]
    async fn test_switch_modes_tool() {
        let service = create_test_service();
        let tool = SwitchModesTool::new(service);

        assert_eq!(tool.name(), "switch_modes");
        assert!(tool.can_edit());

        let result = tool
            .execute(json!({ "modes": ["interactive"] }))
            .await
            .unwrap();

        assert_eq!(result.status, serena_core::ToolStatus::Success);
    }

    #[tokio::test]
    async fn test_list_projects_tool() {
        let service = create_test_service();
        let tool = ListProjectsTool::new(service);

        assert_eq!(tool.name(), "list_projects");

        let result = tool.execute(json!({})).await.unwrap();
        assert_eq!(result.status, serena_core::ToolStatus::Success);

        let data = result.data.unwrap();
        let output: ListProjectsOutput = serde_json::from_value(data).unwrap();
        assert_eq!(output.count, 0);
    }

    #[tokio::test]
    async fn test_create_config_tools() {
        let service = create_test_service();
        let tools = create_config_tools(service);

        assert_eq!(tools.len(), 6);

        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"activate_project"));
        assert!(names.contains(&"get_current_config"));
        assert!(names.contains(&"switch_modes"));
        assert!(names.contains(&"list_projects"));
        assert!(names.contains(&"get_active_tools"));
        assert!(names.contains(&"remove_project"));
    }
}
