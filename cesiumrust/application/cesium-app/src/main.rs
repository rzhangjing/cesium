//! cesium-app: CesiumRust 3D Globe Viewer
//!
//! Interactive 3D globe with:
//! - Base sphere + polar caps (non-LOD safety net)
//! - Dynamic LOD tiles with Bing Maps satellite imagery
//! - Orbit camera (mouse drag to rotate, scroll to zoom)
//! - Atmospheric limb glow + starfield background

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use cesium_bevy_render::{CesiumCorePlugin, CesiumGlobe};

mod orbit_camera;
mod starfield;
mod atmosphere_glow;
mod tile_mesh;
mod dynamic_globe;

use orbit_camera::OrbitCameraPlugin;
use starfield::StarfieldPlugin;
use atmosphere_glow::AtmosphereGlowPlugin;
use dynamic_globe::DynamicGlobePlugin;
use tile_mesh::{create_polar_cap, create_uv_sphere, render_scale};

const TILE_SEGMENTS: u32 = 16;

/// Plugin that spawns the base sphere and polar caps.
struct BaseSpherePlugin;

impl Plugin for BaseSpherePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_base_sphere);
    }
}

fn spawn_base_sphere(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let scale = render_scale();

    // Base sphere — high-subdivision UV sphere so the horizon silhouette is
    // smooth; slightly smaller to stay below tiles and polar caps.
    let base_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.02, 0.09, 0.25),
        perceptual_roughness: 1.0,
        ..default()
    });
    commands.spawn((
        CesiumGlobe,
        Mesh3d(meshes.add(create_uv_sphere(96, 48))),
        MeshMaterial3d(base_material),
        Transform::from_scale(Vec3::splat(scale * 0.99)),
    ));

    // Polar caps — soft ice white, tucked below the tile surface in the mesh.
    let cap_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.88, 0.92, 0.96),
        perceptual_roughness: 0.95,
        ..default()
    });
    for &north in &[true, false] {
        commands.spawn((
            CesiumGlobe,
            Mesh3d(meshes.add(create_polar_cap(north, TILE_SEGMENTS * 4))),
            MeshMaterial3d(cap_material.clone()),
            Transform::from_scale(Vec3::splat(scale)),
        ));
    }
    println!("[BaseSphere] Spawned base sphere + 2 polar caps");
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "CesiumRust - 3D Globe Viewer".into(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::BLACK))
        // FPS / frame-time diagnostics (console) for performance validation
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(LogDiagnosticsPlugin::default())
        // Core: lighting + globe config
        .add_plugins(CesiumCorePlugin)
        // Camera: mouse orbit/zoom
        .add_plugins(OrbitCameraPlugin)
        // Globe rendering
        .add_plugins(BaseSpherePlugin)
        .add_plugins(DynamicGlobePlugin)
        // Visual effects
        .add_plugins(AtmosphereGlowPlugin)
        .add_plugins(StarfieldPlugin)
        // ── New architecture plugins (add gradually) ──
        // Phase 1: uncomment one by one to verify
        // .add_plugins(CesiumTilesetPlugin)
        // .add_plugins(CesiumCameraPlugin)
        // .add_plugins(CesiumImageryPlugin)
        // .add_plugins(CesiumTerrainPlugin)
        // .add_plugins(CesiumEntityPlugin)
        // .add_plugins(CesiumMaterialPlugin)
        // .add_plugins(CesiumAtmospherePlugin)
        .run();
}
