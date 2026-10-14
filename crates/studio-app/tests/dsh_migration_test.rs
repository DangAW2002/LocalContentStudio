use studio_app::{
    dsh_migration::{
        DshConnectorService, LegacyAssetRef, LegacyMigrationService, LegacyProjectRecord,
    },
    ApplicationHost, CreateProjectInput,
};
use studio_core::{
    actor::ActorContext, capabilities::CapabilityRegistry, error::StudioError,
    operation::OperationEnvelope, project::ProjectType,
};
use studio_storage::{config::StorageConfig, initialize_storage};
use tempfile::tempdir;

#[test]
fn test_dsh_connector_and_legacy_migration() {
    let temp = tempdir().unwrap();
    let config = StorageConfig::new(temp.path());
    let (catalog, blob_storage, _) = initialize_storage(&config).unwrap();
    let caps = CapabilityRegistry::default();
    let host = ApplicationHost::new(catalog.clone(), blob_storage, caps);

    let human_actor = ActorContext::new_human("editor-1", "ws-brand-a");
    let foreign_actor = ActorContext::new_human("editor-2", "ws-brand-b");

    // 1. Create project in ws-brand-a
    let proj = host
        .create_project(
            &human_actor,
            OperationEnvelope::new(
                "project.create",
                CreateProjectInput {
                    title: "Dự án Thiết kế Mùa hè".into(),
                    project_type: ProjectType::Carousel,
                },
            ),
        )
        .unwrap()
        .result;

    // 2. DSH DeepLink generation
    let card = DshConnectorService::create_studio_deeplink(&human_actor, &proj.id).unwrap();
    assert_eq!(card.card_type, "project_preview");
    assert_eq!(card.workspace_id, "ws-brand-a");
    assert!(card.action_deeplink.contains(&proj.id));

    // Extract grant token
    let grant = card.action_deeplink.split("grant=").nth(1).unwrap();

    // 3. Verify valid scoped grant
    assert!(DshConnectorService::verify_scoped_grant(&human_actor, "ws-brand-a", grant).is_ok());

    // 4. Cross-workspace boundary attack: Foreign actor in ws-brand-b tries to use grant on ws-brand-a -> DENIED
    let cross_ws_err =
        DshConnectorService::verify_scoped_grant(&foreign_actor, "ws-brand-a", grant);
    assert!(cross_ws_err.is_err());
    match cross_ws_err.err().unwrap() {
        StudioError::ForbiddenScope { resource_id, .. } => {
            assert_eq!(resource_id, "ws-brand-a");
        }
        other => panic!("Expected ForbiddenScope, got {:?}", other),
    }

    // 5. Tampered grant token -> DENIED
    let tampered_err =
        DshConnectorService::verify_scoped_grant(&human_actor, "ws-brand-a", "forged_grant_token");
    assert!(tampered_err.is_err());
    match tampered_err.err().unwrap() {
        StudioError::Unauthorized { .. } => {}
        other => panic!("Expected Unauthorized, got {:?}", other),
    }

    // 6. Legacy Migration Dry-Run
    let doc1_json = serde_json::to_string(&studio_core::document::CreativeDocument::new(
        "doc-legacy-1",
    ))
    .unwrap();
    let doc2_json = serde_json::to_string(&studio_core::document::CreativeDocument::new(
        "doc-legacy-2",
    ))
    .unwrap();

    let legacy_records = vec![
        LegacyProjectRecord {
            legacy_id: "legacy-p1".into(),
            title: "Báo cáo Social Q1 Cũ".into(),
            kind: "report".into(),
            original_workspace_id: None, // unassigned
            content_json: doc1_json,
            asset_refs: vec![LegacyAssetRef {
                asset_id: "legacy-asset-1".into(),
                filename: "chart.png".into(),
                sha256: "hash_chart_123".into(),
                mime_type: "image/png".into(),
                size_bytes: 2048,
            }],
        },
        LegacyProjectRecord {
            legacy_id: "legacy-p2".into(),
            title: "Poster Khuyến Mãi Cũ".into(),
            kind: "poster".into(),
            original_workspace_id: Some("ws-brand-a".into()),
            content_json: doc2_json,
            asset_refs: vec![],
        },
    ];

    // Dry-run with missing blob
    let dry_run_missing = LegacyMigrationService::dry_run(
        &legacy_records,
        "ws-brand-a",
        &[], // No blobs available
    );
    assert_eq!(dry_run_missing.total_projects, 2);
    assert_eq!(dry_run_missing.unassigned_projects, 1);
    assert!(!dry_run_missing.safe_to_import);
    assert_eq!(dry_run_missing.missing_blobs.len(), 1);

    // Dry-run with available blobs
    let available_blobs = vec!["hash_chart_123".to_string()];
    let dry_run_ready =
        LegacyMigrationService::dry_run(&legacy_records, "ws-brand-a", &available_blobs);
    assert!(dry_run_ready.safe_to_import);
    assert_eq!(dry_run_ready.missing_blobs.len(), 0);

    // 7. Execute Migration Import
    let import_result = LegacyMigrationService::execute_import(
        &catalog,
        &human_actor,
        &legacy_records,
        "ws-brand-a",
    )
    .unwrap();
    assert_eq!(import_result.imported_projects, 2);
    assert_eq!(import_result.imported_assets, 1);
    assert_eq!(import_result.quarantined_assets, 1); // Assets imported must default to QUARANTINED!

    // Verify imported assets in catalog
    let assets = catalog.list_assets("ws-brand-a").unwrap();
    assert!(assets.iter().any(|a| a.blob_sha256 == "hash_chart_123"
        && a.rights_state == studio_core::asset::RightsState::Quarantined));

    // Verify imported projects in catalog
    let projects = catalog.list_projects("ws-brand-a").unwrap();
    assert!(projects.iter().any(|p| p.title == "Báo cáo Social Q1 Cũ"));
    assert!(projects.iter().any(|p| p.title == "Poster Khuyến Mãi Cũ"));
}
