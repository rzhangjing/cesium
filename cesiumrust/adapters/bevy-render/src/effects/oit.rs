//! M6.4: cesiumrust OIT (Order-Independent Transparency) — MRT render graph adapter.
//!
//! Implements the Bevy ViewNode infrastructure for weighted-blended OIT using
//! two MRT render targets (Rgba16Float accumulation + R8Unorm revealage) and a
//! fullscreen composite pass.
//!
//! # Blueprints
//! - `packages/engine/Source/Scene/OIT.js` L1-947 (28KB main implementation)
//!   - L30-34: capability detection (drawBuffers && colorBufferFloat && depthTexture && floatBlend)
//!   - L137-156: updateTextures (accumulation RGBA FLOAT + revealage RGBA FLOAT)
//!   - L164-226: updateFramebuffers (MRT 2-attachment FBO)
//!   - L408-417: translucentMRTBlend (RGB additive, Alpha multiplicative)
//!   - L487-492: mrtShaderSource (Ci*wzi → FragData_0, ai*wzi → FragData_1)
//!   - L786-828: executeTranslucentCommandsSortedMRT
//!   - L872-874: composite execution
//! - `packages/engine/Source/Shaders/CompositeOITFS.glsl` L1-32 (901B composite)
//! - `packages/engine/Source/Shaders/Builtin/Functions/alphaWeight.glsl` L4-11
//! - `domain/effects/src/oit.rs` L1-366 (CPU reference: compute_weight, accumulate, composite)
//!
//! # Architecture
//! - `OitNode`: ViewNode running the MRT accumulate pass (reads scene colour + depth,
//!   outputs to 2 attachments with appropriate blend states).
//! - `OitCompositeNode`: ViewNode running the fullscreen composite (reads accumulate +
//!   revealage + opaque, writes final colour to destination).
//! - `OitTextureCache`: render-world resource holding per-view accumulate + revealage
//!   textures (recreated on viewport resize).
//! - Capability probe: `Plugin::finish` reads `RenderDevice::limits()` to verify
//!   `max_color_attachments >= 2` (MRT). Probe failure → gate OFF, no panic.
//!
//! # DEVIATIONS
//! See `docs/deviations.md#dev-031` (draft, integrator registers).

use std::collections::HashMap;
use std::sync::Mutex;

use bevy::core_pipeline::{
    core_3d::graph::Core3d,
    fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prepass::{DepthPrepass, ViewPrepassTextures},
};
use bevy::ecs::query::QueryItem;
use bevy::prelude::*;
use bevy::render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_graph::{
        NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
    },
    render_resource::{
        binding_types::{sampler, texture_2d, texture_depth_2d, uniform_buffer},
        BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, BlendComponent,
        BlendFactor, BlendOperation, BlendState, CachedRenderPipelineId, ColorTargetState,
        ColorWrites, Extent3d, FilterMode, FragmentState, MultisampleState,
        Operations, PipelineCache, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
        RenderPipelineDescriptor, Sampler as GpuSampler, SamplerBindingType, SamplerDescriptor,
        ShaderStages, Texture, TextureDescriptor, TextureDimension, TextureFormat,
        TextureSampleType, TextureUsages, TextureView,
    },
    renderer::{RenderContext, RenderDevice},
    view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
    Render, RenderApp, RenderSet,
};
// `wgpu::Color` is not re-exported by name through Bevy's `render_resource`
// (only `ColorTargetState`/`ColorWrites` are); the MRT accumulate pass needs it
// for per-frame attachment clears. See `Cargo.toml` for why the direct dep is
// version-safe (unifies to the same 23.0.1 instance Bevy uses).
use wgpu::Color as GpuColor;
use cesium_effects::{OitCapabilities, OitConfig as DomainOitConfig, OitMode};

use super::graph::gate_from_env_value;

// ─── Shader handles ─────────────────────────────────────────────────────────

/// Unique handle for the embedded `oit_accumulate.wgsl` shader.
pub const OIT_ACCUMULATE_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0xCE51_0170_0006_0004);

/// Unique handle for the embedded `oit_composite.wgsl` shader.
pub const OIT_COMPOSITE_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0xCE51_0170_0006_0005);

// ─── Gate ───────────────────────────────────────────────────────────────────

