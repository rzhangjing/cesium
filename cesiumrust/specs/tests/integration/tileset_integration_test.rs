use cesium_bevy_render::{
    CesiumTileNode, CesiumTilesetPlugin, CesiumTilesetRoot,
    TileContentState, TilesetLoadingState,
};

use super::create_test_app;

#[test]
fn test_tileset_plugin_registers() {
    let mut app = create_test_app();
    app.add_plugins(CesiumTilesetPlugin);
}

#[test]
fn test_tile_selection_resource_initialized() {
    let mut app = create_test_app();
    app.add_plugins(CesiumTilesetPlugin);

    let selection = app.world().get_resource::<cesium_bevy_render::tileset::TileSelection>();
    assert!(selection.is_some());
    let selection = selection.unwrap();
    assert!(selection.tiles_to_load.is_empty());
    assert!(selection.tiles_to_unload.is_empty());
}

#[test]
fn test_tileset_root_component_spawn() {
    let mut app = create_test_app();
    app.add_plugins(CesiumTilesetPlugin);

    let root_entity = app.world_mut().spawn(CesiumTilesetRoot {
        url: "https://example.com/tileset.json".into(),
        loading_state: TilesetLoadingState::NotLoaded,
    }).id();

    let root = app.world().get::<CesiumTilesetRoot>(root_entity);
    assert!(root.is_some());
    let root = root.unwrap();
    assert_eq!(root.url, "https://example.com/tileset.json");
    assert!(matches!(root.loading_state, TilesetLoadingState::NotLoaded));
}

#[test]
fn test_tileset_loading_state_transitions() {
    let states = vec![
        TilesetLoadingState::NotLoaded,
        TilesetLoadingState::Loading,
        TilesetLoadingState::Ready,
        TilesetLoadingState::Failed("error".into()),
    ];

    let mut app = create_test_app();

    for state in states {
        let entity = app.world_mut().spawn(CesiumTilesetRoot {
            url: "test".into(),
            loading_state: state,
        }).id();

        let loaded = app.world().get::<CesiumTilesetRoot>(entity);
        assert!(loaded.is_some());
    }
}

#[test]
fn test_tile_node_spawning_with_bounding_volume() {
    let mut app = create_test_app();
    app.add_plugins(CesiumTilesetPlugin);

    let node = CesiumTileNode {
        path: vec![0, 1, 2],
        screen_space_error: 16.0,
        geometric_error: 100.0,
        state: TileContentState::Unloaded,
        bounding_sphere_center: Some(glam::DVec3::new(1.0, 2.0, 3.0)),
        bounding_sphere_radius: Some(6378137.0),
    };

    let entity = app.world_mut().spawn(node).id();

    let node = app.world().get::<CesiumTileNode>(entity);
    assert!(node.is_some());
    let node = node.unwrap();
    assert_eq!(node.path, vec![0, 1, 2]);
    assert!(matches!(node.state, TileContentState::Unloaded));
    assert!(node.bounding_sphere_center.is_some());
    assert!((node.bounding_sphere_radius.unwrap() - 6378137.0).abs() < 1e-10);
}
