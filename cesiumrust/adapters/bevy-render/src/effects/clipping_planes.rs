//! M6.2: ClippingPlanes `ViewNode` + uniform injection infrastructure.
//!
//! Ports the upstream CesiumJS clipping-plane capability
//! (`Scene/ClippingPlaneCollection.js` + `Shaders/Model/ModelClippingPlanesStageFS.glsl`)
//! onto the M5-E render-graph infrastructure (`graph.rs`). Mirrors the
//! [`super::fxaa`] / [`super::ao`] pattern: this module registers the node /
//! resources / systems **but never creates graph edges** — the single linear
//! `Core3d` chain in `graph.rs::register_render_graph` owns them, so no diamond
//! can form. Wiring the clipping node into that chain is integration task #81.
//!
//! # Two clipping paths (see `shaders/clipping.wgsl`)
//! 1. **Forward pass (faithful)** — `apply_clipping_planes(world_pos)` is
//!    `#import`-ed into the globe / tileset fragment shaders so clipped fragments
//!    `discard` during geometry rasterisation, exactly where upstream
//!    `modelClippingPlanesStage(inout vec4 color)` runs. That wiring touches the
//!    material shaders (out of M6.2 file scope) and is deferred to #81.
//! 2. **Screen-space node (this file)** — [`ClippingPlanesNode`] reconstructs the
//!    world position from the depth prepass (the WGSL analogue of upstream
//!    `czm_windowToEyeCoordinates(gl_FragCoord)`) and applies the same decision,
//!    overwriting clipped fragments with a background colour + edge highlight.
//!    DEVIATION: a screen-space node cannot `discard` already-rasterised geometry,
//!    and the planes are pre-transformed on the CPU instead of per-fragment
//!    `czm_transformPlane` — see `docs/deviations.md#dev-023`.
//!
//! # Gate (single source of truth)
//! The gate name is owned by the app-layer registry
//! `application/cesium-app/src/feature_flags.rs` (`ENV_ENABLE_CLIPPING` /
//! `clipping_enabled()`); [`ENV_ENABLE_CLIPPING`] below is a byte-identical mirror
//! forced by the crate dependency direction (`cesium-app` → `cesium-bevy-render`).
//! Default OFF ⇒ `effects::graph::M6WaveARenderGraphPlugin` (task #81) returns
//! early from both halves ⇒ [`register_clipping_planes_node`] is never called and
//! no `Core3d` edges exist ⇒ the node never runs ⇒ dynamic_globe v0 baselines stay
//! pixel-neutral (PSNR = ∞).
//!
//! # Red lines honoured
//! - domain stays metric **f64**; the metric → render-unit conversion
//!   (`distance / METERS_PER_RENDER_UNIT`, `METERS_PER_RENDER_UNIT = 6378137`)
//!   happens ONLY at the [`ClippingPlanesUniform::from_domain`] GPU boundary.
//! - `clipping.wgsl` keeps `dot(n,p)` and `+ w` as two roundings (no FMA fusion).
//! - glam fast-math disabled repo-wide (nothing here relies on non-IEEE floats).
//!
//! # Blueprint
//! - `packages/engine/Source/Scene/ClippingPlane.js` (Hessian normal form)
//! - `packages/engine/Source/Scene/ClippingPlaneCollection.js` L146-152 (union /
//!   intersection), L251-257 (`clippingPlanesState`), L404-602 (GPU packing)
//! - `packages/engine/Source/Shaders/Model/ModelClippingPlanesStageFS.glsl` L39-88
//! - `bevy_pbr-0.15.3/src/ssao/mod.rs` (ViewNode + depth prepass reconstruction)

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
        BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
        CachedRenderPipelineId, ColorTargetState, ColorWrites, FilterMode, FragmentState,
        MultisampleState, Operations, PipelineCache, PrimitiveState, RenderPassColorAttachment,
        RenderPassDescriptor, RenderPipelineDescriptor, Sampler as GpuSampler, SamplerBindingType,
        SamplerDescriptor, ShaderStages, TextureFormat, TextureSampleType,
        UniformBuffer,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
    Render, RenderApp, RenderSet,
};
use cesium_effects::clipping::ClippingPlaneCollection;

use crate::resources::METERS_PER_RENDER_UNIT;
use super::graph::gate_from_env_value;

// ─── Shader handle ───────────────────────────────────────────────────────────

/// Unique handle for the embedded `clipping.wgsl` shader (distinct from the
/// pass-through / FXAA / AO handles — see the collision test below).
pub const CLIPPING_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0xCE51_C1C1_0006_0002);

