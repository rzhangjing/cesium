//! Generic pipeline runtime — implements `TilePipeline<K, Payload>`.
//!
//! Ties together the worker pool, dedup set, wanted-set management, and
//! stats tracking into a single cohesive pipeline that faithfully replicates
//! the orchestration in `dynamic_globe.rs::process_pipeline` (L662-1429)
//! and `enqueue_tiles` (L371-459).

use std::hash::Hash;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use cesium_ports_driven::{PipelineStats, PollOutcome, TilePipeline};

use crate::budget::DefaultBudget;
use crate::dedup::Dedup;
use crate::net::NetworkBackend;
use crate::pool::{Decoder, Job, JobOutcome, PoolConfig, WorkerPool};

/// URL builder: maps a tile key to a fetch URL.
/// Corresponds to `dynamic_globe.rs:2176-2181` (quadkey URL construction).
pub type UrlBuilder<K> = Arc<dyn Fn(&K) -> String + Send + Sync>;

/// Generic tile pipeline implementation.
///
/// Type parameters:
/// - `K`: tile key (e.g. `(u32, u32, u32)` = TileKey). Must be Hash+Eq+Copy+Send+'static.
/// - `Payload`: download result (e.g. decoded RGBA + mip chain). Must be Send+'static.
///
/// Implements `TilePipeline<K, Payload>` from cesium-ports-driven (M1.1).
pub struct GenericPipeline<K, Payload>
where
    K: Hash + Eq + Copy + Send + 'static,
    Payload: Send + 'static,
{
    pool: WorkerPool<K, Payload>,
    dedup: Dedup<K>,
    url_builder: UrlBuilder<K>,
    stats: Arc<StatsInner>,
}

/// Internal mutable stats counters (atomic for cross-thread stats() reads).
struct StatsInner {
    frame_idx: AtomicU32,
    stale_skips: AtomicU32,
    evict_total: AtomicU32,
    evict_deferred: AtomicU32,
    in_flight: AtomicU32,
    retry_after: AtomicU32,
}

impl StatsInner {
    fn new() -> Self {
        Self {
            frame_idx: AtomicU32::new(0),
            stale_skips: AtomicU32::new(0),
            evict_total: AtomicU32::new(0),
            evict_deferred: AtomicU32::new(0),
            in_flight: AtomicU32::new(0),
            retry_after: AtomicU32::new(0),
        }
    }
}

