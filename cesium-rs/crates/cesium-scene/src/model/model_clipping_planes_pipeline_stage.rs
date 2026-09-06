//! Ported from `packages/engine/Source/Scene/Model/ModelClippingPlanesPipelineStage.js`.

/// Pipeline stage for model clipping planes.
///
/// Applies clipping plane intersections to model rendering.
pub struct ModelClippingPlanesPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl ModelClippingPlanesPipelineStage {
    /// Creates a new ModelClippingPlanesPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for ModelClippingPlanesPipelineStage {
    fn default() -> Self { Self::new() }
}
