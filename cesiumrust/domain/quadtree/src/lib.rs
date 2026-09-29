//! cesium-quadtree：四叉树遍历与瓦片调度。
//!
//! 领域层 —— 纯 Rust，f64 精度。
//!
//! CesiumJS 映射：
//! - `Scene/QuadtreePrimitive.js` → traversal
//! - 瓦片加载/缓存 → cache

pub mod cache;
pub mod quadtree_tile_adjacency;
pub mod traversal;

pub use cache::{
    QueuedTile, SchedulerConfig, SchedulerStats, TileCache, TileId, TileLoadQueue, TilePriority,
};
pub use traversal::{
    QuadtreeConfig, QuadtreePrimitive, QuadtreeTile, TileState, TraversalResult,
};
