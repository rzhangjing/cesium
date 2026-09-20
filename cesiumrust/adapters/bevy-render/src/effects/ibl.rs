//! M6.5: Image-Based Lighting (IBL) `ViewNode` + environment PBR injection.
//!
//! Ports the upstream CesiumJS image-based-lighting capability
//! (`Scene/ImageBasedLighting.js` + `Shaders/Model/ImageBasedLightingStageFS.glsl`
//! `textureIBL`) onto the M5-E render-graph infrastructure (`graph.rs`). Mirrors
//! the [`super::clipping_planes`] / [`super::ao`] pattern: this module registers
//! the node / resources / systems **but never creates graph edges** — the single
//! linear `Core3d` chain in `graph.rs::register_render_graph` owns them, so no
//! diamond can form. Wiring the IBL node into that chain is integration task #81.
//!
//! # Two IBL paths (see `shaders/ibl.wgsl`)
//! 1. **Forward material stage (faithful)** — `texture_ibl(...)` is `#import`-ed
//!    into the globe / tileset / fabric fragment shaders so the environment BRDF
//!    runs per-fragment against the glTF material, exactly where upstream
//!    `textureIBL` runs in `ImageBasedLightingStageFS`. That wiring touches the
//!    material shaders (out of M6.5 file scope) and is deferred to #81.
//! 2. **Screen-space node (this file)** — [`IblNode`] reconstructs the world
//!    position + normal from the depth / normal prepass (the WGSL analogue of
//!    upstream eye-space reconstruction) and adds the environment lighting onto the
//!    HDR scene colour, reading material params from a uniform (DEVIATION: see
//!    `docs/deviations.md#dev-024` and the header of `shaders/ibl.wgsl` —
//!    single-material scene + inline split-sum BRDF).
//!
//! # Gate (single source of truth)
//! The gate name is owned by the app-layer registry
//! `application/cesium-app/src/feature_flags.rs` (`ENV_ENABLE_IBL` /
//! `ibl_enabled()`); [`ENV_ENABLE_IBL`] below is a byte-identical mirror forced by
//! the crate dependency direction (`cesium-app` → `cesium-bevy-render`). Default
//! OFF ⇒ `effects::graph::M6WaveARenderGraphPlugin` (task #81) returns early from
//! both halves ⇒ [`register_ibl_node`] is never called and no `Core3d` edges exist
//! ⇒ the node never runs ⇒ dynamic_globe v0 baselines stay pixel-neutral
//! (PSNR = ∞).
//!
//! # Red lines honoured
//! - domain stays **f64** ([`ImageBasedLighting`] SH coefficients, [`IblMaterial`]
//!   params); the f64 → f32 projection happens ONLY at
//!   [`IblUniform::from_domain`] (the GPU uniform boundary).
//! - `ibl.wgsl` keeps the reflection vector and the Fresnel pow5 as two roundings
//!   (NO FMA contraction — IBL numerics are integration-sensitive).
//! - Environment COLOUR maps are sRGB on the CPU side; the LUT / prefilter outputs
//!   are LINEAR (never sRGB). glam fast-math is disabled repo-wide.
//!
//! # Blueprint
//! - `packages/engine/Source/Scene/ImageBasedLighting.js`
//! - `packages/engine/Source/Shaders/Model/ImageBasedLightingStageFS.glsl` (textureIBL)
//! - `packages/engine/Source/Shaders/Builtin/Functions/{sphericalHarmonics,pbrLighting}.glsl`
//! - `packages/engine/Source/Shaders/{BrdfLutGeneratorFS,ConvolveSpecularMapFS}.glsl`
//! - `domain/effects/src/ibl.rs` (f64 CPU reference, cross-validated by these tests)
//! - `bevy_pbr-0.15.3/src/ssao/mod.rs` (ViewNode + depth/normal prepass reconstruction)

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
        NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
    },
    render_resource::{
        binding_types::{sampler, texture_2d, texture_cube, texture_depth_2d, uniform_buffer},
        BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
        CachedRenderPipelineId, ColorTargetState, ColorWrites, Extent3d, FilterMode, FragmentState,
        MultisampleState, Operations, PipelineCache, PrimitiveState, RenderPassColorAttachment,
        RenderPassDescriptor, RenderPipelineDescriptor, Sampler as GpuSampler, SamplerBindingType,
        SamplerDescriptor, ShaderStages, Texture, TextureDescriptor, TextureDimension,
        TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor,
        TextureViewDimension, UniformBuffer,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
    Render, RenderApp, RenderSet,
};
use cesium_effects::ibl::{default_spherical_harmonics, IblMaterial, ImageBasedLighting};

use super::graph::gate_from_env_value;

// ─── Shader handle ───────────────────────────────────────────────────────────

