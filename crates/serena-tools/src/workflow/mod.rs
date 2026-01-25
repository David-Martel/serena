//! Workflow tools for guiding agent behavior
//!
//! These tools provide prompt templates that help the agent stay on track,
//! reflect on progress, and maintain context across conversations.

use async_trait::async_trait;
use serde_json::{json, Value};
use serena_core::{PromptFactory, SerenaError, Tool, ToolResult};

// ============================================================================
// CheckOnboardingPerformedTool
// ============================================================================

/// Tool for checking if onboarding has been performed
pub struct CheckOnboardingPerformedTool {
    /// Function to check if memories exist
    check_memories_fn: Box<dyn Fn() -> bool + Send + Sync>,
}

impl CheckOnboardingPerformedTool {
    /// Create a new CheckOnboardingPerformedTool
    pub fn new<F>(check_memories_fn: F) -> Self
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        Self {
            check_memories_fn: Box::new(check_memories_fn),
        }
    }

    /// Create with default behavior (always returns false, no memories)
    pub fn default_no_memories() -> Self {
        Self::new(|| false)
    }
}

#[async_trait]
impl Tool for CheckOnboardingPerformedTool {
    fn name(&self) -> &str {
        "check_onboarding_performed"
    }

    fn description(&self) -> &str {
        "Checks whether project onboarding was already performed. \
        You should always call this tool before beginning to work on a project."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _params: Value) -> Result<ToolResult, SerenaError> {
        let has_memories = (self.check_memories_fn)();

        let message = if has_memories {
            "Onboarding was already performed. Project memories are available. \
            Do not read them all immediately, just remember they exist and read them when needed."
                .to_string()
        } else {
            "Onboarding not performed yet (no memories available). \
            You should perform onboarding by calling the `onboarding` tool before proceeding with the task."
                .to_string()
        };

        Ok(ToolResult::success(json!({
            "onboarded": has_memories,
            "message": message
        })))
    }

    fn requires_project(&self) -> bool {
        true
    }

    fn tags(&self) -> Vec<String> {
        vec!["workflow".to_string(), "onboarding".to_string()]
    }
}

// ============================================================================
// OnboardingTool
// ============================================================================

/// Tool for performing project onboarding
pub struct OnboardingTool;

impl OnboardingTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OnboardingTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for OnboardingTool {
    fn name(&self) -> &str {
        "onboarding"
    }

    fn description(&self) -> &str {
        "Performs onboarding for a new project. Provides instructions for identifying \
        project structure, coding conventions, and essential commands."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _params: Value) -> Result<ToolResult, SerenaError> {
        let system = std::env::consts::OS;
        let prompt = PromptFactory::onboarding(system);

        Ok(ToolResult::success(json!({
            "instructions": prompt
        })))
    }

    fn requires_project(&self) -> bool {
        true
    }

    fn tags(&self) -> Vec<String> {
        vec!["workflow".to_string(), "onboarding".to_string()]
    }
}

// ============================================================================
// ThinkAboutCollectedInformationTool
// ============================================================================

/// Tool for reflecting on collected information
pub struct ThinkAboutCollectedInformationTool;

impl ThinkAboutCollectedInformationTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ThinkAboutCollectedInformationTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ThinkAboutCollectedInformationTool {
    fn name(&self) -> &str {
        "think_about_collected_information"
    }

    fn description(&self) -> &str {
        "Thinking tool for pondering the completeness of collected information. \
        Call this after completing a non-trivial sequence of searching steps."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _params: Value) -> Result<ToolResult, SerenaError> {
        let prompt = PromptFactory::think_about_collected_information();
        Ok(ToolResult::success(json!({
            "instructions": prompt
        })))
    }

    fn tags(&self) -> Vec<String> {
        vec!["workflow".to_string(), "thinking".to_string()]
    }
}

// ============================================================================
// ThinkAboutTaskAdherenceTool
// ============================================================================

/// Tool for reflecting on task adherence
pub struct ThinkAboutTaskAdherenceTool;

impl ThinkAboutTaskAdherenceTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ThinkAboutTaskAdherenceTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ThinkAboutTaskAdherenceTool {
    fn name(&self) -> &str {
        "think_about_task_adherence"
    }

    fn description(&self) -> &str {
        "Thinking tool for determining whether you are still on track with the current task. \
        Call this before inserting, replacing, or deleting code."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _params: Value) -> Result<ToolResult, SerenaError> {
        let prompt = PromptFactory::think_about_task_adherence();
        Ok(ToolResult::success(json!({
            "instructions": prompt
        })))
    }

    fn tags(&self) -> Vec<String> {
        vec!["workflow".to_string(), "thinking".to_string()]
    }
}

// ============================================================================
// ThinkAboutWhetherYouAreDoneTool
// ============================================================================

/// Tool for reflecting on task completion
pub struct ThinkAboutWhetherYouAreDoneTool;

impl ThinkAboutWhetherYouAreDoneTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ThinkAboutWhetherYouAreDoneTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ThinkAboutWhetherYouAreDoneTool {
    fn name(&self) -> &str {
        "think_about_whether_you_are_done"
    }

