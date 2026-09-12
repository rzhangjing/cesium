//! Ported from `packages/engine/Source/Scene/Model/ModelRuntimePrimitive.js`
//! (`configurePipeline`) plus the shared per-stage GPU helpers.
//!
//! DEVIATION (B3.5, adapted pipeline): CesiumJS `ModelRuntimePrimitive`
//! assembles an ordered array of pipeline *stage objects* whose static
//! `process(renderResources, primitive, frameState)` methods append GLSL to a
//! `ShaderBuilder` (`addDefine` / `addUniform` / `addFragmentLines`). The wgpu
//! port renders through a fixed pair of pre-written WGSL shaders, so there is
//! no GLSL to generate. This module keeps the CesiumJS *structure* — an ordered
//! chain of stages, each mutating a [`PrimitiveRenderResources`] bag — but each
//! stage instead configures the values the static WGSL shaders actually
//! consume: the accumulated vertex `attributes` + `index_buffer`, the material
//! base color / texture, the resolved lighting model, and the derived
//! `render_state` / `pass`.
//!
//! Only the stages that map onto the ported render path are wired in
//! (`Geometry` → `Material` → `ModelColor` → `Lighting` → `Alpha`). The many
//! conditional CesiumJS stages whose dependencies are not yet ported
//! (`Wireframe`, `Classification`, `MorphTargets`, `Skinning`, `PointCloud`,
//! `Dequantization`, `Imagery`, `FeatureId`, `Metadata`, `CustomShader`,
//! `Picking`, `Outline`, `Instancing`, `VerticalExaggeration`,
//! `PrimitiveStatistics`, `SceneMode2D`, `Tileset`) are deferred — see
//! `docs/deviations.md`.

use cesium_core::color::Color;
use cesium_core::runtime_error::RuntimeError;
use cesium_renderer::buffer::Buffer;
use cesium_renderer::buffer_usage::BufferUsage;
use cesium_renderer::context::Context;
use cesium_renderer::pass::Pass;
use cesium_renderer::vertex_array::VertexAttribute;

use crate::gltf_loader::{GltfJson, GltfPrimitive};
use crate::gltf_loader_util::GltfLoaderUtil;
use crate::gltf_vertex_buffer_loader::{GltfVertexBufferLoader, GltfVertexBufferLoaderOptions};
use crate::model::alpha_pipeline_stage::AlphaPipelineStage;
use crate::model::geometry_pipeline_stage::GeometryPipelineStage;
use crate::model::lighting_pipeline_stage::LightingPipelineStage;
use crate::model::material_pipeline_stage::MaterialPipelineStage;
use crate::model::model::ColorBlendMode;
use crate::model::model_color_pipeline_stage::ModelColorPipelineStage;
use crate::model::primitive_render_resources::PrimitiveRenderResources;

/// The immutable inputs shared by every stage while processing one glTF
/// primitive.
///
/// Rust analogue of the CesiumJS `(renderResources, primitive, frameState)`
/// stage arguments, flattened to the values the ported stages actually read:
/// the parsed glTF asset + primitive, the GPU `context` for buffer/texture
/// upload, and the model-level appearance flags (`color`, `colorBlendMode`,
/// `enableLighting`, `backFaceCulling`, `opaquePass`).
pub struct PipelineContext<'a> {
    /// The parsed glTF asset.
    pub gltf: &'a GltfJson,
    /// The glTF primitive being processed.
    pub primitive: &'a GltfPrimitive,
    /// The index of the scene-graph node that owns this primitive.
    pub node_index: usize,
    /// The renderer context (GPU device/queue) for buffer + texture upload.
    pub context: &'a Context,
    /// The model color blended into the base color (mirrors `model.color`).
    pub model_color: Color,
    /// The model color blend mode (mirrors `model.colorBlendMode`).
    pub color_blend_mode: ColorBlendMode,
    /// The model color blend amount (mirrors `model.colorBlendAmount`).
    pub color_blend_amount: f64,
    /// Whether lighting is enabled (mirrors `model.enableLighting`).
    pub enable_lighting: bool,
    /// Whether back-face culling is enabled (mirrors `model.backFaceCulling`;
    /// finalized per-frame in the draw path, carried here for fidelity).
    pub back_face_culling: bool,
    /// The pass used for opaque primitives (mirrors `model.opaquePass`).
    pub opaque_pass: Pass,
}

/// A pipeline stage's `process` entry point: mutates the render-resource bag
/// in place. Mirrors the CesiumJS static `process(renderResources, primitive,
/// frameState)` signature (adapted to the flattened [`PipelineContext`]).
pub type StageProcess =
    fn(&mut PrimitiveRenderResources, &PipelineContext) -> Result<(), RuntimeError>;

