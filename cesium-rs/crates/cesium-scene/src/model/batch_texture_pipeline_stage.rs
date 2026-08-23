//! Ported from `packages/engine/Source/Scene/Model/BatchTexturePipelineStage.js`.

/// Pipeline stage for batch texture processing.
pub struct BatchTexturePipelineStage {
    _private: (),
}

impl BatchTexturePipelineStage {
    /// Creates a new BatchTexturePipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BatchTexturePipelineStage {
    fn default() -> Self { Self::new() }
}
