//! M6.3 — Panorama / SkyBox render node (cubemap + equirectangular).
//!
//! Implements the cesiumrust panorama draw as a `Core3d` [`ViewNode`], following the
//! M5-E0 render-graph house pattern established by [`super::fxaa`] and
//! [`super::graph`].
//!
//! # Blueprint (upstream truth source, `packages/engine/Source/`)
//! - `Scene/SkyBox.js` (164 lines) — delegates **completely** to `CubeMapPanorama`
//!   (L39-43, L100 comment "Delegate completely").
//! - `Scene/CubeMapPanorama.js` (352 lines) — `pass: Pass.ENVIRONMENT` (L105-106,
//!   comment "render before everything else"), 2×2×2 `BoxGeometry` scaled to
//!   `czm_entireFrustum.y`, `depthTest: {enabled: false}`, `depthMask: false`,
//!   `blending: ALPHA_BLEND`, and L232 `if (!defined(this._cubeMap)) return undefined;`.
//! - `Scene/EquirectangularPanorama.js` (266 lines) — `DEFAULT_RADIUS = 100000.0` m,
//!   `SphereGeometry`, Fabric `Image` material with
//!   `repeat: new Cartesian2(-repeatHorizontal, repeatVertical)` (L117),
//!   `MaterialAppearance({ closed: true, translucent: false, renderState: { cull: { enabled: false } } })`.
//! - `Shaders/SkyBoxVS.glsl`, `Shaders/SkyBoxFS.glsl`, `Shaders/CubeMapPanoramaVS.glsl`.
//! - `Renderer/AutomaticUniforms.js` L329/L341 (`czm_viewRotation` is a **mat3**),
//!   L1064 (`czm_entireFrustum` is a **vec2** `(near, far)`).
//!
//! The domain half lives in `cesium_effects::panorama` (`domain/effects/src/panorama.rs`)
//! and carries all f64 geometry plus the CPU reference for the upstream vertex
//! shader. This file only narrows to f32 at the uniform boundary.
//!
//! # Gate
//! [`ENV_ENABLE_PANORAMA`] (`CESIUM_ENABLE_PANORAMA`), **default OFF**. The gate is
//! evaluated by the *application* layer, which decides whether to call
//! [`register_panorama_node`] at all; [`panorama_gate_enabled`] is the adapter-local
//! mirror used by tests and by any adapter that has to ask. With the gate OFF the
//! node is never added to the graph, no entity ever carries [`CesiumPanorama`], and
//! the eight v0 baselines are untouched (PSNR = infinity).
//!
//! The env name is duplicated here rather than imported from
//! `application/cesium-app/src/feature_flags.rs` because the adapter layer cannot
//! depend on the application layer (DDD) — same convention as
//! [`crate::atmosphere::sky_dome::ENV_ENABLE_SKYDOME`] and
//! `effects::graph::ENV_ENABLE_POSTPROCESS`.
//!
//! # Render order — where the node belongs, and why
//!
//! Upstream draws the panorama in `Pass.ENVIRONMENT`, i.e. *before everything else*,
//! with depth test and depth write disabled. Bevy cannot express that literally:
//!
//! * `MainOpaquePass3dNode`
//!   (`bevy_core_pipeline-0.15.3/src/core_3d/main_opaque_pass_3d_node.rs` L66) takes
//!   its colour attachment through `ViewTarget::get_color_attachment()`, whose
//!   `ColorAttachment::get_attachment`
//!   (`bevy_render-0.15.3/src/texture/texture_attachment.rs` L62-74) issues
//!   `LoadOp::Clear` on the **first** call and `LoadOp::Load` on every later one.
//!   `DepthAttachment::get_attachment` (same file, L102-106) behaves identically.
//!   A node placed *before* `Node3d::MainOpaquePass` therefore has its output erased
//!   by the clear, and a node placed before it cannot draw at all because no pass is
//!   open yet.
//! * Drawing *after* the whole main pass with depth test off would paint over the
//!   globe, which is the opposite of what is wanted.
//!
//! So the node goes **between `Node3d::MainOpaquePass` and
//! `Node3d::MainTransmissivePass`**, with `depth_compare = GreaterEqual` and
//! `frag_depth = 0.0` (Bevy uses reversed-Z, so `0.0` is the far plane). The opaque
//! pass has already claimed every pixel that has geometry and left the rest at the
//! cleared far depth, so this node writes **exactly** the sky pixels. The final
//! framebuffer is bit-identical to upstream's "skybox first, everything over it",
//! because upstream's `ALPHA_BLEND` with `a = czm_morphTime = 1.0` is a plain
//! overwrite (see DEVIATION 4 in `shaders/panorama.wgsl`).
//!
//! Bevy reaches the same conclusion on its own: `MainOpaquePass3dNode` L113-127 draws
//! the built-in skybox as a fullscreen triangle *after* the opaque and alpha-mask
//! phases, with exactly this depth state
//! (`bevy_core_pipeline-0.15.3/src/skybox/mod.rs` L201-216:
//! `depth_write_enabled: false, depth_compare: GreaterEqual`).
//!
//! ## Relative to the starfield and the sky dome
//! Both are `Transparent3d` entities centred on the world origin —
//! `application/cesium-app/src/starfield.rs` L114/L162-173 (`radius = 50.0`,
//! `AlphaMode::Blend`, `unlit: true`, `cull_mode: None`, one draw call) and
//! `atmosphere/sky_dome.rs` (`SKY_DOME_RADIUS = 40.0`, `AlphaMode::Premultiplied`,
//! `SKY_DOME_DEPTH_BIAS = 1000.0` pinning it after the starfield, `cull_mode =
//! Some(Face::Front)`). `Transparent3d` runs in `MainTransparentPass3dNode`, which is
//! **after** `Node3d::MainTransmissivePass`. Placing the panorama before that node
//! therefore gives, per pixel:
//!
//! ```text
//!   1. MainOpaquePass        globe writes colour + depth
//!   2. CesiumPanoramaLabel   panorama fills the remaining far-depth pixels
//!                            (SKYBOX: no depth write; BUBBLE: real depth write)
//!   3. MainTransmissivePass  (unused by cesiumrust)
//!   4. MainTransparentPass   starfield (r = 50, Blend) then sky dome
//!                            (r = 40, Premultiplied, depth_bias 1000)
//! ```
//!
//! which is upstream's ordering exactly: `Pass.ENVIRONMENT` panorama first, then
//! primitives, then the star box (`SkyBox` is itself a `CubeMapPanorama`). The sky
//! dome's transmittance still extinguishes the starfield, and the starfield still
//! blends over the panorama, because neither depth state nor sort order of the two
//! transparent entities is touched. In `BUBBLE` placement the panorama additionally
//! writes real depth, so it correctly occludes the starfield and dome — the
//! equivalent of upstream's `translucent: false` opaque sphere.
//!
//! **No edge is created here.** [`register_panorama_node`] adds the node only;
//! `effects::graph::register_render_graph` owns the single linear `Core3d` chain
//! (Daniel H2, upstream CesiumJS parity) and task #81 wires the edges. See
//! [`insertion_hint`].
//!
//! # DEVIATIONS
//! Logged in `docs/deviations.md#dev-025`; the shader-side ones are listed in the
//! header of `shaders/panorama.wgsl`.
//! 1. Fullscreen triangle instead of far-plane-scaled box geometry (Bevy uses an
//!    infinite-reverse projection, so the far plane is at infinity).
//! 2. Depth-test substitution for `Pass.ENVIRONMENT` (above).
//! 3. No `czm_gammaCorrect` — sRGB texture format + sRGB framebuffer do it in hardware.
//! 4. No `czm_morphTime` alpha — cesiumrust has no 2D/Columbus-View morph.
//! 5. Non-HDR colour target format is `TextureFormat::bevy_default()`
//!    (`Bgra8UnormSrgb`), not `Rgba8UnormSrgb`. This is the format of the
//!    `ViewTarget` main texture; `super::fxaa` uses `Rgba8UnormSrgb` because it
//!    writes to a `create_post_process_texture` intermediate instead. Both are sRGB,
//!    so the project red line ("sRGB colour textures use an sRGB format") holds —
//!    and the panorama *asset* itself is created as `Rgba8UnormSrgb`.
//! 6. Cube-map **placement** works for both texture layouts and vice versa, because
//!    `mode` (placement) and `source` (layout) are independent uniforms. Upstream
//!    hard-wires `CubeMapPanorama` = (skybox, cube) and `EquirectangularPanorama`
//!    = (bubble, equirect).

use std::fmt::Write as _;

use bevy::core_pipeline::core_3d::{
    graph::{Core3d, Node3d},
    CORE_3D_DEPTH_FORMAT,
};
use bevy::ecs::query::QueryItem;
use bevy::image::BevyDefault;
use bevy::prelude::*;
use bevy::render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_asset::RenderAssets,
    render_graph::{
        NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
    },
    render_resource::{
        binding_types::{sampler, texture_2d, texture_cube, uniform_buffer},
        BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, BindingResource,
        CachedRenderPipelineId, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState,
        DepthStencilState, Extent3d, FilterMode, FragmentState, MultisampleState, PipelineCache,
        PrimitiveState, RenderPassDescriptor, RenderPipelineDescriptor,
        Sampler as GpuSampler, SamplerBindingType, SamplerDescriptor, Shader, ShaderStages,
        SpecializedRenderPipeline, SpecializedRenderPipelines,
        StencilFaceState,
        StencilState, StoreOp, Texture, TextureDescriptor, TextureDimension, TextureFormat,
        TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor,
        TextureViewDimension, UniformBuffer, VertexState,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    texture::GpuImage,
    view::{
        ExtractedView, Msaa, ViewDepthTexture, ViewTarget, ViewUniform, ViewUniformOffset,
        ViewUniforms,
    },
    Render, RenderApp, RenderSet,
};
use cesium_effects::panorama::{
    CubeMapPanorama, EquirectangularPanorama, PanoramaPlacement, PanoramaSource,
    PANORAMA_METERS_PER_RENDER_UNIT,
};
use glam::{DMat3, DMat4, DVec3};

