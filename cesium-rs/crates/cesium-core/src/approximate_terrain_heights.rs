//! Ported from `packages/engine/Source/Core/ApproximateTerrainHeights.js`.

/// Approximate terrain heights for common locations.
pub struct ApproximateTerrainHeights {
    _private: (),
}

impl ApproximateTerrainHeights {
    /// Creates a new ApproximateTerrainHeights.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ApproximateTerrainHeights {
    fn default() -> Self { Self::new() }
}