/// Env var gating the M6.4 OIT node.
///
/// **Single source of truth (task #81)**: the owner of this name is the app-layer
/// registry `application/cesium-app/src/feature_flags.rs` (`ENV_ENABLE_OIT`
/// listed in `RESERVED_FLAGS`, default OFF). This const is a *mirror* that
/// exists only because `cesium-app` depends on `cesium-bevy-render` (never the
/// reverse). Integration task #93 promotes it to ACTIVE.
pub const ENV_ENABLE_OIT: &str = "CESIUM_ENABLE_OIT";

/// Returns `true` when the OIT gate is enabled. Reuses the single authoritative
/// truthy parser (`gate_from_env_value`, the crate-wide `{1, true, yes, on}` set).
#[inline]
pub fn oit_gate_enabled() -> bool {
    gate_from_env_value(std::env::var(ENV_ENABLE_OIT).ok())
}

// ─── Render graph labels ────────────────────────────────────────────────────

/// Node label for the OIT accumulate (MRT) pass in `Core3d`.
///
/// Recommended insertion: `MainTransmissivePass → CesiumOitLabel → CesiumOitCompositeLabel → EndMainPass`.
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct CesiumOitLabel;

/// Node label for the OIT composite pass in `Core3d`.
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct CesiumOitCompositeLabel;

// ─── Component ──────────────────────────────────────────────────────────────

/// Marker component enabling cesiumrust OIT on a camera entity.
///
/// Extracted to render world via `ExtractComponentPlugin`. The `OitNode`
/// early-returns when `enabled == false` (zero GPU cost, gate OFF = v0 zero diff).
#[derive(Component, Clone, Debug, ExtractComponent)]
pub struct CesiumOit {
    /// Master enable for the OIT pass on this camera.
    pub enabled: bool,
}

impl Default for CesiumOit {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// Per-view cached pipeline ID for the OIT accumulate node.
#[derive(Component)]
pub struct CameraOitPipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

/// Per-view cached pipeline ID for the OIT composite node.
#[derive(Component)]
pub struct CameraOitCompositePipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

// ─── Adapter OitConfig Resource (preserved from scaffold) ───────────────────

#[derive(Resource, Debug, Clone)]
pub struct OitConfig {
    pub enabled: bool,
    pub mode: OitMode,
    pub depth_test: bool,
    pub depth_write: bool,
    pub accumulation_clear: [f64; 4],
    pub revealage_clear: f64,
}

impl Default for OitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: OitMode::None,
            depth_test: true,
            depth_write: true,
            accumulation_clear: [0.0, 0.0, 0.0, 0.0],
            revealage_clear: 1.0,
        }
    }
}

impl OitConfig {
    pub fn from_domain(config: &DomainOitConfig) -> Self {
        Self {
            enabled: config.is_active(),
            mode: config.mode,
            depth_test: true,
            depth_write: true,
            accumulation_clear: [0.0, 0.0, 0.0, 0.0],
            revealage_clear: 1.0,
        }
    }
}

// ─── OITPlugin ──────────────────────────────────────────────────────────────
// M6.1 Split scaffolding (`SplitConfig` / `SplitDragEvent` /
// `split_direction_system`) migrated to `effects::split` in FIX-SPLIT (Phase 3);
// this plugin now initialises only the OIT config.
pub struct OITPlugin;

impl Plugin for OITPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OitConfig>();
    }
}

// ─── OIT Pipeline Resource ──────────────────────────────────────────────────

/// Render-world resource: bind group layouts + samplers for OIT passes.
#[derive(Resource)]
pub struct OitPipeline {
    /// Layout for the accumulate pass (depth + scene_colour + sampler + view uniform).
    pub accumulate_bind_group_layout: BindGroupLayout,
    /// Layout for the composite pass (opaque + accumulation + revealage + sampler).
    pub composite_bind_group_layout: BindGroupLayout,
    /// Point sampler (non-filtering, for depth).
    pub point_sampler: GpuSampler,
    /// Linear sampler (filtering, for colour textures).
    pub linear_sampler: GpuSampler,
}

