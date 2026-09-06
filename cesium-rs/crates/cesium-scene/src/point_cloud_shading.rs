//! Ported from `packages/engine/Source/Scene/PointCloudShading.js`.

/// Point cloud shading.
///
/// Controls shading parameters for point cloud rendering.
pub struct PointCloudShading {
    /// Whether point cloud shading is enabled.
    pub enabled: bool,
    /// The attenuation factor.
    pub attenuation: bool,
    /// The geometric error scale.
    pub geometric_error_scale: f64,
    /// The maximum attenuation.
    pub maximum_attenuation: f64,
}

impl PointCloudShading {
    /// Creates a new PointCloudShading.
    pub fn new() -> Self {
        Self { enabled: false, attenuation: false, geometric_error_scale: 16.0, maximum_attenuation: 0.0 }
    }
}

impl Default for PointCloudShading {
    fn default() -> Self { Self::new() }
}
