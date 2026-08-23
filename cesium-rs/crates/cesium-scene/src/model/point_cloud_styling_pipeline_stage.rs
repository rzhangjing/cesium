//! Ported from `packages/engine/Source/Scene/Model/PointCloudStylingPipelineStage.js`.

/// Pipeline stage for point cloud styling.
pub struct PointCloudStylingPipelineStage {
    _private: (),
}

impl PointCloudStylingPipelineStage {
    /// Creates a new PointCloudStylingPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PointCloudStylingPipelineStage {
    fn default() -> Self { Self::new() }
}
