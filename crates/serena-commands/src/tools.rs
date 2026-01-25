//! Command execution tools for Serena MCP server

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use serena_core::{SerenaError, Tool, ToolError, ToolResult};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;
use tracing::debug;

// ============================================================================
// ExecuteShellCommandTool
// ============================================================================

/// Tool for executing shell commands in the project context
pub struct ExecuteShellCommandTool {
    project_root: PathBuf,
}

#[derive(Debug, Deserialize)]
struct ExecuteShellCommandParams {
    command: String,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default = "default_capture_stderr")]
    capture_stderr: bool,
    #[serde(default = "default_max_chars")]
    max_answer_chars: i32,
    #[serde(default = "default_timeout_secs")]
    timeout_secs: u64,
}

fn default_capture_stderr() -> bool {
    true
}

fn default_max_chars() -> i32 {
    -1
}

fn default_timeout_secs() -> u64 {
    60
}

/// Output from command execution
struct CommandOutput {
    stdout: String,
    stderr: String,
    exit_code: i32,
}

impl ExecuteShellCommandTool {
    /// Create a new ExecuteShellCommandTool with the given project root
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }

    /// Validate command for basic security checks
    fn validate_command(command: &str) -> Result<(), SerenaError> {
        let cmd_lower = command.to_lowercase();

        // Dangerous patterns to reject
        let dangerous_patterns = [
            "rm -rf /",
            "del /f /s /q c:\\",
            "format c:",
            "mkfs",
            "dd if=/dev/zero",
            ":(){:|:&};:", // fork bomb
        ];

        for pattern in &dangerous_patterns {
            if cmd_lower.contains(pattern) {
                return Err(SerenaError::Tool(ToolError::ExecutionFailed(format!(
                    "Command contains dangerous pattern: {}",
                    pattern
                ))));
            }
        }

        // Check for empty command
        if command.trim().is_empty() {
            return Err(SerenaError::InvalidParameter(
                "Command cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    /// Execute the command and capture output
    async fn execute_command(
        command: &str,
        working_dir: &PathBuf,
        capture_stderr: bool,
    ) -> Result<CommandOutput, String> {
        // Determine shell based on platform
        let (shell, shell_arg) = if cfg!(target_os = "windows") {
            ("cmd.exe", "/C")
        } else {
            ("sh", "-c")
        };

        let mut cmd = Command::new(shell);
        cmd.arg(shell_arg)
            .arg(command)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(if capture_stderr {
                Stdio::piped()
            } else {
                Stdio::null()
            });

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn command: {}", e))?;

        // Capture stdout
        let stdout_handle = child.stdout.take();
        let stdout_task = async {
            if let Some(stdout) = stdout_handle {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                let mut output = String::new();
                while let Ok(Some(line)) = lines.next_line().await {
                    output.push_str(&line);
                    output.push('\n');
                }
                output
            } else {
                String::new()
            }
        };

        // Capture stderr
        let stderr_handle = child.stderr.take();
        let stderr_task = async {
            if let Some(stderr) = stderr_handle {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                let mut output = String::new();
                while let Ok(Some(line)) = lines.next_line().await {
                    output.push_str(&line);
                    output.push('\n');
                }
                output
            } else {
                String::new()
            }
        };

        // Wait for both streams and process to complete
        let (stdout, stderr) = tokio::join!(stdout_task, stderr_task);
        let status = child
            .wait()
            .await
            .map_err(|e| format!("Failed to wait for command: {}", e))?;

        Ok(CommandOutput {
            stdout,
            stderr,
            exit_code: status.code().unwrap_or(-1),
        })
    }

    /// Truncate output based on max_answer_chars
    fn truncate_output(output: &str, max_chars: i32) -> String {
        if max_chars < 0 {
            output.to_string()
        } else {
            let max_chars = max_chars as usize;
            if output.len() <= max_chars {
                output.to_string()
            } else {
                let mut truncated = output.chars().take(max_chars).collect::<String>();
                truncated.push_str("\n...[output truncated]");
                truncated
            }
        }
    }
}

#[async_trait]
impl Tool for ExecuteShellCommandTool {
    fn name(&self) -> &str {
        "execute_shell_command"
    }

    fn description(&self) -> &str {
        "Execute a shell command in the project context. Use for build, test, and other \
        development commands. Returns stdout, stderr, and exit code."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "cwd": {
                    "type": "string",
                    "description": "Working directory for command execution (defaults to project root)"
                },
                "capture_stderr": {
                    "type": "boolean",
                    "description": "Capture stderr in addition to stdout",
                    "default": true
                },
                "max_answer_chars": {
                    "type": "integer",
                    "description": "Maximum number of output characters to return (-1 for unlimited)",
                    "default": -1
                },
                "timeout_secs": {
                    "type": "integer",
                    "description": "Command timeout in seconds",
                    "default": 60
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: ExecuteShellCommandParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        debug!("Executing shell command: {}", params.command);

        // Security validation
        Self::validate_command(&params.command)?;

        // Determine working directory
        let working_dir = if let Some(ref cwd) = params.cwd {
            let cwd_path = PathBuf::from(cwd);
            if cwd_path.is_absolute() {
                cwd_path
            } else {
                self.project_root.join(cwd)
            }
        } else {
            self.project_root.clone()
        };

        // Validate working directory exists
        if !working_dir.exists() {
            return Err(SerenaError::NotFound(format!(
                "Working directory does not exist: {}",
                working_dir.display()
            )));
        }

        // Execute command with timeout
        let result = timeout(
            Duration::from_secs(params.timeout_secs),
            Self::execute_command(&params.command, &working_dir, params.capture_stderr),
        )
        .await
        .map_err(|_| {
            SerenaError::Tool(ToolError::ExecutionFailed(format!(
                "Command timed out after {} seconds",
                params.timeout_secs
            )))
        })?;

        match result {
            Ok(output) => {
                // Apply output size limit if specified
                let stdout = Self::truncate_output(&output.stdout, params.max_answer_chars);
                let stderr = Self::truncate_output(&output.stderr, params.max_answer_chars);

                Ok(ToolResult::success(json!({
                    "stdout": stdout,
                    "stderr": stderr,
                    "exit_code": output.exit_code,
                    "working_directory": working_dir.display().to_string(),
                    "command": params.command,
                })))
            }
            Err(e) => Err(SerenaError::Tool(ToolError::ExecutionFailed(format!(
                "Failed to execute command: {}",
                e
            )))),
        }
    }

    fn can_edit(&self) -> bool {
        true // Commands can modify files
    }

    fn tags(&self) -> Vec<String> {
        vec![
            "command".to_string(),
            "shell".to_string(),
            "execute".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_command_empty() {
        let result = ExecuteShellCommandTool::validate_command("");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_command_dangerous() {
        let result = ExecuteShellCommandTool::validate_command("rm -rf /");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_command_valid() {
        let result = ExecuteShellCommandTool::validate_command("echo hello");
        assert!(result.is_ok());
    }

    #[test]
    fn test_truncate_output_unlimited() {
        let output = "Hello, World!";
        let result = ExecuteShellCommandTool::truncate_output(output, -1);
        assert_eq!(result, output);
    }

    #[test]
    fn test_truncate_output_limited() {
        let output = "Hello, World!";
        let result = ExecuteShellCommandTool::truncate_output(output, 5);
        assert!(result.starts_with("Hello"));
        assert!(result.contains("truncated"));
    }
}