/// Unique handle for the embedded `ibl.wgsl` shader (distinct from the pass-through
/// / FXAA / AO / clipping handles — see the collision test below).
pub const IBL_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(0xCE51_1B15_1810_0065);

// ─── Gate (mirror of the app-layer registry — see below) ─────────────────────

/// Env var gating the M6.5 IBL node.
///
/// **Single source of truth (task #81)**: the owner of this name is the app-layer
/// registry `application/cesium-app/src/feature_flags.rs` (`ENV_ENABLE_IBL` + the
/// `ibl_enabled()` accessor, listed in `RESERVED_FLAGS`, default OFF). This const
/// is a *mirror* that exists only because `cesium-app` depends on
/// `cesium-bevy-render` (never the reverse), so this crate cannot import the
/// registry. It is `pub` so
/// `feature_flags::adapter_gate_mirrors_are_byte_identical_to_the_registry` can
/// assert byte-equality across the crate boundary; the registration itself is
/// driven by `effects::graph::M6WaveARenderGraphPlugin`, which reads
/// [`ibl_gate_enabled`] once per plugin phase.
pub const ENV_ENABLE_IBL: &str = "CESIUM_ENABLE_IBL";

/// Returns `true` when the IBL gate is enabled. Reuses the single authoritative
/// truthy parser (`gate_from_env_value`, the crate-wide `{1, true, yes, on}` set)
/// so it agrees with every other cesium gate.
#[inline]
pub fn ibl_gate_enabled() -> bool {
    gate_from_env_value(std::env::var(ENV_ENABLE_IBL).ok())
}

// ─── Render graph label (local — #81 wires it into the Core3d chain) ─────────

/// Node label for the cesium IBL node in `Core3d`.
///
/// Defined locally so this module does not have to edit the shared
/// `CesiumPostProcessLabel` enum in `graph.rs`. Integration task #81 creates the
/// edges; recommended position is immediately after `Node3d::EndMainPass` (the node
/// adds environment lighting to the HDR scene before AO / tonemapping):
/// `EndMainPass → CesiumIblLabel → PassThrough → AmbientOcclusion → …`.
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct CesiumIblLabel;

// ─── Component ───────────────────────────────────────────────────────────────

/// Component carrying an active [`ImageBasedLighting`] environment for a view.
///
/// Placed on the camera (like [`super::fxaa::CesiumFxaa`]) to drive the
/// screen-space node. Extracted to the render world via `ExtractComponentPlugin`;
/// the node early-returns when [`CesiumIbl::is_active`] is `false` (zero GPU cost).
#[derive(Component, Clone, Debug, ExtractComponent)]
pub struct CesiumIbl {
    /// Master enable for the IBL node on this view.
    pub enabled: bool,
    /// Domain IBL environment (SH coefficients + factor, metric f64).
    pub ibl: ImageBasedLighting,
    /// Domain PBR surface params for the screen-space apply (uniform material).
    pub material: IblMaterial,
    /// Cubemap max LOD (roughness → mip scale); `0.0` for a single-level default.
    pub max_lod: f32,
}

impl Default for CesiumIbl {
    fn default() -> Self {
        Self {
            // Conservative default: disabled (mirrors CesiumPassThrough / clipping).
            enabled: false,
            ibl: ImageBasedLighting::default(),
            material: IblMaterial::default(),
            max_lod: 0.0,
        }
    }
}

impl CesiumIbl {
    /// Convenience constructor for an enabled IBL environment.
    pub fn new(ibl: ImageBasedLighting, material: IblMaterial) -> Self {
        Self {
            enabled: true,
            ibl,
            material,
            max_lod: 0.0,
        }
    }

    /// Whether IBL should actually run: enabled AND at least one factor non-zero.
    #[inline]
    pub fn is_active(&self) -> bool {
        self.enabled
            && (self.ibl.image_based_lighting_factor[0] > 0.0
                || self.ibl.image_based_lighting_factor[1] > 0.0)
    }
}

/// Per-view cached pipeline ID for the IBL node.
#[derive(Component)]
pub struct CameraIblPipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

/// Per-view GPU uniform buffer holding the packed IBL environment.
#[derive(Component)]
pub struct ViewIblUniform {
    pub buffer: UniformBuffer<IblUniform>,
}

// ─── GPU uniform (f32 boundary) ──────────────────────────────────────────────

/// GPU-facing IBL uniform. **f32 only** — the domain [`ImageBasedLighting`] and
/// [`IblMaterial`] stay metric f64; [`IblUniform::from_domain`] performs the single
/// f64 → f32 projection at this boundary (red line).
///
/// Layout must match `struct IblData` in `shaders/ibl.wgsl` (all `vec4` for
/// alignment; asserted by `wgsl_uniform_layout_matches_rust`).
///
/// The struct lives in a private `ibl_uniform` module carrying
/// `#![allow(dead_code)]` (the `sky_dome.rs` / `clipping_planes.rs` convention): the
/// encase `ShaderType` derive emits a module-level helper the dead-code pass flags
/// even though every field is uploaded via `write_buffer`. Field values are asserted
/// in the `from_domain_*` unit tests.
pub use ibl_uniform::IblUniform;

