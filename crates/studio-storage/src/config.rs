use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub root_dir: PathBuf,
}

impl StorageConfig {
    pub fn new(root_dir: impl AsRef<Path>) -> Self {
        Self {
            root_dir: root_dir.as_ref().to_path_buf(),
        }
    }

    pub fn db_path(&self) -> PathBuf {
        self.root_dir.join("catalog.sqlite")
    }

    pub fn blobs_dir(&self) -> PathBuf {
        self.root_dir.join("managed").join("blobs")
    }

    pub fn staging_dir(&self) -> PathBuf {
        self.root_dir.join("staging")
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.root_dir.join("backups")
    }

    pub fn exports_dir(&self) -> PathBuf {
        self.root_dir.join("exports")
    }

    pub fn ensure_directories(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.root_dir)?;
        std::fs::create_dir_all(self.blobs_dir())?;
        std::fs::create_dir_all(self.staging_dir())?;
        std::fs::create_dir_all(self.backups_dir())?;
        std::fs::create_dir_all(self.exports_dir())?;
        Ok(())
    }
}
