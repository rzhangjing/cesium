//! Ported from `packages/engine/Source/Scene/TileBoundingSphere.js`.

/// A sphere bounding volume for a tile.
pub struct TileBoundingSphere {
    _private: (),
}

impl TileBoundingSphere {
    /// Creates a new TileBoundingSphere.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for TileBoundingSphere {
    fn default() -> Self { Self::new() }
}
