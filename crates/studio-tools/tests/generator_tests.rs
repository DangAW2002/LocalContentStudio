use base64::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use studio_jobs::{EstimateStore, MemoryEstimateStore, Scheduler, SchedulerConfig, SysinfoProbe};
use studio_tools::manifest::SideEffect;
use studio_tools::probe::probe;
use studio_tools::runner::{run_tool, ToolRun, ToolRunContext};
use studio_tools::secret::{MemoryStore, SecretStore};
use studio_tools::Registry;
use studio_workspace::Workspace;
use tempfile::tempdir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

// Sample valid media payloads
const SAMPLE_JPEG_BYTES: &[u8] = &[
    0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00, 0x01,
];
const SAMPLE_PNG_BYTES: &[u8] = &[
    0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
];
const SAMPLE_MP4_BYTES: &[u8] = &[
    0x00, 0x00, 0x00, 0x20, b'f', b't', b'y', b'p', b'i', b's', b'o', b'm',
];

fn test_scheduler() -> Arc<Scheduler> {
    let config = SchedulerConfig::default();
    let probe = Arc::new(SysinfoProbe::new());
    let estimates: Arc<dyn EstimateStore> = Arc::new(MemoryEstimateStore::new());
    Scheduler::new(config, probe, estimates)
}

struct MockServer {
    pub base_url: String,
    _shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

impl MockServer {
    async fn new<F>(handler: F) -> Self
    where
        F: Fn(String, String, String) -> (u16, Vec<(&'static str, String)>, Vec<u8>)
            + Send
            + Sync
            + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let base_url = format!("http://127.0.0.1:{}", port);
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        let handler = Arc::new(handler);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    accept_res = listener.accept() => {
                        let Ok((mut socket, _)) = accept_res else { break; };
                        let handler = Arc::clone(&handler);

                        tokio::spawn(async move {
                            let mut buf = vec![0u8; 8192];
                            let n = match socket.read(&mut buf).await {
                                Ok(n) if n > 0 => n,
                                _ => return,
                            };

                            let request_str = String::from_utf8_lossy(&buf[..n]).to_string();
                            let mut lines = request_str.lines();
                            let first_line = lines.next().unwrap_or("");
                            let mut parts = first_line.split_whitespace();
                            let method = parts.next().unwrap_or("GET").to_string();
                            let path = parts.next().unwrap_or("/").to_string();

                            // Extract body if any
                            let body = if let Some(idx) = request_str.find("\r\n\r\n") {
                                request_str[idx + 4..].to_string()
                            } else {
                                String::new()
                            };

                            let (status, headers, body_bytes) = handler(method, path, body);

                            let reason = match status {
                                200 => "OK",
                                400 => "Bad Request",
                                429 => "Too Many Requests",
                                500 => "Internal Server Error",
                                _ => "Status",
                            };

                            let mut resp_str = format!("HTTP/1.1 {} {}\r\nContent-Length: {}\r\n", status, reason, body_bytes.len());
                            for (k, v) in headers {
                                resp_str.push_str(&format!("{}: {}\r\n", k, v));
                            }
                            resp_str.push_str("\r\n");

                            let _ = socket.write_all(resp_str.as_bytes()).await;
                            let _ = socket.write_all(&body_bytes).await;
                        });
                    }
                }
            }
        });

        Self {
            base_url,
            _shutdown_tx: shutdown_tx,
        }
    }
}

#[test]
fn test_probe_google_imagen_no_key_unavailable() {
    let secret_store = MemoryStore::new();
    let ws_reg =
        Registry::load(&Workspace::create(tempdir().unwrap().path().join("ws"), "ws").unwrap());
    let tool = ws_reg
        .get_tool("google.imagen")
        .expect("google.imagen builtin must exist");

    let result = probe(tool, &secret_store, None);
    assert!(!result.available);
    assert_eq!(result.missing, vec!["secret:GEMINI_API_KEY"]);
    assert_eq!(
        result.note_vi,
        "Chưa có khoá API GEMINI_API_KEY — thêm trong Cài đặt → Khoá API"
    );
}