// ─── Gate ────────────────────────────────────────────────────────────────────

/// Env var gating panorama registration. **Default OFF.**
///
/// **Single source of truth (task #81)**: the owner of this name is the app-layer
/// registry `application/cesium-app/src/feature_flags.rs` (`ENV_ENABLE_PANORAMA`
/// and the `panorama_enabled()` accessor, listed in `RESERVED_FLAGS`). This const is
/// a *mirror* that exists only because `cesium-app` depends on
/// `cesium-bevy-render` (never the reverse), so this crate cannot import the
/// registry. It is `pub` so
/// `feature_flags::adapter_gate_mirrors_are_byte_identical_to_the_registry` can
/// assert byte-equality across the crate boundary; the registration itself is
/// driven by `effects::graph::M6WaveARenderGraphPlugin`, which reads
/// [`panorama_gate_enabled`] once per plugin phase.
pub const ENV_ENABLE_PANORAMA: &str = "CESIUM_ENABLE_PANORAMA";

/// Adapter-local evaluation of [`ENV_ENABLE_PANORAMA`].
///
/// Truthy set is `crate::pipeline::fetch::gate_from_env_value`'s
/// (`"1"|"true"|"yes"|"on"`, trimmed + lowercased), byte-identical to
/// `feature_flags::env_flag`.
#[inline]
pub fn panorama_gate_enabled() -> bool {
    crate::pipeline::fetch::gate_from_env_value(std::env::var(ENV_ENABLE_PANORAMA).ok())
}

// ─── Shader handle ───────────────────────────────────────────────────────────

/// Unique handle for the embedded `shaders/panorama.wgsl`.
///
/// Follows the `CE51` ("CESI") prefix convention of
/// `super::fxaa::FXAA_SHADER_HANDLE` (`0xCE51_E1E1_F4AA_0012`); the `9A4E_0A70`
/// middle is a phonetic `PAN` + `ORAMA`, and the trailing `0063` is the M6.3
/// milestone number in hex.
pub const PANORAMA_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0xCE51_9A4E_0A70_0063);

// ─── Shader constant mirrors ─────────────────────────────────────────────────

/// `shaders/panorama.wgsl` `MODE_SKYBOX`. Wire value of [`PanoramaPlacement::Skybox`].
pub const MODE_SKYBOX: u32 = PanoramaPlacement::Skybox.as_u32();

/// `shaders/panorama.wgsl` `MODE_BUBBLE`. Wire value of [`PanoramaPlacement::Bubble`].
pub const MODE_BUBBLE: u32 = PanoramaPlacement::Bubble.as_u32();

/// `shaders/panorama.wgsl` `SOURCE_CUBEMAP`. Wire value of [`PanoramaSource::CubeMap`].
pub const SOURCE_CUBEMAP: u32 = PanoramaSource::CubeMap.as_u32();

/// `shaders/panorama.wgsl` `SOURCE_EQUIRECTANGULAR`. Wire value of
/// [`PanoramaSource::Equirectangular`].
pub const SOURCE_EQUIRECTANGULAR: u32 = PanoramaSource::Equirectangular.as_u32();

/// f32 mirror of `shaders/panorama.wgsl`
/// `const DEGENERATE_DIRECTION_SQUARED_EPSILON: f32 = 1.0e-24;`.
///
/// Deliberately an independent literal rather than
/// `cesium_effects::panorama::DEGENERATE_DIRECTION_SQUARED_EPSILON as f32`: the f64
/// constant is correctly rounded from decimal once, and casting it would round a
/// second time, so the two could differ by an ULP.
/// [`tests::the_wgsl_literals_match_the_rust_mirrors_bit_for_bit`] parses the
/// literal straight out of the shader source and compares `to_bits()`, which closes
/// the loop without any double rounding.
pub const PANORAMA_DEGENERATE_DIRECTION_SQUARED_EPSILON_F32: f32 = 1.0e-24;

// ─── Component ───────────────────────────────────────────────────────────────

/// Marker component enabling the cesiumrust panorama draw on a camera entity.
///
/// Extracted to the render world via [`ExtractComponentPlugin`]. [`PanoramaNode`]
/// early-returns when `enabled == false`, and [`prepare_panorama_pipelines`] skips
/// such views entirely, so a disabled panorama costs nothing on the GPU.
#[derive(Component, Clone, Debug, ExtractComponent)]
pub struct CesiumPanorama {
    /// Master enable for this camera.
    pub enabled: bool,
    /// Infinite camera-centred skybox, or finite anchored sphere.
    pub placement: PanoramaPlacement,
    /// Cube-map or 2:1 equirectangular texture.
    pub source: PanoramaSource,
    /// The panorama colour image. Create it as `Rgba8UnormSrgb` (project red line
    /// for sRGB colour textures) so the hardware performs the sRGB→linear decode
    /// that upstream's `czm_gammaCorrect` did by hand.
    pub image: Handle<Image>,
    /// Radiance multiplier. `1.0` is neutral; upstream has no equivalent.
    pub brightness: f32,
    /// **world → local** panorama transform (the inverse of the domain transform,
    /// with its translation expressed in render units). Directions are multiplied by
    /// it with `w = 0.0`, so only its rotation matters in `Skybox` placement; the
    /// bubble centre travels in [`Self::center`] instead.
    ///
    /// Same convention as Bevy's `SkyboxUniforms.transform`
    /// (`bevy_core_pipeline-0.15.3/src/skybox/mod.rs` L127-129).
    pub transform: Mat4,
    /// Bubble centre in world render units. Unused in `Skybox` placement.
    pub center: Vec3,
    /// Bubble radius in render units. Unused in `Skybox` placement.
    pub radius: f32,
    /// Sampler repeat, `(-repeat_horizontal, repeat_vertical)` — upstream
    /// `EquirectangularPanorama.js` L117. Must be paired with
    /// `AddressMode::Repeat`, which [`PanoramaPipeline::from_world`] provides.
    pub repeat: Vec2,
}

impl Default for CesiumPanorama {
    fn default() -> Self {
        Self {
            enabled: false,
            placement: PanoramaPlacement::Skybox,
            source: PanoramaSource::CubeMap,
            image: Handle::default(),
            brightness: 1.0,
            transform: Mat4::IDENTITY,
            center: Vec3::ZERO,
            radius: 0.0,
            repeat: Vec2::ONE,
        }
    }
}

impl CesiumPanorama {
    /// Build the component from a domain [`EquirectangularPanorama`].
    ///
    /// This is the **only** place f64 domain geometry is narrowed to f32: every
    /// `as f32` in this function sits on the uniform boundary, per the project red
    /// line. `image` is supplied by the caller because the domain layer holds a URL
    /// string and knows nothing about Bevy asset handles.
    pub fn from_domain_equirectangular(
        panorama: &EquirectangularPanorama,
        image: Handle<Image>,
        brightness: f32,
    ) -> Self {
        // Upstream composes `transform` from a position plus heading/pitch/roll
        // (`EquirectangularPanorama.js` L46-61), i.e. a rigid transform: an orthonormal
        // 3x3 plus a translation in metres. Only the translation needs rescaling.
        let mut world_from_local = panorama.transform;
        world_from_local.w_axis.x /= PANORAMA_METERS_PER_RENDER_UNIT;
        world_from_local.w_axis.y /= PANORAMA_METERS_PER_RENDER_UNIT;
        world_from_local.w_axis.z /= PANORAMA_METERS_PER_RENDER_UNIT;

        // `DMat4::inverse()` returns a NaN/inf-filled matrix for a singular input
        // (e.g. a zero scale) rather than erroring, which would silently poison the
        // uniform and blank the panorama. Guard on a finite, well-conditioned
        // determinant; on failure fall back to identity (render the panorama
        // untransformed) and warn instead of propagating NaNs.
        let det = world_from_local.determinant();
        let world_from_local_inv = if det.is_finite() && det.abs() > 1.0e-12 {
            world_from_local.inverse()
        } else {
            bevy::log::warn!(
                "panorama: singular / non-finite transform (det={det}); falling back to identity"
            );
            DMat4::IDENTITY
        };

        Self {
            enabled: panorama.show,
            placement: panorama.placement(),
            source: panorama.source(),
            image,
            brightness,
            transform: f32_mat4(world_from_local_inv),
            center: f32_vec3(panorama.center_render_units()),
            radius: panorama.radius_render_units() as f32,
            repeat: f32_vec2_from_dvec2(panorama.texture_repeat()),
        }
    }

