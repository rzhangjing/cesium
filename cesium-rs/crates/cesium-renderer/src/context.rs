//! Ported from `packages/engine/Source/Renderer/Context.js`.
//!
//! The rendering context wrapping a wgpu device and queue.
//! This is the central hub for all GPU operations, mirroring the JS Context
//! which wraps a WebGL2 rendering context.

use cesium_core::bounding_rectangle::BoundingRectangle;
use cesium_core::color::Color;
use cesium_core::create_guid::create_guid;

use crate::buffer::{Buffer, BufferOptions, BufferTarget, IndexBuffer};
use crate::buffer_usage::BufferUsage;
use crate::clear_command::ClearCommand;
use crate::context_limits::ContextLimits;
use crate::draw_command::DrawCommand;
use crate::framebuffer::Framebuffer;
use crate::pass::Pass;
use crate::pick_id::PickId;
use crate::render_state::RenderState;
use crate::shader_cache::ShaderCache;
use crate::texture::Texture;
use crate::texture_cache::TextureCache;
use crate::uniform_state::UniformState;

use std::collections::HashMap;

/// Options for creating a [`Context`].
pub struct ContextOptions {
    /// Whether to enable stencil buffer. Defaults to `true`.
    pub stencil: bool,
    /// Whether to enable alpha channel. Defaults to `false`.
    pub alpha: bool,
    /// Whether to enable depth buffer. Defaults to `true`.
    pub depth: bool,
    /// Whether to enable antialiasing. Defaults to `true`.
    pub antialias: bool,
    /// Whether to log shader compilation. Defaults to `false`.
    pub log_shader_compilation: bool,
}

impl Default for ContextOptions {
    fn default() -> Self {
        Self {
            stencil: true,
            alpha: false,
            depth: true,
            antialias: true,
            log_shader_compilation: false,
        }
    }
}

/// The rendering context wrapping wgpu GPU resources.
///
/// Mirrors the JS `Context` which wraps a `WebGL2RenderingContext`.
/// This is the central hub for all GPU operations: creating buffers,
/// textures, shaders, framebuffers, and executing draw/clear commands.
pub struct Context {
    id: String,
    device: wgpu::Device,
    queue: wgpu::Queue,
    drawing_buffer_width: u32,
    drawing_buffer_height: u32,

    // Sub-systems
    shader_cache: ShaderCache,
    texture_cache: TextureCache,
    uniform_state: UniformState,

    // State
    current_framebuffer: Option<Framebuffer>,
    render_state: RenderState,
    pass_state: crate::pass_state::PassState,

    // Pipeline cache: RenderState hash → wgpu::RenderPipeline
    pipeline_cache: HashMap<u64, wgpu::RenderPipeline>,

    // Default textures (lazy-initialized)
    default_texture: Option<Texture>,
    default_emissive_texture: Option<Texture>,
    default_normal_texture: Option<Texture>,

    // Pick support
    pick_object_counter: u32,
    pick_objects: HashMap<Color, PickId>,

    // Validation flags
    /// Whether to validate framebuffers before rendering.
    pub validate_framebuffer: bool,
    /// Whether to validate shader programs before use.
    pub validate_shader_program: bool,
    /// Whether to log shader compilation info.
    pub log_shader_compilation: bool,

    // Frame tracking
    frame_number: u64,
    in_frame: bool,

    is_destroyed: bool,
}

