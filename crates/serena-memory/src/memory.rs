use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a single memory/knowledge entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    /// Unique name/identifier for the memory (without .md extension)
    pub name: String,

    /// Markdown content of the memory
    pub content: String,

    /// When the memory was created
    pub created_at: DateTime<Utc>,

    /// When the memory was last modified
    pub updated_at: DateTime<Utc>,

    /// Optional metadata tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,

    /// File size in bytes
    pub size_bytes: usize,
}

impl Memory {
    /// Create a new memory with the given name and content
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        let content = content.into();
        let size_bytes = content.len();
        let now = Utc::now();

        Self {
            name: name.into(),
            content,
            created_at: now,
            updated_at: now,
            tags: Vec::new(),
            size_bytes,
        }
    }

    /// Update the memory content
    pub fn update_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
        self.size_bytes = self.content.len();
        self.updated_at = Utc::now();
    }

    /// Add a tag to the memory
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        let tag = tag.into();
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.updated_at = Utc::now();
        }
    }

    /// Remove a tag from the memory
    pub fn remove_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Check if memory has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Get the memory name with .md extension
    pub fn filename(&self) -> String {
        format!("{}.md", self.name)
    }
}

/// Metadata summary for a memory (without full content)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub size_bytes: usize,
}

impl From<&Memory> for MemoryMetadata {
    fn from(memory: &Memory) -> Self {
        Self {
            name: memory.name.clone(),
            created_at: memory.created_at,
            updated_at: memory.updated_at,
            tags: memory.tags.clone(),
            size_bytes: memory.size_bytes,
        }
    }
}

impl From<Memory> for MemoryMetadata {
    fn from(memory: Memory) -> Self {
        Self {
            name: memory.name,
            created_at: memory.created_at,
            updated_at: memory.updated_at,
            tags: memory.tags,
            size_bytes: memory.size_bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_memory() {
        let memory = Memory::new("test", "# Test Content");
        assert_eq!(memory.name, "test");
        assert_eq!(memory.content, "# Test Content");
        assert_eq!(memory.size_bytes, 14);
        assert!(memory.tags.is_empty());
    }

    #[test]
    fn test_update_content() {
        let mut memory = Memory::new("test", "original");
        let original_updated = memory.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        memory.update_content("updated");

        assert_eq!(memory.content, "updated");
        assert_eq!(memory.size_bytes, 7);
        assert!(memory.updated_at > original_updated);
    }

    #[test]
    fn test_tags() {
        let mut memory = Memory::new("test", "content");

        memory.add_tag("rust");
        memory.add_tag("testing");
        assert_eq!(memory.tags.len(), 2);

        memory.add_tag("rust"); // Duplicate
        assert_eq!(memory.tags.len(), 2); // Should not add duplicate

        assert!(memory.has_tag("rust"));
        assert!(!memory.has_tag("python"));
    }

    #[test]
    fn test_filename() {
        let memory = Memory::new("my-memory", "content");
        assert_eq!(memory.filename(), "my-memory.md");
    }
}
