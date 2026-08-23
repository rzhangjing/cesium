//! Ported from `packages/engine/Source/Scene/Tonemapper.js`.

/// Tone mapping for HDR rendering.
pub struct Tonemapper {
    _private: (),
}

impl Tonemapper {
    /// Creates a new Tonemapper.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Tonemapper {
    fn default() -> Self { Self::new() }
}
