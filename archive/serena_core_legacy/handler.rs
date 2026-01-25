//! Serena MCP Server Handler - Routes tool calls to implementations

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use rust_mcp_sdk::mcp_server::ServerHandler;
use rust_mcp_sdk::schema::mcp_2025_06_18::schema_utils::CallToolError;
use rust_mcp_sdk::schema::{
    CallToolRequest, CallToolResult, ListToolsRequest, ListToolsResult, RpcError,
};
use rust_mcp_sdk::McpServer;

use super::error::SerenaResult;
use super::tools::cli::CommandArguments;
use super::tools::{FileService, MemoryService, SerenaTools, SymbolService};

/// Main handler for the Serena MCP server
pub struct SerenaServerHandler {
    file_service: Arc<FileService>,
    symbol_service: Arc<SymbolService>,
    memory_service: Arc<MemoryService>,
    project_root: PathBuf,
}

impl SerenaServerHandler {
    pub fn new(args: &CommandArguments) -> SerenaResult<Self> {
        let project_root = args
            .project_path
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let file_service = Arc::new(FileService::new(&project_root)?);
        let symbol_service = Arc::new(SymbolService::new(&project_root)?);
        let memory_service = Arc::new(MemoryService::new(&project_root)?);

        Ok(Self {
            file_service,
            symbol_service,
            memory_service,
            project_root,
        })
    }

    pub fn startup_message(&self) -> String {
        format!(
            "Serena MCP Server started\nProject root: {}\nTools: file, symbol, memory",
            self.project_root.display()
        )
    }
}

#[async_trait]
impl ServerHandler for SerenaServerHandler {
    async fn handle_list_tools_request(
        &self,
        _req: ListToolsRequest,
        _rt: Arc<dyn McpServer>,
    ) -> Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult {
            tools: SerenaTools::all_tools(),
            meta: None,
            next_cursor: None,
        })
    }

    async fn handle_call_tool_request(
        &self,
        request: CallToolRequest,
        _rt: Arc<dyn McpServer>,
    ) -> Result<CallToolResult, CallToolError> {
        let tool_params: SerenaTools =
            SerenaTools::try_from(request.params).map_err(CallToolError::from_message)?;

        match tool_params {
            // File Tools
            SerenaTools::ReadFile(params) => params.run_tool(&self.file_service).await,
            SerenaTools::CreateTextFile(params) => params.run_tool(&self.file_service).await,
            SerenaTools::ListDir(params) => params.run_tool(&self.file_service).await,
            SerenaTools::FindFile(params) => params.run_tool(&self.file_service).await,
            SerenaTools::ReplaceContent(params) => params.run_tool(&self.file_service).await,
            SerenaTools::SearchForPattern(params) => params.run_tool(&self.file_service).await,

            // Symbol Tools
            SerenaTools::GetSymbolsOverview(params) => params.run_tool(&self.symbol_service).await,
            SerenaTools::FindSymbol(params) => params.run_tool(&self.symbol_service).await,
            SerenaTools::FindReferencingSymbols(params) => {
                params.run_tool(&self.symbol_service).await
            }
            SerenaTools::ReplaceSymbolBody(params) => params.run_tool(&self.symbol_service).await,
            SerenaTools::RenameSymbol(params) => params.run_tool(&self.symbol_service).await,

            // Memory Tools
            SerenaTools::WriteMemory(params) => params.run_tool(&self.memory_service).await,
            SerenaTools::ReadMemory(params) => params.run_tool(&self.memory_service).await,
            SerenaTools::ListMemories(params) => params.run_tool(&self.memory_service).await,
            SerenaTools::DeleteMemory(params) => params.run_tool(&self.memory_service).await,
            SerenaTools::EditMemory(params) => params.run_tool(&self.memory_service).await,

            // Command Tools
            SerenaTools::ExecuteShellCommand(params) => params.run_tool(&self.file_service).await,

            // Config Tools
            SerenaTools::GetCurrentConfig(params) => params.run_tool(&self.file_service).await,
            SerenaTools::InitialInstructions(params) => params.run_tool().await,
            SerenaTools::Think(params) => params.run_tool().await,
            SerenaTools::ThinkMore(params) => params.run_tool().await,
            SerenaTools::ThinkDifferent(params) => params.run_tool().await,
            SerenaTools::Onboarding(params) => params.run_tool().await,
            SerenaTools::CheckOnboardingPerformed(params) => {
                params.run_tool(&self.memory_service).await
            }
        }
    }
}
