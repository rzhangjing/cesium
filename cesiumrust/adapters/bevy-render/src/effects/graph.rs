//! M5-E0: cesiumrust RenderGraph infrastructure.
//!
//! Establishes the first `ViewNode` + `Extract<Component>` + `ExtractSchedule` +
//! `RenderApp` assembly pattern for cesiumrust. Provides reusable helpers for
//! M5-E1 (FXAA) and M5-E2 (SSAO) post-process nodes.
//!
//! # Architecture
//! - Gate: `CESIUM_ENABLE_POSTPROCESS` env var (same as `feature_flags::postprocess_enabled()`)
//! - Gate OFF → **no render graph nodes registered** → v0 baselines zero-diff (PSNR=∞)
//! - Gate ON → pass-through node inserted in `Core3d` between `EndMainPass` and
//!   `Tonemapping`, proving pixel-neutral infrastructure. Daniel H2 chain order
//!   (upstream CesiumJS parity): `EndMainPass → PassThrough → AmbientOcclusion →
//!   Tonemapping → Fxaa → EndMainPassPostProcessing` — AO before tonemapping (HDR
//!   scene), FXAA last (final LDR image).
//!
//! # DEVIATION
//! cesiumrust uses Bevy's RenderGraph (sub-graph `Core3d`, `ViewNode` trait,
//! `RenderApp` sub-app extraction) whereas the blueprint cesium-rs uses a raw
//! wgpu render-pass chain without a graph abstraction. See `docs/deviations.md#dev-016`.
//!
//! # References (Bevy 0.15.3)
//! - `bevy_pbr/src/ssao/mod.rs` L25–131: ViewNode + ExtractSchedule + RenderApp pattern
//! - `bevy_core_pipeline/src/fxaa/node.rs`: simplest post-process ViewNode (86 lines)
//! - `bevy_core_pipeline/src/core_3d/mod.rs` L6–41: `Core3d`/`Node3d` labels
//! - `bevy_core_pipeline/src/fullscreen_vertex_shader/`: fullscreen triangle vertex state

use std::sync::Mutex;

use bevy::core_pipeline::{
    core_3d::graph::{Core3d, Node3d},
    fullscreen_vertex_shader::fullscreen_shader_vertex_state,
};
use bevy::ecs::query::QueryItem;
use bevy::prelude::*;
use bevy::render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_graph::{
        InternedRenderLabel, NodeRunError, RenderGraph, RenderGraphApp, RenderGraphContext,
        RenderLabel, ViewNode, ViewNodeRunner,
    },
    render_resource::{
        binding_types::{sampler, texture_2d},
        BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
        CachedRenderPipelineId, ColorTargetState, ColorWrites, Extent3d, FilterMode,
        FragmentState, MultisampleState, Operations, PipelineCache, PrimitiveState,
        RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor,
        Sampler as GpuSampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
        SpecializedRenderPipeline, SpecializedRenderPipelines, Texture, TextureDescriptor,
        TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView,
        TextureViewDescriptor, TextureViewId,
    },
    renderer::{RenderContext, RenderDevice},
    view::{ExtractedView, ViewTarget},
    Render, RenderApp, RenderSet,
};

// ─── Shader handle ───────────────────────────────────────────────────────────

/// Unique handle for the embedded `pass_through.wgsl` shader.
/// Value chosen to avoid collision with Bevy internal handles.
pub const PASS_THROUGH_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0xCE51_E0E0_F00D_0001);

// ─── Gate ────────────────────────────────────────────────────────────────────

/// Env var read by `feature_flags::postprocess_enabled()` in cesium-app.
/// Duplicated here because adapter layer cannot import application layer (DDD).
const ENV_ENABLE_POSTPROCESS: &str = "CESIUM_ENABLE_POSTPROCESS";

/// Env var read by `feature_flags::postprocess_builtin_enabled()` (M4.2 gate for
/// tonemapping / bloom / HDR fog). Duplicated here for the same DDD reason.
const ENV_ENABLE_POSTPROCESS_BUILTIN: &str = "CESIUM_ENABLE_POSTPROCESS_BUILTIN";

/// Returns `true` when the post-process gate is enabled.
/// Reads the same env var as `feature_flags::postprocess_enabled()`.
/// Gates the M5-E render-graph chain (pass-through + FXAA + AO).
#[inline]
pub fn postprocess_gate_enabled() -> bool {
    gate_from_env_value(std::env::var(ENV_ENABLE_POSTPROCESS).ok())
}

/// Returns `true` when the built-in post-process gate is enabled.
/// Reads the same env var as `feature_flags::postprocess_builtin_enabled()`.
/// Gates the M4.2 fog clear-color system (tonemapping / bloom live on the
/// camera bundle in orbit_camera.rs). Kept separate from the M5-E FXAA/AO gate
/// so the two features can be toggled independently (leader ruling Q5).
#[inline]
pub fn builtin_gate_enabled() -> bool {
    gate_from_env_value(std::env::var(ENV_ENABLE_POSTPROCESS_BUILTIN).ok())
}

// Daniel H1 / L2: the authoritative gate parser lives in `pipeline::fetch` — the
// crate-wide 4-token truthy set `{1, true, yes, on}`, trimmed + lowercased
// (byte-identical to `feature_flags::truthy`). The former local 2-token copy here
// (`"1" | "true"`, untrimmed) diverged from it, and `effects::gate_from_env_value`
// shadowed a different meaning for the same name. Re-export the single source of
// truth so `postprocess_gate_enabled` / `builtin_gate_enabled` and every
// `effects::*` consumer agree.
pub use crate::pipeline::fetch::gate_from_env_value;

// ─── Render graph labels ─────────────────────────────────────────────────────

/// Custom node labels for cesium post-process nodes in the `Core3d` sub-graph.
///
/// Pre-declares labels for M5-E1 (FXAA) and M5-E2 (AO) so they do not need
/// separate label definitions — just reuse these variants.
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub enum CesiumPostProcessLabel {
    /// M5-E0: infrastructure proof-of-concept pass-through node.
    PassThrough,
    /// M5-E1: FXAA anti-aliasing node.
    Fxaa,
    /// M5-E2: Screen-space ambient occlusion node.
    AmbientOcclusion,
}

// ─── Component ───────────────────────────────────────────────────────────────

/// Marker component on cameras that should run the cesium post-process chain.
/// Extracted to the render world via `ExtractSchedule`.
#[derive(Component, Clone, Debug, Default, ExtractComponent)]
pub struct CesiumPassThrough {
    /// When `false`, the node's `run()` early-returns (zero cost).
    pub enabled: bool,
}

/// Per-view pipeline ID stored in the render world after extraction + preparation.
#[derive(Component)]
pub struct CameraPassThroughPipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

// ─── Pipeline ────────────────────────────────────────────────────────────────

/// Render-world resource holding the bind group layout and sampler for the
/// pass-through (and future post-process) pipelines.
#[derive(Resource)]
pub struct PassThroughPipeline {
    pub texture_bind_group_layout: BindGroupLayout,
    pub sampler: GpuSampler,
}

impl FromWorld for PassThroughPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();

        let texture_bind_group_layout = render_device.create_bind_group_layout(
            "cesium_pass_through_texture_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );

        let sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_pass_through_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture_bind_group_layout,
            sampler,
        }
    }
}

/// Specialization key: only the output texture format varies (HDR vs LDR).
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct PassThroughPipelineKey {
    pub texture_format: TextureFormat,
}

impl SpecializedRenderPipeline for PassThroughPipeline {
    type Key = PassThroughPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("cesium_pass_through_pipeline".into()),
            layout: vec![self.texture_bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: PASS_THROUGH_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: key.texture_format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges: Vec::new(),
            zero_initialize_workgroup_memory: false,
        }
    }
}

// ─── ViewNode ────────────────────────────────────────────────────────────────

/// The cesiumrust pass-through `ViewNode` — first of its kind in this codebase.
///
/// Semantics (matching CesiumJS `PassThrough.glsl`): sample the input texture
/// and write it unchanged to the output. Pixel-neutral by construction.
#[derive(Default)]
pub struct PassThroughNode {
    cached_texture_bind_group: Mutex<Option<(TextureViewId, BindGroup)>>,
}

