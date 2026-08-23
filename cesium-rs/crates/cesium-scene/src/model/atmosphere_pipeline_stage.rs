//! Ported from `packages/engine/Source/Scene/Model/AtmospherePipelineStage.js`.

/// Pipeline stage for atmosphere effects.
pub struct AtmospherePipelineStage {
    _private: (),
}

impl AtmospherePipelineStage {
    /// Creates a new AtmospherePipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for AtmospherePipelineStage {
    fn default() -> Self { Self::new() }
}
