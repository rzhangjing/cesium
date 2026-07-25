//! cesium-app: CesiumRust Globe Viewer
//!
//! An interactive 3D globe application mimicking CesiumJS Hello World:
//! - Procedural Earth texture (oceans, continents, ice caps)
//! - Atmospheric limb glow
//! - Starfield background
//! - Orbit camera (drag to rotate, scroll to zoom)

use bevy::prelude::*;
use cesium_bevy_render::CesiumRenderPlugin;

mod orbit_camera;
mod starfield;
mod atmosphere_glow;

use orbit_camera::OrbitCameraPlugin;
use starfield::StarfieldPlugin;
use atmosphere_glow::AtmosphereGlowPlugin;

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
        // Core: globe mesh + sun light
        .add_plugins(CesiumRenderPlugin)
        // Interactive orbit camera
        .add_plugins(OrbitCameraPlugin)
        // Starfield background
        .add_plugins(StarfieldPlugin)
        // Atmosphere rim glow
        .add_plugins(AtmosphereGlowPlugin)
        .run();
}
