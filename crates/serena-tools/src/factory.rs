//! Tool factory for creating and registering tool instances
//!
//! This module provides a centralized way to create all tool instances
//! with their required dependencies.

use crate::{
    CreateTextFileTool, DeleteLinesTool, FindFileTool, InsertAtLineTool, ListDirectoryTool,
    ReadFileTool, ReplaceContentTool, ReplaceLinesTool, SearchFilesTool,
};
use crate::workflow::{
    CheckOnboardingPerformedTool, InitialInstructionsTool, OnboardingTool,
    PrepareForNewConversationTool, SummarizeChangesTool, ThinkAboutCollectedInformationTool,
    ThinkAboutTaskAdherenceTool, ThinkAboutWhetherYouAreDoneTool,
};
use serena_commands::ExecuteShellCommandTool;
use serena_core::{Tool, ToolRegistry, ToolRegistryBuilder};
use std::path::PathBuf;
use std::sync::Arc;

/// Factory for creating tool instances with their dependencies
pub struct ToolFactory {
    project_root: PathBuf,
}

impl ToolFactory {
    /// Create a new tool factory
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }

    /// Get the project root path
    pub fn project_root(&self) -> &PathBuf {
        &self.project_root
    }

    /// Create all file operation tools
    pub fn file_tools(&self) -> Vec<Arc<dyn Tool>> {
        vec![
            Arc::new(ReadFileTool::new(&self.project_root)),
            Arc::new(CreateTextFileTool::new(&self.project_root)),
            Arc::new(ListDirectoryTool::new(&self.project_root)),
            Arc::new(FindFileTool::new(&self.project_root)),
            Arc::new(SearchFilesTool::new(&self.project_root)),
            Arc::new(ReplaceContentTool::new(&self.project_root)),
        ]
    }

    /// Create all line editor tools
    pub fn editor_tools(&self) -> Vec<Arc<dyn Tool>> {
        vec![
            Arc::new(DeleteLinesTool::new(&self.project_root)),
            Arc::new(InsertAtLineTool::new(&self.project_root)),
            Arc::new(ReplaceLinesTool::new(&self.project_root)),
        ]
    }

    /// Create all workflow tools
    pub fn workflow_tools(&self) -> Vec<Arc<dyn Tool>> {
        vec![
            Arc::new(CheckOnboardingPerformedTool::default_no_memories()),
            Arc::new(OnboardingTool),
            Arc::new(ThinkAboutCollectedInformationTool),
            Arc::new(ThinkAboutTaskAdherenceTool),
            Arc::new(ThinkAboutWhetherYouAreDoneTool),
            Arc::new(SummarizeChangesTool),
            Arc::new(PrepareForNewConversationTool),
            Arc::new(InitialInstructionsTool),
        ]
    }

    /// Create command execution tools
    pub fn command_tools(&self) -> Vec<Arc<dyn Tool>> {
        vec![
            Arc::new(ExecuteShellCommandTool::new(&self.project_root)),
        ]
    }

    /// Create all non-LSP tools (file, editor, workflow, command)
    /// These tools don't require an LSP client to function.
    pub fn core_tools(&self) -> Vec<Arc<dyn Tool>> {
        let mut tools = Vec::with_capacity(18);
        tools.extend(self.file_tools());
        tools.extend(self.editor_tools());
        tools.extend(self.workflow_tools());
        tools.extend(self.command_tools());
        tools
    }

    /// Build a tool registry with all core tools
    pub fn build_core_registry(&self) -> ToolRegistry {
        ToolRegistryBuilder::new()
            .add_tools(self.core_tools())
            .build()
    }
}

/// Create all non-LSP tools for a given project root
///
/// This is a convenience function for quickly getting all core tools.
pub fn create_core_tools(project_root: impl Into<PathBuf>) -> Vec<Arc<dyn Tool>> {
    ToolFactory::new(project_root).core_tools()
}

/// Build a registry with all core tools for a given project root
///
/// This is a convenience function for quickly building a populated registry.
pub fn build_core_registry(project_root: impl Into<PathBuf>) -> ToolRegistry {
    ToolFactory::new(project_root).build_core_registry()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_factory_creation() {
        let factory = ToolFactory::new(temp_dir());
        assert!(!factory.project_root().as_os_str().is_empty());
    }

    #[test]
    fn test_file_tools_count() {
        let factory = ToolFactory::new(temp_dir());
        let tools = factory.file_tools();
        assert_eq!(tools.len(), 6);
    }

    #[test]
    fn test_editor_tools_count() {
        let factory = ToolFactory::new(temp_dir());
        let tools = factory.editor_tools();
        assert_eq!(tools.len(), 3);
    }

    #[test]
    fn test_workflow_tools_count() {
        let factory = ToolFactory::new(temp_dir());
        let tools = factory.workflow_tools();
        assert_eq!(tools.len(), 8);
    }

    #[test]
    fn test_command_tools_count() {
        let factory = ToolFactory::new(temp_dir());
        let tools = factory.command_tools();
        assert_eq!(tools.len(), 1);
    }

    #[test]
    fn test_core_tools_count() {
        let factory = ToolFactory::new(temp_dir());
        let tools = factory.core_tools();
        assert_eq!(tools.len(), 18); // 6 + 3 + 8 + 1
    }

    #[test]
    fn test_build_core_registry() {
        let factory = ToolFactory::new(temp_dir());
        let registry = factory.build_core_registry();
        assert_eq!(registry.len(), 18);

        // Verify some specific tools are registered
        assert!(registry.has_tool("read_file"));
        assert!(registry.has_tool("delete_lines"));
        assert!(registry.has_tool("onboarding"));
        assert!(registry.has_tool("execute_shell_command"));
    }

    #[test]
    fn test_convenience_functions() {
        let tools = create_core_tools(temp_dir());
        assert_eq!(tools.len(), 18);

        let registry = build_core_registry(temp_dir());
        assert_eq!(registry.len(), 18);
    }
}
