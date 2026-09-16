//! M5-E2: Screen-Space Ambient Occlusion (SSAO) `ViewNode`.
//!
//! Self-implemented hemisphere 16-sample SSAO kernel + 4×4 box blur, inserted
//! into the M5-E0 render-graph infrastructure (`graph.rs`) between the
//! pass-through node and FXAA. Mirrors the [`super::fxaa`] `register_fxaa_node`
//! pattern: this module registers the node / resources / systems **but never
//! creates graph edges** — `graph.rs::register_render_graph` owns the single
//! linear chain so no diamond can form in `Core3d`.
//!
//! # Chain position (owned by graph.rs)
//! `Tonemapping → PassThrough → AmbientOcclusion → Fxaa → EndMainPassPostProcessing`
//! AO runs after tonemapping (HDR linear → LDR done) and before FXAA, matching
//! CesiumJS `PostProcessStageLibrary.createAmbientOcclusionStage` ordering.
//!
//! # Inputs: DepthPrepass + NormalPrepass
//! SSAO needs view-space depth + normals. Bevy exposes them through
//! [`ViewPrepassTextures`] once the camera carries [`DepthPrepass`] +
//! [`NormalPrepass`] (both previously **zero-usage** repo-wide). They are enabled
//! here in the adapter layer by [`setup_ao_prepass`] — **not** in the app-layer
//! camera bundle (orbit_camera.rs is out of scope per the M5-E2 red line).
//!
//! # Blueprint
//! - `packages/engine/Source/Shaders/PostProcessStages/AmbientOcclusionGenerate.glsl` L1-144
//! - `packages/engine/Source/Shaders/PostProcessStages/AmbientOcclusionModulate.glsl` L1-11
//! - `packages/engine/Source/Scene/PostProcessStageLibrary.js` L496 / L599
//! - `bevy_pbr-0.15.3/src/ssao/mod.rs` L224-323 (SsaoNode) / L682-772 (prepass binding)
//!
//! # DEVIATION
//! WGSL rewrite; hemisphere-kernel SSAO (not CesiumJS HBAO ray-march). See
//! `docs/deviations.md#dev-018`.

use std::sync::Mutex;

use bevy::core_pipeline::{
    core_3d::graph::Core3d,
    fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prepass::{DepthPrepass, NormalPrepass, ViewPrepassTextures},
};
use bevy::ecs::query::QueryItem;
use bevy::prelude::*;
use bevy::render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_graph::{
        NodeRunError, RenderGraphApp, RenderGraphContext, ViewNode, ViewNodeRunner,
    },
    render_resource::{
        binding_types::{sampler, texture_2d, texture_depth_2d, uniform_buffer},
        BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId,
        ColorTargetState, ColorWrites, FilterMode, FragmentState, MultisampleState, Operations,
        PipelineCache, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
        RenderPipelineDescriptor, Sampler as GpuSampler, SamplerBindingType, SamplerDescriptor,
        ShaderStages, Texture, TextureFormat, TextureSampleType, TextureView,
    },
    renderer::{RenderContext, RenderDevice},
    view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
    Render, RenderApp, RenderSet,
};

use super::graph::{create_post_process_texture, CesiumPostProcessLabel};

// ─── Shader handle ───────────────────────────────────────────────────────────

/// Unique handle for the embedded `ao.wgsl` shader (distinct from pass-through /
/// FXAA handles — see the collision test below).
pub const AO_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(0xCE51_E2E2_A0A0_0016);

/// Format of the AO intermediate buffer (single scalar AO replicated to RGBA).
const AO_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

// ─── Component ───────────────────────────────────────────────────────────────

/// Marker component enabling cesiumrust SSAO on a camera entity.
///
/// Extracted to the render world via `ExtractComponentPlugin`. The [`AoNode`]
/// early-returns when `enabled == false` (zero GPU cost). `enabled` is synced
/// from `PostProcessConfig::ambient_occlusion_enabled` by `ao_system`
/// (post_process.rs), mirroring `fxaa_system` → `CesiumFxaa`.
#[derive(Component, Clone, Debug, ExtractComponent)]
pub struct CesiumAmbientOcclusion {
    /// Master enable for the SSAO passes on this camera.
    pub enabled: bool,
}

