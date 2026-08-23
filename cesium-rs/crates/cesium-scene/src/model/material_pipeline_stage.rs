//! Ported from `packages/engine/Source/Scene/Model/MaterialPipelineStage.js`.

/// Pipeline stage for material processing.
pub struct MaterialPipelineStage {
    _private: (),
}

impl MaterialPipelineStage {
    /// Creates a new MaterialPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for MaterialPipelineStage {
    fn default() -> Self { Self::new() }
}