mod ibl_uniform {
    #![allow(dead_code)]
    use bevy::prelude::Vec4;
    use bevy::render::render_resource::ShaderType;

    /// GPU IBL uniform; layout matches `struct IblData` in `shaders/ibl.wgsl`
    /// (encase std140, all vec4 for alignment).
    #[derive(ShaderType, Clone, Debug)]
    pub struct IblUniform {
        /// 9 CesiumJS-convention (PRE-SCALED) SH irradiance coefficients; rgb in `.xyz`.
        pub sh: [Vec4; 9],
        /// `x` = perceptual roughness, `yzw` = specular F0 reflectance (linear rgb).
        pub material: Vec4,
        /// `xyz` = lambertian base colour (linear rgb), `w` = specular weight.
        pub diffuse: Vec4,
        /// `x` = diffuse IBL factor, `y` = specular IBL factor, `z` = cubemap max LOD.
        pub params: Vec4,
    }
}

impl Default for IblUniform {
    fn default() -> Self {
        Self {
            sh: [Vec4::ZERO; 9],
            material: Vec4::ZERO,
            diffuse: Vec4::ZERO,
            // params.x/y = 0 ⇒ the shader passes the source colour through
            // untouched (pixel-neutral) even if the node somehow runs.
            params: Vec4::ZERO,
        }
    }
}

impl IblUniform {
    /// Packs a domain [`ImageBasedLighting`] + [`IblMaterial`] into the GPU uniform.
    ///
    /// The f64 SH coefficients (or [`default_spherical_harmonics`] when unset) and
    /// the f64 material params are each cast to f32 **here and only here** (red
    /// line). `params.z` is the cubemap max LOD used by the WGSL roughness → mip
    /// mapping. When both IBL factors are zero the node stays a pure pass-through.
    pub fn from_domain(ibl: &ImageBasedLighting, material: &IblMaterial, max_lod: f32) -> Self {
        let coeffs = ibl
            .spherical_harmonic_coefficients
            .unwrap_or_else(default_spherical_harmonics);

        let mut sh = [Vec4::ZERO; 9];
        for (slot, c) in sh.iter_mut().zip(coeffs.iter()) {
            *slot = Vec4::new(c[0] as f32, c[1] as f32, c[2] as f32, 0.0);
        }

        Self {
            sh,
            material: Vec4::new(
                material.roughness as f32,
                material.specular_f0[0] as f32,
                material.specular_f0[1] as f32,
                material.specular_f0[2] as f32,
            ),
            diffuse: Vec4::new(
                material.diffuse[0] as f32,
                material.diffuse[1] as f32,
                material.diffuse[2] as f32,
                material.specular_weight as f32,
            ),
            params: Vec4::new(
                ibl.image_based_lighting_factor[0] as f32,
                ibl.image_based_lighting_factor[1] as f32,
                max_lod,
                0.0,
            ),
        }
    }
}

// ─── Pipeline ────────────────────────────────────────────────────────────────

/// Render-world resource: bind group layout + samplers + a default specular
/// cubemap for the IBL node.
///
/// Group 0 bindings (must match `ibl.wgsl` + the binding-coverage test):
/// - 0: depth prepass (`texture_depth_2d`) — world-position reconstruction
/// - 1: normal prepass (`texture_2d<f32>`) — world normal
/// - 2: colour source (`texture_2d<f32>`) — HDR scene to add IBL onto
/// - 3: point sampler (NonFiltering, depth + normal)
/// - 4: linear sampler (Filtering, colour + cubemap)
/// - 5: `ViewUniform` (dynamic offset) — `view_from_clip` / `world_from_view`
/// - 6: `IblUniform` — the packed SH + material + factor
/// - 7: specular environment (`texture_cube<f32>`) — a neutral 1×1 default cube (#81 binds a real prefiltered cubemap on the material path).
///
/// The layout is the SUPERSET used by every `ibl.wgsl` entry; wgpu permits a
/// pipeline layout to declare more bindings than a given entry statically uses, so
/// the three offline generator entries share this same layout.
#[derive(Resource)]
pub struct IblPipeline {
    pub bind_group_layout: BindGroupLayout,
    pub point_sampler: GpuSampler,
    pub linear_sampler: GpuSampler,
    /// Neutral 1×1×6 cube kept alive so its view stays valid (binding 7).
    pub default_specular_cubemap: Texture,
    pub default_specular_cubemap_view: TextureView,
}

