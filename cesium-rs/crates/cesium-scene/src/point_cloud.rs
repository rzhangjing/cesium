//! Ported from `packages/engine/Source/Scene/PointCloud.js`.

/// Point cloud.
///
/// Represents a point cloud primitive.
pub struct PointCloud {
    /// Whether the point cloud is visible.
    pub show: bool,
    /// The number of points.
    pub points_length: u32,
}

impl PointCloud {
    /// Creates a new PointCloud.
    pub fn new() -> Self { Self { show: true, points_length: 0 } }
}

impl Default for PointCloud {
    fn default() -> Self { Self::new() }
}
