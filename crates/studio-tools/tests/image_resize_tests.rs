use base64::prelude::*;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use studio_jobs::{EstimateStore, MemoryEstimateStore, Scheduler, SchedulerConfig, SysinfoProbe};
use studio_tools::providers::openai::{build_openai_edits_multipart, MultipartFile};
use studio_tools::providers::resize::{fit_image_file, probe_image_dimensions};
use studio_tools::runner::{run_tool, ToolRun, ToolRunContext};
use studio_tools::secret::{MemoryStore, SecretStore};
use studio_tools::Registry;
use studio_workspace::Workspace;
use tempfile::tempdir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

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
        F: Fn(String, String, Vec<u8>) -> (u16, Vec<(&'static str, String)>, Vec<u8>)
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
                            let mut buf = vec![0u8; 65536];
                            let n = match socket.read(&mut buf).await {
                                Ok(n) if n > 0 => n,
                                _ => return,
                            };

                            let request_bytes = &buf[..n];
                            let header_end = request_bytes
                                .windows(4)
                                .position(|w| w == b"\r\n\r\n")
                                .unwrap_or(n);

                            let header_str = String::from_utf8_lossy(&request_bytes[..header_end]).to_string();
                            let mut lines = header_str.lines();
                            let first_line = lines.next().unwrap_or("");
                            let mut parts = first_line.split_whitespace();
                            let method = parts.next().unwrap_or("GET").to_string();
                            let path = parts.next().unwrap_or("/").to_string();

                            let body = if header_end + 4 < n {
                                request_bytes[header_end + 4..].to_vec()
                            } else {
                                Vec::new()
                            };

                            let (status, headers, body_bytes) = handler(method, path, body);

                            let reason = match status {
                                200 => "OK",
                                400 => "Bad Request",
                                404 => "Not Found",
                                _ => "Status",
                            };

                            let mut resp_str = format!(
                                "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n",
                                status, reason, body_bytes.len()
                            );
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

/// Helper to generate a real image using FFmpeg color source.
fn generate_color_image(path: &std::path::Path, width: u64, height: u64, color: &str) {
    let ffmpeg_exe = studio_tools::probe::find_in_path("ffmpeg")
        .expect("ffmpeg must be in PATH for image tests");
    let status = Command::new(ffmpeg_exe)
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            &format!("color=c={color}:s={width}x{height}:d=1"),
            "-frames:v",
            "1",
            "-update",
            "1",
        ])
        .arg(path)
        .status()
        .expect("ffmpeg execution should succeed");
    assert!(status.success(), "ffmpeg should create test image");
}

#[test]
fn test_build_openai_edits_multipart_pure() {
    let boundary = "TEST_BOUNDARY_12345";
    let files = vec![
        MultipartFile {
            field_name: "image[]".to_string(),
            filename: "char1.png".to_string(),
            mime_type: "image/png".to_string(),
            bytes: b"FAKE_PNG_1".to_vec(),
        },
        MultipartFile {
            field_name: "image[]".to_string(),
            filename: "char2.jpg".to_string(),
            mime_type: "image/jpeg".to_string(),
            bytes: b"FAKE_JPG_2".to_vec(),
        },
    ];

    let body = build_openai_edits_multipart(
        boundary,
        "gpt-image-1",
        "A photo of two characters talking",
        "1792x1024",
        1,
        &files,
    );

    let text = String::from_utf8_lossy(&body);

    assert!(text.contains("--TEST_BOUNDARY_12345\r\nContent-Disposition: form-data; name=\"model\"\r\n\r\ngpt-image-1\r\n"));
    assert!(text.contains("--TEST_BOUNDARY_12345\r\nContent-Disposition: form-data; name=\"prompt\"\r\n\r\nA photo of two characters talking\r\n"));
    assert!(text.contains("--TEST_BOUNDARY_12345\r\nContent-Disposition: form-data; name=\"size\"\r\n\r\n1792x1024\r\n"));
    assert!(text.contains(
        "--TEST_BOUNDARY_12345\r\nContent-Disposition: form-data; name=\"n\"\r\n\r\n1\r\n"
    ));
    assert!(text.contains(
        "name=\"image[]\"; filename=\"char1.png\"\r\nContent-Type: image/png\r\n\r\nFAKE_PNG_1\r\n"
    ));
    assert!(text.contains("name=\"image[]\"; filename=\"char2.jpg\"\r\nContent-Type: image/jpeg\r\n\r\nFAKE_JPG_2\r\n"));
    assert!(text.ends_with("--TEST_BOUNDARY_12345--\r\n"));
}

