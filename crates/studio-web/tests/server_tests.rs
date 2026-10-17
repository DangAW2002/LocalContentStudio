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
        events_tx: tx.clone(),
        lan_allowed: false,
        port: 3000,
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