    fn description(&self) -> &str {
        "Thinking tool for determining whether the task is truly completed. \
        Call this whenever you feel that you are done with what the user asked for."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _params: Value) -> Result<ToolResult, SerenaError> {
        let prompt = PromptFactory::think_about_whether_you_are_done();
        Ok(ToolResult::success(json!({
            "instructions": prompt
        })))
    }

    fn tags(&self) -> Vec<String> {
        vec!["workflow".to_string(), "thinking".to_string()]
    }
}

// ============================================================================
// SummarizeChangesTool
// ============================================================================

/// Tool for summarizing changes made during the conversation
pub struct SummarizeChangesTool;

impl SummarizeChangesTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SummarizeChangesTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SummarizeChangesTool {
    fn name(&self) -> &str {
        "summarize_changes"
    }

    fn description(&self) -> &str {
        "Provides instructions for summarizing the changes made to the codebase. \
        Call this after completing any non-trivial coding task."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _params: Value) -> Result<ToolResult, SerenaError> {
        let prompt = PromptFactory::summarize_changes();
        Ok(ToolResult::success(json!({
            "instructions": prompt
        })))
    }

    fn tags(&self) -> Vec<String> {
        vec!["workflow".to_string(), "summary".to_string()]
    }
}

// ============================================================================
// PrepareForNewConversationTool
// ============================================================================

/// Tool for preparing context for a new conversation
pub struct PrepareForNewConversationTool;

impl PrepareForNewConversationTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PrepareForNewConversationTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for PrepareForNewConversationTool {
    fn name(&self) -> &str {
        "prepare_for_new_conversation"
    }

    fn description(&self) -> &str {
        "Provides instructions for preparing for a new conversation. \
        Use this when context is running low and task needs to continue in a new session."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _params: Value) -> Result<ToolResult, SerenaError> {
        let prompt = PromptFactory::prepare_for_new_conversation();
        Ok(ToolResult::success(json!({
            "instructions": prompt
        })))
    }

    fn tags(&self) -> Vec<String> {
        vec!["workflow".to_string(), "context".to_string()]
    }
}

// ============================================================================
// InitialInstructionsTool
// ============================================================================

/// Tool for providing the Serena instructions manual
pub struct InitialInstructionsTool;

impl InitialInstructionsTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for InitialInstructionsTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for InitialInstructionsTool {
    fn name(&self) -> &str {
        "initial_instructions"
    }

    fn description(&self) -> &str {
        "Provides the 'Serena Instructions Manual' with essential information on how to use the Serena toolbox. \
        Call this if you have not yet read the manual."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _params: Value) -> Result<ToolResult, SerenaError> {
        let prompt = PromptFactory::initial_instructions();
        Ok(ToolResult::success(json!({
            "instructions": prompt
        })))
    }

    fn requires_project(&self) -> bool {
        false // Can be called without an active project
    }

    fn tags(&self) -> Vec<String> {
        vec!["workflow".to_string(), "documentation".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_onboarding_tool() {
        let tool = OnboardingTool::new();
        let result = tool.execute(json!({})).await.unwrap();
        assert_eq!(result.status, serena_core::ToolStatus::Success);
        let data = result.data.unwrap();
        assert!(data["instructions"].as_str().unwrap().contains("project"));
    }

    #[tokio::test]
    async fn test_thinking_tools() {
        let tools: Vec<Box<dyn Tool>> = vec![
            Box::new(ThinkAboutCollectedInformationTool::new()),
            Box::new(ThinkAboutTaskAdherenceTool::new()),
            Box::new(ThinkAboutWhetherYouAreDoneTool::new()),
        ];

        for tool in tools {
            let result = tool.execute(json!({})).await.unwrap();
            assert_eq!(result.status, serena_core::ToolStatus::Success);
            let data = result.data.unwrap();
            assert!(!data["instructions"].as_str().unwrap().is_empty());
        }
    }

    #[tokio::test]
    async fn test_summarize_changes_tool() {
        let tool = SummarizeChangesTool::new();
        let result = tool.execute(json!({})).await.unwrap();
        assert_eq!(result.status, serena_core::ToolStatus::Success);
        let data = result.data.unwrap();
        assert!(data["instructions"].as_str().unwrap().contains("changes"));
    }

    #[tokio::test]
    async fn test_check_onboarding_with_memories() {
        let tool = CheckOnboardingPerformedTool::new(|| true);
        let result = tool.execute(json!({})).await.unwrap();
        let data = result.data.unwrap();
        assert!(data["onboarded"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_check_onboarding_without_memories() {
        let tool = CheckOnboardingPerformedTool::new(|| false);
        let result = tool.execute(json!({})).await.unwrap();
        let data = result.data.unwrap();
        assert!(!data["onboarded"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_initial_instructions_tool() {
        let tool = InitialInstructionsTool::new();
        assert!(!tool.requires_project());
        let result = tool.execute(json!({})).await.unwrap();
        let data = result.data.unwrap();
        assert!(data["instructions"].as_str().unwrap().contains("Serena"));
    }
}
