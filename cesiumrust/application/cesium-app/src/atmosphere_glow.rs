//! Atmosphere glow — nested translucent blue shells around the globe.
//!
//! Simulates the atmospheric limb glow visible from space. Each shell is
//! rendered BACK-FACES ONLY with additive blending, so it appears solely as a
//! halo ring around the limb. Eight shells with decreasing alpha produce a
//! soft gradient falloff instead of a single hard edge.

use bevy::prelude::*;

/// Plugin that spawns the atmosphere glow shells.
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
    // a mesh with radius ~6378137 m). Effective globe radius in render units
    // ≈ 1.0. Shells step outward with halving alpha for a soft limb falloff.
    let mesh = meshes.add(
        Sphere::new(1.0)
            .mesh()
            .ico(6)
            .expect("ico subdivision failed"),
    );

    // (scale, additive alpha): innermost brightest, outermost faintest. Eight
    // shells keep the per-shell alpha step small enough that no banding or
    // hard outer silhouette is perceptible against black space.
    let shells: [(f32, f32); 8] = [
        (1.012, 0.14),
        (1.025, 0.095),
        (1.038, 0.062),
        (1.052, 0.040),
        (1.066, 0.025),
        (1.081, 0.015),
        (1.096, 0.008),
        (1.112, 0.004),
    ];

    for (scale, alpha) in shells {
        commands.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.3, 0.6, 1.0, alpha),
                unlit: true,
                alpha_mode: AlphaMode::Add,
                // Back faces only: the shell's far hemisphere shows up as a
                // halo ring just outside the globe's silhouette.
                cull_mode: Some(bevy::render::render_resource::Face::Front),
                ..default()
            })),
            Transform::from_scale(Vec3::splat(scale)),
        ));
    }
}
