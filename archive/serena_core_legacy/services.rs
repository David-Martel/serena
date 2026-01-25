//! Service layers for tool implementations

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::lsp::LspClient;
use crate::mcp::error::{SerenaError, SerenaResult};
use crate::symbol_graph::SymbolGraph;

/// File system service for file operations
pub struct FileService {
    project_root: PathBuf,
    allowed_dirs: Vec<PathBuf>,
}

impl FileService {
    pub fn new(project_root: &Path) -> SerenaResult<Self> {
        let project_root = project_root
            .canonicalize()
            .unwrap_or_else(|_| project_root.to_path_buf());

        Ok(Self {
            project_root: project_root.clone(),
            allowed_dirs: vec![project_root],
        })
    }

    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// Check if a path is within allowed directories
    pub fn validate_path(&self, path: &Path) -> SerenaResult<PathBuf> {
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.project_root.join(path)
        };

        let canonical = full_path
            .canonicalize()
            .map_err(|_| SerenaError::FileNotFound(path.display().to_string()))?;

        for allowed in &self.allowed_dirs {
            if canonical.starts_with(allowed) {
                return Ok(canonical);
            }
        }

        Err(SerenaError::PathNotAllowed(path.display().to_string()))
    }

    /// Read file contents
    pub async fn read_file(&self, path: &Path) -> SerenaResult<String> {
        let full_path = self.validate_path(path)?;
        tokio::fs::read_to_string(&full_path)
            .await
            .map_err(|e| SerenaError::Io(e))
    }

    /// Write file contents
    pub async fn write_file(&self, path: &Path, content: &str) -> SerenaResult<()> {
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.project_root.join(path)
        };

        // Ensure parent directory exists
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&full_path, content)
            .await
            .map_err(|e| SerenaError::Io(e))
    }

    /// List directory contents
    pub async fn list_dir(
        &self,
        path: &Path,
        recursive: bool,
    ) -> SerenaResult<(Vec<String>, Vec<String>)> {
        let full_path = self.validate_path(path)?;
        let mut dirs = Vec::new();
        let mut files = Vec::new();

        if recursive {
            for entry in walkdir::WalkDir::new(&full_path)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let rel_path = entry
                    .path()
                    .strip_prefix(&self.project_root)
                    .unwrap_or(entry.path())
                    .to_string_lossy()
                    .replace('\\', "/");

                if entry.file_type().is_dir() {
                    dirs.push(rel_path);
                } else {
                    files.push(rel_path);
                }
            }
        } else {
            let mut read_dir = tokio::fs::read_dir(&full_path).await?;
            while let Some(entry) = read_dir.next_entry().await? {
                let rel_path = entry
                    .path()
                    .strip_prefix(&self.project_root)
                    .unwrap_or(&entry.path())
                    .to_string_lossy()
                    .replace('\\', "/");

                if entry.file_type().await?.is_dir() {
                    dirs.push(rel_path);
                } else {
                    files.push(rel_path);
                }
            }
        }

        Ok((dirs, files))
    }
}

/// Symbol service for LSP-based operations
pub struct SymbolService {
    project_root: PathBuf,
    lsp_client: RwLock<Option<Arc<LspClient>>>,
    symbol_graph: Arc<SymbolGraph>,
}

impl SymbolService {
    pub fn new(project_root: &Path) -> SerenaResult<Self> {
        Ok(Self {
            project_root: project_root.to_path_buf(),
            lsp_client: RwLock::new(None),
            symbol_graph: Arc::new(SymbolGraph::new()),
        })
    }

    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn symbol_graph(&self) -> &Arc<SymbolGraph> {
        &self.symbol_graph
    }

    /// Start the language server
    pub async fn start_lsp(&self, command: &str, args: Vec<String>) -> SerenaResult<()> {
        let client = LspClient::new(command.to_string(), args)
            .await
            .map_err(|e| SerenaError::Lsp(e.to_string()))?;

        let root_uri = lsp_types::Url::from_directory_path(&self.project_root)
            .map_err(|_| SerenaError::InvalidPath(self.project_root.display().to_string()))?;

        client
            .initialize(root_uri)
            .await
            .map_err(|e| SerenaError::Lsp(e.to_string()))?;

        *self.lsp_client.write().await = Some(Arc::new(client));
        Ok(())
    }

    /// Get LSP client if available
    pub async fn get_client(&self) -> SerenaResult<Arc<LspClient>> {
        self.lsp_client
            .read()
            .await
            .clone()
            .ok_or(SerenaError::LspNotStarted)
    }
}

/// Memory service for persistent knowledge storage
pub struct MemoryService {
    project_root: PathBuf,
    memories_dir: PathBuf,
}

impl MemoryService {
    pub fn new(project_root: &Path) -> SerenaResult<Self> {
        let memories_dir = project_root.join(".serena").join("memories");
        std::fs::create_dir_all(&memories_dir)?;

        Ok(Self {
            project_root: project_root.to_path_buf(),
            memories_dir,
        })
    }

    pub fn memories_dir(&self) -> &Path {
        &self.memories_dir
    }

    /// Write a memory file
    pub async fn write(&self, name: &str, content: &str) -> SerenaResult<()> {
        let file_path = self.memories_dir.join(format!("{}.md", name));
        tokio::fs::write(&file_path, content)
            .await
            .map_err(|e| SerenaError::Io(e))
    }

    /// Read a memory file
    pub async fn read(&self, name: &str) -> SerenaResult<String> {
        let file_path = self.memories_dir.join(format!("{}.md", name));
        tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| SerenaError::Io(e))
    }

    /// List all memories
    pub async fn list(&self) -> SerenaResult<Vec<String>> {
        let mut memories = Vec::new();
        let mut read_dir = tokio::fs::read_dir(&self.memories_dir).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".md") {
                    memories.push(name.trim_end_matches(".md").to_string());
                }
            }
        }

        Ok(memories)
    }

    /// Delete a memory file
    pub async fn delete(&self, name: &str) -> SerenaResult<()> {
        let file_path = self.memories_dir.join(format!("{}.md", name));
        tokio::fs::remove_file(&file_path)
            .await
            .map_err(|e| SerenaError::Io(e))
    }
}
