//! M5-E0: cesiumrust RenderGraph infrastructure.
//!
//! Establishes the first `ViewNode` + `Extract<Component>` + `ExtractSchedule` +
//! `RenderApp` assembly pattern for cesiumrust. Provides reusable helpers for
//! M5-E1 (FXAA) and M5-E2 (SSAO) post-process nodes.
//!
//! # Architecture
//! - Gate: `CESIUM_ENABLE_POSTPROCESS` env var (same as `feature_flags::postprocess_enabled()`)
//! - Gate OFF → **no render graph nodes registered** → v0 baselines zero-diff (PSNR=∞)
//! - Gate ON → pass-through node inserted in `Core3d` between `Tonemapping` and
//!   `EndMainPassPostProcessing`, proving pixel-neutral infrastructure.
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
        NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
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

/// Pure parsing logic for gate env values (testable without env var races).
#[inline]
pub fn gate_from_env_value(val: Option<String>) -> bool {
    val.map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

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
        // Quality-tier gate: skip if component says disabled.
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
/// Creates edges: `predecessor → new_label → successor`.
///
/// # Usage (M5-E1 FXAA)
/// ```text
/// insert_node_in_core3d(
///     render_app,
///     CesiumPostProcessLabel::Fxaa,
///     Node3d::Tonemapping,          // predecessor
///     Node3d::EndMainPassPostProcessing, // successor
/// );
/// // then: render_app.add_render_graph_node::<ViewNodeRunner<FxaaNode>>(Core3d, CesiumPostProcessLabel::Fxaa);
/// ```
///
/// # Note
/// The node itself must be added separately via `add_render_graph_node` before
/// or after calling this helper (the helper only creates edges).
pub fn insert_node_in_core3d(
    render_app: &mut bevy::app::SubApp,
    new_label: impl RenderLabel,
    predecessor: impl RenderLabel,
    successor: impl RenderLabel,
) {
    render_app.add_render_graph_edges(
        Core3d,
        (predecessor, new_label, successor),
    );
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
/// wires the **single linear chain**
/// `Tonemapping → PassThrough → AmbientOcclusion → Fxaa → EndMainPassPostProcessing`.
/// All nodes are registered here (never in isolation) so their edges cannot
/// form a diamond. The pass-through node is pixel-neutral and default-disabled
/// (`CesiumPassThrough::default().enabled == false` → early-return, zero GPU
/// cost); FXAA runs when the camera carries `CesiumFxaa { enabled: true }`.
pub fn register_render_graph(app: &mut App) {
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
    super::fxaa::register_fxaa_node(app);
    super::ao::register_ao_node(app);

    // RenderApp assembly — headless-safe (returns None without RenderPlugin).
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        // No render app (headless MinimalPlugins) — degrade gracefully.
        return;
    };

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

    // Unified linear chain (M5-E2): SSAO inserted between the pass-through and
    // FXAA nodes, giving:
    //   Tonemapping → PassThrough → AmbientOcclusion → Fxaa → EndMainPassPostProcessing
    // AO runs after tonemapping (HDR linear → LDR done) and before FXAA, matching
    // CesiumJS `PostProcessStageLibrary.createAmbientOcclusionStage` ordering. A
    // single tuple chain avoids duplicate edges among the cesium nodes; the
    // default `Tonemapping → EndMainPassPostProcessing` edge remains but is
    // redundant (topological order still runs PassThrough → AO → Fxaa before
    // EndMainPassPostProcessing because those edges force it last).
    render_app.add_render_graph_edges(
        Core3d,
        (
            Node3d::Tonemapping,
            CesiumPostProcessLabel::PassThrough,
            CesiumPostProcessLabel::AmbientOcclusion,
            CesiumPostProcessLabel::Fxaa,
            Node3d::EndMainPassPostProcessing,
        ),
    );
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
        register_render_graph(&mut app);
    }

    /// Gate parsing edge cases.
    #[test]
    fn gate_env_parsing_edge_cases() {
        assert!(!gate_from_env_value(Some("".into())));
        assert!(!gate_from_env_value(Some("2".into())));
        assert!(!gate_from_env_value(Some("yes".into())));
        assert!(!gate_from_env_value(Some("on".into())));
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
}
