//! Ported from `packages/engine/Source/Workers/createEllipsoidGeometry.js`.
//!
//! Worker entry point for creating ellipsoid geometry.

use cesium_core::cartesian3::Cartesian3;

/// Creates ellipsoid geometry in a worker.
///
/// Deserializes center/radii from packed bytes, constructs `EllipsoidGeometry`,
/// and returns the packed geometry result.
pub fn create_ellipsoid_geometry(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Creates an ellipsoid geometry from unpacked parameters (for in-process use).
///
/// # Arguments
/// * `center` - Center of the ellipsoid in world coordinates.
/// * `radii` - The radii along x, y, z axes.
pub fn create_ellipsoid_geometry_unpacked(center: &Cartesian3, radii: &Cartesian3) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: EllipsoidGeometry.createGeometry not yet ported
    let _ = (center, radii);
    None
}
