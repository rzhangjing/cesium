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
            // M4.3: terrain_render_system reads ImageryCache + ImageryLayerManager
            // for the simple draping path. init_resource is idempotent — when
            // CesiumImageryPlugin is also registered, these calls are no-ops.
            // Without them, CesiumTerrainPlugin used standalone would panic on
            // missing resources.
            .init_resource::<ImageryCache>()
            .init_resource::<ImageryLayerManager>()
            // terrain_tile_load_system writes TileLoadStats; idempotent with
            // CesiumCorePlugin's registration.
            .init_resource::<TileLoadStats>()
            .add_systems(PreUpdate, terrain_lod_system)
            .add_systems(
                Update,
                // Defensive ordering: the loader must run before the renderer so
                // freshly-resolved `Ready` tiles can be picked up in the same frame.
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

    /// Regression test for Ultra Review Critical: CesiumTerrainPlugin must
    /// register ImageryCache + ImageryLayerManager + TileLoadStats so it can
    /// run without CesiumImageryPlugin or CesiumCorePlugin.
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

    /// Full integration: CesiumTerrainPlugin standalone with asset infra,
    /// app.update() must not panic on missing resources.
    #[test]
    fn terrain_plugin_standalone_update_does_not_panic() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.init_asset::<Image>();
        app.add_plugins(CesiumTerrainPlugin);
        // Must not panic — all cesiumrust resources are self-registered.
        app.update();
    }
}
