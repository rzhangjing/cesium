//! Ported from `packages/engine/Source/Scene/Cesium3DTilesVoxelProvider.js`.

/// 3D tiles voxel provider.
pub struct Cesium3DTilesVoxelProvider {
    _private: (),
}

impl Cesium3DTilesVoxelProvider {
    /// Creates a new Cesium3DTilesVoxelProvider.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Cesium3DTilesVoxelProvider {
    fn default() -> Self { Self::new() }
}
