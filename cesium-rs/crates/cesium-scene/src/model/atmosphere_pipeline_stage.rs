//! Ported from `packages/engine/Source/Scene/Model/AtmospherePipelineStage.js`.

/// Pipeline stage for atmosphere processing.
///
/// Applies atmospheric scattering effects to model rendering.
pub struct AtmospherePipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl AtmospherePipelineStage {
    /// Creates a new AtmospherePipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for AtmospherePipelineStage {
    fn default() -> Self { Self::new() }
}
