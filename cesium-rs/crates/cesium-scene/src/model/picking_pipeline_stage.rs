//! Ported from `packages/engine/Source/Scene/Model/PickingPipelineStage.js`.

/// Pipeline stage for picking.
///
/// Sets up pick color encoding for GPU-based object selection.
pub struct PickingPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl PickingPipelineStage {
    /// Creates a new PickingPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for PickingPipelineStage {
    fn default() -> Self { Self::new() }
}
