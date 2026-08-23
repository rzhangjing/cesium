//! Ported from `packages/engine/Source/Scene/Model/EdgeDetectionPipelineStage.js`.

/// Pipeline stage for edge detection.
pub struct EdgeDetectionPipelineStage {
    _private: (),
}

impl EdgeDetectionPipelineStage {
    /// Creates a new EdgeDetectionPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for EdgeDetectionPipelineStage {
    fn default() -> Self { Self::new() }
}
