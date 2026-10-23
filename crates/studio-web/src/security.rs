//! Security checks: Origin/Host verification, CSRF headers, and sudo protection.

use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

fn extract_host(raw: &str) -> &str {
    let trimmed = raw
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    trimmed.split(['/', ':']).next().unwrap_or("")
}

fn extract_scheme(raw: &str) -> Option<&str> {
    if raw.starts_with("https://") {
        Some("https")
    } else if raw.starts_with("http://") {
        Some("http")
    } else {
        None
    }
}

/// Check if the Origin and Host headers are valid to prevent DNS rebinding and cross-site requests.
///
/// When `lan_allowed` is true, accepts either HTTP or HTTPS Origin matching the Host or LAN IP/loopback.
/// Rejects origins with mismatched hosts or non-HTTP/HTTPS schemes.
/// Check if the Origin and Host headers are valid to prevent DNS rebinding and cross-site requests.
///
/// When `lan_allowed` is true, accepts either HTTP or HTTPS Origin matching the Host or LAN IP/loopback.
/// When `lan_allowed` is false, accepts loopback (127.0.0.1/localhost) or any host matching `allowed_hosts`.
/// If Origin is provided, it must match the requested Host (preventing CSRF from foreign origins).
pub fn verify_origin_and_host(
    headers: &HeaderMap,
    lan_allowed: bool,
    allowed_hosts: &[String],
) -> bool {
    let host_hdr = match headers.get("host").and_then(|v| v.to_str().ok()) {
        Some(h) => h,
        None => return false,
    };
    let host_name = extract_host(host_hdr);

    // Check if host is loopback or in allowed_hosts (e.g. Tailscale domain or custom hostname)
    let is_loopback = host_name == "127.0.0.1" || host_name == "localhost";
    let is_allowed_host = allowed_hosts.iter().any(|allowed| {
        let pattern = extract_host(allowed);
        if pattern.starts_with("*.") {
            host_name.ends_with(&pattern[1..])
        } else {
            pattern.eq_ignore_ascii_case(host_name)
        }
    });

    if !lan_allowed && !is_loopback && !is_allowed_host {
        return false;
    }

    // If Origin is present (e.g., standard browser fetch / POST), it must match the Host.
    if let Some(origin_val) = headers.get("origin").and_then(|v| v.to_str().ok()) {
        let scheme = match extract_scheme(origin_val) {
            Some(s) => s,
            None => return false, // Reject non-http/https origins like file://, javascript:, etc.
        };

        if scheme != "http" && scheme != "https" {
            return false;
        }

        let origin_host = extract_host(origin_val);
        // Compare hosts
        if !origin_host.eq_ignore_ascii_case(host_name) {
            // Also allow localhost <-> 127.0.0.1 equivalence on loopback
            let is_loopback = (origin_host == "127.0.0.1" || origin_host == "localhost")
                && (host_name == "127.0.0.1" || host_name == "localhost");
            if !is_loopback {
                return false;
            }
        }
    }

    true
}

/// Verify custom CSRF header on POST, PUT, DELETE requests.
pub fn verify_csrf_header(method: &Method, headers: &HeaderMap) -> bool {
    if *method == Method::POST || *method == Method::PUT || *method == Method::DELETE {
        // Must contain X-Studio-Csrf or X-Requested-With header
        let has_csrf = headers
            .get("x-studio-csrf")
            .and_then(|v| v.to_str().ok())
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false);
        let has_requested_with = headers
            .get("x-requested-with")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.eq_ignore_ascii_case("studio") || v.eq_ignore_ascii_case("xmlhttprequest"))
            .unwrap_or(false);

        has_csrf || has_requested_with
    } else {
        true
    }
}

/// Standard forbidden response for security violations.
pub fn forbidden_response(reason: &str) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(json!({
            "error": reason,
            "code": "FORBIDDEN"
        })),
    )
        .into_response()
}

