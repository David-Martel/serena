//! Configuration tools for Serena MCP server

use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::mcp_2025_06_18::schema_utils::CallToolError;
use rust_mcp_sdk::schema::{CallToolResult, TextContent};
use serde_json::json;

use super::services::{FileService, MemoryService};

// ============================================================================
// get_current_config
// ============================================================================

#[mcp_tool(
    name = "get_current_config",
    description = "Get the current Serena configuration including active project, available tools, context, and mode. This tool provides agent self-awareness by returning the current operational state.",
    destructive_hint = false,
    idempotent_hint = true,
    read_only_hint = true
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct GetCurrentConfigTool {}

impl GetCurrentConfigTool {
    pub async fn run_tool(self, service: &FileService) -> Result<CallToolResult, CallToolError> {
        let project_root = service.project_root();

        // Get project name from the last component of the path
        let project_name = project_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // List of all available tools
        let available_tools = vec![
            // File Tools
            "read_file",
            "create_text_file",
            "list_dir",
            "find_file",
            "replace_content",
            "search_for_pattern",
            // Symbol Tools
            "get_symbols_overview",
            "find_symbol",
            "find_referencing_symbols",
            "replace_symbol_body",
            "rename_symbol",
            "insert_after_symbol",
            "insert_before_symbol",
            // Memory Tools
            "write_memory",
            "read_memory",
            "list_memories",
            "delete_memory",
            "edit_memory",
            // Command Tools
            "execute_shell_command",
            // Config Tools
            "get_current_config",
            "initial_instructions",
            "think",
            "think_more",
            "think_different",
            "onboarding",
            "check_onboarding_performed",
        ];

        let config = json!({
            "active_project": {
                "path": project_root.display().to_string(),
                "name": project_name
            },
            "available_tools": available_tools,
            "context": "default",
            "mode": "default",
            "version": env!("CARGO_PKG_VERSION"),
            "server": "serena-mcp-server"
        });

        Ok(CallToolResult::text_content(vec![TextContent::from(
            serde_json::to_string_pretty(&config)
                .map_err(|e| CallToolError::from_message(e.to_string()))?,
        )]))
    }
}

// ============================================================================
// initial_instructions
// ============================================================================

#[mcp_tool(
    name = "initial_instructions",
    description = "Get the Serena Instructions Manual. Call this first to understand how to use Serena tools effectively. IMPORTANT: Some MCP clients (including Claude Desktop) do not read the system prompt automatically, so you should call this tool at the beginning of a conversation to understand how to work with Serena.",
    destructive_hint = false,
    idempotent_hint = true,
    read_only_hint = true
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct InitialInstructionsTool {}

impl InitialInstructionsTool {
    pub async fn run_tool(self) -> Result<CallToolResult, CallToolError> {
        Ok(CallToolResult::text_content(vec![TextContent::from(
            SERENA_INSTRUCTIONS.to_string(),
        )]))
    }
}

/// The comprehensive Serena instructions manual
const SERENA_INSTRUCTIONS: &str = r#"# Serena Instructions Manual

You are a professional coding agent using Serena's semantic coding tools.

## Core Philosophy

Some tasks may require you to understand the architecture of large parts of the codebase, while for others,
it may be enough to read a small set of symbols or a single file. You avoid reading entire files unless
absolutely necessary, relying on intelligent step-by-step acquisition of information.

## Available Tools

### File Operations
- read_file, create_text_file, list_dir, find_file, replace_content, search_for_pattern

### Symbol Operations (Semantic Tools)
- get_symbols_overview, find_symbol, find_referencing_symbols
- replace_symbol_body, rename_symbol, insert_after_symbol, insert_before_symbol

### Memory Operations
- write_memory, read_memory, list_memories, delete_memory, edit_memory

### Shell Commands
- execute_shell_command

## Workflow: Understanding a New Codebase
1. Start with list_dir to explore structure
2. Use find_file to locate key files
3. Use get_symbols_overview to understand file structure without reading full content
4. Use find_symbol with include_body=false to explore symbol hierarchies
5. Only use include_body=true or read_file when you need implementation details
6. Use write_memory to save important insights

## Workflow: Making Code Changes
1. Use find_symbol to locate the symbol
2. Check find_referencing_symbols to understand impact
3. Use replace_symbol_body for symbol-level edits (preferred over text replacement)
4. Use rename_symbol if you need to rename and update all references
5. Verify changes with execute_shell_command to run tests

## Best Practices
- Don't read entire files unnecessarily
- Use targeted searches with relative_path parameter
- Prefer symbolic editing over text-based replacement
- Use LSP features like rename_symbol for refactoring
- Save important findings with write_memory

## Symbol Hierarchies by Language
- Python: ClassName/method_name, ClassName/__init__, function_name
- Rust: impl StructName/method_name, trait TraitName/method_name, fn function_name
- TypeScript/JavaScript: ClassName/methodName, functionName, interfaceName

## You Have Now Read the Serena Instructions Manual
You do not need to call this tool again in the same conversation.
"#;

// ============================================================================
// think
// ============================================================================

#[mcp_tool(
    name = "think",
    description = "Pause and think about the collected information and whether it's sufficient for the task.",
    destructive_hint = false,
    idempotent_hint = true,
    read_only_hint = true
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct ThinkTool {}

