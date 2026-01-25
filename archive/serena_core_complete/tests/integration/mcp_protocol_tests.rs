//! Integration tests for MCP protocol compliance
//!
//! These tests verify that the Serena MCP server properly implements the MCP protocol.

use async_trait::async_trait;
use rust_mcp_sdk::auth::AuthInfo;
use rust_mcp_sdk::error::McpSdkError;
use rust_mcp_sdk::mcp_server::ServerHandler;
use rust_mcp_sdk::schema::{
    CallToolRequest, CallToolRequestParams, ContentBlock, Implementation,
    InitializeRequestParams, InitializeResult, ListToolsRequest, RequestId,
    LATEST_PROTOCOL_VERSION,
};
use rust_mcp_sdk::schema::schema_utils::{ClientMessage, MessageFromServer, NotificationFromServer, ServerMessage};
use serena_core::mcp::handler::SerenaServerHandler;
use serena_core::mcp::server::server_details;
use serena_core::mcp::tools::cli::CommandArguments;
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::{RwLock, RwLockReadGuard};

/// Helper to create a test handler
fn setup_handler(temp_dir: &TempDir) -> SerenaServerHandler {
    let args = CommandArguments {
        project_path: Some(temp_dir.path().to_path_buf()),
        language_server: None,
        lsp_args: Vec::new(),
        debug: false,
        memory_db: None,
        allowed_dirs: Vec::new(),
    };

    SerenaServerHandler::new(&args).unwrap()
}

/// Mock MCP server for testing
struct MockMcpServer {
    server_info: InitializeResult,
    auth_info: RwLock<Option<AuthInfo>>,
}

impl MockMcpServer {
    fn new() -> Self {
        Self {
            server_info: InitializeResult {
                protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
                capabilities: Default::default(),
                server_info: Implementation {
                    name: "mock-server".to_string(),
                    version: "1.0.0".to_string(),
                    title: None,
                },
                instructions: None,
                meta: None,
            },
            auth_info: RwLock::new(None),
        }
    }
}

#[async_trait]
impl rust_mcp_sdk::McpServer for MockMcpServer {
    async fn start(self: Arc<Self>) -> Result<(), McpSdkError> {
        Ok(())
    }

    async fn set_client_details(&self, _params: InitializeRequestParams) -> Result<(), McpSdkError> {
        Ok(())
    }

    fn server_info(&self) -> &InitializeResult {
        &self.server_info
    }

    fn client_info(&self) -> Option<InitializeRequestParams> {
        None
    }

    async fn auth_info(&self) -> RwLockReadGuard<'_, Option<AuthInfo>> {
        self.auth_info.read().await
    }

    async fn auth_info_cloned(&self) -> Option<AuthInfo> {
        self.auth_info.read().await.clone()
    }

    async fn update_auth_info(&self, auth: Option<AuthInfo>) {
        *self.auth_info.write().await = auth;
    }

    async fn wait_for_initialization(&self) {
        // Do nothing - already initialized
    }

    async fn send(
        &self,
        _msg: MessageFromServer,
        _id: Option<RequestId>,
        _timeout: Option<Duration>,
    ) -> Result<Option<ClientMessage>, McpSdkError> {
        Ok(None)
    }

    async fn send_batch(
        &self,
        _msgs: Vec<ServerMessage>,
        _timeout: Option<Duration>,
    ) -> Result<Option<Vec<ClientMessage>>, McpSdkError> {
        Ok(None)
    }

    async fn send_notification(&self, _notification: NotificationFromServer) -> Result<(), McpSdkError> {
        Ok(())
    }

    async fn stderr_message(&self, _msg: String) -> Result<(), McpSdkError> {
        Ok(())
    }
}

#[tokio::test]
async fn test_server_metadata() {
    let details = server_details();

    // Verify server info
    assert_eq!(details.server_info.name, "serena-mcp-server");
    assert!(!details.server_info.version.is_empty());
    assert_eq!(
        details.server_info.title,
        Some("Serena MCP Server".to_string())
    );

    // Verify protocol version
    assert_eq!(details.protocol_version, LATEST_PROTOCOL_VERSION);

    // Verify capabilities
    assert!(details.capabilities.tools.is_some());

    // Verify instructions are provided
    assert!(details.instructions.is_some());
    let instructions = details.instructions.unwrap();
    assert!(instructions.contains("semantic coding tools"));
}

