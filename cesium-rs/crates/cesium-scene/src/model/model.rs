//! Ported from `packages/engine/Source/Scene/Model/Model.js`.
//!
//! A 3D model based on glTF.
//!
//! T5 materialization: the glTF asset is kept as parsed JSON and, on the
//! first `update` with a renderer context, the runtime primitives are
//! built — one GPU vertex array per glTF primitive (vertex/index buffers
//! through the [`GltfVertexBufferLoader`] / [`GltfIndexBufferLoader`] GPU
//! paths, base color textures through [`GltfTextureLoader`]) — and each
//! frame generates one [`DrawCommand`] per primitive through the model
//! WGSL pairs (color / textured), folding the scene-graph node world
//! transform into the per-draw model matrix.
//!
//! DEVIATION: the CesiumJS pipeline stage chain (lighting, PBR metallic
//! roughness, skinning, morph targets, custom shaders, point cloud
//! shading, clipping, silhouette, wireframe) is deferred; the wgpu port
//! shades with the base color factor (× base color texture when present).

use std::sync::Arc;

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;
use cesium_core::event::Event;
use cesium_core::matrix4::Matrix4;
use cesium_core::quaternion::Quaternion;
use cesium_core::runtime_error::RuntimeError;
use cesium_core::webgl_constants::WebGLConstants;
use cesium_renderer::buffer::Buffer;
use cesium_renderer::buffer_usage::BufferUsage;
use cesium_renderer::context::Context;
use cesium_renderer::draw_command::{DrawCommand, UniformValue};
use cesium_renderer::pass::Pass;
use cesium_renderer::render_state::{BlendEquation, BlendingFactor, RenderState};
use cesium_renderer::shader_program::ShaderProgram;
use cesium_renderer::texture::Texture;
use cesium_renderer::vertex_array::{VertexArray, VertexAttribute};
use cesium_shaders::wgsl;

use crate::frame_state::FrameState;
use crate::gltf_index_buffer_loader::{GltfIndexBufferLoader, GltfIndexBufferLoaderOptions};
use crate::gltf_loader::{GltfAccessor, GltfJson, GltfNode, GltfPrimitive};
use crate::gltf_loader_util::GltfLoaderUtil;
use crate::gltf_texture_loader::{GltfTextureLoader, GltfTextureLoaderOptions};
use crate::gltf_vertex_buffer_loader::{GltfVertexBufferLoader, GltfVertexBufferLoaderOptions};
use crate::model::model_node::ModelNode;
use crate::model::model_runtime_primitive::ModelRuntimePrimitive;
use crate::model::model_scene_graph::ModelSceneGraph;
use crate::primitive_collection::ScenePrimitive;
use crate::shadow_mode::ShadowMode;

/// A 3D model based on glTF, the runtime asset format for WebGL, OpenGL ES, and OpenGL.
///
/// Use [`Model::from_gltf`] to construct from a parsed glTF asset.
/// Mirrors CesiumJS `Model` (3376 lines).
pub struct Model {
    // ---- identity ----
    /// A user-defined ID for this model.
    pub id: Option<String>,
    /// The type of model (GLTF, B3DM, I3DM, PNTS, GEOJSON).
    pub model_type: ModelType,

    // ---- transform ----
    /// The 4x4 transformation matrix from model to world coordinates.
    pub model_matrix: Matrix4,
    /// A uniform scale applied to this model.
    pub scale: f64,
    /// The minimum pixel size of the model regardless of zoom.
    pub minimum_pixel_size: f64,
    /// The maximum scale size of the model.
    pub maximum_scale: Option<f64>,

    // ---- appearance ----
    /// Whether the model is shown.
    pub show: bool,
    /// The color to blend with the model's base color.
    pub color: Color,
    /// The color blend mode.
    pub color_blend_mode: ColorBlendMode,
    /// The color blend amount (0.0 to 1.0).
    pub color_blend_amount: f64,
    /// The silhouette color.
    pub silhouette_color: Color,
    /// The silhouette size.
    pub silhouette_size: f64,
    /// The shadow mode.
    pub shadows: ShadowMode,
    /// The split direction.
    pub split_direction: SplitDirection,
    /// Whether the model has a custom shader.
    pub has_custom_shader: bool,

