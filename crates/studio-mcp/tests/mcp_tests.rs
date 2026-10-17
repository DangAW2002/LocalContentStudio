use std::path::Path;
use std::sync::Arc;

use serde_json::{json, Value};
use studio_jobs::{EstimateStore, MemoryEstimateStore, Scheduler, SchedulerConfig, SysinfoProbe};
use studio_mcp::{
    server, shim, DefaultBroker, EngineKind, RuntimeInfo, ServerState, TokenRegistry, TurnContext,
};
use studio_tools::MemoryStore;
use studio_workspace::Workspace;
use tempfile::tempdir;

fn state() -> Arc<ServerState> {
    let estimates: Arc<dyn EstimateStore> = Arc::new(MemoryEstimateStore::new());
    Arc::new(ServerState {
        tokens: Arc::new(TokenRegistry::new()),
        scheduler: Scheduler::new(
            SchedulerConfig::default(),
            Arc::new(SysinfoProbe::new()),
            estimates,
        ),
        secrets: Arc::new(MemoryStore::new()),
        broker: Arc::new(DefaultBroker),
        catalog_paths: vec![],
        shim_secret: "shim-secret".into(),
        endpoints: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
    })
}

fn ctx(ws: &Path, engine: EngineKind) -> TurnContext {
    TurnContext {
        workspace_dir: ws.to_path_buf(),
        turn_id: "turn-1".into(),
        project: None,
        engine,
        allowed_capabilities: None,
        budget_usd: Some(1.0),
    }
}

async fn post(port: u16, token: Option<&str>, origin: Option<&str>, body: Value) -> (u16, Value) {
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let mut req = client
        .post(format!("http://127.0.0.1:{port}/mcp"))
        .json(&body);
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    if let Some(o) = origin {
        req = req.header("Origin", o);
    }
    let resp = req.send().await.unwrap();
    let status = resp.status().as_u16();
    (status, resp.json().await.unwrap_or(Value::Null))
}

fn call(name: &str, args: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": args } })
}

fn text_of(resp: &Value) -> (String, bool) {
    let text = resp
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let err = resp
        .pointer("/result/isError")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    (text, err)
}

#[tokio::test]
async fn auth_origin_and_tools_list() {
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("a"), "A").unwrap();
    let st = state();
    let token = st.tokens.issue(ctx(ws.root(), EngineKind::Claude));
    let srv = server::start(st.clone(), 0).await.unwrap();
    let port = srv.addr.port();
    let list = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" });

    assert_eq!(post(port, None, None, list.clone()).await.0, 401);
    assert_eq!(
        post(port, Some("lcs_wrong"), None, list.clone()).await.0,
        401
    );
    assert_eq!(
        post(
            port,
            Some(&token),
            Some("https://evil.example"),
            list.clone()
        )
        .await
        .0,
        403
    );

    let (status, body) = post(port, Some(&token), Some("http://127.0.0.1:3000"), list).await;
    assert_eq!(status, 200);
    let names: Vec<&str> = body["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    for expected in [
        "workspace_map",
        "capability_run",
        "check_run",
        "approve",
        "catalog_enable",
    ] {
        assert!(names.contains(&expected), "missing {expected}");
    }

    st.tokens.revoke(&token);
    let (status, _) = post(
        port,
        Some(&token),
        None,
        json!({ "jsonrpc": "2.0", "id": 2, "method": "ping" }),
    )
    .await;
    assert_eq!(status, 401);
}

