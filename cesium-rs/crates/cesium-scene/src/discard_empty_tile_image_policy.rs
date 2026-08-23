//! Ported from `packages/engine/Source/Scene/DiscardEmptyTileImagePolicy.js`.

/// A tile image policy that discards empty tiles.
///
/// DEVIATION: stub implementation.
pub struct DiscardEmptyTileImagePolicy {
    _private: (),
}

impl DiscardEmptyTileImagePolicy {
    /// Creates a new discard empty tile image policy.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for DiscardEmptyTileImagePolicy {
    fn default() -> Self { Self::new() }
}
