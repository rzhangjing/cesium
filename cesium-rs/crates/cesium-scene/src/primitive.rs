//! Ported from `packages/engine/Source/Scene/Primitive.js`.
//!
//! A renderable primitive composed of geometry instances.
//!
//! B4/S3 materialization: the CPU half of the CesiumJS pipeline is ported
//! one-to-one — geometry instances run through the cesium-core
//! `GeometryPipeline` helpers (`transformToWorldCoordinates`,
//! `createAttributeLocations`) and become GPU vertex/index buffers feeding
//! per-instance [`DrawCommand`]s (one draw per instance, mirroring the JS
//! `VertexArrayCache` + per-instance command layout).
//!
//! DEVIATION (documented per-item):
//! - CesiumJS binds the primitive `modelMatrix` through `czm_model` at draw
//!   time; the wgpu port bakes `primitive.model_matrix ∘ instance.modelMatrix`
//!   into the vertex positions during geometry preparation (the draw-time
//!   model matrix is identity — see `primitive_vs.wgsl`).
//! - the appearance's GLSL sources are replaced by the fixed
//!   `PerInstanceColorAppearance`-trimmed WGSL pair (`PRIMITIVE_VS`/`PRIMITIVE_FS`);
//!   the appearance still drives translucency/render-state decisions;
//! - per-instance `color` attributes are flattened to one `u_color` draw
//!   uniform per instance (JS expands them to per-vertex attributes);
//! - 16-bit index buffers are widened to 32-bit (the wgpu port's index path
//!   is `UnsignedInt`-only, matching the globe geometry path).

use std::sync::Arc;

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::geometry::Geometry;
use cesium_core::geometry_instance::GeometryInstance;
use cesium_core::geometry_pipeline::attribute_locations::create_attribute_locations;
use cesium_core::geometry_pipeline::transform::transform_to_world_coordinates;
use cesium_core::index_datatype::{IndexDatatype, IndexStorage};
use cesium_core::matrix3::Matrix3;
use cesium_core::matrix4::Matrix4;
use cesium_core::primitive_type::PrimitiveType;
use cesium_core::webgl_constants::WebGLConstants;
use cesium_renderer::buffer_usage::BufferUsage;
use cesium_renderer::context::Context;
use cesium_renderer::draw_command::{DrawCommand, UniformValue};
use cesium_renderer::pass::Pass;
use cesium_renderer::render_state::{BlendEquation, BlendingFactor, RenderState};
use cesium_renderer::shader_program::ShaderProgram;
use cesium_renderer::vertex_array::{VertexArray, VertexAttribute};
use cesium_shaders::wgsl;

use crate::appearance::Appearance;
use crate::classification_type::ClassificationType;
use crate::frame_state::FrameState;
use crate::shadow_mode::ShadowMode;

/// GPU resources for one geometry instance draw.
struct InstanceDrawResources {
    /// The instance's vertex array (positions + normals + indices).
    vertex_array: Arc<VertexArray>,
    /// Number of indices (or vertices for non-indexed geometry).
    index_count: u32,
    /// The instance color flattened to the `u_color` draw uniform.
    color: [f32; 4],
    /// The geometry's WebGL primitive type constant.
    primitive_type: u32,
}

