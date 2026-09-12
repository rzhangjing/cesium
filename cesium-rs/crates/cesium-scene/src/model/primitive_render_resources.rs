//! Ported from `packages/engine/Source/Scene/Model/PrimitiveRenderResources.js`.
//!
//! Per-primitive render resources: the bag that the model pipeline stages
//! mutate as they process one glTF primitive.
//!
//! DEVIATION (B3.5, adapted pipeline): CesiumJS `PrimitiveRenderResources`
//! inherits a `ShaderBuilder` from the node resources and every stage adds
//! GLSL (`addDefine` / `addUniform` / `addFragmentLines`) that is compiled
//! into a per-primitive shader. The wgpu port renders through a fixed pair of
//! pre-written WGSL shaders (color / textured base color), so the `shaderBuilder`
//! field is replaced by the values those static shaders actually consume:
//! the accumulated vertex `attributes` + `index_buffer` (assembled into a
//! `VertexArray` once every stage has run), the `uniform` inputs folded into
//! the draw command, the `render_state` / `pass`, and the material / alpha /
//! lighting option bags. Stages therefore configure shader *selection* +
//! uniforms + render state instead of generating shader source.

use std::sync::Arc;

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;
use cesium_renderer::buffer::IndexBuffer;
use cesium_renderer::pass::Pass;
use cesium_renderer::render_state::RenderState;
use cesium_renderer::texture::Texture;
use cesium_renderer::vertex_array::{VertexArray, VertexAttribute};

use crate::model::lighting_model::LightingModel;
use crate::model::model_alpha_options::ModelAlphaOptions;
use crate::model::model_lighting_options::ModelLightingOptions;

/// Per-primitive rendering resources mutated by the model pipeline stages.
///
/// Rust analogue of the CesiumJS `PrimitiveRenderResources` (298 lines),
/// adapted to the port's static-WGSL render path (see the module DEVIATION).
/// The fields mirror the JS bag: the accumulated vertex `attributes`, the
/// `uniform`/material inputs, the `alphaOptions` / `lightingOptions`, the
/// derived `renderStateOptions` / `pass`, and the primitive draw range.
pub struct PrimitiveRenderResources {
    // ---- geometry stage outputs ----
    /// The accumulated vertex attributes (POSITION at location 0, then any
    /// material-driven attributes such as TEXCOORD_0). Assembled into the
    /// `VertexArray` once every stage has run (mirrors the JS pattern where
    /// stages push into `renderResources.attributes` and the vertex array is
    /// created afterwards).
    pub attributes: Vec<VertexAttribute>,
    /// The index buffer (defined for indexed primitives).
    pub index_buffer: Option<IndexBuffer>,
    /// The number of indices (or vertices when unindexed) to draw.
    pub count: u32,
    /// The offset into the index buffer (always 0 for the ported path).
    pub offset: u32,
    /// The primitive topology (WebGL constant).
    pub primitive_type: u32,
    /// The minimum POSITION value (from the accessor `min`).
    pub position_min: Cartesian3,
    /// The maximum POSITION value (from the accessor `max`).
    pub position_max: Cartesian3,
    /// The bounding sphere containing all vertices (model-local).
    pub bounding_sphere: BoundingSphere,

    // ---- material stage outputs ----
    /// The material base color factor (RGBA).
    pub base_color_factor: [f32; 4],
    /// The base color texture (defined when the primitive renders through the
    /// textured shader pair).
    pub base_color_texture: Option<Arc<Texture>>,
    /// Whether this primitive renders through the textured shader pair.
    pub textured: bool,
    /// Whether the material is double sided (disables back-face culling).
    pub double_sided: bool,
    /// Options configuring the lighting stage (the lighting model selected by
    /// the material). Mirrors the JS `lightingOptions`.
    pub lighting_options: ModelLightingOptions,
    /// Options configuring the alpha stage (pass + alpha cutoff). Mirrors the
    /// JS `alphaOptions`.
    pub alpha_options: ModelAlphaOptions,

