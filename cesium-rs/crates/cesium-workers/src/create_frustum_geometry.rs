//! Ported from `packages/engine/Source/Workers/createFrustumGeometry.js`.
//!
//! Worker entry point for creating frustum geometry.

use cesium_core::cartesian3::Cartesian3;

/// Creates frustum geometry in a worker.
///
/// Deserializes frustum parameters (origin, direction, up, fov, aspect ratio,
/// near, far) from packed bytes, constructs `FrustumGeometry`, and returns
/// the packed result.
pub fn create_frustum_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createFrustumGeometry"))
}

/// Creates a frustum geometry from unpacked parameters (for in-process use).
pub fn create_frustum_geometry_unpacked(
    _origin: &Cartesian3,
    _direction: &Cartesian3,
    _up: &Cartesian3,
    _fov: f64,
    _aspect_ratio: f64,
    _near: f64,
    _far: f64,
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: FrustumGeometry not yet ported
    None
}
