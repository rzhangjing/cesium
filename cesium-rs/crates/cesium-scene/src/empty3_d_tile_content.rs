//! Ported from `packages/engine/Source/Scene/Empty3DTileContent.js`.

/// Empty 3D tile content.
pub struct Empty3DTileContent {
    _private: (),
}

impl Empty3DTileContent {
    /// Creates a new Empty3DTileContent.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Empty3DTileContent {
    fn default() -> Self { Self::new() }
}