impl FromWorld for IblPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();

        let bind_group_layout = render_device.create_bind_group_layout(
            "cesium_ibl_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_depth_2d(),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::NonFiltering),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<ViewUniform>(true),
                    uniform_buffer::<IblUniform>(false),
                    texture_cube(TextureSampleType::Float { filterable: true }),
                ),
            ),
        );

        let point_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_ibl_point_sampler"),
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        let linear_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_ibl_linear_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            // LINEAR mip filtering so roughness → LOD blurs the prefiltered env.
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        // Neutral 1×1×6 cube (wgpu zero-initialises ⇒ black environment ⇒ the
        // specular term contributes nothing until #81 binds a real cubemap; the
        // diffuse SH term still lights the scene from the uniform). Rgba16Float is
        // a filterable float format (LINEAR data, never sRGB — red line).
        let default_specular_cubemap = render_device.create_texture(&TextureDescriptor {
            label: Some("cesium_ibl_default_specular_cubemap"),
            size: Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba16Float,
            usage: TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let default_specular_cubemap_view = default_specular_cubemap.create_view(
            &TextureViewDescriptor {
                label: Some("cesium_ibl_default_specular_cubemap_view"),
                dimension: Some(TextureViewDimension::Cube),
                ..Default::default()
            },
        );

        Self {
            bind_group_layout,
            point_sampler,
            linear_sampler,
            default_specular_cubemap,
            default_specular_cubemap_view,
        }
    }
}

// ─── ViewNode ────────────────────────────────────────────────────────────────

/// Screen-space IBL `ViewNode`.
///
/// Reconstructs the world position + normal from the depth / normal prepass and
/// adds the environment lighting (SH diffuse + prefiltered-cubemap specular via the
/// Fdez-Aguera split-sum) onto the HDR scene colour. Early-returns (pixel-neutral)
/// when the component is inactive or the prepass / pipeline is unavailable.
#[derive(Default)]
pub struct IblNode {
    /// Guards against binding a stale group across a source-texture change.
    cached_source_id: Mutex<Option<bevy::render::render_resource::TextureViewId>>,
}

impl ViewNode for IblNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static CameraIblPipeline,
        &'static CesiumIbl,
        &'static ViewIblUniform,
        &'static ViewPrepassTextures,
        &'static ViewUniformOffset,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (target, pipeline_handle, ibl, ibl_uniform, prepass, view_uniform_offset): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if !ibl.is_active() {
            return Ok(());
        }

        // IBL needs both the depth (world position) and normal (orientation) prepass.
        let (Some(depth_view), Some(normal_view)) = (prepass.depth_view(), prepass.normal_view())
        else {
            return Ok(());
        };

        let pipeline_cache = world.resource::<PipelineCache>();
        let ibl_pipeline = world.resource::<IblPipeline>();
        let view_uniforms = world.resource::<ViewUniforms>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_handle.pipeline_id) else {
            return Ok(());
        };
        let Some(view_uniform_binding) = view_uniforms.uniforms.binding() else {
            return Ok(());
        };
        let Some(ibl_binding) = ibl_uniform.buffer.binding() else {
            return Ok(());
        };

        let render_device = render_context.render_device().clone();

        let post_process = target.post_process_write();
        let source = post_process.source;
        let destination = post_process.destination;

        // Record the source id (cache-invalidation marker for future per-frame bind
        // group reuse; the group is rebuilt each frame here since it depends on the
        // per-frame prepass views + post-process source).
        // FIX-IBL-MUTEX: this runs on the render-world `prepare` path — a poisoned
        // mutex must never cascade-panic every subsequent frame. Recover the inner
        // value from the poison instead of `unwrap()`-panicking.
        *self.cached_source_id.lock().unwrap_or_else(|e| e.into_inner()) = Some(source.id());

        let bind_group = render_device.create_bind_group(
            Some("cesium_ibl_bind_group"),
            &ibl_pipeline.bind_group_layout,
            &BindGroupEntries::sequential((
                depth_view,
                normal_view,
                source,
                &ibl_pipeline.point_sampler,
                &ibl_pipeline.linear_sampler,
                view_uniform_binding.clone(),
                ibl_binding,
                &ibl_pipeline.default_specular_cubemap_view,
            )),
        );

        let pass_descriptor = RenderPassDescriptor {
            label: Some("cesium_ibl_pass"),
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
        // Binding 5 (ViewUniform) is dynamic-offset; supply its offset (Ryan C2
        // lesson: a `&[]` offset list would fail dynamic-buffer validation).
        render_pass.set_bind_group(0, &bind_group, &[view_uniform_offset.offset]);
        render_pass.draw(0..3, 0..1); // fullscreen triangle

        Ok(())
    }
}

// ─── Render systems ──────────────────────────────────────────────────────────

