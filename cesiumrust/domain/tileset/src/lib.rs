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

pub mod batch_table;
pub mod bounding_volume;
pub mod content_decoder;
pub mod point_cloud;
pub mod styling;
pub mod tile;
pub mod tileset;
pub mod lod_selection;
pub mod traversal;

pub use batch_table::{
    AccessorType, BatchPropertyValue, BatchTable, BatchTableHierarchy, BinaryPropertyRef,
    ComponentType, FeatureTable, HierarchyClass, TileFeature,
};
pub use bounding_volume::BoundingVolume;
pub use content_decoder::{
    B3dmContent, CmptContent, DecodeError, DecodedTile, I3dmContent, PntsContent,
    TileContentType, decode_tile_content, detect_content_type, parse_b3dm, parse_cmpt,
    parse_i3dm, parse_pnts,
};
pub use tile::{Tile, TileRefine, TileContent, TileContentState, TileRuntimeState};
pub use tileset::{TilesetJson, TilesetAsset, TilesetState, PropertyStats};
pub use lod_selection::{
    CameraState, LodSelectionContext, SelectedTile, TileSelectionResult,
    select_tiles, compute_tile_sse, get_tile_by_path,
};
pub use traversal::{
    MemoryAdjustedSse, TilePriority, TileRequest, TraversalContext, TraversalResult,
    TraversalStrategy, can_traverse, sort_children_by_distance, traverse,
};
pub use styling::{
    BinaryOperator, Condition, ConditionsExpression, EvalResult, Expression, StyleExpression,
    TileStyle, UnaryOperator,
};
pub use point_cloud::{
    PointCloud, PointCloudShading, QuantizedPositions, TimeDynamicPointCloud,
};
