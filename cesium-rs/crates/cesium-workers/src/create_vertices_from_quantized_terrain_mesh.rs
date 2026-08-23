//! Ported from `packages/engine/Source/Workers/createVerticesFromQuantizedTerrainMesh.js`.
//!
//! Worker entry point for creating terrain vertices from quantized mesh data.
//! Quantized mesh is a compressed terrain format used by Cesium ion and other
//! terrain servers.

/// Creates vertices from quantized terrain mesh.
///
/// In CesiumJS, this receives quantized mesh data (vertex positions stored
/// as uint16 values in a normalized [0, 65535] range), the tile's extent,
/// and metadata (encoding offsets/scales). It dequantizes the positions
/// into Cartesian3 coordinates on the ellipsoid.
///
/// The quantized mesh format includes:
/// - u, v, h arrays (uint16) for vertex positions
/// - Triangle indices
/// - Edge indices (for stitching adjacent tiles)
pub fn create_vertices_from_quantized_terrain_mesh(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Creates terrain vertices from quantized mesh data (for in-process use).
///
/// # Arguments
/// * `quantized_vertices` - Packed uint16 vertex data (u, v, h interleaved).
/// * `west` - Western longitude in radians.
/// * `south` - Southern latitude in radians.
/// * `east` - Eastern longitude in radians.
/// * `north` - Northern latitude in radians.
///
/// Returns dequantized vertex positions as a flat `Vec<f64>` (x,y,z triplets).
pub fn create_vertices_from_quantized_terrain_mesh_unpacked(
    _quantized_vertices: &[u16],
    _west: f64,
    _south: f64,
    _east: f64,
    _north: f64,
) -> Vec<f64> {
    // DEVIATION: Full dequantization requires ellipsoid transformation
    Vec::new()
}
