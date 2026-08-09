use cesium_bevy_render::{
    CesiumEntityPlugin, AnimationClock,
    BillboardGraphicsComponent,
    CesiumEntity, EntityWrapper,
    ModelGraphicsComponent, NeedsVisualUpdate,
    PointGraphicsComponent, PolygonGraphicsComponent,
    PolylineGraphicsComponent, TimeDynamicProperties,
};
use cesium_datasource::entity::Entity as DomainEntity;
use cesium_datasource::entity_collection::EntityCollection;

use super::create_test_app;

#[test]
fn test_entity_plugin_registers() {
    let mut app = create_test_app();
    app.add_plugins(CesiumEntityPlugin);
}

#[test]
fn test_globe_ellipsoid_resource_initialized() {
    let mut app = create_test_app();
    app.add_plugins(CesiumEntityPlugin);

    let ellipsoid = app.world().get_resource::<cesium_bevy_render::GlobeEllipsoid>();
    assert!(ellipsoid.is_some());
}

#[test]
fn test_animation_clock_resource_initialized() {
    let mut app = create_test_app();
    app.add_plugins(CesiumEntityPlugin);

    let clock = app.world().get_resource::<AnimationClock>();
    assert!(clock.is_some());
}

#[test]
fn test_point_graphics_component() {
    let mut app = create_test_app();
    app.add_plugins(CesiumEntityPlugin);

    let pg = PointGraphicsComponent {
        pixel_size: 5.0,
        color: [1.0, 0.0, 0.0, 1.0],
        outline_color: [0.0, 0.0, 0.0, 1.0],
        outline_width: 2.0,
    };

    let entity = app.world_mut().spawn(pg).id();

    let pg = app.world().get::<PointGraphicsComponent>(entity);
    assert!(pg.is_some());
    let pg = pg.unwrap();
    assert!((pg.pixel_size - 5.0).abs() < 1e-6);
    assert_eq!(pg.color, [1.0, 0.0, 0.0, 1.0]);
}

#[test]
fn test_polyline_graphics_component() {
    let mut app = create_test_app();
    app.add_plugins(CesiumEntityPlugin);

    let pl = PolylineGraphicsComponent {
        positions: vec![
            glam::DVec3::new(0.0, 0.0, 0.0),
            glam::DVec3::new(1.0, 1.0, 1.0),
        ],
        width: 3.0,
        material_color: [0.0, 1.0, 0.0, 1.0],
        clamp_to_ground: true,
    };

    let entity = app.world_mut().spawn(pl).id();

    let pl = app.world().get::<PolylineGraphicsComponent>(entity);
    assert!(pl.is_some());
    let pl = pl.unwrap();
    assert_eq!(pl.positions.len(), 2);
    assert!((pl.width - 3.0).abs() < 1e-6);
    assert!(pl.clamp_to_ground);
}

#[test]
fn test_polygon_graphics_component() {
    let mut app = create_test_app();
    app.add_plugins(CesiumEntityPlugin);

    let poly = PolygonGraphicsComponent {
        positions: vec![
            glam::DVec3::new(0.0, 0.0, 0.0),
            glam::DVec3::new(1.0, 0.0, 0.0),
            glam::DVec3::new(0.0, 1.0, 0.0),
        ],
        holes: Vec::new(),
        height: 100.0,
        extruded_height: Some(200.0),
        material_color: [0.0, 0.0, 1.0, 0.8],
        outline: true,
        outline_color: [1.0, 1.0, 1.0, 1.0],
    };

    let entity = app.world_mut().spawn(poly).id();

    let poly = app.world().get::<PolygonGraphicsComponent>(entity);
    assert!(poly.is_some());
    let poly = poly.unwrap();
    assert!((poly.height - 100.0).abs() < 1e-10);
    assert_eq!(poly.extruded_height, Some(200.0));
    assert!(poly.outline);
}

#[test]
fn test_billboard_graphics_component() {
    let mut app = create_test_app();
    app.add_plugins(CesiumEntityPlugin);

    let bb = BillboardGraphicsComponent {
        image_url: Some("https://example.com/image.png".into()),
        scale: 2.0,
        color: [1.0, 1.0, 1.0, 1.0],
    };

    let entity = app.world_mut().spawn(bb).id();

    let bb = app.world().get::<BillboardGraphicsComponent>(entity);
    assert!(bb.is_some());
    let bb = bb.unwrap();
    assert_eq!(bb.image_url.as_deref(), Some("https://example.com/image.png"));
    assert!((bb.scale - 2.0).abs() < 1e-6);
}

#[test]
fn test_model_graphics_component() {
    let mut app = create_test_app();
    app.add_plugins(CesiumEntityPlugin);

    let model = ModelGraphicsComponent {
        uri: "models/building.glb".into(),
        scale: 1.5,
        minimum_pixel_size: 32.0,
    };

    let entity = app.world_mut().spawn(model).id();

    let model = app.world().get::<ModelGraphicsComponent>(entity);
    assert!(model.is_some());
    let model = model.unwrap();
    assert_eq!(model.uri, "models/building.glb");
    assert!((model.scale - 1.5).abs() < 1e-6);
}

#[test]
fn test_cesium_entity_component() {
    let mut app = create_test_app();
    app.add_plugins(CesiumEntityPlugin);

    let entity = CesiumEntity::new("entity-01", "Test Entity");

    let e = app.world_mut().spawn(entity).id();
    let ce = app.world().get::<CesiumEntity>(e);
    assert!(ce.is_some());
    assert_eq!(ce.unwrap().entity_id, "entity-01");
}

#[test]
fn test_time_dynamic_properties() {
    let mut app = create_test_app();
    app.add_plugins(CesiumEntityPlugin);

    let props = TimeDynamicProperties {
        has_interpolated_position: true,
        has_interpolated_color: true,
        has_interpolated_orientation: false,
        has_availability: true,
    };

    let entity = app.world_mut().spawn(props).id();

    let props = app.world().get::<TimeDynamicProperties>(entity);
    assert!(props.is_some());
    assert!(props.unwrap().has_interpolated_position);
}

#[test]
fn test_entity_wrapper_spawn() {
    let mut app = create_test_app();
    app.add_plugins(CesiumEntityPlugin);

    let domain_entity = DomainEntity::new("test-01");
    let wrapper = EntityWrapper::new(domain_entity);

    let entity = app.world_mut().spawn((wrapper, NeedsVisualUpdate)).id();

    let wrapper = app.world().get::<EntityWrapper>(entity);
    assert!(wrapper.is_some());
    assert!(app.world().get::<NeedsVisualUpdate>(entity).is_some());
}

#[test]
fn test_entity_collection_operations() {
    let mut collection = EntityCollection::new();
    assert_eq!(collection.len(), 0);

    let entity = DomainEntity::new("add-01");
    collection.add(entity.clone());
    assert_eq!(collection.len(), 1);

    let found = collection.get("add-01");
    assert!(found.is_some());

    assert!(collection.remove_by_id("add-01"));
    assert_eq!(collection.len(), 0);
    assert!(collection.get("add-01").is_none());
}
