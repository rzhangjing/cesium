//! M8.3 — Network `ResourceBackend` adapter (adapters/network).
//!
//! This module carries the M8.3 structural refactor of the network adapter:
//!
//! 1. [`resource_fetch_backend_enabled`] — a **local** env-gate reader for
//!    `CESIUM_ENABLE_RESOURCE_FETCH_BACKEND`. It mirrors the M9.2
//!    `ENV_ENABLE_GLTF_UPGRADE` precedent in
//!    `adapters/bevy-render/src/tileset/content_loader.rs:489-496` (adapter-
//!    local read; **no** dependency on the `cesium-app` `feature_flags`
//!    registry, which is being co-edited by parallel milestones). The name is
//!    **deliberately distinct** from `ENV_ENABLE_RESOURCE_BACKEND` (M12) — see
//!    the M8-vs-M12 naming-isolation ruling in
//!    `ports/driven/src/resource.rs:69-85`.
//! 2. [`spawn_blocking_fetch`] / [`block_on_noop`] — the **sole** tokio-free
//!    async-boundary helpers used by [`crate::HttpTileFetcher::fetch`] after
//!    the M8.3 tokio purge. Both are `std`-only (`std::thread` +
//!    `std::sync::mpsc` + `Waker::noop`), matching the
//!    `PipelineResourceBackend::request_stream` "sync-in-async" pattern
//!    (`adapters/pipeline/src/resource_backend.rs:296-343`).
//! 3. [`NetworkResourceBackend`] — a [`ResourceBackend<u64>`] implementation
//!    for network-streamed bulk assets. It is a **thin wrapper** around
//!    [`PipelineResourceBackend<u64>`] from `cesium-pipeline`, so the 16-worker
//!    keep-alive [`WorkerPool`](cesium_pipeline::pool::WorkerPool) +
//!    [`UreqBackend`](cesium_pipeline::net::ureq_backend::UreqBackend) +
//!    hot/warm cache hierarchy + in-flight dedup are **reused verbatim** — no
//!    parallel pool, no duplicated cache, no new HTTP client. The wrapper only
//!    adds a URL registry so callers can key requests by URL string (hashed to
//!    `u64` via [`url_hash`]) while satisfying the pipeline's `K: Copy` bound.
//!
//! # Hard constraints honoured
//!
//! * **No tokio Runtime, no `block_on` from an executor.** All blocking is on
//!   `std::thread` + `std::sync::mpsc`; the returned futures resolve `Ready`
//!   on the first poll (`Waker::noop()` is sufficient to drive them).
//! * **No parallel pool.** The heavy lifting (retry + keep-alive + dedup +
//!   hot/warm cache) is delegated verbatim to `PipelineResourceBackend` so the
//!   three eviction invariants that protect the `dynamic_globe` golden path
//!   from 花屏 stay in force.
//! * **Gate OFF = byte-identical to pre-M8.3.** `resource_fetch_backend_enabled`
//!   returns `false` unless `CESIUM_ENABLE_RESOURCE_FETCH_BACKEND` is truthy,
//!   and `HttpTileFetcher::fetch` only takes the new-backend branch when it is.
//!   Default (unset) ⇒ identical rate-limit + retry + cancellation semantics
//!   to the pre-M8.3 `HttpTileFetcher`, so the v0 8-image baseline stays
//!   zero-diff.
//! * **domain/resource stays network-free.** This adapter consumes the M8.1
//!   domain types (`FetchDescriptor`, `RetryPolicy`, `classify_status`) but
//!   never re-exports ureq / reqwest / tokio back into the domain layer.

use std::collections::HashMap;
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread;

use cesium_pipeline::net::ureq_backend::UreqBackend;
use cesium_pipeline::net::NetworkBackend;
use cesium_pipeline::resource_backend::PipelineResourceBackend;
use cesium_pipeline::runtime::UrlBuilder;
use cesium_ports_driven::{CacheTier, PortResult, ResourceBackend, ResourceStats};

// ─────────────────────────────────────────────────────────────────────────────
// Env-gate (M8.3) — local read, no `feature_flags.rs` dependency
// ─────────────────────────────────────────────────────────────────────────────

/// Env-var name for the M8.3 network-resource-backend gate.
///
/// **Deliberately distinct** from `ENV_ENABLE_RESOURCE_BACKEND` (M12 — the
/// CesiumJS `Resource` object abstraction: URL templates, query parameters,
/// retry headers). The two flags are semantically unrelated and must never be
/// wired to each other; see the M8-vs-M12 naming-isolation ruling in
/// `ports/driven/src/resource.rs:69-85`.
pub const ENV_ENABLE_RESOURCE_FETCH_BACKEND: &str = "CESIUM_ENABLE_RESOURCE_FETCH_BACKEND";

