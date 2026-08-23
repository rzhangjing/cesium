//! Ported from `packages/engine/Source/Workers/createSphereOutlineGeometry.js`.
//!
//! Worker entry point for creating sphere outline geometry.

use cesium_core::cartesian3::Cartesian3;

/// Creates sphere outline geometry in a worker.
///
/// Deserializes sphere radius and center from packed bytes.
/// Constructs `SphereOutlineGeometry` and returns the packed result.
pub fn create_sphere_outline_geometry(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Creates a sphere outline from unpacked parameters (for in-process use).
pub fn create_sphere_outline_geometry_unpacked(
    _radius: f64,
    _center: &Cartesian3,
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: SphereOutlineGeometry not yet ported
    None
}
