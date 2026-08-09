pub mod tileset_integration_test;
pub mod terrain_integration_test;
pub mod camera_integration_test;
pub mod entity_integration_test;
pub mod imagery_integration_test;
pub mod material_integration_test;

use bevy::prelude::*;
use cesium_bevy_render::CesiumCorePlugin;

pub fn create_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(CesiumCorePlugin);
    app
}
