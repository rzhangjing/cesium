//! Ported from `packages/engine/Source/Core/sampleTerrainMostDetailed.js`.

/// Samples terrain at the most detailed available level.
pub struct SampleTerrainMostDetailed {
    _private: (),
}

impl SampleTerrainMostDetailed {
    /// Creates a new SampleTerrainMostDetailed.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for SampleTerrainMostDetailed {
    fn default() -> Self { Self::new() }
}
