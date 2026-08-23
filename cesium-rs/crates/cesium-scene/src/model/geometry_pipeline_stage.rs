//! Ported from `packages/engine/Source/Scene/Model/GeometryPipelineStage.js`.

/// Pipeline stage for geometry processing.
pub struct GeometryPipelineStage {
    _private: (),
}

impl GeometryPipelineStage {
    /// Creates a new GeometryPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GeometryPipelineStage {
    fn default() -> Self { Self::new() }
}