#[test]
fn test_image_resize_cover_contain_none_and_odd_sizes() {
    let tmp = tempdir().unwrap();
    let src_cover = tmp.path().join("source_cover.png");
    generate_color_image(&src_cover, 1792, 1024, "red");

    // 1. Cover: 1792x1024 -> 1280x720
    let res_cover = fit_image_file(&src_cover, 1280, 720, "cover", false, false).unwrap();
    assert!(res_cover.resized);
    let dims_cover = probe_image_dimensions(&res_cover.final_path).unwrap();
    assert_eq!(dims_cover, (1280, 720));

    // 2. Contain: 1792x1024 -> 1280x720
    let src_contain = tmp.path().join("source_contain.png");
    generate_color_image(&src_contain, 1792, 1024, "blue");
    let res_contain = fit_image_file(&src_contain, 1280, 720, "contain", false, false).unwrap();
    assert!(res_contain.resized);
    let dims_contain = probe_image_dimensions(&res_contain.final_path).unwrap();
    assert_eq!(dims_contain, (1280, 720));

    // 3. None: stays 1792x1024, resized == false
    let src_none = tmp.path().join("source_none.png");
    generate_color_image(&src_none, 1792, 1024, "green");
    let res_none = fit_image_file(&src_none, 1280, 720, "none", false, false).unwrap();
    assert!(!res_none.resized);
    let dims_none = probe_image_dimensions(&res_none.final_path).unwrap();
    assert_eq!(dims_none, (1792, 1024));

    // 4. Odd size: 1281x721
    let src_odd = tmp.path().join("source_odd.png");
    generate_color_image(&src_odd, 1792, 1024, "yellow");
    let res_odd = fit_image_file(&src_odd, 1281, 721, "cover", false, false).unwrap();
    assert!(res_odd.resized);
    let dims_odd = probe_image_dimensions(&res_odd.final_path).unwrap();
    assert_eq!(dims_odd, (1281, 721));

    // 5. Keep original: true -> preserves .orig.png
    let src_orig = tmp.path().join("hero.png");
    generate_color_image(&src_orig, 1792, 1024, "purple");
    let res_orig = fit_image_file(&src_orig, 1280, 720, "cover", true, false).unwrap();
    assert!(res_orig.resized);
    let backup_path = tmp.path().join("hero.orig.png");
    assert!(
        backup_path.is_file(),
        "hero.orig.png must exist when keep_original: true"
    );
    let dims_backup = probe_image_dimensions(&backup_path).unwrap();
    assert_eq!(dims_backup, (1792, 1024));
    let dims_final = probe_image_dimensions(&res_orig.final_path).unwrap();
    assert_eq!(dims_final, (1280, 720));
}