impl ViewNode for PassThroughNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static CameraPassThroughPipeline,
        &'static CesiumPassThrough,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (target, pipeline_handle, pass_through): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        // FIX-GRAPH-WIRING / FIX-CAPPROBE: this is still a *component-enabled*
        // early-return only — it reads the node's `enabled` flag. The pure
        // capability-probe / quality-tier **core** now exists
        // (`crate::effects::capability::{probe_quality_tier, QualityTier, ...}`,
        // headless-tested), but it is not yet wired to *gate rendering on the
        // tier*: that needs a real `RenderDevice`→snapshot probe + frame-time
        // instrumentation to prove the plan's "tier=off → baseline ±3%", both
        // GPU-gated and deferred (M11.6 / `docs/deferred.md#68`). Under headless
        // there is no `RenderAdapter`/`Features` to interrogate, so degrading to
        // the enabled-check (not a guessed tier) is the correct, honest behaviour.
        if !pass_through.enabled {
            return Ok(());
        }

        let pipeline_cache = world.resource::<PipelineCache>();
        let pass_through_pipeline = world.resource::<PassThroughPipeline>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_handle.pipeline_id) else {
            return Ok(());
        };

        let post_process = target.post_process_write();
        let source = post_process.source;
        let destination = post_process.destination;

        // Cache bind group across frames (invalidated when source texture changes).
        let mut cached = self.cached_texture_bind_group.lock().unwrap();
        let bind_group = match &mut *cached {
            Some((id, bg)) if source.id() == *id => bg,
            slot => {
                let bg = render_context.render_device().create_bind_group(
                    Some("cesium_pass_through_bind_group"),
                    &pass_through_pipeline.texture_bind_group_layout,
                    &BindGroupEntries::sequential((source, &pass_through_pipeline.sampler)),
                );
                let (_, bg) = slot.insert((source.id(), bg));
                bg
            }
        };

        let pass_descriptor = RenderPassDescriptor {
            label: Some("cesium_pass_through_pass"),
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
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1); // fullscreen triangle

        Ok(())
    }
}

// ─── Render systems ──────────────────────────────────────────────────────────

/// Prepares specialized pipelines for each camera view that has [`CesiumPassThrough`].
/// Runs in `Render` schedule, `RenderSet::Prepare`.
pub fn prepare_pass_through_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<PassThroughPipeline>>,
    pass_through_pipeline: Res<PassThroughPipeline>,
    views: Query<(Entity, &ExtractedView, &CesiumPassThrough)>,
) {
    for (entity, view, pass_through) in &views {
        if !pass_through.enabled {
            continue;
        }

        let texture_format = if view.hdr {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::Rgba8UnormSrgb
        };

        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pass_through_pipeline,
            PassThroughPipelineKey { texture_format },
        );

        commands
            .entity(entity)
            .insert(CameraPassThroughPipeline { pipeline_id });
    }
}

// `ExtractSchedule` is demonstrated via `ExtractComponentPlugin<CesiumPassThrough>`
// registered in `register_render_graph`. The plugin internally uses
// `Extract<Query<(Entity, &CesiumPassThrough)>>` in `ExtractSchedule` to copy
// the component from the main world to the render world each frame.
// This is the same pattern used by Bevy's `FxaaPlugin` (bevy_core_pipeline 0.15.3).

// ─── Reusable helpers ────────────────────────────────────────────────────────

/// Insert a render graph node **between** two existing labels in `Core3d`.
///
/// Creates edges: `predecessor → new_label → successor`, and — since M6 Wave A
/// (task #81) — **first removes** the pre-existing `predecessor → successor`
/// edge so the insertion is genuinely *serial*.
///
/// # Why the removal is mandatory
/// `RenderGraph::add_node_edges` only ever *adds*. Leaving the original
/// `predecessor → successor` edge in place yields a **diamond**: the graph then
/// contains both the direct edge and the `predecessor → new_label → successor`
/// path, so topological sort is free to schedule the inserted node anywhere
/// between the two (including *after* the successor's consumers, or in parallel
/// with the successor). That is exactly the defect class Ultra Review flagged as
/// Daniel H2, and Lee raised it for the M6.3 panorama slot specifically. Bevy
/// 0.15.3's `RenderGraphApp` extension trait exposes **no** removal helper, so
/// the `RenderGraph` resource has to be reached directly — see
/// [`remove_core3d_edge`].
///
/// # Usage (M6.3 panorama)
/// ```text
/// insert_node_in_core3d(
///     render_app,
///     CesiumPanoramaLabel,
///     Node3d::MainOpaquePass,          // predecessor
///     Node3d::MainTransmissivePass,    // successor
/// );
/// // then: render_app.add_render_graph_node::<ViewNodeRunner<PanoramaNode>>(Core3d, CesiumPanoramaLabel);
/// ```
///
/// # Note
/// The node itself must be added separately via `add_render_graph_node` before
/// or after calling this helper (the helper only manages edges).
pub fn insert_node_in_core3d(
    render_app: &mut bevy::app::SubApp,
    new_label: impl RenderLabel,
    predecessor: impl RenderLabel + Clone,
    successor: impl RenderLabel + Clone,
) {
    remove_core3d_edge(render_app, predecessor.clone(), successor.clone());
    render_app.add_render_graph_edges(Core3d, (predecessor, new_label, successor));
}

/// Remove a `Core3d` node edge, tolerating "the edge is not there".
///
/// `RenderGraph::remove_node_edge` returns
/// `Err(RenderGraphError::EdgeDoesNotExist)` when the edge is absent — its doc
/// sentence "if either node does not exist then nothing happens" describes the
/// *effect*, not the return value, and the call never panics. A missing edge is
/// the normal case for this helper's callers (they remove defensively before
/// re-wiring a serial chain, and the removal must stay idempotent so the
/// function can be called from any plugin-build order), so the error is logged
/// at `debug!` and swallowed.
///
/// Also degrades silently when there is no `RenderGraph` resource or no `Core3d`
/// sub-graph (headless `MinimalPlugins`), matching the `add_render_graph_*`
/// family's `warn!`-and-continue posture.
fn remove_core3d_edge(
    render_app: &mut bevy::app::SubApp,
    output_node: impl RenderLabel,
    input_node: impl RenderLabel,
) {
    let Some(mut render_graph) = render_app.world_mut().get_resource_mut::<RenderGraph>() else {
        return;
    };
    let Some(graph) = render_graph.get_sub_graph_mut(Core3d) else {
        return;
    };
    if let Err(err) = graph.remove_node_edge(output_node, input_node) {
        bevy::log::debug!("remove_core3d_edge: {err} (tolerated: edge was already absent)");
    }
}

/// Add a `Core3d` node edge, tolerating an already-present edge.
///
/// The counterpart to [`remove_core3d_edge`], required because
/// `RenderGraph::add_node_edge` `unwrap()`s `try_add_node_edge`, so rebuilding the
/// same serial chain twice (plugin build order is not fully controllable from
/// `main.rs`; see `wiring_is_idempotent`) would panic with `EdgeAlreadyExists` on
/// the intra-node edges created by the first pass. [`wire_m6_edges`] rebuilds its
/// chain edge-by-edge, so an existing edge is the *normal* second-call case:
/// swallow exactly that error and keep every other failure loud (an `InvalidNode`
/// still means a node was never registered — the property the fixed-tuple
/// `add_render_graph_edges` gave for free). Also degrades to a no-op without a
/// `RenderGraph` / `Core3d` (headless `MinimalPlugins`).
fn add_core3d_edge(
    render_app: &mut bevy::app::SubApp,
    output_node: impl RenderLabel,
    input_node: impl RenderLabel,
) {
    let Some(mut render_graph) = render_app.world_mut().get_resource_mut::<RenderGraph>() else {
        return;
    };
    let Some(graph) = render_graph.get_sub_graph_mut(Core3d) else {
        return;
    };
    if let Err(err) = graph.try_add_node_edge(output_node, input_node) {
        match err {
            bevy::render::render_graph::RenderGraphError::EdgeAlreadyExists { .. } => {
                bevy::log::debug!("add_core3d_edge: {err} (tolerated: idempotent re-wire)");
            }
            other => panic!("add_core3d_edge failed: {other}"),
        }
    }
}

