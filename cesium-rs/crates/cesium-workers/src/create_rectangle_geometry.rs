//! Ported from `packages/engine/Source/Workers/createRectangleGeometry.js`.
//!
//! Worker entry point for creating rectangle geometry on the ellipsoid.

use cesium_core::cartographic::Cartographic;

/// Creates rectangle geometry in a worker.
///
/// Deserializes rectangle bounds (west, south, east, north), height,
/// extruded height, granularity, vertex format, and rotation from
/// packed bytes. Constructs `RectangleGeometry` and returns packed result.
pub fn create_rectangle_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createRectangleGeometry"))
}

/// Creates a rectangle geometry from unpacked parameters (for in-process use).
///
/// # Arguments
/// * `west` - Western longitude in radians.
/// * `south` - Southern latitude in radians.
/// * `east` - Eastern longitude in radians.
/// * `north` - Northern latitude in radians.
/// * `height` - Height above the ellipsoid.
/// * `extruded_height` - Extruded height (0.0 for flat).
pub fn create_rectangle_geometry_unpacked(
    _west: f64,
    _south: f64,
    _east: f64,
    _north: f64,
    _height: f64,
    _extruded_height: f64,
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: RectangleGeometry.createGeometry not yet ported
    let _ = Cartographic::new(0.0, 0.0, 0.0);
    None
}
