//! Ported from `packages/engine/Source/Scene/Model/SceneMode2DPipelineStage.js`.

/// Pipeline stage for 2D scene mode.
///
/// Transforms model positions for 2D/Columbus View projections.
pub struct SceneMode2DPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl SceneMode2DPipelineStage {
    /// Creates a new SceneMode2DPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for SceneMode2DPipelineStage {
    fn default() -> Self { Self::new() }
}