    /// Build the component from a domain [`CubeMapPanorama`].
    ///
    /// Upstream's cube-map transform is a **`Matrix3`** — a skybox has orientation
    /// but no position — so only [`CubeMapPanorama::orientation`] is carried over and
    /// [`Self::center`] / [`Self::radius`] stay at zero.
    pub fn from_domain_cubemap(
        panorama: &CubeMapPanorama,
        image: Handle<Image>,
        brightness: f32,
    ) -> Self {
        Self {
            enabled: panorama.show,
            placement: panorama.placement(),
            source: panorama.source(),
            image,
            brightness,
            transform: Mat4::from_mat3(f32_mat3(panorama.orientation().inverse())),
            center: Vec3::ZERO,
            radius: 0.0,
            repeat: Vec2::ONE,
        }
    }
}

/// Per-view cached pipeline id, mirroring `super::fxaa::CameraFxaaPipeline`.
#[derive(Component)]
pub struct CameraPanoramaPipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

/// Per-view bind group plus the uniform buffer that backs it.
///
/// The buffer is stored alongside the bind group on purpose: `UniformBuffer::binding`
/// hands out a `BufferBinding` that borrows the GPU buffer, and dropping the
/// `UniformBuffer` before the render pass consumes the bind group would release the
/// last Rust-side handle to it.
#[derive(Component)]
pub struct CameraPanoramaBindGroup {
    pub bind_group: BindGroup,
    pub uniforms: UniformBuffer<PanoramaUniforms>,
}

// ─── Uniforms ────────────────────────────────────────────────────────────────

/// `shaders/panorama.wgsl` `struct PanoramaUniforms` — 112 bytes.
///
/// Field order and padding are the contract; the shader-side layout table is in that
/// file's header. `center` is a `Vec3` (align 16, size 12) so `_pad_c` is required
/// before the `Mat4`.
///
/// The struct lives in a private `panorama_uniform` module carrying
/// `#![allow(dead_code)]` — the `sky_dome.rs` / `clipping_planes.rs` / `ibl.rs`
/// convention: the encase `ShaderType` derive emits a module-level `check` helper the
/// dead-code pass flags even though every field is uploaded through `write_buffer`.
/// Offsets are pinned field-for-field against the WGSL text by
/// [`tests::the_uniform_layout_matches_the_wgsl_struct_field_for_field`].
pub use panorama_uniform::PanoramaUniforms;

mod panorama_uniform {
    #![allow(dead_code)]
    use bevy::prelude::{Mat4, Vec2, Vec3};
    use bevy::render::render_resource::ShaderType;

    /// GPU panorama uniform; layout matches `struct PanoramaUniforms` in
    /// `shaders/panorama.wgsl` (encase std140, 112 bytes).
    #[derive(ShaderType, Clone, Copy, Debug, PartialEq)]
    pub struct PanoramaUniforms {
        /// [`super::MODE_SKYBOX`] or [`super::MODE_BUBBLE`].
        pub mode: u32,
        /// [`super::SOURCE_CUBEMAP`] or [`super::SOURCE_EQUIRECTANGULAR`].
        pub source: u32,
        /// Radiance multiplier.
        pub brightness: f32,
        /// Bubble radius in render units.
        pub radius: f32,
        /// `(-repeat_horizontal, repeat_vertical)`.
        pub repeat: Vec2,
        /// Aligns `center` to 16 bytes.
        pub _pad_b: Vec2,
        /// Bubble centre in world render units.
        pub center: Vec3,
        /// Aligns `transform` to 16 bytes.
        pub _pad_c: u32,
        /// world → local panorama transform.
        pub transform: Mat4,
    }
}

// ─── Pipeline ────────────────────────────────────────────────────────────────

/// Render-world resource: bind group layout, sampler, and the two placeholder
/// texture views that let one layout serve all four mode × source combinations.
#[derive(Resource)]
pub struct PanoramaPipeline {
    pub bind_group_layout: BindGroupLayout,
    pub sampler: GpuSampler,
    /// Bound into the `texture_cube` slot when the active source is equirectangular.
    pub placeholder_cube_view: TextureView,
    /// Bound into the `texture_2d` slot when the active source is a cube map.
    pub placeholder_flat_view: TextureView,
    /// Keeps the placeholder cube's GPU buffer alive for the resource's lifetime.
    placeholder_cube: Texture,
    /// Keeps the placeholder flat texture's GPU buffer alive.
    placeholder_flat: Texture,
}

impl PanoramaPipeline {
    /// One 1×1 texel per face, `Rgba8UnormSrgb` (an sRGB format, so it satisfies the
    /// `Float { filterable: true }` sample type of the cube slot).
    fn placeholder_cube(device: &RenderDevice) -> (Texture, TextureView) {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("cesium_panorama_placeholder_cube"),
            size: Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor {
            label: Some("cesium_panorama_placeholder_cube_view"),
            dimension: Some(TextureViewDimension::Cube),
            array_layer_count: Some(6),
            ..Default::default()
        });
        (texture, view)
    }

    /// One 1×1 texel, `Rgba8UnormSrgb` (the format the project red line requires for
    /// sRGB panorama colour textures).
    fn placeholder_flat(device: &RenderDevice) -> (Texture, TextureView) {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("cesium_panorama_placeholder_flat"),
            size: Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor {
            label: Some("cesium_panorama_placeholder_flat_view"),
            ..Default::default()
        });
        (texture, view)
    }
}

impl FromWorld for PanoramaPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();

        let bind_group_layout = render_device.create_bind_group_layout(
            "cesium_panorama_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_cube(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    // The view uniform is read by the fragment stage too (bubble
                    // depth + ray reconstruction), and Bevy binds it dynamically.
                    uniform_buffer::<ViewUniform>(true)
                        .visibility(ShaderStages::VERTEX_FRAGMENT),
                    // **Not** dynamic. `prepare_panorama_bind_groups` writes a plain
                    // `UniformBuffer` per camera (`uniform_buffer.binding()`, no
                    // dynamic offset), so `set_bind_group` supplies exactly one
                    // dynamic offset — the view uniform's. Declaring this binding
                    // dynamic made wgpu expect 2 offsets and fail validation at
                    // `RenderPass::end` ("BindGroup with
                    // 'cesium_panorama_bind_group' label 0 expects 2 dynamic
                    // offsets. However 1 dynamic offset were provided."), found by
                    // the first real-GPU run with `CESIUM_ENABLE_PANORAMA=1`
                    // (task #81). Matches the sibling nodes: `ibl.rs` L307 and
                    // `clipping_planes.rs` L293 both use `(false)`.
                    uniform_buffer::<PanoramaUniforms>(false),
                ),
            ),
        );

        // `AddressMode::Repeat` is mandatory: `PanoramaUniforms::repeat` carries
        // upstream's negative horizontal component, so the sampled u is negative for
        // every direction and must wrap exactly like GL_REPEAT.
        let sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_panorama_sampler"),
            address_mode_u: bevy::render::render_resource::AddressMode::Repeat,
            address_mode_v: bevy::render::render_resource::AddressMode::Repeat,
            address_mode_w: bevy::render::render_resource::AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        let (placeholder_cube, placeholder_cube_view) = Self::placeholder_cube(render_device);
        let (placeholder_flat, placeholder_flat_view) = Self::placeholder_flat(render_device);

        let pipeline = Self {
            bind_group_layout,
            sampler,
            placeholder_cube_view,
            placeholder_flat_view,
            placeholder_cube,
            placeholder_flat,
        };

        // The two `Texture` fields exist purely to keep the GPU objects behind
        // `placeholder_*_view` alive: `wgpu::TextureView` holds only `Arc<C>` and
        // `Box<Data>` (wgpu-23.0.1/src/api/texture_view.rs L12-15) and **not** a
        // reference to its `Texture`, while dropping a `Texture` destroys the backing
        // resource — so without them the views would dangle. Nothing else reads the
        // fields, so reading them here turns that keep-alive invariant into a checked
        // fact rather than an `#[allow(dead_code)]`, and a placeholder with the wrong
        // shape (which would be bound into a slot whose sample type it does not
        // satisfy) fails loudly in every debug build and test run.
        debug_assert_eq!(
            pipeline.placeholder_cube.depth_or_array_layers(),
            6,
            "the cube placeholder must have one layer per face"
        );
        debug_assert_eq!(
            pipeline.placeholder_flat.depth_or_array_layers(),
            1,
            "the equirectangular placeholder must be a single layer"
        );
        debug_assert_eq!(pipeline.placeholder_cube.width(), 1);
        debug_assert_eq!(pipeline.placeholder_flat.width(), 1);

        pipeline
    }
}

/// Specialization key. `placement` is part of the key because it is the only axis
/// that changes pipeline state: `Bubble` writes depth (a finite sphere must occlude
/// the transparent draws that follow it) while `Skybox` does not.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct PanoramaPipelineKey {
    pub hdr: bool,
    pub samples: u32,
    pub depth_format: TextureFormat,
    pub placement: PanoramaPlacement,
}

