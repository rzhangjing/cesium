//! Ported from `packages/engine/Source/Scene/Model/ModelColorPipelineStage.js`.
//!
//! DEVIATION (B3.5, adapted pipeline): CesiumJS `ModelColorPipelineStage` is a
//! *model*-level stage that adds the `HAS_MODEL_COLOR` define plus the
//! `model_color` (vec4) and `model_colorBlend` (float) fragment uniforms, and
//! zeroes the color mask when the model color is fully transparent. The static
//! WGSL base-color shaders have no such uniforms, so the port folds the model
//! color into the base color factor on the CPU at draw time (see
//! `Model::update`). This stage keeps the two effects that influence the
//! *pipeline configuration*: it records the `ColorBlendMode.getColorBlend`
//! factor for fidelity, and it forces the translucent pass when the model color
//! is translucent (mirrors `ModelColorPipelineStage.js` L65-67).

use cesium_core::runtime_error::RuntimeError;
use cesium_renderer::pass::Pass;

use crate::model::model_pipeline_stage::PipelineContext;
use crate::model::primitive_render_resources::PrimitiveRenderResources;

/// Pipeline stage for model color.
///
/// Applies the model-level color blend factor and, when the model color is
/// translucent, routes the primitive through the translucent pass (adapted from
/// the CesiumJS `ModelColorPipelineStage`).
pub struct ModelColorPipelineStage;

impl ModelColorPipelineStage {
    /// The stage name (mirrors the CesiumJS stage constructor name).
    pub const NAME: &'static str = "ModelColorPipelineStage";

    /// Processes the model color into the render-resource bag.
    ///
    /// Mirrors the CesiumJS `ModelColorPipelineStage.process` for the subset
    /// that affects pipeline configuration (the color fold itself happens at
    /// draw time — see the module DEVIATION).
    pub fn process(
        render_resources: &mut PrimitiveRenderResources,
        ctx: &PipelineContext,
    ) -> Result<(), RuntimeError> {
        // ColorBlendMode.getColorBlend(mode, amount) — recorded for fidelity.
        render_resources.color_blend = ctx
            .color_blend_mode
            .get_color_blend(ctx.color_blend_amount);

        // A translucent model color forces the translucent pass (mirrors the JS
        // `if (defined(model_color) && model_color.alpha < 1.0)` branch).
        if ctx.model_color.alpha < 1.0 {
            render_resources.alpha_options.pass = Some(Pass::Translucent);
        }

        Ok(())
    }
}