#[tokio::test]
async fn token_is_scoped_to_its_workspace() {
    let tmp = tempdir().unwrap();
    let a = Workspace::create(tmp.path().join("a"), "A").unwrap();
    let b = Workspace::create(tmp.path().join("b"), "B").unwrap();
    std::fs::write(b.root().join("secret.md"), "workspace B").unwrap();
    std::fs::write(a.root().join("hello.md"), "workspace A").unwrap();
    let st = state();
    let token = st.tokens.issue(ctx(a.root(), EngineKind::Claude));
    let srv = server::start(st, 0).await.unwrap();
    let port = srv.addr.port();

    let (_, ok) = post(
        port,
        Some(&token),
        None,
        call("artifact_get", json!({ "path": "hello.md" })),
    )
    .await;
    assert!(text_of(&ok).0.contains("workspace A"));

    for path in [
        "../b/secret.md",
        b.root().join("secret.md").to_string_lossy().as_ref(),
    ] {
        let (_, resp) = post(
            port,
            Some(&token),
            None,
            call("artifact_get", json!({ "path": path })),
        )
        .await;
        let (text, is_err) = text_of(&resp);
        assert!(is_err, "{path} should be rejected");
        assert!(!text.contains("workspace B"));
    }
}

#[tokio::test]
async fn approve_policy() {
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("a"), "A").unwrap();
    let st = state();
    let token = st.tokens.issue(ctx(ws.root(), EngineKind::Claude));
    let srv = server::start(st, 0).await.unwrap();
    let port = srv.addr.port();
    let decide = |input: Value| call("approve", json!({ "tool_name": "Bash", "input": input }));

    let (_, r) = post(
        port,
        Some(&token),
        None,
        decide(json!({ "command": "npx skills add heygen-com/hyperframes" })),
    )
    .await;
    let v: Value = serde_json::from_str(&text_of(&r).0).unwrap();
    assert_eq!(v["behavior"], "deny");

    let outside = tmp.path().join("outside.txt").to_string_lossy().to_string();
    let (_, r) = post(
        port,
        Some(&token),
        None,
        call(
            "approve",
            json!({ "tool_name": "Write", "input": { "file_path": outside } }),
        ),
    )
    .await;
    let v: Value = serde_json::from_str(&text_of(&r).0).unwrap();
    assert_eq!(v["behavior"], "deny");

    let (_, r) = post(
        port,
        Some(&token),
        None,
        decide(json!({ "command": "node -e \"console.log(1)\"" })),
    )
    .await;
    let v: Value = serde_json::from_str(&text_of(&r).0).unwrap();
    assert_eq!(v["behavior"], "allow");
    assert_eq!(v["updatedInput"]["command"], "node -e \"console.log(1)\"");
}

#[tokio::test]
async fn capability_run_echo_and_search() {
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("a"), "A").unwrap();
    let st = state();
    let token = st.tokens.issue(ctx(ws.root(), EngineKind::Claude));
    let srv = server::start(st, 0).await.unwrap();
    let port = srv.addr.port();

    let (_, r) = post(
        port,
        Some(&token),
        None,
        call("capability_search", json!({ "query": "tts" })),
    )
    .await;
    assert!(text_of(&r).0.contains("tts.synthesize@1"));

    let (_, r) = post(
        port,
        Some(&token),
        None,
        call(
            "capability_run",
            json!({ "capability": "echo@1", "input": { "hello": "studio" } }),
        ),
    )
    .await;
    let (text, is_err) = text_of(&r);
    assert!(!is_err, "{text}");
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["result"]["hello"], "studio");
}

#[tokio::test]
async fn shim_obtains_agy_token_and_forwards() {
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("a"), "A").unwrap();
    let st = state();
    let srv = server::start(st.clone(), 0).await.unwrap();
    let info = RuntimeInfo {
        port: srv.addr.port(),
        shim_secret: "shim-secret".into(),
        pid: std::process::id(),
    };
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let nested = ws.root().join("projects");

    // No active agy turn yet.
    assert!(
        shim::fetch_token(&client, &info, std::slice::from_ref(&nested))
            .await
            .is_err()
    );
    // A claude turn does not count.
    st.tokens.issue(ctx(ws.root(), EngineKind::Claude));
    assert!(
        shim::fetch_token(&client, &info, std::slice::from_ref(&nested))
            .await
            .is_err()
    );
    // Wrong secret.
    let bad = RuntimeInfo {
        shim_secret: "nope".into(),
        ..info.clone()
    };
    st.tokens.issue(ctx(ws.root(), EngineKind::Agy));
    assert!(
        shim::fetch_token(&client, &bad, std::slice::from_ref(&nested))
            .await
            .is_err()
    );

    let token = shim::fetch_token(&client, &info, &[nested]).await.unwrap();
    let reply = shim::forward(
        &client,
        info.port,
        &token,
        &json!({ "jsonrpc": "2.0", "id": 7, "method": "tools/list" }),
    )
    .await
    .unwrap();
    assert_eq!(reply["id"], 7);
    assert!(reply["result"]["tools"].as_array().unwrap().len() >= 10);
    assert!(shim::forward(
        &client,
        info.port,
        &token,
        &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })
    )
    .await
    .is_none());
}

