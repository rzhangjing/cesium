//! Ported from `packages/engine/Source/Scene/TilesetMetadata.js`.

/// Metadata for a tileset.
pub struct TilesetMetadata {
    _private: (),
}

impl TilesetMetadata {
    /// Creates a new TilesetMetadata.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for TilesetMetadata {
    fn default() -> Self { Self::new() }
}
