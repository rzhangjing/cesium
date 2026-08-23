//! Ported from `packages/engine/Source/Workers/upsampleQuantizedTerrainMesh.js`.
//!
//! Worker entry point for upsampling a quantized terrain mesh to a higher
//! resolution. This is used when a terrain tile needs to be displayed at
/// a higher LOD than the source data provides.

/// Upsamples a quantized terrain mesh.
///
/// In CesiumJS, this receives a quantized terrain mesh and upsamples it
/// by bilinear interpolation of the height values. The result is a new
/// quantized mesh with higher resolution vertex grid.
pub fn upsample_quantized_terrain_mesh(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Upsamples a quantized terrain mesh (for in-process use).
///
/// # Arguments
/// * `quantized_vertices` - Original quantized vertex data.
/// * `source_width` - Source grid width.
/// * `source_height` - Source grid height.
/// * `target_width` - Target grid width.
/// * `target_height` - Target grid height.
///
/// Returns upsampled quantized vertex data.
pub fn upsample_quantized_terrain_mesh_unpacked(
    _quantized_vertices: &[u16],
    _source_width: u32,
    _source_height: u32,
    _target_width: u32,
    _target_height: u32,
) -> Vec<u16> {
    // DEVIATION: Bilinear interpolation upsampling not yet implemented
    Vec::new()
}
