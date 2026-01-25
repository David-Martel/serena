//! Resource management for downloading and installing language servers
//!
//! This module provides utilities to download language server binaries from URLs,
//! extract archives (zip, tar.gz), and manage their installation in a local directory.

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};
use tar::Archive;
use zip::ZipArchive;

/// Manages language server resource downloads and installations
///
/// The ResourceManager handles downloading language server binaries from URLs,
/// extracting compressed archives, and caching them locally for reuse.
#[derive(Debug, Clone)]
pub struct ResourceManager {
    /// Root directory where tools will be installed
    pub root_dir: PathBuf,
}

impl ResourceManager {
    /// Create a new ResourceManager with the specified root directory
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }

    /// Get the installation path for a specific tool
    pub fn get_tool_path(&self, tool_name: &str) -> PathBuf {
        self.root_dir.join(tool_name)
    }

    /// Ensure a tool is installed, downloading it if necessary
    ///
    /// # Arguments
    /// * `tool_name` - Name of the tool (used as subdirectory name)
    /// * `url` - Download URL for the tool
    /// * `executable_name` - Name of the executable within the tool directory
    ///
    /// # Returns
    /// Path to the executable on success
    ///
    /// # Supported Formats
    /// - `.zip` - ZIP archives
    /// - `.tar.gz` / `.tgz` - Gzipped tarballs
    /// - Other - Treated as raw binary download
    pub async fn ensure_tool(
        &self,
        tool_name: &str,
        url: &str,
        executable_name: &str,
    ) -> Result<PathBuf> {
        let tool_dir = self.get_tool_path(tool_name);
        let executable_path = tool_dir.join(executable_name);

        // Return early if already installed
        if executable_path.exists() {
            tracing::debug!("Tool {} already installed at {:?}", tool_name, executable_path);
            return Ok(executable_path);
        }

        tracing::info!("Downloading {} from {}", tool_name, url);
        fs::create_dir_all(&tool_dir).context("Failed to create tool directory")?;

        let response = reqwest::get(url).await.context("Failed to download tool")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to download {}: HTTP {}",
                tool_name,
                response.status()
            );
        }

        let bytes = response
            .bytes()
            .await
            .context("Failed to read response bytes")?;

        // Extract based on file extension
        if url.ends_with(".zip") {
            self.extract_zip(&bytes, &tool_dir)?;
        } else if url.ends_with(".tar.gz") || url.ends_with(".tgz") {
            self.extract_tar_gz(&bytes, &tool_dir)?;
        } else {
            // Assume single binary download
            self.write_binary(&bytes, &executable_path)?;
        }

        // Verify the executable exists after extraction
        if !executable_path.exists() {
            // Try to find it in subdirectories (common with archives)
            if let Some(found_path) = self.find_executable_in_dir(&tool_dir, executable_name)? {
                tracing::info!("Found executable at {:?}", found_path);
                return Ok(found_path);
            }

            anyhow::bail!(
                "Executable {} not found after extraction in {:?}",
                executable_name,
                tool_dir
            );
        }

        tracing::info!("Successfully installed {} at {:?}", tool_name, executable_path);
        Ok(executable_path)
    }

    /// Extract a ZIP archive to the destination directory
    fn extract_zip(&self, bytes: &[u8], dest: &Path) -> Result<()> {
        let cursor = Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor).context("Failed to open ZIP archive")?;
        archive.extract(dest).context("Failed to extract ZIP archive")?;

        // Set executable permissions on Unix
        #[cfg(unix)]
        self.set_executable_permissions(dest)?;

        Ok(())
    }

    /// Extract a gzipped tarball to the destination directory
    fn extract_tar_gz(&self, bytes: &[u8], dest: &Path) -> Result<()> {
        let cursor = Cursor::new(bytes);
        let tar = GzDecoder::new(cursor);
        let mut archive = Archive::new(tar);
        archive.unpack(dest).context("Failed to extract tar.gz archive")?;

        // Set executable permissions on Unix
        #[cfg(unix)]
        self.set_executable_permissions(dest)?;

        Ok(())
    }

    /// Write a binary file directly
    fn write_binary(&self, bytes: &[u8], path: &Path) -> Result<()> {
        let mut file = fs::File::create(path).context("Failed to create executable file")?;
        std::io::copy(&mut Cursor::new(bytes), &mut file)
            .context("Failed to write executable")?;

        // Set executable permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = file.metadata()?.permissions();
            perms.set_mode(0o755);
            file.set_permissions(perms)?;
        }

        Ok(())
    }

    /// Set executable permissions on all files in a directory (Unix only)
    #[cfg(unix)]
    fn set_executable_permissions(&self, dir: &Path) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        use walkdir::WalkDir;

        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Ok(metadata) = entry.metadata() {
                    let mut perms = metadata.permissions();
                    // Add execute permission if it looks like an executable
                    let current_mode = perms.mode();
                    if current_mode & 0o111 == 0 {
                        // Check if it might be a binary (no extension or common binary extensions)
                        let path = entry.path();
                        let extension = path.extension().and_then(|e| e.to_str());
                        if extension.is_none() || matches!(extension, Some("so" | "dylib")) {
                            perms.set_mode(current_mode | 0o755);
                            let _ = fs::set_permissions(path, perms);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Find an executable in a directory tree (handles nested archive structures)
    fn find_executable_in_dir(&self, dir: &Path, name: &str) -> Result<Option<PathBuf>> {
        for entry in walkdir::WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name == name {
                        return Ok(Some(entry.path().to_path_buf()));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Check if a tool is installed
    pub fn is_installed(&self, tool_name: &str, executable_name: &str) -> bool {
        self.get_tool_path(tool_name).join(executable_name).exists()
    }

    /// Remove an installed tool
    pub fn remove_tool(&self, tool_name: &str) -> Result<()> {
        let tool_dir = self.get_tool_path(tool_name);
        if tool_dir.exists() {
            fs::remove_dir_all(&tool_dir).context("Failed to remove tool directory")?;
            tracing::info!("Removed tool {} from {:?}", tool_name, tool_dir);
        }
        Ok(())
    }

    /// List all installed tools
    pub fn list_installed_tools(&self) -> Result<Vec<String>> {
        let mut tools = Vec::new();
        if self.root_dir.exists() {
            for entry in fs::read_dir(&self.root_dir)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        tools.push(name.to_string());
                    }
                }
            }
        }
        Ok(tools)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_get_tool_path() {
        let temp = tempdir().unwrap();
        let manager = ResourceManager::new(temp.path().to_path_buf());

        let path = manager.get_tool_path("rust-analyzer");
        assert!(path.ends_with("rust-analyzer"));
    }

    #[test]
    fn test_is_installed_false() {
        let temp = tempdir().unwrap();
        let manager = ResourceManager::new(temp.path().to_path_buf());

        assert!(!manager.is_installed("nonexistent", "binary"));
    }

    #[test]
    fn test_list_installed_tools_empty() {
        let temp = tempdir().unwrap();
        let manager = ResourceManager::new(temp.path().to_path_buf());

        let tools = manager.list_installed_tools().unwrap();
        assert!(tools.is_empty());
    }
}
