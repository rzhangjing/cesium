use bevy::prelude::*;

use super::debug_system::{
    debug_toggle_system, draw_bounding_volumes, spawn_stats_overlay, update_tile_stats,
    DebugConfig,
};

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugConfig>()
            .add_systems(Startup, spawn_stats_overlay)
            .add_systems(
                Update,
                (debug_toggle_system, draw_bounding_volumes, update_tile_stats),
            );
    }
}
