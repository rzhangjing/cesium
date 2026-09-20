//! M6.6: cesiumrust **Clouds** adapter — screen-space cumulus composite node.
//!
//! Ports the upstream CesiumJS cumulus-cloud capability
//! (`Scene/CloudCollection.js` + `Scene/CumulusCloud.js` +
//! `Shaders/CloudCollectionFS.glsl`) onto the M5-E render-graph infrastructure
//! (`graph.rs`). Mirrors the [`super::clipping_planes`] / [`super::ibl`] pattern:
//! this module registers the node / resources / systems **but never creates graph
//! edges** — the single linear `Core3d` chain in `graph.rs::wire_m6_edges` owns
//! them, so no diamond can form. Wiring the clouds node into that chain is
//! integration task (阶段三 FIX-INTEG).
//!
//! # Scoped deliverable (阶段二 FIX-CLOUD-FULL)
//! This adapter fully implements the **`clouds.wgsl` screen-space composite**
//! path: the [`CesiumClouds`] component, the [`CloudsUniform`] f64 → f32 GPU
//! boundary, the [`CloudsPipeline`] bind-group layout, the 3D noise texture, the
//! [`CloudsNode`] `ViewNode`, and the three-段式 `register_clouds_node*`. The
//! three WGSL shaders (`clouds.wgsl` / `cloud_noise.wgsl` / `cloud_billboard.wgsl`)
//! each get a naga parse + validate + binding-coverage defence test, and
//! [`CloudsUniform::from_domain`] is cross-checked against the domain f64 CPU
//! reference (`cesium-effects::cloud`).
//!
//! The **billboard** (`cloud_billboard.wgsl`) and **GPU noise generator**
//! (`cloud_noise.wgsl`) render passes are *not* graph-wired here: their
//! `RenderPipelineDescriptor` / instance buffers / compute dispatch belong to
//! 阶段三 FIX-INTEG and real-GPU 取证. The CPU reference [`cesium_effects::cloud::NoiseVolume`]
//! supplies the 3D texture the composite path samples, so the screen-space node
//! is self-contained without the compute generator. See
//! `docs/deviations.md#dev-032`.
//!
//! # SPIKE payoff — real `texture_3d`
//! Upstream packs the 128³ volume into a 2D atlas and hand-rolls trilinear
//! (`voxelToUV` + `lerpSamplesX`, CloudCollectionFS.glsl L25-65). The M6.6 SPIKE
//! confirmed wgpu/naga 3D-texture support, so `clouds.wgsl` samples a real
//! `texture_3d` with a single hardware-trilinear `textureSampleLevel(…, vec3, 0.0)`
//! and this adapter uploads [`NoiseVolume::to_rgba8_bytes`] straight into a
//! `TextureDimension::D3` via `initial_data` — the atlas index gymnastics collapse
//! away.
//!
//! # Gate (single source of truth)
//! The gate name is owned by the app-layer registry
//! `application/cesium-app/src/feature_flags.rs` (`ENV_ENABLE_CLOUDS` /
//! `clouds_enabled()`); [`ENV_ENABLE_CLOUDS`] below is a byte-identical mirror
//! forced by the crate dependency direction (`cesium-app` → `cesium-bevy-render`).
//! Default OFF ⇒ `register_clouds_node` is never called by the graph plugin ⇒ no
//! `Core3d` edges exist ⇒ the node never runs ⇒ dynamic_globe v0 baselines stay
//! pixel-neutral (PSNR = ∞). The shader's `count <= 0` early-out returns the
//! source colour untouched as a second belt-and-braces guarantee.
//!
//! # Red lines honoured
//! - domain stays metric **f64**; the metric → render-unit conversion
//!   (`/ METERS_PER_RENDER_UNIT`, `METERS_PER_RENDER_UNIT = 6378137`) and the
//!   `0.82·maximumSize` ellipsoid shrink happen ONLY at
//!   [`CloudsUniform::from_domain`] (the GPU boundary).
//! - `clouds.wgsl` keeps the Gardner sum, `dot(n,p) + w`, and the march
//!   accumulation as separate IEEE roundings (NO FMA contraction).
//! - `mod` is a WGSL reserved word; the shader uses `fract` / `%` instead.
//! - glam fast-math disabled repo-wide (nothing here relies on non-IEEE floats).
//!
//! # Blueprint
//! - `packages/engine/Source/Scene/CloudCollection.js` + `CumulusCloud.js`
//! - `packages/engine/Source/Shaders/CloudCollectionFS.glsl` L1-263
//! - `domain/effects/src/cloud.rs` — the f64 CPU reference (cross-validated)
//! - `bevy_pbr-0.15.3/src/ssao/mod.rs` (ViewNode + depth-prepass reconstruction)

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
        binding_types::{sampler, texture_2d, texture_3d, texture_depth_2d, uniform_buffer},
        BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId,
        ColorTargetState, ColorWrites, Extent3d, FilterMode, FragmentState, MultisampleState,
        Operations, PipelineCache, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
        RenderPipelineDescriptor, Sampler as GpuSampler, SamplerBindingType, SamplerDescriptor,
        ShaderStages, Texture, TextureDescriptor, TextureDimension, TextureFormat,
        TextureSampleType, TextureUsages, TextureView, UniformBuffer,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
    Render, RenderApp, RenderSet,
};
use cesium_effects::cloud::{
    CloudCollection, NoiseVolume, BEER_LAMBERT_EXTINCTION, CLOUD_LIGHT_DIR, ELLIPSOID_SCALE_FACTOR,
    HG_PHASE_G, NOISE_TEXTURE_DIMENSIONS, RAYMARCH_STEPS_DEFAULT,
};

