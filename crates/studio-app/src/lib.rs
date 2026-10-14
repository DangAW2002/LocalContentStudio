pub mod ai_gateway;
pub mod brand_validator;
pub mod dsh_migration;
pub mod plugin_host;
pub mod research_extractor;
pub mod research_validator;
pub mod social_gateway;
pub mod static_export;

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use studio_core::{
    actor::{ActorContext, ActorRole},
    asset::{Asset, AssetKind, AssetScope, RightsAttestation, RightsState},
    brand::{BrandKit, BrandVersion, BrandVersionState, VisualTokens, VoiceGuidelines},
    capabilities::CapabilityRegistry,
    document::CreativeDocument,
    error::StudioError,
    operation::{OperationEnvelope, OperationResult},
    preview::{ExportReceipt, PreviewReceipt},
    project::{Project, ProjectRevision, ProjectType},
    research::ResearchSource,
    session::{AuthenticatedSession, HumanConfirmationProof},
};
use studio_storage::{BlobStorage, SqliteCatalog};

#[derive(Clone)]
pub struct ApplicationHost {
    catalog: SqliteCatalog,
    blob_storage: BlobStorage,
    capabilities: Arc<CapabilityRegistry>,
    sessions: Arc<Mutex<HashMap<String, AuthenticatedSession>>>,
    proofs: Arc<Mutex<HashMap<String, HumanConfirmationProof>>>,
}