    // ---- lighting ----
    /// Whether lighting is enabled.
    pub enable_lighting: bool,
    /// The image-based lighting intensity.
    pub image_based_lighting_intensity: f64,
    /// Whether to use image-based lighting.
    pub use_image_based_lighting: bool,
    /// Whether to use specular environment maps.
    pub use_specular_environment_maps: bool,
    /// Whether to use diffuse environment maps.
    pub use_diffuse_environment_maps: bool,

    // ---- point cloud ----
    /// The point cloud shading attenuation distance.
    pub point_cloud_shading_attenuation: bool,

    // ---- clipping ----
    /// Whether back-face culling is enabled.
    pub back_face_culling: bool,
    /// Whether to show debug wireframe.
    pub debug_wireframe: bool,
    /// Whether to show debug bounding volume.
    pub debug_bounding_volume: bool,

    // ---- state ----
    /// Whether the model is ready (loaded and processed).
    pub ready: bool,
    /// Whether the model has been destroyed.
    is_destroyed: bool,
    /// The bounding sphere of the model (model-local coordinates).
    pub bounding_sphere: BoundingSphere,
    /// The active time for animations.
    pub active_time: f64,
    /// Whether vertical exaggeration is enabled.
    pub enable_vertical_exaggeration: bool,

    // ---- events ----
    /// Event raised when the model is ready.
    pub ready_event: Event,

    // ---- runtime (wgpu) ----
    /// The parsed glTF asset (kept for deferred GPU resource creation).
    gltf: Option<GltfJson>,
    /// The scene graph (node hierarchy + world transforms).
    scene_graph: ModelSceneGraph,
    /// The runtime primitives (one per glTF primitive).
    runtime_primitives: Vec<ModelRuntimePrimitive>,
    /// Whether the runtime primitives have been created.
    runtime_built: bool,
    /// The flat base-color shader pair.
    color_program: Option<Arc<ShaderProgram>>,
    /// The textured base-color shader pair.
    textured_program: Option<Arc<ShaderProgram>>,
}

/// The type of a 3D model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelType {
    /// A standard glTF model.
    Gltf,
    /// Batched 3D Model.
    B3dm,
    /// Instanced 3D Model.
    I3dm,
    /// Point Cloud.
    Pnts,
    /// GeoJSON vector tile.
    GeoJson,
}

/// The color blend mode for a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorBlendMode {
    /// Highlight: blend between original and highlight color.
    Highlight,
    /// Replace: replace original color entirely.
    Replace,
    /// Mix: mix original and highlight color.
    Mix,
}

/// The split direction for a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    /// Render on the left side.
    Left,
    /// Render on both sides.
    None,
    /// Render on the right side.
    Right,
}

/// The local transform of a glTF node: the explicit column-major `matrix`
/// when present, else the TRS composition (glTF spec defaults: identity
/// translation/rotation, unit scale).
fn node_local_matrix(node: &GltfNode) -> Matrix4 {
    if let Some(matrix) = &node.matrix {
        return Matrix4::from_column_major_array_new(matrix);
    }
    let translation = node
        .translation
        .map(|t| Cartesian3::new(t[0], t[1], t[2]))
        .unwrap_or(Cartesian3::ZERO);
    let rotation = node
        .rotation
        .map(|r| Quaternion::new(r[0], r[1], r[2], r[3]))
        .unwrap_or(Quaternion::IDENTITY);
    let scale = node
        .scale
        .map(|s| Cartesian3::new(s[0], s[1], s[2]))
        .unwrap_or(Cartesian3::new(1.0, 1.0, 1.0));
    Matrix4::from_translation_quaternion_rotation_scale_new(&translation, &rotation, &scale)
}

/// The bounding sphere of a POSITION accessor from its glTF `min`/`max`
/// (zero sphere when the asset omits them).
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

