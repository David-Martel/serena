//! Project configuration structures

use crate::{language::Language, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Project-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Project name
    pub name: String,

    /// Project root directory
    pub root: PathBuf,

    /// Supported languages in this project
    #[serde(default)]
    pub languages: Vec<Language>,

    /// File encoding (default: utf-8)
    #[serde(default = "default_encoding")]
    pub encoding: String,

    /// Whether the project is read-only
    #[serde(default)]
    pub read_only: bool,

    /// Tools explicitly included for this project
    #[serde(default)]
    pub included_tools: Vec<String>,

    /// Tools explicitly excluded for this project
    #[serde(default)]
    pub excluded_tools: Vec<String>,

    /// Patterns to ignore when searching/indexing
    #[serde(default = "default_ignore_patterns")]
    pub ignore_patterns: Vec<String>,

    /// Maximum file size to process (in bytes)
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,

    /// Whether to enable symbol indexing
    #[serde(default = "default_true")]
    pub enable_indexing: bool,

    /// Whether to use memory/knowledge persistence
    #[serde(default = "default_true")]
    pub enable_memory: bool,

    /// Custom language server configurations
    #[serde(default)]
    pub language_server_config: std::collections::HashMap<String, serde_json::Value>,
}

fn default_encoding() -> String {
    "utf-8".to_string()
}

fn default_ignore_patterns() -> Vec<String> {
    vec![
        "node_modules/".to_string(),
        ".git/".to_string(),
        ".serena/".to_string(),
        "target/".to_string(),
        "build/".to_string(),
        "dist/".to_string(),
        "__pycache__/".to_string(),
        "*.pyc".to_string(),
        ".venv/".to_string(),
        "venv/".to_string(),
        ".mypy_cache/".to_string(),
        ".pytest_cache/".to_string(),
        "coverage/".to_string(),
        ".coverage".to_string(),
    ]
}

fn default_max_file_size() -> u64 {
    10 * 1024 * 1024 // 10 MB
}

fn default_true() -> bool {
    true
}

impl ProjectConfig {
    /// Create a new project configuration
    pub fn new(name: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            root: root.into(),
            languages: Vec::new(),
            encoding: default_encoding(),
            read_only: false,
            included_tools: Vec::new(),
            excluded_tools: Vec::new(),
            ignore_patterns: default_ignore_patterns(),
            max_file_size: default_max_file_size(),
            enable_indexing: true,
            enable_memory: true,
            language_server_config: std::collections::HashMap::new(),
        }
    }

    /// Check if a tool is enabled for this project
    pub fn is_tool_enabled(&self, tool_name: &str) -> bool {
        // If explicitly included, it's enabled
        if self.included_tools.contains(&tool_name.to_string()) {
            return true;
        }

        // If explicitly excluded, it's disabled
        if self.excluded_tools.contains(&tool_name.to_string()) {
            return false;
        }

        // Default: enabled
        true
    }

    /// Check if a file should be ignored
    pub fn should_ignore(&self, path: &str) -> bool {
        for pattern in &self.ignore_patterns {
            if pattern.ends_with('/') {
                // Directory pattern
                if path.contains(pattern) {
                    return true;
                }
            } else if pattern.contains('*') {
                // Glob pattern - simple implementation
                if Self::matches_glob(pattern, path) {
                    return true;
                }
            } else if path.contains(pattern) {
                return true;
            }
        }
        false
    }

    /// Simple glob matching (supports * wildcard)
    fn matches_glob(pattern: &str, path: &str) -> bool {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.is_empty() {
            return false;
        }

        let mut pos = 0;
        for (i, part) in parts.iter().enumerate() {
            if i == 0 {
                // First part must match at start
                if !path[pos..].starts_with(part) {
                    return false;
                }
                pos += part.len();
            } else if i == parts.len() - 1 {
                // Last part must match at end
                if !path.ends_with(part) {
                    return false;
                }
            } else {
                // Middle parts must be found
                if let Some(index) = path[pos..].find(part) {
                    pos += index + part.len();
                } else {
                    return false;
                }
            }
        }
        true
    }

    /// Detect languages from project files
    pub fn detect_languages(&mut self) -> Result<()> {
        use std::collections::HashSet;
        let mut detected = HashSet::new();

        // Walk the project directory
        if self.root.exists() {
            for entry in walkdir::WalkDir::new(&self.root)
                .max_depth(5)
                .follow_links(false)
                .into_iter()
                .flatten()
            {
                if entry.file_type().is_file() {
                    if let Some(ext) = entry.path().extension() {
                        if let Some(ext_str) = ext.to_str() {
                            if let Some(lang) = Language::from_extension(ext_str) {
                                detected.insert(lang);
                            }
                        }
                    }
                }
            }
        }

        self.languages = detected.into_iter().collect();
        Ok(())
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(crate::ConfigError::Invalid(
                "Project name cannot be empty".to_string(),
            ));
        }

        if !self.root.exists() {
            return Err(crate::ConfigError::Invalid(format!(
                "Project root does not exist: {}",
                self.root.display()
            )));
        }

        if !self.root.is_dir() {
            return Err(crate::ConfigError::Invalid(format!(
                "Project root is not a directory: {}",
                self.root.display()
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_project_config() {
        let config = ProjectConfig::new("test-project", "/tmp/test");
        assert_eq!(config.name, "test-project");
        assert_eq!(config.encoding, "utf-8");
        assert!(!config.read_only);
    }

    #[test]
    fn test_is_tool_enabled() {
        let mut config = ProjectConfig::new("test", "/tmp");
        assert!(config.is_tool_enabled("any_tool"));

        config.excluded_tools.push("disabled_tool".to_string());
        assert!(!config.is_tool_enabled("disabled_tool"));

        config.included_tools.push("disabled_tool".to_string());
        assert!(config.is_tool_enabled("disabled_tool"));
    }

    #[test]
    fn test_should_ignore() {
        let config = ProjectConfig::new("test", "/tmp");
        assert!(config.should_ignore("node_modules/package.json"));
        assert!(config.should_ignore("src/__pycache__/test.pyc"));
        assert!(config.should_ignore("test.pyc"));
        assert!(!config.should_ignore("src/main.rs"));
    }

    #[test]
    fn test_matches_glob() {
        assert!(ProjectConfig::matches_glob("*.pyc", "test.pyc"));
        assert!(ProjectConfig::matches_glob("*.py", "main.py"));
        assert!(!ProjectConfig::matches_glob("*.rs", "main.py"));
        assert!(ProjectConfig::matches_glob("test_*.py", "test_main.py"));
    }
}
