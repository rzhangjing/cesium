//! M8 / P2.3 — `ResourceBackend` adapter: bulk asset streaming via the
//! pipeline-managed cache hierarchy.
//!
//! This module implements the [`cesium_ports_driven::ResourceBackend`] contract
//! for bulk asset streaming (textures / meshes / 3D Tiles content). It is a
//! **pure `std`, tokio-free** IO/cache-layer adapter that lives alongside the
//! M1.2 tile pipeline and *reuses* its cache primitives rather than building a
//! parallel cache system:
//!
//! | Tier | Reused primitive | Role |
//! |------|------------------|------|
//! | Hot  | [`GpuCache`]     | FIFO eviction + base-layer lock + live deferral (the three M1.2 invariants) |
//! | Warm | [`HiddenLru`]    | hidden warm-fallback tracking, LRU-first despawn under budget |
//! | —    | [`Dedup`]        | in-flight deduplication (one network fetch per cold key) |
//! | —    | [`WorkerPool`]   | the existing 16-worker blocking pool + keep-alive (ureq) |
//!
//! # Hard constraints honoured here
//!
//! - **No parallel cache system.** Eviction/dedup semantics are delegated to
//!   [`GpuCache`] / [`HiddenLru`] / [`Dedup`] verbatim, so the three invariants
//!   (BASE_LAYER exemption / live deferral / termination) that protect the
//!   `dynamic_globe` golden path from 花屏 are preserved.
//! - **No tokio main runtime.** Network streaming rides the existing blocking
//!   [`WorkerPool`] (std threads + `mpsc`, ureq keep-alive). The boxed future
//!   returned by [`ResourceBackend::request_stream`] blocks the *calling*
//!   thread on an `mpsc` receiver until the internal dispatcher delivers — it
//!   must therefore be driven from an IO/worker context, never the frame
//!   thread. This matches the blocking-pool philosophy of `pool.rs` and the
//!   `Waker::noop` `block_on` pattern used elsewhere (no executor, no tokio).
//! - **No glam / no coordinate math.** M8 is an IO/cache layer; keys are opaque
//!   (`Hash + Eq + Copy`) and the only numeric type is the `f64` priority.
//! - **Opt-in, not wired.** This backend is *not* registered into any default
//!   `cesium-app` plugin (M1.3 rollout guard): the golden path stays
//!   pixel-neutral until M8 is explicitly promoted.
//!
//! # Naming clarification (M8 vs M12)
//!
//! See the trait-level doc on [`cesium_ports_driven::ResourceBackend`]. In
//! short: this streaming/cache contract is **semantically unrelated** to the
//! M12 `ENV_ENABLE_RESOURCE_BACKEND` flag (the `Resource` object abstraction:
//! URL templates / query parameters / retry headers). M8 does **not** read or
//! reuse that flag.

use std::collections::HashMap;
use std::future::Future;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cesium_ports_driven::{
    CacheTier, PortError, PortResult, ResourceBackend, ResourceStats,
};

use crate::base_layer::BaseLayerGuard;
use crate::budget::DefaultBudget;
use crate::dedup::Dedup;
use crate::gpu_cache::{EvictionResult, GpuCache};
use crate::hidden_lru::HiddenLru;
use crate::net::NetworkBackend;
use crate::pool::{Decoder, Job, JobOutcome, PoolConfig, WorkerPool};
use crate::runtime::UrlBuilder;

/// Per-key completion senders. A cold [`ResourceBackend::request_stream`] call
/// registers a sender here; the dispatcher thread delivers the streamed result
/// to every waiter for that key (so concurrent cold requests for the same key
/// share a single network fetch).
type Waiters<K> = HashMap<K, Vec<mpsc::Sender<PortResult<Vec<u8>>>>>;

/// Cumulative streaming counters (atomic — read cross-thread by `stats()`).
struct RbStats {
    /// Assets streamed successfully into the cache.
    streamed: AtomicU32,
    /// Cold requests deduplicated (already in-flight).
    deduped: AtomicU32,
    /// Hot-tier FIFO evictions.
    evicted: AtomicU32,
}

impl RbStats {
    fn new() -> Self {
        Self {
            streamed: AtomicU32::new(0),
            deduped: AtomicU32::new(0),
            evicted: AtomicU32::new(0),
        }
    }
}

