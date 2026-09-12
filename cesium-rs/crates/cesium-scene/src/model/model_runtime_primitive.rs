//! Ported from `packages/engine/Source/Scene/Model/ModelRuntimePrimitive.js`.
//!
//! A runtime primitive in a model: one glTF primitive's GPU resources plus
//! the material state needed to assemble its [`DrawCommand`].

use std::sync::Arc;

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::webgl_constants::WebGLConstants;
use cesium_renderer::pass::Pass;
use cesium_renderer::render_state::RenderState;
use cesium_renderer::texture::Texture;
use cesium_renderer::vertex_array::VertexArray;

use crate::model::lighting_model::LightingModel;
use crate::model::primitive_render_resources::PrimitiveRenderResources;

/// A runtime primitive in a model.
///
/// Rust analogue of the CesiumJS `ModelRuntimePrimitive`: the GPU vertex
/// array + draw range of one glTF primitive, plus the material inputs the
/// draw command binds at group(1) (base color factor and, when textured,
/// the base color texture).
pub struct ModelRuntimePrimitive {
    /// The GPU vertex array (attributes + optional index buffer).
    pub vertex_array: Option<Arc<VertexArray>>,
    /// The number of indices (or vertices when unindexed) to draw.
    pub count: u32,
    /// The offset into the index buffer (always 0 for the ported path).
    pub offset: u32,
    /// The primitive topology (WebGL constant; TRIANGLES for the ported
    /// path — other modes are deferred, mirroring the JS mode validation).
    pub primitive_type: u32,
    /// The material base color factor (RGBA).
    pub base_color_factor: [f32; 4],
    /// The base color texture (defined when the primitive renders through
    /// the textured shader pair).
    pub base_color_texture: Option<Arc<Texture>>,
    /// Whether this primitive renders through the textured shader pair.
    pub textured: bool,
    /// Whether the material is double sided (disables back-face culling).
    pub double_sided: bool,
    /// Whether the material is translucent (`alphaMode: "BLEND"`).
    pub translucent: bool,
    /// The index of the scene-graph node that owns this primitive.
    pub node_index: usize,
    /// The bounding sphere of the primitive in model-local coordinates
    /// (derived from the POSITION accessor min/max).
    pub bounding_sphere: BoundingSphere,
    /// The render state finalized by the alpha stage (depth test / depth mask /
    /// blending). Back-face culling is applied per-frame in [`Model::update`]
    /// (it depends on the model's live `backFaceCulling`), mirroring the JS
    /// per-frame derived-command cull update.
    pub render_state: RenderState,
    /// The render pass (Opaque or Translucent) resolved by the alpha stage.
    pub pass: Pass,
    /// The resolved lighting model (recorded for fidelity; the static WGSL
    /// shaders shade unlit — see [`LightingPipelineStage`]).
    pub lighting_model: LightingModel,
    /// The `ColorBlendMode.getColorBlend` factor (recorded for fidelity; the
    /// model color is folded into the base color factor at draw time).
    pub color_blend: f32,
}

impl ModelRuntimePrimitive {
    /// Creates a new ModelRuntimePrimitive with safe defaults.
    pub fn new() -> Self {
        Self {
            vertex_array: None,
            count: 0,
            offset: 0,
            primitive_type: WebGLConstants::TRIANGLES,
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            base_color_texture: None,
            textured: false,
            double_sided: false,
            translucent: false,
            node_index: 0,
            bounding_sphere: BoundingSphere::new(Cartesian3::ZERO, 0.0),
            render_state: RenderState::default(),
            pass: Pass::Opaque,
            lighting_model: LightingModel::Unlit,
            color_blend: 0.0,
        }
    }

    /// Builds the runtime primitive from a fully-processed
    /// [`PrimitiveRenderResources`] bag plus the assembled GPU vertex array.
    ///
    /// This is the terminal step of the adapted pipeline chain: after every
    /// stage has mutated the render resources, their outputs are copied into
    /// the immutable runtime primitive the draw path reads each frame.
    pub fn from_render_resources(
        render_resources: &PrimitiveRenderResources,
        vertex_array: Arc<VertexArray>,
    ) -> Self {
        Self {
            vertex_array: Some(vertex_array),
            count: render_resources.count,
            offset: render_resources.offset,
            primitive_type: render_resources.primitive_type,
            base_color_factor: render_resources.base_color_factor,
            base_color_texture: render_resources.base_color_texture.clone(),
            textured: render_resources.textured,
            double_sided: render_resources.double_sided,
            translucent: render_resources.translucent,
            node_index: render_resources.node_index,
            bounding_sphere: render_resources.bounding_sphere.clone(),
            render_state: render_resources.render_state.clone(),
            pass: render_resources.pass,
            lighting_model: render_resources.lighting_model,
            color_blend: render_resources.color_blend,
        }
    }

    /// Whether this primitive draws through the textured shader pair
    /// (base color texture present AND the TEXCOORD_0 attribute exists).
    pub fn is_textured(&self) -> bool {
        self.textured && self.base_color_texture.is_some()
    }
}

impl Default for ModelRuntimePrimitive {
    fn default() -> Self { Self::new() }
}
