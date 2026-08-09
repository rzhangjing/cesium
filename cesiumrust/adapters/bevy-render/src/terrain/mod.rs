pub mod lod_system;
pub mod render_system;
pub mod tile_loader;

use bevy::prelude::*;

pub use lod_system::{terrain_lod_system, TerrainSelection};
pub use render_system::{terrain_render_system, TerrainRenderMap};
pub use tile_loader::{terrain_tile_load_system, TerrainLoadState, TerrainPendingLoads};

pub struct CesiumTerrainPlugin;

impl Plugin for CesiumTerrainPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainSelection>()
            .init_resource::<TerrainLoadState>()
            .init_resource::<TerrainPendingLoads>()
            .init_resource::<TerrainRenderMap>()
            .add_systems(PreUpdate, terrain_lod_system)
            .add_systems(
                Update,
                (terrain_tile_load_system, terrain_render_system),
            );
    }
}
