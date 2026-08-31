//! Ported from `packages/engine/Source/Workers/createRectangleOutlineGeometry.js`.
//!
//! Worker entry point for creating rectangle outline geometry on the ellipsoid.

use cesium_core::rectangle::Rectangle;
use cesium_core::rectangle_outline_geometry::RectangleOutlineGeometry;

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
///
/// Mirror of the JS worker body: builds `RectangleOutlineGeometry` from the
/// unpacked rectangle bounds and delegates `createGeometry` to the core
/// port.
///
/// # Arguments
/// * `west`/`south`/`east`/`north` - Rectangle bounds in radians.
/// * `height` - Height above the ellipsoid.
/// * `extruded_height` - Extruded height (`None` for a flat outline,
///   mirroring the optional JS `extrudedHeight`).
pub fn create_rectangle_outline_geometry_unpacked(
    west: f64,
    south: f64,
    east: f64,
    north: f64,
    height: f64,
    extruded_height: Option<f64>,
) -> Option<cesium_core::geometry::Geometry> {
    let rectangle = Rectangle::new(west, south, east, north);
    let rectangle_outline_geometry =
        RectangleOutlineGeometry::new(rectangle, Some(height), extruded_height, None);
    RectangleOutlineGeometry::create_geometry(&rectangle_outline_geometry)
}
