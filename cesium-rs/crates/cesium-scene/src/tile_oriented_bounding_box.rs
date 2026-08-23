//! Ported from `packages/engine/Source/Scene/TileOrientedBoundingBox.js`.

/// An oriented bounding box for a tile.
pub struct TileOrientedBoundingBox {
    _private: (),
}

impl TileOrientedBoundingBox {
    /// Creates a new TileOrientedBoundingBox.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for TileOrientedBoundingBox {
    fn default() -> Self { Self::new() }
}
