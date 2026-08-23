//! Ported from `packages/engine/Source/Scene/findTileMetadata.js`.

/// Finds tile metadata.
pub struct FindTileMetadata {
    _private: (),
}

impl FindTileMetadata {
    /// Creates a new FindTileMetadata.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for FindTileMetadata {
    fn default() -> Self { Self::new() }
}
