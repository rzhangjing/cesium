//! Ported from `packages/engine/Source/Scene/Vector3DTileContent.js`.

/// Content of a vector 3D tile.
pub struct Vector3DTileContent {
    _private: (),
}

impl Vector3DTileContent {
    /// Creates a new Vector3DTileContent.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Vector3DTileContent {
    fn default() -> Self { Self::new() }
}