impl Model {
    /// Creates a new Model with default values.
    pub fn new() -> Self {
        Self {
            id: None,
            model_type: ModelType::Gltf,
            model_matrix: Matrix4::IDENTITY,
            scale: 1.0,
            minimum_pixel_size: 0.0,
            maximum_scale: None,
            show: true,
            color: Color::new(1.0, 1.0, 1.0, 1.0),
            color_blend_mode: ColorBlendMode::Highlight,
            color_blend_amount: 0.0,
            silhouette_color: Color::new(1.0, 1.0, 1.0, 1.0),
            silhouette_size: 0.0,
            shadows: ShadowMode::Enabled,
            split_direction: SplitDirection::None,
            has_custom_shader: false,
            enable_lighting: true,
            image_based_lighting_intensity: 1.0,
            use_image_based_lighting: true,
            use_specular_environment_maps: false,
            use_diffuse_environment_maps: false,
            point_cloud_shading_attenuation: true,
            back_face_culling: true,
            debug_wireframe: false,
            debug_bounding_volume: false,
            ready: false,
            is_destroyed: false,
            bounding_sphere: BoundingSphere::new(Cartesian3::ZERO, 0.0),
            active_time: 0.0,
            enable_vertical_exaggeration: true,
            ready_event: Event::new(),
            gltf: None,
            scene_graph: ModelSceneGraph::new(),
            runtime_primitives: Vec::new(),
            runtime_built: false,
            color_program: None,
            textured_program: None,
        }
    }

    /// Creates a model from a parsed glTF asset.
    ///
    /// Rust analogue of CesiumJS `Model.fromGltfAsync` with the fetch and
    /// resource-cache pipeline already resolved by the caller: the JSON is
    /// kept and the GPU resources are created on the first `update`
    /// (mirroring the JS `ready` promise resolving before first draw).
    pub fn from_gltf(gltf: GltfJson) -> Model {
        let mut model = Model::new();

        // Scene graph: one ModelNode per glTF node, in glTF index order.
        let mut referenced: Vec<bool> = vec![false; gltf.nodes.len()];
        for node in &gltf.nodes {
            for child in &node.children {
                if (*child as usize) < referenced.len() {
                    referenced[*child as usize] = true;
                }
            }
        }
        for (index, node) in gltf.nodes.iter().enumerate() {
            let mut model_node = ModelNode::new(node.name.as_deref().unwrap_or(""));
            model_node.node_index = index;
            model_node.matrix = node_local_matrix(node);
            model_node.children = node.children.iter().map(|child| *child as usize).collect();
            model.scene_graph.add_node(model_node);
        }
        let roots: Vec<usize> = match gltf
            .scene
            .and_then(|scene_id| gltf.scenes.get(scene_id as usize))
        {
            Some(scene) => scene.nodes.iter().map(|node| *node as usize).collect(),
            // No default scene: the glTF spec leaves the roots implicit —
            // use the nodes that no other node references as children.
            None => (0..gltf.nodes.len()).filter(|index| !referenced[*index]).collect(),
        };
        model.scene_graph.set_root_nodes(roots);

        model.gltf = Some(gltf);
        model
    }

    /// Returns the scene graph (nodes + world transforms).
    pub fn scene_graph(&self) -> &ModelSceneGraph {
        &self.scene_graph
    }

    /// Returns the runtime primitives (defined after the first GPU build).
    pub fn runtime_primitives(&self) -> &[ModelRuntimePrimitive] {
        &self.runtime_primitives
    }

