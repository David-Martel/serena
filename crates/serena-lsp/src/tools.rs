//! MCP Tool wrappers for LSP operations
//!
//! These tools provide a consistent interface for AI agents to manage
//! language servers via the MCP protocol.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serena_config::Language;
use serena_core::{SerenaError, Tool, ToolResult};
use tracing::debug;

use crate::manager::LanguageServerManager;

// ============================================================================
// RestartLanguageServerTool
// ============================================================================

/// Tool for restarting a language server
pub struct RestartLanguageServerTool {
    manager: Arc<LanguageServerManager>,
}

#[derive(Debug, Deserialize)]
struct RestartServerParams {
    language: String,
}

impl RestartLanguageServerTool {
    pub fn new(manager: Arc<LanguageServerManager>) -> Self {
        Self { manager }
    }

    fn parse_language(language: &str) -> Result<Language, SerenaError> {
        let lang = match language.to_lowercase().as_str() {
            "rust" => Language::Rust,
            "python" => Language::Python,
            "javascript" | "js" => Language::JavaScript,
            "typescript" | "ts" => Language::TypeScript,
            "go" | "golang" => Language::Go,
            "java" => Language::Java,
            "ruby" => Language::Ruby,
            "php" => Language::PHP,
            "csharp" | "c#" => Language::CSharp,
            "swift" => Language::Swift,
            "kotlin" => Language::Kotlin,
            "scala" => Language::Scala,
            "elixir" => Language::Elixir,
            "perl" => Language::Perl,
            "bash" | "shell" => Language::Bash,
            "terraform" | "tf" => Language::Terraform,
            "vue" => Language::Vue,
            "clojure" => Language::Clojure,
            "powershell" => Language::PowerShell,
            "cpp" | "c++" => Language::Cpp,
            "c" => Language::C,
            "haskell" => Language::Haskell,
            "lua" => Language::Lua,
            "dart" => Language::Dart,
            other => {
                return Err(SerenaError::InvalidParameter(format!(
                    "Unknown language: {}. Supported: rust, python, javascript, typescript, \
                    go, java, ruby, php, csharp, swift, kotlin, scala, elixir, perl, bash, \
                    terraform, vue, clojure, powershell, cpp, c, haskell, lua, dart",
                    other
                )));
            }
        };
        Ok(lang)
    }
}

#[async_trait]
impl Tool for RestartLanguageServerTool {
    fn name(&self) -> &str {
        "restart_language_server"
    }

    fn description(&self) -> &str {
        "Restarts the language server for the specified programming language. \
        This stops the server if running, clears its cache, and starts it again. \
        Useful when the language server becomes unresponsive or out of sync."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "language": {
                    "type": "string",
                    "description": "The programming language (e.g., 'rust', 'python', 'typescript')"
                }
            },
            "required": ["language"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: RestartServerParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        let language = Self::parse_language(&params.language)?;

        debug!("Restarting language server for: {:?}", language);

        self.manager
            .restart_server(language)
            .await
            .map_err(|e| SerenaError::Internal(e.to_string()))?;

        Ok(ToolResult::success(json!({
            "message": format!("Language server for {:?} restarted successfully", language),
            "language": format!("{:?}", language)
        })))
    }

    fn can_edit(&self) -> bool {
        false
    }

    fn requires_project(&self) -> bool {
        true
    }

    fn tags(&self) -> Vec<String> {
        vec!["lsp".to_string(), "language-server".to_string()]
    }
}

// ============================================================================
// ListLanguageServersTool
// ============================================================================

