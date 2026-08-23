//! Ported from `packages/engine/Source/Scene/PointCloud.js`.

/// A point cloud primitive.
pub struct PointCloud {
    _private: (),
}

impl PointCloud {
    /// Creates a new PointCloud.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PointCloud {
    fn default() -> Self { Self::new() }
}