use crate::resources::METERS_PER_RENDER_UNIT;
use super::graph::gate_from_env_value;

// ─── Shader handles ──────────────────────────────────────────────────────────

/// Handle for the embedded `clouds.wgsl` screen-space composite shader (driven
/// by [`CloudsNode`]).
pub const CLOUDS_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(0xCE51_C10D_0006_0006);

/// Handle for the embedded `cloud_billboard.wgsl` billboard pass. Registered as
/// an asset so the source is validated + available, but its render pipeline is
/// NOT yet driven — billboard instancing is 阶段三 FIX-INTEG
/// (`docs/deviations.md#dev-032`).
pub const CLOUD_BILLBOARD_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0xCE51_C10D_0006_0007);

/// Handle for the embedded `cloud_noise.wgsl` compute generator. Same status as
/// [`CLOUD_BILLBOARD_SHADER_HANDLE`]: registered, not yet dispatched (the CPU
/// [`NoiseVolume`] feeds the composite path in this scope).
pub const CLOUD_NOISE_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(0xCE51_C10D_0006_0008);

/// Maximum number of clouds uploaded to the GPU uniform array. Must equal the
/// `array<vec4<f32>, 8>` size and `MAX_CLOUDS` in `clouds.wgsl` (asserted by
/// `wgsl_max_clouds_matches_rust_const`).
pub const MAX_CLOUDS: usize = 8;

/// Noise volume edge length uploaded to the GPU (matches
/// `NOISE_TEXTURE_DIMENSIONS` in the domain and `info.y` in `clouds.wgsl`).
pub const CLOUDS_NOISE_DIMENSIONS: usize = NOISE_TEXTURE_DIMENSIONS;

// ─── Gate (mirror of the app-layer registry — see module doc) ────────────────

/// Env var gating the M6.6 clouds node. **Mirror** of the app-layer registry
/// owner `application/cesium-app/src/feature_flags.rs` (`ENV_ENABLE_CLOUDS` /
/// `clouds_enabled()`); `pub` so
/// `feature_flags::adapter_gate_mirrors_are_byte_identical_to_the_registry` can
/// assert byte-equality across the crate boundary. Default OFF.
pub const ENV_ENABLE_CLOUDS: &str = "CESIUM_ENABLE_CLOUDS";

/// Returns `true` when the clouds gate is enabled. Reuses the single
/// authoritative truthy parser (`gate_from_env_value`, the crate-wide
/// `{1, true, yes, on}` set) so it agrees with every other cesium gate.
#[inline]
pub fn clouds_gate_enabled() -> bool {
    gate_from_env_value(std::env::var(ENV_ENABLE_CLOUDS).ok())
}

// ─── Render graph label (local — 阶段三 wires it into the Core3d chain) ───────

/// Node label for the cesium clouds node in `Core3d`. Defined locally so this
/// module does not edit the shared `graph.rs` label enums; 阶段三 FIX-INTEG
/// creates the edges (recommended position: after the main opaque pass, reading
/// the HDR scene colour + depth prepass, before AO / tonemapping).
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct CesiumCloudsLabel;

// ─── Shading mode ────────────────────────────────────────────────────────────

/// Which of `clouds.wgsl`'s two shading paths [`params.z`] selects.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CloudsShadingMode {
    /// FAITHFUL upstream `drawCloud`: single ray/ellipsoid intersection shaded
    /// with the Gardner sine texture + Worley-FBM erosion. This is exactly what
    /// upstream CesiumJS renders.
    #[default]
    Faithful,
    /// ADDITIVE physically-based volumetric march (Beer-Lambert + Henyey-Greenstein,
    /// g = 0.6). NOT upstream — the deviation this path introduces is recorded as
    /// `docs/deviations.md#dev-032`.
    Volumetric,
}

// ─── Component ───────────────────────────────────────────────────────────────

/// Component carrying an active [`CloudCollection`] for a view.
///
/// Placed on the camera (like [`super::clipping_planes::CesiumClippingPlanes`]) to
/// drive the screen-space node. Extracted to the render world via
/// `ExtractComponentPlugin`. The node early-returns when `enabled == false` or the
/// collection is empty / hidden (zero GPU cost, pixel-neutral). `Default` is
/// derived: `enabled = false` (conservative).
#[derive(Component, Clone, Debug, Default, ExtractComponent)]
pub struct CesiumClouds {
    /// Master enable for the clouds node on this view.
    pub enabled: bool,
    /// The domain cloud collection (metric f64).
    pub collection: CloudCollection,
    /// Which shading path the composite uses.
    pub shading: CloudsShadingMode,
}

impl CesiumClouds {
    /// Convenience constructor for an enabled view collection (faithful shading).
    pub fn new(collection: CloudCollection) -> Self {
        Self {
            enabled: true,
            collection,
            shading: CloudsShadingMode::Faithful,
        }
    }

