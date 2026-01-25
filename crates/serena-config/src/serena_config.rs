//! Main Serena configuration structure

use crate::{
    context_mode::{Context, Mode},
    loader::ConfigLoader,
    project::ProjectConfig,
    ConfigError, Result,
};
use serde::{Deserialize, Serialize};

/// Main Serena configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerenaConfig {
    /// Logging level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// Configured projects
    #[serde(default)]
    pub projects: Vec<ProjectConfig>,

    /// Default context to use
    #[serde(default = "default_context")]
    pub default_context: String,

    /// Default modes to activate
    #[serde(default = "default_modes")]
    pub default_modes: Vec<String>,

    /// Tool execution timeout in seconds
    #[serde(default)]
    pub tool_timeout: Option<u64>,

    /// Whether to enable web dashboard
    #[serde(default)]
    pub web_dashboard: bool,

    /// Web dashboard port
    #[serde(default = "default_web_port")]
    pub web_dashboard_port: u16,

    /// Available contexts
    #[serde(default = "default_contexts")]
    pub contexts: Vec<Context>,

    /// Available modes
    #[serde(default = "default_modes_list")]
    pub modes: Vec<Mode>,

    /// Maximum number of concurrent language servers
    #[serde(default = "default_max_language_servers")]
    pub max_language_servers: usize,

    /// Whether to enable automatic indexing
    #[serde(default = "default_true")]
    pub auto_index: bool,

    /// Whether to enable memory persistence
    #[serde(default = "default_true")]
    pub enable_memory: bool,

    /// Memory directory path (relative to project root)
    #[serde(default = "default_memory_dir")]
    pub memory_dir: String,

    /// Custom configuration values
    #[serde(default)]
    pub custom: std::collections::HashMap<String, serde_json::Value>,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_context() -> String {
    "desktop-app".to_string()
}

fn default_modes() -> Vec<String> {
    vec!["interactive".to_string(), "editing".to_string()]
}

fn default_web_port() -> u16 {
    3000
}

fn default_contexts() -> Vec<Context> {
    Context::defaults()
}

fn default_modes_list() -> Vec<Mode> {
    Mode::defaults()
}

fn default_max_language_servers() -> usize {
    10
}

fn default_true() -> bool {
    true
}

fn default_memory_dir() -> String {
    ".serena/memories".to_string()
}

impl Default for SerenaConfig {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
            projects: Vec::new(),
            default_context: default_context(),
            default_modes: default_modes(),
            tool_timeout: None,
            web_dashboard: false,
            web_dashboard_port: default_web_port(),
            contexts: default_contexts(),
            modes: default_modes_list(),
            max_language_servers: default_max_language_servers(),
            auto_index: true,
            enable_memory: true,
            memory_dir: default_memory_dir(),
            custom: std::collections::HashMap::new(),
        }
    }
}

impl SerenaConfig {
    /// Load configuration from default locations
    pub fn load() -> Result<Self> {
        ConfigLoader::new().load()
    }

    /// Load configuration from a specific file
    pub fn load_from(path: impl AsRef<std::path::Path>) -> Result<Self> {
        ConfigLoader::new().load_from_file(path.as_ref())
    }

    /// Save configuration to a file
    pub fn save(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        ConfigLoader::new().save_to_file(self, path.as_ref())
    }

    /// Get a project by name
    pub fn get_project(&self, name: &str) -> Option<&ProjectConfig> {
        self.projects.iter().find(|p| p.name == name)
    }

    /// Get a mutable project by name
    pub fn get_project_mut(&mut self, name: &str) -> Option<&mut ProjectConfig> {
        self.projects.iter_mut().find(|p| p.name == name)
    }

    /// Add or update a project
    pub fn upsert_project(&mut self, project: ProjectConfig) {
        if let Some(existing) = self.get_project_mut(&project.name) {
            *existing = project;
        } else {
            self.projects.push(project);
        }
    }

