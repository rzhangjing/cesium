//! Ported from `packages/engine/Source/Workers/createWallOutlineGeometry.js`.
//!
//! Worker entry point for creating wall outline geometry on the ellipsoid.

use cesium_core::cartesian3::Cartesian3;

/// Creates wall outline geometry in a worker.
///
/// Deserializes wall positions and maximum/minimum heights from packed bytes.
/// Constructs `WallOutlineGeometry` and returns the packed result.
pub fn create_wall_outline_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createWallOutlineGeometry"))
}

/// Creates a wall outline from unpacked parameters (for in-process use).
pub fn create_wall_outline_geometry_unpacked(
    _positions: &[Cartesian3],
    _maximum_height: f64,
    _minimum_height: f64,
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: WallOutlineGeometry not yet ported
    None
}
