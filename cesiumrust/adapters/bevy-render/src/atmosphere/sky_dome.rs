//! Sky dome: geometry, material and the CPU-side f32 mirror of
//! `sky_atmosphere.wgsl` (M5-C).
//!
//! # What this is
//! CesiumJS renders the sky as `Scene/SkyAtmosphere.js`: a large sphere around
//! the Earth shaded by a per-pixel single-scattering integral
//! (`czm_computeScattering` + `czm_computeAtmosphereColor`). This module is the
//! cesiumrust counterpart:
//!
//! * [`build_sky_dome_mesh`] — the dome geometry (a UV sphere of
//!   [`SKY_DOME_RADIUS`] render units, rendered from the inside).
//! * [`SkyDomeMaterial`] — the Bevy `Material` binding
//!   [`sky_atmosphere.wgsl`](super) at `@group(2) @binding(0)`.
//! * [`SkyAtmosphereParams`] — the uniform block, holding every physical
//!   parameter already converted from metres (domain, f64) to render units
//!   (GPU, f32). See [`SkyAtmosphereParams::from_domain`].
//! * [`spawn_sky_dome`] / [`despawn_sky_dome`] — the entity spawn/tear-down
//!   logic, driven from [`super::sky_system::sky_dome_setup`].
//! * the `*_f32` free functions — an **op-for-op CPU mirror** of every helper
//!   in the WGSL, so the production scattering algorithm is unit-testable
//!   headlessly (no GPU, no window, no wgpu) instead of only being checkable
//!   on the xvfb e2e runner.
//!
//! # Gating
//! [`sky_dome_gate_enabled()`] is the adapter-local read of
//! `CESIUM_ENABLE_SKYDOME`, the same variable `feature_flags::skydome_enabled()`
//! reads in cesium-app to decide whether `CesiumAtmospherePlugin` is registered
//! at all (main.rs L513-516). It is duplicated here because the adapter layer
//! cannot import the application layer (DDD); the parsing is delegated to
//! [`crate::pipeline::fetch::gate_from_env_value`], whose truthy set
//! (`"1"|"true"|"yes"|"on"`, trimmed + lowercased) is byte-identical to
//! `feature_flags::env_flag`. Same pattern as
//! `effects::graph::postprocess_gate_enabled` and
//! `tileset::content_loader::gltf_upgrade_gate_enabled`.
//!
//! When the gate is OFF the plugin is never registered, [`super::sky_system::sky_system`]
//! never runs, and the sky stays the pre-M5-C `ClearColor` — so the eight v0
//! baselines are untouched (PSNR = infinity).
//!
//! # Render order
//! The requirement is "dome after starfield, before globe". Bevy's `Core3d`
//! phase order is fixed (`Opaque3d` → `AlphaMask3d` → `Transmissive3d` →
//! `Transparent3d`), so a transparent dome can never be *literally* drawn
//! before the opaque globe without a custom render-graph node (M5-E0
//! territory). Three mechanisms combine to the required pixel result instead:
//!
//! 1. **dome after starfield** — both entities are centred on the world
//!    origin, so `ViewRangefinder3d::distance_translation` returns the *same*
//!    view-space Z for both and `Transparent3d`'s `sort_by_key(distance)`
//!    (ascending, i.e. back-to-front) would be a coin flip — non-reproducible
//!    frames. [`SkyDomeMaterial::depth_bias`] adds [`SKY_DOME_DEPTH_BIAS`] to
//!    the dome's sort distance, which pins it strictly *after* the starfield at
//!    every camera distance in `orbit_camera`'s `[1.005, 20.0]` range. This
//!    matters because the starfield is `AlphaMode::Blend`, and alpha blending
//!    is not commutative.
//! 2. **globe occludes dome** — `AlphaMode::Premultiplied` puts the dome in
//!    `Transparent3d` with `depth_write_enabled = false` and
//!    `depth_compare = GreaterEqual` (reversed-Z). The opaque globe has already
//!    written the depth buffer, so every dome fragment behind it fails the
//!    depth test. Pixel-identical to "dome drawn first, globe over it".
//! 3. **single shell** — `cull_mode = Some(Face::Front)` (via
//!    [`SkyDomeMaterial::specialize`]) keeps only the far hemisphere, so the
//!    scattering integral is never applied twice along a ray. Same trick as
//!    `atmosphere_glow.rs`.
//!
//! Logged as `docs/deviations.md#dev-018`.
//!
//! # Units
//! `1 render unit = METERS_PER_RENDER_UNIT = 6_378_137 m`. Lengths are divided
//! by it, scattering coefficients are multiplied by it, and the product
//! (optical depth) plus every density ratio `exp(-h/H)` are therefore
//! **exactly invariant** under the rescaling. That invariance is what makes the
//! f64 domain reference and the f32 GPU result comparable; it is asserted
//! numerically by [`tests::unit_invariance_optical_depth_and_density`].

use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::mesh::MeshVertexBufferLayoutRef;
use bevy::render::render_resource::{
    AsBindGroup, Face, RenderPipelineDescriptor, Shader, ShaderRef, ShaderType,
    SpecializedMeshPipelineError,
};
use cesium_atmosphere::scattering::AtmosphereParameters;

use crate::resources::METERS_PER_RENDER_UNIT;

/// Env var read by `feature_flags::skydome_enabled()` in cesium-app, which gates
/// `CesiumAtmospherePlugin` registration (main.rs L513-516). Duplicated here
/// because the adapter layer cannot import the application layer (DDD); see the
/// module docs.
pub const ENV_ENABLE_SKYDOME: &str = "CESIUM_ENABLE_SKYDOME";

/// Adapter-local evaluation of [`ENV_ENABLE_SKYDOME`], semantically identical to
/// `feature_flags::skydome_enabled()`.
#[inline]
pub fn sky_dome_gate_enabled() -> bool {
    crate::pipeline::fetch::gate_from_env_value(std::env::var(ENV_ENABLE_SKYDOME).ok())
}

/// Strong handle to the embedded single-scattering WGSL shader.
///
/// Registered headless-safely through
/// [`crate::shader_registry::try_load_internal_shader`] from
/// [`super::CesiumAtmospherePlugin`], never through a bare `load_internal_asset!`
/// (which panics under `MinimalPlugins` — see `docs/deviations.md#dev-005`).
///
/// The u128 is the ASCII bytes of `"SKYDOMEATMOS\0\0"`, following the
/// `FABRIC_MATERIAL_SHADER_HANDLE` convention.
pub const SKY_ATMOSPHERE_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0x534B_5944_4F4D_4541_544D_4F53_0000);

/// Dome radius in render units.
///
/// Constraints, all asserted by [`tests::dome_radius_fits_the_camera_frustum`]:
/// * `> orbit_camera::OrbitState::max_distance` (20.0) so the camera is always
///   *inside* the dome;
/// * `< starfield`'s sphere radius (50.0) so the dome shells the globe but sits
///   inside the star field, letting the stars be extinguished by it;
/// * `< orbit_camera::CAMERA_FAR` (200.0) so it is not clipped;
/// * `> cesium_atmosphere::scattering::constants::OUTER_RADIUS / MPU`
///   (= 1.0157) so the dome geometrically encloses the modelled atmosphere.
pub const SKY_DOME_RADIUS: f32 = 40.0;

/// Icosphere subdivision level of the dome mesh.
///
/// `5` yields `20 * 4^5 = 20480` triangles, i.e. ~160 silhouette segments —
/// sub-0.2 px chord error at 1080p, so the limb reads as a smooth circle.
/// The scattering integral itself is per-fragment, so the geometry only has to
/// be round enough for `world_position` interpolation to stay accurate.
pub const SKY_DOME_SUBDIVISIONS: u32 = 5;

/// Sort-distance bias forcing the dome strictly after the starfield in
/// `Transparent3d`. Positive because `Transparent3d::sort` is *ascending* on
/// view-space Z and the camera looks down −Z, so ascending == back-to-front;
/// a larger distance therefore means "drawn later". See the module docs.
pub const SKY_DOME_DEPTH_BIAS: f32 = 1000.0;

/// `sky_atmosphere.wgsl` `MODE_RAYMARCH`: production Hillaire/CesiumJS double
/// ray-march single scattering.
pub const MODE_RAYMARCH: u32 = 0;

/// `sky_atmosphere.wgsl` `MODE_CLOSED_FORM`: op-for-op mirror of domain
/// [`cesium_atmosphere::scattering::compute_sky_color`] at sea level, used as
/// the GPU↔CPU parity probe. Not a production look.
pub const MODE_CLOSED_FORM: u32 = 1;

/// Rayleigh phase normalisation `3/(16*pi)`.
///
/// Identical in the blueprint (`computeAtmosphereColor.glsl` L33, written there
/// as `3.0/50.2654824574`) and in the domain (`scattering.rs` L82) — the only
/// phase constant the two agree on.
///
/// The literal is written as the *f64 expression* rather than a decimal so that
/// (a) `clippy::excessive_precision` cannot fire and (b) the value is provably
/// the correctly-rounded f32 of the exact real `3/(16*pi)`, which is bit-equal
/// to what naga narrows the identical decimal literal in
/// `sky_atmosphere.wgsl` L193 to. [`tests::wgsl_constants_match_the_rust_constants`]
/// asserts that bit-equality.
pub const RAYLEIGH_PHASE_K: f32 = (3.0_f64 / (16.0 * std::f64::consts::PI)) as f32;

/// Mie/Henyey-Greenstein phase normalisation `1/(4*pi)` — the **domain** value
/// (`scattering.rs` L94) and this project's default. Chosen because the M5-C
/// acceptance gate is parity with the domain f64 reference. Written as the f64
/// expression for the same bit-equality reason as [`RAYLEIGH_PHASE_K`].
pub const MIE_PHASE_K_DOMAIN: f32 = (1.0_f64 / (4.0 * std::f64::consts::PI)) as f32;

/// Mie phase normalisation `3/(8*pi)` — the **blueprint** value
/// (`computeAtmosphereColor.glsl` L35, written there as `3.0/25.1327412287`).
/// Exactly 1.5x [`MIE_PHASE_K_DOMAIN`]; same shape, different normalisation.
/// Kept so the shader can be switched to blueprint-faithful output. Written as
/// the f64 expression; bit-equal to `sky_atmosphere.wgsl` L196.
pub const MIE_PHASE_K_BLUEPRINT: f32 = (3.0_f64 / (8.0 * std::f64::consts::PI)) as f32;

/// Sharpness of the horizon/sky step-split sigmoid
/// (`sky_atmosphere.wgsl` D1), in units of `sin(elevation)`.
pub const HORIZON_SPLIT_SHARPNESS: f32 = 8.0;

/// `computeScattering.glsl` L26 `PRIMARY_STEPS_MAX`.
pub const PRIMARY_STEPS_MAX: u32 = 16;

/// `computeScattering.glsl` L27 `LIGHT_STEPS_MAX`.
pub const LIGHT_STEPS_MAX: u32 = 4;

/// Squared epsilon below which the sun-direction uniform is not re-pushed, so
/// the bind group is not dirtied every frame under `FIXED_TIME`.
const SUN_UNIFORM_EPSILON_SQ: f32 = 1.0e-12;

// ---------------------------------------------------------------------------
// Uniform block
// ---------------------------------------------------------------------------

/// The `@group(2) @binding(0)` uniform block of `sky_atmosphere.wgsl`.
///
/// Field order, padding and offsets are load-bearing: they must match the WGSL
/// `struct SkyAtmosphereParams` exactly (encase std140 layout, total size 80
/// bytes, alignment 16). [`tests::wgsl_uniform_layout_matches_rust`] guards it.
///
/// Lives in a private module with `allow(dead_code)` because the `ShaderType`
/// derive generates accessors for every field while only some are read from
/// Rust — the same false-positive suppression pattern `fabric_material.rs`
/// uses for `FabricParams`.
mod sky_params {
    #![allow(dead_code)]
    use super::*;

