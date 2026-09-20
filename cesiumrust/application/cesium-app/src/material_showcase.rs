//! Fabric material showcase (P1.3 visual verification + M5-D Water baselines).
//!
//! Renders the built-in Fabric procedural materials from the domain
//! [`cesium_material`] crate onto spheres arranged in an arc on the
//! camera-facing side of the globe. This exercises the full pipeline:
//!
//! domain `MaterialSystem::from_type` (Fabric JSON -> `Material`) ->
//! adapter `fabric_material_from_domain` (uniform packing) ->
//! GPU `FabricMaterial` (WGSL procedural patterns).
//!
//! M5-D adds three Water sea-state entries (calm / medium / rough) driven by a
//! domain [`cesium_shadow::OceanSurface`] (Gerstner wave stack) with
//! procedurally-generated normal / specular maps, so the faithfully-ported
//! `Water.glsl` (case 17u) can be captured as `specs/baselines/v2_water/*`.
//!
//! Maps to the P1.3 acceptance criterion "棋盘/条纹/网格材质贴球" (checkerboard /
//! stripe / grid materials on spheres), plus the other built-in patterns and the
//! M5-D Water material.

use bevy::math::DVec3;
use bevy::prelude::*;
use cesium_bevy_render::fabric_material::{
    fabric_material_from_domain, water_material_from_preset, FabricMaterial, WaterPreset,
};
use cesium_bevy_render::{
    create_imagery_texture, geometry_to_mesh, CesiumMaterialPlugin, MaterialSystemResource,
};
use cesium_geospatial::geometry::{self, VertexFormat};
use cesium_material::{MaterialSystem, UniformValue};
use std::collections::BTreeMap;

/// Plugin that registers the Fabric material pipeline and spawns the showcase.
///
/// Opt-in via `CESIUM_ENABLE_MATERIAL_SHOWCASE=1` (see `main.rs`); default OFF
/// keeps the v0 baselines pixel-neutral.
pub struct MaterialShowcasePlugin;

impl Plugin for MaterialShowcasePlugin {
    fn build(&self, app: &mut App) {
        // `CesiumMaterialPlugin` bundles `FabricMaterialPlugin` + the
        // `MaterialAnimationTime` resource + the per-frame Water animation system
        // (`update_material_uniforms`), so the Water cases animate. The
        // `MaterialSystemResource` is inserted so the bundled
        // `apply_fabric_materials` system finds it and stays a silent no-op (the
        // showcase spawns `FabricMaterial` directly, not via `MaterialRef`).
        app.add_plugins(CesiumMaterialPlugin)
            .insert_resource(MaterialSystemResource::with_builtin_materials())
            .add_systems(Startup, setup_material_showcase);
    }
}

