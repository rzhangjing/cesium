//! Ported from `packages/engine/Source/Scene/Model/MaterialPipelineStage.js`.
//!
//! DEVIATION (B3.5, adapted pipeline): CesiumJS `MaterialPipelineStage` walks
//! the glTF material's PBR / emissive / normal / occlusion / KHR extensions and
//! appends the corresponding GLSL uniforms, varyings, and fragment lines to the
//! `ShaderBuilder`. The wgpu port renders through a fixed pair of base-color
//! WGSL shaders, so this stage resolves the values those shaders consume: the
//! base color factor, the base color texture (+ its TEXCOORD_0 attribute), the
//! double-sided flag, and the `alphaOptions` / `lightingOptions` that the later
//! alpha / lighting stages read. Emissive / normal / occlusion / metallic /
//! roughness textures and the KHR material extensions are deferred (the static
//! shaders do not sample them).

use std::sync::Arc;

use cesium_core::runtime_error::RuntimeError;
use cesium_renderer::context::Context;
use cesium_renderer::pass::Pass;
use cesium_renderer::texture::Texture;

use crate::gltf_loader::GltfJson;
use crate::gltf_texture_loader::{GltfTextureLoader, GltfTextureLoaderOptions};
use crate::model::lighting_model::LightingModel;
use crate::model::model_pipeline_stage::{create_vertex_attribute, PipelineContext};
use crate::model::primitive_render_resources::PrimitiveRenderResources;

/// Pipeline stage for material processing.
///
/// Resolves the base color factor / texture, double-sided flag, and the alpha /
/// lighting options for one glTF primitive (adapted from the CesiumJS
/// `MaterialPipelineStage`).
pub struct MaterialPipelineStage;

impl MaterialPipelineStage {
    /// The stage name (mirrors the CesiumJS stage constructor name).
    pub const NAME: &'static str = "MaterialPipelineStage";

    /// Processes one primitive's material into the render-resource bag.
    ///
    /// Mirrors the CesiumJS `MaterialPipelineStage.process` for the subset the
    /// ported base-color shaders consume.
    pub fn process(
        render_resources: &mut PrimitiveRenderResources,
        ctx: &PipelineContext,
    ) -> Result<(), RuntimeError> {
        let material = ctx
            .primitive
            .material
            .and_then(|material_id| ctx.gltf.materials.get(material_id as usize));
        let pbr = material.and_then(|material| material.pbr_metallic_roughness.as_ref());

        // ---- base color factor (defaults to opaque white) ----
        render_resources.base_color_factor = match pbr {
            Some(pbr) => [
                pbr.base_color_factor[0] as f32,
                pbr.base_color_factor[1] as f32,
                pbr.base_color_factor[2] as f32,
                pbr.base_color_factor[3] as f32,
            ],
            None => [1.0, 1.0, 1.0, 1.0],
        };

        // ---- double sided (disables back-face culling) ----
        render_resources.double_sided = material.map(|material| material.double_sided).unwrap_or(false);

        // ---- lighting model: PBR when a metallic-roughness material exists ----
        // (Mirrors the JS material stage selecting the lighting model; the
        // resolved model is recorded by the lighting stage. DEVIATION: the
        // static WGSL shaders shade unlit, so PBR is deferred.)
        render_resources.lighting_options.lighting_model = if pbr.is_some() {
            LightingModel::Pbr
        } else {
            LightingModel::Unlit
        };

        // ---- alpha options from the glTF alphaMode ----
        // (Mirrors the JS material stage setting `alphaOptions.pass` for BLEND
        // and `alphaOptions.alphaCutoff` for MASK.)
        if let Some(material) = material {
            match material.alpha_mode.as_str() {
                "BLEND" => render_resources.alpha_options.pass = Some(Pass::Translucent),
                "MASK" => {
                    render_resources.alpha_options.alpha_cutoff = Some(material.alpha_cutoff as f32)
                }
                _ => {}
            }
        }

        // ---- base color texture + TEXCOORD_0 attribute ----
        if let Some(pbr) = pbr {
            if let Some(info) = &pbr.base_color_texture {
                let texcoord_id = ctx.primitive.attributes.get("TEXCOORD_0");
                if info.tex_coord != 0 {
                    log::warn!(
                        "DEVIATION: baseColorTexture texCoord set {} is deferred \
                         (only TEXCOORD_0 is supported).",
                        info.tex_coord
                    );
                } else if let Some(texcoord_id) = texcoord_id {
                    match load_base_color_texture(ctx.gltf, ctx.context, info.index) {
                        Ok(texture) => {
                            // TEXCOORD_0 takes the next free attribute location
                            // (1, after POSITION).
                            let location = render_resources.attribute_index() as u32;
                            let attribute = create_vertex_attribute(
                                ctx.gltf,
                                ctx.context,
                                "TEXCOORD_0",
                                *texcoord_id,
                                location,
                            )?;
                            render_resources.add_attribute(attribute);
                            render_resources.base_color_texture = Some(texture);
                            render_resources.textured = true;
                        }
                        Err(error) => {
                            log::warn!(
                                "model base color texture {} deferred: {}",
                                info.index,
                                error.message
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

/// Loads one base color texture through the [`GltfTextureLoader`] GPU path
/// (embedded images only — external URIs stay deferred per the T4
/// caller-injection contract). Moved from the former inline `Model` helper so
/// the material stage owns the texture upload.
fn load_base_color_texture(
    gltf: &GltfJson,
    context: &Context,
    texture_id: u32,
) -> Result<Arc<Texture>, RuntimeError> {
    let mut loader = GltfTextureLoader::try_new(
        gltf,
        GltfTextureLoaderOptions {
            texture_id,
            cache_key: None,
        },
    )?;
    loader.load(gltf)?;
    loader.create_texture(context, gltf)?;
    loader.texture().ok_or_else(|| {
        RuntimeError::new(Some(&format!(
            "Texture {texture_id} produced no GPU texture."
        )))
    })
}
