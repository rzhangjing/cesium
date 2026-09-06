//! Ported from `packages/engine/Source/Scene/Model/EdgeDetectionPipelineStage.js`.

/// Pipeline stage for edge detection.
///
/// Detects silhouette edges for model outline rendering.
pub struct EdgeDetectionPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl EdgeDetectionPipelineStage {
    /// Creates a new EdgeDetectionPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for EdgeDetectionPipelineStage {
    fn default() -> Self { Self::new() }
}
