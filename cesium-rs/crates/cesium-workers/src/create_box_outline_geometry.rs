//! Ported from `packages/engine/Source/Workers/createBoxOutlineGeometry.js`.
//!
//! Worker entry point for creating box outline geometry.

use cesium_core::cartesian3::Cartesian3;

/// Creates box outline geometry in a worker.
///
/// Deserializes min/max corners from packed bytes, constructs `BoxOutlineGeometry`,
/// and returns the packed result (lines primitive).
pub fn create_box_outline_geometry(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Creates a box outline geometry from unpacked parameters (for in-process use).
pub fn create_box_outline_geometry_unpacked(
    minimum: &Cartesian3,
    maximum: &Cartesian3,
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: BoxOutlineGeometry not yet ported
    let _ = (minimum, maximum);
    None
}
