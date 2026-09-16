//! cesium-ports-driven: Driven ports (Domain → External)
//! Trait contracts for external services that the domain depends on.
//!
//! In hexagonal architecture, driven ports define how the domain
//! communicates with external systems (adapters implement these traits).

use cesium_geospatial::{GeometryData, Rectangle};
use std::future::Future;
use std::pin::Pin;

/// Error type for port operations.
#[derive(Debug, thiserror::Error)]
pub enum PortError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("Decode error: {0}")]
    Decode(String),
    #[error("Cache error: {0}")]
    Cache(String),
    #[error("GPU error: {0}")]
    Gpu(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Cancelled")]
    Cancelled,
}

/// Result type for port operations.
pub type PortResult<T> = Result<T, PortError>;

// ============================================================================
// Data Fetching Ports
// ============================================================================

/// Fetches raw bytes from a URL.
/// Implemented by HTTP adapters (reqwest, browser fetch, etc.)
pub trait TileFetcher: Send + Sync {
    /// Fetches bytes from the given URL.
    fn fetch<'a>(
        &'a self,
        url: &'a str,
        priority: f64,
    ) -> Pin<Box<dyn Future<Output = PortResult<Vec<u8>>> + Send + 'a>>;

    /// Cancels a pending fetch.
    fn cancel(&self, url: &str);
}

/// Fetches imagery tiles.
pub trait ImageryProvider: Send + Sync {
    /// Gets the rectangle covered by this imagery provider.
    fn rectangle(&self) -> Rectangle;

    /// Gets the minimum zoom level.
    fn minimum_level(&self) -> u32;

    /// Gets the maximum zoom level.
    fn maximum_level(&self) -> u32;

    /// Gets the tile width in pixels.
    fn tile_width(&self) -> u32;

    /// Gets the tile height in pixels.
    fn tile_height(&self) -> u32;

    /// Requests an imagery tile.
    fn request_image<'a>(
        &'a self,
        x: u32,
        y: u32,
        level: u32,
    ) -> Pin<Box<dyn Future<Output = PortResult<Vec<u8>>> + Send + 'a>>;
}

/// Fetches terrain tiles.
pub trait TerrainProvider: Send + Sync {
    /// Gets the rectangle covered by this terrain provider.
    fn rectangle(&self) -> Rectangle;

    /// Gets the maximum zoom level.
    fn maximum_level(&self) -> u32;

    /// Requests a terrain tile.
    fn request_tile_geometry<'a>(
        &'a self,
        x: u32,
        y: u32,
        level: u32,
    ) -> Pin<Box<dyn Future<Output = PortResult<GeometryData>> + Send + 'a>>;

    /// Gets the availability of terrain data at a position.
    fn get_availability(&self, x: u32, y: u32, level: u32) -> bool;
}

// ============================================================================
// GPU/Rendering Ports
// ============================================================================

/// A handle to a GPU resource (texture, buffer, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GpuHandle(pub u64);

/// Sends geometry and textures to the GPU.
/// Implemented by rendering adapters (Bevy, wgpu, etc.)
pub trait GpuSink: Send + Sync {
    /// Uploads geometry data to the GPU, returns a handle.
    fn upload_geometry(&mut self, geometry: &GeometryData) -> PortResult<GpuHandle>;

    /// Uploads texture data to the GPU, returns a handle.
    fn upload_texture(
        &mut self,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> PortResult<GpuHandle>;

    /// Updates an existing texture.
    fn update_texture(
        &mut self,
        handle: GpuHandle,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> PortResult<()>;

    /// Deletes a GPU resource.
    fn delete(&mut self, handle: GpuHandle);
}

// ============================================================================
// Decoding Ports
// ============================================================================

/// Decodes compressed/encoded data formats.
pub trait Decoder: Send + Sync {
    /// Decodes Draco-compressed geometry.
    fn decode_draco(&self, data: &[u8]) -> PortResult<GeometryData>;

    /// Decodes an image (PNG, JPEG, WebP, etc.)
    fn decode_image(&self, data: &[u8]) -> PortResult<DecodedImage>;

    /// Decodes gzip-compressed data.
    fn decode_gzip(&self, data: &[u8]) -> PortResult<Vec<u8>>;
}

/// A decoded image with raw pixel data.
#[derive(Debug, Clone)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub channels: u32,
    pub data: Vec<u8>,
}

// ============================================================================
// Caching Ports
// ============================================================================

/// A cache for storing fetched/decoded data.
pub trait Cache: Send + Sync {
    /// Gets a cached value by key.
    fn get(&self, key: &str) -> Option<Vec<u8>>;