impl FromWorld for OitPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();

        // Accumulate pass: depth_prepass(0) + scene_color(1) + point_sampler(2) + view(3)
        let accumulate_bind_group_layout = render_device.create_bind_group_layout(
            "cesium_oit_accumulate_bgl",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_depth_2d(),
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    sampler(SamplerBindingType::NonFiltering),
                    uniform_buffer::<ViewUniform>(true),
                ),
            ),
        );

        // Composite pass: opaque(0) + accumulation(1) + revealage(2) + linear_sampler(3)
        let composite_bind_group_layout = render_device.create_bind_group_layout(
            "cesium_oit_composite_bgl",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );

        let point_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_oit_point_sampler"),
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        let linear_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_oit_linear_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            accumulate_bind_group_layout,
            composite_bind_group_layout,
            point_sampler,
            linear_sampler,
        }
    }
}

// ─── OIT Texture Cache ──────────────────────────────────────────────────────

/// Per-view OIT intermediate textures (accumulation + revealage).
///
/// Recreated on viewport resize. Shared between `OitNode` (writes) and
/// `OitCompositeNode` (reads) within the same frame.
#[derive(Resource, Default)]
pub struct OitTextureCache {
    pub cache: Mutex<HashMap<Entity, OitViewTextures>>,
}

pub struct OitViewTextures {
    pub width: u32,
    pub height: u32,
    pub accumulation_texture: Texture,
    pub revealage_texture: Texture,
    /// Bindable views for the composite pass (kept as TextureView objects).
    pub accumulation_bind_view: TextureView,
    pub revealage_bind_view: TextureView,
}

