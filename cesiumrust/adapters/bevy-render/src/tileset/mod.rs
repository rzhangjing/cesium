pub mod content_loader;
pub mod debug_plugin;
pub mod debug_system;
pub mod loader;
pub mod picking;
pub mod picking_system;
pub mod render_system;
pub mod style_system;
pub mod traversal_system;

use bevy::prelude::*;

pub use content_loader::tile_content_load_system;
pub use loader::{tileset_load_system, LoadedTileset, TilesetFetchState};
pub use render_system::{tile_render_system, TileRenderMap};
pub use style_system::tile_style_system;
pub use traversal_system::{tileset_traversal_system, TileSelection};

pub struct CesiumTilesetPlugin;

impl Plugin for CesiumTilesetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadedTileset>()
            .init_resource::<TilesetFetchState>()
            .init_resource::<TileSelection>()
            .init_resource::<TileRenderMap>()
            .init_resource::<content_loader::PendingTileLoads>()
            .add_systems(PreUpdate, (tileset_load_system, tileset_traversal_system))
            .add_systems(
                Update,
                (tile_content_load_system, tile_render_system, tile_style_system),
            );
    }
}
