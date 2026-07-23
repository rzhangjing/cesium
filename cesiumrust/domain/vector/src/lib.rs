//! cesium-vector: Vector data formats (WKT, TopoJSON).
//!
//! Domain layer - pure Rust, f64 precision.
//!
//! CesiumJS mapping:
//! - WKT geometry parsing
//! - `ThirdParty/topojson.js` → topojson

pub mod topojson;
pub mod wkt;

pub use topojson::{
    decode_arc, decode_arc_reversed, is_clockwise, resolve_linestring, resolve_polygon, ring_area,
    TopoGeometry, TopoObject, Topology, Transform,
};
pub use wkt::{parse_wkt, to_wkt, WktError, WktGeometry};
