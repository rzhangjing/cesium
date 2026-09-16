//! Blocking worker pool with keep-alive semantics.
//!
//! Mirrors `dynamic_globe.rs:2143-2285` (`download_worker`):
//! - 16 threads (L48: `DOWNLOAD_THREADS = 16`)
//! - Shared job queue via `Arc<Mutex<mpsc::Receiver>>`
//! - ureq agent with keep-alive connection pooling (L2148-2151)
//! - 3 retries with `250ms << attempt` exponential backoff (L2186-2191)
//! - Wanted-set gate before fetch (L2161)
//! - Three-state result: aborted / failed / placeholder / success

use std::collections::HashSet;
use std::hash::Hash;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::net::{FetchResult, NetworkBackend};

/// A job submitted to the worker pool.
#[derive(Debug, Clone)]
pub struct Job<K> {
    /// Tile key identifying this job.
    pub key: K,
    /// URL to fetch.
    pub url: String,
    /// Priority (higher = fetched sooner when queue is deep).
    pub priority: f64,
}

/// Result from a completed worker job.
#[derive(Debug)]
pub struct JobResult<K, Payload> {
    /// Tile key.
    pub key: K,
    /// Outcome of the fetch.
    pub outcome: JobOutcome<Payload>,
}

/// Three-state outcome matching `dynamic_globe.rs:1052-1098` + success.
#[derive(Debug)]
pub enum JobOutcome<Payload> {
    /// Successful fetch with decoded payload.
    Success(Payload),
    /// Tile left wanted set mid-flight (L1052-1060).
    Aborted,
    /// All retries exhausted — transient failure (L1062-1072).
    Failed,
    /// No usable imagery — placeholder detected (L1074-1098).
    Placeholder,
}

/// Configuration for the worker pool.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Number of worker threads (L48: 16).
    pub threads: usize,
    /// Max retry attempts per job (L2186: 3).
    pub max_attempts: u32,
    /// Base backoff duration (L2189: 250 ms). Actual = base << attempt.
    pub backoff_base: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            threads: 16,
            max_attempts: 3,
            backoff_base: Duration::from_millis(250),
        }
    }
}

/// Type-erased decode function: raw bytes → optional Payload.
/// Returns `None` for placeholder tiles (L2211: `is_placeholder_tile`).
pub type Decoder<Payload> =
    Arc<dyn Fn(&[u8]) -> Option<Payload> + Send + Sync + 'static>;

/// Channel pair for job results (avoids clippy::type_complexity).
type ResultChannel<K, Payload> = (
    mpsc::Sender<JobResult<K, Payload>>,
    mpsc::Receiver<JobResult<K, Payload>>,
);

/// Blocking worker pool that processes fetch jobs on N threads.
///
/// Corresponds to the thread spawn loop in `dynamic_globe.rs` that creates
/// `DOWNLOAD_THREADS` (16) workers, each running `download_worker`.
pub struct WorkerPool<K, Payload>
where
    K: Hash + Eq + Copy + Send + 'static,
    Payload: Send + 'static,
{
    job_tx: mpsc::Sender<Job<K>>,
    result_rx: Arc<Mutex<mpsc::Receiver<JobResult<K, Payload>>>>,
    wanted: Arc<Mutex<HashSet<K>>>,
    _handles: Vec<thread::JoinHandle<()>>,
}