#[test]
fn test_probe_google_imagen_with_key_available() {
    let secret_store = MemoryStore::new();
    secret_store
        .set("", None, "GEMINI_API_KEY", "secret-test-key-12345")
        .unwrap();

    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();
    let reg = Registry::load(&ws);
    let tool = reg
        .get_tool("google.imagen")
        .expect("google.imagen must exist");

    let result = probe(tool, &secret_store, Some(&ws));
    assert!(result.available);
    assert!(result.missing.is_empty());
    assert_eq!(result.note_vi, "Sẵn sàng");
}

#[test]
fn test_probe_openai_images_no_key_unavailable() {
    let secret_store = MemoryStore::new();
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();
    let reg = Registry::load(&ws);
    let tool = reg
        .get_tool("openai.images")
        .expect("openai.images must exist");

    let result = probe(tool, &secret_store, Some(&ws));
    assert!(!result.available);
    assert_eq!(result.missing, vec!["secret:OPENAI_API_KEY"]);
    assert_eq!(
        result.note_vi,
        "Chưa có khoá API OPENAI_API_KEY — thêm trong Cài đặt → Khoá API"
    );
}

#[tokio::test]
async fn test_google_imagen_success_fixture() {
    let b64_jpeg = BASE64_STANDARD.encode(SAMPLE_JPEG_BYTES);
    let fixture_resp = serde_json::json!({
        "predictions": [
            {
                "bytesBase64Encoded": b64_jpeg,
                "mimeType": "image/jpeg"
            }
        ]
    });
    let fixture_bytes = serde_json::to_vec(&fixture_resp).unwrap();

    let server = MockServer::new(move |_method, path, _body| {
        if path.contains("imagen-3.0-generate-002:predict") {
            (
                200,
                vec![("Content-Type", "application/json".into())],
                fixture_bytes.clone(),
            )
        } else {
            (404, vec![], b"Not Found".to_vec())
        }
    })
    .await;

    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();
    let secret_store = MemoryStore::new();
    secret_store
        .set(
            ws.org().name(),
            Some(ws.name()),
            "GEMINI_API_KEY",
            "dummy-gemini-key",
        )
        .unwrap();

    let reg = Registry::load(&ws);
    let scheduler = test_scheduler();

    let ctx = ToolRunContext {
        workspace: &ws,
        turn_id: Some("turn-imagen-ok".into()),
        budget: Some(1.0),
        approved: false,
        scheduler: &scheduler,
        secret_store: &secret_store,
        api_base_override: Some(server.base_url.clone()),
    };

    let input = serde_json::json!({
        "prompt": "A cinematic shot of a sunset over mountains",
        "width": 1280,
        "height": 720
    });

    let run_res = run_tool(&reg, "google.imagen", input, &ctx).await.unwrap();

    match run_res {
        ToolRun::Success { output, cost_usd } => {
            assert_eq!(cost_usd, 0.04);
            let files = output["files"].as_array().expect("files array");
            assert_eq!(files.len(), 1);
            let file_rel = files[0].as_str().unwrap();
            assert!(file_rel.contains("turns/turn-imagen-ok"));

            let full_file_path = ws.root().join(file_rel);
            assert!(
                full_file_path.is_file(),
                "Generated image file must exist on disk"
            );
            let written_bytes = std::fs::read(&full_file_path).unwrap();
            assert_eq!(written_bytes, SAMPLE_JPEG_BYTES);

            assert_eq!(output["provider_metadata"]["provider"], "google.imagen");
            assert_eq!(output["provider_metadata"]["estimated_cost_usd"], 0.04);
        }
        other => panic!("Expected ToolRun::Success, got {:?}", other),
    }
}

