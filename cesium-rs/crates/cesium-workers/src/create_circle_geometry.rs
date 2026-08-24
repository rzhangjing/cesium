//! Ported from `packages/engine/Source/Workers/createCircleGeometry.js`.
//!
//! Worker entry point for creating circle geometry on the ellipsoid.
//! CircleGeometry is a special case of EllipseGeometry where
//! `semiMajorAxis == semiMinorAxis == radius`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::circle_geometry::CircleGeometry;
use cesium_core::ellipsoid::Ellipsoid;

/// Creates circle geometry in a worker.
///
/// Deserializes center/radius/ellipsoid from packed bytes, constructs
/// `CircleGeometry`, and returns the packed geometry result.
pub fn create_circle_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createCircleGeometry"))
}

/// Creates a `CircleGeometry` from unpacked parameters (for in-process use).
pub fn create_circle_geometry_unpacked(
    center: Cartesian3,
    radius: f64,
    ellipsoid: Option<Ellipsoid>,
) -> CircleGeometry {
    CircleGeometry::new(center, radius, ellipsoid, None, None, None, None, None)
}