impl SpecializedRenderPipeline for PanoramaPipeline {
    type Key = PanoramaPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("cesium_panorama_pipeline".into()),
            layout: vec![self.bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            vertex: VertexState {
                shader: PANORAMA_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "panorama_vertex".into(),
                buffers: Vec::new(),
            },
            primitive: PrimitiveState::default(),
            depth_stencil: Some(DepthStencilState {
                format: key.depth_format,
                // BUBBLE writes real depth so the starfield (r = 50) and the sky dome
                // (r = 40) are correctly occluded by a finite panorama. SKYBOX must
                // not: it sits at the far plane and would block nothing anyway, and
                // not writing keeps the depth buffer pristine for the transparent
                // pass that follows.
                depth_write_enabled: key.placement == PanoramaPlacement::Bubble,
                // Reversed-Z: cleared depth is 0.0 (the far plane), so GreaterEqual
                // admits the panorama only where the opaque pass left sky. Identical
                // to Bevy's own skybox (skybox/mod.rs L204) and to
                // `atmosphere/sky_dome.rs`'s premultiplied dome.
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: key.samples,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                shader: PANORAMA_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "panorama_fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: if key.hdr {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        // The ViewTarget main texture's format — see DEVIATION 5.
                        TextureFormat::bevy_default()
                    },
                    // `None` == REPLACE. Upstream's `ALPHA_BLEND` with
                    // `a = czm_morphTime = 1.0` degenerates to exactly this.
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            zero_initialize_workgroup_memory: false,
        }
    }
}

// ─── Render graph label ──────────────────────────────────────────────────────

/// `Core3d` node label for the panorama draw.
///
/// Defined here rather than as a `CesiumPostProcessLabel` variant because that enum
/// lives in `effects/graph.rs`, which the M6 Wave A integration task (#81) owns.
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct CesiumPanoramaLabel;

/// The edges task #81 must add, spelled out for the integration report.
///
/// `effects::graph::insert_node_in_core3d(render_app, CesiumPanoramaLabel,
/// Node3d::MainOpaquePass, Node3d::MainTransmissivePass)` — see the module docs for
/// why this slot and not another.
pub fn insertion_hint() -> String {
    let mut hint = String::new();
    let _ = write!(
        hint,
        "insert_node_in_core3d(render_app, CesiumPanoramaLabel, {:?}, {:?})",
        Node3d::MainOpaquePass,
        Node3d::MainTransmissivePass,
    );
    hint
}

// ─── ViewNode ────────────────────────────────────────────────────────────────

/// The panorama draw. Runs between `Node3d::MainOpaquePass` and
/// `Node3d::MainTransmissivePass`; see the module docs.
///
/// Stateless by design: unlike `super::fxaa::FxaaNode` there is nothing to cache,
/// because the bind group is rebuilt per frame in
/// [`prepare_panorama_bind_groups`] (its content depends on the current image and
/// view uniforms) and the pipeline is looked up by id.
#[derive(Default)]
pub struct PanoramaNode;

impl ViewNode for PanoramaNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static ViewDepthTexture,
        &'static CameraPanoramaPipeline,
        &'static CameraPanoramaBindGroup,
        &'static ViewUniformOffset,
        &'static CesiumPanorama,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (target, depth, pipeline_handle, bind_group, view_uniform_offset, panorama): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if !panorama.enabled {
            return Ok(());
        }

        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_handle.pipeline_id) else {
            return Ok(());
        };

        // Both attachments are second-or-later users this frame, so
        // `ColorAttachment::get_attachment` / `DepthAttachment::get_attachment`
        // return `LoadOp::Load` (bevy_render-0.15.3/src/texture/texture_attachment.rs
        // L62-74, L102-106). The opaque pass already did the clearing.
        let pass_descriptor = RenderPassDescriptor {
            label: Some("cesium_panorama_pass"),
            color_attachments: &[Some(target.get_color_attachment())],
            depth_stencil_attachment: Some(depth.get_attachment(StoreOp::Store)),
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        let mut render_pass = render_context
            .command_encoder()
            .begin_render_pass(&pass_descriptor);

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group.bind_group, &[view_uniform_offset.offset]);
        render_pass.draw(0..3, 0..1); // fullscreen triangle

        Ok(())
    }
}

// ─── Render systems ──────────────────────────────────────────────────────────

/// Prepares the specialized panorama pipeline for every enabled view.
/// Runs in `Render`, `RenderSet::Prepare`.
pub fn prepare_panorama_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<PanoramaPipeline>>,
    pipeline: Res<PanoramaPipeline>,
    views: Query<(Entity, &ExtractedView, &Msaa, &CesiumPanorama)>,
) {
    for (entity, view, msaa, panorama) in &views {
        if !panorama.enabled {
            continue;
        }

        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            PanoramaPipelineKey {
                hdr: view.hdr,
                samples: msaa.samples(),
                depth_format: CORE_3D_DEPTH_FORMAT,
                placement: panorama.placement,
            },
        );

        commands.entity(entity).insert(CameraPanoramaPipeline { pipeline_id });
    }
}

/// Builds the per-view bind group. Runs in `Render`, `RenderSet::PrepareBindGroups`
/// (after `write_view_uniforms`), the same set Bevy's own
/// `prepare_skybox_bind_groups` uses.
///
/// Views whose image is not resident yet are **skipped**, which is upstream parity:
/// `CubeMapPanorama.js` L232 returns `undefined` — no draw command at all — while the
/// cube map is still loading. A skipped view has no [`CameraPanoramaBindGroup`], so
/// [`PanoramaNode`]'s query does not match it and nothing is drawn.
pub fn prepare_panorama_bind_groups(
    mut commands: Commands,
    pipeline: Res<PanoramaPipeline>,
    view_uniforms: Res<ViewUniforms>,
    images: Res<RenderAssets<GpuImage>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    views: Query<(Entity, &CesiumPanorama)>,
) {
    for (entity, panorama) in &views {
        if !panorama.enabled {
            continue;
        }

        let Some(gpu_image) = images.get(&panorama.image) else {
            continue;
        };

        // The active source decides which slot the real image goes into; the other
        // slot takes a 1×1 placeholder so a single bind group layout serves all four
        // mode × source combinations. A resident image whose shape does not match its
        // slot is treated as "not loaded yet" rather than bound anyway — wgpu rejects
        // a `D2` view in a `texture_cube` slot at bind-group creation time, which would
        // be a hard panic on the render thread.
        //
        // `GpuImage` does not carry `Image::texture_view_dimension`, so the cube test
        // is the array-layer count: a wgpu cube texture is a 2D array texture with
        // exactly six layers, while an ordinary equirectangular image has one. Being
        // conservative here is upstream parity, not a workaround —
        // `CubeMapPanorama.js` L232 likewise emits no draw command until the cube map
        // is fully resident.
        let dimension_matches = match panorama.source {
            PanoramaSource::CubeMap => gpu_image.texture.depth_or_array_layers() == 6,
            PanoramaSource::Equirectangular => gpu_image.texture.depth_or_array_layers() == 1,
        };
        if !dimension_matches {
            continue;
        }

        let Some(view_binding) = view_uniforms.uniforms.binding() else {
            continue;
        };

        let uniforms = PanoramaUniforms {
            mode: panorama.placement.as_u32(),
            source: panorama.source.as_u32(),
            brightness: panorama.brightness,
            radius: panorama.radius,
            repeat: panorama.repeat,
            _pad_b: Vec2::ZERO,
            center: panorama.center,
            _pad_c: 0,
            transform: panorama.transform,
        };
        let mut uniform_buffer = UniformBuffer::from(uniforms);
        uniform_buffer.write_buffer(&render_device, &render_queue);
        let Some(uniforms_binding) = uniform_buffer.binding() else {
            continue;
        };

        let (cube_view, flat_view): (&TextureView, &TextureView) = match panorama.source {
            PanoramaSource::CubeMap => (&gpu_image.texture_view, &pipeline.placeholder_flat_view),
            PanoramaSource::Equirectangular => {
                (&pipeline.placeholder_cube_view, &gpu_image.texture_view)
            }
        };

        let bind_group = render_device.create_bind_group(
            Some("cesium_panorama_bind_group"),
            &pipeline.bind_group_layout,
            &BindGroupEntries::sequential((
                BindingResource::TextureView(cube_view),
                BindingResource::TextureView(flat_view),
                BindingResource::Sampler(&pipeline.sampler),
                view_binding,
                uniforms_binding,
            )),
        );

        commands.entity(entity).insert(CameraPanoramaBindGroup {
            bind_group,
            uniforms: uniform_buffer,
        });
    }
}

// ─── Registration ────────────────────────────────────────────────────────────

/// Register the panorama node into `RenderApp` (shader + extract + node + systems).
///
/// Called from cesium-app's `main.rs` only when [`ENV_ENABLE_PANORAMA`] is truthy —
/// same shape as `atmosphere::CesiumAtmospherePlugin` and `register_fxaa_node`.
///
/// This function registers the node **but does not create graph edges**:
/// `effects::graph::register_m6_render_graph` owns the single linear `Core3d`
/// chain (Daniel H2 / Lee M6.3 diamond warning). Task #81 wired it — see
/// [`insertion_hint`] for the shape actually produced.
#[deprecated = "DEV-029 / FIX-REG-FACADE: call `register_panorama_node_main_world` from `Plugin::build` and `register_panorama_node_render_world` from `Plugin::finish`; this facade runs the finish half against a possibly device-less render world."]
pub fn register_panorama_node(app: &mut App) {
    register_panorama_node_main_world(app);
    // Headless `MinimalPlugins` has no `RenderApp` — degrade gracefully.
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        register_panorama_node_render_world(render_app);
    }
}

