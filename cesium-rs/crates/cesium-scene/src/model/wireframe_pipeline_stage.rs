//! Ported from `packages/engine/Source/Scene/Model/WireframePipelineStage.js`.

/// Pipeline stage for wireframe rendering.
///
/// Converts solid triangles to wireframe line rendering.
pub struct WireframePipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl WireframePipelineStage {
    /// Creates a new WireframePipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for WireframePipelineStage {
    fn default() -> Self { Self::new() }
}