impl Default for CesiumAmbientOcclusion {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// Per-view cached pipeline IDs (generate + blur/modulate) for the AO node.
#[derive(Component)]
pub struct CameraAoPipeline {
    pub generate_id: CachedRenderPipelineId,
    pub blur_id: CachedRenderPipelineId,
}

// ─── Pipeline ────────────────────────────────────────────────────────────────

/// Render-world resource: bind group layouts + samplers for the two AO passes.
///
/// Both passes share WGSL group(0) but disjoint binding indices:
/// - generate: bindings 0..3 (depth, normal, point sampler, view uniform)
/// - blur/modulate: bindings 4..6 (ao texture, colour texture, linear sampler)
#[derive(Resource)]
pub struct AoPipeline {
    pub generate_bind_group_layout: BindGroupLayout,
    pub blur_bind_group_layout: BindGroupLayout,
    /// Non-filtering sampler for the depth + normal prepass textures.
    pub point_sampler: GpuSampler,
    /// Filtering sampler for the AO box blur + colour modulation.
    pub linear_sampler: GpuSampler,
}

impl FromWorld for AoPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();

        // Generate pass: depth (0), normal (1), point sampler (2), view uniform (3).
        let generate_bind_group_layout = render_device.create_bind_group_layout(
            "cesium_ao_generate_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_depth_2d(),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::NonFiltering),
                    uniform_buffer::<ViewUniform>(true),
                ),
            ),
        );

        // Blur/modulate pass: ao texture (4), colour texture (5), linear sampler (6).
        let blur_bind_group_layout = render_device.create_bind_group_layout(
            "cesium_ao_blur_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::FRAGMENT,
                (
                    (4, texture_2d(TextureSampleType::Float { filterable: true })),
                    (5, texture_2d(TextureSampleType::Float { filterable: true })),
                    (6, sampler(SamplerBindingType::Filtering)),
                ),
            ),
        );

        let point_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_ao_point_sampler"),
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        let linear_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_ao_linear_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            generate_bind_group_layout,
            blur_bind_group_layout,
            point_sampler,
            linear_sampler,
        }
    }
}

// ─── ViewNode ────────────────────────────────────────────────────────────────

/// SSAO `ViewNode`: hemisphere 16-sample generate pass → AO intermediate texture,
/// then a 4×4 box-blur + modulate pass onto the (tonemapped) colour target.
///
/// Caches the fullscreen AO intermediate texture keyed by viewport size; bind
/// groups are rebuilt per frame (they depend on the per-frame prepass views +
/// post-process source). Per-frame bind-group caching is a known perf refinement
/// deferred to the M11.2 xvfb e2e (see `docs/deferred.md`).
#[derive(Default)]
pub struct AoNode {
    /// (width, height, texture, view) — recreated when the viewport size changes.
    cached_ao_texture: Mutex<Option<(u32, u32, Texture, TextureView)>>,
}

