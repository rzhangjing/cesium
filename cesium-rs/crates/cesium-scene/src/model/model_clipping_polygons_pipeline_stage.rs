//! Ported from `packages/engine/Source/Scene/Model/ModelClippingPolygonsPipelineStage.js`.

/// Pipeline stage for clipping polygons.
pub struct ModelClippingPolygonsPipelineStage {
    _private: (),
}

impl ModelClippingPolygonsPipelineStage {
    /// Creates a new ModelClippingPolygonsPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ModelClippingPolygonsPipelineStage {
    fn default() -> Self { Self::new() }
}
