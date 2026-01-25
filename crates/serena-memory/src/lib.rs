//! # serena-memory
//!
//! Memory and knowledge persistence for Serena.
//!
//! This crate provides:
//! - SQLite-based storage for efficient querying
//! - File-based markdown memories for human readability
//! - High-level memory management API
//! - Search and metadata indexing
//!
//! ## Usage
//!
//! ```rust,no_run
//! use serena_memory::MemoryManager;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create a memory manager for a project
//!     let manager = MemoryManager::new("/path/to/project")?;
//!
//!     // Save a memory
//!     manager.save_memory("api-notes", "# API Design Notes\n...").await?;
//!
//!     // Load a memory
//!     let content = manager.load_memory("api-notes").await?;
//!     println!("{}", content);
//!
//!     // List all memories
//!     let memories = manager.list_memories().await?;
//!     for name in memories {
//!         println!("- {}", name);
//!     }
//!
//!     // Search memories
//!     let results = manager.search("API").await?;
//!     println!("Found {} matching memories", results.len());
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! The memory system uses a dual-storage approach:
//!
//! 1. **File Storage**: Memories are stored as markdown files in `.serena/memories/`
//!    for human readability and version control compatibility.
//!
//! 2. **Database Storage**: Metadata is stored in SQLite (`.serena/memories.db`)
//!    for efficient querying, searching, and indexing.
//!
//! This approach provides:
//! - Fast search and query capabilities (via SQLite)
//! - Human-readable storage (markdown files)
//! - Git-friendly versioning (text files)
//! - Metadata tracking (timestamps, tags, sizes)

pub mod error;
pub mod manager;
pub mod memory;
pub mod store;
pub mod tools;

// Re-export main types
pub use manager::{MemoryManager, ReplaceMode};
pub use memory::{Memory, MemoryMetadata};
pub use store::MemoryStore;

// Re-export errors
pub use error::MemoryError;

// Re-export tools
pub use tools::{
    create_memory_tools, DeleteMemoryTool, EditMemoryTool, ListMemoriesTool, ReadMemoryTool,
    SearchMemoriesTool, WriteMemoryTool,
};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::error::MemoryError;
    pub use crate::manager::{MemoryManager, ReplaceMode};
    pub use crate::memory::{Memory, MemoryMetadata};
    pub use crate::store::MemoryStore;
}