/// Shared interior state, owned by the backend handle and the dispatcher
/// thread (via `Arc`).
struct Inner<K>
where
    K: Hash + Eq + Copy + Send + 'static,
{
    /// Diagnostics name.
    name: String,
    /// Hot/warm data store (reused `GpuCache`, FIFO + three invariants).
    cache: Mutex<GpuCache<K, Vec<u8>>>,
    /// Warm-tier hidden-LRU tracking (reused `HiddenLru`).
    hidden: Mutex<HiddenLru<K>>,
    /// In-flight dedup (reused `Dedup`).
    dedup: Dedup<K>,
    /// Existing 16-worker blocking pool + ureq keep-alive (reused `WorkerPool`).
    pool: WorkerPool<K, Vec<u8>>,
    /// Per-key completion waiters (push-pool → pull-future bridge).
    waiters: Mutex<Waiters<K>>,
    /// Key → fetch URL.
    url_builder: UrlBuilder<K>,
    /// Serializes the cold-intake decision (cache-miss check + dedup mark +
    /// submit) against the dispatcher's completion (dedup clear + cache
    /// insert). Without it a completion could sneak between a caller's
    /// cache-miss check and its `dedup.insert`, producing a redundant second
    /// fetch for the same cold key. Lock order is always `intake` first, then
    /// the short-lived `cache` / `dedup` / `waiters` locks (never two of those
    /// held at once), so there is no inversion.
    intake: Mutex<()>,
    /// Cumulative counters.
    stats: RbStats,
    /// Set on `Drop` to stop the dispatcher thread.
    shutdown: AtomicBool,
    /// Test-only seam (M2): when set, the dispatcher panics on its next
    /// non-empty result batch so the `catch_unwind` + waiter-drain recovery in
    /// [`run_dispatcher_guarded`] can be exercised deterministically.
    #[cfg(test)]
    test_panic: AtomicBool,
}

/// Pipeline-backed [`ResourceBackend`] for bulk asset streaming.
///
/// Generic over the asset key `K` (e.g. `TileKey = (u32, u32, u32)`, or a
/// `u64` content hash). Dyn-compatible once `K` is concrete:
/// `Box<dyn ResourceBackend<TileKey>>`.
///
/// Construct via [`PipelineResourceBackend::new`] (golden-path defaults) or
/// [`PipelineResourceBackend::with_config`] (explicit pool/cache config, used
/// by tests). Dropping the backend stops the internal dispatcher thread and
/// closes the worker pool.
pub struct PipelineResourceBackend<K>
where
    K: Hash + Eq + Copy + Send + 'static,
{
    inner: Arc<Inner<K>>,
    /// Internal dispatcher: drains pool results → cache + waiters.
    dispatcher: Option<thread::JoinHandle<()>>,
}

