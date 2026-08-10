//! Atmosphere glow — nested translucent blue shells around the globe.
//!
//! Simulates the atmospheric limb glow visible from space. Each shell is
//! rendered BACK-FACES ONLY with additive blending, so it appears solely as a
//! halo ring around the limb. Eight shells with decreasing alpha produce a
//! soft gradient falloff instead of a single hard edge.
//!
//! The back-face trick only works while the camera stays OUTSIDE the shells.
//! Once a zoom brings the camera inside them, the far hemispheres surround
//! the view and paint large flat blue regions across the screen. The shells
//! therefore fade out with camera distance and vanish entirely near the
//! surface (where no limb halo is visible anyway).

use bevy::prelude::*;

use crate::orbit_camera::OrbitState;

/// Plugin that spawns the atmosphere glow shells.
pub struct AtmosphereGlowPlugin;

impl Plugin for AtmosphereGlowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_atmosphere)
            .add_systems(Update, fade_atmosphere_with_distance);
    }
}

/// Per-shell base additive alpha, restored when the camera is far away.
#[derive(Component)]
struct AtmosphereShell {
    base_alpha: f32,
}

/// Glow is fully visible from space (distance >= 3 R) and fades to zero as
/// the camera approaches the surface (<= 1.5 R), so zooming in never shows
/// blue shell interiors.
fn glow_fade(distance: f32) -> f32 {
    let t = ((distance - 1.5) / 1.5).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
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
            AtmosphereShell { base_alpha: alpha },
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

/// Scale every shell's additive alpha by the camera-distance fade so the
/// halo never paints blue regions once the camera dives inside the shells.
fn fade_atmosphere_with_distance(
    orbit: Res<OrbitState>,
    shells: Query<(&AtmosphereShell, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let fade = glow_fade(orbit.distance);
    for (shell, mat_handle) in &shells {
        if let Some(mat) = materials.get_mut(mat_handle) {
            mat.base_color = Color::srgba(0.3, 0.6, 1.0, shell.base_alpha * fade);
        }
    }
}