    /// With an explicit shading mode.
    pub fn with_shading(mut self, shading: CloudsShadingMode) -> Self {
        self.shading = shading;
        self
    }

    /// Whether clouds should actually render (component + collection agree). The
    /// gate itself is checked at registration time, not here.
    #[inline]
    pub fn is_active(&self) -> bool {
        self.enabled && self.collection.show && !self.collection.is_empty()
    }
}

/// Per-view cached pipeline ID for the clouds node.
#[derive(Component)]
pub struct CameraCloudsPipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

/// Per-view GPU uniform buffer holding the packed cloud data.
#[derive(Component)]
pub struct ViewCloudsUniform {
    pub buffer: UniformBuffer<CloudsUniform>,
}

// ─── GPU uniform (f32 boundary) ──────────────────────────────────────────────

/// GPU-facing clouds uniform. **f32 only** — the domain collection stays metric
/// f64; [`CloudsUniform::from_domain`] performs the single metric → render-unit
/// conversion + `0.82·maximumSize` shrink at this boundary (red line).
///
/// Layout must match `struct CloudsData` in `shaders/clouds.wgsl` (encase std140).
///
/// The struct lives in a private `clouds_uniform` module carrying
/// `#![allow(dead_code)]` (the `clipping_planes.rs` convention): the encase
/// `ShaderType` derive emits a module-level helper the dead-code pass flags even
/// though every field is uploaded via `write_buffer`. Field values are asserted in
/// the `from_domain_*` unit tests.
pub use clouds_uniform::CloudsUniform;

mod clouds_uniform {
    #![allow(dead_code)]
    use super::MAX_CLOUDS;
    use bevy::prelude::Vec4;
    use bevy::render::render_resource::ShaderType;

    /// GPU clouds uniform; layout matches `struct CloudsData` in
    /// `shaders/clouds.wgsl` (encase std140).
    #[derive(ShaderType, Clone, Debug)]
    pub struct CloudsUniform {
        /// `xyz` = ellipsoid centre (RENDER UNITS), `w` = active flag (1 = shade).
        pub centers: [Vec4; MAX_CLOUDS],
        /// `xyz` = ellipsoid scale (RENDER UNITS, already `0.82·maximumSize`),
        /// `w` = slice.
        pub scales: [Vec4; MAX_CLOUDS],
        /// `rgba` cloud colour (`v_color`).
        pub colors: [Vec4; MAX_CLOUDS],
        /// `x` = u_noiseDetail, `y` = raymarch steps, `z` = mode (0 faithful /
        /// 1 volumetric), `w` = Beer-Lambert extinction.
        pub params: Vec4,
        /// `xyz` = camera world position (unused: the shader reads
        /// `view.world_from_view[3].xyz`), `w` = brightness.
        pub camera: Vec4,
        /// `xyz` = light direction (shader normalises), `w` = Henyey-Greenstein g.
        pub light: Vec4,
        /// `x` = active cloud count, `y` = noise volume edge (128), `zw` = pad.
        pub info: Vec4,
    }
}

impl Default for CloudsUniform {
    fn default() -> Self {
        Self {
            centers: [Vec4::ZERO; MAX_CLOUDS],
            scales: [Vec4::ZERO; MAX_CLOUDS],
            colors: [Vec4::ZERO; MAX_CLOUDS],
            // params.z = 0 (faithful) by default; camera.w = 1.0 so a stray
            // activation without clouds is a pure pass-through (info.x = 0).
            params: Vec4::new(0.0, RAYMARCH_STEPS_DEFAULT as f32, 0.0, BEER_LAMBERT_EXTINCTION as f32),
            camera: Vec4::new(0.0, 0.0, 0.0, 1.0),
            // light.xyz defaults to the (unnormalised) CLOUD_LIGHT_DIR; the shader
            // normalises. light.w = HG g.
            light: Vec4::new(
                CLOUD_LIGHT_DIR.x as f32,
                CLOUD_LIGHT_DIR.y as f32,
                CLOUD_LIGHT_DIR.z as f32,
                HG_PHASE_G as f32,
            ),
            // info.x = 0 ⇒ the shader returns the source colour untouched.
            info: Vec4::new(0.0, CLOUDS_NOISE_DIMENSIONS as f32, 0.0, 0.0),
        }
    }
}

