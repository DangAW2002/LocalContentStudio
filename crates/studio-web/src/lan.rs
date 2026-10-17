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

/// Format the access URL for LAN users.
pub fn format_lan_url(port: u16) -> String {
    if let Some(ip) = detect_lan_ip() {
        format!("http://{ip}:{port}")
    } else {
        format!("http://127.0.0.1:{port}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_lan_url() {
        let url = format_lan_url(3000);
        assert!(url.starts_with("http://"));
        assert!(url.ends_with(":3000"));
    }
}
