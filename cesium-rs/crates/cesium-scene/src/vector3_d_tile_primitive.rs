//! Ported from `packages/engine/Source/Scene/Vector3DTilePrimitive.js`.

/// A primitive for vector 3D tiles.
pub struct Vector3DTilePrimitive {
    _private: (),
}

impl Vector3DTilePrimitive {
    /// Creates a new Vector3DTilePrimitive.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Vector3DTilePrimitive {
    fn default() -> Self { Self::new() }
}
