//! Ported from `packages/engine/Source/Scene/TileSelectionResult.js`.

/// The result of tile selection.
pub struct TileSelectionResult {
    _private: (),
}

impl TileSelectionResult {
    /// Creates a new TileSelectionResult.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for TileSelectionResult {
    fn default() -> Self { Self::new() }
}
