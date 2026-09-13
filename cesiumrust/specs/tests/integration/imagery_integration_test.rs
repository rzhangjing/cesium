use cesium_bevy_render::{
    CesiumImageryPlugin,
    imagery::{ImageryCache, ImageryLayerManager},
    CesiumImageryLayer,
};

use super::create_test_app;

#[test]
fn test_imagery_plugin_registers() {
    let mut app = create_test_app();
    app.add_plugins(CesiumImageryPlugin);
}

#[test]
fn test_imagery_layer_manager_resource_initialized() {
    let mut app = create_test_app();
    app.add_plugins(CesiumImageryPlugin);

    let mgr = app.world().get_resource::<ImageryLayerManager>();
    assert!(mgr.is_some());
    let mgr = mgr.unwrap();
    assert_eq!(mgr.layer_count(), 0);
}

#[test]
fn test_imagery_layer_manager_operations() {
    let mut app = create_test_app();
    app.add_plugins(CesiumImageryPlugin);

    {
        let world = app.world_mut();
        let mut mgr = world.resource_mut::<ImageryLayerManager>();
        let id = mgr.add_layer("https://tiles/{z}/{x}/{y}.png", 0.8, 0, 18);
        assert_eq!(id, 1);
        assert_eq!(mgr.layer_count(), 1);
    }

    {
        let world = app.world_mut();
        let mgr = world.resource::<ImageryLayerManager>();
        assert!(mgr.get_layer(1).is_some());
        assert_eq!(mgr.visible_layers().count(), 1);
    }

    {
        let world = app.world_mut();
        let mut mgr = world.resource_mut::<ImageryLayerManager>();
        mgr.remove_layer(1);
        assert_eq!(mgr.layer_count(), 0);
    }
}

#[test]
fn test_imagery_cache_resource_initialized() {
    let mut app = create_test_app();
    app.add_plugins(CesiumImageryPlugin);

    let cache = app.world().get_resource::<ImageryCache>();
    assert!(cache.is_some());
}

#[test]
fn test_imagery_pending_loads_resource_initialized() {
    let mut app = create_test_app();
    app.add_plugins(CesiumImageryPlugin);

    let pending = app.world().get_resource::<cesium_bevy_render::imagery::ImageryPendingLoads>();
    assert!(pending.is_some());
}

#[test]
fn test_imagery_blend_cache_resource_initialized() {
    let mut app = create_test_app();
    app.add_plugins(CesiumImageryPlugin);

    let blend = app.world().get_resource::<cesium_bevy_render::imagery::ImageryBlendCache>();
    assert!(blend.is_some());
}

#[test]
fn test_imagery_layer_component_spawn() {
    let mut app = create_test_app();
    app.add_plugins(CesiumImageryPlugin);

    let layer = CesiumImageryLayer {
        layer_index: 0,
        opacity: 0.85,
        visible: true,
    };

    let entity = app.world_mut().spawn(layer).id();

    let layer = app.world().get::<CesiumImageryLayer>(entity);
    assert!(layer.is_some());
    let layer = layer.unwrap();
    assert_eq!(layer.layer_index, 0);
    assert!((layer.opacity - 0.85).abs() < 1e-6);
    assert!(layer.visible);
}

#[test]
fn test_multiple_imagery_layers() {
    let mut app = create_test_app();
    app.add_plugins(CesiumImageryPlugin);

    for i in 0..5 {
        app.world_mut().spawn(CesiumImageryLayer {
            layer_index: i,
            opacity: 1.0,
            visible: true,
        });
    }

    let mut query = app.world_mut().query::<&CesiumImageryLayer>();
    let count = query.iter(app.world()).count();
    assert_eq!(count, 5);
}