/// A renderable primitive composed of geometry instances.
///
/// Primitives are the main way to render geometry in CesiumJS. Each primitive
/// holds one or more GeometryInstances, an Appearance, and manages the GPU
/// pipeline for rendering.
pub struct Primitive {
    /// The geometry instances to render.
    geometry_instances: Vec<GeometryInstance>,
    /// Whether this primitive is shown.
    pub show: bool,
    /// The model matrix applied to all geometry instances.
    ///
    /// DEVIATION: baked into the vertex positions when the primitive becomes
    /// ready (see module docs); changes after `ready` are ignored until the
    /// next `invalidate()` (the JS updates `czm_model` per frame).
    pub model_matrix: Matrix4,
    /// The material applied when the appearance has no material of its own.
    pub depth_fail_material: Option<crate::material::Material>,
    /// Whether to allow picking of individual instances.
    pub allow_picking: bool,
    /// Whether to enable compression of geometry attributes.
    pub compress: bool,
    /// Whether to release the geometry instances once the vertex data is
    /// uploaded (mirrors CesiumJS `releaseGeometryInstances`).
    pub release_geometry_instances: bool,
    /// Whether to cull back faces.
    pub cull: bool,
    /// Whether this primitive is translucent (overridable; otherwise follows
    /// the appearance).
    pub translucent: Option<bool>,
    /// The appearance describing shading + render state.
    appearance: Option<Appearance>,
    /// The shadow mode for this primitive.
    pub shadows: ShadowMode,
    /// The classification type (if this is a classification primitive).
    pub classification_type: Option<ClassificationType>,
    /// Whether this primitive has been destroyed.
    is_destroyed: bool,
    /// Whether this primitive is ready for rendering (vertex data uploaded).
    ready: bool,
    /// Whether the geometry processing runs asynchronously.
    asynchronous: bool,
    /// The trimmed PerInstanceColorAppearance WGSL program (lazy).
    shader_program: Option<Arc<ShaderProgram>>,
    /// Per-instance GPU resources, in geometry-instance order.
    draw_resources: Vec<InstanceDrawResources>,
    /// Union bounding sphere of all instance geometries (world coordinates).
    bounding_sphere: Option<BoundingSphere>,
}

impl Primitive {
    /// Creates a new Primitive.
    pub fn new() -> Self {
        Self::with_options(PrimitiveOptions::default())
    }

    /// Creates a new Primitive from explicit options (mirrors the JS
    /// `options` object of the constructor).
    pub fn with_options(options: PrimitiveOptions) -> Self {
        let geometry_instances = options.geometry_instances.into_iter().collect();
        Self {
            geometry_instances,
            show: options.show.unwrap_or(true),
            model_matrix: options.model_matrix.unwrap_or(Matrix4::IDENTITY.clone()),
            depth_fail_material: None,
            allow_picking: options.allow_picking.unwrap_or(true),
            compress: options.compress.unwrap_or(true),
            release_geometry_instances: options.release_geometry_instances.unwrap_or(true),
            cull: options.cull.unwrap_or(true),
            translucent: options.translucent,
            appearance: options.appearance,
            shadows: options.shadows.unwrap_or(ShadowMode::Disabled),
            classification_type: options.classification_type,
            is_destroyed: false,
            ready: false,
            asynchronous: options.asynchronous.unwrap_or(true),
            shader_program: None,
            draw_resources: Vec::new(),
            bounding_sphere: None,
        }
    }

    /// Adds a geometry instance to this primitive.
    ///
    /// Mirrors CesiumJS `Primitive#addInstance` (invalidating readiness).
    pub fn add_instance(&mut self, instance: GeometryInstance) {
        self.geometry_instances.push(instance);
        self.invalidate();
    }

    /// Returns the number of geometry instances.
    ///
    /// Mirrors CesiumJS `Primitive#getGeometryInstanceCount`.
    pub fn instance_count(&self) -> usize {
        self.geometry_instances.len()
    }

    /// Returns the geometry instances (for spec inspection).
    pub fn geometry_instances(&self) -> &[GeometryInstance] {
        &self.geometry_instances
    }

    /// Returns the appearance, if any.
    pub fn appearance(&self) -> Option<&Appearance> {
        self.appearance.as_ref()
    }

    /// Sets the appearance (invalidates readiness, mirroring the JS setter).
    pub fn set_appearance(&mut self, appearance: Option<Appearance>) {
        self.appearance = appearance;
    }

    /// Returns whether this primitive is translucent, resolving the explicit
    /// override against the appearance (mirrors the JS getter chain).
    pub fn is_translucent(&self) -> bool {
        if let Some(translucent) = self.translucent {
            return translucent;
        }
        self.appearance.as_ref().map(|a| a.translucent).unwrap_or(false)
    }

    /// Returns whether this primitive is ready for rendering.
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Returns the union bounding sphere (available once ready).
    pub fn bounding_sphere(&self) -> Option<&BoundingSphere> {
        self.bounding_sphere.as_ref()
    }

