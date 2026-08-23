//! Ported from `packages/engine/Source/Scene/DepthPlane.js`.

/// A depth plane.
pub struct DepthPlane {
    _private: (),
}

impl DepthPlane {
    /// Creates a new DepthPlane.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for DepthPlane {
    fn default() -> Self { Self::new() }
}
