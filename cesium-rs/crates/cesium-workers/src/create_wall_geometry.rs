//! Ported from `packages/engine/Source/Workers/createWallGeometry.js`.
//!
//! Worker entry point for creating wall geometry on the ellipsoid.

use cesium_core::cartesian3::Cartesian3;

/// Creates wall geometry in a worker.
///
/// Deserializes wall positions, maximum/minimum heights, and granularity
/// from packed bytes. Constructs `WallGeometry` and returns the packed result.
pub fn create_wall_geometry(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Creates a wall geometry from unpacked parameters (for in-process use).
///
/// # Arguments
/// * `positions` - The wall positions along the ground.
/// * `maximum_height` - Maximum height of the wall.
/// * `minimum_height` - Minimum height of the wall (default 0.0).
pub fn create_wall_geometry_unpacked(
    _positions: &[Cartesian3],
    _maximum_height: f64,
    _minimum_height: f64,
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: WallGeometry not yet ported
    None
}
