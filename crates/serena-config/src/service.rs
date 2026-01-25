//! Configuration service for managing Serena configuration state
//!
//! Provides a higher-level interface for configuration operations with
//! automatic persistence and project lifecycle management.

use crate::{ConfigError, ProjectConfig, Result, SerenaConfig};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tracing::{debug, info, warn};

/// Configuration service for managing Serena configuration
///
/// Thread-safe wrapper around SerenaConfig that provides:
/// - Automatic config loading and saving
/// - Project activation and deactivation
/// - Mode switching
/// - Config change notifications
pub struct ConfigService {
    config: Arc<RwLock<SerenaConfig>>,
    config_path: Option<PathBuf>,
    active_project: Arc<RwLock<Option<String>>>,
    active_context: Arc<RwLock<String>>,
    active_modes: Arc<RwLock<Vec<String>>>,
}

impl ConfigService {
    /// Create a new ConfigService with default configuration
    pub fn new() -> Self {
        let config = SerenaConfig::default();
        let default_context = config.default_context.clone();
        let default_modes = config.default_modes.clone();

        Self {
            config: Arc::new(RwLock::new(config)),
            config_path: None,
            active_project: Arc::new(RwLock::new(None)),
            active_context: Arc::new(RwLock::new(default_context)),
            active_modes: Arc::new(RwLock::new(default_modes)),
        }
    }

