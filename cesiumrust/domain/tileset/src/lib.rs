//! cesium-tileset: 3D Tiles domain models
//!
//! Maps to CesiumJS:
//! - `Scene/Cesium3DTileset.js`
//! - `Scene/Cesium3DTile.js`
//! - `Scene/Cesium3DTileBoundingVolume.js`
//! - `Scene/Cesium3DTilesetTraversal.js`
//!
//! # Features
//! - tileset.json parsing (serde deserialization)
//! - Bounding volumes (Box, Region, Sphere)
//! - Tile tree structure with refinement modes
//! - LOD selection based on screen-space error

pub mod bounding_volume;
pub mod tile;
pub mod tileset;
pub mod lod_selection;

pub use bounding_volume::BoundingVolume;
pub use tile::{Tile, TileRefine, TileContent, TileContentState, TileRuntimeState};
pub use tileset::{TilesetJson, TilesetAsset, TilesetState, PropertyStats};
pub use lod_selection::{
    CameraState, LodSelectionContext, SelectedTile, TileSelectionResult,
    select_tiles, compute_tile_sse, get_tile_by_path,
};
