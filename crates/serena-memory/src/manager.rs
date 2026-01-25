use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use async_trait::async_trait;
use regex::Regex;
use serena_core::{MemoryStorage, SerenaError};
use tracing::{debug, info};

use crate::memory::{Memory, MemoryMetadata};
use crate::store::MemoryStore;

/// High-level memory manager that combines file-based and database storage
///
/// This manager stores memories as markdown files in a directory AND tracks
/// metadata in a SQLite database for efficient querying and management.
pub struct MemoryManager {
    /// Path to the memories directory
    memory_dir: PathBuf,

    /// SQLite store for metadata and indexing
    store: MemoryStore,
}

impl MemoryManager {
    /// Create a new memory manager for the given project
    ///
    /// # Arguments
    /// * `project_root` - Root directory of the project
    ///
    /// # Returns
    /// A new `MemoryManager` instance
    pub fn new(project_root: impl AsRef<Path>) -> Result<Self> {
        let project_root = project_root.as_ref();
        let memory_dir = project_root.join(".serena").join("memories");
        let db_path = project_root.join(".serena").join("memories.db");

        // Create directories
        std::fs::create_dir_all(&memory_dir)
            .with_context(|| format!("Failed to create memory directory: {:?}", memory_dir))?;

        let store = MemoryStore::new(&db_path).context("Failed to create memory store")?;

        info!("Initialized memory manager at {:?}", memory_dir);

        Ok(Self { memory_dir, store })
    }

    /// Create a memory manager with custom paths
    pub fn with_paths(memory_dir: impl AsRef<Path>, db_path: impl AsRef<Path>) -> Result<Self> {
        let memory_dir = memory_dir.as_ref().to_path_buf();

        std::fs::create_dir_all(&memory_dir)
            .with_context(|| format!("Failed to create memory directory: {:?}", memory_dir))?;

        let store = MemoryStore::new(db_path).context("Failed to create memory store")?;

        Ok(Self { memory_dir, store })
    }

    /// Get the file path for a memory
    ///
    /// Strips .md extension from name if present to avoid double extensions
    pub fn get_memory_file_path(&self, name: &str) -> PathBuf {
        let name = name.strip_suffix(".md").unwrap_or(name);
        self.memory_dir.join(format!("{}.md", name))
    }

    /// Save a memory to both file and database
    pub async fn save_memory(&self, name: &str, content: &str) -> Result<String> {
        let file_path = self.get_memory_file_path(name);

        // Write to file
        std::fs::write(&file_path, content)
            .with_context(|| format!("Failed to write memory file: {:?}", file_path))?;

        // Save to database
        let memory = Memory::new(name, content);
        self.store
            .save(&memory)
            .await
            .context("Failed to save memory to database")?;

        info!("Saved memory: {}", name);
        Ok(format!("Memory {} written.", name))
    }

    /// Load a memory from file
    pub async fn load_memory(&self, name: &str) -> Result<String> {
        let file_path = self.get_memory_file_path(name);

        if !file_path.exists() {
            return Ok(format!(
                "Memory file {} not found, consider creating it with the `write_memory` tool if you need it.",
                name
            ));
        }

        let content = std::fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read memory file: {:?}", file_path))?;

        debug!("Loaded memory: {}", name);
        Ok(content)
    }

    /// List all available memories
    pub async fn list_memories(&self) -> Result<Vec<String>> {
        let entries = std::fs::read_dir(&self.memory_dir)
            .with_context(|| format!("Failed to read memory directory: {:?}", self.memory_dir))?;

        let mut names = Vec::new();
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(name) = path.file_stem() {
                    if let Some(name_str) = name.to_str() {
                        names.push(name_str.to_string());
                    }
                }
            }
        }

        names.sort();
        debug!("Listed {} memories", names.len());
        Ok(names)
    }

    /// List memories with full metadata
    pub async fn list_memories_with_metadata(&self) -> Result<Vec<MemoryMetadata>> {
        self.store.list().await
    }

    /// Delete a memory
    pub async fn delete_memory(&self, name: &str) -> Result<String> {
        let file_path = self.get_memory_file_path(name);

        // Delete file
        if file_path.exists() {
            std::fs::remove_file(&file_path)
                .with_context(|| format!("Failed to delete memory file: {:?}", file_path))?;
        }

        // Delete from database
        self.store
            .delete(name)
            .await
            .context("Failed to delete from database")?;

        info!("Deleted memory: {}", name);
        Ok(format!("Memory {} deleted.", name))
    }

    /// Check if a memory exists
    pub async fn exists(&self, name: &str) -> bool {
        let file_path = self.get_memory_file_path(name);
        file_path.exists()
    }

    /// Search memories by query
    pub async fn search(&self, query: &str) -> Result<Vec<MemoryMetadata>> {
        self.store.search(query).await
    }

    /// Update a memory by applying a regex replacement
    pub async fn replace_content(
        &self,
        name: &str,
        needle: &str,
        replacement: &str,
        mode: ReplaceMode,
    ) -> Result<String> {
        let content = self.load_memory(name).await?;

        if content.starts_with("Memory file") && content.contains("not found") {
            return Ok(content);
        }

        let new_content = match mode {
            ReplaceMode::Literal => content.replace(needle, replacement),
            ReplaceMode::Regex => {
                let re = Regex::new(needle)
                    .with_context(|| format!("Invalid regex pattern: {}", needle))?;
                re.replace_all(&content, replacement).to_string()
            }
        };

        self.save_memory(name, &new_content).await
    }

    /// Get the memory directory path
    pub fn memory_dir(&self) -> &Path {
        &self.memory_dir
    }

    /// Synchronize file system with database
    ///
    /// Scans the memory directory and updates the database to match
    pub async fn sync(&self) -> Result<usize> {
        let mut synced = 0;

        let entries = std::fs::read_dir(&self.memory_dir)
            .with_context(|| format!("Failed to read memory directory: {:?}", self.memory_dir))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    let content = std::fs::read_to_string(&path)?;
                    let memory = Memory::new(name, content);
                    self.store.save(&memory).await?;
                    synced += 1;
                }
            }
        }

        info!("Synchronized {} memories", synced);
        Ok(synced)
    }
}