    /// Create a ConfigService by loading configuration from default locations
    pub fn load() -> Result<Self> {
        let config = SerenaConfig::load()?;
        let default_context = config.default_context.clone();
        let default_modes = config.default_modes.clone();

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            config_path: None,
            active_project: Arc::new(RwLock::new(None)),
            active_context: Arc::new(RwLock::new(default_context)),
            active_modes: Arc::new(RwLock::new(default_modes)),
        })
    }

    /// Create a ConfigService by loading from a specific file
    pub fn load_from(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let config = SerenaConfig::load_from(&path)?;
        let default_context = config.default_context.clone();
        let default_modes = config.default_modes.clone();

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            config_path: Some(path),
            active_project: Arc::new(RwLock::new(None)),
            active_context: Arc::new(RwLock::new(default_context)),
            active_modes: Arc::new(RwLock::new(default_modes)),
        })
    }

    /// Activate a project by name or path
    ///
    /// If `name_or_path` matches an existing project name, that project is activated.
    /// If it's a path, a new project is created with the directory name as the project name.
    pub fn activate_project(&self, name_or_path: &str) -> Result<ProjectConfig> {
        let path = Path::new(name_or_path);

        // First, check if it's an existing project name
        if let Ok(config) = self.config.read() {
            if let Some(project) = config.get_project(name_or_path) {
                debug!("Activating existing project: {}", name_or_path);
                let project_clone = project.clone();

                // Set as active
                if let Ok(mut active) = self.active_project.write() {
                    *active = Some(name_or_path.to_string());
                }

                info!("Activated project: {}", project_clone.name);
                return Ok(project_clone);
            }
        }

        // Check if it's a valid path
        if path.exists() && path.is_dir() {
            let project_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unnamed-project")
                .to_string();

            let canonical_path = path.canonicalize().map_err(|e| {
                ConfigError::Invalid(format!("Cannot canonicalize path: {}", e))
            })?;

            debug!("Creating new project from path: {:?}", canonical_path);

            let mut project = ProjectConfig::new(&project_name, canonical_path);

            // Auto-detect languages
            if let Err(e) = project.detect_languages() {
                warn!("Failed to detect languages for {}: {}", project_name, e);
            }

            // Add to config
            if let Ok(mut config) = self.config.write() {
                config.upsert_project(project.clone());
            }

            // Set as active
            if let Ok(mut active) = self.active_project.write() {
                *active = Some(project_name.clone());
            }

            info!("Activated new project: {}", project_name);
            return Ok(project);
        }

        Err(ConfigError::ProjectNotFound(format!(
            "Project or path not found: {}",
            name_or_path
        )))
    }

    /// Deactivate the current project
    pub fn deactivate_project(&self) -> Option<String> {
        if let Ok(mut active) = self.active_project.write() {
            let prev = active.take();
            if let Some(ref name) = prev {
                info!("Deactivated project: {}", name);
            }
            prev
        } else {
            None
        }
    }

    /// Get the currently active project configuration
    pub fn get_active_project(&self) -> Option<ProjectConfig> {
        let active_name = self.active_project.read().ok()?.clone()?;
        self.config
            .read()
            .ok()?
            .get_project(&active_name)
            .cloned()
    }

    /// Get the active project name
    pub fn get_active_project_name(&self) -> Option<String> {
        self.active_project.read().ok()?.clone()
    }

    /// Remove a project from the configuration
    pub fn remove_project(&self, name: &str) -> Result<bool> {
        // Can't remove the active project
        if let Ok(active) = self.active_project.read() {
            if active.as_deref() == Some(name) {
                return Err(ConfigError::Invalid(format!(
                    "Cannot remove active project '{}'. Deactivate it first.",
                    name
                )));
            }
        }

        if let Ok(mut config) = self.config.write() {
            let removed = config.remove_project(name);
            if removed {
                info!("Removed project: {}", name);
            }
            Ok(removed)
        } else {
            Err(ConfigError::Invalid("Failed to acquire config lock".to_string()))
        }
    }

    /// Get the current configuration (read-only clone)
    pub fn get_config(&self) -> Result<SerenaConfig> {
        self.config
            .read()
            .map(|c| c.clone())
            .map_err(|_| ConfigError::Invalid("Failed to acquire config lock".to_string()))
    }

    /// Update the configuration with a function
    pub fn update_config<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce(&mut SerenaConfig),
    {
        if let Ok(mut config) = self.config.write() {
            f(&mut config);
            Ok(())
        } else {
            Err(ConfigError::Invalid("Failed to acquire config lock".to_string()))
        }
    }

    /// Save the current configuration to file
    pub fn save(&self) -> Result<()> {
        let config = self.get_config()?;

        if let Some(ref path) = self.config_path {
            config.save(path)?;
            info!("Saved configuration to: {:?}", path);
        } else {
            // Save to default location using directories crate
            let base_dirs = directories::BaseDirs::new()
                .ok_or_else(|| ConfigError::Invalid("Cannot determine home directory".to_string()))?;

            let default_path = base_dirs.home_dir().join(".serena").join("serena_config.yml");

            if let Some(parent) = default_path.parent() {
                std::fs::create_dir_all(parent).map_err(ConfigError::Io)?;
            }

            config.save(&default_path)?;
            info!("Saved configuration to: {:?}", default_path);
        }

        Ok(())
    }

    /// Switch the active context
    pub fn switch_context(&self, context_name: &str) -> Result<()> {
        // Verify context exists
        if let Ok(config) = self.config.read() {
            if !config.has_context(context_name) {
                return Err(ConfigError::Invalid(format!(
                    "Context '{}' not found",
                    context_name
                )));
            }
        }

        if let Ok(mut active) = self.active_context.write() {
            *active = context_name.to_string();
            info!("Switched to context: {}", context_name);
            Ok(())
        } else {
            Err(ConfigError::Invalid("Failed to acquire context lock".to_string()))
        }
    }

    /// Get the active context name
    pub fn get_active_context(&self) -> Result<String> {
        self.active_context
            .read()
            .map(|c| c.clone())
            .map_err(|_| ConfigError::Invalid("Failed to acquire context lock".to_string()))
    }

    /// Switch modes
    pub fn switch_modes(&self, mode_names: Vec<String>) -> Result<()> {
        // Verify all modes exist
        if let Ok(config) = self.config.read() {
            for mode in &mode_names {
                if !config.has_mode(mode) {
                    return Err(ConfigError::Invalid(format!("Mode '{}' not found", mode)));
                }
            }
        }

        if let Ok(mut active) = self.active_modes.write() {
            *active = mode_names.clone();
            info!("Switched to modes: {:?}", mode_names);
            Ok(())
        } else {
            Err(ConfigError::Invalid("Failed to acquire modes lock".to_string()))
        }
    }

    /// Add a mode to active modes
    pub fn add_mode(&self, mode_name: &str) -> Result<()> {
        // Verify mode exists
        if let Ok(config) = self.config.read() {
            if !config.has_mode(mode_name) {
                return Err(ConfigError::Invalid(format!("Mode '{}' not found", mode_name)));
            }
        }

        if let Ok(mut active) = self.active_modes.write() {
            if !active.contains(&mode_name.to_string()) {
                active.push(mode_name.to_string());
                info!("Added mode: {}", mode_name);
            }
            Ok(())
        } else {
            Err(ConfigError::Invalid("Failed to acquire modes lock".to_string()))
        }
    }

    /// Remove a mode from active modes
    pub fn remove_mode(&self, mode_name: &str) -> Result<bool> {
        if let Ok(mut active) = self.active_modes.write() {
            if let Some(pos) = active.iter().position(|m| m == mode_name) {
                active.remove(pos);
                info!("Removed mode: {}", mode_name);
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Err(ConfigError::Invalid("Failed to acquire modes lock".to_string()))
        }
    }

    /// Get active modes
    pub fn get_active_modes(&self) -> Result<Vec<String>> {
        self.active_modes
            .read()
            .map(|m| m.clone())
            .map_err(|_| ConfigError::Invalid("Failed to acquire modes lock".to_string()))
    }

    /// List all projects
    pub fn list_projects(&self) -> Result<Vec<ProjectConfig>> {
        self.config
            .read()
            .map(|c| c.projects.clone())
            .map_err(|_| ConfigError::Invalid("Failed to acquire config lock".to_string()))
    }

    /// List all contexts
    pub fn list_contexts(&self) -> Result<Vec<String>> {
        self.config
            .read()
            .map(|c| c.contexts.iter().map(|ctx| ctx.name.clone()).collect())
            .map_err(|_| ConfigError::Invalid("Failed to acquire config lock".to_string()))
    }

    /// List all modes
    pub fn list_modes(&self) -> Result<Vec<String>> {
        self.config
            .read()
            .map(|c| c.modes.iter().map(|m| m.name.clone()).collect())
            .map_err(|_| ConfigError::Invalid("Failed to acquire config lock".to_string()))
    }

    /// Get active tools based on current context and project
    pub fn get_active_tools(&self) -> Result<Vec<String>> {
        let context = self.get_active_context()?;
        let project = self.get_active_project();

        self.config
            .read()
            .map(|c| c.get_active_tools(&context, project.as_ref()))
            .map_err(|_| ConfigError::Invalid("Failed to acquire config lock".to_string()))
    }
}

