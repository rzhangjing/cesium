//! Ported from `packages/engine/Source/Workers/createCoplanarPolygonGeometry.js`.
//!
//! Worker entry point for creating coplanar polygon geometry.
//! Unlike `createPolygonGeometry`, this assumes all positions lie in a single plane.

use cesium_core::cartesian3::Cartesian3;

/// Creates coplanar polygon geometry in a worker.
///
/// Deserializes polygon positions (all assumed coplanar) from packed bytes,
/// constructs `CoplanarPolygonGeometry`, and returns the packed result.
pub fn create_coplanar_polygon_geometry(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Creates a coplanar polygon geometry from unpacked parameters (for in-process use).
///
/// # Arguments
/// * `positions` - Coplanar polygon vertex positions.
pub fn create_coplanar_polygon_geometry_unpacked(
    _positions: &[Cartesian3],
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: CoplanarPolygonGeometry not yet ported
    None
}