/// `Plugin::build`-time half of [`register_panorama_node`]: everything that
/// lives in the **main** world (WGSL shader asset + `ExtractComponentPlugin`).
///
/// Split out by task #81 — see `docs/deviations.md#dev-029`. `PanoramaPipeline`'s
/// `FromWorld` (L471) reads `RenderDevice`, which Bevy only inserts into the
/// render world in `RenderPlugin::finish` (`bevy_render/src/lib.rs` L399-430),
/// so the render-world half below must run from a plugin's `finish` — calling it
/// from `build` panics with "RenderDevice does not exist in the World".
pub fn register_panorama_node_main_world(app: &mut App) {
    // Headless-safe: a no-op when `Assets<Shader>` is absent, instead of the
    // `load_internal_asset!` panic (docs/deviations.md#dev-005).
    crate::shader_registry::try_load_internal_shader(
        app,
        PANORAMA_SHADER_HANDLE,
        include_str!("../../shaders/panorama.wgsl"),
        "shaders/panorama.wgsl",
    );

    app.add_plugins(ExtractComponentPlugin::<CesiumPanorama>::default());
}

/// `Plugin::finish`-time half of [`register_panorama_node`]: the render-world
/// pipeline resources + the `Core3d` node.
pub fn register_panorama_node_render_world(render_app: &mut bevy::app::SubApp) {
    // FIX-REG-FACADE (DEV-029): degrade to a no-op when `RenderDevice` is absent
    // (finish half reached from `build`, or a bare render world). See
    // `crate::effects::render_world_missing_device`.
    if crate::effects::render_world_missing_device(render_app) {
        return;
    }

    render_app
        .init_resource::<PanoramaPipeline>()
        .init_resource::<SpecializedRenderPipelines<PanoramaPipeline>>()
        .add_systems(
            Render,
            (
                prepare_panorama_pipelines.in_set(RenderSet::Prepare),
                prepare_panorama_bind_groups.in_set(RenderSet::PrepareBindGroups),
            ),
        )
        .add_render_graph_node::<ViewNodeRunner<PanoramaNode>>(Core3d, CesiumPanoramaLabel);

    // NOTE: edges are created by `effects::graph::wire_m6_edges` (task #81) — see
    // `insertion_hint()`.
}

// ─── f64 → f32 boundary helpers ──────────────────────────────────────────────

/// Narrow a domain `DMat3` to the `Mat3` the uniform buffer needs.
///
/// Column-major, matching glam's `from_cols_array` and WGSL's `mat3x3` layout.
pub fn f32_mat3(value: DMat3) -> Mat3 {
    Mat3::from_cols_array(&[
        value.x_axis.x as f32,
        value.x_axis.y as f32,
        value.x_axis.z as f32,
        value.y_axis.x as f32,
        value.y_axis.y as f32,
        value.y_axis.z as f32,
        value.z_axis.x as f32,
        value.z_axis.y as f32,
        value.z_axis.z as f32,
    ])
}

/// Narrow a domain `DMat4` to the `Mat4` the uniform buffer needs.
pub fn f32_mat4(value: DMat4) -> Mat4 {
    Mat4::from_cols_array(&[
        value.x_axis.x as f32,
        value.x_axis.y as f32,
        value.x_axis.z as f32,
        value.x_axis.w as f32,
        value.y_axis.x as f32,
        value.y_axis.y as f32,
        value.y_axis.z as f32,
        value.y_axis.w as f32,
        value.z_axis.x as f32,
        value.z_axis.y as f32,
        value.z_axis.z as f32,
        value.z_axis.w as f32,
        value.w_axis.x as f32,
        value.w_axis.y as f32,
        value.w_axis.z as f32,
        value.w_axis.w as f32,
    ])
}

/// Narrow a domain `DVec3` to render-unit `Vec3`.
pub fn f32_vec3(value: DVec3) -> Vec3 {
    Vec3::new(value.x as f32, value.y as f32, value.z as f32)
}

