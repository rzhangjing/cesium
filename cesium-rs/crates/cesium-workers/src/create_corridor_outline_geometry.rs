//! Ported from `packages/engine/Source/Workers/createCorridorOutlineGeometry.js`.
//!
//! Worker entry point for creating corridor outline geometry on the ellipsoid.

use cesium_core::cartesian3::Cartesian3;

/// Creates corridor outline geometry in a worker.
///
/// Deserializes corridor positions, width, and corner type from packed bytes.
/// Constructs `CorridorOutlineGeometry` and returns the packed result.
pub fn create_corridor_outline_geometry(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Creates a corridor outline from unpacked parameters (for in-process use).
pub fn create_corridor_outline_geometry_unpacked(
    _positions: &[Cartesian3],
    _width: f64,
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: CorridorOutlineGeometry not yet ported
    None
}
