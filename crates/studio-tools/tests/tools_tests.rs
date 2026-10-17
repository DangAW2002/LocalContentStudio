use std::fs;
use std::sync::Arc;
use studio_jobs::{
    EstimateStore, MemoryEstimateStore, ResourceSpec, Scheduler, SchedulerConfig, SysinfoProbe,
};
use studio_tools::{
    probe, remove_agy_at, route, run_tool, setup_agy_at, MemoryStore, Registry, SecretStore,
    SideEffect, ToolCost, ToolKind, ToolManifest, ToolRun, ToolRunContext, ToolRunSpec,
    ToolRunType,
};
use studio_workspace::Workspace;
use tempfile::tempdir;

fn test_scheduler() -> Arc<Scheduler> {
    let config = SchedulerConfig::default();
    let probe = Arc::new(SysinfoProbe::new());
    let estimates: Arc<dyn EstimateStore> = Arc::new(MemoryEstimateStore::new());
    Scheduler::new(config, probe, estimates)
}

#[test]
fn test_registry_override_and_invalid_manifest_reporting() {
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "test-ws").unwrap();

    // 1. Create a workspace tool overriding "echo"
    let override_dir = ws.root().join("tools").join("echo");
    fs::create_dir_all(&override_dir).unwrap();
    let override_yaml = r#"
id: echo
kind: generator
description: "Overridden echo in workspace"
provides:
  - "echo@1"
run:
  type: cli
  command: "echo"
  args: ["overridden"]
needs: []
cost:
  unit: "call"
  estimate: 0.0
side_effect: none
"#;
    fs::write(override_dir.join("tool.yaml"), override_yaml).unwrap();

    // 2. Create an invalid tool manifest
    let broken_dir = ws.root().join("tools").join("broken-tool");
    fs::create_dir_all(&broken_dir).unwrap();
    fs::write(
        broken_dir.join("tool.yaml"),
        "invalid: yaml: [syntax error:",
    )
    .unwrap();

    // Load registry
    let reg = Registry::load(&ws);

    // Verify override
    let echo_tool = reg.get_tool("echo").expect("echo should be loaded");
    assert_eq!(echo_tool.description, "Overridden echo in workspace");

    // Verify invalid manifest is reported instead of crashing
    assert!(
        !reg.problems().is_empty(),
        "Registry should record invalid manifest problems"
    );
    let problem_found = reg
        .problems()
        .iter()
        .any(|p| p.path.to_string_lossy().contains("broken-tool"));
    assert!(
        problem_found,
        "Should report problem for broken-tool/tool.yaml"
    );
}

#[test]
fn test_routing_order() {
    let tmp = tempdir().unwrap();
    let org_dir = tmp.path().join("my-org");
    fs::create_dir_all(&org_dir).unwrap();

    // Create org.yaml with default provider
    fs::write(
        org_dir.join("org.yaml"),
        r#"
name: my-org
providers:
  "tts.synthesize@1": "tts.fake"
"#,
    )
    .unwrap();

    // Create workspace inside org
    let ws_dir = org_dir.join("ws-1");
    let ws = Workspace::create(&ws_dir, "ws-1").unwrap();

    // Set workspace default provider in studio.yaml
    fs::write(
        ws.root().join("studio.yaml"),
        r#"
name: ws-1
providers:
  "tts.synthesize@1": "tts.windows-sapi"
"#,
    )
    .unwrap();

    let reg = Registry::load(&ws);
    let tools: Vec<ToolManifest> = reg.list_tools().into_iter().cloned().collect();

    // 1. Explicit provider takes precedence
    let providers = route("tts.synthesize@1", Some("tts.custom"), &ws, &tools);
    assert_eq!(providers[0], "tts.custom");
    assert_eq!(providers[1], "tts.windows-sapi");
    assert_eq!(providers[2], "tts.fake");

    // 2. Without explicit provider, workspace default is first
    let providers_no_explicit = route("tts.synthesize@1", None, &ws, &tools);
    assert_eq!(providers_no_explicit[0], "tts.windows-sapi");
    assert_eq!(providers_no_explicit[1], "tts.fake");
}

#[test]
fn test_probe_missing_binary() {
    let secret_store = MemoryStore::new();
    let mut tool = ToolManifest {
        id: "test-missing-bin".into(),
        kind: ToolKind::Checker,
        description: "Test".into(),
        provides: vec![],
        run: ToolRunSpec {
            run_type: ToolRunType::Cli,
            command: "nonexistent".into(),
            args: vec![],
        },
        needs: vec!["bin:nonexistent_binary_xyz_987654".into()],
        cost: None,
        side_effect: SideEffect::None,
        resources: ResourceSpec::default(),
        lifecycle: studio_jobs::Lifecycle::PerCall,
    };

    let result = probe(&tool, &secret_store, None);
    assert!(!result.available);
    assert_eq!(result.missing, vec!["bin:nonexistent_binary_xyz_987654"]);
    assert!(result.note_vi.contains("Thiếu phụ thuộc"));

    // Add a real binary like powershell
    tool.needs = vec!["bin:powershell".into()];
    let res_powershell = probe(&tool, &secret_store, None);
    #[cfg(windows)]
    assert!(res_powershell.available);
}

