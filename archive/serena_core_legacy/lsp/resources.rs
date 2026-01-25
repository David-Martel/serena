use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};
use tar::Archive;
use zip::ZipArchive;

pub struct ResourceManager {
    pub root_dir: PathBuf,
}

impl ResourceManager {
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }

    pub fn get_tool_path(&self, tool_name: &str) -> PathBuf {
        self.root_dir.join(tool_name)
    }

    pub async fn ensure_tool(
        &self,
        tool_name: &str,
        url: &str,
        executable_name: &str,
    ) -> Result<PathBuf> {
        let tool_dir = self.get_tool_path(tool_name);
        let executable_path = tool_dir.join(executable_name);

        if executable_path.exists() {
            return Ok(executable_path);
        }

        tracing::info!("Downloading {} from {}", tool_name, url);
        fs::create_dir_all(&tool_dir).context("Failed to create tool directory")?;

        let response = reqwest::get(url).await.context("Failed to download tool")?;
        let bytes = response
            .bytes()
            .await
            .context("Failed to read response bytes")?;

        if url.ends_with(".zip") {
            self.extract_zip(&bytes, &tool_dir)?;
        } else if url.ends_with(".tar.gz") || url.ends_with(".tgz") {
            self.extract_tar_gz(&bytes, &tool_dir)?;
        } else {
            // Assume single binary
            // For simple binary downloads (like marksman)
            let mut file = fs::File::create(&executable_path)?;
            std::io::copy(&mut Cursor::new(bytes), &mut file)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = file.metadata()?.permissions();
                perms.set_mode(0o755);
                file.set_permissions(perms)?;
            }
        }

        if !executable_path.exists() {
            // If it was an archive, maybe the structure is different.
            // For brute force, we just warn and hope the user fixes it or we refine logic later.
            // Often archives have a top-level folder.
            anyhow::bail!(
                "Executable {} not found after extraction in {:?}",
                executable_name,
                tool_dir
            );
        }

        Ok(executable_path)
    }

    fn extract_zip(&self, bytes: &[u8], dest: &Path) -> Result<()> {
        let cursor = Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor)?;
        archive.extract(dest)?;
        Ok(())
    }

    fn extract_tar_gz(&self, bytes: &[u8], dest: &Path) -> Result<()> {
        let cursor = Cursor::new(bytes);
        let tar = GzDecoder::new(cursor);
        let mut archive = Archive::new(tar);
        archive.unpack(dest)?;
        Ok(())
    }
}
