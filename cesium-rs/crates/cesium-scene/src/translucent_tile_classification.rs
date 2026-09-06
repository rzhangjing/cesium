//! Ported from `packages/engine/Source/Scene/TranslucentTileClassification.js`.

/// Translucent tile classification.
///
/// Manages classification rendering for translucent 3D tiles.
pub struct TranslucentTileClassification {
    /// Whether classification is active.
    pub active: bool,
}

impl TranslucentTileClassification {
    /// Creates a new TranslucentTileClassification.
    pub fn new() -> Self { Self { active: false } }
}

impl Default for TranslucentTileClassification {
    fn default() -> Self { Self::new() }
}