    /// Invalidates the GPU resources so the next `update` rebuilds them
    /// (mirrors CesiumJS's `_createVertexArray` re-entry through the
    /// `ready` flag reset).
    pub fn invalidate(&mut self) {
        self.ready = false;
        self.draw_resources.clear();
    }

    /// Updates this primitive for the current frame.
    ///
    /// Mirrors CesiumJS `Primitive#update`: prepare the vertex data through
    /// the GeometryPipeline on first use, then append the per-instance draw
    /// commands to the frame's command list.
    pub fn update(&mut self, frame_state: &FrameState, context: &mut Context) {
        if !self.show {
            return;
        }
        if self.geometry_instances.is_empty() && !self.ready {
            return;
        }

        if !self.ready {
            self.create_vertex_array(frame_state, context);
        }

        let translucent = self.is_translucent();
        let cull = self.cull;
        let bounding_sphere = self.bounding_sphere.clone();
        let shader_program = match self.shader_program.clone() {
            Some(program) => program,
            None => return,
        };

        for resources in &self.draw_resources {
            let mut render_state = RenderState::default();
            render_state.depth_test.enabled = true;
            if translucent {
                // Standard alpha blending; translucent geometry must not
                // write depth (mirrors the JS appearance render state).
                render_state.depth_mask = false;
                render_state.blending.enabled = true;
                render_state.blending.equation_rgb = BlendEquation::FuncAdd;
                render_state.blending.equation_alpha = BlendEquation::FuncAdd;
                render_state.blending.function_source_rgb = BlendingFactor::SrcAlpha;
                render_state.blending.function_source_alpha = BlendingFactor::One;
                render_state.blending.function_destination_rgb =
                    BlendingFactor::OneMinusSrcAlpha;
                render_state.blending.function_destination_alpha =
                    BlendingFactor::OneMinusSrcAlpha;
            }
            render_state.cull.enabled = cull;

            let mut command = DrawCommand::new();
            command.bounding_volume = bounding_sphere.clone();
            command.primitive_type = resources.primitive_type;
            command.vertex_array = Some(resources.vertex_array.clone());
            command.count = Some(resources.index_count);
            command.offset = 0;
            command.shader_program = Some(shader_program.clone());
            command.uniform_overrides = vec![(
                "u_color".to_string(),
                UniformValue::Vec4(resources.color),
            )];
            command.render_state = render_state;
            command.framebuffer = None;
            command.pass = Some(if translucent {
                Pass::Translucent as u32
            } else {
                Pass::Opaque as u32
            });
            command.owner = Some("Primitive".to_string());
            context.draw(command);
        }
    }

    /// Processes the geometry instances into per-instance vertex arrays.
    ///
    /// Mirrors the CesiumJS `_createVertexArray` chain:
    /// `GeometryPipeline.transformToWorldCoordinates` per instance,
    /// `GeometryPipeline.createAttributeLocations`, then vertex/index buffer
    /// creation.
    fn create_vertex_array(&mut self, _frame_state: &FrameState, context: &mut Context) {
        if self.shader_program.is_none() {
            match ShaderProgram::from_wgsl(
                wgsl::PRIMITIVE_VS,
                wgsl::PRIMITIVE_FS,
                "primitive_per_instance_color".to_string(),
            ) {
                Ok(program) => self.shader_program = Some(Arc::new(program)),
                Err(error) => {
                    log::error!("primitive shader compilation failed: {error}");
                    return;
                }
            }
        }

        // Take the instances out so each can be consumed (the JS releases
        // them after the asynchronous processing completes).
        let mut instances = std::mem::take(&mut self.geometry_instances);
        let mut union_sphere: Option<BoundingSphere> = None;

        for instance in instances.iter_mut() {
            // Bake the instance model matrix into the geometry (JS:
            // GeometryPipeline.transformToWorldCoordinates inside the
            // worker pipeline).
            transform_to_world_coordinates(instance);

            // Collect the geometry sources: the main geometry plus the
            // longitude-split hemispheres when present (each becomes its own
            // draw, mirroring the JS per-instance VAO list).
            let mut sources: Vec<Geometry> = Vec::new();
            if let Some(geometry) = instance.geometry.as_geometry() {
                sources.push(geometry.clone());
            }
            if let Some(geometry) = instance.west_hemisphere_geometry.as_ref() {
                sources.push(geometry.clone());
            }
            if let Some(geometry) = instance.east_hemisphere_geometry.as_ref() {
                sources.push(geometry.clone());
            }

            // Mirrors CesiumJS `GeometryPipeline.createAttributeLocations`
            // (the locations bind the fixed WGSL layout: position = 0,
            // normal = 1 — well-known semantics take the leading slots).
            let color = instance_color(instance);

            for mut geometry in sources {
                let _locations = create_attribute_locations(&geometry);

                // DEVIATION (module docs): bake the primitive model matrix.
                apply_model_matrix(&mut geometry, &self.model_matrix);

                if let Some(sphere) = geometry.bounding_sphere.clone() {
                    union_sphere = Some(match union_sphere {
                        Some(previous) => BoundingSphere::union(&previous, &sphere, None),
                        None => sphere,
                    });
                }

                if let Some(resources) =
                    upload_instance_geometry(&geometry, color, context)
                {
                    self.draw_resources.push(resources);
                }
            }
        }

        self.bounding_sphere = union_sphere;
        if self.release_geometry_instances {
            // JS drops the CPU-side geometry once the VAO exists.
            instances.clear();
        }
        self.geometry_instances = instances;
        self.ready = true;
    }

