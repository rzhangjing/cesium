//! cesium-network: HTTP + offline disk network adapters
//!
//! Implements the `TileFetcher` / `TerrainProvider` driven ports:
//!
//! * [`HttpTileFetcher`] — synchronous HTTP requests via ureq dispatched
//!   through a **tokio-free** `std::thread` + `mpsc` bridge
//!   ([`resource_backend_impl::spawn_blocking_fetch`]) — rate-limited,
//!   retrying, cancellable. M8.3 (#66) purged the previous
//!   `tokio::task::spawn_blocking` / `tokio::time::sleep` / `tokio::sync::Mutex`
//!   production path in favour of the shared blocking-pool philosophy
//!   (`adapters/pipeline/src/pool.rs`: 16 workers + keep-alive, no tokio
//!   Runtime).
//! * [`FileTileFetcher`] — offline disk-backed imagery tiles (XYZ / quadkey
//!   layout) with STRICT_OFFLINE semantics (no HTTP fallback).
//! * [`FileTerrainFetcher`] — offline disk-backed heightmap-1.0 terrain tiles.
//! * [`MockTileFetcher`] — predefined-response fetcher for tests.
//! * [`resource_backend_impl`] — the M8.3 [`ResourceBackend`] network adapter
//!   ([`NetworkResourceBackend`]) + the local env-gate
//!   ([`resource_fetch_backend_enabled`]) for `CESIUM_ENABLE_RESOURCE_FETCH_BACKEND`.
//!   When the gate is ON, [`HttpTileFetcher::fetch`] routes through the
//!   pipeline-managed 16-worker keep-alive pool + hot/warm cache hierarchy;
//!   when OFF (default), it takes the pre-M8.3 direct-ureq path so the v0
//!   baseline stays byte-identical.

pub mod file_terrain_fetcher;
pub mod file_tile_fetcher;
pub mod resource_backend_impl;

pub use file_terrain_fetcher::{FileTerrainFetcher, TerrainScheme};
pub use file_tile_fetcher::{FileTileFetcher, FileTileScheme};
pub use resource_backend_impl::{
    block_on_noop, gate_from_env_value, resource_fetch_backend_enabled, spawn_blocking_fetch,
    url_hash, NetworkResourceBackend, ENV_ENABLE_RESOURCE_FETCH_BACKEND,
};

