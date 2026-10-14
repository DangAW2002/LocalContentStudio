use studio_app::{
    ai_gateway::{AIGateway, AIProviderConfig, ProviderKind},
    ApplicationHost,
};
use studio_core::{actor::ActorContext, capabilities::CapabilityRegistry};
use studio_storage::{config::StorageConfig, initialize_storage};
use tempfile::tempdir;

#[tokio::test]
async fn test_ai_gateway_neutral_provider_and_fallback_mode() {
    let temp = tempdir().unwrap();
    let config = StorageConfig::new(temp.path());
    let (catalog, blob_storage, _) = initialize_storage(&config).unwrap();
    let caps = CapabilityRegistry::default();
    let host = ApplicationHost::new(catalog, blob_storage, caps);

    let actor = ActorContext::new_human("creator-ai", "ws-ai-lab");

    // 1. Local fallback simulated generator (No API key, No external endpoint)
    let fallback_config = AIProviderConfig {
        id: "provider-simulated".into(),
        kind: ProviderKind::Local,
        base_url: None,
        model: "llama-3-local-mock".into(),
        enabled: true,
        timeout_seconds: 10,
        max_tokens: 500,
    };

    let gateway = AIGateway::new(fallback_config, None);

    let res = host
        .generate_ai_draft(
            &actor,
            &gateway,
            "Gợi ý 3 câu slogan ra mắt dòng sản phẩm hữu cơ",
            "Bạn là trợ lý sáng tạo nội dung của thương hiệu.",
        )
        .await;

    assert!(res.is_err());
    match res.err().unwrap() {
        studio_core::error::StudioError::PreconditionFailed { reason, .. } => {
            assert!(reason.contains("No-AI mode"));
        }
        other => panic!("Expected PreconditionFailed, got {:?}", other),
    }

    // Probe check: unconfigured returns Ok(false)
    let probe_ok = gateway.probe().await.unwrap();
    assert!(!probe_ok);

    // 2. Disabled provider triggers typed PreconditionFailed error allowing No-AI manual mode
    let mut disabled_config = gateway.provider_config().clone();
    disabled_config.enabled = false;
    let disabled_gateway = AIGateway::new(disabled_config, None);

    let err = host
        .generate_ai_draft(
            &actor,
            &disabled_gateway,
            "Prompt khi AI bị tắt",
            "System prompt",
        )
        .await;

    assert!(err.is_err());
    match err.err().unwrap() {
        studio_core::error::StudioError::PreconditionFailed { reason, .. } => {
            assert!(reason.contains("No-AI mode"));
        }
        other => panic!("Expected PreconditionFailed, got {:?}", other),
    }
}