/// Narrow a domain `DVec2` (here: the texture repeat) to `Vec2`.
pub fn f32_vec2_from_dvec2(value: glam::DVec2) -> Vec2 {
    Vec2::new(value.x as f32, value.y as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::graph::CesiumPostProcessLabel;
    use crate::resources::METERS_PER_RENDER_UNIT;
    use naga::valid::{Capabilities, ValidationFlags, Validator};
    use std::collections::BTreeSet;

    // ─── naga defence line (M5 Ultra Review findings C1 / C2) ────────────────

    /// Stand-in for `#import bevy_render::view::View`.
    ///
    /// Name- and type-faithful for every field `panorama.wgsl` reads
    /// (`view_from_clip`, `clip_from_view`, `world_from_view`, `view_from_world`,
    /// `world_position`, `viewport` — all present in
    /// `bevy_render-0.15.3/src/view/view.wgsl` L17-27), but *not* offset-faithful:
    /// naga only type-checks, it does not know the real buffer layout, and at runtime
    /// Bevy supplies the genuine `View`.
    const PANORAMA_WGSL_IMPORT_STUBS: &str = "\
struct View {
    view_from_clip: mat4x4<f32>,
    clip_from_view: mat4x4<f32>,
    world_from_view: mat4x4<f32>,
    view_from_world: mat4x4<f32>,
    world_position: vec3<f32>,
    viewport: vec4<f32>,
}
";

    /// The shader source with `#import` lines replaced by [`PANORAMA_WGSL_IMPORT_STUBS`].
    fn panorama_stubbed_wgsl() -> String {
        let mut source = String::from(PANORAMA_WGSL_IMPORT_STUBS);
        for line in include_str!("../../shaders/panorama.wgsl").lines() {
            if line.starts_with("#import") {
                continue;
            }
            source.push_str(line);
            source.push('\n');
        }
        source
    }

    fn panorama_wgsl_source() -> &'static str {
        include_str!("../../shaders/panorama.wgsl")
    }

    /// Line-ending normalisation for **multi-line** `contains` assertions.
    ///
    /// `include_str!` embeds whatever line endings the file has on disk, and this
    /// checkout is on Windows, where both files are CRLF. A needle written with `\n`
    /// therefore never matches a byte-identical source file. `str::lines()` already
    /// strips a trailing `\r`, which is why the line-oriented assertions in this
    /// module never needed the helper — only the ones that span a line break do.
    fn normalized(source: &str) -> String {
        source.replace("\r\n", "\n")
    }

    /// Parse one `const NAME: f32 = LITERAL;` out of the shader source.
    fn wgsl_f32_const(source: &str, name: &str) -> f32 {
        let prefix = format!("const {name}: f32 = ");
        let tail = source
            .split(prefix.as_str())
            .nth(1)
            .unwrap_or_else(|| panic!("shaders/panorama.wgsl has no `const {name}: f32 = ...`"));
        let literal = tail
            .split(';')
            .next()
            .unwrap_or_else(|| panic!("unterminated `const {name}` literal"));
        literal
            .trim()
            .parse::<f32>()
            .unwrap_or_else(|error| panic!("`const {name}` literal {literal:?} is not an f32: {error}"))
    }

    /// Parse one `const NAME: u32 = LITERALu;` out of the shader source.
    fn wgsl_u32_const(source: &str, name: &str) -> u32 {
        let prefix = format!("const {name}: u32 = ");
        let tail = source
            .split(prefix.as_str())
            .nth(1)
            .unwrap_or_else(|| panic!("shaders/panorama.wgsl has no `const {name}: u32 = ...`"));
        let literal = tail
            .split(';')
            .next()
            .unwrap_or_else(|| panic!("unterminated `const {name}` literal"));
        literal
            .trim()
            .trim_end_matches('u')
            .parse::<u32>()
            .unwrap_or_else(|error| panic!("`const {name}` literal {literal:?} is not a u32: {error}"))
    }

    /// C1-class regression: a new WGSL file must parse **and** type-check under the
    /// exact front end Bevy compiles with.
    #[test]
    fn panorama_wgsl_parses_and_type_checks_under_naga() {
        let source = panorama_stubbed_wgsl();
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!(
                "panorama.wgsl does not parse:\n{}",
                error.emit_to_string(&source)
            )
        });

        // `ValidationFlags::all()` includes the uniformity analysis, which is what
        // makes the `uniforms.mode` / `uniforms.source` branches around
        // `textureSample` legal — a non-uniform branch there is a hard error, not a
        // warning.
        Validator::new(ValidationFlags::all(), Capabilities::all())
            .validate(&module)
            .expect("panorama.wgsl does not validate");

        let entry_points = module
            .entry_points
            .iter()
            .map(|entry| (entry.name.as_str(), entry.stage))
            .collect::<Vec<_>>();
        assert_eq!(
            entry_points,
            vec![
                ("panorama_vertex", naga::ShaderStage::Vertex),
                ("panorama_fragment", naga::ShaderStage::Fragment),
            ],
            "the Rust side asks for exactly these two entry points"
        );
    }

    /// C2-class regression: every global the entry points actually touch must have a
    /// slot in the Rust bind group layout, or the pipeline silently binds garbage.
    #[test]
    fn panorama_wgsl_entry_bindings_are_covered_by_the_rust_layout() {
        let source = panorama_stubbed_wgsl();
        let module = naga::front::wgsl::parse_str(&source)
            .unwrap_or_else(|error| panic!("panorama.wgsl does not parse:\n{error}"));
        Validator::new(ValidationFlags::all(), Capabilities::all())
            .validate(&module)
            .expect("panorama.wgsl does not validate");

        // Mirrors `PanoramaPipeline::from_world`: cube, flat, sampler, view, uniforms.
        let layout: BTreeSet<(u32, u32)> =
            [(0, 0), (0, 1), (0, 2), (0, 3), (0, 4)].into_iter().collect();

        let mut used: BTreeSet<(u32, u32)> = BTreeSet::new();

        // Walk the **whole module**, not just the entry points. `panorama_fragment`
        // reaches `panorama_cube` / `panorama_equirect` / `panorama_sampler` only
        // through the `sample_panorama` helper it calls, and naga keeps each
        // function's expressions in its own arena. Scanning entry points alone would
        // under-report `used` by exactly those three slots, which would make the
        // "no slot is paid for and left unused" assertion below fail on bindings that
        // are very much used.
        let visit = |expressions: &naga::Arena<naga::Expression>,
                     used: &mut BTreeSet<(u32, u32)>| {
            for (_, expression) in expressions.iter() {
                if let naga::Expression::GlobalVariable(global) = expression {
                    if let Some(binding) = &module.global_variables[*global].binding {
                        used.insert((binding.group, binding.binding));
                    }
                }
            }
        };
        for (_, function) in module.functions.iter() {
            visit(&function.expressions, &mut used);
        }
        for entry in &module.entry_points {
            visit(&entry.function.expressions, &mut used);
        }

        let uncovered: Vec<(u32, u32)> = used.difference(&layout).copied().collect();
        assert!(
            uncovered.is_empty(),
            "panorama.wgsl reads bindings {uncovered:?} that PanoramaPipeline never binds \
             (C2-class regression: the pipeline would validate but sample garbage)"
        );

        // And the converse: no slot is paid for and left unused.
        let unused: Vec<(u32, u32)> = layout.difference(&used).copied().collect();
        assert!(
            unused.is_empty(),
            "PanoramaPipeline binds slots {unused:?} that no entry point reads"
        );
    }

    /// The shader's mode/source literals must equal the domain enum discriminants,
    /// or `PanoramaUniforms` selects the wrong branch on the GPU.
    #[test]
    fn the_wgsl_mode_and_source_literals_match_the_domain_discriminants() {
        let source = panorama_wgsl_source();

        assert_eq!(wgsl_u32_const(source, "MODE_SKYBOX"), MODE_SKYBOX);
        assert_eq!(wgsl_u32_const(source, "MODE_BUBBLE"), MODE_BUBBLE);
        assert_eq!(wgsl_u32_const(source, "SOURCE_CUBEMAP"), SOURCE_CUBEMAP);
        assert_eq!(
            wgsl_u32_const(source, "SOURCE_EQUIRECTANGULAR"),
            SOURCE_EQUIRECTANGULAR
        );

        // And they are the domain's, not a local re-invention.
        assert_eq!(MODE_SKYBOX, PanoramaPlacement::Skybox.as_u32());
        assert_eq!(MODE_BUBBLE, PanoramaPlacement::Bubble.as_u32());
        assert_eq!(SOURCE_CUBEMAP, PanoramaSource::CubeMap.as_u32());
        assert_eq!(
            SOURCE_EQUIRECTANGULAR,
            PanoramaSource::Equirectangular.as_u32()
        );
        assert_eq!(MODE_SKYBOX, CubeMapPanorama::default().placement().as_u32());
        assert_eq!(
            MODE_BUBBLE,
            EquirectangularPanorama::default().placement().as_u32()
        );
    }

    /// Bit-exact cross-check of every f32 constant the shader and Rust share.
    ///
    /// The literals are parsed **out of the shader source** rather than cast from the
    /// domain's f64 constants, so double rounding cannot hide a mismatch.
    #[test]
    fn the_wgsl_literals_match_the_rust_mirrors_bit_for_bit() {
        let source = panorama_wgsl_source();

        assert_eq!(
            wgsl_f32_const(source, "PANORAMA_PI").to_bits(),
            std::f32::consts::PI.to_bits(),
            "the shader's PI must be the correctly-rounded f32 pi"
        );
        assert_eq!(
            wgsl_f32_const(source, "PANORAMA_TAU").to_bits(),
            std::f32::consts::TAU.to_bits()
        );
        assert_eq!(
            wgsl_f32_const(source, "PANORAMA_HALF_PI").to_bits(),
            std::f32::consts::FRAC_PI_2.to_bits()
        );
        assert_eq!(
            wgsl_f32_const(source, "DEGENERATE_DIRECTION_SQUARED_EPSILON").to_bits(),
            PANORAMA_DEGENERATE_DIRECTION_SQUARED_EPSILON_F32.to_bits()
        );

        // The domain's f64 epsilon is the same decimal, so the two guards reject the
        // same class of input even though they live in different precisions.
        assert_eq!(
            cesium_effects::panorama::DEGENERATE_DIRECTION_SQUARED_EPSILON,
            1.0e-24
        );
    }

    // ─── Gate ────────────────────────────────────────────────────────────────

    /// The gate must default OFF so the golden path stays pixel-neutral.
    ///
    /// Runs with the env var removed; `std::env::remove_var` is unsafe-free on this
    /// toolchain but is *not* thread-safe, so the assertion is written to tolerate a
    /// parallel test having set it — the invariant that matters is that an unset or
    /// empty value reads as OFF.
    #[test]
    fn an_unset_or_empty_gate_reads_as_off() {
        use crate::pipeline::fetch::gate_from_env_value;

        assert!(!gate_from_env_value(None));
        assert!(!gate_from_env_value(Some(String::new())));
        assert!(!gate_from_env_value(Some("   ".to_string())));
        assert!(!gate_from_env_value(Some("0".to_string())));
        assert!(!gate_from_env_value(Some("false".to_string())));
        assert!(!gate_from_env_value(Some("off".to_string())));
        assert!(!gate_from_env_value(Some("no".to_string())));

        assert!(gate_from_env_value(Some("1".to_string())));
        assert!(gate_from_env_value(Some("TRUE".to_string())));
        assert!(gate_from_env_value(Some(" yes ".to_string())));
        assert!(gate_from_env_value(Some("on".to_string())));

        assert_eq!(ENV_ENABLE_PANORAMA, "CESIUM_ENABLE_PANORAMA");

        // `CesiumPanorama::default()` is disabled even when the plugin is registered,
        // so a stray component cannot turn the draw on by accident.
        assert!(!CesiumPanorama::default().enabled);
    }

    // ─── Render order ────────────────────────────────────────────────────────

    /// Graph labels must be unique, or `add_render_graph_node` silently overwrites an
    /// existing node's runner.
    #[test]
    fn the_panorama_label_collides_with_no_existing_label() {
        let panorama = format!("{CesiumPanoramaLabel:?}");
        assert_eq!(panorama, "CesiumPanoramaLabel");

        for existing in [
            format!("{:?}", CesiumPostProcessLabel::PassThrough),
            format!("{:?}", CesiumPostProcessLabel::Fxaa),
            format!("{:?}", CesiumPostProcessLabel::AmbientOcclusion),
        ] {
            assert_ne!(
                panorama, existing,
                "the panorama label must not shadow a post-process label"
            );
        }

        for builtin in [
            Node3d::StartMainPass,
            Node3d::MainOpaquePass,
            Node3d::MainTransmissivePass,
            Node3d::MainTransparentPass,
            Node3d::EndMainPass,
            Node3d::Tonemapping,
            Node3d::EndMainPassPostProcessing,
        ] {
            assert_ne!(
                panorama,
                format!("{builtin:?}"),
                "the panorama label must not shadow the built-in {builtin:?}"
            );
        }

        // The documented insertion point, so the integration task has a machine-checkable
        // string rather than prose.
        let hint = insertion_hint();
        assert!(hint.contains("CesiumPanoramaLabel"), "{hint}");
        assert!(hint.contains("MainOpaquePass"), "{hint}");
        assert!(hint.contains("MainTransmissivePass"), "{hint}");
    }

    /// `Skybox` and `Bubble` must not share a pipeline: only `Bubble` writes depth.
    ///
    /// This is deliberately **key-level**, not descriptor-level. `PanoramaPipeline`
    /// holds a `BindGroupLayout`, a `GpuSampler` and two placeholder `Texture`s,
    /// none of which can be fabricated without a `RenderDevice` — and `RenderDevice`
    /// does not exist under `MinimalPlugins` (M5 Ultra Review / Robin #80 headless
    /// work makes the same point). Specializing a fake pipeline would `panic!`, so
    /// the descriptor facts are pinned device-free by
    /// [`the_pipeline_descriptor_shape_is_pinned_by_source`] instead, which asserts
    /// the exact source lines `specialize` emits.
    ///
    /// What *is* provable headlessly, and is the actual correctness requirement, is
    /// that the two placements produce **different keys**: `SpecializedRenderPipelines`
    /// caches by `Hash + Eq` of the key, so equal keys would hand the skybox a
    /// depth-writing pipeline (or the bubble a depth-blind one) and corrupt the frame.
    #[test]
    fn the_pipeline_key_separates_the_placements_because_depth_write_differs() {
        let skybox = PanoramaPipelineKey {
            hdr: false,
            samples: 1,
            depth_format: CORE_3D_DEPTH_FORMAT,
            placement: PanoramaPlacement::Skybox,
        };
        let bubble = PanoramaPipelineKey {
            placement: PanoramaPlacement::Bubble,
            ..skybox
        };
        assert_ne!(
            skybox, bubble,
            "placement is a specialization axis; equal keys would share one pipeline"
        );

        // Hash must separate them too, not just Eq: `SpecializedRenderPipelines`
        // looks up through a `HashMap`.
        let hash_of = |key: &PanoramaPipelineKey| {
            let mut state = std::collections::hash_map::DefaultHasher::new();
            std::hash::Hash::hash(key, &mut state);
            std::hash::Hasher::finish(&state)
        };
        assert_ne!(
            hash_of(&skybox),
            hash_of(&bubble),
            "two placements that hash identically would collide in the pipeline cache"
        );

        // And the other two axes stay independent, so a key is not accidentally
        // degenerate.
        assert_ne!(
            skybox,
            PanoramaPipelineKey { hdr: true, ..skybox },
            "HDR is an independent axis (ViewTarget::TEXTURE_FORMAT_HDR vs bevy_default)"
        );
        assert_ne!(
            skybox,
            PanoramaPipelineKey { samples: 4, ..skybox },
            "MSAA sample count is an independent axis"
        );
        assert_ne!(
            skybox,
            PanoramaPipelineKey {
                // Not `Depth32Float`: that **is** `CORE_3D_DEPTH_FORMAT` in Bevy 0.15
                // (reversed-Z needs a float depth buffer), so it would compare equal
                // to `skybox` and the assertion would test nothing.
                depth_format: TextureFormat::Depth24PlusStencil8,
                ..skybox
            },
            "depth format is an independent axis"
        );
        assert_eq!(
            CORE_3D_DEPTH_FORMAT,
            TextureFormat::Depth32Float,
            "if Bevy ever changes the core_3d depth format, the assertion above has to \
             pick a different counter-example"
        );

        // The placement discriminants the key carries are the same wire values the
        // shader branches on, so a reorder of `PanoramaPlacement` cannot silently
        // swap the two pipelines.
        assert_eq!(PanoramaPlacement::Skybox.as_u32(), MODE_SKYBOX);
        assert_eq!(PanoramaPlacement::Bubble.as_u32(), MODE_BUBBLE);
    }

    /// Device-free pin of the descriptor facts that the GPU-backed test above cannot
    /// reach under `MinimalPlugins`: the specialization source text itself.
    #[test]
    fn the_pipeline_descriptor_shape_is_pinned_by_source() {
        let source = normalized(include_str!("panorama.rs"));
        let source = source.as_str();

        assert!(
            source.contains("depth_write_enabled: key.placement == PanoramaPlacement::Bubble"),
            "depth write must stay tied to the placement axis"
        );
        assert!(
            source.contains("depth_compare: CompareFunction::GreaterEqual"),
            "reversed-Z sky selection must stay GreaterEqual"
        );
        assert!(
            source.contains("depth_format: CORE_3D_DEPTH_FORMAT"),
            "the depth format must come from the core_3d constant, not be hardcoded"
        );
        assert!(
            source.contains("entry_point: \"panorama_vertex\".into()")
                && source.contains("entry_point: \"panorama_fragment\".into()"),
            "the pipeline must ask for the two entry points the shader defines"
        );
        assert!(
            source.contains("draw(0..3, 0..1)"),
            "the fullscreen triangle must stay three vertices, one instance"
        );
        assert!(
            source.contains("TextureFormat::bevy_default()"),
            "the non-HDR colour format must match the ViewTarget main texture"
        );
        assert!(
            source.contains("AddressMode::Repeat"),
            "the sampler must wrap: upstream's negative horizontal repeat makes u negative"
        );
        assert!(
            source.contains("blend: None"),
            "upstream's ALPHA_BLEND degenerates to REPLACE at czm_morphTime == 1.0; \
             Bevy documents None as both faster and equivalent (skybox/mod.rs L232-233)"
        );
        assert!(
            source.contains("if key.hdr {\n                        ViewTarget::TEXTURE_FORMAT_HDR"),
            "the colour format must track ViewTarget, never a hardcoded TextureFormat"
        );
    }

    // ─── Domain → adapter mapping ────────────────────────────────────────────

    /// The scale constant must agree with the adapter's own, or every panorama radius
    /// and centre is wrong by the ratio.
    #[test]
    fn panorama_meters_per_render_unit_matches_the_adapter_constant() {
        assert_eq!(PANORAMA_METERS_PER_RENDER_UNIT, 6_378_137.0);
        assert_eq!(
            PANORAMA_METERS_PER_RENDER_UNIT as f32,
            METERS_PER_RENDER_UNIT as f32,
            "domain and adapter must share the metres-per-render-unit scale"
        );
    }

    /// FIX-PANO-INVERSE: a singular (zero-scale) domain transform must not poison
    /// the uniform with NaN — the guard falls back to identity instead.
    #[test]
    fn singular_transform_falls_back_to_identity_instead_of_nan() {
        let mut broken = EquirectangularPanorama::new("panorama.jpg");
        broken.transform = DMat4::from_scale(DVec3::ZERO); // determinant == 0
        let component =
            CesiumPanorama::from_domain_equirectangular(&broken, Handle::default(), 1.0);
        for col in component.transform.to_cols_array_2d() {
            assert!(col.iter().all(|v| v.is_finite()), "singular transform leaked NaN into the uniform");
        }
        assert_eq!(component.transform, Mat4::IDENTITY);

        // A well-conditioned rigid transform still inverts correctly.
        let mut ok = EquirectangularPanorama::new("panorama.jpg");
        ok.transform = DMat4::from_rotation_z(std::f64::consts::FRAC_PI_2);
        let ok_component = CesiumPanorama::from_domain_equirectangular(&ok, Handle::default(), 1.0);
        assert_ne!(ok_component.transform, Mat4::IDENTITY, "a real rotation must not be dropped");
    }

    /// f64 stays f64 in the domain; the narrowing happens here and only here.
    #[test]
    fn the_domain_constructors_narrow_to_f32_only_at_the_uniform_boundary() {
        // Equirectangular: 100 km bubble anchored 1 render unit up the +Z axis.
        let anchored = EquirectangularPanorama::with_transform(
            DMat4::from_translation(DVec3::new(0.0, 0.0, PANORAMA_METERS_PER_RENDER_UNIT)),
            "panorama.jpg",
        );
        let component =
            CesiumPanorama::from_domain_equirectangular(&anchored, Handle::default(), 1.0);

        assert!(component.enabled, "`show` defaults to true upstream");
        assert_eq!(component.placement, PanoramaPlacement::Bubble);
        assert_eq!(component.source, PanoramaSource::Equirectangular);
        assert_eq!(component.center, Vec3::new(0.0, 0.0, 1.0));
        assert!(
            (component.radius - 0.0156787).abs() < 1.0e-6,
            "100 km must be ~0.015678 render units, got {}",
            component.radius
        );
        // Upstream L117: repeat = (-repeatHorizontal, repeatVertical).
        assert_eq!(component.repeat, Vec2::new(-1.0, 1.0));

        // `transform` is world→local, so it inverts the domain transform.
        let round_trip = component.transform.inverse();
        assert!(
            (round_trip.w_axis - Vec4::new(0.0, 0.0, 1.0, 1.0)).length() < 1.0e-6,
            "{}",
            round_trip
        );

        // A heading rotation must survive the round trip.
        let mut oriented = EquirectangularPanorama::new("panorama.jpg");
        oriented.transform = DMat4::from_rotation_z(std::f64::consts::FRAC_PI_2);
        let oriented_component =
            CesiumPanorama::from_domain_equirectangular(&oriented, Handle::default(), 2.0);
        assert_eq!(oriented_component.brightness, 2.0);
        // world→local of Rz(+90°) is Rz(-90°): +X maps to -Y.
        let mapped = oriented_component.transform.transform_vector3(Vec3::X);
        assert!(
            (mapped - Vec3::new(0.0, -1.0, 0.0)).length() < 1.0e-6,
            "{mapped}"
        );

        // CubeMap: orientation only, never a position (upstream Matrix3).
        let mut cube = CubeMapPanorama::new([
            "px.jpg".into(),
            "nx.jpg".into(),
            "py.jpg".into(),
            "ny.jpg".into(),
            "pz.jpg".into(),
            "nz.jpg".into(),
        ]);
        cube.transform = DMat4::from_rotation_z(std::f64::consts::FRAC_PI_2)
            * DMat4::from_translation(DVec3::new(1.0e9, 2.0e9, 3.0e9));
        let cube_component = CesiumPanorama::from_domain_cubemap(&cube, Handle::default(), 1.0);
        assert_eq!(cube_component.placement, PanoramaPlacement::Skybox);
        assert_eq!(cube_component.source, PanoramaSource::CubeMap);
        assert_eq!(
            cube_component.center,
            Vec3::ZERO,
            "a cube-map skybox is camera-centred: the domain translation must be dropped"
        );
        assert_eq!(cube_component.radius, 0.0);
        assert_eq!(cube_component.repeat, Vec2::ONE);
        let cube_mapped = cube_component.transform.transform_vector3(Vec3::X);
        assert!(
            (cube_mapped - Vec3::new(0.0, -1.0, 0.0)).length() < 1.0e-6,
            "{cube_mapped}"
        );
        // The rotation part stays orthonormal after narrowing.
        let basis = cube_component.transform;
        assert!((basis.x_axis.truncate().length() - 1.0).abs() < 1.0e-6);
        assert!((basis.y_axis.truncate().length() - 1.0).abs() < 1.0e-6);
        assert!((basis.z_axis.truncate().length() - 1.0).abs() < 1.0e-6);

        // `show = false` propagates, so a hidden panorama never reaches the GPU.
        let mut hidden = EquirectangularPanorama::new("panorama.jpg");
        hidden.show = false;
        assert!(!CesiumPanorama::from_domain_equirectangular(&hidden, Handle::default(), 1.0).enabled);
        let hidden_cube = CubeMapPanorama {
            show: false,
            ..Default::default()
        };
        assert!(!CesiumPanorama::from_domain_cubemap(&hidden_cube, Handle::default(), 1.0).enabled);
    }

    /// The uniform struct must be 112 bytes with the documented field offsets, or the
    /// shader reads a different field than the Rust side wrote.
    ///
    /// `encase` is **not** a direct dependency of this crate (Bevy re-exports only the
    /// `ShaderType` derive, not `encase::internal::SizeValue`), so the size is derived
    /// here from the WGSL text using the uniform address space's alignment rules —
    /// which is a stronger check anyway, because it validates the *shader's* declared
    /// layout rather than trusting the derive macro to have produced it.
    #[test]
    fn the_uniform_layout_matches_the_wgsl_struct_field_for_field() {
        let source = panorama_wgsl_source();

        let struct_body = source
            .split("struct PanoramaUniforms {")
            .nth(1)
            .expect("shaders/panorama.wgsl must define struct PanoramaUniforms");
        let struct_body = struct_body.split('}').next().expect("unterminated struct");
        let wgsl_fields: Vec<(&str, &str)> = struct_body
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with("//"))
            .map(|line| {
                let (name, rest) = line
                    .split_once(':')
                    .unwrap_or_else(|| panic!("`{line}` is not a `name: type,` field"));
                let ty = rest.trim().trim_end_matches(',').trim();
                (name.trim(), ty)
            })
            .collect();

        // The field ORDER is the contract, so check it before anything else.
        assert_eq!(
            wgsl_fields
                .iter()
                .map(|(name, ty)| format!("{name}: {ty},"))
                .collect::<Vec<_>>(),
            vec![
                "mode: u32,",
                "source: u32,",
                "brightness: f32,",
                "radius: f32,",
                "repeat: vec2<f32>,",
                "_pad_b: vec2<f32>,",
                "center: vec3<f32>,",
                "_pad_c: u32,",
                "transform: mat4x4<f32>,"
            ],
            "reordering these fields silently changes every offset"
        );

        // Walk the WGSL uniform address-space layout rules: each member starts at the
        // next multiple of its own alignment, and the struct's size rounds up to the
        // largest member alignment.
        let align_and_size = |ty: &str| -> (usize, usize) {
            match ty {
                "u32" | "i32" | "f32" => (4, 4),
                "vec2<f32>" => (8, 8),
                // vec3 aligns like vec4 but only occupies 12 bytes.
                "vec3<f32>" => (16, 12),
                // Four vec4 columns, each 16-aligned.
                "mat4x4<f32>" => (16, 64),
                other => panic!("unhandled WGSL uniform type {other:?}"),
            }
        };
        let mut offset = 0usize;
        let mut struct_align = 1usize;
        let mut computed: Vec<(&str, usize)> = Vec::new();
        for (name, ty) in &wgsl_fields {
            let (align, size) = align_and_size(ty);
            struct_align = struct_align.max(align);
            offset = offset.div_ceil(align) * align;
            computed.push((name, offset));
            offset += size;
        }
        let total = offset.div_ceil(struct_align) * struct_align;

        assert_eq!(
            total, 112,
            "shaders/panorama.wgsl documents a 112-byte PanoramaUniforms"
        );

        // The offsets the module doc's layout table advertises, field for field.
        let documented = [
            ("mode", 0),
            ("source", 4),
            ("brightness", 8),
            ("radius", 12),
            ("repeat", 16),
            ("_pad_b", 24),
            ("center", 32),
            ("_pad_c", 44),
            ("transform", 48),
        ];
        assert_eq!(computed.as_slice(), documented.as_slice());

        // And the Rust side must declare the same names in the same order with types
        // that map onto the WGSL ones, otherwise `#[derive(ShaderType)]` writes a
        // different buffer than the shader reads.
        let rust_source = include_str!("panorama.rs");
        let rust_body = rust_source
            .split("pub struct PanoramaUniforms {")
            .nth(1)
            .expect("this file must define pub struct PanoramaUniforms");
        // `str::lines()` (not `split('\n')` + `collect::<String>()`): collecting the
        // slices back into a `String` drops every newline, which fuses the whole
        // struct body into one line that starts with the first field's `///` doc
        // comment and is then filtered out wholesale.
        let rust_fields: Vec<(String, String)> = rust_body
            .lines()
            // `trim_start` before the `}` test: the struct now lives inside the
            // private `panorama_uniform` module, so its closing brace is indented and
            // would otherwise be parsed as a field.
            .take_while(|line| !line.trim_start().starts_with('}'))
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with("//"))
            .map(|line| {
                let (name, rest) = line
                    .split_once(':')
                    .unwrap_or_else(|| panic!("`{line}` is not a `name: Type,` field"));
                (
                    name.trim().trim_start_matches("pub ").to_owned(),
                    rest.trim().trim_end_matches(',').trim().to_owned(),
                )
            })
            .collect();

        let type_map = [
            ("mode", "u32"),
            ("source", "u32"),
            ("brightness", "f32"),
            ("radius", "f32"),
            ("repeat", "Vec2"),
            ("_pad_b", "Vec2"),
            ("center", "Vec3"),
            ("_pad_c", "u32"),
            ("transform", "Mat4"),
        ];
        assert_eq!(
            rust_fields,
            type_map
                .iter()
                .map(|(name, ty)| (name.to_string(), ty.to_string()))
                .collect::<Vec<_>>(),
            "the Rust uniform struct and the WGSL struct must stay field-for-field identical"
        );
    }

    // ─── Headless registration ───────────────────────────────────────────────

    /// `register_panorama_node` must not panic under `MinimalPlugins` (no
    /// `Assets<Shader>`, no `RenderApp`) — the `load_internal_asset!` panic class of
    /// `docs/deviations.md#dev-005`.
    #[test]
    fn registering_under_minimal_plugins_degrades_gracefully() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        #[allow(deprecated)]
        register_panorama_node(&mut app);

        // No shader storage, so nothing was inserted...
        assert!(!crate::shader_registry::shader_assets_available(&app));
        // ...and the extract-component plugin is still there, because it is pure ECS.
        app.update();
    }

    /// With an asset backend but no render app the shader is registered and the node
    /// registration still degrades instead of panicking.
    #[test]
    fn registering_with_an_asset_backend_but_no_render_app_loads_the_shader() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());
        <App as bevy::asset::AssetApp>::init_asset::<Shader>(&mut app);

        #[allow(deprecated)]
        register_panorama_node(&mut app);

        let shaders = app.world().resource::<Assets<Shader>>();
        assert!(
            shaders.get(&PANORAMA_SHADER_HANDLE).is_some(),
            "the embedded panorama.wgsl must be registered under its weak handle"
        );
        app.update();
    }

    /// The embedded shader source really is the file on disk, and it really does
    /// carry the two entry points the pipeline asks for.
    #[test]
    fn the_embedded_shader_is_the_file_on_disk() {
        let owned = normalized(panorama_wgsl_source());
        let source = owned.as_str();
        assert!(source.contains("@vertex\nfn panorama_vertex"));
        assert!(source.contains("@fragment\nfn panorama_fragment"));
        assert!(source.contains("#import bevy_render::view::View"));

        // The five bindings, in slot order.
        for declaration in [
            "@group(0) @binding(0) var panorama_cube: texture_cube<f32>;",
            "@group(0) @binding(1) var panorama_equirect: texture_2d<f32>;",
            "@group(0) @binding(2) var panorama_sampler: sampler;",
            "@group(0) @binding(3) var<uniform> view: View;",
            "@group(0) @binding(4) var<uniform> uniforms: PanoramaUniforms;",
        ] {
            assert!(
                source.contains(declaration),
                "shaders/panorama.wgsl is missing `{declaration}`"
            );
        }

        // The upstream-faithful bits that must not be "cleaned up" later.
        assert!(
            source.contains("vec3(1.0, 1.0, -1.0)"),
            "the left-handed cube correction must stay"
        );
        assert!(
            source.contains("* uniforms.repeat"),
            "the equirectangular flip lives in `repeat`, not in a hardcoded negation"
        );
        assert!(
            source.contains("discard;"),
            "a bubble ray that misses the sphere must not write colour"
        );
        assert!(
            source.contains("clamp(direction.z, -1.0, 1.0)"),
            "asin of a 1-ULP-over-length z is NaN; the clamp is mandatory"
        );
    }

    /// `PanoramaNode` must stay stateless: a cached bind group keyed on a texture
    /// view id would go stale the moment the panorama image asset is reloaded.
    #[test]
    fn the_node_is_stateless_and_early_returns_when_disabled() {
        // The struct has no fields, so there is nothing to cache and nothing to
        // invalidate; this test pins that fact against a future refactor.
        let node = PanoramaNode;
        let _ = &node;
        assert_eq!(
            std::mem::size_of::<PanoramaNode>(),
            0,
            "PanoramaNode must stay a zero-sized, stateless ViewNode"
        );

        // A disabled component is skipped by both prepare systems, so no pipeline id
        // and no bind group ever reach the node's query.
        let component = CesiumPanorama::default();
        assert!(!component.enabled);
        assert_eq!(component.placement.as_u32(), MODE_SKYBOX);
        assert_eq!(component.source.as_u32(), SOURCE_CUBEMAP);
    }
}
