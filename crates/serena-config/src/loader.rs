//! Configuration file loading and discovery

use crate::{ConfigError, Result, SerenaConfig};
use std::path::{Path, PathBuf};

/// Configuration loader with support for multiple file formats and locations
pub struct ConfigLoader {
    /// Paths to search for configuration files (in order)
    search_paths: Vec<PathBuf>,
}

impl ConfigLoader {
    /// Create a new configuration loader
    pub fn new() -> Self {
        Self {
            search_paths: Self::default_search_paths(),
        }
    }

    /// Get default search paths for configuration files
    fn default_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // 1. Current directory
        if let Ok(current) = std::env::current_dir() {
            paths.push(current.join(".serena"));
            paths.push(current);
        }

        // 2. User home directory
        if let Some(home) = directories::UserDirs::new() {
            paths.push(home.home_dir().join(".serena"));
        }

        // 3. System config directory
        if let Some(config_dir) = directories::ProjectDirs::from("", "", "serena") {
            paths.push(config_dir.config_dir().to_path_buf());
        }

        paths
    }

    /// Add a custom search path
    pub fn add_search_path(&mut self, path: impl Into<PathBuf>) {
        self.search_paths.push(path.into());
    }

    /// Find configuration file in search paths
    pub fn find_config_file(&self) -> Option<PathBuf> {
        let config_names = ["serena_config.yml", "serena_config.yaml", "config.yml"];

        for search_path in &self.search_paths {
            for config_name in &config_names {
                let candidate = search_path.join(config_name);
                if candidate.exists() && candidate.is_file() {
                    tracing::debug!("Found config file: {}", candidate.display());
                    return Some(candidate);
                }
            }
        }

        None
    }

    /// Find project configuration file
    pub fn find_project_config(&self, project_root: &Path) -> Option<PathBuf> {
        let config_names = ["project.yml", "project.yaml", ".serena/project.yml"];

        for config_name in &config_names {
            let candidate = project_root.join(config_name);
            if candidate.exists() && candidate.is_file() {
                tracing::debug!("Found project config: {}", candidate.display());
                return Some(candidate);
            }
        }

        None
    }

    /// Load configuration from a file
    pub fn load_from_file(&self, path: &Path) -> Result<SerenaConfig> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            tracing::error!("Failed to read config file {}: {}", path.display(), e);
            ConfigError::Io(e)
        })?;

        let config = if path
            .extension()
            .and_then(|e| e.to_str()) == Some("json")
        {
            serde_json::from_str(&content)?
        } else {
            serde_yaml::from_str(&content)?
        };

        tracing::info!("Loaded config from: {}", path.display());
        Ok(config)
    }

    /// Load configuration with fallback to defaults
    pub fn load(&self) -> Result<SerenaConfig> {
        match self.find_config_file() {
            Some(config_path) => self.load_from_file(&config_path),
            None => {
                tracing::info!("No config file found, using defaults");
                Ok(SerenaConfig::default())
            }
        }
    }

    /// Load and merge multiple configuration sources
    pub fn load_merged(&self, additional_paths: &[PathBuf]) -> Result<SerenaConfig> {
        let mut config = self.load()?;

        for path in additional_paths {
            if path.exists() {
                let additional = self.load_from_file(path)?;
                config.merge(additional);
            }
        }

        Ok(config)
    }

    /// Save configuration to a file
    pub fn save_to_file(&self, config: &SerenaConfig, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = if path
            .extension()
            .and_then(|e| e.to_str()) == Some("json")
        {
            serde_json::to_string_pretty(config)?
        } else {
            serde_yaml::to_string(config)?
        };

        std::fs::write(path, content)?;
        tracing::info!("Saved config to: {}", path.display());

        Ok(())
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Load configuration from default locations
pub fn load_config() -> Result<SerenaConfig> {
    ConfigLoader::new().load()
}

/// Load configuration from a specific file
pub fn load_config_from(path: impl AsRef<Path>) -> Result<SerenaConfig> {
    ConfigLoader::new().load_from_file(path.as_ref())
}

/// Save configuration to a file
pub fn save_config(config: &SerenaConfig, path: impl AsRef<Path>) -> Result<()> {
    ConfigLoader::new().save_to_file(config, path.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_loader_creation() {
        let loader = ConfigLoader::new();
        assert!(!loader.search_paths.is_empty());
    }

    #[test]
    fn test_add_search_path() {
        let mut loader = ConfigLoader::new();
        let original_len = loader.search_paths.len();
        loader.add_search_path("/tmp/custom");
        assert_eq!(loader.search_paths.len(), original_len + 1);
    }

    #[test]
    fn test_load_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.yml");

        let yaml_content = r#"
log_level: debug
web_dashboard: true
web_dashboard_port: 3000
default_context: agent
default_modes:
  - editing
  - interactive
"#;

        std::fs::write(&config_path, yaml_content).unwrap();

        let loader = ConfigLoader::new();
        let config = loader.load_from_file(&config_path).unwrap();

        assert_eq!(config.log_level, "debug");
        assert!(config.web_dashboard);
        assert_eq!(config.web_dashboard_port, 3000);
    }

    #[test]
    fn test_save_to_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("saved_config.yml");

        let config = SerenaConfig::default();
        let loader = ConfigLoader::new();

        loader.save_to_file(&config, &config_path).unwrap();
        assert!(config_path.exists());

        let loaded = loader.load_from_file(&config_path).unwrap();
        assert_eq!(loaded.log_level, config.log_level);
    }
}