#[tokio::test]
async fn test_check_run_render_handling() {
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("a"), "A").unwrap();
    let proj = ws.create_project("test-proj", "Test Project").unwrap();

    // Setup brand and channel
    let brand_dir = ws.root().join("brands").join("example");
    std::fs::create_dir_all(&brand_dir).unwrap();
    std::fs::write(
        brand_dir.join("brand.yaml"),
        "version: brand@1\nname: Example\ncolors:\n  primary: \"#000000\"\n  accent: \"#FFFFFF\"\n  bg: \"#FFFFFF\"\n  text: \"#000000\"\nfonts:\n  heading: Inter\n  body: Inter\nlogos:\n  full: logo.svg\n  mark: mark.svg\nvoice: voice.md\n",
    ).unwrap();
    std::fs::write(brand_dir.join("logo.svg"), "<svg></svg>").unwrap();
    std::fs::write(brand_dir.join("mark.svg"), "<svg></svg>").unwrap();
    std::fs::write(brand_dir.join("voice.md"), "Voice guidelines").unwrap();

    let channels_dir = ws.root().join("channels");
    std::fs::create_dir_all(&channels_dir).unwrap();
    std::fs::write(
        channels_dir.join("youtube-thumb.yaml"),
        "version: channel@1\nid: youtube-thumb\nplatform: youtube\ntitle: YouTube Thumbnail\nformats:\n  - name: thumb\n    width: 1280\n    height: 720\n    aspect: \"16:9\"\n",
    ).unwrap();

    // Create artifact
    let art_dir = proj.artifacts_dir().join("thumb-01");
    std::fs::create_dir_all(&art_dir).unwrap();
    std::fs::write(
        art_dir.join("artifact.yaml"),
        "id: thumb-01\nkind: design\ntitle: Thumbnail 1\nbrand: example\nchannels: [youtube-thumb]\nsource: index.html\n",
    ).unwrap();
    std::fs::write(
        art_dir.join("index.html"),
        "<html><body>Hello</body></html>",
    )
    .unwrap();

    // 1. Check with NO renders -> must return warning finding "Chưa có bản render"
    let args_no_renders = json!({
        "artifact_dir": format!("projects/{}/artifacts/thumb-01", proj.slug()),
        "brand": "example",
        "channels": ["youtube-thumb"],
        "renders": []
    });
    let res = studio_mcp::tools::check_run(&ws, &args_no_renders).unwrap();
    let findings = res["channels"]["youtube-thumb"]["findings"]
        .as_array()
        .unwrap();
    assert!(findings.iter().any(|f| f["code"] == "NO_RENDERS"
        && f["message_vi"]
            .as_str()
            .unwrap()
            .contains("Chưa có bản render")));

    // 2. Create a 1080x1350 render image in renders directory -> fail 1280x720 channel spec
    let renders_dir = proj.renders_dir().join("thumb-01").join("hash1234");
    std::fs::create_dir_all(&renders_dir).unwrap();
    let render_file = renders_dir.join("output.png");
    let img = image::RgbImage::new(1080, 1350);
    img.save(&render_file).unwrap();

    // check_run should automatically discover this latest render
    let res_mismatch = studio_mcp::tools::check_run(&ws, &args_no_renders).unwrap();
    let findings_mismatch = res_mismatch["channels"]["youtube-thumb"]["findings"]
        .as_array()
        .unwrap();
    assert!(findings_mismatch
        .iter()
        .any(|f| f["code"] == "DIMENSIONS_MISMATCH"));
    assert_eq!(res_mismatch["passed"], false);
    // Check channel expected_specs is populated
    assert_eq!(
        res_mismatch["channels"]["youtube-thumb"]["expected_specs"]["formats"][0]["width"],
        1280
    );

    // 3. Create matching 1280x720 render image -> passes
    let img_pass = image::RgbImage::new(1280, 720);
    img_pass.save(&render_file).unwrap();
    let res_pass = studio_mcp::tools::check_run(&ws, &args_no_renders).unwrap();
    let findings_pass = res_pass["channels"]["youtube-thumb"]["findings"]
        .as_array()
        .unwrap();
    assert!(findings_pass.is_empty() || !findings_pass.iter().any(|f| f["severity"] == "error"));
    assert_eq!(res_pass["passed"], true);
}

