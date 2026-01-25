//! Language server manager
//!
//! Manages multiple language server instances, handling their lifecycle,
//! caching, and providing a unified interface for LSP operations.

use crate::{cache::LspCache, client::LspClient, languages::get_config};
use dashmap::DashMap;
use lsp_types::Uri;
use serena_config::Language;
use std::{path::PathBuf, str::FromStr, sync::Arc};
use tracing::{debug, info, warn};

/// Manager for language server instances
///
/// The manager handles:
/// - Starting and stopping language servers
/// - Caching language server instances
/// - Providing access to language servers by language
/// - Managing the workspace root path
pub struct LanguageServerManager {
    /// Active language server instances, keyed by language
    servers: DashMap<Language, Arc<LspClient>>,

    /// Root path of the workspace
    root_path: PathBuf,

    /// Cache for LSP responses
    cache: Arc<LspCache>,
}

impl LanguageServerManager {
    /// Create a new language server manager
    ///
    /// # Arguments
    /// * `root_path` - The root directory of the workspace
    pub fn new(root_path: PathBuf) -> Self {
        Self {
            servers: DashMap::new(),
            root_path,
            cache: Arc::new(LspCache::new()),
        }
    }

    /// Start a language server for the specified language
    ///
    /// If a server for this language is already running, this is a no-op.
    ///
    /// # Arguments
    /// * `language` - The programming language to start a server for
    ///
    /// # Returns
    /// `Ok(())` if the server was started successfully or was already running
    pub async fn start_server(&self, language: Language) -> anyhow::Result<()> {
        // Check if server is already running
        if self.servers.contains_key(&language) {
            debug!("Language server for {:?} is already running", language);
            return Ok(());
        }

        info!("Starting language server for {:?}", language);

        // Get language server configuration
        let config = get_config(language)?;

        // Spawn the language server
        let client = LspClient::new(config.command, config.args).await?;

        // Initialize the server with workspace root
        // Uri in lsp-types 0.97+ is a type alias for String, construct file:// URI manually
        let path_str = self
            .root_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid root path: {:?}", self.root_path))?;

        // Convert Windows paths to file:// URIs
        let uri_string = if cfg!(windows) {
            format!("file:///{}", path_str.replace('\\', "/"))
        } else {
            format!("file://{}", path_str)
        };
        let root_uri =
            Uri::from_str(&uri_string).map_err(|e| anyhow::anyhow!("Invalid URI: {:?}", e))?;

        client.initialize(root_uri).await?;

        // Store the client
        self.servers.insert(language, Arc::new(client));

        info!("Language server for {:?} started successfully", language);
        Ok(())
    }

    /// Stop a language server for the specified language
    ///
    /// # Arguments
    /// * `language` - The programming language to stop the server for
    ///
    /// # Returns
    /// `Ok(())` if the server was stopped successfully or wasn't running
    pub async fn stop_server(&self, language: Language) -> anyhow::Result<()> {
        if let Some((_, client)) = self.servers.remove(&language) {
            info!("Stopping language server for {:?}", language);

            // Try to get mutable access to shutdown gracefully
            // If we can't (multiple references), it will be killed on drop
            if let Some(mut client) = Arc::into_inner(client) {
                if let Err(e) = client.shutdown().await {
                    warn!(
                        "Error during graceful shutdown of {:?} server: {}",
                        language, e
                    );
                }
            } else {
                debug!("Server has multiple references, will be killed on drop");
            }

            info!("Language server for {:?} stopped", language);
        } else {
            debug!("No language server running for {:?}", language);
        }
        Ok(())
    }

    /// Get a reference to the language server client for the specified language
    ///
    /// # Arguments
    /// * `language` - The programming language
    ///
    /// # Returns
    /// An `Arc` to the LSP client if the server is running, or `None` if not
    pub fn get_server(&self, language: Language) -> Option<Arc<LspClient>> {
        self.servers.get(&language).map(|entry| Arc::clone(&entry))
    }

