//! Ported from `packages/engine/Source/Scene/Model/InstancingPipelineStage.js`.

/// Pipeline stage for instancing.
///
/// Sets up per-instance attributes for GPU instanced rendering.
pub struct InstancingPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl InstancingPipelineStage {
    /// Creates a new InstancingPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for InstancingPipelineStage {
    fn default() -> Self { Self::new() }
}
