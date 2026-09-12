//! Ported from `packages/engine/Source/Scene/Model/LightingPipelineStage.js`.
//!
//! DEVIATION (B3.5, adapted pipeline): CesiumJS `LightingPipelineStage` emits
//! the `LIGHTING_PBR` / `LIGHTING_UNLIT` / `USE_CUSTOM_LIGHT_COLOR` defines,
//! the `model_lightColorHdr` uniform, and the `LightingStageFS` fragment lines
//! into the `ShaderBuilder`. The static WGSL base-color shaders shade unlit
//! base color and have no lighting branch, so this stage records the resolved
//! [`LightingModel`] on the render-resource bag for fidelity (and future PBR
//! wiring) but does not alter shading. The resolution mirrors the JS: lighting
//! disabled → `UNLIT`, otherwise the material-selected model from
//! `lightingOptions`.

use cesium_core::runtime_error::RuntimeError;

use crate::model::lighting_model::LightingModel;
use crate::model::model_pipeline_stage::PipelineContext;
use crate::model::primitive_render_resources::PrimitiveRenderResources;

/// Pipeline stage for lighting.
///
/// Resolves the lighting model for one primitive (adapted from the CesiumJS
/// `LightingPipelineStage`).
pub struct LightingPipelineStage;

impl LightingPipelineStage {
    /// The stage name (mirrors the CesiumJS stage constructor name).
    pub const NAME: &'static str = "LightingPipelineStage";

    /// Resolves the lighting model into the render-resource bag.
    ///
    /// Mirrors the CesiumJS `LightingPipelineStage.process` lighting-model
    /// selection (`enableLighting` gate + `lightingOptions.lightingModel`).
    pub fn process(
        render_resources: &mut PrimitiveRenderResources,
        ctx: &PipelineContext,
    ) -> Result<(), RuntimeError> {
        render_resources.lighting_model = if !ctx.enable_lighting {
            LightingModel::Unlit
        } else {
            render_resources.lighting_options.lighting_model
        };
        Ok(())
    }
}
