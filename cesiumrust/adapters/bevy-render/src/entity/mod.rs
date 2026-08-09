//! Cesium entity plugin for Bevy.
//!
//! Provides entity rendering, time-dynamic updates, and visualizer systems.

pub mod components;
pub mod time_system;
pub mod visualizer;

use bevy::prelude::*;

use self::components::GlobeEllipsoid;
use self::time_system::{entity_visibility_system, time_dynamic_update_system, AnimationClock};
use self::visualizer::{billboard_face_camera_system, entity_visualizer_system};

pub struct CesiumEntityPlugin;

impl Plugin for CesiumEntityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GlobeEllipsoid>()
            .init_resource::<AnimationClock>()
            .add_systems(
                Update,
                (
                    time_dynamic_update_system,
                    entity_visualizer_system,
                    entity_visibility_system,
                    billboard_face_camera_system,
                ),
            );
    }
}
