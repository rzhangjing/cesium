//! Ported from `packages/engine/Source/Workers/createWallGeometry.js`.
//!
//! Worker entry point for creating wall geometry on the ellipsoid.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::vertex_format::VertexFormat;
use cesium_core::wall_geometry::WallGeometry;

/// Creates wall geometry in a worker.
///
/// Deserializes wall positions, maximum/minimum heights, and granularity
/// from packed bytes. Constructs `WallGeometry` and returns the packed result.
pub fn create_wall_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createWallGeometry"))
}

/// Creates a wall geometry from unpacked parameters (for in-process use).
///
/// Mirror of the JS worker body: the geometry is built from the unpacked
/// parameters (constant heights variant, as the packed worker data carries
/// a single maximum/minimum height) and `createGeometry` is delegated to
/// the core port.
///
/// # Arguments
/// * `positions` - The wall positions along the ground.
/// * `maximum_height` - Maximum height of the wall.
/// * `minimum_height` - Minimum height of the wall (default 0.0).
pub fn create_wall_geometry_unpacked(
    positions: &[Cartesian3],
    maximum_height: f64,
    minimum_height: f64,
) -> Option<cesium_core::geometry::Geometry> {
    let wall_geometry = WallGeometry::from_constant_heights(
        positions.to_vec(),
        Some(minimum_height),
        Some(maximum_height),
        // JS `WallGeometry` defaults `vertexFormat` to `VertexFormat.DEFAULT`
        // (position/normal/st all true); pass it explicitly.
        Some(VertexFormat::default_format()),
        None,
    );
    wall_geometry.create_geometry()
}
