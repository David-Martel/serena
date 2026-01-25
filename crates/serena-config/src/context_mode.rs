//! Context and mode configurations

use serde::{Deserialize, Serialize};

/// A context defines the environment and available tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// Context name
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// Tools available in this context
    pub tools: Vec<String>,

    /// Whether this is a default context
    #[serde(default)]
    pub is_default: bool,

    /// Additional settings for this context
    #[serde(default)]
    pub settings: std::collections::HashMap<String, serde_json::Value>,
}

impl Context {
    /// Create a new context
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            tools: Vec::new(),
            is_default: false,
            settings: std::collections::HashMap::new(),
        }
    }

    /// Add a tool to this context
    pub fn with_tool(mut self, tool: impl Into<String>) -> Self {
        self.tools.push(tool.into());
        self
    }

    /// Add multiple tools to this context
    pub fn with_tools(mut self, tools: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tools.extend(tools.into_iter().map(|t| t.into()));
        self
    }

    /// Set as default context
    pub fn as_default(mut self) -> Self {
        self.is_default = true;
        self
    }

    /// Check if a tool is available in this context
    pub fn has_tool(&self, tool: &str) -> bool {
        self.tools.contains(&tool.to_string())
    }
}

/// A mode defines operational behavior and patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mode {
    /// Mode name
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// Behavioral flags for this mode
    pub behaviors: Vec<String>,

    /// Whether this is a default mode
    #[serde(default)]
    pub is_default: bool,

    /// Additional settings for this mode
    #[serde(default)]
    pub settings: std::collections::HashMap<String, serde_json::Value>,
}

impl Mode {
    /// Create a new mode
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            behaviors: Vec::new(),
            is_default: false,
            settings: std::collections::HashMap::new(),
        }
    }

    /// Add a behavior to this mode
    pub fn with_behavior(mut self, behavior: impl Into<String>) -> Self {
        self.behaviors.push(behavior.into());
        self
    }

    /// Add multiple behaviors to this mode
    pub fn with_behaviors(
        mut self,
        behaviors: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.behaviors
            .extend(behaviors.into_iter().map(|b| b.into()));
        self
    }

    /// Set as default mode
    pub fn as_default(mut self) -> Self {
        self.is_default = true;
        self
    }

    /// Check if a behavior is enabled in this mode
    pub fn has_behavior(&self, behavior: &str) -> bool {
        self.behaviors.contains(&behavior.to_string())
    }
}

/// Built-in contexts
impl Context {
    /// Desktop application context - full tool access
    pub fn desktop_app() -> Self {
        Self::new("desktop-app", "Full desktop application with all tools")
            .with_tools([
                "read_file",
                "write_file",
                "list_dir",
                "search_files",
                "find_symbol",
                "replace_symbol_body",
                "execute_shell_command",
                "read_memory",
                "write_memory",
            ])
            .as_default()
    }

    /// IDE assistant context - code editing focused
    pub fn ide_assistant() -> Self {
        Self::new("ide-assistant", "IDE assistant with code editing tools").with_tools([
            "read_file",
            "write_file",
            "find_symbol",
            "replace_symbol_body",
            "insert_after_symbol",
            "insert_before_symbol",
            "rename_symbol",
            "search_for_pattern",
        ])
    }

    /// Agent context - autonomous operation
    pub fn agent() -> Self {
        Self::new("agent", "Autonomous agent with extended capabilities").with_tools([
            "read_file",
            "write_file",
            "list_dir",
            "find_file",
            "search_for_pattern",
            "find_symbol",
            "replace_symbol_body",
            "execute_shell_command",
            "read_memory",
            "write_memory",
            "activate_project",
            "switch_modes",
        ])
    }

    /// Get all default contexts
    pub fn defaults() -> Vec<Self> {
        vec![Self::desktop_app(), Self::ide_assistant(), Self::agent()]
    }
}

/// Built-in modes
impl Mode {
    /// Planning mode - analysis and design
    pub fn planning() -> Self {
        Self::new("planning", "Analysis and planning mode")
            .with_behaviors(["read-only", "analytical", "high-level"])
            .as_default()
    }

    /// Editing mode - code modification
    pub fn editing() -> Self {
        Self::new("editing", "Code editing and modification mode").with_behaviors([
            "write-enabled",
            "precise",
            "symbol-aware",
        ])
    }

    /// Interactive mode - conversational assistance
    pub fn interactive() -> Self {
        Self::new("interactive", "Interactive conversational mode")
            .with_behaviors(["conversational", "explanatory", "step-by-step"])
            .as_default()
    }

    /// One-shot mode - single task execution
    pub fn one_shot() -> Self {
        Self::new("one-shot", "Single task execution mode").with_behaviors([
            "focused",
            "completion-oriented",
            "minimal-output",
        ])
    }

    /// Get all default modes
    pub fn defaults() -> Vec<Self> {
        vec![
            Self::planning(),
            Self::editing(),
            Self::interactive(),
            Self::one_shot(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let ctx = Context::new("test", "Test context")
            .with_tool("tool1")
            .with_tool("tool2");

        assert_eq!(ctx.name, "test");
        assert_eq!(ctx.tools.len(), 2);
        assert!(ctx.has_tool("tool1"));
        assert!(!ctx.has_tool("tool3"));
    }

    #[test]
    fn test_mode_creation() {
        let mode = Mode::new("test", "Test mode")
            .with_behavior("behavior1")
            .with_behavior("behavior2");

        assert_eq!(mode.name, "test");
        assert_eq!(mode.behaviors.len(), 2);
        assert!(mode.has_behavior("behavior1"));
        assert!(!mode.has_behavior("behavior3"));
    }

    #[test]
    fn test_default_contexts() {
        let contexts = Context::defaults();
        assert!(contexts.len() >= 3);
        assert!(contexts.iter().any(|c| c.name == "desktop-app"));
        assert!(contexts.iter().any(|c| c.name == "ide-assistant"));
        assert!(contexts.iter().any(|c| c.name == "agent"));
    }

    #[test]
    fn test_default_modes() {
        let modes = Mode::defaults();
        assert!(modes.len() >= 4);
        assert!(modes.iter().any(|m| m.name == "planning"));
        assert!(modes.iter().any(|m| m.name == "editing"));
        assert!(modes.iter().any(|m| m.name == "interactive"));
        assert!(modes.iter().any(|m| m.name == "one-shot"));
    }
}
