use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use studio_web::dispatcher::{BoxFuture, WebDispatcher};
use studio_web::server::{build_router, WebServerState};
use studio_web::{hash_password, WebEvent};
use tokio::sync::broadcast;
use tower::ServiceExt;

struct MockDispatcher {
    password_hash: Arc<RwLock<Option<String>>>,
    workspace_dir: tempfile::TempDir,
}

impl MockDispatcher {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("preview.txt"),
            b"Hello Local Content Studio",
        )
        .expect("write test file");
        Self {
            password_hash: Arc::new(RwLock::new(None)),
            workspace_dir: dir,
        }
    }
}

impl WebDispatcher for MockDispatcher {
    fn invoke<'a>(
        &'a self,
        command: &'a str,
        _args: Value,
        _is_sudo: bool,
    ) -> BoxFuture<'a, Result<Value, String>> {
        Box::pin(async move {
            match command {
                "project_list" => Ok(json!([{ "slug": "demo", "title": "Demo Project" }])),
                "checkpoint_restore" => Ok(json!({ "restored": true })),
                other => Err(format!("Unknown command: {other}")),
            }
        })
    }

    fn resolve_file(&self, _project: Option<&str>, path: &str) -> Result<PathBuf, String> {
        let p = Path::new(path);
        if p.is_absolute() || path.contains("..") {
            return Err("Tệp nằm ngoài workspace.".into());
        }
        let full = self.workspace_dir.path().join(p);
        if full.exists() {
            Ok(full)
        } else {
            Err("Không tìm thấy tệp.".into())
        }
    }

    fn get_password_hash(&self) -> Result<Option<String>, String> {
        Ok(self.password_hash.read().unwrap().clone())
    }

    fn set_password_hash(&self, hash: &str) -> Result<(), String> {
        *self.password_hash.write().unwrap() = Some(hash.to_string());
        Ok(())
    }
}

fn setup_test_app() -> (
    axum::Router,
    Arc<MockDispatcher>,
    broadcast::Sender<WebEvent>,
) {
    let dispatcher = Arc::new(MockDispatcher::new());
    let (tx, _) = broadcast::channel(16);
    let state = Arc::new(WebServerState {
        dispatcher: dispatcher.clone(),
        sessions: Arc::new(studio_web::auth::SessionStore::new()),
        rate_limiter: Arc::new(studio_web::auth::LoginRateLimiter::new()),
        journal: studio_web::events::EventJournal::attach(&tx),
        lan_allowed: false,
        port: 3000,
        tls_info: None,
        limits: studio_web::WebLimitsConfig::default(),
        allowed_hosts: Vec::new(),
        behind_https_proxy: false,
    });
    (build_router(state), dispatcher, tx)
}

#[tokio::test]
async fn test_auth_required_for_invoke_and_file() {
    let (app, _, _) = setup_test_app();

    // 1. Invoke without session -> 401
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/invoke/project_list")
        .header("host", "127.0.0.1:3000")
        .header("x-studio-csrf", "1")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 2. File without session -> 401
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/file?path=preview.txt")
        .header("host", "127.0.0.1:3000")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_csrf_and_origin_rejection() {
    let (app, _, _) = setup_test_app();

    // 1. POST without CSRF header -> 403
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/auth/login")
        .header("host", "127.0.0.1:3000")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "password": "pass" }).to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 2. Request with malicious Origin -> 403
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/auth/login")
        .header("host", "127.0.0.1:3000")
        .header("origin", "http://evil-site.com")
        .header("x-studio-csrf", "1")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "password": "pass" }).to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 3. Request with external Host when LAN disabled -> 403
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/auth/login")
        .header("host", "192.168.1.50:3000")
        .header("x-studio-csrf", "1")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "password": "pass" }).to_string()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_wrong_password_lockout() {
    let (app, dispatcher, _) = setup_test_app();
    let hashed = hash_password("correct_password").unwrap();
    dispatcher.set_password_hash(&hashed).unwrap();

    // 4 wrong attempts -> 401
    for _ in 0..4 {
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/auth/login")
            .header("host", "127.0.0.1:3000")
            .header("x-studio-csrf", "1")
            .header("content-type", "application/json")
            .body(Body::from(json!({ "password": "wrong" }).to_string()))
            .unwrap();

        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    // 5th wrong attempt -> 429 Too Many Requests (locked out)
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/auth/login")
        .header("host", "127.0.0.1:3000")
        .header("x-studio-csrf", "1")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "password": "wrong" }).to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);

    // Even with correct password, now locked out
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/auth/login")
        .header("host", "127.0.0.1:3000")
        .header("x-studio-csrf", "1")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "password": "correct_password" }).to_string(),
        ))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn test_dispatcher_allowlist_and_successful_invoke() {
    let (app, dispatcher, _) = setup_test_app();
    let hashed = hash_password("secret123").unwrap();
    dispatcher.set_password_hash(&hashed).unwrap();

    // Login successfully
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/auth/login")
        .header("host", "127.0.0.1:3000")
        .header("x-studio-csrf", "1")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "password": "secret123" }).to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cookie = res
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // 1. Invoke allowed command -> 200 OK
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/invoke/project_list")
        .header("host", "127.0.0.1:3000")
        .header("x-studio-csrf", "1")
        .header("cookie", &cookie)
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(val[0]["slug"], "demo");

    // 2. Invoke disallowed command -> 403 Forbidden
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/invoke/evil_disallowed_command")
        .header("host", "127.0.0.1:3000")
        .header("x-studio-csrf", "1")
        .header("cookie", &cookie)
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_file_endpoint_scoping_and_range() {
    let (app, dispatcher, _) = setup_test_app();
    let hashed = hash_password("secret123").unwrap();
    dispatcher.set_password_hash(&hashed).unwrap();

    // Login
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/auth/login")
        .header("host", "127.0.0.1:3000")
        .header("x-studio-csrf", "1")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "password": "secret123" }).to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    let cookie = res
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // 1. Path traversal refusal -> 403
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/file?path=../windows/system32/cmd.exe")
        .header("host", "127.0.0.1:3000")
        .header("cookie", &cookie)
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 2. Normal file read -> 200 OK
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/file?path=preview.txt")
        .header("host", "127.0.0.1:3000")
        .header("cookie", &cookie)
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&bytes[..], b"Hello Local Content Studio");

    // 3. Range request for video/media streaming -> 206 Partial Content
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/file?path=preview.txt")
        .header("host", "127.0.0.1:3000")
        .header("cookie", &cookie)
        .header("range", "bytes=0-4")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        res.headers().get(header::CONTENT_RANGE).unwrap(),
        "bytes 0-4/26"
    );
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&bytes[..], b"Hello");
}

