//! cesium-vector: Vector data formats (WKT, TopoJSON, 3D Tiles Vector).
//!
//! Domain layer - pure Rust, f64 precision.
//!
//! CesiumJS mapping:
//! - WKT geometry parsing
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
