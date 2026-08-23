//! Ported from `packages/engine/Source/Scene/Implicit3DTileContent.js`.

/// Implicit 3D tile content.
pub struct Implicit3DTileContent {
    _private: (),
}

impl Implicit3DTileContent {
    /// Creates a new Implicit3DTileContent.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Implicit3DTileContent {
    fn default() -> Self { Self::new() }
}
