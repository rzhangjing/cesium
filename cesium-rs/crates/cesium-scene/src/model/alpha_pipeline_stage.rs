//! Ported from `packages/engine/Source/Scene/Model/AlphaPipelineStage.js`.
//!
//! DEVIATION (B3.5, adapted pipeline): CesiumJS `AlphaPipelineStage` writes the
//! derived `renderStateOptions` (cull / depth mask / blending) and, for
//! `alphaCutoff`, adds the `ALPHA_MODE_MASK` define + `u_alphaCutoff` uniform.
//! The wgpu port bakes render state into immutable pipelines, so this stage
//! builds the concrete [`RenderState`] + [`Pass`] from the `alphaOptions` the
//! earlier stages populated. Two adaptations: (1) back-face culling is *not*
//! finalized here — it depends on the model's live `backFaceCulling` flag and is
//! applied per-frame in `Model::update` (mirroring the JS per-frame
//! derived-command cull update); (2) `ALPHA_MODE_MASK` discard is recorded in
//! `alphaOptions.alphaCutoff` but the static WGSL shaders have no discard path,
//! so alpha-test is deferred.

use cesium_core::runtime_error::RuntimeError;
use cesium_renderer::pass::Pass;
use cesium_renderer::render_state::{BlendEquation, BlendingFactor, RenderState};

use crate::model::model_pipeline_stage::PipelineContext;
use crate::model::primitive_render_resources::PrimitiveRenderResources;

/// Pipeline stage for alpha processing.
///
/// Finalizes the render state + pass from the alpha options for one primitive
/// (adapted from the CesiumJS `AlphaPipelineStage`).
pub struct AlphaPipelineStage;

impl AlphaPipelineStage {
    /// The stage name (mirrors the CesiumJS stage constructor name).
    pub const NAME: &'static str = "AlphaPipelineStage";

    /// Finalizes the render state / pass into the render-resource bag.
    ///
    /// Mirrors the CesiumJS `AlphaPipelineStage.process`: default the pass to
    /// the model's opaque pass, then for the translucent pass disable the depth
    /// mask and enable alpha blending (back-face culling is left to the
    /// per-frame draw path).
    pub fn process(
        render_resources: &mut PrimitiveRenderResources,
        ctx: &PipelineContext,
    ) -> Result<(), RuntimeError> {
        // alphaOptions.pass ?? model.opaquePass (mirrors JS L18-20).
        let pass = render_resources.alpha_options.pass.unwrap_or(ctx.opaque_pass);
        render_resources.pass = pass;
        render_resources.translucent = pass == Pass::Translucent;

        let mut render_state = RenderState::default();
        render_state.depth_test.enabled = true;
        if render_resources.translucent {
            // Translucent: no depth write + ALPHA_BLEND (mirrors JS L26-40;
            // `BlendingState.ALPHA_BLEND`). Culling is finalized per-frame.
            render_state.depth_mask = false;
            render_state.blending.enabled = true;
            render_state.blending.equation_rgb = BlendEquation::FuncAdd;
            render_state.blending.equation_alpha = BlendEquation::FuncAdd;
            render_state.blending.function_source_rgb = BlendingFactor::SrcAlpha;
            render_state.blending.function_source_alpha = BlendingFactor::One;
            render_state.blending.function_destination_rgb = BlendingFactor::OneMinusSrcAlpha;
            render_state.blending.function_destination_alpha = BlendingFactor::OneMinusSrcAlpha;
        } else {
            render_state.depth_mask = true;
        }
        render_resources.render_state = render_state;

        Ok(())
    }
}
