//! Ported from `packages/engine/Source/Workers/createCoplanarPolygonOutlineGeometry.js`.
//!
//! Worker entry point for creating coplanar polygon outline geometry.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::coplanar_polygon_outline_geometry::CoplanarPolygonOutlineGeometry;

/// Creates coplanar polygon outline geometry in a worker.
///
/// Deserializes coplanar polygon positions from packed bytes.
/// Constructs `CoplanarPolygonOutlineGeometry` and returns the packed result.
pub fn create_coplanar_polygon_outline_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createCoplanarPolygonOutlineGeometry"))
}

/// Creates a coplanar polygon outline from unpacked parameters (for in-process use).
///
/// Mirror of the JS worker body: builds the outline from the positions
/// (`fromPositions` variant) and delegates `createGeometry` to the core
/// port.
pub fn create_coplanar_polygon_outline_geometry_unpacked(
    positions: &[Cartesian3],
) -> Option<cesium_core::geometry::Geometry> {
    let polygon_outline_geometry = CoplanarPolygonOutlineGeometry::new(positions.to_vec());
    polygon_outline_geometry.create_geometry()
}
