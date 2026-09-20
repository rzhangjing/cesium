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
//! `EndMainPass → PassThrough → AmbientOcclusion → Tonemapping → Fxaa → EndMainPassPostProcessing`
//! AO runs on the HDR scene colour **before** tonemapping and FXAA runs on the
//! final LDR image **after** tonemapping — matching the upstream CesiumJS
//! `PostProcessStageCollection` order (AO → … → Tonemapping → … → FXAA last).
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

use std::collections::HashMap;
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
use super::post_process::PostProcessConfig;

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
/// Both passes share WGSL group(0). The `view` uniform (binding 3) is statically
/// used by **both** entry points, so it must appear in **both** layouts (Ryan C2 —
/// the blur layout previously omitted it, which made `create_render_pipeline`
/// layout validation fail, the blur pipeline resolve to `None`, and the `AND`
/// early-return in [`AoNode::run`] silently no-op the whole AO node — pixels then
/// equalled gate OFF and `pixel_diff` reported a false green):
/// - generate: bindings 0..=3 (depth, normal, point sampler, view uniform)
/// - blur/modulate: bindings 3..=6 (view uniform, ao texture, colour texture, linear sampler)
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

        // Blur/modulate pass: view uniform (3, shared with generate — Ryan C2),
        // ao texture (4), colour texture (5), linear sampler (6).
        let blur_bind_group_layout = render_device.create_bind_group_layout(
            "cesium_ao_blur_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::FRAGMENT,
                (
                    (3, uniform_buffer::<ViewUniform>(true)),
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
/// then a 4×4 box-blur + modulate pass onto the scene colour target.
///
/// Caches the fullscreen AO intermediate texture **per view entity** (Ryan L3 — the
/// previous single-slot cache thrashed and carried an implicit ordering dependency
/// whenever more than one camera was active). Bind groups are rebuilt per frame
/// (they depend on the per-frame prepass views + post-process source); per-frame
/// bind-group caching is a known perf refinement deferred to the M11.2 xvfb e2e
/// (see `docs/deferred.md`).
#[derive(Default)]
pub struct AoNode {
    /// view entity → (width, height, texture, view); an entry is recreated when
    /// that view's viewport size changes.
    cached_ao_textures: Mutex<HashMap<Entity, (u32, u32, Texture, TextureView)>>,
}

impl ViewNode for AoNode {
    type ViewQuery = (
        Entity,
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
        (view_entity, target, _view, pipeline_ids, ao, prepass, view_uniform_offset): QueryItem<
            Self::ViewQuery,
        >,
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

        // ── AO intermediate texture (viewport-sized, cached per view entity) ──
        // Ryan L3: keyed by view entity so multiple active cameras never thrash a
        // shared slot (the previous single-slot `Mutex<Option<..>>` also carried an
        // implicit cross-view ordering dependency). An entry is recreated only when
        // that view's viewport size changes.
        let width = prepass.size.width.max(1);
        let height = prepass.size.height.max(1);
        let mut cache = self.cached_ao_textures.lock().unwrap();
        let needs_recreate = cache
            .get(&view_entity)
            .map(|(w, h, _, _)| (*w, *h) != (width, height))
            .unwrap_or(true);
        if needs_recreate {
            let (texture, view) = create_post_process_texture(
                &render_device,
                "cesium_ao_texture",
                width,
                height,
                AO_TEXTURE_FORMAT,
            );
            cache.insert(view_entity, (width, height, texture, view));
        }
        let ao_texture_view = &cache.get(&view_entity).unwrap().3;

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
                (3, view_uniform_binding.clone()),
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
            // Ryan C2: the blur layout now binds the dynamic-offset view uniform
            // (binding 3, shared with generate), so its offset must be supplied
            // here — a `&[]` offset list would fail the dynamic-buffer validation.
            pass.set_bind_group(0, &blur_bind_group, &[view_uniform_offset.offset]);
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
/// Runs in `Update`. When AO is disabled the prepass markers are removed so the
/// extra geometry pass is not incurred — **except** where a co-located clipping /
/// IBL effect still needs them (FIX-AO-PREPASS, see below). Only added to the
/// schedule when the post-process gate is ON, so gate OFF ⇒ no prepass ⇒ v0
/// baselines pixel-neutral.
///
/// FIX-AO-PREPASS: `DepthPrepass` / `NormalPrepass` are **shared** with the
/// clipping and IBL effects, whose own `setup_*_prepass` systems are insert-only
/// (never remove) and run in the same `Update` with no cross-system ordering. The
/// old blanket `remove` here therefore yanked a marker a clipping / IBL camera had
/// just been given, with the outcome dependent on scheduler order (and
/// `NormalPrepass` has no other inserter at all for an AO+clipping camera). The
/// removal is now guarded by each camera's *persistent driver components* —
/// `CesiumClippingPlanes` (wants `DepthPrepass`) and `CesiumIbl` (wants both) —
/// so the decision never reads the transient marker itself and is order-independent.
#[allow(clippy::type_complexity)] // Bevy system param: (Entity, Option<&C>, Option<&I>) filter
pub fn setup_ao_prepass(
    mut commands: Commands,
    config: Res<PostProcessConfig>,
    cameras: Query<
        (
            Entity,
            Option<&super::clipping_planes::CesiumClippingPlanes>,
            Option<&super::ibl::CesiumIbl>,
        ),
        With<Camera3d>,
    >,
    ao_markers: Query<&CesiumAmbientOcclusion>,
) {
    for (entity, clipping, ibl) in &cameras {
        let mut ecmd = commands.entity(entity);
        let enabled = match ao_markers.get(entity) {
            Ok(marker) => marker.enabled,
            Err(_) => {
                // First sighting: seed the marker from the config so the AO sub-gate
                // (Daniel M2 — `PostProcessConfig::ambient_occlusion_enabled`) decides
                // whether AO is on at all; `ao_system` reconciles it every frame after.
                ecmd.insert(CesiumAmbientOcclusion {
                    enabled: config.ambient_occlusion_enabled,
                });
                config.ambient_occlusion_enabled
            }
        };

        if enabled {
            ecmd.insert(DepthPrepass);
            ecmd.insert(NormalPrepass);
        } else {
            // FIX-AO-PREPASS: only drop a shared marker when *this* camera has no
            // other active consumer of it. `NormalPrepass` is demanded by AO + IBL;
            // `DepthPrepass` by AO + clipping + IBL. Presence of the driver component
            // is the demand signal (a disabled-but-present effect is treated as a
            // consumer, which is conservative: it can only avoid an over-eager
            // removal, never break the other effect).
            if ibl.is_none() {
                ecmd.remove::<NormalPrepass>();
            }
            if clipping.is_none() && ibl.is_none() {
                ecmd.remove::<DepthPrepass>();
            }
        }
    }
}

#[cfg(test)]
mod ao_prepass_tests {
    use super::*;
    use crate::effects::clipping_planes::CesiumClippingPlanes;
    use crate::effects::ibl::CesiumIbl;
    use cesium_effects::ibl::{IblMaterial, ImageBasedLighting};

    fn app_with(ao_enabled: bool) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(PostProcessConfig {
            ambient_occlusion_enabled: ao_enabled,
            ..Default::default()
        });
        app.add_systems(Update, setup_ao_prepass);
        app
    }

    fn has<T: Component>(app: &App, e: Entity) -> bool {
        app.world().get::<T>(e).is_some()
    }

    /// FIX-AO-PREPASS: AO off with no other consumer ⇒ AO owns the markers and
    /// removes both.
    #[test]
    fn ao_off_yanks_prepass_when_no_other_consumer() {
        let mut app = app_with(false);
        let cam = app
            .world_mut()
            .spawn((Camera3d::default(), DepthPrepass, NormalPrepass))
            .id();
        app.update();
        assert!(!has::<DepthPrepass>(&app, cam), "no consumer ⇒ depth removed");
        assert!(!has::<NormalPrepass>(&app, cam), "no consumer ⇒ normal removed");
    }

    /// FIX-AO-PREPASS: AO off on a camera that also drives clipping / IBL must NOT
    /// yank the shared markers those effects insert (and rely on).
    #[test]
    fn ao_off_preserves_shared_prepass_for_clipping_and_ibl() {
        let mut app = app_with(false);
        let clip_cam = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                CesiumClippingPlanes::default(),
                DepthPrepass,
                NormalPrepass,
            ))
            .id();
        let ibl_cam = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                CesiumIbl::new(ImageBasedLighting::default(), IblMaterial::default()),
                DepthPrepass,
                NormalPrepass,
            ))
            .id();
        app.update();
        // Clipping keeps DepthPrepass; NormalPrepass has no other consumer here → yanked.
        assert!(has::<DepthPrepass>(&app, clip_cam), "clipping must retain depth prepass");
        assert!(!has::<NormalPrepass>(&app, clip_cam), "no ibl ⇒ normal prepass may be yanked");
        // IBL demands both → neither is yanked.
        assert!(has::<DepthPrepass>(&app, ibl_cam), "ibl must retain depth prepass");
        assert!(has::<NormalPrepass>(&app, ibl_cam), "ibl must retain normal prepass");
    }

    /// AO on ⇒ AO inserts both markers (unchanged happy path).
    #[test]
    fn ao_on_inserts_depth_and_normal_prepass() {
        let mut app = app_with(true);
        let cam = app.world_mut().spawn(Camera3d::default()).id();
        app.update();
        assert!(has::<DepthPrepass>(&app, cam), "AO on ⇒ depth prepass inserted");
        assert!(has::<NormalPrepass>(&app, cam), "AO on ⇒ normal prepass inserted");
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
#[deprecated = "DEV-029 / FIX-REG-FACADE: call `register_ao_node_main_world` from `Plugin::build` and `register_ao_node_render_world` from `Plugin::finish`; this facade runs the finish half against a possibly device-less render world."]
pub fn register_ao_node(app: &mut App) {
    register_ao_node_main_world(app);
    // Headless `MinimalPlugins` has no `RenderApp` — degrade gracefully.
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        register_ao_node_render_world(render_app);
    }
}

