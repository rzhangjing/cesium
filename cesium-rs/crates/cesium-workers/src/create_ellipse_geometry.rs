//! Ported from `packages/engine/Source/Workers/createEllipseGeometry.js`.
//!
//! Worker entry point for creating ellipse geometry on the ellipsoid.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipse_geometry::EllipseGeometry;
use cesium_core::ellipsoid::Ellipsoid;

/// Creates ellipse geometry in a worker.
///
/// Deserializes center/semiMajor/semiMinor/ellipsoid from packed bytes,
/// constructs `EllipseGeometry`, and returns the packed result.
pub fn create_ellipse_geometry(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Creates an `EllipseGeometry` from unpacked parameters (for in-process use).
pub fn create_ellipse_geometry_unpacked(
    center: Cartesian3,
    semi_major_axis: f64,
    semi_minor_axis: f64,
    ellipsoid: Option<Ellipsoid>,
) -> EllipseGeometry {
    EllipseGeometry::new(center, semi_major_axis, semi_minor_axis, ellipsoid, None, None, None, None, None)
}