impl ApplicationHost {
    pub fn new(
        catalog: SqliteCatalog,
        blob_storage: BlobStorage,
        capabilities: CapabilityRegistry,
    ) -> Self {
        Self {
            catalog,
            blob_storage,
            capabilities: Arc::new(capabilities),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            proofs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn capabilities(&self) -> &CapabilityRegistry {
        &self.capabilities
    }

    pub fn compute_action_hash(target_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(target_id.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub fn authenticate_local_os_user(
        &self,
        active_workspace_id: &str,
    ) -> Result<AuthenticatedSession, StudioError> {
        let username = std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "local-operator".to_string());
        let subject_id = format!("os:{}", username);
        let os_profile_id = format!("profile:{}", username);
        let allowed_workspaces = vec![active_workspace_id.to_string(), "default".to_string()];

        let session = AuthenticatedSession::new(
            subject_id,
            os_profile_id,
            active_workspace_id,
            allowed_workspaces,
            ActorRole::Human,
            3600,
        );

        let mut sessions = self.sessions.lock().unwrap();
        sessions.insert(session.session_id.clone(), session.clone());
        Ok(session)
    }

    pub fn issue_human_proof(
        &self,
        session: &AuthenticatedSession,
        operation: &str,
        action_payload_hash: &str,
    ) -> Result<HumanConfirmationProof, StudioError> {
        if session.is_expired() {
            return Err(StudioError::Unauthorized {
                correlation_id: session.subject_id.clone(),
            });
        }
        if session.role != ActorRole::Human {
            return Err(StudioError::ForbiddenHumanOnly {
                operation: operation.to_string(),
                correlation_id: session.subject_id.clone(),
            });
        }
        if !session.can_access_workspace(&session.active_workspace_id) {
            return Err(StudioError::ForbiddenScope {
                resource_id: session.active_workspace_id.clone(),
                correlation_id: session.subject_id.clone(),
            });
        }

        let proof = HumanConfirmationProof::new(
            &session.subject_id,
            &session.active_workspace_id,
            operation,
            action_payload_hash,
            300,
        );

        let mut proofs = self.proofs.lock().unwrap();
        proofs.insert(proof.proof_id.clone(), proof.clone());
        Ok(proof)
    }

    // Security Gate: Check ActorContext and Scope
    fn check_scope(
        &self,
        actor: &ActorContext,
        target_workspace_id: &str,
    ) -> Result<(), StudioError> {
        if actor.active_workspace_id != target_workspace_id {
            return Err(StudioError::ForbiddenScope {
                resource_id: target_workspace_id.to_string(),
                correlation_id: actor.actor_id.clone(),
            });
        }
        Ok(())
    }

    // Security Gate: Human-Only Operations
    fn check_human_only(
        &self,
        actor: &ActorContext,
        operation_name: &str,
    ) -> Result<(), StudioError> {
        if self.capabilities.is_human_only(operation_name) && !actor.is_human() {
            return Err(StudioError::ForbiddenHumanOnly {
                operation: operation_name.to_string(),
                correlation_id: actor.actor_id.clone(),
            });
        }
        Ok(())
    }

    pub fn check_human_only_with_proof(
        &self,
        actor: &ActorContext,
        operation_name: &str,
        payload_hash: &str,
        proof_token: Option<&str>,
    ) -> Result<(), StudioError> {
        self.check_human_only(actor, operation_name)?;

        if self.capabilities.is_human_only(operation_name) {
            let token = proof_token.ok_or_else(|| StudioError::HumanConfirmationRequired {
                operation: operation_name.to_string(),
                action_payload_hash: payload_hash.to_string(),
                correlation_id: actor.actor_id.clone(),
            })?;

            let mut proofs = self.proofs.lock().unwrap();
            let proof = proofs
                .get_mut(token)
                .ok_or_else(|| StudioError::PreconditionFailed {
                    reason: format!("Mã xác thực người dùng không tồn tại: {}", token),
                    correlation_id: actor.actor_id.clone(),
                })?;

            if !proof.is_valid_for(
                &actor.actor_id,
                &actor.active_workspace_id,
                operation_name,
                payload_hash,
            ) {
                return Err(StudioError::PreconditionFailed {
                    reason: "Mã xác thực người dùng không hợp lệ, không khớp với workspace/thao tác/payload hoặc đã hết hạn".into(),
                    correlation_id: actor.actor_id.clone(),
                });
            }

            proof.consumed = true;
        }

        Ok(())
    }

    pub fn verify_media_magic_bytes(
        bytes: &[u8],
        declared_mime: &str,
    ) -> Result<String, StudioError> {
        if bytes.len() < 4 {
            return Err(StudioError::InvalidInput {
                field: "bytes".into(),
                reason: "Dữ liệu tệp quá ngắn (dưới 4 bytes), không thể xác minh chữ ký ma thuật"
                    .into(),
                correlation_id: "media_validator".into(),
            });
        }

        let detected_mime = if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A])
        {
            "image/png"
        } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            "image/jpeg"
        } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
            "image/gif"
        } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
            "image/webp"
        } else if bytes.starts_with(b"<svg")
            || (bytes.starts_with(b"<?xml") && bytes.windows(4).any(|w| w == b"<svg"))
        {
            "image/svg+xml"
        } else if bytes.starts_with(b"%PDF-") {
            "application/pdf"
        } else if bytes.len() >= 8 && &bytes[4..8] == b"ftyp" {
            "video/mp4"
        } else if bytes.starts_with(b"ID3")
            || (bytes.len() >= 2 && bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0)
        {
            "audio/mpeg"
        } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
            "audio/wav"
        } else {
            return Err(StudioError::InvalidInput {
                field: "bytes.magic_signature".into(),
                reason: format!(
                    "Chữ ký ma thuật (magic bytes) không khớp với bất kỳ định dạng đa phương tiện an toàn nào được hỗ trợ (khai báo: '{}')",
                    declared_mime
                ),
                correlation_id: "media_validator".into(),
            });
        };

        Ok(detected_mime.to_string())
    }

    // 1. Project Operations
    pub fn create_project(
        &self,
        actor: &ActorContext,
        envelope: OperationEnvelope<CreateProjectInput>,
    ) -> Result<OperationResult<Project>, StudioError> {
        self.check_human_only(actor, &envelope.operation)?;

        let project = Project::new(
            &actor.active_workspace_id,
            envelope.input.title,
            envelope.input.project_type,
        );

        self.catalog.create_project(&project)?;

        let audit_id = self.catalog.record_audit(
            &actor.actor_id,
            actor.role,
            &actor.active_workspace_id,
            &envelope.operation,
            "DRAFT",
            Some(&project.id),
            None,
        )?;

        Ok(OperationResult {
            request_id: envelope.request_id,
            aggregate_id: Some(project.id.clone()),
            new_revision_id: None,
            result: project,
            warnings: vec![],
            audit_id,
            capability_snapshot: None,
        })
    }

    pub fn get_project(
        &self,
        actor: &ActorContext,
        project_id: &str,
    ) -> Result<Project, StudioError> {
        let p = self
            .catalog
            .get_project(project_id)?
            .ok_or_else(|| StudioError::NotFound {
                resource_type: "project".into(),
                resource_id: project_id.into(),
                correlation_id: actor.actor_id.clone(),
            })?;

        self.check_scope(actor, &p.workspace_id)?;
        Ok(p)
    }

    pub fn list_projects(&self, actor: &ActorContext) -> Result<Vec<Project>, StudioError> {
        self.catalog.list_projects(&actor.active_workspace_id)
    }

    // 2. Creative Document & Revision Operations with CAS
    pub fn save_revision(
        &self,
        actor: &ActorContext,
        envelope: OperationEnvelope<SaveRevisionInput>,
    ) -> Result<OperationResult<ProjectRevision>, StudioError> {
        let project = self.get_project(actor, &envelope.input.project_id)?;
        self.check_scope(actor, &project.workspace_id)?;

        let rev = self.catalog.save_revision_and_document(
            &envelope.input.project_id,
            envelope.expected_revision_id.as_deref(),
            &envelope.input.document,
            &actor.actor_id,
            &envelope.input.summary,
        )?;

        let audit_id = self.catalog.record_audit(
            &actor.actor_id,
            actor.role,
            &actor.active_workspace_id,
            &envelope.operation,
            "REVERSIBLE",
            Some(&project.id),
            Some(&serde_json::json!({
                "revision_id": rev.id,
                "source_hash": rev.source_hash,
            })),
        )?;

        Ok(OperationResult {
            request_id: envelope.request_id,
            aggregate_id: Some(project.id),
            new_revision_id: Some(rev.id.clone()),
            result: rev,
            warnings: vec![],
            audit_id,
            capability_snapshot: None,
        })
    }

    pub fn get_latest_revision(
        &self,
        actor: &ActorContext,
        project_id: &str,
    ) -> Result<Option<ProjectRevision>, StudioError> {
        let project = self.get_project(actor, project_id)?;
        self.check_scope(actor, &project.workspace_id)?;
        self.catalog.get_latest_revision(project_id)
    }

    pub fn get_document_by_revision(
        &self,
        actor: &ActorContext,
        project_id: &str,
        revision_id: &str,
    ) -> Result<CreativeDocument, StudioError> {
        let project = self.get_project(actor, project_id)?;
        self.check_scope(actor, &project.workspace_id)?;

        self.catalog
            .get_document_by_revision_scoped(&actor.active_workspace_id, project_id, revision_id)?
            .ok_or_else(|| StudioError::NotFound {
                resource_type: "creative_document".into(),
                resource_id: revision_id.into(),
                correlation_id: actor.actor_id.clone(),
            })
    }

    // 3. Asset Operations with Quarantine & Human-Only Rights Review
    pub fn import_asset_bytes(
        &self,
        actor: &ActorContext,
        envelope: OperationEnvelope<ImportAssetInput>,
    ) -> Result<OperationResult<Asset>, StudioError> {
        const MAX_ASSET_BYTES: usize = 50 * 1024 * 1024; // 50MB hard quota
        if envelope.input.bytes.len() > MAX_ASSET_BYTES {
            return Err(StudioError::QuotaExceeded {
                reason: format!(
                    "Dung lượng tệp ({} bytes) vượt quá hạn mức tối đa cho phép là 50MB",
                    envelope.input.bytes.len()
                ),
                correlation_id: actor.actor_id.clone(),
            });
        }

        let verified_mime =
            Self::verify_media_magic_bytes(&envelope.input.bytes, &envelope.input.declared_mime)?;

        let (sha256, size) = self
            .blob_storage
            .write_blob(&envelope.input.bytes)
            .map_err(|e| StudioError::Internal {
                message: format!("Failed to write blob: {}", e),
                correlation_id: actor.actor_id.clone(),
            })?;

        let asset = Asset::new_quarantined(
            envelope.input.scope,
            if envelope.input.scope == AssetScope::Workspace {
                Some(actor.active_workspace_id.clone())
            } else {
                None
            },
            envelope.input.kind,
            envelope.input.original_name,
            envelope.input.declared_mime,
            verified_mime,
            sha256,
            size,
            envelope.input.provenance,
        );

        self.catalog.save_asset(&asset)?;

        let audit_id = self.catalog.record_audit(
            &actor.actor_id,
            actor.role,
            &actor.active_workspace_id,
            &envelope.operation,
            "DRAFT",
            Some(&asset.id),
            None,
        )?;

        Ok(OperationResult {
            request_id: envelope.request_id,
            aggregate_id: Some(asset.id.clone()),
            new_revision_id: None,
            result: asset,
            warnings: vec!["Asset được nhập ở trạng thái QUARANTINED. Cần người dùng xác nhận bản quyền trước khi sử dụng để xuất bản.".into()],
            audit_id,
            capability_snapshot: None,
        })
    }

    pub fn get_asset(&self, actor: &ActorContext, asset_id: &str) -> Result<Asset, StudioError> {
        let asset = self
            .catalog
            .get_asset_scoped(&actor.active_workspace_id, asset_id)?
            .ok_or_else(|| StudioError::NotFound {
                resource_type: "asset".into(),
                resource_id: asset_id.into(),
                correlation_id: actor.actor_id.clone(),
            })?;

        if asset.scope == AssetScope::Workspace {
            if let Some(ref owner_ws) = asset.owner_workspace_id {
                self.check_scope(actor, owner_ws)?;
            }
        }

        Ok(asset)
    }

    pub fn list_assets(&self, actor: &ActorContext) -> Result<Vec<Asset>, StudioError> {
        self.catalog.list_assets(&actor.active_workspace_id)
    }

    pub fn attest_asset_rights(
        &self,
        actor: &ActorContext,
        envelope: OperationEnvelope<AttestRightsInput>,
    ) -> Result<OperationResult<Asset>, StudioError> {
        // Enforce HUMAN ONLY with valid confirmation proof!
        let action_hash = Self::compute_action_hash(&envelope.input.asset_id);
        self.check_human_only_with_proof(
            actor,
            "rights.attest",
            &action_hash,
            envelope.human_proof_token.as_deref(),
        )?;

        let mut asset = self.get_asset(actor, &envelope.input.asset_id)?;

        let attestation = RightsAttestation {
            source: envelope.input.source,
            license_basis: envelope.input.license_basis,
            attribution_text: envelope.input.attribution_text,
            allowed_uses: envelope.input.allowed_uses,
            attested_by_human_actor_id: actor.actor_id.clone(),
            attested_at: chrono::Utc::now(),
        };

        self.catalog.update_asset_rights(
            &asset.id,
            RightsState::Ready,
            Some(attestation.clone()),
        )?;
        asset.rights_state = RightsState::Ready;
        asset.rights_attestation = Some(attestation);

        let audit_id = self.catalog.record_audit(
            &actor.actor_id,
            actor.role,
            &actor.active_workspace_id,
            "rights.attest",
            "REVERSIBLE",
            Some(&asset.id),
            None,
        )?;

        Ok(OperationResult {
            request_id: envelope.request_id,
            aggregate_id: Some(asset.id.clone()),
            new_revision_id: None,
            result: asset,
            warnings: vec![],
            audit_id,
            capability_snapshot: None,
        })
    }

    pub fn revoke_asset_rights(
        &self,
        actor: &ActorContext,
        envelope: OperationEnvelope<RevokeRightsInput>,
    ) -> Result<OperationResult<Asset>, StudioError> {
        let action_hash = Self::compute_action_hash(&envelope.input.asset_id);
        self.check_human_only_with_proof(
            actor,
            "rights.revoke",
            &action_hash,
            envelope.human_proof_token.as_deref(),
        )?;

        let mut asset = self.get_asset(actor, &envelope.input.asset_id)?;
        self.catalog
            .update_asset_rights(&asset.id, RightsState::Revoked, None)?;
        asset.rights_state = RightsState::Revoked;
        asset.rights_attestation = None;

        let audit_id = self.catalog.record_audit(
            &actor.actor_id,
            actor.role,
            &actor.active_workspace_id,
            "rights.revoke",
            "REVERSIBLE",
            Some(&asset.id),
            Some(&serde_json::json!({ "reason": envelope.input.reason })),
        )?;

        Ok(OperationResult {
            request_id: envelope.request_id,
            aggregate_id: Some(asset.id.clone()),
            new_revision_id: None,
            result: asset,
            warnings: vec!["Quyền sử dụng asset đã bị thu hồi (REVOKED). Mọi hành động xuất bản mới sẽ bị chặn.".into()],
            audit_id,
            capability_snapshot: None,
        })
    }

    pub fn transfer_asset_scope(
        &self,
        actor: &ActorContext,
        envelope: OperationEnvelope<TransferScopeInput>,
    ) -> Result<OperationResult<Asset>, StudioError> {
        let action_hash = Self::compute_action_hash(&envelope.input.asset_id);
        self.check_human_only_with_proof(
            actor,
            "asset.transfer_scope",
            &action_hash,
            envelope.human_proof_token.as_deref(),
        )?;

        let source_asset = self.get_asset(actor, &envelope.input.asset_id)?;

        // Contract: Transferring scope creates a NEW asset record with QUARANTINED rights
        let new_asset = Asset::new_quarantined(
            envelope.input.target_scope,
            if envelope.input.target_scope == AssetScope::Workspace {
                Some(
                    envelope
                        .input
                        .target_workspace_id
                        .clone()
                        .unwrap_or_else(|| actor.active_workspace_id.clone()),
                )
            } else {
                None
            },
            source_asset.kind,
            source_asset.original_name,
            source_asset.declared_mime,
            source_asset.verified_mime,
            source_asset.blob_sha256,
            source_asset.file_size_bytes,
            format!(
                "Sao chép từ asset {} (reset quyền sử dụng)",
                source_asset.id
            ),
        );

        self.catalog.save_asset(&new_asset)?;

        let audit_id = self.catalog.record_audit(
            &actor.actor_id,
            actor.role,
            &actor.active_workspace_id,
            "asset.transfer_scope",
            "REVERSIBLE",
            Some(&new_asset.id),
            Some(&serde_json::json!({
                "source_asset_id": source_asset.id,
                "target_scope": envelope.input.target_scope,
            })),
        )?;

        Ok(OperationResult {
            request_id: envelope.request_id,
            aggregate_id: Some(new_asset.id.clone()),
            new_revision_id: None,
            result: new_asset,
            warnings: vec!["Chuyển đổi phạm vi Workspace <-> Global đã reset trạng thái bản quyền về QUARANTINED theo đúng hợp đồng bảo vệ.".into()],
            audit_id,
            capability_snapshot: None,
        })
    }

    pub fn delete_asset(
        &self,
        actor: &ActorContext,
        envelope: OperationEnvelope<DeleteAssetInput>,
    ) -> Result<OperationResult<String>, StudioError> {
        let action_hash = Self::compute_action_hash(&envelope.input.asset_id);
        self.check_human_only_with_proof(
            actor,
            "asset.delete",
            &action_hash,
            envelope.human_proof_token.as_deref(),
        )?;

        let asset = self.get_asset(actor, &envelope.input.asset_id)?;
        self.catalog.delete_asset(&asset.id)?;

        let audit_id = self.catalog.record_audit(
            &actor.actor_id,
            actor.role,
            &actor.active_workspace_id,
            "asset.delete",
            "IRREVERSIBLE",
            Some(&asset.id),
            None,
        )?;

        Ok(OperationResult {
            request_id: envelope.request_id,
            aggregate_id: Some(asset.id.clone()),
            new_revision_id: None,
            result: asset.id,
            warnings: vec![],
            audit_id,
            capability_snapshot: None,
        })
    }

    pub fn fetch_and_ingest_url(
        &self,
        actor: &ActorContext,
        envelope: OperationEnvelope<IngestUrlInput>,
        max_bytes: usize,
    ) -> Result<OperationResult<ResearchSource>, StudioError> {
        let html_content =
            research_validator::SafeUrlValidator::fetch_url_safe(&envelope.input.url, max_bytes)?;
        self.ingest_url_content(actor, envelope, &html_content)
    }

    pub fn ingest_url_content(
        &self,
        actor: &ActorContext,
        envelope: OperationEnvelope<IngestUrlInput>,
        html_content: &str,
    ) -> Result<OperationResult<ResearchSource>, StudioError> {
        let validated_url =
            research_validator::SafeUrlValidator::validate_url(&envelope.input.url)?;
        let domain = validated_url.host_str().unwrap_or("unknown").to_string();

        let snapshot = research_extractor::ResearchExtractor::extract_from_html(
            html_content,
            validated_url.as_str(),
        );

        let mut source =
            ResearchSource::new(&actor.active_workspace_id, validated_url.as_str(), domain);
        source.fetch_status = "COMPLETED".to_string();
        source.snapshot = Some(snapshot);

        self.catalog.save_research_source(&source)?;

        let audit_id = self.catalog.record_audit(
            &actor.actor_id,
            actor.role,
            &actor.active_workspace_id,
            &envelope.operation,
            "REVERSIBLE",
            Some(&source.id),
            Some(&serde_json::json!({
                "canonical_url": source.canonical_url,
                "title": source.snapshot.as_ref().map(|s| &s.title),
            })),
        )?;

        Ok(OperationResult {
            request_id: envelope.request_id,
            aggregate_id: Some(source.id.clone()),
            new_revision_id: None,
            result: source,
            warnings: vec!["Nội dung web là dữ liệu thô chưa được kiểm duyệt bản quyền. Không sử dụng trực tiếp khi xuất bản mà chưa rà soát.".into()],
            audit_id,
            capability_snapshot: None,
        })
    }

    pub fn list_research_sources(
        &self,
        actor: &ActorContext,
    ) -> Result<Vec<ResearchSource>, StudioError> {
        self.catalog
            .list_research_sources(&actor.active_workspace_id)
    }

    pub fn get_research_source(
        &self,
        actor: &ActorContext,
        id: &str,
    ) -> Result<ResearchSource, StudioError> {
        let s = self
            .catalog
            .get_research_source(id)?
            .ok_or_else(|| StudioError::NotFound {
                resource_type: "research_source".into(),
                resource_id: id.into(),
                correlation_id: actor.actor_id.clone(),
            })?;

        self.check_scope(actor, &s.workspace_id)?;
        Ok(s)
    }

    // 5. Brand Operations
    pub fn create_brand_kit(
        &self,
        actor: &ActorContext,
        envelope: OperationEnvelope<CreateBrandKitInput>,
    ) -> Result<OperationResult<BrandKit>, StudioError> {
        let kit = BrandKit::new(&actor.active_workspace_id, envelope.input.name);
        self.catalog.save_brand_kit(&kit)?;

        let audit_id = self.catalog.record_audit(
            &actor.actor_id,
            actor.role,
            &actor.active_workspace_id,
            &envelope.operation,
            "DRAFT",
            Some(&kit.id),
            None,
        )?;

        Ok(OperationResult {
            request_id: envelope.request_id,
            aggregate_id: Some(kit.id.clone()),
            new_revision_id: None,
            result: kit,
            warnings: vec![],
            audit_id,
            capability_snapshot: None,
        })
    }

    pub fn propose_brand_version(
        &self,
        actor: &ActorContext,
        envelope: OperationEnvelope<ProposeBrandVersionInput>,
    ) -> Result<OperationResult<BrandVersion>, StudioError> {
        let kit = self
            .catalog
            .get_brand_kit(&envelope.input.brand_kit_id)?
            .ok_or_else(|| StudioError::NotFound {
                resource_type: "brand_kit".into(),
                resource_id: envelope.input.brand_kit_id.clone(),
                correlation_id: actor.actor_id.clone(),
            })?;
        self.check_scope(actor, &kit.workspace_id)?;

        let existing_versions = self.catalog.list_brand_versions(&kit.id)?;
        let next_number = (existing_versions.len() as u32) + 1;

        let version = BrandVersion {
            id: uuid::Uuid::new_v4().to_string(),
            brand_kit_id: kit.id.clone(),
            version_number: next_number,
            state: BrandVersionState::Draft,
            visual_tokens: envelope.input.visual_tokens,
            voice: envelope.input.voice,
            logos: envelope.input.logos,
            provenance: format!("Đề xuất bởi actor: {} ({:?})", actor.actor_id, actor.role),
            created_at: chrono::Utc::now(),
        };

        self.catalog.save_brand_version(&version)?;

        let audit_id = self.catalog.record_audit(
            &actor.actor_id,
            actor.role,
            &actor.active_workspace_id,
            &envelope.operation,
            "DRAFT",
            Some(&version.id),
            None,
        )?;

        Ok(OperationResult {
            request_id: envelope.request_id,
            aggregate_id: Some(version.id.clone()),
            new_revision_id: None,
            result: version,
            warnings: vec!["Phiên bản thương hiệu được tạo ở trạng thái DRAFT. Cần con người phê duyệt để PUBLISHED.".into()],
            audit_id,
            capability_snapshot: None,
        })
    }

    pub fn publish_brand_version(
        &self,
        actor: &ActorContext,
        envelope: OperationEnvelope<PublishBrandVersionInput>,
    ) -> Result<OperationResult<BrandVersion>, StudioError> {
        // Enforce HUMAN ONLY with valid confirmation proof!
        let action_hash = Self::compute_action_hash(&envelope.input.brand_version_id);
        self.check_human_only_with_proof(
            actor,
            "brand.publish",
            &action_hash,
            envelope.human_proof_token.as_deref(),
        )?;

        let mut version = self
            .catalog
            .get_brand_version(&envelope.input.brand_version_id)?
            .ok_or_else(|| StudioError::NotFound {
                resource_type: "brand_version".into(),
                resource_id: envelope.input.brand_version_id.clone(),
                correlation_id: actor.actor_id.clone(),
            })?;

        let mut kit = self
            .catalog
            .get_brand_kit(&version.brand_kit_id)?
            .ok_or_else(|| StudioError::NotFound {
                resource_type: "brand_kit".into(),
                resource_id: version.brand_kit_id.clone(),
                correlation_id: actor.actor_id.clone(),
            })?;
        self.check_scope(actor, &kit.workspace_id)?;

        // Deterministic validator check
        brand_validator::BrandValidator::validate_for_publish(&version)?;

        version.state = BrandVersionState::Published;
        self.catalog.save_brand_version(&version)?;

        kit.active_version_id = Some(version.id.clone());
        self.catalog.save_brand_kit(&kit)?;

        let audit_id = self.catalog.record_audit(
            &actor.actor_id,
            actor.role,
            &actor.active_workspace_id,
            "brand.publish",
            "REVERSIBLE",
            Some(&version.id),
            None,
        )?;

        Ok(OperationResult {
            request_id: envelope.request_id,
            aggregate_id: Some(version.id.clone()),
            new_revision_id: None,
            result: version,
            warnings: vec![],
            audit_id,
            capability_snapshot: None,
        })
    }

    pub fn get_brand_kit(&self, actor: &ActorContext, id: &str) -> Result<BrandKit, StudioError> {
        let kit = self
            .catalog
            .get_brand_kit(id)?
            .ok_or_else(|| StudioError::NotFound {
                resource_type: "brand_kit".into(),
                resource_id: id.into(),
                correlation_id: actor.actor_id.clone(),
            })?;
        self.check_scope(actor, &kit.workspace_id)?;
        Ok(kit)
    }

    pub fn list_brand_kits(&self, actor: &ActorContext) -> Result<Vec<BrandKit>, StudioError> {
        self.catalog.list_brand_kits(&actor.active_workspace_id)
    }

    pub fn list_brand_versions(
        &self,
        actor: &ActorContext,
        brand_kit_id: &str,
    ) -> Result<Vec<BrandVersion>, StudioError> {
        let kit = self.get_brand_kit(actor, brand_kit_id)?;
        self.check_scope(actor, &kit.workspace_id)?;
        self.catalog.list_brand_versions(brand_kit_id)
    }

    // 6. Preview & Static Export Operations
    pub fn get_preview_receipt(
        &self,
        actor: &ActorContext,
        project_id: &str,
        revision_id: &str,
        is_draft: bool,
    ) -> Result<PreviewReceipt, StudioError> {
        let doc = self.get_document_by_revision(actor, project_id, revision_id)?;
        Ok(static_export::StaticExportEngine::generate_preview_receipt(
            &doc,
            revision_id,
            is_draft,
        ))
    }

    pub fn export_project_svg(
        &self,
        actor: &ActorContext,
        project_id: &str,
        revision_id: &str,
    ) -> Result<ExportReceipt, StudioError> {
        let doc = self.get_document_by_revision(actor, project_id, revision_id)?;
        static_export::StaticExportEngine::export_artboards_svg(project_id, revision_id, &doc)
    }

    pub fn export_project_html(
        &self,
        actor: &ActorContext,
        project_id: &str,
        revision_id: &str,
    ) -> Result<ExportReceipt, StudioError> {
        let doc = self.get_document_by_revision(actor, project_id, revision_id)?;
        static_export::StaticExportEngine::export_static_html(project_id, revision_id, &doc)
    }

    // 7. AI Gateway Operations
    pub async fn generate_ai_draft(
        &self,
        actor: &ActorContext,
        gateway: &ai_gateway::AIGateway,
        prompt: &str,
        system_instructions: &str,
    ) -> Result<ai_gateway::NormalizedCompletionResponse, StudioError> {
        let resp = gateway
            .generate_completion(prompt, system_instructions)
            .await?;

        let _ = self.catalog.record_audit(
            &actor.actor_id,
            actor.role,
            &actor.active_workspace_id,
            "ai.generate_draft",
            "REVERSIBLE",
            None,
            Some(&serde_json::json!({
                "model": resp.model,
                "usage_tokens": resp.usage_tokens,
            })),
        );

        Ok(resp)
    }

    // 8. Social Handoff Operations
    pub fn check_publish_readiness(
        &self,
        actor: &ActorContext,
        project_id: &str,
        revision_id: &str,
        target_page_id: &str,
        media_items: &[studio_core::handoff::DeliveryMediaItem],
    ) -> Result<studio_core::handoff::PublishReadiness, StudioError> {
        let project = self.get_project(actor, project_id)?;
        self.check_scope(actor, &project.workspace_id)?;

        let rev = self
            .catalog
            .get_latest_revision(project_id)?
            .ok_or_else(|| StudioError::NotFound {
                resource_type: "project_revision".into(),
                resource_id: revision_id.into(),
                correlation_id: actor.actor_id.clone(),
            })?;

        if rev.id != revision_id {
            return Err(StudioError::ConflictCas {
                expected: revision_id.to_string(),
                current: rev.id,
                correlation_id: actor.actor_id.clone(),
            });
        }

        // Check assets for quarantined or revoked state
        let doc = self.get_document_by_revision(actor, project_id, revision_id)?;
        let mut has_unapproved = false;
        for ab in &doc.artboards {
            for el in &ab.elements {
                if let Some(ref asset_ref) = el.asset_ref {
                    if let Ok(asset) = self.get_asset(actor, asset_ref) {
                        if asset.rights_state != RightsState::Ready {
                            has_unapproved = true;
                            break;
                        }
                    }
                }
            }
        }

        let readiness = social_gateway::SocialGatewayService::check_readiness(
            target_page_id,
            revision_id,
            &rev.source_hash,
            media_items,
            has_unapproved,
        );

        Ok(readiness)
    }

    pub fn prepare_delivery_package(
        &self,
        actor: &ActorContext,
        envelope: OperationEnvelope<PrepareDeliveryInput>,
    ) -> Result<OperationResult<studio_core::handoff::DeliveryPackage>, StudioError> {
        let project = self.get_project(actor, &envelope.input.project_id)?;
        self.check_scope(actor, &project.workspace_id)?;

        let rev = self
            .catalog
            .get_latest_revision(&envelope.input.project_id)?
            .ok_or_else(|| StudioError::NotFound {
                resource_type: "project_revision".into(),
                resource_id: envelope.input.revision_id.clone(),
                correlation_id: actor.actor_id.clone(),
            })?;

        if rev.id != envelope.input.revision_id {
            return Err(StudioError::ConflictCas {
                expected: envelope.input.revision_id,
                current: rev.id,
                correlation_id: actor.actor_id.clone(),
            });
        }

        let pkg = social_gateway::SocialGatewayService::prepare_delivery_package(
            &actor.active_workspace_id,
            &envelope.input.project_id,
            &envelope.input.revision_id,
            &rev.source_hash,
            &envelope.input.platform,
            &envelope.input.caption,
            &envelope.input.page_id,
            envelope.input.media,
        )?;

        self.catalog.save_delivery_package(&pkg)?;

        let audit_id = self.catalog.record_audit(
            &actor.actor_id,
            actor.role,
            &actor.active_workspace_id,
            "delivery.prepare",
            "REVERSIBLE",
            Some(&pkg.delivery_id),
            Some(&serde_json::json!({
                "page_id": pkg.target_selectors.page_id,
                "idempotency_key": pkg.idempotency_key,
            })),
        )?;

        Ok(OperationResult {
            request_id: envelope.request_id,
            aggregate_id: Some(pkg.delivery_id.clone()),
            new_revision_id: None,
            result: pkg,
            warnings: vec!["DeliveryPackage đã được tạo ở trạng thái PREPARED. Cần con người xác nhận trước khi gửi sang Social Outbox.".into()],
            audit_id,
            capability_snapshot: None,
        })
    }

    pub fn get_delivery_package(
        &self,
        actor: &ActorContext,
        delivery_id: &str,
    ) -> Result<studio_core::handoff::DeliveryPackage, StudioError> {
        let pkg = self
            .catalog
            .get_delivery_package_scoped(&actor.active_workspace_id, delivery_id)?
            .ok_or_else(|| StudioError::NotFound {
                resource_type: "delivery_package".into(),
                resource_id: delivery_id.into(),
                correlation_id: actor.actor_id.clone(),
            })?;
        Ok(pkg)
    }

    pub fn confirm_publish_and_dispatch(
        &self,
        actor: &ActorContext,
        envelope: OperationEnvelope<ConfirmPublishInput>,
    ) -> Result<OperationResult<studio_core::handoff::OutboxItem>, StudioError> {
        // Enforce HUMAN ONLY with valid confirmation proof!
        let action_hash = Self::compute_action_hash(&envelope.input.delivery_id);
        self.check_human_only_with_proof(
            actor,
            "publish.confirm",
            &action_hash,
            envelope.human_proof_token.as_deref(),
        )?;

        let mut pkg = self.get_delivery_package(actor, &envelope.input.delivery_id)?;
        self.check_scope(actor, &pkg.studio_workspace_id)?;

        // Verify latest revision has not drifted (stale approval protection)
        let latest_rev = self
            .catalog
            .get_latest_revision(&pkg.project_id)?
            .ok_or_else(|| StudioError::NotFound {
                resource_type: "project_revision".into(),
                resource_id: pkg.source_revision_id.clone(),
                correlation_id: actor.actor_id.clone(),
            })?;

        if latest_rev.id != pkg.source_revision_id || latest_rev.source_hash != pkg.source_hash {
            pkg.status = studio_core::handoff::HandoffStatus::Rejected;
            self.catalog
                .update_delivery_status(&pkg.delivery_id, pkg.status)?;
            return Err(StudioError::PreconditionFailed {
                reason: "Bản thảo đã bị thay đổi sau khi tạo gói handoff. Phê duyệt đã bị vô hiệu hóa (stale approval). Cần tạo gói handoff mới.".into(),
                correlation_id: actor.actor_id.clone(),
            });
        }

        // Recheck publish readiness: assets rights, page_id, media
        let readiness = self.check_publish_readiness(
            actor,
            &pkg.project_id,
            &pkg.source_revision_id,
            &pkg.target_selectors.page_id,
            &pkg.media,
        )?;

        if !readiness.is_ready {
            return Err(StudioError::PreconditionFailed {
                reason: format!(
                    "Gói bàn giao không đủ điều kiện xuất bản: {}",
                    readiness.blockers.join("; ")
                ),
                correlation_id: actor.actor_id.clone(),
            });
        }

        let destination = format!("social-gateway:{}", pkg.variant.platform);

        // Check if outbox item already exists (idempotency unique check)
        let mut outbox_item = if let Some(existing) = self
            .catalog
            .get_outbox_by_idempotency_key(&destination, &pkg.idempotency_key)?
        {
            existing
        } else {
            let item = social_gateway::SocialGatewayService::create_outbox_item(&pkg);
            self.catalog.save_outbox_item(&item)?;
            item
        };

        // Dispatch via SocialConnector
        let connector =
            social_gateway::SocialConnector::new(std::env::var("STUDIO_SOCIAL_ENDPOINT").ok());
        let dispatch_result = connector.dispatch_package(&pkg);

        match dispatch_result {
            social_gateway::SocialConnectorResult::Accepted(receipt) => {
                outbox_item.state = studio_core::handoff::OutboxState::Accepted;
                outbox_item.remote_ref = Some(receipt.remote_draft_id.clone());
                self.catalog.update_outbox_state(
                    &outbox_item.id,
                    studio_core::handoff::OutboxState::Accepted,
                    Some(&receipt.remote_draft_id),
                )?;

                pkg.status = studio_core::handoff::HandoffStatus::Accepted;
                self.catalog
                    .update_delivery_status(&pkg.delivery_id, pkg.status)?;
            }
            social_gateway::SocialConnectorResult::UnknownTimeout(_reason) => {
                outbox_item.state = studio_core::handoff::OutboxState::Unknown;
                self.catalog.update_outbox_state(
                    &outbox_item.id,
                    studio_core::handoff::OutboxState::Unknown,
                    None,
                )?;

                pkg.status = studio_core::handoff::HandoffStatus::NeedsReconciliation;
                self.catalog
                    .update_delivery_status(&pkg.delivery_id, pkg.status)?;
            }
            social_gateway::SocialConnectorResult::Rejected(_reason) => {
                outbox_item.state = studio_core::handoff::OutboxState::Failed;
                self.catalog.update_outbox_state(
                    &outbox_item.id,
                    studio_core::handoff::OutboxState::Failed,
                    None,
                )?;

                pkg.status = studio_core::handoff::HandoffStatus::Rejected;
                self.catalog
                    .update_delivery_status(&pkg.delivery_id, pkg.status)?;
            }
        }

        let audit_id = self.catalog.record_audit(
            &actor.actor_id,
            actor.role,
            &actor.active_workspace_id,
            "publish.confirm",
            "REVERSIBLE",
            Some(&pkg.delivery_id),
            Some(&serde_json::json!({
                "outbox_id": outbox_item.id,
                "idempotency_key": outbox_item.idempotency_key,
                "handoff_status": pkg.status,
            })),
        )?;

        Ok(OperationResult {
            request_id: envelope.request_id,
            aggregate_id: Some(pkg.delivery_id),
            new_revision_id: None,
            result: outbox_item,
            warnings: vec![],
            audit_id,
            capability_snapshot: None,
        })
    }
}