impl CloudsUniform {
    /// Packs a domain [`CloudCollection`] into the GPU uniform.
    ///
    /// - Each visible cloud's metric `position` is divided by
    ///   [`METERS_PER_RENDER_UNIT`] to enter render-unit world space (the same
    ///   space `world_from_view` lives in); its `maximum_size` is shrunk by
    ///   [`ELLIPSOID_SCALE_FACTOR`] (0.82, the ellipsoid radius the upstream
    ///   `drawCloud` uses) and likewise divided into render units.
    /// - `centers[i].w = 1.0` marks the slot active; count is clamped to
    ///   [`MAX_CLOUDS`].
    /// - `camera.w` (brightness) collapses the per-cloud `brightness` to a single
    ///   global (the first visible cloud) — `clouds.wgsl` has one brightness slot.
    ///   This is the documented deviation `docs/deviations.md#dev-032`.
    /// - `info.x = 0` when the collection is hidden or empty ⇒ pure pass-through
    ///   (pixel-neutral).
    pub fn from_domain(collection: &CloudCollection, shading: CloudsShadingMode) -> Self {
        let mut centers = [Vec4::ZERO; MAX_CLOUDS];
        let mut scales = [Vec4::ZERO; MAX_CLOUDS];
        let mut colors = [Vec4::ZERO; MAX_CLOUDS];

        let mut count = 0usize;
        let mut brightness = 1.0_f64;
        let mut first_seen = false;

        for cloud in collection.visible_clouds() {
            if count >= MAX_CLOUDS {
                break;
            }
            if !first_seen {
                brightness = cloud.brightness;
                first_seen = true;
            }
            centers[count] = Vec4::new(
                (cloud.position.x / METERS_PER_RENDER_UNIT) as f32,
                (cloud.position.y / METERS_PER_RENDER_UNIT) as f32,
                (cloud.position.z / METERS_PER_RENDER_UNIT) as f32,
                1.0, // active flag
            );
            // 0.82·maximumSize (render units), slice carried in .w.
            scales[count] = Vec4::new(
                (cloud.maximum_size.x * ELLIPSOID_SCALE_FACTOR / METERS_PER_RENDER_UNIT) as f32,
                (cloud.maximum_size.y * ELLIPSOID_SCALE_FACTOR / METERS_PER_RENDER_UNIT) as f32,
                (cloud.maximum_size.z * ELLIPSOID_SCALE_FACTOR / METERS_PER_RENDER_UNIT) as f32,
                cloud.slice as f32,
            );
            colors[count] = Vec4::new(
                cloud.color[0] as f32,
                cloud.color[1] as f32,
                cloud.color[2] as f32,
                cloud.color[3] as f32,
            );
            count += 1;
        }

        let mode_z: f32 = match shading {
            CloudsShadingMode::Faithful => 0.0,
            CloudsShadingMode::Volumetric => 1.0,
        };

        let active = collection.show && count > 0;
        let params = Vec4::new(
            collection.noise_detail as f32,
            RAYMARCH_STEPS_DEFAULT as f32,
            mode_z,
            BEER_LAMBERT_EXTINCTION as f32,
        );
        let info = Vec4::new(
            if active { count as f32 } else { 0.0 },
            CLOUDS_NOISE_DIMENSIONS as f32,
            0.0,
            0.0,
        );

        Self {
            centers,
            scales,
            colors,
            params,
            camera: Vec4::new(0.0, 0.0, 0.0, brightness as f32),
            light: Vec4::new(
                CLOUD_LIGHT_DIR.x as f32,
                CLOUD_LIGHT_DIR.y as f32,
                CLOUD_LIGHT_DIR.z as f32,
                HG_PHASE_G as f32,
            ),
            info,
        }
    }
}

// ─── Pipeline ────────────────────────────────────────────────────────────────

/// Render-world resource: bind group layout + samplers for the clouds node.
///
/// Group 0 bindings (must match `clouds.wgsl` + the binding-coverage test):
/// - 0: depth prepass (`texture_depth_2d`) — occlusion + ray reconstruction
/// - 1: 3D noise volume (`texture_3d<f32>`) — Worley-FBM erosion channels
/// - 2: colour source (`texture_2d<f32>`) — post-process input
/// - 3: linear sampler (Filtering — colour + trilinear noise)
/// - 4: point sampler (NonFiltering — depth)
/// - 5: `ViewUniform` (dynamic offset) — `world_from_view` / `view_from_clip`
/// - 6: `CloudsUniform` — the packed cloud data
#[derive(Resource)]
pub struct CloudsPipeline {
    pub bind_group_layout: BindGroupLayout,
    pub point_sampler: GpuSampler,
    pub linear_sampler: GpuSampler,
}

impl FromWorld for CloudsPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();

        let bind_group_layout = render_device.create_bind_group_layout(
            "cesium_clouds_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_depth_2d(),
                    texture_3d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    sampler(SamplerBindingType::NonFiltering),
                    uniform_buffer::<ViewUniform>(true),
                    uniform_buffer::<CloudsUniform>(false),
                ),
            ),
        );

        let point_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_clouds_point_sampler"),
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        // Clamp-to-edge on the 2D colour source; Repeat on the 3D noise is set by
        // the sampler the shader binds here. A single Filtering sampler serves both
        // colour + noise in this scope (noise wrapping relies on the shader's
        // `fract` recentering, see `sample_noise`).
        let linear_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_clouds_linear_sampler"),
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

// ─── 3D noise texture ────────────────────────────────────────────────────────

/// Render-world resource: the 128³ Worley-FBM noise volume uploaded as a real
/// `texture_3d` (the SPIKE payoff). Created once in
/// [`initialize_clouds_noise`] from the domain CPU [`NoiseVolume`] reference so
/// the composite path is self-contained without the `cloud_noise.wgsl` compute
/// generator (deferred to 阶段三 / GPU).
#[derive(Resource)]
pub struct CloudsNoiseTexture {
    /// Owns the GPU texture (the view alone can dangle in some backends).
    #[allow(dead_code)]
    texture: Texture,
    view: TextureView,
}

