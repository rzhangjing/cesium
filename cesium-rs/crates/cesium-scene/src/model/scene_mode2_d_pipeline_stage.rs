//! Ported from `packages/engine/Source/Scene/Model/SceneMode2DPipelineStage.js`.

/// Pipeline stage for 2D scene mode.
pub struct SceneMode2DPipelineStage {
    _private: (),
}

impl SceneMode2DPipelineStage {
    /// Creates a new SceneMode2DPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for SceneMode2DPipelineStage {
    fn default() -> Self { Self::new() }
}
