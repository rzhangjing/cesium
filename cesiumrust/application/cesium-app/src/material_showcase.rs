//! Fabric material showcase (P1.3 visual verification).
//!
//! Renders the built-in Fabric procedural materials from the domain
//! [`cesium_material`] crate onto spheres arranged in an arc on the
//! camera-facing side of the globe. This exercises the full pipeline:
//!
//! domain `MaterialSystem::from_type` (Fabric JSON -> `Material`) ->
//! adapter `fabric_material_from_domain` (uniform packing) ->
//! GPU `FabricMaterial` (WGSL procedural patterns).
//!
//! Maps to the P1.3 acceptance criterion "棋盘/条纹/网格材质贴球" (checkerboard /
//! stripe / grid materials on spheres), plus the other built-in patterns.

use bevy::math::DVec3;
use bevy::prelude::*;
use cesium_bevy_render::fabric_material::{
    fabric_material_from_domain, FabricMaterial, FabricMaterialPlugin,
};
use cesium_bevy_render::{create_imagery_texture, geometry_to_mesh};
use cesium_geospatial::geometry::{self, VertexFormat};
use cesium_material::{MaterialSystem, UniformValue};
use std::collections::BTreeMap;

/// Plugin that registers the Fabric material pipeline and spawns the showcase.
pub struct MaterialShowcasePlugin;

impl Plugin for MaterialShowcasePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FabricMaterialPlugin)
            .add_systems(Startup, setup_material_showcase);
    }
}

/// A single showcase entry: a built-in material type plus uniform overrides.
struct ShowcaseEntry {
    /// The built-in Fabric material type name (e.g. `"Checkerboard"`).
    type_name: &'static str,
    /// Uniform overrides applied on top of the material's defaults.
    overrides: Vec<(&'static str, UniformValue)>,
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
    let demo_image = images.add(make_demo_image());

    // A shared unit-radius sphere mesh, scaled per instance.
    let sphere_geometry =
        geometry::ellipsoid_geometry(DVec3::splat(1.0), 32, 64, VertexFormat::ALL);
    let sphere_mesh = meshes.add(geometry_to_mesh(&sphere_geometry));

    let entries = [
        ShowcaseEntry {
            type_name: "Color",
            overrides: vec![("color", UniformValue::Vec4([0.9, 0.15, 0.15, 1.0]))],
        },
        ShowcaseEntry {
            type_name: "Checkerboard",
            overrides: vec![],
        },
        ShowcaseEntry {
            type_name: "Stripe",
            overrides: vec![
                ("evenColor", UniformValue::Vec4([1.0, 1.0, 1.0, 1.0])),
                ("oddColor", UniformValue::Vec4([0.1, 0.3, 0.9, 1.0])),
            ],
        },
        ShowcaseEntry {
            type_name: "Grid",
            overrides: vec![
                ("color", UniformValue::Vec4([0.0, 1.0, 0.45, 1.0])),
                ("cellAlpha", UniformValue::Float(0.15)),
            ],
        },
        ShowcaseEntry {
            type_name: "Dot",
            overrides: vec![
                ("lightColor", UniformValue::Vec4([1.0, 0.85, 0.0, 1.0])),
                ("darkColor", UniformValue::Vec4([0.12, 0.18, 0.35, 1.0])),
            ],
        },
        ShowcaseEntry {
            type_name: "Fade",
            overrides: vec![],
        },
        ShowcaseEntry {
            type_name: "Image",
            overrides: vec![("repeat", UniformValue::Vec2([3.0, 3.0]))],
        },
    ];

    let n = entries.len();
    // Arc layout: spheres sit on a circle of `arc_radius` around the globe
    // centre, spread over `total_span_deg` on the camera-facing hemisphere.
    let arc_radius = 1.7_f32;
    let sphere_radius = 0.26_f32;
    let total_span_deg = 108.0_f32; // -54° .. +54°

    for (i, entry) in entries.iter().enumerate() {
        let mut overrides = BTreeMap::new();
        for (key, value) in &entry.overrides {
            overrides.insert((*key).to_string(), value.clone());
        }
        let domain_material = system
            .from_type(entry.type_name, overrides)
            .unwrap_or_else(|e| panic!("failed to build material {}: {}", entry.type_name, e));

        let material = fabric_material_from_domain(&domain_material, demo_image.clone());

        let t = if n == 1 { 0.5 } else { i as f32 / (n as f32 - 1.0) };
        let angle = (-total_span_deg / 2.0 + t * total_span_deg).to_radians();
        let position = Vec3::new(arc_radius * angle.sin(), 0.0, arc_radius * angle.cos());

        commands.spawn((
            Name::new(format!("FabricMaterial_{}", entry.type_name)),
            Mesh3d(sphere_mesh.clone()),
            MeshMaterial3d(fabric_materials.add(material)),
            Transform::from_translation(position).with_scale(Vec3::splat(sphere_radius)),
        ));
    }
}

/// Builds a small colourful test-card texture so the `Image` material has
/// something distinctive to sample (red/green ramps + a checker blue channel).
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
