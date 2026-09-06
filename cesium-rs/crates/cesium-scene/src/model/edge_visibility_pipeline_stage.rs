//! Ported from `packages/engine/Source/Scene/Model/EdgeVisibilityPipelineStage.js`.

/// Pipeline stage for edge visibility.
///
/// Controls which edges are visible in wireframe/outline mode.
pub struct EdgeVisibilityPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl EdgeVisibilityPipelineStage {
    /// Creates a new EdgeVisibilityPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for EdgeVisibilityPipelineStage {
    fn default() -> Self { Self::new() }
}
