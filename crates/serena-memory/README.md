# serena-memory

Memory and knowledge persistence for Serena, providing efficient storage and retrieval of project-specific information.

## Features

- **Dual-storage architecture**: Combines file-based markdown storage with SQLite indexing
- **Async API**: Built on Tokio for non-blocking operations
- **Full-text search**: Fast search across memory content and names
- **Metadata tracking**: Timestamps, tags, and file sizes
- **Project-scoped**: Memories are isolated per project in `.serena/memories/`
- **Git-friendly**: Markdown files can be version controlled
- **MemoryStorage trait**: Implements the `serena-core::MemoryStorage` trait

## Architecture

The memory system uses a dual-storage approach for optimal performance and usability:

### File Storage (`.serena/memories/`)
- Memories stored as markdown files (e.g., `api-notes.md`)
- Human-readable and editable
- Version control friendly
- Easy to backup and migrate

### Database Storage (`.serena/memories.db`)
- SQLite database for metadata and indexing
- Fast queries and searches
- Tracks creation/update timestamps
- Manages tags and relationships

## Usage

### Basic Operations

```rust
use serena_memory::MemoryManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create a memory manager for a project
    let manager = MemoryManager::new("/path/to/project")?;

    // Save a memory
    manager.save_memory("api-design", "# API Design Notes\n\n...").await?;

    // Load a memory
    let content = manager.load_memory("api-design").await?;
    println!("{}", content);

    // List all memories
    let memories = manager.list_memories().await?;
    for name in memories {
        println!("- {}", name);
    }

    // Search memories
    let results = manager.search("API").await?;
    println!("Found {} matching memories", results.len());

    // Delete a memory
    manager.delete_memory("old-notes").await?;

    Ok(())
}
```

### Advanced Features

```rust
use serena_memory::{MemoryManager, ReplaceMode};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let manager = MemoryManager::new("/path/to/project")?;

    // Check if memory exists
    if manager.exists("config").await {
        // Update with literal replacement
        manager.replace_content(
            "config",
            "old value",
            "new value",
            ReplaceMode::Literal
        ).await?;
    }

    // Update with regex replacement
    manager.replace_content(
        "config",
        r"version: \d+\.\d+",
        "version: 2.0",
        ReplaceMode::Regex
    ).await?;

    // List with metadata
    let metadata = manager.list_memories_with_metadata().await?;
    for mem in metadata {
        println!("{}: {} bytes, updated at {}",
                 mem.name, mem.size_bytes, mem.updated_at);
    }

    // Sync filesystem with database
    let synced = manager.sync().await?;
    println!("Synchronized {} memories", synced);

    Ok(())
}
```

### Using the MemoryStorage Trait

```rust
use serena_core::MemoryStorage;
use serena_memory::MemoryManager;

async fn example(storage: &dyn MemoryStorage) -> Result<(), serena_core::SerenaError> {
    // Write
    storage.write("notes", "# Notes\n\nSome content").await?;

    // Read
    let content = storage.read("notes").await?;

    // List
    let all = storage.list().await?;

    // Search
    let results = storage.search("content").await?;

    // Delete
    storage.delete("notes").await?;

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let manager = MemoryManager::new("/path/to/project")?;
    example(&manager).await?;
    Ok(())
}
```

## Memory Model

### Memory Struct

```rust
pub struct Memory {
    /// Unique name/identifier (without .md extension)
    pub name: String,

    /// Markdown content
    pub content: String,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,

    /// Optional tags for categorization
    pub tags: Vec<String>,

    /// File size in bytes
    pub size_bytes: usize,
}
```

### MemoryMetadata Struct

Lightweight metadata without full content:

```rust
pub struct MemoryMetadata {
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub size_bytes: usize,
}
```

## Directory Structure

When you create a `MemoryManager` for a project, it creates:

```
project-root/
└── .serena/
    ├── memories/           # Markdown files
    │   ├── api-notes.md
    │   ├── config.md
    │   └── todos.md
    └── memories.db         # SQLite database
```

## Error Handling

The crate provides comprehensive error types:

```rust
use serena_memory::MemoryError;

match manager.load_memory("nonexistent").await {
    Ok(content) => println!("{}", content),
    Err(e) => match e.downcast_ref::<MemoryError>() {
        Some(MemoryError::NotFound(name)) => {
            eprintln!("Memory '{}' not found", name);
        }
        Some(MemoryError::PermissionDenied(msg)) => {
            eprintln!("Permission denied: {}", msg);
        }
        _ => eprintln!("Error: {}", e),
    }
}
```

## Testing

The crate includes comprehensive unit tests:

```bash
cargo test -p serena-memory
```

Integration tests with temporary directories:

```bash
cargo test -p serena-memory -- --nocapture
```

## Performance

- **Fast queries**: SQLite indexing provides O(log n) lookups
- **Lazy loading**: Content loaded only when needed
- **Async I/O**: Non-blocking file operations
- **Connection pooling**: Reuses database connections
- **Minimal overhead**: Lightweight metadata tracking

## Compatibility

- **Rust**: 1.75+
- **Tokio**: 1.41+
- **SQLite**: Bundled (no system dependency)
- **Platforms**: Windows, Linux, macOS

## Dependencies

Core dependencies:
- `tokio`: Async runtime
- `rusqlite`: SQLite database (bundled)
- `chrono`: DateTime handling with serde support
- `serde`: Serialization
- `anyhow`: Error handling
- `tracing`: Logging

## License

MIT License - see LICENSE file for details

## Contributing

Contributions welcome! Please ensure:
- All tests pass: `cargo test -p serena-memory`
- Code is formatted: `cargo fmt`
- No clippy warnings: `cargo clippy`
- Documentation is updated

## Related Crates

- `serena-core`: Core traits and types
- `serena-tools`: Tool implementations using memory storage
- `serena-mcp`: MCP server exposing memory operations
