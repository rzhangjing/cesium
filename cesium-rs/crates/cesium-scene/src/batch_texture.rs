//! Ported from `packages/engine/Source/Scene/BatchTexture.js`.

/// A texture used for batch ID rendering.
pub struct BatchTexture {
    _private: (),
}

impl BatchTexture {
    /// Creates a new BatchTexture.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BatchTexture {
    fn default() -> Self { Self::new() }
}
