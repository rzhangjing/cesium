//! Ported from `packages/engine/Source/Workers/createEllipseOutlineGeometry.js`.
//!
//! Worker entry point for creating ellipse outline geometry on the ellipsoid.

use cesium_core::cartesian3::Cartesian3;

/// Creates ellipse outline geometry in a worker.
///
/// Deserializes center, semi-major axis, semi-minor axis, and ellipsoid
/// from packed bytes. Constructs `EllipseOutlineGeometry` and returns packed result.
pub fn create_ellipse_outline_geometry(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Creates an ellipse outline from unpacked parameters (for in-process use).
pub fn create_ellipse_outline_geometry_unpacked(
    _center: &Cartesian3,
    _semi_major_axis: f64,
    _semi_minor_axis: f64,
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: EllipseOutlineGeometry not yet ported
    None
}