/// Assembles the ordered pipeline for one primitive.
///
/// Rust analogue of CesiumJS `ModelRuntimePrimitive.configurePipeline`
/// (`ModelRuntimePrimitive.js` L192-344), trimmed to the stages that map onto
/// the ported static-WGSL render path. The order mirrors the JS chain:
/// geometry first (attributes/indices), then material (base color/texture +
/// alpha/lighting options), then the model color (may force the translucent
/// pass), then lighting (resolve the lighting model), then alpha (finalize the
/// render state + pass). The JS `ModelColorPipelineStage` is a *model*-level
/// stage; it is folded into this per-primitive chain so the model color can
/// influence the primitive pass, matching the JS effect.
pub fn configure_pipeline(_ctx: &PipelineContext) -> Vec<(&'static str, StageProcess)> {
    vec![
        (GeometryPipelineStage::NAME, GeometryPipelineStage::process as StageProcess),
        (MaterialPipelineStage::NAME, MaterialPipelineStage::process as StageProcess),
        (
            ModelColorPipelineStage::NAME,
            ModelColorPipelineStage::process as StageProcess,
        ),
        (LightingPipelineStage::NAME, LightingPipelineStage::process as StageProcess),
        (AlphaPipelineStage::NAME, AlphaPipelineStage::process as StageProcess),
    ]
}

/// Creates one GPU vertex attribute from a glTF accessor (buffer view bytes
/// uploaded through [`GltfVertexBufferLoader`]).
///
/// Shared by the geometry stage (POSITION) and the material stage
/// (TEXCOORD_0). Moved verbatim from the former inline `Model` helper so the
/// stages own the GPU upload, mirroring the JS stages' vertex-array setup.
///
/// DEVIATION: when the accessor has a non-zero `byteOffset`, the wgpu port
/// slices the buffer data starting at that offset and sets the GPU attribute
/// offset to zero. This avoids a wgpu validation pitfall where
/// `attribute.offset + format.size()` must not exceed `array_stride` — the JS
/// path uses `gl.vertexAttribPointer` which accepts arbitrary byte offsets
/// without this constraint.
pub(crate) fn create_vertex_attribute(
    gltf: &GltfJson,
    context: &Context,
    semantic: &str,
    accessor_id: u32,
    location: u32,
) -> Result<VertexAttribute, RuntimeError> {
    let accessor = gltf
        .accessors
        .get(accessor_id as usize)
        .ok_or_else(|| {
            RuntimeError::new(Some(&format!(
                "{semantic} accessor {accessor_id} is out of range."
            )))
        })?;
    let format = GltfLoaderUtil::vertex_format(accessor).ok_or_else(|| {
        RuntimeError::new(Some(&format!(
            "{semantic} accessor type {} (componentType {}) has no GPU vertex format.",
            accessor.gl_type, accessor.component_type
        )))
    })?;
    let buffer_view_id = accessor.buffer_view.ok_or_else(|| {
        RuntimeError::new(Some(&format!(
            "{semantic} accessor {accessor_id} has no bufferView."
        )))
    })?;
    let buffer_view = gltf.buffer_views.get(buffer_view_id as usize).ok_or_else(|| {
        RuntimeError::new(Some(&format!(
            "{semantic} bufferView {buffer_view_id} is out of range."
        )))
    })?;
    let stride = buffer_view
        .byte_stride
        .unwrap_or_else(|| GltfLoaderUtil::accessor_element_stride(accessor));

    let mut loader = GltfVertexBufferLoader::try_new(GltfVertexBufferLoaderOptions {
        buffer_view_id: Some(buffer_view_id),
        primitive: None,
        draco: None,
        spz: None,
        attribute_semantic: Some(semantic.to_string()),
        accessor_id: Some(accessor_id),
        cache_key: None,
        load_buffer: true,
        load_typed_array: true,
    })?;
    loader.load(gltf)?;

    // When the accessor has a non-zero byteOffset, slice the pending bytes
    // starting at that offset so the GPU attribute offset is zero (satisfies
    // wgpu's offset + format.size() <= stride check).
    let byte_offset = accessor.byte_offset;
    if byte_offset > 0 {
        let full_bytes = loader.typed_array().ok_or_else(|| {
            RuntimeError::new(Some(&format!(
                "Failed to read {semantic} typed array for byte-offset slicing."
            )))
        })?;
        let sliced = full_bytes[byte_offset as usize..].to_vec();
        // Replace the pending upload bytes with the sliced data.
        let _ = loader.take_buffer(); // discard any existing buffer
        // Re-create with sliced bytes through a fresh buffer.
        let buffer = Buffer::create_vertex_buffer(
            context.device(),
            Some(&sliced),
            None,
            BufferUsage::StaticDraw,
        );
        // Upload immediately since we have the context's queue.
        let mut buffer = buffer;
        buffer.upload_pending_data(context.queue());
        return Ok(VertexAttribute {
            index: location,
            buffer,
            components_per_attribute: GltfLoaderUtil::number_of_components_for_type(
                &accessor.gl_type,
            ),
            component_datatype: format,
            normalize: accessor.normalized,
            stride_in_bytes: stride,
            offset_in_bytes: 0,
        });
    }

    loader.create_buffer(context)?;
    let buffer = loader.take_buffer().ok_or_else(|| {
        RuntimeError::new(Some(&format!(
            "Failed to create {semantic} vertex buffer."
        )))
    })?;

    Ok(VertexAttribute {
        index: location,
        buffer,
        components_per_attribute: GltfLoaderUtil::number_of_components_for_type(&accessor.gl_type),
        component_datatype: format,
        normalize: accessor.normalized,
        stride_in_bytes: stride,
        offset_in_bytes: 0,
    })
}
