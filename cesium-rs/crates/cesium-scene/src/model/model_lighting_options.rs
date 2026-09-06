//! Ported from `packages/engine/Source/Scene/Model/ModelLightingOptions.js`.

use crate::model::lighting_model::LightingModel;

/// Options for configuring the `LightingPipelineStage`.
#[derive(Debug, Clone)]
pub struct ModelLightingOptions {
    /// The lighting model to use, such as Unlit or PBR.
    /// This is determined by the primitive's material.
    pub lighting_model: LightingModel,
}

impl Default for ModelLightingOptions {
    fn default() -> Self {
        Self {
            lighting_model: LightingModel::Unlit,
        }
    }
}
