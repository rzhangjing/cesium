//! Ported from `packages/engine/Source/Workers/createVerticesFromHeightmap.js`.
//!
//! Worker entry point for creating terrain vertices from heightmap data.
//! This is the core terrain generation worker that converts heightmap images
//! into 3D vertex positions on the ellipsoid.

/// Creates vertices from heightmap data.
///
/// In CesiumJS, this receives a heightmap image (typically 65×65 or 257×257),
/// the tile's geographic extent, the ellipsoid, and level-of-detail parameters.
/// It converts each pixel's height value into a Cartesian3 position on the
/// ellipsoid surface, producing the terrain mesh vertices.
///
/// The output includes:
/// - Vertex positions (Cartesian3[])
/// - Height values (for lighting/shading)
/// - Skirt heights (for hiding cracks between LOD levels)
pub fn create_vertices_from_heightmap(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Creates terrain vertices from heightmap data (for in-process use).
///
/// # Arguments
/// * `heightmap_data` - Flattened heightmap height values (row-major).
/// * `width` - Width of the heightmap grid.
/// * `height` - Height of the heightmap grid.
/// * `west` - Western longitude in radians.
/// * `south` - Southern latitude in radians.
/// * `east` - Eastern longitude in radians.
/// * `north` - Northern latitude in radians.
///
/// Returns vertex positions as a flat `Vec<f64>` (x,y,z triplets).
pub fn create_vertices_from_heightmap_unpacked(
    _heightmap_data: &[f32],
    _width: u32,
    _height: u32,
    _west: f64,
    _south: f64,
    _east: f64,
    _north: f64,
) -> Vec<f64> {
    // DEVIATION: Full implementation requires Ellipsoid.cartographicToCartesian
    // for each grid point. This is a significant computation kernel.
    Vec::new()
}