/// Maximum number of clipping planes uploaded to the GPU uniform.
///
/// Upstream packs an arbitrary count into a texture; this port caps at 8 (a clip
/// box needs 6). Must equal the `array<vec4<f32>, 8>` size in `clipping.wgsl`
/// (asserted by `wgsl_max_planes_matches_rust_const`).
pub const MAX_CLIPPING_PLANES: usize = 8;

// ─── Gate (mirror of the app-layer registry — see below) ─────────────────────

/// Env var gating the M6.2 clipping node.
///
/// **Single source of truth (task #81)**: the owner of this name is the app-layer
/// registry `application/cesium-app/src/feature_flags.rs` (`ENV_ENABLE_CLIPPING`
/// and the `clipping_enabled()` accessor, listed in `RESERVED_FLAGS`, default OFF).
/// This const is a *mirror* that exists only because `cesium-app` depends on
/// `cesium-bevy-render` (never the reverse), so this crate cannot import the
/// registry. It is `pub` so
/// `feature_flags::adapter_gate_mirrors_are_byte_identical_to_the_registry` can
/// assert byte-equality across the crate boundary; the registration itself is
/// driven by `effects::graph::M6WaveARenderGraphPlugin`, which reads
/// [`clipping_gate_enabled`] once per plugin phase.
pub const ENV_ENABLE_CLIPPING: &str = "CESIUM_ENABLE_CLIPPING";

/// Returns `true` when the clipping gate is enabled. Reuses the single
/// authoritative truthy parser (`gate_from_env_value`, the crate-wide
/// `{1, true, yes, on}` set) so it agrees with every other cesium gate.
#[inline]
pub fn clipping_gate_enabled() -> bool {
    gate_from_env_value(std::env::var(ENV_ENABLE_CLIPPING).ok())
}

// ─── Render graph label (local — #81 wires it into the Core3d chain) ─────────

/// Node label for the cesium clipping node in `Core3d`.
///
/// Defined locally so this module does not have to edit the shared
/// `CesiumPostProcessLabel` enum in `graph.rs`. Integration task #81 creates the
/// edges: recommended position is immediately after `Node3d::EndMainPass` (the
/// node operates on the HDR scene right after geometry, before AO / tonemapping):
/// `EndMainPass → CesiumClippingLabel → PassThrough → AmbientOcclusion → …`.
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct CesiumClippingLabel;

// ─── Component ───────────────────────────────────────────────────────────────

/// Component carrying an active [`ClippingPlaneCollection`] for a view.
///
/// Placed on the camera (like [`super::fxaa::CesiumFxaa`]) to drive the
/// screen-space node; #81 may also attach collections to globe / tileset entities
/// for the faithful forward-injection path. Extracted to the render world via
/// `ExtractComponentPlugin`. The node early-returns when `enabled == false` or the
/// collection is empty / disabled (zero GPU cost, pixel-neutral).
///
/// `Default` is derived: `enabled = false` (conservative, mirrors
/// `CesiumPassThrough`) and an empty `ClippingPlaneCollection`.
#[derive(Component, Clone, Debug, Default, ExtractComponent)]
pub struct CesiumClippingPlanes {
    /// Master enable for the clipping node on this view.
    pub enabled: bool,
    /// The domain clipping-plane collection (metric f64).
    pub collection: ClippingPlaneCollection,
}

impl CesiumClippingPlanes {
    /// Convenience constructor for an enabled view clipping collection.
    pub fn new(collection: ClippingPlaneCollection) -> Self {
        Self {
            enabled: true,
            collection,
        }
    }

    /// Whether clipping should actually run (all three gates agree).
    #[inline]
    pub fn is_active(&self) -> bool {
        self.enabled && self.collection.enabled && !self.collection.is_empty()
    }
}

/// Per-view cached pipeline ID for the clipping node.
#[derive(Component)]
pub struct CameraClippingPipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

/// Per-view GPU uniform buffer holding the packed clipping planes.
#[derive(Component)]
pub struct ViewClippingUniform {
    pub buffer: UniformBuffer<ClippingPlanesUniform>,
}

// ─── GPU uniform (f32 boundary) ──────────────────────────────────────────────

/// GPU-facing clipping uniform. **f32 only** — the domain collection stays metric
/// f64; [`ClippingPlanesUniform::from_domain`] performs the single metric →
/// render-unit conversion at this boundary (red line).
///
/// Layout must match `struct ClippingPlanes` in `shaders/clipping.wgsl`.
///
/// The struct lives in a private `clipping_uniform` module carrying
/// `#![allow(dead_code)]` (the `sky_dome.rs` convention): the encase `ShaderType`
/// derive emits a module-level helper the dead-code pass flags even though every
/// field is uploaded via `write_buffer`. Field values are asserted in the
/// `from_domain_*` unit tests.
pub use clipping_uniform::ClippingPlanesUniform;

