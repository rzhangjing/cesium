//! Ported from `packages/engine/Source/Scene/Vector3DTileBatch.js`.

/// A batch of vector 3D tile data.
pub struct Vector3DTileBatch {
    _private: (),
}

impl Vector3DTileBatch {
    /// Creates a new Vector3DTileBatch.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Vector3DTileBatch {
    fn default() -> Self { Self::new() }
}
