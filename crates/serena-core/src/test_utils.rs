//! Test Utilities for Serena
//!
//! Common test fixtures, mocks, and utilities for testing Serena components.
//!
//! # Example Usage
//!
//! ```rust,ignore
//! use serena_core::test_utils::*;
//!
//! #[tokio::test]
//! async fn test_tool_invocation() {
//!     let mock_server = MockMcpServer::new();
//!     mock_server.add_tool("find_symbol", "Find symbols", json!({"type": "object"}));
//!     let response = mock_server.handle_call("tools/list", None);
//!     assert!(response.get("tools").is_some());
//! }
//! ```

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

// These imports are used by TestProject when test-fixtures feature is enabled
#[cfg(feature = "test-fixtures")]
use std::path::{Path, PathBuf};

/// Mock MCP server for testing
#[derive(Debug, Default)]
pub struct MockMcpServer {
    tools: Arc<Mutex<Vec<MockTool>>>,
    resources: Arc<Mutex<Vec<MockResource>>>,
    call_history: Arc<Mutex<Vec<McpCall>>>,
}

/// Mock tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// Mock resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockResource {
    pub uri: String,
    pub name: String,
    pub mime_type: Option<String>,
}

/// Record of an MCP call
#[derive(Debug, Clone)]
pub struct McpCall {
    pub method: String,
    pub params: Option<Value>,
    pub timestamp: std::time::Instant,
}

impl MockMcpServer {
    /// Create a new mock MCP server
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with default Serena tools
    pub fn with_default_tools() -> Self {
        let server = Self::new();
        server.add_tool(
            "find_symbol",
            "Find symbols in the codebase by name pattern",
            json!({
                "type": "object",
                "properties": {
                    "name_path_pattern": {"type": "string"},
                    "relative_path": {"type": "string"},
                    "include_body": {"type": "boolean", "default": false}
                },
                "required": ["name_path_pattern"]
            }),
        );
        server.add_tool(
            "read_file",
            "Read a file from the filesystem",
            json!({
                "type": "object",
                "properties": {
                    "relative_path": {"type": "string"},
                    "start_line": {"type": "integer"},
                    "end_line": {"type": "integer"}
                },
                "required": ["relative_path"]
            }),
        );
        server.add_tool(
            "replace_content",
            "Replace content in a file using regex or literal match",
            json!({
                "type": "object",
                "properties": {
                    "relative_path": {"type": "string"},
                    "needle": {"type": "string"},
                    "repl": {"type": "string"},
                    "mode": {"type": "string", "enum": ["literal", "regex"]}
                },
                "required": ["relative_path", "needle", "repl", "mode"]
            }),
        );
        server
    }

    /// Add a tool to the mock server
    pub fn add_tool(&self, name: &str, description: &str, input_schema: Value) {
        let mut tools = self.tools.lock().unwrap();
        tools.push(MockTool {
            name: name.to_string(),
            description: description.to_string(),
            input_schema,
        });
    }

    /// Add a resource to the mock server
    pub fn add_resource(&self, uri: &str, name: &str, mime_type: Option<&str>) {
        let mut resources = self.resources.lock().unwrap();
        resources.push(MockResource {
            uri: uri.to_string(),
            name: name.to_string(),
            mime_type: mime_type.map(|s| s.to_string()),
        });
    }

    /// Handle an MCP call
    pub fn handle_call(&self, method: &str, params: Option<Value>) -> Value {
        // Record the call
        {
            let mut history = self.call_history.lock().unwrap();
            history.push(McpCall {
                method: method.to_string(),
                params: params.clone(),
                timestamp: std::time::Instant::now(),
            });
        }

        match method {
            "tools/list" => {
                let tools = self.tools.lock().unwrap();
                json!({
                    "tools": tools.iter().map(|t| {
                        json!({
                            "name": t.name,
                            "description": t.description,
                            "inputSchema": t.input_schema
                        })
                    }).collect::<Vec<_>>()
                })
            }
            "resources/list" => {
                let resources = self.resources.lock().unwrap();
                json!({
                    "resources": resources.iter().map(|r| {
                        json!({
                            "uri": r.uri,
                            "name": r.name,
                            "mimeType": r.mime_type
                        })
                    }).collect::<Vec<_>>()
                })
            }
            "tools/call" => {
                if let Some(ref p) = params {
                    let tool_name = p.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                    json!({
                        "content": [{
                            "type": "text",
                            "text": format!("Mock result from tool: {}", tool_name)
                        }]
                    })
                } else {
                    json!({"error": {"code": -32602, "message": "Missing params"}})
                }
            }
            "ping" => {
                json!({})
            }
            _ => {
                json!({
                    "error": {
                        "code": -32601,
                        "message": format!("Unknown method: {}", method)
                    }
                })
            }
        }
    }

    /// Get the number of calls to a specific method
    pub fn call_count(&self, method: &str) -> usize {
        let history = self.call_history.lock().unwrap();
        history.iter().filter(|c| c.method == method).count()
    }

    /// Get all recorded calls
    pub fn get_calls(&self) -> Vec<McpCall> {
        self.call_history.lock().unwrap().clone()
    }

    /// Clear call history
    pub fn clear_history(&self) {
        self.call_history.lock().unwrap().clear();
    }
}

/// Test project fixture - creates a temporary project structure for testing
#[cfg(feature = "test-fixtures")]
pub struct TestProject {
    pub temp_dir: tempfile::TempDir,
    pub root: PathBuf,
}