    /// Returns true if this object was destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys the WebGL resources held by this object.
    pub fn destroy(&mut self) {
        self.geometry_instances.clear();
        self.draw_resources.clear();
        self.is_destroyed = true;
    }
}

impl Default for Primitive {
    fn default() -> Self { Self::new() }
}

/// The constructor options of [`Primitive`], mirroring the JS `options`
/// object (all fields optional except the geometry instances).
#[derive(Default)]
pub struct PrimitiveOptions {
    /// The geometry instances (single instance or list, like the JS).
    pub geometry_instances: Vec<GeometryInstance>,
    pub show: Option<bool>,
    pub model_matrix: Option<Matrix4>,
    pub appearance: Option<Appearance>,
    pub allow_picking: Option<bool>,
    pub compress: Option<bool>,
    pub release_geometry_instances: Option<bool>,
    pub cull: Option<bool>,
    pub asynchronous: Option<bool>,
    pub translucent: Option<bool>,
    pub shadows: Option<ShadowMode>,
    pub classification_type: Option<ClassificationType>,
}

/// Extracts the per-instance color, mirroring the JS
/// `instance.attributes.color` consumed by `PerInstanceColorAppearance`
/// (defaults to white when absent).
fn instance_color(instance: &GeometryInstance) -> [f32; 4] {
    if let Some(attribute) = instance.attributes.get("color") {
        let values = attribute.value();
        if values.len() >= 4 {
            // Color instance attributes carry normalized byte values
            // (0..255) — mirror the JS `Color.unpack` normalization.
            let scale = if attribute.normalize() { 255.0 } else { 1.0 };
            return [
                (values[0] / scale) as f32,
                (values[1] / scale) as f32,
                (values[2] / scale) as f32,
                (values[3] / scale) as f32,
            ];
        }
    }
    [1.0, 1.0, 1.0, 1.0]
}

