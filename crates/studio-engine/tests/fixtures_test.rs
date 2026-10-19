use std::fs;
use std::path::Path;
use studio_engine::{
    AgyEngine, AgyParser, ClaudeEngine, ClaudeParser, DoneStatus, Engine, FileOp, Transcript,
};

#[test]
fn test_claude_turn1_mcp_edit() {
    let fixture_path = Path::new("tests/fixtures/claude/turn1-mcp-edit.ndjson");
    let content = fs::read_to_string(fixture_path).expect("read fixture turn1");

    let mut parser = ClaudeParser::new();
    let mut transcript = Transcript::new();

    for line in content.lines() {
        let events = parser.feed(line);
        transcript.ingest(&events);
    }

    // Assert session id
    assert_eq!(
        transcript.session_id.as_deref(),
        Some("ebfe2498-c3df-47ad-9e5d-2a04365b7be5")
    );

    // Assert ToolCall mcp__studio__studio_ping
    let has_studio_ping = content.lines().any(|l| {
        let evs = ClaudeParser::new().feed(l);
        evs.iter().any(|e| match e {
            studio_engine::Event::ToolCall { name, .. } => name == "mcp__studio__studio_ping",
            _ => false,
        })
    });
    assert!(
        has_studio_ping,
        "must have ToolCall mcp__studio__studio_ping"
    );

    // Assert FileChange hello.md
    let has_hello_file = transcript
        .file_changes
        .iter()
        .any(|(p, op)| p.ends_with("hello.md") && *op == FileOp::Write);
    assert!(has_hello_file, "must record FileChange for hello.md");

    // Assert cost ~ 0.00482
    // Token breakdown from the result line (cache read vs. cache write/warm-up).
    assert_eq!(transcript.input_tokens, 8);
    assert_eq!(transcript.output_tokens, 447);
    assert_eq!(transcript.cache_read_tokens, 58852);
    assert_eq!(transcript.cache_write_tokens, 20021);
    assert_eq!(transcript.reasoning_tokens, 138);

    let cost = transcript.cost_usd.expect("cost_usd must be present");
    assert!((cost - 0.00481702).abs() < 1e-6);

    // Assert Done Success
    assert_eq!(transcript.done_status, Some(DoneStatus::Success));
    assert!(transcript.text.contains("hello.md"));
}

#[test]
fn test_claude_turn2_resume_skill_permission() {
    let fixture_path = Path::new("tests/fixtures/claude/turn2-resume-skill-permission.ndjson");
    let content = fs::read_to_string(fixture_path).expect("read fixture turn2");

    let mut parser = ClaudeParser::new();
    let mut transcript = Transcript::new();

    for line in content.lines() {
        let events = parser.feed(line);
        transcript.ingest(&events);
    }

    assert_eq!(
        transcript.session_id.as_deref(),
        Some("ebfe2498-c3df-47ad-9e5d-2a04365b7be5")
    );
    assert_eq!(transcript.done_status, Some(DoneStatus::Success));
    assert!(transcript.text.contains("XIN CHÀO STUDIO-7731"));
    assert!(transcript.cost_usd.is_some());
}

#[test]
fn test_claude_turn3_permission_prompt() {
    let fixture_path = Path::new("tests/fixtures/claude/turn3-permission-prompt.ndjson");
    let content = fs::read_to_string(fixture_path).expect("read fixture turn3");

    let mut parser = ClaudeParser::new();
    let mut transcript = Transcript::new();

    for line in content.lines() {
        let events = parser.feed(line);
        transcript.ingest(&events);
    }

    assert_eq!(
        transcript.session_id.as_deref(),
        Some("ebfe2498-c3df-47ad-9e5d-2a04365b7be5")
    );
    // Assert one PermissionDenied for curl / Bash
    assert_eq!(
        transcript.permission_denials.len(),
        1,
        "must have 1 permission denial"
    );
    let (tool, input) = &transcript.permission_denials[0];
    assert_eq!(tool, "Bash");
    let cmd = input.get("command").and_then(|c| c.as_str()).unwrap_or("");
    assert!(cmd.contains("curl"), "denied command was curl");

    assert_eq!(transcript.done_status, Some(DoneStatus::Success));
}

#[test]
fn test_claude_turn4_cancelled() {
    let fixture_path = Path::new("tests/fixtures/claude/turn4-cancelled.ndjson");
    let content = fs::read_to_string(fixture_path).expect("read fixture turn4");

    let mut parser = ClaudeParser::new();
    let mut transcript = Transcript::new();

    for line in content.lines() {
        let events = parser.feed(line);
        transcript.ingest(&events);
    }

    // Killed turn: no Done in stream, but file change long.md recorded
    assert_eq!(transcript.done_status, None);

    let has_long_file = transcript
        .file_changes
        .iter()
        .any(|(p, op)| p.ends_with("long.md") && *op == FileOp::Write);
    assert!(has_long_file, "must have FileChange for long.md");
}

