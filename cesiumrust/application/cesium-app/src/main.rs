//! cesium-app: CesiumRust Globe Viewer
//!
//! An interactive 3D globe application using per-tile rendering (CesiumJS architecture):
//! - Each Web Mercator tile gets its own mesh patch on the ellipsoid
//! - Each tile has its own satellite imagery texture
//! - Dynamic LOD: tiles switch to higher resolution when zooming in
//! - Atmospheric limb glow
//! - Starfield background
//! - Orbit camera (drag to rotate, scroll to zoom)

use bevy::prelude::*;
use cesium_bevy_render::{CesiumRenderPlugin, Globe};

mod orbit_camera;
mod starfield;
mod atmosphere_glow;
mod tile_loader;
mod bing_tile_loader;
mod tile_mesh;
mod dynamic_globe;

use orbit_camera::OrbitCameraPlugin;
use starfield::StarfieldPlugin;
use atmosphere_glow::AtmosphereGlowPlugin;
use dynamic_globe::DynamicGlobePlugin;
use tile_mesh::{create_polar_cap, render_scale};

/// Mesh subdivisions per tile (16x16 quads = 17x17 vertices).
const TILE_SEGMENTS: u32 = 16;

/// Plugin that spawns the base sphere and polar caps (non-LOD elements).
struct BaseSpherePlugin;

impl Plugin for BaseSpherePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_base_sphere);
    }
}

/// Spawns the base sphere and polar caps that underlie the dynamic tiles.
fn spawn_base_sphere(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let scale = render_scale();

    // --- Base sphere (safety net) ---
    // A slightly-smaller solid ocean-blue sphere underneath the tiles. It is
    // never visible where tiles/caps exist, but it guarantees that no gap,
    // seam, or uncovered region ever shows through to the black background.
    let base_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.02, 0.09, 0.25),
        perceptual_roughness: 1.0,
        ..default()
    });
    commands.spawn((
        Globe,
        Mesh3d(meshes.add(Sphere::new(1.0).mesh().build())),
        MeshMaterial3d(base_material),
        // Slightly inside the tile surface (tiles sit at radius ~0.9966..1.0
        // after scaling), so it never z-fights with the imagery tiles.
        Transform::from_scale(Vec3::splat(scale * 0.99)),
    ));

    // --- Polar caps ---
    // Web Mercator tiles only cover +/-85.05 deg latitude, leaving holes at
    // the poles. Cap them with white ice meshes (matches CesiumJS, where the
    // globe terrain covers the poles and imagery only drapes the tiled zone).
    let cap_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.92, 0.95, 0.98),
        perceptual_roughness: 0.95,
        ..default()
    });
    for &north in &[true, false] {
        commands.spawn((
            Globe,
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
                title: "CesiumRust Globe Viewer".to_string(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        // Black space background
        .insert_resource(ClearColor(Color::BLACK))
        // Core: lighting
        .add_plugins(CesiumRenderPlugin)
        // Base sphere + polar caps (non-LOD)
        .add_plugins(BaseSpherePlugin)
        // Dynamic LOD globe tiles with Bing Maps imagery
        .add_plugins(DynamicGlobePlugin)
        // Interactive orbit camera
        .add_plugins(OrbitCameraPlugin)
        // Starfield background
        .add_plugins(StarfieldPlugin)
        // Atmosphere rim glow
        .add_plugins(AtmosphereGlowPlugin)
        .run();
}
