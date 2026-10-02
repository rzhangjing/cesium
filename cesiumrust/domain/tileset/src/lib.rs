//! cesium-tileset：3D Tiles domain 模型
//!
//! 提供 3D Tiles 的领域侧数据模型：瓦片集根、逐瓦片的包围体与
//! 层级树、内容解码、特征/批量表、结构性元数据与样式化表达式。
//!
//! # 特性
//! - tileset.json 解析（serde 反序列化）
//! - 包围体（Box、Region、Sphere）
//! - 带 refinement 模式的瓦片树结构
//! - 基于屏幕空间误差的 LOD 选择

// 子模块声明：按职责拆分为包围体、内容解码、元数据、样式、
// 瓦片树与 LOD/遍历等模块。

pub mod batch_table;
pub mod bounding_volume;
pub mod content_decoder;
pub mod json_metadata_table;
pub mod point_cloud;
pub mod structural_metadata;
pub mod styling;
pub mod tile;
pub mod tileset;
pub mod lod_selection;
pub mod tile_replacement_queue;
pub mod traversal;

// 对外重新导出各子模块的核心类型与入口函数，聚合为统一命名空间。
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
pub use structural_metadata::{
    MetadataClass, MetadataClassProperty, MetadataComponentType, MetadataEnum, MetadataType,
    MetadataValue, PropertyAttribute, PropertyAttributeProperty, PropertyTable, PropertyTexture,
    PropertyTextureProperty, StructuralMetadata,
};