mod clipping_uniform {
    #![allow(dead_code)]
    use super::MAX_CLIPPING_PLANES;
    use bevy::prelude::Vec4;
    use bevy::render::render_resource::ShaderType;

    /// GPU clipping uniform; layout matches `struct ClippingPlanes` in
    /// `shaders/clipping.wgsl` (encase std140).
    #[derive(ShaderType, Clone, Debug)]
    pub struct ClippingPlanesUniform {
        /// `xyz` = unit normal (world space), `w` = signed distance in RENDER UNITS.
        pub planes: [Vec4; MAX_CLIPPING_PLANES],
        /// `rgb` = edge highlight colour, `a` = edge width in PIXELS.
        pub edge_color: Vec4,
        /// Written for clipped fragments on the node path.
        pub background_color: Vec4,
        /// `x` = plane count, `y` = union flag, `z` = enabled, `w` = pad.
        pub params: Vec4,
    }
}

impl Default for ClippingPlanesUniform {
    fn default() -> Self {
        Self {
            planes: [Vec4::ZERO; MAX_CLIPPING_PLANES],
            edge_color: Vec4::new(1.0, 1.0, 1.0, 0.0),
            // params.z = 0 ⇒ the shader passes the source colour through untouched
            // (pixel-neutral). FIX-CLIP-BGCOLOR: when the node IS active it writes
            // background_color over clipped pixels with `blend: None`, and FXAA
            // carries alpha straight through to present, so a default alpha of 0
            // would read downstream as a fully-transparent hole. Default to an
            // opaque black "void" reveal instead; users override via the collection.
            background_color: Vec4::new(0.0, 0.0, 0.0, 1.0),
            params: Vec4::ZERO,
        }
    }
}

impl ClippingPlanesUniform {
    /// Packs a domain [`ClippingPlaneCollection`] into the GPU uniform.
    ///
    /// - Planes are baked into world space on the CPU via
    ///   [`ClippingPlaneCollection::world_planes`] (the CPU equivalent of upstream
    ///   per-fragment `czm_transformPlane`), then each plane's **metric distance is
    ///   divided by [`METERS_PER_RENDER_UNIT`]** to enter render-unit world space —
    ///   the same space `world_pos` reconstructed from depth lives in. Normals are
    ///   unit directions (scale-free), so they are cast straight to f32.
    /// - Count is clamped to [`MAX_CLIPPING_PLANES`].
    /// - `params.z` (enabled) is `0.0` when the collection is disabled or empty,
    ///   which makes the node a pure pass-through (pixel-neutral).
    pub fn from_domain(collection: &ClippingPlaneCollection) -> Self {
        let mut planes = [Vec4::ZERO; MAX_CLIPPING_PLANES];

        let world = collection.world_planes();
        let count = world.len().min(MAX_CLIPPING_PLANES);
        for (slot, plane) in planes.iter_mut().zip(world.iter().take(count)) {
            *slot = Vec4::new(
                plane.normal.x as f32,
                plane.normal.y as f32,
                plane.normal.z as f32,
                // metric → render unit (red line: divide by 6378137).
                (plane.distance / METERS_PER_RENDER_UNIT) as f32,
            );
        }

        let edge_color = Vec4::new(
            collection.edge_color[0] as f32,
            collection.edge_color[1] as f32,
            collection.edge_color[2] as f32,
            collection.edge_width as f32, // stays in PIXELS (shader uses fwidth)
        );

        let active = collection.enabled && !collection.is_empty();
        let params = Vec4::new(
            count as f32,
            if collection.union_clipping_regions { 1.0 } else { 0.0 },
            if active { 1.0 } else { 0.0 },
            0.0,
        );

        Self {
            planes,
            edge_color,
            // FIX-CLIP-BGCOLOR: opaque alpha so the node-path background is not a
            // transparent hole once FXAA carries alpha through to present. The
            // domain collection exposes no background colour yet; when it does,
            // thread it here (deferred #51 tracks the faithful per-geometry discard).
            background_color: Vec4::new(0.0, 0.0, 0.0, 1.0),
            params,
        }
    }
}

// ─── Pipeline ────────────────────────────────────────────────────────────────

