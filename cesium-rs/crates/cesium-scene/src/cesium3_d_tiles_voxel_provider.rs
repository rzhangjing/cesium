//! Ported from `packages/engine/Source/Scene/Cesium3DTilesVoxelProvider.js`.

/// Voxel provider for 3D Tiles voxel data.
///
/// Loads and provides voxel tile data for volume rendering.
pub struct Cesium3DTilesVoxelProvider {
    /// Whether the provider is ready.
    pub ready: bool,
    /// The number of tiles.
    pub tile_count: u32,
}

impl Cesium3DTilesVoxelProvider {
    /// Creates a new Cesium3DTilesVoxelProvider.
    pub fn new() -> Self { Self { ready: false, tile_count: 0 } }
}

impl Default for Cesium3DTilesVoxelProvider {
    fn default() -> Self { Self::new() }
}