/// Bakes `model_matrix` into the geometry positions/normals.
///
/// DEVIATION: CesiumJS keeps the primitive `modelMatrix` in `czm_model`;
/// the wgpu port folds it into the vertex data at preparation time.
fn apply_model_matrix(geometry: &mut Geometry, model_matrix: &Matrix4) {
    if Matrix4::equals(model_matrix, &Matrix4::IDENTITY) {
        return;
    }
    if let Some(positions) = geometry.attributes.get_mut("position") {
        let mut point = Cartesian3::ZERO;
        let mut i = 0usize;
        while i + 3 <= positions.values.len() {
            Cartesian3::unpack(&positions.values, Some(i), &mut point);
            point = Matrix4::multiply_by_point_new(model_matrix, &point);
            Cartesian3::pack(&point, &mut positions.values, Some(i));
            i += 3;
        }
    }
    if let Some(normals) = geometry.attributes.get_mut("normal") {
        // Rotation part only (mirrors the JS normal matrix of the model
        // transform for rigid transforms).
        let mut rotation = Matrix3::default();
        Matrix4::get_matrix3(model_matrix, &mut rotation);
        let mut normal = Cartesian3::ZERO;
        let mut i = 0usize;
        while i + 3 <= normals.values.len() {
            Cartesian3::unpack(&normals.values, Some(i), &mut normal);
            normal = Matrix3::multiply_by_vector_new(&rotation, &normal);
            normal = Cartesian3::normalize_new(&normal);
            Cartesian3::pack(&normal, &mut normals.values, Some(i));
            i += 3;
        }
    }
    if let Some(sphere) = geometry.bounding_sphere.clone() {
        geometry.bounding_sphere =
            Some(BoundingSphere::transform(&sphere, model_matrix, None));
    }
}

/// Uploads one instance geometry as position/normal vertex buffers plus the
/// (widened) index buffer, returning its draw resources.
///
/// DEVIATION: interleaving stays split across two vertex buffers (same
/// constraint as the globe geometry path); 16-bit indices widen to 32-bit.
fn upload_instance_geometry(
    geometry: &Geometry,
    color: [f32; 4],
    context: &Context,
) -> Option<InstanceDrawResources> {
    let positions = geometry.attributes.get("position")?;
    let vertex_count = positions.values.len() / positions.components_per_attribute as usize;
    if vertex_count == 0 {
        return None;
    }

    let position_components = positions.components_per_attribute;
    let mut position_f32: Vec<f32> = Vec::with_capacity(vertex_count * 3);
    for vertex in 0..vertex_count {
        let base = vertex * position_components as usize;
        position_f32.push(positions.values[base] as f32);
        position_f32.push(positions.values[base + 1] as f32);
        position_f32.push(
            if position_components >= 3 { positions.values[base + 2] } else { 0.0 } as f32,
        );
    }

    // Normals: use the attribute when present; otherwise derive flat
    // per-vertex normals from the index data (the JS requires a `normal`
    // attribute for lit appearances, but ellipsoid geometry always provides
    // one — this keeps arbitrary test geometries renderable).
    let normal_f32: Vec<f32> = match geometry.attributes.get("normal") {
        Some(normals) => normals
            .values
            .iter()
            .take(vertex_count * 3)
            .map(|value| *value as f32)
            .collect(),
        None => compute_flat_normals(&position_f32, geometry.indices.as_ref()),
    };

    // Indices (widened to u32 for the port's UnsignedInt index path).
    let indices: Vec<u32> = match geometry.indices.as_ref() {
        Some(IndexStorage::U16(values)) => values.iter().map(|index| *index as u32).collect(),
        Some(IndexStorage::U32(values)) => values.clone(),
        None => (0..vertex_count as u32).collect(),
    };
    let index_count = indices.len() as u32;

    let to_bytes = |values: &[f32]| -> Vec<u8> {
        values.iter().flat_map(|value| value.to_le_bytes()).collect()
    };
    let position_buffer = context.create_vertex_buffer(
        Some(&to_bytes(&position_f32)),
        None,
        BufferUsage::StaticDraw,
    );
    let normal_buffer = context.create_vertex_buffer(
        Some(&to_bytes(&normal_f32)),
        None,
        BufferUsage::StaticDraw,
    );
    let index_bytes: Vec<u8> = indices
        .iter()
        .flat_map(|index| index.to_le_bytes())
        .collect();
    let index_buffer = context.create_index_buffer(
        Some(&index_bytes),
        None,
        BufferUsage::StaticDraw,
        IndexDatatype::UnsignedInt,
    );

    let attributes = vec![
        VertexAttribute {
            index: 0,
            buffer: position_buffer,
            components_per_attribute: 3,
            component_datatype: wgpu::VertexFormat::Float32x3,
            normalize: false,
            stride_in_bytes: 12,
            offset_in_bytes: 0,
        },
        VertexAttribute {
            index: 1,
            buffer: normal_buffer,
            components_per_attribute: 3,
            component_datatype: wgpu::VertexFormat::Float32x3,
            normalize: false,
            stride_in_bytes: 12,
            offset_in_bytes: 0,
        },
    ];
    let vertex_array = Arc::new(VertexArray::new(attributes, Some(index_buffer)));

    Some(InstanceDrawResources {
        vertex_array,
        index_count,
        color,
        primitive_type: primitive_type_constant(geometry.primitive_type),
    })
}

