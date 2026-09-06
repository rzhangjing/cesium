//! Ported from `packages/engine/Source/Scene/Model/GeometryPipelineStage.js`.

/// Pipeline stage for geometry processing.
///
/// Prepares vertex/index buffers and attribute bindings for rendering.
pub struct GeometryPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl GeometryPipelineStage {
    /// Creates a new GeometryPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for GeometryPipelineStage {
    fn default() -> Self { Self::new() }
}
