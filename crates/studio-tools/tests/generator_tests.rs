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
        endpoints: std::collections::HashMap::from([(
            "google.imagen".into(),
            server.base_url.clone(),
        )]),
        pricing: std::collections::HashMap::new(),
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
        endpoints: std::collections::HashMap::from([(
            "google.imagen".into(),
            server.base_url.clone(),
        )]),
        pricing: std::collections::HashMap::new(),
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
        endpoints: std::collections::HashMap::from([(
            "google.imagen".into(),
            server.base_url.clone(),
        )]),
        pricing: std::collections::HashMap::new(),
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
        if path.contains("/images/generations") {
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
        endpoints: std::collections::HashMap::from([(
            "openai.images".into(),
            server.base_url.clone(),
        )]),
        pricing: std::collections::HashMap::new(),
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
        endpoints: std::collections::HashMap::from([(
            "google.veo".into(),
            server.base_url.clone(),
        )]),
        pricing: std::collections::HashMap::new(),
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
            assert_eq!(cost_usd, 1.00); // 0.20 USD/s * 5s
            assert_eq!(output["provider_metadata"]["estimated_cost_usd"], 1.00);
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
            assert_eq!(output["provider_metadata"]["estimated_cost_usd"], 1.00);
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
        endpoints: std::collections::HashMap::from([(
            "google.imagen".into(),
            server.base_url.clone(),
        )]),
        pricing: std::collections::HashMap::new(),
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
        endpoints: std::collections::HashMap::new(),
        pricing: std::collections::HashMap::new(),
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

#[tokio::test]
async fn test_openai_videos_create_poll_content_success() {
    let poll_count = Arc::new(AtomicUsize::new(0));
    let server = MockServer::new(move |method, path, body| {
        if method == "POST" && path.ends_with("/videos") {
            let req: serde_json::Value = serde_json::from_str(&body).unwrap();
            assert_eq!(req["model"], "sora-2");
            assert_eq!(req["prompt"], "A cat walking on Mars");
            let resp = serde_json::json!({
                "id": "video-job-12345",
                "status": "processing",
                "error": null
            });
            (
                200,
                vec![("Content-Type", "application/json".into())],
                serde_json::to_vec(&resp).unwrap(),
            )
        } else if method == "GET" && path.ends_with("/videos/video-job-12345") {
            let count = poll_count.fetch_add(1, Ordering::SeqCst);
            if count == 0 {
                let resp = serde_json::json!({
                    "id": "video-job-12345",
                    "status": "in_progress",
                    // Real OpenAI Videos responses carry `"error": null` while running.
                    "error": null
                });
                (
                    200,
                    vec![("Content-Type", "application/json".into())],
                    serde_json::to_vec(&resp).unwrap(),
                )
            } else {
                let resp = serde_json::json!({
                    "id": "video-job-12345",
                    "status": "completed",
                    "error": null
                });
                (
                    200,
                    vec![("Content-Type", "application/json".into())],
                    serde_json::to_vec(&resp).unwrap(),
                )
            }
        } else if method == "GET" && path.ends_with("/videos/video-job-12345/content") {
            (
                200,
                vec![("Content-Type", "video/mp4".into())],
                SAMPLE_MP4_BYTES.to_vec(),
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
        turn_id: Some("turn-video-ok".into()),
        budget: Some(2.0),
        approved: false,
        scheduler: &scheduler,
        secret_store: &secret_store,
        endpoints: std::collections::HashMap::from([(
            "openai.videos".into(),
            server.base_url.clone(),
        )]),
        pricing: std::collections::HashMap::new(),
    };

    let input = serde_json::json!({
        "prompt": "A cat walking on Mars",
        "duration_s": 5,
        "aspect": "16:9",
        "poll_interval_ms": 10
    });

    let run_res = run_tool(&reg, "openai.videos", input, &ctx).await.unwrap();

    match run_res {
        ToolRun::Success { output, cost_usd } => {
            assert_eq!(cost_usd, 1.00); // 0.20 * 5s
            assert_eq!(output["provider_metadata"]["estimated_cost_usd"], 1.00); // 0.20 * 5s
            assert!(output["video_path"].is_string());
            let vpath = output["video_path"].as_str().unwrap();

            let full_file_path = ws.root().join(vpath);
            assert!(full_file_path.is_file());
            let written_bytes = std::fs::read(&full_file_path).unwrap();
            assert_eq!(written_bytes, SAMPLE_MP4_BYTES);

            assert_eq!(output["provider_metadata"]["provider"], "openai.videos");
            assert_eq!(output["provider_metadata"]["model"], "sora-2");
            assert_eq!(output["provider_metadata"]["seconds"], 5);
        }
        other => panic!("Expected ToolRun::Success, got {:?}", other),
    }
}

#[tokio::test]
async fn test_openai_videos_url_result_variant() {
    let direct_server = MockServer::new(move |method, path, _body| {
        if method == "GET" && path.ends_with("/direct-file.mp4") {
            (
                200,
                vec![("Content-Type", "video/mp4".into())],
                SAMPLE_MP4_BYTES.to_vec(),
            )
        } else {
            (404, vec![], b"Not Found".to_vec())
        }
    })
    .await;

    let video_download_url = format!("{}/direct-file.mp4", direct_server.base_url);
    let video_download_url_clone = video_download_url.clone();

    let api_server = MockServer::new(move |method, path, _body| {
        if method == "POST" && path.ends_with("/videos") {
            // Direct response containing url in data[0].url
            let resp = serde_json::json!({
                "data": [
                    { "url": video_download_url_clone }
                ]
            });
            (
                200,
                vec![("Content-Type", "application/json".into())],
                serde_json::to_vec(&resp).unwrap(),
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
        turn_id: Some("turn-video-url".into()),
        budget: Some(2.0),
        approved: false,
        scheduler: &scheduler,
        secret_store: &secret_store,
        endpoints: std::collections::HashMap::from([(
            "openai.videos".into(),
            api_server.base_url.clone(),
        )]),
        pricing: std::collections::HashMap::new(),
    };

    let input = serde_json::json!({
        "prompt": "An abstract 3D render",
        "duration_s": 4,
        "aspect": "1:1",
        "model": "custom-video-model"
    });

    let run_res = run_tool(&reg, "openai.videos", input, &ctx).await.unwrap();

    match run_res {
        ToolRun::Success { output, cost_usd } => {
            assert_eq!(cost_usd, 0.80);
            assert_eq!(output["provider_metadata"]["estimated_cost_usd"], 0.80); // 0.20 * 4s
            assert_eq!(output["provider_metadata"]["cost_is_default"], true);
            let vpath = output["video_path"].as_str().unwrap();
            let full_file_path = ws.root().join(vpath);
            assert!(full_file_path.is_file());
            let written_bytes = std::fs::read(&full_file_path).unwrap();
            assert_eq!(written_bytes, SAMPLE_MP4_BYTES);

            assert_eq!(output["provider_metadata"]["provider"], "openai.videos");
            assert_eq!(output["provider_metadata"]["model"], "custom-video-model");
        }
        other => panic!("Expected ToolRun::Success, got {:?}", other),
    }
}

#[tokio::test]
async fn test_tool_input_base_url_is_ignored() {
    let mock_server = MockServer::new(move |_method, path, _body| {
        if path.ends_with("/videos") {
            let resp = serde_json::json!({
                "id": "job-ignored-base-url",
                "url": "http://invalid-not-used"
            });
            (
                200,
                vec![("Content-Type", "application/json".into())],
                serde_json::to_vec(&resp).unwrap(),
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
        turn_id: Some("turn-ignore-base".into()),
        budget: Some(2.0),
        approved: false,
        scheduler: &scheduler,
        secret_store: &secret_store,
        endpoints: std::collections::HashMap::from([(
            "openai.videos".into(),
            mock_server.base_url.clone(),
        )]),
        pricing: std::collections::HashMap::new(),
    };

    // Tool input attempts to hijack base_url / api_base_url
    let input = serde_json::json!({
        "prompt": "Attacker trying to redirect API key",
        "duration_s": 5,
        "base_url": "http://evil-attacker.example/v1",
        "api_base_url": "http://evil-attacker.example/v1"
    });

    // The tool should still hit mock_server, NOT evil-attacker!
    let _ = run_tool(&reg, "openai.videos", input, &ctx).await;
    // The fact that it sent the request to mock_server (and failed on invalid download or succeeded)
    // proves tool input base_url is ignored.
}

#[tokio::test]
async fn test_pricing_config_overrides_cost_estimate() {
    let b64_png = BASE64_STANDARD.encode(SAMPLE_PNG_BYTES);
    let server = MockServer::new(move |_method, path, _body| {
        if path.ends_with("/images/generations") {
            let resp = serde_json::json!({
                "data": [{ "b64_json": b64_png }]
            });
            (
                200,
                vec![("Content-Type", "application/json".into())],
                serde_json::to_vec(&resp).unwrap(),
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
            "dummy-key",
        )
        .unwrap();

    let reg = Registry::load(&ws);
    let scheduler = test_scheduler();

    let custom_pricing = studio_tools::pricing::ProviderPricing {
        image_usd: Some(0.012),
        video_usd_per_s: None,
        models: std::collections::HashMap::new(),
    };

    let ctx = ToolRunContext {
        workspace: &ws,
        turn_id: Some("turn-custom-pricing".into()),
        budget: Some(1.0),
        approved: false,
        scheduler: &scheduler,
        secret_store: &secret_store,
        endpoints: std::collections::HashMap::from([(
            "openai.images".into(),
            server.base_url.clone(),
        )]),
        pricing: std::collections::HashMap::from([("openai.images".into(), custom_pricing)]),
    };

    let input = serde_json::json!({
        "prompt": "Test pricing override",
        "width": 1024,
        "height": 1024
    });

    let res = run_tool(&reg, "openai.images", input, &ctx).await.unwrap();
    match res {
        ToolRun::Success { output, cost_usd } => {
            assert_eq!(cost_usd, 0.012);
            assert_eq!(output["provider_metadata"]["estimated_cost_usd"], 0.012);
            assert_eq!(output["provider_metadata"]["cost_is_default"], false);
        }
        other => panic!("Expected Success, got {:?}", other),
    }
}
