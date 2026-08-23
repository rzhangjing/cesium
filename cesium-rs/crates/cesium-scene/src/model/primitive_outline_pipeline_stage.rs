//! Ported from `packages/engine/Source/Scene/Model/PrimitiveOutlinePipelineStage.js`.

/// Pipeline stage for primitive outlines.
pub struct PrimitiveOutlinePipelineStage {
    _private: (),
}

impl PrimitiveOutlinePipelineStage {
    /// Creates a new PrimitiveOutlinePipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PrimitiveOutlinePipelineStage {
    fn default() -> Self { Self::new() }
}