/// Create a fullscreen-resolution texture suitable for post-process intermediate
/// targets (e.g., AO blur buffer, FXAA history).
///
/// Returns a `(Texture, TextureView)` pair. The caller is responsible for
/// lifetime management (typically stored in a render-world resource/component).
pub fn create_post_process_texture(
    render_device: &RenderDevice,
    label: &str,
    width: u32,
    height: u32,
    format: TextureFormat,
) -> (Texture, TextureView) {
    let size = Extent3d {
        width: width.max(1),
        height: height.max(1),
        depth_or_array_layers: 1,
    };

    let texture = render_device.create_texture(&TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::TEXTURE_BINDING
            | TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let view = texture.create_view(&TextureViewDescriptor::default());
    (texture, view)
}

// ─── Plugin registration entry point ─────────────────────────────────────────

/// Register the cesium render-graph infrastructure in `RenderApp`.
///
/// Called by [`CesiumEffectsPlugin`](super::CesiumEffectsPlugin) **only** when
/// `postprocess_gate_enabled()` returns `true`. Gate OFF = no-op = zero nodes =
/// pixel-neutral (v0 baselines unchanged).
///
/// This function demonstrates the full pattern:
/// 1. `get_sub_app_mut(RenderApp)` — headless-safe (returns `None` without render plugin)
/// 2. Register shader via `shader_registry::try_load_internal_shader`
/// 3. Init render-world resources
/// 4. Add systems to `ExtractSchedule` and `Render`
/// 5. Add node + edges to `Core3d` sub-graph
///
/// M5-E1: also registers the FXAA node ([`super::fxaa::register_fxaa_node`]).
/// M5-E2: also registers the SSAO node ([`super::ao::register_ao_node`]) and
/// wires the **single linear chain** (Daniel H2, upstream CesiumJS parity)
/// `EndMainPass → PassThrough → AmbientOcclusion → Tonemapping → Fxaa →
/// EndMainPassPostProcessing` — AO before tonemapping (HDR scene), FXAA last
/// (final LDR image).
/// All nodes are registered here (never in isolation) so their edges cannot
/// form a diamond. The pass-through node is pixel-neutral and default-disabled
/// (`CesiumPassThrough::default().enabled == false` → early-return, zero GPU
/// cost); FXAA runs when the camera carries `CesiumFxaa { enabled: true }`.
#[deprecated = "DEV-029 / FIX-REG-FACADE: call `register_render_graph_main_world` from `Plugin::build` and `register_render_graph_render_world` (or `finish_render_graph`) from `Plugin::finish`; this facade runs the finish half against a possibly device-less render world."]
pub fn register_render_graph(app: &mut App) {
    register_render_graph_main_world(app);
    // RenderApp assembly — headless-safe (returns None without RenderPlugin).
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        register_render_graph_render_world(render_app);
    }
}

/// `Plugin::finish` entry point for the cesium post-process render graph.
///
/// **Why this exists** (task #81, `docs/deviations.md#dev-029`): every
/// `*Pipeline::from_world` in this module reads `RenderDevice`, and Bevy only
/// inserts `RenderDevice` / `RenderQueue` / `RenderAdapter` into the render world
/// in `RenderPlugin::finish` (`bevy_render/src/lib.rs` L399-430) — *not* in its
/// `build`. So `render_app.init_resource::<PassThroughPipeline>()` (and the
/// FXAA/AO equivalents) invoked from a plugin's `build` panics with "Requested
/// resource RenderDevice does not exist in the World" on **any** real-GPU run
/// with `CESIUM_ENABLE_POSTPROCESS=1`. Bevy splits exactly this way itself
/// (`bevy_pbr/src/ssao/mod.rs` L54 `build` = main world, L80 `finish` =
/// `init_resource::<SsaoPipelines>()`), so
/// [`CesiumEffectsPlugin`](super::CesiumEffectsPlugin) now calls
/// [`register_render_graph_main_world`] from `build` and this function from
/// `finish`.
///
/// Headless-safe: a no-op when there is no `RenderApp` sub-app.
pub fn finish_render_graph(app: &mut App) {
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    register_render_graph_render_world(render_app);
}

/// `Plugin::build`-time half of [`register_render_graph`]: shaders +
/// `ExtractComponentPlugin`s + main-world prepass systems for the pass-through,
/// FXAA and AO nodes. Touches the **main** world only, so it is safe before
/// `RenderDevice` exists.
pub fn register_render_graph_main_world(app: &mut App) {
    // Register the pass-through WGSL shader (headless-safe via shader_registry).
    crate::shader_registry::try_load_internal_shader(
        app,
        PASS_THROUGH_SHADER_HANDLE,
        include_str!("../../shaders/pass_through.wgsl"),
        "shaders/pass_through.wgsl",
    );

    // Register ExtractComponentPlugin — internally uses ExtractSchedule +
    // Extract<Query<(Entity, &CesiumPassThrough)>> to copy the component from
    // main world to render world each frame (same pattern as Bevy FxaaPlugin).
    app.add_plugins(ExtractComponentPlugin::<CesiumPassThrough>::default());

    // M5-E1: register the FXAA node (shader + extract + render-world node/systems).
    // M5-E2: register the SSAO node (shader + extract + node + depth/normal prepass
    //        enablement in the main world). Edges are NOT created here — the
    //        unified chain below owns them, so no diamond can form in `Core3d`.
    super::fxaa::register_fxaa_node_main_world(app);
    super::ao::register_ao_node_main_world(app);
}

/// `Plugin::finish`-time half of [`register_render_graph`]: render-world pipeline
/// resources, `Render`-schedule systems, the three `Core3d` nodes and the
/// unified linear chain (Daniel H2). Requires `RenderDevice`, hence `finish`.
pub fn register_render_graph_render_world(render_app: &mut bevy::app::SubApp) {
    // FIX-REG-FACADE (DEV-029): degrade to a no-op when `RenderDevice` is absent
    // (finish half reached from `build`, or a bare render world) — `PassThroughPipeline`
    // and the FXAA/AO sub-halves all dereference it. See
    // `crate::effects::render_world_missing_device`.
    if crate::effects::render_world_missing_device(render_app) {
        return;
    }

    super::fxaa::register_fxaa_node_render_world(render_app);
    super::ao::register_ao_node_render_world(render_app);

    render_app
        .init_resource::<PassThroughPipeline>()
        .init_resource::<SpecializedRenderPipelines<PassThroughPipeline>>()
        // Render schedule: pipeline specialization per view
        .add_systems(
            Render,
            prepare_pass_through_pipelines.in_set(RenderSet::Prepare),
        )
        // Render graph node registration
        .add_render_graph_node::<ViewNodeRunner<PassThroughNode>>(
            Core3d,
            CesiumPostProcessLabel::PassThrough,
        );

    // Unified linear chain (Daniel H2 — upstream CesiumJS parity). CesiumJS runs
    // AO first (on the HDR scene), then Bloom / AutoExposure / Tonemapping, then
    // FXAA LAST (on the final LDR image): PostProcessStageCollection.js L799-834.
    // The previous chain ran AO *after* Tonemapping yet claimed "matching CesiumJS"
    // — a false claim. Reordered so AO precedes Tonemapping and FXAA is the last
    // cesium node:
    //   EndMainPass → PassThrough → AmbientOcclusion → Tonemapping → Fxaa → EndMainPassPostProcessing
    // A single tuple chain avoids duplicate edges among the cesium nodes; the
    // default `EndMainPass → Tonemapping` and `Tonemapping → EndMainPassPostProcessing`
    // edges remain but are redundant (topological order still runs PassThrough → AO
    // before Tonemapping, and Fxaa before EndMainPassPostProcessing). AO reads the
    // depth/normal prepass (filled at frame start) and the HDR colour target — both
    // available at `EndMainPass`.
    render_app.add_render_graph_edges(
        Core3d,
        (
            Node3d::EndMainPass,
            CesiumPostProcessLabel::PassThrough,
            CesiumPostProcessLabel::AmbientOcclusion,
            Node3d::Tonemapping,
            CesiumPostProcessLabel::Fxaa,
            Node3d::EndMainPassPostProcessing,
        ),
    );
}

// ─── M6 Wave A integration (task #81) ───────────────────────────────────────

/// Register the M6 Wave A nodes + `Core3d` edges (task #81 integration).
///
/// Called by `application/cesium-app/src/main.rs` **after**
/// [`CesiumEffectsPlugin`](super::CesiumEffectsPlugin) has been added, so that
/// when the M5-E master gate is ON the `PassThrough` / `AmbientOcclusion` /
/// `Fxaa` nodes and Robin's #72 H2 chain already exist and can be spliced into.
///
/// # Gate contract
/// Each of M6.2 clipping / M6.3 panorama / M6.5 IBL is registered and wired
/// **only** when its own gate is ON. With all three OFF (the default) this
/// function returns before touching anything: no node, no edge, and — critically
/// — no removal, so `Core3d` stays byte-for-byte the pre-M6 graph and the v0
/// baselines remain pixel-neutral (PSNR=∞).
///
/// # Headless
/// Degrades gracefully when there is no `RenderApp` sub-app (headless
/// `MinimalPlugins`): the main-world plugins are still added, but no graph work
/// is attempted.
#[deprecated = "DEV-029 / FIX-REG-FACADE: call `register_m6_render_graph_main_world` from `Plugin::build` and `register_m6_render_graph_render_world` from `Plugin::finish`; this facade runs the finish half against a possibly device-less render world."]
pub fn register_m6_render_graph(app: &mut App) {
    register_m6_render_graph_main_world(app);
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        register_m6_render_graph_render_world(render_app);
    }
}