#[tokio::test]
async fn test_secret_only_reaches_env_when_declared() {
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();
    let secret_store = MemoryStore::new();

    // Set two secrets
    secret_store
        .set(
            ws.org().name(),
            Some(ws.name()),
            "ALLOWED_SECRET",
            "secret_value_123",
        )
        .unwrap();
    secret_store
        .set(
            ws.org().name(),
            Some(ws.name()),
            "FORBIDDEN_SECRET",
            "super_secret_456",
        )
        .unwrap();

    let mut reg = Registry::new();
    // Tool only asks for ALLOWED_SECRET
    let tool = ToolManifest {
        id: "secret-reader".into(),
        kind: ToolKind::Generator,
        description: "Echo env test".into(),
        provides: vec!["secret.test@1".into()],
        run: ToolRunSpec {
            run_type: ToolRunType::Cli,
            #[cfg(windows)]
            command: "powershell".into(),
            #[cfg(not(windows))]
            command: "sh".into(),
            #[cfg(windows)]
            args: vec![
                "-NoProfile".into(),
                "-Command".into(),
                r#"[PSCustomObject]@{ allowed = $env:ALLOWED_SECRET; forbidden = $env:FORBIDDEN_SECRET } | ConvertTo-Json -Compress"#.into(),
            ],
            #[cfg(not(windows))]
            args: vec![
                "-c".into(),
                r#"echo "{\"allowed\":\"$ALLOWED_SECRET\",\"forbidden\":\"$FORBIDDEN_SECRET\"}""#.into(),
            ],
        },
        needs: vec!["secret:ALLOWED_SECRET".into()],
        cost: None,
        side_effect: SideEffect::None,
        resources: ResourceSpec::default(),
        lifecycle: studio_jobs::Lifecycle::PerCall,
    };
    reg.register_tool(tool);

    let scheduler = test_scheduler();
    let ctx = ToolRunContext {
        workspace: &ws,
        turn_id: Some("turn-1".into()),
        budget: Some(10.0),
        approved: true,
        scheduler: &scheduler,
        secret_store: &secret_store,
        endpoints: std::collections::HashMap::new(),
        pricing: std::collections::HashMap::new(),
    };

    let result = run_tool(&reg, "secret-reader", serde_json::json!({}), &ctx)
        .await
        .unwrap();

    match result {
        ToolRun::Success { output, .. } => {
            assert_eq!(output["allowed"], "secret_value_123");
            // FORBIDDEN_SECRET was not declared in tool.needs, so it MUST NOT be in env!
            assert!(output["forbidden"].is_null() || output["forbidden"] == "");
        }
        other => panic!("Expected ToolRun::Success, got {:?}", other),
    }
}

#[tokio::test]
async fn test_run_tool_echo_end_to_end_through_scheduler() {
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();
    let reg = Registry::load(&ws);
    let secret_store = MemoryStore::new();
    let scheduler = test_scheduler();

    let ctx = ToolRunContext {
        workspace: &ws,
        turn_id: Some("turn-test".into()),
        budget: Some(5.0),
        approved: false,
        scheduler: &scheduler,
        secret_store: &secret_store,
        endpoints: std::collections::HashMap::new(),
        pricing: std::collections::HashMap::new(),
    };

    let input_payload = serde_json::json!({
        "message": "Hello from Local Content Studio",
        "nested": { "count": 42 }
    });

    let res = run_tool(&reg, "echo", input_payload.clone(), &ctx)
        .await
        .expect("echo tool execution failed");

    match res {
        ToolRun::Success { output, .. } => {
            assert_eq!(output, input_payload);
        }
        other => panic!("Expected ToolRun::Success, got {:?}", other),
    }
}