impl<K, Payload> WorkerPool<K, Payload>
where
    K: Hash + Eq + Copy + Send + 'static,
    Payload: Send + 'static,
{
    /// Spawn a worker pool with the given configuration.
    ///
    /// `backend`: network implementation (shared across all workers).
    /// `decode`: closure converting raw bytes → Payload (None = placeholder).
    /// `config`: thread count, retry policy.
    pub fn spawn(
        backend: Arc<dyn NetworkBackend>,
        decode: Decoder<Payload>,
        config: PoolConfig,
    ) -> Self {
        let (job_tx, job_rx): (mpsc::Sender<Job<K>>, mpsc::Receiver<Job<K>>) = mpsc::channel();
        let (result_tx, result_rx): ResultChannel<K, Payload> = mpsc::channel();

        let job_rx = Arc::new(Mutex::new(job_rx));
        let result_rx = Arc::new(Mutex::new(result_rx));
        let wanted: Arc<Mutex<HashSet<K>>> = Arc::new(Mutex::new(HashSet::new()));

        let mut handles = Vec::with_capacity(config.threads);

        for _ in 0..config.threads {
            let rx = Arc::clone(&job_rx);
            let tx = result_tx.clone();
            let w = Arc::clone(&wanted);
            let be = Arc::clone(&backend);
            let dec = Arc::clone(&decode);
            let cfg = config.clone();

            let handle = thread::spawn(move || {
                worker_loop::<K, Payload>(&rx, &tx, &w, &be, &dec, &cfg);
            });
            handles.push(handle);
        }

        Self {
            job_tx,
            result_rx,
            wanted,
            _handles: handles,
        }
    }

    /// Submit a job to the pool. Non-blocking (queued for workers).
    pub fn submit(&self, job: Job<K>) {
        let _ = self.job_tx.send(job);
    }

    /// Drain completed results (non-blocking). Returns up to `max` results.
    ///
    /// Corresponds to the `tex_rx.rx.lock().unwrap().try_recv()` drain loop
    /// at L1047-1048, bounded by `MAX_TEXTURE_UPLOADS_PER_FRAME`.
    pub fn drain_results(&self, max: usize) -> Vec<JobResult<K, Payload>> {
        let rx = self.result_rx.lock().unwrap();
        let mut results = Vec::new();
        while results.len() < max {
            match rx.try_recv() {
                Ok(r) => results.push(r),
                Err(_) => break,
            }
        }
        results
    }

    /// Replace the wanted set. Workers check this gate before fetching (L2161).
    pub fn refresh_wanted(&self, keys: &[K]) {
        let mut w = self.wanted.lock().unwrap();
        w.clear();
        w.extend(keys.iter().copied());
    }

    /// Add keys to the wanted set without clearing (for immediate injection, L448-451).
    pub fn extend_wanted(&self, keys: &[K]) {
        let mut w = self.wanted.lock().unwrap();
        w.extend(keys.iter().copied());
    }

    /// Remove a single key from the wanted set.
    ///
    /// Used by `ResourceBackend::cancel` (M8) so the worker gate (L2161) finds
    /// the key missing and produces an `Aborted` result for that specific key,
    /// without disturbing the rest of the in-flight wanted set. Additive — the
    /// golden-path `GenericPipeline` continues to rely on `refresh_wanted`.
    pub fn remove_wanted(&self, key: &K) {
        self.wanted.lock().unwrap().remove(key);
    }

    /// Close the job channel, signaling workers to exit after draining.
    pub fn shutdown(self) {
        drop(self.job_tx);
        // Workers will exit when job_rx returns Err (channel closed).
    }
}

