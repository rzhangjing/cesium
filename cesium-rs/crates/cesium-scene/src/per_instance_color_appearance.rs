//! Ported from `packages/engine/Source/Scene/PerInstanceColorAppearance.js`.

/// An appearance that uses per-instance colors.
pub struct PerInstanceColorAppearance {
    _private: (),
}

impl PerInstanceColorAppearance {
    /// Creates a new PerInstanceColorAppearance.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PerInstanceColorAppearance {
    fn default() -> Self { Self::new() }
}
