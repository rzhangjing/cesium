//! cesium-performance: Performance optimization utilities.
//!
//! STATUS (P2 code-health audit, 2026-09-27): implemented and covered by the
//! `cesium-specs` suite, but **not wired into any production runtime path** — no
//! adapter or application crate depends on it (the live frame budget / LRU cache
//! used by the golden path lives in `cesium-app/src/dynamic_globe.rs`, not here).
//! Retained as a CesiumJS feature-parity domain model reserved for future adapter
//! bridging; do NOT read it as a shipped capability.
//! See docs/ARCHITECTURE.md "Test-only domain crates".
//!
//! Domain layer - pure Rust, f64 precision.
//!
//! CesiumJS mapping:
//! - Frame rate control
//! - Request scheduling
//! - Memory management
//! - `Scene/Cesium3DTilesetCache.js` → cache::TilesetCache
//! - `Scene/ResourceCache.js` → cache::ResourceCache
//! - `Scene/ResourceCacheStatistics.js` → cache::CacheStatistics

pub mod cache;
pub mod performance;

pub use cache::{CacheStatistics, LruCache, ResourceCache, TilesetCache};
pub use performance::{
    FrameRateConfig, FrameRateController, MemoryBudget, MemoryTracker, RequestPriority,
    RequestScheduler, ScheduledRequest,
};