#[tokio::test]
async fn test_list_tools() {
    let temp_dir = TempDir::new().unwrap();
    let handler = setup_handler(&temp_dir);
    let mock_server = Arc::new(MockMcpServer::new()) as Arc<dyn rust_mcp_sdk::McpServer>;

    let request = ListToolsRequest::new(None);

    let result = handler
        .handle_list_tools_request(request, mock_server)
        .await;

    assert!(result.is_ok());
    let tools_result = result.unwrap();

    // Verify we have all expected tools
    let tool_names: Vec<String> = tools_result.tools.iter().map(|t| t.name.clone()).collect();

    // File tools
    assert!(tool_names.contains(&"read_file".to_string()));
    assert!(tool_names.contains(&"create_text_file".to_string()));
    assert!(tool_names.contains(&"list_dir".to_string()));
    assert!(tool_names.contains(&"find_file".to_string()));
    assert!(tool_names.contains(&"replace_content".to_string()));
    assert!(tool_names.contains(&"search_for_pattern".to_string()));

    // Symbol tools
    assert!(tool_names.contains(&"get_symbols_overview".to_string()));
    assert!(tool_names.contains(&"find_symbol".to_string()));
    assert!(tool_names.contains(&"find_referencing_symbols".to_string()));
    assert!(tool_names.contains(&"replace_symbol_body".to_string()));
    assert!(tool_names.contains(&"rename_symbol".to_string()));

    // Memory tools
    assert!(tool_names.contains(&"write_memory".to_string()));
    assert!(tool_names.contains(&"read_memory".to_string()));
    assert!(tool_names.contains(&"list_memories".to_string()));
    assert!(tool_names.contains(&"delete_memory".to_string()));
    assert!(tool_names.contains(&"edit_memory".to_string()));

    // Command tools
    assert!(tool_names.contains(&"execute_shell_command".to_string()));

    // Config tools
    assert!(tool_names.contains(&"get_current_config".to_string()));
    assert!(tool_names.contains(&"initial_instructions".to_string()));

    // Verify tool count
    assert!(tools_result.tools.len() >= 18);
}

