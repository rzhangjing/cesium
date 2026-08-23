//! Ported from `packages/engine/Source/Core/VectorPipeline.js`.

/// Pipeline for processing vector data.
pub struct VectorPipeline {
    _private: (),
}

impl VectorPipeline {
    /// Creates a new VectorPipeline.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for VectorPipeline {
    fn default() -> Self { Self::new() }
}
