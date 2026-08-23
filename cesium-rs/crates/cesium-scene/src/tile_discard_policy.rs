//! Ported from `packages/engine/Source/Scene/TileDiscardPolicy.js`.

/// A policy for discarding tiles.
pub struct TileDiscardPolicy {
    _private: (),
}

impl TileDiscardPolicy {
    /// Creates a new TileDiscardPolicy.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for TileDiscardPolicy {
    fn default() -> Self { Self::new() }
}