/// Truthy-token predicate — **byte-identical** to
/// `pipeline::fetch::truthy` (`adapters/bevy-render/src/pipeline/fetch.rs:75`)
/// and `feature_flags::truthy` in `cesium-app`. Accepts (case-insensitive,
/// surrounding whitespace trimmed): `1`, `true`, `yes`, `on`. Everything else
/// (including unset) is `false`.
fn truthy(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Pure gate evaluation from a raw env value (`None` = unset). Split out from
/// [`resource_fetch_backend_enabled`] so the truth-table is unit-testable
/// **without** mutating process-global env (which would race parallel tests).
/// Mirrors `pipeline::fetch::gate_from_env_value` in bevy-render.
pub fn gate_from_env_value(raw: Option<String>) -> bool {
    match raw {
        Some(v) => truthy(&v),
        None => false,
    }
}

/// Reads the M8.3 gate locally, mirroring the M9.2 `ENV_ENABLE_GLTF_UPGRADE`
/// precedent (`adapters/bevy-render/src/tileset/content_loader.rs:489-496`).
///
/// Defaults OFF, so the pre-M8.3 [`crate::HttpTileFetcher`] path stays
/// byte-identical unless `CESIUM_ENABLE_RESOURCE_FETCH_BACKEND` is explicitly
/// truthy. **Does not** consult `cesium-app::feature_flags` (that registry is
/// co-edited by parallel milestones; adapter-local reads avoid the conflict).
#[inline]
pub fn resource_fetch_backend_enabled() -> bool {
    gate_from_env_value(std::env::var(ENV_ENABLE_RESOURCE_FETCH_BACKEND).ok())
}

// ─────────────────────────────────────────────────────────────────────────────
// URL hashing — bridge `String` URLs into the pipeline's `K: Copy` contract
// ─────────────────────────────────────────────────────────────────────────────

/// Hashes a URL to a stable `u64` key using [`std::collections::hash_map::DefaultHasher`]
/// (SipHash-1-3 with a per-process seed — stable within a process, which is
/// the required scope for the pipeline cache).
///
/// Used by [`NetworkResourceBackend`] to key the pipeline cache hierarchy by
/// URL without holding `String` keys (the pipeline `K: Copy` bound excludes
/// `String`). Hash collisions are handled by the URL registry: a colliding key
/// would resolve to whichever URL was inserted last, so callers relying on
/// long-lived caches should prefer content-hashed keys (e.g. tile quadkey
/// packed into `u64`) over URL hashes.
pub fn url_hash(url: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut h);
    h.finish()
}

// ─────────────────────────────────────────────────────────────────────────────
// spawn_blocking_fetch / block_on_noop — the tokio-free Future bridge
// ─────────────────────────────────────────────────────────────────────────────

/// Runs `work` on a dedicated `std::thread` and returns a `Send` future that
/// resolves `Ready` with the closure's return value on the first poll.
///
/// This is the **sole** async-boundary helper used by
/// [`crate::HttpTileFetcher::fetch`] after the M8.3 tokio purge. It matches
/// the "sync-in-async" pattern used by `PipelineResourceBackend::request_stream`
/// (`adapters/pipeline/src/resource_backend.rs:296-343`): the returned future
/// blocks its polling thread on `mpsc::recv()` and never yields `Pending`, so
/// a `Waker::noop()` single-poll driver (see [`block_on_noop`]) is sufficient
/// — no executor, no tokio Runtime.
///
/// Callers **must** drive the future from an IO/worker context (never the
/// frame thread), matching the pipeline's blocking-pool philosophy.
pub fn spawn_blocking_fetch<F, T>(work: F) -> Pin<Box<dyn Future<Output = T> + Send>>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = mpsc::channel::<T>();
    // Detached std::thread — no tokio, no JoinHandle tracking. If the caller
    // drops the future before completion the sender's `send` returns Err and
    // is silently ignored (the worker still runs to completion, mirroring
    // `tokio::task::spawn_blocking`'s drop semantics).
    thread::spawn(move || {
        let out = work();
        let _ = tx.send(out);
    });
    Box::pin(BlockedOnRecv { rx: Some(rx) })
}

