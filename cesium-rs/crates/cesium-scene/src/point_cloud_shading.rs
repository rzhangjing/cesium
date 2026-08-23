//! Ported from `packages/engine/Source/Scene/PointCloudShading.js`.

/// Shading settings for point clouds.
pub struct PointCloudShading {
    _private: (),
}

impl PointCloudShading {
    /// Creates a new PointCloudShading.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PointCloudShading {
    fn default() -> Self { Self::new() }
}
