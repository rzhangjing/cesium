//! Ported from `packages/engine/Source/Workers/createRectangleGeometry.js`.
//!
//! Worker entry point for creating rectangle geometry on the ellipsoid.

use cesium_core::rectangle::Rectangle;
use cesium_core::rectangle_geometry::RectangleGeometry;
use cesium_core::vertex_format::VertexFormat;

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
/// Mirror of the JS worker body: builds `RectangleGeometry` from the
/// unpacked rectangle bounds and delegates `createGeometry` to the core
/// port (granularity/vertex format default as in the JS constructor).
///
/// # Arguments
/// * `west` - Western longitude in radians.
/// * `south` - Southern latitude in radians.
/// * `east` - Eastern longitude in radians.
/// * `north` - Northern latitude in radians.
/// * `height` - Height above the ellipsoid.
/// * `extruded_height` - Extruded height (`None` for a flat rectangle,
///   mirroring the optional JS `extrudedHeight`).
pub fn create_rectangle_geometry_unpacked(
    west: f64,
    south: f64,
    east: f64,
    north: f64,
    height: f64,
    extruded_height: Option<f64>,
) -> Option<cesium_core::geometry::Geometry> {
    let rectangle = Rectangle::new(west, south, east, north);
    let rectangle_geometry = RectangleGeometry::new(
        rectangle,
        Some(height),
        extruded_height,
        None,
        // JS `RectangleGeometry` defaults `vertexFormat` to
        // `VertexFormat.DEFAULT` (position/normal/st); pass it explicitly
        // because the core port treats `None` as an all-false format.
        Some(VertexFormat::default_format()),
    );
    RectangleGeometry::create_geometry(&rectangle_geometry)
}
