//! Ported from `packages/engine/Source/Scene/TimeDynamicPointCloud.js`.

/// A time-dynamic point cloud.
pub struct TimeDynamicPointCloud {
    _private: (),
}

impl TimeDynamicPointCloud {
    /// Creates a new TimeDynamicPointCloud.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for TimeDynamicPointCloud {
    fn default() -> Self { Self::new() }
}
