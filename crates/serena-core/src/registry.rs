use crate::traits::tool::Tool;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Registry for managing tool instances
///
/// Supports both static initialization via `ToolRegistryBuilder` and
/// dynamic extension via `extend()` for LSP-dependent tools.
#[derive(Clone)]
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Arc<dyn Tool>>>>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a registry from a list of tools
    pub fn from_tools(tools: Vec<Arc<dyn Tool>>) -> Self {
        let mut map = HashMap::new();
        for tool in tools {
            map.insert(tool.name().to_string(), tool);
        }
        Self {
            tools: Arc::new(RwLock::new(map)),
        }
    }

    /// Get a tool by name
    pub fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools
            .read()
            .ok()
            .and_then(|guard| guard.get(name).cloned())
    }

    /// List all registered tools
    pub fn list_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools
            .read()
            .map(|guard| guard.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all tool names
    pub fn tool_names(&self) -> Vec<String> {
        self.tools
            .read()
            .map(|guard| guard.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Check if a tool exists
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools
            .read()
            .map(|guard| guard.contains_key(name))
            .unwrap_or(false)
    }

    /// Get the number of registered tools
    pub fn len(&self) -> usize {
        self.tools.read().map(|guard| guard.len()).unwrap_or(0)
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.tools
            .read()
            .map(|guard| guard.is_empty())
            .unwrap_or(true)
    }

    /// Extend registry with additional tools dynamically
    ///
    /// This is used to add LSP-dependent symbol tools when a project is activated.
    /// Returns the number of tools successfully added.
    pub fn extend(&self, tools: Vec<Arc<dyn Tool>>) -> usize {
        if let Ok(mut guard) = self.tools.write() {
            let initial_len = guard.len();
            for tool in tools {
                guard.insert(tool.name().to_string(), tool);
            }
            guard.len() - initial_len
        } else {
            0
        }
    }

    /// Remove tools by name prefix
    ///
    /// Used to clean up LSP-dependent tools when a project is deactivated.
    /// Returns the number of tools removed.
    pub fn remove_by_prefix(&self, prefix: &str) -> usize {
        if let Ok(mut guard) = self.tools.write() {
            let initial_len = guard.len();
            guard.retain(|name, _| !name.starts_with(prefix));
            initial_len - guard.len()
        } else {
            0
        }
    }

    /// Remove a specific tool by name
    /// Returns the removed tool if it existed.
    pub fn remove(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.write().ok().and_then(|mut guard| guard.remove(name))
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating a tool registry
pub struct ToolRegistryBuilder {
    tools: Vec<Arc<dyn Tool>>,
}

impl ToolRegistryBuilder {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn add_tool(mut self, tool: Arc<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn add_tools(mut self, tools: Vec<Arc<dyn Tool>>) -> Self {
        self.tools.extend(tools);
        self
    }

    pub fn build(self) -> ToolRegistry {
        ToolRegistry::from_tools(self.tools)
    }
}

impl Default for ToolRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SerenaError, ToolResult};
    use async_trait::async_trait;
    use serde_json::json;

    /// Mock tool for testing
    #[derive(Clone)]
    struct MockTool {
        name: String,
    }

    impl MockTool {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
            }
        }
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "A mock tool for testing"
        }

        fn parameters_schema(&self) -> serde_json::Value {
            json!({"type": "object"})
        }

        async fn execute(&self, _params: serde_json::Value) -> Result<ToolResult, SerenaError> {
            Ok(ToolResult::success(json!({"tool": self.name})))
        }
    }

    #[test]
    fn test_registry_new_is_empty() {
        let registry = ToolRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_from_tools() {
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(MockTool::new("tool1")),
            Arc::new(MockTool::new("tool2")),
        ];
        let registry = ToolRegistry::from_tools(tools);

        assert_eq!(registry.len(), 2);
        assert!(registry.has_tool("tool1"));
        assert!(registry.has_tool("tool2"));
    }

    #[test]
    fn test_registry_get_tool() {
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(MockTool::new("test_tool"))];
        let registry = ToolRegistry::from_tools(tools);

        let tool = registry.get_tool("test_tool");
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().name(), "test_tool");

        let missing = registry.get_tool("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_registry_list_tools() {
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(MockTool::new("alpha")),
            Arc::new(MockTool::new("beta")),
        ];
        let registry = ToolRegistry::from_tools(tools);

        let listed = registry.list_tools();
        assert_eq!(listed.len(), 2);

        let names: Vec<&str> = listed.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"alpha"));
        assert!(names.contains(&"beta"));
    }

    #[test]
    fn test_registry_tool_names() {
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(MockTool::new("alpha")),
            Arc::new(MockTool::new("beta")),
        ];
        let registry = ToolRegistry::from_tools(tools);

        let names = registry.tool_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"alpha".to_string()));
        assert!(names.contains(&"beta".to_string()));
    }

    #[test]
    fn test_registry_extend() {
        let registry = ToolRegistry::new();
        assert!(registry.is_empty());

        // Extend with first batch
        let tools1: Vec<Arc<dyn Tool>> = vec![
            Arc::new(MockTool::new("tool1")),
            Arc::new(MockTool::new("tool2")),
        ];
        let added = registry.extend(tools1);
        assert_eq!(added, 2);
        assert_eq!(registry.len(), 2);

        // Extend with second batch
        let tools2: Vec<Arc<dyn Tool>> = vec![
            Arc::new(MockTool::new("tool3")),
            Arc::new(MockTool::new("tool4")),
        ];
        let added = registry.extend(tools2);
        assert_eq!(added, 2);
        assert_eq!(registry.len(), 4);

        // Verify all tools are present
        assert!(registry.has_tool("tool1"));
        assert!(registry.has_tool("tool2"));
        assert!(registry.has_tool("tool3"));
        assert!(registry.has_tool("tool4"));
    }

    #[test]
    fn test_registry_extend_replaces_existing() {
        let registry = ToolRegistry::new();

        let tools1: Vec<Arc<dyn Tool>> = vec![Arc::new(MockTool::new("tool1"))];
        registry.extend(tools1);
        assert_eq!(registry.len(), 1);

        // Extending with same name replaces
        let tools2: Vec<Arc<dyn Tool>> = vec![Arc::new(MockTool::new("tool1"))];
        let added = registry.extend(tools2);
        // Returns 0 because same key, net addition is 0
        assert_eq!(added, 0);
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_remove_by_prefix() {
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(MockTool::new("symbol_find")),
            Arc::new(MockTool::new("symbol_replace")),
            Arc::new(MockTool::new("symbol_rename")),
            Arc::new(MockTool::new("file_read")),
            Arc::new(MockTool::new("file_write")),
        ];
        let registry = ToolRegistry::from_tools(tools);
        assert_eq!(registry.len(), 5);

        // Remove all symbol tools
        let removed = registry.remove_by_prefix("symbol_");
        assert_eq!(removed, 3);
        assert_eq!(registry.len(), 2);

        // Verify symbol tools are gone
        assert!(!registry.has_tool("symbol_find"));
        assert!(!registry.has_tool("symbol_replace"));
        assert!(!registry.has_tool("symbol_rename"));

        // Verify file tools remain
        assert!(registry.has_tool("file_read"));
        assert!(registry.has_tool("file_write"));
    }

    #[test]
    fn test_registry_remove_by_prefix_no_matches() {
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(MockTool::new("file_read")),
            Arc::new(MockTool::new("file_write")),
        ];
        let registry = ToolRegistry::from_tools(tools);

        let removed = registry.remove_by_prefix("symbol_");
        assert_eq!(removed, 0);
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_registry_remove() {
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(MockTool::new("tool1")),
            Arc::new(MockTool::new("tool2")),
        ];
        let registry = ToolRegistry::from_tools(tools);

        let removed = registry.remove("tool1");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name(), "tool1");
        assert_eq!(registry.len(), 1);
        assert!(!registry.has_tool("tool1"));

        // Remove nonexistent returns None
        let not_found = registry.remove("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_builder_pattern() {
        let registry = ToolRegistryBuilder::new()
            .add_tool(Arc::new(MockTool::new("tool1")))
            .add_tool(Arc::new(MockTool::new("tool2")))
            .add_tools(vec![
                Arc::new(MockTool::new("tool3")),
                Arc::new(MockTool::new("tool4")),
            ])
            .build();

        assert_eq!(registry.len(), 4);
        assert!(registry.has_tool("tool1"));
        assert!(registry.has_tool("tool2"));
        assert!(registry.has_tool("tool3"));
        assert!(registry.has_tool("tool4"));
    }

    #[test]
    fn test_registry_clone_shares_state() {
        let registry = ToolRegistry::new();
        let cloned = registry.clone();

        // Extend original
        registry.extend(vec![Arc::new(MockTool::new("tool1"))]);

        // Clone sees the same state (shares Arc<RwLock<...>>)
        assert_eq!(cloned.len(), 1);
        assert!(cloned.has_tool("tool1"));

        // Extend via clone affects original
        cloned.extend(vec![Arc::new(MockTool::new("tool2"))]);
        assert_eq!(registry.len(), 2);
        assert!(registry.has_tool("tool2"));
    }

    #[test]
    fn test_registry_thread_safety() {
        use std::thread;

        let registry = ToolRegistry::new();
        let registry_clone1 = registry.clone();
        let registry_clone2 = registry.clone();

        // Spawn threads to extend concurrently
        let handle1 = thread::spawn(move || {
            for i in 0..100 {
                registry_clone1.extend(vec![Arc::new(MockTool::new(&format!("a_{}", i)))]);
            }
        });

        let handle2 = thread::spawn(move || {
            for i in 0..100 {
                registry_clone2.extend(vec![Arc::new(MockTool::new(&format!("b_{}", i)))]);
            }
        });

        handle1.join().unwrap();
        handle2.join().unwrap();

        // Should have all 200 tools (thread-safe)
        assert_eq!(registry.len(), 200);
    }

    #[test]
    fn test_default_implementations() {
        let registry: ToolRegistry = Default::default();
        assert!(registry.is_empty());

        let builder: ToolRegistryBuilder = Default::default();
        let built = builder.build();
        assert!(built.is_empty());
    }
}
