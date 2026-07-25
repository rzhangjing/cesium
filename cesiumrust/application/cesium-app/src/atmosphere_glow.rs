//! Atmosphere glow — a translucent blue shell around the globe.
//!
//! Simulates the atmospheric limb glow visible from space using a
//! slightly larger sphere with additive-blend transparent material
//! rendered on the back face (Fresnel-like rim effect).

use bevy::prelude::*;

/// Plugin that spawns the atmosphere glow shell.
pub struct AtmosphereGlowPlugin;

impl Plugin for AtmosphereGlowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_atmosphere);
    }
}

fn setup_atmosphere(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Globe renders as unit sphere (scale = 1/METERS_PER_RENDER_UNIT applied to
    // a mesh with radius ~6378137 m). Effective globe radius in render units ≈ 1.0.
    // Atmosphere shell is ~2.5% larger.
    let atmosphere_scale = 1.025_f32;

    let mesh = meshes.add(
        Sphere::new(1.0)
            .mesh()
            .ico(5)
            .expect("ico subdivision failed"),
    );

    // Outer glow: rendered on front faces, additive blend, very transparent
    commands.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.3, 0.6, 1.0, 0.12),
            emissive: LinearRgba::new(0.1, 0.3, 0.8, 1.0),
            unlit: true,
            alpha_mode: AlphaMode::Add,
            cull_mode: None, // Render both faces for glow volume
            ..default()
        })),
        Transform::from_scale(Vec3::splat(atmosphere_scale)),
    ));

    // Inner rim: slightly larger, back-face only for limb brightening
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.4, 0.7, 1.0, 0.08),
            emissive: LinearRgba::new(0.15, 0.4, 1.0, 1.0),
            unlit: true,
            alpha_mode: AlphaMode::Add,
            cull_mode: Some(bevy::render::render_resource::Face::Front), // Show back faces only
            ..default()
        })),
        Transform::from_scale(Vec3::splat(atmosphere_scale * 1.015)),
    ));
}