    /// GPU-side atmosphere parameters, in **render units** and **f32**.
    #[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
    pub struct SkyAtmosphereParams {
        /// Unit vector towards the sun, world space (offset 0).
        ///
        /// World space here *is* the ECI frame `celestial_system` publishes, so
        /// this is the same vector that drives the `DirectionalLight`.
        pub sun_direction: Vec3,
        /// Earth surface radius, render units — `inner_radius / MPU` = 1.0 (offset 12).
        pub inner_radius: f32,
        /// Rayleigh scale height — `8000 m / MPU` (offset 16).
        pub rayleigh_scale_height: f32,
        /// Mie scale height — `1200 m / MPU` (offset 20).
        pub mie_scale_height: f32,
        /// Atmosphere outer radius — `(Earth + 100_000 m) / MPU` (offset 24).
        pub outer_radius: f32,
        /// std140 padding to align the next `vec3` to 16 (offset 28).
        pub pad0: f32,
        /// Rayleigh scattering coefficients × MPU, per metre → per render unit (offset 32).
        pub rayleigh_coefficient: Vec3,
        /// Mie scattering coefficient × MPU (offset 44).
        pub mie_coefficient: f32,
        /// Henyey-Greenstein anisotropy `g` (offset 48). Dimensionless.
        pub mie_anisotropy: f32,
        /// Solar intensity multiplier (offset 52). Dimensionless.
        pub solar_intensity: f32,
        /// Mie phase normalisation constant (offset 56). See [`MIE_PHASE_K_DOMAIN`].
        pub mie_phase_k: f32,
        /// [`MODE_RAYMARCH`] or [`MODE_CLOSED_FORM`] (offset 60).
        pub mode: u32,
        /// Primary ray-march step budget (offset 64).
        pub primary_steps_max: u32,
        /// Light ray-march step budget (offset 68).
        pub light_steps_max: u32,
        /// std140 tail padding (offset 72).
        pub pad1: u32,
        /// std140 tail padding (offset 76).
        pub pad2: u32,
    }

    impl Default for SkyAtmosphereParams {
        fn default() -> Self {
            Self::from_domain(&AtmosphereParameters::default())
        }
    }
}

pub use sky_params::SkyAtmosphereParams;

impl SkyAtmosphereParams {
    /// Converts the domain's f64 metre-based [`AtmosphereParameters`] into the
    /// f32 render-unit uniform block.
    ///
    /// **Every length is divided by [`METERS_PER_RENDER_UNIT`] and every
    /// per-metre scattering coefficient is multiplied by it** — the two
    /// conversions cancel in the optical depth, so the shading result is
    /// unit-invariant (see the module docs and
    /// [`tests::unit_invariance_optical_depth_and_density`]).
    ///
    /// `sun_direction` is left at `Vec3::ZERO` and filled per frame by
    /// [`super::sky_system::sky_system`] from `LightingParams`.
    pub fn from_domain(p: &AtmosphereParameters) -> Self {
        let mpu = METERS_PER_RENDER_UNIT;
        Self {
            sun_direction: Vec3::ZERO,
            // lengths: metres -> render units
            inner_radius: (p.inner_radius / mpu) as f32,
            outer_radius: (p.outer_radius / mpu) as f32,
            rayleigh_scale_height: (p.rayleigh_scale_height / mpu) as f32,
            mie_scale_height: (p.mie_scale_height / mpu) as f32,
            // per-metre coefficients -> per-render-unit coefficients
            rayleigh_coefficient: Vec3::new(
                (p.rayleigh_coefficients[0] * mpu) as f32,
                (p.rayleigh_coefficients[1] * mpu) as f32,
                (p.rayleigh_coefficients[2] * mpu) as f32,
            ),
            mie_coefficient: (p.mie_coefficient * mpu) as f32,
            // dimensionless: straight f64 -> f32 narrowing at the GPU boundary
            mie_anisotropy: p.mie_anisotropy as f32,
            solar_intensity: p.solar_intensity as f32,
            mie_phase_k: MIE_PHASE_K_DOMAIN,
            mode: MODE_RAYMARCH,
            primary_steps_max: PRIMARY_STEPS_MAX,
            light_steps_max: LIGHT_STEPS_MAX,
            pad0: 0.0,
            pad1: 0,
            pad2: 0,
        }
    }

    /// Atmosphere thickness in render units (`outer_radius - inner_radius`).
    #[inline]
    pub fn thickness(&self) -> f32 {
        self.outer_radius - self.inner_radius
    }
}

// ---------------------------------------------------------------------------
// Material
// ---------------------------------------------------------------------------

/// Bevy material binding `sky_atmosphere.wgsl` to the dome mesh.
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct SkyDomeMaterial {
    /// The single-scattering uniform block.
    #[uniform(0)]
    pub params: SkyAtmosphereParams,
}

impl Material for SkyDomeMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(SKY_ATMOSPHERE_SHADER_HANDLE)
    }

    /// Premultiplied so the fragment's per-channel transmittance composites the
    /// starfield/globe behind the dome as `dst = radiance + dst * transmittance`
    /// — the physically-correct single-scattering composite. Also forces
    /// `depth_write_enabled = false` and routes the mesh into `Transparent3d`,
    /// which is what lets the opaque globe occlude the dome via the depth test.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Premultiplied
    }

    /// Pins the dome's `Transparent3d` sort distance strictly above the
    /// starfield's, so the two origin-centred spheres never tie (see the module
    /// docs, mechanism 1).
    fn depth_bias(&self) -> f32 {
        SKY_DOME_DEPTH_BIAS
    }

    /// Overriding the default `cull_mode: Some(Face::Back)` with `Face::Front`
    /// keeps only the hemisphere *far* from the camera, so the scattering
    /// integral is applied once per ray (module docs, mechanism 3).
    ///
    /// `descriptor.vertex.buffers` is already populated by
    /// `MeshPipeline::specialize` before this hook runs (bevy_pbr 0.15
    /// `material.rs` L411 → L422), so only the primitive state is touched.
    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = sky_dome_cull_mode();
        Ok(())
    }
}

/// The dome's rasterizer cull mode, factored out of
/// [`Material::specialize`] so the decision is unit-testable headlessly
/// (constructing a `MaterialPipeline` needs a live `RenderDevice`).
///
/// `Face::Front` because the camera is always *inside* the dome
/// ([`SKY_DOME_RADIUS`] > `orbit_camera`'s 20.0 max distance): the near
/// hemisphere is front-facing and gets culled, leaving exactly one shell along
/// every ray. Same trick as `atmosphere_glow.rs` L83.
pub fn sky_dome_cull_mode() -> Option<Face> {
    Some(Face::Front)
}

/// Marker for the sky dome entity, so it can be found and torn down
/// independently of the rest of the scene.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SkyDome;

// ---------------------------------------------------------------------------
// Geometry + entity spawn
// ---------------------------------------------------------------------------

/// Builds the dome mesh: a full icosphere of [`SKY_DOME_RADIUS`] render units,
/// centred on the world origin (the Earth centre).
///
/// A *full* sphere rather than a hemisphere: with `cull_mode = Face::Front` the
/// near half is culled, so the visible geometry is exactly the far hemisphere,
/// and the horizon stays closed for every camera orientation (a fixed hemisphere
/// would leave a hole whenever the camera looks away from its axis).
///
/// `SphereKind::Ico` is the same builder `atmosphere_glow.rs` already uses, and
/// emits the attribute set `MeshPipeline::specialize` requests.
pub fn build_sky_dome_mesh(meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
    meshes.add(build_sky_dome_mesh_asset())
}

/// The dome [`Mesh`] itself, without an [`Assets`] store — split out so it is
/// unit-testable headlessly (constructing `Assets<Mesh>` needs an `AssetServer`).
pub fn build_sky_dome_mesh_asset() -> Mesh {
    Sphere::new(SKY_DOME_RADIUS)
        .mesh()
        .ico(SKY_DOME_SUBDIVISIONS)
        // Infallible for these constants: `ico` only rejects a non-finite radius
        // or a subdivision level above 16. Same `.expect` shape as
        // `atmosphere_glow.rs` L54.
        .expect("sky dome icosphere subdivision failed")
}

/// Spawns the sky dome entity.
///
/// The `Transform` is identity: the dome is concentric with the globe and with
/// the starfield, which is what makes their `Transparent3d` sort distances tie
/// (and hence what makes [`SKY_DOME_DEPTH_BIAS`] necessary).
pub fn spawn_sky_dome(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<SkyDomeMaterial>,
    params: SkyAtmosphereParams,
) -> Entity {
    let mesh = build_sky_dome_mesh(meshes);
    let material = materials.add(SkyDomeMaterial { params });
    commands
        .spawn((SkyDome, Mesh3d(mesh), MeshMaterial3d(material)))
        .id()
}

/// Despawns every sky dome entity. Idempotent.
pub fn despawn_sky_dome(commands: &mut Commands, domes: impl Iterator<Item = Entity>) {
    for entity in domes {
        commands.entity(entity).despawn();
    }
}

/// Re-pushes the sun direction into the dome material, but only when it moved
/// by more than [`SUN_UNIFORM_EPSILON_SQ`]. Under `FIXED_TIME` the clock is
/// frozen so this is a no-op after the first frame, keeping the baseline
/// capture deterministic and the bind group untouched.
///
/// Returns `true` if the uniform was written.
pub fn update_sun_direction(
    materials: &mut Assets<SkyDomeMaterial>,
    handle: &Handle<SkyDomeMaterial>,
    sun_direction: Vec3,
) -> bool {
    let Some(material) = materials.get_mut(handle) else {
        return false;
    };
    let next = sun_direction.normalize_or_zero();
    let delta_sq = (material.params.sun_direction - next).length_squared();
    if delta_sq <= SUN_UNIFORM_EPSILON_SQ {
        return false;
    }
    material.params.sun_direction = next;
    true
}

// ---------------------------------------------------------------------------
// CPU mirrors of the WGSL helpers (op-for-op, f32)
// ---------------------------------------------------------------------------

/// Mirror of `sky_atmosphere.wgsl::approximate_tanh` (blueprint
/// `approximateTanh.glsl` L7-10).
#[inline]
pub fn approximate_tanh_f32(x: f32) -> f32 {
    let x2 = x * x;
    let numerator = x * (27.0 + x2);
    let denominator = 27.0 + 9.0 * x2;
    (numerator / denominator).clamp(-1.0, 1.0)
}

/// Mirror of `sky_atmosphere.wgsl::ray_sphere_interval` (blueprint
/// `raySphereIntersectionInterval.glsl` L1-37), with the sphere centred on the
/// origin. Returns `(start, stop)`; `stop <= start` means "no intersection".
pub fn ray_sphere_interval_f32(origin: Vec3, direction: Vec3, radius: f32) -> (f32, f32) {
    let oc = origin;
    let a = direction.dot(direction);
    let b = 2.0 * direction.dot(oc);
    let radius_sq = radius * radius;
    let oc_sq = oc.dot(oc);
    let c = oc_sq - radius_sq;
    let b_sq = b * b;
    let four_ac = 4.0 * a * c;
    let det = b_sq - four_ac;
    if det < 0.0 {
        return (1.0, -1.0);
    }
    let sqrt_det = det.sqrt();
    let two_a = 2.0 * a;
    ((-b - sqrt_det) / two_a, (-b + sqrt_det) / two_a)
}

/// Mirror of `sky_atmosphere.wgsl::rayleigh_phase` (blueprint
/// `computeAtmosphereColor.glsl` L33 ≡ domain `scattering.rs` L81-83).
#[inline]
pub fn rayleigh_phase_f32(cos_theta: f32) -> f32 {
    let cos_sq = cos_theta * cos_theta;
    let one_plus_cos_sq = 1.0 + cos_sq;
    RAYLEIGH_PHASE_K * one_plus_cos_sq
}

/// Mirror of `sky_atmosphere.wgsl::mie_phase` (blueprint
/// `computeAtmosphereColor.glsl` L35 ≡ domain `scattering.rs` L90-95).
///
/// `k` is the normalisation constant; pass [`MIE_PHASE_K_DOMAIN`] for domain
/// parity or [`MIE_PHASE_K_BLUEPRINT`] for blueprint parity. Every product is
/// bound separately so nothing fuses into an FMA, matching both the WGSL and
/// the (fast-math-disabled) domain code.
pub fn mie_phase_f32(cos_theta: f32, g: f32, k: f32) -> f32 {
    let g_sq = g * g;
    let cos_sq = cos_theta * cos_theta;
    let one_minus_g_sq = 1.0 - g_sq;
    let one_plus_cos_sq = 1.0 + cos_sq;
    let numerator_a = one_minus_g_sq * one_plus_cos_sq;
    let two_plus_g_sq = 2.0 + g_sq;
    let one_plus_g_sq = 1.0 + g_sq;
    let two_g = 2.0 * g;
    let two_g_cos = two_g * cos_theta;
    let base = one_plus_g_sq - two_g_cos;
    let base_p15 = base.max(1.0e-20).powf(1.5);
    let denominator_a = two_plus_g_sq * base_p15;
    let scaled_numerator = k * numerator_a;
    scaled_numerator / denominator_a
}