/// Prepares the IBL pipeline + per-view uniform buffer for each active view.
/// Runs in `Render`, `RenderSet::Prepare` (before the render graph executes).
pub fn prepare_ibl(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    ibl_pipeline: Res<IblPipeline>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    views: Query<(Entity, &ExtractedView, &CesiumIbl)>,
) {
    for (entity, view, ibl) in &views {
        if !ibl.is_active() {
            continue;
        }

        let destination_format = if view.hdr {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::Rgba8UnormSrgb
        };

        let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("cesium_ibl_pipeline".into()),
            layout: vec![ibl_pipeline.bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: IBL_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: destination_format,
                    // Additive environment lighting would use ONE_MINUS_SRC_COLOR-style
                    // blending on a real forward pass; here the shader reads the source
                    // and writes source+ibl, so no hardware blend is needed.
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

        // Pack the domain environment (f64) into the GPU uniform (f32 boundary).
        let mut buffer =
            UniformBuffer::from(IblUniform::from_domain(&ibl.ibl, &ibl.material, ibl.max_lod));
        buffer.write_buffer(&render_device, &render_queue);

        commands.entity(entity).insert((
            CameraIblPipeline { pipeline_id },
            ViewIblUniform { buffer },
        ));
    }
}

// ─── Main-world systems ──────────────────────────────────────────────────────

/// Ensures cameras driving an active IBL environment carry [`DepthPrepass`] +
/// [`NormalPrepass`] (the node's world-position + normal inputs). Adapter-layer
/// enablement so the app-layer camera bundle stays untouched (same discipline as
/// `setup_ao_prepass` / `setup_clipping_prepass`).
///
/// Only ADDS the prepasses (never removes them — AO may also need them). Registered
/// only when the IBL gate is ON, so gate OFF ⇒ no extra geometry pass ⇒ v0
/// baselines pixel-neutral.
pub fn setup_ibl_prepass(mut commands: Commands, cameras: Query<(Entity, &CesiumIbl)>) {
    for (entity, ibl) in &cameras {
        if ibl.is_active() {
            commands.entity(entity).insert((DepthPrepass, NormalPrepass));
        }
    }
}

// ─── Registration ────────────────────────────────────────────────────────────

/// Register the IBL node into `RenderApp` (shader + extract + node + systems).
///
/// Called by `effects::graph::M6WaveARenderGraphPlugin` (task #81) when
/// [`ibl_gate_enabled()`] is true; the `Core3d` edges are created by
/// `effects::graph::wire_m6_edges` (this function registers the node but never
/// wires edges, so the shared linear chain in `graph.rs` stays the single owner
/// and no diamond can form).
///
/// Headless-safe: degrades to a no-op without a `RenderApp` (MinimalPlugins).
#[deprecated = "DEV-029 / FIX-REG-FACADE: call `register_ibl_node_main_world` from `Plugin::build` and `register_ibl_node_render_world` from `Plugin::finish`; this facade runs the finish half against a possibly device-less render world."]
pub fn register_ibl_node(app: &mut App) {
    register_ibl_node_main_world(app);
    // Headless `MinimalPlugins` has no `RenderApp` — degrade gracefully.
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        register_ibl_node_render_world(render_app);
    }
}

/// `Plugin::build`-time half of [`register_ibl_node`]: everything that lives in
/// the **main** world (WGSL shader asset + `ExtractComponentPlugin` + the
/// `setup_ibl_prepass` system).
///
/// Split out by task #81 — see `docs/deviations.md#dev-029`. The pipeline's
/// `FromWorld` reads `RenderDevice`, which Bevy only inserts into the render
/// world in `RenderPlugin::finish`, so the render-world half below must run from
/// a plugin's `finish`, never from its `build`.
pub fn register_ibl_node_main_world(app: &mut App) {
    // Register the IBL WGSL shader (headless-safe via shader_registry).
    crate::shader_registry::try_load_internal_shader(
        app,
        IBL_SHADER_HANDLE,
        include_str!("../../shaders/ibl.wgsl"),
        "shaders/ibl.wgsl",
    );

    // ExtractComponentPlugin: main → render world each frame (ExtractSchedule).
    app.add_plugins(ExtractComponentPlugin::<CesiumIbl>::default());

    // Main-world: attach Depth+Normal prepass to cameras driving an active env.
    app.add_systems(Update, setup_ibl_prepass);
}