/// The three M6 gate values, read **once per phase** from the env.
///
/// `build` and `finish` are separate calls, so the gates are re-read in each;
/// the env is process-global and stable for the app's lifetime, so both phases
/// always agree. Kept private: [`wire_m6_edges`] is the testable surface.
#[derive(Clone, Copy, Debug)]
struct M6Gates {
    panorama: bool,
    clipping: bool,
    ibl: bool,
    oit: bool,
    clouds: bool,
    split: bool,
}

impl M6Gates {
    fn from_env() -> Self {
        Self {
            panorama: crate::effects::panorama::panorama_gate_enabled(),
            clipping: crate::effects::clipping_planes::clipping_gate_enabled(),
            ibl: crate::effects::ibl::ibl_gate_enabled(),
            oit: crate::effects::oit::oit_gate_enabled(),
            clouds: crate::effects::clouds::clouds_gate_enabled(),
            split: crate::effects::split::split_gate_enabled(),
        }
    }

    fn any(self) -> bool {
        self.panorama
            || self.clipping
            || self.ibl
            || self.oit
            || self.clouds
            || self.split
    }
}

/// `Plugin::build`-time half of [`register_m6_render_graph`]: WGSL shaders +
/// `ExtractComponentPlugin`s + the main-world prepass systems of whichever M6
/// gates are ON. Main world only, so it needs no `RenderDevice`.
pub fn register_m6_render_graph_main_world(app: &mut App) {
    let gates = M6Gates::from_env();
    if !gates.any() {
        return;
    }

    // Node + shader + ExtractComponentPlugin. Each `register_*_node_main_world`
    // deliberately builds **no edges**, so the single wiring point in
    // [`register_m6_render_graph_render_world`] is the only place a diamond could
    // form.
    if gates.panorama {
        crate::effects::panorama::register_panorama_node_main_world(app);
    }
    if gates.clipping {
        crate::effects::clipping_planes::register_clipping_planes_node_main_world(app);
    }
    if gates.ibl {
        crate::effects::ibl::register_ibl_node_main_world(app);
    }
    if gates.oit {
        crate::effects::oit::register_oit_node_main_world(app);
    }
    if gates.clouds {
        crate::effects::clouds::register_clouds_node_main_world(app);
    }
    if gates.split {
        crate::effects::split::register_split_node_main_world(app);
    }
}

/// `Plugin::finish`-time half of [`register_m6_render_graph`]: render-world
/// pipeline resources + `Core3d` nodes + the edges. Must run from `finish`
/// because each pipeline's `FromWorld` reads `RenderDevice`
/// (`docs/deviations.md#dev-029`).
pub fn register_m6_render_graph_render_world(render_app: &mut bevy::app::SubApp) {
    // FIX-REG-FACADE (DEV-029): degrade to a no-op when `RenderDevice` is absent
    // (finish half reached from `build`, or a bare render world) — every per-node
    // render half and `wire_m6_edges` below need the device. See
    // `crate::effects::render_world_missing_device`.
    if crate::effects::render_world_missing_device(render_app) {
        return;
    }

    let gates = M6Gates::from_env();
    if !gates.any() {
        return;
    }

    if gates.panorama {
        crate::effects::panorama::register_panorama_node_render_world(render_app);
    }
    if gates.clipping {
        crate::effects::clipping_planes::register_clipping_planes_node_render_world(render_app);
    }
    if gates.ibl {
        crate::effects::ibl::register_ibl_node_render_world(render_app);
    }
    if gates.oit {
        crate::effects::oit::register_oit_node_render_world(render_app);
    }
    if gates.clouds {
        crate::effects::clouds::register_clouds_node_render_world(render_app);
    }
    if gates.split {
        crate::effects::split::register_split_node_render_world(render_app);
    }

    wire_m6_edges(
        render_app,
        gates.panorama,
        gates.clipping,
        gates.ibl,
        gates.oit,
        gates.clouds,
        gates.split,
        postprocess_gate_enabled(),
    );
}

/// The plugin `application/cesium-app/src/main.rs` adds for M6 Wave A (task #81).
///
/// Splitting the registration across `build`/`finish` is **mandatory**, not
/// stylistic: `PanoramaPipeline` / `ClippingPlanesPipeline` / `IblPipeline` all
/// implement `FromWorld` by reading `RenderDevice`, which Bevy inserts into the
/// render world only in `RenderPlugin::finish`. Doing the render-world half in
/// `build` panics on a real GPU (reproduced locally with all three M6 gates ON,
/// `panorama.rs:471`). Mirrors `bevy_pbr`'s `ScreenSpaceAmbientOcclusionPlugin`.
///
/// Must be added **after** [`CesiumEffectsPlugin`](super::CesiumEffectsPlugin)
/// (i.e. later in the plugin registry) so that, with the M5-E master gate ON,
/// Robin's #72 H2 chain already exists when [`wire_m6_edges`] splices into it.
///
/// Gate contract: with all three M6 gates OFF (the default) both halves return
/// immediately — no shader, no plugin, no node, no edge, no removal — so `Core3d`
/// stays byte-for-byte the pre-M6 graph and the v0 baselines remain
/// pixel-neutral (PSNR=∞).
pub struct M6WaveARenderGraphPlugin;

impl bevy::app::Plugin for M6WaveARenderGraphPlugin {
    fn build(&self, app: &mut App) {
        register_m6_render_graph_main_world(app);
    }

    fn finish(&self, app: &mut App) {
        // Headless `MinimalPlugins` (and any app without `RenderPlugin`) has no
        // `RenderApp` sub-app — degrade gracefully instead of panicking.
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        register_m6_render_graph_render_world(render_app);
    }
}

