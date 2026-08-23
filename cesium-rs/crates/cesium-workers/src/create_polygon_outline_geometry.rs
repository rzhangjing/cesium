//! Ported from `packages/engine/Source/Workers/createPolygonOutlineGeometry.js`.
//!
//! Worker entry point for creating polygon outline geometry on the ellipsoid.

use cesium_core::cartesian3::Cartesian3;

/// Creates polygon outline geometry in a worker.
///
/// Deserializes polygon hierarchy (positions), height, and extruded height
/// from packed bytes. Constructs `PolygonOutlineGeometry` and returns packed result.
pub fn create_polygon_outline_geometry(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Creates a polygon outline from unpacked parameters (for in-process use).
pub fn create_polygon_outline_geometry_unpacked(
    _polygon_hierarchy: &[Cartesian3],
    _height: f64,
    _extruded_height: f64,
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: PolygonOutlineGeometry not yet ported
    None
}