    /// Get or start a language server for the specified language
    ///
    /// This is a convenience method that ensures a server is running and returns it.
    ///
    /// # Arguments
    /// * `language` - The programming language
    ///
    /// # Returns
    /// An `Arc` to the LSP client
    pub async fn get_or_start_server(&self, language: Language) -> anyhow::Result<Arc<LspClient>> {
        if let Some(server) = self.get_server(language) {
            return Ok(server);
        }

        self.start_server(language).await?;
        self.get_server(language)
            .ok_or_else(|| anyhow::anyhow!("Failed to start server for {:?}", language))
    }

    /// Stop all running language servers
    ///
    /// This is typically called during shutdown to clean up resources.
    pub async fn stop_all_servers(&self) {
        info!("Stopping all language servers");

        let languages: Vec<Language> = self.servers.iter().map(|entry| *entry.key()).collect();

        for language in languages {
            if let Err(e) = self.stop_server(language).await {
                warn!("Error stopping server for {:?}: {}", language, e);
            }
        }

        info!("All language servers stopped");
    }

    /// Restart a language server for the specified language
    ///
    /// This stops the server if running, clears its cache entries, and restarts it.
    ///
    /// # Arguments
    /// * `language` - The programming language to restart the server for
    ///
    /// # Returns
    /// `Ok(())` if the server was restarted successfully
    pub async fn restart_server(&self, language: Language) -> anyhow::Result<()> {
        info!("Restarting language server for {:?}", language);

        // Stop the server if running
        self.stop_server(language).await?;

        // Clear cache for this language's files
        self.cache.clear();

        // Start the server again
        self.start_server(language).await?;

        info!("Language server for {:?} restarted successfully", language);
        Ok(())
    }

    /// Clear the LSP response cache
    ///
    /// This removes all cached LSP responses, forcing fresh requests to the servers.
    pub fn clear_cache(&self) {
        info!("Clearing LSP cache");
        self.cache.clear();
    }

    /// Clear expired cache entries
    ///
    /// This removes only the cache entries that have exceeded their TTL.
    pub fn prune_cache(&self) {
        self.cache.prune_expired();
    }

    /// Get the number of running language servers
    pub fn server_count(&self) -> usize {
        self.servers.len()
    }

    /// Check if a server is running for the specified language
    pub fn is_server_running(&self, language: Language) -> bool {
        self.servers.contains_key(&language)
    }

    /// Get the workspace root path
    pub fn root_path(&self) -> &PathBuf {
        &self.root_path
    }

    /// Get access to the LSP response cache
    pub fn cache(&self) -> &Arc<LspCache> {
        &self.cache
    }

    /// List all currently running language servers
    pub fn list_running_servers(&self) -> Vec<Language> {
        self.servers.iter().map(|entry| *entry.key()).collect()
    }
}

