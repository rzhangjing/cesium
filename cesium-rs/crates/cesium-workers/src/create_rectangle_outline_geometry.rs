//! Ported from `packages/engine/Source/Workers/createRectangleOutlineGeometry.js`.
//!
//! Worker entry point for creating rectangle outline geometry on the ellipsoid.

/// Creates rectangle outline geometry in a worker.
///
/// Deserializes rectangle bounds (west, south, east, north), height,
/// and extruded height from packed bytes.
/// Constructs `RectangleOutlineGeometry` and returns the packed result.
pub fn create_rectangle_outline_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createRectangleOutlineGeometry"))
}

/// Creates a rectangle outline from unpacked parameters (for in-process use).
pub fn create_rectangle_outline_geometry_unpacked(
    _west: f64,
    _south: f64,
    _east: f64,
    _north: f64,
    _height: f64,
    _extruded_height: f64,
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: RectangleOutlineGeometry not yet ported
    None
}