    /// Stores a value in the cache.
    fn set(&self, key: &str, value: Vec<u8>);

    /// Removes a value from the cache.
    fn remove(&self, key: &str) -> bool;

    /// Clears all cached data.
    fn clear(&self);

    /// Gets the current cache size in bytes.
    fn size(&self) -> usize;

    /// Gets the maximum cache size in bytes.
    fn max_size(&self) -> usize;
}

// ============================================================================
// Time/Clock Ports
// ============================================================================

/// Provides the current system time.
pub trait SystemClock: Send + Sync {
    /// Gets the current time in seconds since Unix epoch.
    fn now_secs(&self) -> f64;

    /// Gets the elapsed time since the last call (for frame timing).
    fn delta_secs(&mut self) -> f64;
}

// ============================================================================
// Scene/Rendering Ports
// ============================================================================

/// Provides access to the rendering context.
pub trait RenderContext: Send + Sync {
    /// Gets the drawing buffer width.
    fn drawing_buffer_width(&self) -> u32;

    /// Gets the drawing buffer height.
    fn drawing_buffer_height(&self) -> u32;

    /// Gets the device pixel ratio.
    fn device_pixel_ratio(&self) -> f64;
}

// ============================================================================
// M1.1 — Tile Pipeline Contracts
// ============================================================================
//
// Seven traits defining the generic tile pipeline abstraction extracted from
// `application/cesium-app/src/dynamic_globe.rs` (the "golden path" reference
// implementation). These contracts are **additive** — the existing 8 traits
// above remain unchanged and the P0 three-loader compilation is preserved.
//
// Design decisions (leader-ruled):
// - Q1: All traits are dyn-compatible after concretizing K/Payload.
//   Async methods use `Pin<Box<dyn Future + Send>>` (matching TileFetcher style).
//   No generic methods, no `Self: Sized` bounds.
// - Q2: No `BlockingTileFetcher` here; synchronous network is provided by
//   `adapters/pipeline::NetworkBackend` (ureq pool), not a ports/driven trait.
//
// Type parameter bounds:
// - `K: Hash + Eq + Copy + 'static` — tile key (e.g. `(u32, u32, u32)` = TileKey)
// - `Payload: Send + 'static` — download result (e.g. TileDownloadResult)

use std::collections::VecDeque;
use std::hash::Hash;
use std::time::Duration;

// ── TilePipeline ────────────────────────────────────────────────────────────

/// Outcome of polling a submitted tile from the pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PollOutcome<K, Payload> {
    /// Tile download completed successfully with payload.
    Ready(K, Payload),
    /// Tile was aborted (left the wanted set mid-flight).
    /// Corresponds to `dynamic_globe.rs:1052-1060` — only clears in_flight,
    /// counts stale_skip, does NOT stamp no-data.
    Aborted(K),
    /// Tile fetch failed after all retries (transient throttle/timeout).
    /// Corresponds to `dynamic_globe.rs:1062-1072` — enters retry_after
    /// cooldown (10 s), does NOT stamp permanent no-data.
    Failed(K),
    /// Tile is a placeholder (no usable imagery, e.g. Bing gradient JPEG).
    /// Corresponds to `dynamic_globe.rs:1074-1098` — stamps permanent no-data,
    /// inherits ancestor coverage.
    Placeholder(K),
}

