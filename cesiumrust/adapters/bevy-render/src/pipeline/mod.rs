//! Bevy binding layer for the `cesium-pipeline` pure-std core (M1.3).
//!
//! This module is deliberately **thin**: it contains only the pieces that
//! require Bevy types —
//! - a `Handle`-typed GPU cache (`gpu_handle`) that delegates all eviction
//!   logic to the core `cesium_pipeline::GpuCache` (which implements the M1.1
//!   `EvictionPolicy` contract),
//! - Component/Resource bridges (`bindings`),
//! - Bevy system assembly (`system_wiring`),
//! - the opt-in `CesiumPipelinePlugin` (`bevy_pipeline`).
//!
//! **No business logic lives here.** Budget/eviction/staleness/retry semantics
//! are owned by the core crate, which faithfully replicates the protected
//! golden path `application/cesium-app/src/dynamic_globe.rs`.
//!
//! # Rollout
//! `CesiumPipelinePlugin` is **opt-in** and is NOT added to the default
//! cesium-app runtime in M1.3 (`CESIUM_ENABLE_PIPELINE` stays OFF). Wiring the
//! plugin into the live app is deferred to M1.4/M1.5. This guarantees the
//! golden path is pixel-neutral with respect to this change.
//!
//! # Async runtime
//! The core uses a ureq blocking worker pool; this binding layer introduces no
//! tokio main runtime (tokio remains at most an optional transitive dep of the
//! core's reqwest backend, never the pipeline runtime).

pub mod bevy_pipeline;
pub mod bindings;
pub mod budget;
pub mod fetch;
pub mod gpu_handle;
pub mod system_wiring;

/// Tile key `(x, y, zoom)`.
///
/// Matches `dynamic_globe.rs:77` (`type TileKey = (u32, u32, u32)`). The zoom
/// component (`.2`) drives the base-layer exemption invariant
/// (`dynamic_globe.rs:1483`).
pub type TileKey = (u32, u32, u32);

pub use bevy_pipeline::CesiumPipelinePlugin;
pub use bindings::{PipelineEvictionStats, PipelineTile};
pub use budget::{
    imagery_texture_budget, terrain_mesh_budget, tileset_mesh_budget, UNBOUNDED,
};
pub use fetch::{fetch_gated, pipeline_gate_enabled, FetchRoute};
pub use gpu_handle::{BevyGpuHandleCache, GpuTileHandles};
pub use system_wiring::gpu_cache_eviction_system;
