//! Ported from `packages/engine/Source/Scene/Model/ModelClippingPlanesPipelineStage.js`.

/// Pipeline stage for clipping planes.
pub struct ModelClippingPlanesPipelineStage {
    _private: (),
}

impl ModelClippingPlanesPipelineStage {
    /// Creates a new ModelClippingPlanesPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ModelClippingPlanesPipelineStage {
    fn default() -> Self { Self::new() }
}