/// Render-world resource: bind group layout + samplers for the clipping node.
///
/// Group 0 bindings (must match `clipping.wgsl` + the binding-coverage test):
/// - 0: depth prepass (`texture_depth_2d`) — world-position reconstruction
/// - 1: colour source (`texture_2d<f32>`) — post-process input
/// - 2: point sampler (NonFiltering, depth)
/// - 3: linear sampler (Filtering, colour)
/// - 4: `ViewUniform` (dynamic offset) — `view_from_clip` / `world_from_view`
/// - 5: `ClippingPlanesUniform` — the packed planes
#[derive(Resource)]
pub struct ClippingPlanesPipeline {
    pub bind_group_layout: BindGroupLayout,
    pub point_sampler: GpuSampler,
    pub linear_sampler: GpuSampler,
}

impl FromWorld for ClippingPlanesPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();

        let bind_group_layout = render_device.create_bind_group_layout(
            "cesium_clipping_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_depth_2d(),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::NonFiltering),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<ViewUniform>(true),
                    uniform_buffer::<ClippingPlanesUniform>(false),
                ),
            ),
        );

        let point_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_clipping_point_sampler"),
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        let linear_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_clipping_linear_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            bind_group_layout,
            point_sampler,
            linear_sampler,
        }
    }
}

// ─── ViewNode ────────────────────────────────────────────────────────────────

/// Screen-space clipping `ViewNode`.
///
/// Reconstructs the world position from the depth prepass and applies the
/// collection's union / intersection decision, overwriting clipped fragments with
/// the configured background colour and tinting the edge band. Early-returns
/// (pixel-neutral) when the component is inactive or the depth prepass / pipeline
/// is unavailable.
#[derive(Default)]
pub struct ClippingPlanesNode;

impl ViewNode for ClippingPlanesNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static CameraClippingPipeline,
        &'static CesiumClippingPlanes,
        &'static ViewClippingUniform,
        &'static ViewPrepassTextures,
        &'static ViewUniformOffset,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (target, pipeline_handle, clipping, clipping_uniform, prepass, view_uniform_offset): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if !clipping.is_active() {
            return Ok(());
        }

        // Clipping needs the depth prepass to reconstruct world position.
        let Some(depth_view) = prepass.depth_view() else {
            return Ok(());
        };

        let pipeline_cache = world.resource::<PipelineCache>();
        let clipping_pipeline = world.resource::<ClippingPlanesPipeline>();
        let view_uniforms = world.resource::<ViewUniforms>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_handle.pipeline_id) else {
            return Ok(());
        };
        let Some(view_uniform_binding) = view_uniforms.uniforms.binding() else {
            return Ok(());
        };
        let Some(clipping_binding) = clipping_uniform.buffer.binding() else {
            return Ok(());
        };

        let render_device = render_context.render_device().clone();

        let post_process = target.post_process_write();
        let source = post_process.source;
        let destination = post_process.destination;

        let bind_group = render_device.create_bind_group(
            Some("cesium_clipping_bind_group"),
            &clipping_pipeline.bind_group_layout,
            &BindGroupEntries::sequential((
                depth_view,
                source,
                &clipping_pipeline.point_sampler,
                &clipping_pipeline.linear_sampler,
                view_uniform_binding.clone(),
                clipping_binding,
            )),
        );

        let pass_descriptor = RenderPassDescriptor {
            label: Some("cesium_clipping_pass"),
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
        // Binding 4 (ViewUniform) is dynamic-offset; supply its offset (Ryan C2
        // lesson: a `&[]` offset list would fail dynamic-buffer validation).
        render_pass.set_bind_group(0, &bind_group, &[view_uniform_offset.offset]);
        render_pass.draw(0..3, 0..1); // fullscreen triangle

        Ok(())
    }
}

// ─── Render systems ──────────────────────────────────────────────────────────

