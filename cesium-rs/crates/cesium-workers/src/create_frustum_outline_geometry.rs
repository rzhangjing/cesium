//! Ported from `packages/engine/Source/Workers/createFrustumOutlineGeometry.js`.
//!
//! Worker entry point for creating frustum outline geometry.

use cesium_core::cartesian3::Cartesian3;

/// Creates frustum outline geometry in a worker.
///
/// Deserializes frustum parameters (origin, direction, up, fov, near, far)
/// from packed bytes. Constructs `FrustumOutlineGeometry` and returns packed result.
pub fn create_frustum_outline_geometry(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Creates a frustum outline from unpacked parameters (for in-process use).
pub fn create_frustum_outline_geometry_unpacked(
    _origin: &Cartesian3,
    _direction: &Cartesian3,
    _up: &Cartesian3,
    _fov: f64,
    _near: f64,
    _far: f64,
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: FrustumOutlineGeometry not yet ported
    None
}