#[tokio::test]
async fn test_https_session_cookie_secure_flag() {
    let (app, dispatcher, _) = setup_test_app();
    let hashed = hash_password("secret123").unwrap();
    dispatcher.set_password_hash(&hashed).unwrap();

    // 1. Plain HTTP request: no Secure
    let req_http = Request::builder()
        .method(Method::POST)
        .uri("/api/auth/login")
        .header("host", "127.0.0.1:3000")
        .header("x-studio-csrf", "1")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "password": "secret123" }).to_string()))
        .unwrap();

    let res_http = app.clone().oneshot(req_http).await.unwrap();
    assert_eq!(res_http.status(), StatusCode::OK);
    let cookie_http = res_http
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(!cookie_http.contains("Secure"));

    // 2. HTTPS request with x-forwarded-proto: https -> has Secure
    let req_https = Request::builder()
        .method(Method::POST)
        .uri("/api/auth/login")
        .header("host", "127.0.0.1:3000")
        .header("x-forwarded-proto", "https")
        .header("x-studio-csrf", "1")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "password": "secret123" }).to_string()))
        .unwrap();

    let res_https = app.clone().oneshot(req_https).await.unwrap();
    assert_eq!(res_https.status(), StatusCode::OK);
    let cookie_https = res_https
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(cookie_https.contains("Secure"));

    // 3. HTTPS request with Origin: https://... -> has Secure
    let req_origin_https = Request::builder()
        .method(Method::POST)
        .uri("/api/auth/login")
        .header("host", "127.0.0.1:3000")
        .header("origin", "https://127.0.0.1:3000")
        .header("x-studio-csrf", "1")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "password": "secret123" }).to_string()))
        .unwrap();

    let res_origin = app.oneshot(req_origin_https).await.unwrap();
    assert_eq!(res_origin.status(), StatusCode::OK);
    let cookie_origin = res_origin
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(cookie_origin.contains("Secure"));
}

