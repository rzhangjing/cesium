//! Starfield background — renders stars on a large celestial sphere.
//!
//! Uses the domain-layer `StarSphere` builtin catalog for bright stars,
//! plus procedurally scattered dim stars for a realistic night sky.

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

fn setup_starfield(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let radius = 50.0_f32;

    // --- Bright stars from domain catalog ---
    let star_sphere = StarSphere::with_builtin_catalog();
    let bright_stars: Vec<(Vec3, f32)> = star_sphere
        .visible_stars()
        .map(|star| {
            // Convert RA/Dec to 3D position on celestial sphere
            let ra = star.right_ascension as f32;
            let dec = star.declination as f32;
            let pos = Vec3::new(
                radius * dec.cos() * ra.cos(),
                radius * dec.sin(),
                radius * dec.cos() * ra.sin(),
            );
            // Brighter stars (lower magnitude) → larger size
            let size = (0.3 + 0.15 * (6.0 - star.magnitude as f32).max(0.0)) as f32;
            (pos, size)
        })
        .collect();

    // --- Procedural dim stars (~1500 random points) ---
    let dim_count = 1500u32;
    let mut dim_positions: Vec<(Vec3, f32)> = Vec::with_capacity(dim_count as usize);
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
        dim_positions.push((pos, size));
    }

    // Spawn bright stars as small emissive quads
    let star_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::new(4.0, 4.0, 4.0, 1.0),
        unlit: true,
        cull_mode: None, // Visible from both sides
        ..default()
    });

    for (pos, size) in bright_stars.iter().chain(dim_positions.iter()) {
        let mesh = meshes.add(
            Sphere::new(*size * 0.5)
                .mesh()
                .ico(1)
                .expect("ico subdivision failed"),
        );
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(star_material.clone()),
            Transform::from_translation(*pos),
        ));
    }
}
