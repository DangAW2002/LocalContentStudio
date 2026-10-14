use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use studio_core::{
    actor::ActorContext,
    asset::{Asset, AssetKind, AssetScope},
    error::StudioError,
    project::{Project, ProjectRevision, ProjectType},
};
use studio_storage::SqliteCatalog;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopedGrant {
    pub grant_token: String,
    pub subject_id: String,
    pub target_workspace_id: String,
    pub target_project_id: Option<String>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
}

// Thread-safe grant store for DSH scoped tokens
static GRANT_STORE: std::sync::LazyLock<Arc<Mutex<HashMap<String, ScopedGrant>>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DshSessionToken {
    pub session_id: String,
    pub workspace_id: String,
    pub grant_token: String,
    pub target_project_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DshArtifactCard {
    pub card_type: String, // "project_preview", "render_job", "publish_readiness"
    pub title: String,
    pub workspace_id: String,
    pub project_id: String,
    pub revision_id: Option<String>,
    pub status: String,
    pub action_deeplink: String,
}

pub struct DshConnectorService;

impl DshConnectorService {
    /// Generates deep link URL and cryptographically stored scoped token to open Studio project from DSH chat.
    /// Invariant: DSH session alone cannot grant access; active workspace binding must match.
    pub fn create_studio_deeplink(
        actor: &ActorContext,
        project_id: &str,
    ) -> Result<DshArtifactCard, StudioError> {
        let grant_token = format!(
            "grant_{}_{}_{}",
            actor.active_workspace_id,
            project_id,
            uuid::Uuid::new_v4().simple()
        );

        let grant = ScopedGrant {
            grant_token: grant_token.clone(),
            subject_id: actor.actor_id.clone(),
            target_workspace_id: actor.active_workspace_id.clone(),
            target_project_id: Some(project_id.to_string()),
            issued_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::minutes(15),
            used: false,
        };

        GRANT_STORE
            .lock()
            .unwrap()
            .insert(grant_token.clone(), grant);

        let deeplink = format!(
            "contentstudio://workspace/{}/project/{}?grant={}",
            actor.active_workspace_id, project_id, grant_token
        );

        Ok(DshArtifactCard {
            card_type: "project_preview".into(),
            title: format!("Dự án Content Studio: {}", project_id),
            workspace_id: actor.active_workspace_id.clone(),
            project_id: project_id.to_string(),
            revision_id: None,
            status: "READY".into(),
            action_deeplink: deeplink,
        })
    }

    /// Verifies scoped deep link token, expiration, single use, and workspace boundary.
    /// Rejects cross-workspace tampering (Workspace B trying to open Workspace A).
    /// Rejects forged prefix tokens, expired tokens, and token replays.
    pub fn verify_scoped_grant(
        actor: &ActorContext,
        target_workspace_id: &str,
        grant_token: &str,
    ) -> Result<(), StudioError> {
        if actor.active_workspace_id != target_workspace_id {
            return Err(StudioError::ForbiddenScope {
                resource_id: target_workspace_id.to_string(),
                correlation_id: actor.actor_id.clone(),
            });
        }

        let mut store = GRANT_STORE.lock().unwrap();
        let grant = store
            .get_mut(grant_token)
            .ok_or_else(|| StudioError::Unauthorized {
                correlation_id: actor.actor_id.clone(),
            })?;

        if grant.used {
            return Err(StudioError::Unauthorized {
                correlation_id: actor.actor_id.clone(),
            });
        }

        if Utc::now() > grant.expires_at {
            return Err(StudioError::Unauthorized {
                correlation_id: actor.actor_id.clone(),
            });
        }

        if grant.target_workspace_id != target_workspace_id {
            return Err(StudioError::ForbiddenScope {
                resource_id: target_workspace_id.to_string(),
                correlation_id: actor.actor_id.clone(),
            });
        }

        // Single-use token: mark used
        grant.used = true;
        Ok(())
    }
}

// Legacy Migration Subsystem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyProjectRecord {
    pub legacy_id: String,
    pub title: String,
    pub kind: String,                          // "poster", "carousel", "report"
    pub original_workspace_id: Option<String>, // NULL if unassigned
    pub content_json: String,
    pub asset_refs: Vec<LegacyAssetRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyAssetRef {
    pub asset_id: String,
    pub filename: String,
    pub sha256: String,
    pub mime_type: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationDryRunReport {
    pub total_projects: usize,
    pub unassigned_projects: usize,
    pub total_assets: usize,
    pub missing_blobs: Vec<String>,
    pub mapped_workspace_id: String,
    pub conflicts: Vec<String>,
    pub safe_to_import: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationImportResult {
    pub imported_projects: usize,
    pub imported_assets: usize,
    pub quarantined_assets: usize,
    pub audit_log_id: String,
}

pub struct LegacyMigrationService;

impl LegacyMigrationService {
    /// Dry-run simulation of legacy project import.
    /// Checks record counts, blob integrity, and flags unassigned legacy projects.
    pub fn dry_run(
        records: &[LegacyProjectRecord],
        target_workspace_id: &str,
        available_blob_hashes: &[String],
    ) -> MigrationDryRunReport {
        let mut unassigned = 0;
        let mut total_assets = 0;
        let mut missing_blobs = Vec::new();
        let mut conflicts = Vec::new();

        for rec in records {
            if rec.original_workspace_id.is_none() {
                unassigned += 1;
            } else if let Some(ref orig_ws) = rec.original_workspace_id {
                if orig_ws != target_workspace_id {
                    conflicts.push(format!(
                        "Project '{}' ({}) thuộc workspace '{}', xung đột với target '{}'",
                        rec.title, rec.legacy_id, orig_ws, target_workspace_id
                    ));
                }
            }

            for asset in &rec.asset_refs {
                total_assets += 1;
                if !available_blob_hashes.contains(&asset.sha256) {
                    missing_blobs.push(format!("{}: {}", asset.asset_id, asset.sha256));
                }
            }
        }

        let safe_to_import = conflicts.is_empty() && missing_blobs.is_empty();

        MigrationDryRunReport {
            total_projects: records.len(),
            unassigned_projects: unassigned,
            total_assets,
            missing_blobs,
            mapped_workspace_id: target_workspace_id.to_string(),
            conflicts,
            safe_to_import,
        }
    }

    /// Executes import of legacy projects and assets into Studio Catalog.
    /// Invariants:
    /// 1. Copy, never mutate original legacy DB.
    /// 2. Imported assets MUST default to QUARANTINED state until human review.
    /// 3. Creates explicit audit log entry.
    pub fn execute_import(
        catalog: &SqliteCatalog,
        actor: &ActorContext,
        records: &[LegacyProjectRecord],
        target_workspace_id: &str,
    ) -> Result<MigrationImportResult, StudioError> {
        Self::execute_import_with_options(catalog, actor, records, target_workspace_id, false)
    }

    pub fn execute_import_with_options(
        catalog: &SqliteCatalog,
        actor: &ActorContext,
        records: &[LegacyProjectRecord],
        target_workspace_id: &str,
        dry_run: bool,
    ) -> Result<MigrationImportResult, StudioError> {
        if actor.active_workspace_id != target_workspace_id {
            return Err(StudioError::ForbiddenScope {
                resource_id: target_workspace_id.to_string(),
                correlation_id: actor.actor_id.clone(),
            });
        }

        let mut imported_projects = 0;
        let mut imported_assets = 0;
        let mut quarantined_assets = 0;

        for rec in records {
            // Strictly validate document JSON structure; reject malformed JSON instead of silent fallback!
            let doc: studio_core::document::CreativeDocument =
                serde_json::from_str(&rec.content_json).map_err(|e| StudioError::InvalidInput {
                    field: "content_json".into(),
                    reason: format!(
                        "Cấu trúc JSON tài liệu của dự án cũ '{}' không hợp lệ: {}",
                        rec.legacy_id, e
                    ),
                    correlation_id: rec.legacy_id.clone(),
                })?;

            if !dry_run {
                let p_type = match rec.kind.as_str() {
                    "carousel" => ProjectType::Carousel,
                    "report" => ProjectType::Report,
                    _ => ProjectType::Poster,
                };

                let project = Project::new(target_workspace_id, &rec.title, p_type);
                catalog.create_project(&project)?;

                let _rev: ProjectRevision = catalog.save_revision_and_document(
                    &project.id,
                    None,
                    &doc,
                    &actor.actor_id,
                    &format!("Di trú từ dự án cũ [{}]", rec.legacy_id),
                )?;

                // Import referenced assets in QUARANTINED state
                for a_ref in &rec.asset_refs {
                    let asset = Asset::new_quarantined(
                        AssetScope::Workspace,
                        Some(target_workspace_id.to_string()),
                        AssetKind::Image,
                        &a_ref.filename,
                        &a_ref.mime_type,
                        &a_ref.mime_type,
                        &a_ref.sha256,
                        a_ref.size_bytes,
                        format!("Di trú từ Social v1 legacy asset {}", a_ref.asset_id),
                    );
                    catalog.save_asset(&asset)?;
                }
            }

            imported_projects += 1;
            imported_assets += rec.asset_refs.len();
            quarantined_assets += rec.asset_refs.len();
        }

        let audit_id = if !dry_run {
            catalog.record_audit(
                &actor.actor_id,
                actor.role,
                target_workspace_id,
                "migration.import_legacy",
                "REVERSIBLE",
                None,
                Some(&serde_json::json!({
                    "imported_projects": imported_projects,
                    "imported_assets": imported_assets,
                    "quarantined_assets": quarantined_assets,
                })),
            )?
        } else {
            "dry-run-preview".to_string()
        };

        Ok(MigrationImportResult {
            imported_projects,
            imported_assets,
            quarantined_assets,
            audit_log_id: audit_id,
        })
    }
}
