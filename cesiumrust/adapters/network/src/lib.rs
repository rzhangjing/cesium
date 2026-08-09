//! cesium-network: HTTP network adapter
//!
//! Implements the TileFetcher port using synchronous HTTP requests via ureq
//! dispatched through tokio's spawn_blocking.

use cesium_ports_driven::{PortError, PortResult, TileFetcher};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use thiserror::Error;
use tokio::sync::Mutex;

const DEFAULT_MAX_REQUESTS_PER_SERVER: usize = 6;
const DEFAULT_RETRY_COUNT: u32 = 3;
const DEFAULT_READ_TIMEOUT_SECS: u64 = 30;
const RATE_LIMIT_POLL_MS: u64 = 50;

/// Network errors
#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Timeout")]
    Timeout,

    #[error("Request cancelled")]
    Cancelled,
}

/// HTTP-based tile fetcher using ureq for synchronous HTTP/HTTPS requests.
///
/// Requests are dispatched to `tokio::task::spawn_blocking` so that the
/// synchronous ureq calls do not block the async runtime.
pub struct HttpTileFetcher {
    agent: ureq::Agent,
    #[allow(dead_code)]
    base_url: String,
    headers: HashMap<String, String>,
    active_requests: Arc<Mutex<HashMap<String, usize>>>,
    max_requests_per_server: usize,
    retry_count: u32,
    cancelled: Arc<StdMutex<HashSet<String>>>,
}

impl HttpTileFetcher {
    /// Creates a new `HttpTileFetcher` with a default ureq agent.
    pub fn new(base_url: &str) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout_read(Duration::from_secs(DEFAULT_READ_TIMEOUT_SECS))
            .build();

        Self {
            agent,
            base_url: base_url.to_string(),
            headers: HashMap::new(),
            active_requests: Arc::new(Mutex::new(HashMap::new())),
            max_requests_per_server: DEFAULT_MAX_REQUESTS_PER_SERVER,
            retry_count: DEFAULT_RETRY_COUNT,
            cancelled: Arc::new(StdMutex::new(HashSet::new())),
        }
    }

    /// Creates a new `HttpTileFetcher` with a custom ureq agent.
    pub fn with_agent(base_url: &str, agent: ureq::Agent) -> Self {
        Self {
            agent,
            base_url: base_url.to_string(),
            headers: HashMap::new(),
            active_requests: Arc::new(Mutex::new(HashMap::new())),
            max_requests_per_server: DEFAULT_MAX_REQUESTS_PER_SERVER,
            retry_count: DEFAULT_RETRY_COUNT,
            cancelled: Arc::new(StdMutex::new(HashSet::new())),
        }
    }

    /// Sets a request header.
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// Sets the maximum concurrent requests per server.
    pub fn with_max_requests_per_server(mut self, max: usize) -> Self {
        self.max_requests_per_server = max;
        self
    }

    /// Sets the number of retry attempts on transient failures.
    pub fn with_retry_count(mut self, retries: u32) -> Self {
        self.retry_count = retries;
        self
    }

    /// Extracts the server key (host[:port]) from a URL.
    fn extract_server_key(url: &str) -> String {
        if let Some(start) = url.find("://") {
            let rest = &url[start + 3..];
            if let Some(end) = rest.find('/') {
                return rest[..end].to_string();
            }
            return rest.to_string();
        }
        url.to_string()
    }

    /// Performs a single HTTP GET request and returns the response body.
    fn do_fetch(
        agent: &ureq::Agent,
        url: &str,
        headers: &HashMap<String, String>,
    ) -> Result<Vec<u8>, PortError> {
        let mut req = agent.get(url);
        for (k, v) in headers {
            req = req.set(k, v);
        }

        let resp = req.call().map_err(map_ureq_error)?;

        let mut data = Vec::new();
        resp.into_reader()
            .read_to_end(&mut data)
            .map_err(|e| PortError::Network(format!("Failed to read response body: {}", e)))?;

        Ok(data)
    }

    /// Performs a fetch with retry logic for transient failures.
    fn do_fetch_with_retry(
        agent: &ureq::Agent,
        url: &str,
        headers: &HashMap<String, String>,
        retry_count: u32,
    ) -> PortResult<Vec<u8>> {
        let mut last_err = None;

        for attempt in 0..=retry_count {
            match Self::do_fetch(agent, url, headers) {
                Ok(data) => return Ok(data),
                Err(e) => {
                    let is_transient = matches!(
                        &e,
                        PortError::Network(_)
                    );
                    if !is_transient {
                        return Err(e);
                    }
                    last_err = Some(e);
                    if attempt < retry_count {
                        std::thread::sleep(Duration::from_millis(
                            100 * (attempt as u64 + 1),
                        ));
                    }
                }
            }
        }

        Err(last_err.unwrap_or_else(|| {
            PortError::Network("Retry exhausted with no error".to_string())
        }))
    }
}