/// Core tile pipeline trait: submit work, cancel stale requests, poll results.
///
/// Mirrors the orchestration in `dynamic_globe.rs::process_pipeline` (L662-1429)
/// and `download_worker` (L2143-2285). The pipeline owns the download thread
/// pool, deduplication, wanted-set gating, and three-state result dispatch.
///
/// Dyn-compatible when `K` and `Payload` are concrete.
pub trait TilePipeline<K, Payload>: Send + Sync
where
    K: Hash + Eq + Copy + Send + 'static,
    Payload: Send + 'static,
{
    /// Submit a tile for downloading. Priority determines fetch order
    /// (higher = sooner). Corresponds to `enqueue_tiles` (L371-459):
    /// dedup via in_flight set, priority sort, wanted-set injection.
    fn submit(&self, key: K, priority: f64);

    /// Cancel a pending/in-flight tile. Removes from wanted set so the
    /// download worker's gate (L2161) produces an Aborted result.
    fn cancel(&self, key: &K);

    /// Poll completed tiles (non-blocking drain). Returns up to `budget`
    /// results. Corresponds to the tex_rx drain loop (L1046-1048) bounded
    /// by `MAX_TEXTURE_UPLOADS_PER_FRAME`.
    fn poll_ready(&self, budget: usize) -> Vec<PollOutcome<K, Payload>>;

    /// Replace the wanted set (called once per frame at L1402-1409).
    /// Tiles not in the new set become abort candidates at the worker gate.
    fn refresh_wanted(&self, wanted: &[K]);

    /// Snapshot current pipeline statistics.
    fn stats(&self) -> PipelineStats;
}

// ── BudgetPolicy ────────────────────────────────────────────────────────────

/// Per-frame budget configuration for the tile pipeline.
///
/// Default values are **verbatim** from `dynamic_globe.rs` constants (L48-73):
/// - `DOWNLOAD_THREADS = 16` (L48)
/// - `MAX_MESH_UPLOADS_PER_FRAME = 12` (L53)
/// - `MAX_SPAWNS_PER_FRAME = 16` (L54)
/// - `MAX_TEXTURE_UPLOADS_PER_FRAME = 16` (L55)
/// - `MAX_DESPAWNS_PER_FRAME = 24` (L58)
/// - `MAX_TILE_ENTITIES = 1800` (L65)
/// - `BASE_LAYER_ZOOM = 3` (L70)
/// - `MAX_GPU_CACHE_ENTRIES = 3000` (L73)
///
/// These budgets slice GPU work across frames so a zoom/pan never produces
/// a multi-hundred-millisecond hitch (L50-52 doc comment).
pub trait BudgetPolicy: Send + Sync {
    /// Number of parallel download worker threads (L48: 16).
    fn download_threads(&self) -> usize;

    /// Max mesh GPU uploads per frame (L53: 12).
    fn max_mesh_uploads_per_frame(&self) -> usize;

    /// Max entity spawns per frame (L54: 16).
    fn max_spawns_per_frame(&self) -> usize;

    /// Max texture GPU uploads per frame (L55: 16).
    fn max_texture_uploads_per_frame(&self) -> usize;

    /// Max entity despawns per frame (L58: 24).
    fn max_despawns_per_frame(&self) -> usize;

    /// Hard cap on live tile entities (L65: 1800). Hidden tiles count
    /// toward this cap; it bounds entity/handle storage, not draw calls.
    fn max_tile_entities(&self) -> usize;

    /// Coarsest zoom level kept permanently resident (L70: 3). Tiles at
    /// z <= base_layer_zoom are never evicted or despawned.
    fn base_layer_zoom(&self) -> u32;

    /// Upper bound for GPU handle caches (L73: 3000). Oldest entries
    /// evicted FIFO when exceeded.
    fn max_gpu_cache_entries(&self) -> usize;
}

// ── EvictionPolicy ──────────────────────────────────────────────────────────

