//! ureq-based synchronous network backend (default).
//!
//! Mirrors `dynamic_globe.rs:2148-2151`:
//! ```text
//! let agent = ureq::AgentBuilder::new()
//!     .user_agent("Mozilla/5.0 CesiumRust/0.1")
//!     .timeout(Duration::from_secs(10))
//!     .build();
//! ```
//!
//! The ureq agent maintains a connection pool with keep-alive, so tile-server
//! connections stay warm across fetches (L2140-2142 doc comment). This is the
//! **default and recommended** backend for the pipeline worker pool.

use std::io::Read;
use std::time::Duration;

use super::{FetchResult, NetworkBackend};

/// Blocking HTTP backend using ureq with keep-alive connection pooling.
///
/// Thread-safe: the ureq `Agent` is internally `Send + Sync` and manages
/// its own connection pool across threads.
pub struct UreqBackend {
    agent: ureq::Agent,
    timeout: Duration,
}

impl UreqBackend {
    /// Create a backend with default settings matching dynamic_globe.rs:
    /// - User-Agent: "Mozilla/5.0 CesiumRust/0.1"
    /// - Timeout: 10 seconds
    /// - Keep-alive: enabled (ureq default)
    pub fn new() -> Self {
        let timeout = Duration::from_secs(10);
        let agent = ureq::AgentBuilder::new()
            .user_agent("Mozilla/5.0 CesiumRust/0.1")
            .timeout(timeout)
            .build();
        Self { agent, timeout }
    }

    /// Create a backend with custom timeout.
    pub fn with_timeout(timeout: Duration) -> Self {
        let agent = ureq::AgentBuilder::new()
            .user_agent("Mozilla/5.0 CesiumRust/0.1")
            .timeout(timeout)
            .build();
        Self { agent, timeout }
    }
}

impl Default for UreqBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkBackend for UreqBackend {
    fn fetch(&self, url: &str) -> FetchResult {
        match self.agent.get(url).call() {
            Ok(resp) => {
                let status = resp.status();
                let mut reader = resp.into_reader();
                let mut data = Vec::new();
                match reader.read_to_end(&mut data) {
                    Ok(_) => FetchResult::Ok(data),
                    Err(e) => FetchResult::Transient(format!(
                        "read error (status {status}): {e}"
                    )),
                }
            }
            Err(ureq::Error::Status(code, _)) => {
                // 403/429 = throttling (transient, retry with backoff)
                // 404 = permanent (tile does not exist)
                if code == 404 {
                    FetchResult::Permanent(format!("HTTP 404: {url}"))
                } else {
                    FetchResult::Transient(format!("HTTP {code}: {url}"))
                }
            }
            Err(ureq::Error::Transport(e)) => {
                // Timeouts, connection resets, DNS failures — all transient
                FetchResult::Transient(format!("transport: {e}"))
            }
        }
    }

    fn name(&self) -> &str {
        "ureq"
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_name_is_ureq() {
        let b = UreqBackend::new();
        assert_eq!(b.name(), "ureq");
    }

    #[test]
    fn default_timeout_is_10s() {
        let b = UreqBackend::new();
        assert_eq!(b.timeout(), Duration::from_secs(10));
    }

    #[test]
    fn custom_timeout() {
        let b = UreqBackend::with_timeout(Duration::from_secs(5));
        assert_eq!(b.timeout(), Duration::from_secs(5));
    }

    /// Worker connection reuse (L2148-2151 keep-alive): a single ureq `Agent`
    /// must pool the TCP connection across sequential fetches, so the server
    /// observes fewer connections than requests.
    #[test]
    fn keepalive_reuses_connection() {
        use crate::net::test_server;
        use std::sync::atomic::Ordering;

        let server = test_server::spawn(b"tiledata".to_vec());
        let backend = UreqBackend::new();
        let url = server.url("/tile");

        for _ in 0..5 {
            match backend.fetch(&url) {
                FetchResult::Ok(bytes) => assert_eq!(bytes, b"tiledata"),
                other => panic!("expected Ok, got {other:?}"),
            }
        }

        let reqs = server.stats.requests.load(Ordering::SeqCst);
        let conns = server.stats.connections.load(Ordering::SeqCst);
        assert_eq!(reqs, 5, "all 5 requests must reach the server");
        assert!(
            conns < reqs,
            "keep-alive must reuse connections (conns={conns}, reqs={reqs})"
        );
    }
}
