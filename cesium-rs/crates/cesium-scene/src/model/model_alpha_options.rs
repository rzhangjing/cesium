//! Ported from `packages/engine/Source/Scene/Model/ModelAlphaOptions.js`.

use cesium_renderer::pass::Pass;

/// Options for configuring the `AlphaPipelineStage`.
#[derive(Debug, Clone, Default)]
pub struct ModelAlphaOptions {
    /// Which render pass will render the model.
    pub pass: Option<Pass>,
    /// Determines the alpha threshold below which fragments are discarded.
    pub alpha_cutoff: Option<f32>,
}