impl<K> PipelineResourceBackend<K>
where
    K: Hash + Eq + Copy + Send + 'static,
{
    /// Create a backend with golden-path defaults: 16 download threads
    /// (`DefaultBudget::DOWNLOAD_THREADS`), a 3000-entry hot cache
    /// (`MAX_GPU_CACHE_ENTRIES`), base-layer zoom 3, and the standard
    /// dual-timescale retry (3 attempts, 250 ms base backoff).
    pub fn new(
        name: impl Into<String>,
        backend: Arc<dyn NetworkBackend>,
        url_builder: UrlBuilder<K>,
        zoom_of: fn(&K) -> u32,
    ) -> Self {
        Self::with_config(
            name,
            backend,
            url_builder,
            zoom_of,
            BaseLayerGuard::new(),
            DefaultBudget::MAX_GPU_CACHE_ENTRIES,
            PoolConfig {
                threads: DefaultBudget::DOWNLOAD_THREADS,
                max_attempts: 3,
                backoff_base: Duration::from_millis(250),
            },
        )
    }

    /// Create a backend with explicit configuration (used by tests).
    ///
    /// - `base_guard`: base-layer zoom exemption (invariant 1).
    /// - `max_cache_entries`: hot-cache FIFO cap (invariant 3 termination).
    /// - `config`: worker-pool thread count + retry policy.
    pub fn with_config(
        name: impl Into<String>,
        backend: Arc<dyn NetworkBackend>,
        url_builder: UrlBuilder<K>,
        zoom_of: fn(&K) -> u32,
        base_guard: BaseLayerGuard,
        max_cache_entries: usize,
        config: PoolConfig,
    ) -> Self {
        // Identity decoder: non-empty bytes stream through as-is; an empty
        // response is classified as a placeholder (no usable asset), matching
        // the tile pipeline's `is_placeholder_tile` convention.
        let decode: Decoder<Vec<u8>> = Arc::new(|d: &[u8]| {
            if d.is_empty() {
                None
            } else {
                Some(d.to_vec())
            }
        });

        let pool = WorkerPool::spawn(backend, decode, config);
        let inner = Arc::new(Inner {
            name: name.into(),
            cache: Mutex::new(GpuCache::new(max_cache_entries, base_guard, zoom_of)),
            hidden: Mutex::new(HiddenLru::new()),
            dedup: Dedup::new(),
            pool,
            waiters: Mutex::new(HashMap::new()),
            url_builder,
            intake: Mutex::new(()),
            stats: RbStats::new(),
            shutdown: AtomicBool::new(false),
            #[cfg(test)]
            test_panic: AtomicBool::new(false),
        });

        let disp = Arc::clone(&inner);
        let dispatcher = thread::spawn(move || run_dispatcher_guarded(disp));

        Self {
            inner,
            dispatcher: Some(dispatcher),
        }
    }

    // ── Inherent cache-hierarchy management (host + test surface) ──────────
    //
    // These delegate verbatim to the reused `GpuCache` / `HiddenLru` / `Dedup`
    // so the eviction/dedup invariants stay identical to the tile pipeline.

    /// Directly populate the hot cache with an asset (e.g. decoded locally or
    /// re-parented from an ancestor). Bypasses the network path.
    pub fn insert(&self, key: K, bytes: Vec<u8>) {
        self.inner.cache.lock().unwrap().insert(key, bytes);
    }

    /// Run FIFO eviction on the hot cache. The three invariants (base-layer
    /// exemption / live deferral / termination) are enforced by [`GpuCache`].
    /// `is_live` corresponds to `mgr.tile_entities.contains_key(&old)` (L1487).
    pub fn evict<F>(&self, is_live: F) -> EvictionResult
    where
        F: Fn(&K) -> bool,
    {
        let res = self.inner.cache.lock().unwrap().evict(is_live);
        self.inner.stats.evicted.fetch_add(res.evicted, Ordering::Relaxed);
        res
    }

    /// Mark a key as backed (or not) by a live reference (invariant 2 set).
    pub fn set_live(&self, key: K, is_live: bool) {
        self.inner.cache.lock().unwrap().set_live(key, is_live);
    }

    /// Demote a cached asset to the warm tier (hidden LRU fallback).
    pub fn hide(&self, key: K) {
        self.inner.hidden.lock().unwrap().hide(key);
    }

    /// Promote a warm asset back to hot. Returns true if it was hidden.
    pub fn show(&self, key: &K) -> bool {
        self.inner.hidden.lock().unwrap().show(key)
    }

    /// Advance the warm-tier LRU frame tick (call once per frame).
    pub fn begin_frame(&self) {
        self.inner.hidden.lock().unwrap().advance_frame();
    }

    /// Read cached bytes (hot or warm), if present.
    pub fn cached_bytes(&self, key: &K) -> Option<Vec<u8>> {
        self.inner.cache.lock().unwrap().get(key).cloned()
    }

    /// Number of entries in the hot/warm cache store.
    pub fn cache_len(&self) -> usize {
        self.inner.cache.lock().unwrap().len()
    }

    /// True if a stream request for `key` is currently in-flight (dedup set).
    pub fn is_in_flight(&self, key: &K) -> bool {
        self.inner.dedup.contains(key)
    }

    /// Read-only view of the hot-cache FIFO order (oldest first). Mirrors
    /// `GpuCache::order()` for invariant assertions.
    pub fn evict_order_len(&self) -> usize {
        self.inner.cache.lock().unwrap().order().len()
    }

    /// Test seam (M2): arm the dispatcher to panic on its next non-empty
    /// result batch, exercising the `catch_unwind` + waiter-drain recovery.
    #[cfg(test)]
    fn arm_dispatcher_panic(&self) {
        self.inner.test_panic.store(true, Ordering::Relaxed);
    }
}

