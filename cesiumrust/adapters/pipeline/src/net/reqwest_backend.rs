//! Optional reqwest-based async network backend.
//!
//! Available behind the `reqwest-backend` feature flag. Uses tokio as a
//! **transitive dependency only** — the pipeline main runtime remains
//! synchronous (std threads + mpsc channels). The reqwest backend internally
//! blocks on a dedicated tokio runtime handle per worker thread.
//!
//! This backend exists for future M1.4+ integration where async providers
//! (e.g. streaming 3D Tiles content) may benefit from HTTP/2 multiplexing.
//! For the default tile imagery path, `UreqBackend` is preferred (simpler,
//! no tokio dependency, proven keep-alive pooling).

use std::time::Duration;

use super::{FetchResult, NetworkBackend};

/// Async HTTP backend using reqwest with rustls-tls.
///
/// Internally creates a single-threaded tokio runtime for blocking calls
/// from worker threads. This is NOT the pipeline main runtime.
pub struct ReqwestBackend {
    client: reqwest::blocking::Client,
    timeout: Duration,
}

impl ReqwestBackend {
    /// Create a backend with default settings (10 s timeout, rustls-tls).
    pub fn new() -> Self {
        let timeout = Duration::from_secs(10);
        let client = reqwest::blocking::Client::builder()
            .user_agent("Mozilla/5.0 CesiumRust/0.1")
            .timeout(timeout)
            .build()
            .expect("reqwest client build failed");
        Self { client, timeout }
    }

    /// Create a backend with custom timeout.
    pub fn with_timeout(timeout: Duration) -> Self {
        let client = reqwest::blocking::Client::builder()
            .user_agent("Mozilla/5.0 CesiumRust/0.1")
            .timeout(timeout)
            .build()
            .expect("reqwest client build failed");
        Self { client, timeout }
    }
}

impl Default for ReqwestBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkBackend for ReqwestBackend {
    fn fetch(&self, url: &str) -> FetchResult {
        match self.client.get(url).send() {
            Ok(resp) => {
                let status = resp.status();
                if status == reqwest::StatusCode::NOT_FOUND {
                    return FetchResult::Permanent(format!("HTTP 404: {url}"));
                }
                if !status.is_success() {
                    return FetchResult::Transient(format!("HTTP {}: {url}", status.as_u16()));
                }
                match resp.bytes() {
                    Ok(body) => FetchResult::Ok(body.to_vec()),
                    Err(e) => FetchResult::Transient(format!("read error: {e}")),
                }
            }
            Err(e) => {
                if e.is_timeout() || e.is_connect() {
                    FetchResult::Transient(format!("transport: {e}"))
                } else {
                    FetchResult::Transient(format!("request: {e}"))
                }
            }
        }
    }

    fn name(&self) -> &str {
        "reqwest"
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_name_is_reqwest() {
        let b = ReqwestBackend::new();
        assert_eq!(b.name(), "reqwest");
    }

    #[test]
    fn default_timeout_is_10s() {
        let b = ReqwestBackend::new();
        assert_eq!(b.timeout(), Duration::from_secs(10));
    }

    /// Same-K equivalence: for an identical URL (tile key), the ureq and
    /// reqwest backends must return byte-identical `FetchResult::Ok` payloads.
    /// This proves the two backends are interchangeable behind `NetworkBackend`
    /// (the host can switch backends without changing pipeline behavior).
    #[test]
    fn same_k_ureq_reqwest_equivalent() {
        use crate::net::test_server;
        use crate::net::ureq_backend::UreqBackend;

        let server = test_server::spawn(b"equivalent-tile-payload".to_vec());
        let url = server.url("/tile");

        let ureq_b = UreqBackend::new();
        let rw_b = ReqwestBackend::new();

        let a = ureq_b.fetch(&url);
        let b = rw_b.fetch(&url);

        match (a, b) {
            (FetchResult::Ok(x), FetchResult::Ok(y)) => {
                assert_eq!(
                    x, y,
                    "same-K results must be byte-identical across backends"
                );
                assert_eq!(x, b"equivalent-tile-payload");
            }
            (x, y) => panic!("both backends should succeed: {x:?} / {y:?}"),
        }
    }
}