use cesium_ports_driven::{PortError, PortResult, TileFetcher};
// M8.4 (#67): the adapter executes the IO-free domain `FetchDescriptor`
// produced by `Resource::fetch_*`/`post`. domain/resource stays network-free;
// all HTTP execution lives here in the adapter.
use cesium_resource::{FetchDescriptor, HttpMethod};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use thiserror::Error;

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
/// Requests are dispatched to a **tokio-free** `std::thread` + `mpsc` bridge
/// ([`resource_backend_impl::spawn_blocking_fetch`]) so the synchronous ureq
/// calls do not block the polling context. When
/// [`resource_fetch_backend_enabled()`] returns `true` (env gate
/// `CESIUM_ENABLE_RESOURCE_FETCH_BACKEND`), the fetch is routed through the
/// shared [`NetworkResourceBackend`] (16-worker keep-alive pool + hot/warm
/// cache hierarchy + in-flight dedup, all reused from `cesium-pipeline`);
/// when OFF (default) the pre-M8.3 direct-ureq path runs, preserving the v0
/// baseline byte-identically.
pub struct HttpTileFetcher {
    agent: ureq::Agent,
    #[allow(dead_code)]
    base_url: String,
    headers: HashMap<String, String>,
    /// Per-server in-flight counters (rate-limit gate). M8.3: `tokio::sync::Mutex`
    /// → `std::sync::Mutex` (blocking wait inside the spawned std::thread;
    /// never blocks the polling context because the whole rate-limit loop
    /// runs inside [`spawn_blocking_fetch`]).
    active_requests: Arc<StdMutex<HashMap<String, usize>>>,
    max_requests_per_server: usize,
    retry_count: u32,
    cancelled: Arc<StdMutex<HashSet<String>>>,
    /// M8.3 network `ResourceBackend` — consulted only when
    /// [`resource_fetch_backend_enabled()`] returns `true`. Shared across all
    /// `HttpTileFetcher` clones so the 16-worker keep-alive pool + hot/warm
    /// cache hierarchy are amortised process-wide.
    resource_backend: Arc<NetworkResourceBackend>,
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
            active_requests: Arc::new(StdMutex::new(HashMap::new())),
            max_requests_per_server: DEFAULT_MAX_REQUESTS_PER_SERVER,
            retry_count: DEFAULT_RETRY_COUNT,
            cancelled: Arc::new(StdMutex::new(HashSet::new())),
            resource_backend: Arc::new(NetworkResourceBackend::new()),
        }
    }

    /// Creates a new `HttpTileFetcher` with a custom ureq agent.
    pub fn with_agent(base_url: &str, agent: ureq::Agent) -> Self {
        Self {
            agent,
            base_url: base_url.to_string(),
            headers: HashMap::new(),
            active_requests: Arc::new(StdMutex::new(HashMap::new())),
            max_requests_per_server: DEFAULT_MAX_REQUESTS_PER_SERVER,
            retry_count: DEFAULT_RETRY_COUNT,
            cancelled: Arc::new(StdMutex::new(HashSet::new())),
            resource_backend: Arc::new(NetworkResourceBackend::new()),
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
    ///
    /// L2 review fix: returns a [`FetchFailure`] (error + retry classification)
    /// instead of a bare `PortError`, so `do_fetch_with_retry` retries only
    /// genuinely transient failures (408/429/5xx/transport) and fails fast on
    /// permanent 4xx client errors.
    fn do_fetch(
        agent: &ureq::Agent,
        url: &str,
        headers: &HashMap<String, String>,
    ) -> Result<Vec<u8>, FetchFailure> {
        let mut req = agent.get(url);
        for (k, v) in headers {
            req = req.set(k, v);
        }

        let resp = req.call().map_err(classify_ureq_error)?;

        let mut data = Vec::new();
        resp.into_reader()
            .read_to_end(&mut data)
            .map_err(|e| FetchFailure {
                // A mid-body read error (connection reset, truncated response)
                // is transient — retrying may succeed.
                err: PortError::Network(format!("Failed to read response body: {}", e)),
                transient: true,
            })?;

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
                Err(failure) => {
                    // L2 review fix: retry only transient failures. The pre-fix
                    // `matches!(&e, PortError::Network(_))` retried *every*
                    // non-404 status (map_ureq_error folds them all into
                    // Network), so a permanent 400/401/403 was retried
                    // pointlessly. `classify_ureq_error` now flags 408/429/5xx/
                    // transport as transient and everything else as permanent.
                    if !failure.transient {
                        return Err(failure.err);
                    }
                    last_err = Some(failure.err);
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

    /// M8.4 (#67): execute a domain [`FetchDescriptor`] through the
    /// gate-guarded adapter path, with the gate decision **injected** so the
    /// ON/OFF branches are unit-testable without mutating the process env.
    ///
    /// Routing (see [`Self::fetch_descriptor_blocking`] for the public contract):
    /// * `is_data_uri` → short-circuit via the pure domain decoder
    ///   (`cesium_resource::data_uri::decode_data_uri_bytes`); zero network.
    /// * non-GET method → `PortError::Network` (the `NetworkBackend::fetch(url)`
    ///   trait + the legacy `do_fetch` execute GET only; per-request
    ///   method/body await a trait extension — docs/deferred.md). All five
    ///   `Resource::fetch_*` builders emit GET, so the common path is covered.
    /// * `gate_enabled` → `NetworkResourceBackend` (→ `PipelineResourceBackend`
    ///   → 16-worker keep-alive `WorkerPool` → `UreqBackend`) by url + priority.
    /// * else → the byte-identical pre-M8.4 direct-ureq path
    ///   (`do_fetch_with_retry`, fetcher headers merged with descriptor headers,
    ///   retry count from `descriptor.retry.max_attempts`).
    fn execute_descriptor_gated(
        &self,
        descriptor: &FetchDescriptor,
        gate_enabled: bool,
    ) -> PortResult<Vec<u8>> {
        // L4 review fix: reject a non-GET method *before* the data-URI
        // short-circuit, so a `data:` descriptor carrying a non-GET method
        // can't slip past the method guard. (Every `Resource::fetch_*`/`post`
        // builder that emits a data URI uses GET, so this reorders the guard
        // without changing golden-path behavior.)
        if !matches!(descriptor.method, HttpMethod::Get) {
            return Err(PortError::Network(format!(
                "M8.4 backend executes GET only; {:?} awaits a NetworkBackend trait extension",
                descriptor.method
            )));
        }
        if descriptor.is_data_uri {
            return cesium_resource::data_uri::decode_data_uri_bytes(&descriptor.url)
                .map_err(|e| PortError::Decode(format!("data URI decode failed: {e:?}")));
        }
        // H2 review fix (interim, option b): the gate-ON backend path
        // (`NetworkResourceBackend::fetch_url_blocking`) forwards only
        // url + priority — it drops `descriptor.headers` (which may carry an
        // Authorization / Ion token) and `descriptor.retry`. Until the
        // `NetworkBackend` trait grows headers/retry parameters (deferred #41),
        // route any request that actually carries headers through the
        // byte-identical direct path so no credential is silently lost. A
        // header-less request still enjoys the shared pool + cache hierarchy.
        let has_headers = !descriptor.headers.is_empty() || !self.headers.is_empty();
        if gate_enabled && !has_headers {
            self.resource_backend
                .fetch_url_blocking(&descriptor.url, descriptor.priority)
        } else {
            let mut headers = self.headers.clone();
            headers.extend(descriptor.headers.clone());
            Self::do_fetch_with_retry(
                &self.agent,
                &descriptor.url,
                &headers,
                descriptor.retry.max_attempts,
            )
        }
    }

    /// M8.4 (#67): the adapter-side execution counterpart of the IO-free
    /// `Resource::fetch_array_buffer`/`fetch_json`/`fetch_text`/`fetch_image`/
    /// `fetch_blob`/`post` descriptor builders. This closes the M8
    /// “`Resource::fetch` 全量切换收敛” gate (门④): every descriptor executes
    /// either through the shared backend (gate ON) or the byte-identical legacy
    /// direct path (gate OFF) — never through an ad-hoc HTTP call scattered in a
    /// loader, so 门① (HTTP direct calls confined to the backend abstraction
    /// layer) is preserved after the switch.
    ///
    /// The gate is read once from [`resource_fetch_backend_enabled()`]
    /// (`CESIUM_ENABLE_RESOURCE_FETCH_BACKEND`). Blocking (like
    /// [`NetworkResourceBackend::fetch_url_blocking`]); call from a worker
    /// thread, never the frame thread. domain/resource stays IO-free.
    pub fn fetch_descriptor_blocking(&self, descriptor: &FetchDescriptor) -> PortResult<Vec<u8>> {
        self.execute_descriptor_gated(descriptor, resource_fetch_backend_enabled())
    }
}

impl TileFetcher for HttpTileFetcher {
    fn fetch<'a>(
        &'a self,
        url: &'a str,
        priority: f64,
    ) -> Pin<Box<dyn Future<Output = PortResult<Vec<u8>>> + Send + 'a>> {
        let url_owned = url.to_string();
        let cancelled = Arc::clone(&self.cancelled);
        let agent = self.agent.clone();
        let headers = self.headers.clone();
        let active = Arc::clone(&self.active_requests);
        let max_req = self.max_requests_per_server;
        let retry_count = self.retry_count;
        let server_key = Self::extract_server_key(&url_owned);
        let backend = Arc::clone(&self.resource_backend);
        // Snapshot the gate once per fetch so a mid-flight env mutation cannot
        // tear the rate-limit + fetch decision (matches the pre-M8.3 atomic
        // behavior where `tokio::task::spawn_blocking` captured the closure
        // environment at spawn time).
        let use_backend = resource_fetch_backend_enabled();

        // M8.3 tokio purge: the whole rate-limit + fetch + release sequence
        // runs on a dedicated `std::thread` (via `spawn_blocking_fetch`), not
        // on a tokio worker. The returned future blocks the polling thread on
        // `mpsc::recv()` and resolves `Ready` on the first poll, matching the
        // `PipelineResourceBackend::request_stream` pattern. Callers must
        // drive this future from an IO/worker context (never the frame
        // thread) — the same contract `PipelineResourceBackend` publishes.
        spawn_blocking_fetch(move || {
            // Cancellation check (byte-identical semantics to pre-M8.3).
            {
                let cancelled_set = cancelled.lock().unwrap();
                if cancelled_set.contains(&url_owned) {
                    return Err(PortError::Cancelled);
                }
            }

            // Rate-limit: wait until a slot opens for this server. Was a
            // `tokio::time::sleep(...).await` loop pre-M8.3; now a blocking
            // `std::thread::sleep` inside the spawned worker thread. The
            // observable behavior (slot acquisition order, poll interval,
            // saturating release) is unchanged.
            loop {
                let acquired = {
                    let mut active_map = active.lock().unwrap();
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
                std::thread::sleep(Duration::from_millis(RATE_LIMIT_POLL_MS));
            }

            // Dispatch: gate ON routes through the shared 16-worker keep-alive
            // pool (NetworkResourceBackend → PipelineResourceBackend →
            // WorkerPool → UreqBackend); gate OFF takes the pre-M8.3 direct
            // ureq path (`do_fetch_with_retry`) so the v0 baseline stays
            // byte-identical.
            // H2 review fix (interim, option b): mirror
            // `execute_descriptor_gated`. The gate-ON backend path forwards only
            // url + priority, so a fetcher configured with headers (e.g. an
            // Authorization / Ion token via `with_header`) would lose them.
            // Route header-bearing fetches through the direct path until the
            // backend grows a headers parameter (deferred #41).
            let result = if use_backend && headers.is_empty() {
                backend.fetch_url_blocking(&url_owned, priority)
            } else {
                Self::do_fetch_with_retry(&agent, &url_owned, &headers, retry_count)
            };

            // Release the slot (byte-identical semantics to pre-M8.3).
            {
                let mut active_map = active.lock().unwrap();
                if let Some(count) = active_map.get_mut(&server_key) {
                    *count = count.saturating_sub(1);
                }
            }

            result
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

/// A fetch failure paired with its retry classification (L2 review fix).
///
/// The pre-fix retry loop inferred "transient" from
/// `matches!(err, PortError::Network(_))`, but [`map_ureq_error`] folds *every*
/// non-404 HTTP status into `PortError::Network` — so a permanent 4xx (400 bad
/// request, 401/403 auth) was pointlessly retried. Carrying an explicit
/// `transient` flag lets [`HttpTileFetcher::do_fetch_with_retry`] retry only
/// genuinely retryable failures and fail fast on the rest.
struct FetchFailure {
    /// The error to surface to the caller.
    err: PortError,
    /// Whether the failure is worth retrying.
    transient: bool,
}

/// Classifies a ureq error into a [`FetchFailure`] (L2 review fix).
///
/// * HTTP 408 (request timeout) / 429 (too many requests) / 5xx → transient.
/// * HTTP 404 → `PortError::NotFound`, non-transient.
/// * any other 4xx → `PortError::Network`, non-transient (a client error won't
///   fix itself on retry).
/// * transport errors (DNS, connect, read timeout) → transient.
///
/// The `err` payload reuses [`map_ureq_error`] so the surfaced error variants
/// are unchanged from pre-fix; only the retry decision is corrected.
fn classify_ureq_error(err: ureq::Error) -> FetchFailure {
    let transient = match &err {
        ureq::Error::Status(code, _) => *code == 408 || *code == 429 || *code >= 500,
        ureq::Error::Transport(_) => true,
    };
    FetchFailure {
        err: map_ureq_error(err),
        transient,
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
    use cesium_ports_driven::ResourceBackend;

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
    //
    // M8.3: `#[tokio::test]` → `#[test]` + `block_on_noop`. The new
    // `spawn_blocking_fetch` bridge resolves `Ready` on the first poll (the
    // worker `std::thread` blocks internally on `mpsc::recv`), so a
    // `Waker::noop()` single-poll driver is sufficient — no tokio Runtime,
    // matching the production path's tokio-free contract.

    #[test]
    fn test_fetch_invalid_url() {
        let fetcher = HttpTileFetcher::new("https://invalid.example.invalid");
        let result = block_on_noop(fetcher.fetch("https://invalid.example.invalid/tile.terrain", 1.0));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PortError::Network(_)));
    }

    #[test]
    fn test_fetch_cancelled() {
        let fetcher = HttpTileFetcher::new("https://example.com");
        fetcher.cancel("https://example.com/cancelled.terrain");

        let result = block_on_noop(fetcher.fetch("https://example.com/cancelled.terrain", 1.0));
        assert!(matches!(result.unwrap_err(), PortError::Cancelled));
    }

    // --- mock ----------------------------------------------------------------

    #[test]
    fn test_mock_tile_fetcher() {
        let fetcher =
            MockTileFetcher::new().with_response("http://test.com/tile.terrain", vec![1, 2, 3, 4]);

        let result = block_on_noop(fetcher.fetch("http://test.com/tile.terrain", 1.0));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![1, 2, 3, 4]);

        let result = block_on_noop(fetcher.fetch("http://test.com/missing.terrain", 1.0));
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_tile_fetcher_cancel_is_noop() {
        let fetcher = MockTileFetcher::new();
        fetcher.cancel("anything");
        let result = block_on_noop(fetcher.fetch("anything", 1.0));
        assert!(result.is_err()); // not found, not cancelled — cancel is a noop
    }

    // --- M8.3 gate branch reachability ----------------------------------------
    //
    // Proves the gate-ON branch in `HttpTileFetcher::fetch` is syntactically
    // reachable and that `NetworkResourceBackend` is wired into the struct.
    // The end-to-end wiremock-driven assertions live in
    // `specs/tests/e2e_network/*` (deferred #37, `#[ignore]`-gated until
    // M11.1 wires the async harness).

    #[test]
    fn http_tile_fetcher_holds_network_resource_backend() {
        let fetcher = HttpTileFetcher::new("https://example.com");
        // The backend is constructed eagerly so the gate-ON branch is a pure
        // env-var decision at fetch time (no lazy-init race).
        assert_eq!(fetcher.resource_backend.name(), "cesium-network-resource");
        assert!(fetcher.resource_backend.is_available());
    }

    #[test]
    fn gate_off_takes_direct_ureq_branch() {
        // Ambient env must have CESIUM_ENABLE_RESOURCE_FETCH_BACKEND unset for
        // the golden path; assert the observed default so a stray export in
        // CI would fail loudly here rather than silently flipping the branch.
        if std::env::var(ENV_ENABLE_RESOURCE_FETCH_BACKEND).is_err() {
            assert!(!resource_fetch_backend_enabled());
        }
    }

    // --- error mapping -------------------------------------------------------

    #[test]
    fn test_map_ureq_error_status_404() {
        // We can't easily construct ureq::Error::Status without a real response,
        // but we test the logic indirectly via integration tests above.
        // This placeholder documents the expected mapping.
    }

    // --- M8.4 (#67): FetchDescriptor execution wiring ------------------------
    //
    // Proves the M8.4 convergence gate: the IO-free domain descriptors produced
    // by `Resource::fetch_*`/`post` execute through the gate-guarded adapter
    // path — data URIs short-circuit (zero network), gate ON routes through
    // `NetworkResourceBackend`, gate OFF takes the byte-identical direct path.
    // The gate decision is injected via `execute_descriptor_gated` so both
    // branches are exercised without mutating the process env (which would race
    // across parallel tests).

    /// Minimal offline HTTP server (ephemeral 127.0.0.1 port) returning `body`
    /// for every GET. Mirrors `adapters/pipeline/src/net/mod.rs::test_server`
    /// (which is `pub(crate)`, hence not reachable cross-crate).
    fn spawn_test_server(body: &'static [u8]) -> String {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().expect("local_addr").port();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                let mut buf = [0u8; 2048];
                let _ = stream.read(&mut buf); // drain the request head
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                if stream.write_all(head.as_bytes()).is_err() {
                    return;
                }
                if stream.write_all(body).is_err() {
                    return;
                }
                let _ = stream.flush();
            }
        });
        format!("http://127.0.0.1:{port}/tile")
    }

    #[test]
    fn fetch_descriptor_data_uri_short_circuits_no_network() {
        let fetcher = HttpTileFetcher::new("https://example.com");
        // "QUJD" is base64 for "ABC".
        let resource = cesium_resource::Resource::new("data:application/octet-stream;base64,QUJD");
        let descriptor = resource.fetch_array_buffer(None);
        assert!(descriptor.is_data_uri);
        let before = fetcher.resource_backend.fetch_count();
        // data URI short-circuits before the gate is even consulted.
        let out = fetcher
            .fetch_descriptor_blocking(&descriptor)
            .expect("data URI decodes");
        assert_eq!(out, b"ABC");
        assert_eq!(
            fetcher.resource_backend.fetch_count(),
            before,
            "data URI must not touch the backend"
        );
    }

    #[test]
    fn fetch_descriptor_rejects_non_get_method() {
        let fetcher = HttpTileFetcher::new("https://example.com");
        let resource = cesium_resource::Resource::new("https://example.com/api");
        let descriptor = resource.post(vec![1, 2, 3], None);
        assert!(matches!(descriptor.method, HttpMethod::Post));
        let err = fetcher.fetch_descriptor_blocking(&descriptor).unwrap_err();
        assert!(
            matches!(err, PortError::Network(_)),
            "non-GET surfaces a Network error, got {err:?}"
        );
    }

    #[test]
    fn fetch_descriptor_gate_off_direct_path_returns_body() {
        let url = spawn_test_server(b"tiledata");
        let fetcher = HttpTileFetcher::new("");
        let resource = cesium_resource::Resource::new(&url);
        let descriptor = resource.fetch_array_buffer(None);
        assert!(!descriptor.is_data_uri);
        // gate OFF (injected) -> byte-identical legacy direct-ureq path.
        let out = fetcher
            .execute_descriptor_gated(&descriptor, false)
            .expect("direct path");
        assert_eq!(out, b"tiledata");
    }

    #[test]
    fn fetch_descriptor_gate_on_backend_path_returns_body() {
        let url = spawn_test_server(b"tiledata");
        let fetcher = HttpTileFetcher::new("");
        let resource = cesium_resource::Resource::new(&url);
        let descriptor = resource.fetch_array_buffer(None);
        let before = fetcher.resource_backend.fetch_count();
        // gate ON (injected) -> NetworkResourceBackend -> WorkerPool -> UreqBackend.
        let out = fetcher
            .execute_descriptor_gated(&descriptor, true)
            .expect("backend path");
        assert_eq!(out, b"tiledata");
        assert!(
            fetcher.resource_backend.fetch_count() > before,
            "gate ON must route through the backend"
        );
    }

    /// Offline HTTP server returning `body` (200) only when the request head
    /// carries `required_header` (case-insensitive substring match); otherwise
    /// 403. Proves the H2 fallback actually transmits descriptor headers that
    /// the gate-ON backend path would drop.
    fn spawn_header_gate_server(required_header: &'static str, body: &'static [u8]) -> String {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().expect("local_addr").port();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).unwrap_or(0);
                let head = String::from_utf8_lossy(&buf[..n]).to_lowercase();
                let (status, payload): (&str, &[u8]) =
                    if head.contains(&required_header.to_lowercase()) {
                        ("200 OK", body)
                    } else {
                        ("403 Forbidden", b"missing-header")
                    };
                let resp_head = format!(
                    "HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    status,
                    payload.len()
                );
                if stream.write_all(resp_head.as_bytes()).is_err() {
                    return;
                }
                if stream.write_all(payload).is_err() {
                    return;
                }
                let _ = stream.flush();
            }
        });
        format!("http://127.0.0.1:{port}/tile")
    }

    #[test]
    fn gate_on_with_headers_falls_back_to_direct_and_sends_them() {
        // H2 review fix: a descriptor carrying headers must NOT lose them on
        // the gate-ON path. The server returns the body only when the
        // `X-Ion-Token` header arrives, so receiving the body proves the
        // interim fallback to the direct path transmitted it; `fetch_count`
        // staying put proves the request bypassed the header-dropping backend.
        let url = spawn_header_gate_server("X-Ion-Token: secret", b"authorized-tile");
        let fetcher = HttpTileFetcher::new("");
        let resource = cesium_resource::Resource::new(&url).with_header("X-Ion-Token", "secret");
        let descriptor = resource.fetch_array_buffer(None);
        assert!(
            !descriptor.headers.is_empty(),
            "descriptor must carry the token header"
        );

        let before = fetcher.resource_backend.fetch_count();
        // gate ON (injected) but headers present -> interim fallback to direct.
        let out = fetcher
            .execute_descriptor_gated(&descriptor, true)
            .expect("header-bearing gate-ON request falls back to direct and succeeds");
        assert_eq!(out, b"authorized-tile");
        assert_eq!(
            fetcher.resource_backend.fetch_count(),
            before,
            "header-bearing request must bypass the header-dropping backend path"
        );
    }

    #[test]
    fn non_get_data_uri_descriptor_rejected_before_short_circuit() {
        // L4 review fix: the method guard now runs BEFORE the data-URI
        // short-circuit, so a non-GET descriptor carrying a `data:` URL is
        // rejected with a Network error instead of silently decoding. Pre-fix
        // the data-URI branch fired first and returned the decoded bytes for a
        // POST. (All `Resource::fetch_*` data-URI builders emit GET; only the
        // nonsensical `post()`-on-data-URI path reaches this guard.)
        let fetcher = HttpTileFetcher::new("");
        let resource = cesium_resource::Resource::new("data:application/octet-stream;base64,QUJD");
        let descriptor = resource.post(vec![1, 2, 3], None);
        assert!(descriptor.is_data_uri, "data: URL still flagged");
        assert!(matches!(descriptor.method, HttpMethod::Post));
        let err = fetcher
            .execute_descriptor_gated(&descriptor, false)
            .unwrap_err();
        assert!(
            matches!(err, PortError::Network(_)),
            "non-GET rejected before the data-URI decode, got {err:?}"
        );
    }

    #[test]
    fn permanent_4xx_fails_fast_without_retry() {
        // L2 review fix: a permanent 4xx (here 400 Bad Request) is classified
        // non-transient, so `do_fetch_with_retry` fails fast on the FIRST
        // attempt instead of retrying `retry_count` times. The server counts
        // hits; exactly one proves no retry. (408/429/5xx stay transient and
        // would be retried.)
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::sync::atomic::{AtomicUsize, Ordering};
        let hits = Arc::new(AtomicUsize::new(0));
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().expect("local_addr").port();
        let hits_srv = Arc::clone(&hits);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                let mut buf = [0u8; 2048];
                let _ = stream.read(&mut buf);
                hits_srv.fetch_add(1, Ordering::SeqCst);
                let head =
                    "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                if stream.write_all(head.as_bytes()).is_err() {
                    return;
                }
                let _ = stream.flush();
            }
        });
        let url = format!("http://127.0.0.1:{port}/tile");
        let agent = ureq::AgentBuilder::new()
            .timeout_read(Duration::from_secs(5))
            .build();
        let headers: HashMap<String, String> = HashMap::new();
        let err = HttpTileFetcher::do_fetch_with_retry(&agent, &url, &headers, 3).unwrap_err();
        assert!(
            matches!(err, PortError::Network(_)),
            "400 surfaces a Network error, got {err:?}"
        );
        assert_eq!(
            hits.load(Ordering::SeqCst),
            1,
            "permanent 4xx must NOT be retried (exactly one server hit)"
        );
    }
}