/// Wire the M6 `Core3d` edges for an **explicit** gate combination.
///
/// Split out of [`register_m6_render_graph`] so every combination is testable
/// without mutating process-global env vars (which would race with the rest of
/// the suite). Covers the Wave A gates (panorama / clipping / ibl) plus the
/// Phase-3 FIX-INTEG gates (oit / clouds).
///
/// # Chain shapes produced
///
/// Panorama — serial insertion inside the main pass:
/// `MainOpaquePass → CesiumPanoramaLabel → MainTransmissivePass`.
///
/// Why *after* `MainOpaquePass`: Bevy's `texture_attachment.rs` issues
/// `LoadOp::Clear` on a view target's **first** use and `LoadOp::Load`
/// afterwards, so a panorama drawn before the opaque pass would simply be erased
/// by it. Drawing after means the panorama fills only the pixels the globe left
/// at far depth, and the later `MainTransparentPass` (starfield r=50 → sky dome
/// r=40) still paints over it. The sky dome's three ordering mechanisms
/// (depth_bias / `Premultiplied` / `cull_mode: Front`) are **not** touched.
///
/// OIT (Phase-3 FIX-INTEG) — a two-node serial pair spliced into the transparent
/// tail: `MainTransparentPass → CesiumOitLabel → CesiumOitCompositeLabel →
/// EndMainPass`. The accumulate node draws right after Bevy's own transparent pass
/// and the composite resolves before `EndMainPass`, so the post-process region sees
/// the blended result. Independent of the EndMainPass region below, so the OIT gate
/// neither multiplies the EndMainPass combinations nor is affected by them.
/// (The faithful re-routing of Bevy's transparent geometry *into* the MRT targets
/// needs a transparent-phase render-mesh pipeline modifier — deferred; see
/// `docs/deviations.md#dev-031`.)
///
/// Clipping + IBL + Clouds — serial insertion in the post-process (HDR) region:
/// `EndMainPass → [CesiumClippingLabel] → [CesiumIblLabel] → [CesiumCloudsLabel] →
/// <successor>`,
/// where `<successor>` is `CesiumPostProcessLabel::PassThrough` when the M5-E
/// master gate is ON — so Robin's #72 H2 chain
/// `PassThrough → AmbientOcclusion → Tonemapping → Fxaa →
/// EndMainPassPostProcessing` (AO on HDR before tonemapping, FXAA last on LDR;
/// upstream CesiumJS `PostProcessStageCollection.js` parity) is preserved
/// verbatim — and `Node3d::Tonemapping` when it is OFF (Bevy's own default
/// successor of `EndMainPass`).
///
/// Full shape with everything ON:
/// `MainTransparentPass → Oit → OitComposite → EndMainPass → Clipping → Ibl →
/// Clouds → PassThrough → AmbientOcclusion → Tonemapping → Fxaa →
/// EndMainPassPostProcessing`.
///
/// All three EndMainPass-region nodes are screen-space and their relative order is
/// a documented deviation (`docs/deviations.md`): IBL additively injects
/// environment light into the HDR scene, clipping overpaints the clipped region,
/// and clouds composite volumetric sky colour, so clipping runs first and wins.
/// The faithful upstream path (per-fragment `discard` in the material shader for
/// clipping, per-material IBL factors, per-cloud billboards) is deferred.
//
// The arity is intentional: each M6 gate is a separate `bool` so every subset
// combination can be exercised headlessly without mutating process-global env
// vars (which would race the rest of the suite). A `M6Gates`-by-value parameter
// was rejected because the wiring tests drive the flags independently.
#[allow(clippy::too_many_arguments)]
pub fn wire_m6_edges(
    render_app: &mut bevy::app::SubApp,
    panorama: bool,
    clipping: bool,
    ibl: bool,
    oit: bool,
    clouds: bool,
    split: bool,
    postprocess: bool,
) {
    if panorama {
        // `insert_node_in_core3d` removes `MainOpaquePass → MainTransmissivePass`
        // first, so the panorama is *the* path between them rather than one of
        // two (Lee's M6.3 diamond warning; same defect class as Daniel H2).
        insert_node_in_core3d(
            render_app,
            crate::effects::panorama::CesiumPanoramaLabel,
            Node3d::MainOpaquePass,
            Node3d::MainTransmissivePass,
        );
    }

    // OIT — its own serial slot in the transparent tail, independent of the
    // EndMainPass post-process region below: `MainTransparentPass → Oit →
    // OitComposite → EndMainPass`. The accumulate node runs right after Bevy's
    // own transparent pass (its MRT targets) and the composite resolves before
    // `EndMainPass` so downstream post-process nodes see the blended result.
    // Two chained `insert_node_in_core3d` calls keep it diamond-free; with the
    // gate OFF nothing here touches the graph (v0 neutral).
    if oit {
        insert_node_in_core3d(
            render_app,
            crate::effects::oit::CesiumOitLabel,
            Node3d::MainTransparentPass,
            Node3d::EndMainPass,
        );
        insert_node_in_core3d(
            render_app,
            crate::effects::oit::CesiumOitCompositeLabel,
            crate::effects::oit::CesiumOitLabel,
            Node3d::EndMainPass,
        );
    }

    // Clipping + IBL + Clouds + Split — serial insertion in the post-process (HDR)
    // region, ending at the post-process head (`PassThrough` when the M5-E
    // master gate is ON) or Bevy's own `Tonemapping` otherwise. With none of the
    // four ON this whole region is skipped, so `EndMainPass`'s edges stay exactly
    // as Bevy left them (v0 pixel-neutral).
    if !(clipping || ibl || clouds || split) {
        return;
    }

    // Build the ordered chain as type-erased `InternedRenderLabel`s so any subset
    // of the three optional nodes composes without the combinatorial `match` the
    // two-node case needed. `InternedRenderLabel` is `Copy` and implements
    // `RenderLabel`, so consecutive pairs feed `add_render_graph_edge` directly.
    let successor: InternedRenderLabel = if postprocess {
        CesiumPostProcessLabel::PassThrough.intern()
    } else {
        Node3d::Tonemapping.intern()
    };

    // The `EndMainPass → successor` edge(s) must be dropped before rebuilding a
    // single serial path through the new nodes, otherwise the graph stays a
    // diamond (topological sort could bypass the inserted nodes). Drop *both*
    // candidates defensively: when postprocess is OFF only `Tonemapping` exists;
    // when ON, `register_render_graph` leaves the redundant direct edge too.
    remove_core3d_edge(render_app, Node3d::EndMainPass, Node3d::Tonemapping);
    if postprocess {
        remove_core3d_edge(
            render_app,
            Node3d::EndMainPass,
            CesiumPostProcessLabel::PassThrough,
        );
    }

    let end_main_pass = Node3d::EndMainPass.intern();
    let mut chain: Vec<InternedRenderLabel> = Vec::with_capacity(5);
    chain.push(end_main_pass);
    if clipping {
        chain.push(crate::effects::clipping_planes::CesiumClippingLabel.intern());
    }
    if ibl {
        chain.push(crate::effects::ibl::CesiumIblLabel.intern());
    }
    if clouds {
        chain.push(crate::effects::clouds::CesiumCloudsLabel.intern());
    }
    if split {
        chain.push(crate::effects::split::CesiumSplitLabel.intern());
    }
    chain.push(successor);

    // Consecutive pairwise edges. Each node has exactly one successor ⇒ a strict
    // serial chain, never a diamond. `add_core3d_edge` (not the panicking
    // `add_render_graph_edge`) tolerates the intra-chain edges an earlier wiring
    // pass already created, so re-running is idempotent (`wiring_is_idempotent`).
    // `add_render_graph_edge` is the singular Bevy API (`IntoRenderNodeArray` only
    // covers fixed tuples, which cannot express a gate-dependent node list).
    for pair in chain.windows(2) {
        add_core3d_edge(render_app, pair[0], pair[1]);
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Gate OFF: `register_render_graph` is never called → no nodes in graph.
    /// This is the structural assertion proving pixel neutrality (PSNR=∞).
    #[test]
    fn gate_off_no_render_graph_registration() {
        // Pure logic test: no env var manipulation needed.
        assert!(!gate_from_env_value(None));
        assert!(!gate_from_env_value(Some("0".into())));
        assert!(!gate_from_env_value(Some("false".into())));

        // MinimalPlugins app (headless, no RenderApp).
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Do NOT call register_render_graph (that's what CesiumEffectsPlugin does when gate OFF).
        // Structural assertion: no RenderApp sub-app exists → graph cannot contain our nodes.
        assert!(app.get_sub_app_mut(RenderApp).is_none());
    }

    /// Gate ON but headless (no RenderApp): `register_render_graph` degrades gracefully.
    #[test]
    fn gate_on_headless_graceful_degradation() {
        // Verify parsing accepts "1" and "true" (case-insensitive).
        assert!(gate_from_env_value(Some("1".into())));
        assert!(gate_from_env_value(Some("true".into())));
        assert!(gate_from_env_value(Some("TRUE".into())));
        assert!(gate_from_env_value(Some("True".into())));

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Should NOT panic — degrades because get_sub_app_mut(RenderApp) returns None.
        // (Simulates gate ON in a headless environment.)
        #[allow(deprecated)]
        register_render_graph(&mut app);
    }

    /// FIX-REG-FACADE (DEV-029 收口): a render world that *exists* but carries no
    /// `RenderDevice` — the exact state during a plugin's `build`, and what a
    /// headless `MinimalPlugins` app can never reproduce (it has no `RenderApp` at
    /// all, so the misuse used to hide and the *headless graceful* tests were
    /// vacuously green). Every `register_*_render_world` half must now degrade to a
    /// no-op instead of panicking, and leave **no** half-initialised pipeline
    /// resource behind. This is the assertion §8.10 demands to break the
    /// always-green pattern: drop the guard and the calls panic here.
    #[test]
    fn render_world_without_device_degrades_to_noop() {
        // A bare render world: no `RenderDevice`, as seen from `Plugin::build`.
        let mut render_app = bevy::app::SubApp::new();
        assert!(
            render_app.world().get_resource::<RenderDevice>().is_none(),
            "fixture must have no RenderDevice for the guard to trigger",
        );

        // The post-process render half (PassThrough + FXAA + AO + edges).
        register_render_graph_render_world(&mut render_app);
        assert!(
            render_app
                .world()
                .get_resource::<PassThroughPipeline>()
                .is_none(),
            "guard must skip PassThroughPipeline init when RenderDevice is absent",
        );

        // A representative M6 per-node render half (split).
        crate::effects::split::register_split_node_render_world(&mut render_app);
        assert!(
            render_app
                .world()
                .get_resource::<crate::effects::split::SplitPipeline>()
                .is_none(),
            "guard must skip SplitPipeline init when RenderDevice is absent",
        );

        // The m6 umbrella render half is likewise device-gated (returns early).
        register_m6_render_graph_render_world(&mut render_app);
    }

    /// Gate parsing edge cases (Daniel H1: now the authoritative 4-token truthy
    /// set `{1, true, yes, on}`, trimmed + lowercased — `pipeline::fetch`).
    #[test]
    fn gate_env_parsing_edge_cases() {
        assert!(!gate_from_env_value(Some("".into())));
        assert!(!gate_from_env_value(Some("2".into())));
        assert!(!gate_from_env_value(Some("off".into())));
        assert!(!gate_from_env_value(Some("no".into())));
        // `yes` / `on` were rejected by the old 2-token local copy; the authoritative
        // parser accepts them (case-insensitively).
        assert!(gate_from_env_value(Some("yes".into())));
        assert!(gate_from_env_value(Some("on".into())));
        assert!(gate_from_env_value(Some("YES".into())));
        assert!(gate_from_env_value(Some("On".into())));
        // Trimmed + lowercased — the old copy did neither.
        assert!(gate_from_env_value(Some(" 1 ".into())));
        assert!(gate_from_env_value(Some("\ttrue\n".into())));
        assert!(gate_from_env_value(Some(" True ".into())));
    }

    /// CesiumPassThrough component default: disabled (conservative).
    #[test]
    fn component_default_disabled() {
        let c = CesiumPassThrough::default();
        assert!(!c.enabled);
    }

    /// Label enum: ensures all planned variants exist at compile time.
    #[test]
    fn labels_compile_time_existence() {
        let _pt = CesiumPostProcessLabel::PassThrough;
        let _fxaa = CesiumPostProcessLabel::Fxaa;
        let _ao = CesiumPostProcessLabel::AmbientOcclusion;
    }

    /// M5-E1: the two post-process gates read DISTINCT env vars, so FXAA/AO
    /// (`CESIUM_ENABLE_POSTPROCESS`) and tonemapping/bloom/fog
    /// (`CESIUM_ENABLE_POSTPROCESS_BUILTIN`) can be toggled independently
    /// (leader ruling Q5). Asserted via the const names to avoid env-var races.
    #[test]
    fn postprocess_gates_are_independent() {
        assert_ne!(ENV_ENABLE_POSTPROCESS, ENV_ENABLE_POSTPROCESS_BUILTIN);
        assert_eq!(ENV_ENABLE_POSTPROCESS, "CESIUM_ENABLE_POSTPROCESS");
        assert_eq!(ENV_ENABLE_POSTPROCESS_BUILTIN, "CESIUM_ENABLE_POSTPROCESS_BUILTIN");
        // Same truthy predicate for both (pure, no env mutation).
        assert!(gate_from_env_value(Some("1".into())));
        assert!(!gate_from_env_value(None));
    }

    // ─── M6 Wave A wiring tests (task #81) ────────────────────────────────────

    // Imported here rather than at the top of the file: `EmptyNode` is only ever
    // needed by the fixture, and a crate-level import would be `unused` in a
    // non-test build (clippy `-D warnings`).
    use bevy::render::render_graph::EmptyNode;

    /// Build an `App` whose world carries a `Core3d` sub-graph reproducing the
    /// Bevy 0.15.3 default chain (`bevy_core_pipeline/src/core_3d/mod.rs`
    /// L193-209):
    /// `StartMainPass → MainOpaquePass → MainTransmissivePass →
    /// MainTransparentPass → EndMainPass → Tonemapping →
    /// EndMainPassPostProcessing → Upscaling`.
    ///
    /// `with_postprocess` additionally adds the cesium M5-E nodes and Robin's
    /// #72 H2 chain exactly as `register_render_graph` builds it.
    fn core3d_fixture(with_postprocess: bool) -> App {
        let mut app = App::new();
        let mut sub = RenderGraph::default();

        for label in [
            Node3d::StartMainPass,
            Node3d::MainOpaquePass,
            Node3d::MainTransmissivePass,
            Node3d::MainTransparentPass,
            Node3d::EndMainPass,
            Node3d::Tonemapping,
            Node3d::EndMainPassPostProcessing,
            Node3d::Upscaling,
        ] {
            sub.add_node(label, EmptyNode);
        }
        sub.add_node_edges((
            Node3d::StartMainPass,
            Node3d::MainOpaquePass,
            Node3d::MainTransmissivePass,
            Node3d::MainTransparentPass,
            Node3d::EndMainPass,
            Node3d::Tonemapping,
            Node3d::EndMainPassPostProcessing,
            Node3d::Upscaling,
        ));

        // The M6 nodes themselves always exist in the fixture: in production
        // `register_*_node` adds them before `wire_m6_edges` runs, and
        // `add_render_graph_edges` panics on a label with no node.
        sub.add_node(crate::effects::panorama::CesiumPanoramaLabel, EmptyNode);
        sub.add_node(crate::effects::clipping_planes::CesiumClippingLabel, EmptyNode);
        sub.add_node(crate::effects::ibl::CesiumIblLabel, EmptyNode);
        // Phase-3 FIX-INTEG: OIT (accumulate + composite) and Clouds nodes always
        // exist in the fixture too (production `register_*_node` adds them before
        // `wire_m6_edges`), so the gate combinations can be exercised headlessly.
        sub.add_node(crate::effects::oit::CesiumOitLabel, EmptyNode);
        sub.add_node(crate::effects::oit::CesiumOitCompositeLabel, EmptyNode);
        sub.add_node(crate::effects::clouds::CesiumCloudsLabel, EmptyNode);
        sub.add_node(crate::effects::split::CesiumSplitLabel, EmptyNode);

        if with_postprocess {
            sub.add_node(CesiumPostProcessLabel::PassThrough, EmptyNode);
            sub.add_node(CesiumPostProcessLabel::AmbientOcclusion, EmptyNode);
            sub.add_node(CesiumPostProcessLabel::Fxaa, EmptyNode);
            sub.add_node_edges((
                Node3d::EndMainPass,
                CesiumPostProcessLabel::PassThrough,
                CesiumPostProcessLabel::AmbientOcclusion,
                Node3d::Tonemapping,
                CesiumPostProcessLabel::Fxaa,
                Node3d::EndMainPassPostProcessing,
            ));
        }

        let mut root = RenderGraph::default();
        root.add_sub_graph(Core3d, sub);
        app.insert_resource(root);
        app
    }

    /// Sorted debug labels of a node's outgoing neighbours.
    fn out_labels(app: &App, label: impl RenderLabel) -> Vec<String> {
        let graph = app.world().resource::<RenderGraph>();
        let sub = graph.get_sub_graph(Core3d).expect("Core3d sub-graph");
        let mut v: Vec<String> = sub
            .iter_node_outputs(label)
            .expect("node exists")
            .map(|(_edge, node)| format!("{:?}", node.label))
            .collect();
        v.sort();
        v
    }

    /// Sorted debug labels of a node's incoming neighbours.
    fn in_labels(app: &App, label: impl RenderLabel) -> Vec<String> {
        let graph = app.world().resource::<RenderGraph>();
        let sub = graph.get_sub_graph(Core3d).expect("Core3d sub-graph");
        let mut v: Vec<String> = sub
            .iter_node_inputs(label)
            .expect("node exists")
            .map(|(_edge, node)| format!("{:?}", node.label))
            .collect();
        v.sort();
        v
    }

    fn name(label: impl RenderLabel) -> String {
        format!("{:?}", label)
    }

    /// **All M6 gates OFF is a strict no-op**: not one edge added, not one edge
    /// removed. This is the structural proof of v0 pixel neutrality (PSNR=∞).
    #[test]
    fn all_gates_off_leaves_the_default_chain_untouched() {
        for pp in [false, true] {
            let app = core3d_fixture(pp);
            let before: Vec<(String, Vec<String>)> = [
                Node3d::MainOpaquePass,
                Node3d::MainTransmissivePass,
                Node3d::MainTransparentPass,
                Node3d::EndMainPass,
                Node3d::Tonemapping,
            ]
            // `Node3d` is `Clone` but **not** `Copy` (the `Dyn` variant owns an
            // `InternedRenderLabel`), so `array::map` by value + one `clone` is the
            // move-clean way to feed the same label to two by-value parameters.
            .map(|l| (name(l.clone()), out_labels(&app, l)))
            .to_vec();

            let mut app = app;
            wire_m6_edges(app.main_mut(), false, false, false, false, false, false, pp);

            let after: Vec<(String, Vec<String>)> = [
                Node3d::MainOpaquePass,
                Node3d::MainTransmissivePass,
                Node3d::MainTransparentPass,
                Node3d::EndMainPass,
                Node3d::Tonemapping,
            ]
            .map(|l| (name(l.clone()), out_labels(&app, l)))
            .to_vec();

            assert_eq!(before, after, "gate-OFF must not touch Core3d (pp={pp})");
            // And the M6 nodes stayed isolated: no edges at all.
            assert!(out_labels(&app, crate::effects::panorama::CesiumPanoramaLabel).is_empty());
            assert!(out_labels(&app, crate::effects::clipping_planes::CesiumClippingLabel).is_empty());
            assert!(out_labels(&app, crate::effects::ibl::CesiumIblLabel).is_empty());
            assert!(out_labels(&app, crate::effects::oit::CesiumOitLabel).is_empty());
            assert!(out_labels(&app, crate::effects::oit::CesiumOitCompositeLabel).is_empty());
            assert!(out_labels(&app, crate::effects::clouds::CesiumCloudsLabel).is_empty());
            assert!(out_labels(&app, crate::effects::split::CesiumSplitLabel).is_empty());
            // The transparent tail Bevy owns stays intact: MainTransparentPass →
            // EndMainPass directly (no OIT nodes spliced in when the gate is OFF).
            assert_eq!(
                out_labels(&app, Node3d::MainTransparentPass),
                vec![name(Node3d::EndMainPass)]
            );
        }
    }

    /// Panorama insertion is **serial, not a diamond** (Lee's M6.3 warning):
    /// the pre-existing `MainOpaquePass → MainTransmissivePass` edge must be
    /// gone, leaving exactly one path through `CesiumPanoramaLabel`.
    #[test]
    fn panorama_insertion_is_serial_not_a_diamond() {
        let mut app = core3d_fixture(false);
        wire_m6_edges(app.main_mut(), true, false, false, false, false, false, false);

        assert_eq!(
            out_labels(&app, Node3d::MainOpaquePass),
            vec![name(crate::effects::panorama::CesiumPanoramaLabel)],
            "MainOpaquePass must have exactly ONE successor (the diamond edge removed)"
        );
        assert_eq!(
            in_labels(&app, Node3d::MainTransmissivePass),
            vec![name(crate::effects::panorama::CesiumPanoramaLabel)],
            "MainTransmissivePass must have exactly ONE predecessor"
        );
        assert_eq!(
            out_labels(&app, crate::effects::panorama::CesiumPanoramaLabel),
            vec![name(Node3d::MainTransmissivePass)]
        );
        // Downstream ordering is untouched: the transparent pass (starfield r=50
        // → sky dome r=40) still follows the transmissive pass.
        assert_eq!(
            out_labels(&app, Node3d::MainTransmissivePass),
            vec![name(Node3d::MainTransparentPass)]
        );
    }

    /// Clipping + IBL splice into the HDR region **without disturbing** Robin's
    /// #72 H2 chain (`PassThrough → AmbientOcclusion → Tonemapping → Fxaa →
    /// EndMainPassPostProcessing`).
    #[test]
    fn clipping_and_ibl_prepend_the_h2_chain() {
        let mut app = core3d_fixture(true);
        wire_m6_edges(app.main_mut(), false, true, true, false, false, false, true);

        assert_eq!(
            out_labels(&app, Node3d::EndMainPass),
            vec![name(crate::effects::clipping_planes::CesiumClippingLabel)],
            "the redundant direct EndMainPass→Tonemapping edge must be gone too"
        );
        assert_eq!(
            out_labels(&app, crate::effects::clipping_planes::CesiumClippingLabel),
            vec![name(crate::effects::ibl::CesiumIblLabel)]
        );
        assert_eq!(
            out_labels(&app, crate::effects::ibl::CesiumIblLabel),
            vec![name(CesiumPostProcessLabel::PassThrough)]
        );
        // H2 reorder preserved verbatim: AO on the HDR scene before tonemapping,
        // FXAA last on the LDR image.
        assert_eq!(
            out_labels(&app, CesiumPostProcessLabel::PassThrough),
            vec![name(CesiumPostProcessLabel::AmbientOcclusion)]
        );
        assert_eq!(
            out_labels(&app, CesiumPostProcessLabel::AmbientOcclusion),
            vec![name(Node3d::Tonemapping)]
        );
        let tone = out_labels(&app, Node3d::Tonemapping);
        assert!(tone.contains(&name(CesiumPostProcessLabel::Fxaa)), "got {tone:?}");
        assert_eq!(
            out_labels(&app, CesiumPostProcessLabel::Fxaa),
            vec![name(Node3d::EndMainPassPostProcessing)]
        );
        // Tonemapping must NOT have gained a direct EndMainPass predecessor.
        assert!(!in_labels(&app, Node3d::Tonemapping)
            .contains(&name(Node3d::EndMainPass)));
    }

    /// With the M5-E master gate OFF the cesium post-process nodes do not exist,
    /// so clipping/IBL must splice onto Bevy's own `Tonemapping` successor and
    /// never name `PassThrough` (which would panic with `InvalidNode`).
    #[test]
    fn clipping_only_without_postprocess_targets_tonemapping() {
        let mut app = core3d_fixture(false);
        wire_m6_edges(app.main_mut(), false, true, false, false, false, false, false);

        assert_eq!(
            out_labels(&app, Node3d::EndMainPass),
            vec![name(crate::effects::clipping_planes::CesiumClippingLabel)]
        );
        assert_eq!(
            out_labels(&app, crate::effects::clipping_planes::CesiumClippingLabel),
            vec![name(Node3d::Tonemapping)]
        );
        assert_eq!(
            out_labels(&app, Node3d::Tonemapping),
            vec![name(Node3d::EndMainPassPostProcessing)]
        );
        // IBL stayed isolated.
        assert!(out_labels(&app, crate::effects::ibl::CesiumIblLabel).is_empty());
    }

    /// IBL alone, master gate ON.
    #[test]
    fn ibl_only_with_postprocess_targets_pass_through() {
        let mut app = core3d_fixture(true);
        wire_m6_edges(app.main_mut(), false, false, true, false, false, false, true);

        assert_eq!(
            out_labels(&app, Node3d::EndMainPass),
            vec![name(crate::effects::ibl::CesiumIblLabel)]
        );
        assert_eq!(
            out_labels(&app, crate::effects::ibl::CesiumIblLabel),
            vec![name(CesiumPostProcessLabel::PassThrough)]
        );
        assert!(out_labels(&app, crate::effects::clipping_planes::CesiumClippingLabel).is_empty());
    }

    /// All three M6 gates ON simultaneously: panorama in the main pass *and*
    /// clipping+IBL in the HDR region, both serial.
    #[test]
    fn all_three_gates_on_compose_without_diamonds() {
        let mut app = core3d_fixture(true);
        wire_m6_edges(app.main_mut(), true, true, true, false, false, false, true);

        assert_eq!(out_labels(&app, Node3d::MainOpaquePass).len(), 1);
        assert_eq!(in_labels(&app, Node3d::MainTransmissivePass).len(), 1);
        assert_eq!(out_labels(&app, Node3d::EndMainPass).len(), 1);
        assert_eq!(
            out_labels(&app, Node3d::EndMainPass),
            vec![name(crate::effects::clipping_planes::CesiumClippingLabel)]
        );
        // Every M6 node has at most one successor: no diamond anywhere.
        assert_eq!(out_labels(&app, crate::effects::panorama::CesiumPanoramaLabel).len(), 1);
        assert_eq!(out_labels(&app, crate::effects::clipping_planes::CesiumClippingLabel).len(), 1);
        assert_eq!(out_labels(&app, crate::effects::ibl::CesiumIblLabel).len(), 1);
    }

    /// FIX-GRAPH-WIRING: `panorama=T, clipping=T, ibl=F` — a cell of the 8-combination
    /// matrix that was previously untested. The main-pass panorama and the HDR-region
    /// clipping each compose **serially** (exactly one successor apiece); IBL stays
    /// isolated because its gate is off.
    #[test]
    fn panorama_and_clipping_without_ibl_compose_serially() {
        let mut app = core3d_fixture(true);
        wire_m6_edges(app.main_mut(), true, true, false, false, false, false, true);

        // Panorama is the sole path MainOpaquePass → MainTransmissivePass.
        assert_eq!(
            out_labels(&app, Node3d::MainOpaquePass),
            vec![name(crate::effects::panorama::CesiumPanoramaLabel)]
        );
        assert_eq!(
            in_labels(&app, Node3d::MainTransmissivePass),
            vec![name(crate::effects::panorama::CesiumPanoramaLabel)]
        );
        // Clipping splices onto the post-process head (master gate ON).
        assert_eq!(
            out_labels(&app, Node3d::EndMainPass),
            vec![name(crate::effects::clipping_planes::CesiumClippingLabel)]
        );
        assert_eq!(
            out_labels(&app, crate::effects::clipping_planes::CesiumClippingLabel),
            vec![name(CesiumPostProcessLabel::PassThrough)]
        );
        // IBL gate off ⇒ node isolated (no edges).
        assert!(
            out_labels(&app, crate::effects::ibl::CesiumIblLabel).is_empty(),
            "ibl gate off ⇒ node stays isolated"
        );
        // No diamond: one successor each.
        assert_eq!(
            out_labels(&app, crate::effects::panorama::CesiumPanoramaLabel).len(),
            1
        );
        assert_eq!(
            out_labels(&app, crate::effects::clipping_planes::CesiumClippingLabel).len(),
            1
        );
    }

    /// FIX-GRAPH-WIRING: `panorama=T, clipping=F, ibl=T` — the last uncovered cell
    /// of the 8-combination matrix. Panorama + IBL each compose serially; clipping
    /// stays isolated.
    #[test]
    fn panorama_and_ibl_without_clipping_compose_serially() {
        let mut app = core3d_fixture(true);
        wire_m6_edges(app.main_mut(), true, false, true, false, false, false, true);

        assert_eq!(
            out_labels(&app, Node3d::MainOpaquePass),
            vec![name(crate::effects::panorama::CesiumPanoramaLabel)]
        );
        assert_eq!(
            in_labels(&app, Node3d::MainTransmissivePass),
            vec![name(crate::effects::panorama::CesiumPanoramaLabel)]
        );
        // IBL splices onto the post-process head; clipping untouched.
        assert_eq!(
            out_labels(&app, Node3d::EndMainPass),
            vec![name(crate::effects::ibl::CesiumIblLabel)]
        );
        assert_eq!(
            out_labels(&app, crate::effects::ibl::CesiumIblLabel),
            vec![name(CesiumPostProcessLabel::PassThrough)]
        );
        assert!(
            out_labels(&app, crate::effects::clipping_planes::CesiumClippingLabel).is_empty(),
            "clipping gate off ⇒ node stays isolated"
        );
        assert_eq!(
            out_labels(&app, crate::effects::panorama::CesiumPanoramaLabel).len(),
            1
        );
        assert_eq!(
            out_labels(&app, crate::effects::ibl::CesiumIblLabel).len(),
            1
        );
    }

    /// FIX-INTEG (Phase 3): OIT alone composes a serial chain in the transparent
    /// tail — `MainTransparentPass → Oit → OitComposite → EndMainPass` — with the
    /// pre-existing `MainTransparentPass → EndMainPass` edge removed (no diamond),
    /// and leaves the EndMainPass post-process region untouched (clipping/ibl/
    /// clouds gates off).
    #[test]
    fn oit_composes_serially_in_transparent_tail() {
        let mut app = core3d_fixture(true);
        wire_m6_edges(app.main_mut(), false, false, false, true, false, false, true);

        use crate::effects::oit::{CesiumOitCompositeLabel, CesiumOitLabel};
        assert_eq!(
            out_labels(&app, Node3d::MainTransparentPass),
            vec![name(CesiumOitLabel)],
            "the direct MainTransparentPass→EndMainPass edge must be gone"
        );
        assert_eq!(out_labels(&app, CesiumOitLabel), vec![name(CesiumOitCompositeLabel)]);
        assert_eq!(
            out_labels(&app, CesiumOitCompositeLabel),
            vec![name(Node3d::EndMainPass)]
        );
        assert_eq!(in_labels(&app, Node3d::EndMainPass), vec![name(CesiumOitCompositeLabel)]);
        // Post-process region untouched (clipping/ibl/clouds all OFF): the M6
        // nodes stay isolated and EndMainPass keeps Bevy's own default successors
        // (v0-neutral — the region is only spliced when one of them is ON).
        assert!(out_labels(&app, crate::effects::clipping_planes::CesiumClippingLabel).is_empty());
        assert!(out_labels(&app, crate::effects::ibl::CesiumIblLabel).is_empty());
        assert!(out_labels(&app, crate::effects::clouds::CesiumCloudsLabel).is_empty());
    }

    /// FIX-INTEG (Phase 3): Clouds splice onto the EndMainPass HDR chain, after
    /// clipping + IBL when those gates are also ON, targeting the post-process
    /// head. `EndMainPass → Clipping → Ibl → Clouds → PassThrough`.
    #[test]
    fn clouds_append_the_h2_chain() {
        let mut app = core3d_fixture(true);
        wire_m6_edges(app.main_mut(), false, true, true, false, true, false, true);

        use crate::effects::clipping_planes::CesiumClippingLabel;
        use crate::effects::clouds::CesiumCloudsLabel;
        use crate::effects::ibl::CesiumIblLabel;
        assert_eq!(out_labels(&app, Node3d::EndMainPass), vec![name(CesiumClippingLabel)]);
        assert_eq!(out_labels(&app, CesiumClippingLabel), vec![name(CesiumIblLabel)]);
        assert_eq!(out_labels(&app, CesiumIblLabel), vec![name(CesiumCloudsLabel)]);
        assert_eq!(
            out_labels(&app, CesiumCloudsLabel),
            vec![name(CesiumPostProcessLabel::PassThrough)]
        );
        // Clouds gate ON alone (no clipping/ibl) still lands on PassThrough.
        let mut app2 = core3d_fixture(true);
        wire_m6_edges(app2.main_mut(), false, false, false, false, true, false, true);
        assert_eq!(
            out_labels(&app2, Node3d::EndMainPass),
            vec![name(CesiumCloudsLabel)]
        );
        assert_eq!(
            out_labels(&app2, CesiumCloudsLabel),
            vec![name(CesiumPostProcessLabel::PassThrough)]
        );
    }

    /// FIX-INTEG / FIX-SPLIT (Phase 3): all six M6 gates ON compose without a
    /// single diamond — panorama in the main pass, the OIT pair in the transparent
    /// tail, and the clipping→ibl→clouds→split chain in the HDR region, each node
    /// exactly one successor.
    #[test]
    fn all_six_gates_on_compose_without_diamonds() {
        let mut app = core3d_fixture(true);
        wire_m6_edges(app.main_mut(), true, true, true, true, true, true, true);

        use crate::effects::clipping_planes::CesiumClippingLabel;
        use crate::effects::clouds::CesiumCloudsLabel;
        use crate::effects::ibl::CesiumIblLabel;
        use crate::effects::oit::{CesiumOitCompositeLabel, CesiumOitLabel};
        use crate::effects::panorama::CesiumPanoramaLabel;
        use crate::effects::split::CesiumSplitLabel;
        // Main-pass panorama: single path.
        assert_eq!(out_labels(&app, CesiumPanoramaLabel).len(), 1);
        // Transparent tail: exactly one successor each.
        assert_eq!(out_labels(&app, Node3d::MainTransparentPass), vec![name(CesiumOitLabel)]);
        assert_eq!(out_labels(&app, CesiumOitLabel).len(), 1);
        assert_eq!(out_labels(&app, CesiumOitCompositeLabel).len(), 1);
        // HDR region: EndMainPass → Clipping → Ibl → Clouds → Split → PassThrough.
        assert_eq!(out_labels(&app, Node3d::EndMainPass), vec![name(CesiumClippingLabel)]);
        assert_eq!(out_labels(&app, CesiumClippingLabel), vec![name(CesiumIblLabel)]);
        assert_eq!(out_labels(&app, CesiumIblLabel), vec![name(CesiumCloudsLabel)]);
        assert_eq!(out_labels(&app, CesiumCloudsLabel), vec![name(CesiumSplitLabel)]);
        assert_eq!(
            out_labels(&app, CesiumSplitLabel),
            vec![name(CesiumPostProcessLabel::PassThrough)]
        );
    }

    /// Wiring is idempotent: calling it twice must not add a second edge nor
    /// panic (plugin build order is not fully controllable from `main.rs`).
    #[test]
    fn wiring_is_idempotent() {
        let mut app = core3d_fixture(true);
        wire_m6_edges(app.main_mut(), true, true, true, false, false, false, true);
        let first: Vec<Vec<String>> = [
            Node3d::MainOpaquePass,
            Node3d::EndMainPass,
            Node3d::Tonemapping,
        ]
        .map(|l| out_labels(&app, l))
        .to_vec();

        wire_m6_edges(app.main_mut(), true, true, true, false, false, false, true);
        let second: Vec<Vec<String>> = [
            Node3d::MainOpaquePass,
            Node3d::EndMainPass,
            Node3d::Tonemapping,
        ]
        .map(|l| out_labels(&app, l))
        .to_vec();

        assert_eq!(first, second, "re-wiring must be a no-op, not an edge duplicate");
    }

    /// `register_m6_render_graph` must not panic on a headless `MinimalPlugins`
    /// app (no `RenderApp`, no `RenderGraph`) regardless of the ambient gates.
    #[test]
    fn register_m6_render_graph_is_headless_safe() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        #[allow(deprecated)]
        register_m6_render_graph(&mut app);
        assert!(app.get_sub_app_mut(RenderApp).is_none());
    }
}
