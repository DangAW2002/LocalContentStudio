//! Local Area Network (LAN) discovery and URL formatting.

use std::net::{IpAddr, UdpSocket};

/// Detect the primary local LAN IPv4 address of the machine.
/// Connects a UDP socket to a public address (no packets actually sent) to query OS routing table.
pub fn detect_lan_ip() -> Option<IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    // Routing probe (does not send network packets)
    socket.connect("8.8.8.8:80").ok()?;
    let addr = socket.local_addr().ok()?.ip();
    if addr.is_loopback() || addr.is_unspecified() {
        None
    } else {
        Some(addr)
    }
}

/// Format the access URL for LAN users (defaults to https when TLS is enabled).
pub fn format_lan_url(port: u16, use_https: bool) -> String {
    let scheme = if use_https { "https" } else { "http" };
    if let Some(ip) = detect_lan_ip() {
        format!("{scheme}://{ip}:{port}")
    } else {
        format!("{scheme}://127.0.0.1:{port}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_lan_url() {
        let url_http = format_lan_url(3000, false);
        assert!(url_http.starts_with("http://"));
        assert!(url_http.ends_with(":3000"));

        let url_https = format_lan_url(3000, true);
        assert!(url_https.starts_with("https://"));
        assert!(url_https.ends_with(":3000"));
    }
}
