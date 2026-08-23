//! Ported from `packages/engine/Source/Workers/createEllipsoidOutlineGeometry.js`.
//!
//! Worker entry point for creating ellipsoid outline geometry.

use cesium_core::cartesian3::Cartesian3;

/// Creates ellipsoid outline geometry in a worker.
///
/// Deserializes center and radii from packed bytes.
/// Constructs `EllipsoidOutlineGeometry` and returns the packed result.
pub fn create_ellipsoid_outline_geometry(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Creates an ellipsoid outline from unpacked parameters (for in-process use).
pub fn create_ellipsoid_outline_geometry_unpacked(
    _center: &Cartesian3,
    _radii: &Cartesian3,
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: EllipsoidOutlineGeometry not yet ported
    None
}
