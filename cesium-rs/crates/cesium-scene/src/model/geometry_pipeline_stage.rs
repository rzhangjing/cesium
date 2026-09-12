//! Ported from `packages/engine/Source/Scene/Model/GeometryPipelineStage.js`.
//!
//! DEVIATION (B3.5, adapted pipeline): CesiumJS `GeometryPipelineStage` binds
//! attribute semantics to shader varyings and pushes `VertexAttribute`
//! *descriptors* into `renderResources.attributes` (the GPU buffers are created
//! later by the loader). The wgpu port has no ShaderBuilder, so this stage
//! performs the equivalent geometry setup directly: it uploads the POSITION
//! vertex buffer + index buffer through the glTF loaders, records the draw
//! `count` / POSITION `min` / `max`, and derives the model-local bounding
//! sphere. Material-driven attributes (TEXCOORD_0) are added by the material
//! stage, mirroring the JS split where the material stage adds texture
//! coordinate varyings.

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::runtime_error::RuntimeError;
use cesium_core::webgl_constants::WebGLConstants;

use crate::gltf_index_buffer_loader::{GltfIndexBufferLoader, GltfIndexBufferLoaderOptions};
use crate::gltf_loader::GltfAccessor;
use crate::model::model_pipeline_stage::{create_vertex_attribute, PipelineContext};
use crate::model::primitive_render_resources::PrimitiveRenderResources;

/// Pipeline stage for geometry processing.
///
/// Prepares the vertex/index buffers, draw count, and bounding volume for one
/// glTF primitive (adapted from the CesiumJS `GeometryPipelineStage`).
pub struct GeometryPipelineStage;

impl GeometryPipelineStage {
    /// The stage name (mirrors the CesiumJS stage constructor name, used for
    /// per-stage error logging in the pipeline chain).
    pub const NAME: &'static str = "GeometryPipelineStage";

    /// Processes one primitive's geometry into the render-resource bag.
    ///
    /// Mirrors the CesiumJS `GeometryPipelineStage.process`: validate the
    /// primitive mode, bind the POSITION attribute, and set the draw range +
    /// bounding volume. The GPU upload (absent from the JS stage, which only
    /// builds descriptors) is folded in here for the ported path.
    pub fn process(
        render_resources: &mut PrimitiveRenderResources,
        ctx: &PipelineContext,
    ) -> Result<(), RuntimeError> {
        let primitive = ctx.primitive;
        // Mode validation (mirrors the JS stage-chain TRIANGLES-only guard the
        // port applies; other topologies are deferred).
        if primitive.mode != WebGLConstants::TRIANGLES {
            return Err(RuntimeError::new(Some(&format!(
                "Primitive mode {} is not supported yet (only TRIANGLES).",
                primitive.mode
            ))));
        }

        let position_id = *primitive
            .attributes
            .get("POSITION")
            .ok_or_else(|| RuntimeError::new(Some("Primitive has no POSITION attribute.")))?;
        let position = ctx
            .gltf
            .accessors
            .get(position_id as usize)
            .ok_or_else(|| RuntimeError::new(Some("POSITION accessor is out of range.")))?;

        // POSITION occupies attribute location 0 (mirrors the JS `attributeIndex`
        // bookkeeping: the next free location).
        let location = render_resources.attribute_index() as u32;
        let attribute =
            create_vertex_attribute(ctx.gltf, ctx.context, "POSITION", position_id, location)?;
        render_resources.add_attribute(attribute);

        // POSITION min/max + bounding sphere (from the accessor's glTF min/max).
        render_resources.position_min = accessor_min(position);
        render_resources.position_max = accessor_max(position);
        render_resources.bounding_sphere = position_bounding_sphere(position);

        // Index buffer + draw count (indexed → indices.count, else vertices).
        match primitive.indices {
            Some(indices_id) => {
                let mut loader = GltfIndexBufferLoader::try_new(
                    ctx.gltf,
                    GltfIndexBufferLoaderOptions {
                        accessor_id: indices_id,
                        draco: None,
                        cache_key: None,
                        load_buffer: true,
                        load_typed_array: false,
                    },
                )?;
                loader.load(ctx.gltf)?;
                loader.create_buffer(ctx.context)?;
                render_resources.count = ctx
                    .gltf
                    .accessors
                    .get(indices_id as usize)
                    .map(|accessor| accessor.count)
                    .unwrap_or(0);
                render_resources.index_buffer = loader.take_buffer();
            }
            None => {
                render_resources.count = position.count;
            }
        }
        render_resources.primitive_type = WebGLConstants::TRIANGLES;
        render_resources.offset = 0;

        Ok(())
    }
}

/// The POSITION accessor `min` as a [`Cartesian3`] (zero when omitted/short).
fn accessor_min(accessor: &GltfAccessor) -> Cartesian3 {
    accessor
        .min
        .as_deref()
        .filter(|min| min.len() >= 3)
        .map(|min| Cartesian3::new(min[0], min[1], min[2]))
        .unwrap_or(Cartesian3::ZERO)
}

/// The POSITION accessor `max` as a [`Cartesian3`] (zero when omitted/short).
fn accessor_max(accessor: &GltfAccessor) -> Cartesian3 {
    accessor
        .max
        .as_deref()
        .filter(|max| max.len() >= 3)
        .map(|max| Cartesian3::new(max[0], max[1], max[2]))
        .unwrap_or(Cartesian3::ZERO)
}

/// The bounding sphere of a POSITION accessor from its glTF `min`/`max`
/// (center = midpoint, radius = half the diagonal; zero sphere when the asset
/// omits them). Mirrors the JS `BoundingSphere.fromCornerPoints` of the
/// accessor min/max corner.
fn position_bounding_sphere(accessor: &GltfAccessor) -> BoundingSphere {
    match (accessor.min.as_deref(), accessor.max.as_deref()) {
        (Some(min), Some(max)) if min.len() >= 3 && max.len() >= 3 => {
            let center = Cartesian3::new(
                (min[0] + max[0]) * 0.5,
                (min[1] + max[1]) * 0.5,
                (min[2] + max[2]) * 0.5,
            );
            let dx = max[0] - min[0];
            let dy = max[1] - min[1];
            let dz = max[2] - min[2];
            let radius = 0.5 * (dx * dx + dy * dy + dz * dz).sqrt();
            BoundingSphere::new(center, radius)
        }
        _ => BoundingSphere::new(Cartesian3::ZERO, 0.0),
    }
}
