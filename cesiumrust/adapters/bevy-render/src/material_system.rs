//! Bridge domain Material → Bevy FabricMaterial.
//!
//! Provides the [`CesiumMaterialPlugin`] which reads entity components and
//! applies Fabric procedural materials to Bevy meshes, plus per-frame uniform
//! updates (e.g. water animation time).

// legacy CesiumJS-port style debt (deferred.md #18); revisit at M13 lint-cleanup 或本文件在其里程碑被重写时
#![allow(clippy::type_complexity)]
use bevy::prelude::*;
use cesium_material::{MaterialSystem, UniformValue};

use crate::fabric_material::{
    fabric_material_from_domain, fabric_material_from_domain_with_maps, generate_water_normal_map,
    generate_water_specular_map, FabricKind, FabricMaterial, FabricMaterialPlugin,
};
use cesium_shadow::{OceanConfig, OceanSurface};

/// Component that references a CesiumJS Fabric material to apply to an entity.
///
/// Attach this to any entity with a [`MeshMaterial3d<FabricMaterial>`] target
/// (or insert [`FabricMaterial`] directly) to have the material generated from
/// the domain layer.
#[derive(Component, Clone, Debug)]
pub struct MaterialRef {
    /// The CesiumJS material type name (e.g. `"ElevationContour"`,
    /// `"RimLighting"`, `"Color"`).
    pub type_name: String,
    /// Optional uniform overrides (keyed by Fabric uniform name).
    pub uniforms: std::collections::BTreeMap<String, UniformValue>,
}

impl MaterialRef {
    /// Create a material reference from a type name with default uniforms.
    pub fn new(type_name: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            uniforms: std::collections::BTreeMap::new(),
        }
    }

    /// Create with uniform overrides.
    pub fn with_uniforms(
        type_name: impl Into<String>,
        uniforms: std::collections::BTreeMap<String, UniformValue>,
    ) -> Self {
        Self {
            type_name: type_name.into(),
            uniforms,
        }
    }
}

/// Resource holding time for animated materials (e.g. Water).
#[derive(Resource, Default)]
pub struct MaterialAnimationTime {
    /// Accumulated delta-seconds since startup (legacy wall-clock field).
    pub time: f32,
    /// Monotonic frame counter mirroring CesiumJS `czm_frameNumber`
    /// (Water.glsl L18: `time = czm_frameNumber * animationSpeed`).
    /// DEVIATION: Water animation now advances per *frame* (matching CesiumJS)
    /// rather than per accumulated second; see docs/deviations.md#dev-019.
    /// FROZEN under FIXED_TIME (`delta_secs() == 0.0`) — see [`advance_animation`].
    pub frame_number: u32,
    /// The `frame_number` last published into Water's `extra_c.z` (Ryan L6).
    /// `None` until the first write; equal to the current frame ⇒ the write is
    /// skipped so the bind group is not needlessly re-encoded.
    last_written_frame: Option<u32>,
}

/// Water animation frame period (Ryan L4).
///
/// `extra_c.z` is written as `frame_number as f32`. f32 represents every integer
/// exactly only up to 2^24 (16_777_216 ≈ 3.2 days @ 60 fps); past that the
/// counter stops advancing in f32 and the Water animation freezes, and the `u32`
/// itself wraps after ~2.3 years. Water.glsl consumes `time` only through
/// `fract()` (czm_get_water_noise multiplies `time` by the sampling directions
/// then `fract`s the UVs), so the visible motion is periodic and the counter can
/// be reduced modulo a period far below 2^24. 2^20 (1_048_576 ≈ 4.8 h @ 60 fps)
/// keeps `extra_c.z` exactly representable and bounds the wrap to one
/// imperceptible phase step.
pub const WATER_FRAME_PERIOD: u32 = 1 << 20;