impl<K> ResourceBackend<K> for PipelineResourceBackend<K>
where
    K: Hash + Eq + Copy + Send + 'static,
{
    fn request_stream<'a>(
        &'a self,
        key: K,
        priority: f64,
    ) -> Pin<Box<dyn Future<Output = PortResult<Vec<u8>>> + Send + 'a>> {
        let inner = Arc::clone(&self.inner);
        // The returned future is actually `'static` (it owns an `Arc<Inner>`),
        // which trivially satisfies the `'a` bound from `&'a self`.
        Box::pin(async move {
            // Atomic cold-intake: hold `intake` across the cache-miss check AND
            // the dedup mark + submit, so the dispatcher's completion (which
            // also takes `intake`) can never interleave and cause a redundant
            // second fetch for the same cold key.
            let intake = inner.intake.lock().unwrap();

            // 1. Cache hit (hot or warm) → resolve immediately, promote warm.
            let cached = inner.cache.lock().unwrap().get(&key).cloned();
            if let Some(bytes) = cached {
                drop(intake);
                // Warm → hot promotion (a warm asset is still cached).
                inner.hidden.lock().unwrap().show(&key);
                return Ok(bytes);
            }

            // 2. Cold miss → register a waiter, dedup, submit to the pool.
            let (tx, rx) = mpsc::channel();
            let is_new = inner.dedup.insert(key);
            inner
                .waiters
                .lock()
                .unwrap()
                .entry(key)
                .or_default()
                .push(tx);
            if is_new {
                let url = (inner.url_builder)(&key);
                inner.pool.extend_wanted(&[key]);
                inner.pool.submit(Job { key, url, priority });
            } else {
                // Already in-flight: share the single network fetch.
                inner.stats.deduped.fetch_add(1, Ordering::Relaxed);
            }
            drop(intake);

            // 3. Block the calling thread until the dispatcher delivers.
            //    std-only (mpsc), no tokio, no executor. See module docs:
            //    drive from an IO/worker context, not the frame thread.
            match rx.recv() {
                Ok(res) => res,
                Err(_) => Err(PortError::Cancelled),
            }
        })
    }

    fn cancel(&self, key: &K) {
        // Clear the in-flight slot so the asset can be re-requested.
        self.inner.dedup.remove(key);
        // Drop the key from the pool's wanted set so the worker gate (L2161)
        // produces an Aborted result instead of fetching.
        self.inner.pool.remove_wanted(key);
        // Unblock any waiters immediately with Cancelled.
        if let Some(senders) = self.inner.waiters.lock().unwrap().remove(key) {
            for s in senders {
                let _ = s.send(Err(PortError::Cancelled));
            }
        }
    }

    fn cache_tier(&self, key: &K) -> CacheTier {
        // Cold: not in the cache store at all.
        if !self.inner.cache.lock().unwrap().contains_key(key) {
            return CacheTier::Cold;
        }
        // Warm: cached but tracked as hidden (LRU fallback).
        if self.inner.hidden.lock().unwrap().is_hidden(key) {
            CacheTier::Warm
        } else {
            CacheTier::Hot
        }
    }

    fn stats(&self) -> ResourceStats {
        let cache_len = self.inner.cache.lock().unwrap().len() as u32;
        let hidden_len = self.inner.hidden.lock().unwrap().len() as u32;
        // Warm = hidden entries that are still cached; hot = the remainder.
        let warm_entries = hidden_len.min(cache_len);
        let hot_entries = cache_len.saturating_sub(warm_entries);
        ResourceStats {
            hot_entries,
            warm_entries,
            in_flight: self.inner.dedup.len() as u32,
            streamed: self.inner.stats.streamed.load(Ordering::Relaxed),
            deduped: self.inner.stats.deduped.load(Ordering::Relaxed),
            evicted: self.inner.stats.evicted.load(Ordering::Relaxed),
        }
    }

    fn name(&self) -> &str {
        &self.inner.name
    }

    fn is_available(&self) -> bool {
        !self.inner.shutdown.load(Ordering::Relaxed)
    }
}

impl<K> Drop for PipelineResourceBackend<K>
where
    K: Hash + Eq + Copy + Send + 'static,
{
    fn drop(&mut self) {
        // Stop the dispatcher, then let the `Arc<Inner>` unwind: dropping the
        // `WorkerPool` closes its job channel and the (detached) workers exit.
        self.inner.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.dispatcher.take() {
            let _ = handle.join();
        }
    }
}

