//! Integration tests for ExecuteShellCommandTool

use serde_json::Value;
use serena_core::mcp::tools::{ExecuteShellCommandTool, FileService};
use tempfile::TempDir;

/// Helper function to extract JSON from CallToolResult
fn get_json_from_result(result: &rust_mcp_sdk::schema::CallToolResult) -> Value {
    // The content is a Vec<ContentBlock>, and we expect the first item to be TextContent
    let content_item = &result.content[0];

    // Extract text based on the actual structure
    let text = match content_item {
        rust_mcp_sdk::schema::ContentBlock::TextContent(t) => &t.text,
        rust_mcp_sdk::schema::ContentBlock::ImageContent(_) => panic!("Expected text content"),
        rust_mcp_sdk::schema::ContentBlock::AudioContent(_) => panic!("Expected text content"),
        rust_mcp_sdk::schema::ContentBlock::ResourceLink(_) => panic!("Expected text content"),
        rust_mcp_sdk::schema::ContentBlock::EmbeddedResource(_) => panic!("Expected text content"),
    };

    serde_json::from_str(text).expect("Result should be valid JSON")
}

#[tokio::test]
async fn test_execute_simple_command() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let service = FileService::new(temp_dir.path()).expect("Failed to create file service");

    let tool = ExecuteShellCommandTool {
        command: if cfg!(target_os = "windows") {
            "echo Hello World".to_string()
        } else {
            "echo 'Hello World'".to_string()
        },
        cwd: None,
        capture_stderr: true,
        max_answer_chars: -1,
        timeout_secs: 10,
    };

    let result = tool.run_tool(&service).await;
    assert!(result.is_ok(), "Command execution should succeed");

    let output = result.unwrap();
    let json = get_json_from_result(&output);

    assert_eq!(json["exit_code"], 0, "Exit code should be 0");
    assert!(
        json["stdout"].as_str().unwrap().contains("Hello World"),
        "Output should contain 'Hello World'"
    );
}

#[tokio::test]
async fn test_execute_command_in_custom_directory() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let service = FileService::new(temp_dir.path()).expect("Failed to create file service");

    let tool = ExecuteShellCommandTool {
        command: if cfg!(target_os = "windows") {
            "cd".to_string() // Windows
        } else {
            "pwd".to_string() // Unix
        },
        cwd: None,
        capture_stderr: true,
        max_answer_chars: -1,
        timeout_secs: 10,
    };

    let result = tool.run_tool(&service).await;
    assert!(result.is_ok(), "Command execution should succeed");

    let output = result.unwrap();
    let json = get_json_from_result(&output);

    assert_eq!(json["exit_code"], 0, "Exit code should be 0");

    let stdout = json["stdout"].as_str().unwrap();
    let temp_path = temp_dir.path().to_string_lossy();

    // Normalize paths for comparison (handle backslashes on Windows)
    let stdout_normalized = stdout.replace('\\', "/").trim().to_lowercase();
    let expected_normalized = temp_path.replace('\\', "/").trim().to_lowercase();

    assert!(
        stdout_normalized.contains(&expected_normalized),
        "Working directory should be temp dir. Expected: {}, Got: {}",
        expected_normalized,
        stdout_normalized
    );
}

#[tokio::test]
async fn test_execute_command_with_nonzero_exit_code() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let service = FileService::new(temp_dir.path()).expect("Failed to create file service");

    let tool = ExecuteShellCommandTool {
        command: if cfg!(target_os = "windows") {
            "exit 42".to_string()
        } else {
            "exit 42".to_string()
        },
        cwd: None,
        capture_stderr: true,
        max_answer_chars: -1,
        timeout_secs: 10,
    };

    let result = tool.run_tool(&service).await;
    assert!(
        result.is_ok(),
        "Command should execute even with non-zero exit"
    );

    let output = result.unwrap();
    let json = get_json_from_result(&output);

    assert_eq!(json["exit_code"], 42, "Exit code should be 42");
}

#[tokio::test]
async fn test_execute_command_output_truncation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let service = FileService::new(temp_dir.path()).expect("Failed to create file service");

    let tool = ExecuteShellCommandTool {
        command: if cfg!(target_os = "windows") {
            "echo This is a test message that should be truncated".to_string()
        } else {
            "echo 'This is a test message that should be truncated'".to_string()
        },
        cwd: None,
        capture_stderr: true,
        max_answer_chars: 10, // Truncate to 10 characters
        timeout_secs: 10,
    };

    let result = tool.run_tool(&service).await;
    assert!(result.is_ok(), "Command execution should succeed");

    let output = result.unwrap();
    let json = get_json_from_result(&output);

    let stdout = json["stdout"].as_str().unwrap();
    assert!(
        stdout.contains("[output truncated]"),
        "Output should be truncated"
    );
    assert!(
        stdout.len() < 50,
        "Truncated output should be shorter than original"
    );
}

#[tokio::test]
async fn test_dangerous_command_rejection() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let service = FileService::new(temp_dir.path()).expect("Failed to create file service");

    let tool = ExecuteShellCommandTool {
        command: "rm -rf /".to_string(),
        cwd: None,
        capture_stderr: true,
        max_answer_chars: -1,
        timeout_secs: 10,
    };

    let result = tool.run_tool(&service).await;
    assert!(result.is_err(), "Dangerous command should be rejected");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("dangerous pattern"),
        "Error should mention dangerous pattern"
    );
}

#[tokio::test]
async fn test_empty_command_rejection() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let service = FileService::new(temp_dir.path()).expect("Failed to create file service");

    let tool = ExecuteShellCommandTool {
        command: "".to_string(),
        cwd: None,
        capture_stderr: true,
        max_answer_chars: -1,
        timeout_secs: 10,
    };

    let result = tool.run_tool(&service).await;
    assert!(result.is_err(), "Empty command should be rejected");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("cannot be empty"),
        "Error should mention empty command"
    );
}

#[tokio::test]
async fn test_timeout_handling() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let service = FileService::new(temp_dir.path()).expect("Failed to create file service");

    let tool = ExecuteShellCommandTool {
        command: if cfg!(target_os = "windows") {
            "timeout /t 10".to_string() // Wait 10 seconds on Windows
        } else {
            "sleep 10".to_string() // Wait 10 seconds on Unix
        },
        cwd: None,
        capture_stderr: true,
        max_answer_chars: -1,
        timeout_secs: 1, // Timeout after 1 second
    };

    let result = tool.run_tool(&service).await;
    assert!(result.is_err(), "Command should timeout");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("timed out"),
        "Error should mention timeout. Got: {}",
        error_msg
    );
}
