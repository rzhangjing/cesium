//! Ported from `packages/engine/Source/Scene/Model/PointCloudStylingPipelineStage.js`.

/// Pipeline stage for point cloud styling.
///
/// Applies color/size/styling to point cloud render commands.
pub struct PointCloudStylingPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl PointCloudStylingPipelineStage {
    /// Creates a new PointCloudStylingPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for PointCloudStylingPipelineStage {
    fn default() -> Self { Self::new() }
}