/// Runs [`dispatcher_loop`] under `catch_unwind` and, on ANY exit (panic or
/// normal shutdown), drains + fails every still-registered waiter.
///
/// M2 review fix: if the dispatcher panicked (e.g. a poisoned lock or an
/// internal bug), the waiters' `Sender`s would otherwise stay alive inside
/// `Arc<Inner>` forever, leaving every in-flight `request_stream` caller
/// blocked on `rx.recv()` with no wake-up and no supervisor. Catching the
/// unwind and explicitly sending each waiter an error unblocks all callers
/// deterministically (`recv` yields the error, never a hang). On panic the
/// shutdown flag is also set so `is_available()` reports the backend down.
fn run_dispatcher_guarded<K>(inner: Arc<Inner<K>>)
where
    K: Hash + Eq + Copy + Send + 'static,
{
    let cleanup = Arc::clone(&inner);
    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        dispatcher_loop(inner)
    }))
    .is_err();

    if panicked {
        // Mark the backend unavailable so callers detect the dead dispatcher
        // instead of queueing more work onto it.
        cleanup.shutdown.store(true, Ordering::Relaxed);
    }

    // Drain every still-registered waiter and fail it so no caller stays
    // blocked on `rx.recv()` after the dispatcher exits. Recover from a
    // possibly-poisoned `waiters` lock (the panic may have struck while the
    // dispatcher held it) via `into_inner`.
    let mut waiters = cleanup
        .waiters
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    for (_key, senders) in waiters.drain() {
        for s in senders {
            let _ = s.send(Err(PortError::Network(
                "resource dispatcher exited before delivering this stream".into(),
            )));
        }
    }
}

/// Background dispatcher: drains worker-pool results, populates the hot cache,
/// and routes each result to its registered waiters.
///
/// This is the single consumer of the pool's result queue (no draining races).
/// It polls every 1 ms when idle and exits promptly once `shutdown` is set.
fn dispatcher_loop<K>(inner: Arc<Inner<K>>)
where
    K: Hash + Eq + Copy + Send + 'static,
{
    loop {
        if inner.shutdown.load(Ordering::Relaxed) {
            break;
        }
        let results = inner.pool.drain_results(64);
        if results.is_empty() {
            thread::sleep(Duration::from_millis(1));
            continue;
        }
        // M2 test seam (cfg(test) only): deterministically panic the dispatcher
        // once a result is in hand (so a waiter is registered for it) to
        // exercise the catch_unwind + waiter-drain recovery in
        // `run_dispatcher_guarded`.
        #[cfg(test)]
        if inner.test_panic.load(Ordering::Relaxed) {
            panic!("test-injected dispatcher panic (M2 recovery path)");
        }
        for r in results {
            // Completion is atomic w.r.t. cold-intake: hold `intake` across the
            // in-flight clear + cache insert + WAITER ROUTING, so a concurrent
            // `request_stream` for the same key can never register a NEW waiter
            // in the window between the cache write and the waiter drain.
            //
            // H3 review fix: pre-fix the `intake` guard was dropped before
            // `waiters.remove`, so for a Failed/Aborted outcome (cache NOT
            // written) a caller entering in that window registered a fresh
            // waiter that this stale result then swept up — the new caller got
            // the old failure and its own submitted job's result was orphaned.
            // `mpsc::Sender::send` is non-blocking and acquires no lock, so
            // holding `intake` across the fan-out is safe. Lock order stays
            // intake → {dedup, cache, waiters}; only one inner lock is held at
            // a time (each is a temporary dropped at statement end).
            let _intake = inner.intake.lock().unwrap();
            // The stream completed (or aborted) — free the in-flight slot.
            inner.dedup.remove(&r.key);
            // Successful streams land in the hot cache (reused GpuCache, so the
            // FIFO order / base-layer lock / live deferral all apply).
            if let JobOutcome::Success(ref bytes) = r.outcome {
                inner.cache.lock().unwrap().insert(r.key, bytes.clone());
                inner.stats.streamed.fetch_add(1, Ordering::Relaxed);
            }
            // Route the outcome to every waiter for this key — STILL under
            // `intake`, making {dedup.remove, cache.insert, waiter routing}
            // atomic for this key.
            if let Some(senders) = inner.waiters.lock().unwrap().remove(&r.key) {
                for s in senders {
                    let msg = match &r.outcome {
                        JobOutcome::Success(b) => Ok(b.clone()),
                        JobOutcome::Aborted => Err(PortError::Cancelled),
                        JobOutcome::Failed => {
                            Err(PortError::Network("resource stream retries exhausted".into()))
                        }
                        JobOutcome::Placeholder => {
                            Err(PortError::NotFound("no usable asset content".into()))
                        }
                    };
                    let _ = s.send(msg);
                }
            }
        }
    }
}