    // ---- model color stage outputs ----
    /// The model color blend factor (`ColorBlendMode.getColorBlend`), recorded
    /// for fidelity. DEVIATION: the static WGSL base-color shaders do not yet
    /// consume a `model_colorBlend` uniform, so the port folds the model color
    /// into the base color factor at draw time (see `ModelColorPipelineStage`).
    pub color_blend: f32,

    // ---- lighting stage outputs ----
    /// The resolved lighting model. DEVIATION: the static WGSL model shaders
    /// shade unlit base color, so PBR is recorded but not yet applied (see
    /// `LightingPipelineStage`).
    pub lighting_model: LightingModel,

    // ---- alpha stage outputs ----
    /// The render state assembled from the alpha options (depth test / depth
    /// mask / blending). Back-face culling is finalized per-frame in the draw
    /// path (it depends on the model's live `backFaceCulling`), mirroring the
    /// JS per-frame derived-command cull update.
    pub render_state: RenderState,
    /// The render pass (Opaque or Translucent).
    pub pass: Pass,
    /// Whether the primitive takes the translucent path.
    pub translucent: bool,

    // ---- node / instance ----
    /// The index of the scene-graph node that owns this primitive.
    pub node_index: usize,
    /// The number of instances (0 when instancing is not used; the ported path
    /// does not instance — see the deferred `InstancingPipelineStage`).
    pub instance_count: usize,
}

impl PrimitiveRenderResources {
    /// Creates a new `PrimitiveRenderResources` for the node at `node_index`.
    ///
    /// Mirrors the JS constructor's inherited defaults: an empty attribute
    /// list, `attributeIndex` past POSITION, default (opaque) alpha options,
    /// and an unlit lighting model until the material stage runs.
    pub fn new(node_index: usize) -> Self {
        Self {
            attributes: Vec::new(),
            index_buffer: None,
            count: 0,
            offset: 0,
            primitive_type: cesium_core::webgl_constants::WebGLConstants::TRIANGLES,
            position_min: Cartesian3::ZERO,
            position_max: Cartesian3::ZERO,
            bounding_sphere: BoundingSphere::new(Cartesian3::ZERO, 0.0),
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            base_color_texture: None,
            textured: false,
            double_sided: false,
            lighting_options: ModelLightingOptions::default(),
            alpha_options: ModelAlphaOptions::default(),
            color_blend: 0.0,
            lighting_model: LightingModel::Unlit,
            render_state: RenderState::default(),
            pass: Pass::Opaque,
            translucent: false,
            node_index,
            instance_count: 0,
        }
    }

    /// The next vertex attribute location (POSITION occupies 0). Mirrors the
    /// JS `attributeIndex` bookkeeping.
    pub fn attribute_index(&self) -> usize {
        self.attributes.len()
    }

    /// Appends a vertex attribute and returns its assigned location.
    pub fn add_attribute(&mut self, attribute: VertexAttribute) -> usize {
        let index = self.attributes.len();
        self.attributes.push(attribute);
        index
    }

    /// Assembles the accumulated `attributes` + `index_buffer` into the GPU
    /// `VertexArray` (the adapted equivalent of the JS vertex-array creation
    /// that runs after the pipeline stages have populated `attributes`).
    ///
    /// Takes the attributes/index buffer by move (`VertexAttribute` and
    /// `Buffer` are not `Clone`); the resources are consumed right after the
    /// pipeline chain finishes, so draining them here is safe.
    pub fn create_vertex_array(&mut self) -> Arc<VertexArray> {
        let attributes = std::mem::take(&mut self.attributes);
        let index_buffer = self.index_buffer.take();
        Arc::new(VertexArray::new(attributes, index_buffer))
    }

    /// Applies the model color to the material base color factor (the adapted
    /// equivalent of the JS `model_color` fragment uniform, folded on the CPU
    /// so the static WGSL base-color shader consumes a single `vec4`).
    pub fn base_color_with_model_color(&self, model_color: Color) -> [f32; 4] {
        [
            self.base_color_factor[0] * model_color.red as f32,
            self.base_color_factor[1] * model_color.green as f32,
            self.base_color_factor[2] * model_color.blue as f32,
            self.base_color_factor[3] * model_color.alpha as f32,
        ]
    }
}

impl Default for PrimitiveRenderResources {
    fn default() -> Self {
        Self::new(0)
    }
}