/// GPU cache eviction policy expressing the three invariants from
/// `dynamic_globe.rs::evict_gpu_cache` (L1476-1502):
///
/// 1. **BASE_LAYER exemption** (L1483): tiles at z <= base_layer_zoom are
///    never evicted — permanent global fallback layer.
/// 2. **Live-entity deferral** (L1487-1491): tiles with active entities are
///    pushed back to the queue tail (花屏防护核心 — prevents blank frames).
/// 3. **Termination guarantee** (L1471-1472): MAX_TILE_ENTITIES(1800) <<
///    MAX_GPU_CACHE_ENTRIES(3000), so evictable (dead) entries always exist.
pub trait EvictionPolicy<K>: Send + Sync
where
    K: Hash + Eq + Copy + Send + 'static,
{
    /// Returns the FIFO eviction order (oldest first).
    /// Corresponds to `mgr.gpu_tex_order: VecDeque<TileKey>` (L1479).
    fn evict_order(&self) -> &VecDeque<K>;

    /// Returns true if this key must NEVER be evicted (base layer lock).
    /// Corresponds to L1483: `if old.2 <= BASE_LAYER_ZOOM { continue }`.
    fn never_evict(&self, key: &K) -> bool;

    /// Returns true if eviction should be deferred (tile still has a live
    /// entity). Corresponds to L1487: `if mgr.tile_entities.contains_key(&old)`.
    /// Deferred entries are pushed back to the queue tail (L1489).
    fn defer_if_live(&self, key: &K) -> bool;
}

// ── StalenessPolicy ─────────────────────────────────────────────────────────

/// Three-state result classification for downloaded tiles.
///
/// Mirrors the dispatch at `dynamic_globe.rs:1052-1098`. The three states
/// are **semantically distinct and must not be merged**:
///
/// - `Aborted` (L1052-1060): tile left wanted set mid-flight. Only clears
///   in_flight + reupload guard, counts stale_skip. No permanent state change.
/// - `Failed` (L1062-1072): all retries exhausted (transient). Enters
///   retry_after cooldown (10 s). NOT permanent no-data.
/// - `Placeholder` (L1074-1098): no usable imagery (Bing gradient JPEG).
///   Stamps permanent no-data, inherits ancestor coverage via UV upsample.
pub trait StalenessPolicy: Send + Sync {
    /// Classify a download result into one of the three staleness states.
    /// Returns the appropriate `PollOutcome` variant tag.
    fn classify(&self, aborted: bool, failed: bool, placeholder: bool) -> StalenessVerdict;

    /// Duration of the retry cooldown after a Failed verdict (L1070: 10 s).
    fn retry_cooldown(&self) -> Duration;
}

/// Verdict from staleness classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StalenessVerdict {
    /// Tile left wanted set; clear in_flight, count stale_skip (L1052-1060).
    Aborted,
    /// Transient failure; enter retry cooldown (L1062-1072).
    Failed,
    /// Permanent no-data; inherit ancestor coverage (L1074-1098).
    Placeholder,
    /// Successful download with usable payload.
    Fresh,
}

// ── RetryPolicy ─────────────────────────────────────────────────────────────

/// Dual-timescale retry configuration mirroring `dynamic_globe.rs`:
///
/// 1. **Worker-level retries** (L2186-2191): 3 attempts with exponential
///    backoff `250ms << attempt` (250 ms, 500 ms, 1000 ms). Handles
///    transient 403/429/timeout from tile servers.
/// 2. **Pipeline-level cooldown** (L1068-1071): after all worker retries
///    fail, the tile enters `retry_after` map with a 10 s cooldown before
///    the enqueue path (L398-400, L419-421) will re-issue the fetch.
///
/// Both timescales are necessary: worker retries handle momentary hiccups;
/// pipeline cooldown prevents hammering a throttling server every frame.
pub trait RetryPolicy: Send + Sync {
    /// Number of worker-level retry attempts (L2186: 3).
    fn max_attempts(&self) -> u32;