/// Mirror of `sky_atmosphere.wgsl::horizon_split_weight` (the D1 rewrite of
/// blueprint `computeScattering.glsl` L50-53).
#[inline]
pub fn horizon_split_weight_f32(sin_elevation: f32) -> f32 {
    let sharpened = sin_elevation * HORIZON_SPLIT_SHARPNESS;
    let t = approximate_tanh_f32(sharpened);
    0.5 * (1.0 + t)
}

/// Mirror of `sky_atmosphere.wgsl::sin_elevation_at` (the D1 rewrite).
pub fn sin_elevation_at_f32(ray_origin: Vec3, ray_direction: Vec3) -> f32 {
    let origin_length = ray_origin.length();
    if origin_length < 1.0e-9 {
        return 0.0;
    }
    let up = ray_origin / origin_length;
    ray_direction.dot(up)
}

/// Mirror of `sky_atmosphere.wgsl::closed_form_radiance`, i.e. of domain
/// `scattering.rs::compute_sky_color` at `camera_height = 0.0` (where both
/// `atmospheric_density` factors are exactly 1.0).
///
/// This is the GPU↔CPU parity probe: it consumes render-unit f32 parameters and
/// must reproduce the f64 metre-based domain function to within f32 rounding.
pub fn closed_form_sky_color_f32(view_direction: Vec3, params: &SkyAtmosphereParams) -> [f32; 3] {
    let cos_theta = view_direction.dot(params.sun_direction);
    let rayleigh_p = rayleigh_phase_f32(cos_theta);
    let mie_p = mie_phase_f32(cos_theta, params.mie_anisotropy, params.mie_phase_k);

    // scattering.rs L131-133 with camera_height = 0.0: exp(-0) = 1.
    let rayleigh_density = 1.0_f32;
    let mie_density = 1.0_f32;

    // scattering.rs L136.
    let path_length = params.thickness();

    // scattering.rs L139-143, per channel, one product per binding.
    let mut color = [0.0_f32; 3];
    for (channel, beta) in color.iter_mut().zip(
        [
            params.rayleigh_coefficient.x,
            params.rayleigh_coefficient.y,
            params.rayleigh_coefficient.z,
        ]
        .iter(),
    ) {
        let beta_times_density = *beta * rayleigh_density;
        let beta_density_phase = beta_times_density * rayleigh_p;
        let rayleigh_term = beta_density_phase * path_length;

        let mie_beta_density = params.mie_coefficient * mie_density;
        let mie_beta_density_phase = mie_beta_density * mie_p;
        let mie_term = mie_beta_density_phase * path_length;

        let summed = rayleigh_term + mie_term;
        *channel = summed * params.solar_intensity;
    }
    color
}

/// Accumulators produced by one primary ray march.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScatteringMarch {
    /// `rayleighAccumulation` (`computeScattering.glsl` L77/L136), render units.
    pub rayleigh_accumulation: Vec3,
    /// `mieAccumulation` (L78/L137), render units.
    pub mie_accumulation: Vec3,
    /// `opticalDepth` (L79/L99): `x` = rayleigh, `y` = mie.
    pub optical_depth: Vec2,
    /// Primary steps actually taken (`PRIMARY_STEPS`, L66).
    pub primary_steps: i32,
    /// Light steps actually taken (`LIGHT_STEPS`, L67).
    pub light_steps: i32,
    /// `w_inside_atmosphere` (the D2 rewrite of L65), in `[0, 1]`.
    pub w_inside_atmosphere: f32,
    /// `w_stop_gt_lprl` (the D1 rewrite of L53), in `[0, 1]`.
    pub w_stop_gt_lprl: f32,
}

/// Final shaded sky for one ray.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkyShading {
    /// In-scattered radiance (`computeAtmosphereColor.glsl` L41), exposure-scaled.
    pub radiance: Vec3,
    /// Per-channel transmittance (the D3 rewrite of `computeScattering.glsl` L148).
    pub transmittance: Vec3,
    /// `1 - mean(transmittance)`, clamped (the premultiplied-alpha scalar).
    pub alpha: f32,
    /// The march that produced it.
    pub march: ScatteringMarch,
}

/// Mirror of `sky_atmosphere.wgsl::march_single_scattering`
/// (`computeScattering.glsl` L83-141, with the D1/D2/D5 rewrites applied).
///
/// Returns `None` when the ray misses the atmosphere (L39-44), in which case
/// the shader emits a fully transparent fragment.
pub fn march_single_scattering_f32(
    ray_origin: Vec3,
    ray_direction: Vec3,
    primary_ray_length: f32,
    params: &SkyAtmosphereParams,
) -> Option<ScatteringMarch> {
    // L39-44.
    let interval = ray_sphere_interval_f32(ray_origin, ray_direction, params.outer_radius);
    if interval.1 <= interval.0 {
        return None;
    }

    // L56-59.
    let start_0 = interval.0;
    let start = start_0.max(0.0);
    let stop = interval.1.min(primary_ray_length);
    if stop <= start {
        return None;
    }

    // L64-65, D2 rewrite: dimensionless camera height.
    let thickness = params.thickness();
    let origin_radius = ray_origin.length();
    let camera_height = origin_radius - params.inner_radius;
    let camera_height_norm = camera_height / thickness;
    let w_tanh = approximate_tanh_f32(camera_height_norm);
    let w_inside_atmosphere = 1.0 - 0.5 * (1.0 + w_tanh);

    // L66-67, clamped to >= 1 so no division below can be by zero.
    let primary_steps_f = params.primary_steps_max as f32 - w_inside_atmosphere * 12.0;
    let light_steps_f = params.light_steps_max as f32 - w_inside_atmosphere * 2.0;
    let primary_steps = (primary_steps_f as i32).max(1);
    let light_steps = (light_steps_f as i32).max(1);

    // L70, L73-75, with the D1 rewrite of `w_stop_gt_lprl`.
    let w_stop_gt_lprl = horizon_split_weight_f32(sin_elevation_at_f32(ray_origin, ray_direction));
    let ray_position_length = start;
    let total_ray_length = stop - ray_position_length;
    let tri = (primary_steps * (primary_steps + 1)) as f32;
    let half_tri = tri * 0.5;
    let one_minus_w = 1.0 - w_inside_atmosphere;
    let ramp_numerator = one_minus_w * total_ray_length;
    let ray_step_length_increase = w_inside_atmosphere * (ramp_numerator / half_tri);
    let base_weight = one_minus_w.max(w_stop_gt_lprl);
    let base_numerator = base_weight * total_ray_length;
    let base_denominator = (7.0 * w_inside_atmosphere).max(primary_steps as f32);
    let ray_step_length = base_numerator / base_denominator;

    // L77-80.
    let height_scale = Vec2::new(params.rayleigh_scale_height, params.mie_scale_height);
    let mut optical_depth = Vec2::ZERO;
    let mut rayleigh_accumulation = Vec3::ZERO;
    let mut mie_accumulation = Vec3::ZERO;
    let mut cursor = ray_position_length;
    let mut step_length = ray_step_length;

    // L83-141.
    for _ in 0..primary_steps {
        // L92, L95.
        let sample_length = cursor + step_length;
        let sample_position = ray_origin + ray_direction * sample_length;
        let sample_radius = sample_position.length();
        // D7 (see the WGSL header): floored at sea level so a primary ray that
        // pierces the Earth cannot produce `exp(+797) = inf` and then the
        // NaN-valued `inf * exp(-inf)` at L136.
        let sample_height = (sample_radius - params.inner_radius).max(0.0);

        // L98-99: component-wise over (rayleigh, mie).
        let neg_height_over_scale = -sample_height / height_scale;
        let sample_density = neg_height_over_scale.exp() * step_length;
        optical_depth += sample_density;

        // L102-105.
        let light_direction = params.sun_direction;
        let light_interval =
            ray_sphere_interval_f32(sample_position, light_direction, params.outer_radius);
        let light_span = (light_interval.1 - light_interval.0).max(0.0);
        let light_step_length = light_span / light_steps as f32;

        // L111-130.
        let mut light_optical_depth = Vec2::ZERO;
        let mut light_cursor = 0.0_f32;
        for _ in 0..light_steps {
            light_cursor += light_step_length;
            let light_position = sample_position + light_direction * light_cursor;
            let light_radius = light_position.length();
            // D7: the light ray crosses the Earth for every shadowed sample.
            let light_height = (light_radius - params.inner_radius).max(0.0);
            let light_neg_h_over_scale = -light_height / height_scale;
            light_optical_depth += light_neg_h_over_scale.exp() * light_step_length;
        }

        // L133: two-way (primary + light) extinction, per channel.
        let total_depth = optical_depth + light_optical_depth;
        let mie_depth = params.mie_coefficient * total_depth.y;
        let rayleigh_depth = params.rayleigh_coefficient * total_depth.x;
        let extinction = mie_depth + rayleigh_depth;
        let attenuation = (-extinction).exp();

        // L136-137.
        let rayleigh_contribution = attenuation * sample_density.x;
        let mie_contribution = attenuation * sample_density.y;
        rayleigh_accumulation += rayleigh_contribution;
        mie_accumulation += mie_contribution;

        // L140.
        cursor += step_length;
        step_length += ray_step_length_increase;
    }

    Some(ScatteringMarch {
        rayleigh_accumulation,
        mie_accumulation,
        optical_depth,
        primary_steps,
        light_steps,
        w_inside_atmosphere,
        w_stop_gt_lprl,
    })
}