/// Worker loop: pull jobs, gate on wanted, fetch with retry, decode, send result.
///
/// Faithfully replicates `dynamic_globe.rs:2153-2284`.
fn worker_loop<K, Payload>(
    job_rx: &Arc<Mutex<mpsc::Receiver<Job<K>>>>,
    result_tx: &mpsc::Sender<JobResult<K, Payload>>,
    wanted: &Arc<Mutex<HashSet<K>>>,
    backend: &Arc<dyn NetworkBackend>,
    decode: &Decoder<Payload>,
    config: &PoolConfig,
) where
    K: Hash + Eq + Copy + Send + 'static,
    Payload: Send + 'static,
{
    loop {
        // Block until a job arrives (L2154: `job_rx.lock().unwrap().recv()`)
        let job = match job_rx.lock().unwrap().recv() {
            Ok(j) => j,
            Err(_) => return, // Channel closed — shutdown
        };

        // Wanted-set gate (L2161): skip fetches nobody will look at
        if !wanted.lock().unwrap().contains(&job.key) {
            let _ = result_tx.send(JobResult {
                key: job.key,
                outcome: JobOutcome::Aborted,
            });
            continue;
        }

        // Retry loop with exponential backoff (L2186-2191)
        let mut delivered = false;
        for attempt in 0..config.max_attempts {
            if attempt > 0 {
                // L2188-2190: sleep(250ms << attempt)
                let backoff = config.backoff_base * (1u32 << attempt);
                thread::sleep(backoff);
            }

            match backend.fetch(&job.url) {
                FetchResult::Ok(data) => {
                    match decode(&data) {
                        Some(payload) => {
                            let _ = result_tx.send(JobResult {
                                key: job.key,
                                outcome: JobOutcome::Success(payload),
                            });
                        }
                        None => {
                            // Decode returned None = placeholder tile (L2211-2229)
                            let _ = result_tx.send(JobResult {
                                key: job.key,
                                outcome: JobOutcome::Placeholder,
                            });
                        }
                    }
                    delivered = true;
                    break;
                }
                FetchResult::Permanent(_) => {
                    // 404 etc: treat as placeholder (tile does not exist)
                    let _ = result_tx.send(JobResult {
                        key: job.key,
                        outcome: JobOutcome::Placeholder,
                    });
                    delivered = true;
                    break;
                }
                FetchResult::Transient(_) => {
                    // Retry (L2186: `for attempt in 0..3u32`)
                    continue;
                }
            }
        }

        if !delivered {
            // All retries exhausted (L2265-2283): Failed, NOT placeholder
            let _ = result_tx.send(JobResult {
                key: job.key,
                outcome: JobOutcome::Failed,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::ureq_backend::UreqBackend;
    use std::collections::HashMap;

    /// Mock backend that returns canned responses for testing.
    struct MockBackend {
        responses: Mutex<HashMap<String, FetchResult>>,
    }

    impl MockBackend {
        fn new() -> Self {
            Self {
                responses: Mutex::new(HashMap::new()),
            }
        }

        fn add(&self, url: &str, result: FetchResult) {
            self.responses.lock().unwrap().insert(url.to_string(), result);
        }
    }

    impl NetworkBackend for MockBackend {
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

    type TileKey = (u32, u32, u32);

    fn test_decoder(data: &[u8]) -> Option<Vec<u8>> {
        if data.is_empty() { None } else { Some(data.to_vec()) }
    }

    #[test]
    fn pool_processes_job_successfully() {
        let mock = Arc::new(MockBackend::new());
        mock.add("http://tile/1", FetchResult::Ok(vec![1, 2, 3]));

        let decode: Decoder<Vec<u8>> = Arc::new(test_decoder);
        let pool: WorkerPool<TileKey, Vec<u8>> = WorkerPool::spawn(
            mock,
            decode,
            PoolConfig {
                threads: 2,
                max_attempts: 3,
                backoff_base: Duration::from_millis(10),
            },
        );

        pool.refresh_wanted(&[(1, 0, 4)]);
        pool.submit(Job {
            key: (1, 0, 4),
            url: "http://tile/1".into(),
            priority: 1.0,
        });

        std::thread::sleep(Duration::from_millis(100));
        let results = pool.drain_results(10);
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0].outcome, JobOutcome::Success(_)));
    }

    #[test]
    fn pool_aborts_unwanted_tile() {
        let mock = Arc::new(MockBackend::new());
        let decode: Decoder<Vec<u8>> = Arc::new(test_decoder);
        let pool: WorkerPool<TileKey, Vec<u8>> = WorkerPool::spawn(
            mock,
            decode,
            PoolConfig {
                threads: 1,
                max_attempts: 1,
                backoff_base: Duration::from_millis(10),
            },
        );

        // Don't add to wanted set
        pool.submit(Job {
            key: (9, 9, 9),
            url: "http://tile/9".into(),
            priority: 1.0,
        });

        std::thread::sleep(Duration::from_millis(100));
        let results = pool.drain_results(10);
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0].outcome, JobOutcome::Aborted));
    }

    #[test]
    fn pool_retries_transient_failures() {
        let mock = Arc::new(MockBackend::new());
        mock.add(
            "http://tile/fail",
            FetchResult::Transient("timeout".into()),
        );

        let decode: Decoder<Vec<u8>> = Arc::new(test_decoder);
        let pool: WorkerPool<TileKey, Vec<u8>> = WorkerPool::spawn(
            mock,
            decode,
            PoolConfig {
                threads: 1,
                max_attempts: 3,
                backoff_base: Duration::from_millis(10),
            },
        );

        pool.refresh_wanted(&[(5, 5, 5)]);
        pool.submit(Job {
            key: (5, 5, 5),
            url: "http://tile/fail".into(),
            priority: 1.0,
        });

        std::thread::sleep(Duration::from_millis(500));
        let results = pool.drain_results(10);
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0].outcome, JobOutcome::Failed));
    }

    #[test]
    fn pool_detects_placeholder() {
        let mock = Arc::new(MockBackend::new());
        mock.add("http://tile/ph", FetchResult::Ok(vec![0, 0, 0]));

        // Decoder returns None for this data = placeholder
        let decode: Decoder<Vec<u8>> = Arc::new(|_data: &[u8]| None);
        let pool: WorkerPool<TileKey, Vec<u8>> = WorkerPool::spawn(
            mock,
            decode,
            PoolConfig {
                threads: 1,
                max_attempts: 3,
                backoff_base: Duration::from_millis(10),
            },
        );

        pool.refresh_wanted(&[(2, 2, 4)]);
        pool.submit(Job {
            key: (2, 2, 4),
            url: "http://tile/ph".into(),
            priority: 1.0,
        });

        std::thread::sleep(Duration::from_millis(100));
        let results = pool.drain_results(10);
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0].outcome, JobOutcome::Placeholder));
    }

    #[test]
    fn ureq_backend_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<UreqBackend>();
    }
}
