pub mod backup;
pub mod blob;
pub mod catalog;
pub mod config;
pub mod migration;
pub mod portable;

pub use backup::BackupService;
pub use blob::BlobStorage;
pub use catalog::SqliteCatalog;
pub use config::StorageConfig;
pub use migration::MigrationEngine;
pub use portable::PortablePackageService;

use rusqlite::Connection;
use studio_core::error::StudioError;

pub fn initialize_storage(
    config: &StorageConfig,
) -> Result<(SqliteCatalog, BlobStorage, BackupService), StudioError> {
    config
        .ensure_directories()
        .map_err(|e| StudioError::Internal {
            message: format!("Failed to create directories: {}", e),
            correlation_id: "init".into(),
        })?;

    let mut conn = Connection::open(config.db_path()).map_err(|e| StudioError::Internal {
        message: format!("Failed to open sqlite: {}", e),
        correlation_id: "init".into(),
    })?;

    MigrationEngine::apply(&mut conn).map_err(|e| StudioError::Internal {
        message: format!("Migration error: {}", e),
        correlation_id: "migration".into(),
    })?;

    let catalog = SqliteCatalog::new(conn);
    let blob_storage = BlobStorage::new(config.blobs_dir(), config.staging_dir());
    let backup_service = BackupService::new(&config.root_dir, config.backups_dir());

    Ok((catalog, blob_storage, backup_service))
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::{
        actor::ActorRole,
        document::CreativeDocument,
        project::{Project, ProjectType},
    };
    use tempfile::tempdir;

    #[test]
    fn test_storage_lifecycle_and_cas() {
        let temp = tempdir().unwrap();
        let config = StorageConfig::new(temp.path());
        let (catalog, blob_storage, backup_service) = initialize_storage(&config).unwrap();

        // 1. Create project
        let project = Project::new("ws-1", "Dự án Poster Mẫu", ProjectType::Poster);
        catalog.create_project(&project).unwrap();

        let retrieved = catalog.get_project(&project.id).unwrap().unwrap();
        assert_eq!(retrieved.title, "Dự án Poster Mẫu");

        // 2. Save revision 1
        let doc1 = CreativeDocument::new("doc-1");
        let rev1 = catalog
            .save_revision_and_document(&project.id, None, &doc1, "user-1", "Bản khởi tạo")
            .unwrap();
        assert_eq!(rev1.project_id, project.id);

        // 3. Save revision 2 with valid CAS
        let mut doc2 = doc1.clone();
        doc2.artboards.push(studio_core::document::Artboard {
            id: "ab-1".into(),
            width: 1080,
            height: 1080,
            background: studio_core::document::ArtboardBackground {
                token: "brand.surface".into(),
            },
            elements: vec![],
        });
        let rev2 = catalog
            .save_revision_and_document(
                &project.id,
                Some(&rev1.id),
                &doc2,
                "user-1",
                "Thêm artboard 1",
            )
            .unwrap();
        assert_ne!(rev1.id, rev2.id);

        // 4. Stale CAS should fail!
        let stale_result = catalog.save_revision_and_document(
            &project.id,
            Some(&rev1.id), // rev1 is stale now
            &doc2,
            "user-2",
            "Cố tình ghi đè bằng phiên bản cũ",
        );
        assert!(stale_result.is_err());
        match stale_result.err().unwrap() {
            StudioError::ConflictCas {
                expected, current, ..
            } => {
                assert_eq!(expected, rev1.id);
                assert_eq!(current, rev2.id);
            }
            other => panic!("Expected ConflictCas, got {:?}", other),
        }

        // 5. Blob storage deduplication
        let test_bytes = b"Hello Local Content Studio!";
        let (hash1, size1) = blob_storage.write_blob(test_bytes).unwrap();
        let (hash2, size2) = blob_storage.write_blob(test_bytes).unwrap();
        assert_eq!(hash1, hash2);
        assert_eq!(size1, size2);
        let read_bytes = blob_storage.read_blob(&hash1).unwrap();
        assert_eq!(read_bytes, test_bytes);

        // 6. Audit log
        let audit_id = catalog
            .record_audit(
                "user-1",
                ActorRole::Human,
                "ws-1",
                "project.create",
                "DRAFT",
                Some(&project.id),
                None,
            )
            .unwrap();
        assert!(!audit_id.is_empty());

        // 7. Backup creation
        let backup_id = backup_service.create_backup().unwrap();
        assert!(backup_id.starts_with("backup-"));
    }
}
