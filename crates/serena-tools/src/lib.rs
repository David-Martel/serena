pub mod editor;
pub mod factory;
pub mod file;
pub mod registry;
pub mod workflow;

pub use factory::{build_core_registry, create_core_tools, ToolFactory};
pub use registry::{ToolRegistry, ToolRegistryBuilder};
pub use serena_core::{SerenaError, Tool, ToolResult, ToolStatus};

// Re-export commonly used file tools
pub use file::{
    CreateTextFileTool, FindFileTool, ListDirectoryTool, ReadFileTool, ReplaceContentTool,
    SearchFilesTool,
};

// Re-export editor tools
pub use editor::{DeleteLinesTool, InsertAtLineTool, ReplaceLinesTool};

// Re-export workflow tools
pub use workflow::{
    CheckOnboardingPerformedTool, InitialInstructionsTool, OnboardingTool,
    PrepareForNewConversationTool, SummarizeChangesTool, ThinkAboutCollectedInformationTool,
    ThinkAboutTaskAdherenceTool, ThinkAboutWhetherYouAreDoneTool,
};

// Re-export async_trait for users implementing tools
pub use async_trait::async_trait;
