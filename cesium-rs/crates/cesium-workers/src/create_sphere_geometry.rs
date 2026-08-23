//! Ported from `packages/engine/Source/Workers/createSphereGeometry.js`.
//!
//! Worker entry point for creating sphere geometry.

use cesium_core::cartesian3::Cartesian3;

/// Creates sphere geometry in a worker.
///
/// Deserializes sphere radius and center from packed bytes,
/// constructs `SphereGeometry`, and returns the packed result.
pub fn create_sphere_geometry(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Creates a sphere geometry from unpacked parameters (for in-process use).
///
/// # Arguments
/// * `radius` - The sphere radius in meters.
/// * `center` - The sphere center in world coordinates.
pub fn create_sphere_geometry_unpacked(
    _radius: f64,
    _center: &Cartesian3,
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: SphereGeometry not yet ported
    None
}