/// Prepares the clipping pipeline + per-view uniform buffer for each active view.
/// Runs in `Render`, `RenderSet::Prepare` (before the render graph executes).
pub fn prepare_clipping(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    clipping_pipeline: Res<ClippingPlanesPipeline>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    views: Query<(Entity, &ExtractedView, &CesiumClippingPlanes)>,
) {
    for (entity, view, clipping) in &views {
        if !clipping.is_active() {
            continue;
        }

        let destination_format = if view.hdr {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::Rgba8UnormSrgb
        };

        let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("cesium_clipping_pipeline".into()),
            layout: vec![clipping_pipeline.bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: CLIPPING_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "fragment".into(),
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

        // Pack the domain collection (metric f64) into the GPU uniform (f32,
        // render-unit distances) and upload it.
        let mut buffer = UniformBuffer::from(ClippingPlanesUniform::from_domain(
            &clipping.collection,
        ));
        buffer.write_buffer(&render_device, &render_queue);

        commands.entity(entity).insert((
            CameraClippingPipeline { pipeline_id },
            ViewClippingUniform { buffer },
        ));
    }
}

// ─── Main-world systems ──────────────────────────────────────────────────────

/// Ensures cameras driving an active clipping collection carry [`DepthPrepass`]
/// (the node's world-position input). Adapter-layer enablement so the app-layer
/// camera bundle stays untouched (same discipline as `setup_ao_prepass`).
///
/// Only ADDS `DepthPrepass` (never removes it — AO may also need it). Registered
/// only when the clipping gate is ON, so gate OFF ⇒ no extra geometry pass ⇒ v0
/// baselines pixel-neutral.
pub fn setup_clipping_prepass(
    mut commands: Commands,
    cameras: Query<(Entity, &CesiumClippingPlanes)>,
) {
    for (entity, clipping) in &cameras {
        if clipping.is_active() {
            commands.entity(entity).insert(DepthPrepass);
        }
    }
}

// ─── Registration ────────────────────────────────────────────────────────────

/// Register the clipping node into `RenderApp` (shader + extract + node + systems).
///
/// Called by `effects::graph::M6WaveARenderGraphPlugin` (task #81) when
/// [`clipping_gate_enabled()`] is true; the `Core3d` edges are created by
/// `effects::graph::wire_m6_edges` (this function registers the node but never
/// wires edges, so the shared linear chain in `graph.rs` stays the single owner
/// and no diamond can form).
///
/// Headless-safe: degrades to a no-op without a `RenderApp` (MinimalPlugins).
#[deprecated = "DEV-029 / FIX-REG-FACADE: call `register_clipping_planes_node_main_world` from `Plugin::build` and `register_clipping_planes_node_render_world` from `Plugin::finish`; this facade runs the finish half against a possibly device-less render world."]
pub fn register_clipping_planes_node(app: &mut App) {
    register_clipping_planes_node_main_world(app);
    // Headless `MinimalPlugins` has no `RenderApp` — degrade gracefully.
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        register_clipping_planes_node_render_world(render_app);
    }
}

/// `Plugin::build`-time half of [`register_clipping_planes_node`]: everything
/// that lives in the **main** world (WGSL shader asset +
/// `ExtractComponentPlugin` + the `setup_clipping_prepass` system).
///
/// Split out by task #81 — see `docs/deviations.md#dev-029`. The pipeline's
/// `FromWorld` reads `RenderDevice`, which Bevy only inserts into the render
/// world in `RenderPlugin::finish`, so the render-world half below must run from
/// a plugin's `finish`, never from its `build`.
pub fn register_clipping_planes_node_main_world(app: &mut App) {
    // Register the clipping WGSL shader (headless-safe via shader_registry).
    crate::shader_registry::try_load_internal_shader(
        app,
        CLIPPING_SHADER_HANDLE,
        include_str!("../../shaders/clipping.wgsl"),
        "shaders/clipping.wgsl",
    );

    // ExtractComponentPlugin: main → render world each frame (ExtractSchedule).
    app.add_plugins(ExtractComponentPlugin::<CesiumClippingPlanes>::default());

    // Main-world: attach DepthPrepass to cameras driving an active collection.
    app.add_systems(Update, setup_clipping_prepass);
}

