use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};
use studio_core::{
    actor::ActorRole,
    asset::{Asset, AssetKind, AssetScope, RightsAttestation, RightsState},
    brand::{BrandKit, BrandVersion, BrandVersionState, VisualTokens, VoiceGuidelines},
    document::CreativeDocument,
    error::StudioError,
    media::{RenderJob, RenderJobState},
    project::{Project, ProjectRevision, ProjectStatus, ProjectType},
    research::{ResearchSnapshot, ResearchSource},
};

#[derive(Clone)]
pub struct SqliteCatalog {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteCatalog {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    // Projects
    pub fn create_project(&self, project: &Project) -> Result<(), StudioError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO projects (id, workspace_id, title, project_type, status, manifest_version, brand_ref, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                project.id,
                project.workspace_id,
                project.title,
                serde_json::to_string(&project.project_type).unwrap().trim_matches('"'),
                serde_json::to_string(&project.status).unwrap().trim_matches('"'),
                project.manifest_version,
                project.brand_ref,
                project.created_at.to_rfc3339(),
                project.updated_at.to_rfc3339(),
            ],
        ).map_err(|e| StudioError::Internal {
            message: e.to_string(),
            correlation_id: project.id.clone(),
        })?;
        Ok(())
    }

    pub fn get_project(&self, id: &str) -> Result<Option<Project>, StudioError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, workspace_id, title, project_type, status, manifest_version, brand_ref, created_at, updated_at
             FROM projects WHERE id = ?1",
        ).map_err(|e| StudioError::Internal { message: e.to_string(), correlation_id: id.into() })?;

        let res = stmt.query_row(params![id], |row| {
            let p_type_str: String = row.get(3)?;
            let status_str: String = row.get(4)?;
            let created_at_str: String = row.get(7)?;
            let updated_at_str: String = row.get(8)?;

            Ok(Project {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                title: row.get(2)?,
                project_type: serde_json::from_str(&format!("\"{}\"", p_type_str))
                    .unwrap_or(ProjectType::Poster),
                status: serde_json::from_str(&format!("\"{}\"", status_str))
                    .unwrap_or(ProjectStatus::Active),
                manifest_version: row.get(5)?,
                brand_ref: row.get(6)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
        });

        match res {
            Ok(p) => Ok(Some(p)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StudioError::Internal {
                message: e.to_string(),
                correlation_id: id.into(),
            }),
        }
    }

    pub fn list_projects(&self, workspace_id: &str) -> Result<Vec<Project>, StudioError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, workspace_id, title, project_type, status, manifest_version, brand_ref, created_at, updated_at
             FROM projects WHERE workspace_id = ?1 AND status != 'deleted' ORDER BY updated_at DESC",
        ).map_err(|e| StudioError::Internal { message: e.to_string(), correlation_id: workspace_id.into() })?;

        let rows = stmt
            .query_map(params![workspace_id], |row| {
                let p_type_str: String = row.get(3)?;
                let status_str: String = row.get(4)?;
                let created_at_str: String = row.get(7)?;
                let updated_at_str: String = row.get(8)?;

                Ok(Project {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    title: row.get(2)?,
                    project_type: serde_json::from_str(&format!("\"{}\"", p_type_str))
                        .unwrap_or(ProjectType::Poster),
                    status: serde_json::from_str(&format!("\"{}\"", status_str))
                        .unwrap_or(ProjectStatus::Active),
                    manifest_version: row.get(5)?,
                    brand_ref: row.get(6)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                })
            })
            .map_err(|e| StudioError::Internal {
                message: e.to_string(),
                correlation_id: workspace_id.into(),
            })?;

        let mut list = Vec::new();
        for p in rows.flatten() {
            list.push(p);
        }
        Ok(list)
    }

    // Revision & Creative Document with CAS Check
    pub fn save_revision_and_document(
        &self,
        project_id: &str,
        expected_revision_id: Option<&str>,
        document: &CreativeDocument,
        author_actor_id: &str,
        summary: &str,
    ) -> Result<ProjectRevision, StudioError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction().map_err(|e| StudioError::Internal {
            message: e.to_string(),
            correlation_id: project_id.into(),
        })?;

        // 1. Check current latest revision
        let current_rev: Option<String> = {
            let mut stmt = tx.prepare(
                "SELECT id FROM project_revisions WHERE project_id = ?1 ORDER BY created_at DESC LIMIT 1"
            ).map_err(|e| StudioError::Internal { message: e.to_string(), correlation_id: project_id.into() })?;
            stmt.query_row(params![project_id], |r| r.get(0)).ok()
        };

        // 2. CAS verification
        if let Some(expected) = expected_revision_id {
            let actual = current_rev.as_deref().unwrap_or("");
            if actual != expected {
                return Err(StudioError::ConflictCas {
                    expected: expected.to_string(),
                    current: actual.to_string(),
                    correlation_id: project_id.into(),
                });
            }
        }

        // 3. Insert new revision and document
        let new_rev_id = uuid::Uuid::new_v4().to_string();
        let source_hash = document.compute_hash();
        let now = chrono::Utc::now();
        let doc_json = serde_json::to_string(document).unwrap_or_default();

        tx.execute(
            "INSERT INTO creative_documents (id, project_id, revision_id, document_json, source_hash, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                document.id,
                project_id,
                new_rev_id,
                doc_json,
                source_hash,
                now.to_rfc3339(),
            ],
        ).map_err(|e| StudioError::Internal { message: e.to_string(), correlation_id: project_id.into() })?;

        tx.execute(
            "INSERT INTO project_revisions (id, project_id, parent_id, source_hash, author_actor_id, document_id, summary, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                new_rev_id,
                project_id,
                current_rev,
                source_hash,
                author_actor_id,
                document.id,
                summary,
                now.to_rfc3339(),
            ],
        ).map_err(|e| StudioError::Internal { message: e.to_string(), correlation_id: project_id.into() })?;

        // 4. Update project updated_at
        tx.execute(
            "UPDATE projects SET updated_at = ?1 WHERE id = ?2",
            params![now.to_rfc3339(), project_id],
        )
        .map_err(|e| StudioError::Internal {
            message: e.to_string(),
            correlation_id: project_id.into(),
        })?;

        tx.commit().map_err(|e| StudioError::Internal {
            message: e.to_string(),
            correlation_id: project_id.into(),
        })?;

        Ok(ProjectRevision {
            id: new_rev_id,
            project_id: project_id.to_string(),
            parent_id: current_rev,
            source_hash,
            author_actor_id: author_actor_id.to_string(),
            document_id: document.id.clone(),
            created_at: now,
            summary: summary.to_string(),
        })
    }

    pub fn get_latest_revision(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectRevision>, StudioError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, parent_id, source_hash, author_actor_id, document_id, summary, created_at
             FROM project_revisions WHERE project_id = ?1 ORDER BY created_at DESC LIMIT 1"
        ).map_err(|e| StudioError::Internal { message: e.to_string(), correlation_id: project_id.into() })?;

        let res = stmt.query_row(params![project_id], |row| {
            let created_at_str: String = row.get(7)?;
            Ok(ProjectRevision {
                id: row.get(0)?,
                project_id: row.get(1)?,
                parent_id: row.get(2)?,
                source_hash: row.get(3)?,
                author_actor_id: row.get(4)?,
                document_id: row.get(5)?,
                summary: row.get(6)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
        });

        match res {
            Ok(rev) => Ok(Some(rev)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StudioError::Internal {
                message: e.to_string(),
                correlation_id: project_id.into(),
            }),
        }
    }

    pub fn get_document_by_revision(
        &self,
        revision_id: &str,
    ) -> Result<Option<CreativeDocument>, StudioError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT document_json FROM creative_documents WHERE revision_id = ?1")
            .map_err(|e| StudioError::Internal {
                message: e.to_string(),
                correlation_id: revision_id.into(),
            })?;

        let res = stmt.query_row(params![revision_id], |row| {
            let json_str: String = row.get(0)?;
            Ok(json_str)
        });

        match res {
            Ok(json_str) => {
                let doc: CreativeDocument =
                    serde_json::from_str(&json_str).map_err(|e| StudioError::Internal {
                        message: format!("Document json parse error: {}", e),
                        correlation_id: revision_id.into(),
                    })?;
                Ok(Some(doc))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StudioError::Internal {
                message: e.to_string(),
                correlation_id: revision_id.into(),
            }),
        }
    }

    pub fn get_document_by_revision_scoped(
        &self,
        workspace_id: &str,
        project_id: &str,
        revision_id: &str,
    ) -> Result<Option<CreativeDocument>, StudioError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT cd.document_json 
                 FROM creative_documents cd 
                 JOIN projects p ON cd.project_id = p.id 
                 WHERE cd.revision_id = ?1 AND cd.project_id = ?2 AND p.workspace_id = ?3",
            )
            .map_err(|e| StudioError::Internal {
                message: e.to_string(),
                correlation_id: revision_id.into(),
            })?;

        let res = stmt.query_row(params![revision_id, project_id, workspace_id], |row| {
            let json_str: String = row.get(0)?;
            Ok(json_str)
        });

        match res {
            Ok(json_str) => {
                let doc: CreativeDocument =
                    serde_json::from_str(&json_str).map_err(|e| StudioError::Internal {
                        message: format!("Document json parse error: {}", e),
                        correlation_id: revision_id.into(),
                    })?;
                Ok(Some(doc))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StudioError::Internal {
                message: e.to_string(),
                correlation_id: revision_id.into(),
            }),
        }
    }

    // Assets
    pub fn save_asset(&self, asset: &Asset) -> Result<(), StudioError> {
        let conn = self.conn.lock().unwrap();
        let attestation_json = asset
            .rights_attestation
            .as_ref()
            .map(|a| serde_json::to_string(a).unwrap());
        conn.execute(
            "INSERT INTO assets (id, scope, owner_workspace_id, kind, original_name, declared_mime, verified_mime, blob_sha256, file_size_bytes, provenance, rights_state, rights_attestation, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                asset.id,
                serde_json::to_string(&asset.scope).unwrap().trim_matches('"'),
                asset.owner_workspace_id,
                serde_json::to_string(&asset.kind).unwrap().trim_matches('"'),
                asset.original_name,
                asset.declared_mime,
                asset.verified_mime,
                asset.blob_sha256,
                asset.file_size_bytes,
                asset.provenance,
                serde_json::to_string(&asset.rights_state).unwrap().trim_matches('"'),
                attestation_json,
                asset.created_at.to_rfc3339(),
            ],
        ).map_err(|e| StudioError::Internal { message: e.to_string(), correlation_id: asset.id.clone() })?;
        Ok(())
    }

    pub fn get_asset(&self, id: &str) -> Result<Option<Asset>, StudioError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, scope, owner_workspace_id, kind, original_name, declared_mime, verified_mime, blob_sha256, file_size_bytes, provenance, rights_state, rights_attestation, created_at
             FROM assets WHERE id = ?1",
        ).map_err(|e| StudioError::Internal { message: e.to_string(), correlation_id: id.into() })?;

        let res = stmt.query_row(params![id], |row| {
            let scope_str: String = row.get(1)?;
            let kind_str: String = row.get(3)?;
            let rights_str: String = row.get(10)?;
            let attestation_raw: Option<String> = row.get(11)?;
            let created_at_str: String = row.get(12)?;

            let attestation: Option<RightsAttestation> =
                attestation_raw.and_then(|s| serde_json::from_str(&s).ok());

            Ok(Asset {
                id: row.get(0)?,
                scope: serde_json::from_str(&format!("\"{}\"", scope_str))
                    .unwrap_or(AssetScope::Workspace),
                owner_workspace_id: row.get(2)?,
                kind: serde_json::from_str(&format!("\"{}\"", kind_str))
                    .unwrap_or(AssetKind::Image),
                original_name: row.get(4)?,
                declared_mime: row.get(5)?,
                verified_mime: row.get(6)?,
                blob_sha256: row.get(7)?,
                file_size_bytes: row.get(8)?,
                provenance: row.get(9)?,
                rights_state: serde_json::from_str(&format!("\"{}\"", rights_str))
                    .unwrap_or(RightsState::Quarantined),
                rights_attestation: attestation,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
        });

        match res {
            Ok(a) => Ok(Some(a)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StudioError::Internal {
                message: e.to_string(),
                correlation_id: id.into(),
            }),
        }
    }

    pub fn get_asset_scoped(
        &self,
        workspace_id: &str,
        id: &str,
    ) -> Result<Option<Asset>, StudioError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, scope, owner_workspace_id, kind, original_name, declared_mime, verified_mime, blob_sha256, file_size_bytes, provenance, rights_state, rights_attestation, created_at
             FROM assets WHERE id = ?1 AND (scope = 'global' OR owner_workspace_id = ?2)",
        ).map_err(|e| StudioError::Internal { message: e.to_string(), correlation_id: id.into() })?;

        let res = stmt.query_row(params![id, workspace_id], |row| {
            let scope_str: String = row.get(1)?;
            let kind_str: String = row.get(3)?;
            let rights_str: String = row.get(10)?;
            let attestation_raw: Option<String> = row.get(11)?;
            let created_at_str: String = row.get(12)?;

            let attestation: Option<RightsAttestation> =
                attestation_raw.and_then(|s| serde_json::from_str(&s).ok());

            Ok(Asset {
                id: row.get(0)?,
                scope: serde_json::from_str(&format!("\"{}\"", scope_str))
                    .unwrap_or(AssetScope::Workspace),
                owner_workspace_id: row.get(2)?,
                kind: serde_json::from_str(&format!("\"{}\"", kind_str))
                    .unwrap_or(AssetKind::Image),
                original_name: row.get(4)?,
                declared_mime: row.get(5)?,
                verified_mime: row.get(6)?,
                blob_sha256: row.get(7)?,
                file_size_bytes: row.get(8)?,
                provenance: row.get(9)?,
                rights_state: serde_json::from_str(&format!("\"{}\"", rights_str))
                    .unwrap_or(RightsState::Quarantined),
                rights_attestation: attestation,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
        });

        match res {
            Ok(a) => Ok(Some(a)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StudioError::Internal {
                message: e.to_string(),
                correlation_id: id.into(),
            }),
        }
    }

    pub fn list_assets(&self, workspace_id: &str) -> Result<Vec<Asset>, StudioError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, scope, owner_workspace_id, kind, original_name, declared_mime, verified_mime, blob_sha256, file_size_bytes, provenance, rights_state, rights_attestation, created_at
             FROM assets WHERE owner_workspace_id = ?1 OR scope = 'global' ORDER BY created_at DESC",
        ).map_err(|e| StudioError::Internal { message: e.to_string(), correlation_id: workspace_id.into() })?;

        let rows = stmt
            .query_map(params![workspace_id], |row| {
                let scope_str: String = row.get(1)?;
                let kind_str: String = row.get(3)?;
                let rights_str: String = row.get(10)?;
                let attestation_raw: Option<String> = row.get(11)?;
                let created_at_str: String = row.get(12)?;
                let attestation: Option<RightsAttestation> =
                    attestation_raw.and_then(|s| serde_json::from_str(&s).ok());

                Ok(Asset {
                    id: row.get(0)?,
                    scope: serde_json::from_str(&format!("\"{}\"", scope_str))
                        .unwrap_or(AssetScope::Workspace),
                    owner_workspace_id: row.get(2)?,
                    kind: serde_json::from_str(&format!("\"{}\"", kind_str))
                        .unwrap_or(AssetKind::Image),
                    original_name: row.get(4)?,
                    declared_mime: row.get(5)?,
                    verified_mime: row.get(6)?,
                    blob_sha256: row.get(7)?,
                    file_size_bytes: row.get(8)?,
                    provenance: row.get(9)?,
                    rights_state: serde_json::from_str(&format!("\"{}\"", rights_str))
                        .unwrap_or(RightsState::Quarantined),
                    rights_attestation: attestation,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                })
            })
            .map_err(|e| StudioError::Internal {
                message: e.to_string(),
                correlation_id: workspace_id.into(),
            })?;

        let mut list = Vec::new();
        for a in rows.flatten() {
            list.push(a);
        }
        Ok(list)
    }

    pub fn update_asset_rights(
        &self,
        asset_id: &str,
        state: RightsState,
        attestation: Option<RightsAttestation>,
    ) -> Result<(), StudioError> {
        let conn = self.conn.lock().unwrap();
        let attestation_json = attestation
            .as_ref()
            .map(|a| serde_json::to_string(a).unwrap());
        let state_str = serde_json::to_string(&state)
            .unwrap()
            .trim_matches('"')
            .to_string();

        conn.execute(
            "UPDATE assets SET rights_state = ?1, rights_attestation = ?2 WHERE id = ?3",
            params![state_str, attestation_json, asset_id],
        )
        .map_err(|e| StudioError::Internal {
            message: e.to_string(),
            correlation_id: asset_id.into(),
        })?;

        Ok(())
    }

    pub fn delete_asset(&self, asset_id: &str) -> Result<(), StudioError> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM assets WHERE id = ?1", params![asset_id])
            .map_err(|e| StudioError::Internal {
                message: e.to_string(),
                correlation_id: asset_id.into(),
            })?;
        Ok(())
    }

    // Research Sources
    pub fn save_research_source(&self, source: &ResearchSource) -> Result<(), StudioError> {
        let conn = self.conn.lock().unwrap();
        let snapshot_json = source
            .snapshot
            .as_ref()
            .map(|s| serde_json::to_string(s).unwrap());

        conn.execute(
            "INSERT INTO research_sources (id, workspace_id, canonical_url, domain, fetch_status, snapshot_json, fetched_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                fetch_status = excluded.fetch_status,
                snapshot_json = excluded.snapshot_json,
                fetched_at = excluded.fetched_at",
            params![
                source.id,
                source.workspace_id,
                source.canonical_url,
                source.domain,
                source.fetch_status,
                snapshot_json,
                source.fetched_at.to_rfc3339(),
            ],
        ).map_err(|e| StudioError::Internal {
            message: e.to_string(),
            correlation_id: source.id.clone(),
        })?;

        Ok(())
    }

    pub fn get_research_source(&self, id: &str) -> Result<Option<ResearchSource>, StudioError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, workspace_id, canonical_url, domain, fetch_status, snapshot_json, fetched_at
                 FROM research_sources WHERE id = ?1",
            )
            .map_err(|e| StudioError::Internal {
                message: e.to_string(),
                correlation_id: id.into(),
            })?;

        let res = stmt.query_row(params![id], |row| {
            let snapshot_raw: Option<String> = row.get(5)?;
            let fetched_at_str: String = row.get(6)?;
            let snapshot: Option<ResearchSnapshot> =
                snapshot_raw.and_then(|s| serde_json::from_str(&s).ok());

            Ok(ResearchSource {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                canonical_url: row.get(2)?,
                domain: row.get(3)?,
                fetch_status: row.get(4)?,
                snapshot,
                fetched_at: chrono::DateTime::parse_from_rfc3339(&fetched_at_str)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
        });

        match res {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StudioError::Internal {
                message: e.to_string(),
                correlation_id: id.into(),
            }),
        }
    }

    pub fn list_research_sources(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<ResearchSource>, StudioError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, workspace_id, canonical_url, domain, fetch_status, snapshot_json, fetched_at
                 FROM research_sources WHERE workspace_id = ?1 ORDER BY fetched_at DESC",
            )
            .map_err(|e| StudioError::Internal {
                message: e.to_string(),
                correlation_id: workspace_id.into(),
            })?;

        let rows = stmt
            .query_map(params![workspace_id], |row| {
                let snapshot_raw: Option<String> = row.get(5)?;
                let fetched_at_str: String = row.get(6)?;
                let snapshot: Option<ResearchSnapshot> =
                    snapshot_raw.and_then(|s| serde_json::from_str(&s).ok());

                Ok(ResearchSource {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    canonical_url: row.get(2)?,
                    domain: row.get(3)?,
                    fetch_status: row.get(4)?,
                    snapshot,
                    fetched_at: chrono::DateTime::parse_from_rfc3339(&fetched_at_str)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                })
            })
            .map_err(|e| StudioError::Internal {
                message: e.to_string(),
                correlation_id: workspace_id.into(),
            })?;

        let mut list = Vec::new();
        for r in rows.flatten() {
            list.push(r);
        }
        Ok(list)
    }

    // Brand Kits & Versions
    pub fn save_brand_kit(&self, kit: &BrandKit) -> Result<(), StudioError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO brand_kits (id, workspace_id, name, active_version_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                active_version_id = excluded.active_version_id",
            params![
                kit.id,
                kit.workspace_id,
                kit.name,
                kit.active_version_id,
                kit.created_at.to_rfc3339(),
            ],
        )
        .map_err(|e| StudioError::Internal {
            message: e.to_string(),
            correlation_id: kit.id.clone(),
        })?;
        Ok(())
    }

    pub fn get_brand_kit(&self, id: &str) -> Result<Option<BrandKit>, StudioError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, workspace_id, name, active_version_id, created_at FROM brand_kits WHERE id = ?1")
            .map_err(|e| StudioError::Internal { message: e.to_string(), correlation_id: id.into() })?;

        let res = stmt.query_row(params![id], |row| {
            let created_at_str: String = row.get(4)?;
            Ok(BrandKit {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                name: row.get(2)?,
                active_version_id: row.get(3)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
        });

        match res {
            Ok(k) => Ok(Some(k)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StudioError::Internal {
                message: e.to_string(),
                correlation_id: id.into(),
            }),
        }
    }

    pub fn list_brand_kits(&self, workspace_id: &str) -> Result<Vec<BrandKit>, StudioError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, workspace_id, name, active_version_id, created_at FROM brand_kits WHERE workspace_id = ?1 ORDER BY created_at DESC")
            .map_err(|e| StudioError::Internal { message: e.to_string(), correlation_id: workspace_id.into() })?;

        let rows = stmt
            .query_map(params![workspace_id], |row| {
                let created_at_str: String = row.get(4)?;
                Ok(BrandKit {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    name: row.get(2)?,
                    active_version_id: row.get(3)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                })
            })
            .map_err(|e| StudioError::Internal {
                message: e.to_string(),
                correlation_id: workspace_id.into(),
            })?;

        let mut list = Vec::new();
        for k in rows.flatten() {
            list.push(k);
        }
        Ok(list)
    }

    pub fn save_brand_version(&self, ver: &BrandVersion) -> Result<(), StudioError> {
        let conn = self.conn.lock().unwrap();
        let tokens_json = serde_json::to_string(&ver.visual_tokens).unwrap();
        let voice_json = serde_json::to_string(&ver.voice).unwrap();
        let logos_json = serde_json::to_string(&ver.logos).unwrap();
        let state_str = serde_json::to_string(&ver.state)
            .unwrap()
            .trim_matches('"')
            .to_string();

        conn.execute(
            "INSERT INTO brand_versions (id, brand_kit_id, version_number, state, visual_tokens, voice_guidelines, logos, provenance, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
                state = excluded.state",
            params![
                ver.id,
                ver.brand_kit_id,
                ver.version_number,
                state_str,
                tokens_json,
                voice_json,
                logos_json,
                ver.provenance,
                ver.created_at.to_rfc3339(),
            ],
        ).map_err(|e| StudioError::Internal { message: e.to_string(), correlation_id: ver.id.clone() })?;
        Ok(())
    }

    pub fn get_brand_version(&self, id: &str) -> Result<Option<BrandVersion>, StudioError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, brand_kit_id, version_number, state, visual_tokens, voice_guidelines, logos, provenance, created_at
                 FROM brand_versions WHERE id = ?1",
            )
            .map_err(|e| StudioError::Internal { message: e.to_string(), correlation_id: id.into() })?;

        let res = stmt.query_row(params![id], |row| {
            let state_str: String = row.get(3)?;
            let tokens_raw: String = row.get(4)?;
            let voice_raw: String = row.get(5)?;
            let logos_raw: String = row.get(6)?;
            let created_at_str: String = row.get(8)?;

            let tokens: VisualTokens = serde_json::from_str(&tokens_raw).unwrap();
            let voice: VoiceGuidelines = serde_json::from_str(&voice_raw).unwrap();
            let logos: Vec<String> = serde_json::from_str(&logos_raw).unwrap();

            Ok(BrandVersion {
                id: row.get(0)?,
                brand_kit_id: row.get(1)?,
                version_number: row.get(2)?,
                state: serde_json::from_str(&format!("\"{}\"", state_str))
                    .unwrap_or(BrandVersionState::Draft),
                visual_tokens: tokens,
                voice,
                logos,
                provenance: row.get(7)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
        });

        match res {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StudioError::Internal {
                message: e.to_string(),
                correlation_id: id.into(),
            }),
        }
    }

    pub fn list_brand_versions(
        &self,
        brand_kit_id: &str,
    ) -> Result<Vec<BrandVersion>, StudioError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, brand_kit_id, version_number, state, visual_tokens, voice_guidelines, logos, provenance, created_at
                 FROM brand_versions WHERE brand_kit_id = ?1 ORDER BY version_number ASC",
            )
            .map_err(|e| StudioError::Internal { message: e.to_string(), correlation_id: brand_kit_id.into() })?;

        let rows = stmt
            .query_map(params![brand_kit_id], |row| {
                let state_str: String = row.get(3)?;
                let tokens_raw: String = row.get(4)?;
                let voice_raw: String = row.get(5)?;
                let logos_raw: String = row.get(6)?;
                let created_at_str: String = row.get(8)?;

                let tokens: VisualTokens = serde_json::from_str(&tokens_raw).unwrap();
                let voice: VoiceGuidelines = serde_json::from_str(&voice_raw).unwrap();
                let logos: Vec<String> = serde_json::from_str(&logos_raw).unwrap();

                Ok(BrandVersion {
                    id: row.get(0)?,
                    brand_kit_id: row.get(1)?,
                    version_number: row.get(2)?,
                    state: serde_json::from_str(&format!("\"{}\"", state_str))
                        .unwrap_or(BrandVersionState::Draft),
                    visual_tokens: tokens,
                    voice,
                    logos,
                    provenance: row.get(7)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                })
            })
            .map_err(|e| StudioError::Internal {
                message: e.to_string(),
                correlation_id: brand_kit_id.into(),
            })?;

        let mut list = Vec::new();
        for v in rows.flatten() {
            list.push(v);
        }
        Ok(list)
    }

    // Render Jobs
    pub fn save_render_job(&self, job: &RenderJob) -> Result<(), StudioError> {
        let conn = self.conn.lock().unwrap();
        let state_str = serde_json::to_string(&job.state)
            .unwrap()
            .trim_matches('"')
            .to_string();

        conn.execute(
            "INSERT INTO render_jobs (id, workspace_id, project_id, revision_id, state, progress_percent, input_snapshot_hash, output_asset_ref, output_hash, error_message, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(id) DO UPDATE SET
                state = excluded.state,
                progress_percent = excluded.progress_percent,
                output_asset_ref = excluded.output_asset_ref,
                output_hash = excluded.output_hash,
                error_message = excluded.error_message,
                updated_at = excluded.updated_at",
            params![
                job.id,
                job.workspace_id,
                job.project_id,
                job.revision_id,
                state_str,
                job.progress_percent,
                job.input_snapshot_hash,
                job.output_asset_ref,
                job.output_hash,
                job.error_message,
                job.created_at.to_rfc3339(),
                job.updated_at.to_rfc3339(),
            ],
        ).map_err(|e| StudioError::Internal { message: e.to_string(), correlation_id: job.id.clone() })?;

        Ok(())
    }

    pub fn get_render_job(&self, id: &str) -> Result<Option<RenderJob>, StudioError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, workspace_id, project_id, revision_id, state, progress_percent, input_snapshot_hash, output_asset_ref, output_hash, error_message, created_at, updated_at
                 FROM render_jobs WHERE id = ?1",
            )
            .map_err(|e| StudioError::Internal { message: e.to_string(), correlation_id: id.into() })?;

        let res = stmt.query_row(params![id], |row| {
            let state_str: String = row.get(4)?;
            let created_at_str: String = row.get(10)?;
            let updated_at_str: String = row.get(11)?;

            Ok(RenderJob {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                revision_id: row.get(3)?,
                state: serde_json::from_str(&format!("\"{}\"", state_str))
                    .unwrap_or(RenderJobState::Pending),
                progress_percent: row.get(5)?,
                input_snapshot_hash: row.get(6)?,
                output_asset_ref: row.get(7)?,
                output_hash: row.get(8)?,
                error_message: row.get(9)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
        });

        match res {
            Ok(j) => Ok(Some(j)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StudioError::Internal {
                message: e.to_string(),
                correlation_id: id.into(),
            }),
        }
    }

    pub fn list_render_jobs(&self, workspace_id: &str) -> Result<Vec<RenderJob>, StudioError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, workspace_id, project_id, revision_id, state, progress_percent, input_snapshot_hash, output_asset_ref, output_hash, error_message, created_at, updated_at
                 FROM render_jobs WHERE workspace_id = ?1 ORDER BY created_at DESC",
            )
            .map_err(|e| StudioError::Internal { message: e.to_string(), correlation_id: workspace_id.into() })?;

        let rows = stmt
            .query_map(params![workspace_id], |row| {
                let state_str: String = row.get(4)?;
                let created_at_str: String = row.get(10)?;
                let updated_at_str: String = row.get(11)?;

                Ok(RenderJob {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    project_id: row.get(2)?,
                    revision_id: row.get(3)?,
                    state: serde_json::from_str(&format!("\"{}\"", state_str))
                        .unwrap_or(RenderJobState::Pending),
                    progress_percent: row.get(5)?,
                    input_snapshot_hash: row.get(6)?,
                    output_asset_ref: row.get(7)?,
                    output_hash: row.get(8)?,
                    error_message: row.get(9)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                })
            })
            .map_err(|e| StudioError::Internal {
                message: e.to_string(),
                correlation_id: workspace_id.into(),
            })?;

        let mut list = Vec::new();
        for j in rows.flatten() {
            list.push(j);
        }
        Ok(list)
    }

    // Audit Logging
    #[allow(clippy::too_many_arguments)]
    pub fn record_audit(
        &self,
        actor_id: &str,
        actor_role: ActorRole,
        workspace_id: &str,
        operation: &str,
        side_effect_class: &str,
        aggregate_id: Option<&str>,
        details: Option<&serde_json::Value>,
    ) -> Result<String, StudioError> {
        let conn = self.conn.lock().unwrap();
        let audit_id = uuid::Uuid::new_v4().to_string();
        let details_str = details.map(|d| serde_json::to_string(d).unwrap_or_default());
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO audit_logs (id, actor_id, actor_role, workspace_id, operation, side_effect_class, aggregate_id, details, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                audit_id,
                actor_id,
                serde_json::to_string(&actor_role).unwrap().trim_matches('"'),
                workspace_id,
                operation,
                side_effect_class,
                aggregate_id,
                details_str,
                now,
            ],
        ).map_err(|e| StudioError::Internal { message: e.to_string(), correlation_id: audit_id.clone() })?;

        Ok(audit_id)
    }

    // Delivery Packages
    pub fn save_delivery_package(
        &self,
        pkg: &studio_core::handoff::DeliveryPackage,
    ) -> Result<(), StudioError> {
        let conn = self.conn.lock().unwrap();
        let variant_json = serde_json::to_string(&pkg.variant).unwrap_or_default();
        let media_json = serde_json::to_string(&pkg.media).unwrap_or_default();
        let targets_json = serde_json::to_string(&pkg.target_selectors).unwrap_or_default();
        let status_str = serde_json::to_string(&pkg.status)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();

        conn.execute(
            "INSERT INTO delivery_packages (id, studio_workspace_id, project_id, source_revision_id, source_hash, variant_json, media_json, target_selectors_json, idempotency_key, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(id) DO UPDATE SET
                status = excluded.status",
            params![
                pkg.delivery_id,
                pkg.studio_workspace_id,
                pkg.project_id,
                pkg.source_revision_id,
                pkg.source_hash,
                variant_json,
                media_json,
                targets_json,
                pkg.idempotency_key,
                status_str,
                pkg.created_at.to_rfc3339(),
            ],
        ).map_err(|e| StudioError::Internal { message: e.to_string(), correlation_id: pkg.delivery_id.clone() })?;

        Ok(())
    }

    pub fn get_delivery_package(
        &self,
        id: &str,
    ) -> Result<Option<studio_core::handoff::DeliveryPackage>, StudioError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, studio_workspace_id, project_id, source_revision_id, source_hash, variant_json, media_json, target_selectors_json, idempotency_key, status, created_at
             FROM delivery_packages WHERE id = ?1"
        ).map_err(|e| StudioError::Internal { message: e.to_string(), correlation_id: id.into() })?;

        let res = stmt.query_row(params![id], |row| {
            let var_str: String = row.get(5)?;
            let media_str: String = row.get(6)?;
            let targets_str: String = row.get(7)?;
            let status_str: String = row.get(9)?;
            let created_at_str: String = row.get(10)?;

            let variant: studio_core::handoff::DeliveryVariant =
                serde_json::from_str(&var_str).unwrap();
            let media: Vec<studio_core::handoff::DeliveryMediaItem> =
                serde_json::from_str(&media_str).unwrap();
            let targets: studio_core::handoff::DeliveryTargetSelectors =
                serde_json::from_str(&targets_str).unwrap();

            Ok(studio_core::handoff::DeliveryPackage {
                schema_version: studio_core::handoff::DeliveryPackage::SCHEMA_V1.to_string(),
                delivery_id: row.get(0)?,
                studio_workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                source_revision_id: row.get(3)?,
                source_hash: row.get(4)?,
                variant,
                media,
                target_selectors: targets,
                idempotency_key: row.get(8)?,
                status: serde_json::from_str(&format!("\"{}\"", status_str))
                    .unwrap_or(studio_core::handoff::HandoffStatus::Prepared),
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
        });

        match res {
            Ok(p) => Ok(Some(p)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StudioError::Internal {
                message: e.to_string(),
                correlation_id: id.into(),
            }),
        }
    }

    pub fn update_delivery_status(
        &self,
        delivery_id: &str,
        status: studio_core::handoff::HandoffStatus,
    ) -> Result<(), StudioError> {
        let conn = self.conn.lock().unwrap();
        let status_str = serde_json::to_string(&status)
            .unwrap()
            .trim_matches('"')
            .to_string();

        conn.execute(
            "UPDATE delivery_packages SET status = ?1 WHERE id = ?2",
            params![status_str, delivery_id],
        )
        .map_err(|e| StudioError::Internal {
            message: e.to_string(),
            correlation_id: delivery_id.into(),
        })?;

        Ok(())
    }

    // Outbox Operations
    pub fn save_outbox_item(
        &self,
        item: &studio_core::handoff::OutboxItem,
    ) -> Result<(), StudioError> {
        let conn = self.conn.lock().unwrap();
        let state_str = serde_json::to_string(&item.state)
            .unwrap()
            .trim_matches('"')
            .to_string();

        conn.execute(
            "INSERT INTO outbox (id, destination, payload_hash, idempotency_key, state, remote_ref, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                item.id,
                item.destination,
                item.payload_hash,
                item.idempotency_key,
                state_str,
                item.remote_ref,
                item.created_at.to_rfc3339(),
            ],
        ).map_err(|e| StudioError::Internal { message: e.to_string(), correlation_id: item.id.clone() })?;

        Ok(())
    }

    pub fn get_outbox_item(
        &self,
        id: &str,
    ) -> Result<Option<studio_core::handoff::OutboxItem>, StudioError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, destination, payload_hash, idempotency_key, state, remote_ref, created_at
             FROM outbox WHERE id = ?1"
        ).map_err(|e| StudioError::Internal { message: e.to_string(), correlation_id: id.into() })?;

        let res = stmt.query_row(params![id], |row| {
            let state_str: String = row.get(4)?;
            let created_at_str: String = row.get(6)?;

            Ok(studio_core::handoff::OutboxItem {
                id: row.get(0)?,
                destination: row.get(1)?,
                payload_hash: row.get(2)?,
                idempotency_key: row.get(3)?,
                state: serde_json::from_str(&format!("\"{}\"", state_str))
                    .unwrap_or(studio_core::handoff::OutboxState::Pending),
                remote_ref: row.get(5)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
        });

        match res {
            Ok(item) => Ok(Some(item)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StudioError::Internal {
                message: e.to_string(),
                correlation_id: id.into(),
            }),
        }
    }

    pub fn update_outbox_state(
        &self,
        id: &str,
        state: studio_core::handoff::OutboxState,
        remote_ref: Option<&str>,
    ) -> Result<(), StudioError> {
        let conn = self.conn.lock().unwrap();
        let state_str = serde_json::to_string(&state)
            .unwrap()
            .trim_matches('"')
            .to_string();

        conn.execute(
            "UPDATE outbox SET state = ?1, remote_ref = ?2 WHERE id = ?3",
            params![state_str, remote_ref, id],
        )
        .map_err(|e| StudioError::Internal {
            message: e.to_string(),
            correlation_id: id.into(),
        })?;

        Ok(())
    }

    pub fn get_delivery_package_scoped(
        &self,
        workspace_id: &str,
        delivery_id: &str,
    ) -> Result<Option<studio_core::handoff::DeliveryPackage>, StudioError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, studio_workspace_id, project_id, source_revision_id, source_hash, variant_json, media_json, target_selectors_json, idempotency_key, status, created_at
                 FROM delivery_packages WHERE id = ?1 AND studio_workspace_id = ?2",
            )
            .map_err(|e| StudioError::Internal {
                message: e.to_string(),
                correlation_id: delivery_id.into(),
            })?;

        let res = stmt.query_row(params![delivery_id, workspace_id], |row| {
            let var_str: String = row.get(5)?;
            let media_str: String = row.get(6)?;
            let targets_str: String = row.get(7)?;
            let status_str: String = row.get(9)?;
            let created_at_str: String = row.get(10)?;

            let variant: studio_core::handoff::DeliveryVariant =
                serde_json::from_str(&var_str).unwrap();
            let media: Vec<studio_core::handoff::DeliveryMediaItem> =
                serde_json::from_str(&media_str).unwrap();
            let targets: studio_core::handoff::DeliveryTargetSelectors =
                serde_json::from_str(&targets_str).unwrap();

            Ok(studio_core::handoff::DeliveryPackage {
                schema_version: studio_core::handoff::DeliveryPackage::SCHEMA_V1.to_string(),
                delivery_id: row.get(0)?,
                studio_workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                source_revision_id: row.get(3)?,
                source_hash: row.get(4)?,
                variant,
                media,
                target_selectors: targets,
                idempotency_key: row.get(8)?,
                status: serde_json::from_str(&format!("\"{}\"", status_str))
                    .unwrap_or(studio_core::handoff::HandoffStatus::Prepared),
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
        });

        match res {
            Ok(p) => Ok(Some(p)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StudioError::Internal {
                message: e.to_string(),
                correlation_id: delivery_id.into(),
            }),
        }
    }

    pub fn get_outbox_by_idempotency_key(
        &self,
        destination: &str,
        idempotency_key: &str,
    ) -> Result<Option<studio_core::handoff::OutboxItem>, StudioError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, destination, payload_hash, idempotency_key, state, remote_ref, created_at
                 FROM outbox WHERE destination = ?1 AND idempotency_key = ?2",
            )
            .map_err(|e| StudioError::Internal {
                message: e.to_string(),
                correlation_id: idempotency_key.into(),
            })?;

        let res = stmt.query_row(params![destination, idempotency_key], |row| {
            let state_str: String = row.get(4)?;
            let created_at_str: String = row.get(6)?;

            Ok(studio_core::handoff::OutboxItem {
                id: row.get(0)?,
                destination: row.get(1)?,
                payload_hash: row.get(2)?,
                idempotency_key: row.get(3)?,
                state: serde_json::from_str(&format!("\"{}\"", state_str))
                    .unwrap_or(studio_core::handoff::OutboxState::Pending),
                remote_ref: row.get(5)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
        });

        match res {
            Ok(item) => Ok(Some(item)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StudioError::Internal {
                message: e.to_string(),
                correlation_id: idempotency_key.into(),
            }),
        }
    }
}
