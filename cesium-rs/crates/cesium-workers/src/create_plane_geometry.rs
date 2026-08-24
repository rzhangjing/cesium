//! Ported from `packages/engine/Source/Workers/createPlaneGeometry.js`.
//!
//! Worker entry point for creating plane geometry.

use cesium_core::geometry::Geometry;
use cesium_core::plane_geometry::PlaneGeometry;
use cesium_core::vertex_format::VertexFormat;

/// Creates plane geometry in a worker.
///
/// In CesiumJS, this unpacks the vertex format from the packed
/// parameters and returns `PlaneGeometry.createGeometry(...)`.
/// The Rust packed byte entry is not implemented yet (no binary pack
/// format for geometry parameters/results), so it returns an explicit
/// error; use [`create_plane_geometry_unpacked`] for in-process
/// geometry creation.
pub fn create_plane_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createPlaneGeometry"))
}

/// Creates a plane geometry from unpacked parameters (for in-process use).
///
/// Mirrors the JS worker body: `PlaneGeometry.createGeometry(new
/// PlaneGeometry({ vertexFormat }))`. CesiumJS `PlaneGeometry` is a
/// unit plane centered at the origin with only a vertex format option.
pub fn create_plane_geometry_unpacked(vertex_format: Option<VertexFormat>) -> Option<Geometry> {
    Some(PlaneGeometry::new(vertex_format).create_geometry())
}
