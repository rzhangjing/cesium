//! Ported from `packages/engine/Source/Scene/Model/WireframePipelineStage.js`.

/// Pipeline stage for wireframe rendering.
pub struct WireframePipelineStage {
    _private: (),
}

impl WireframePipelineStage {
    /// Creates a new WireframePipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for WireframePipelineStage {
    fn default() -> Self { Self::new() }
}
