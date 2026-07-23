//! cesium-quadtree: Quadtree traversal and tile scheduling.
//!
//! Domain layer - pure Rust, f64 precision.
//!
//! CesiumJS mapping:
//! - `Scene/QuadtreePrimitive.js` → traversal
//! - Tile loading/caching → cache

pub mod cache;
pub mod traversal;

pub use cache::{
    QueuedTile, SchedulerConfig, SchedulerStats, TileCache, TileId, TileLoadQueue, TilePriority,
};
pub use traversal::{
    QuadtreeConfig, QuadtreePrimitive, QuadtreeTile, TileState, TraversalResult,
};
