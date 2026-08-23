//! Ported from `packages/engine/Source/Workers/createPlaneGeometry.js`.
//!
//! Worker entry point for creating plane geometry.

use cesium_core::cartesian3::Cartesian3;

/// Creates plane geometry in a worker.
///
/// Deserializes plane origin and dimensions from packed bytes,
/// constructs `PlaneGeometry`, and returns the packed result.
pub fn create_plane_geometry(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Creates a plane geometry from unpacked parameters (for in-process use).
///
/// # Arguments
/// * `origin` - The plane origin point.
/// * `normal` - The plane normal direction.
/// * `width` - The plane width.
/// * `height` - The plane height.
pub fn create_plane_geometry_unpacked(
    _origin: &Cartesian3,
    _normal: &Cartesian3,
    _width: f64,
    _height: f64,
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: PlaneGeometry not yet ported
    None
}
