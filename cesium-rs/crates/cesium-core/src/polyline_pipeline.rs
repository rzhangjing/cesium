//! Ported from `packages/engine/Source/Core/PolylinePipeline.js`.

/// Pipeline for processing polyline geometry.
pub struct PolylinePipeline {
    _private: (),
}

impl PolylinePipeline {
    /// Creates a new PolylinePipeline.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PolylinePipeline {
    fn default() -> Self { Self::new() }
}