#[tokio::test]
async fn test_capability_relocation_and_indexing() {
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("a"), "A").unwrap();
    let proj = ws.create_project("my-project", "My Project").unwrap();

    let st = state();
    let mut turn_ctx = ctx(ws.root(), EngineKind::Claude);
    turn_ctx.project = Some(proj.slug().to_string());
    turn_ctx.turn_id = "turn-ai-1".into();

    let token = st.tokens.issue(turn_ctx.clone());
    let srv = server::start(st.clone(), 0).await.unwrap();
    let port = srv.addr.port();

    // Create a temporary generated file in the turn output directory to simulate tool output
    let turn_dir = ws
        .root()
        .join(".studio")
        .join("cache")
        .join("turns")
        .join("turn-ai-1");
    std::fs::create_dir_all(&turn_dir).unwrap();
    let temp_asset = turn_dir.join("generated_art.png");
    let img = image::RgbImage::new(512, 512);
    img.save(&temp_asset).unwrap();

    let rel_source = ".studio/cache/turns/turn-ai-1/generated_art.png".to_string();

    let call_body = call(
        "capability_run",
        json!({
            "capability": "echo@1",
            "provider": "echo",
            "input": {
                "prompt": "generate a sunset picture",
                "files": [rel_source],
                "provider_metadata": {
                    "provider": "test.ai",
                    "model": "model-v1",
                    "estimated_cost_usd": 0.05
                }
            }
        }),
    );

    let (status, resp) = post(port, Some(&token), None, call_body).await;
    assert_eq!(status, 200, "Response: {:?}", resp);

    let (text, is_err) = text_of(&resp);
    assert!(!is_err, "Error in capability_run: {}", text);

    let out_json: Value = serde_json::from_str(&text).unwrap();
    let result_obj = &out_json["result"];
    let files = result_obj["files"].as_array().unwrap();
    assert_eq!(files.len(), 1);

    let rel_file = files[0].as_str().unwrap();
    // Path should be relocated to projects/my-project/assets/generated/<YYYY-MM-DD>/...
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let expected_prefix = format!("projects/my-project/assets/generated/{}", today);
    assert!(
        rel_file.starts_with(&expected_prefix),
        "Relocated path {} should start with {}",
        rel_file,
        expected_prefix
    );

    // Verify physical file exists at relocated path
    let abs_file = ws.scoped_path(rel_file).unwrap();
    assert!(abs_file.exists());

    // Verify assets_index entry in WorkspaceDb
    let db = studio_workspace::WorkspaceDb::open(&ws).unwrap();
    let mut stmt = db
        .conn()
        .prepare("SELECT path, name, metadata_json FROM assets_index WHERE id = ?1")
        .unwrap();
    let row: (String, String, String) = stmt
        .query_row([rel_file], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap();
    assert_eq!(row.0, rel_file);
    let meta: Value = serde_json::from_str(&row.2).unwrap();
    assert_eq!(meta["turn_id"], "turn-ai-1");
    assert_eq!(meta["prompt"], "generate a sunset picture");
}
