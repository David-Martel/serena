use serena_core::Tool;
use std::collections::HashMap;
use std::sync::Arc;

/// Registry for managing tool instances
///
/// Uses Arc<dyn Tool> for thread-safe shared ownership, consistent with serena-core.
#[derive(Clone)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a new empty tool registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool in the registry
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools.insert(name, tool);
    }

    /// Register a boxed tool (convenience method that wraps in Arc)
    pub fn register_boxed(&mut self, tool: Box<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools.insert(name, Arc::from(tool));
    }

    /// Get a tool by name (returns Arc for shared ownership)
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// Get a reference to a tool by name
    pub fn get_ref(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    /// List all registered tool names
    pub fn list(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }

    /// Get all tools as Arc references
    pub fn list_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.values().cloned().collect()
    }

    /// Get the number of registered tools
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Remove a tool from the registry
    pub fn remove(&mut self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.remove(name)
    }

    /// Clear all tools from the registry
    pub fn clear(&mut self) {
        self.tools.clear();
    }

    /// Check if a tool is registered
    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get all tools matching a tag
    pub fn get_by_tag(&self, tag: &str) -> Vec<Arc<dyn Tool>> {
        self.tools
            .values()
            .filter(|t| t.tags().iter().any(|t_tag| t_tag == tag))
            .cloned()
            .collect()
    }

    /// Get tool names matching a tag
    pub fn get_names_by_tag(&self, tag: &str) -> Vec<&str> {
        self.tools
            .iter()
            .filter(|(_, t)| t.tags().iter().any(|t_tag| t_tag == tag))
            .map(|(name, _)| name.as_str())
            .collect()
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

    pub fn add_boxed_tool(mut self, tool: Box<dyn Tool>) -> Self {
        self.tools.push(Arc::from(tool));
        self
    }

    pub fn add_tools(mut self, tools: Vec<Arc<dyn Tool>>) -> Self {
        self.tools.extend(tools);
        self
    }

    pub fn build(self) -> ToolRegistry {
        let mut registry = ToolRegistry::new();
        for tool in self.tools {
            registry.register(tool);
        }
        registry
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
    use async_trait::async_trait;
    use serde_json::Value;
    use serena_core::{SerenaError, ToolResult};

    struct MockTool {
        name: String,
        tags: Vec<String>,
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "A mock tool"
        }

        fn parameters_schema(&self) -> Value {
            serde_json::json!({})
        }

        async fn execute(&self, _params: Value) -> Result<ToolResult, SerenaError> {
            Ok(ToolResult::success(serde_json::json!({})))
        }

        fn tags(&self) -> Vec<String> {
            self.tags.clone()
        }
    }

    #[test]
    fn test_registry_new() {
        let registry = ToolRegistry::new();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_register_and_get() {
        let mut registry = ToolRegistry::new();
        let tool: Arc<dyn Tool> = Arc::new(MockTool {
            name: "test_tool".to_string(),
            tags: vec![],
        });
        registry.register(tool);

        assert_eq!(registry.len(), 1);
        assert!(registry.contains("test_tool"));
        assert!(registry.get("test_tool").is_some());
    }

    #[test]
    fn test_register_boxed() {
        let mut registry = ToolRegistry::new();
        let tool: Box<dyn Tool> = Box::new(MockTool {
            name: "test_tool".to_string(),
            tags: vec![],
        });
        registry.register_boxed(tool);

        assert_eq!(registry.len(), 1);
        assert!(registry.contains("test_tool"));
    }

    #[test]
    fn test_list() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool {
            name: "tool1".to_string(),
            tags: vec![],
        }));
        registry.register(Arc::new(MockTool {
            name: "tool2".to_string(),
            tags: vec![],
        }));

        let names = registry.list();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"tool1"));
        assert!(names.contains(&"tool2"));
    }

    #[test]
    fn test_get_by_tag() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool {
            name: "tool1".to_string(),
            tags: vec!["file".to_string()],
        }));
        registry.register(Arc::new(MockTool {
            name: "tool2".to_string(),
            tags: vec!["symbol".to_string()],
        }));
        registry.register(Arc::new(MockTool {
            name: "tool3".to_string(),
            tags: vec!["file".to_string(), "search".to_string()],
        }));

        let file_tools = registry.get_by_tag("file");
        assert_eq!(file_tools.len(), 2);
    }

    #[test]
    fn test_builder() {
        let registry = ToolRegistryBuilder::new()
            .add_tool(Arc::new(MockTool {
                name: "tool1".to_string(),
                tags: vec![],
            }))
            .add_boxed_tool(Box::new(MockTool {
                name: "tool2".to_string(),
                tags: vec![],
            }))
            .build();

        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_clone() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool {
            name: "tool1".to_string(),
            tags: vec![],
        }));

        let cloned = registry.clone();
        assert_eq!(cloned.len(), 1);
        assert!(cloned.contains("tool1"));
    }
}