#[test]
fn test_claude_turn5_after_cancel() {
    let fixture_path = Path::new("tests/fixtures/claude/turn5-after-cancel.ndjson");
    let content = fs::read_to_string(fixture_path).expect("read fixture turn5");

    let mut parser = ClaudeParser::new();
    let mut transcript = Transcript::new();

    for line in content.lines() {
        let events = parser.feed(line);
        transcript.ingest(&events);
    }

    assert_eq!(transcript.done_status, Some(DoneStatus::Success));
    assert!(transcript.text.contains("long.md"));
}

#[test]
fn test_agy_turn1_edit() {
    let fixture_path = Path::new("tests/fixtures/agy/turn1-edit.ndjson");
    let content = fs::read_to_string(fixture_path).expect("read fixture turn1");

    let mut parser = AgyParser::new();
    let mut transcript = Transcript::new();

    for line in content.lines() {
        let events = parser.feed(line);
        transcript.ingest(&events);
    }

    // Assert session id
    assert_eq!(
        transcript.session_id.as_deref(),
        Some("71b3bcd3-8678-4036-901d-38b2f6678630")
    );

    // Assert FileChange hello-agy.md
    let has_hello_file = transcript
        .file_changes
        .iter()
        .any(|(p, op)| p.ends_with("hello-agy.md") && *op == FileOp::Write);
    assert!(has_hello_file, "must record FileChange for hello-agy.md");

    // Assert Done Success
    assert_eq!(transcript.done_status, Some(DoneStatus::Success));
    // agy token cost is None
    assert_eq!(transcript.cost_usd, None);
    assert!(transcript.input_tokens > 0);
    assert!(transcript.text.contains("hello-agy.md"));
}

#[test]
fn test_agy_turn2_mcp_permission() {
    let fixture_path = Path::new("tests/fixtures/agy/turn2-mcp-permission.ndjson");
    let content = fs::read_to_string(fixture_path).expect("read fixture turn2");

    let mut parser = AgyParser::new();
    let mut transcript = Transcript::new();

    for line in content.lines() {
        let events = parser.feed(line);
        transcript.ingest(&events);
    }

    // Assert PermissionDenied mcp
    assert_eq!(transcript.permission_denials.len(), 1);
    let (tool, _) = &transcript.permission_denials[0];
    assert_eq!(tool, "mcp");
    assert_eq!(transcript.done_status, Some(DoneStatus::Success));
}

#[test]
fn test_agy_turn3_project_settings() {
    let fixture_path = Path::new("tests/fixtures/agy/turn3-project-settings.ndjson");
    let content = fs::read_to_string(fixture_path).expect("read fixture turn3");

    let mut parser = AgyParser::new();
    let mut transcript = Transcript::new();

    for line in content.lines() {
        let events = parser.feed(line);
        transcript.ingest(&events);
    }

    assert_eq!(transcript.permission_denials.len(), 1);
    assert_eq!(transcript.done_status, Some(DoneStatus::Success));
}

#[test]
fn test_agy_turn4_mode_auto() {
    let fixture_path = Path::new("tests/fixtures/agy/turn4-mode-auto.ndjson");
    let content = fs::read_to_string(fixture_path).expect("read fixture turn4");

    let mut parser = AgyParser::new();
    let mut transcript = Transcript::new();

    for line in content.lines() {
        let events = parser.feed(line);
        transcript.ingest(&events);
    }

    assert_eq!(transcript.permission_denials.len(), 1);
    assert_eq!(transcript.done_status, Some(DoneStatus::Success));
}

#[test]
fn test_agy_turn5_resume() {
    let fixture_path = Path::new("tests/fixtures/agy/turn5-resume.ndjson");
    let content = fs::read_to_string(fixture_path).expect("read fixture turn5");

    let mut parser = AgyParser::new();
    let mut transcript = Transcript::new();

    for line in content.lines() {
        let events = parser.feed(line);
        transcript.ingest(&events);
    }

    assert_eq!(
        transcript.session_id.as_deref(),
        Some("71b3bcd3-8678-4036-901d-38b2f6678630")
    );
    assert_eq!(transcript.done_status, Some(DoneStatus::Success));
    assert!(transcript.text.contains("hello-agy.md"));
}

#[tokio::test]
#[ignore = "live CLI test"]
async fn test_live_claude_probe_and_turn() {
    let engine = ClaudeEngine::new();
    let probe = engine.probe().await;
    println!("Claude probe: {:?}", probe);

    if probe.installed {
        let tmp = tempfile::tempdir().unwrap();
        let req = studio_engine::TurnRequest {
            workspace_dir: tmp.path().to_path_buf(),
            prompt: "echo live test".to_string(),
            session: studio_engine::SessionRef::New(uuid::Uuid::new_v4()),
            model: None,
            mcp: None,
            plugin_dir: None,
            permission_prompt_tool: None,
            append_system_prompt: None,
            max_budget_usd: Some(0.1),
        };

        let mut turn = engine.start_turn(req).await.unwrap();
        while let Some(ev) = turn.next_event().await {
            println!("Claude live event: {:?}", ev);
        }
    }
}

