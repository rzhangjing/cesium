//! Ported from `packages/engine/Source/Workers/createWallOutlineGeometry.js`.
//!
//! Worker entry point for creating wall outline geometry on the ellipsoid.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::wall_outline_geometry::WallOutlineGeometry;

/// Creates wall outline geometry in a worker.
///
/// Deserializes wall positions and maximum/minimum heights from packed bytes.
/// Constructs `WallOutlineGeometry` and returns the packed result.
pub fn create_wall_outline_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createWallOutlineGeometry"))
}

/// Creates a wall outline from unpacked parameters (for in-process use).
///
/// Mirror of the JS worker body: builds the outline geometry from the
/// unpacked constant-height parameters and delegates `createGeometry` to
/// the core port.
pub fn create_wall_outline_geometry_unpacked(
    positions: &[Cartesian3],
    maximum_height: f64,
    minimum_height: f64,
) -> Option<cesium_core::geometry::Geometry> {
    let wall_outline_geometry = WallOutlineGeometry::from_constant_heights(
        positions.to_vec(),
        Some(minimum_height),
        Some(maximum_height),
        None,
    );
    wall_outline_geometry.create_geometry()
}
