use bevy::prelude::*;
use glam::DVec3;

use crate::resources::RenderScale;

use super::picking_system::{handle_mouse_click, ray_cast_tiles, report_pick_results, PendingPick};

#[derive(Event)]
pub struct TilePickEvent {
    pub screen_x: f32,
    pub screen_y: f32,
    pub tileset_entity: Entity,
    pub tile_path: Vec<usize>,
    pub position: Option<DVec3>,
}

pub struct TilePickingPlugin;

impl Plugin for TilePickingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingPick>()
            .init_resource::<RenderScale>()
            .add_event::<TilePickEvent>()
            .add_systems(
                Update,
                (handle_mouse_click, ray_cast_tiles, report_pick_results).chain(),
            );
    }
}