/// Tool for listing running language servers
pub struct ListLanguageServersTool {
    manager: Arc<LanguageServerManager>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListServersOutput {
    servers: Vec<String>,
    count: usize,
}

impl ListLanguageServersTool {
    pub fn new(manager: Arc<LanguageServerManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for ListLanguageServersTool {
    fn name(&self) -> &str {
        "list_language_servers"
    }

    fn description(&self) -> &str {
        "Lists all currently running language servers. Returns the names of languages \
        that have active language servers."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _params: Value) -> Result<ToolResult, SerenaError> {
        debug!("Listing running language servers");

        let languages = self.manager.list_running_servers();
        let servers: Vec<String> = languages.iter().map(|l| format!("{:?}", l)).collect();
        let count = servers.len();

        Ok(ToolResult::success(
            serde_json::to_value(ListServersOutput { servers, count })
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
        vec!["lsp".to_string(), "language-server".to_string(), "list".to_string()]
    }
}

// ============================================================================
// StopLanguageServerTool
// ============================================================================

/// Tool for stopping a language server
pub struct StopLanguageServerTool {
    manager: Arc<LanguageServerManager>,
}

#[derive(Debug, Deserialize)]
struct StopServerParams {
    language: String,
}

impl StopLanguageServerTool {
    pub fn new(manager: Arc<LanguageServerManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for StopLanguageServerTool {
    fn name(&self) -> &str {
        "stop_language_server"
    }

    fn description(&self) -> &str {
        "Stops the language server for the specified programming language. \
        This is useful for freeing up resources when you're done working with \
        a particular language."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "language": {
                    "type": "string",
                    "description": "The programming language (e.g., 'rust', 'python', 'typescript')"
                }
            },
            "required": ["language"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: StopServerParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        let language = RestartLanguageServerTool::parse_language(&params.language)?;

        debug!("Stopping language server for: {:?}", language);

        let was_running = self.manager.is_server_running(language);

        self.manager
            .stop_server(language)
            .await
            .map_err(|e| SerenaError::Internal(e.to_string()))?;

        if was_running {
            Ok(ToolResult::success(json!({
                "message": format!("Language server for {:?} stopped", language),
                "language": format!("{:?}", language)
            })))
        } else {
            Ok(ToolResult::success(json!({
                "message": format!("No language server was running for {:?}", language),
                "language": format!("{:?}", language)
            })))
        }
    }

    fn can_edit(&self) -> bool {
        false
    }

    fn requires_project(&self) -> bool {
        true
    }

    fn tags(&self) -> Vec<String> {
        vec!["lsp".to_string(), "language-server".to_string()]
    }
}

// ============================================================================
// ClearLspCacheTool
// ============================================================================

/// Tool for clearing the LSP response cache
pub struct ClearLspCacheTool {
    manager: Arc<LanguageServerManager>,
}

impl ClearLspCacheTool {
    pub fn new(manager: Arc<LanguageServerManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for ClearLspCacheTool {
    fn name(&self) -> &str {
        "clear_lsp_cache"
    }

    fn description(&self) -> &str {
        "Clears the LSP response cache. This forces fresh requests to language servers \
        for subsequent operations. Useful when files have been modified outside the editor."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _params: Value) -> Result<ToolResult, SerenaError> {
        debug!("Clearing LSP cache");

        self.manager.clear_cache();

        Ok(ToolResult::success(json!({
            "message": "LSP cache cleared successfully"
        })))
    }

    fn can_edit(&self) -> bool {
        false
    }

    fn requires_project(&self) -> bool {
        true
    }

    fn tags(&self) -> Vec<String> {
        vec!["lsp".to_string(), "cache".to_string()]
    }
}

// ============================================================================
// Factory functions
// ============================================================================

/// Create all LSP tools with a shared LanguageServerManager
pub fn create_lsp_tools(manager: Arc<LanguageServerManager>) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(RestartLanguageServerTool::new(Arc::clone(&manager))),
        Arc::new(ListLanguageServersTool::new(Arc::clone(&manager))),
        Arc::new(StopLanguageServerTool::new(Arc::clone(&manager))),
        Arc::new(ClearLspCacheTool::new(manager)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn create_test_manager() -> Arc<LanguageServerManager> {
        Arc::new(LanguageServerManager::new(env::temp_dir()))
    }

    #[tokio::test]
    async fn test_list_language_servers_tool() {
        let manager = create_test_manager();
        let tool = ListLanguageServersTool::new(manager);

        assert_eq!(tool.name(), "list_language_servers");
        assert!(!tool.can_edit());

        let result = tool.execute(json!({})).await.unwrap();
        assert_eq!(result.status, serena_core::ToolStatus::Success);

        let data = result.data.unwrap();
        let output: ListServersOutput = serde_json::from_value(data).unwrap();
        assert_eq!(output.count, 0);
    }

    #[tokio::test]
    async fn test_clear_lsp_cache_tool() {
        let manager = create_test_manager();
        let tool = ClearLspCacheTool::new(manager);

        assert_eq!(tool.name(), "clear_lsp_cache");

        let result = tool.execute(json!({})).await.unwrap();
        assert_eq!(result.status, serena_core::ToolStatus::Success);
    }

    #[test]
    fn test_parse_language() {
        assert!(RestartLanguageServerTool::parse_language("rust").is_ok());
        assert!(RestartLanguageServerTool::parse_language("Rust").is_ok());
        assert!(RestartLanguageServerTool::parse_language("RUST").is_ok());
        assert!(RestartLanguageServerTool::parse_language("python").is_ok());
        assert!(RestartLanguageServerTool::parse_language("typescript").is_ok());
        assert!(RestartLanguageServerTool::parse_language("ts").is_ok());
        assert!(RestartLanguageServerTool::parse_language("javascript").is_ok());
        assert!(RestartLanguageServerTool::parse_language("js").is_ok());
        assert!(RestartLanguageServerTool::parse_language("unknown").is_err());
    }

    #[test]
    fn test_create_lsp_tools() {
        let manager = create_test_manager();
        let tools = create_lsp_tools(manager);

        assert_eq!(tools.len(), 4);

        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"restart_language_server"));
        assert!(names.contains(&"list_language_servers"));
        assert!(names.contains(&"stop_language_server"));
        assert!(names.contains(&"clear_lsp_cache"));
    }
}
