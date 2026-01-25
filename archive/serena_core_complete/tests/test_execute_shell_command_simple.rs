//! Simple integration tests for ExecuteShellCommandTool

use serena_core::mcp::tools::{ExecuteShellCommandTool, FileService};
use tempfile::TempDir;

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
            // Windows: ping localhost with 10 requests (takes ~10 seconds)
            "ping -n 11 127.0.0.1".to_string()
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