#[tokio::test]
async fn test_openai_images_with_reference_images_calls_edits() {
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();

    // Create 2 test reference images in workspace
    let ref1_path = ws.root().join("ref1.png");
    let ref2_path = ws.root().join("ref2.png");
    generate_color_image(&ref1_path, 256, 256, "red");
    generate_color_image(&ref2_path, 256, 256, "blue");

    let server_called_path = Arc::new(std::sync::Mutex::new(String::new()));
    let server_called_body = Arc::new(std::sync::Mutex::new(Vec::new()));

    let called_path_clone = Arc::clone(&server_called_path);
    let called_body_clone = Arc::clone(&server_called_body);

    let dummy_img = tmp.path().join("mock_out.png");
    generate_color_image(&dummy_img, 1792, 1024, "black");
    let dummy_bytes = std::fs::read(&dummy_img).unwrap();
    let b64 = BASE64_STANDARD.encode(&dummy_bytes);

    let server = MockServer::new(move |_method, path, body| {
        *called_path_clone.lock().unwrap() = path.clone();
        *called_body_clone.lock().unwrap() = body;

        let resp_json = serde_json::json!({
            "data": [{ "b64_json": b64 }]
        });
        (
            200,
            vec![("Content-Type", "application/json".into())],
            serde_json::to_vec(&resp_json).unwrap(),
        )
    })
    .await;

    let secret_store = MemoryStore::new();
    secret_store
        .set(
            ws.org().name(),
            Some(ws.name()),
            "OPENAI_API_KEY",
            "test-key",
        )
        .unwrap();

    let reg = Registry::load(&ws);
    let scheduler = test_scheduler();
    let ctx = ToolRunContext {
        workspace: &ws,
        turn_id: Some("turn-edits-ok".into()),
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
        "prompt": "Draw character with reference styles",
        "width": 1280,
        "height": 720,
        "reference_images": ["ref1.png", "ref2.png"]
    });

    let run_res = run_tool(&reg, "openai.images", input, &ctx).await.unwrap();
    match run_res {
        ToolRun::Success { output, .. } => {
            let path_called = server_called_path.lock().unwrap().clone();
            assert_eq!(
                path_called, "/images/edits",
                "Must call /images/edits endpoint when reference_images present"
            );

            let body_called = server_called_body.lock().unwrap().clone();
            let body_str = String::from_utf8_lossy(&body_called);
            assert!(body_str.contains("name=\"image[]\"; filename=\"ref1.png\""));
            assert!(body_str.contains("name=\"image[]\"; filename=\"ref2.png\""));

            // Check resized post-processing
            assert_eq!(output["requested_size"], "1280x720");
            assert_eq!(output["native_size"], "1792x1024");
            assert_eq!(output["resized"], true);

            let files = output["files"].as_array().unwrap();
            let file_rel = files[0].as_str().unwrap();
            let full_p = ws.root().join(file_rel);
            let dims = probe_image_dimensions(&full_p).unwrap();
            assert_eq!(dims, (1280, 720));
        }
        other => panic!("Expected ToolRun::Success, got {:?}", other),
    }
}