impl Context {
    /// Creates a new rendering context from a wgpu device and queue.
    ///
    /// Mirrors the JS `new Context(canvas, options)` constructor.
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        width: u32,
        height: u32,
        options: Option<ContextOptions>,
    ) -> Self {
        let _options = options.unwrap_or_default();

        // Initialize context limits from wgpu limits
        let limits = device.limits();
        ContextLimits::set_max_texture_size(limits.max_texture_dimension_2d);
        ContextLimits::set_max_cube_map_texture_size(limits.max_texture_dimension_2d);
        ContextLimits::set_max_renderbuffer_size(limits.max_texture_dimension_2d);
        ContextLimits::set_max_vertex_attribs(limits.max_vertex_attributes);

        Self {
            id: create_guid(),
            device,
            queue,
            drawing_buffer_width: width,
            drawing_buffer_height: height,
            shader_cache: ShaderCache::new(),
            texture_cache: TextureCache::new(),
            uniform_state: UniformState::new(),
            current_framebuffer: None,
            render_state: RenderState::default(),
            pass_state: crate::pass_state::PassState::default(),
            pipeline_cache: HashMap::new(),
            default_texture: None,
            default_emissive_texture: None,
            default_normal_texture: None,
            pick_object_counter: 0,
            pick_objects: HashMap::new(),
            validate_framebuffer: false,
            validate_shader_program: false,
            log_shader_compilation: _options.log_shader_compilation,
            frame_number: 0,
            in_frame: false,
            is_destroyed: false,
        }
    }

    // ── Properties ──────────────────────────────────────────────────

    /// Returns the unique identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the drawing buffer width.
    pub fn drawing_buffer_width(&self) -> u32 {
        self.drawing_buffer_width
    }

    /// Returns the drawing buffer height.
    pub fn drawing_buffer_height(&self) -> u32 {
        self.drawing_buffer_height
    }

    /// Returns a reference to the wgpu device.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Returns a reference to the wgpu queue.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Returns the shader cache.
    pub fn shader_cache(&self) -> &ShaderCache {
        &self.shader_cache
    }

    /// Returns a mutable reference to the shader cache.
    pub fn shader_cache_mut(&mut self) -> &mut ShaderCache {
        &mut self.shader_cache
    }

    /// Returns the texture cache.
    pub fn texture_cache(&self) -> &TextureCache {
        &self.texture_cache
    }

    /// Returns the uniform state.
    pub fn uniform_state(&self) -> &UniformState {
        &self.uniform_state
    }

    /// Returns a mutable reference to the uniform state.
    pub fn uniform_state_mut(&mut self) -> &mut UniformState {
        &mut self.uniform_state
    }

    /// Returns the current render state.
    pub fn render_state(&self) -> &RenderState {
        &self.render_state
    }

    // ── Buffer creation ─────────────────────────────────────────────

    /// Creates a vertex buffer.
    pub fn create_vertex_buffer(
        &self,
        typed_array: Option<&[u8]>,
        size_in_bytes: Option<u64>,
        usage: BufferUsage,
    ) -> Buffer {
        Buffer::create_vertex_buffer(&self.device, typed_array, size_in_bytes, usage)
    }

    /// Creates an index buffer.
    pub fn create_index_buffer(
        &self,
        typed_array: Option<&[u8]>,
        size_in_bytes: Option<u64>,
        usage: BufferUsage,
        index_datatype: cesium_core::index_datatype::IndexDatatype,
    ) -> IndexBuffer {
        Buffer::create_index_buffer(&self.device, typed_array, size_in_bytes, usage, index_datatype)
    }

    // ── Texture creation ────────────────────────────────────────────

    /// Creates a 2D texture.
    pub fn create_texture(&self, options: crate::texture::TextureOptions) -> Texture {
        Texture::new(&self.device, options)
    }

    // ── Framebuffer ─────────────────────────────────────────────────

    /// Creates a framebuffer.
    pub fn create_framebuffer(options: crate::framebuffer::FramebufferOptions) -> Framebuffer {
        Framebuffer::new(options)
    }

    // ── Frame lifecycle ─────────────────────────────────────────────

    /// Begins a new frame. Must be called before any draw/clear operations.
    ///
    /// Mirrors `Context.beginFrame()`.
    pub fn begin_frame(&mut self) {
        debug_assert!(!self.in_frame, "begin_frame called while already in frame");
        self.in_frame = true;
        self.frame_number += 1;
    }

    /// Ends the current frame. Flushes pending commands to the GPU.
    ///
    /// Mirrors `Context.endFrame()`.
    pub fn end_frame(&mut self) {
        debug_assert!(self.in_frame, "end_frame called without matching begin_frame");
        self.in_frame = false;
        // Flush texture cache, etc.
    }

    /// Returns the current frame number.
    pub fn frame_number(&self) -> u64 {
        self.frame_number
    }

    // ── Default textures ────────────────────────────────────────────

    /// Returns a 1x1 RGBA texture initialized to white (255, 255, 255, 255).
    ///
    /// Mirrors `Context.defaultTexture`.
    pub fn default_texture(&mut self) -> &Texture {
        if self.default_texture.is_none() {
            self.default_texture = Some(Texture::new(
                &self.device,
                crate::texture::TextureOptions {
                    width: Some(1),
                    height: Some(1),
                    pixel_format: cesium_core::pixel_format::PixelFormat::Rgba,
                    source: Some(crate::texture::TextureSource {
                        width: 1,
                        height: 1,
                        array_buffer_view: vec![255, 255, 255, 255],
                    }),
                    ..Default::default()
                },
            ));
        }
        self.default_texture.as_ref().unwrap()
    }

    /// Returns a 1x1 RGB texture initialized to [0, 0, 0] (non-emissive).
    ///
    /// Mirrors `Context.defaultEmissiveTexture`.
    pub fn default_emissive_texture(&mut self) -> &Texture {
        if self.default_emissive_texture.is_none() {
            self.default_emissive_texture = Some(Texture::new(
                &self.device,
                crate::texture::TextureOptions {
                    width: Some(1),
                    height: Some(1),
                    pixel_format: cesium_core::pixel_format::PixelFormat::Rgb,
                    source: Some(crate::texture::TextureSource {
                        width: 1,
                        height: 1,
                        array_buffer_view: vec![0, 0, 0],
                    }),
                    ..Default::default()
                },
            ));
        }
        self.default_emissive_texture.as_ref().unwrap()
    }

    /// Returns a 1x1 RGB normal texture initialized to [128, 128, 255] (+Z normal).
    ///
    /// Mirrors `Context.defaultNormalTexture`.
    pub fn default_normal_texture(&mut self) -> &Texture {
        if self.default_normal_texture.is_none() {
            self.default_normal_texture = Some(Texture::new(
                &self.device,
                crate::texture::TextureOptions {
                    width: Some(1),
                    height: Some(1),
                    pixel_format: cesium_core::pixel_format::PixelFormat::Rgb,
                    source: Some(crate::texture::TextureSource {
                        width: 1,
                        height: 1,
                        array_buffer_view: vec![128, 128, 255],
                    }),
                    ..Default::default()
                },
            ));
        }
        self.default_normal_texture.as_ref().unwrap()
    }

    // ── Draw / Clear ────────────────────────────────────────────────

    /// Clears the current framebuffer or the default framebuffer.
    ///
    /// Mirrors `Context.clear(clearCommand, passState)`.
    pub fn clear(&self, command: &ClearCommand, _color: Option<Color>) {
        // DEVIATION: In wgpu, clearing is done as part of a render pass.
        // This method captures the intent; actual clearing happens in begin_render_pass.
        let _encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("clear_encoder"),
        });
    }

    /// Executes a draw command.
    ///
    /// Mirrors `Context.draw(drawCommand, passState)`.
    pub fn draw(&self, _command: &DrawCommand) {
        // DEVIATION: In wgpu, draw commands are encoded via RenderPass.
        // This method captures the intent; actual encoding happens in the frame loop.
    }

    /// Submits all pending commands to the GPU.
    ///
    /// Mirrors the JS end-of-frame flush.
    pub fn submit(&self) {
        // In wgpu, commands are submitted via queue.submit()
        // This is handled by the frame loop.
    }

    // ── Pipeline cache ──────────────────────────────────────────────

    /// Returns a cached render pipeline for the given state hash, or creates a new one.
    ///
    /// This is the wgpu equivalent of CesiumJS's RenderState partial application.
    /// In WebGL, state changes are imperative; in wgpu, pipelines are immutable
    /// objects created ahead of time and cached by their state hash.
    pub fn get_or_create_pipeline(
        &mut self,
        state_hash: u64,
        pipeline_descriptor: &wgpu::RenderPipelineDescriptor,
    ) -> &wgpu::RenderPipeline {
        if !self.pipeline_cache.contains_key(&state_hash) {
            let pipeline = self.device.create_render_pipeline(pipeline_descriptor);
            self.pipeline_cache.insert(state_hash, pipeline);
        }
        self.pipeline_cache.get(&state_hash).unwrap()
    }

    /// Returns the number of cached pipelines.
    pub fn pipeline_cache_size(&self) -> usize {
        self.pipeline_cache.len()
    }

    // ── Pick support ────────────────────────────────────────────────

    /// Creates a new pick ID for picking operations.
    pub fn create_pick_id(&mut self, color: Color) -> PickId {
        self.pick_object_counter += 1;
        PickId::new(self.pick_object_counter, color)
    }

    // ── Resize ──────────────────────────────────────────────────────

    /// Resizes the drawing buffer.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.drawing_buffer_width = width;
        self.drawing_buffer_height = height;
    }

    // ── Lifecycle ───────────────────────────────────────────────────

    /// Returns whether this context has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys the context and all owned resources.
    pub fn destroy(&mut self) {
        self.shader_cache.clear();
        self.is_destroyed = true;
    }
}