#[cfg(feature = "test-fixtures")]
impl TestProject {
    /// Create a new test project with basic structure
    pub fn new() -> std::io::Result<Self> {
        let temp_dir = tempfile::TempDir::new()?;
        let root = temp_dir.path().to_path_buf();

        // Create directory structure
        std::fs::create_dir_all(root.join("src"))?;
        std::fs::create_dir_all(root.join("test"))?;
        std::fs::create_dir_all(root.join(".serena/memories"))?;

        // Create basic project config
        std::fs::write(
            root.join(".serena/project.yml"),
            "name: test_project\nlanguage: rust\n",
        )?;

        Ok(Self { temp_dir, root })
    }

    /// Create a Python test project
    pub fn python() -> std::io::Result<Self> {
        let project = Self::new()?;

        // Create sample Python files
        std::fs::write(
            project.root.join("src/main.py"),
            r#""""Main module."""

class MyClass:
    """A sample class."""

    def __init__(self, name: str) -> None:
        self.name = name

    def greet(self) -> str:
        return f"Hello, {self.name}!"


def main() -> None:
    obj = MyClass("World")
    print(obj.greet())


if __name__ == "__main__":
    main()
"#,
        )?;

        std::fs::write(
            project.root.join("src/utils.py"),
            r#""""Utility functions."""

def add(a: int, b: int) -> int:
    """Add two numbers."""
    return a + b


CONSTANT: int = 42
"#,
        )?;

        Ok(project)
    }

    /// Create a Rust test project
    pub fn rust() -> std::io::Result<Self> {
        let project = Self::new()?;

        // Create Cargo.toml
        std::fs::write(
            project.root.join("Cargo.toml"),
            r#"[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
"#,
        )?;

        // Create sample Rust file
        std::fs::write(
            project.root.join("src/lib.rs"),
            r#"//! Test library

/// A sample struct
pub struct MyStruct {
    name: String,
}

impl MyStruct {
    /// Create a new instance
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }

    /// Get a greeting
    pub fn greet(&self) -> String {
        format!("Hello, {}!", self.name)
    }
}

/// Add two numbers
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        let s = MyStruct::new("World");
        assert_eq!(s.greet(), "Hello, World!");
    }

    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
    }
}
"#,
        )?;

        Ok(project)
    }

    /// Write a file to the project
    pub fn write_file(&self, relative_path: &str, content: &str) -> std::io::Result<PathBuf> {
        let path = self.root.join(relative_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, content)?;
        Ok(path)
    }

    /// Read a file from the project
    pub fn read_file(&self, relative_path: &str) -> std::io::Result<String> {
        std::fs::read_to_string(self.root.join(relative_path))
    }

    /// Get the project root path
    pub fn path(&self) -> &Path {
        &self.root
    }
}

/// Mock LSP symbol for testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockSymbol {
    pub name: String,
    pub kind: u32,
    pub range: MockRange,
    pub children: Vec<MockSymbol>,
}

/// Mock LSP range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockRange {
    pub start_line: u32,
    pub start_char: u32,
    pub end_line: u32,
    pub end_char: u32,
}

impl MockSymbol {
    /// Create a new mock symbol
    pub fn new(name: &str, kind: u32, start_line: u32, end_line: u32) -> Self {
        Self {
            name: name.to_string(),
            kind,
            range: MockRange {
                start_line,
                start_char: 0,
                end_line,
                end_char: 0,
            },
            children: Vec::new(),
        }
    }

    /// Add a child symbol
    pub fn with_child(mut self, child: MockSymbol) -> Self {
        self.children.push(child);
        self
    }

    /// Add multiple children
    pub fn with_children(mut self, children: Vec<MockSymbol>) -> Self {
        self.children.extend(children);
        self
    }
}

/// Assert helper macros for tests
#[macro_export]
macro_rules! assert_mcp_success {
    ($response:expr) => {
        assert!(
            !$response.get("error").is_some(),
            "Expected success but got error: {:?}",
            $response.get("error")
        );
    };
}

#[macro_export]
macro_rules! assert_mcp_error {
    ($response:expr, $code:expr) => {
        let error = $response.get("error").expect("Expected error response");
        let code = error.get("code").and_then(|v| v.as_i64());
        assert_eq!(code, Some($code), "Expected error code {}", $code);
    };
}

#[macro_export]
macro_rules! assert_contains_tool {
    ($tools:expr, $name:expr) => {
        assert!(
            $tools
                .iter()
                .any(|t| t.get("name").and_then(|v| v.as_str()) == Some($name)),
            "Tool '{}' not found in {:?}",
            $name,
            $tools
        );
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_mcp_server() {
        let server = MockMcpServer::with_default_tools();

        let response = server.handle_call("tools/list", None);
        let tools = response.get("tools").unwrap().as_array().unwrap();

        assert!(tools.len() >= 3);
        assert_eq!(server.call_count("tools/list"), 1);
    }

    #[test]
    fn test_mock_mcp_tool_call() {
        let server = MockMcpServer::with_default_tools();

        let response = server.handle_call(
            "tools/call",
            Some(json!({
                "name": "find_symbol",
                "arguments": {"name_path_pattern": "MyClass"}
            })),
        );

        let content = response.get("content").unwrap().as_array().unwrap();
        assert!(!content.is_empty());
    }

    #[test]
    fn test_mock_symbol() {
        let symbol = MockSymbol::new("MyClass", 5, 1, 10)
            .with_child(MockSymbol::new("my_method", 6, 3, 8));

        assert_eq!(symbol.name, "MyClass");
        assert_eq!(symbol.children.len(), 1);
        assert_eq!(symbol.children[0].name, "my_method");
    }
}
