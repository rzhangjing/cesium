//! Starfield background — renders stars on a large celestial sphere.
//!
//! Uses the domain-layer `StarSphere` builtin catalog for bright stars,
//! plus procedurally scattered dim stars for a realistic night sky.
//!
//! ALL stars are merged into ONE mesh (camera-facing quads) so the whole
//! sky costs a single draw call — spawning one entity per star would add
//! ~1500 draw calls and tank the frame rate.

use bevy::prelude::*;
use cesium_atmosphere::StarSphere;

/// Plugin that spawns the starfield.
pub struct StarfieldPlugin;

impl Plugin for StarfieldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_starfield);
    }
}

/// Simple deterministic hash-based pseudo-random [0, 1).
fn hash_rand(seed: u32) -> f32 {
    let mut x = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    x ^= x >> 16;
    x = x.wrapping_mul(0x45d9f3b);
    x ^= x >> 16;
    (x & 0x00FF_FFFF) as f32 / 16777216.0
}

/// Appends one camera-facing quad (4 verts / 2 tris) centered at `center`
/// with half-size `size`. The quad plane is perpendicular to the radial
/// direction, so it always faces a camera near the origin.
fn push_star_quad(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
    center: Vec3,
    size: f32,
) {
    let radial = center.normalize();
    // Tangent frame in the plane perpendicular to the radial direction.
    let up = if radial.dot(Vec3::Z).abs() > 0.95 {
        Vec3::X
    } else {
        Vec3::Z
    };
    let t = radial.cross(up).normalize();
    let b = radial.cross(t).normalize();

    let base = positions.len() as u32;
    let corners = [
        center - t * size - b * size,
        center + t * size - b * size,
        center + t * size + b * size,
        center - t * size + b * size,
    ];
    let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    for (c, u) in corners.iter().zip(uv.iter()) {
        positions.push(c.to_array());
        normals.push(radial.to_array());
        uvs.push(*u);
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn setup_starfield(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let radius = 50.0_f32;

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // --- Bright stars from domain catalog ---
    let star_sphere = StarSphere::with_builtin_catalog();
    for star in star_sphere.visible_stars() {
        // Convert RA/Dec to 3D position on celestial sphere
        let ra = star.right_ascension as f32;
        let dec = star.declination as f32;
        let pos = Vec3::new(
            radius * dec.cos() * ra.cos(),
            radius * dec.sin(),
            radius * dec.cos() * ra.sin(),
        );
        // Brighter stars (lower magnitude) → larger size
        let size = 0.3 + 0.15 * (6.0 - star.magnitude as f32).max(0.0);
        push_star_quad(&mut positions, &mut normals, &mut uvs, &mut indices, pos, size * 0.5);
    }

    // --- Procedural dim stars (~1500 random points) ---
    let dim_count = 1500u32;
    for i in 0..dim_count {
        // Uniform distribution on sphere via hash
        let theta = hash_rand(i * 3 + 1) * std::f32::consts::TAU; // azimuth
        let phi = (hash_rand(i * 3 + 2) * 2.0 - 1.0).acos(); // polar
        let pos = Vec3::new(
            radius * phi.sin() * theta.cos(),
            radius * phi.cos(),
            radius * phi.sin() * theta.sin(),
        );
        let size = 0.1 + hash_rand(i * 3 + 3) * 0.15;
        push_star_quad(&mut positions, &mut normals, &mut uvs, &mut indices, pos, size * 0.5);
    }

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

    let star_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::new(4.0, 4.0, 4.0, 1.0),
        unlit: true,
        cull_mode: None, // Visible from both sides
        ..default()
    });

    // ONE entity / ONE draw call for the entire sky.
    commands.spawn((Mesh3d(meshes.add(mesh)), MeshMaterial3d(star_material)));
}
