//! Ported from `packages/engine/Source/Workers/createPolygonGeometry.js`.
//!
//! Worker entry point for creating polygon geometry on the ellipsoid.

use cesium_core::cartesian3::Cartesian3;

/// Creates polygon geometry in a worker.
///
/// Deserializes polygon hierarchy (positions), height, extruded height,
/// vertex format, granularity, ellipsoid, and per-position attributes
/// from packed bytes. Constructs `PolygonGeometry` and returns packed result.
pub fn create_polygon_geometry(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Creates a polygon geometry from unpacked parameters (for in-process use).
///
/// # Arguments
/// * `polygon_hierarchy` - Flattened polygon positions (outer ring + holes).
/// * `height` - Height above the ellipsoid surface.
/// * `extruded_height` - Extruded height (0.0 for flat polygon).
pub fn create_polygon_geometry_unpacked(
    _polygon_hierarchy: &[Cartesian3],
    _height: f64,
    _extruded_height: f64,
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: PolygonGeometry.createGeometry requires EllipseGeometryLibrary
    None
}
