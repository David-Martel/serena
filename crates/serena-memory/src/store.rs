use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use tokio::sync::Mutex;
use tracing::{debug, info};

use crate::memory::{Memory, MemoryMetadata};

/// SQLite-based storage for memories
pub struct MemoryStore {
    /// Path to the SQLite database file
    db_path: PathBuf,

    /// Database connection wrapped in Arc<Mutex> for async access
    conn: Arc<Mutex<Connection>>,
}

impl MemoryStore {
    /// Create a new memory store at the given path
    ///
    /// # Arguments
    /// * `db_path` - Path to the SQLite database file
    ///
    /// # Returns
    /// A new `MemoryStore` instance
    pub fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }

        // Open database connection with bundled SQLite
        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open database at {:?}", db_path))?;

        // Initialize schema synchronously
        Self::init_schema_sync(&conn)?;

        let store = Self {
            db_path: db_path.clone(),
            conn: Arc::new(Mutex::new(conn)),
        };

        info!("Initialized memory store at {:?}", store.db_path);
        Ok(store)
    }

    /// Initialize the database schema
    fn init_schema_sync(conn: &Connection) -> Result<()> {
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS memories (
                name TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                tags TEXT,
                size_bytes INTEGER NOT NULL
            )
            "#,
            [],
        )
        .context("Failed to create memories table")?;

        // Create index for faster searches
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_updated_at ON memories(updated_at)",
            [],
        )
        .context("Failed to create index")?;

        debug!("Database schema initialized");
        Ok(())
    }

    /// Save or update a memory
    pub async fn save(&self, memory: &Memory) -> Result<()> {
        let conn = self.conn.lock().await;

        let tags_json = serde_json::to_string(&memory.tags)?;

        conn.execute(
            r#"
            INSERT INTO memories (name, content, created_at, updated_at, tags, size_bytes)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(name) DO UPDATE SET
                content = excluded.content,
                updated_at = excluded.updated_at,
                tags = excluded.tags,
                size_bytes = excluded.size_bytes
            "#,
            params![
                memory.name,
                memory.content,
                memory.created_at.to_rfc3339(),
                memory.updated_at.to_rfc3339(),
                tags_json,
                memory.size_bytes as i64,
            ],
        )
        .with_context(|| format!("Failed to save memory: {}", memory.name))?;

        debug!("Saved memory: {}", memory.name);
        Ok(())
    }

    /// Load a memory by name
    pub async fn load(&self, name: &str) -> Result<Option<Memory>> {
        let conn = self.conn.lock().await;

        let result = conn
            .query_row(
                "SELECT name, content, created_at, updated_at, tags, size_bytes FROM memories WHERE name = ?1",
                params![name],
                |row| {
                    let tags_json: String = row.get(4)?;
                    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

                    Ok(Memory {
                        name: row.get(0)?,
                        content: row.get(1)?,
                        created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        tags,
                        size_bytes: row.get::<_, i64>(5)? as usize,
                    })
                },
            )
            .optional()
            .with_context(|| format!("Failed to load memory: {}", name))?;

        if result.is_some() {
            debug!("Loaded memory: {}", name);
        }

        Ok(result)
    }

    /// List all memory metadata
    pub async fn list(&self) -> Result<Vec<MemoryMetadata>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare("SELECT name, created_at, updated_at, tags, size_bytes FROM memories ORDER BY updated_at DESC")
            .context("Failed to prepare list statement")?;

        let memories = stmt
            .query_map([], |row| {
                let tags_json: String = row.get(3)?;
                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

                Ok(MemoryMetadata {
                    name: row.get(0)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    tags,
                    size_bytes: row.get::<_, i64>(4)? as usize,
                })
            })
            .context("Failed to query memories")?
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to collect memories")?;

        debug!("Listed {} memories", memories.len());
        Ok(memories)
    }

    /// Delete a memory by name
    pub async fn delete(&self, name: &str) -> Result<bool> {
        let conn = self.conn.lock().await;

        let rows_affected = conn
            .execute("DELETE FROM memories WHERE name = ?1", params![name])
            .with_context(|| format!("Failed to delete memory: {}", name))?;

        let deleted = rows_affected > 0;
        if deleted {
            debug!("Deleted memory: {}", name);
        }

        Ok(deleted)
    }

    /// Check if a memory exists
    pub async fn exists(&self, name: &str) -> Result<bool> {
        let conn = self.conn.lock().await;

        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM memories WHERE name = ?1)",
                params![name],
                |row| row.get(0),
            )
            .context("Failed to check memory existence")?;

        Ok(exists)
    }

    /// Search memories by content or name
    pub async fn search(&self, query: &str) -> Result<Vec<MemoryMetadata>> {
        let conn = self.conn.lock().await;

        let search_pattern = format!("%{}%", query);

        let mut stmt = conn
            .prepare(
                r#"
                SELECT name, created_at, updated_at, tags, size_bytes
                FROM memories
                WHERE name LIKE ?1 OR content LIKE ?1
                ORDER BY updated_at DESC
                "#,
            )
            .context("Failed to prepare search statement")?;

        let memories = stmt
            .query_map(params![search_pattern], |row| {
                let tags_json: String = row.get(3)?;
                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

                Ok(MemoryMetadata {
                    name: row.get(0)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    tags,
                    size_bytes: row.get::<_, i64>(4)? as usize,
                })
            })
            .context("Failed to search memories")?
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to collect search results")?;

        debug!("Search for '{}' found {} results", query, memories.len());
        Ok(memories)
    }

    /// Clear all memories
    pub async fn clear_all(&self) -> Result<()> {
        let conn = self.conn.lock().await;

        conn.execute("DELETE FROM memories", [])
            .context("Failed to clear all memories")?;

        info!("Cleared all memories");
        Ok(())
    }

    /// Get the database path
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_create_store() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let store = MemoryStore::new(&db_path).unwrap();
        assert_eq!(store.db_path(), db_path);
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = MemoryStore::new(&db_path).unwrap();

        let memory = Memory::new("test", "# Test Content");
        store.save(&memory).await.unwrap();

        let loaded = store.load("test").await.unwrap().unwrap();
        assert_eq!(loaded.name, "test");
        assert_eq!(loaded.content, "# Test Content");
    }

    #[tokio::test]
    async fn test_list() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = MemoryStore::new(&db_path).unwrap();

        let mem1 = Memory::new("test1", "content1");
        let mem2 = Memory::new("test2", "content2");

        store.save(&mem1).await.unwrap();
        store.save(&mem2).await.unwrap();

        let list = store.list().await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_delete() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = MemoryStore::new(&db_path).unwrap();

        let memory = Memory::new("test", "content");
        store.save(&memory).await.unwrap();

        assert!(store.exists("test").await.unwrap());

        let deleted = store.delete("test").await.unwrap();
        assert!(deleted);

        assert!(!store.exists("test").await.unwrap());
    }

    #[tokio::test]
    async fn test_search() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = MemoryStore::new(&db_path).unwrap();

        let mem1 = Memory::new("rust-notes", "Learning Rust programming");
        let mem2 = Memory::new("python-notes", "Python scripting");

        store.save(&mem1).await.unwrap();
        store.save(&mem2).await.unwrap();

        let results = store.search("rust").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "rust-notes");
    }
}
