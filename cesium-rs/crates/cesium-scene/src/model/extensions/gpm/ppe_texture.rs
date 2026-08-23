//! Ported from `packages/engine/Source/Scene/Model/extensions/gpm/`.

/// Per-pixel effect texture.
pub struct PpeTexture {
    _private: (),
}

impl PpeTexture {
    /// Creates a new PpeTexture.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PpeTexture {
    fn default() -> Self { Self::new() }
}
