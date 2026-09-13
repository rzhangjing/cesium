//! Draco mesh decoding.
//!
//! TODO: Implement Draco decoding via C FFI bindings to Google's draco library
//! (<https://github.com/google/draco>) or via the `draco-rs` crate when it matures.
//! Draco provides lossy compression for 3D meshes and point clouds, widely used
//! in 3D Tiles and glTF for efficient geometry transmission.

use cesium_geospatial::GeometryData;
use cesium_ports_driven::{PortError, PortResult};

pub fn decode_draco(_data: &[u8]) -> PortResult<GeometryData> {
    Err(PortError::Decode(
        "Draco decoding not yet implemented".to_string(),
    ))
}