/// Future that blocks the polling thread on `mpsc::recv()` and returns `Ready`
/// on the first poll. Panics only if the worker thread dropped the sender
/// without sending (would require `std::mem::forget`-style misuse inside the
/// closure — treated as a bug, not a recoverable state).
struct BlockedOnRecv<T> {
    rx: Option<mpsc::Receiver<T>>,
}

impl<T: Send + 'static> Future for BlockedOnRecv<T> {
    type Output = T;
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<T> {
        let rx = self
            .rx
            .take()
            .expect("BlockedOnRecv polled after completion (single-poll future)");
        match rx.recv() {
            Ok(v) => Poll::Ready(v),
            Err(_) => panic!("spawn_blocking_fetch worker dropped sender without a result"),
        }
    }
}

/// Drives a boxed future to completion using [`Waker::noop`] + a single poll.
///
/// Suitable only for futures that block internally and never yield `Pending`
/// ([`spawn_blocking_fetch`], `PipelineResourceBackend::request_stream`). This
/// is **not** a general-purpose executor — for futures that yield, use a real
/// runtime (which this codebase intentionally avoids on the frame thread).
pub fn block_on_noop<F>(fut: F) -> F::Output
where
    F: Future,
{
    let mut fut = Box::pin(fut);
    let mut cx = Context::from_waker(Waker::noop());
    match fut.as_mut().poll(&mut cx) {
        Poll::Ready(v) => v,
        Poll::Pending => {
            panic!("block_on_noop: future returned Pending (must block internally to Ready)")
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// NetworkResourceBackend — the M8.3 network-flavored ResourceBackend
// ─────────────────────────────────────────────────────────────────────────────

/// URL registry shared between the backend handle and the `url_builder`
/// closure handed to [`PipelineResourceBackend`]. Inserted on every
/// [`NetworkResourceBackend::fetch_url_blocking`] call so the pipeline's
/// `url_builder: fn(&u64) -> String` can resolve keys back to URLs.
type UrlRegistry = Arc<Mutex<HashMap<u64, String>>>;

/// Network-flavored [`ResourceBackend`] for bulk asset streaming (M8.3).
///
/// Thin wrapper around [`PipelineResourceBackend<u64>`] that keys requests by
/// a `u64` URL hash ([`url_hash`]) and manages a URL registry so the pipeline's
/// `url_builder: fn(&K) -> String` closure can resolve keys back to URLs. All
/// heavy lifting — the 16-worker keep-alive pool, retry, hot/warm cache
/// hierarchy, in-flight dedup, and the three eviction invariants — is
/// delegated verbatim to the pipeline backend. No parallel pool, no
/// duplicated cache.
///
/// # Concrete instantiation
///
/// Monomorphic over `K = u64` (URL hash). Callers needing a different key
/// type should either (a) hash to `u64` at the call site, or (b) construct
/// [`PipelineResourceBackend<K>`] directly with their own `url_builder` — the
/// wrapper exists only to bridge URL-string fetches into the `K: Copy`
/// pipeline contract.
///
/// # Dyn-compatibility
///
/// Implements [`ResourceBackend<u64>`] so it can be held as
/// `Box<dyn ResourceBackend<u64>>` alongside the pipeline's own
/// `PipelineResourceBackend<TileKey>` (which uses `(u32, u32, u32)` keys).
pub struct NetworkResourceBackend {
    inner: PipelineResourceBackend<u64>,
    registry: UrlRegistry,
    fetch_count: AtomicU64,
    available: AtomicBool,
}

impl NetworkResourceBackend {
    /// Creates a backend with the default ureq network layer (16 workers,
    /// 10 s timeout, keep-alive pooling — see
    /// [`UreqBackend::new`]) and the pipeline's golden-path cache/pool
    /// configuration (3000-entry hot cache, base-layer zoom 3, dual-timescale
    /// retry: 3 attempts, 250 ms base backoff).
    pub fn new() -> Self {
        Self::with_name("cesium-network-resource")
    }

    /// Creates a backend with a custom diagnostic name (surfaced through
    /// [`ResourceBackend::name`]).
    pub fn with_name(name: &str) -> Self {
        let registry: UrlRegistry = Arc::new(Mutex::new(HashMap::new()));
        let reg_builder = Arc::clone(&registry);
        let url_builder: UrlBuilder<u64> = Arc::new(move |k: &u64| {
            reg_builder
                .lock()
                .unwrap()
                .get(k)
                .cloned()
                .unwrap_or_default()
        });
        let net: Arc<dyn NetworkBackend> = Arc::new(UreqBackend::new());
        // Zoom-of is unused for URL-hash keys (no LOD pyramid); return 0 to
        // keep the pipeline's `BaseLayerGuard` invariant trivially satisfied
        // — no key is ever treated as a protected base-layer tile, which is
        // correct for generic bulk asset streaming (the base-layer exemption
        // applies to tile pyramids, not URL-keyed assets).
        let inner = PipelineResourceBackend::new(name, net, url_builder, |_k: &u64| 0);
        Self {
            inner,
            registry,
            fetch_count: AtomicU64::new(0),
            available: AtomicBool::new(true),
        }
    }

    /// Registers `url` under a stable hash key and streams the asset bytes
    /// through the pipeline-managed cache hierarchy. **Blocking** — call from
    /// an IO/worker thread, never the frame thread.
    ///
    /// On cache hit, resolves immediately (no network round-trip). On cold
    /// miss, submits to the 16-worker keep-alive pool and blocks on the
    /// dispatcher's completion signal (via
    /// [`PipelineResourceBackend::request_stream`] → `mpsc::recv`).
    ///
    /// Concurrent calls with the same URL collapse to a single network fetch
    /// via the pipeline's `Dedup` set (see
    /// `adapters/pipeline/src/resource_backend.rs:592-654` for the invariant
    /// test).
    pub fn fetch_url_blocking(&self, url: &str, priority: f64) -> PortResult<Vec<u8>> {
        let key = url_hash(url);
        // Register the URL under its hash key (idempotent for repeat fetches;
        // a hash collision would overwrite, but SipHash-1-3 collisions on
        // realistic URL sets are astronomically unlikely).
        self.registry.lock().unwrap().insert(key, url.to_string());
        self.fetch_count.fetch_add(1, Ordering::Relaxed);
        // `request_stream` blocks internally (mpsc::recv on the dispatcher);
        // a `Waker::noop()` single-poll driver is sufficient.
        block_on_noop(self.inner.request_stream(key, priority))
    }

    /// Cumulative count of [`Self::fetch_url_blocking`] invocations
    /// (diagnostic — counts cold misses + cache hits + dedup-shares alike).
    pub fn fetch_count(&self) -> u64 {
        self.fetch_count.load(Ordering::Relaxed)
    }

    /// Direct access to the underlying pipeline backend (for cache hierarchy
    /// management — `insert` / `evict` / `hide` / `show` / `cached_bytes`).
    pub fn inner(&self) -> &PipelineResourceBackend<u64> {
        &self.inner
    }

    /// Marks the backend unavailable (subsequent [`ResourceBackend::is_available`]
    /// returns `false`). Used by graceful-shutdown paths; the underlying
    /// pipeline backend continues to drain in-flight work.
    pub fn mark_unavailable(&self) {
        self.available.store(false, Ordering::Relaxed);
    }
}

impl Default for NetworkResourceBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceBackend<u64> for NetworkResourceBackend {
    fn request_stream<'a>(
        &'a self,
        key: u64,
        priority: f64,
    ) -> Pin<Box<dyn Future<Output = PortResult<Vec<u8>>> + Send + 'a>> {
        self.inner.request_stream(key, priority)
    }

    fn cancel(&self, key: &u64) {
        self.inner.cancel(key);
    }

    fn cache_tier(&self, key: &u64) -> CacheTier {
        self.inner.cache_tier(key)
    }

    fn stats(&self) -> ResourceStats {
        self.inner.stats()
    }

    fn name(&self) -> &str {
        self.inner.name()
    }

    fn is_available(&self) -> bool {
        self.available.load(Ordering::Relaxed) && self.inner.is_available()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // --- gate_from_env_value truth table (pure, no env mutation) -------------

    #[test]
    fn gate_unset_is_off() {
        assert!(!gate_from_env_value(None));
    }

    #[test]
    fn gate_truthy_tokens_are_on() {
        for t in ["1", "true", "TRUE", "True", "yes", "YES", "on", "ON", " 1 ", "\ttrue\n"] {
            assert!(
                gate_from_env_value(Some(t.to_string())),
                "{t:?} must be truthy"
            );
        }
    }

    #[test]
    fn gate_falsy_tokens_are_off() {
        for t in ["0", "false", "no", "off", "", "  ", "maybe", "2", "-1"] {
            assert!(
                !gate_from_env_value(Some(t.to_string())),
                "{t:?} must be falsy"
            );
        }
    }

    // --- url_hash stability ---------------------------------------------------

    #[test]
    fn url_hash_is_stable_within_process() {
        let a = url_hash("https://tiles.example.com/1/2/3.b3dm");
        let b = url_hash("https://tiles.example.com/1/2/3.b3dm");
        assert_eq!(a, b);
    }

    #[test]
    fn url_hash_distinguishes_distinct_urls() {
        let a = url_hash("https://tiles.example.com/1/2/3.b3dm");
        let b = url_hash("https://tiles.example.com/1/2/4.b3dm");
        assert_ne!(a, b);
    }

    // --- block_on_noop / spawn_blocking_fetch ---------------------------------

    #[test]
    fn spawn_blocking_fetch_returns_ready_on_first_poll() {
        let fut = spawn_blocking_fetch(|| 42u32);
        let v = block_on_noop(fut);
        assert_eq!(v, 42);
    }

    #[test]
    fn spawn_blocking_fetch_propagates_closure_result() {
        let fut = spawn_blocking_fetch(|| Ok::<_, ()>(vec![1u8, 2, 3]));
        let v = block_on_noop(fut).unwrap();
        assert_eq!(v, vec![1, 2, 3]);
    }

    #[test]
    fn spawn_blocking_fetch_runs_on_separate_thread() {
        let main_id = thread::current().id();
        let fut = spawn_blocking_fetch(move || {
            let worker_id = thread::current().id();
            (worker_id, worker_id != main_id)
        });
        let (_id, differs) = block_on_noop(fut);
        assert!(differs, "worker must run on a distinct std::thread");
    }

    // --- NetworkResourceBackend cache hierarchy -------------------------------
    //
    // These tests exercise the wrapper's cache-hit path (which resolves
    // synchronously without any network) so they stay hermetic and offline-
    // safe. Cold-miss + wiremock-driven network paths are covered by the
    // e2e_network skeleton in `specs/tests/e2e_network/*` (deferred #37,
    // gated behind `#[ignore]` until M11.1 wires the async harness).

    #[test]
    fn backend_reports_name_and_availability() {
        let be = NetworkResourceBackend::with_name("unit-test-backend");
        assert_eq!(be.name(), "unit-test-backend");
        assert!(be.is_available());
        be.mark_unavailable();
        assert!(!be.is_available());
    }

    #[test]
    fn backend_starts_with_empty_cache() {
        let be = NetworkResourceBackend::new();
        let stats = be.stats();
        assert_eq!(stats.hot_entries, 0);
        assert_eq!(stats.warm_entries, 0);
        assert_eq!(stats.in_flight, 0);
        assert_eq!(stats.streamed, 0);
        assert_eq!(be.fetch_count(), 0);
    }

    #[test]
    fn backend_cache_hit_resolves_without_network() {
        let be = NetworkResourceBackend::new();
        let key = url_hash("https://cached.example/asset.bin");
        // Directly seed the hot cache via the inner pipeline backend, then
        // assert `request_stream` resolves from cache (no wiremock needed).
        be.inner().insert(key, vec![9, 8, 7]);
        assert_eq!(be.cache_tier(&key), CacheTier::Hot);

        let bytes = block_on_noop(be.request_stream(key, 1.0)).unwrap();
        assert_eq!(bytes, vec![9, 8, 7]);
    }

    #[test]
    fn backend_is_dyn_compatible() {
        let be = NetworkResourceBackend::new();
        let boxed: Box<dyn ResourceBackend<u64>> = Box::new(be);
        assert_eq!(boxed.name(), "cesium-network-resource");
        assert!(boxed.is_available());
        let key = url_hash("https://dyn.example/x");
        assert_eq!(boxed.cache_tier(&key), CacheTier::Cold);
    }

    #[test]
    fn backend_cancel_on_unknown_key_is_noop() {
        let be = NetworkResourceBackend::new();
        let key = url_hash("https://never-requested.example/x");
        // Must not panic; the underlying pipeline backend treats unknown-key
        // cancel as a no-op (dedup.remove + pool.remove_wanted are idempotent).
        be.cancel(&key);
        assert_eq!(be.cache_tier(&key), CacheTier::Cold);
    }

    // --- env-gate default (unset) ---------------------------------------------

    #[test]
    fn resource_fetch_backend_enabled_defaults_off_when_unset() {
        // Sanity check: the ambient test environment must not have
        // CESIUM_ENABLE_RESOURCE_FETCH_BACKEND set (otherwise every other test
        // in the workspace that touches the gate would flip behavior).
        //
        // We do NOT unset it here (that would race parallel tests); we only
        // assert the observed default. CI / local runs must keep this var
        // unset for the golden path.
        if std::env::var(ENV_ENABLE_RESOURCE_FETCH_BACKEND).is_err() {
            assert!(!resource_fetch_backend_enabled());
        }
    }
}