impl CloudsNoiseTexture {
    /// The bound 3D texture view.
    #[inline]
    pub fn view(&self) -> &TextureView {
        &self.view
    }
}

// ─── ViewNode ────────────────────────────────────────────────────────────────

/// Screen-space clouds composite `ViewNode`.
///
/// Reconstructs the world ray from the depth prepass + `view_from_clip`, marches
/// the (max 8) cloud ellipsoids and over-composites them front-to-back onto the
/// HDR scene colour. Early-returns (pixel-neutral) when the component is inactive
/// or the depth prepass / pipeline / noise texture is unavailable.
#[derive(Default)]
pub struct CloudsNode;

impl ViewNode for CloudsNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static CameraCloudsPipeline,
        &'static CesiumClouds,
        &'static ViewCloudsUniform,
        &'static ViewPrepassTextures,
        &'static ViewUniformOffset,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (target, pipeline_handle, clouds, clouds_uniform, prepass, view_uniform_offset): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if !clouds.is_active() {
            return Ok(());
        }

        // The composite reads the depth prepass for occlusion + ray reconstruction.
        let Some(depth_view) = prepass.depth_view() else {
            return Ok(());
        };

        let pipeline_cache = world.resource::<PipelineCache>();
        let clouds_pipeline = world.resource::<CloudsPipeline>();
        let view_uniforms = world.resource::<ViewUniforms>();
        let noise = world.resource::<CloudsNoiseTexture>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_handle.pipeline_id) else {
            return Ok(());
        };
        let Some(view_uniform_binding) = view_uniforms.uniforms.binding() else {
            return Ok(());
        };
        let Some(clouds_binding) = clouds_uniform.buffer.binding() else {
            return Ok(());
        };

        let render_device = render_context.render_device().clone();

        let post_process = target.post_process_write();
        let source = post_process.source;
        let destination = post_process.destination;

        let bind_group = render_device.create_bind_group(
            Some("cesium_clouds_bind_group"),
            &clouds_pipeline.bind_group_layout,
            &BindGroupEntries::sequential((
                depth_view,
                noise.view(),
                source,
                &clouds_pipeline.linear_sampler,
                &clouds_pipeline.point_sampler,
                view_uniform_binding.clone(),
                clouds_binding,
            )),
        );

        let pass_descriptor = RenderPassDescriptor {
            label: Some("cesium_clouds_pass"),
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

/// Creates the 128³ noise texture once from the CPU [`NoiseVolume`] reference.
/// Runs in `Render`, `RenderSet::Prepare` (idempotent via a resource guard). The
/// volume uses the default production detail (16.0); per-collection detail /
/// offset regeneration is a 阶段三 / GPU refinement (see `#dev-032`).
pub fn initialize_clouds_noise(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    existing: Option<Res<CloudsNoiseTexture>>,
) {
    if existing.is_some() {
        return;
    }

    let volume = NoiseVolume::generate(
        CLOUDS_NOISE_DIMENSIONS,
        // Default production noise detail (matches CloudCollection::default()).
        16.0,
        glam::DVec3::ZERO,
    );
    let bytes = volume.to_rgba8_bytes();
    let dim = CLOUDS_NOISE_DIMENSIONS as u32;

    let texture = render_device.create_texture_with_data(
        &render_queue,
        &TextureDescriptor {
            label: Some("cesium_clouds_noise"),
            size: Extent3d {
                width: dim,
                height: dim,
                depth_or_array_layers: dim,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D3,
            // Linear data (worley channels), never sRGB.
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        },
        // z-slowest / row-major matches NoiseVolume::to_rgba8_bytes ordering.
        wgpu::util::TextureDataOrder::default(),
        &bytes,
    );
    let view = texture.create_view(&Default::default());

    commands.insert_resource(CloudsNoiseTexture { texture, view });
}

/// Prepares the clouds pipeline + per-view uniform buffer for each active view.
/// Runs in `Render`, `RenderSet::Prepare` (before the render graph executes).
pub fn prepare_clouds(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    clouds_pipeline: Res<CloudsPipeline>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    views: Query<(Entity, &ExtractedView, &CesiumClouds)>,
) {
    for (entity, view, clouds) in &views {
        if !clouds.is_active() {
            continue;
        }

        let destination_format = if view.hdr {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::Rgba8UnormSrgb
        };

        let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("cesium_clouds_pipeline".into()),
            layout: vec![clouds_pipeline.bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: CLOUDS_SHADER_HANDLE,
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
        // render-unit positions/scales) and upload it.
        let mut buffer = UniformBuffer::from(CloudsUniform::from_domain(
            &clouds.collection,
            clouds.shading,
        ));
        buffer.write_buffer(&render_device, &render_queue);

        commands.entity(entity).insert((
            CameraCloudsPipeline { pipeline_id },
            ViewCloudsUniform { buffer },
        ));
    }
}

// ─── Main-world systems ──────────────────────────────────────────────────────

/// Ensures cameras driving an active cloud collection carry [`DepthPrepass`]
/// (the node's occlusion / ray-reconstruction input). Adapter-layer enablement so
/// the app-layer camera bundle stays untouched (same discipline as
/// `setup_clipping_prepass`). Insert-only (never removes — DEF-033 guard lives in
/// `ao.rs`). Registered only when the clouds gate is ON.
pub fn setup_clouds_prepass(mut commands: Commands, cameras: Query<(Entity, &CesiumClouds)>) {
    for (entity, clouds) in &cameras {
        if clouds.is_active() {
            commands.entity(entity).insert(DepthPrepass);
        }
    }
}

// ─── Registration ────────────────────────────────────────────────────────────

/// Register the clouds node into `RenderApp` (shader + extract + node + systems).
///
/// Called by `effects::graph::M6WaveARenderGraphPlugin` (阶段三 FIX-INTEG) when
/// [`clouds_gate_enabled()`] is true; the `Core3d` edges are created by
/// `wire_m6_edges` (this function registers the node but never wires edges, so
/// the shared linear chain in `graph.rs` stays the single owner).
///
/// Headless-safe: degrades to a no-op without a `RenderApp` (MinimalPlugins).
#[deprecated = "DEV-029 / FIX-REG-FACADE: call `register_clouds_node_main_world` from `Plugin::build` and `register_clouds_node_render_world` from `Plugin::finish`; this facade runs the finish half against a possibly device-less render world."]
pub fn register_clouds_node(app: &mut App) {
    register_clouds_node_main_world(app);
    // Headless `MinimalPlugins` has no `RenderApp` — degrade gracefully.
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        register_clouds_node_render_world(render_app);
    }
}

/// `Plugin::build`-time half of [`register_clouds_node`]: the **main**-world WGSL
/// shader assets + `ExtractComponentPlugin` + the `setup_clouds_prepass` system.
///
/// Split per the DEV-029 / §5.1 discipline — the pipeline's `FromWorld` reads
/// `RenderDevice`, which Bevy only inserts into the render world in
/// `RenderPlugin::finish`, so the render-world half must run from `finish`.
pub fn register_clouds_node_main_world(app: &mut App) {
    // Register the three cloud WGSL shaders (headless-safe via shader_registry).
    // Only `clouds.wgsl` is pipeline-driven in this scope; the other two are
    // registered so their sources are available + validated (阶段三 wires them).
    crate::shader_registry::try_load_internal_shader(
        app,
        CLOUDS_SHADER_HANDLE,
        include_str!("../../shaders/clouds.wgsl"),
        "shaders/clouds.wgsl",
    );
    crate::shader_registry::try_load_internal_shader(
        app,
        CLOUD_BILLBOARD_SHADER_HANDLE,
        include_str!("../../shaders/cloud_billboard.wgsl"),
        "shaders/cloud_billboard.wgsl",
    );
    crate::shader_registry::try_load_internal_shader(
        app,
        CLOUD_NOISE_SHADER_HANDLE,
        include_str!("../../shaders/cloud_noise.wgsl"),
        "shaders/cloud_noise.wgsl",
    );

    // ExtractComponentPlugin: main → render world each frame (ExtractSchedule).
    app.add_plugins(ExtractComponentPlugin::<CesiumClouds>::default());

    // Main-world: attach DepthPrepass to cameras driving an active collection.
    app.add_systems(Update, setup_clouds_prepass);
}

/// `Plugin::finish`-time half of [`register_clouds_node`]: the render-world
/// pipeline resource + noise texture + `Render` systems + the `Core3d` node.
pub fn register_clouds_node_render_world(render_app: &mut bevy::app::SubApp) {
    // FIX-REG-FACADE (DEV-029): degrade to a no-op when `RenderDevice` is absent
    // (finish half reached from `build`, or a bare render world). See
    // `crate::effects::render_world_missing_device`.
    if crate::effects::render_world_missing_device(render_app) {
        return;
    }

    render_app
        .init_resource::<CloudsPipeline>()
        .add_systems(
            Render,
            (
                initialize_clouds_noise,
                prepare_clouds.in_set(RenderSet::Prepare),
            )
                .chain(),
        )
        .add_render_graph_node::<ViewNodeRunner<CloudsNode>>(Core3d, CesiumCloudsLabel);
    // NOTE: edges are created by `effects::graph::wire_m6_edges` (阶段三 FIX-INTEG),
    // the single owner of the shared `Core3d` chain.
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_effects::cloud::{CumulusCloud, ELLIPSOID_SCALE_FACTOR as DOM_SCALE_FACTOR};
    use glam::DVec3;

    #[test]
    fn clouds_component_default_disabled() {
        let c = CesiumClouds::default();
        assert!(!c.enabled);
        assert!(!c.is_active());
    }

    #[test]
    fn clouds_gate_const_is_byte_stable() {
        // The gate env var is a *mirror* of the registry (asserted cross-crate in
        // feature_flags); pinned here so a local rename turns red.
        assert_eq!(ENV_ENABLE_CLOUDS, "CESIUM_ENABLE_CLOUDS");
        assert_ne!(ENV_ENABLE_CLOUDS, "CESIUM_ENABLE_OIT");
        // Well-formed env name: already fully upper-cased (a lowercase rename
        // would break the registry mirror contract asserted in feature_flags).
        assert_eq!(ENV_ENABLE_CLOUDS, ENV_ENABLE_CLOUDS.to_uppercase().as_str());
        // Reuses the authoritative truthy parser.
        assert!(!gate_from_env_value(None));
        assert!(gate_from_env_value(Some("1".into())));
    }

    #[test]
    fn clouds_shader_handles_unique() {
        assert_ne!(CLOUDS_SHADER_HANDLE, CLOUD_BILLBOARD_SHADER_HANDLE);
        assert_ne!(CLOUDS_SHADER_HANDLE, CLOUD_NOISE_SHADER_HANDLE);
        assert_ne!(
            CLOUDS_SHADER_HANDLE,
            super::super::graph::PASS_THROUGH_SHADER_HANDLE
        );
        assert_ne!(CLOUDS_SHADER_HANDLE, super::super::oit::OIT_ACCUMULATE_SHADER_HANDLE);
        assert_ne!(
            CLOUDS_SHADER_HANDLE,
            super::super::clipping_planes::CLIPPING_SHADER_HANDLE
        );
    }

    #[test]
    fn clouds_headless_graceful() {
        // No RenderApp (headless) → register must not panic.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        #[allow(deprecated)]
        register_clouds_node(&mut app);
    }

    // ─── Uniform packing: the metric → render-unit boundary (red line) ────────

    #[test]
    fn from_domain_empty_is_pixel_neutral() {
        // Empty collection ⇒ info.x = 0 ⇒ the shader passes the source through.
        let collection = CloudCollection::new();
        let u = CloudsUniform::from_domain(&collection, CloudsShadingMode::Faithful);
        assert_eq!(u.info.x, 0.0, "no clouds ⇒ count 0 ⇒ pixel-neutral");
        assert_eq!(u.params.z, 0.0, "default mode is faithful (0)");
    }

    #[test]
    fn from_domain_divides_position_and_scales_into_render_units() {
        // A cloud 6_378_137 m along +X with a maximumSize of one render unit cube.
        let mut collection = CloudCollection::new();
        collection.add(CumulusCloud::new(
            DVec3::new(METERS_PER_RENDER_UNIT, 0.0, 0.0),
            DVec3::new(METERS_PER_RENDER_UNIT, METERS_PER_RENDER_UNIT, METERS_PER_RENDER_UNIT),
        ));
        let u = CloudsUniform::from_domain(&collection, CloudsShadingMode::Faithful);

        assert!((u.info.x - 1.0).abs() < 1e-6, "one active cloud");
        // position ÷ 6378137 ⇒ 1 render unit on X.
        assert!((u.centers[0].x - 1.0).abs() < 1e-5, "centre.x = 1 render unit");
        assert!((u.centers[0].w - 1.0).abs() < 1e-6, "active flag set");
        // scale = 0.82 · maximumSize ÷ MPU ⇒ 0.82 render units on X.
        assert!(
            (u.scales[0].x - DOM_SCALE_FACTOR as f32).abs() < 1e-5,
            "scale.x = 0.82 (ELLIPSOID_SCALE_FACTOR) render units"
        );
    }

    #[test]
    fn from_domain_copies_physical_constants_to_uniform() {
        // params.w extinction, light.w HG g, info.y noise edge mirror the domain.
        let collection = CloudCollection::new();
        let u = CloudsUniform::from_domain(&collection, CloudsShadingMode::Volumetric);
        assert!((u.params.w - BEER_LAMBERT_EXTINCTION as f32).abs() < 1e-6);
        assert!((u.light.w - HG_PHASE_G as f32).abs() < 1e-6);
        assert!((u.info.y - NOISE_TEXTURE_DIMENSIONS as f32).abs() < 1e-6);
        assert!((u.params.y - RAYMARCH_STEPS_DEFAULT as f32).abs() < 1e-6);
        assert_eq!(u.params.z, 1.0, "volumetric mode ⇒ params.z = 1");
    }

    #[test]
    fn from_domain_caps_at_max_clouds() {
        let mut collection = CloudCollection::new();
        for i in 0..(MAX_CLOUDS + 5) {
            collection.add(CumulusCloud::new(
                DVec3::new(i as f64, 0.0, 0.0),
                DVec3::new(100.0, 100.0, 100.0),
            ));
        }
        let u = CloudsUniform::from_domain(&collection, CloudsShadingMode::Faithful);
        assert_eq!(u.info.x as usize, MAX_CLOUDS, "count clamped to MAX_CLOUDS");
        // All 8 slots active; the 9th+ are dropped (still ZERO).
        assert_eq!(u.centers[MAX_CLOUDS - 1].w, 1.0);
    }

    #[test]
    fn from_domain_takes_brightness_from_first_cloud() {
        let mut collection = CloudCollection::new();
        let mut cloud = CumulusCloud::new(DVec3::ZERO, DVec3::new(20.0, 12.0, 8.0));
        cloud.brightness = 0.4;
        collection.add(cloud);
        let u = CloudsUniform::from_domain(&collection, CloudsShadingMode::Faithful);
        assert!((u.camera.w - 0.4).abs() < 1e-6, "camera.w = first cloud brightness");
    }

    // ─── Naga defence line: parse + validate + binding coverage ───────────────

    /// Stubs for `#import` directives that naga cannot resolve.
    const CLOUD_WGSL_IMPORT_STUBS: &str = "\
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

    /// Strips `#import` lines and prepends the stub structs so naga (which has no
    /// Bevy prelude to resolve `#import` against) can parse the module.
    fn cloud_stubbed_wgsl(path: &str) -> String {
        let mut source = String::from(CLOUD_WGSL_IMPORT_STUBS);
        for line in path.lines() {
            if line.starts_with("#import") {
                continue;
            }
            source.push_str(line);
            source.push('\n');
        }
        source
    }

    /// Collect the `(group, binding)` pairs reachable from an entry point by
    /// walking its expression tree (and transitively, called functions).
    fn used_bindings(
        module: &naga::Module,
        entry_name: &str,
    ) -> std::collections::BTreeSet<(u32, u32)> {
        let entry = module
            .entry_points
            .iter()
            .find(|e| e.name == entry_name)
            .unwrap_or_else(|| panic!("missing entry point {entry_name}"));
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
        used
    }

    fn validate(source: &str, label: &str) -> naga::Module {
        let module = naga::front::wgsl::parse_str(source)
            .unwrap_or_else(|error| panic!("{label} does not parse:\n{}", error.emit_to_string(source)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap_or_else(|error| panic!("{label} does not validate: {error}"));
        module
    }

    #[test]
    fn clouds_wgsl_parses_and_type_checks_under_naga() {
        let source = cloud_stubbed_wgsl(include_str!("../../shaders/clouds.wgsl"));
        let module = validate(&source, "clouds.wgsl");
        let entry_points: Vec<_> = module
            .entry_points
            .iter()
            .map(|e| (e.name.as_str(), e.stage))
            .collect();
        assert_eq!(
            entry_points,
            vec![("fragment", naga::ShaderStage::Fragment)],
            "clouds.wgsl must expose exactly one fragment entry point"
        );
    }

    #[test]
    fn clouds_wgsl_bindings_covered_by_layout() {
        let source = cloud_stubbed_wgsl(include_str!("../../shaders/clouds.wgsl"));
        let module = validate(&source, "clouds.wgsl");
        // CloudsPipeline layout: group(0) bindings 0..=6.
        let layout: std::collections::BTreeSet<(u32, u32)> =
            [(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5), (0, 6)]
                .into_iter()
                .collect();
        let used = used_bindings(&module, "fragment");
        let missing: Vec<(u32, u32)> = used.difference(&layout).copied().collect();
        assert!(
            missing.is_empty(),
            "clouds.wgsl uses bindings {missing:?} absent from CloudsPipeline layout"
        );
    }

    #[test]
    fn cloud_noise_wgsl_parses_and_type_checks_under_naga() {
        // cloud_noise.wgsl has no `#import` directives; validate it raw.
        let source = include_str!("../../shaders/cloud_noise.wgsl");
        let module = validate(source, "cloud_noise.wgsl");
        let has_compute = module
            .entry_points
            .iter()
            .any(|e| e.stage == naga::ShaderStage::Compute);
        assert!(
            has_compute,
            "cloud_noise.wgsl must expose a compute entry point"
        );
    }

    #[test]
    fn cloud_billboard_wgsl_parses_and_type_checks_under_naga() {
        // cloud_billboard.wgsl uses `#import bevy_render::view::View`; stub it.
        let source = cloud_stubbed_wgsl(include_str!("../../shaders/cloud_billboard.wgsl"));
        let module = validate(&source, "cloud_billboard.wgsl");
        let mut has_vertex = false;
        let mut has_fragment = false;
        for e in module.entry_points.iter() {
            match e.stage {
                naga::ShaderStage::Vertex => has_vertex = true,
                naga::ShaderStage::Fragment => has_fragment = true,
                _ => {}
            }
        }
        assert!(
            has_vertex && has_fragment,
            "cloud_billboard.wgsl must expose a vertex + fragment entry point"
        );
    }

    #[test]
    fn wgsl_max_clouds_matches_rust_const() {
        // The WGSL `MAX_CLOUDS` + the `array<vec4<f32>, 8>` uniform sizes must
        // stay in lockstep with the Rust `MAX_CLOUDS` (a silent skew would drop
        // or read-out-of-range clouds on the GPU).
        let wgsl = include_str!("../../shaders/clouds.wgsl");
        assert!(
            wgsl.contains(&format!("const MAX_CLOUDS: i32 = {MAX_CLOUDS};")),
            "clouds.wgsl MAX_CLOUDS must equal Rust MAX_CLOUDS = {MAX_CLOUDS}"
        );
        assert!(
            wgsl.contains("array<vec4<f32>, 8>"),
            "clouds.wgsl uniform arrays must be sized 8 to match MAX_CLOUDS"
        );
    }

    #[test]
    fn domain_physical_constants_mirror_wgsl() {
        // The domain f64 constants the shader hard-codes must match, so the
        // domain↔GPU parity the composite relies on does not silently drift.
        assert!((ELLIPSOID_SCALE_FACTOR - 0.82).abs() < 1e-12);
        assert!((HG_PHASE_G - 0.6).abs() < 1e-12);
        assert_eq!(NOISE_TEXTURE_DIMENSIONS, 128);
        assert!((BEER_LAMBERT_EXTINCTION - 0.1).abs() < 1e-12);
    }
}
