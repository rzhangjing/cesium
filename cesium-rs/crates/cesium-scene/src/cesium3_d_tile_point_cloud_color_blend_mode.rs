//! Ported from `packages/engine/Source/Scene/Cesium3DTilePointCloudColorBlendMode.js`.

/// Color blend mode for 3D Tiles point clouds.
pub struct Cesium3DTilePointCloudColorBlendMode {
    _private: (),
}

impl Cesium3DTilePointCloudColorBlendMode {
    /// Creates a new Cesium3DTilePointCloudColorBlendMode.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Cesium3DTilePointCloudColorBlendMode {
    fn default() -> Self { Self::new() }
}
