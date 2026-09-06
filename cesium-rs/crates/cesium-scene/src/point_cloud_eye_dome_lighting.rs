//! Ported from `packages/engine/Source/Scene/PointCloudEyeDomeLighting.js`.

/// Point cloud eye dome lighting.
///
/// Applies eye dome lighting (EDL) to point cloud rendering.
pub struct PointCloudEyeDomeLighting {
    /// Whether EDL is enabled.
    pub enabled: bool,
    /// The EDL strength.
    pub strength: f32,
}

impl PointCloudEyeDomeLighting {
    /// Creates a new PointCloudEyeDomeLighting.
    pub fn new() -> Self { Self { enabled: true, strength: 1.0 } }
}

impl Default for PointCloudEyeDomeLighting {
    fn default() -> Self { Self::new() }
}
