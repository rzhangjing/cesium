//! Ported from `packages/engine/Source/Scene/Model/PickingPipelineStage.js`.

/// Pipeline stage for picking.
pub struct PickingPipelineStage {
    _private: (),
}

impl PickingPipelineStage {
    /// Creates a new PickingPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PickingPipelineStage {
    fn default() -> Self { Self::new() }
}
