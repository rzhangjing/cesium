//! cesium-vector：矢量数据格式（WKT、TopoJSON、3D Tiles Vector）。
//!
//! 领域层 —— 纯 Rust，f64 精度。
//!
//! CesiumJS 映射：
//! - WKT 几何解析
//! - `ThirdParty/topojson.js` → topojson
//! - `Scene/Vector3DTileContent.js` → vector_3d_tile

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