/// Advance the material animation clock by `delta_secs`, returning the frame
/// counter to publish into Water's `extra_c.z`.
///
/// Ryan M3 / dev-019 方案 A: under FIXED_TIME (`delta_secs() == 0.0`, which
/// `main.rs` sets via `TimeUpdateStrategy::ManualDuration(ZERO)`) the frame
/// counter is FROZEN, so the Water phase — and therefore every captured baseline
/// — is bit-reproducible instead of drifting with the number of Update ticks that
/// ran before the screenshot. The three Water close-ups stay distinguishable
/// because they differ by preset (amplitude/frequency), not by phase.
fn advance_animation(anim: &mut MaterialAnimationTime, delta_secs: f32) -> u32 {
    anim.time += delta_secs;
    if delta_secs != 0.0 {
        anim.frame_number = anim.frame_number.wrapping_add(1);
    }
    anim.frame_number
}

/// Cached handles of every [`FabricMaterial`] whose `kind == Water` (Ryan L6).
///
/// `update_material_uniforms` used to `iter_mut()` the whole
/// `Assets<FabricMaterial>` every frame; each `Mut<T>` drop unconditionally flags
/// the asset changed, so all Water materials were re-dirtied → `as_bind_group`
/// re-encoded the 192 B uniform and rebuilt the bind group every frame. We cache
/// the Water handles here and `get_mut` only those, and only when the published
/// frame actually changed.
///
/// The cache self-heals: it is rebuilt (via a non-dirtying immutable `iter()`)
/// whenever `Assets<FabricMaterial>::len()` changes, so Water materials spawned
/// directly by the application showcase — which bypass `apply_fabric_materials` —
/// are picked up too.
#[derive(Resource, Default)]
pub struct WaterMaterialHandles {
    handles: Vec<Handle<FabricMaterial>>,
    known_len: usize,
}

/// Shared Water normal/specular maps for the DEFAULT [`OceanConfig`] (Daniel L4).
///
/// `apply_fabric_materials` used to build a fresh
/// `OceanSurface::new(OceanConfig::default())` plus two 128×128 (64 KB each)
/// procedural textures for *every* Water entity — O(N) byte-identical texture
/// pairs for N water entities, since the default config is constant. The first
/// Water entity generates the pair once here; later default-config entities reuse
/// the same handles. Non-default configs (the showcase's Calm/Rough presets) still
/// generate per-entity.
#[derive(Resource, Default)]
pub struct SharedWaterTextures {
    normal_map: Option<Handle<Image>>,
    specular_map: Option<Handle<Image>>,
}

/// Plugin bridging domain [`cesium_material::Material`] → Bevy [`FabricMaterial`].
///
/// Adds systems that:
/// - Apply [`MaterialRef`] components to entities as [`FabricMaterial`] instances.
/// - Update animated uniform values per-frame (water time, etc.).
pub struct CesiumMaterialPlugin;

impl Plugin for CesiumMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FabricMaterialPlugin)
            .init_resource::<MaterialAnimationTime>()
            .init_resource::<WaterMaterialHandles>()
            .init_resource::<SharedWaterTextures>();

        // The two Update systems dereference `Assets<Image>` /
        // `Assets<FabricMaterial>` and `Time`, all absent under a headless
        // `MinimalPlugins` app (no AssetPlugin) — running them there panics. Gate
        // registration on the same `asset_backend_available` predicate
        // `FabricMaterialPlugin` already uses for `MaterialPlugin`, so headless apps
        // (and the Mark M-3 showcase smoke test) install the plugin as a silent
        // no-op while the GPU path is unchanged (pixel-neutral).
        if crate::shader_registry::asset_backend_available(app) {
            app.add_systems(
                Update,
                (apply_fabric_materials, update_material_uniforms),
            );
        }
    }
}

