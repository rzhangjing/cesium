//! Ported from `packages/engine/Source/Scene/Composite3DTileContent.js`.

/// Composite 3D tile content.
pub struct Composite3DTileContent {
    _private: (),
}

impl Composite3DTileContent {
    /// Creates a new Composite3DTileContent.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Composite3DTileContent {
    fn default() -> Self { Self::new() }
}