/// `Plugin::finish`-time half of [`register_clipping_planes_node`]: the
/// render-world pipeline resource + the `Core3d` node.
pub fn register_clipping_planes_node_render_world(render_app: &mut bevy::app::SubApp) {
    // FIX-REG-FACADE (DEV-029): degrade to a no-op when `RenderDevice` is absent
    // (finish half reached from `build`, or a bare render world). See
    // `crate::effects::render_world_missing_device`.
    if crate::effects::render_world_missing_device(render_app) {
        return;
    }

    render_app
        .init_resource::<ClippingPlanesPipeline>()
        .add_systems(Render, prepare_clipping.in_set(RenderSet::Prepare))
        .add_render_graph_node::<ViewNodeRunner<ClippingPlanesNode>>(
            Core3d,
            CesiumClippingLabel,
        );
    // NOTE: edges are created by `effects::graph::wire_m6_edges` (task #81), the
    // single owner of the shared `Core3d` chain.
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_effects::clipping::ClippingPlane;
    use glam::DVec3;

    #[test]
    fn clipping_component_default_disabled() {
        let c = CesiumClippingPlanes::default();
        assert!(!c.enabled);
        assert!(!c.is_active());
    }

    #[test]
    fn clipping_gate_const_is_local_and_stable() {
        // The gate env var is defined LOCALLY (isolation discipline) and must not
        // collide with the post-process gates. Pure assertion (no env mutation).
        assert_eq!(ENV_ENABLE_CLIPPING, "CESIUM_ENABLE_CLIPPING");
        assert_ne!(ENV_ENABLE_CLIPPING, "CESIUM_ENABLE_POSTPROCESS");
        // Reuses the authoritative truthy parser.
        assert!(!gate_from_env_value(None));
        assert!(gate_from_env_value(Some("1".into())));
    }

    #[test]
    fn clipping_shader_handle_unique() {
        assert_ne!(
            CLIPPING_SHADER_HANDLE,
            super::super::graph::PASS_THROUGH_SHADER_HANDLE
        );
        assert_ne!(CLIPPING_SHADER_HANDLE, super::super::fxaa::FXAA_SHADER_HANDLE);
        assert_ne!(CLIPPING_SHADER_HANDLE, super::super::ao::AO_SHADER_HANDLE);
    }

    #[test]
    fn clipping_headless_graceful() {
        // No RenderApp (headless) → register must not panic.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        #[allow(deprecated)]
        register_clipping_planes_node(&mut app);
    }

    // ─── Uniform packing: the metric → render-unit boundary (red line) ────────

    #[test]
    fn from_domain_divides_distance_by_meters_per_render_unit() {
        // A plane 6_378_137 m (== 1 render unit) along +Y. Its normal is unit
        // (unchanged); its metric distance must arrive divided by 6378137.
        let collection = ClippingPlaneCollection::with_planes(vec![ClippingPlane::new(
            DVec3::Y,
            METERS_PER_RENDER_UNIT,
        )]);
        let u = ClippingPlanesUniform::from_domain(&collection);

        // identity model_matrix ⇒ world plane == local plane.
        let plane = u.planes[0];
        assert!((plane.x).abs() < 1e-6, "normal.x");
        assert!((plane.y - 1.0).abs() < 1e-6, "normal.y preserved");
        assert!((plane.z).abs() < 1e-6, "normal.z");
        // 6378137 m / 6378137 = 1.0 render unit.
        assert!(
            (plane.w - 1.0).abs() < 1e-5,
            "distance must be metric / METERS_PER_RENDER_UNIT, got {}",
            plane.w
        );

        // params: count = 1, union = 0, enabled = 1.
        assert!((u.params.x - 1.0).abs() < 1e-6);
        assert!((u.params.y).abs() < 1e-6);
        assert!((u.params.z - 1.0).abs() < 1e-6);
    }

    #[test]
    fn from_domain_union_flag_and_edge_width() {
        let mut collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
            ClippingPlane::new(DVec3::X, 0.0),
        ]);
        collection.union_clipping_regions = true;
        collection.edge_width = 3.0;
        collection.edge_color = [1.0, 0.0, 0.0, 1.0];

        let u = ClippingPlanesUniform::from_domain(&collection);
        assert!((u.params.x - 2.0).abs() < 1e-6, "count");
        assert!((u.params.y - 1.0).abs() < 1e-6, "union flag set");
        assert!((u.params.z - 1.0).abs() < 1e-6, "enabled");
        assert!((u.edge_color.w - 3.0).abs() < 1e-6, "edge width in pixels");
        assert!((u.edge_color.x - 1.0).abs() < 1e-6, "edge colour r");
    }

    #[test]
    fn from_domain_disabled_or_empty_is_pixel_neutral() {
        // Empty collection ⇒ params.z = 0 ⇒ shader passes source through (neutral).
        let empty = ClippingPlaneCollection::default();
        assert!((ClippingPlanesUniform::from_domain(&empty).params.z).abs() < 1e-6);

        // Non-empty but disabled ⇒ also params.z = 0.
        let mut disabled = ClippingPlaneCollection::with_planes(vec![ClippingPlane::new(
            DVec3::Y,
            0.0,
        )]);
        disabled.enabled = false;
        assert!((ClippingPlanesUniform::from_domain(&disabled).params.z).abs() < 1e-6);
    }

    #[test]
    fn from_domain_caps_at_max_planes() {
        let planes: Vec<ClippingPlane> = (0..(MAX_CLIPPING_PLANES + 4))
            .map(|i| ClippingPlane::new(DVec3::Y, i as f64))
            .collect();
        let collection = ClippingPlaneCollection::with_planes(planes);
        let u = ClippingPlanesUniform::from_domain(&collection);
        assert!((u.params.x - MAX_CLIPPING_PLANES as f32).abs() < 1e-6);
    }

    #[test]
    fn setup_clipping_prepass_adds_depth_prepass_when_active() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, setup_clipping_prepass);

        let collection =
            ClippingPlaneCollection::with_planes(vec![ClippingPlane::new(DVec3::Y, 0.0)]);
        let cam = app
            .world_mut()
            .spawn((Camera3d::default(), CesiumClippingPlanes::new(collection)))
            .id();

        app.update();
        assert!(
            app.world().get::<DepthPrepass>(cam).is_some(),
            "active clipping ⇒ DepthPrepass attached"
        );

        // Disable ⇒ no new prepass forced on a fresh camera.
        let cam2 = app
            .world_mut()
            .spawn((Camera3d::default(), CesiumClippingPlanes::default()))
            .id();
        app.update();
        assert!(app.world().get::<DepthPrepass>(cam2).is_none());
    }

    // ─── Ryan C1/C2 defence line: headless naga parse + validate + layout ──────

    /// naga has no preprocessor, so the two `#import`s in `clipping.wgsl` are
    /// replaced by struct stubs declaring exactly the fields the shader reads:
    /// `FullscreenVertexOutput.{position, uv}` and `View.{view_from_clip,
    /// world_from_view}`. The `view` **binding** itself is declared by the real
    /// shader text (group 0, binding 4), so it is deliberately not stubbed here.
    const CLIPPING_WGSL_IMPORT_STUBS: &str = "\
struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct View {
    view_from_clip: mat4x4<f32>,
    world_from_view: mat4x4<f32>,
}
";

    fn clipping_stubbed_wgsl() -> String {
        let mut source = String::from(CLIPPING_WGSL_IMPORT_STUBS);
        for line in include_str!("../../shaders/clipping.wgsl").lines() {
            if line.starts_with("#import") {
                continue;
            }
            source.push_str(line);
            source.push('\n');
        }
        source
    }

    /// Headless proof the clipping shader is real: parsed + type-checked by
    /// **naga** (the same WGSL front end `bevy_render` compiles it with on the GPU
    /// path). Guards the M5 C1 (reserved word) class of silent-failure bug.
    #[test]
    fn clipping_wgsl_parses_and_type_checks_under_naga() {
        let source = clipping_stubbed_wgsl();
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("clipping.wgsl does not parse:\n{}", error.emit_to_string(&source))
        });
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("clipping.wgsl does not validate");

        let entry_points = module
            .entry_points
            .iter()
            .map(|entry| (entry.name.as_str(), entry.stage))
            .collect::<Vec<_>>();
        assert_eq!(
            entry_points,
            vec![("fragment", naga::ShaderStage::Fragment)],
            "clipping.wgsl must expose exactly one fragment entry point"
        );
    }

    /// Ryan C1 (catches the **C2** class of bug): every binding the `fragment`
    /// entry statically uses must be present in the Rust `ClippingPlanesPipeline`
    /// layout (group 0: bindings 0..=5). Guards against a silent pipeline-build
    /// failure that would no-op clipping while `pixel_diff` reported a false green.
    #[test]
    fn clipping_wgsl_entry_bindings_are_covered_by_the_rust_layout() {
        let source = clipping_stubbed_wgsl();
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("clipping.wgsl does not parse:\n{}", error.emit_to_string(&source))
        });

        // Mirror of `ClippingPlanesPipeline::from_world` (group 0 binding indices).
        let layout: std::collections::BTreeSet<(u32, u32)> =
            [(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5)].into_iter().collect();

        let entry = module
            .entry_points
            .iter()
            .find(|e| e.name == "fragment")
            .expect("clipping.wgsl must have a `fragment` entry point");

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
                    naga::Expression::CallResult(func_handle)
                        if visited.insert(func_handle) =>
                    {
                        stack.push(&module.functions[func_handle]);
                    }
                    _ => {}
                }
            }
        }
        let missing: Vec<(u32, u32)> = used.difference(&layout).copied().collect();
        assert!(
            missing.is_empty(),
            "clipping.wgsl `fragment` statically uses bindings {missing:?} absent from \
             ClippingPlanesPipeline's layout (C2-class regression: the pipeline would fail \
             to build and clipping would silently no-op)"
        );
    }

    /// Cross-check: the WGSL uniform array size must equal the Rust
    /// [`MAX_CLIPPING_PLANES`] const, so `from_domain`'s clamp matches the shader.
    #[test]
    fn wgsl_max_planes_matches_rust_const() {
        let wgsl = include_str!("../../shaders/clipping.wgsl");
        assert!(
            wgsl.contains(&format!("array<vec4<f32>, {MAX_CLIPPING_PLANES}>")),
            "clipping.wgsl plane array size must match MAX_CLIPPING_PLANES = {MAX_CLIPPING_PLANES}"
        );
        assert!(
            wgsl.contains(&format!("const MAX_CLIPPING_PLANES: i32 = {MAX_CLIPPING_PLANES};")),
            "clipping.wgsl MAX_CLIPPING_PLANES const must match Rust"
        );
    }

    /// Red-line guard: the WGSL must keep the metric → render-unit conversion note
    /// and the NO-FMA-CONTRACTION invariant visible in-source (mirrors the
    /// sky_atmosphere.wgsl header assertions).
    #[test]
    fn wgsl_declares_red_lines() {
        let wgsl = include_str!("../../shaders/clipping.wgsl");
        assert!(wgsl.contains("METERS_PER_RENDER_UNIT = 6378137"));
        assert!(wgsl.contains("NO FMA CONTRACTION"));
    }

    /// f32 mirror of the GPU `apply_clipping_planes` accumulation
    /// (`clipping.wgsl`), kept here so the shader's signed min/max + `<= 0.0`
    /// semantics are cross-validated against the f64 domain reference
    /// [`ClippingPlaneCollection::clip_signed`]. FIX-CLIP-CPUREF.
    fn gpu_clip_mirror_f32(planes: &[(glam::Vec3, f32)], union: bool, world_pos: glam::Vec3) -> (bool, f32) {
        let mut any_outside = false;
        let mut all_outside = true;
        let mut clip_amount = 0f32;
        for (i, (n, d0)) in planes.iter().enumerate() {
            let d = n.dot(world_pos) + *d0;
            if d <= 0.0 {
                any_outside = true;
            } else {
                all_outside = false;
            }
            clip_amount = if union {
                if i == 0 { d } else { d.min(clip_amount) }
            } else {
                d.max(clip_amount)
            };
        }
        (if union { any_outside } else { all_outside }, clip_amount)
    }

    #[test]
    fn clipping_gpu_signed_accumulation_matches_the_cpu_reference() {
        // Deterministic LCG (no rand dependency). Values in [0, 1).
        struct Lcg(u64);
        impl Lcg {
            fn next01(&mut self) -> f64 {
                self.0 = self
                    .0
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ((self.0 >> 40) as f64) / ((1u64 << 24) as f64)
            }
            fn signed(&mut self, scale: f64) -> f64 {
                (self.next01() * 2.0 - 1.0) * scale
            }
        }
        let mut rng = Lcg(0x9E37_79B9_7F4A_7C15);

        for union in [true, false] {
            for trial in 0..200u32 {
                let plane_count = 1 + (rng.next01() * 4.0).min(4.0) as usize;
                let mut planes = Vec::new();
                let mut domain = Vec::new();
                for _ in 0..plane_count {
                    let n = DVec3::new(rng.signed(1.0), rng.signed(1.0), rng.signed(1.0));
                    let n = if n.length_squared() < 1e-12 { DVec3::Y } else { n.normalize() };
                    let dist = rng.signed(6.0);
                    let p = ClippingPlane::new(n, dist);
                    // ClippingPlane::new re-normalizes; read the authoritative
                    // normal/distance back so the f32 mirror sees the *same* plane.
                    domain.push((glam::Vec3::new(p.normal.x as f32, p.normal.y as f32, p.normal.z as f32), p.distance as f32));
                    planes.push(p);
                }
                let point = DVec3::new(rng.signed(8.0), rng.signed(8.0), rng.signed(8.0));

                let mut collection = ClippingPlaneCollection::with_planes(planes);
                collection.union_clipping_regions = union;
                let (cpu_clipped, cpu_amount) = collection.clip_signed(point);

                let gpu_point = glam::Vec3::new(point.x as f32, point.y as f32, point.z as f32);
                let (gpu_clipped, gpu_amount) = gpu_clip_mirror_f32(&domain, union, gpu_point);
                let gpu_amount: f64 = gpu_amount as f64;

                assert_eq!(
                    cpu_clipped, gpu_clipped,
                    "trial {trial} union={union}: clipped mismatch"
                );
                assert!(
                    (cpu_amount - gpu_amount).abs() < 1e-3,
                    "trial {trial} union={union}: clip_amount {cpu_amount} vs {gpu_amount}"
                );
            }
        }
    }
}