/// Create or retrieve OIT textures for a view entity. Returns `None` if allocation fails.
fn ensure_oit_textures(
    cache: &OitTextureCache,
    render_device: &RenderDevice,
    entity: Entity,
    width: u32,
    height: u32,
) -> bool {
    let mut map = cache.cache.lock().unwrap();
    if let Some(existing) = map.get(&entity) {
        if existing.width == width && existing.height == height {
            return true; // Already allocated at correct size.
        }
    }

    // Allocate accumulation texture (Rgba16Float).
    let accumulation_texture = render_device.create_texture(&TextureDescriptor {
        label: Some("cesium_oit_accumulation"),
        size: Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba16Float,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let accumulation_bind_view = accumulation_texture.create_view(&Default::default());

    // Allocate revealage texture (R8Unorm).
    let revealage_texture = render_device.create_texture(&TextureDescriptor {
        label: Some("cesium_oit_revealage"),
        size: Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::R8Unorm,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let revealage_bind_view = revealage_texture.create_view(&Default::default());

    map.insert(
        entity,
        OitViewTextures {
            width,
            height,
            accumulation_texture,
            revealage_texture,
            accumulation_bind_view,
            revealage_bind_view,
        },
    );
    true
}

// ─── OitNode (MRT accumulate pass) ─────────────────────────────────────────

/// ViewNode that runs the OIT MRT accumulate pass.
///
/// Reads scene colour + depth prepass, outputs to two render targets:
/// - Attachment 0 (Rgba16Float): additive blend → Σ(Ci·wzi), Σ(ai·wzi)
/// - Attachment 1 (R8Unorm): multiplicative blend → Π(1−ai)
#[derive(Default)]
pub struct OitNode;

impl ViewNode for OitNode {
    type ViewQuery = (
        Entity,
        &'static ViewTarget,
        &'static ExtractedView,
        &'static CameraOitPipeline,
        &'static CesiumOit,
        &'static ViewPrepassTextures,
        &'static ViewUniformOffset,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (entity, target, _view, pipeline_handle, oit, prepass, view_uniform_offset): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if !oit.enabled {
            return Ok(());
        }

        let pipeline_cache = world.resource::<PipelineCache>();
        let oit_pipeline = world.resource::<OitPipeline>();
        let texture_cache = world.resource::<OitTextureCache>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_handle.pipeline_id) else {
            return Ok(());
        };

        let Some(depth_view) = prepass.depth_view() else {
            return Ok(());
        };

        // Ensure OIT textures exist for this view.
        let target_size = target.main_texture_view();
        let _ = target_size; // Used indirectly through the texture cache.
        let viewport_size = world.resource::<OitCapabilitiesResource>();
        if !viewport_size.mrt_supported {
            return Ok(()); // Capability probe failed — graceful exit.
        }

        // Get texture dimensions from the view target.
        let (width, height) = {
            let cache = texture_cache.cache.lock().unwrap();
            if let Some(tex) = cache.get(&entity) {
                (tex.width, tex.height)
            } else {
                return Ok(()); // Textures not yet prepared.
            }
        };
        let _ = (width, height);

        // Build bind group: depth + scene source + sampler + view uniform.
        let post_process = target.post_process_write();
        let source = post_process.source;

        let view_uniforms = world.resource::<ViewUniforms>();
        let Some(view_uniform_buffer) = view_uniforms.uniforms.binding() else {
            return Ok(());
        };

        let bind_group = render_context.render_device().create_bind_group(
            Some("cesium_oit_accumulate_bg"),
            &oit_pipeline.accumulate_bind_group_layout,
            &BindGroupEntries::sequential((
                depth_view,
                source,
                &oit_pipeline.point_sampler,
                view_uniform_buffer,
            )),
        );

        // Get the OIT texture views for MRT attachments.
        let cache = texture_cache.cache.lock().unwrap();
        let Some(textures) = cache.get(&entity) else {
            return Ok(());
        };

        let pass_descriptor = RenderPassDescriptor {
            label: Some("cesium_oit_accumulate_pass"),
            color_attachments: &[
                // Attachment 0: accumulation (Rgba16Float, additive blend)
                Some(RenderPassColorAttachment {
                    view: &textures.accumulation_bind_view,
                    resolve_target: None,
                    ops: Operations {
                        load: bevy::render::render_resource::LoadOp::Clear(GpuColor::TRANSPARENT),
                        store: bevy::render::render_resource::StoreOp::Store,
                    },
                }),
                // Attachment 1: revealage (R8Unorm, multiplicative blend)
                Some(RenderPassColorAttachment {
                    view: &textures.revealage_bind_view,
                    resolve_target: None,
                    ops: Operations {
                        load: bevy::render::render_resource::LoadOp::Clear(GpuColor::WHITE),
                        store: bevy::render::render_resource::StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        let mut render_pass = render_context
            .command_encoder()
            .begin_render_pass(&pass_descriptor);

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[view_uniform_offset.offset]);
        render_pass.draw(0..3, 0..1); // fullscreen triangle

        Ok(())
    }
}

// ─── OitCompositeNode (fullscreen composite) ────────────────────────────────

/// ViewNode that composites OIT buffers with the opaque scene.
///
/// Reads accumulation + revealage textures and the opaque scene colour,
/// applies the CompositeOITFS formula, writes the final blended result.
#[derive(Default)]
pub struct OitCompositeNode;

impl ViewNode for OitCompositeNode {
    type ViewQuery = (
        Entity,
        &'static ViewTarget,
        &'static CameraOitCompositePipeline,
        &'static CesiumOit,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (entity, target, pipeline_handle, oit): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if !oit.enabled {
            return Ok(());
        }

        let pipeline_cache = world.resource::<PipelineCache>();
        let oit_pipeline = world.resource::<OitPipeline>();
        let texture_cache = world.resource::<OitTextureCache>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_handle.pipeline_id) else {
            return Ok(());
        };

        let post_process = target.post_process_write();
        let source = post_process.source; // opaque scene
        let destination = post_process.destination;

        // Get OIT texture views.
        let cache = texture_cache.cache.lock().unwrap();
        let Some(textures) = cache.get(&entity) else {
            return Ok(());
        };

        let bind_group = render_context.render_device().create_bind_group(
            Some("cesium_oit_composite_bg"),
            &oit_pipeline.composite_bind_group_layout,
            &BindGroupEntries::sequential((
                source,
                &textures.accumulation_bind_view,
                &textures.revealage_bind_view,
                &oit_pipeline.linear_sampler,
            )),
        );
        drop(cache);

        let pass_descriptor = RenderPassDescriptor {
            label: Some("cesium_oit_composite_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        let mut render_pass = render_context
            .command_encoder()
            .begin_render_pass(&pass_descriptor);

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1); // fullscreen triangle

        Ok(())
    }
}

// ─── Capability probe resource ──────────────────────────────────────────────

/// Render-world resource storing the OIT capability probe result.
///
/// Populated during `Plugin::finish` by reading `RenderDevice::limits()`.
/// Probe failure → `mrt_supported = false` → nodes early-return (never panic).
#[derive(Resource, Debug, Clone, Default)]
pub struct OitCapabilitiesResource {
    pub mrt_supported: bool,
    pub max_color_attachments: u32,
}

impl OitCapabilitiesResource {
    /// Probe the render device for OIT MRT support.
    ///
    /// The only reliable runtime discriminator in wgpu/WebGPU is
    /// `max_color_attachments >= 2` (maps to upstream `context.drawBuffers`).
    /// Float blending and color-buffer-float are core-guaranteed in WebGPU
    /// (no runtime query API exists); depth textures likewise.
    ///
    /// Corresponds to OIT.js L30-34:
    /// ```js
    /// extensionsSupported = colorBufferFloat && depthTexture && floatBlend;
    /// _translucentMRTSupport = drawBuffers && extensionsSupported;
    /// ```
    pub fn probe(render_device: &RenderDevice) -> Self {
        let limits = render_device.limits();
        let max_ca = limits.max_color_attachments;
        let mrt_supported = max_ca >= 2;

        Self {
            mrt_supported,
            max_color_attachments: max_ca,
        }
    }

    /// Convert to domain capabilities for interop.
    pub fn to_domain_caps(&self) -> OitCapabilities {
        OitCapabilities {
            mrt_supported: self.mrt_supported,
            // In wgpu/WebGPU these are core-guaranteed when Rgba16Float is renderable.
            float_blend_supported: self.mrt_supported,
            depth_texture_supported: true,
            color_buffer_float: true,
        }
    }
}

// ─── Render systems ─────────────────────────────────────────────────────────

/// Prepares OIT textures and specialized pipelines per camera view.
/// Runs in `Render` schedule, `RenderSet::Prepare`.
pub fn prepare_oit_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
    oit_pipeline: Res<OitPipeline>,
    caps: Res<OitCapabilitiesResource>,
    texture_cache: Res<OitTextureCache>,
    views: Query<(Entity, &ExtractedView, &CesiumOit, &ViewTarget)>,
) {
    if !caps.mrt_supported {
        return;
    }

    for (entity, view, oit, _target) in &views {
        if !oit.enabled {
            continue;
        }

        let width = view.viewport.z.max(1);
        let height = view.viewport.w.max(1);

        // Ensure OIT intermediate textures are allocated.
        ensure_oit_textures(&texture_cache, &render_device, entity, width, height);

        // Accumulate pipeline: 2 color targets (Rgba16Float additive + R8Unorm multiplicative).
        let accumulate_pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("cesium_oit_accumulate_pipeline".into()),
            layout: vec![oit_pipeline.accumulate_bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: OIT_ACCUMULATE_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "fragment".into(),
                targets: vec![
                    // Attachment 0: Rgba16Float, additive blend (ONE + ONE)
                    Some(ColorTargetState {
                        format: TextureFormat::Rgba16Float,
                        blend: Some(BlendState {
                            color: BlendComponent {
                                src_factor: BlendFactor::One,
                                dst_factor: BlendFactor::One,
                                operation: BlendOperation::Add,
                            },
                            alpha: BlendComponent {
                                src_factor: BlendFactor::One,
                                dst_factor: BlendFactor::One,
                                operation: BlendOperation::Add,
                            },
                        }),
                        write_mask: ColorWrites::ALL,
                    }),
                    // Attachment 1: R8Unorm, multiplicative blend (ZERO + ONE_MINUS_SRC)
                    Some(ColorTargetState {
                        format: TextureFormat::R8Unorm,
                        blend: Some(BlendState {
                            color: BlendComponent {
                                src_factor: BlendFactor::Zero,
                                dst_factor: BlendFactor::OneMinusSrc,
                                operation: BlendOperation::Add,
                            },
                            alpha: BlendComponent {
                                src_factor: BlendFactor::Zero,
                                dst_factor: BlendFactor::OneMinusSrc,
                                operation: BlendOperation::Add,
                            },
                        }),
                        write_mask: ColorWrites::RED,
                    }),
                ],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges: Vec::new(),
            zero_initialize_workgroup_memory: false,
        });

        // Composite pipeline: single target (view format), no blend.
        let output_format = if view.hdr {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::Rgba8UnormSrgb
        };

        let composite_pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("cesium_oit_composite_pipeline".into()),
            layout: vec![oit_pipeline.composite_bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: OIT_COMPOSITE_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: output_format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges: Vec::new(),
            zero_initialize_workgroup_memory: false,
        });

        commands.entity(entity).insert((
            CameraOitPipeline { pipeline_id: accumulate_pipeline_id },
            CameraOitCompositePipeline { pipeline_id: composite_pipeline_id },
        ));
    }
}

/// Main-world system: attaches `DepthPrepass` to cameras with `CesiumOit`.
///
/// The accumulate pass needs depth for weight computation.
pub fn setup_oit_prepass(
    mut commands: Commands,
    cameras: Query<Entity, (With<CesiumOit>, Without<DepthPrepass>)>,
) {
    for entity in &cameras {
        commands.entity(entity).insert(DepthPrepass);
    }
}

// ─── Registration ───────────────────────────────────────────────────────────

/// Register the OIT nodes into `RenderApp` (shader + extract + nodes + systems).
///
/// Called from `M6WaveARenderGraphPlugin` when the OIT gate is ON.
/// Does NOT create graph edges — integrator #93 owns the chain topology.
#[deprecated = "DEV-029 / FIX-REG-FACADE: call `register_oit_node_main_world` from `Plugin::build` and `register_oit_node_render_world` from `Plugin::finish`; this facade runs the finish half against a possibly device-less render world."]
pub fn register_oit_node(app: &mut App) {
    register_oit_node_main_world(app);
    // Headless `MinimalPlugins` has no `RenderApp` — degrade gracefully.
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        register_oit_node_render_world(render_app);
    }
}

/// `Plugin::build`-time half: everything that lives in the **main** world.
///
/// Split out per DEV-029 pattern. `OitPipeline`'s `FromWorld` reads `RenderDevice`,
/// and Bevy only inserts `RenderDevice` into the render world in `RenderPlugin::finish`,
/// so the render-world half must be called from `finish`.
pub fn register_oit_node_main_world(app: &mut App) {
    // Register OIT WGSL shaders (headless-safe).
    crate::shader_registry::try_load_internal_shader(
        app,
        OIT_ACCUMULATE_SHADER_HANDLE,
        include_str!("../../shaders/oit_accumulate.wgsl"),
        "shaders/oit_accumulate.wgsl",
    );
    crate::shader_registry::try_load_internal_shader(
        app,
        OIT_COMPOSITE_SHADER_HANDLE,
        include_str!("../../shaders/oit_composite.wgsl"),
        "shaders/oit_composite.wgsl",
    );

    // ExtractComponentPlugin for CesiumOit.
    app.add_plugins(ExtractComponentPlugin::<CesiumOit>::default());

    // Main-world prepass setup.
    app.add_systems(bevy::app::Last, setup_oit_prepass);
}

/// `Plugin::finish`-time half: the render-world pipeline resources + nodes.
///
/// **MUST** be called from `Plugin::finish` (DEV-029: RenderDevice only exists
/// after RenderPlugin::finish inserts it).
pub fn register_oit_node_render_world(render_app: &mut bevy::app::SubApp) {
    // FIX-REG-FACADE (DEV-029): degrade to a no-op when `RenderDevice` is absent
    // (finish half reached from `build`, or a bare render world) — the capability
    // probe below dereferences it. See
    // `crate::effects::render_world_missing_device`.
    if crate::effects::render_world_missing_device(render_app) {
        return;
    }

    // Capability probe — reads RenderDevice.limits() (only available in finish).
    let render_device = render_app
        .world()
        .resource::<RenderDevice>()
        .clone();
    let caps = OitCapabilitiesResource::probe(&render_device);

    render_app
        .insert_resource(caps)
        .init_resource::<OitPipeline>()
        .init_resource::<OitTextureCache>()
        .add_systems(Render, prepare_oit_pipelines.in_set(RenderSet::Prepare))
        .add_render_graph_node::<ViewNodeRunner<OitNode>>(Core3d, CesiumOitLabel)
        .add_render_graph_node::<ViewNodeRunner<OitCompositeNode>>(Core3d, CesiumOitCompositeLabel);
    // NOTE: edges are created by integrator #93 (wire_m6_edges or equivalent).
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_effects::OitCapabilities;

    #[test]
    fn test_oit_config_default() {
        let config = OitConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.mode, OitMode::None);
        assert!(config.depth_test);
    }

    #[test]
    fn test_oit_config_from_domain_active() {
        let domain = DomainOitConfig::default();
        let config = OitConfig::from_domain(&domain);
        assert!(!config.enabled);
    }

    #[test]
    fn test_oit_config_from_domain_mrt() {
        let caps = OitCapabilities {
            mrt_supported: true,
            float_blend_supported: true,
            depth_texture_supported: true,
            color_buffer_float: true,
        };
        let domain = DomainOitConfig::from_capabilities(&caps);
        let config = OitConfig::from_domain(&domain);
        assert!(config.enabled);
        assert_eq!(config.mode, OitMode::WeightedBlendedMrt);
    }

    // M6.1 Split tests (`test_split_config_*` / `test_split_direction_properties`)
    // migrated with the scaffolding to `effects::split` in FIX-SPLIT (Phase 3).

    // ─── New M6.4 tests ─────────────────────────────────────────────────────

    #[test]
    fn test_oit_gate_default_off() {
        // Without env var set, gate should be OFF.
        std::env::remove_var(ENV_ENABLE_OIT);
        assert!(!oit_gate_enabled());
    }

    #[test]
    fn test_oit_shader_handles_unique() {
        assert_ne!(OIT_ACCUMULATE_SHADER_HANDLE, OIT_COMPOSITE_SHADER_HANDLE);
        assert_ne!(
            OIT_ACCUMULATE_SHADER_HANDLE,
            super::super::graph::PASS_THROUGH_SHADER_HANDLE
        );
    }

    #[test]
    fn test_cesium_oit_component_default() {
        let oit = CesiumOit::default();
        assert!(oit.enabled);
    }

    #[test]
    fn test_oit_capabilities_resource_default() {
        let caps = OitCapabilitiesResource::default();
        assert!(!caps.mrt_supported);
        assert_eq!(caps.max_color_attachments, 0);
    }

    #[test]
    fn test_oit_capabilities_to_domain() {
        let caps = OitCapabilitiesResource {
            mrt_supported: true,
            max_color_attachments: 8,
        };
        let domain = caps.to_domain_caps();
        assert!(domain.mrt_supported);
        assert!(domain.float_blend_supported);
        assert!(domain.depth_texture_supported);
        assert!(domain.color_buffer_float);
        assert!(domain.translucent_mrt_supported());
    }

    #[test]
    fn test_oit_headless_graceful() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // Should not panic (no RenderApp).
        #[allow(deprecated)]
        register_oit_node(&mut app);
    }

    // ─── Naga defence line: parse + validate + binding coverage ─────────────

    /// Stubs for `#import` directives that naga cannot resolve.
    const OIT_WGSL_IMPORT_STUBS: &str = "\
struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
struct View {
    view_from_clip: mat4x4<f32>,
    clip_from_view: mat4x4<f32>,
    view_from_world: mat4x4<f32>,
    world_from_view: mat4x4<f32>,
    clip_from_world: mat4x4<f32>,
    world_from_clip: mat4x4<f32>,
    world_position: vec3<f32>,
    near: f32,
    far: f32,
    width: f32,
    height: f32,
    viewport: vec4<f32>,
    frustum: vec4<f32>,
}
";

    fn oit_stubbed_wgsl(path: &str) -> String {
        let mut source = String::from(OIT_WGSL_IMPORT_STUBS);
        for line in path.lines() {
            if line.starts_with("#import") {
                continue;
            }
            source.push_str(line);
            source.push('\n');
        }
        source
    }

    #[test]
    fn oit_accumulate_wgsl_parses_and_type_checks_under_naga() {
        let source = oit_stubbed_wgsl(include_str!("../../shaders/oit_accumulate.wgsl"));
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("oit_accumulate.wgsl does not parse:\n{}", error.emit_to_string(&source))
        });
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("oit_accumulate.wgsl does not validate");

        let entry_points: Vec<_> = module
            .entry_points
            .iter()
            .map(|e| (e.name.as_str(), e.stage))
            .collect();
        assert_eq!(
            entry_points,
            vec![("fragment", naga::ShaderStage::Fragment)],
            "oit_accumulate.wgsl must expose exactly one fragment entry point"
        );
    }

    #[test]
    fn oit_composite_wgsl_parses_and_type_checks_under_naga() {
        let source = oit_stubbed_wgsl(include_str!("../../shaders/oit_composite.wgsl"));
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("oit_composite.wgsl does not parse:\n{}", error.emit_to_string(&source))
        });
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("oit_composite.wgsl does not validate");

        let entry_points: Vec<_> = module
            .entry_points
            .iter()
            .map(|e| (e.name.as_str(), e.stage))
            .collect();
        assert_eq!(
            entry_points,
            vec![("fragment", naga::ShaderStage::Fragment)],
            "oit_composite.wgsl must expose exactly one fragment entry point"
        );
    }

    #[test]
    fn oit_accumulate_wgsl_bindings_covered_by_layout() {
        let source = oit_stubbed_wgsl(include_str!("../../shaders/oit_accumulate.wgsl"));
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("oit_accumulate.wgsl parse failed:\n{}", error.emit_to_string(&source))
        });

        // Rust layout: group(0) bindings 0..3 (depth, scene_color, point_sampler, view)
        let layout: std::collections::BTreeSet<(u32, u32)> =
            [(0, 0), (0, 1), (0, 2), (0, 3)].into_iter().collect();

        let entry = module
            .entry_points
            .iter()
            .find(|e| e.name == "fragment")
            .expect("must have fragment entry");

        let mut used = std::collections::BTreeSet::new();
        let mut visited = std::collections::HashSet::new();
        let mut stack: Vec<&naga::Function> = vec![&entry.function];
        while let Some(func) = stack.pop() {
            for (_, expr) in func.expressions.iter() {
                match *expr {
                    naga::Expression::GlobalVariable(handle) => {
                        if let Some(binding) = &module.global_variables[handle].binding {
                            used.insert((binding.group, binding.binding));
                        }
                    }
                    naga::Expression::CallResult(func_handle) if visited.insert(func_handle) => {
                        stack.push(&module.functions[func_handle]);
                    }
                    _ => {}
                }
            }
        }

        let missing: Vec<(u32, u32)> = used.difference(&layout).copied().collect();
        assert!(
            missing.is_empty(),
            "oit_accumulate.wgsl uses bindings {missing:?} absent from OitPipeline layout"
        );
    }

    #[test]
    fn oit_composite_wgsl_bindings_covered_by_layout() {
        let source = oit_stubbed_wgsl(include_str!("../../shaders/oit_composite.wgsl"));
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("oit_composite.wgsl parse failed:\n{}", error.emit_to_string(&source))
        });

        // Rust layout: group(0) bindings 0..3 (opaque, accumulation, revealage, linear_sampler)
        let layout: std::collections::BTreeSet<(u32, u32)> =
            [(0, 0), (0, 1), (0, 2), (0, 3)].into_iter().collect();

        let entry = module
            .entry_points
            .iter()
            .find(|e| e.name == "fragment")
            .expect("must have fragment entry");

        let mut used = std::collections::BTreeSet::new();
        let mut visited = std::collections::HashSet::new();
        let mut stack: Vec<&naga::Function> = vec![&entry.function];
        while let Some(func) = stack.pop() {
            for (_, expr) in func.expressions.iter() {
                match *expr {
                    naga::Expression::GlobalVariable(handle) => {
                        if let Some(binding) = &module.global_variables[handle].binding {
                            used.insert((binding.group, binding.binding));
                        }
                    }
                    naga::Expression::CallResult(func_handle) if visited.insert(func_handle) => {
                        stack.push(&module.functions[func_handle]);
                    }
                    _ => {}
                }
            }
        }

        let missing: Vec<(u32, u32)> = used.difference(&layout).copied().collect();
        assert!(
            missing.is_empty(),
            "oit_composite.wgsl uses bindings {missing:?} absent from OitPipeline layout"
        );
    }
}