// Input DTOs
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IngestUrlInput {
    pub url: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateProjectInput {
    pub title: String,
    pub project_type: ProjectType,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SaveRevisionInput {
    pub project_id: String,
    pub document: CreativeDocument,
    pub summary: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportAssetInput {
    pub scope: AssetScope,
    pub kind: AssetKind,
    pub original_name: String,
    pub declared_mime: String,
    pub verified_mime: String,
    pub provenance: String,
    #[serde(skip_serializing)]
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AttestRightsInput {
    pub asset_id: String,
    pub source: String,
    pub license_basis: String,
    pub attribution_text: Option<String>,
    pub allowed_uses: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RevokeRightsInput {
    pub asset_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransferScopeInput {
    pub asset_id: String,
    pub target_scope: AssetScope,
    pub target_workspace_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeleteAssetInput {
    pub asset_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateBrandKitInput {
    pub name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProposeBrandVersionInput {
    pub brand_kit_id: String,
    pub visual_tokens: VisualTokens,
    pub voice: VoiceGuidelines,
    pub logos: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PublishBrandVersionInput {
    pub brand_version_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrepareDeliveryInput {
    pub project_id: String,
    pub revision_id: String,
    pub platform: String,
    pub caption: String,
    pub page_id: String,
    pub media: Vec<studio_core::handoff::DeliveryMediaItem>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfirmPublishInput {
    pub delivery_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::document::CreativeDocument;
    use studio_storage::{config::StorageConfig, initialize_storage};
    use tempfile::tempdir;

    #[test]
    fn test_application_host_security_and_operations() {
        let temp = tempdir().unwrap();
        let config = StorageConfig::new(temp.path());
        let (catalog, blob_storage, _) = initialize_storage(&config).unwrap();
        let caps = CapabilityRegistry::default();
        let host = ApplicationHost::new(catalog, blob_storage, caps);

        let human_actor = ActorContext::new_human("user-1", "ws-marketing");
        let agent_actor = ActorContext::new_agent("agent-1", "ws-marketing");
        let foreign_actor = ActorContext::new_human("user-2", "ws-foreign");

        // 1. Create project
        let create_op = OperationEnvelope::new(
            "project.create",
            CreateProjectInput {
                title: "Chiến dịch Q4".into(),
                project_type: ProjectType::Carousel,
            },
        );
        let res = host.create_project(&human_actor, create_op).unwrap();
        let project = res.result;
        assert_eq!(project.title, "Chiến dịch Q4");

        // 2. Foreign actor access should fail with ForbiddenScope
        let get_err = host.get_project(&foreign_actor, &project.id);
        assert!(get_err.is_err());
        match get_err.err().unwrap() {
            StudioError::ForbiddenScope { resource_id, .. } => {
                assert_eq!(resource_id, "ws-marketing");
            }
            other => panic!("Expected ForbiddenScope, got {:?}", other),
        }

        // 3. Save initial revision
        let doc = CreativeDocument::new("doc-q4");
        let save_op = OperationEnvelope::new(
            "revision.save",
            SaveRevisionInput {
                project_id: project.id.clone(),
                document: doc.clone(),
                summary: "Bản nháp carousel ban đầu".into(),
            },
        );
        let save_res = host.save_revision(&human_actor, save_op).unwrap();
        let rev1 = save_res.result;
        assert_eq!(rev1.project_id, project.id);

        // 4. Import asset with real PNG magic bytes (should default to QUARANTINED)
        let asset_op = OperationEnvelope::new(
            "asset.import",
            ImportAssetInput {
                scope: AssetScope::Workspace,
                kind: AssetKind::Image,
                original_name: "poster.png".into(),
                declared_mime: "image/png".into(),
                verified_mime: "image/png".into(),
                provenance: "Tải lên từ máy thiết kế".into(),
                bytes: b"\x89PNG\r\n\x1a\nPNG_TEST_IMAGE_BYTES".to_vec(),
            },
        );
        let asset_res = host.import_asset_bytes(&human_actor, asset_op).unwrap();
        let asset = asset_res.result;
        assert_eq!(asset.rights_state, RightsState::Quarantined);
        assert_eq!(asset.verified_mime, "image/png");

        // 5. Agent trying to attest rights MUST be blocked with ForbiddenHumanOnly!
        let attest_op = OperationEnvelope::new(
            "rights.attest",
            AttestRightsInput {
                asset_id: asset.id.clone(),
                source: "Unsplash".into(),
                license_basis: "Free commercial".into(),
                attribution_text: None,
                allowed_uses: vec!["marketing".into()],
            },
        );
        let agent_attest_err = host.attest_asset_rights(&agent_actor, attest_op.clone());
        assert!(agent_attest_err.is_err());
        match agent_attest_err.err().unwrap() {
            StudioError::ForbiddenHumanOnly { operation, .. } => {
                assert_eq!(operation, "rights.attest");
            }
            other => panic!("Expected ForbiddenHumanOnly, got {:?}", other),
        }

        // 6. Human without confirmation proof MUST be blocked with HumanConfirmationRequired!
        let no_proof_err = host.attest_asset_rights(&human_actor, attest_op.clone());
        assert!(no_proof_err.is_err());
        match no_proof_err.err().unwrap() {
            StudioError::HumanConfirmationRequired { operation, .. } => {
                assert_eq!(operation, "rights.attest");
            }
            other => panic!("Expected HumanConfirmationRequired, got {:?}", other),
        }

        // 7. Human with valid proof can attest rights successfully
        let session = host.authenticate_local_os_user("ws-marketing").unwrap();
        let session_human = ActorContext::new_human(&session.subject_id, "ws-marketing");
        let hash = ApplicationHost::compute_action_hash(&asset.id);
        let proof = host
            .issue_human_proof(&session, "rights.attest", &hash)
            .unwrap();
        let human_attest_res = host
            .attest_asset_rights(&session_human, attest_op.with_human_proof(proof.proof_id))
            .unwrap();
        assert_eq!(human_attest_res.result.rights_state, RightsState::Ready);
    }
}
