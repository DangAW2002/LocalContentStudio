use studio_app::{
    brand_validator::BrandValidator, ApplicationHost, CreateBrandKitInput,
    ProposeBrandVersionInput, PublishBrandVersionInput,
};
use studio_core::{
    actor::ActorContext,
    brand::{BrandVersionState, VisualTokens, VoiceGuidelines},
    capabilities::CapabilityRegistry,
    error::StudioError,
    operation::OperationEnvelope,
};
use studio_storage::{config::StorageConfig, initialize_storage};
use tempfile::tempdir;

#[test]
fn test_brand_deterministic_validators_and_contrast() {
    let voice = VoiceGuidelines {
        tone: "Chuyên nghiệp, tin cậy".into(),
        prohibited_terms: vec!["rẻ tiền".into(), "lừa đảo".into(), "cam kết 100%".into()],
        required_disclaimers: vec!["Kết quả phụ thuộc vào từng trường hợp cụ thể.".into()],
    };

    // 1. Prohibited terms trigger violation
    let bad_copy = "Sản phẩm của chúng tôi không hề rẻ tiền và cam kết 100% hiệu quả!";
    let res1 = BrandValidator::validate_copy(bad_copy, &voice);
    assert!(!res1.is_valid);
    assert_eq!(res1.violations.len(), 3); // 2 banned terms + 1 missing disclaimer

    // 2. Clean copy with disclaimer passes
    let good_copy = "Giải pháp tối ưu dành cho doanh nghiệp hiện đại. Kết quả phụ thuộc vào từng trường hợp cụ thể.";
    let res2 = BrandValidator::validate_copy(good_copy, &voice);
    assert!(res2.is_valid);
    assert!(res2.violations.is_empty());

    // 3. Contrast calculation
    let good_tokens = VisualTokens {
        primary_color: "#0f172a".into(), // Dark navy
        secondary_color: "#3b82f6".into(),
        surface_color: "#ffffff".into(), // White surface
        heading_font: "Inter".into(),
        body_font: "Inter".into(),
    };
    let ratio_high = BrandValidator::validate_contrast(&good_tokens).unwrap();
    assert!(ratio_high > 10.0); // High contrast ~16:1

    let bad_contrast_tokens = VisualTokens {
        primary_color: "#e2e8f0".into(), // Very light gray on white
        secondary_color: "#94a3b8".into(),
        surface_color: "#ffffff".into(),
        heading_font: "Inter".into(),
        body_font: "Inter".into(),
    };
    let ratio_low = BrandValidator::validate_contrast(&bad_contrast_tokens).unwrap();
    assert!(ratio_low < 2.0); // Fails WCAG AA
}

#[test]
fn test_brand_lifecycle_proposal_and_human_only_publish() {
    let temp = tempdir().unwrap();
    let config = StorageConfig::new(temp.path());
    let (catalog, blob_storage, _) = initialize_storage(&config).unwrap();
    let caps = CapabilityRegistry::default();
    let host = ApplicationHost::new(catalog, blob_storage, caps);

    let human_actor = ActorContext::new_human("lead-designer", "ws-brandco");
    let agent_actor = ActorContext::new_agent("copywriter-bot", "ws-brandco");

    // 1. Create Brand Kit
    let create_kit_op = OperationEnvelope::new(
        "brand.create",
        CreateBrandKitInput {
            name: "Bộ nhận diện BrandCo 2026".into(),
        },
    );
    let kit_res = host.create_brand_kit(&human_actor, create_kit_op).unwrap();
    let kit = kit_res.result;
    assert_eq!(kit.name, "Bộ nhận diện BrandCo 2026");
    assert!(kit.active_version_id.is_none());

    // 2. Agent proposes BrandVersion (DRAFT)
    let propose_op = OperationEnvelope::new(
        "brand.propose",
        ProposeBrandVersionInput {
            brand_kit_id: kit.id.clone(),
            visual_tokens: VisualTokens {
                primary_color: "#1e293b".into(),
                secondary_color: "#0284c7".into(),
                surface_color: "#ffffff".into(),
                heading_font: "Plus Jakarta Sans".into(),
                body_font: "Inter".into(),
            },
            voice: VoiceGuidelines {
                tone: "Thân thiện, truyền cảm hứng".into(),
                prohibited_terms: vec!["tuyệt đối".into()],
                required_disclaimers: vec![],
            },
            logos: vec!["logo-dark.png".into()],
        },
    );

    let ver_res = host
        .propose_brand_version(&agent_actor, propose_op)
        .unwrap();
    let draft_ver = ver_res.result;
    assert_eq!(draft_ver.state, BrandVersionState::Draft);
    assert_eq!(draft_ver.version_number, 1);

    // 3. Agent attempts to publish BrandVersion -> REJECTED (Human-only gate)
    let agent_pub_op = OperationEnvelope::new(
        "brand.publish",
        PublishBrandVersionInput {
            brand_version_id: draft_ver.id.clone(),
        },
    );
    let agent_err = host.publish_brand_version(&agent_actor, agent_pub_op);
    assert!(agent_err.is_err());
    match agent_err.err().unwrap() {
        StudioError::ForbiddenHumanOnly { operation, .. } => {
            assert_eq!(operation, "brand.publish");
        }
        other => panic!("Expected ForbiddenHumanOnly, got {:?}", other),
    }

    // 4. Human without proof -> REJECTED with HumanConfirmationRequired
    let human_pub_op = OperationEnvelope::new(
        "brand.publish",
        PublishBrandVersionInput {
            brand_version_id: draft_ver.id.clone(),
        },
    );
    let no_proof_err = host.publish_brand_version(&human_actor, human_pub_op.clone());
    assert!(no_proof_err.is_err());
    match no_proof_err.err().unwrap() {
        StudioError::HumanConfirmationRequired { operation, .. } => {
            assert_eq!(operation, "brand.publish");
        }
        other => panic!("Expected HumanConfirmationRequired, got {:?}", other),
    }

    // 5. Human with valid proof publishes BrandVersion -> PUBLISHED & Set active in kit
    let session = host.authenticate_local_os_user("ws-brandco").unwrap();
    let session_human = ActorContext::new_human(&session.subject_id, "ws-brandco");
    let action_hash = ApplicationHost::compute_action_hash(&draft_ver.id);
    let proof = host
        .issue_human_proof(&session, "brand.publish", &action_hash)
        .unwrap();
    let pub_res = host
        .publish_brand_version(
            &session_human,
            human_pub_op.with_human_proof(proof.proof_id),
        )
        .unwrap();
    let published_ver = pub_res.result;
    assert_eq!(published_ver.state, BrandVersionState::Published);

    // Verify BrandKit active version updated
    let reloaded_kit = host.get_brand_kit(&session_human, &kit.id).unwrap();
    assert_eq!(reloaded_kit.active_version_id, Some(draft_ver.id.clone()));
}
