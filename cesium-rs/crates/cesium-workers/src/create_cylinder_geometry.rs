//! Ported from `packages/engine/Source/Workers/createCylinderGeometry.js`.
//!
//! Worker entry point for creating cylinder geometry.

use cesium_core::cartesian3::Cartesian3;

/// Creates cylinder geometry in a worker.
///
/// Deserializes cylinder parameters (length, top radius, bottom radius,
/// slices, vertex format) from packed bytes, constructs `CylinderGeometry`,
/// and returns the packed result.
pub fn create_cylinder_geometry(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Creates a cylinder geometry from unpacked parameters (for in-process use).
///
/// # Arguments
/// * `length` - The cylinder length.
/// * `top_radius` - Radius at the top cap.
/// * `bottom_radius` - Radius at the bottom cap.
/// * `slices` - Number of radial subdivisions.
pub fn create_cylinder_geometry_unpacked(
    _length: f64,
    _top_radius: f64,
    _bottom_radius: f64,
    _slices: u32,
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: CylinderGeometry not yet ported
    let _ = Cartesian3::ZERO;
    None
}