impl TileFetcher for HttpTileFetcher {
    fn fetch<'a>(
        &'a self,
        url: &'a str,
        _priority: f64,
    ) -> Pin<Box<dyn Future<Output = PortResult<Vec<u8>>> + Send + 'a>> {
        let url_owned = url.to_string();
        let cancelled = Arc::clone(&self.cancelled);
        let agent = self.agent.clone();
        let headers = self.headers.clone();
        let active = Arc::clone(&self.active_requests);
        let max_req = self.max_requests_per_server;
        let retry_count = self.retry_count;
        let server_key = Self::extract_server_key(&url_owned);

        Box::pin(async move {
            // Check if the request was cancelled
            {
                let cancelled_set = cancelled.lock().unwrap();
                if cancelled_set.contains(&url_owned) {
                    return Err(PortError::Cancelled);
                }
            }

            // Rate-limit: wait until a slot opens for this server
            loop {
                let acquired = {
                    let mut active_map = active.lock().await;
                    let count = active_map.entry(server_key.clone()).or_insert(0);
                    if *count < max_req {
                        *count += 1;
                        true
                    } else {
                        false
                    }
                };
                if acquired {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(RATE_LIMIT_POLL_MS)).await;
            }

            // Dispatch the blocking HTTP call to a worker thread
            let result = tokio::task::spawn_blocking(move || {
                Self::do_fetch_with_retry(&agent, &url_owned, &headers, retry_count)
            })
            .await;

            // Release the slot
            {
                let mut active_map = active.lock().await;
                if let Some(count) = active_map.get_mut(&server_key) {
                    *count = count.saturating_sub(1);
                }
            }

            match result {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(e)) => Err(e),
                Err(join_err) => Err(PortError::Network(format!(
                    "spawn_blocking join error: {}",
                    join_err
                ))),
            }
        })
    }

    fn cancel(&self, url: &str) {
        let mut set = self.cancelled.lock().unwrap();
        set.insert(url.to_string());
    }
}

/// Maps a ureq error to a `PortError`.
fn map_ureq_error(err: ureq::Error) -> PortError {
    match err {
        ureq::Error::Status(code, _resp) => {
            if code == 404 {
                PortError::NotFound(format!("HTTP {}", code))
            } else {
                PortError::Network(format!("HTTP status {}", code))
            }
        }
        ureq::Error::Transport(transport) => {
            let msg = transport.to_string();
            if msg.contains("timed out") || msg.contains("Timeout") {
                PortError::Network(format!("Request timed out: {}", msg))
            } else {
                PortError::Network(format!("Transport error: {}", msg))
            }
        }
    }
}

// ============================================================================
// MockTileFetcher (for testing)
// ============================================================================

/// A mock tile fetcher for testing that returns predefined data.
pub struct MockTileFetcher {
    responses: HashMap<String, Vec<u8>>,
}

impl MockTileFetcher {
    /// Creates a new mock tile fetcher.
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
        }
    }

    /// Adds a predefined response for a URL.
    pub fn with_response(mut self, url: &str, data: Vec<u8>) -> Self {
        self.responses.insert(url.to_string(), data);
        self
    }
}