#[tokio::test]
async fn test_call_tool_read_file() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "Hello, MCP!").unwrap();

    let handler = setup_handler(&temp_dir);
    let mock_server = Arc::new(MockMcpServer::new()) as Arc<dyn rust_mcp_sdk::McpServer>;

    let mut arguments = serde_json::Map::new();
    arguments.insert(
        "relative_path".to_string(),
        serde_json::Value::String("test.txt".to_string()),
    );

    let request = CallToolRequest::new(CallToolRequestParams {
        name: "read_file".to_string(),
        arguments: Some(arguments),
    });

    let result = handler.handle_call_tool_request(request, mock_server).await;

    assert!(result.is_ok());
    let tool_result = result.unwrap();

    // Verify content
    assert_eq!(tool_result.content.len(), 1);
    if let ContentBlock::TextContent(text) = &tool_result.content[0] {
        assert_eq!(text.text, "Hello, MCP!");
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_call_tool_create_file() {
    let temp_dir = TempDir::new().unwrap();
    let handler = setup_handler(&temp_dir);
    let mock_server = Arc::new(MockMcpServer::new()) as Arc<dyn rust_mcp_sdk::McpServer>;

    let mut arguments = serde_json::Map::new();
    arguments.insert(
        "relative_path".to_string(),
        serde_json::Value::String("new_file.txt".to_string()),
    );
    arguments.insert(
        "content".to_string(),
        serde_json::Value::String("Created via MCP!".to_string()),
    );

    let request = CallToolRequest::new(CallToolRequestParams {
        name: "create_text_file".to_string(),
        arguments: Some(arguments),
    });

    let result = handler.handle_call_tool_request(request, mock_server).await;

    assert!(result.is_ok());

    // Verify file was created
    let content = fs::read_to_string(temp_dir.path().join("new_file.txt")).unwrap();
    assert_eq!(content, "Created via MCP!");
}

#[tokio::test]
async fn test_call_tool_list_dir() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("file1.txt"), "content1").unwrap();
    fs::write(temp_dir.path().join("file2.txt"), "content2").unwrap();
    fs::create_dir(temp_dir.path().join("subdir")).unwrap();

    let handler = setup_handler(&temp_dir);
    let mock_server = Arc::new(MockMcpServer::new()) as Arc<dyn rust_mcp_sdk::McpServer>;

    let mut arguments = serde_json::Map::new();
    arguments.insert(
        "relative_path".to_string(),
        serde_json::Value::String(".".to_string()),
    );
    arguments.insert("recursive".to_string(), serde_json::Value::Bool(false));

    let request = CallToolRequest::new(CallToolRequestParams {
        name: "list_dir".to_string(),
        arguments: Some(arguments),
    });

    let result = handler.handle_call_tool_request(request, mock_server).await;

    assert!(result.is_ok());
    let tool_result = result.unwrap();

    if let ContentBlock::TextContent(text) = &tool_result.content[0] {
        let data: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        let files = data["files"].as_array().unwrap();
        let dirs = data["dirs"].as_array().unwrap();

        assert_eq!(files.len(), 2);
        // At least 1 dir (subdir), but .serena may also exist
        assert!(dirs.len() >= 1);
        assert!(dirs.iter().any(|d| d.as_str().unwrap().contains("subdir")));
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_call_tool_write_memory() {
    let temp_dir = TempDir::new().unwrap();
    let handler = setup_handler(&temp_dir);
    let mock_server = Arc::new(MockMcpServer::new()) as Arc<dyn rust_mcp_sdk::McpServer>;

    let mut arguments = serde_json::Map::new();
    arguments.insert(
        "memory_file_name".to_string(),
        serde_json::Value::String("test_memory".to_string()),
    );
    arguments.insert(
        "content".to_string(),
        serde_json::Value::String("# Test Memory\n\nContent here.".to_string()),
    );

    let request = CallToolRequest::new(CallToolRequestParams {
        name: "write_memory".to_string(),
        arguments: Some(arguments),
    });

    let result = handler.handle_call_tool_request(request, mock_server).await;

    assert!(result.is_ok());

    // Verify memory file was created
    let memory_file = temp_dir
        .path()
        .join(".serena")
        .join("memories")
        .join("test_memory.md");
    assert!(memory_file.exists());
    let content = fs::read_to_string(memory_file).unwrap();
    assert_eq!(content, "# Test Memory\n\nContent here.");
}

#[tokio::test]
async fn test_call_tool_list_memories() {
    let temp_dir = TempDir::new().unwrap();
    let handler = setup_handler(&temp_dir);
    let mock_server = Arc::new(MockMcpServer::new()) as Arc<dyn rust_mcp_sdk::McpServer>;

    // Create some memories
    let memories_dir = temp_dir.path().join(".serena").join("memories");
    fs::create_dir_all(&memories_dir).unwrap();
    fs::write(memories_dir.join("memory1.md"), "Content 1").unwrap();
    fs::write(memories_dir.join("memory2.md"), "Content 2").unwrap();

    let request = CallToolRequest::new(CallToolRequestParams {
        name: "list_memories".to_string(),
        arguments: Some(serde_json::Map::new()),
    });

    let result = handler.handle_call_tool_request(request, mock_server).await;

    assert!(result.is_ok());
    let tool_result = result.unwrap();

    if let ContentBlock::TextContent(text) = &tool_result.content[0] {
        let data: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        let memories = data["memories"].as_array().unwrap();
        let count = data["count"].as_i64().unwrap();

        assert_eq!(count, 2);
        assert_eq!(memories.len(), 2);

        let memory_names: Vec<&str> = memories.iter().map(|m| m.as_str().unwrap()).collect();
        assert!(memory_names.contains(&"memory1"));
        assert!(memory_names.contains(&"memory2"));
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_call_tool_invalid_tool_name() {
    let temp_dir = TempDir::new().unwrap();
    let handler = setup_handler(&temp_dir);
    let mock_server = Arc::new(MockMcpServer::new()) as Arc<dyn rust_mcp_sdk::McpServer>;

    let request = CallToolRequest::new(CallToolRequestParams {
        name: "nonexistent_tool".to_string(),
        arguments: Some(serde_json::Map::new()),
    });

    let result = handler.handle_call_tool_request(request, mock_server).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_call_tool_missing_required_argument() {
    let temp_dir = TempDir::new().unwrap();
    let handler = setup_handler(&temp_dir);
    let mock_server = Arc::new(MockMcpServer::new()) as Arc<dyn rust_mcp_sdk::McpServer>;

    // read_file requires relative_path, but we don't provide it
    let request = CallToolRequest::new(CallToolRequestParams {
        name: "read_file".to_string(),
        arguments: Some(serde_json::Map::new()),
    });

    let result = handler.handle_call_tool_request(request, mock_server).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_call_tool_invalid_argument_type() {
    let temp_dir = TempDir::new().unwrap();
    let handler = setup_handler(&temp_dir);
    let mock_server = Arc::new(MockMcpServer::new()) as Arc<dyn rust_mcp_sdk::McpServer>;

    let mut arguments = serde_json::Map::new();
    // relative_path should be string, but we provide a number
    arguments.insert(
        "relative_path".to_string(),
        serde_json::Value::Number(42.into()),
    );

    let request = CallToolRequest::new(CallToolRequestParams {
        name: "read_file".to_string(),
        arguments: Some(arguments),
    });

    let result = handler.handle_call_tool_request(request, mock_server).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_tool_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let handler = setup_handler(&temp_dir);
    let mock_server = Arc::new(MockMcpServer::new()) as Arc<dyn rust_mcp_sdk::McpServer>;

    // Try to read a file that doesn't exist
    let mut arguments = serde_json::Map::new();
    arguments.insert(
        "relative_path".to_string(),
        serde_json::Value::String("nonexistent.txt".to_string()),
    );

    let request = CallToolRequest::new(CallToolRequestParams {
        name: "read_file".to_string(),
        arguments: Some(arguments),
    });

    let result = handler.handle_call_tool_request(request, mock_server).await;

    // Should return error
    assert!(result.is_err());
}

#[tokio::test]
async fn test_multiple_sequential_tool_calls() {
    let temp_dir = TempDir::new().unwrap();
    let handler = setup_handler(&temp_dir);
    let mock_server = Arc::new(MockMcpServer::new()) as Arc<dyn rust_mcp_sdk::McpServer>;

    // 1. Create a file
    let mut create_args = serde_json::Map::new();
    create_args.insert(
        "relative_path".to_string(),
        serde_json::Value::String("test.txt".to_string()),
    );
    create_args.insert(
        "content".to_string(),
        serde_json::Value::String("Initial content".to_string()),
    );

    let create_request = CallToolRequest::new(CallToolRequestParams {
        name: "create_text_file".to_string(),
        arguments: Some(create_args),
    });

    assert!(handler
        .handle_call_tool_request(create_request, mock_server.clone())
        .await
        .is_ok());

    // 2. Read the file
    let mut read_args = serde_json::Map::new();
    read_args.insert(
        "relative_path".to_string(),
        serde_json::Value::String("test.txt".to_string()),
    );

    let read_request = CallToolRequest::new(CallToolRequestParams {
        name: "read_file".to_string(),
        arguments: Some(read_args),
    });

    let read_result = handler
        .handle_call_tool_request(read_request, mock_server.clone())
        .await
        .unwrap();

    if let ContentBlock::TextContent(text) = &read_result.content[0] {
        assert_eq!(text.text, "Initial content");
    } else {
        panic!("Expected text content");
    }

    // 3. Replace content
    let mut replace_args = serde_json::Map::new();
    replace_args.insert(
        "relative_path".to_string(),
        serde_json::Value::String("test.txt".to_string()),
    );
    replace_args.insert(
        "needle".to_string(),
        serde_json::Value::String("Initial".to_string()),
    );
    replace_args.insert(
        "repl".to_string(),
        serde_json::Value::String("Modified".to_string()),
    );
    replace_args.insert(
        "mode".to_string(),
        serde_json::Value::String("literal".to_string()),
    );
    replace_args.insert(
        "allow_multiple_occurrences".to_string(),
        serde_json::Value::Bool(false),
    );

    let replace_request = CallToolRequest::new(CallToolRequestParams {
        name: "replace_content".to_string(),
        arguments: Some(replace_args),
    });

    assert!(handler
        .handle_call_tool_request(replace_request, mock_server.clone())
        .await
        .is_ok());

    // 4. Read again to verify
    let mut read_args2 = serde_json::Map::new();
    read_args2.insert(
        "relative_path".to_string(),
        serde_json::Value::String("test.txt".to_string()),
    );

    let read_request2 = CallToolRequest::new(CallToolRequestParams {
        name: "read_file".to_string(),
        arguments: Some(read_args2),
    });

    let read_result2 = handler
        .handle_call_tool_request(read_request2, mock_server)
        .await
        .unwrap();

    if let ContentBlock::TextContent(text) = &read_result2.content[0] {
        assert_eq!(text.text, "Modified content");
    } else {
        panic!("Expected text content");
    }
}
