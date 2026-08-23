//! Ported from `packages/engine/Source/Scene/PointCloudEyeDomeLighting.js`.

/// Eye dome lighting for point clouds.
pub struct PointCloudEyeDomeLighting {
    _private: (),
}

impl PointCloudEyeDomeLighting {
    /// Creates a new PointCloudEyeDomeLighting.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PointCloudEyeDomeLighting {
    fn default() -> Self { Self::new() }
}