/// Sudo required response.
pub fn sudo_required_response() -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(json!({
            "code": "SUDO_REQUIRED",
            "error": "Hành động yêu cầu xác thực lại mật khẩu của chủ sở hữu."
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_origin_and_host_loopback() {
        let mut headers = HeaderMap::new();
        headers.insert("host", HeaderValue::from_static("127.0.0.1:3000"));
        headers.insert("origin", HeaderValue::from_static("http://127.0.0.1:3000"));

        assert!(verify_origin_and_host(&headers, false, &[]));
        assert!(verify_origin_and_host(&headers, true, &[]));

        // Cross-origin attack attempt
        headers.insert(
            "origin",
            HeaderValue::from_static("http://evil-attacker.com"),
        );
        assert!(!verify_origin_and_host(&headers, false, &[]));
        assert!(!verify_origin_and_host(&headers, true, &[]));
    }

    #[test]
    fn test_origin_and_host_https_lan() {
        let mut headers = HeaderMap::new();
        headers.insert("host", HeaderValue::from_static("192.168.1.50:3000"));
        headers.insert(
            "origin",
            HeaderValue::from_static("https://192.168.1.50:3000"),
        );

        // LAN allowed -> accepted
        assert!(verify_origin_and_host(&headers, true, &[]));

        // Wrong host rejected
        headers.insert(
            "origin",
            HeaderValue::from_static("https://192.168.1.99:3000"),
        );
        assert!(!verify_origin_and_host(&headers, true, &[]));

        // Wrong scheme (e.g. ftp:// or file://) rejected
        headers.insert(
            "origin",
            HeaderValue::from_static("file://192.168.1.50:3000"),
        );
        assert!(!verify_origin_and_host(&headers, true, &[]));
    }

    #[test]
    fn test_origin_and_host_lan_rejection_when_disabled() {
        let mut headers = HeaderMap::new();
        headers.insert("host", HeaderValue::from_static("192.168.1.15:3000"));
        headers.insert(
            "origin",
            HeaderValue::from_static("http://192.168.1.15:3000"),
        );

        // Rejected when LAN not allowed
        assert!(!verify_origin_and_host(&headers, false, &[]));
        // Accepted when LAN allowed
        assert!(verify_origin_and_host(&headers, true, &[]));
    }

    #[test]
    fn test_origin_and_host_allowed_hosts_ts_net() {
        let allowed = vec!["dang-server.my-tailnet.ts.net".to_string()];
        let mut headers = HeaderMap::new();
        headers.insert(
            "host",
            HeaderValue::from_static("dang-server.my-tailnet.ts.net"),
        );
        headers.insert(
            "origin",
            HeaderValue::from_static("https://dang-server.my-tailnet.ts.net"),
        );

        // Even with LAN false (loopback bind behind tailscale serve), allowed-host is accepted
        assert!(verify_origin_and_host(&headers, false, &allowed));

        // Foreign origin attack to legitimate host is strictly rejected (403 Origin mismatch)
        headers.insert(
            "origin",
            HeaderValue::from_static("https://evil-attacker.com"),
        );
        assert!(!verify_origin_and_host(&headers, false, &allowed));

        // Untrusted Host is rejected
        let mut bad_host_headers = HeaderMap::new();
        bad_host_headers.insert("host", HeaderValue::from_static("evil-attacker.com"));
        bad_host_headers.insert(
            "origin",
            HeaderValue::from_static("https://evil-attacker.com"),
        );
        assert!(!verify_origin_and_host(&bad_host_headers, false, &allowed));
    }

    #[test]
    fn test_origin_and_host_allowed_hosts_wildcard() {
        let allowed = vec!["*.ts.net".to_string()];
        let mut headers = HeaderMap::new();
        headers.insert("host", HeaderValue::from_static("box-1.custom.ts.net"));
        headers.insert(
            "origin",
            HeaderValue::from_static("https://box-1.custom.ts.net"),
        );

        assert!(verify_origin_and_host(&headers, false, &allowed));

        // Non ts.net host rejected
        headers.insert("host", HeaderValue::from_static("box-1.other.net"));
        headers.insert(
            "origin",
            HeaderValue::from_static("https://box-1.other.net"),
        );
        assert!(!verify_origin_and_host(&headers, false, &allowed));
    }

    #[test]
    fn test_csrf_header_verification() {
        let mut headers = HeaderMap::new();
        // GET does not require CSRF
        assert!(verify_csrf_header(&Method::GET, &headers));

        // POST without CSRF header fails
        assert!(!verify_csrf_header(&Method::POST, &headers));

        // POST with X-Studio-Csrf passes
        headers.insert("x-studio-csrf", HeaderValue::from_static("1"));
        assert!(verify_csrf_header(&Method::POST, &headers));
    }
}
