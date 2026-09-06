//! Ported from `packages/engine/Source/Scene/TimeDynamicPointCloud.js`.

/// Time-dynamic point cloud.
///
/// Manages point cloud data that changes over time.
pub struct TimeDynamicPointCloud {
    /// Whether the point cloud is visible.
    pub show: bool,
    /// Whether the point cloud is ready.
    pub ready: bool,
}

impl TimeDynamicPointCloud {
    /// Creates a new TimeDynamicPointCloud.
    pub fn new() -> Self { Self { show: true, ready: false } }
}

impl Default for TimeDynamicPointCloud {
    fn default() -> Self { Self::new() }
}
