//! cesium-app: CesiumRust Globe Viewer
//!
//! An interactive 3D globe application using per-tile rendering (CesiumJS architecture):
//! - Each Web Mercator tile gets its own mesh patch on the ellipsoid
//! - Each tile has its own satellite imagery texture
//! - Atmospheric limb glow
//! - Starfield background
//! - Orbit camera (drag to rotate, scroll to zoom)

use bevy::prelude::*;
use cesium_bevy_render::{CesiumRenderPlugin, Globe};

mod orbit_camera;
mod starfield;
mod atmosphere_glow;
mod tile_loader;
mod tile_mesh;

use orbit_camera::OrbitCameraPlugin;
use starfield::StarfieldPlugin;
use atmosphere_glow::AtmosphereGlowPlugin;
use tile_loader::TileLoaderPlugin;
use tile_mesh::{create_polar_cap, create_tile_mesh, render_scale, GlobeTile};

/// Zoom level for the globe tile grid.
const GLOBE_ZOOM: u32 = 3;
/// Mesh subdivisions per tile (16x16 quads = 17x17 vertices).
const TILE_SEGMENTS: u32 = 16;

/// Plugin that spawns per-tile globe entities.
struct GlobeTilePlugin;

impl Plugin for GlobeTilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_globe_tiles);
    }
}

/// Spawns one entity per Web Mercator tile at the configured zoom level.
fn spawn_globe_tiles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let num_tiles = 1u32 << GLOBE_ZOOM;
    let scale = render_scale();

    for ty in 0..num_tiles {
        for tx in 0..num_tiles {
            let mesh = create_tile_mesh(tx, ty, GLOBE_ZOOM, TILE_SEGMENTS);

            // Each tile needs its OWN material instance (handles are references,
            // so sharing one material would make all tiles show the same texture).
            // base_color starts as ocean blue as a fallback before imagery loads;
            // it will be reset to white when the texture is applied (since Bevy
            // multiplies base_color with base_color_texture).
            let material = materials.add(StandardMaterial {
                base_color: Color::srgb(0.04, 0.15, 0.4),
                perceptual_roughness: 0.9,
                ..default()
            });

            commands.spawn((
                Globe,
                GlobeTile {
                    x: tx,
                    y: ty,
                    z: GLOBE_ZOOM,
                },
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                Transform::from_scale(Vec3::splat(scale)),
            ));
        }
    }

    println!(
        "[GlobeTile] Spawned {}x{} = {} tile entities at zoom {}",
        num_tiles,
        num_tiles,
        num_tiles * num_tiles,
        GLOBE_ZOOM
    );

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
    println!("[GlobeTile] Spawned base sphere + 2 polar caps");
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
        // Per-tile globe entities
        .add_plugins(GlobeTilePlugin)
        // Interactive orbit camera
        .add_plugins(OrbitCameraPlugin)
        // Starfield background
        .add_plugins(StarfieldPlugin)
        // Atmosphere rim glow
        .add_plugins(AtmosphereGlowPlugin)
        // Per-tile satellite imagery
        .add_plugins(TileLoaderPlugin)
        .run();
}
