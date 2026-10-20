//! TLS Certificate and Private Key management for Local Content Studio web mode.
//!
//! Generates self-signed X.509 certificates with Subject Alternative Names (SAN)
//! covering `localhost`, `127.0.0.1`, and the active LAN IP.
//! Computes SHA-256 fingerprint for verification on mobile devices.

use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, SanType};
use sha2::{Digest, Sha256};

use crate::lan::detect_lan_ip;

/// Container for TLS certificate and private key paths or contents, plus SHA-256 fingerprint.
#[derive(Debug, Clone)]
pub struct TlsCertificateInfo {
    /// Path to certificate PEM file.
    pub cert_path: PathBuf,
    /// Path to private key PEM file.
    pub key_path: PathBuf,
    /// Hex-encoded SHA-256 fingerprint with colon separators (e.g. `AA:BB:CC:...`).
    pub fingerprint_sha256: String,
    /// IP address or SANs included in the certificate.
    pub san_ips: Vec<String>,
}

/// Helper to format raw sha256 bytes into a colon-separated uppercase hex string.
pub fn format_fingerprint(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}

/// Calculate SHA-256 fingerprint of a DER certificate.
pub fn sha256_fingerprint_from_der(der_bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(der_bytes);
    format_fingerprint(&hasher.finalize())
}

/// Generate a new self-signed certificate and private key, saving them to `cert_dir`.
///
/// Sets SANs to:
/// - "localhost" (DNS)
/// - 127.0.0.1 (IP)
/// - `current_lan_ip` (if provided, or auto-detected)
pub fn generate_self_signed_cert(
    cert_dir: &Path,
    override_lan_ip: Option<IpAddr>,
) -> Result<TlsCertificateInfo, String> {
    fs::create_dir_all(cert_dir).map_err(|e| format!("Không tạo được thư mục TLS: {e}"))?;

    let lan_ip = override_lan_ip.or_else(detect_lan_ip);

    let mut subject_alt_names = vec![
        SanType::DnsName(
            "localhost"
                .try_into()
                .map_err(|e| format!("Invalid DNS name: {e}"))?,
        ),
        SanType::IpAddress(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
    ];

    let mut san_ip_strings = vec!["127.0.0.1".to_string(), "localhost".to_string()];

    if let Some(ip) = lan_ip {
        if !ip.is_loopback() {
            subject_alt_names.push(SanType::IpAddress(ip));
            san_ip_strings.push(ip.to_string());
        }
    }

    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "Local Content Studio");
    dn.push(DnType::OrganizationName, "Personal Studio Web");

    let mut params = CertificateParams::default();
    params.distinguished_name = dn;
    params.subject_alt_names = subject_alt_names;

    let key_pair = KeyPair::generate().map_err(|e| format!("Lỗi tạo TLS key pair: {e}"))?;
    let cert = params
        .self_signed(&key_pair)
        .map_err(|e| format!("Lỗi tự ký chứng chỉ TLS: {e}"))?;

    let cert_pem = cert.pem();
    let key_pem = key_pair.serialize_pem();
    let cert_der = cert.der();

    let cert_path = cert_dir.join("web_cert.pem");
    let key_path = cert_dir.join("web_key.pem");
    let meta_path = cert_dir.join("web_cert_meta.json");

    fs::write(&cert_path, cert_pem.as_bytes()).map_err(|e| format!("Lỗi ghi cert.pem: {e}"))?;
    fs::write(&key_path, key_pem.as_bytes()).map_err(|e| format!("Lỗi ghi key.pem: {e}"))?;

    // Set restrictive permissions on key file on Unix if applicable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600));
    }

    let fingerprint = sha256_fingerprint_from_der(cert_der);

    let meta = serde_json::json!({
        "fingerprint": fingerprint,
        "san_ips": san_ip_strings,
        "lan_ip": lan_ip.map(|ip| ip.to_string()),
    });
    let _ = fs::write(
        &meta_path,
        serde_json::to_string_pretty(&meta).unwrap_or_default(),
    );

    Ok(TlsCertificateInfo {
        cert_path,
        key_path,
        fingerprint_sha256: fingerprint,
        san_ips: san_ip_strings,
    })
}

/// Check if existing certificate in `cert_dir` is valid for the current LAN IP.
/// If not present or LAN IP changed, regenerates certificate.
pub fn ensure_tls_certificate(
    cert_dir: &Path,
    override_lan_ip: Option<IpAddr>,
) -> Result<TlsCertificateInfo, String> {
    let cert_path = cert_dir.join("web_cert.pem");
    let key_path = cert_dir.join("web_key.pem");
    let meta_path = cert_dir.join("web_cert_meta.json");

    let current_lan_ip = override_lan_ip.or_else(detect_lan_ip);
    let current_lan_ip_str = current_lan_ip.map(|ip| ip.to_string());

    if cert_path.exists() && key_path.exists() && meta_path.exists() {
        if let Ok(meta_bytes) = fs::read(&meta_path) {
            if let Ok(meta) = serde_json::from_slice::<serde_json::Value>(&meta_bytes) {
                let saved_lan_ip = meta.get("lan_ip").and_then(|v| v.as_str());
                let saved_fingerprint = meta.get("fingerprint").and_then(|v| v.as_str());
                let saved_sans: Vec<String> = meta
                    .get("san_ips")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                // If LAN IP matches (or both are None) and fingerprint exists
                let ip_matches = saved_lan_ip == current_lan_ip_str.as_deref();
                if let (true, Some(fp)) = (ip_matches, saved_fingerprint) {
                    return Ok(TlsCertificateInfo {
                        cert_path,
                        key_path,
                        fingerprint_sha256: fp.to_string(),
                        san_ips: saved_sans,
                    });
                }
            }
        }
    }

    // Otherwise generate or regenerate
    generate_self_signed_cert(cert_dir, current_lan_ip)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_generate_and_ensure_cert_san_and_fingerprint() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let lan_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));

        let cert_info = generate_self_signed_cert(temp_dir.path(), Some(lan_ip)).expect("generate");

        assert!(cert_info.cert_path.exists());
        assert!(cert_info.key_path.exists());
        assert!(!cert_info.fingerprint_sha256.is_empty());
        assert!(cert_info.fingerprint_sha256.contains(':'));
        assert!(cert_info.san_ips.contains(&"192.168.1.100".to_string()));
        assert!(cert_info.san_ips.contains(&"localhost".to_string()));
        assert!(cert_info.san_ips.contains(&"127.0.0.1".to_string()));

        // Ensure reuse when IP unchanged
        let reused = ensure_tls_certificate(temp_dir.path(), Some(lan_ip)).expect("reused");
        assert_eq!(reused.fingerprint_sha256, cert_info.fingerprint_sha256);

        // Regenerate when IP changed
        let new_lan_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 200));
        let regenerated = ensure_tls_certificate(temp_dir.path(), Some(new_lan_ip)).expect("regen");
        assert_ne!(regenerated.fingerprint_sha256, cert_info.fingerprint_sha256);
        assert!(regenerated.san_ips.contains(&"192.168.1.200".to_string()));
    }
}
