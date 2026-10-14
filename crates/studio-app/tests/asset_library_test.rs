use studio_app::{
    ApplicationHost, AttestRightsInput, DeleteAssetInput, ImportAssetInput, RevokeRightsInput,
    TransferScopeInput,
};
use studio_core::{
    actor::ActorContext,
    asset::{AssetKind, AssetScope, RightsState},
    capabilities::CapabilityRegistry,
    error::StudioError,
    operation::OperationEnvelope,
};
use studio_storage::{config::StorageConfig, initialize_storage};
use tempfile::tempdir;

#[test]
fn test_asset_lifecycle_quarantine_attest_revoke() {
    let temp = tempdir().unwrap();
    let config = StorageConfig::new(temp.path());
    let (catalog, blob_storage, _) = initialize_storage(&config).unwrap();
    let caps = CapabilityRegistry::default();
    let host = ApplicationHost::new(catalog, blob_storage, caps);

    let session = host.authenticate_local_os_user("ws-marketing").unwrap();
    let human_actor = ActorContext::new_human(&session.subject_id, "ws-marketing");
    let agent_actor = ActorContext::new_agent("bot-assistant", "ws-marketing");

    // 1. Upload/import creates asset in QUARANTINED state with verified magic bytes
    let dummy_image_bytes = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x01];
    let import_op = OperationEnvelope::new(
        "asset.import",
        ImportAssetInput {
            scope: AssetScope::Workspace,
            kind: AssetKind::Image,
            original_name: "poster-banner.png".into(),
            declared_mime: "image/png".into(),
            verified_mime: "image/png".into(),
            provenance: "Chụp từ thiết kế Photoshop".into(),
            bytes: dummy_image_bytes,
        },
    );

    let import_res = host.import_asset_bytes(&human_actor, import_op).unwrap();
    let asset = import_res.result;
    assert_eq!(asset.rights_state, RightsState::Quarantined);
    assert!(asset.rights_attestation.is_none());
    assert_eq!(asset.blob_sha256.len(), 64);
    assert_eq!(asset.verified_mime, "image/png");

    // 2. Agent attempts rights.attest -> DENIED (Human-only gate)
    let agent_attest_op = OperationEnvelope::new(
        "rights.attest",
        AttestRightsInput {
            asset_id: asset.id.clone(),
            source: "Agent claim".into(),
            license_basis: "Fair use".into(),
            attribution_text: None,
            allowed_uses: vec!["all".into()],
        },
    );
    let agent_err = host.attest_asset_rights(&agent_actor, agent_attest_op);
    assert!(agent_err.is_err());
    match agent_err.err().unwrap() {
        StudioError::ForbiddenHumanOnly { operation, .. } => {
            assert_eq!(operation, "rights.attest");
        }
        other => panic!("Expected ForbiddenHumanOnly, got {:?}", other),
    }

    // 3. Human attests rights without proof -> DENIED with HumanConfirmationRequired
    let human_attest_op = OperationEnvelope::new(
        "rights.attest",
        AttestRightsInput {
            asset_id: asset.id.clone(),
            source: "Được mua bản quyền từ Shutterstock (ID #98234)".into(),
            license_basis: "Commercial Standard License".into(),
            attribution_text: Some("© 2026 BrandCo".into()),
            allowed_uses: vec!["digital_advertising".into(), "social_media".into()],
        },
    );
    let no_proof_err = host.attest_asset_rights(&human_actor, human_attest_op.clone());
    assert!(no_proof_err.is_err());
    match no_proof_err.err().unwrap() {
        StudioError::HumanConfirmationRequired { operation, .. } => {
            assert_eq!(operation, "rights.attest");
        }
        other => panic!("Expected HumanConfirmationRequired, got {:?}", other),
    }

    // 4. Human issues proof and attests rights -> Transitions to READY
    let action_hash = ApplicationHost::compute_action_hash(&asset.id);
    let proof = host
        .issue_human_proof(&session, "rights.attest", &action_hash)
        .unwrap();
    let ready_res = host
        .attest_asset_rights(
            &human_actor,
            human_attest_op.with_human_proof(proof.proof_id),
        )
        .unwrap();
    let ready_asset = ready_res.result;
    assert_eq!(ready_asset.rights_state, RightsState::Ready);
    let attestation = ready_asset
        .rights_attestation
        .expect("Attestation must exist");
    assert_eq!(attestation.attested_by_human_actor_id, session.subject_id);

    // 5. Human revokes rights with valid proof -> Transitions to REVOKED
    let proof_revoke = host
        .issue_human_proof(&session, "rights.revoke", &action_hash)
        .unwrap();
    let revoke_op = OperationEnvelope::new(
        "rights.revoke",
        RevokeRightsInput {
            asset_id: asset.id.clone(),
            reason: "Hết hạn hợp đồng bản quyền vào ngày 2026-10-12".into(),
        },
    )
    .with_human_proof(proof_revoke.proof_id);
    let revoke_res = host.revoke_asset_rights(&human_actor, revoke_op).unwrap();
    assert_eq!(revoke_res.result.rights_state, RightsState::Revoked);

    // 6. Transfer scope Workspace -> Global resets rights to QUARANTINED
    let proof_transfer = host
        .issue_human_proof(&session, "asset.transfer_scope", &action_hash)
        .unwrap();
    let transfer_op = OperationEnvelope::new(
        "asset.transfer_scope",
        TransferScopeInput {
            asset_id: asset.id.clone(),
            target_scope: AssetScope::Global,
            target_workspace_id: None,
        },
    )
    .with_human_proof(proof_transfer.proof_id);
    let transfer_res = host
        .transfer_asset_scope(&human_actor, transfer_op)
        .unwrap();
    let global_asset = transfer_res.result;
    assert_ne!(global_asset.id, asset.id); // New record created
    assert_eq!(global_asset.scope, AssetScope::Global);
    assert_eq!(global_asset.rights_state, RightsState::Quarantined);
    assert!(global_asset.rights_attestation.is_none());
    assert_eq!(global_asset.blob_sha256, asset.blob_sha256); // Shares blob hash deduplication

    // 7. Delete asset with proof
    let proof_del = host
        .issue_human_proof(&session, "asset.delete", &action_hash)
        .unwrap();
    let delete_op = OperationEnvelope::new(
        "asset.delete",
        DeleteAssetInput {
            asset_id: asset.id.clone(),
        },
    )
    .with_human_proof(proof_del.proof_id);
    let del_res = host.delete_asset(&human_actor, delete_op).unwrap();
    assert_eq!(del_res.result, asset.id);

    // Verifying deletion
    let not_found = host.get_asset(&human_actor, &asset.id);
    assert!(not_found.is_err());
}
