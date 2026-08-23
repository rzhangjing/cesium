//! Ported from `packages/engine/Source/Core/sampleTerrain.js`.

/// Samples terrain heights at given positions.
pub struct SampleTerrain {
    _private: (),
}

impl SampleTerrain {
    /// Creates a new SampleTerrain.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for SampleTerrain {
    fn default() -> Self { Self::new() }
}
