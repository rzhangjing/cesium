//! IO adapters for the scene document (plan §12, M8).
//!
//! Currently just [`geojson`]: a lossless (via an `x-plot` extension member) and
//! interoperable (standard GeoJSON geometries) encoder / decoder.

pub mod geojson;

pub use geojson::{from_geojson, to_geojson, PlotIoError};
