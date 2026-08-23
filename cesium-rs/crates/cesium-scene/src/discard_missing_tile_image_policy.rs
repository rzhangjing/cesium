//! Ported from `packages/engine/Source/Scene/DiscardMissingTileImagePolicy.js`.

/// A tile image policy that discards missing tiles.
///
/// DEVIATION: stub implementation.
pub struct DiscardMissingTileImagePolicy {
    _private: (),
}

impl DiscardMissingTileImagePolicy {
    /// Creates a new discard missing tile image policy.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for DiscardMissingTileImagePolicy {
    fn default() -> Self { Self::new() }
}