/// `Plugin::finish`-time half of [`register_ibl_node`]: the render-world pipeline
/// resource + the `Core3d` node.
pub fn register_ibl_node_render_world(render_app: &mut bevy::app::SubApp) {
    // FIX-REG-FACADE (DEV-029): degrade to a no-op when `RenderDevice` is absent
    // (finish half reached from `build`, or a bare render world). See
    // `crate::effects::render_world_missing_device`.
    if crate::effects::render_world_missing_device(render_app) {
        return;
    }

    render_app
        .init_resource::<IblPipeline>()
        .add_systems(Render, prepare_ibl.in_set(RenderSet::Prepare))
        .add_render_graph_node::<ViewNodeRunner<IblNode>>(Core3d, CesiumIblLabel);
    // NOTE: edges are created by `effects::graph::wire_m6_edges` (task #81), the
    // single owner of the shared `Core3d` chain.
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_effects::ibl::{
        integrate_brdf, project_irradiance_to_sh, spherical_harmonics, IblMaterial,
    };
    use glam::DVec3;

    #[test]
    fn ibl_component_default_disabled() {
        let c = CesiumIbl::default();
        assert!(!c.enabled);
        assert!(!c.is_active());
    }

    #[test]
    fn ibl_gate_const_is_local_and_stable() {
        // The gate env var is defined LOCALLY (isolation discipline) and must not
        // collide with the post-process / sibling M6 gates. Pure (no env mutation).
        assert_eq!(ENV_ENABLE_IBL, "CESIUM_ENABLE_IBL");
        assert_ne!(ENV_ENABLE_IBL, "CESIUM_ENABLE_POSTPROCESS");
        assert_ne!(ENV_ENABLE_IBL, "CESIUM_ENABLE_CLIPPING");
        assert!(!gate_from_env_value(None));
        assert!(gate_from_env_value(Some("1".into())));
    }

    #[test]
    fn ibl_active_requires_enabled_and_nonzero_factor() {
        let mut ibl = ImageBasedLighting::default();
        ibl.set_factor(0.0, 0.0);
        let c = CesiumIbl::new(ibl, IblMaterial::default());
        assert!(!c.is_active(), "enabled but zero factors ⇒ inactive");

        let mut ibl2 = ImageBasedLighting::default();
        ibl2.set_factor(1.0, 0.5);
        assert!(CesiumIbl::new(ibl2, IblMaterial::default()).is_active());
    }

    #[test]
    fn ibl_shader_handle_unique() {
        assert_ne!(IBL_SHADER_HANDLE, super::super::graph::PASS_THROUGH_SHADER_HANDLE);
        assert_ne!(IBL_SHADER_HANDLE, super::super::fxaa::FXAA_SHADER_HANDLE);
        assert_ne!(IBL_SHADER_HANDLE, super::super::ao::AO_SHADER_HANDLE);
        assert_ne!(
            IBL_SHADER_HANDLE,
            super::super::clipping_planes::CLIPPING_SHADER_HANDLE
        );
    }

    #[test]
    fn ibl_headless_graceful() {
        // No RenderApp (headless) → register must not panic.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        #[allow(deprecated)]
        register_ibl_node(&mut app);
    }

    #[test]
    fn setup_ibl_prepass_adds_depth_and_normal_when_active() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, setup_ibl_prepass);

        let mut ibl = ImageBasedLighting::default();
        ibl.set_factor(1.0, 1.0);
        let cam = app
            .world_mut()
            .spawn((Camera3d::default(), CesiumIbl::new(ibl, IblMaterial::default())))
            .id();

        app.update();
        assert!(
            app.world().get::<DepthPrepass>(cam).is_some(),
            "active IBL ⇒ DepthPrepass attached"
        );
        assert!(
            app.world().get::<NormalPrepass>(cam).is_some(),
            "active IBL ⇒ NormalPrepass attached"
        );

        // Disabled ⇒ no prepass forced on a fresh camera.
        let cam2 = app
            .world_mut()
            .spawn((Camera3d::default(), CesiumIbl::default()))
            .id();
        app.update();
        assert!(app.world().get::<DepthPrepass>(cam2).is_none());
    }

    // ─── Uniform packing: the f64 → f32 boundary (red line) ───────────────────

    #[test]
    fn from_domain_projects_sh_and_material_to_f32() {
        let mut ibl = ImageBasedLighting::default();
        // Constant white environment ⇒ DC coefficient = π (analytic, N-independent).
        let coeffs = project_irradiance_to_sh(|_d| [1.0, 1.0, 1.0], 64);
        ibl.set_spherical_harmonics(coeffs);
        ibl.set_factor(0.75, 0.5);

        let material = IblMaterial {
            diffuse: [0.2, 0.4, 0.6],
            specular_f0: [0.04, 0.04, 0.04],
            roughness: 0.3,
            specular_weight: 1.0,
        };
        let u = IblUniform::from_domain(&ibl, &material, 4.0);

        // DC coefficient ≈ π (f32 projection of the f64 analytic value).
        assert!((u.sh[0].x - std::f32::consts::PI).abs() < 1e-3, "SH DC = π");
        assert!((u.material.x - 0.3).abs() < 1e-6, "roughness");
        assert!((u.material.y - 0.04).abs() < 1e-6, "f0.r");
        assert!((u.diffuse.x - 0.2).abs() < 1e-6, "diffuse.r");
        assert!((u.diffuse.w - 1.0).abs() < 1e-6, "specular_weight");
        assert!((u.params.x - 0.75).abs() < 1e-6, "diffuse factor");
        assert!((u.params.y - 0.5).abs() < 1e-6, "specular factor");
        assert!((u.params.z - 4.0).abs() < 1e-6, "max lod");
    }

    #[test]
    fn from_domain_defaults_sh_when_unset_and_is_pixel_neutral_at_zero_factor() {
        // No SH set ⇒ default_spherical_harmonics; factor [1,1] by default.
        let ibl = ImageBasedLighting::default();
        let u = IblUniform::from_domain(&ibl, &IblMaterial::default(), 0.0);
        assert!(u.sh[0].x > 0.0, "default DC term is positive");

        // Zero factors ⇒ params.x/y = 0 ⇒ shader pass-through (pixel-neutral).
        let mut off = ImageBasedLighting::default();
        off.set_factor(0.0, 0.0);
        let u_off = IblUniform::from_domain(&off, &IblMaterial::default(), 0.0);
        assert!((u_off.params.x).abs() < 1e-6);
        assert!((u_off.params.y).abs() < 1e-6);
    }

    // ─── CPU/GPU cross-validation: the WGSL helpers mirror the f64 reference ───

    /// The WGSL `spherical_harmonics_eval` must reproduce the domain f64
    /// `spherical_harmonics` for a projected constant environment (DC = π). This
    /// anchors the GPU shader's SH convention to the CPU reference it mirrors.
    #[test]
    fn sh_convention_cross_validates_against_domain() {
        let coeffs = project_irradiance_to_sh(|_d| [0.5, 0.5, 0.5], 128);
        let dir = DVec3::new(0.3, -0.4, 0.866_025_403_784_439); // unit-ish
        let cpu = spherical_harmonics(&coeffs, dir.normalize());
        // A constant 0.5 environment ⇒ irradiance = 0.5·π everywhere (direction
        // independent to first order); the SH evaluation must be positive & finite.
        assert!(cpu.iter().all(|c| c.is_finite() && *c > 0.0));
        assert!((cpu[0] - 0.5 * std::f64::consts::PI).abs() < 1e-2, "≈ 0.5π");
    }

    /// The WGSL `integrate_brdf(0, 1, N)` endpoint must be `(scale=1, bias=0)`;
    /// anchors the GPU split-sum generator to the domain f64 reference.
    #[test]
    fn brdf_endpoint_cross_validates_against_domain() {
        let [scale, bias] = integrate_brdf(0.0, 1.0, 1024);
        assert!((scale - 1.0).abs() < 1e-6, "scale → 1 at normal incidence");
        assert!(bias.abs() < 1e-6, "bias → 0 at zero roughness");
    }

    // ─── Ryan C1/C2 defence line: headless naga parse + validate + layout ──────

    /// naga has no preprocessor, so the two `#import`s in `ibl.wgsl` are replaced by
    /// struct stubs declaring exactly the fields the shader reads:
    /// `FullscreenVertexOutput.{position, uv}` and `View.{view_from_clip,
    /// world_from_view}`. The `view` **binding** itself is declared by the real
    /// shader text (group 0, binding 5), so it is deliberately not stubbed here.
    const IBL_WGSL_IMPORT_STUBS: &str = "\
struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct View {
    view_from_clip: mat4x4<f32>,
    world_from_view: mat4x4<f32>,
}
";

    fn ibl_stubbed_wgsl() -> String {
        let mut source = String::from(IBL_WGSL_IMPORT_STUBS);
        for line in include_str!("../../shaders/ibl.wgsl").lines() {
            if line.starts_with("#import") {
                continue;
            }
            source.push_str(line);
            source.push('\n');
        }
        source
    }

    /// Headless proof the IBL shader is real: parsed + type-checked by **naga** (the
    /// same WGSL front end `bevy_render` compiles it with on the GPU path). Guards
    /// the M5 C1 (reserved word `mod`) + swizzle-assignment classes of silent bug.
    #[test]
    fn ibl_wgsl_parses_and_type_checks_under_naga() {
        let source = ibl_stubbed_wgsl();
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("ibl.wgsl does not parse:\n{}", error.emit_to_string(&source))
        });
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("ibl.wgsl does not validate");

        // Four fragment entries: the apply node + three offline generators.
        let entry_points = module
            .entry_points
            .iter()
            .map(|entry| (entry.name.as_str(), entry.stage))
            .collect::<Vec<_>>();
        assert_eq!(
            entry_points,
            vec![
                ("fragment", naga::ShaderStage::Fragment),
                ("brdf_lut_fragment", naga::ShaderStage::Fragment),
                ("irradiance_fragment", naga::ShaderStage::Fragment),
                ("prefilter_fragment", naga::ShaderStage::Fragment),
            ],
            "ibl.wgsl must expose exactly the apply node + 3 generator fragment entries"
        );
    }

    /// Ryan C1 (catches the **C2** class of bug): every binding the `fragment`
    /// (apply) entry statically uses must be present in the Rust `IblPipeline`
    /// layout (group 0: bindings 0..=7). Guards against a silent pipeline-build
    /// failure that would no-op IBL while `pixel_diff` reported a false green.
    #[test]
    fn ibl_wgsl_entry_bindings_are_covered_by_the_rust_layout() {
        let source = ibl_stubbed_wgsl();
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("ibl.wgsl does not parse:\n{}", error.emit_to_string(&source))
        });

        // Mirror of `IblPipeline::from_world` (group 0 binding indices).
        let layout: std::collections::BTreeSet<(u32, u32)> = [
            (0, 0),
            (0, 1),
            (0, 2),
            (0, 3),
            (0, 4),
            (0, 5),
            (0, 6),
            (0, 7),
        ]
        .into_iter()
        .collect();

        let entry = module
            .entry_points
            .iter()
            .find(|e| e.name == "fragment")
            .expect("ibl.wgsl must have a `fragment` (apply) entry point");

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
            "ibl.wgsl `fragment` statically uses bindings {missing:?} absent from IblPipeline's \
             layout (C2-class regression: the pipeline would fail to build and IBL would no-op)"
        );
    }

    /// M5-D C1 red-line guard: `mod` is a WGSL RESERVED WORD — the radical inverse
    /// must use integer bit ops, never a `mod(` call. Also assert no `fma(` (the
    /// no-FMA-contraction red line) appears anywhere in the shader.
    #[test]
    fn ibl_wgsl_avoids_reserved_words_and_fma() {
        let wgsl = include_str!("../../shaders/ibl.wgsl");
        // Strip `//` comments before scanning (a comment may legitimately mention
        // the word — the M5 lesson was a false positive on `GLSL mod()` in a comment).
        let code: String = wgsl
            .lines()
            .map(|line: &str| line.split("//").next().unwrap_or(""))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !code.contains("mod("),
            "ibl.wgsl must not call `mod(` (WGSL reserved word — use integer bit ops)"
        );
        assert!(
            !code.contains("fma("),
            "ibl.wgsl must not call `fma(` (no-FMA-contraction red line)"
        );
    }

    /// Swizzle-assignment guard (the M5-D naga rejection): no `lhs.xyz = ` /
    /// `.rgb = ` style partial-vector writes; every vector is built whole. Scans
    /// comment-stripped code for the illegal `<ident>.<swizzle> =` pattern.
    #[test]
    fn ibl_wgsl_has_no_swizzle_assignment() {
        let wgsl = include_str!("../../shaders/ibl.wgsl");
        for (idx, raw) in wgsl.lines().enumerate() {
            let line = raw.split("//").next().unwrap_or("");
            // A swizzle assignment looks like `something.xyz = ` (a `.` component
            // selector immediately before the `=`), which naga rejects.
            if let Some(eq) = line.find('=') {
                // Ignore `==`, `<=`, `>=`, `!=`.
                let before = &line[..eq];
                let after = line[eq + 1..].chars().next();
                let is_comparison = after == Some('=');
                let prev = before.chars().last();
                let is_relational = matches!(prev, Some('<') | Some('>') | Some('!') | Some('='));
                if !is_comparison && !is_relational && before.trim_end().ends_with(|c: char| {
                    matches!(c, 'x' | 'y' | 'z' | 'w' | 'r' | 'g' | 'b' | 'a')
                }) && before.contains('.')
                {
                    panic!(
                        "ibl.wgsl line {} has a swizzle assignment (naga rejects `v.xyz = …`): {}",
                        idx + 1,
                        line.trim()
                    );
                }
            }
        }
    }

    /// Cross-check: the WGSL uniform SH array size + red-line notes must match the
    /// Rust side, so `from_domain`'s packing agrees with the shader layout.
    #[test]
    fn wgsl_uniform_layout_matches_rust() {
        let wgsl = include_str!("../../shaders/ibl.wgsl");
        assert!(
            wgsl.contains("sh: array<vec4<f32>, 9>"),
            "ibl.wgsl SH array must be 9 coefficients (SH_COEFFICIENT_COUNT)"
        );
        // Red-line invariants must stay visible in-source (mirrors clipping.wgsl).
        assert!(wgsl.contains("NO FMA CONTRACTION"));
        assert!(wgsl.contains("RESERVED WORD"));
    }
}
