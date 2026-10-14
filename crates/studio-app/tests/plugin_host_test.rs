use studio_app::plugin_host::{
    PluginHostEngine, PluginIntegrity, PluginManifest, PluginPermissions, PluginStatus,
};
use studio_core::error::StudioError;

#[test]
fn test_plugin_host_security_gates_and_sandboxed_hooks() {
    let mut engine = PluginHostEngine::new();
    let current_os = std::env::consts::OS.to_string();

    // 1. Valid Internal Adapter Plugin
    let valid_manifest = PluginManifest {
        id: "com.studio.watermark".into(),
        version: "1.0.0".into(),
        sdk_version: PluginManifest::SDK_V1.into(),
        kind: "internal-adapter".into(),
        capabilities: vec!["asset.watermark".into(), "export.filter".into()],
        permissions: PluginPermissions {
            workspace_assets: "selected-read".into(),
            network_hosts: vec![],
            allow_process_exec: false,
        },
        entrypoint: "internal::WatermarkFilter".into(),
        supported_os: vec![current_os.clone()],
        dependencies: vec![],
        integrity: PluginIntegrity {
            sha256: "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08".into(),
        },
    };

    let reg_valid = engine.register_plugin(valid_manifest);
    assert_eq!(reg_valid.status, PluginStatus::Active);

    // 2. Reject Incompatible SDK Version Handshake
    let bad_sdk_manifest = PluginManifest {
        id: "com.bad.sdk".into(),
        version: "1.0.0".into(),
        sdk_version: "studio.plugin/v99-unsupported".into(),
        kind: "internal-adapter".into(),
        capabilities: vec![],
        permissions: PluginPermissions {
            workspace_assets: "none".into(),
            network_hosts: vec![],
            allow_process_exec: false,
        },
        entrypoint: "main".into(),
        supported_os: vec![current_os.clone()],
        dependencies: vec![],
        integrity: PluginIntegrity {
            sha256: "abcdef".into(),
        },
    };
    let reg_bad_sdk = engine.register_plugin(bad_sdk_manifest);
    assert_eq!(reg_bad_sdk.status, PluginStatus::Rejected);
    assert!(reg_bad_sdk
        .rejection_reason
        .unwrap()
        .contains("Incompatible SDK version"));

    // 3. Reject Unsupported OS
    let foreign_os_manifest = PluginManifest {
        id: "com.foreign.os".into(),
        version: "1.0.0".into(),
        sdk_version: PluginManifest::SDK_V1.into(),
        kind: "internal-adapter".into(),
        capabilities: vec![],
        permissions: PluginPermissions {
            workspace_assets: "none".into(),
            network_hosts: vec![],
            allow_process_exec: false,
        },
        entrypoint: "main".into(),
        supported_os: vec!["solaris-unknown".into()],
        dependencies: vec![],
        integrity: PluginIntegrity {
            sha256: "abcdef".into(),
        },
    };
    let reg_foreign_os = engine.register_plugin(foreign_os_manifest);
    assert_eq!(reg_foreign_os.status, PluginStatus::Rejected);
    assert!(reg_foreign_os
        .rejection_reason
        .unwrap()
        .contains("Unsupported operating system"));

    // 4. Reject Missing Integrity Hash
    let empty_hash_manifest = PluginManifest {
        id: "com.no.hash".into(),
        version: "1.0.0".into(),
        sdk_version: PluginManifest::SDK_V1.into(),
        kind: "internal-adapter".into(),
        capabilities: vec![],
        permissions: PluginPermissions {
            workspace_assets: "none".into(),
            network_hosts: vec![],
            allow_process_exec: false,
        },
        entrypoint: "main".into(),
        supported_os: vec![current_os.clone()],
        dependencies: vec![],
        integrity: PluginIntegrity { sha256: "".into() },
    };
    let reg_no_hash = engine.register_plugin(empty_hash_manifest);
    assert_eq!(reg_no_hash.status, PluginStatus::Rejected);
    assert!(reg_no_hash
        .rejection_reason
        .unwrap()
        .contains("Missing integrity SHA-256"));

    // 5. External Process requesting broad 'full' asset access defaults to QUARANTINED
    let dangerous_external_manifest = PluginManifest {
        id: "com.dangerous.external".into(),
        version: "2.0.0".into(),
        sdk_version: PluginManifest::SDK_V1.into(),
        kind: "external-process".into(),
        capabilities: vec!["render.3d".into()],
        permissions: PluginPermissions {
            workspace_assets: "full".into(), // Dangerous wildcard!
            network_hosts: vec![],
            allow_process_exec: true,
        },
        entrypoint: "renderer.exe".into(),
        supported_os: vec![current_os],
        dependencies: vec![],
        integrity: PluginIntegrity {
            sha256: "1234567890abcdef".into(),
        },
    };
    let reg_quarantined = engine.register_plugin(dangerous_external_manifest);
    assert_eq!(reg_quarantined.status, PluginStatus::Quarantined);
    assert!(reg_quarantined
        .rejection_reason
        .unwrap()
        .contains("Quarantined"));

    // 6. Sandboxed Hook Execution
    let sample_payload = b"IMAGE_BINARY_FOR_WATERMARK";
    let exec_res = engine
        .execute_sandboxed_hook("com.studio.watermark", "asset.watermark", sample_payload)
        .unwrap();
    assert_eq!(exec_res.status, "SUCCESS");
    assert!(!exec_res.output_hash.is_empty());
    assert_eq!(exec_res.output_bytes, sample_payload);

    // 7. Requesting Undeclared Capability fails safely
    let bad_cap_res = engine.execute_sandboxed_hook(
        "com.studio.watermark",
        "undeclared.dangerous_op",
        sample_payload,
    );
    assert!(bad_cap_res.is_err());
    match bad_cap_res.err().unwrap() {
        StudioError::UnsupportedCapability { capability, .. } => {
            assert!(capability.contains("does not declare required capability"));
        }
        other => panic!("Expected UnsupportedCapability, got {:?}", other),
    }

    // 8. Attempting to execute Quarantined plugin fails safely
    let quarantined_exec =
        engine.execute_sandboxed_hook("com.dangerous.external", "render.3d", sample_payload);
    assert!(quarantined_exec.is_err());
}