impl Default for ConfigService {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ConfigService {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            config_path: self.config_path.clone(),
            active_project: Arc::clone(&self.active_project),
            active_context: Arc::clone(&self.active_context),
            active_modes: Arc::clone(&self.active_modes),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_new_service() {
        let service = ConfigService::new();
        assert!(service.get_active_project().is_none());
        assert_eq!(service.get_active_context().unwrap(), "desktop-app");
    }

    #[test]
    fn test_activate_project_by_path() {
        let temp_dir = TempDir::new().unwrap();
        let service = ConfigService::new();

        let project = service
            .activate_project(temp_dir.path().to_str().unwrap())
            .unwrap();

        assert!(!project.name.is_empty());
        assert!(service.get_active_project().is_some());
    }

    #[test]
    fn test_deactivate_project() {
        let temp_dir = TempDir::new().unwrap();
        let service = ConfigService::new();

        service
            .activate_project(temp_dir.path().to_str().unwrap())
            .unwrap();
        assert!(service.get_active_project().is_some());

        service.deactivate_project();
        assert!(service.get_active_project().is_none());
    }

    #[test]
    fn test_switch_context() {
        let service = ConfigService::new();

        // Default contexts exist
        assert!(service.switch_context("desktop-app").is_ok());
        assert_eq!(service.get_active_context().unwrap(), "desktop-app");

        assert!(service.switch_context("agent").is_ok());
        assert_eq!(service.get_active_context().unwrap(), "agent");

        // Invalid context
        assert!(service.switch_context("nonexistent").is_err());
    }

    #[test]
    fn test_switch_modes() {
        let service = ConfigService::new();

        // Default modes exist
        assert!(service
            .switch_modes(vec!["interactive".to_string()])
            .is_ok());
        assert_eq!(service.get_active_modes().unwrap(), vec!["interactive"]);

        // Invalid mode
        assert!(service.switch_modes(vec!["nonexistent".to_string()]).is_err());
    }

    #[test]
    fn test_list_projects() {
        let temp_dir = TempDir::new().unwrap();
        let service = ConfigService::new();

        assert!(service.list_projects().unwrap().is_empty());

        service
            .activate_project(temp_dir.path().to_str().unwrap())
            .unwrap();

        assert_eq!(service.list_projects().unwrap().len(), 1);
    }

    #[test]
    fn test_remove_project() {
        let temp_dir = TempDir::new().unwrap();
        let service = ConfigService::new();

        let project = service
            .activate_project(temp_dir.path().to_str().unwrap())
            .unwrap();

        // Can't remove active project
        assert!(service.remove_project(&project.name).is_err());

        // Deactivate first
        service.deactivate_project();
        assert!(service.remove_project(&project.name).unwrap());
        assert!(service.list_projects().unwrap().is_empty());
    }
}