impl ViewNode for AoNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static ExtractedView,
        &'static CameraAoPipeline,
        &'static CesiumAmbientOcclusion,
        &'static ViewPrepassTextures,
        &'static ViewUniformOffset,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (target, _view, pipeline_ids, ao, prepass, view_uniform_offset): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if !ao.enabled {
            return Ok(());
        }

        // SSAO requires both depth + normal prepass inputs.
        let (Some(depth_view), Some(normal_view)) = (prepass.depth_view(), prepass.normal_view())
        else {
            return Ok(());
        };

        let pipeline_cache = world.resource::<PipelineCache>();
        let ao_pipeline = world.resource::<AoPipeline>();
        let view_uniforms = world.resource::<ViewUniforms>();

        let (Some(generate_pipeline), Some(blur_pipeline)) = (
            pipeline_cache.get_render_pipeline(pipeline_ids.generate_id),
            pipeline_cache.get_render_pipeline(pipeline_ids.blur_id),
        ) else {
            return Ok(());
        };

        let Some(view_uniform_binding) = view_uniforms.uniforms.binding() else {
            return Ok(());
        };

        // Clone the RenderDevice (Arc-backed, cheap) so it does not hold an
        // immutable borrow of `render_context` across the later mutable
        // `render_context.command_encoder()` calls.
        let render_device = render_context.render_device().clone();

        // ── AO intermediate texture (viewport-sized, cached across frames) ──
        let width = prepass.size.width.max(1);
        let height = prepass.size.height.max(1);
        let mut cached = self.cached_ao_texture.lock().unwrap();
        if cached
            .as_ref()
            .map(|(w, h, _, _)| (*w, *h) != (width, height))
            .unwrap_or(true)
        {
            let (texture, view) = create_post_process_texture(
                &render_device,
                "cesium_ao_texture",
                width,
                height,
                AO_TEXTURE_FORMAT,
            );
            *cached = Some((width, height, texture, view));
        }
        let ao_texture_view = &cached.as_ref().unwrap().3;

        // ── Pass 1: generate SSAO into the AO intermediate texture ──
        let generate_bind_group = render_device.create_bind_group(
            Some("cesium_ao_generate_bind_group"),
            &ao_pipeline.generate_bind_group_layout,
            &BindGroupEntries::sequential((
                depth_view,
                normal_view,
                &ao_pipeline.point_sampler,
                view_uniform_binding.clone(),
            )),
        );

        let generate_pass = RenderPassDescriptor {
            label: Some("cesium_ao_generate_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: ao_texture_view,
                resolve_target: None,
                // Fullscreen triangle covers every texel, so the clear value is
                // always overwritten; `Operations::default()` (clear + store) is
                // sufficient and matches the FXAA / pass-through nodes.
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        {
            let mut pass = render_context
                .command_encoder()
                .begin_render_pass(&generate_pass);
            pass.set_pipeline(generate_pipeline);
            pass.set_bind_group(0, &generate_bind_group, &[view_uniform_offset.offset]);
            pass.draw(0..3, 0..1); // fullscreen triangle
        }

        // ── Pass 2: 4×4 box blur + modulate onto the colour target ──
        let post_process = target.post_process_write();
        let source = post_process.source;
        let destination = post_process.destination;

        let blur_bind_group = render_device.create_bind_group(
            Some("cesium_ao_blur_bind_group"),
            &ao_pipeline.blur_bind_group_layout,
            &BindGroupEntries::with_indices((
                (4, ao_texture_view),
                (5, source),
                (6, &ao_pipeline.linear_sampler),
            )),
        );

        let blur_pass = RenderPassDescriptor {
            label: Some("cesium_ao_blur_modulate_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        {
            let mut pass = render_context
                .command_encoder()
                .begin_render_pass(&blur_pass);
            pass.set_pipeline(blur_pipeline);
            pass.set_bind_group(0, &blur_bind_group, &[]);
            pass.draw(0..3, 0..1); // fullscreen triangle
        }

        Ok(())
    }
}

// ─── Render systems ──────────────────────────────────────────────────────────

/// Prepares the two specialized AO render pipelines per enabled camera view.
/// Runs in `Render`, `RenderSet::Prepare`.
pub fn prepare_ao_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    ao_pipeline: Res<AoPipeline>,
    views: Query<(Entity, &ExtractedView, &CesiumAmbientOcclusion)>,
) {
    for (entity, view, ao) in &views {
        if !ao.enabled {
            continue;
        }

        // Destination format matches the post-process target (HDR vs LDR).
        let destination_format = if view.hdr {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::Rgba8UnormSrgb
        };

        let generate_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("cesium_ao_generate_pipeline".into()),
            layout: vec![ao_pipeline.generate_bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: AO_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "fragment_generate".into(),
                targets: vec![Some(ColorTargetState {
                    format: AO_TEXTURE_FORMAT,
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

        let blur_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("cesium_ao_blur_pipeline".into()),
            layout: vec![ao_pipeline.blur_bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: AO_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "fragment_blur_modulate".into(),
                targets: vec![Some(ColorTargetState {
                    format: destination_format,
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

        commands.entity(entity).insert(CameraAoPipeline {
            generate_id,
            blur_id,
        });
    }
}

// ─── Main-world systems ──────────────────────────────────────────────────────

/// Ensures every `Camera3d` carries the SSAO marker + the depth/normal prepass
/// components it needs. **Adapter-layer** enablement of `DepthPrepass` +
/// `NormalPrepass` (both previously zero-usage repo-wide) so the app-layer camera
/// bundle (`orbit_camera.rs`) does not need to change (M5-E2 red line).
///
/// Runs in `Update` **after** `ao_system` (which syncs the marker's `enabled`
/// flag from `PostProcessConfig::ambient_occlusion_enabled`). When AO is
/// disabled the prepass markers are removed so the extra geometry pass is not
/// incurred. Only added to the schedule when the post-process gate is ON, so
/// gate OFF ⇒ no prepass ⇒ v0 baselines pixel-neutral.
pub fn setup_ao_prepass(
    mut commands: Commands,
    cameras: Query<Entity, With<Camera3d>>,
    ao_markers: Query<&CesiumAmbientOcclusion>,
) {
    for entity in &cameras {
        let mut ecmd = commands.entity(entity);
        let enabled = match ao_markers.get(entity) {
            Ok(marker) => marker.enabled,
            Err(_) => {
                // First sighting: insert the marker (enabled=true); `ao_system`
                // reconciles it with the config on the next frame.
                ecmd.insert(CesiumAmbientOcclusion { enabled: true });
                true
            }
        };

        if enabled {
            ecmd.insert(DepthPrepass);
            ecmd.insert(NormalPrepass);
        } else {
            ecmd.remove::<DepthPrepass>();
            ecmd.remove::<NormalPrepass>();
        }
    }
}

// ─── Registration ────────────────────────────────────────────────────────────

/// Register the SSAO node into `RenderApp` (shader + extract + node + systems)
/// and enable the depth/normal prepass on cameras in the main world.
///
/// Called from `register_render_graph` (graph.rs) when the post-process gate is
/// ON. Registers the node **but does not create graph edges** — the unified
/// linear chain in graph.rs owns them (no diamond in `Core3d`). Mirrors
/// [`super::fxaa::register_fxaa_node`].
pub fn register_ao_node(app: &mut App) {
    // Register the AO WGSL shader (headless-safe via shader_registry).
    crate::shader_registry::try_load_internal_shader(
        app,
        AO_SHADER_HANDLE,
        include_str!("../../shaders/ao.wgsl"),
        "shaders/ao.wgsl",
    );

    // ExtractComponentPlugin: main → render world each frame (ExtractSchedule).
    app.add_plugins(ExtractComponentPlugin::<CesiumAmbientOcclusion>::default());

    // Main-world: attach DepthPrepass + NormalPrepass to cameras (SSAO inputs).
    // Added here (gate ON only) so the app-layer camera bundle stays untouched.
    app.add_systems(Update, setup_ao_prepass);

    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        // No render app (headless MinimalPlugins) — degrade gracefully.
        return;
    };

    render_app
        .init_resource::<AoPipeline>()
        .add_systems(Render, prepare_ao_pipelines.in_set(RenderSet::Prepare))
        .add_render_graph_node::<ViewNodeRunner<AoNode>>(
            Core3d,
            CesiumPostProcessLabel::AmbientOcclusion,
        );
    // NOTE: edges are created by `register_render_graph` (unified linear chain).
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ao_component_default_enabled() {
        assert!(CesiumAmbientOcclusion::default().enabled);
    }

    #[test]
    fn ao_shader_handle_unique() {
        // No collision with pass-through / FXAA handles.
        assert_ne!(AO_SHADER_HANDLE, super::super::graph::PASS_THROUGH_SHADER_HANDLE);
        assert_ne!(AO_SHADER_HANDLE, super::super::fxaa::FXAA_SHADER_HANDLE);
    }

    #[test]
    fn ao_headless_graceful() {
        // No RenderApp (headless) → register_ao_node must not panic.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        register_ao_node(&mut app);
    }

    /// Adapter-layer prepass enablement: a `Camera3d` gains the SSAO marker +
    /// `DepthPrepass` + `NormalPrepass` (the previously zero-usage inputs), and
    /// loses the prepass markers when AO is disabled. Headless-testable because
    /// it is a pure main-world ECS system — no GPU / render graph required.
    #[test]
    fn setup_ao_prepass_attaches_and_removes_prepass() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, setup_ao_prepass);

        let cam = app.world_mut().spawn(Camera3d::default()).id();

        // Frame 1: marker absent → inserted (enabled) + prepass attached.
        app.update();
        assert!(app.world().get::<CesiumAmbientOcclusion>(cam).is_some());
        assert!(app.world().get::<DepthPrepass>(cam).is_some());
        assert!(app.world().get::<NormalPrepass>(cam).is_some());

        // Disable AO (mirrors ao_system syncing config=false) → prepass removed.
        app.world_mut()
            .get_mut::<CesiumAmbientOcclusion>(cam)
            .unwrap()
            .enabled = false;
        app.update();
        assert!(app.world().get::<DepthPrepass>(cam).is_none());
        assert!(app.world().get::<NormalPrepass>(cam).is_none());
    }
}
