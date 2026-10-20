//! OAuth loopback redirect receiver shared by the YouTube and Facebook connectors.

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Why no OAuth redirect was received.
#[derive(Debug)]
pub enum LoopbackError {
    /// The browser did not come back before the deadline.
    Timeout,
    /// The listener failed.
    Io(std::io::Error),
}

/// Waits for the browser's OAuth redirect (a request whose query has `code=` or `error=`).
///
/// Browsers often open speculative connections first, or ask for `/favicon.ico`; those get a 404
/// and the wait goes on, so only the real redirect is returned — with its stream, to answer it,
/// and the raw request text.
pub async fn accept_oauth_redirect(
    listener: &TcpListener,
    timeout: Duration,
) -> Result<(TcpStream, String), LoopbackError> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let (mut stream, _) = tokio::time::timeout_at(deadline, listener.accept())
            .await
            .map_err(|_| LoopbackError::Timeout)?
            .map_err(LoopbackError::Io)?;
        let mut buf = [0u8; 8192];
        // A preconnect sends nothing: do not let it hold the wait.
        let n = match tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buf)).await {
            Ok(Ok(n)) => n,
            _ => 0,
        };
        let request = String::from_utf8_lossy(&buf[..n]).to_string();
        if is_oauth_redirect(&request) {
            return Ok((stream, request));
        }
        if n > 0 {
            let _ = stream
                .write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await;
        }
    }
}

fn is_oauth_redirect(request: &str) -> bool {
    let first_line = request.lines().next().unwrap_or_default();
    let target = first_line.split_whitespace().nth(1).unwrap_or("");
    let query = target.split_once('?').map(|(_, q)| q).unwrap_or("");
    query
        .split('&')
        .any(|p| p.starts_with("code=") || p.starts_with("error="))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_only_redirects() {
        assert!(is_oauth_redirect("GET /?code=abc&state=s HTTP/1.1\r\n"));
        assert!(is_oauth_redirect(
            "GET /callback?state=s&code=abc HTTP/1.1\r\n"
        ));
        assert!(is_oauth_redirect(
            "GET /callback?error=access_denied HTTP/1.1\r\n"
        ));
        assert!(!is_oauth_redirect("GET /favicon.ico HTTP/1.1\r\n"));
        assert!(!is_oauth_redirect(""));
        assert!(!is_oauth_redirect("GET /?xcode=1 HTTP/1.1\r\n"));
    }

    #[tokio::test]
    async fn skips_preconnect_and_favicon_then_returns_the_redirect() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let client = tokio::spawn(async move {
            // Speculative connection that never sends a request.
            let idle = TcpStream::connect(addr).await.unwrap();
            let mut fav = TcpStream::connect(addr).await.unwrap();
            fav.write_all(b"GET /favicon.ico HTTP/1.1\r\n\r\n")
                .await
                .unwrap();
            let mut cb = TcpStream::connect(addr).await.unwrap();
            cb.write_all(b"GET /callback?code=xyz&state=s1 HTTP/1.1\r\n\r\n")
                .await
                .unwrap();
            (idle, fav, cb)
        });
        let (_stream, request) = accept_oauth_redirect(&listener, Duration::from_secs(20))
            .await
            .unwrap();
        assert!(request.starts_with("GET /callback?code=xyz"));
        drop(client.await.unwrap());
    }

    #[tokio::test]
    async fn times_out_without_a_redirect() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let res = accept_oauth_redirect(&listener, Duration::from_millis(200)).await;
        assert!(matches!(res, Err(LoopbackError::Timeout)));
    }
}