/// Mirror of the whole `sky_atmosphere.wgsl` mode-0 fragment body: march, apply
/// the phase functions and the solar intensity (blueprint
/// `computeAtmosphereColor.glsl` L33-41), then derive the D3 transmittance.
///
/// `exposure` is Bevy's `view.exposure`.
pub fn shade_sky_f32(
    ray_origin: Vec3,
    ray_direction: Vec3,
    primary_ray_length: f32,
    params: &SkyAtmosphereParams,
    exposure: f32,
) -> Option<SkyShading> {
    let march = march_single_scattering_f32(ray_origin, ray_direction, primary_ray_length, params)?;

    // L144-145.
    let betas = params.rayleigh_coefficient;
    let rayleigh_color = betas * march.rayleigh_accumulation;
    let mie_color = march.mie_accumulation * params.mie_coefficient;

    // computeAtmosphereColor.glsl L33-41.
    let cos_theta = ray_direction.dot(params.sun_direction);
    let rayleigh_p = rayleigh_phase_f32(cos_theta);
    let mie_p = mie_phase_f32(cos_theta, params.mie_anisotropy, params.mie_phase_k);

    let rayleigh_scattered = rayleigh_color * rayleigh_p;
    let mie_scattered = mie_color * mie_p;
    let scattered = rayleigh_scattered + mie_scattered;
    let gain = params.solar_intensity * exposure;
    let radiance = scattered * gain;

    // D3: per-channel transmittance, scalar alpha from its mean.
    let total_mie_depth = params.mie_coefficient * march.optical_depth.y;
    let total_rayleigh_depth = betas * march.optical_depth.x;
    let extinction = total_mie_depth + total_rayleigh_depth;
    let transmittance = (-extinction).exp();
    let mean_transmittance = transmittance.dot(Vec3::splat(1.0 / 3.0));
    let alpha = (1.0 - mean_transmittance).clamp(0.0, 1.0);

    Some(SkyShading {
        radiance,
        transmittance,
        alpha,
        march,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atmosphere::celestial_system::LightingParams;
    use crate::atmosphere::sky_system::SkyAtmosphere;
    use crate::atmosphere::CesiumAtmospherePlugin;
    use crate::entity::time_system::AnimationClock;
    use crate::pipeline::fetch::gate_from_env_value;
    use bevy::asset::AssetPlugin;
    use bevy::render::mesh::VertexAttributeValues;
    use bevy::render::render_resource::PrimitiveTopology;
    use cesium_atmosphere::celestial::compute_sun_direction_eci;
    use cesium_atmosphere::scattering::{
        compute_horizon_glow, compute_sky_color, mie_phase as mie_phase_domain,
    };
    use glam::DVec3;
    use std::sync::OnceLock;

    /// `AnimationClock::default()`'s epoch: 2024-01-01T00:00:00 UTC
    /// (`entity/time_system.rs` L67-74). `FIXED_TIME` freezes the clock at its
    /// start, so this Julian date is what *every* `v2_sky` baseline shot is
    /// captured under — the single input the whole capture depends on.
    const FROZEN_JULIAN_DATE: f64 = 2_460_310.5;

    /// The sun direction at [`FROZEN_JULIAN_DATE`], spelled out so that any drift
    /// in `compute_sun_direction_eci` fails here loudly instead of silently
    /// invalidating all six `specs/scripts/v2_sky.toml` shots.
    const FROZEN_SUN_DIRECTION: [f64; 3] = [0.174_500_86, -0.903_426_49, -0.391_624_86];

    /// `orbit_camera`'s distance bounds, restated here because the adapter layer
    /// cannot import the application layer (DDD). Source:
    /// `application/cesium-app/src/orbit_camera.rs`.
    const ORBIT_MAX_DISTANCE: f32 = 20.0;
    /// `main.rs`'s perspective far plane.
    const CAMERA_FAR: f32 = 200.0;
    /// The star sphere radius `atmosphere_glow.rs` spawns; the starfield must
    /// shell the dome so the dome's transmittance can extinguish it.
    const STARFIELD_RADIUS: f32 = 50.0;

    /// `sky_atmosphere.wgsl` with CRLF normalised, so multi-line `contains`
    /// assertions hold regardless of the checkout's line endings.
    fn wgsl() -> &'static str {
        static NORMALISED: OnceLock<String> = OnceLock::new();
        NORMALISED.get_or_init(|| include_str!("sky_atmosphere.wgsl").replace("\r\n", "\n"))
    }

    /// The shader's leading `//`-only comment block — the blueprint citation the
    /// task mandates ("顶部注释必须标注蓝本路径 + 行号").
    fn wgsl_header() -> String {
        wgsl()
            .lines()
            .take_while(|line| line.starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn frozen_sun() -> DVec3 {
        compute_sun_direction_eci(FROZEN_JULIAN_DATE)
    }

    fn vec3_of(v: DVec3) -> Vec3 {
        Vec3::new(v.x as f32, v.y as f32, v.z as f32)
    }

    fn dvec3_of(v: Vec3) -> DVec3 {
        DVec3::new(f64::from(v.x), f64::from(v.y), f64::from(v.z))
    }

    fn rel_err(got: f64, expected: f64) -> f64 {
        (got - expected).abs() / expected.abs().max(1.0e-12)
    }

    /// GPU parameters seeded from the domain defaults, sun pinned to the frozen
    /// v2_sky baseline direction.
    fn gpu_params() -> SkyAtmosphereParams {
        let mut params = SkyAtmosphereParams::from_domain(&AtmosphereParameters::default());
        params.sun_direction = vec3_of(frozen_sun());
        params
    }

    // -- the frozen time base -------------------------------------------------

    #[test]
    fn frozen_sun_direction_is_the_baseline_constant() {
        let sun = frozen_sun();
        println!("FROZEN_SUN_DIRECTION = [{:.9}, {:.9}, {:.9}]", sun.x, sun.y, sun.z);
        for (got, expected) in [sun.x, sun.y, sun.z].iter().zip(FROZEN_SUN_DIRECTION) {
            assert!(
                (got - expected).abs() < 1.0e-8,
                "the frozen sun direction drifted: got [{:.9}, {:.9}, {:.9}], the v2_sky baselines were captured against {FROZEN_SUN_DIRECTION:?}",
                sun.x, sun.y, sun.z
            );
        }
        assert!(
            (sun.length() - 1.0).abs() < 1.0e-12,
            "the sun direction must be a unit vector, got {}",
            sun.length()
        );
    }

    // -- the metres -> render-units red line ----------------------------------

    /// The M5-C red line: every metre-based *length* is divided by
    /// `METERS_PER_RENDER_UNIT` and every *per-metre coefficient* is multiplied
    /// by it before reaching the GPU. This proves the rescaling is **inert** for
    /// the two quantities the shader actually integrates, which is what makes the
    /// f32 GPU result and the f64 domain result comparable rather than merely
    /// similar.
    #[test]
    fn unit_invariance_optical_depth_and_density() {
        let domain = AtmosphereParameters::default();
        let gpu = SkyAtmosphereParams::from_domain(&domain);
        let mpu = METERS_PER_RENDER_UNIT;
        let atmosphere_height_m = domain.outer_radius - domain.inner_radius;

        // (1) lengths divide by MPU
        for (label, metres, render_units) in [
            ("inner_radius", domain.inner_radius, f64::from(gpu.inner_radius)),
            ("outer_radius", domain.outer_radius, f64::from(gpu.outer_radius)),
            (
                "rayleigh_scale_height",
                domain.rayleigh_scale_height,
                f64::from(gpu.rayleigh_scale_height),
            ),
            (
                "mie_scale_height",
                domain.mie_scale_height,
                f64::from(gpu.mie_scale_height),
            ),
        ] {
            let err = rel_err(render_units * mpu, metres);
            assert!(err < 1.0e-6, "{label}: {render_units} ru * MPU != {metres} m (rel err {err:.3e})");
        }
        // `thickness()` subtracts two radii agreeing to two decimals, so the f32
        // cancellation costs ~4e-6 relative. That is the single largest rounding
        // in the whole conversion and still 25x inside the parity budget below.
        let thickness_err = rel_err(f64::from(gpu.thickness()) * mpu, atmosphere_height_m);
        assert!(
            thickness_err < 1.0e-5,
            "atmosphere thickness: {thickness_err:.3e}"
        );

        // (2) coefficients multiply by MPU
        for (label, per_metre, render_units) in [
            ("rayleigh.r", domain.rayleigh_coefficients[0], f64::from(gpu.rayleigh_coefficient.x)),
            ("rayleigh.g", domain.rayleigh_coefficients[1], f64::from(gpu.rayleigh_coefficient.y)),
            ("rayleigh.b", domain.rayleigh_coefficients[2], f64::from(gpu.rayleigh_coefficient.z)),
            ("mie", domain.mie_coefficient, f64::from(gpu.mie_coefficient)),
        ] {
            let err = rel_err(render_units, per_metre * mpu);
            assert!(err < 1.0e-6, "{label}: {render_units} != {per_metre}/m * MPU (rel err {err:.3e})");
        }

        // (3) the invariant that actually matters: optical depth `beta * L` and
        //     density `exp(-h/H)` are both ratios, so they do not move at all.
        let mut worst_optical_depth = 0.0_f64;
        for (label, beta_per_metre, beta_render_unit) in [
            ("rayleigh.r", domain.rayleigh_coefficients[0], gpu.rayleigh_coefficient.x),
            ("rayleigh.g", domain.rayleigh_coefficients[1], gpu.rayleigh_coefficient.y),
            ("rayleigh.b", domain.rayleigh_coefficients[2], gpu.rayleigh_coefficient.z),
            ("mie", domain.mie_coefficient, gpu.mie_coefficient),
        ] {
            let od_metres = beta_per_metre * atmosphere_height_m;
            let od_render_units = f64::from(beta_render_unit) * f64::from(gpu.thickness());
            let err = rel_err(od_render_units, od_metres);
            assert!(
                err < 1.0e-5,
                "optical depth for {label} is not unit-invariant: {od_metres:.6} m vs {od_render_units:.6} ru (rel err {err:.3e})"
            );
            worst_optical_depth = worst_optical_depth.max(err);
        }

        let mut worst_density = 0.0_f64;
        for (scale_metres, scale_render_units) in [
            (domain.rayleigh_scale_height, f64::from(gpu.rayleigh_scale_height)),
            (domain.mie_scale_height, f64::from(gpu.mie_scale_height)),
        ] {
            for height_metres in [0.0_f64, 1_000.0, 8_000.0, 50_000.0, 100_000.0] {
                let density_metres = (-height_metres / scale_metres).exp();
                let density_render_units = (-(height_metres / mpu) / scale_render_units).exp();
                // the exponent amplifies the f32 rounding of the scale height by
                // |h/H|, hence 1e-4 rather than 1e-6
                let err =
                    (density_render_units - density_metres).abs() / density_metres.max(f64::MIN_POSITIVE);
                assert!(
                    err < 1.0e-4,
                    "density at {height_metres} m / H={scale_metres} m is not unit-invariant (rel err {err:.3e})"
                );
                worst_density = worst_density.max(err);
            }
        }
        println!(
            "unit invariance: worst optical-depth rel err {worst_optical_depth:.3e}, worst density rel err {worst_density:.3e}"
        );
    }

    /// `from_domain` performs the conversion, field by field, and leaves the
    /// dimensionless quantities alone.
    #[test]
    fn from_domain_converts_metres_to_render_units() {
        let domain = AtmosphereParameters::default();
        let gpu = SkyAtmosphereParams::from_domain(&domain);

        assert_eq!(gpu.inner_radius, 1.0, "the Earth radius is the render unit by definition");
        assert!((gpu.outer_radius - 1.015_678_6).abs() < 1.0e-6, "{}", gpu.outer_radius);
        assert!((gpu.rayleigh_scale_height - 1.254_286e-3).abs() < 1.0e-9, "{}", gpu.rayleigh_scale_height);
        assert!((gpu.mie_scale_height - 1.881_429e-4).abs() < 1.0e-10, "{}", gpu.mie_scale_height);
        assert!((gpu.rayleigh_coefficient.x - 36.9932).abs() < 1.0e-3, "{}", gpu.rayleigh_coefficient.x);
        assert!((gpu.rayleigh_coefficient.y - 86.1048).abs() < 1.0e-3, "{}", gpu.rayleigh_coefficient.y);
        assert!((gpu.rayleigh_coefficient.z - 211.1165).abs() < 1.0e-2, "{}", gpu.rayleigh_coefficient.z);
        assert!((gpu.mie_coefficient - 133.9409).abs() < 1.0e-2, "{}", gpu.mie_coefficient);
        assert!((gpu.thickness() - 0.015_678_6).abs() < 1.0e-6, "{}", gpu.thickness());

        // dimensionless: passed through unchanged
        assert_eq!(gpu.mie_anisotropy, domain.mie_anisotropy as f32);
        assert_eq!(gpu.solar_intensity, domain.solar_intensity as f32);
        // the domain Mie normalisation wins (D6), and production is mode 0
        assert_eq!(gpu.mie_phase_k, MIE_PHASE_K_DOMAIN);
        assert_eq!(gpu.mode, MODE_RAYMARCH);
        assert_eq!(gpu.primary_steps_max, PRIMARY_STEPS_MAX);
        assert_eq!(gpu.light_steps_max, LIGHT_STEPS_MAX);
        // the sun is not `from_domain`'s business
        assert_eq!(gpu.sun_direction, Vec3::ZERO);
        // padding stays zero so the 80-byte upload is deterministic
        assert_eq!((gpu.pad0, gpu.pad1, gpu.pad2), (0.0, 0, 0));
    }

    // -- the CPU reference parity gate ---------------------------------------

    /// The acceptance gate's "CPU reference vs GPU" comparison, at the tightest
    /// resolution reachable headlessly.
    ///
    /// `sky_atmosphere.wgsl::closed_form_radiance` (mode 1) is an op-for-op
    /// transcription of [`closed_form_sky_color_f32`], which is an op-for-op
    /// transcription of the domain's f64
    /// [`compute_sky_color`] at sea level. This therefore bounds the *whole*
    /// f64-domain → f32-render-unit → WGSL chain: everything the GPU does
    /// differently from the CPU reference is f32 rounding of a quantity the
    /// unit-invariance test above proved is unit-neutral.
    ///
    /// A literal device readback additionally needs a GPU; that half is
    /// delegated to the xvfb e2e runner
    /// (`.github/workflows/cesiumrust-e2e.yml`, M11.2) — see `docs/deferred.md`.
    #[test]
    fn closed_form_f32_mirror_matches_f64_domain() {
        let domain = AtmosphereParameters::default();
        let gpu = gpu_params();
        let sun = frozen_sun();
        // an orthonormal basis with the sun as its first axis, so the sweep hits
        // every scattering angle exactly once
        let helper = if sun.z.abs() < 0.9 { DVec3::Z } else { DVec3::X };
        let u = sun.cross(helper).normalize();

        let mut worst = 0.0_f64;
        let mut worst_degree = 0.0_f64;
        for degree in 0..=360 {
            let theta = f64::from(degree).to_radians();
            let view = (sun * theta.cos() + u * theta.sin()).normalize();
            let expected = compute_sky_color(view, sun, 0.0, &domain);
            let got = closed_form_sky_color_f32(vec3_of(view), &gpu);
            for (e, g) in expected.iter().zip(got.iter()) {
                let err = rel_err(f64::from(*g), *e);
                if err > worst {
                    worst = err;
                    worst_degree = f64::from(degree);
                }
            }
        }
        println!("closed-form parity vs domain compute_sky_color: worst rel err {worst:.3e} at theta = {worst_degree} deg");
        assert!(
            worst < 1.0e-4,
            "the f32 render-unit mirror diverges from the f64 domain reference by {worst:.3e} (worst at theta = {worst_degree} deg)"
        );
    }

    /// D6: the domain normalises the Henyey-Greenstein phase with `1/(4*pi)`,
    /// the blueprint (`computeAtmosphereColor.glsl` L35) with `3/(8*pi)`. Same
    /// shape, exactly 1.5x apart; the shader exposes both and defaults to the
    /// domain value.
    #[test]
    fn mie_phase_domain_and_blueprint_differ_only_by_the_normalisation() {
        let ratio = f64::from(MIE_PHASE_K_BLUEPRINT) / f64::from(MIE_PHASE_K_DOMAIN);
        assert!((ratio - 1.5).abs() < 1.0e-6, "3/(8*pi) must be 1.5 * 1/(4*pi), got {ratio}");

        let domain = AtmosphereParameters::default();
        let mut worst = 0.0_f64;
        for degree in 0..=360 {
            let cos_theta = f64::from(degree).to_radians().cos();
            let expected = mie_phase_domain(cos_theta, domain.mie_anisotropy);
            let got = f64::from(mie_phase_f32(
                cos_theta as f32,
                domain.mie_anisotropy as f32,
                MIE_PHASE_K_DOMAIN,
            ));
            let err = rel_err(got, expected);
            assert!(err < 1.0e-5, "mie_phase at cos={cos_theta} diverges by {err:.3e}");
            worst = worst.max(err);
        }
        println!("mie_phase f32-mirror vs f64 domain: worst rel err {worst:.3e}");

        // and the Rayleigh phase agrees with the domain *and* the blueprint
        // (scattering.rs L82 == computeAtmosphereColor.glsl L33), exactly
        for degree in 0..=180 {
            let cos_theta = f64::from(degree).to_radians().cos();
            let expected = cesium_atmosphere::scattering::rayleigh_phase(cos_theta);
            let got = f64::from(rayleigh_phase_f32(cos_theta as f32));
            assert!(rel_err(got, expected) < 1.0e-6, "rayleigh_phase at cos={cos_theta}");
        }
    }

    /// B2 (`approximateTanh.glsl` L7-10): odd, saturating to `[-1, 1]`, and
    /// tracking `tanh` to ~2e-2 — all B1 ever needs from it, since it only uses
    /// it as a 0..1 weight.
    #[test]
    fn approximate_tanh_is_odd_bounded_and_close_to_tanh() {
        assert_eq!(approximate_tanh_f32(0.0), 0.0);
        for x in [-1.0e6_f32, -8.0, -2.0, -1.0, -0.5, -1.0e-6, 1.0e-6, 0.5, 1.0, 2.0, 8.0, 1.0e6] {
            let y = approximate_tanh_f32(x);
            assert!((-1.0..=1.0).contains(&y), "approximate_tanh({x}) = {y} escaped [-1, 1]");
            assert!(
                (y + approximate_tanh_f32(-x)).abs() < 1.0e-6,
                "approximate_tanh must be odd, got {y} at {x}"
            );
            assert!(
                (y - x.tanh()).abs() < 3.0e-2,
                "approximate_tanh({x}) = {y} vs tanh = {}",
                x.tanh()
            );
        }
        // so the D1 weight it feeds is a monotone 0..1 sigmoid centred on the
        // horizon, with the 10/90 crossover a few degrees either side of it
        assert!(horizon_split_weight_f32(-1.0) < 0.01);
        assert!(horizon_split_weight_f32(-0.12) < 0.15, "looking 7 deg below the horizon");
        assert!((horizon_split_weight_f32(0.0) - 0.5).abs() < 1.0e-6);
        assert!(horizon_split_weight_f32(0.12) > 0.85, "looking 7 deg above the horizon");
        assert!(horizon_split_weight_f32(1.0) > 0.99);
        // and `sin_elevation_at` really is the projection onto the local zenith
        let up = vec3_of(frozen_sun());
        assert!((sin_elevation_at_f32(up * 2.0, up) - 1.0).abs() < 1.0e-6);
        assert!((sin_elevation_at_f32(up * 2.0, -up) + 1.0).abs() < 1.0e-6);
        assert!(sin_elevation_at_f32(Vec3::ZERO, up).abs() < 1.0e-9, "a degenerate origin has no zenith");
    }

    /// B4 (`raySphereIntersectionInterval.glsl` L1-37), origin-centred: the four
    /// geometric cases the shader branches on.
    #[test]
    fn ray_sphere_interval_covers_every_geometric_case() {
        let gpu = gpu_params();
        let outer = gpu.outer_radius;

        // (a) a genuine miss (negative discriminant) -> the empty-interval sentinel
        let (start, stop) = ray_sphere_interval_f32(Vec3::new(0.0, 0.0, 3.0), Vec3::X, outer);
        assert_eq!((start, stop), (1.0, -1.0), "a tangent-outside ray must report EMPTY_INTERVAL");

        // (b) outside, aimed at the centre -> the chord [d-r, d+r]
        let (start, stop) = ray_sphere_interval_f32(Vec3::new(0.0, 0.0, 3.0), Vec3::NEG_Z, outer);
        assert!((start - (3.0 - outer)).abs() < 1.0e-5, "entry {start}");
        assert!((stop - (3.0 + outer)).abs() < 1.0e-5, "exit {stop}");

        // (c) inside -> start < 0 < stop, spanning the full chord
        let (start, stop) = ray_sphere_interval_f32(Vec3::new(0.0, 0.0, 0.5), Vec3::Z, outer);
        assert!(start < 0.0 && stop > 0.0, "an interior origin must straddle zero: ({start}, {stop})");
        assert!((stop - start - 2.0 * outer).abs() < 1.0e-4, "chord {start}..{stop}");

        // (d) exactly tangent -> a degenerate interval the caller reads as a miss
        let (start, stop) = ray_sphere_interval_f32(Vec3::new(0.0, 0.0, outer), Vec3::X, outer);
        assert!(stop - start < 1.0e-6, "a tangent ray must have a zero-length interval, got ({start}, {stop})");
    }

    // -- the shader source contract -------------------------------------------

    /// The task's hard requirement (plan L152 + risk table L347): the shader
    /// header must cite the blueprint by **path and line number**, the CPU
    /// reference it is validated against, and every deliberate departure.
    #[test]
    fn wgsl_header_cites_blueprint_paths_and_line_numbers() {
        let header = wgsl_header();
        assert!(
            header.lines().count() > 100,
            "the citation block must be the shader's leading comment, got {} lines",
            header.lines().count()
        );

        for path in [
            "cesium-rs/crates/cesium-shaders/shaders/Builtin/Functions/computeScattering.glsl",
            "packages/engine/Source/Shaders/Builtin/Functions/computeScattering.glsl",
            "approximateTanh.glsl",
            "computeGroundAtmosphereScattering.glsl",
            "raySphereIntersectionInterval.glsl",
            "computeAtmosphereColor.glsl",
            "SkyAtmosphereFS.glsl",
            "domain/atmosphere/src/scattering.rs",
        ] {
            assert!(header.contains(path), "the header must cite the blueprint path `{path}`");
        }
        // line-precise, not merely file-precise
        for line_ref in [
            "L16-24", "L25", "L26-27", "L34", "L39-44", "L50", "L53", "L64", "L65", "L66-67",
            "L83-89", "L98-99", "L102-105", "L133", "L136-137", "L144-145", "L148",
            "L43-73", "L81-83", "L90-95", "L102-104", "L118-146",
        ] {
            assert!(header.contains(line_ref), "the header must cite blueprint line `{line_ref}`");
        }
        // every deliberate departure is documented and numbered
        for index in 1..=7 {
            let marker = format!("// D{index} ");
            assert!(header.contains(&marker), "deviation D{index} must be documented in the header");
        }
        assert!(header.contains("REWRITE, not a"), "the header must say this is a rewrite, not a transpilation");
        assert!(header.contains("DELIBERATE DEVIATIONS FROM THE BLUEPRINT"));
        // the two maths red lines are stated in the shader itself
        assert!(header.contains("METERS_PER_RENDER_UNIT = 6378137"));
        assert!(header.contains("NO FMA CONTRACTION"));
        assert!(header.contains("docs/deviations.md#dev-018"));
    }

    /// The shader's structural contract with Bevy, plus the no-FMA red line.
    #[test]
    fn wgsl_is_a_bevy_material_fragment_with_a_premultiplied_output() {
        let source = wgsl();

        // exactly the two naga_oil imports a forward `Material` fragment needs
        let imports = source.lines().filter(|line| line.starts_with("#import")).collect::<Vec<_>>();
        assert_eq!(
            imports,
            vec![
                "#import bevy_pbr::forward_io::VertexOutput",
                "#import bevy_pbr::mesh_view_bindings"
            ],
            "the imports must match the naga-validation stubs below"
        );
        // the bind group Bevy reserves for `#[uniform(0)]` on a `Material`
        assert!(source.contains("@group(2) @binding(0) var<uniform> params: SkyAtmosphereParams;"));
        // the single entry point, with the signature `Material` expects
        assert!(source.contains("@fragment\nfn fragment(in: VertexOutput) -> @location(0) vec4<f32> {"));
        // Bevy 0.15's `View` field is `world_position`; the pre-0.15
        // `view_world_position` silently reads garbage.
        assert!(source.contains("view.world_position"));
        assert!(!source.contains("view_world_position"));
        assert!(source.contains("view.exposure"));
        // NO FMA CONTRACTION: `a*b + c` must keep its two roundings
        assert!(!source.contains("fma("), "the shader must not contract a*b+c into an FMA");
        // both modes exist, and the miss path is fully transparent
        assert!(source.contains("if (params.mode == MODE_CLOSED_FORM) {"));
        assert_eq!(
            source.matches("return vec4<f32>(0.0, 0.0, 0.0, 0.0);").count(),
            2,
            "the degenerate-ray and the missed-atmosphere paths must both emit a transparent fragment"
        );
        // the premultiplied composite
        assert!(source.contains("return vec4<f32>(radiance, alpha);"));
        assert!(source.contains("return vec4<f32>(closed_form_radiance(ray_direction), 1.0);"));
        // D7: both density evaluations are floored at sea level
        assert!(source.contains("let sample_height = max(sample_radius - params.inner_radius, 0.0);"));
        assert!(source.contains("let light_height = max(light_radius - params.inner_radius, 0.0);"));
    }

    /// Stubs for the two `#import`s. naga has no preprocessor, so they are
    /// replaced by declarations of exactly the bindings the shader reads:
    /// `VertexOutput.world_position` (forward_io) and `view.world_position` /
    /// `view.exposure` (mesh_view_bindings). Everything else is the real shader
    /// text, so this validates the actual scattering code.
    const WGSL_IMPORT_STUBS: &str = "\
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
}

struct View {
    world_position: vec3<f32>,
    exposure: f32,
}

@group(0) @binding(0) var<uniform> view: View;
";

    fn stubbed_wgsl() -> String {
        let mut source = String::from(WGSL_IMPORT_STUBS);
        for line in wgsl().lines() {
            if line.starts_with("#import") {
                continue;
            }
            source.push_str(line);
            source.push('\n');
        }
        source
    }

    /// The strongest headless evidence that the shader is real: it is parsed and
    /// type-checked by **naga**, the same WGSL front end `bevy_render` compiles
    /// it with on the GPU path (naga 23.1, already in `Cargo.lock` as a
    /// transitive dependency). A literal device readback still needs xvfb —
    /// `.github/workflows/cesiumrust-e2e.yml` (M11.2).
    #[test]
    fn sky_atmosphere_wgsl_parses_and_type_checks_under_naga() {
        let source = stubbed_wgsl();
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("sky_atmosphere.wgsl does not parse:\n{}", error.emit_to_string(&source))
        });
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("sky_atmosphere.wgsl does not validate");

        let entry_points = module
            .entry_points
            .iter()
            .map(|entry| (entry.name.as_str(), entry.stage))
            .collect::<Vec<_>>();
        assert_eq!(
            entry_points,
            vec![("fragment", naga::ShaderStage::Fragment)],
            "the shader must expose exactly one fragment entry point"
        );
    }

    /// The uniform block is load-bearing: `encase` writes
    /// [`SkyAtmosphereParams`] positionally, so Rust and WGSL must agree field by
    /// field, offset by offset.
    #[test]
    fn wgsl_uniform_block_matches_the_rust_struct_field_by_field() {
        let rust_fields = [
            "sun_direction", "inner_radius", "rayleigh_scale_height", "mie_scale_height",
            "outer_radius", "pad0", "rayleigh_coefficient", "mie_coefficient",
            "mie_anisotropy", "solar_intensity", "mie_phase_k", "mode",
            "primary_steps_max", "light_steps_max", "pad1", "pad2",
        ];
        let offsets = [0u32, 12, 16, 20, 24, 28, 32, 44, 48, 52, 56, 60, 64, 68, 72, 76];

        // (1) parse the WGSL declarations and their commented std140 offsets
        let body = wgsl()
            .split("struct SkyAtmosphereParams {")
            .nth(1)
            .expect("sky_atmosphere.wgsl must declare struct SkyAtmosphereParams")
            .split("};")
            .next()
            .expect("the uniform struct must be closed");
        let mut parsed: Vec<(String, u32)> = Vec::new();
        for line in body.lines() {
            let (declaration, comment) = line.split_once("//").unwrap_or((line, ""));
            let declaration = declaration.trim();
            if declaration.is_empty() {
                continue;
            }
            let name = declaration.split(':').next().unwrap_or_default().trim();
            let offset = comment.trim().split(':').next().unwrap_or_default().trim();
            parsed.push((
                name.to_string(),
                offset.parse::<u32>().unwrap_or_else(|error| {
                    panic!("uniform field `{name}` carries no std140 offset comment (`{offset}`): {error}")
                }),
            ));
        }
        assert_eq!(parsed.len(), rust_fields.len(), "uniform field count: {parsed:?}");
        for (index, (name, offset)) in parsed.iter().enumerate() {
            // the WGSL pads are `_`-prefixed, the Rust ones are not
            assert_eq!(
                name.trim_start_matches('_'),
                rust_fields[index],
                "uniform field order/name mismatch at {index}: {parsed:?}"
            );
            assert_eq!(*offset, offsets[index], "std140 offset mismatch for `{}`", rust_fields[index]);
        }

        // (2) recompute those offsets from the field types, so the table above
        //     cannot silently drift from the std140 rules
        let sizes_aligns = [
            (12u32, 16u32), // vec3<f32>
            (4, 4), (4, 4), (4, 4), (4, 4), (4, 4),
            (12, 16), // vec3<f32>
            (4, 4), (4, 4), (4, 4), (4, 4), (4, 4), (4, 4), (4, 4), (4, 4),
        ];
        let mut cursor = 0u32;
        let mut computed = Vec::with_capacity(sizes_aligns.len());
        for (size, align) in sizes_aligns {
            cursor = cursor.div_ceil(align) * align;
            computed.push(cursor);
            cursor += size;
        }
        assert_eq!(computed.as_slice(), &offsets[..], "std140 offsets recomputed from the field types disagree");
        assert_eq!(cursor.div_ceil(16) * 16, 80, "the uniform block must be 80 bytes");
        assert!(
            wgsl().contains("// size 80, align 16"),
            "the WGSL block must document its 80-byte / 16-aligned size"
        );

        // (3) and let `encase` itself settle it — this is the layout the GPU
        //     actually receives
        assert_eq!(
            ShaderType::size(&gpu_params()).get(),
            80,
            "encase's std140 size for SkyAtmosphereParams must equal the WGSL block"
        );
    }

    /// The WGSL `const`s must be **bit-equal** to the Rust `const`s they mirror,
    /// otherwise the CPU reference tests above would be testing different maths
    /// than the shader runs.
    #[test]
    fn wgsl_constants_are_bit_equal_to_the_rust_constants() {
        fn const_literal(line_prefix: &str) -> String {
            let line = wgsl()
                .lines()
                .find(|line| line.starts_with(line_prefix))
                .unwrap_or_else(|| panic!("sky_atmosphere.wgsl has no `{line_prefix}`"));
            line.split('=')
                .nth(1)
                .unwrap_or_default()
                .trim()
                .trim_end_matches(';')
                .to_string()
        }

        for (name, rust_value) in [
            ("RAYLEIGH_PHASE_K", RAYLEIGH_PHASE_K),
            ("MIE_PHASE_K_BLUEPRINT", MIE_PHASE_K_BLUEPRINT),
            ("HORIZON_SPLIT_SHARPNESS", HORIZON_SPLIT_SHARPNESS),
        ] {
            let literal = const_literal(&format!("const {name}: f32 ="));
            let wgsl_value: f32 = literal
                .parse()
                .unwrap_or_else(|error| panic!("cannot parse `{name} = {literal}`: {error}"));
            assert_eq!(
                wgsl_value.to_bits(),
                rust_value.to_bits(),
                "{name} is not bit-equal between WGSL ({literal}) and Rust"
            );
        }
        for (name, rust_value) in [("MODE_RAYMARCH", MODE_RAYMARCH), ("MODE_CLOSED_FORM", MODE_CLOSED_FORM)] {
            let literal = const_literal(&format!("const {name}: u32 ="));
            let wgsl_value: u32 = literal
                .trim_end_matches('u')
                .parse()
                .unwrap_or_else(|error| panic!("cannot parse `{name} = {literal}`: {error}"));
            assert_eq!(wgsl_value, rust_value, "{name} differs between WGSL and Rust");
        }
        // the domain Mie normalisation is the *default uniform value*, so the
        // WGSL only mentions it in prose — but it must mention it
        assert!(
            wgsl().contains("0.07957747154594767"),
            "the WGSL must document the domain Mie normalisation it defaults to"
        );
        assert_eq!(MIE_PHASE_K_DOMAIN.to_bits(), ((1.0_f64 / (4.0 * std::f64::consts::PI)) as f32).to_bits());
    }

    // -- geometry / material / gate -------------------------------------------

    #[test]
    fn dome_radius_fits_between_the_camera_and_the_starfield() {
        let gpu = gpu_params();
        assert!(SKY_DOME_RADIUS > ORBIT_MAX_DISTANCE, "the camera must always be inside the dome");
        assert!(SKY_DOME_RADIUS < STARFIELD_RADIUS, "the dome must stay inside the starfield so the stars can be extinguished by it");
        assert!(SKY_DOME_RADIUS < CAMERA_FAR, "the dome must not be clipped by the far plane");
        assert!(SKY_DOME_RADIUS > gpu.outer_radius, "the dome must geometrically enclose the modelled atmosphere");
        // `Transparent3d::sort` is *ascending* on view-space Z (bevy_core_pipeline
        // 0.15 `core_3d/mod.rs` L515-517) and the camera looks down -Z, so
        // ascending == back-to-front and a *positive* bias means "drawn later".
        assert!(SKY_DOME_DEPTH_BIAS > 0.0, "the bias must be positive to sort the dome after the starfield");
        assert_eq!(sky_dome_cull_mode(), Some(Face::Front), "only the far shell may be rasterised");
    }

    #[test]
    fn material_properties_pin_the_render_order() {
        let material = SkyDomeMaterial { params: gpu_params() };
        assert_eq!(
            material.alpha_mode(),
            AlphaMode::Premultiplied,
            "Premultiplied -> depth_write_enabled = false + GreaterEqual, so the opaque globe occludes the dome"
        );
        assert_eq!(material.depth_bias(), SKY_DOME_DEPTH_BIAS);
        match SkyDomeMaterial::fragment_shader() {
            ShaderRef::Handle(handle) => assert_eq!(
                handle, SKY_ATMOSPHERE_SHADER_HANDLE,
                "the dome must use the headless-safe registered shader"
            ),
            ShaderRef::Path(path) => panic!("the dome must not load its shader from a path: {path:?}"),
            // naga_oil `#import` resolution; our shader is self-contained.
            ShaderRef::Default => panic!("the dome must not fall back to the default PBR shader"),
        }
    }

    #[test]
    fn dome_mesh_is_a_closed_shell_at_the_right_radius() {
        let mesh = build_sky_dome_mesh_asset();
        let VertexAttributeValues::Float32x3(positions) = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("the dome mesh must carry positions")
        else {
            panic!("dome positions must be Float32x3");
        };
        assert!(
            positions.len() > 1_000,
            "ico({SKY_DOME_SUBDIVISIONS}) must give a smooth limb, got {} vertices",
            positions.len()
        );
        let mut worst_radius = 0.0_f32;
        for position in positions {
            worst_radius = worst_radius.max((Vec3::from_slice(position).length() - SKY_DOME_RADIUS).abs());
        }
        assert!(
            worst_radius < 1.0e-3,
            "every vertex must sit on the {SKY_DOME_RADIUS} ru sphere, worst off by {worst_radius}"
        );
        // outward normals are what make `Face::Front` cull the *near* hemisphere
        let VertexAttributeValues::Float32x3(normals) = mesh
            .attribute(Mesh::ATTRIBUTE_NORMAL)
            .expect("the dome mesh must carry normals")
        else {
            panic!("dome normals must be Float32x3");
        };
        assert_eq!(normals.len(), positions.len());
        let mut worst_dot = 1.0_f32;
        for (position, normal) in positions.iter().zip(normals.iter()) {
            let radial = Vec3::from_slice(position).normalize();
            worst_dot = worst_dot.min(radial.dot(Vec3::from_slice(normal)));
        }
        assert!(worst_dot > 0.999, "normals must be radial, worst dot {worst_dot}");
        assert_eq!(mesh.primitive_topology(), PrimitiveTopology::TriangleList);
        println!(
            "sky dome mesh: {} vertices, worst radius error {worst_radius:.3e}, worst normal dot {worst_dot:.6}",
            positions.len()
        );
    }

    #[test]
    fn gate_parsing_matches_feature_flags_without_touching_the_environment() {
        // `sky_dome_gate_enabled()` is `gate_from_env_value(env::var(..))`, and
        // `feature_flags::env_flag` (application layer, unreadable from an
        // adapter) accepts exactly the same truthy set. Asserting the *pure*
        // function covers both without mutating the process environment, which
        // the parallel test threads share.
        for (raw, expected) in [
            (None, false),
            (Some(""), false),
            (Some("0"), false),
            (Some("false"), false),
            (Some("no"), false),
            (Some("off"), false),
            (Some("anything else"), false),
            (Some("1"), true),
            (Some("true"), true),
            (Some("yes"), true),
            (Some("on"), true),
            (Some(" TRUE "), true),
            (Some("On"), true),
            (Some("YeS"), true),
        ] {
            assert_eq!(
                gate_from_env_value(raw.map(str::to_string)),
                expected,
                "CESIUM_ENABLE_SKYDOME={raw:?}"
            );
        }
        assert_eq!(ENV_ENABLE_SKYDOME, "CESIUM_ENABLE_SKYDOME");
        // reading the real variable must not panic, whatever it is set to
        let _ = sky_dome_gate_enabled();
    }

    // -- the production ray march ---------------------------------------------

    /// A camera just above the surface on the sunlit side, looking tangentially
    /// (90 deg scattering).
    fn sunlit_surface_setup() -> (SkyAtmosphereParams, Vec3, Vec3) {
        let params = gpu_params();
        let origin = params.sun_direction * (params.inner_radius + 0.001);
        let tangent = params.sun_direction.cross(Vec3::Z);
        let direction = if tangent.length_squared() < 1.0e-8 {
            Vec3::X
        } else {
            tangent.normalize()
        };
        (params, origin, direction)
    }

    /// In the optically thin limit the channel ordering is set by the Rayleigh
    /// coefficients alone (`beta_b : beta_g : beta_r = 33.1 : 13.5 : 5.8`), and
    /// in the real (thick) limit the transmittance is still wavelength-ordered.
    #[test]
    fn raymarch_orders_the_channels_by_wavelength() {
        let (params, origin, direction) = sunlit_surface_setup();

        // optically thin, Mie switched off: two-way extinction ~ 1, so the ratio
        // must reproduce the coefficient ratio
        let mut thin = params;
        thin.rayleigh_coefficient *= 1.0e-3;
        thin.mie_coefficient = 0.0;
        let thin = shade_sky_f32(origin, direction, SKY_DOME_RADIUS, &thin, 1.0)
            .expect("the thin ray must still intersect the shell");
        assert!(thin.radiance.is_finite());
        assert!(thin.radiance.z > thin.radiance.y, "blue must dominate in the thin limit, got {:?}", thin.radiance);
        assert!(thin.radiance.y > thin.radiance.x, "green must sit between blue and red, got {:?}", thin.radiance);
        let got = f64::from(thin.radiance.z / thin.radiance.x);
        let expected = 33.1 / 5.8;
        assert!(
            rel_err(got, expected) < 2.0e-2,
            "thin-limit blue/red = {got}, expected the coefficient ratio {expected}"
        );

        // the real coefficients: extinction is wavelength-selective, so blue is
        // extinguished most
        let thick = shade_sky_f32(origin, direction, SKY_DOME_RADIUS, &params, 1.0)
            .expect("the thick ray must still intersect the shell");
        assert!(thick.radiance.is_finite() && thick.transmittance.is_finite());
        assert!(thick.radiance.min_element() >= 0.0, "in-scattered radiance cannot be negative: {:?}", thick.radiance);
        assert!(
            thick.transmittance.x > thick.transmittance.y
                && thick.transmittance.y > thick.transmittance.z,
            "transmittance must fall with wavelength-selective extinction: {:?}",
            thick.transmittance
        );
        for channel in [thick.transmittance.x, thick.transmittance.y, thick.transmittance.z] {
            assert!((0.0..=1.0).contains(&channel), "transmittance must be a fraction, got {channel}");
        }
        assert!((0.0..=1.0).contains(&thick.alpha));
        // alpha is exactly the complement of the mean transmittance (D3)
        let mean = thick.transmittance.dot(Vec3::splat(1.0 / 3.0));
        assert!((thick.alpha - (1.0 - mean).clamp(0.0, 1.0)).abs() < 1.0e-6);
        println!(
            "90 deg scattering: thin radiance {:?}, thick radiance {:?}, transmittance {:?}, alpha {:.6}",
            thin.radiance, thick.radiance, thick.transmittance, thick.alpha
        );
    }

    /// The D7 regression guard. A primary ray that pierces the Earth puts the
    /// sample points below the surface, where `-h/H` reaches +797 and
    /// `exp(+797)` is `inf` in f32. Without the sea-level floor the accumulation
    /// becomes `inf * exp(-inf)` = `inf * 0` = **NaN**, which is
    /// non-deterministic on the GPU and would break every baseline.
    #[test]
    fn raymarch_through_the_earth_stays_finite() {
        let params = gpu_params();
        let origin = params.sun_direction * (params.inner_radius + 0.001);
        let direction = -params.sun_direction; // straight down, through the planet
        let shading = shade_sky_f32(origin, direction, SKY_DOME_RADIUS, &params, 1.0)
            .expect("the origin is inside the shell, so the ray must intersect it");
        let march = shading.march;
        for value in [
            shading.radiance.x, shading.radiance.y, shading.radiance.z,
            shading.transmittance.x, shading.transmittance.y, shading.transmittance.z,
            shading.alpha, march.optical_depth.x, march.optical_depth.y,
            march.rayleigh_accumulation.x, march.rayleigh_accumulation.z,
            march.mie_accumulation.y,
        ] {
            assert!(value.is_finite(), "D7 regression: NaN/inf escaped the march");
        }
        assert!(march.optical_depth.x > 0.0 && march.optical_depth.y > 0.0);
        assert_eq!(
            shading.transmittance,
            Vec3::ZERO,
            "a ray through the whole atmosphere plus the planet transmits nothing"
        );
        assert!((shading.alpha - 1.0).abs() < 1.0e-6);
        assert!(
            shading.radiance.length() < 1.0e-3,
            "the fully shadowed ray must be black, got {:?}",
            shading.radiance
        );
    }

    /// The night side: the sun is below the local horizon, so every light ray
    /// crosses the Earth and the two-way extinction saturates to zero.
    #[test]
    fn raymarch_on_the_night_side_is_dark_and_the_day_side_is_bright() {
        let params = gpu_params();
        let night_up = -params.sun_direction;
        let night_origin = night_up * (params.inner_radius + 0.001);
        let night = shade_sky_f32(night_origin, night_up, SKY_DOME_RADIUS, &params, 1.0)
            .expect("the night-side zenith ray must still intersect the shell");
        assert!(night.radiance.is_finite(), "got {:?}", night.radiance);
        assert!(
            night.radiance.length() < 1.0e-2,
            "the night sky must be dark, got {:?}",
            night.radiance
        );

        let day_up = params.sun_direction;
        let day_origin = day_up * (params.inner_radius + 0.001);
        let day = shade_sky_f32(day_origin, day_up, SKY_DOME_RADIUS, &params, 1.0)
            .expect("the sub-solar zenith ray must still intersect the shell");
        assert!(day.radiance.is_finite(), "got {:?}", day.radiance);
        assert!(
            day.radiance.length() > night.radiance.length(),
            "the day side must be brighter than the night side: day {:?} vs night {:?}",
            day.radiance,
            night.radiance
        );
        assert!(day.radiance.length() > 1.0e-3, "the sub-solar zenith must be lit, got {:?}", day.radiance);
        println!("day zenith radiance {:?}, night zenith radiance {:?}", day.radiance, night.radiance);
    }

    #[test]
    fn raymarch_rejects_rays_that_never_enter_the_atmosphere() {
        let params = gpu_params();
        // (a) a genuine geometric miss
        let outside = Vec3::new(0.0, 0.0, SKY_DOME_RADIUS);
        assert!(march_single_scattering_f32(outside, Vec3::X, SKY_DOME_RADIUS, &params).is_none());
        assert!(shade_sky_f32(outside, Vec3::X, SKY_DOME_RADIUS, &params, 1.0).is_none());
        // (b) outside and moving away: the infinite line still intersects, but
        //     wholly behind the origin, so `stop = min(interval.y, length)`
        //     rejects it (B1 L56-59)
        assert!(march_single_scattering_f32(Vec3::new(0.0, 0.0, 3.0), Vec3::Z, 39.0, &params).is_none());
        // (c) a degenerate zero-length ray
        let inside = params.sun_direction * (params.inner_radius + 0.001);
        assert!(march_single_scattering_f32(inside, params.sun_direction, 0.0, &params).is_none());
    }

    /// B1 L66-67 with the D2 rewrite: the step budget shrinks as the camera
    /// sinks into the atmosphere, and the `max(1, ..)` clamp means none of the
    /// divisions below it can ever be by zero.
    #[test]
    fn raymarch_step_budget_shrinks_as_the_camera_descends() {
        let params = gpu_params();
        let up = params.sun_direction;
        let mut previous_steps = PRIMARY_STEPS_MAX as i32 + 1;
        let mut previous_weight = f32::NEG_INFINITY;
        for height_in_thicknesses in [100.0_f32, 10.0, 3.0, 1.0, 0.3, 0.1, 0.0] {
            let origin = up * (params.inner_radius + height_in_thicknesses * params.thickness());
            let march = march_single_scattering_f32(origin, -up, SKY_DOME_RADIUS, &params)
                .unwrap_or_else(|| {
                    panic!("a ray from {height_in_thicknesses} thicknesses up, aimed at the Earth, must hit the shell")
                });
            assert!(march.primary_steps >= 1 && march.light_steps >= 1, "the L66-67 clamp must hold");
            assert!(march.primary_steps <= PRIMARY_STEPS_MAX as i32);
            assert!(march.light_steps <= LIGHT_STEPS_MAX as i32);
            assert!((0.0..=1.0).contains(&march.w_inside_atmosphere));
            assert!(
                march.primary_steps <= previous_steps,
                "the budget must shrink monotonically as the camera descends: {} after {}",
                march.primary_steps,
                previous_steps
            );
            assert!(march.w_inside_atmosphere >= previous_weight);
            previous_steps = march.primary_steps;
            previous_weight = march.w_inside_atmosphere;
        }
        // at the surface `w_inside_atmosphere == 0.5` exactly, so B1 L66-67 give
        // 16 - int(0.5*12) = 10 primary steps
        assert_eq!(previous_steps, 10, "at the surface the D2 weight must be exactly 0.5");
        assert!((previous_weight - 0.5).abs() < 1.0e-6);

        // high above the atmosphere the full B1 budget is spent
        let far = up * (params.inner_radius + 100.0 * params.thickness());
        let march = march_single_scattering_f32(far, -up, SKY_DOME_RADIUS, &params)
            .expect("a ray from far above, aimed at the Earth, must hit the shell");
        assert_eq!(
            (march.primary_steps, march.light_steps),
            (PRIMARY_STEPS_MAX as i32, LIGHT_STEPS_MAX as i32),
            "B1 L26-27 PRIMARY_STEPS_MAX / LIGHT_STEPS_MAX must be reachable"
        );
        assert!(march.w_inside_atmosphere < 1.0e-6);
    }

    #[test]
    fn transmittance_and_alpha_stay_in_range_for_every_view_direction() {
        let params = gpu_params();
        let origin = params.sun_direction * (params.inner_radius + 0.001);
        let helper = if params.sun_direction.z.abs() < 0.9 { Vec3::Z } else { Vec3::X };
        let u = params.sun_direction.cross(helper).normalize();
        let mut hits = 0;
        for degree in 0..=180 {
            let theta = (f64::from(degree).to_radians()) as f32;
            let direction = (params.sun_direction * theta.cos() + u * theta.sin()).normalize();
            let Some(shading) = shade_sky_f32(origin, direction, SKY_DOME_RADIUS, &params, 1.0) else {
                continue;
            };
            hits += 1;
            assert!(shading.radiance.is_finite() && shading.transmittance.is_finite());
            assert!(
                shading.radiance.min_element() >= 0.0,
                "in-scattered radiance cannot be negative at {degree} deg: {:?}",
                shading.radiance
            );
            for channel in [shading.transmittance.x, shading.transmittance.y, shading.transmittance.z] {
                assert!((0.0..=1.0).contains(&channel), "transmittance must be a fraction at {degree} deg: {channel}");
            }
            assert!((0.0..=1.0).contains(&shading.alpha), "alpha must be a fraction at {degree} deg: {}", shading.alpha);
        }
        assert_eq!(hits, 181, "every direction from a camera inside the shell must intersect it");
    }

    // -- the v2_sky baselines -------------------------------------------------

    /// Builds an `orbit_camera`-style pose: the camera sits `distance` render
    /// units along `zenith` and looks at the world origin. `up_hint` fixes the
    /// roll (Gram-Schmidt-orthogonalised against `zenith` first).
    fn look_at_origin_pose(zenith: DVec3, up_hint: DVec3, distance: f64) -> (DVec3, Quat) {
        let back = zenith.normalize();
        let position = back * distance;
        let up = (up_hint - up_hint.dot(back) * back).normalize();
        let right = up.cross(back);
        let rotation = Quat::from_mat3(&Mat3::from_cols(vec3_of(right), vec3_of(up), vec3_of(back)));
        (position, rotation)
    }

    /// The three v2_sky lighting regimes as `(zenith, up_hint)` pairs relative to
    /// the frozen sun. The regime *is* the sun's elevation above the camera's
    /// local horizon, i.e. `dot(sun, zenith)` — because `orbit_camera` always
    /// looks at the origin, and therefore always looks along `-zenith`.
    fn regime_axes(regime: &str, sun: DVec3) -> (DVec3, DVec3) {
        match regime {
            // sun at the local zenith (+90 deg)
            "noon" => (sun, DVec3::Z),
            // sun exactly on the local horizon (0 deg); `right` comes out equal
            // to `sun`, so the sunset sits on the frame's right edge
            "dusk" => {
                let horizontal = (DVec3::Z - DVec3::Z.dot(sun) * sun).normalize();
                (horizontal, horizontal.cross(sun))
            }
            // sun at the nadir (-90 deg): the camera stands on the anti-solar
            // point and the planet shadows every light ray
            "night" => (-sun, DVec3::Z),
            other => unreachable!("unknown v2_sky regime {other}"),
        }
    }

    /// The six `specs/scripts/v2_sky.toml` shots in file order: the three
    /// regimes at the standard orbit distance, then the same three at the wide
    /// distance. Frames follow the v1_postfx/v2_fxaa 180-frame cadence.
    fn baseline_shots() -> Vec<(String, u32, f64, DVec3, Quat)> {
        let sun = frozen_sun();
        let mut shots = Vec::with_capacity(6);
        let mut frame = 180_u32;
        for distance in [3.0_f64, 8.0] {
            for regime in ["noon", "dusk", "night"] {
                let (zenith, up_hint) = regime_axes(regime, sun);
                let (position, rotation) = look_at_origin_pose(zenith, up_hint, distance);
                shots.push((format!("sky_{regime}_{}", distance as u32), frame, distance, position, rotation));
                frame += 180;
            }
        }
        shots
    }

    /// The six poses as transcribed into `specs/scripts/v2_sky.toml`. Keeping the
    /// numbers here too locks the TOML and the code together: if either drifts,
    /// this test fails and says which shot.
    const BASELINE_SHOT_TABLE: [(&str, u32, [f64; 3], [f32; 4]); 6] = [
        ("sky_noon_3", 180, [0.523503, -2.710279, -1.174875], [0.0, 0.0, 0.0, 1.0]),
        ("sky_dusk_3", 360, [0.222810, -1.154279, -0.480657], [0.0, 0.0, 0.0, 1.0]),
        ("sky_night_3", 540, [-0.523503, 2.710279, 1.174875], [0.0, 0.0, 0.0, 1.0]),
        ("sky_noon_8", 720, [1.396007, -7.227412, -3.133000], [0.0, 0.0, 0.0, 1.0]),
        ("sky_dusk_8", 900, [0.594159, -3.078077, -1.281752], [0.0, 0.0, 0.0, 1.0]),
        ("sky_night_8", 1080, [-1.396007, 7.227412, 3.133000], [0.0, 0.0, 0.0, 1.0]),
    ];

    #[test]
    fn sky_baseline_poses_encode_the_three_lighting_regimes() {
        let sun = frozen_sun();
        let sun32 = vec3_of(sun);
        let shots = baseline_shots();
        assert_eq!(shots.len(), 6);

        for (index, (label, frame, distance, position, rotation)) in shots.iter().enumerate() {
            assert_eq!(*frame, 180 * (index as u32 + 1), "{label} must be captured on frame {frame}");
            assert!((position.length() - *distance).abs() < 1.0e-9, "{label} distance");

            // orbit_camera looks at the world origin
            let forward = *rotation * Vec3::NEG_Z;
            let to_origin = (-vec3_of(*position)).normalize();
            assert!(
                forward.dot(to_origin) > 0.9999,
                "{label} must look at the world origin, dot {}",
                forward.dot(to_origin)
            );

            // the regime is the sun elevation above the camera's local horizon
            let elevation = vec3_of(*position).normalize().dot(sun32);
            let expected = match label.split('_').nth(1).unwrap_or_default() {
                "noon" => 1.0_f32,
                "dusk" => 0.0,
                "night" => -1.0,
                other => unreachable!("{other}"),
            };
            assert!(
                (elevation - expected).abs() < 1.0e-5,
                "{label}: the sun elevation must be {expected}, got {elevation}"
            );

            // ...and the pose is the one v2_sky.toml carries
            let (toml_label, toml_frame, toml_pos, toml_quat) = BASELINE_SHOT_TABLE[index];
            assert_eq!(toml_label, label.as_str(), "v2_sky.toml shot {index} name drifted");
            assert_eq!(toml_frame, *frame, "v2_sky.toml shot {index} frame drifted");
            for (axis, want) in [position.x, position.y, position.z].iter().zip(toml_pos) {
                assert!((axis - want).abs() < 1.0e-6, "{label}: v2_sky.toml pos drifted");
            }
            for (component, want) in [rotation.x, rotation.y, rotation.z, rotation.w].iter().zip(toml_quat) {
                assert!((component - want).abs() < 1.0e-6, "{label}: v2_sky.toml quat drifted");
            }

            println!(
                "[[shot]]\nframe = {frame}\nname = \"{label}\"\npos = [{:.6}, {:.6}, {:.6}]\nquat = [{:.6}, {:.6}, {:.6}, {:.6}]\nfov_y = 60.0\n",
                position.x, position.y, position.z,
                rotation.x, rotation.y, rotation.z, rotation.w
            );
        }

        // "dusk" additionally puts the sun on the frame's right axis
        for (label, _, _, _, rotation) in shots.iter().filter(|shot| shot.0.contains("dusk")) {
            let right = *rotation * Vec3::X;
            assert!(
                right.dot(sun32) > 0.9999,
                "{label}: the sunset must sit on the frame's right edge, dot {}",
                right.dot(sun32)
            );
        }
    }

    // -- headless app integration --------------------------------------------

    /// A headless app with the plugin installed and **no** asset backend,
    /// mirroring `terrain/mod.rs`'s standalone-plugin test.
    fn headless_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CesiumAtmospherePlugin);
        app.world_mut().insert_resource(AnimationClock::default());
        app
    }

    fn dome_count(app: &App) -> usize {
        app.world()
            .iter_entities()
            .filter(|entity| entity.contains::<SkyDome>())
            .count()
    }

    fn dome_material(app: &App) -> Handle<SkyDomeMaterial> {
        let mut found = None;
        for entity in app.world().iter_entities() {
            if let Some(material) = entity.get::<MeshMaterial3d<SkyDomeMaterial>>() {
                found = Some(material.0.clone());
            }
        }
        found.expect("the dome must carry a SkyDomeMaterial")
    }

    /// The pre-M5-C CPU sky, transcribed verbatim from `sky_system`'s gate-OFF
    /// branch (including its no-camera `view_dir == sun_dir` fallback), so that
    /// branch has an independent oracle instead of being compared against itself.
    fn expected_clear_color(sun_direction: &Vec3, params: &AtmosphereParameters) -> Color {
        let sun_dir = dvec3_of(*sun_direction);
        let view_dir = sun_dir; // headless: `camera_query.get_single()` fails
        let sky_color = compute_sky_color(view_dir, sun_dir, 1000.0, params);
        let horizon_glow = compute_horizon_glow(sun_dir.z);
        let r = (sky_color[0] as f32 * 0.3 + horizon_glow[0] as f32 * 0.3).clamp(0.0, 1.0);
        let g = (sky_color[1] as f32 * 0.3 + horizon_glow[1] as f32 * 0.3).clamp(0.0, 1.0);
        let b = (sky_color[2] as f32 * 0.3 + horizon_glow[2] as f32 * 0.3).clamp(0.0, 1.0);
        Color::srgb(r, g, b)
    }

    /// The dev-005 / dev-011 class defect guard: `CesiumAtmospherePlugin` must
    /// build and run frames under `MinimalPlugins` — no `AssetPlugin`, no
    /// `RenderPlugin`, no `Assets<Shader>` — without panicking. That is what
    /// `shader_registry::try_load_internal_shader` and the
    /// `Option<ResMut<Assets<_>>>` system parameters buy.
    #[test]
    fn plugin_builds_and_runs_frames_headlessly() {
        let mut app = headless_app();
        app.update();
        app.update();
        assert!(
            app.world().contains_resource::<ClearColor>(),
            "the plugin must own ClearColor so it cannot be used without it"
        );
        assert_eq!(
            app.world().resource::<SkyAtmosphere>().dome,
            sky_dome_gate_enabled(),
            "the plugin must seed SkyAtmosphere::dome from CESIUM_ENABLE_SKYDOME"
        );
        // no asset backend -> MaterialPlugin was skipped -> no dome can be
        // spawned, and the setup system must degrade to a no-op
        assert!(!crate::shader_registry::asset_backend_available(&app));
        assert_eq!(dome_count(&app), 0);
    }

    /// Gate OFF: `sky_system` must reproduce the pre-M5-C CPU `ClearColor` sky
    /// **byte for byte**. This is the "gate OFF -> the eight v0 baselines are
    /// zero-diff" red line.
    #[test]
    fn gate_off_reproduces_the_pre_m5c_clear_color_sky_byte_for_byte() {
        let mut app = headless_app();
        app.world_mut().resource_mut::<SkyAtmosphere>().dome = false;
        app.update();

        let lighting = app.world().resource::<LightingParams>().clone();
        let sky = app.world().resource::<SkyAtmosphere>().clone();
        let clear = app.world().resource::<ClearColor>().0;

        // under the frozen AnimationClock epoch the sun is exactly the v2_sky
        // baseline sun, which is what makes the capture deterministic
        let expected_sun = vec3_of(frozen_sun());
        assert!(
            (lighting.sun_direction - expected_sun).length() < 1.0e-6,
            "celestial_system must publish the frozen baseline sun, got {:?} want {:?}",
            lighting.sun_direction,
            expected_sun
        );

        let expected = expected_clear_color(&lighting.sun_direction, &sky.atmosphere_params);
        assert_eq!(clear, expected, "the gate-OFF sky must be byte-for-byte the pre-M5-C ClearColor");
        assert_ne!(clear, Color::BLACK, "the CPU sky must actually paint something");
        println!("gate-OFF ClearColor = {clear:?}");
    }

    /// Gate ON: the WGSL owns the sky, so the CPU ClearColor branch must not run
    /// at all — otherwise the dome composite is painted twice.
    #[test]
    fn gate_on_leaves_the_clear_color_alone() {
        let mut app = headless_app();
        app.world_mut().resource_mut::<SkyAtmosphere>().dome = true;
        let before = app.world().resource::<ClearColor>().0;
        app.update();
        assert_eq!(
            app.world().resource::<ClearColor>().0,
            before,
            "with the dome on, sky_system must not touch ClearColor"
        );
        // and the gate-OFF value differs, proving the two branches really split
        app.world_mut().resource_mut::<SkyAtmosphere>().dome = false;
        app.update();
        assert_ne!(
            app.world().resource::<ClearColor>().0,
            before,
            "with the dome off the CPU sky must take over"
        );
    }

    /// With an asset backend present the plugin registers
    /// `MaterialPlugin::<SkyDomeMaterial>`, `sky_dome_setup` spawns exactly one
    /// dome seeded from the domain parameters, `sky_system` pushes the frozen sun
    /// uniform (and then stops dirtying it, which is what keeps `FIXED_TIME`
    /// captures bit-reproducible), and flipping the gate off tears it down.
    #[test]
    fn sky_dome_setup_is_idempotent_and_tears_down() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());
        app.add_plugins(CesiumAtmospherePlugin);
        app.world_mut().insert_resource(AnimationClock::default());
        app.world_mut().resource_mut::<SkyAtmosphere>().dome = true;

        assert!(
            app.world().contains_resource::<Assets<SkyDomeMaterial>>(),
            "with an AssetServer the plugin must register MaterialPlugin::<SkyDomeMaterial>"
        );

        app.update();
        app.update(); // a second frame must not add a second dome
        assert_eq!(dome_count(&app), 1, "exactly one dome must exist");

        let handle = dome_material(&app);
        {
            let materials = app.world().resource::<Assets<SkyDomeMaterial>>();
            let params = &materials.get(&handle).expect("the dome material must exist").params;
            assert_eq!(params.inner_radius, 1.0);
            assert_eq!(params.mode, MODE_RAYMARCH, "production must ray-march, not use the parity probe");
            assert_eq!(params.primary_steps_max, PRIMARY_STEPS_MAX);
            assert_eq!(params.light_steps_max, LIGHT_STEPS_MAX);
            let expected_sun = vec3_of(frozen_sun());
            assert!(
                (params.sun_direction - expected_sun).length() < 1.0e-6,
                "sky_system must push the frozen sun, got {:?}",
                params.sun_direction
            );
        }
        // re-pushing the same direction must not dirty the material again
        {
            let mut materials = app.world_mut().resource_mut::<Assets<SkyDomeMaterial>>();
            let expected_sun = vec3_of(frozen_sun());
            assert!(
                !update_sun_direction(&mut materials, &handle, expected_sun),
                "FIXED_TIME must leave the bind group untouched after the first frame"
            );
            assert!(update_sun_direction(&mut materials, &handle, -expected_sun), "a real change must go through");
            assert!(
                !update_sun_direction(&mut materials, &handle, expected_sun * 7.5),
                "a rescaled direction normalises to the value already stored"
            );
        }

        // tear-down
        app.world_mut().resource_mut::<SkyAtmosphere>().dome = false;
        app.update();
        assert_eq!(dome_count(&app), 0, "gate OFF must despawn the dome");
    }
}