    /// Updates the model for the current frame.
    ///
    /// Mirrors CesiumJS `Model#update`: lazily finish GPU resource
    /// creation on first use, propagate the scene-graph transforms, and
    /// append one draw command per runtime primitive.
    pub fn update(&mut self, frame_state: &FrameState, context: &mut Context) {
        if !self.show {
            return;
        }
        if !frame_state.passes.main {
            return;
        }

        if !self.runtime_built {
            let gltf = match self.gltf.take() {
                Some(gltf) => gltf,
                None => return,
            };
            self.create_runtime(&gltf, context);
            self.gltf = Some(gltf);
            if !self.ready {
                return;
            }
        }

        self.scene_graph.update(frame_state);

        for primitive in &self.runtime_primitives {
            let vertex_array = match &primitive.vertex_array {
                Some(vertex_array) => vertex_array.clone(),
                None => continue,
            };
            if primitive.count == 0 {
                continue;
            }
            // Visibility follows the owning node (JS runtime-node show).
            match self.scene_graph.get_node(primitive.node_index) {
                Some(node) if !node.show => continue,
                _ => {}
            }
            let shader_program = if primitive.is_textured() {
                match self.textured_program.clone() {
                    Some(program) => program,
                    None => continue,
                }
            } else {
                match self.color_program.clone() {
                    Some(program) => program,
                    None => continue,
                }
            };

            // Per-draw model matrix: model matrix × node world transform ×
            // uniform scale (mirrors the JS ModelDrawCommand composition).
            let node_world = self
                .scene_graph
                .world_matrix(primitive.node_index)
                .unwrap_or(Matrix4::IDENTITY);
            let mut model_to_world = Matrix4::multiply_new(&self.model_matrix, &node_world);
            if self.scale != 1.0 {
                let mut scaled = Matrix4::IDENTITY;
                Matrix4::multiply_by_uniform_scale(&model_to_world, self.scale, &mut scaled);
                model_to_world = scaled;
            }
            let world_bounding_sphere = BoundingSphere::transform(
                &primitive.bounding_sphere,
                &model_to_world,
                None,
            );

            // Render state: opaque depth-tested geometry by default;
            // double-sided materials and the model's backFaceCulling option
            // control culling; BLEND materials take the translucent path.
            let mut render_state = RenderState::default();
            render_state.depth_test.enabled = true;
            render_state.depth_mask = !primitive.translucent;
            render_state.cull.enabled = self.back_face_culling && !primitive.double_sided;
            let pass = if primitive.translucent {
                render_state.blending.enabled = true;
                render_state.blending.equation_rgb = BlendEquation::FuncAdd;
                render_state.blending.equation_alpha = BlendEquation::FuncAdd;
                render_state.blending.function_source_rgb = BlendingFactor::SrcAlpha;
                render_state.blending.function_source_alpha = BlendingFactor::One;
                render_state.blending.function_destination_rgb = BlendingFactor::OneMinusSrcAlpha;
                render_state.blending.function_destination_alpha = BlendingFactor::OneMinusSrcAlpha;
                Pass::Translucent as u32
            } else {
                Pass::Opaque as u32
            };

            // Base color factor blended with the model color (DEVIATION:
            // colorBlendMode nuances beyond the multiply are deferred).
            let base_color_factor = [
                primitive.base_color_factor[0] * self.color.red as f32,
                primitive.base_color_factor[1] * self.color.green as f32,
                primitive.base_color_factor[2] * self.color.blue as f32,
                primitive.base_color_factor[3] * self.color.alpha as f32,
            ];
            let mut uniform_overrides = vec![(
                "u_baseColorFactor".to_string(),
                UniformValue::Vec4(base_color_factor),
            )];
            if let Some(texture) = &primitive.base_color_texture {
                uniform_overrides.push((
                    "u_baseColorTexture".to_string(),
                    UniformValue::Texture(texture.clone()),
                ));
            }

            let mut command = DrawCommand::new();
            command.bounding_volume = Some(world_bounding_sphere);
            command.model_matrix = Some(model_to_world);
            command.primitive_type = primitive.primitive_type;
            command.vertex_array = Some(vertex_array);
            command.count = Some(primitive.count);
            command.offset = primitive.offset;
            command.shader_program = Some(shader_program);
            command.uniform_overrides = uniform_overrides;
            command.render_state = render_state;
            command.framebuffer = None;
            command.pass = Some(pass);
            command.owner = Some("Model".to_string());
            context.draw(command);
        }
    }

    /// Creates the GPU runtime resources from the parsed glTF (first-use
    /// batch; mirrors the JS pipeline stages' GPU upload jobs).
    fn create_runtime(&mut self, gltf: &GltfJson, context: &Context) {
        // Shader programs (mirrors the JS ModelShader compilation step,
        // trimmed to the color / textured base color pairs).
        if self.color_program.is_none() {
            match ShaderProgram::from_wgsl(
                wgsl::MODEL_COLOR_VS,
                wgsl::MODEL_COLOR_FS,
                "model_color".to_string(),
            ) {
                Ok(program) => self.color_program = Some(Arc::new(program)),
                Err(error) => {
                    log::error!("model color shader compilation failed: {error}");
                    return;
                }
            }
        }
        if self.textured_program.is_none() {
            match ShaderProgram::from_wgsl(
                wgsl::MODEL_TEXTURED_VS,
                wgsl::MODEL_TEXTURED_FS,
                "model_textured".to_string(),
            ) {
                Ok(program) => self.textured_program = Some(Arc::new(program)),
                Err(error) => {
                    log::error!("model textured shader compilation failed: {error}");
                    return;
                }
            }
        }

        let mut model_sphere: Option<BoundingSphere> = None;
        for (node_index, node) in gltf.nodes.iter().enumerate() {
            let mesh_id = match node.mesh {
                Some(mesh_id) => mesh_id as usize,
                None => continue,
            };
            let mesh = match gltf.meshes.get(mesh_id) {
                Some(mesh) => mesh,
                None => continue,
            };
            for primitive in &mesh.primitives {
                match self.build_runtime_primitive(gltf, context, primitive, node_index) {
                    Ok(runtime_primitive) => {
                        let sphere = runtime_primitive.bounding_sphere.clone();
                        model_sphere = Some(match model_sphere {
                            Some(existing) => BoundingSphere::union(&existing, &sphere, None),
                            None => sphere,
                        });
                        self.runtime_primitives.push(runtime_primitive);
                    }
                    Err(error) => {
                        // Per-primitive failures skip the primitive
                        // (mirrors the JS stage-chain error logging).
                        log::warn!("model primitive skipped: {}", error.message);
                    }
                }
            }
        }

        if let Some(sphere) = model_sphere {
            self.bounding_sphere = sphere;
        }
        self.runtime_built = true;
        self.ready = !self.runtime_primitives.is_empty();
        if self.ready {
            self.ready_event.raise_event(&());
        }
    }

