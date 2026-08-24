//! Ported from `packages/engine/Source/Renderer/Context.js`.
//!
//! The rendering context wrapping a wgpu device and queue.
//! This is the central hub for all GPU operations, mirroring the JS Context
//! which wraps a WebGL2 rendering context.
//!
//! DEVIATION (B2.4 — immediate mode vs. frame orchestration):
//! CesiumJS is immediate-mode over WebGL: `Context.draw()` applies render
//! state with `gl.*` calls and issues `gl.drawElements` right away, and
//! `Context.clear()` calls `gl.clear` synchronously. wgpu is a command-buffer
//! API with immutable pipeline objects, so this port defers execution:
//!
//! 1. `begin_frame()` creates the per-frame `wgpu::CommandEncoder`.
//! 2. `draw()` / `clear()` collect typed commands into a per-frame queue.
//! 3. `execute()` aggregates commands into runs keyed by
//!    (Pass, Framebuffer), opens one render pass per run (the run's leading
//!    `ClearCommand` decides the color/depth `LoadOp`), then for each draw:
//!    `set_pipeline` (through `pipeline_cache`, keyed by RenderState hash ⊕
//!    shader key ⊕ vertex layout hash ⊕ surface format ⊕ topology ⊕ depth
//!    format), `set_bind_group` (group(0) automatic uniforms with dynamic
//!    offset, group(1) per-draw material resources), `set_vertex_buffer`,
//!    `set_index_buffer`, `draw_indexed`/`draw`.
//! 4. `end_frame()` submits the encoder via `queue.submit`. Surface
//!    presentation stays with the application (see `examples/viewer-demo`),
//!    which supplies the default render target to `execute`.

use cesium_core::color::Color;
use cesium_core::create_guid::create_guid;

use crate::automatic_uniforms::{AutomaticUniformRing, AutomaticUniforms};
use crate::buffer::{Buffer, IndexBuffer};
use crate::buffer_usage::BufferUsage;
use crate::clear_command::ClearCommand;
use crate::context_limits::ContextLimits;
use crate::draw_command::{DrawCommand, UniformValue};
use crate::framebuffer::Framebuffer;
use crate::pick_id::PickId;
use crate::render_state::RenderState;
use crate::shader_cache::ShaderCache;
use crate::shader_program::{BindingKind, ShaderProgram};
use crate::texture::Texture;
use crate::texture_cache::TextureCache;
use crate::uniform_state::UniformState;
use crate::vertex_array::VertexArray;

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

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

/// The default (swapchain) render target supplied by the application each
/// frame. CesiumJS renders to the canvas implicitly; wgpu requires the
/// surface texture view explicitly.
pub struct DefaultRenderTarget<'a> {
    /// The current surface texture view.
    pub view: &'a wgpu::TextureView,
    /// The surface format.
    pub format: wgpu::TextureFormat,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

/// A collected per-frame command.
enum FrameCommand {
    Clear(ClearCommand),
    Draw(DrawCommand),
}

/// Target of a command run, resolved during `execute()`.
enum RunTarget {
    /// The default (surface) render target.
    Default,
    /// An off-screen framebuffer.
    Offscreen(Arc<Framebuffer>),
}

/// One draw, fully resolved against GPU resources (execute phase 1 output).
struct ResolvedDraw {
    pipeline_key: u64,
    vertex_array: Arc<VertexArray>,
    /// group(1) material bind group, when the shader declares group(1).
    material_bind_group: Option<wgpu::BindGroup>,
    /// Dynamic offsets for group(1), in binding order.
    material_dynamic_offsets: Vec<u32>,
    /// Dynamic offset for the group(0) automatic-uniforms bind group.
    automatic_offset: Option<u32>,
    count: u32,
    offset: u32,
    instance_count: u32,
}

/// One render pass worth of commands (execute phase 1 output).
struct ResolvedRun {
    target: RunTarget,
    color_load: wgpu::LoadOp<wgpu::Color>,
    depth_clear: Option<f32>,
    draws: Vec<ResolvedDraw>,
}

