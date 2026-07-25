//! cesium-performance: Performance optimization utilities.
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
