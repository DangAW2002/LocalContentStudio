use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct BlobStorage {
    blobs_dir: PathBuf,
    staging_dir: PathBuf,
}

impl BlobStorage {
    pub fn new(blobs_dir: impl AsRef<Path>, staging_dir: impl AsRef<Path>) -> Self {
        Self {
            blobs_dir: blobs_dir.as_ref().to_path_buf(),
            staging_dir: staging_dir.as_ref().to_path_buf(),
        }
    }

    pub fn write_blob(&self, data: &[u8]) -> std::io::Result<(String, u64)> {
        let size = data.len() as u64;
        let mut hasher = Sha256::new();
        hasher.update(data);
        let sha256 = hex::encode(hasher.finalize());

        let target_path = self.blobs_dir.join(&sha256);
        if target_path.exists() {
            // Already stored (deduplicated)
            return Ok((sha256, size));
        }

        let stage_file_name = format!("stage-{}", uuid::Uuid::new_v4());
        let stage_path = self.staging_dir.join(stage_file_name);

        {
            let mut file = File::create(&stage_path)?;
            file.write_all(data)?;
            file.sync_all()?;
        }

        std::fs::rename(&stage_path, &target_path)?;
        Ok((sha256, size))
    }

    pub fn read_blob(&self, sha256: &str) -> std::io::Result<Vec<u8>> {
        let target_path = self.blobs_dir.join(sha256);
        if !target_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Blob {} not found", sha256),
            ));
        }
        let mut file = File::open(&target_path)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Ok(buf)
    }

    pub fn blob_exists(&self, sha256: &str) -> bool {
        self.blobs_dir.join(sha256).exists()
    }
}