/// A single showcase entry: a built-in material type plus uniform overrides.
struct ShowcaseEntry {
    /// The built-in Fabric material type name (e.g. `"Checkerboard"`).
    type_name: &'static str,
    /// Uniform overrides applied on top of the material's defaults.
    overrides: Vec<(&'static str, UniformValue)>,
    /// When set, this is a Water entry driven by a domain `OceanSurface` preset
    /// (calm / medium / rough) with procedurally-generated normal / specular maps.
    water: Option<WaterPreset>,
}

/// The ten showcased Fabric materials: seven static built-ins (slots 0..=6) then
/// the three M5-D Water sea states Calm/Medium/Rough (slots 7/8/9).
///
/// Extracted from [`setup_material_showcase`] so the Mark M-3 tests can assert the
/// count, the Water slots and the preset order without spawning a GPU app.
fn showcase_entries() -> Vec<ShowcaseEntry> {
    vec![
        ShowcaseEntry {
            type_name: "Color",
            overrides: vec![("color", UniformValue::Vec4([0.9, 0.15, 0.15, 1.0]))],
            water: None,
        },
        ShowcaseEntry {
            type_name: "Checkerboard",
            overrides: vec![],
            water: None,
        },
        ShowcaseEntry {
            type_name: "Stripe",
            overrides: vec![
                ("evenColor", UniformValue::Vec4([1.0, 1.0, 1.0, 1.0])),
                ("oddColor", UniformValue::Vec4([0.1, 0.3, 0.9, 1.0])),
            ],
            water: None,
        },
        ShowcaseEntry {
            type_name: "Grid",
            overrides: vec![
                ("color", UniformValue::Vec4([0.0, 1.0, 0.45, 1.0])),
                ("cellAlpha", UniformValue::Float(0.15)),
            ],
            water: None,
        },
        ShowcaseEntry {
            type_name: "Dot",
            overrides: vec![
                ("lightColor", UniformValue::Vec4([1.0, 0.85, 0.0, 1.0])),
                ("darkColor", UniformValue::Vec4([0.12, 0.18, 0.35, 1.0])),
            ],
            water: None,
        },
        ShowcaseEntry {
            type_name: "Fade",
            overrides: vec![],
            water: None,
        },
        ShowcaseEntry {
            type_name: "Image",
            overrides: vec![("repeat", UniformValue::Vec2([3.0, 3.0]))],
            water: None,
        },
        // ── M5-D: three Water sea states (calm / medium / rough) ──
        ShowcaseEntry {
            type_name: "Water",
            overrides: vec![],
            water: Some(WaterPreset::Calm),
        },
        ShowcaseEntry {
            type_name: "Water",
            overrides: vec![],
            water: Some(WaterPreset::Medium),
        },
        ShowcaseEntry {
            type_name: "Water",
            overrides: vec![],
            water: Some(WaterPreset::Rough),
        },
    ]
}

/// Spawns one sphere per built-in Fabric material, arranged in an arc on the
/// camera-facing side of the globe so every pattern is clearly visible.
fn setup_material_showcase(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut images: Option<ResMut<Assets<Image>>>,
    mut fabric_materials: Option<ResMut<Assets<FabricMaterial>>>,
) {
    // Headless-safe (Mark M-3): under `MinimalPlugins` there is no asset backend, so
    // these `ResMut<Assets<_>>` are absent and a non-`Option` param would panic when
    // the Startup system runs. Degrade to a no-op instead, mirroring
    // `sky_system::sky_dome_setup`'s `Option<ResMut<_>>` pattern. On the GPU path all
    // three are present and the behaviour is unchanged (pixel-neutral).
    let (Some(meshes), Some(images), Some(fabric_materials)) = (
        meshes.as_deref_mut(),
        images.as_deref_mut(),
        fabric_materials.as_deref_mut(),
    ) else {
        return;
    };

    let system = MaterialSystem::with_builtin_materials();

    // A procedural demo texture: used by the `Image` material and as the
    // `czm_defaultImage` stand-in that backs every material's sampler binding.
    // Colour data → Rgba8UnormSrgb (via create_imagery_texture). Water's
    // normal/specular maps are generated separately as LINEAR Rgba8Unorm.
    let demo_image = images.add(make_demo_image());

    // A shared unit-radius sphere mesh, scaled per instance.
    let sphere_geometry =
        geometry::ellipsoid_geometry(DVec3::splat(1.0), 32, 64, VertexFormat::ALL);
    let sphere_mesh = meshes.add(geometry_to_mesh(&sphere_geometry, None));

    let entries = showcase_entries();

    let n = entries.len();
    // Arc layout: spheres sit on a circle of `arc_radius` around the globe
    // centre, spread over `total_span_deg` on the camera-facing hemisphere.
    let arc_radius = 1.9_f32;
    let sphere_radius = 0.24_f32;
    let total_span_deg = 150.0_f32; // -75° .. +75°

    for (i, entry) in entries.iter().enumerate() {
        let mut overrides = BTreeMap::new();
        for (key, value) in &entry.overrides {
            overrides.insert((*key).to_string(), value.clone());
        }
        // Water presets contribute their own frequency/amplitude/animationSpeed/
        // specularIntensity overrides on top of the domain defaults.
        if let Some(preset) = entry.water {
            for (key, value) in preset.uniform_overrides() {
                overrides.insert(key.to_string(), value);
            }
        }

        let domain_material = system
            .from_type(entry.type_name, overrides)
            .unwrap_or_else(|e| panic!("failed to build material {}: {}", entry.type_name, e));

        let material = if let Some(preset) = entry.water {
            water_material_from_preset(&mut *images, &domain_material, demo_image.clone(), preset)
        } else {
            fabric_material_from_domain(&domain_material, demo_image.clone())
        };

        let t = if n == 1 { 0.5 } else { i as f32 / (n as f32 - 1.0) };
        let angle = (-total_span_deg / 2.0 + t * total_span_deg).to_radians();
        let position = Vec3::new(arc_radius * angle.sin(), 0.0, arc_radius * angle.cos());

        let label = match entry.water {
            Some(preset) => format!("Water_{}", preset.label()),
            None => entry.type_name.to_string(),
        };
        commands.spawn((
            Name::new(format!("FabricMaterial_{}", label)),
            Mesh3d(sphere_mesh.clone()),
            MeshMaterial3d(fabric_materials.add(material)),
            Transform::from_translation(position).with_scale(Vec3::splat(sphere_radius)),
        ));
    }
}

/// Builds a small colourful test-card texture so the `Image` material has
/// something distinctive to sample (red/green ramps + a checker blue channel).
///
/// Colour (imagery-like) data → `Rgba8UnormSrgb` via [`create_imagery_texture`].
/// This Srgb format must NOT be reused for Water's linear normal/specular maps.
fn make_demo_image() -> Image {
    let size = 64u32;
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 / (size - 1) as f32;
            let fy = y as f32 / (size - 1) as f32;
            let r = (fx * 255.0) as u8;
            let g = (fy * 255.0) as u8;
            let b = (((x / 8) + (y / 8)) % 2 * 255) as u8;
            data.extend_from_slice(&[r, g, b, 255]);
        }
    }
    create_imagery_texture(size, size, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (a) The showcase plugin must install and run headlessly without panicking.
    ///
    /// Under `MinimalPlugins` there is no asset backend, so the `Startup` system
    /// [`setup_material_showcase`] IS scheduled and runs, but degrades to a no-op via
    /// its `Option<ResMut<_>>` guard (no `Assets<Mesh/Image/FabricMaterial>`);
    /// `CesiumMaterialPlugin` likewise gates its `Update` systems off. This is the
    /// Mark M-3 concern: the showcase was the one M5 delivery module with no test.
    #[test]
    fn material_showcase_plugin_headless_minimal_does_not_panic() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(MaterialShowcasePlugin);
        // Must not panic (Startup system early-returns; no material Update systems).
        app.update();
        // The CPU-side resource the showcase relies on is present even headless.
        assert!(app.world().get_resource::<MaterialSystemResource>().is_some());
    }

    /// (b) Ten entries total; the Water presets Calm/Medium/Rough occupy slots
    /// 7/8/9 (and only those), so `v2_water.toml`'s three close-ups map 1:1.
    #[test]
    fn showcase_entries_are_ten_with_three_water_presets_in_slots_7_8_9() {
        let entries = showcase_entries();
        assert_eq!(entries.len(), 10, "showcase must present 10 Fabric materials");

        let water_slots: Vec<usize> = entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.water.is_some())
            .map(|(i, _)| i)
            .collect();
        assert_eq!(
            water_slots,
            vec![7, 8, 9],
            "exactly three Water entries, in the final slots"
        );
        assert!(entries[..7].iter().all(|e| e.water.is_none()));

        assert_eq!(entries[7].type_name, "Water");
        assert_eq!(entries[8].type_name, "Water");
        assert_eq!(entries[9].type_name, "Water");
        assert_eq!(entries[7].water, Some(WaterPreset::Calm));
        assert_eq!(entries[8].water, Some(WaterPreset::Medium));
        assert_eq!(entries[9].water, Some(WaterPreset::Rough));
    }

    /// (c) The Water arc poses must match `specs/scripts/v2_water.toml`, which
    /// documents the three close-ups at arc angles 41.67° / 58.33° / 75.00°
    /// (2-decimal rounding of the exact 41.6667 / 58.3333 / 75.0 the layout formula
    /// produces for slots 7/8/9 of 10 over a 150° span).
    ///
    /// The formula is recomputed independently here (not by running the GPU setup),
    /// asserted tightly against the exact analytic values (1e-3) and rounding-aware
    /// against the toml's documented degrees (5e-3 = half the last printed digit).
    /// Each camera pose `3.5·(sinθ, 0, cosθ)` is printed alongside the toml's stored
    /// `pos` so any drift yields a ready-to-paste correction table for the script.
    #[test]
    fn water_arc_angles_match_the_v2_water_script_poses() {
        // Layout constants mirrored from `setup_material_showcase`.
        let n = 10_f32;
        let arc_radius = 1.9_f32;
        let total_span_deg = 150.0_f32;
        let angle_deg = |i: f32| -> f32 {
            let t = i / (n - 1.0);
            -total_span_deg / 2.0 + t * total_span_deg
        };

        // (slot, exact analytic degrees, v2_water.toml documented degrees,
        //  toml stored camera pos [x, z] at radius 3.5).
        let cases = [
            (7_f32, 41.6667, 41.67, [2.326_8, 2.614_5]),
            (8.0, 58.3333, 58.33, [2.978_9, 1.839_3]),
            (9.0, 75.0, 75.00, [3.380_7, 0.905_8]),
        ];

        for (slot, exact, documented, toml_pos) in cases {
            let got = angle_deg(slot);
            // Tight self-consistency of the layout formula.
            assert!(
                (got - exact).abs() < 1e-3,
                "arc formula drift at slot {slot}: got {got:.6}°, expected {exact:.6}°"
            );
            // Rounding-aware match to the toml's documented degrees.
            assert!(
                (got - documented).abs() < 5e-3,
                "slot {slot} angle {got:.4}° no longer matches v2_water.toml's {documented}°"
            );
            // Print the camera pose (radius 3.5) vs the toml's stored pos so a drift
            // produces a correction table.
            let theta = got.to_radians();
            let cam = Vec3::new(3.5 * theta.sin(), 0.0, 3.5 * theta.cos());
            let sphere = Vec3::new(arc_radius * theta.sin(), 0.0, arc_radius * theta.cos());
            println!(
                "slot {slot:.0}: angle={got:.4}°  sphere_pos=({:.4}, 0, {:.4})  \
                 camera=({:.4}, {:.4}, {:.4})  toml_pos=({:.4}, 0, {:.4})",
                sphere.x, sphere.z, cam.x, cam.y, cam.z,
                toml_pos[0], toml_pos[1],
            );
        }
    }
}
