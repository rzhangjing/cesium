//! Ported from `packages/engine/Source/Scene/SunLight.js`.

/// A directional light from the sun.
pub struct SunLight {
    _private: (),
}

impl SunLight {
    /// Creates a new SunLight.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for SunLight {
    fn default() -> Self { Self::new() }
}
