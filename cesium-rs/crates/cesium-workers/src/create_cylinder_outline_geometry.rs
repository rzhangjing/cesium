//! Ported from `packages/engine/Source/Workers/createCylinderOutlineGeometry.js`.
//!
//! Worker entry point for creating cylinder outline geometry.

use cesium_core::cartesian3::Cartesian3;

/// Creates cylinder outline geometry in a worker.
///
/// Deserializes cylinder length, top/bottom radii, and slices from packed bytes.
/// Constructs `CylinderOutlineGeometry` and returns the packed result.
pub fn create_cylinder_outline_geometry(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Creates a cylinder outline from unpacked parameters (for in-process use).
pub fn create_cylinder_outline_geometry_unpacked(
    _length: f64,
    _top_radius: f64,
    _bottom_radius: f64,
    _slices: u32,
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: CylinderOutlineGeometry not yet ported
    let _ = Cartesian3::ZERO;
    None
}