#[tokio::test]
async fn test_openai_images_edits_400_fallback_to_generations() {
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();

    let ref_path = ws.root().join("profile.png");
    generate_color_image(&ref_path, 256, 256, "cyan");

    let dummy_img = tmp.path().join("mock_gen.png");
    generate_color_image(&dummy_img, 1792, 1024, "magenta");
    let dummy_bytes = std::fs::read(&dummy_img).unwrap();
    let b64 = BASE64_STANDARD.encode(&dummy_bytes);

    let edits_attempts = Arc::new(AtomicUsize::new(0));
    let gen_attempts = Arc::new(AtomicUsize::new(0));

    let e_cnt = Arc::clone(&edits_attempts);
    let g_cnt = Arc::clone(&gen_attempts);

    let server = MockServer::new(move |_method, path, _body| {
        if path == "/images/edits" {
            e_cnt.fetch_add(1, Ordering::SeqCst);
            // Gateway does not support edits -> returns 400 Bad Request
            (
                400,
                vec![("Content-Type", "application/json".into())],
                br#"{"error":{"message":"Endpoint not supported"}}"#.to_vec(),
            )
        } else if path == "/images/generations" {
            g_cnt.fetch_add(1, Ordering::SeqCst);
            let resp_json = serde_json::json!({
                "data": [{ "b64_json": b64 }]
            });
            (
                200,
                vec![("Content-Type", "application/json".into())],
                serde_json::to_vec(&resp_json).unwrap(),
            )
        } else {
            (404, vec![], b"Not Found".to_vec())
        }
    })
    .await;

    let secret_store = MemoryStore::new();
    secret_store
        .set(
            ws.org().name(),
            Some(ws.name()),
            "OPENAI_API_KEY",
            "test-key",
        )
        .unwrap();

    let reg = Registry::load(&ws);
    let scheduler = test_scheduler();
    let ctx = ToolRunContext {
        workspace: &ws,
        turn_id: Some("turn-fallback-ok".into()),
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
        "prompt": "Draw character with reference",
        "width": 1280,
        "height": 720,
        "reference_images": ["profile.png"]
    });

    let run_res = run_tool(&reg, "openai.images", input, &ctx).await.unwrap();
    match run_res {
        ToolRun::Success { output, .. } => {
            assert_eq!(
                edits_attempts.load(Ordering::SeqCst),
                1,
                "Should have attempted /images/edits once"
            );
            assert_eq!(
                gen_attempts.load(Ordering::SeqCst),
                1,
                "Should have fallen back to /images/generations once"
            );

            let warnings = output["warnings"]
                .as_array()
                .expect("warnings array in output");
            let warn_text = warnings[0].as_str().unwrap();
            assert!(
                warn_text.contains("không hỗ trợ /images/edits"),
                "Must contain Vietnamese warning about edits fallback: {warn_text}"
            );

            // Resized image to 1280x720
            assert_eq!(output["requested_size"], "1280x720");
            assert_eq!(output["resized"], true);
        }
        other => panic!("Expected ToolRun::Success, got {:?}", other),
    }
}

#[tokio::test]
async fn test_openai_images_seed_warning() {
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();

    let dummy_img = tmp.path().join("mock_seed.png");
    generate_color_image(&dummy_img, 1024, 1024, "white");
    let dummy_bytes = std::fs::read(&dummy_img).unwrap();
    let b64 = BASE64_STANDARD.encode(&dummy_bytes);

    let server = MockServer::new(move |_method, _path, _body| {
        let resp_json = serde_json::json!({
            "data": [{ "b64_json": b64 }]
        });
        (
            200,
            vec![("Content-Type", "application/json".into())],
            serde_json::to_vec(&resp_json).unwrap(),
        )
    })
    .await;

    let secret_store = MemoryStore::new();
    secret_store
        .set(
            ws.org().name(),
            Some(ws.name()),
            "OPENAI_API_KEY",
            "test-key",
        )
        .unwrap();

    let reg = Registry::load(&ws);
    let scheduler = test_scheduler();
    let ctx = ToolRunContext {
        workspace: &ws,
        turn_id: Some("turn-seed-warn".into()),
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
        "prompt": "Test seed warning",
        "width": 1024,
        "height": 1024,
        "seed": 42
    });

    let run_res = run_tool(&reg, "openai.images", input, &ctx).await.unwrap();
    match run_res {
        ToolRun::Success { output, .. } => {
            let warnings = output["warnings"]
                .as_array()
                .expect("warnings array in output");
            assert!(
                warnings
                    .iter()
                    .any(|w| w.as_str().unwrap().contains("không hỗ trợ seed")),
                "Must warn that openai.images ignores seed"
            );
        }
        other => panic!("Expected ToolRun::Success, got {:?}", other),
    }
}

#[tokio::test]
async fn test_openai_images_rejects_outside_workspace_ref() {
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();

    let secret_store = MemoryStore::new();
    secret_store
        .set(
            ws.org().name(),
            Some(ws.name()),
            "OPENAI_API_KEY",
            "test-key",
        )
        .unwrap();

    let reg = Registry::load(&ws);
    let scheduler = test_scheduler();
    let ctx = ToolRunContext {
        workspace: &ws,
        turn_id: Some("turn-outside-ref".into()),
        budget: Some(1.0),
        approved: false,
        scheduler: &scheduler,
        secret_store: &secret_store,
        endpoints: std::collections::HashMap::new(),
        pricing: std::collections::HashMap::new(),
    };

    let input = serde_json::json!({
        "prompt": "Malicious path attempt",
        "width": 1024,
        "height": 1024,
        "reference_images": ["../../outside.png"]
    });

    let run_res = run_tool(&reg, "openai.images", input, &ctx).await;
    assert!(run_res.is_err(), "Must reject outside workspace paths");
}