#[tokio::test]
async fn test_json_and_upload_body_limits() {
    let dispatcher = Arc::new(MockDispatcher::new());
    let (tx, _) = broadcast::channel(16);
    // Custom limit: JSON limit 500 bytes, Upload limit 2 KB
    let limits = studio_web::WebLimitsConfig {
        json_body_limit_bytes: 500,
        upload_body_limit_bytes: 2048,
    };
    let state = Arc::new(WebServerState {
        dispatcher: dispatcher.clone(),
        sessions: Arc::new(studio_web::auth::SessionStore::new()),
        rate_limiter: Arc::new(studio_web::auth::LoginRateLimiter::new()),
        journal: studio_web::events::EventJournal::attach(&tx),
        lan_allowed: false,
        port: 3000,
        tls_info: None,
        limits,
        allowed_hosts: Vec::new(),
        behind_https_proxy: false,
    });
    let app = build_router(state.clone());

    // 1. JSON payload over 500 bytes -> 413 Payload Too Large
    let big_json = serde_json::to_string(&json!({ "password": "a".repeat(600) })).unwrap();
    let req_big_json = Request::builder()
        .method(Method::POST)
        .uri("/api/auth/login")
        .header("host", "127.0.0.1:3000")
        .header("x-studio-csrf", "1")
        .header("content-type", "application/json")
        .body(Body::from(big_json))
        .unwrap();

    let res_big_json = app.clone().oneshot(req_big_json).await.unwrap();
    assert_eq!(res_big_json.status(), StatusCode::PAYLOAD_TOO_LARGE);

    let session = state.sessions.create_session();
    // 2. Streamed multipart body exceeding upload limit -> 413 Payload Too Large
    let boundary = "------------------------1234567890";
    let chunk = vec![b'x'; 3000];
    let stream = futures_util::stream::iter(vec![
        Ok::<_, std::io::Error>(bytes::Bytes::from(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"limit-test-413.bin\"\r\nContent-Type: application/octet-stream\r\n\r\n"))),
        Ok(bytes::Bytes::from(chunk.clone())),
        Ok(bytes::Bytes::from(chunk.clone())),
        Ok(bytes::Bytes::from(chunk.clone())), // > 3KB total, exceeds 2048 bytes
        Ok(bytes::Bytes::from(format!("\r\n--{boundary}--\r\n"))),
    ]);
    let req_big_upload = Request::builder()
        .method(Method::POST)
        .uri("/api/upload")
        .header("host", "127.0.0.1:3000")
        .header("cookie", format!("studio_session={}", session.token))
        .header("x-studio-csrf", "1")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from_stream(stream))
        .unwrap();

    let res_big_upload = app.oneshot(req_big_upload).await.unwrap();
    assert_eq!(res_big_upload.status(), StatusCode::PAYLOAD_TOO_LARGE);

    // The rejected upload leaves no partial file behind.
    let leftovers = std::fs::read_dir(std::env::temp_dir().join("studio_uploads"))
        .map(|d| {
            d.filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .ends_with("_limit-test-413.bin")
                })
                .count()
        })
        .unwrap_or(0);
    assert_eq!(leftovers, 0);
}

#[tokio::test]
async fn test_allowed_hosts_and_origin_blocking() {
    let dispatcher = Arc::new(MockDispatcher::new());
    let (tx, _) = broadcast::channel(16);
    let state = Arc::new(WebServerState {
        dispatcher: dispatcher.clone(),
        sessions: Arc::new(studio_web::auth::SessionStore::new()),
        rate_limiter: Arc::new(studio_web::auth::LoginRateLimiter::new()),
        journal: studio_web::events::EventJournal::attach(&tx),
        lan_allowed: false,
        port: 3000,
        tls_info: None,
        limits: studio_web::WebLimitsConfig::default(),
        allowed_hosts: vec!["dang-server.tailnet.ts.net".to_string()],
        behind_https_proxy: true,
    });
    let app = build_router(state);

    // 1. Valid request via Tailscale domain
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/auth/status")
        .header("host", "dang-server.tailnet.ts.net")
        .header("origin", "https://dang-server.tailnet.ts.net")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 2. Foreign Origin attack attempting to access the tailscale server -> 403 Forbidden
    let evil_req = Request::builder()
        .method(Method::GET)
        .uri("/api/auth/status")
        .header("host", "dang-server.tailnet.ts.net")
        .header("origin", "https://malicious-website.com")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(evil_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 3. Unknown host not in allowed_hosts -> 403 Forbidden
    let bad_host_req = Request::builder()
        .method(Method::GET)
        .uri("/api/auth/status")
        .header("host", "random-domain.com")
        .header("origin", "https://random-domain.com")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(bad_host_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_behind_https_proxy_cookie_secure() {
    let dispatcher = Arc::new(MockDispatcher::new());
    let test_pw = ["test-", "proxy-", "pw123"].concat();
    let hashed = studio_web::auth::hash_password(&test_pw).unwrap();
    dispatcher.set_password_hash(&hashed).unwrap();

    let (tx, _) = broadcast::channel(16);
    let state = Arc::new(WebServerState {
        dispatcher: dispatcher.clone(),
        sessions: Arc::new(studio_web::auth::SessionStore::new()),
        rate_limiter: Arc::new(studio_web::auth::LoginRateLimiter::new()),
        journal: studio_web::events::EventJournal::attach(&tx),
        lan_allowed: false,
        port: 3000,
        tls_info: None,
        limits: studio_web::WebLimitsConfig::default(),
        allowed_hosts: vec!["dang-server.ts.net".to_string()],
        behind_https_proxy: true,
    });
    let app = build_router(state);

    // Login through reverse proxy over plain HTTP connection to loopback
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/auth/login")
        .header("host", "dang-server.ts.net")
        .header("origin", "http://dang-server.ts.net")
        .header("x-studio-csrf", "1")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "password": test_pw }).to_string()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let cookie = res
        .headers()
        .get(header::SET_COOKIE)
        .expect("set-cookie")
        .to_str()
        .unwrap();
    assert!(
        cookie.contains("Secure"),
        "Cookie must contain Secure flag when behind_https_proxy is true: {}",
        cookie
    );
}
