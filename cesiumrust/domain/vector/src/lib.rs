//! cesium-vector：矢量数据格式（WKT、TopoJSON、3D Tiles Vector）。
//!
//! 领域层 —— 纯 Rust，f64 精度。
//!
//! 领域层提供三类矢量数据格式的解析与建模：
//! - wkt：WKT（Well-Known Text）几何文本的双向解析与序列化；
//! - topojson：拓扑编码 JSON 的弧段解码与几何重建；
//! - vector_3d_tile：3D Tiles 矢量瓦片中的点/线/面内容模型。

pub mod topojson;
pub mod vector_3d_tile;
pub mod wkt;

pub use topojson::{
    decode_arc, decode_arc_reversed, is_clockwise, resolve_linestring, resolve_polygon, ring_area,
    TopoGeometry, TopoObject, Topology, Transform,
};
pub use vector_3d_tile::{
    decode_mvt_geometry, MvtFeature, MvtGeometryType, MvtLayer, MvtValue, Vector3DTileContent,
    Vector3DTilePoints, Vector3DTilePolygons, Vector3DTilePolylines, Vector3DTileType,
};
pub use wkt::{parse_wkt, to_wkt, WktError, WktGeometry};
