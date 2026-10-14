use std::fs::File;
use std::io::Write;
use studio_core::{
    document::CreativeDocument,
    error::StudioError,
    project::{Project, ProjectType},
};
use studio_storage::{config::StorageConfig, initialize_storage, PortablePackageService};
use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[test]
fn test_portable_export_import_and_security() {
    let temp = tempdir().unwrap();
    let config = StorageConfig::new(temp.path().join("storage-a"));
    let (catalog, blob_storage, backup_service) = initialize_storage(&config).unwrap();
    let portable_svc = PortablePackageService::new(catalog.clone(), blob_storage);

    // 1. Create and populate a project
    let project = Project::new("ws-1", "Dự án Xuất Nhập", ProjectType::Carousel);
    catalog.create_project(&project).unwrap();

    let doc = CreativeDocument::new("doc-export-test");
    catalog
        .save_revision_and_document(&project.id, None, &doc, "tester", "Bản lưu trước xuất")
        .unwrap();

    // 2. Export package to .contentstudio
    let export_zip = temp.path().join("package.contentstudio");
    let hash = portable_svc
        .export_package(&project.id, &export_zip)
        .unwrap();
    assert!(!hash.is_empty());
    assert!(export_zip.exists());

    // 3. Import package into a new workspace
    let imported_project = portable_svc
        .import_package(&export_zip, "ws-2", Some("Dự án Đã Nhập ở WS2"))
        .unwrap();
    assert_eq!(imported_project.workspace_id, "ws-2");
    assert_eq!(imported_project.title, "Dự án Đã Nhập ở WS2");

    // 4. Security test: Malicious ZIP containing path traversal MUST fail!
    let malicious_zip_path = temp.path().join("malicious.contentstudio");
    {
        let file = File::create(&malicious_zip_path).unwrap();
        let mut zip = ZipWriter::new(file);
        let opts = SimpleFileOptions::default();

        zip.start_file("../etc_passwd.txt", opts).unwrap();
        zip.write_all(b"malicious payload").unwrap();
        zip.finish().unwrap();
    }

    let traversal_res = portable_svc.import_package(&malicious_zip_path, "ws-2", None);
    assert!(traversal_res.is_err());
    match traversal_res.err().unwrap() {
        StudioError::InvalidInput { field, reason, .. } => {
            assert_eq!(field, "package.path");
            assert!(reason.contains("Path traversal"));
        }
        other => panic!(
            "Expected InvalidInput with traversal rejection, got {:?}",
            other
        ),
    }

    // 5. Backup and restore integrity
    let backup_id = backup_service.create_backup().unwrap();
    assert!(backup_id.starts_with("backup-"));
    backup_service.restore_backup(&backup_id).unwrap();
}
