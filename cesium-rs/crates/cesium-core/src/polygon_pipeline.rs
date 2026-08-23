//! Ported from `packages/engine/Source/Core/PolygonPipeline.js`.

/// Pipeline for processing polygon geometry.
pub struct PolygonPipeline {
    _private: (),
}

impl PolygonPipeline {
    /// Creates a new PolygonPipeline.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PolygonPipeline {
    fn default() -> Self { Self::new() }
}
