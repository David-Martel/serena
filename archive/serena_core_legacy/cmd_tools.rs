//! Command execution tools for Serena MCP server

use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::mcp_2025_06_18::schema_utils::CallToolError;
use rust_mcp_sdk::schema::{CallToolResult, TextContent};
use serde_json::json;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

use super::services::FileService;

// ============================================================================
// execute_shell_command
// ============================================================================

#[mcp_tool(
    name = "execute_shell_command",
    description = "Execute a shell command in the project context. Use for build, test, and other development commands. Returns stdout, stderr, and exit code.",
    destructive_hint = true,
    idempotent_hint = false,
    read_only_hint = false
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct ExecuteShellCommandTool {
    /// The shell command to execute
    pub command: String,

    /// Working directory for command execution (defaults to project root)
    #[serde(default)]
    pub cwd: Option<String>,

    /// Capture stderr in addition to stdout (default: true)
    #[serde(default = "default_capture_stderr")]
    pub capture_stderr: bool,

    /// Maximum number of output characters to return (-1 for unlimited, default: -1)
    #[serde(default = "default_max_chars")]
    pub max_answer_chars: i32,

    /// Command timeout in seconds (default: 60)
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
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

impl ExecuteShellCommandTool {
    pub async fn run_tool(self, service: &FileService) -> Result<CallToolResult, CallToolError> {
        // Security validation
        self.validate_command()?;

        // Determine working directory
        let working_dir = if let Some(ref cwd) = self.cwd {
            let cwd_path = PathBuf::from(cwd);
            if cwd_path.is_absolute() {
                cwd_path
            } else {
                service.project_root().join(cwd)
            }
        } else {
            service.project_root().to_path_buf()
        };

        // Validate working directory exists
        if !working_dir.exists() {
            return Err(CallToolError::from_message(format!(
                "Working directory does not exist: {}",
                working_dir.display()
            )));
        }

        // Execute command with timeout
        let result = timeout(
            Duration::from_secs(self.timeout_secs),
            self.execute_command(&working_dir),
        )
        .await
        .map_err(|_| {
            CallToolError::from_message(format!(
                "Command timed out after {} seconds",
                self.timeout_secs
            ))
        })?;

        match result {
            Ok(output) => {
                // Apply output size limit if specified
                let stdout = self.truncate_output(&output.stdout);
                let stderr = self.truncate_output(&output.stderr);

                // Format result as JSON
                let result_json = json!({
                    "stdout": stdout,
                    "stderr": stderr,
                    "exit_code": output.exit_code,
                    "working_directory": working_dir.display().to_string(),
                    "command": self.command,
                });

                Ok(CallToolResult::text_content(vec![TextContent::from(
                    serde_json::to_string_pretty(&result_json)
                        .unwrap_or_else(|_| "Error serializing result".to_string()),
                )]))
            }
            Err(e) => Err(CallToolError::from_message(format!(
                "Failed to execute command: {}",
                e
            ))),
        }
    }

    /// Validate command for basic security checks
    fn validate_command(&self) -> Result<(), CallToolError> {
        let cmd_lower = self.command.to_lowercase();

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
                return Err(CallToolError::from_message(format!(
                    "Command contains dangerous pattern: {}",
                    pattern
                )));
            }
        }

        // Check for empty command
        if self.command.trim().is_empty() {
            return Err(CallToolError::from_message(
                "Command cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    /// Execute the command and capture output
    async fn execute_command(&self, working_dir: &PathBuf) -> Result<CommandOutput, String> {
        // Determine shell based on platform
        let (shell, shell_arg) = if cfg!(target_os = "windows") {
            ("cmd.exe", "/C")
        } else {
            ("sh", "-c")
        };

        let mut cmd = Command::new(shell);
        cmd.arg(shell_arg)
            .arg(&self.command)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(if self.capture_stderr {
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
    fn truncate_output(&self, output: &str) -> String {
        if self.max_answer_chars < 0 {
            output.to_string()
        } else {
            let max_chars = self.max_answer_chars as usize;
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

/// Output from command execution
struct CommandOutput {
    stdout: String,
    stderr: String,
    exit_code: i32,
}
