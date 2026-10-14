use chrono::Utc;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use studio_core::error::StudioError;

#[derive(Debug, Clone)]
pub struct BackupService {
    root_dir: PathBuf,
    backups_dir: PathBuf,
}

impl BackupService {
    pub fn new(root_dir: impl AsRef<Path>, backups_dir: impl AsRef<Path>) -> Self {
        Self {
            root_dir: root_dir.as_ref().to_path_buf(),
            backups_dir: backups_dir.as_ref().to_path_buf(),
        }
    }

    pub fn create_backup(&self) -> Result<String, StudioError> {
        let timestamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
        let backup_id = format!("backup-{}", timestamp);
        let backup_folder = self.backups_dir.join(&backup_id);
        std::fs::create_dir_all(&backup_folder).map_err(|e| StudioError::Internal {
            message: e.to_string(),
            correlation_id: backup_id.clone(),
        })?;

        // 1. Consistent SQLite backup with WAL checkpointing
        let db_src = self.root_dir.join("catalog.sqlite");
        let mut db_checksum = String::new();
        if db_src.exists() {
            let db_dest = backup_folder.join("catalog.sqlite");
            // Use rusqlite backup API to copy SQLite database with active WAL
            let src_conn =
                rusqlite::Connection::open(&db_src).map_err(|e| StudioError::Internal {
                    message: format!("Failed to open source database for backup: {}", e),
                    correlation_id: backup_id.clone(),
                })?;
            // Force checkpoint of WAL into main database
            let _ = src_conn.pragma_update(None, "wal_checkpoint", "TRUNCATE");

            let mut dst_conn =
                rusqlite::Connection::open(&db_dest).map_err(|e| StudioError::Internal {
                    message: format!("Failed to create backup database file: {}", e),
                    correlation_id: backup_id.clone(),
                })?;

            let backup = rusqlite::backup::Backup::new(&src_conn, &mut dst_conn).map_err(|e| {
                StudioError::Internal {
                    message: format!("Failed to initialize SQLite backup: {}", e),
                    correlation_id: backup_id.clone(),
                }
            })?;

            backup
                .run_to_completion(10, std::time::Duration::from_millis(10), None)
                .map_err(|e| StudioError::Internal {
                    message: format!("SQLite backup run failed: {}", e),
                    correlation_id: backup_id.clone(),
                })?;

            drop(backup);
            drop(dst_conn);
            drop(src_conn);

            // Compute SHA-256 of backed-up database
            let db_bytes = std::fs::read(&db_dest).map_err(|e| StudioError::Internal {
                message: e.to_string(),
                correlation_id: backup_id.clone(),
            })?;
            let mut hasher = Sha256::new();
            hasher.update(&db_bytes);
            db_checksum = hex::encode(hasher.finalize());
        }

        // 2. Backup managed Blobs directory
        let blobs_src = self.root_dir.join("blobs");
        let blobs_dest = backup_folder.join("blobs");
        std::fs::create_dir_all(&blobs_dest).map_err(|e| StudioError::Internal {
            message: e.to_string(),
            correlation_id: backup_id.clone(),
        })?;

        let mut blob_manifest_items = Vec::new();
        if blobs_src.exists() {
            if let Ok(entries) = std::fs::read_dir(&blobs_src) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let file_name = entry.file_name().to_string_lossy().to_string();
                        let bytes = std::fs::read(&path).map_err(|e| StudioError::Internal {
                            message: format!("Failed to read blob {}: {}", file_name, e),
                            correlation_id: backup_id.clone(),
                        })?;
                        let mut hasher = Sha256::new();
                        hasher.update(&bytes);
                        let hash = hex::encode(hasher.finalize());

                        std::fs::copy(&path, blobs_dest.join(&file_name)).map_err(|e| {
                            StudioError::Internal {
                                message: format!("Failed to copy blob {}: {}", file_name, e),
                                correlation_id: backup_id.clone(),
                            }
                        })?;

                        blob_manifest_items.push(serde_json::json!({
                            "sha256": hash,
                            "size_bytes": bytes.len(),
                            "file_name": file_name,
                        }));
                    }
                }
            }
        }

        // 3. Write backup manifest with checksums
        let manifest_content = serde_json::json!({
            "backup_id": backup_id,
            "created_at": Utc::now().to_rfc3339(),
            "schema_version": 2,
            "db_checksum": db_checksum,
            "blobs": blob_manifest_items,
        });
        let manifest_bytes = serde_json::to_vec_pretty(&manifest_content).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(&manifest_bytes);
        let checksum = hex::encode(hasher.finalize());

        std::fs::write(backup_folder.join("manifest.json"), &manifest_bytes).map_err(|e| {
            StudioError::Internal {
                message: e.to_string(),
                correlation_id: backup_id.clone(),
            }
        })?;

        std::fs::write(backup_folder.join("manifest.sha256"), checksum).map_err(|e| {
            StudioError::Internal {
                message: e.to_string(),
                correlation_id: backup_id.clone(),
            }
        })?;

        Ok(backup_id)
    }

    pub fn restore_backup(&self, backup_id: &str) -> Result<(), StudioError> {
        let backup_folder = self.backups_dir.join(backup_id);
        if !backup_folder.exists() {
            return Err(StudioError::NotFound {
                resource_type: "backup".into(),
                resource_id: backup_id.into(),
                correlation_id: backup_id.into(),
            });
        }

        // 1. Verify manifest and manifest checksum
        let manifest_path = backup_folder.join("manifest.json");
        let manifest_sha_path = backup_folder.join("manifest.sha256");
        if !manifest_path.exists() || !manifest_sha_path.exists() {
            return Err(StudioError::InvalidInput {
                field: "manifest".into(),
                reason: "Tệp manifest.json hoặc manifest.sha256 bị thiếu trong bản sao lưu".into(),
                correlation_id: backup_id.into(),
            });
        }

        let manifest_bytes = std::fs::read(&manifest_path).map_err(|e| StudioError::Internal {
            message: e.to_string(),
            correlation_id: backup_id.into(),
        })?;
        let expected_sha = std::fs::read_to_string(&manifest_sha_path)
            .map_err(|e| StudioError::Internal {
                message: e.to_string(),
                correlation_id: backup_id.into(),
            })?
            .trim()
            .to_string();

        let mut hasher = Sha256::new();
        hasher.update(&manifest_bytes);
        let actual_sha = hex::encode(hasher.finalize());
        if actual_sha != expected_sha {
            return Err(StudioError::InvalidInput {
                field: "manifest.sha256".into(),
                reason:
                    "Mã băm manifest không khớp, tệp sao lưu có dấu hiệu bị can thiệp hoặc hỏng"
                        .into(),
                correlation_id: backup_id.into(),
            });
        }

        let manifest_val: serde_json::Value =
            serde_json::from_slice(&manifest_bytes).map_err(|e| StudioError::InvalidInput {
                field: "manifest.json".into(),
                reason: format!("Lỗi phân tích manifest JSON: {}", e),
                correlation_id: backup_id.into(),
            })?;

        // 2. Verify database checksum
        let expected_db_sha = manifest_val["db_checksum"].as_str().unwrap_or("");
        let db_src = backup_folder.join("catalog.sqlite");
        if db_src.exists() && !expected_db_sha.is_empty() {
            let db_bytes = std::fs::read(&db_src).map_err(|e| StudioError::Internal {
                message: e.to_string(),
                correlation_id: backup_id.into(),
            })?;
            let mut hasher = Sha256::new();
            hasher.update(&db_bytes);
            let actual_db_sha = hex::encode(hasher.finalize());
            if actual_db_sha != expected_db_sha {
                return Err(StudioError::InvalidInput {
                    field: "catalog.sqlite".into(),
                    reason: "Mã băm catalog.sqlite không khớp với manifest".into(),
                    correlation_id: backup_id.into(),
                });
            }
        }

        // 3. Verify all blobs in manifest before mutating target root
        let blobs_src = backup_folder.join("blobs");
        if let Some(blobs_arr) = manifest_val["blobs"].as_array() {
            for b in blobs_arr {
                let file_name = b["file_name"].as_str().unwrap_or("");
                let expected_sha = b["sha256"].as_str().unwrap_or("");
                let b_path = blobs_src.join(file_name);
                if !b_path.exists() {
                    return Err(StudioError::InvalidInput {
                        field: "blobs".into(),
                        reason: format!("Tệp blob '{}' bị thiếu trong bản sao lưu", file_name),
                        correlation_id: backup_id.into(),
                    });
                }
                let b_bytes = std::fs::read(&b_path).map_err(|e| StudioError::Internal {
                    message: e.to_string(),
                    correlation_id: backup_id.into(),
                })?;
                let mut hasher = Sha256::new();
                hasher.update(&b_bytes);
                let actual_sha = hex::encode(hasher.finalize());
                if actual_sha != expected_sha {
                    return Err(StudioError::InvalidInput {
                        field: "blobs.checksum".into(),
                        reason: format!("Mã băm blob '{}' bị hỏng", file_name),
                        correlation_id: backup_id.into(),
                    });
                }
            }
        }

        // 4. Remove stale WAL and SHM files in target root_dir
        let _ = std::fs::remove_file(self.root_dir.join("catalog.sqlite-wal"));
        let _ = std::fs::remove_file(self.root_dir.join("catalog.sqlite-shm"));

        // 5. Restore database
        if db_src.exists() {
            let db_dest = self.root_dir.join("catalog.sqlite");
            std::fs::copy(&db_src, &db_dest).map_err(|e| StudioError::Internal {
                message: e.to_string(),
                correlation_id: backup_id.into(),
            })?;

            // Verify integrity of restored database
            let conn = rusqlite::Connection::open(&db_dest).map_err(|e| StudioError::Internal {
                message: format!("Không thể mở database sau khôi phục: {}", e),
                correlation_id: backup_id.into(),
            })?;
            let integrity: String = conn
                .query_row("PRAGMA integrity_check", [], |row| row.get(0))
                .unwrap_or_else(|_| "error".into());
            if integrity != "ok" {
                return Err(StudioError::Internal {
                    message: format!("Kiểm tra tính toàn vẹn SQLite thất bại: {}", integrity),
                    correlation_id: backup_id.into(),
                });
            }
        }

        // 6. Restore blobs
        if blobs_src.exists() {
            let blobs_dest = self.root_dir.join("blobs");
            std::fs::create_dir_all(&blobs_dest).map_err(|e| StudioError::Internal {
                message: e.to_string(),
                correlation_id: backup_id.into(),
            })?;
            if let Ok(entries) = std::fs::read_dir(&blobs_src) {
                for entry in entries.flatten() {
                    let file_name = entry.file_name();
                    let _ = std::fs::copy(entry.path(), blobs_dest.join(file_name));
                }
            }
        }

        Ok(())
    }
}