    /// Base backoff duration for worker retries (L2189: 250 ms).
    /// Actual sleep = `base << attempt` (exponential).
    fn backoff_base(&self) -> Duration;

    /// Pipeline-level cooldown after all attempts exhausted (L1070: 10 s).
    fn cooldown(&self) -> Duration;
}

// ── PipelineStats ───────────────────────────────────────────────────────────

/// Pipeline statistics snapshot, aligned with M0.4 `PerfCounters`
/// (`application/cesium-app/src/perf_counters.rs`) and the 17-column CSV
/// trace format (`perf_trace.rs::CSV_HEADER`).
///
/// Field mapping to PerfCounters / CSV columns:
/// | PipelineStats field   | PerfCounters field | CSV column (1-indexed) |
/// |-----------------------|--------------------|------------------------|
/// | `frame_idx`           | `frame_idx`        | 1                      |
/// | `dt_ms`               | `dt_ms`            | 2                      |
/// | `visible_n`           | `tile_entities`    | 4                      |
/// | `partition_n`         | `spawn_queue`      | 5                      |
/// | `load_n`              | `load_set`         | 6                      |
/// | `spawn_n`             | `frame_spawn`      | 7                      |
/// | `tex_upload_n`        | `frame_tex`        | 8                      |
/// | `evict_n`             | `evict_total`      | 9                      |
/// | `gpu_tex_cache`       | `gpu_tex_order`    | 10                     |
/// | `mesh_backlog`        | `backlog`          | 11                     |
/// | `dl_in_flight`        | `in_flight`        | 12                     |
/// | `stale_skips`         | `stale_skips`      | 13                     |
/// | `retry_after`         | `retry_after`      | 14                     |
/// | `frame_mesh`          | `frame_mesh`       | 15                     |
/// | `frame_despawn`       | `frame_despawn`    | 16                     |
/// | `evict_deferred`      | `evict_deferred`   | 17                     |
///
/// Column 3 (`view_sse`) is a quadtree per-tile value not tracked at pipeline
/// level; reserved as 0 placeholder (see deferred DEFER-M0-VIEWSSE).
#[derive(Debug, Clone, Default)]
pub struct PipelineStats {
    /// Monotonic frame index (CSV col 1).
    pub frame_idx: u32,
    /// Wall-clock frame delta in milliseconds (CSV col 2).
    pub dt_ms: f64,
    /// Live tile entities (CSV col 4 = PerfCounters::tile_entities).
    pub visible_n: u32,
    /// Spawn queue depth (CSV col 5 = PerfCounters::spawn_queue).
    pub partition_n: u32,
    /// Load set size (CSV col 6 = PerfCounters::load_set).
    pub load_n: u32,
    /// Entities spawned this frame (CSV col 7).
    pub spawn_n: u32,
    /// Textures uploaded this frame (CSV col 8).
    pub tex_upload_n: u32,
    /// Cumulative evictions (CSV col 9).
    pub evict_n: u32,
    /// GPU texture cache entries (CSV col 10).
    pub gpu_tex_cache: u32,
    /// Mesh backlog depth (CSV col 11).
    pub mesh_backlog: u32,
    /// Downloads in flight (CSV col 12).
    pub dl_in_flight: u32,
    /// Cumulative stale skips (CSV col 13).
    pub stale_skips: u32,
    /// Tiles in retry cooldown (CSV col 14).
    pub retry_after: u32,
    /// Mesh uploads this frame (CSV col 15).
    pub frame_mesh: u32,
    /// Desawns this frame (CSV col 16).
    pub frame_despawn: u32,
    /// Deferred evictions (CSV col 17).
    pub evict_deferred: u32,
}

// ── ResourceBackend (M8 / P2.3) ─────────────────────────────────────────────
// Moved to `ports/driven/src/resource.rs` (M8.2 position fix). Re-exported
// below so all existing `use cesium_ports_driven::{CacheTier, ResourceBackend,
// ResourceStats}` paths remain valid.
pub mod resource;
pub use resource::{CacheTier, ResourceBackend, ResourceStats};