#[tokio::test]
async fn test_google_imagen_429_then_success_retry() {
    std::env::set_var("STUDIO_TEST_FAST_RETRY", "1");
    let b64_jpeg = BASE64_STANDARD.encode(SAMPLE_JPEG_BYTES);
    let fixture_resp = serde_json::json!({
        "predictions": [
            {
                "bytesBase64Encoded": b64_jpeg,
                "mimeType": "image/jpeg"
            }
        ]
    });
    let fixture_bytes = serde_json::to_vec(&fixture_resp).unwrap();

    let attempts = Arc::new(AtomicUsize::new(0));
    let server = MockServer::new(move |_method, _path, _body| {
        let count = attempts.fetch_add(1, Ordering::SeqCst);
        if count == 0 {
            // First attempt: 429 Too Many Requests
            (
                429,
                vec![("Retry-After", "0".into())],
                b"Rate limit exceeded".to_vec(),
            )
        } else {
            // Second attempt: 200 OK
            (
                200,
                vec![("Content-Type", "application/json".into())],
                fixture_bytes.clone(),
            )
        }
    })
    .await;

    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();
    let secret_store = MemoryStore::new();
    secret_store
        .set(
            ws.org().name(),
            Some(ws.name()),
            "GEMINI_API_KEY",
            "dummy-key",
        )
        .unwrap();

    let reg = Registry::load(&ws);
    let scheduler = test_scheduler();

    let ctx = ToolRunContext {
        workspace: &ws,
        turn_id: Some("turn-retry".into()),
        budget: Some(1.0),
        approved: false,
        scheduler: &scheduler,
        secret_store: &secret_store,
        api_base_override: Some(server.base_url.clone()),
    };

    let input = serde_json::json!({
        "prompt": "Test retry prompt",
        "width": 1024,
        "height": 1024
    });

    let run_res = run_tool(&reg, "google.imagen", input, &ctx).await.unwrap();
    assert!(matches!(run_res, ToolRun::Success { .. }));
}

#[tokio::test]
async fn test_google_imagen_error_body_fails_without_retry() {
    let server = MockServer::new(|_method, _path, _body| {
        let err_json = serde_json::json!({
            "error": {
                "code": 400,
                "message": "Prompt contains disallowed policy terms"
            }
        });
        (
            400,
            vec![("Content-Type", "application/json".into())],
            serde_json::to_vec(&err_json).unwrap(),
        )
    })
    .await;

    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();
    let secret_store = MemoryStore::new();
    secret_store
        .set(
            ws.org().name(),
            Some(ws.name()),
            "GEMINI_API_KEY",
            "dummy-key",
        )
        .unwrap();

    let reg = Registry::load(&ws);
    let scheduler = test_scheduler();

    let ctx = ToolRunContext {
        workspace: &ws,
        turn_id: Some("turn-err".into()),
        budget: Some(1.0),
        approved: false,
        scheduler: &scheduler,
        secret_store: &secret_store,
        api_base_override: Some(server.base_url.clone()),
    };

    let input = serde_json::json!({
        "prompt": "Forbidden content",
        "width": 1024,
        "height": 1024
    });

    let run_err = run_tool(&reg, "google.imagen", input, &ctx)
        .await
        .unwrap_err();
    let err_str = run_err.to_string();
    assert!(
        err_str.contains("Prompt contains disallowed policy terms"),
        "Error was: {err_str}"
    );
}

