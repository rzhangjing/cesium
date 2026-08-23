//! Ported from `packages/engine/Source/Scene/Model/ModelColorPipelineStage.js`.

/// Pipeline stage for model color.
pub struct ModelColorPipelineStage {
    _private: (),
}

impl ModelColorPipelineStage {
    /// Creates a new ModelColorPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ModelColorPipelineStage {
    fn default() -> Self { Self::new() }
}