/// Mode for content replacement
#[derive(Debug, Clone, Copy)]
pub enum ReplaceMode {
    /// Literal string replacement
    Literal,
    /// Regex pattern replacement
    Regex,
}

#[async_trait]
impl MemoryStorage for MemoryManager {
    async fn write(&self, name: &str, content: &str) -> Result<(), SerenaError> {
        self.save_memory(name, content)
            .await
            .map(|_| ())
            .map_err(|e| SerenaError::Internal(e.to_string()))
    }

    async fn read(&self, name: &str) -> Result<String, SerenaError> {
        self.load_memory(name)
            .await
            .map_err(|e| SerenaError::Internal(e.to_string()))
    }

    async fn list(&self) -> Result<Vec<String>, SerenaError> {
        self.list_memories()
            .await
            .map_err(|e| SerenaError::Internal(e.to_string()))
    }

    async fn delete(&self, name: &str) -> Result<(), SerenaError> {
        self.delete_memory(name)
            .await
            .map(|_| ())
            .map_err(|e| SerenaError::Internal(e.to_string()))
    }

    async fn search(&self, query: &str) -> Result<Vec<(String, String)>, SerenaError> {
        let results = self
            .search(query)
            .await
            .map_err(|e| SerenaError::Internal(e.to_string()))?;

        let mut pairs = Vec::new();
        for metadata in results {
            let content = self.read(&metadata.name).await?;
            pairs.push((metadata.name, content));
        }

        Ok(pairs)
    }

    async fn exists(&self, name: &str) -> Result<bool, SerenaError> {
        Ok(self.exists(name).await)
    }

    fn storage_path(&self) -> &str {
        self.memory_dir.to_str().unwrap_or("")
    }

    async fn clear_all(&self) -> Result<(), SerenaError> {
        // Delete all files
        let entries = std::fs::read_dir(&self.memory_dir)
            .map_err(|e| SerenaError::Internal(e.to_string()))?;

        for entry in entries {
            let entry = entry.map_err(|e| SerenaError::Internal(e.to_string()))?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                std::fs::remove_file(path).map_err(|e| SerenaError::Internal(e.to_string()))?;
            }
        }

        // Clear database
        self.store
            .clear_all()
            .await
            .map_err(|e| SerenaError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn update<F>(&self, name: &str, updater: F) -> Result<(), SerenaError>
    where
        F: FnOnce(&str) -> Result<String, SerenaError> + Send,
    {
        let content = self.read(name).await?;
        let updated = updater(&content)?;
        self.write(name, &updated).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_manager_new() {
        let dir = tempdir().unwrap();
        let manager = MemoryManager::new(dir.path()).unwrap();

        assert!(manager.memory_dir().exists());
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let manager = MemoryManager::new(dir.path()).unwrap();

        let result = manager.save_memory("test", "# Test Content").await.unwrap();
        assert!(result.contains("written"));

        let content = manager.load_memory("test").await.unwrap();
        assert_eq!(content, "# Test Content");
    }

    #[tokio::test]
    async fn test_list() {
        let dir = tempdir().unwrap();
        let manager = MemoryManager::new(dir.path()).unwrap();

        manager.save_memory("test1", "content1").await.unwrap();
        manager.save_memory("test2", "content2").await.unwrap();

        let list = manager.list_memories().await.unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.contains(&"test1".to_string()));
        assert!(list.contains(&"test2".to_string()));
    }

    #[tokio::test]
    async fn test_delete() {
        let dir = tempdir().unwrap();
        let manager = MemoryManager::new(dir.path()).unwrap();

        manager.save_memory("test", "content").await.unwrap();
        assert!(manager.exists("test").await);

        manager.delete_memory("test").await.unwrap();
        assert!(!manager.exists("test").await);
    }

    #[tokio::test]
    async fn test_replace_literal() {
        let dir = tempdir().unwrap();
        let manager = MemoryManager::new(dir.path()).unwrap();

        manager.save_memory("test", "Hello World").await.unwrap();
        manager
            .replace_content("test", "World", "Rust", ReplaceMode::Literal)
            .await
            .unwrap();

        let content = manager.load_memory("test").await.unwrap();
        assert_eq!(content, "Hello Rust");
    }

    #[tokio::test]
    async fn test_replace_regex() {
        let dir = tempdir().unwrap();
        let manager = MemoryManager::new(dir.path()).unwrap();

        manager.save_memory("test", "Test 123").await.unwrap();
        manager
            .replace_content("test", r"\d+", "456", ReplaceMode::Regex)
            .await
            .unwrap();

        let content = manager.load_memory("test").await.unwrap();
        assert_eq!(content, "Test 456");
    }

    #[tokio::test]
    async fn test_sync() {
        let dir = tempdir().unwrap();
        let manager = MemoryManager::new(dir.path()).unwrap();

        // Create files directly
        let file1 = manager.get_memory_file_path("manual1");
        std::fs::write(&file1, "Manual content 1").unwrap();

        let file2 = manager.get_memory_file_path("manual2");
        std::fs::write(&file2, "Manual content 2").unwrap();

        // Sync to database
        let synced = manager.sync().await.unwrap();
        assert_eq!(synced, 2);

        // Verify in database
        let list = manager.list_memories_with_metadata().await.unwrap();
        assert_eq!(list.len(), 2);
    }
}
