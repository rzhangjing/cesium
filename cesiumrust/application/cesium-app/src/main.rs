//! cesium-app: Bevy App assembly, System orchestration, plugin registration
//!
//! This is the application layer that orchestrates domain and adapters.
//! It sets up the Bevy app with the CesiumRust rendering plugin.

use bevy::prelude::*;
use cesium_bevy_render::CesiumRenderPlugin;

mod material_showcase;
use material_showcase::MaterialShowcasePlugin;

mod geometry_showcase;
use geometry_showcase::GeometryShowcasePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "CesiumRust - DDD + Hexagonal Architecture × Bevy".to_string(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(CesiumRenderPlugin)
        .add_plugins(MaterialShowcasePlugin)
        .add_plugins(GeometryShowcasePlugin)
        .run();
}
