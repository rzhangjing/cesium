pub mod lod_system;
pub mod render_system;
pub mod tile_loader;

use bevy::prelude::*;

pub use lod_system::{terrain_lod_system, TerrainSelection};
pub use render_system::{terrain_render_system, TerrainRenderMap};
pub use tile_loader::{terrain_tile_load_system, TerrainLoadState, TerrainPendingLoads};

use crate::imagery::{ImageryCache, ImageryLayerManager};
use crate::resources::TileLoadStats;

pub struct CesiumTerrainPlugin;

impl Plugin for CesiumTerrainPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainSelection>()
            .init_resource::<TerrainLoadState>()
            .init_resource::<TerrainPendingLoads>()
            .init_resource::<TerrainRenderMap>()
            // M4.3: terrain_render_system 为简单垂辖路径读取 ImageryCache + ImageryLayerManager。
            // init_resource 是幂等的——当 CesiumImageryPlugin 也被注册时，
            // 这些调用是空操作。
            // 若没有它们，单独使用的 CesiumTerrainPlugin 会因
            // resource 缺失而 panic。
            .init_resource::<ImageryCache>()
            .init_resource::<ImageryLayerManager>()
            // terrain_tile_load_system 写入 TileLoadStats；与
            // CesiumCorePlugin 的注册幂等。
            .init_resource::<TileLoadStats>()
            .add_systems(PreUpdate, terrain_lod_system)
            .add_systems(
                Update,
                // 防御性排序：loader 必须在 renderer 之前运行，以便
                // 刚解析完的 `Ready` tile 能在同一帧被取走。
                (terrain_tile_load_system, terrain_render_system).chain(),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::App;
    use bevy::asset::AssetPlugin;
    use bevy::pbr::StandardMaterial;
    use bevy::render::mesh::Mesh;
    use bevy::image::Image;

    /// 针对 Ultra Review Critical 的回归测试：CesiumTerrainPlugin 必须
    /// 注册 ImageryCache + ImageryLayerManager + TileLoadStats，以便它能在
    /// 没有 CesiumImageryPlugin 或 CesiumCorePlugin 时也能运行。
    #[test]
    fn terrain_plugin_registers_imagery_resources() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CesiumTerrainPlugin);
        let world = app.world();
        assert!(
            world.contains_resource::<ImageryCache>(),
            "CesiumTerrainPlugin must init_resource::<ImageryCache>()"
        );
        assert!(
            world.contains_resource::<ImageryLayerManager>(),
            "CesiumTerrainPlugin must init_resource::<ImageryLayerManager>()"
        );
        assert!(
            world.contains_resource::<TileLoadStats>(),
            "CesiumTerrainPlugin must init_resource::<TileLoadStats>()"
        );
    }

    /// 完整集成：CesiumTerrainPlugin 单独配合 asset 基础设施时，
    /// app.update() 必须不因 resource 缺失而 panic。
    #[test]
    fn terrain_plugin_standalone_update_does_not_panic() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.init_asset::<Image>();
        app.add_plugins(CesiumTerrainPlugin);
        // 不得 panic——所有 cesiumrust resource 都自行注册。
        app.update();
    }
}
