//! Ported from `packages/engine/Source/Scene/Light.js`.

/// A light.
pub struct Light {
    _private: (),
}

impl Light {
    /// Creates a new Light.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Light {
    fn default() -> Self { Self::new() }
}