// `Pin` / `Future` are imported at the top of the module.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::FetchResult;
    use std::sync::atomic::AtomicUsize;

    type TileKey = (u32, u32, u32);

    fn zoom_of(k: &TileKey) -> u32 {
        k.2
    }

    fn url_of(k: &TileKey) -> String {
        format!("http://assets/{}/{}/{}", k.2, k.0, k.1)
    }

    /// Instant mock backend returning canned bytes per URL.
    struct MockNet {
        responses: Mutex<HashMap<String, FetchResult>>,
    }

    impl MockNet {
        fn new() -> Self {
            Self {
                responses: Mutex::new(HashMap::new()),
            }
        }
        fn add(&self, url: &str, r: FetchResult) {
            self.responses.lock().unwrap().insert(url.to_string(), r);
        }
    }

    impl NetworkBackend for MockNet {
        fn fetch(&self, url: &str) -> FetchResult {
            self.responses
                .lock()
                .unwrap()
                .get(url)
                .cloned()
                .unwrap_or(FetchResult::Transient("not mocked".into()))
        }
        fn name(&self) -> &str {
            "mock-resource"
        }
        fn timeout(&self) -> Duration {
            Duration::from_secs(1)
        }
    }

    fn fast_config(threads: usize) -> PoolConfig {
        PoolConfig {
            threads,
            max_attempts: 3,
            backoff_base: Duration::from_millis(5),
        }
    }

    fn make_backend(net: Arc<MockNet>, max_cache: usize) -> PipelineResourceBackend<TileKey> {
        PipelineResourceBackend::with_config(
            "test-resource-backend",
            net,
            Arc::new(url_of),
            zoom_of,
            BaseLayerGuard::new(),
            max_cache,
            fast_config(2),
        )
    }

    /// Drive a boxed future to completion with a `Waker::noop`-style single
    /// poll (the future blocks internally, so one poll always yields Ready).
    fn block_on<F: std::future::Future>(fut: F) -> F::Output {
        use std::task::{Context, Poll, Waker};
        let mut fut = Box::pin(fut);
        let mut cx = Context::from_waker(Waker::noop());
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(out) => out,
            Poll::Pending => panic!("resource future returned Pending (must block to Ready)"),
        }
    }

    #[test]
    fn cold_miss_streams_and_populates_hot_cache() {
        let net = Arc::new(MockNet::new());
        net.add("http://assets/5/1/2", FetchResult::Ok(vec![9, 8, 7]));
        let be = make_backend(net, 100);

        assert_eq!(be.cache_tier(&(1, 2, 5)), CacheTier::Cold);
        let bytes = block_on(be.request_stream((1, 2, 5), 1.0)).unwrap();
        assert_eq!(bytes, vec![9, 8, 7]);
        // The dispatcher inserted the streamed asset into the hot cache.
        assert_eq!(be.cache_tier(&(1, 2, 5)), CacheTier::Hot);
        assert_eq!(be.cached_bytes(&(1, 2, 5)), Some(vec![9, 8, 7]));
        assert_eq!(be.stats().streamed, 1);
    }

    #[test]
    fn cache_hit_resolves_without_network() {
        let net = Arc::new(MockNet::new()); // no responses → would fail if hit
        let be = make_backend(net, 100);
        be.insert((3, 3, 6), vec![1, 2, 3]);

        let bytes = block_on(be.request_stream((3, 3, 6), 1.0)).unwrap();
        assert_eq!(bytes, vec![1, 2, 3]);
        assert_eq!(be.cache_tier(&(3, 3, 6)), CacheTier::Hot);
    }

    #[test]
    fn warm_hit_promotes_to_hot() {
        let net = Arc::new(MockNet::new());
        let be = make_backend(net, 100);
        be.insert((4, 4, 7), vec![5, 5]);
        be.hide((4, 4, 7));
        assert_eq!(be.cache_tier(&(4, 4, 7)), CacheTier::Warm);

        let bytes = block_on(be.request_stream((4, 4, 7), 1.0)).unwrap();
        assert_eq!(bytes, vec![5, 5]);
        // Warm → hot promotion on access.
        assert_eq!(be.cache_tier(&(4, 4, 7)), CacheTier::Hot);
    }

    #[test]
    fn concurrent_cold_requests_dedup_to_one_fetch() {
        // A gated, counting backend proves the `Dedup` reuse deterministically:
        // the fetch blocks until released, so all N concurrent cold requests
        // reach the intake decision while the single job is still in-flight.
        // Exactly ONE caller wins `dedup.insert` and submits; the other N-1
        // join as waiters (counted by `deduped`). Hence exactly ONE fetch.
        struct GatedNet {
            count: AtomicUsize,
            release: AtomicBool,
        }
        impl NetworkBackend for GatedNet {
            fn fetch(&self, _url: &str) -> FetchResult {
                self.count.fetch_add(1, Ordering::SeqCst);
                // Hold the single in-flight job open until the test releases it.
                while !self.release.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(1));
                }
                FetchResult::Ok(vec![42])
            }
            fn name(&self) -> &str {
                "gated"
            }
            fn timeout(&self) -> Duration {
                Duration::from_secs(5)
            }
        }

        let net = Arc::new(GatedNet {
            count: AtomicUsize::new(0),
            release: AtomicBool::new(false),
        });
        let be = Arc::new(PipelineResourceBackend::with_config(
            "dedup-backend",
            Arc::clone(&net) as Arc<dyn NetworkBackend>,
            Arc::new(url_of),
            zoom_of,
            BaseLayerGuard::new(),
            100,
            fast_config(4),
        ));

        let key: TileKey = (1, 1, 5);
        let mut handles = Vec::new();
        for _ in 0..8 {
            let be = Arc::clone(&be);
            handles.push(thread::spawn(move || block_on(be.request_stream(key, 1.0))));
        }
        // Let all 8 callers reach the intake decision: one submits + blocks in
        // the gated fetch, the other 7 register as deduped waiters.
        thread::sleep(Duration::from_millis(150));
        assert_eq!(net.count.load(Ordering::SeqCst), 1, "exactly one fetch in-flight");
        assert_eq!(be.stats().deduped, 7, "7 concurrent callers deduped onto the one fetch");

        // Release the gate; the single result fans out to all 8 waiters.
        net.release.store(true, Ordering::SeqCst);
        for h in handles {
            let res = h.join().unwrap();
            assert_eq!(res.unwrap(), vec![42]);
        }
        assert_eq!(net.count.load(Ordering::SeqCst), 1, "dedup must collapse to one fetch");
        assert_eq!(be.cache_tier(&key), CacheTier::Hot);
    }

    #[test]
    fn cancel_unblocks_waiter_with_cancelled() {
        // A backend whose fetch always transient-fails keeps the job in-flight
        // long enough to cancel deterministically.
        struct SlowNet;
        impl NetworkBackend for SlowNet {
            fn fetch(&self, _url: &str) -> FetchResult {
                FetchResult::Transient("slow".into())
            }
            fn name(&self) -> &str {
                "slow"
            }
            fn timeout(&self) -> Duration {
                Duration::from_secs(1)
            }
        }
        let be = Arc::new(PipelineResourceBackend::with_config(
            "cancel-backend",
            Arc::new(SlowNet),
            Arc::new(url_of),
            zoom_of,
            BaseLayerGuard::new(),
            100,
            PoolConfig {
                threads: 1,
                max_attempts: 50,
                backoff_base: Duration::from_millis(20),
            },
        ));

        let key: TileKey = (2, 2, 8);
        let be2 = Arc::clone(&be);
        let handle = thread::spawn(move || block_on(be2.request_stream(key, 1.0)));
        // Give the worker time to pick up the job and start retrying.
        thread::sleep(Duration::from_millis(60));
        assert!(be.is_in_flight(&key));
        be.cancel(&key);
        let res = handle.join().unwrap();
        assert!(matches!(res, Err(PortError::Cancelled)));
        assert!(!be.is_in_flight(&key));
    }

    #[test]
    fn placeholder_maps_to_not_found() {
        let net = Arc::new(MockNet::new());
        net.add("http://assets/6/0/0", FetchResult::Ok(vec![])); // empty → placeholder
        let be = make_backend(net, 100);
        let res = block_on(be.request_stream((0, 0, 6), 1.0));
        assert!(matches!(res, Err(PortError::NotFound(_))));
        // Placeholder assets are NOT cached.
        assert_eq!(be.cache_tier(&(0, 0, 6)), CacheTier::Cold);
    }

    #[test]
    fn backend_is_dyn_compatible() {
        let net = Arc::new(MockNet::new());
        let be = make_backend(net, 100);
        let boxed: Box<dyn ResourceBackend<TileKey>> = Box::new(be);
        assert_eq!(boxed.name(), "test-resource-backend");
        assert!(boxed.is_available());
        assert_eq!(boxed.cache_tier(&(0, 0, 0)), CacheTier::Cold);
    }

    /// H3 review fix: the dispatcher routes waiters INSIDE the intake critical
    /// section, so a re-request for a key whose previous stream FAILED can
    /// never be swept up by the stale Failed result (pre-fix the waiter routing
    /// happened after `intake` was released, opening a window where a
    /// newly-registered waiter was handed the old failure while its own job's
    /// result was orphaned). Per key: (1) first stream fails deterministically
    /// (transient + `max_attempts=1` → Failed); (2) flip the backend to succeed;
    /// (3) re-request the SAME key and assert it receives its OWN fresh Ok
    /// bytes, not the stale Failed. A multi-key loop shakes out residual races.
    #[test]
    fn failed_stream_then_rerequest_gets_fresh_result_not_stale_failure() {
        struct FlipNet {
            ok: AtomicBool,
        }
        impl NetworkBackend for FlipNet {
            fn fetch(&self, _url: &str) -> FetchResult {
                if self.ok.load(Ordering::SeqCst) {
                    FetchResult::Ok(vec![7, 7, 7])
                } else {
                    FetchResult::Transient("fail-first".into())
                }
            }
            fn name(&self) -> &str {
                "flip"
            }
            fn timeout(&self) -> Duration {
                Duration::from_secs(1)
            }
        }

        let net = Arc::new(FlipNet { ok: AtomicBool::new(false) });
        // max_attempts=1 → a single Transient becomes Failed with no retry.
        let be = PipelineResourceBackend::with_config(
            "h3-backend",
            Arc::clone(&net) as Arc<dyn NetworkBackend>,
            Arc::new(url_of),
            zoom_of,
            BaseLayerGuard::new(),
            100,
            PoolConfig {
                threads: 2,
                max_attempts: 1,
                backoff_base: Duration::from_millis(1),
            },
        );

        for i in 0..20u32 {
            let key: TileKey = (i, i, 5);
            // Fail mode: first request for this key must surface Failed → Network.
            net.ok.store(false, Ordering::SeqCst);
            let r1 = block_on(be.request_stream(key, 1.0));
            assert!(
                matches!(r1, Err(PortError::Network(_))),
                "first stream must fail (transient, max_attempts=1); got {r1:?}"
            );
            // Success mode: re-request the SAME key → must get its OWN fresh
            // result, never the stale Failed from the previous job.
            net.ok.store(true, Ordering::SeqCst);
            let r2 = block_on(be.request_stream(key, 1.0));
            assert_eq!(
                r2.unwrap(),
                vec![7, 7, 7],
                "re-request after a failure must get its own fresh result, not the stale Failed"
            );
        }
    }

    /// M2 review fix: if the dispatcher thread dies (panic), every in-flight
    /// `request_stream` caller must be unblocked with an Err — never left
    /// blocked forever on `rx.recv()`. We arm the test-only panic seam, then
    /// issue a cold request; once its result reaches the dispatcher it panics,
    /// `run_dispatcher_guarded` catches the unwind and drains the waiter with a
    /// Network error, and marks the backend unavailable.
    #[test]
    fn dispatcher_panic_unblocks_waiters_with_error() {
        let net = Arc::new(MockNet::new());
        net.add("http://assets/7/3/3", FetchResult::Ok(vec![1, 2, 3]));
        let be = Arc::new(make_backend(net, 100));
        be.arm_dispatcher_panic();

        let key: TileKey = (3, 3, 7);
        let be2 = Arc::clone(&be);
        // Run on a thread: with the fix the caller is unblocked promptly; a
        // regression would hang here (observable as a stuck test).
        let handle = thread::spawn(move || block_on(be2.request_stream(key, 1.0)));
        let res = handle.join().unwrap();
        assert!(
            matches!(res, Err(PortError::Network(_))),
            "dispatcher death must surface as an Err, not a permanent block; got {res:?}"
        );
        // The backend now reports unavailable (shutdown set on dispatcher panic).
        assert!(!be.is_available(), "dead dispatcher must mark the backend unavailable");
    }
}