#[tokio::test]
async fn test_openai_images_success_fixture() {
    let b64_png = BASE64_STANDARD.encode(SAMPLE_PNG_BYTES);
    let fixture_resp = serde_json::json!({
        "created": 1700000000,
        "data": [
            {
                "b64_json": b64_png,
                "revised_prompt": "Revised prompt"
            }
        ]
    });
    let fixture_bytes = serde_json::to_vec(&fixture_resp).unwrap();

    let server = MockServer::new(move |_method, path, _body| {
        if path.contains("/v1/images/generations") {
            (
                200,
                vec![("Content-Type", "application/json".into())],
                fixture_bytes.clone(),
            )
        } else {
            (404, vec![], b"Not Found".to_vec())
        }
    })
    .await;

    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();
    let secret_store = MemoryStore::new();
    secret_store
        .set(
            ws.org().name(),
            Some(ws.name()),
            "OPENAI_API_KEY",
            "dummy-openai-key",
        )
        .unwrap();

    let reg = Registry::load(&ws);
    let scheduler = test_scheduler();

    let ctx = ToolRunContext {
        workspace: &ws,
        turn_id: Some("turn-openai-ok".into()),
        budget: Some(1.0),
        approved: false,
        scheduler: &scheduler,
        secret_store: &secret_store,
        api_base_override: Some(server.base_url.clone()),
    };

    let input = serde_json::json!({
        "prompt": "A modern logo design",
        "width": 1024,
        "height": 1024
    });

    let run_res = run_tool(&reg, "openai.images", input, &ctx).await.unwrap();

    match run_res {
        ToolRun::Success { output, cost_usd } => {
            assert_eq!(cost_usd, 0.04);
            let files = output["files"].as_array().expect("files array");
            assert_eq!(files.len(), 1);
            let file_rel = files[0].as_str().unwrap();

            let full_file_path = ws.root().join(file_rel);
            assert!(full_file_path.is_file());
            let written_bytes = std::fs::read(&full_file_path).unwrap();
            assert_eq!(written_bytes, SAMPLE_PNG_BYTES);

            assert_eq!(output["provider_metadata"]["provider"], "openai.images");
            assert_eq!(output["provider_metadata"]["model"], "gpt-image-1");
        }
        other => panic!("Expected ToolRun::Success, got {:?}", other),
    }
}

#[tokio::test]
async fn test_google_veo_success_fixture() {
    let b64_mp4 = BASE64_STANDARD.encode(SAMPLE_MP4_BYTES);
    let poll_count = Arc::new(AtomicUsize::new(0));

    let server = MockServer::new(move |_method, path, _body| {
        if path.contains("predictLongRunning") {
            let resp = serde_json::json!({
                "name": "operations/veo-op-987"
            });
            (
                200,
                vec![("Content-Type", "application/json".into())],
                serde_json::to_vec(&resp).unwrap(),
            )
        } else if path.contains("operations/veo-op-987") {
            let count = poll_count.fetch_add(1, Ordering::SeqCst);
            if count == 0 {
                // First poll: not done yet
                let resp = serde_json::json!({
                    "name": "operations/veo-op-987",
                    "done": false
                });
                (
                    200,
                    vec![("Content-Type", "application/json".into())],
                    serde_json::to_vec(&resp).unwrap(),
                )
            } else {
                // Second poll: done!
                let resp = serde_json::json!({
                    "name": "operations/veo-op-987",
                    "done": true,
                    "response": {
                        "generateVideoResponse": {
                            "generatedVideos": [
                                {
                                    "video": {
                                        "bytesBase64Encoded": b64_mp4
                                    }
                                }
                            ]
                        }
                    }
                });
                (
                    200,
                    vec![("Content-Type", "application/json".into())],
                    serde_json::to_vec(&resp).unwrap(),
                )
            }
        } else {
            (404, vec![], b"Not Found".to_vec())
        }
    })
    .await;

    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();
    let secret_store = MemoryStore::new();
    secret_store
        .set(
            ws.org().name(),
            Some(ws.name()),
            "GEMINI_API_KEY",
            "dummy-gemini-key",
        )
        .unwrap();

    let reg = Registry::load(&ws);
    let scheduler = test_scheduler();

    let ctx = ToolRunContext {
        workspace: &ws,
        turn_id: Some("turn-veo".into()),
        budget: Some(1.0),
        approved: false,
        scheduler: &scheduler,
        secret_store: &secret_store,
        api_base_override: Some(server.base_url.clone()),
    };

    let input = serde_json::json!({
        "prompt": "Drone shot of ocean waves crashing onto rocks",
        "duration_s": 5,
        "aspect": "16:9",
        "poll_interval_ms": 10
    });

    let run_res = run_tool(&reg, "google.veo", input, &ctx).await.unwrap();

    match run_res {
        ToolRun::Success { output, cost_usd } => {
            assert_eq!(cost_usd, 0.20);
            assert!(output["video_path"].is_string());
            let vpath = output["video_path"].as_str().unwrap();

            let full_file_path = ws.root().join(vpath);
            assert!(full_file_path.is_file());
            let written_bytes = std::fs::read(&full_file_path).unwrap();
            assert_eq!(written_bytes, SAMPLE_MP4_BYTES);

            assert_eq!(output["provider_metadata"]["provider"], "google.veo");
            assert_eq!(
                output["provider_metadata"]["operation_name"],
                "operations/veo-op-987"
            );
            assert_eq!(output["provider_metadata"]["estimated_cost_usd"], 0.20);
        }
        other => panic!("Expected ToolRun::Success, got {:?}", other),
    }
}

