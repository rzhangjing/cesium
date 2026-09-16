//! M8 / P2.3 — `ResourceBackend` port: bulk asset streaming via a
//! pipeline-managed cache hierarchy.
//!
//! Concretized from the M1.1 placeholder (`name()` / `is_available()` only)
//! into the real streaming contract below, following the M1.1 dyn-compatibility
//! ruling (PIPELINE_PROMOTION_PLAN.md L95): async methods use
//! `Pin<Box<dyn Future + Send>>`, no generic methods, no `Self: Sized` bounds.
//! The trait is generic over the asset key `K` exactly like
//! `TilePipeline<K, Payload>` — it is dyn-compatible once `K` is concrete.
//!
//! The ports layer stays **runtime-agnostic**: no tokio, no executor, no glam.
//! The cache-hierarchy semantics (Hot/Warm/Cold) are defined here; the concrete
//! reuse of `GpuCache` / `HiddenLru` / `Dedup` lives in `adapters/pipeline`.

use std::future::Future;
use std::hash::Hash;
use std::pin::Pin;

use crate::PortResult;

/// Position of a bulk asset within the pipeline-managed cache hierarchy.
///
/// The three tiers mirror the M1.2 `cesium-pipeline` cache structures that the
/// adapter is **required to reuse** (no parallel cache system — that would
/// break the three eviction invariants protecting the `dynamic_globe` golden
/// path from 花屏 / blank frames):
///
/// - [`CacheTier::Hot`] — resident in the GPU handle cache (`GpuCache`) and
///   actively referenced; immediately usable, no round-trip.
/// - [`CacheTier::Warm`] — resident in `GpuCache` but tracked by the hidden LRU
///   (`HiddenLru`): a warm fallback retained across visibility changes,
///   re-activatable without a network fetch, evictable LRU-first under budget.
/// - [`CacheTier::Cold`] — not cached anywhere; must be streamed from the
///   network backend before use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheTier {
    /// Resident in the hot GPU cache and actively referenced.
    Hot,
    /// Resident in cache but hidden (warm LRU fallback).
    Warm,
    /// Not cached; must be streamed from the network.
    Cold,
}

/// Statistics snapshot for the [`ResourceBackend`] streaming cache hierarchy.
///
/// Field semantics are intentionally aligned with [`crate::PipelineStats`] (the
/// tile pipeline snapshot) so a host can fold resource-streaming counters into
/// the same M0.4 `PerfCounters` observation path.
#[derive(Debug, Clone, Default)]
pub struct ResourceStats {
    /// Assets resident in the hot tier (`GpuCache`, actively referenced).
    pub hot_entries: u32,
    /// Assets resident in the warm tier (`GpuCache` + `HiddenLru`).
    pub warm_entries: u32,
    /// Assets with an in-flight stream request (`Dedup` set size).
    pub in_flight: u32,
    /// Cumulative assets streamed successfully into the cache.
    pub streamed: u32,
    /// Cumulative cold requests deduplicated (already in-flight or cached).
    pub deduped: u32,
    /// Cumulative hot-tier evictions (FIFO, respecting the three invariants).
    pub evicted: u32,
}

