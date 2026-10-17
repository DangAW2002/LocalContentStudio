//! Local Content Studio Web Server ("Chế độ web cá nhân").
//!
//! Provides a secure, single-owner Axum web server exposing the desktop UI
//! and backend commands over loopback or local LAN.

pub mod allowlist;
pub mod auth;
pub mod dispatcher;
pub mod events;
pub mod file;
pub mod lan;
pub mod qr;
pub mod security;
pub mod server;
pub mod upload;

pub use auth::{hash_password, verify_password, Session, SessionStore};
pub use dispatcher::{BoxFuture, WebDispatcher};
pub use events::WebEvent;
pub use lan::{detect_lan_ip, format_lan_url};
pub use qr::generate_qr_svg;
pub use server::{build_router, start, RunningWebServer, WebServerConfig, WebServerState};