#[tokio::test]
async fn test_paid_over_budget_needs_approval() {
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();
    let secret_store = MemoryStore::new();
    let scheduler = test_scheduler();

    let mut reg = Registry::new();
    let paid_tool = ToolManifest {
        id: "expensive-generator".into(),
        kind: ToolKind::Generator,
        description: "Paid AI generation".into(),
        provides: vec!["video.gen@1".into()],
        run: ToolRunSpec {
            run_type: ToolRunType::Cli,
            command: "echo".into(),
            args: vec!["done".into()],
        },
        needs: vec![],
        cost: Some(ToolCost {
            unit: "second".into(),
            estimate: serde_json::json!(0.50), // Costs $0.50
        }),
        side_effect: SideEffect::Paid,
        resources: ResourceSpec::default(),
        lifecycle: studio_jobs::Lifecycle::PerCall,
    };
    reg.register_tool(paid_tool);

    // Remaining budget is only $0.10, but tool costs $0.50 and approved = false
    let ctx = ToolRunContext {
        workspace: &ws,
        turn_id: Some("turn-budget".into()),
        budget: Some(0.10),
        approved: false,
        scheduler: &scheduler,
        secret_store: &secret_store,
        endpoints: std::collections::HashMap::new(),
        pricing: std::collections::HashMap::new(),
    };

    let outcome = run_tool(&reg, "expensive-generator", serde_json::json!({}), &ctx)
        .await
        .unwrap();

    match outcome {
        ToolRun::NeedsApproval {
            reason,
            side_effect,
        } => {
            assert_eq!(side_effect, SideEffect::Paid);
            assert!(reason.contains("vượt quá ngân sách còn lại"));
        }
        other => panic!("Expected NeedsApproval, got {:?}", other),
    }

    // Now re-run with approved = true
    let approved_ctx = ToolRunContext {
        workspace: &ws,
        turn_id: Some("turn-budget".into()),
        budget: Some(0.10),
        approved: true,
        scheduler: &scheduler,
        secret_store: &secret_store,
        endpoints: std::collections::HashMap::new(),
        pricing: std::collections::HashMap::new(),
    };

    let approved_outcome = run_tool(
        &reg,
        "expensive-generator",
        serde_json::json!({}),
        &approved_ctx,
    )
    .await
    .unwrap();

    assert!(matches!(approved_outcome, ToolRun::Success { .. }));
}

#[test]
fn test_setup_agy_edits_settings_preserving_keys() {
    let tmp = tempdir().unwrap();
    let settings_file = tmp.path().join("settings.json");

    // Initial settings with custom user config
    let initial_json = r#"{
  "theme": "dark",
  "custom_key": 12345,
  "permissions": {
    "allow": ["file_read", "command(git)", "mcp(studio)"]
  }
}"#;
    fs::write(&settings_file, initial_json).unwrap();

    let fake_mcp_exe = tmp.path().join("studio-mcp.exe");
    fs::write(&fake_mcp_exe, "binary").unwrap();

    // 1. Setup agy without running real cli
    setup_agy_at(&settings_file, &fake_mcp_exe, false).unwrap();

    // Verify settings content
    let content = fs::read_to_string(&settings_file).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();

    // Preserved existing keys
    assert_eq!(parsed["theme"], "dark");
    assert_eq!(parsed["custom_key"], 12345);

    // Added mcp(studio)
    let allow = parsed["permissions"]["allow"].as_array().unwrap();
    assert!(allow.contains(&serde_json::json!("file_read")));
    assert!(allow.contains(&serde_json::json!("command(git)")));
    assert!(allow.contains(&serde_json::json!("mcp(studio/*)")));
    // The legacy rule (no longer matched by agy 1.4) is migrated away.
    assert!(!allow.contains(&serde_json::json!("mcp(studio)")));

    // 2. Remove agy
    remove_agy_at(&settings_file, false).unwrap();

    let content_after = fs::read_to_string(&settings_file).unwrap();
    let parsed_after: serde_json::Value = serde_json::from_str(&content_after).unwrap();

    // mcp(studio) is removed, but everything else is preserved
    assert_eq!(parsed_after["theme"], "dark");
    assert_eq!(parsed_after["custom_key"], 12345);
    let allow_after = parsed_after["permissions"]["allow"].as_array().unwrap();
    assert!(!allow_after.contains(&serde_json::json!("mcp(studio/*)")));
    assert!(!allow_after.contains(&serde_json::json!("mcp(studio)")));
    assert!(allow_after.contains(&serde_json::json!("file_read")));
}

#[test]
fn test_setup_agy_refuses_to_overwrite_unparseable_settings() {
    let tmp = tempdir().unwrap();
    let settings_file = tmp.path().join("settings.json");
    let original = "{
  // user comment
  \"theme\": \"dark\"
}";
    fs::write(&settings_file, original).unwrap();
    let fake_mcp_exe = tmp.path().join("studio-mcp.exe");
    fs::write(&fake_mcp_exe, "binary").unwrap();

    assert!(setup_agy_at(&settings_file, &fake_mcp_exe, false).is_err());
    assert_eq!(fs::read_to_string(&settings_file).unwrap(), original);
}

#[test]
fn test_builtins_parse_without_problems() {
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();
    let reg = Registry::load(&ws);
    let builtin_problems: Vec<_> = reg
        .problems()
        .iter()
        .filter(|p| p.path.to_string_lossy().starts_with("builtin:"))
        .collect();
    assert!(builtin_problems.is_empty(), "{builtin_problems:?}");
}

