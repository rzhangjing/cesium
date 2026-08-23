//! Ported from `packages/engine/Source/Scene/Model/EdgeVisibilityPipelineStage.js`.

/// Pipeline stage for edge visibility.
pub struct EdgeVisibilityPipelineStage {
    _private: (),
}

impl EdgeVisibilityPipelineStage {
    /// Creates a new EdgeVisibilityPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for EdgeVisibilityPipelineStage {
    fn default() -> Self { Self::new() }
}
