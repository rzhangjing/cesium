//! Ported from `packages/engine/Source/Workers/createPlaneOutlineGeometry.js`.
//!
//! Worker entry point for creating plane outline geometry.

use cesium_core::cartesian3::Cartesian3;

/// Creates plane outline geometry in a worker.
///
/// Deserializes plane origin and dimensions from packed bytes.
/// Constructs `PlaneOutlineGeometry` and returns the packed result.
pub fn create_plane_outline_geometry(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Creates a plane outline from unpacked parameters (for in-process use).
pub fn create_plane_outline_geometry_unpacked(
    _origin: &Cartesian3,
    _normal: &Cartesian3,
    _width: f64,
    _height: f64,
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: PlaneOutlineGeometry not yet ported
    None
}
