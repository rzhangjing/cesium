//! Ported from `packages/engine/Source/Scene/TileBoundingVolume.js`.

/// A bounding volume for a tile.
pub struct TileBoundingVolume {
    _private: (),
}

impl TileBoundingVolume {
    /// Creates a new TileBoundingVolume.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for TileBoundingVolume {
    fn default() -> Self { Self::new() }
}
