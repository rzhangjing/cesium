//! Ported from `packages/engine/Source/Scene/ViewportQuad.js`.
//!
//! A viewport quad is a full-screen rectangle used for post-processing effects.

use std::sync::Arc;

use cesium_core::webgl_constants::WebGLConstants;
use cesium_renderer::context::Context;
use cesium_renderer::draw_command::{DrawCommand, UniformValue};
use cesium_renderer::framebuffer::Framebuffer;
use cesium_renderer::render_state::RenderState;
use cesium_renderer::shader_program::ShaderProgram;
use cesium_renderer::vertex_array::{VertexArray, VertexAttribute};
use cesium_renderer::buffer_usage::BufferUsage;
use cesium_shaders::wgsl;

use crate::frame_state::FrameState;
use crate::material::Material;

/// Full-viewport clip-space rectangle: two triangles covering [-1, 1]².
///
/// Returns `(positions, texture_coordinates)`, each 6 × vec4, matching the
/// `viewport_quad_vs.wgsl` inputs (`position3DAndHeight` at location 0,
/// `textureCoordAndEncodedAttributes` at location 1).
fn fullscreen_vertex_data() -> (Vec<f32>, Vec<f32>) {
    #[rustfmt::skip]
    let positions: [f32; 6 * 4] = [
        -1.0, -1.0, 0.0, 1.0,
         1.0, -1.0, 0.0, 1.0,
        -1.0,  1.0, 0.0, 1.0,
        -1.0,  1.0, 0.0, 1.0,
         1.0, -1.0, 0.0, 1.0,
         1.0,  1.0, 0.0, 1.0,
    ];
    #[rustfmt::skip]
    let texture_coordinates: [f32; 6 * 4] = [
        0.0, 0.0, 0.0, 0.0,
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        1.0, 0.0, 0.0, 0.0,
        1.0, 1.0, 0.0, 0.0,
    ];
    (positions.to_vec(), texture_coordinates.to_vec())
}

/// A viewport quad is a full-screen rectangle used for post-processing effects.
///
/// Renders a material (shader) over the entire viewport.
///
/// DEVIATION (B3.1): CesiumJS builds the quad from `RectangleGeometry` and a
/// Fabric material compiled to GLSL at first update. The wgpu port uses a
/// fixed two-triangle vertex array plus the hand-written WGSL color material
/// (`viewport_quad_vs.wgsl` + `viewport_quad_color_fs.wgsl`); `color` feeds
/// the group(1) `material` uniform.
pub struct ViewportQuad {
    /// Whether this quad is shown.
    pub show: bool,
    /// The material (shader) applied to the quad.
    pub material: Option<Material>,
    /// The solid color used by the smoke-path color material (RGBA).
    pub color: [f32; 4],
    /// The framebuffer to render into (`None` = default framebuffer).
    ///
    /// Mirrors the `DrawCommand.framebuffer` CesiumJS post-process passes
    /// set when rendering the quad into an intermediate target.
    pub framebuffer: Option<Arc<Framebuffer>>,
    /// Full-screen vertex array (created lazily on first update).
    vertex_array: Option<Arc<VertexArray>>,
    /// The viewport quad WGSL shader program (created lazily).
    shader_program: Option<Arc<ShaderProgram>>,
    /// Whether this quad has been destroyed.
    is_destroyed: bool,
}

impl ViewportQuad {
    /// Creates a new ViewportQuad.
    pub fn new() -> Self {
        Self {
            show: true,
            material: None,
            color: [1.0, 1.0, 1.0, 1.0],
            framebuffer: None,
            vertex_array: None,
            shader_program: None,
            is_destroyed: false,
        }
    }

    /// Creates a new ViewportQuad rendering the given solid color.
    pub fn with_color(color: [f32; 4]) -> Self {
        Self { color, ..Self::new() }
    }

    /// Updates the quad for the current frame: creates GPU resources on
    /// first use and submits the draw command to the context.
    ///
    /// Mirrors CesiumJS `ViewportQuad.update(frameState)` which pushes its
    /// `DrawCommand` onto `frameState.commandList`; the wgpu port hands the
    /// command directly to the collecting [`Context`].
    pub fn update(&mut self, _frame_state: &FrameState, context: &mut Context) {
        if !self.show {
            return;
        }

        if self.vertex_array.is_none() {
            let (positions, texture_coordinates) = fullscreen_vertex_data();
            let to_bytes = |values: &Vec<f32>| -> Vec<u8> {
                values.iter().flat_map(|value| value.to_le_bytes()).collect()
            };
            // DEVIATION: CesiumJS interleaves position + texture coordinates
            // in one buffer; `Buffer` is move-only in this port, so the two
            // attributes use two vertex buffers instead (same layout hash
            // semantics, two `VertexBufferLayout` slots).
            let position_buffer = context.create_vertex_buffer(
                Some(&to_bytes(&positions)),
                None,
                BufferUsage::StaticDraw,
            );
            let texture_coordinate_buffer = context.create_vertex_buffer(
                Some(&to_bytes(&texture_coordinates)),
                None,
                BufferUsage::StaticDraw,
            );
            let attributes = vec![
                VertexAttribute {
                    index: 0,
                    buffer: position_buffer,
                    components_per_attribute: 4,
                    component_datatype: wgpu::VertexFormat::Float32x4,
                    normalize: false,
                    stride_in_bytes: 16,
                    offset_in_bytes: 0,
                },
                VertexAttribute {
                    index: 1,
                    buffer: texture_coordinate_buffer,
                    components_per_attribute: 4,
                    component_datatype: wgpu::VertexFormat::Float32x4,
                    normalize: false,
                    stride_in_bytes: 16,
                    offset_in_bytes: 0,
                },
            ];
            self.vertex_array = Some(Arc::new(VertexArray::new(attributes, None)));
        }

        if self.shader_program.is_none() {
            match ShaderProgram::from_wgsl(
                wgsl::VIEWPORT_QUAD_VS,
                wgsl::VIEWPORT_QUAD_COLOR_FS,
                "viewport_quad_color".to_string(),
            ) {
                Ok(program) => self.shader_program = Some(Arc::new(program)),
                Err(error) => {
                    log::error!("viewport quad shader compilation failed: {error}");
                    return;
                }
            }
        }

        // wgpu pipelines are immutable; no depth attachment on the smoke
        // target, so the depth test must be disabled for this draw.
        let mut render_state = RenderState::default();
        render_state.depth_test.enabled = false;

        let mut command = DrawCommand::new();
        command.primitive_type = WebGLConstants::TRIANGLES;
        command.vertex_array = self.vertex_array.clone();
        command.count = Some(6);
        command.offset = 0;
        command.shader_program = self.shader_program.clone();
        command.uniform_overrides = vec![(
            "material".to_string(),
            UniformValue::Vec4(self.color),
        )];
        command.render_state = render_state;
        command.framebuffer = self.framebuffer.clone();
        command.pass = Some(cesium_renderer::pass::Pass::Translucent as u32);
        command.owner = Some("ViewportQuad".to_string());

        context.draw(command);
    }

    /// Returns true if this object was destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys the WebGL resources held by this object.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for ViewportQuad {
    fn default() -> Self { Self::new() }
}