    /// Remove a project by name
    pub fn remove_project(&mut self, name: &str) -> bool {
        if let Some(pos) = self.projects.iter().position(|p| p.name == name) {
            self.projects.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get a context by name
    pub fn get_context(&self, name: &str) -> Option<&Context> {
        self.contexts.iter().find(|c| c.name == name)
    }

    /// Get a mode by name
    pub fn get_mode(&self, name: &str) -> Option<&Mode> {
        self.modes.iter().find(|m| m.name == name)
    }

    /// Check if a context exists
    pub fn has_context(&self, name: &str) -> bool {
        self.get_context(name).is_some()
    }

    /// Check if a mode exists
    pub fn has_mode(&self, name: &str) -> bool {
        self.get_mode(name).is_some()
    }

    /// Merge another configuration into this one
    pub fn merge(&mut self, other: SerenaConfig) {
        // Merge projects (upsert by name)
        for project in other.projects {
            self.upsert_project(project);
        }

        // Merge contexts (add new ones)
        for context in other.contexts {
            if !self.has_context(&context.name) {
                self.contexts.push(context);
            }
        }

        // Merge modes (add new ones)
        for mode in other.modes {
            if !self.has_mode(&mode.name) {
                self.modes.push(mode);
            }
        }

        // Merge custom settings
        for (key, value) in other.custom {
            self.custom.insert(key, value);
        }

        // Override scalar values if they differ from defaults
        if other.log_level != default_log_level() {
            self.log_level = other.log_level;
        }
        if other.default_context != default_context() {
            self.default_context = other.default_context;
        }
        if !other.default_modes.is_empty() {
            self.default_modes = other.default_modes;
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.log_level.as_str()) {
            return Err(ConfigError::Invalid(format!(
                "Invalid log level: {}. Must be one of: {}",
                self.log_level,
                valid_levels.join(", ")
            )));
        }

        // Validate default context exists
        if !self.has_context(&self.default_context) {
            return Err(ConfigError::Invalid(format!(
                "Default context '{}' not found in available contexts",
                self.default_context
            )));
        }

        // Validate default modes exist
        for mode_name in &self.default_modes {
            if !self.has_mode(mode_name) {
                return Err(ConfigError::Invalid(format!(
                    "Default mode '{}' not found in available modes",
                    mode_name
                )));
            }
        }

        // Validate all projects
        for project in &self.projects {
            project.validate()?;
        }

        Ok(())
    }

    /// Get active tool set based on context and project
    pub fn get_active_tools(
        &self,
        context_name: &str,
        project: Option<&ProjectConfig>,
    ) -> Vec<String> {
        let mut tools = Vec::new();

        // Get tools from context
        if let Some(context) = self.get_context(context_name) {
            tools.extend(context.tools.clone());
        }

        // Filter based on project settings
        if let Some(proj) = project {
            tools.retain(|tool| proj.is_tool_enabled(tool));
        }

        tools
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_default_config() {
        let config = SerenaConfig::default();
        assert_eq!(config.log_level, "info");
        assert_eq!(config.default_context, "desktop-app");
        assert!(!config.web_dashboard);
        assert_eq!(config.web_dashboard_port, 3000);
    }

    #[test]
    fn test_upsert_project() {
        let mut config = SerenaConfig::default();
        let project = ProjectConfig::new("test-project", PathBuf::from("/tmp/test"));

        config.upsert_project(project.clone());
        assert_eq!(config.projects.len(), 1);

        let mut updated = project.clone();
        updated.read_only = true;
        config.upsert_project(updated);
        assert_eq!(config.projects.len(), 1);
        assert!(config.projects[0].read_only);
    }

    #[test]
    fn test_get_project() {
        let mut config = SerenaConfig::default();
        let project = ProjectConfig::new("test-project", PathBuf::from("/tmp/test"));
        config.projects.push(project);

        assert!(config.get_project("test-project").is_some());
        assert!(config.get_project("nonexistent").is_none());
    }

    #[test]
    fn test_remove_project() {
        let mut config = SerenaConfig::default();
        let project = ProjectConfig::new("test-project", PathBuf::from("/tmp/test"));
        config.projects.push(project);

        assert!(config.remove_project("test-project"));
        assert_eq!(config.projects.len(), 0);
        assert!(!config.remove_project("nonexistent"));
    }

    #[test]
    fn test_get_context() {
        let config = SerenaConfig::default();
        assert!(config.get_context("desktop-app").is_some());
        assert!(config.get_context("nonexistent").is_none());
    }

    #[test]
    fn test_validate() {
        let config = SerenaConfig::default();
        assert!(config.validate().is_ok());

        let invalid = SerenaConfig {
            log_level: "invalid".to_string(),
            ..Default::default()
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_merge() {
        let mut config1 = SerenaConfig::default();
        let config2 = SerenaConfig {
            log_level: "debug".to_string(),
            projects: vec![ProjectConfig::new("new-project", PathBuf::from("/tmp/new"))],
            ..Default::default()
        };

        config1.merge(config2);
        assert_eq!(config1.log_level, "debug");
        assert_eq!(config1.projects.len(), 1);
    }

    #[test]
    fn test_get_active_tools() {
        let config = SerenaConfig::default();
        let tools = config.get_active_tools("desktop-app", None);
        assert!(!tools.is_empty());

        let mut project = ProjectConfig::new("test", PathBuf::from("/tmp"));
        project
            .excluded_tools
            .push("execute_shell_command".to_string());
        let filtered_tools = config.get_active_tools("desktop-app", Some(&project));
        assert!(!filtered_tools.contains(&"execute_shell_command".to_string()));
    }
}