    /// Builds the GPU resources of one glTF primitive.
    ///
    /// DEVIATION: attributes sharing one bufferView are uploaded as
    /// separate GPU buffers (the JS interleaves them in a single buffer);
    /// NORMAL and other non-POSITION/TEXCOORD_0 semantics are skipped
    /// because the trimmed shader pairs do not consume them (lighting
    /// deferred).
    fn build_runtime_primitive(
        &mut self,
        gltf: &GltfJson,
        context: &Context,
        primitive: &GltfPrimitive,
        node_index: usize,
    ) -> Result<ModelRuntimePrimitive, RuntimeError> {
        if primitive.mode != WebGLConstants::TRIANGLES {
            return Err(RuntimeError::new(Some(&format!(
                "Primitive mode {} is not supported yet (only TRIANGLES).",
                primitive.mode
            ))));
        }

        let position_id = *primitive.attributes.get("POSITION").ok_or_else(|| {
            RuntimeError::new(Some("Primitive has no POSITION attribute."))
        })?;
        let position = gltf
            .accessors
            .get(position_id as usize)
            .ok_or_else(|| RuntimeError::new(Some("POSITION accessor is out of range.")))?;

        // ---- material resolution (base color factor/texture only) ----
        let material = primitive
            .material
            .and_then(|material_id| gltf.materials.get(material_id as usize));
        let pbr = material.and_then(|material| material.pbr_metallic_roughness.as_ref());
        let base_color_factor = match pbr {
            Some(pbr) => [
                pbr.base_color_factor[0] as f32,
                pbr.base_color_factor[1] as f32,
                pbr.base_color_factor[2] as f32,
                pbr.base_color_factor[3] as f32,
            ],
            None => [1.0, 1.0, 1.0, 1.0],
        };
        let double_sided = material.map(|material| material.double_sided).unwrap_or(false);
        let translucent = material
            .map(|material| material.alpha_mode == "BLEND")
            .unwrap_or(false);

        let mut textured = false;
        let mut base_color_texture: Option<Arc<Texture>> = None;
        if let Some(pbr) = pbr {
            if let Some(info) = &pbr.base_color_texture {
                let texcoord_id = primitive.attributes.get("TEXCOORD_0");
                if info.tex_coord != 0 {
                    log::warn!(
                        "DEVIATION: baseColorTexture texCoord set {} is deferred \
                         (only TEXCOORD_0 is supported).",
                        info.tex_coord
                    );
                } else if texcoord_id.is_some() {
                    match Self::load_base_color_texture(gltf, context, info.index) {
                        Ok(texture) => {
                            base_color_texture = Some(texture);
                            textured = true;
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

        // ---- vertex attributes (POSITION → location 0, TEXCOORD_0 → 1) ----
        let mut attributes = vec![Self::create_vertex_attribute(
            gltf, context, "POSITION", position_id, 0,
        )?];
        if textured {
            let texcoord_id = primitive.attributes["TEXCOORD_0"];
            attributes.push(Self::create_vertex_attribute(
                gltf, context, "TEXCOORD_0", texcoord_id, 1,
            )?);
        }

        // ---- index buffer ----
        let (index_buffer, count) = match primitive.indices {
            Some(indices_id) => {
                let mut loader = GltfIndexBufferLoader::try_new(
                    gltf,
                    GltfIndexBufferLoaderOptions {
                        accessor_id: indices_id,
                        draco: None,
                        cache_key: None,
                        load_buffer: true,
                        load_typed_array: false,
                    },
                )?;
                loader.load(gltf)?;
                loader.create_buffer(context)?;
                let count = gltf
                    .accessors
                    .get(indices_id as usize)
                    .map(|accessor| accessor.count)
                    .unwrap_or(0);
                (loader.take_buffer(), count)
            }
            None => (None, position.count),
        };

        Ok(ModelRuntimePrimitive {
            vertex_array: Some(Arc::new(VertexArray::new(attributes, index_buffer))),
            count,
            offset: 0,
            primitive_type: WebGLConstants::TRIANGLES,
            base_color_factor,
            base_color_texture,
            textured,
            double_sided,
            translucent,
            node_index,
            bounding_sphere: position_bounding_sphere(position),
        })
    }

    /// Creates one GPU vertex attribute from a glTF accessor (buffer view
    /// bytes uploaded through [`GltfVertexBufferLoader`]).
    ///
    /// DEVIATION: when the accessor has a non-zero `byteOffset`, the wgpu
    /// port slices the buffer data starting at that offset and sets the
    /// GPU attribute offset to zero. This avoids a wgpu validation pitfall
    /// where `attribute.offset + format.size()` must not exceed
    /// `array_stride` — the JS path uses `gl.vertexAttribPointer` which
    /// accepts arbitrary byte offsets without this constraint.
    fn create_vertex_attribute(
        gltf: &GltfJson,
        context: &Context,
        semantic: &str,
        accessor_id: u32,
        location: u32,
    ) -> Result<VertexAttribute, RuntimeError> {
        let accessor = gltf
            .accessors
            .get(accessor_id as usize)
            .ok_or_else(|| RuntimeError::new(Some(&format!(
                "{semantic} accessor {accessor_id} is out of range."
            ))))?;
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
        let buffer_view = gltf
            .buffer_views
            .get(buffer_view_id as usize)
            .ok_or_else(|| RuntimeError::new(Some(&format!(
                "{semantic} bufferView {buffer_view_id} is out of range."
            ))))?;
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

        // When the accessor has a non-zero byteOffset, slice the pending
        // bytes starting at that offset so the GPU attribute offset is
        // zero (satisfies wgpu's offset + format.size() <= stride check).
        let byte_offset = accessor.byte_offset;
        if byte_offset > 0 {
            let full_bytes = loader.typed_array().ok_or_else(|| {
                RuntimeError::new(Some(&format!(
                    "Failed to read {semantic} typed array for byte-offset slicing."
                )))
            })?;
            let sliced = full_bytes[byte_offset as usize..].to_vec();
            // Replace the pending upload bytes with the sliced data.
            // The typed_array and pending_bytes are both replaced.
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
            components_per_attribute: GltfLoaderUtil::number_of_components_for_type(
                &accessor.gl_type,
            ),
            component_datatype: format,
            normalize: accessor.normalized,
            stride_in_bytes: stride,
            offset_in_bytes: 0,
        })
    }

    /// Loads one base color texture through the [`GltfTextureLoader`] GPU
    /// path (embedded images only — external URIs stay deferred per the T4
    /// caller-injection contract).
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

    /// Returns whether this model has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys this model and releases GPU resources.
    pub fn destroy(&mut self) {
        self.runtime_primitives.clear();
        self.color_program = None;
        self.textured_program = None;
        self.gltf = None;
        self.is_destroyed = true;
    }

    /// Gets a node by name (mirrors the JS `getNode` lookup by node name).
    pub fn get_node(&self, name: &str) -> Option<&ModelNode> {
        (0..self.scene_graph.nodes_count())
            .filter_map(|index| self.scene_graph.get_node(index))
            .find(|node| node.name == name)
    }
}

impl Default for Model {
    fn default() -> Self { Self::new() }
}

impl ScenePrimitive for Model {
    fn update(&mut self, frame_state: &FrameState, context: &mut Context) {
        Model::update(self, frame_state, context);
    }
    fn show(&self) -> bool { self.show }
    fn set_show(&mut self, show: bool) { self.show = show; }
    fn is_destroyed(&self) -> bool { Model::is_destroyed(self) }
    fn destroy(&mut self) { Model::destroy(self); }
}
