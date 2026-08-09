use cesium_bevy_render::{CesiumTerrainPlugin, CesiumTerrainTile};

use super::create_test_app;

#[test]
fn test_terrain_plugin_registers() {
    let mut app = create_test_app();
    app.add_plugins(CesiumTerrainPlugin);
}

#[test]
fn test_terrain_selection_resource_initialized() {
    let mut app = create_test_app();
    app.add_plugins(CesiumTerrainPlugin);

    let selection = app.world().get_resource::<cesium_bevy_render::terrain::TerrainSelection>();
    assert!(selection.is_some());
}

#[test]
fn test_terrain_load_state_resource_initialized() {
    let mut app = create_test_app();
    app.add_plugins(CesiumTerrainPlugin);

    let state = app.world().get_resource::<cesium_bevy_render::terrain::TerrainLoadState>();
    assert!(state.is_some());
}

#[test]
fn test_terrain_tile_component_spawn() {
    let mut app = create_test_app();
    app.add_plugins(CesiumTerrainPlugin);

    let tile = CesiumTerrainTile {
        x: 5,
        y: 10,
        level: 3,
    };

    let entity = app.world_mut().spawn(tile).id();

    let tile = app.world().get::<CesiumTerrainTile>(entity);
    assert!(tile.is_some());
    let tile = tile.unwrap();
    assert_eq!(tile.x, 5);
    assert_eq!(tile.y, 10);
    assert_eq!(tile.level, 3);
}

#[test]
fn test_multiple_terrain_tiles_spawn() {
    let mut app = create_test_app();
    app.add_plugins(CesiumTerrainPlugin);

    let mut entities = Vec::new();
    for level in 0..3 {
        for x in 0..(1 << level) {
            for y in 0..(1 << level) {
                let entity = app.world_mut().spawn(CesiumTerrainTile { x, y, level }).id();
                entities.push(entity);
            }
        }
    }

    let mut tile_count = 0;
    for entity in &entities {
        if app.world().get::<CesiumTerrainTile>(*entity).is_some() {
            tile_count += 1;
        }
    }
    assert_eq!(tile_count, 1 + 4 + 16);
}
