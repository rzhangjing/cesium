//! Ported from `packages/engine/Source/Scene/TranslucentTileClassification.js`.

/// Classification of translucent tiles.
pub struct TranslucentTileClassification {
    _private: (),
}

impl TranslucentTileClassification {
    /// Creates a new TranslucentTileClassification.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for TranslucentTileClassification {
    fn default() -> Self { Self::new() }
}