/// Maps a cesium-core [`PrimitiveType`] to its WebGL constant (the
/// `DrawCommand.primitive_type` domain used across the port).
fn primitive_type_constant(primitive_type: PrimitiveType) -> u32 {
    match primitive_type {
        PrimitiveType::Points => WebGLConstants::POINTS,
        PrimitiveType::Lines => WebGLConstants::LINES,
        PrimitiveType::LineLoop => WebGLConstants::LINE_LOOP,
        PrimitiveType::LineStrip => WebGLConstants::LINE_STRIP,
        PrimitiveType::Triangles => WebGLConstants::TRIANGLES,
        PrimitiveType::TriangleStrip => WebGLConstants::TRIANGLE_STRIP,
        PrimitiveType::TriangleFan => WebGLConstants::TRIANGLE_FAN,
    }
}

/// Derives flat per-vertex normals by accumulating face normals over the
/// index list (falls back to +Z when no topology is available).
fn compute_flat_normals(positions: &[f32], indices: Option<&IndexStorage>) -> Vec<f32> {
    let vertex_count = positions.len() / 3;
    let mut normals = vec![0.0f32; positions.len()];
    if let Some(indices) = indices {
        let indices: Vec<u32> = match indices {
            IndexStorage::U16(values) => values.iter().map(|index| *index as u32).collect(),
            IndexStorage::U32(values) => values.clone(),
        };
        let mut triangle = 0usize;
        while triangle + 3 <= indices.len() {
            let [a, b, c] = [
                indices[triangle] as usize * 3,
                indices[triangle + 1] as usize * 3,
                indices[triangle + 2] as usize * 3,
            ];
            if a + 2 < positions.len() && b + 2 < positions.len() && c + 2 < positions.len() {
                let u = [
                    positions[b] - positions[a],
                    positions[b + 1] - positions[a + 1],
                    positions[b + 2] - positions[a + 2],
                ];
                let v = [
                    positions[c] - positions[a],
                    positions[c + 1] - positions[a + 1],
                    positions[c + 2] - positions[a + 2],
                ];
                let face = [
                    u[1] * v[2] - u[2] * v[1],
                    u[2] * v[0] - u[0] * v[2],
                    u[0] * v[1] - u[1] * v[0],
                ];
                for base in [a, b, c] {
                    normals[base] += face[0];
                    normals[base + 1] += face[1];
                    normals[base + 2] += face[2];
                }
            }
            triangle += 3;
        }
    }
    for vertex in 0..vertex_count {
        let base = vertex * 3;
        let (x, y, z) = (normals[base], normals[base + 1], normals[base + 2]);
        let magnitude = (x * x + y * y + z * z).sqrt();
        if magnitude > f32::EPSILON {
            normals[base] = x / magnitude;
            normals[base + 1] = y / magnitude;
            normals[base + 2] = z / magnitude;
        } else {
            normals[base + 2] = 1.0;
        }
    }
    normals
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use cesium_core::geometry_attribute::GeometryAttribute;
    use cesium_core::component_datatype::ComponentDatatype;
    use cesium_core::geometry_instance::GeometryInstanceGeometry;
    use cesium_core::geometry_instance_attribute::GeometryInstanceAttribute;

    /// Builds a single-triangle geometry (CCW in the XY plane).
    fn triangle_geometry() -> Geometry {
        let mut attributes = HashMap::new();
        attributes.insert(
            "position".to_string(),
            GeometryAttribute::new(
                ComponentDatatype::Double,
                3,
                false,
                vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            ),
        );
        Geometry::new(attributes, Some(IndexStorage::U16(vec![0, 1, 2])), None, None)
    }

    fn triangle_instance(color: Option<[f64; 4]>) -> GeometryInstance {
        let mut attributes = HashMap::new();
        if let Some(color) = color {
            attributes.insert(
                "color".to_string(),
                GeometryInstanceAttribute::new(
                    ComponentDatatype::UnsignedByte,
                    4,
                    Some(true),
                    color.to_vec(),
                ),
            );
        }
        GeometryInstance::new(
            GeometryInstanceGeometry::Geometry(Box::new(triangle_geometry())),
            None,
            None,
            Some(attributes),
        )
    }

    /// Mirrors `PrimitiveSpec.js` `it("Constructs with options")`.
    #[test]
    fn primitive_constructs_with_options() {
        let primitive = Primitive::with_options(PrimitiveOptions {
            show: Some(false),
            allow_picking: Some(false),
            compress: Some(false),
            release_geometry_instances: Some(false),
            ..Default::default()
        });
        assert!(!primitive.show);
        assert!(!primitive.allow_picking);
        assert!(!primitive.compress);
        assert!(!primitive.release_geometry_instances);
        assert!(primitive.geometry_instances.is_empty());
        assert!(!primitive.is_ready());
        assert!(!primitive.is_destroyed());
    }

    /// Mirrors `PrimitiveSpec.js` `it("addInstance and getGeometryInstanceCount")`.
    #[test]
    fn primitive_add_instance_and_count() {
        let mut primitive = Primitive::new();
        assert_eq!(primitive.instance_count(), 0);
        primitive.add_instance(triangle_instance(None));
        assert_eq!(primitive.instance_count(), 1);
        primitive.add_instance(triangle_instance(Some([255.0, 0.0, 0.0, 255.0])));
        assert_eq!(primitive.instance_count(), 2);
    }

    /// Mirrors the JS normalized-byte color unpacking for instance colors.
    #[test]
    fn primitive_instance_color_unpacks_normalized_bytes() {
        let instance = triangle_instance(Some([255.0, 128.0, 0.0, 204.0]));
        let color = instance_color(&instance);
        assert!((color[0] - 1.0).abs() < f32::EPSILON);
        assert!((color[1] - 128.0 / 255.0).abs() < f32::EPSILON);
        assert!((color[2] - 0.0).abs() < f32::EPSILON);
        assert!((color[3] - 204.0 / 255.0).abs() < f32::EPSILON);
    }

    /// The default instance color is opaque white (JS appearance default).
    #[test]
    fn primitive_instance_color_defaults_to_white() {
        let instance = triangle_instance(None);
        assert_eq!(instance_color(&instance), [1.0, 1.0, 1.0, 1.0]);
    }

    /// Flat normal derivation matches the analytic triangle normal.
    #[test]
    fn primitive_flat_normals_point_up_for_ccw_triangle() {
        let positions: Vec<f32> = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let normals = compute_flat_normals(&positions, Some(&IndexStorage::U16(vec![0, 1, 2])));
        for vertex in 0..3 {
            let base = vertex * 3;
            assert!((normals[base]).abs() < 1.0e-6);
            assert!((normals[base + 1]).abs() < 1.0e-6);
            assert!((normals[base + 2] - 1.0).abs() < 1.0e-6);
        }
    }

    /// Baking the model matrix transforms positions (translation case).
    #[test]
    fn primitive_apply_model_matrix_translates_positions() {
        let mut geometry = triangle_geometry();
        let translation = Matrix4::from_translation_new(&Cartesian3::new(10.0, 20.0, 30.0));
        apply_model_matrix(&mut geometry, &translation);
        let positions = geometry.attributes.get("position").unwrap();
        assert!((positions.values[0] - 10.0).abs() < 1.0e-9);
        assert!((positions.values[1] - 20.0).abs() < 1.0e-9);
        assert!((positions.values[2] - 30.0).abs() < 1.0e-9);
    }

    /// `destroy` mirrors `PrimitiveSpec.js` `it("isDestroyed")`.
    #[test]
    fn primitive_destroy_flags_destroyed() {
        let mut primitive = Primitive::new();
        assert!(!primitive.is_destroyed());
        primitive.destroy();
        assert!(primitive.is_destroyed());
    }
}
