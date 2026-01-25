use async_trait::async_trait;

use crate::SerenaError;

/// Trait for memory/knowledge storage implementations
#[async_trait]
pub trait MemoryStorage: Send + Sync {
    /// Write content to a memory with the given name
    async fn write(&self, name: &str, content: &str) -> Result<(), SerenaError>;

    /// Read content from a memory with the given name
    async fn read(&self, name: &str) -> Result<String, SerenaError>;

    /// List all available memory names
    async fn list(&self) -> Result<Vec<String>, SerenaError>;

    /// Delete a memory with the given name
    async fn delete(&self, name: &str) -> Result<(), SerenaError>;

    /// Search memories for content matching a query
    async fn search(&self, query: &str) -> Result<Vec<(String, String)>, SerenaError>;

    /// Check if a memory exists
    async fn exists(&self, name: &str) -> Result<bool, SerenaError>;

    /// Get the storage path/location
    fn storage_path(&self) -> &str;

    /// Clear all memories (use with caution)
    async fn clear_all(&self) -> Result<(), SerenaError>;

    /// Update a memory by applying a transformation function
    async fn update<F>(&self, name: &str, updater: F) -> Result<(), SerenaError>
    where
        F: FnOnce(&str) -> Result<String, SerenaError> + Send;

    /// Append content to an existing memory
    async fn append(&self, name: &str, content: &str) -> Result<(), SerenaError> {
        let existing = self.read(name).await.unwrap_or_default();
        let updated = format!("{}\n{}", existing, content);
        self.write(name, &updated).await
    }
}