const MATERIAL_SCRATCH_SIZE: u64 = 1 << 20; // 1 MiB per frame
const MATERIAL_ALIGNMENT: u64 = 256;

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

    // Pipeline cache: pipeline key → wgpu::RenderPipeline
    pipeline_cache: HashMap<u64, wgpu::RenderPipeline>,
    // Bind group layouts for group(1), keyed by shader program hash.
    material_layouts: HashMap<u64, wgpu::BindGroupLayout>,
    // Pipeline layouts keyed by shader program hash.
    pipeline_layouts: HashMap<u64, wgpu::PipelineLayout>,

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

    // ── Frame orchestration (B2.4) ────────────────────────────────
    /// The per-frame command encoder (created in `begin_frame`).
    frame_encoder: Option<wgpu::CommandEncoder>,
    /// Collected clear/draw commands for the current frame.
    frame_commands: Vec<FrameCommand>,
    /// Per-frame automatic-uniform ring buffer (group(0)).
    automatic_ring: AutomaticUniformRing,
    /// Per-frame scratch buffer for group(1) uniform data.
    material_scratch: wgpu::Buffer,
    material_scratch_next: u64,
    /// Shared linear sampler for material texture bindings.
    default_sampler: wgpu::Sampler,
    /// Color format of the default (surface) target, refreshed by
    /// `execute()` from [`DefaultRenderTarget`]. Pipelines for draws
    /// targeting the default framebuffer are keyed with this format.
    default_color_format: wgpu::TextureFormat,
    /// Depth format of the default target (None = no depth attachment).
    default_depth_format: Option<wgpu::TextureFormat>,

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

        let automatic_ring = AutomaticUniformRing::new(&device, 64);
        let material_scratch = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("material scratch"),
            size: MATERIAL_SCRATCH_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let default_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("material default sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

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
            material_layouts: HashMap::new(),
            pipeline_layouts: HashMap::new(),
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
            frame_encoder: None,
            frame_commands: Vec::new(),
            automatic_ring,
            material_scratch,
            material_scratch_next: 0,
            default_sampler,
            default_color_format: wgpu::TextureFormat::Rgba8Unorm,
            default_depth_format: None,
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

    /// Creates a vertex buffer (uploading any initial data immediately).
    pub fn create_vertex_buffer(
        &self,
        typed_array: Option<&[u8]>,
        size_in_bytes: Option<u64>,
        usage: BufferUsage,
    ) -> Buffer {
        let mut buffer =
            Buffer::create_vertex_buffer(&self.device, typed_array, size_in_bytes, usage);
        buffer.upload_pending_data(&self.queue);
        buffer
    }

    /// Creates an index buffer (uploading any initial data immediately).
    pub fn create_index_buffer(
        &self,
        typed_array: Option<&[u8]>,
        size_in_bytes: Option<u64>,
        usage: BufferUsage,
        index_datatype: cesium_core::index_datatype::IndexDatatype,
    ) -> IndexBuffer {
        let mut buffer = Buffer::create_index_buffer(
            &self.device,
            typed_array,
            size_in_bytes,
            usage,
            index_datatype,
        );
        buffer.buffer_mut().upload_pending_data(&self.queue);
        buffer
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
        self.uniform_state.next_frame();
        self.frame_commands.clear();
        self.material_scratch_next = 0;
        self.automatic_ring.begin_frame();
        self.frame_encoder = Some(self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("cesium frame encoder"),
            },
        ));
    }

    /// Ends the current frame. Submits the pending encoder to the queue.
    ///
    /// Mirrors `Context.endFrame()`.
    /// DEVIATION: surface presentation is the application's responsibility
    /// (the Context does not own the surface).
    pub fn end_frame(&mut self) {
        debug_assert!(self.in_frame, "end_frame called without matching begin_frame");
        self.in_frame = false;
        if let Some(encoder) = self.frame_encoder.take() {
            self.queue.submit(Some(encoder.finish()));
        }
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
            if let Some(texture) = &self.default_texture {
                texture.upload_source(
                    &self.queue,
                    &crate::texture::TextureSource {
                        width: 1,
                        height: 1,
                        array_buffer_view: vec![255, 255, 255, 255],
                    },
                );
            }
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
    /// DEVIATION: CesiumJS clears immediately via `gl.clear`; the wgpu port
    /// collects the command and applies it as the `LoadOp` of the run's
    /// render pass during `execute()`.
    pub fn clear(&mut self, command: ClearCommand) {
        self.frame_commands.push(FrameCommand::Clear(command));
    }

    /// Collects a draw command for execution this frame.
    ///
    /// Mirrors `Context.draw(drawCommand, passState)`.
    /// DEVIATION: CesiumJS executes immediately; the wgpu port defers to
    /// `execute()` (see the module-level note on immediate mode).
    pub fn draw(&mut self, command: DrawCommand) {
        self.frame_commands.push(FrameCommand::Draw(command));
    }

    /// Executes all collected commands against the GPU.
    ///
    /// `default_target` is the surface render target for commands that draw
    /// to the default framebuffer. Commands are aggregated into runs by
    /// target framebuffer; each run becomes one render pass.
    pub fn execute(&mut self, default_target: Option<DefaultRenderTarget<'_>>) {
        // Refresh the default-target formats before resolving so pipelines
        // for draws on the default framebuffer are keyed (and created) with
        // the real surface format rather than a placeholder.
        if let Some(target) = default_target.as_ref() {
            self.default_color_format = target.format;
        }
        let mut encoder = match self.frame_encoder.take() {
            Some(encoder) => encoder,
            None => self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("cesium frame encoder"),
            }),
        };

        let runs = self.resolve_runs();

        for run in runs {
            self.record_run(&mut encoder, &run, default_target.as_ref());
        }

        self.frame_encoder = Some(encoder);
    }

    /// Submits all pending commands to the GPU.
    ///
    /// Mirrors the JS end-of-frame flush.
    pub fn submit(&mut self) {
        if let Some(encoder) = self.frame_encoder.take() {
            self.queue.submit(Some(encoder.finish()));
        }
        if self.in_frame {
            self.frame_encoder = Some(self.device.create_command_encoder(
                &wgpu::CommandEncoderDescriptor {
                    label: Some("cesium frame encoder (post-submit)"),
                },
            ));
        }
    }

    // ── Frame orchestration internals ───────────────────────────────

    /// Phase 1: resolve the command queue into render-pass runs, creating
    /// pipelines, allocating uniform slots and building bind groups.
    fn resolve_runs(&mut self) -> Vec<ResolvedRun> {
        let commands = std::mem::take(&mut self.frame_commands);
        let mut runs: Vec<ResolvedRun> = Vec::new();

        for command in commands {
            match command {
                FrameCommand::Clear(clear) => {
                    let key_matches = runs.last().is_some_and(|run| match (&run.target, &clear.framebuffer) {
                        (RunTarget::Default, None) => true,
                        (RunTarget::Offscreen(existing), Some(target)) => Arc::ptr_eq(existing, target),
                        _ => false,
                    });
                    if !key_matches {
                        runs.push(ResolvedRun {
                            target: match &clear.framebuffer {
                                Some(framebuffer) => RunTarget::Offscreen(framebuffer.clone()),
                                None => RunTarget::Default,
                            },
                            color_load: wgpu::LoadOp::Load,
                            depth_clear: None,
                            draws: Vec::new(),
                        });
                    }
                    let run = runs.last_mut().unwrap();
                    if let Some(color) = clear.color {
                        run.color_load = wgpu::LoadOp::Clear(wgpu::Color {
                            r: color[0] as f64,
                            g: color[1] as f64,
                            b: color[2] as f64,
                            a: color[3] as f64,
                        });
                    }
                    if let Some(depth) = clear.depth {
                        run.depth_clear = Some(depth as f32);
                    }
                }
                FrameCommand::Draw(draw) => {
                    let key_matches = runs.last().is_some_and(|run| match (&run.target, &draw.framebuffer) {
                        (RunTarget::Default, None) => true,
                        (RunTarget::Offscreen(existing), Some(target)) => Arc::ptr_eq(existing, target),
                        _ => false,
                    });
                    if !key_matches {
                        runs.push(ResolvedRun {
                            target: match &draw.framebuffer {
                                Some(framebuffer) => RunTarget::Offscreen(framebuffer.clone()),
                                None => RunTarget::Default,
                            },
                            color_load: wgpu::LoadOp::Load,
                            depth_clear: None,
                            draws: Vec::new(),
                        });
                    }
                    if let Some(resolved) = self.resolve_draw(&draw) {
                        runs.last_mut().unwrap().draws.push(resolved);
                    }
                }
            }
        }
        runs
    }

    /// Resolves one draw command into GPU resources (pipeline, bind groups,
    /// uniform slots). Returns `None` (with a log) when incomplete.
    fn resolve_draw(&mut self, command: &DrawCommand) -> Option<ResolvedDraw> {
        let program = match &command.shader_program {
            Some(program) => program.clone(),
            None => {
                log::warn!("draw command skipped: no shader program");
                return None;
            }
        };
        let vertex_array = match &command.vertex_array {
            Some(vertex_array) => vertex_array.clone(),
            None => {
                log::warn!("draw command skipped: no vertex array");
                return None;
            }
        };
        let topology = match RenderState::primitive_type_to_wgpu_topology(command.primitive_type) {
            Some(topology) => topology,
            None => {
                log::warn!(
                    "draw command skipped: unsupported primitive type {}",
                    command.primitive_type
                );
                return None;
            }
        };

        // Per-draw model matrix feeds the automatic uniforms, as in CesiumJS
        // (`uniformState.model = drawCommand.modelMatrix`).
        if let Some(model) = &command.model_matrix {
            self.uniform_state.update_model(model.clone());
        }

        // Target formats participate in the pipeline key.
        let (color_format, depth_format) = match &command.framebuffer {
            Some(framebuffer) => (
                framebuffer.color_attachment_format().unwrap_or(wgpu::TextureFormat::Rgba8Unorm),
                framebuffer.depth_stencil_format(),
            ),
            None => {
                // Default target: use the surface format recorded by the
                // most recent `execute()` call (falls back to the initial
                // placeholder before the application supplies a target).
                (self.default_color_format, self.default_depth_format)
            }
        };

        let pipeline_key = Self::pipeline_key(
            &command.render_state,
            program.pipeline_hash(),
            vertex_array.layout_hash(),
            color_format,
            depth_format,
            topology,
        );

        if !self.pipeline_cache.contains_key(&pipeline_key) {
            self.create_pipeline(
                pipeline_key,
                &program,
                &vertex_array,
                &command.render_state,
                color_format,
                depth_format,
                topology,
            );
        }

        // Automatic uniforms (group 0).
        let uses_automatic = program.bindings().iter().any(|binding| binding.group == 0);
        let automatic_offset = if uses_automatic {
            let uniforms = AutomaticUniforms::from_uniform_state(&mut self.uniform_state);
            match self.automatic_ring.allocate(&self.queue, &uniforms) {
                Some(offset) => Some(offset),
                None => {
                    log::warn!("automatic uniform ring exhausted; draw skipped");
                    return None;
                }
            }
        } else {
            None
        };

        // Material resources (group 1).
        let (material_bind_group, material_dynamic_offsets) =
            self.create_material_bind_group(&program, &command.uniform_overrides)?;

        // Element count: explicit count, or the whole index/vertex buffer.
        let count = command.count.unwrap_or_else(|| {
            vertex_array
                .index_buffer()
                .map(|ib| ib.number_of_indices() as u32)
                .unwrap_or(0)
        });

        Some(ResolvedDraw {
            pipeline_key,
            vertex_array,
            material_bind_group,
            material_dynamic_offsets,
            automatic_offset,
            count,
            offset: command.offset,
            instance_count: command.instance_count.max(1),
        })
    }

    /// Computes the pipeline cache key:
    /// RenderState hash ⊕ shader key ⊕ vertex layout hash ⊕ color format ⊕
    /// depth format ⊕ topology.
    fn pipeline_key(
        render_state: &RenderState,
        shader_hash: u64,
        layout_hash: u64,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
        topology: wgpu::PrimitiveTopology,
    ) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        render_state.hash(&mut hasher);
        shader_hash.hash(&mut hasher);
        layout_hash.hash(&mut hasher);
        color_format.hash(&mut hasher);
        depth_format.hash(&mut hasher);
        topology.hash(&mut hasher);
        hasher.finish()
    }

    /// Creates and caches a render pipeline for the resolved state.
    #[allow(clippy::too_many_arguments)]
    fn create_pipeline(
        &mut self,
        key: u64,
        program: &Arc<ShaderProgram>,
        vertex_array: &VertexArray,
        render_state: &RenderState,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
        topology: wgpu::PrimitiveTopology,
    ) {
        let vertex_module = match program.create_vertex_shader_module(&self.device) {
            Ok(module) => module,
            Err(error) => {
                log::error!("vertex shader module creation failed: {error}");
                return;
            }
        };
        let fragment_module = match program.create_fragment_shader_module(&self.device) {
            Ok(module) => module,
            Err(error) => {
                log::error!("fragment shader module creation failed: {error}");
                return;
            }
        };

        let pipeline_layout = self.get_or_create_pipeline_layout(program);

        let owned_layouts = vertex_array.buffer_layouts();
        let buffers: Vec<Option<wgpu::VertexBufferLayout<'_>>> = owned_layouts
            .iter()
            .map(|layout| Some(layout.as_wgpu()))
            .collect();

        let vertex_entry = program.vertex_entry().to_string();
        let fragment_entry = program.fragment_entry().to_string();

        let descriptor = wgpu::RenderPipelineDescriptor {
            label: Some("cesium pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: vertex_module,
                entry_point: Some(&vertex_entry),
                compilation_options: Default::default(),
                buffers: &buffers,
            },
            primitive: render_state.to_wgpu_primitive_state(topology),
            depth_stencil: depth_format
                .map(|format| render_state.to_wgpu_depth_stencil_state(format)),
            multisample: render_state.to_wgpu_multisample_state(),
            fragment: Some(wgpu::FragmentState {
                module: fragment_module,
                entry_point: Some(&fragment_entry),
                compilation_options: Default::default(),
                targets: &[Some(render_state.to_wgpu_color_target_state(color_format))],
            }),
            multiview_mask: None,
            cache: None,
        };

        let pipeline = self.device.create_render_pipeline(&descriptor);
        self.pipeline_cache.insert(key, pipeline);
    }

    /// Returns (creating if needed) the pipeline layout for a program:
    /// group(0) = automatic uniforms ring layout, group(1) = reflected
    /// material layout.
    fn get_or_create_pipeline_layout(&mut self, program: &ShaderProgram) -> wgpu::PipelineLayout {
        let shader_hash = program.pipeline_hash();
        if let Some(layout) = self.pipeline_layouts.get(&shader_hash) {
            return layout.clone();
        }
        let group1_layout = self.get_or_create_material_layout(program);
        let uses_automatic = program.bindings().iter().any(|binding| binding.group == 0);
        // Sparse layout slots: group(0) only when the shader consumes
        // automatic uniforms, group(1) only when material bindings exist.
        let owned_slots: Vec<Option<wgpu::BindGroupLayout>> = vec![
            uses_automatic.then(|| self.automatic_ring.bind_group_layout().clone()),
            group1_layout.clone(),
        ];
        let slots: Vec<Option<&wgpu::BindGroupLayout>> = owned_slots
            .iter()
            .map(|slot| slot.as_ref())
            .collect();
        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cesium pipeline layout"),
            bind_group_layouts: &slots,
            immediate_size: 0,
        });
        self.pipeline_layouts.insert(shader_hash, pipeline_layout.clone());
        pipeline_layout
    }

    /// Returns (creating if needed) the group(1) bind group layout from the
    /// program's reflection, or `None` when the shader declares no group(1).
    fn get_or_create_material_layout(&mut self, program: &ShaderProgram) -> Option<wgpu::BindGroupLayout> {
        let shader_hash = program.pipeline_hash();
        if let Some(layout) = self.material_layouts.get(&shader_hash) {
            return Some(layout.clone());
        }
        let entries: Vec<wgpu::BindGroupLayoutEntry> = program
            .bind_group_layout_entries()
            .into_iter()
            .find(|(group, _)| *group == 1)
            .map(|(_, entries)| entries)?;
        let layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material BGL"),
            entries: &entries,
        });
        self.material_layouts.insert(shader_hash, layout.clone());
        Some(layout)
    }

    /// Builds the group(1) bind group for one draw from its uniform
    /// overrides. Returns `(None, vec![])` when the shader has no group(1).
    fn create_material_bind_group(
        &mut self,
        program: &ShaderProgram,
        overrides: &[(String, UniformValue)],
    ) -> Option<(Option<wgpu::BindGroup>, Vec<u32>)> {
        let layout = match self.get_or_create_material_layout(program) {
            Some(layout) => layout,
            None => return Some((None, Vec::new())),
        };
        let bindings: Vec<_> = program
            .bindings()
            .iter()
            .filter(|binding| binding.group == 1)
            .cloned()
            .collect();

        // Pass A (mutable): allocate scratch slots, upload data, pick
        // textures. Pass B below only takes shared borrows.
        enum Slot {
            Uniform { offset: u32, size: u64 },
            Texture(Arc<Texture>),
            Sampler,
            Unsupported,
        }
        let mut slots: Vec<Slot> = Vec::with_capacity(bindings.len());
        let mut dynamic_offsets: Vec<u32> = Vec::new();
        let default_texture_id = {
            let texture = self.default_texture();
            texture.id().to_string()
        };
        for binding in &bindings {
            let override_value = overrides
                .iter()
                .find(|(name, _)| name == &binding.name)
                .map(|(_, value)| value);
            match binding.kind {
                BindingKind::UniformBuffer => {
                    let offset = match self.allocate_material_scratch(binding.byte_size as u64) {
                        Some(offset) => offset,
                        None => return None,
                    };
                    let mut bytes = vec![0u8; binding.byte_size as usize];
                    match override_value {
                        Some(UniformValue::Vec4(value)) => {
                            for (i, component) in value.iter().take(4).enumerate() {
                                bytes[i * 4..i * 4 + 4]
                                    .copy_from_slice(&component.to_le_bytes());
                            }
                        }
                        Some(UniformValue::Float(value)) => {
                            bytes[0..4].copy_from_slice(&value.to_le_bytes());
                        }
                        _ => {}
                    }
                    self.queue
                        .write_buffer(&self.material_scratch, offset as u64, &bytes);
                    dynamic_offsets.push(offset);
                    slots.push(Slot::Uniform {
                        offset,
                        size: binding.byte_size as u64,
                    });
                }
                BindingKind::Texture => {
                    let texture = match override_value {
                        Some(UniformValue::Texture(texture)) => texture.clone(),
                        _ => {
                            let _ = default_texture_id;
                            Arc::new(Texture::new(
                                &self.device,
                                crate::texture::TextureOptions {
                                    width: Some(1),
                                    height: Some(1),
                                    ..Default::default()
                                },
                            ))
                        }
                    };
                    slots.push(Slot::Texture(texture));
                }
                BindingKind::Sampler => slots.push(Slot::Sampler),
                _ => {
                    log::warn!("unsupported material binding kind for {}", binding.name);
                    slots.push(Slot::Unsupported);
                }
            }
        }

        // Pass B (shared borrows only): build the bind group entries.
        let texture_views: Vec<Option<wgpu::TextureView>> = slots
            .iter()
            .map(|slot| match slot {
                Slot::Texture(texture) => Some(texture.create_view()),
                _ => None,
            })
            .collect();
        let mut entries: Vec<wgpu::BindGroupEntry<'_>> = Vec::with_capacity(slots.len());
        for (index, (binding, slot)) in bindings.iter().zip(slots.iter()).enumerate() {
            match slot {
                Slot::Uniform { size, .. } => {
                    entries.push(wgpu::BindGroupEntry {
                        binding: binding.binding,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &self.material_scratch,
                            offset: 0,
                            size: std::num::NonZeroU64::new(*size),
                        }),
                    });
                }
                Slot::Texture(_) => {
                    if let Some(Some(view)) = texture_views.get(index) {
                        entries.push(wgpu::BindGroupEntry {
                            binding: binding.binding,
                            resource: wgpu::BindingResource::TextureView(view),
                        });
                    }
                }
                Slot::Sampler => {
                    entries.push(wgpu::BindGroupEntry {
                        binding: binding.binding,
                        resource: wgpu::BindingResource::Sampler(&self.default_sampler),
                    });
                }
                Slot::Unsupported => {}
            }
        }

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("material BG"),
            layout: &layout,
            entries: &entries,
        });
        Some((Some(bind_group), dynamic_offsets))
    }

    /// Allocates from the per-frame material scratch buffer (256-aligned).
    fn allocate_material_scratch(&mut self, size: u64) -> Option<u32> {
        let aligned = (self.material_scratch_next + MATERIAL_ALIGNMENT - 1)
            / MATERIAL_ALIGNMENT
            * MATERIAL_ALIGNMENT;
        if aligned + size > MATERIAL_SCRATCH_SIZE {
            log::warn!("material scratch buffer exhausted");
            return None;
        }
        self.material_scratch_next = aligned + size;
        Some(aligned as u32)
    }

    /// Phase 2: record one resolved run into the encoder as a render pass.
    fn record_run(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        run: &ResolvedRun,
        default_target: Option<&DefaultRenderTarget<'_>>,
    ) {
        if run.draws.is_empty() && matches!(run.color_load, wgpu::LoadOp::Load) {
            return;
        }

        // Resolve attachment views for this run.
        let offscreen_views: Option<(wgpu::TextureView, Option<wgpu::TextureView>)> =
            match &run.target {
                RunTarget::Offscreen(framebuffer) => match framebuffer.color_attachment_view(0) {
                    Some(color_view) => Some((color_view, framebuffer.depth_stencil_attachment_view())),
                    None => {
                        log::warn!("offscreen run skipped: framebuffer has no color attachment");
                        return;
                    }
                },
                RunTarget::Default => None,
            };

        let (color_view, color_format) = match (&run.target, &offscreen_views) {
            (RunTarget::Default, _) => match default_target {
                Some(target) => (target.view, target.format),
                None => {
                    log::warn!("run targeting default framebuffer skipped: no surface view");
                    return;
                }
            },
            (RunTarget::Offscreen(framebuffer), Some((view, _))) => (
                view,
                framebuffer
                    .color_attachment_format()
                    .unwrap_or(wgpu::TextureFormat::Rgba8Unorm),
            ),
            _ => return,
        };
        let _ = color_format;

        let depth_attachment = match (&run.target, &offscreen_views) {
            (RunTarget::Offscreen(framebuffer), Some((_, Some(depth_view)))) => {
                let format = framebuffer.depth_stencil_format();
                Some((depth_view, format))
            }
            _ => None,
        };

        let color_attachment = wgpu::RenderPassColorAttachment {
            view: color_view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: run.color_load,
                store: wgpu::StoreOp::Store,
            },
        };
        let depth_stencil_attachment = depth_attachment.map(|(view, format)| {
            // wgpu validation: stencil ops must be `None` for depth-only
            // attachments (e.g. Depth32Float) without a stencil aspect.
            let has_stencil = matches!(
                format,
                Some(wgpu::TextureFormat::Depth24PlusStencil8)
                    | Some(wgpu::TextureFormat::Depth32FloatStencil8)
            );
            wgpu::RenderPassDepthStencilAttachment {
                view,
                depth_ops: Some(wgpu::Operations {
                    load: match run.depth_clear {
                        Some(depth) => wgpu::LoadOp::Clear(depth),
                        None => wgpu::LoadOp::Clear(1.0),
                    },
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: has_stencil.then_some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0),
                    store: wgpu::StoreOp::Store,
                }),
            }
        });

        let color_attachments = [Some(color_attachment)];
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("cesium run"),
            color_attachments: &color_attachments,
            depth_stencil_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        for draw in &run.draws {
            let pipeline = match self.pipeline_cache.get(&draw.pipeline_key) {
                Some(pipeline) => pipeline,
                None => continue,
            };
            pass.set_pipeline(pipeline);
            if let Some(offset) = draw.automatic_offset {
                pass.set_bind_group(0, self.automatic_ring.bind_group(), &[offset]);
            }
            if let Some(bind_group) = &draw.material_bind_group {
                pass.set_bind_group(1, bind_group, &draw.material_dynamic_offsets);
            }
            let vertex_buffers = draw.vertex_array.vertex_buffers();
            for (slot, buffer) in vertex_buffers.iter().enumerate() {
                pass.set_vertex_buffer(slot as u32, buffer.wgpu_buffer().slice(..));
            }
            match draw.vertex_array.index_buffer() {
                Some(index_buffer) => {
                    pass.set_index_buffer(
                        index_buffer.wgpu_buffer().slice(..),
                        index_buffer.index_format(),
                    );
                    let start = draw.offset;
                    let end = draw.offset.saturating_add(draw.count);
                    pass.draw_indexed(start..end, 0, 0..draw.instance_count);
                }
                None => {
                    let start = draw.offset;
                    let end = draw.offset.saturating_add(draw.count);
                    pass.draw(start..end, 0..draw.instance_count);
                }
            }
        }
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
