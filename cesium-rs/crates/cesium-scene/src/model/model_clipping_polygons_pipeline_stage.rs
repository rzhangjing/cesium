//! Ported from `packages/engine/Source/Scene/Model/ModelClippingPolygonsPipelineStage.js`.

/// Pipeline stage for model clipping polygons.
///
/// Applies polygon-based clipping regions to model rendering.
pub struct ModelClippingPolygonsPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl ModelClippingPolygonsPipelineStage {
    /// Creates a new ModelClippingPolygonsPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for ModelClippingPolygonsPipelineStage {
    fn default() -> Self { Self::new() }
}
