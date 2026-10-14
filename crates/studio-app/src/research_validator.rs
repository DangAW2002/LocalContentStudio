use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};
use studio_core::error::StudioError;
use url::Url;

pub struct SafeUrlValidator;

impl SafeUrlValidator {
    /// Validates that a URL is safe to fetch:
    /// 1. Scheme must be http or https
    /// 2. Must have a valid host
    /// 3. Must not resolve to loopback, private RFC1918, link-local, broadcast, or multicast addresses
    pub fn validate_url(url_str: &str) -> Result<Url, StudioError> {
        let url = Url::parse(url_str).map_err(|e| StudioError::InvalidInput {
            field: "url".into(),
            reason: format!("Malformed URL: {}", e),
            correlation_id: "research".into(),
        })?;

        let scheme = url.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(StudioError::InvalidInput {
                field: "url.scheme".into(),
                reason: format!(
                    "Only http and https schemes are supported, got: '{}'",
                    scheme
                ),
                correlation_id: "research".into(),
            });
        }

        // Prohibit user credentials in URL
        if !url.username().is_empty() || url.password().is_some() {
            return Err(StudioError::InvalidInput {
                field: "url.credentials".into(),
                reason: "URL must not contain embedded user credentials".into(),
                correlation_id: "research".into(),
            });
        }

        let host_str = url.host_str().ok_or_else(|| StudioError::InvalidInput {
            field: "url.host".into(),
            reason: "URL must contain a valid hostname".into(),
            correlation_id: "research".into(),
        })?;

        // Prohibit known local hostnames
        if host_str.eq_ignore_ascii_case("localhost")
            || host_str.ends_with(".localhost")
            || host_str.ends_with(".local")
        {
            return Err(StudioError::InvalidInput {
                field: "url.host".into(),
                reason: format!(
                    "SSRF protection: access to local host '{}' is blocked",
                    host_str
                ),
                correlation_id: "research".into(),
            });
        }

        // If host is an IP address directly or resolves to IPs
        if let Ok(ip) = host_str.parse::<IpAddr>() {
            Self::check_ip(ip, host_str)?;
        } else {
            // Check DNS resolution for private addresses (DNS rebinding / SSRF)
            let port = url.port_or_known_default().unwrap_or(80);
            let socket_addr = format!("{}:{}", host_str, port);
            if let Ok(addrs) = socket_addr.to_socket_addrs() {
                for addr in addrs {
                    Self::check_ip(addr.ip(), host_str)?;
                }
            }
        }

        Ok(url)
    }

    /// Fetches a URL safely with:
    /// 1. Pre-connection IP and scheme validation
    /// 2. Validated manual redirect hops (max 5)
    /// 3. Streamed byte cap (rejects oversized responses)
    /// 4. 10-second timeout
    pub async fn fetch_url_safe_async(
        url_str: &str,
        max_bytes: usize,
    ) -> Result<String, StudioError> {
        let mut current_url_str = url_str.to_string();
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| StudioError::Internal {
                message: format!("Failed to build HTTP client: {}", e),
                correlation_id: "research".into(),
            })?;

        let mut hops = 0;
        loop {
            if hops > 5 {
                return Err(StudioError::InvalidInput {
                    field: "url.redirects".into(),
                    reason: "Quá nhiều redirect (vượt quá 5 lần chuyển hướng)".into(),
                    correlation_id: "research".into(),
                });
            }

            // Invariant: MUST validate URL and IP before network request!
            let validated = Self::validate_url(&current_url_str)?;

            let resp =
                client
                    .get(validated.as_str())
                    .send()
                    .await
                    .map_err(|e| StudioError::Internal {
                        message: format!("HTTP request error: {}", e),
                        correlation_id: "research".into(),
                    })?;

            let status = resp.status();
            if status.is_redirection() {
                if let Some(loc) = resp.headers().get(reqwest::header::LOCATION) {
                    let loc_str = loc.to_str().map_err(|_| StudioError::InvalidInput {
                        field: "url.redirect_location".into(),
                        reason: "Header Location chứa ký tự không hợp lệ".into(),
                        correlation_id: "research".into(),
                    })?;
                    let next_url =
                        validated
                            .join(loc_str)
                            .map_err(|e| StudioError::InvalidInput {
                                field: "url.redirect_location".into(),
                                reason: format!("Không thể phân giải URL redirect: {}", e),
                                correlation_id: "research".into(),
                            })?;
                    current_url_str = next_url.to_string();
                    hops += 1;
                    continue;
                } else {
                    return Err(StudioError::InvalidInput {
                        field: "url.redirect".into(),
                        reason: "Redirect response thiếu header Location".into(),
                        correlation_id: "research".into(),
                    });
                }
            }

            if !status.is_success() {
                return Err(StudioError::PreconditionFailed {
                    reason: format!("Máy chủ đích trả về mã lỗi HTTP {}", status),
                    correlation_id: "research".into(),
                });
            }

            let mut stream = resp;
            let mut accumulated = Vec::new();
            while let Some(chunk) = stream.chunk().await.map_err(|e| StudioError::Internal {
                message: format!("Lỗi đọc dữ liệu luồng: {}", e),
                correlation_id: "research".into(),
            })? {
                accumulated.extend_from_slice(&chunk);
                if accumulated.len() > max_bytes {
                    return Err(StudioError::QuotaExceeded {
                        reason: format!(
                            "Dung lượng trang web vượt quá giới hạn an toàn ({} bytes)",
                            max_bytes
                        ),
                        correlation_id: "research".into(),
                    });
                }
            }

            let text = String::from_utf8(accumulated).map_err(|e| StudioError::InvalidInput {
                field: "response.encoding".into(),
                reason: format!("Nội dung trang web không phải UTF-8 hợp lệ: {}", e),
                correlation_id: "research".into(),
            })?;

            return Ok(text);
        }
    }

    pub fn fetch_url_safe(url_str: &str, max_bytes: usize) -> Result<String, StudioError> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| StudioError::Internal {
                message: format!("Runtime error: {}", e),
                correlation_id: "research".into(),
            })?;
        rt.block_on(Self::fetch_url_safe_async(url_str, max_bytes))
    }

    fn check_ip(ip: IpAddr, host_str: &str) -> Result<(), StudioError> {
        let is_private = match ip {
            IpAddr::V4(ipv4) => {
                ipv4.is_loopback() // 127.0.0.0/8
                    || ipv4.is_private() // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
                    || ipv4.is_link_local() // 169.254.0.0/16
                    || ipv4.is_broadcast() // 255.255.255.255
                    || ipv4 == Ipv4Addr::new(0, 0, 0, 0)
            }
            IpAddr::V6(ipv6) => {
                ipv6.is_loopback() // ::1
                    || ipv6 == Ipv6Addr::UNSPECIFIED // ::
                    // IPv4-mapped IPv6 check
                    || if let Some(ipv4) = ipv6.to_ipv4_mapped() {
                        ipv4.is_loopback() || ipv4.is_private() || ipv4.is_link_local()
                    } else {
                        false
                    }
            }
        };

        if is_private {
            return Err(StudioError::InvalidInput {
                field: "url.host".into(),
                reason: format!(
                    "SSRF protection: address '{}' resolved to private/local IP {}",
                    host_str, ip
                ),
                correlation_id: "research".into(),
            });
        }

        Ok(())
    }
}
