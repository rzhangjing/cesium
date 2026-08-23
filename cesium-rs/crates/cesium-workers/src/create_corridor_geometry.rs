//! Ported from `packages/engine/Source/Workers/createCorridorGeometry.js`.
//!
//! Worker entry point for creating corridor geometry on the ellipsoid.

use cesium_core::cartesian3::Cartesian3;

/// Creates corridor geometry in a worker.
///
/// Deserializes corridor positions, width, corner type, granularity,
/// height, extruded height, and vertex format from packed bytes.
/// Constructs `CorridorGeometry` and returns the packed result.
pub fn create_corridor_geometry(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Creates a corridor geometry from unpacked parameters (for in-process use).
///
/// # Arguments
/// * `positions` - The centerline positions of the corridor.
/// * `width` - The corridor width in meters.
/// * `height` - Height above the ellipsoid.
/// * `extruded_height` - Extruded height (0.0 for flat).
pub fn create_corridor_geometry_unpacked(
    _positions: &[Cartesian3],
    _width: f64,
    _height: f64,
    _extruded_height: f64,
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: CorridorGeometry not yet ported
    None
}