/// Bulk asset streaming backend (textures / meshes / 3D Tiles content) via a
/// pipeline-managed cache hierarchy.
///
/// # Naming clarification (M8 vs M12 — do NOT conflate)
///
/// This trait shares a name prefix with
/// `feature_flags.rs:ENV_ENABLE_RESOURCE_BACKEND` (the **M12** plugin gate for
/// CesiumJS-style `Resource` objects), but the two are **semantically
/// unrelated** and must never be wired to each other:
///
/// - **This `ResourceBackend` trait (M8 / P2.3)**: bulk asset streaming
///   (textures, meshes, 3D Tiles content) through a pipeline-managed cache
///   hierarchy (`GpuCache` hot tier + `HiddenLru` warm tier + `Dedup`
///   in-flight), reusing the M1.2 `cesium-pipeline` eviction/dedup semantics.
///   It is an **IO/cache-layer** contract — no coordinate math, no glam.
/// - **`ENV_ENABLE_RESOURCE_BACKEND` (M12)**: toggles the `Resource` object
///   abstraction (URL templates, query parameters, retry headers) used for
///   tileset/imagery provider *configuration* (`domain/resource`).
///
/// M8 must **not** reuse the `ENV_ENABLE_RESOURCE_BACKEND` flag, and the M12
/// `Resource` abstraction must not be implemented through this trait.
///
/// # Dyn-compatibility
///
/// Generic over the asset key `K` (mirroring `TilePipeline<K, Payload>`);
/// dyn-compatible once `K` is concrete. Async methods return
/// `Pin<Box<dyn Future + Send>>` (matching [`crate::TileFetcher`] style). No
/// generic methods, no `Self: Sized` bounds.
pub trait ResourceBackend<K>: Send + Sync
where
    K: Hash + Eq + Copy + Send + 'static,
{
    /// Initiate a streaming request for the bulk asset identified by `key`.
    ///
    /// `priority` orders the fetch when the backend queue is deep (higher =
    /// sooner), matching [`crate::TilePipeline::submit`]. The returned future
    /// resolves with the streamed asset bytes:
    ///
    /// - **Cache hit** ([`CacheTier::Hot`] / [`CacheTier::Warm`]): resolves
    ///   immediately with the cached bytes (a warm hit is promoted to hot).
    /// - **Cache miss** ([`CacheTier::Cold`]): the request is deduplicated via
    ///   the in-flight set and streamed through the blocking worker pool; the
    ///   future resolves once the asset lands in the cache. Concurrent requests
    ///   for the same cold key share a single network fetch.
    ///
    /// Errors map to [`PortError`]: `Cancelled` (aborted mid-flight),
    /// `Network` (retries exhausted), `NotFound` (no usable asset).
    fn request_stream<'a>(
        &'a self,
        key: K,
        priority: f64,
    ) -> Pin<Box<dyn Future<Output = PortResult<Vec<u8>>> + Send + 'a>>;

    /// Cancel a pending / in-flight stream for `key`.
    ///
    /// Removes the key from the wanted set so the worker gate produces an
    /// aborted result, and clears the in-flight dedup entry so the asset can be
    /// re-requested later. Mirrors [`crate::TilePipeline::cancel`].
    fn cancel(&self, key: &K);

    /// Report which tier of the cache hierarchy currently holds `key`.
    fn cache_tier(&self, key: &K) -> CacheTier;

    /// Snapshot the streaming / cache-hierarchy statistics.
    fn stats(&self) -> ResourceStats;

    /// Returns a human-readable name for this backend (diagnostics).
    fn name(&self) -> &str;

    /// Returns true if this backend is currently operational.
    fn is_available(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal concrete impl used purely to prove the trait is object-safe.
    struct DummyResourceBackend;

    type DummyKey = (u32, u32, u32);

    impl ResourceBackend<DummyKey> for DummyResourceBackend {
        fn request_stream<'a>(
            &'a self,
            _key: DummyKey,
            _priority: f64,
        ) -> Pin<Box<dyn Future<Output = PortResult<Vec<u8>>> + Send + 'a>> {
            Box::pin(async { Ok(Vec::new()) })
        }
        fn cancel(&self, _key: &DummyKey) {}
        fn cache_tier(&self, _key: &DummyKey) -> CacheTier {
            CacheTier::Cold
        }
        fn stats(&self) -> ResourceStats {
            ResourceStats::default()
        }
        fn name(&self) -> &str {
            "dummy-resource-backend"
        }
        fn is_available(&self) -> bool {
            true
        }
    }

    /// Compile-time + runtime verification that `ResourceBackend<K>` is
    /// dyn-compatible (object-safe) once `K` is concrete. Mirrors the M1.1
    /// ruling and the `*_is_dyn_compatible` tests in `cesium-pipeline`
    /// (`retry.rs` / `staleness.rs`). If the trait ever gains a generic method
    /// or a `Self: Sized` bound, the `Box<dyn ...>` coercion below stops
    /// compiling — which is exactly the guard we want.
    #[test]
    fn resource_backend_is_dyn_compatible() {
        let boxed: Box<dyn ResourceBackend<DummyKey>> = Box::new(DummyResourceBackend);
        assert_eq!(boxed.name(), "dummy-resource-backend");
        assert!(boxed.is_available());
        assert_eq!(boxed.cache_tier(&(0, 0, 0)), CacheTier::Cold);
        assert_eq!(boxed.stats().hot_entries, 0);
    }

    /// The three cache tiers are pairwise distinct (Hot/Warm/Cold must never be
    /// merged — they drive different eviction/promotion behavior).
    #[test]
    fn cache_tiers_are_distinct() {
        let tiers = [CacheTier::Hot, CacheTier::Warm, CacheTier::Cold];
        for (i, a) in tiers.iter().enumerate() {
            for (j, b) in tiers.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "tiers {i} and {j} must differ");
                }
            }
        }
    }
}
