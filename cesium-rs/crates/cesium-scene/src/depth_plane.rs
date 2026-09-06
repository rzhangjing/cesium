//! Ported from `packages/engine/Source/Scene/DepthPlane.js`.

/// Depth plane.
///
/// Renders a full-screen quad at the far plane for depth compositing.
pub struct DepthPlane {
    /// Whether the plane is visible.
    pub show: bool,
}

impl DepthPlane {
    /// Creates a new DepthPlane.
    pub fn new() -> Self { Self { show: true } }
}

impl Default for DepthPlane {
    fn default() -> Self { Self::new() }
}
