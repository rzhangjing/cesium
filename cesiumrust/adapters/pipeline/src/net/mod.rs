//! Network backend abstraction + implementations.
//!
//! The pipeline uses a **synchronous blocking pool** (ureq, 16 workers with
//! keep-alive) as the default network path, matching `dynamic_globe.rs:2148-2151`:
//!
//! ```text
//! let agent = ureq::AgentBuilder::new()
//!     .user_agent("Mozilla/5.0 CesiumRust/0.1")
//!     .timeout(Duration::from_secs(10))
//!     .build();
//! ```
//!
//! An optional `reqwest`-based async backend is available behind the
//! `reqwest-backend` feature flag for future integration (M1.4+), but
//! **tokio is NOT the pipeline main runtime** — it is at most a transitive
//! dependency of the optional reqwest backend.

pub mod ureq_backend;

#[cfg(feature = "reqwest-backend")]
pub mod reqwest_backend;

use std::time::Duration;

/// Result of a network fetch attempt.
#[derive(Debug, Clone)]
pub enum FetchResult {
    /// Successful fetch with raw bytes.
    Ok(Vec<u8>),
    /// Transient failure (timeout, 403, 429, network error).
    /// The worker will retry according to `RetryPolicy`.
    Transient(String),
    /// Permanent failure (404, invalid URL). No retry.
    Permanent(String),
}

/// Trait for network backends used by the worker pool.
///
/// Implementations must be `Send + Sync` (shared across worker threads).
/// The default implementation is `UreqBackend` (blocking, keep-alive pool).
///
/// Corresponds to the ureq agent in `dynamic_globe.rs:2148-2151` and the
/// fetch loop at L2192-2203.
pub trait NetworkBackend: Send + Sync {
    /// Fetch bytes from a URL. Blocks until complete or timeout.
    ///
    /// This is called from worker threads (not the frame thread), so blocking
    /// is acceptable and expected.
    fn fetch(&self, url: &str) -> FetchResult;

    /// Backend name for diagnostics.
    fn name(&self) -> &str;

    /// Connection timeout (L2150: 10 s).
    fn timeout(&self) -> Duration;
}

/// Minimal keep-alive HTTP server used by backend tests (offline, ephemeral
/// port). Tracks the number of accepted TCP connections vs served requests so
/// tests can assert connection reuse (keep-alive pooling, L2148-2151).
#[cfg(test)]
pub(crate) mod test_server {
    use std::io::{BufRead, BufReader, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    /// Counters observed by tests.
    pub(crate) struct ServerStats {
        /// TCP connections accepted (keep-alive ⇒ fewer than requests).
        pub(crate) connections: AtomicUsize,
        /// HTTP requests served.
        pub(crate) requests: AtomicUsize,
    }

    /// Handle to a running test server.
    pub(crate) struct TestServer {
        base: String,
        /// Shared stats.
        pub(crate) stats: Arc<ServerStats>,
    }

    impl TestServer {
        /// Build a full URL for `path` (e.g. `/tile`).
        pub(crate) fn url(&self, path: &str) -> String {
            format!("{}{}", self.base, path)
        }
    }

    /// Spawn a server returning `body` (HTTP 200, keep-alive) for every GET.
    pub(crate) fn spawn(body: Vec<u8>) -> TestServer {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().expect("local_addr").port();
        let stats = Arc::new(ServerStats {
            connections: AtomicUsize::new(0),
            requests: AtomicUsize::new(0),
        });
        let shared = Arc::clone(&stats);
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { break };
                shared.connections.fetch_add(1, Ordering::SeqCst);
                let b = body.clone();
                let s = Arc::clone(&shared);
                thread::spawn(move || handle_conn(stream, &b, &s));
            }
        });
        TestServer {
            base: format!("http://127.0.0.1:{port}"),
            stats,
        }
    }

    fn handle_conn(mut stream: TcpStream, body: &[u8], stats: &ServerStats) {
        let reader_stream = match stream.try_clone() {
            Ok(s) => s,
            Err(_) => return,
        };
        let mut reader = BufReader::new(reader_stream);
        loop {
            // Read the request line; EOF (0 bytes) means the client closed.
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => return,
                Ok(_) => {}
                Err(_) => return,
            }
            // Drain remaining headers until the blank line terminator.
            loop {
                let mut h = String::new();
                match reader.read_line(&mut h) {
                    Ok(0) => return,
                    Ok(_) => {}
                    Err(_) => return,
                }
                if h == "\r\n" || h == "\n" || h.is_empty() {
                    break;
                }
            }
            stats.requests.fetch_add(1, Ordering::SeqCst);
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n",
                body.len()
            );
            if stream.write_all(head.as_bytes()).is_err() {
                return;
            }
            if stream.write_all(body).is_err() {
                return;
            }
            if stream.flush().is_err() {
                return;
            }
        }
    }
}