#[tokio::test]
#[ignore = "Real live gateway check (costs ~$0.08 USD)"]
async fn test_real_check_live_gateway() {
    let env_content = std::fs::read_to_string("D:/Users/Dang/LocalContentStudio/.env.local")
        .expect("Must be able to read .env.local for real check");
    let mut api_key = None;
    for line in env_content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("MUSE_API_KEY=") {
            api_key = Some(rest.trim().to_string());
            break;
        } else if let Some(rest) = line.strip_prefix("OPENAI_API_KEY=") {
            api_key = Some(rest.trim().to_string());
            break;
        }
    }
    let api_key = api_key.expect("API key not found in .env.local");

    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();

    let secret_store = MemoryStore::new();
    secret_store
        .set(ws.org().name(), Some(ws.name()), "OPENAI_API_KEY", &api_key)
        .unwrap();

    let reg = Registry::load(&ws);
    let scheduler = test_scheduler();
    let ctx = ToolRunContext {
        workspace: &ws,
        turn_id: Some("turn-real-1".into()),
        budget: Some(1.0),
        approved: true,
        scheduler: &scheduler,
        secret_store: &secret_store,
        endpoints: std::collections::HashMap::from([(
            "openai.images".into(),
            "https://apirouter.lol/v1".into(),
        )]),
        pricing: std::collections::HashMap::new(),
    };

    // 1. Generation 1280x720
    let gen_input = serde_json::json!({
        "prompt": "Minimalist cup of Vietnamese drip coffee with condensed milk on wooden background, morning light",
        "width": 1280,
        "height": 720
    });
    let gen_res = run_tool(&reg, "openai.images", gen_input, &ctx)
        .await
        .unwrap();
    let gen_output = match gen_res {
        ToolRun::Success { output, .. } => output,
        other => panic!("Expected Success, got {:?}", other),
    };
    assert_eq!(gen_output["requested_size"], "1280x720");
    assert_eq!(gen_output["resized"], true);
    let gen_files = gen_output["files"].as_array().unwrap();
    let gen_file_rel = gen_files[0].as_str().unwrap();
    let gen_path = ws.scoped_path(gen_file_rel).unwrap();
    let dims1 = probe_image_dimensions(&gen_path).unwrap();
    assert_eq!(
        dims1,
        (1280, 720),
        "Generated image must be exactly 1280x720"
    );

    let evidence_dir = std::path::Path::new(
        "D:/Users/Dang/LocalContentStudio/.aw/tasks/2026-10-21-g3-ho-so-series/evidence",
    );
    std::fs::create_dir_all(evidence_dir).unwrap();
    let ev1_path = evidence_dir.join("real_gen_1280x720.webp");
    // Convert to webp <= 300 KB
    let ffmpeg = studio_tools::probe::find_in_path("ffmpeg").unwrap();
    Command::new(&ffmpeg)
        .args([
            "-y",
            "-i",
            &gen_path.to_string_lossy(),
            "-c:v",
            "libwebp",
            "-quality",
            "80",
        ])
        .arg(&ev1_path)
        .status()
        .unwrap();
    assert!(ev1_path.exists());
    let ev1_len = std::fs::metadata(&ev1_path).unwrap().len();
    assert!(
        ev1_len <= 300 * 1024,
        "Image 1 size must be <= 300 KB, got {} bytes",
        ev1_len
    );

    // 2. Edit with 2 reference images
    let ref1_path = ws.root().join("ref1.png");
    let ref2_path = ws.root().join("ref2.png");
    // Make 2 reference crops/images from gen_path
    Command::new(&ffmpeg)
        .args([
            "-y",
            "-i",
            &gen_path.to_string_lossy(),
            "-vf",
            "crop=512:512:0:0",
            "-update",
            "1",
            "-frames:v",
            "1",
        ])
        .arg(&ref1_path)
        .status()
        .unwrap();
    Command::new(&ffmpeg)
        .args([
            "-y",
            "-i",
            &gen_path.to_string_lossy(),
            "-vf",
            "crop=512:512:500:200",
            "-update",
            "1",
            "-frames:v",
            "1",
        ])
        .arg(&ref2_path)
        .status()
        .unwrap();

    let edit_input = serde_json::json!({
        "prompt": "The same cup of coffee but seen with a small plate of pastries next to it, warm cafe setting",
        "width": 1280,
        "height": 720,
        "reference_images": ["ref1.png", "ref2.png"]
    });
    let edit_res = run_tool(&reg, "openai.images", edit_input, &ctx)
        .await
        .unwrap();
    let edit_output = match edit_res {
        ToolRun::Success { output, .. } => output,
        other => panic!("Expected Success, got {:?}", other),
    };
    assert_eq!(edit_output["requested_size"], "1280x720");
    let edit_files = edit_output["files"].as_array().unwrap();
    let edit_file_rel = edit_files[0].as_str().unwrap();
    let edit_path = ws.scoped_path(edit_file_rel).unwrap();
    let dims2 = probe_image_dimensions(&edit_path).unwrap();
    assert_eq!(dims2, (1280, 720), "Edited image must be exactly 1280x720");

    let ev2_path = evidence_dir.join("real_edit_ref.webp");
    Command::new(&ffmpeg)
        .args([
            "-y",
            "-i",
            &edit_path.to_string_lossy(),
            "-c:v",
            "libwebp",
            "-quality",
            "80",
        ])
        .arg(&ev2_path)
        .status()
        .unwrap();
    assert!(ev2_path.exists());
    let ev2_len = std::fs::metadata(&ev2_path).unwrap().len();
    assert!(
        ev2_len <= 300 * 1024,
        "Image 2 size must be <= 300 KB, got {} bytes",
        ev2_len
    );
}

