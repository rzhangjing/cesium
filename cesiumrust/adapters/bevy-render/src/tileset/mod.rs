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
            // 显式顺序：tileset JSON 必须先被加载，遍历
            // 才能从中选取 tile。
            .add_systems(
                PreUpdate,
                (tileset_load_system, tileset_traversal_system).chain(),
            )
            // 显式顺序：loader 消费 `tiles_to_load` 并把 tile 翻为
            // `Ready`（带 mesh handle），赶在渲染系统扫描它们之前，而样式化最后
            // 运行，使它能看到新创建的材质。若没
            // `.chain()`，Bevy 可能并行运行这些，而
            // 渲染侧会落后 loader 整整一帧。
            .add_systems(
                Update,
                (
                    tile_content_load_system,
                    tile_render_system,
                    tile_style_system,
                )
                    .chain(),
            );
    }
}
