//! Ported from `packages/engine/Source/Scene/Model/AlphaPipelineStage.js`.

/// Pipeline stage for alpha processing.
pub struct AlphaPipelineStage {
    _private: (),
}

impl AlphaPipelineStage {
    /// Creates a new AlphaPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for AlphaPipelineStage {
    fn default() -> Self { Self::new() }
}