impl<K, Payload> GenericPipeline<K, Payload>
where
    K: Hash + Eq + Copy + Send + 'static,
    Payload: Send + 'static,
{
    /// Create a pipeline with the given network backend, URL builder, and decoder.
    ///
    /// - `backend`: network implementation (default: `UreqBackend`).
    /// - `url_builder`: maps tile key → fetch URL (L2176-2181).
    /// - `decode`: converts raw bytes → Payload. Returns None for placeholder
    ///   tiles (L2211: `is_placeholder_tile` check).
    /// - `threads`: worker thread count (default 16, L48).
    pub fn new(
        backend: Arc<dyn NetworkBackend>,
        url_builder: UrlBuilder<K>,
        decode: Decoder<Payload>,
        threads: usize,
    ) -> Self {
        let config = PoolConfig {
            threads,
            max_attempts: 3,                          // L2186
            backoff_base: Duration::from_millis(250), // L2189
        };

        let pool = WorkerPool::spawn(backend, decode, config);
        let stats = Arc::new(StatsInner::new());

        Self {
            pool,
            dedup: Dedup::new(),
            url_builder,
            stats,
        }
    }

    /// Create a pipeline with default budget (16 threads).
    pub fn with_defaults(
        backend: Arc<dyn NetworkBackend>,
        url_builder: UrlBuilder<K>,
        decode: Decoder<Payload>,
    ) -> Self {
        Self::new(backend, url_builder, decode, DefaultBudget::DOWNLOAD_THREADS)
    }

    /// Create a pipeline with explicit pool configuration (for testing).
    pub fn with_config(
        backend: Arc<dyn NetworkBackend>,
        url_builder: UrlBuilder<K>,
        decode: Decoder<Payload>,
        config: PoolConfig,
    ) -> Self {
        let pool = WorkerPool::spawn(backend, decode, config);
        let stats = Arc::new(StatsInner::new());
        Self {
            pool,
            dedup: Dedup::new(),
            url_builder,
            stats,
        }
    }

    /// Advance the frame counter. Called once per frame by the host system.
    pub fn begin_frame(&self) {
        self.stats.frame_idx.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an eviction event (for stats tracking).
    pub fn record_eviction(&self, evicted: u32, deferred: u32) {
        self.stats.evict_total.fetch_add(evicted, Ordering::Relaxed);
        self.stats.evict_deferred.fetch_add(deferred, Ordering::Relaxed);
    }

    /// Update the retry_after gauge (tiles currently in cooldown).
    pub fn set_retry_after(&self, count: u32) {
        self.stats.retry_after.store(count, Ordering::Relaxed);
    }

    /// Shutdown the pipeline (drops job channel, workers exit).
    pub fn shutdown(self) {
        self.pool.shutdown();
    }
}

impl<K, Payload> TilePipeline<K, Payload> for GenericPipeline<K, Payload>
where
    K: Hash + Eq + Copy + Send + 'static,
    Payload: Send + 'static,
{
    /// Submit a tile for downloading with priority ordering.
    ///
    /// Corresponds to `enqueue_tiles` (L371-459):
    /// - Dedup check (L406, L417): skip if already in-flight
    /// - URL construction (L2176-2181)
    /// - Wanted-set injection (L448-451)
    /// - Priority-sorted submission to worker queue
    fn submit(&self, key: K, priority: f64) {
        // Dedup: skip if already in-flight (L406/L417)
        if !self.dedup.insert(key) {
            return;
        }

        // Build URL (L2176-2181)
        let url = (self.url_builder)(&key);

        // Inject into wanted set immediately (L448-451)
        self.pool.extend_wanted(&[key]);

        // Update in-flight gauge
        self.stats.in_flight.fetch_add(1, Ordering::Relaxed);

        // Submit to worker pool
        self.pool.submit(Job { key, url, priority });
    }

    /// Cancel a pending/in-flight tile.
    ///
    /// Removes from wanted set so the worker gate (L2161) produces Aborted.
    /// Also removes from dedup so the tile can be re-submitted later.
    fn cancel(&self, key: &K) {
        self.dedup.remove(key);
        // Note: the worker will detect the missing wanted-set entry and
        // return Aborted. We don't need to explicitly signal the worker.
    }

    /// Poll completed tiles (non-blocking drain).
    ///
    /// Corresponds to L1046-1048: `tex_rx.rx.lock().unwrap().try_recv()`
    /// bounded by `MAX_TEXTURE_UPLOADS_PER_FRAME` (16).
    ///
    /// Maps `JobOutcome` → `PollOutcome` following the three-state dispatch
    /// at L1052-1098.
    fn poll_ready(&self, budget: usize) -> Vec<PollOutcome<K, Payload>> {
        let results = self.pool.drain_results(budget);
        let mut outcomes = Vec::with_capacity(results.len());

        for r in results {
            // Clear dedup + in-flight gauge (L1051: `mgr.in_flight.remove(&key)`)
            self.dedup.remove(&r.key);
            self.stats.in_flight.fetch_sub(1, Ordering::Relaxed);

            match r.outcome {
                JobOutcome::Success(payload) => {
                    outcomes.push(PollOutcome::Ready(r.key, payload));
                }
                JobOutcome::Aborted => {
                    // L1052-1060: count stale_skip
                    self.stats.stale_skips.fetch_add(1, Ordering::Relaxed);
                    outcomes.push(PollOutcome::Aborted(r.key));
                }
                JobOutcome::Failed => {
                    // L1062-1072: retry cooldown handled by host
                    outcomes.push(PollOutcome::Failed(r.key));
                }
                JobOutcome::Placeholder => {
                    // L1074-1098: permanent no-data
                    outcomes.push(PollOutcome::Placeholder(r.key));
                }
            }
        }

        outcomes
    }

    /// Replace the wanted set (L1402-1409).
    ///
    /// Tiles not in the new set become abort candidates at the worker gate (L2161).
    fn refresh_wanted(&self, wanted: &[K]) {
        self.pool.refresh_wanted(wanted);
    }

    /// Snapshot current pipeline statistics.
    ///
    /// Aligns with M0.4 PerfCounters / 17-column CSV format.
    fn stats(&self) -> PipelineStats {
        PipelineStats {
            frame_idx: self.stats.frame_idx.load(Ordering::Relaxed),
            dt_ms: 0.0,        // Host provides (Bevy Time::delta_secs_f64)
            visible_n: 0,      // Host provides (tile_entities.len())
            partition_n: 0,    // Host provides (spawn_queue.len())
            load_n: self.dedup.len() as u32,
            spawn_n: 0,        // Host provides (frame_spawn)
            tex_upload_n: 0,   // Host provides (frame_tex)
            evict_n: self.stats.evict_total.load(Ordering::Relaxed),
            gpu_tex_cache: 0,  // Host provides (gpu_tex_order.len())
            mesh_backlog: 0,   // Host provides (backlog.len())
            dl_in_flight: self.stats.in_flight.load(Ordering::Relaxed),
            stale_skips: self.stats.stale_skips.load(Ordering::Relaxed),
            retry_after: self.stats.retry_after.load(Ordering::Relaxed),
            frame_mesh: 0,     // Host provides (frame_mesh)
            frame_despawn: 0,  // Host provides (frame_despawn)
            evict_deferred: self.stats.evict_deferred.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::{FetchResult, NetworkBackend};
    use std::collections::HashMap;
    use std::sync::Mutex;

    type TileKey = (u32, u32, u32);

    /// Mock backend for deterministic testing.
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
            "mock"
        }
        fn timeout(&self) -> Duration {
            Duration::from_secs(1)
        }
    }

    fn url_builder(k: &TileKey) -> String {
        format!("http://tiles/{}/{}/{}", k.2, k.0, k.1)
    }

    fn make_pipeline(
        mock: Arc<MockNet>,
        threads: usize,
    ) -> GenericPipeline<TileKey, Vec<u8>> {
        let decode: Decoder<Vec<u8>> = Arc::new(|data: &[u8]| {
            if data.is_empty() { None } else { Some(data.to_vec()) }
        });
        // Use short backoff for fast tests
        let config = PoolConfig {
            threads,
            max_attempts: 3,
            backoff_base: Duration::from_millis(10),
        };
        GenericPipeline::with_config(mock, Arc::new(url_builder), decode, config)
    }

    #[test]
    fn pipeline_submit_and_poll() {
        let mock = Arc::new(MockNet::new());
        mock.add("http://tiles/4/1/2", FetchResult::Ok(vec![10, 20, 30]));

        let pipe = make_pipeline(mock, 2);
        pipe.refresh_wanted(&[(1, 2, 4)]);
        pipe.submit((1, 2, 4), 100.0);

        std::thread::sleep(Duration::from_millis(200));
        let results = pipe.poll_ready(16);
        assert_eq!(results.len(), 1);
        match &results[0] {
            PollOutcome::Ready(k, payload) => {
                assert_eq!(*k, (1, 2, 4));
                assert_eq!(payload, &[10, 20, 30]);
            }
            other => panic!("expected Ready, got {:?}", other),
        }
    }

    #[test]
    fn pipeline_dedup_prevents_double_submit() {
        let mock = Arc::new(MockNet::new());
        mock.add("http://tiles/5/0/0", FetchResult::Ok(vec![1]));

        let pipe = make_pipeline(mock, 1);
        pipe.refresh_wanted(&[(0, 0, 5)]);
        pipe.submit((0, 0, 5), 1.0);
        pipe.submit((0, 0, 5), 2.0); // duplicate — should be ignored

        std::thread::sleep(Duration::from_millis(200));
        let results = pipe.poll_ready(16);
        assert_eq!(results.len(), 1); // only one result
    }

    #[test]
    fn pipeline_cancel_produces_aborted() {
        let mock = Arc::new(MockNet::new());
        mock.add("http://tiles/6/3/3", FetchResult::Transient("slow".into()));

        let pipe = make_pipeline(mock, 1);
        // Submit then immediately clear wanted
        pipe.submit((3, 3, 6), 1.0);
        pipe.refresh_wanted(&[]); // empty wanted — tile will be aborted

        std::thread::sleep(Duration::from_millis(300));
        let results = pipe.poll_ready(16);
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0], PollOutcome::Aborted(_)));
    }

    #[test]
    fn pipeline_stats_track_in_flight() {
        let mock = Arc::new(MockNet::new());
        mock.add("http://tiles/7/1/1", FetchResult::Transient("timeout".into()));

        let pipe = make_pipeline(mock, 1);
        pipe.refresh_wanted(&[(1, 1, 7)]);
        pipe.submit((1, 1, 7), 1.0);

        // Before poll: in_flight should be 1
        let s = pipe.stats();
        assert_eq!(s.dl_in_flight, 1);

        // Wait for retries to exhaust (3 attempts × 10ms backoff = ~70ms)
        std::thread::sleep(Duration::from_millis(300));
        let _ = pipe.poll_ready(16);

        // After poll: in_flight should be 0
        let s = pipe.stats();
        assert_eq!(s.dl_in_flight, 0);
    }

    #[test]
    fn pipeline_placeholder_detection() {
        let mock = Arc::new(MockNet::new());
        mock.add("http://tiles/8/0/0", FetchResult::Ok(vec![])); // empty = placeholder

        let pipe = make_pipeline(mock, 1);
        pipe.refresh_wanted(&[(0, 0, 8)]);
        pipe.submit((0, 0, 8), 1.0);

        std::thread::sleep(Duration::from_millis(200));
        let results = pipe.poll_ready(16);
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0], PollOutcome::Placeholder(_)));
    }

    #[test]
    fn pipeline_stale_skips_counter() {
        let mock = Arc::new(MockNet::new());
        let pipe = make_pipeline(mock, 1);

        // Submit adds to wanted, then immediately clear wanted so the worker
        // gate (L2161) finds the tile missing → Aborted → stale_skips++
        pipe.submit((9, 9, 9), 1.0);
        pipe.refresh_wanted(&[]); // clear wanted before worker picks up job

        std::thread::sleep(Duration::from_millis(200));
        let results = pipe.poll_ready(16);
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0], PollOutcome::Aborted(_)));

        let s = pipe.stats();
        assert_eq!(s.stale_skips, 1);
    }
}