/// `Plugin::build`-time half of [`register_ao_node`]: everything that lives in
/// the **main** world (WGSL shader asset + `ExtractComponentPlugin` + the
/// `setup_ao_prepass` system that attaches `DepthPrepass`/`NormalPrepass`).
///
/// Split out by task #81 — see `docs/deviations.md#dev-029`. `AoPipeline`'s
/// `FromWorld` reads `RenderDevice`, which Bevy only inserts into the render
/// world in `RenderPlugin::finish`, so the render-world half below must run from
/// a plugin's `finish`, never from its `build`.
pub fn register_ao_node_main_world(app: &mut App) {
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
}

/// `Plugin::finish`-time half of [`register_ao_node`]: the render-world pipeline
/// resources + the `Core3d` node.
pub fn register_ao_node_render_world(render_app: &mut bevy::app::SubApp) {
    // FIX-REG-FACADE (DEV-029): degrade to a no-op when `RenderDevice` is absent
    // (finish half reached from `build`, or a bare render world). See
    // `crate::effects::render_world_missing_device`.
    if crate::effects::render_world_missing_device(render_app) {
        return;
    }

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
        #[allow(deprecated)]
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
        // AO sub-gate ON (Daniel M2) — `setup_ao_prepass` now seeds the marker from
        // the config, so the resource must exist and enable AO for this test.
        app.insert_resource(PostProcessConfig {
            ambient_occlusion_enabled: true,
            ..Default::default()
        });
        app.add_systems(Update, setup_ao_prepass);

        let cam = app.world_mut().spawn(Camera3d::default()).id();

        // Frame 1: marker absent → seeded enabled from config + prepass attached.
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

    /// Daniel M2: with the AO sub-gate OFF (config default), the first sighting
    /// seeds the marker **disabled** and attaches no prepass — so gate OFF incurs
    /// zero extra geometry passes and the v0 baselines stay pixel-neutral.
    #[test]
    fn setup_ao_prepass_off_seeds_disabled_no_prepass() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<PostProcessConfig>(); // ambient_occlusion_enabled = false
        app.add_systems(Update, setup_ao_prepass);

        let cam = app.world_mut().spawn(Camera3d::default()).id();
        app.update();
        assert!(
            !app.world().get::<CesiumAmbientOcclusion>(cam).unwrap().enabled,
            "AO sub-gate off ⇒ marker seeded disabled"
        );
        assert!(app.world().get::<DepthPrepass>(cam).is_none());
        assert!(app.world().get::<NormalPrepass>(cam).is_none());
    }

    // ─── Ryan C1 defence line: headless naga parse + validate + layout parity ──

    /// naga has no preprocessor, so the two `#import`s in `ao.wgsl` are replaced by
    /// struct stubs declaring exactly the fields the shader reads:
    /// `FullscreenVertexOutput.{position, uv}` and
    /// `View.{view_from_clip, view_from_world, clip_from_view, viewport}`. The
    /// `view` **binding** itself is declared by the real shader text (group 0,
    /// binding 3), so it is deliberately not stubbed here. Everything else is the
    /// actual shader source, so this validates the real SSAO code.
    const AO_WGSL_IMPORT_STUBS: &str = "\
struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct View {
    view_from_clip: mat4x4<f32>,
    view_from_world: mat4x4<f32>,
    clip_from_view: mat4x4<f32>,
    viewport: vec4<f32>,
}
";

    fn ao_stubbed_wgsl() -> String {
        let mut source = String::from(AO_WGSL_IMPORT_STUBS);
        for line in include_str!("../../shaders/ao.wgsl").lines() {
            if line.starts_with("#import") {
                continue;
            }
            source.push_str(line);
            source.push('\n');
        }
        source
    }

    /// The strongest headless evidence that the SSAO shader is real: it is parsed
    /// and type-checked by **naga**, the same WGSL front end `bevy_render` compiles
    /// it with on the GPU path. A literal device readback still needs xvfb
    /// (`.github/workflows/cesiumrust-e2e.yml`, M11.2).
    #[test]
    fn ao_wgsl_parses_and_type_checks_under_naga() {
        let source = ao_stubbed_wgsl();
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("ao.wgsl does not parse:\n{}", error.emit_to_string(&source))
        });
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("ao.wgsl does not validate");

        let mut entry_points = module
            .entry_points
            .iter()
            .map(|entry| (entry.name.as_str(), entry.stage))
            .collect::<Vec<_>>();
        entry_points.sort_by_key(|(name, _)| *name);
        assert_eq!(
            entry_points,
            vec![
                ("fragment_blur_modulate", naga::ShaderStage::Fragment),
                ("fragment_generate", naga::ShaderStage::Fragment),
            ],
            "ao.wgsl must expose exactly the two fragment entry points"
        );
    }

    /// Walk an entry point's call graph and collect every `(group, binding)` it
    /// statically references — `GlobalVariable` expressions, recursing through
    /// local-function `CallResult` expressions (e.g. `view` is only touched inside
    /// `reconstruct_view_position` / `load_view_normal`).
    fn entry_used_bindings(
        module: &naga::Module,
        entry: &naga::EntryPoint,
    ) -> std::collections::BTreeSet<(u32, u32)> {
        let mut out = std::collections::BTreeSet::new();
        let mut visited = std::collections::HashSet::new();
        let mut stack: Vec<&naga::Function> = vec![&entry.function];
        while let Some(func) = stack.pop() {
            for (_, expr) in func.expressions.iter() {
                match *expr {
                    naga::Expression::GlobalVariable(handle) => {
                        if let Some(binding) = &module.global_variables[handle].binding {
                            out.insert((binding.group, binding.binding));
                        }
                    }
                    naga::Expression::CallResult(func_handle)
                        if visited.insert(func_handle) =>
                    {
                        stack.push(&module.functions[func_handle]);
                    }
                    _ => {}
                }
            }
        }
        out
    }

    /// Ryan C1 defence line (catches the **C2** class of bug at test time): every
    /// binding an entry point statically uses must be present in the matching Rust
    /// `BindGroupLayout`. The blur entry uses the shared `view` uniform (group 0,
    /// binding 3); were the Rust blur layout to omit it again, `create_render_pipeline`
    /// would fail, the blur pipeline would resolve to `None`, and the whole AO node
    /// would silently no-op — a false green in `pixel_diff` (gate-OFF-equal pixels).
    #[test]
    fn ao_wgsl_entry_bindings_are_covered_by_the_rust_layouts() {
        let source = ao_stubbed_wgsl();
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("ao.wgsl does not parse:\n{}", error.emit_to_string(&source))
        });

        // Mirror of `AoPipeline::from_world` (group 0 binding indices).
        let generate_layout: std::collections::BTreeSet<(u32, u32)> =
            [(0, 0), (0, 1), (0, 2), (0, 3)].into_iter().collect();
        let blur_layout: std::collections::BTreeSet<(u32, u32)> =
            [(0, 3), (0, 4), (0, 5), (0, 6)].into_iter().collect();

        for entry in &module.entry_points {
            let used = entry_used_bindings(&module, entry);
            let layout = match entry.name.as_str() {
                "fragment_generate" => &generate_layout,
                "fragment_blur_modulate" => &blur_layout,
                other => panic!("unexpected ao.wgsl entry point `{other}`"),
            };
            let missing: Vec<(u32, u32)> = used.difference(layout).copied().collect();
            assert!(
                missing.is_empty(),
                "ao.wgsl entry `{}` statically uses bindings {missing:?} that are absent from its \
                 Rust BindGroupLayout (C2 regression: create_render_pipeline would fail and AO \
                 would silently no-op)",
                entry.name
            );
        }
    }
}