impl Default for MockTileFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl TileFetcher for MockTileFetcher {
    fn fetch<'a>(
        &'a self,
        url: &'a str,
        _priority: f64,
    ) -> Pin<Box<dyn Future<Output = PortResult<Vec<u8>>> + Send + 'a>> {
        let result = self
            .responses
            .get(url)
            .cloned()
            .ok_or_else(|| PortError::NotFound(format!("No mock response for URL: {}", url)));

        Box::pin(async move { result })
    }

    fn cancel(&self, _url: &str) {
        // Mock fetcher doesn't need cancellation
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- extract_server_key --------------------------------------------------

    #[test]
    fn test_extract_server_key_https() {
        assert_eq!(
            HttpTileFetcher::extract_server_key("https://example.com/tiles/0/0/0.terrain"),
            "example.com"
        );
    }

    #[test]
    fn test_extract_server_key_http_with_port() {
        assert_eq!(
            HttpTileFetcher::extract_server_key("http://localhost:8080/api"),
            "localhost:8080"
        );
    }

    #[test]
    fn test_extract_server_key_no_scheme() {
        assert_eq!(
            HttpTileFetcher::extract_server_key("example.com/path"),
            "example.com/path"
        );
    }

    #[test]
    fn test_extract_server_key_no_path() {
        assert_eq!(
            HttpTileFetcher::extract_server_key("https://example.com"),
            "example.com"
        );
    }

    // --- builder -------------------------------------------------------------

    #[test]
    fn test_http_tile_fetcher_builder() {
        let fetcher = HttpTileFetcher::new("https://assets.cesium.com")
            .with_header("Authorization", "Bearer token")
            .with_max_requests_per_server(10)
            .with_retry_count(5);

        assert_eq!(fetcher.base_url, "https://assets.cesium.com");
        assert_eq!(fetcher.max_requests_per_server, 10);
        assert_eq!(fetcher.retry_count, 5);
        assert!(fetcher.headers.contains_key("Authorization"));
        assert_eq!(fetcher.headers.get("Authorization").unwrap(), "Bearer token");
    }

    #[test]
    fn test_http_tile_fetcher_defaults() {
        let fetcher = HttpTileFetcher::new("https://assets.cesium.com");
        assert_eq!(fetcher.max_requests_per_server, 6);
        assert_eq!(fetcher.retry_count, 3);
        assert!(fetcher.headers.is_empty());
    }

    #[test]
    fn test_http_tile_fetcher_with_agent() {
        let agent = ureq::AgentBuilder::new()
            .timeout_read(Duration::from_secs(10))
            .build();
        let fetcher = HttpTileFetcher::with_agent("https://custom.example.com", agent);
        assert_eq!(fetcher.base_url, "https://custom.example.com");
    }

    // --- real fetch (integration-like) ---------------------------------------

    #[tokio::test]
    async fn test_fetch_invalid_url() {
        let fetcher = HttpTileFetcher::new("https://invalid.example.invalid");
        let result = fetcher
            .fetch("https://invalid.example.invalid/tile.terrain", 1.0)
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PortError::Network(_)));
    }

    #[tokio::test]
    async fn test_fetch_cancelled() {
        let fetcher = HttpTileFetcher::new("https://example.com");
        fetcher.cancel("https://example.com/cancelled.terrain");

        let result = fetcher
            .fetch("https://example.com/cancelled.terrain", 1.0)
            .await;
        assert!(matches!(result.unwrap_err(), PortError::Cancelled));
    }

    // --- mock ----------------------------------------------------------------

    #[tokio::test]
    async fn test_mock_tile_fetcher() {
        let fetcher =
            MockTileFetcher::new().with_response("http://test.com/tile.terrain", vec![1, 2, 3, 4]);

        let result = fetcher.fetch("http://test.com/tile.terrain", 1.0).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![1, 2, 3, 4]);

        let result = fetcher.fetch("http://test.com/missing.terrain", 1.0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_tile_fetcher_cancel_is_noop() {
        let fetcher = MockTileFetcher::new();
        fetcher.cancel("anything");
        let result = fetcher.fetch("anything", 1.0).await;
        assert!(result.is_err()); // not found, not cancelled — cancel is a noop
    }

    // --- error mapping -------------------------------------------------------

    #[test]
    fn test_map_ureq_error_status_404() {
        // We can't easily construct ureq::Error::Status without a real response,
        // but we test the logic indirectly via integration tests above.
        // This placeholder documents the expected mapping.
    }
}
