//! Ported from `packages/engine/Source/Workers/createCoplanarPolygonOutlineGeometry.js`.
//!
//! Worker entry point for creating coplanar polygon outline geometry.

use cesium_core::cartesian3::Cartesian3;

/// Creates coplanar polygon outline geometry in a worker.
///
/// Deserializes coplanar polygon positions from packed bytes.
/// Constructs `CoplanarPolygonOutlineGeometry` and returns the packed result.
pub fn create_coplanar_polygon_outline_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createCoplanarPolygonOutlineGeometry"))
}

/// Creates a coplanar polygon outline from unpacked parameters (for in-process use).
pub fn create_coplanar_polygon_outline_geometry_unpacked(
    _positions: &[Cartesian3],
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: CoplanarPolygonOutlineGeometry not yet ported
    None
}