#[tokio::test]
#[ignore = "live CLI test"]
async fn test_live_agy_probe_and_turn() {
    let engine = AgyEngine::new();
    let probe = engine.probe().await;
    println!("Agy probe: {:?}", probe);

    if probe.installed {
        let tmp = tempfile::tempdir().unwrap();
        let req = studio_engine::TurnRequest {
            workspace_dir: tmp.path().to_path_buf(),
            prompt: "echo live test".to_string(),
            session: studio_engine::SessionRef::New(uuid::Uuid::new_v4()),
            model: None,
            mcp: None,
            plugin_dir: None,
            permission_prompt_tool: None,
            append_system_prompt: None,
            max_budget_usd: None,
        };

        let mut turn = engine.start_turn(req).await.unwrap();
        while let Some(ev) = turn.next_event().await {
            println!("Agy live event: {:?}", ev);
        }
    }
}

#[test]
fn agy_result_error_is_surfaced() {
    let mut p = studio_engine::AgyParser::default();
    let line = r#"{"event":"result","result":{"conversation_id":"c1","status":"ERROR","response":"","error":"Individual quota reached.","usage":{"input_tokens":0,"output_tokens":0}}}"#;
    let events = p.feed(line);
    assert!(events
        .iter()
        .any(|e| matches!(e, studio_engine::Event::Error(m) if m.contains("quota"))));
    assert!(events
        .iter()
        .any(|e| matches!(e, studio_engine::Event::Done { .. })));
}

#[test]
fn agy_models_output_is_parsed() {
    // Recorded from `agy models` (agy 1.4.0, 2026-10-17).
    let raw = "Fetching available models...\ngemini-3.8-flash-high\tGemini 3.8 Flash (High)\ngemini-3.8-flash-medium\tGemini 3.8 Flash (Medium)\nclaude-opus-4-6-thinking\tClaude Opus 4.6 (Thinking)\n";
    let models = studio_engine::parse_agy_models(raw);
    assert_eq!(models.len(), 3);
    assert_eq!(models[0].id, "gemini-3.8-flash-high");
    assert_eq!(models[2].label, "Claude Opus 4.6 (Thinking)");
}

#[test]
fn claude_thinking_text_is_forwarded_once() {
    let mut p = ClaudeParser::new();
    let lines = [
        r#"{"type":"stream_event","event":{"type":"message_start","message":{}}}"#,
        r#"{"type":"stream_event","event":{"type":"content_block_start","content_block":{"type":"thinking","thinking":""}}}"#,
        r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"Cân nhắc"}}}"#,
        r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"Cân nhắc"}]}}"#,
    ];
    let events: Vec<_> = lines.iter().flat_map(|l| p.feed(l)).collect();
    let thinking: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            studio_engine::Event::Thinking(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    // Start marker + the streamed delta; the final block is not repeated.
    assert_eq!(thinking, vec!["", "Cân nhắc"]);

    // Redacted thinking (what Claude Code sends by default) only yields the start marker.
    let mut p = ClaudeParser::new();
    let ev = p.feed(r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"","signature":"x"}]}}"#);
    assert!(matches!(ev.as_slice(), [studio_engine::Event::Thinking(t)] if t.is_empty()));
}

#[test]
fn claude_thinking_tokens_live_frame() {
    let mut p = ClaudeParser::new();
    let ev = p.feed(r#"{"type":"system","subtype":"thinking_tokens","estimated_tokens":1234}"#);
    assert_eq!(
        ev,
        vec![studio_engine::Event::Usage {
            input_tokens: 0,
            output_tokens: 0,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            reasoning_tokens: 1234,
            cost_usd: None,
        }]
    );
}

#[test]
fn agy_tool_calls_mapped_to_friendly_names() {
    let mut p = AgyParser::new();
    let ev = p.feed(r#"{"event":"step_update","step_update":{"step_index":2,"state":"ACTIVE","step_type":"tool","tool_name":"write_to_file","tool_info":{"name":"write_to_file","parameters":{"TargetFile":"test.md"}}}}"#);
    assert_eq!(
        ev,
        vec![
            studio_engine::Event::FileChange {
                path: std::path::PathBuf::from("test.md"),
                op: studio_engine::FileOp::Write,
            },
            studio_engine::Event::ToolCall {
                id: "step-2".to_string(),
                name: "Tạo tệp".to_string(),
                input: serde_json::json!({"TargetFile": "test.md"}),
            }
        ]
    );
}
