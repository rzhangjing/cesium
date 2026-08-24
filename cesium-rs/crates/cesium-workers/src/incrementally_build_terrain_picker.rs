//! Ported from `packages/engine/Source/Workers/incrementallyBuildTerrainPicker.js`.
//!
//! Worker entry point for incrementally building the terrain picking data structure.
//! This generates a BVH (Bounding Volume Hierarchy) for efficient terrain ray casting.

/// Incrementally builds the terrain picker.
///
/// In CesiumJS, this receives terrain mesh data and incrementally builds
/// a spatial index (BVH) for efficient point picking and ray intersection
/// with terrain tiles.
pub fn incrementally_build_terrain_picker(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("incrementallyBuildTerrainPicker"))
}

/// Incrementally builds terrain picker data (for in-process use).
///
/// # Arguments
/// * `terrain_vertices` - Terrain mesh vertex positions.
/// * `terrain_indices` - Terrain mesh triangle indices.
///
/// Returns serialized BVH node data.
pub fn incrementally_build_terrain_picker_unpacked(
    _terrain_vertices: &[f64],
    _terrain_indices: &[u32],
) -> Vec<u8> {
    // DEVIATION: BVH construction not yet implemented
    Vec::new()
}