impl Drop for LanguageServerManager {
    fn drop(&mut self) {
        // Note: This is a synchronous drop, so we can't await.
        // Language servers will be killed when their clients are dropped.
        debug!(
            "LanguageServerManager dropped, {} servers will be terminated",
            self.servers.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::tempdir;

    #[test]
    fn test_manager_creation() {
        let temp_dir = env::temp_dir();
        let manager = LanguageServerManager::new(temp_dir.clone());
        assert_eq!(manager.root_path(), &temp_dir);
        assert_eq!(manager.server_count(), 0);
    }

    #[test]
    fn test_server_running_check() {
        let temp_dir = env::temp_dir();
        let manager = LanguageServerManager::new(temp_dir);
        assert!(!manager.is_server_running(Language::Rust));
        assert!(!manager.is_server_running(Language::Python));
        assert!(!manager.is_server_running(Language::TypeScript));
    }

    #[test]
    fn test_list_running_servers() {
        let temp_dir = env::temp_dir();
        let manager = LanguageServerManager::new(temp_dir);
        let servers = manager.list_running_servers();
        assert!(servers.is_empty());
    }

    #[test]
    fn test_manager_with_custom_path() {
        let temp = tempdir().unwrap();
        let manager = LanguageServerManager::new(temp.path().to_path_buf());
        assert_eq!(manager.root_path(), temp.path());
    }

    #[test]
    fn test_cache_operations() {
        let temp_dir = env::temp_dir();
        let manager = LanguageServerManager::new(temp_dir);

        // Cache should be accessible
        let cache = manager.cache();
        assert!(Arc::strong_count(cache) >= 1);

        // Clear cache should not panic
        manager.clear_cache();

        // Prune should not panic
        manager.prune_cache();
    }

    #[test]
    fn test_get_server_returns_none_when_not_started() {
        let temp_dir = env::temp_dir();
        let manager = LanguageServerManager::new(temp_dir);

        assert!(manager.get_server(Language::Rust).is_none());
        assert!(manager.get_server(Language::Python).is_none());
        assert!(manager.get_server(Language::TypeScript).is_none());
    }

    #[tokio::test]
    async fn test_stop_server_when_not_running() {
        let temp_dir = env::temp_dir();
        let manager = LanguageServerManager::new(temp_dir);

        // Should not error when stopping a server that's not running
        let result = manager.stop_server(Language::Rust).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stop_all_servers_when_empty() {
        let temp_dir = env::temp_dir();
        let manager = LanguageServerManager::new(temp_dir);

        // Should not error when stopping all servers and none are running
        manager.stop_all_servers().await;
        assert_eq!(manager.server_count(), 0);
    }

    #[test]
    fn test_manager_drop_safety() {
        let temp_dir = env::temp_dir();
        // Manager should drop without panic
        {
            let _manager = LanguageServerManager::new(temp_dir);
        }
        // If we get here, drop was successful
    }

    // Integration tests that require actual language servers are skipped by default
    // To run them, use: cargo test --features lsp-integration-tests
    #[tokio::test]
    #[ignore = "Requires rust-analyzer installed"]
    async fn test_start_rust_analyzer() {
        let temp_dir = env::temp_dir();
        let manager = LanguageServerManager::new(temp_dir);

        let result = manager.start_server(Language::Rust).await;
        assert!(result.is_ok(), "Failed to start rust-analyzer: {:?}", result.err());

        assert!(manager.is_server_running(Language::Rust));
        assert_eq!(manager.server_count(), 1);

        // Cleanup
        let result = manager.stop_server(Language::Rust).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "Requires pyright installed"]
    async fn test_start_pyright() {
        let temp_dir = env::temp_dir();
        let manager = LanguageServerManager::new(temp_dir);

        let result = manager.start_server(Language::Python).await;
        assert!(result.is_ok(), "Failed to start pyright: {:?}", result.err());

        assert!(manager.is_server_running(Language::Python));

        // Cleanup
        manager.stop_all_servers().await;
    }

    #[tokio::test]
    #[ignore = "Requires language servers installed"]
    async fn test_restart_server() {
        let temp_dir = env::temp_dir();
        let manager = LanguageServerManager::new(temp_dir);

        // Start
        manager.start_server(Language::Rust).await.unwrap();
        assert!(manager.is_server_running(Language::Rust));

        // Restart
        let result = manager.restart_server(Language::Rust).await;
        assert!(result.is_ok());
        assert!(manager.is_server_running(Language::Rust));

        // Cleanup
        manager.stop_all_servers().await;
    }

    #[tokio::test]
    #[ignore = "Requires language servers installed"]
    async fn test_get_or_start_server() {
        let temp_dir = env::temp_dir();
        let manager = LanguageServerManager::new(temp_dir);

        // First call should start the server
        let result = manager.get_or_start_server(Language::Rust).await;
        assert!(result.is_ok());

        // Second call should return the same server
        let result = manager.get_or_start_server(Language::Rust).await;
        assert!(result.is_ok());

        // Should still be only one server
        assert_eq!(manager.server_count(), 1);

        // Cleanup
        manager.stop_all_servers().await;
    }
}
