//! Ported from `packages/engine/Source/Scene/PrimitivePipeline.js`.

/// Pipeline for processing primitive data.
pub struct PrimitivePipeline {
    _private: (),
}

impl PrimitivePipeline {
    /// Creates a new PrimitivePipeline.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PrimitivePipeline {
    fn default() -> Self { Self::new() }
}