/// System: for entities with a [`MaterialRef`] component, look up the material
/// type in the domain [`MaterialSystem`], extract uniform values, and create a
/// Bevy [`FabricMaterial`] instance.
///
/// This runs only when a [`MaterialRef`] is added or changed (via `Changed<MaterialRef>`).
fn apply_fabric_materials(
    mut commands: Commands,
    material_system: Option<Res<MaterialSystemResource>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<FabricMaterial>>,
    mut shared_water: ResMut<SharedWaterTextures>,
    query: Query<(Entity, &MaterialRef, Option<&MeshMaterial3d<FabricMaterial>>), Changed<MaterialRef>>,
) {
    let system = match &material_system {
        Some(res) => &res.0,
        None => {
            warn!("MaterialSystemResource not available; skipping material application");
            return;
        }
    };

    // Nothing added/changed this frame → skip. Avoids allocating a fresh
    // fallback image into `Assets<Image>` every frame (M5-D).
    if query.is_empty() {
        return;
    }

    // Create a 1x1 white fallback image for materials that need a texture.
    // Materials that don't use textures (Color, etc.) ignore this binding.
    // NOTE: Rgba8UnormSrgb is correct for this *colour* fallback (imagery-like),
    // but Water's normalMap/specularMap are LINEAR and use Rgba8Unorm below —
    // never propagate this Srgb format to those (sRGB red-line).
    let fallback_img = Image::new(
        bevy::render::render_resource::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        vec![255u8, 255, 255, 255],
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    let fallback_handle = images.add(fallback_img);

    for (entity, mat_ref, _existing_material) in &query {
        let domain_material = match system.from_type(&mat_ref.type_name, mat_ref.uniforms.clone()) {
            Ok(m) => m,
            Err(e) => {
                warn!(
                    "Failed to build material '{}' for entity {:?}: {}",
                    mat_ref.type_name, entity, e
                );
                continue;
            }
        };

        let fabric_material =
            if FabricKind::from_type_name(&mat_ref.type_name) == FabricKind::Water {
                // M5-D: Water binds real procedurally-generated normal/specular
                // maps (Rgba8Unorm LINEAR, from the domain cesium_shadow
                // OceanSurface); other kinds use the fallback for all bindings.
                //
                // Daniel L4: the default `OceanConfig` is constant, so its
                // 2×(128×128) texture pair is generated ONCE and shared across all
                // default-config Water entities instead of O(N) identical copies.
                let (normal_map, specular_map) =
                    match (&shared_water.normal_map, &shared_water.specular_map) {
                        (Some(n), Some(s)) => (n.clone(), s.clone()),
                        _ => {
                            let ocean = OceanSurface::new(OceanConfig::default());
                            let n = images.add(generate_water_normal_map(128, &ocean, 200.0));
                            let s = images.add(generate_water_specular_map(128, &ocean, 200.0));
                            shared_water.normal_map = Some(n.clone());
                            shared_water.specular_map = Some(s.clone());
                            (n, s)
                        }
                    };
                fabric_material_from_domain_with_maps(
                    &domain_material,
                    fallback_handle.clone(),
                    normal_map,
                    specular_map,
                )
            } else {
                fabric_material_from_domain(&domain_material, fallback_handle.clone())
            };
        let handle = materials.add(fabric_material);
        commands.entity(entity).insert(MeshMaterial3d(handle));
    }
}

/// System: per-frame uniform updates for animated materials.
///
/// Currently updates:
/// - Water time animation
fn update_material_uniforms(
    time: Res<Time>,
    mut animation_time: ResMut<MaterialAnimationTime>,
    mut materials: ResMut<Assets<FabricMaterial>>,
    mut water_handles: ResMut<WaterMaterialHandles>,
) {
    // Ryan M3 / dev-019 方案 A: advance the clock, FREEZING the frame counter when
    // FIXED_TIME holds (`delta_secs() == 0.0`) so the Water phase is bit-reproducible.
    let frame_number = advance_animation(&mut animation_time, time.delta_secs());

    // Daniel M5: `extra_c.z` now carries a per-FRAME counter mirroring CesiumJS
    // `czm_frameNumber` (Water.glsl L18 `time = czm_frameNumber * animationSpeed`),
    // NOT accumulated seconds. Water animation is therefore frame-rate dependent
    // (60 fps advances the waveform twice as fast as 30 fps). Any Water baseline
    // capture MUST lock the frame index, not wall time — see the note in
    // specs/scripts/v2_water.toml and docs/deviations.md#dev-003 / #dev-019.

    // Ryan L6: rebuild the Water-handle cache only when the asset set changes (a
    // spawn/removal), via a non-dirtying immutable `iter()`. Covers both the
    // `apply_fabric_materials` path and materials the showcase spawns directly.
    let current_len = materials.len();
    if current_len != water_handles.known_len {
        water_handles.handles = materials
            .iter()
            .filter(|(_, m)| m.params.kind == FabricKind::Water as u32)
            .map(|(id, _)| Handle::Weak(id))
            .collect();
        water_handles.known_len = current_len;
    }

    // Skip the write entirely when the published frame is unchanged (FIXED_TIME, or
    // any frame that did not advance) so the bind group is not needlessly
    // re-encoded and the Water materials are not spuriously dirtied (Ryan L6).
    if animation_time.last_written_frame == Some(frame_number) {
        return;
    }

    // Ryan L4: reduce modulo WATER_FRAME_PERIOD so `frame as f32` stays exact well
    // beyond the 2^24 f32 integer-precision cliff.
    let phase = (frame_number % WATER_FRAME_PERIOD) as f32;
    for handle in &water_handles.handles {
        if let Some(material) = materials.get_mut(handle) {
            material.params.extra_c.z = phase;
        }
    }
    animation_time.last_written_frame = Some(frame_number);
}

/// Resource wrapping a [`MaterialSystem`] so it can be used as a Bevy resource.
///
/// The [`MaterialSystem`] holds the cached built-in material type definitions
/// (GLSL source + default uniforms) and is required by [`apply_fabric_materials`].
#[derive(Resource)]
pub struct MaterialSystemResource(pub MaterialSystem);

impl MaterialSystemResource {
    /// Create with all built-in CesiumJS material types pre-registered.
    pub fn with_builtin_materials() -> Self {
        Self(MaterialSystem::with_builtin_materials())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        advance_animation, update_material_uniforms, CesiumMaterialPlugin, FabricKind, FabricMaterial,
        MaterialAnimationTime, MaterialRef, WaterMaterialHandles, WATER_FRAME_PERIOD,
    };
    use crate::fabric_material::FabricParams;
    use crate::CesiumCorePlugin;
    use bevy::prelude::*;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    /// Headless regression guard for `docs/deviations.md#dev-005` /
    /// `docs/deferred.md#6` (resolved at M5.1).
    ///
    /// Replicates the exact scenario of the four
    /// `specs/tests/integration/material_integration_test.rs` cases: a
    /// `create_test_app()`-style app (`MinimalPlugins` + [`CesiumCorePlugin`], with
    /// no `AssetPlugin` / `RenderApp` / wgpu device) plus [`CesiumMaterialPlugin`].
    /// Before M5.1 this panicked inside `FabricMaterialPlugin::build`
    /// (`load_internal_asset!` → missing `Assets<Shader>`, then `MaterialPlugin`
    /// → missing `AssetServer`). The headless-safe guards in
    /// [`crate::shader_registry`] make registration a no-op instead, so the plugin
    /// installs and the CPU-side material API stays usable.
    #[test]
    fn cesium_material_plugin_headless_minimal_does_not_panic() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(CesiumCorePlugin)
            .add_plugins(CesiumMaterialPlugin);

        // test_material_animation_time_resource_initialized
        assert!(app.world().get_resource::<MaterialAnimationTime>().is_some());

        // test_material_ref_component
        let entity = app.world_mut().spawn(MaterialRef::new("Color")).id();
        let got = app.world().get::<MaterialRef>(entity);
        assert!(got.is_some());
        assert_eq!(got.unwrap().type_name, "Color");

        // test_material_ref_with_uniforms
        let mut uniforms = std::collections::BTreeMap::new();
        uniforms.insert(
            "color".to_string(),
            cesium_material::UniformValue::Vec4([1.0, 0.0, 0.0, 1.0]),
        );
        let entity2 = app
            .world_mut()
            .spawn(MaterialRef::with_uniforms("Checkerboard", uniforms))
            .id();
        let got2 = app.world().get::<MaterialRef>(entity2).unwrap();
        assert_eq!(got2.type_name, "Checkerboard");
        assert!(got2.uniforms.contains_key("color"));
    }

    /// Ryan M3 / dev-019 方案 A — the pure clock-advance helper: under FIXED_TIME
    /// (`delta_secs() == 0.0`) the frame counter is FROZEN; a live clock advances.
    #[test]
    fn advance_animation_freezes_frame_number_under_fixed_time() {
        let mut anim = MaterialAnimationTime::default();
        // FIXED_TIME: delta_secs() == 0.0 → frame counter must NOT advance.
        assert_eq!(advance_animation(&mut anim, 0.0), 0);
        assert_eq!(advance_animation(&mut anim, 0.0), 0);
        assert_eq!(anim.frame_number, 0, "FIXED_TIME must freeze the Water phase");
        // A live clock still advances monotonically.
        assert_eq!(advance_animation(&mut anim, 1.0 / 60.0), 1);
        assert_eq!(advance_animation(&mut anim, 1.0 / 60.0), 2);
    }

    /// Ryan L4 — the modulo keeps `extra_c.z` exactly representable in f32 far
    /// beyond the 2^24 precision cliff, and the period is well below it.
    #[test]
    fn water_frame_period_stays_below_the_f32_integer_precision_cliff() {
        // Compile-time-checked invariant (a bare runtime `assert!` on constants
        // trips clippy::assertions_on_constants); this fails the build if the
        // period is ever raised to/past 2^24, where `frame as f32` stops being exact.
        const _: () = assert!(
            WATER_FRAME_PERIOD < (1 << 24),
            "WATER_FRAME_PERIOD must be < 2^24 so `frame as f32` stays exact"
        );
        // A frame counter just under the cliff, reduced, is still exact in f32.
        let huge: u32 = (1 << 24) + 12345;
        let reduced = huge % WATER_FRAME_PERIOD;
        let phase = reduced as f32;
        assert_eq!(phase as u32, reduced, "reduced phase must survive f32 exactly");
        assert!(phase < 16_777_216.0);
    }

    /// Ryan M3 — the actual system: two consecutive `update_material_uniforms`
    /// ticks under FIXED_TIME (`ManualDuration(ZERO)`) leave Water's `extra_c.z`
    /// bit-identical, so a captured Water baseline is reproducible.
    #[test]
    fn update_material_uniforms_freezes_water_phase_under_fixed_time() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // The FIXED_TIME contract `main.rs` relies on for bit-reproducible frames.
        // Bevy 0.15.3 idiom (matches main.rs / specs/tests/camera_control.rs): the
        // strategy is a standalone resource; `Time::new_with_update_strategy` was
        // removed. `init_resource::<Time>()` guarantees `Res<Time>` resolves even
        // though `MinimalPlugins` may not add `TimePlugin`; either way the manual
        // ZERO duration (and the never-advanced default) keep `delta_secs() == 0.0`.
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        app.init_resource::<Time>();
        app.init_resource::<MaterialAnimationTime>();
        app.init_resource::<WaterMaterialHandles>();
        app.init_resource::<Assets<FabricMaterial>>();
        app.add_systems(Update, update_material_uniforms);

        // Seed a Water material with a non-zero phase so a spurious write is
        // visible. Struct literal (not `default()` + field reassign, which trips
        // clippy::field_reassign_with_default). extra_c default is
        // `Vec4::new(0.0, 1000.0, 0.0, 0.5)`; we plant z = 123.0.
        let params = FabricParams {
            kind: FabricKind::Water as u32,
            extra_c: Vec4::new(0.0, 1000.0, 123.0, 0.5),
            ..Default::default()
        };
        let mat = FabricMaterial {
            params,
            image: Handle::default(),
            normal_map: Handle::default(),
            specular_map: Handle::default(),
            translucent: true,
        };
        let handle = app
            .world_mut()
            .resource_mut::<Assets<FabricMaterial>>()
            .add(mat);

        app.update();
        let z1 = app
            .world()
            .resource::<Assets<FabricMaterial>>()
            .get(&handle)
            .unwrap()
            .params
            .extra_c
            .z;
        app.update();
        let z2 = app
            .world()
            .resource::<Assets<FabricMaterial>>()
            .get(&handle)
            .unwrap()
            .params
            .extra_c
            .z;

        // Frozen frame (0) ⇒ phase 0.0, identical across both ticks.
        assert_eq!(z1, z2, "Water phase drifted under FIXED_TIME (dev-019 方案 A)");
        assert_eq!(z2, 0.0, "frozen frame_number 0 must publish phase 0.0");
    }
}
