//! Ported from `packages/engine/Source/Scene/EllipsoidSurfaceAppearance.js`.

/// An appearance for an ellipsoid surface.
///
/// DEVIATION: stub implementation.
pub struct EllipsoidSurfaceAppearance {
    _private: (),
}

impl EllipsoidSurfaceAppearance {
    /// Creates a new ellipsoid surface appearance.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for EllipsoidSurfaceAppearance {
    fn default() -> Self { Self::new() }
}