impl ThinkTool {
    pub async fn run_tool(self) -> Result<CallToolResult, CallToolError> {
        let prompt = r#"
# Think About Collected Information

Take a moment to reflect on what you've learned:

1. **Information Completeness**
   - Do I have enough information to proceed?
   - Are there gaps in my understanding?
   - What questions remain unanswered?

2. **Task Alignment**
   - Is my current approach aligned with the user's request?
   - Am I solving the right problem?
   - Have I considered alternative approaches?

3. **Next Steps**
   - What should I do next?
   - What's the most efficient path forward?
   - Are there any risks or concerns?

If information is insufficient, use search or symbol tools to gather more.
If ready, proceed with the implementation.
"#;
        Ok(CallToolResult::text_content(vec![TextContent::from(
            prompt.to_string(),
        )]))
    }
}

// ============================================================================
// think_more
// ============================================================================

#[mcp_tool(
    name = "think_more",
    description = "Continue thinking and reasoning about the current approach. Use when you need to go deeper.",
    destructive_hint = false,
    idempotent_hint = true,
    read_only_hint = true
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct ThinkMoreTool {}

impl ThinkMoreTool {
    pub async fn run_tool(self) -> Result<CallToolResult, CallToolError> {
        let prompt = r#"
# Think More Deeply

Expand your reasoning:

1. **Edge Cases**
   - What could go wrong?
   - Are there boundary conditions to handle?
   - How will this work in unusual scenarios?

2. **Dependencies and Side Effects**
   - What does this change affect?
   - Are there related files or functions to update?
   - Could this break existing functionality?

3. **Code Quality**
   - Is the approach maintainable?
   - Does it follow project patterns?
   - Are there better alternatives?

4. **Testing**
   - How will I verify this works?
   - What tests should be added?
   - How do I handle error cases?
"#;
        Ok(CallToolResult::text_content(vec![TextContent::from(
            prompt.to_string(),
        )]))
    }
}

// ============================================================================
// think_different
// ============================================================================

#[mcp_tool(
    name = "think_different",
    description = "Generate alternative approaches to the current problem. Use when stuck or want fresh perspective.",
    destructive_hint = false,
    idempotent_hint = true,
    read_only_hint = true
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct ThinkDifferentTool {}

impl ThinkDifferentTool {
    pub async fn run_tool(self) -> Result<CallToolResult, CallToolError> {
        let prompt = r#"
# Think Differently

Challenge your assumptions and consider alternatives:

1. **Opposite Approach**
   - What if I did the opposite of my current approach?
   - Could a simpler solution work?
   - Is there a more direct path?

2. **Different Patterns**
   - How would an expert approach this?
   - What design patterns could help?
   - Are there library solutions I'm missing?

3. **Fresh Perspective**
   - If starting from scratch, what would I do?
   - What would make this easier?
   - What's the real problem I'm solving?

4. **Constraints**
   - What if I had unlimited time? What would I do?
   - What if I had to finish in 5 minutes? What's essential?
   - What would I do differently if this was production-critical?

Consider at least 2 alternative approaches before proceeding.
"#;
        Ok(CallToolResult::text_content(vec![TextContent::from(
            prompt.to_string(),
        )]))
    }
}

// ============================================================================
// onboarding
// ============================================================================

#[mcp_tool(
    name = "onboarding",
    description = "Get onboarding instructions for exploring a new project. Call this after activating a project to understand its structure and key components.",
    destructive_hint = false,
    idempotent_hint = true,
    read_only_hint = true
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct OnboardingTool {}

impl OnboardingTool {
    pub async fn run_tool(self) -> Result<CallToolResult, CallToolError> {
        let instructions = r#"
# Project Onboarding

## Step 1: Explore Structure
- Use `list_dir` with path "" to see root directory
- Use `list_dir` with path "src" to explore source code
- Look for README.md, CLAUDE.md, or similar documentation

## Step 2: Identify Key Components
- Use `search_for_pattern` to find main entry points
- Use `get_symbols_overview` on key files to understand structure
- Use `find_symbol` to locate specific functionality

## Step 3: Save Your Findings
- Use `write_memory` to save important discoveries
- Document: architecture patterns, key files, coding conventions
- Create memories for: "project_overview", "key_components", "coding_patterns"

## Step 4: Ready to Work
- You now understand the project structure
- Proceed with the user's task
- Use symbolic editing tools for precise changes
"#;
        Ok(CallToolResult::text_content(vec![TextContent::from(
            instructions.to_string(),
        )]))
    }
}

// ============================================================================
// check_onboarding_performed
// ============================================================================

#[mcp_tool(
    name = "check_onboarding_performed",
    description = "Check if onboarding has been performed for the current project. Returns advice based on existing memories.",
    destructive_hint = false,
    idempotent_hint = true,
    read_only_hint = true
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct CheckOnboardingPerformedTool {}

impl CheckOnboardingPerformedTool {
    pub async fn run_tool(self, service: &MemoryService) -> Result<CallToolResult, CallToolError> {
        // Check if memories exist
        let memories = service
            .list()
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let response = if memories.is_empty() {
            json!({
                "onboarding_performed": false,
                "message": "No project memories found. You should call the 'onboarding' tool to explore the project structure.",
                "advice": "Call onboarding tool next"
            })
        } else {
            json!({
                "onboarding_performed": true,
                "message": format!("Found {} existing memories. Project has been onboarded.", memories.len()),
                "memories": memories,
                "advice": "Project is ready. Proceed with the user's task."
            })
        };

        Ok(CallToolResult::text_content(vec![TextContent::from(
            serde_json::to_string_pretty(&response)
                .map_err(|e| CallToolError::from_message(e.to_string()))?,
        )]))
    }
}