#[tokio::test]
async fn test_magic_bytes_invalid_fails() {
    let invalid_b64 = BASE64_STANDARD.encode(b"THIS IS NOT AN IMAGE OR VIDEO AT ALL");
    let fixture_resp = serde_json::json!({
        "predictions": [
            {
                "bytesBase64Encoded": invalid_b64,
                "mimeType": "image/jpeg"
            }
        ]
    });
    let fixture_bytes = serde_json::to_vec(&fixture_resp).unwrap();

    let server = MockServer::new(move |_method, _path, _body| {
        (
            200,
            vec![("Content-Type", "application/json".into())],
            fixture_bytes.clone(),
        )
    })
    .await;

    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();
    let secret_store = MemoryStore::new();
    secret_store
        .set(
            ws.org().name(),
            Some(ws.name()),
            "GEMINI_API_KEY",
            "dummy-key",
        )
        .unwrap();

    let reg = Registry::load(&ws);
    let scheduler = test_scheduler();

    let ctx = ToolRunContext {
        workspace: &ws,
        turn_id: Some("turn-bad-magic".into()),
        budget: Some(1.0),
        approved: false,
        scheduler: &scheduler,
        secret_store: &secret_store,
        api_base_override: Some(server.base_url.clone()),
    };

    let input = serde_json::json!({
        "prompt": "Test bad magic",
        "width": 1024,
        "height": 1024
    });

    let err = run_tool(&reg, "google.imagen", input, &ctx)
        .await
        .unwrap_err();
    let err_str = err.to_string();
    assert!(
        err_str.contains("magic bytes"),
        "Expected magic bytes failure, got: {err_str}"
    );
}

#[tokio::test]
async fn test_budget_refusal_for_builtin_generator() {
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();
    let secret_store = MemoryStore::new();
    secret_store
        .set(
            ws.org().name(),
            Some(ws.name()),
            "GEMINI_API_KEY",
            "dummy-key",
        )
        .unwrap();

    let reg = Registry::load(&ws);
    let scheduler = test_scheduler();

    // Budget is only $0.01, but google.imagen costs $0.04 and approved = false
    let ctx = ToolRunContext {
        workspace: &ws,
        turn_id: Some("turn-low-budget".into()),
        budget: Some(0.01),
        approved: false,
        scheduler: &scheduler,
        secret_store: &secret_store,
        api_base_override: None,
    };

    let input = serde_json::json!({
        "prompt": "A prompt",
        "width": 1024,
        "height": 1024
    });

    let run_res = run_tool(&reg, "google.imagen", input, &ctx).await.unwrap();

    match run_res {
        ToolRun::NeedsApproval {
            reason,
            side_effect,
        } => {
            assert_eq!(side_effect, SideEffect::Paid);
            assert!(reason.contains("vượt quá ngân sách còn lại"));
        }
        other => panic!("Expected NeedsApproval, got {:?}", other),
    }
}
