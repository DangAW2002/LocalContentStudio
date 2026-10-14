use studio_app::{
    ConfirmPublishInput, CreateProjectInput, ImportAssetInput, PrepareDeliveryInput,
    SaveRevisionInput,
};
use studio_core::{
    actor::ActorContext,
    asset::{AssetKind, AssetScope},
    capabilities::CapabilityRegistry,
    document::{
        Artboard, ArtboardBackground, BoundingBox, CreativeDocument, Element, ElementSemanticRole,
        ElementStyle, ElementType,
    },
    error::StudioError,
    handoff::{DeliveryMediaItem, HandoffStatus, OutboxState, SocialPublicationState},
    operation::OperationEnvelope,
    project::ProjectType,
};
use studio_storage::{config::StorageConfig, initialize_storage};
use tempfile::tempdir;

#[test]
fn test_social_handoff_lifecycle_and_security_gates() {
    let temp = tempdir().unwrap();
    let config = StorageConfig::new(temp.path());
    let (catalog, blob_storage, _) = initialize_storage(&config).unwrap();
    let caps = CapabilityRegistry::default();
    let host = studio_app::ApplicationHost::new(catalog.clone(), blob_storage, caps);

    let session = host.authenticate_local_os_user("ws-marketing").unwrap();
    let human_actor = ActorContext::new_human(&session.subject_id, "ws-marketing");
    let agent_actor = ActorContext::new_agent("ai-bot", "ws-marketing");
    let foreign_actor = ActorContext::new_human("foreign-user", "ws-foreign");

    // 1. Create project & revision
    let proj_res = host
        .create_project(
            &human_actor,
            OperationEnvelope::new(
                "project.create",
                CreateProjectInput {
                    title: "Campaign Q4 Launch".into(),
                    project_type: ProjectType::Poster,
                },
            ),
        )
        .unwrap();
    let project = proj_res.result;

    let mut doc = CreativeDocument::new("doc-poster-1");
    doc.artboards.push(Artboard {
        id: "ab-1".into(),
        width: 1080,
        height: 1350,
        background: ArtboardBackground {
            token: "brand.surface".into(),
        },
        elements: vec![],
    });

    let rev_res = host
        .save_revision(
            &human_actor,
            OperationEnvelope::new(
                "revision.save",
                SaveRevisionInput {
                    project_id: project.id.clone(),
                    document: doc.clone(),
                    summary: "Initial poster revision".into(),
                },
            ),
        )
        .unwrap();
    let rev1 = rev_res.result;

    // 2. Check Readiness: missing media -> blocked
    let ready_res1 = host
        .check_publish_readiness(&human_actor, &project.id, &rev1.id, "fb-page-12345", &[])
        .unwrap();
    assert!(!ready_res1.is_ready);
    assert!(ready_res1.blockers.iter().any(|b| b.contains("media")));

    // 3. Attach media and unapproved asset with genuine PNG magic bytes
    let asset_res = host
        .import_asset_bytes(
            &human_actor,
            OperationEnvelope::new(
                "asset.import",
                ImportAssetInput {
                    scope: AssetScope::Workspace,
                    kind: AssetKind::Image,
                    original_name: "product.png".into(),
                    declared_mime: "image/png".into(),
                    verified_mime: "image/png".into(),
                    provenance: "Unverified upload".into(),
                    bytes: b"\x89PNG\r\n\x1a\nUNAPPROVED_PNG".to_vec(),
                },
            ),
        )
        .unwrap();
    let quarantined_asset = asset_res.result;

    // Add quarantined asset to document
    doc.artboards[0].elements.push(Element {
        id: "el-img-1".into(),
        element_type: ElementType::Image,
        semantic_role: ElementSemanticRole::ProductImage,
        text: None,
        asset_ref: Some(quarantined_asset.id.clone()),
        box_model: BoundingBox {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        },
        style: ElementStyle {
            font_token: None,
            color_token: None,
            font_size: None,
            text_align: None,
        },
        locked: false,
        visible: true,
    });

    let rev2_res = host
        .save_revision(
            &human_actor,
            OperationEnvelope::new(
                "revision.save",
                SaveRevisionInput {
                    project_id: project.id.clone(),
                    document: doc,
                    summary: "Added quarantined product image".into(),
                },
            ),
        )
        .unwrap();
    let rev2 = rev2_res.result;

    let media_item = DeliveryMediaItem {
        asset_version_id: "media-v1".into(),
        sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
        mime: "image/png".into(),
        bytes: 1024,
    };

    // Readiness blocked by QUARANTINED asset!
    let ready_res2 = host
        .check_publish_readiness(
            &human_actor,
            &project.id,
            &rev2.id,
            "fb-page-12345",
            std::slice::from_ref(&media_item),
        )
        .unwrap();
    assert!(!ready_res2.is_ready);
    assert!(ready_res2
        .blockers
        .iter()
        .any(|b| b.contains("QUARANTINED")));

    // 4. Human attests rights -> Readiness clears!
    let attest_hash = studio_app::ApplicationHost::compute_action_hash(&quarantined_asset.id);
    let attest_proof = host
        .issue_human_proof(&session, "rights.attest", &attest_hash)
        .unwrap();

    host.attest_asset_rights(
        &human_actor,
        OperationEnvelope::new(
            "rights.attest",
            studio_app::AttestRightsInput {
                asset_id: quarantined_asset.id,
                source: "Company photographer".into(),
                license_basis: "Work for hire".into(),
                attribution_text: None,
                allowed_uses: vec!["all".into()],
            },
        )
        .with_human_proof(&attest_proof.proof_id),
    )
    .unwrap();

    let ready_res3 = host
        .check_publish_readiness(
            &human_actor,
            &project.id,
            &rev2.id,
            "fb-page-12345",
            std::slice::from_ref(&media_item),
        )
        .unwrap();
    assert!(ready_res3.is_ready);

    // 5. Foreign actor scope check
    let foreign_ready = host.check_publish_readiness(
        &foreign_actor,
        &project.id,
        &rev2.id,
        "fb-page-12345",
        std::slice::from_ref(&media_item),
    );
    assert!(foreign_ready.is_err());

    // 6. Prepare DeliveryPackage (Immutable snapshot)
    let prep_op = OperationEnvelope::new(
        "delivery.prepare",
        PrepareDeliveryInput {
            project_id: project.id.clone(),
            revision_id: rev2.id.clone(),
            platform: "facebook".into(),
            caption: "Chào đón sản phẩm mới tháng 10! #NewRelease".into(),
            page_id: "fb-page-12345".into(),
            media: vec![media_item],
        },
    );
    let pkg_res = host
        .prepare_delivery_package(&human_actor, prep_op)
        .unwrap();
    let delivery_pkg = pkg_res.result;
    assert_eq!(delivery_pkg.status, HandoffStatus::Prepared);
    assert_eq!(delivery_pkg.target_selectors.page_id, "fb-page-12345");

    // 7. Agent trying to confirm publish MUST be denied with ForbiddenHumanOnly!
    let confirm_op = OperationEnvelope::new(
        "publish.confirm",
        ConfirmPublishInput {
            delivery_id: delivery_pkg.delivery_id.clone(),
        },
    );
    let agent_confirm_err = host.confirm_publish_and_dispatch(&agent_actor, confirm_op.clone());
    assert!(agent_confirm_err.is_err());
    match agent_confirm_err.err().unwrap() {
        StudioError::ForbiddenHumanOnly { operation, .. } => {
            assert_eq!(operation, "publish.confirm");
        }
        other => panic!("Expected ForbiddenHumanOnly, got {:?}", other),
    }

    // 8. Human confirms publish with proof -> dispatched to Outbox, state Accepted from receipt
    let confirm_hash = studio_app::ApplicationHost::compute_action_hash(&delivery_pkg.delivery_id);
    let confirm_proof = host
        .issue_human_proof(&session, "publish.confirm", &confirm_hash)
        .unwrap();

    let dispatch_res = host
        .confirm_publish_and_dispatch(
            &human_actor,
            confirm_op.with_human_proof(&confirm_proof.proof_id),
        )
        .unwrap();
    let outbox_item = dispatch_res.result;
    assert_eq!(outbox_item.destination, "social-gateway:facebook");
    assert_eq!(outbox_item.state, OutboxState::Accepted);
    assert!(outbox_item.remote_ref.is_some());

    // Check delivery package in catalog is updated to Accepted
    let stored_pkg = catalog
        .get_delivery_package(&delivery_pkg.delivery_id)
        .unwrap()
        .unwrap();
    assert_eq!(stored_pkg.status, HandoffStatus::Accepted);

    // 9. Stale approval rejection: if revision changes after package was created, second dispatch fails
    // Let's create another package for rev2, then create rev3
    let prep_stale_op = OperationEnvelope::new(
        "delivery.prepare",
        PrepareDeliveryInput {
            project_id: project.id.clone(),
            revision_id: rev2.id.clone(),
            platform: "facebook".into(),
            caption: "Bản thử nghiệm stale".into(),
            page_id: "fb-page-12345".into(),
            media: vec![],
        },
    );
    let stale_pkg = host
        .prepare_delivery_package(&human_actor, prep_stale_op)
        .unwrap()
        .result;

    // Advance project to rev3
    host.save_revision(
        &human_actor,
        OperationEnvelope::new(
            "revision.save",
            SaveRevisionInput {
                project_id: project.id.clone(),
                document: CreativeDocument::new("doc-poster-v3"),
                summary: "Advance to revision 3".into(),
            },
        ),
    )
    .unwrap();

    // Now confirm publish for stale_pkg should be rejected due to stale approval!
    let stale_hash = studio_app::ApplicationHost::compute_action_hash(&stale_pkg.delivery_id);
    let stale_proof = host
        .issue_human_proof(&session, "publish.confirm", &stale_hash)
        .unwrap();

    let stale_confirm = host.confirm_publish_and_dispatch(
        &human_actor,
        OperationEnvelope::new(
            "publish.confirm",
            ConfirmPublishInput {
                delivery_id: stale_pkg.delivery_id.clone(),
            },
        )
        .with_human_proof(&stale_proof.proof_id),
    );
    assert!(stale_confirm.is_err());
    match stale_confirm.err().unwrap() {
        StudioError::PreconditionFailed { reason, .. } => {
            assert!(reason.contains("stale approval"));
        }
        other => panic!("Expected PreconditionFailed, got {:?}", other),
    }

    // 10. Social status projection test
    let proj_verified = studio_app::social_gateway::SocialGatewayService::project_social_status(
        &delivery_pkg.delivery_id,
        SocialPublicationState::Verified,
        Some("https://facebook.com/post/12345"),
        None,
    );
    assert_eq!(proj_verified.state, SocialPublicationState::Verified);
    assert_eq!(
        proj_verified.permalink.as_deref(),
        Some("https://facebook.com/post/12345")
    );

    // Network drop -> UNKNOWN status projection requires reconciliation
    let proj_unknown = studio_app::social_gateway::SocialGatewayService::project_social_status(
        &delivery_pkg.delivery_id,
        SocialPublicationState::Unknown,
        None,
        Some("Network timeout during gateway ack. Reconcile required."),
    );
    assert_eq!(proj_unknown.state, SocialPublicationState::Unknown);
    assert!(proj_unknown
        .error_message
        .unwrap()
        .contains("Reconcile required"));
}
