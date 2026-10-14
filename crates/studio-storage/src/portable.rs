use crate::blob::BlobStorage;
use crate::catalog::SqliteCatalog;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use studio_core::{
    document::CreativeDocument,
    error::StudioError,
    project::{Project, ProjectType},
};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

pub struct PortablePackageService {
    catalog: SqliteCatalog,
    #[allow(dead_code)]
    blob_storage: BlobStorage,
}

impl PortablePackageService {
    pub const MAX_UNCOMPRESSED_BYTES: u64 = 100 * 1024 * 1024; // 100 MB hard limit
    pub const MAX_ENTRIES: usize = 1000;

    pub fn new(catalog: SqliteCatalog, blob_storage: BlobStorage) -> Self {
        Self {
            catalog,
            blob_storage,
        }
    }

    pub fn export_package(
        &self,
        project_id: &str,
        dest_path: impl AsRef<Path>,
    ) -> Result<String, StudioError> {
        let project =
            self.catalog
                .get_project(project_id)?
                .ok_or_else(|| StudioError::NotFound {
                    resource_type: "project".into(),
                    resource_id: project_id.into(),
                    correlation_id: project_id.into(),
                })?;

        let latest_rev = self
            .catalog
            .get_latest_revision(project_id)?
            .ok_or_else(|| StudioError::PreconditionFailed {
                reason: "Project has no revisions to export".into(),
                correlation_id: project_id.into(),
            })?;

        let doc = self
            .catalog
            .get_document_by_revision(&latest_rev.id)?
            .unwrap_or_else(|| CreativeDocument::new(&project.id));

        let file = File::create(dest_path.as_ref()).map_err(|e| StudioError::Internal {
            message: format!("Cannot create export file: {}", e),
            correlation_id: project_id.into(),
        })?;

        let mut zip = ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        // 1. Write manifest
        let manifest = serde_json::json!({
            "schemaVersion": "portable-package/v1",
            "projectId": project.id,
            "title": project.title,
            "projectType": project.project_type,
            "revisionId": latest_rev.id,
            "sourceHash": latest_rev.source_hash,
            "exportedAt": chrono::Utc::now().to_rfc3339()
        });
        zip.start_file("manifest.json", options)
            .map_err(|e| StudioError::Internal {
                message: e.to_string(),
                correlation_id: project_id.into(),
            })?;
        let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap();
        zip.write_all(&manifest_bytes)
            .map_err(|e| StudioError::Internal {
                message: e.to_string(),
                correlation_id: project_id.into(),
            })?;

        // 2. Write document
        zip.start_file("document.json", options)
            .map_err(|e| StudioError::Internal {
                message: e.to_string(),
                correlation_id: project_id.into(),
            })?;
        let doc_bytes = serde_json::to_vec_pretty(&doc).unwrap();
        zip.write_all(&doc_bytes)
            .map_err(|e| StudioError::Internal {
                message: e.to_string(),
                correlation_id: project_id.into(),
            })?;

        zip.finish().map_err(|e| StudioError::Internal {
            message: e.to_string(),
            correlation_id: project_id.into(),
        })?;

        Ok(latest_rev.source_hash)
    }

    pub fn import_package(
        &self,
        package_path: impl AsRef<Path>,
        target_workspace_id: &str,
        new_title: Option<&str>,
    ) -> Result<Project, StudioError> {
        let file = File::open(package_path.as_ref()).map_err(|e| StudioError::Internal {
            message: format!("Cannot open package file: {}", e),
            correlation_id: "import".into(),
        })?;

        let mut archive = ZipArchive::new(file).map_err(|e| StudioError::InvalidInput {
            field: "package".into(),
            reason: format!("Invalid zip archive: {}", e),
            correlation_id: "import".into(),
        })?;

        if archive.len() > Self::MAX_ENTRIES {
            return Err(StudioError::InvalidInput {
                field: "package".into(),
                reason: format!("Archive has too many entries: {}", archive.len()),
                correlation_id: "import".into(),
            });
        }

        let mut total_uncompressed: u64 = 0;
        let mut manifest_data: Option<serde_json::Value> = None;
        let mut document_data: Option<CreativeDocument> = None;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| StudioError::Internal {
                message: e.to_string(),
                correlation_id: "import".into(),
            })?;

            let name = entry.name().to_string();

            // Security check: Path traversal, absolute paths
            if name.starts_with('/') || name.starts_with('\\') || name.contains("..") {
                return Err(StudioError::InvalidInput {
                    field: "package.path".into(),
                    reason: format!("Path traversal attempt detected: '{}'", name),
                    correlation_id: "import".into(),
                });
            }

            // Security check: Zip bomb
            total_uncompressed += entry.size();
            if total_uncompressed > Self::MAX_UNCOMPRESSED_BYTES {
                return Err(StudioError::QuotaExceeded {
                    reason: "Archive uncompressed size exceeds 100MB limit".into(),
                    correlation_id: "import".into(),
                });
            }

            let mut content = Vec::new();
            entry
                .read_to_end(&mut content)
                .map_err(|e| StudioError::Internal {
                    message: e.to_string(),
                    correlation_id: "import".into(),
                })?;

            if name == "manifest.json" {
                manifest_data = serde_json::from_slice(&content).ok();
            } else if name == "document.json" {
                document_data = serde_json::from_slice(&content).ok();
            }
        }

        let manifest = manifest_data.ok_or_else(|| StudioError::InvalidInput {
            field: "manifest".into(),
            reason: "Missing manifest.json in package".into(),
            correlation_id: "import".into(),
        })?;

        let schema_ver = manifest["schemaVersion"].as_str().unwrap_or("");
        if schema_ver != "portable-package/v1" {
            return Err(StudioError::UnsupportedCapability {
                capability: format!("Schema version '{}' is not supported", schema_ver),
                correlation_id: "import".into(),
            });
        }

        let title = new_title
            .or_else(|| manifest["title"].as_str())
            .unwrap_or("Dự án đã nhập");

        let p_type_str = manifest["projectType"].as_str().unwrap_or("poster");
        let p_type = match p_type_str {
            "carousel" => ProjectType::Carousel,
            "report" => ProjectType::Report,
            "motion" => ProjectType::Motion,
            _ => ProjectType::Poster,
        };

        let new_project = Project::new(target_workspace_id, title, p_type);
        self.catalog.create_project(&new_project)?;

        let doc = document_data.unwrap_or_else(|| CreativeDocument::new(&new_project.id));
        self.catalog.save_revision_and_document(
            &new_project.id,
            None,
            &doc,
            "import-service",
            "Nhập từ gói portable .contentstudio",
        )?;

        Ok(new_project)
    }
}
