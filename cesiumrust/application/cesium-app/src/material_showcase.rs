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

/// Spawns one sphere per built-in Fabric material, arranged in an arc on the
/// camera-facing side of the globe so every pattern is clearly visible.
fn setup_material_showcase(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut fabric_materials: ResMut<Assets<FabricMaterial>>,
) {
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

    let entries = [
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
    ];

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
            water_material_from_preset(&mut images, &domain_material, demo_image.clone(), preset)
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
