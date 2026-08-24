//! Ported from `packages/engine/Source/Scene/Model/ModelRuntimePrimitive.js`.
//!
//! A runtime primitive in a model: one glTF primitive's GPU resources plus
//! the material state needed to assemble its [`DrawCommand`].

use std::sync::Arc;

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::webgl_constants::WebGLConstants;
use cesium_renderer::texture::Texture;
use cesium_renderer::vertex_array::VertexArray;

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
