//! Ported from `packages/engine/Source/Scene/NeverTileDiscardPolicy.js`.

/// A tile discard policy that never discards tiles.
pub struct NeverTileDiscardPolicy {
    _private: (),
}

impl NeverTileDiscardPolicy {
    /// Creates a new NeverTileDiscardPolicy.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for NeverTileDiscardPolicy {
    fn default() -> Self { Self::new() }
}
