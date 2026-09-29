//! cesium-performance：性能优化工具。
//!
//! STATUS（P2 代码健康审计，2026-09-27）：已实现并由 `cesium-specs` 测试套件覆盖，
//! 但**未接入任何生产运行时路径** —— 没有 adapter 或 application crate 依赖它
//! （黄金路径实际使用的帧预算 / LRU 缓存位于 `cesium-app/src/dynamic_globe.rs`，
//! 而非此处）。作为 CesiumJS 功能对齐的 domain 模型保留，以供未来 adapter
//! 桥接；不要将其视为已交付的能力。
//! 参见 docs/ARCHITECTURE.md "Test-only domain crates"。
//!
//! Domain 层 - 纯 Rust，f64 精度。
//!
//! CesiumJS 映射：
//! - 帧率控制
//! - 请求调度
//! - 内存管理
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