#[tokio::test]
async fn test_openai_images_edits_auth_error_does_not_fall_back() {
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();

    let ref_path = ws.root().join("profile.png");
    generate_color_image(&ref_path, 256, 256, "cyan");

    let gen_attempts = Arc::new(AtomicUsize::new(0));
    let g_cnt = Arc::clone(&gen_attempts);

    let server = MockServer::new(move |_method, path, _body| {
        if path == "/images/edits" {
            (
                401,
                vec![("Content-Type", "application/json".into())],
                br#"{"error":{"message":"Incorrect API key provided"}}"#.to_vec(),
            )
        } else {
            g_cnt.fetch_add(1, Ordering::SeqCst);
            (500, vec![], b"should not be called".to_vec())
        }
    })
    .await;

    let secret_store = MemoryStore::new();
    secret_store
        .set(
            ws.org().name(),
            Some(ws.name()),
            "OPENAI_API_KEY",
            "bad-key",
        )
        .unwrap();

    let reg = Registry::load(&ws);
    let scheduler = test_scheduler();
    let ctx = ToolRunContext {
        workspace: &ws,
        turn_id: Some("turn-edits-401".into()),
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
        "prompt": "Draw character with reference",
        "width": 1280,
        "height": 720,
        "reference_images": ["profile.png"]
    });

    let run_res = run_tool(&reg, "openai.images", input, &ctx).await;
    assert!(
        !matches!(run_res, Ok(ToolRun::Success { .. })),
        "401 from /images/edits must fail, got {run_res:?}"
    );
    assert_eq!(
        gen_attempts.load(Ordering::SeqCst),
        0,
        "Must not silently fall back to /images/generations on auth errors"
    );
}
