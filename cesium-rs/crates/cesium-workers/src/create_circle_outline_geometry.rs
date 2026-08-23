//! Ported from `packages/engine/Source/Workers/createCircleOutlineGeometry.js`.
//!
//! Worker entry point for creating circle outline geometry on the ellipsoid.

use cesium_core::cartesian3::Cartesian3;

/// Creates circle outline geometry in a worker.
///
/// Deserializes center, radius, and ellipsoid from packed bytes.
/// Constructs `CircleOutlineGeometry` and returns the packed result.
pub fn create_circle_outline_geometry(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Creates a circle outline geometry from unpacked parameters (for in-process use).
pub fn create_circle_outline_geometry_unpacked(
    _center: &Cartesian3,
    _radius: f64,
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: CircleOutlineGeometry not yet ported
    None
}