#[tokio::test]
#[ignore = "real Windows SAPI speech synthesis"]
async fn real_sapi_tts_synthesizes_vietnamese_wav() {
    if !cfg!(windows) {
        println!("SKIP: Windows SAPI is only supported on Windows");
        return;
    }

    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "test-ws").unwrap();
    let reg = Registry::load(&ws);
    let scheduler = test_scheduler();
    let secret_store = MemoryStore::new();

    let ctx = ToolRunContext {
        workspace: &ws,
        turn_id: None,
        budget: Some(10.0),
        approved: true,
        scheduler: &scheduler,
        secret_store: &secret_store,
        endpoints: std::collections::HashMap::new(),
        pricing: std::collections::HashMap::new(),
    };

    let input = serde_json::json!({
        "text": "Xin chào! Đây là bản ghi âm giọng nói tiếng Việt thử nghiệm từ Local Content Studio.",
        "output_path": "vietnamese_sample.wav"
    });

    let run_res = run_tool(&reg, "tts.windows-sapi", input, &ctx)
        .await
        .expect("tts.windows-sapi failed to run");

    let output = match run_res {
        ToolRun::Success { output, .. } => output,
        other => panic!("Expected ToolRun::Success, got {:?}", other),
    };

    println!(
        "SAPI output: {}",
        serde_json::to_string_pretty(&output).unwrap()
    );

    let audio_path_str = output["audio_path"]
        .as_str()
        .expect("audio_path missing in output");
    let audio_path = std::path::PathBuf::from(audio_path_str);
    assert!(
        audio_path.exists(),
        "Synthesized WAV file does not exist: {}",
        audio_path.display()
    );

    // Check voice reporting
    let voice_used = output["voice"].as_str().expect("voice missing in output");
    assert!(!voice_used.is_empty(), "voice used should not be empty");

    let available = output["available_voices"]
        .as_array()
        .expect("available_voices missing");
    assert!(
        !available.is_empty(),
        "available_voices should not be empty on Windows"
    );
    println!("Available voices: {:?}", available);
    println!("Voice used: {}", voice_used);
    if let Some(note) = output["note"].as_str() {
        println!("Voice note: {}", note);
    }

    // Validate WAV header and duration
    let wav_bytes = fs::read(&audio_path).expect("Failed to read WAV file");
    assert!(
        wav_bytes.len() > 44,
        "WAV file too short: {} bytes",
        wav_bytes.len()
    );
    assert_eq!(&wav_bytes[0..4], b"RIFF", "Missing RIFF header");
    assert_eq!(&wav_bytes[8..12], b"WAVE", "Missing WAVE identifier");
    assert_eq!(&wav_bytes[12..16], b"fmt ", "Missing fmt chunk");

    // Channels (offset 22, 2 bytes LE)
    let channels = u16::from_le_bytes([wav_bytes[22], wav_bytes[23]]);
    // Sample rate (offset 24, 4 bytes LE)
    let sample_rate =
        u32::from_le_bytes([wav_bytes[24], wav_bytes[25], wav_bytes[26], wav_bytes[27]]);
    // Byte rate (offset 28, 4 bytes LE)
    let byte_rate =
        u32::from_le_bytes([wav_bytes[28], wav_bytes[29], wav_bytes[30], wav_bytes[31]]);

    assert!(channels >= 1, "Invalid channels: {}", channels);
    assert!(sample_rate >= 8000, "Invalid sample rate: {}", sample_rate);
    assert!(byte_rate > 0, "Invalid byte rate: {}", byte_rate);

    // Find "data" chunk
    let mut data_size = 0u32;
    let mut cursor = 12;
    while cursor + 8 <= wav_bytes.len() {
        let chunk_id = &wav_bytes[cursor..cursor + 4];
        let chunk_len = u32::from_le_bytes([
            wav_bytes[cursor + 4],
            wav_bytes[cursor + 5],
            wav_bytes[cursor + 6],
            wav_bytes[cursor + 7],
        ]);
        if chunk_id == b"data" {
            data_size = chunk_len;
            break;
        }
        cursor += 8 + (chunk_len as usize);
    }
    assert!(data_size > 0, "data chunk not found or empty in WAV");

    let duration_secs = (data_size as f64) / (byte_rate as f64);
    println!(
        "WAV Duration: {:.2}s, Size: {} bytes, Channels: {}, SampleRate: {} Hz",
        duration_secs,
        wav_bytes.len(),
        channels,
        sample_rate
    );
    assert!(
        duration_secs > 0.5,
        "Duration should be > 0.5s for spoken sentence, got {:.2}s",
        duration_secs
    );
}
