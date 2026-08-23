//! Ported from `packages/engine/Source/Scene/TileMetadata.js`.

/// Metadata for a tile.
pub struct TileMetadata {
    _private: (),
}

impl TileMetadata {
    /// Creates a new TileMetadata.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for TileMetadata {
    fn default() -> Self { Self::new() }
}
