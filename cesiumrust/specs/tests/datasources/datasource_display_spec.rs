//! DataSourceDisplay + GeometryVisualizer + StaticGeometryBatch + DynamicGeometryUpdater tests.
//!
//! Maps to CesiumJS:
//! - DataSources/DataSourceDisplaySpec.js
//! - DataSources/GeometryVisualizerSpec.js
//! - DataSources/StaticGeometryColorBatchSpec.js
//! - DataSources/DynamicGeometryUpdaterSpec.js
//!
//! A-class tests: display coordination, geometry caching, primitive sync, batching.

use cesium_datasource::datasource_display::{DataSourceDisplay, MultiDataSourceDisplay};
use cesium_datasource::entity::*;
use cesium_datasource::entity_collection::{DataSource, EntityCollection};
use cesium_datasource::geometry_updater::EntityGeometry;
use cesium_datasource::property::Property;
use cesium_datasource::visualizer::{
    DynamicGeometryUpdater, GeometryVisualizer, StaticGeometryBatch,
};
use cesium_geospatial::Ellipsoid;

// === Helper ===

fn make_test_entities() -> EntityCollection {
    let mut c = EntityCollection::new();
    c.add(
        Entity::new("box-1")
            .with_position(0.0, 0.0, 0.0)
            .with_box(BoxGraphics {
                dimensions: Property::Constant([100.0, 200.0, 300.0]),
                ..Default::default()
            }),
    );
    c.add(
        Entity::new("bb-1")
            .with_position(0.1, 0.2, 0.0)
            .with_billboard(BillboardGraphics {
                image: Property::Constant("icon.png".to_string()),
                scale: Property::Constant(2.5),
                ..Default::default()
            }),
    );
    c.add(
        Entity::new("label-1")
            .with_position(0.3, 0.4, 0.0)
            .with_label(LabelGraphics {
                text: Property::Constant("Test Label".to_string()),
                ..Default::default()
            }),
    );
    c.add(
        Entity::new("point-1")
            .with_position(0.5, 0.6, 0.0)
            .with_point(PointGraphics {
                pixel_size: Property::Constant(8.0),
                color: Property::Constant(cesium_datasource::property::Color::RED),
                ..Default::default()
            }),
    );
    c
}

// === DataSourceDisplay ===

#[test]
fn display_new_wgs84() {
    let display = DataSourceDisplay::wgs84();
    assert_eq!(display.billboard_count(), 0);
    assert_eq!(display.label_count(), 0);
    assert_eq!(display.point_count(), 0);
    assert_eq!(display.geometry_instance_count(), 0);
}

#[test]
fn display_new_custom_ellipsoid() {
    let display = DataSourceDisplay::new(Ellipsoid::WGS84);
    assert_eq!(display.billboard_count(), 0);
}

#[test]
fn display_update_populates_all_collections() {
    let mut display = DataSourceDisplay::wgs84();
    let entities = make_test_entities();
    display.update(&entities, 0.0);

    // box-1 has geometry
    assert!(display.geometry_instance_count() >= 1);
    // bb-1 → billboard
    assert_eq!(display.billboard_count(), 1);
    // label-1 → label
    assert_eq!(display.label_count(), 1);
    // point-1 → point
    assert_eq!(display.point_count(), 1);
}

#[test]
fn display_update_empty_collection() {
    let mut display = DataSourceDisplay::wgs84();
    let entities = EntityCollection::new();
    display.update(&entities, 0.0);

    assert_eq!(display.geometry_instance_count(), 0);
    assert_eq!(display.billboard_count(), 0);
    assert_eq!(display.label_count(), 0);
    assert_eq!(display.point_count(), 0);
}

#[test]
fn display_hidden_entity_excluded() {
    let mut display = DataSourceDisplay::wgs84();
    let mut entities = EntityCollection::new();
    let mut e = Entity::new("hidden")
        .with_position(0.0, 0.0, 0.0)
        .with_billboard(BillboardGraphics {
            image: Property::Constant("x.png".to_string()),
            ..Default::default()
        });
    e.show = false;
    entities.add(e);

    display.update(&entities, 0.0);
    assert_eq!(display.billboard_count(), 0);
}

#[test]
fn display_entity_without_position_uses_default() {
    let mut display = DataSourceDisplay::wgs84();
    let mut entities = EntityCollection::new();
    // Entity with billboard but no position
    entities.add(Entity::new("no-pos").with_billboard(BillboardGraphics {
        image: Property::Constant("y.png".to_string()),
        ..Default::default()
    }));

    display.update(&entities, 0.0);
    // Should still create billboard at default position [0,0,0]
    assert_eq!(display.billboard_count(), 1);
}

#[test]
fn display_time_change_resyncs_primitives() {
    let mut display = DataSourceDisplay::wgs84();
    let entities = make_test_entities();

    display.update(&entities, 0.0);
    let bb_count_1 = display.billboard_count();

    // Same time → no resync
    display.update(&entities, 0.0);
    assert_eq!(display.billboard_count(), bb_count_1);

    // Different time → resync
    display.update(&entities, 1.0);
    assert_eq!(display.billboard_count(), bb_count_1);
}

#[test]
fn display_mark_dirty_forces_rebuild() {
    let mut display = DataSourceDisplay::wgs84();
    let entities = make_test_entities();

    display.update(&entities, 0.0);
    let count = display.geometry_instance_count();

    display.mark_dirty();
    display.update(&entities, 0.0);
    assert_eq!(display.geometry_instance_count(), count);
}

#[test]
fn display_get_entity_geometry() {
    let mut display = DataSourceDisplay::wgs84();
    let entities = make_test_entities();
    display.update(&entities, 0.0);

    let geo = display.get_entity_geometry("box-1");
    assert!(geo.is_some());
    assert!(!geo.unwrap().fill_instances.is_empty());

    // Non-existent entity
    assert!(display.get_entity_geometry("nonexistent").is_none());
}

// === GeometryVisualizer ===

#[test]
fn visualizer_initial_state() {
    let viz = GeometryVisualizer::wgs84();
    assert_eq!(viz.entity_count(), 0);
    assert_eq!(viz.instance_count(), 0);
}

#[test]
fn visualizer_update_returns_count() {
    let mut viz = GeometryVisualizer::wgs84();
    let entities = make_test_entities();

    let updated = viz.update(&entities, 0.0);
    // Only box-1 produces geometry (billboard/label/point don't produce geometry instances)
    assert!(updated >= 1);
    assert!(viz.entity_count() >= 1);
}

#[test]
fn visualizer_no_update_same_time() {
    let mut viz = GeometryVisualizer::wgs84();
    let entities = make_test_entities();

    viz.update(&entities, 0.0);
    let updated = viz.update(&entities, 0.0);
    assert_eq!(updated, 0);
}

#[test]
fn visualizer_time_change_updates_all() {
    let mut viz = GeometryVisualizer::wgs84();
    let entities = make_test_entities();

    viz.update(&entities, 0.0);
    let updated = viz.update(&entities, 5.0);
    assert!(updated > 0);
}

#[test]
fn visualizer_entity_removal_cleanup() {
    let mut viz = GeometryVisualizer::wgs84();
    let mut entities = make_test_entities();

    viz.update(&entities, 0.0);
    let count_before = viz.entity_count();

    entities.remove("box-1");
    viz.mark_dirty();
    viz.update(&entities, 0.0);
    assert!(viz.entity_count() < count_before);
}

#[test]
fn visualizer_get_geometry_specific() {
    let mut viz = GeometryVisualizer::wgs84();
    let entities = make_test_entities();
    viz.update(&entities, 0.0);

    assert!(viz.get_geometry("box-1").is_some());
    assert!(viz.get_geometry("nonexistent").is_none());
}

#[test]
fn visualizer_all_fill_instances() {
    let mut viz = GeometryVisualizer::wgs84();
    let entities = make_test_entities();
    viz.update(&entities, 0.0);

    let fills = viz.all_fill_instances();
    assert!(!fills.is_empty());
}

#[test]
fn visualizer_all_outline_instances() {
    let mut viz = GeometryVisualizer::wgs84();
    let entities = make_test_entities();
    viz.update(&entities, 0.0);

    // outlines may or may not exist depending on entity config
    let _outlines = viz.all_outline_instances();
}

#[test]
fn visualizer_clear_resets() {
    let mut viz = GeometryVisualizer::wgs84();
    let entities = make_test_entities();
    viz.update(&entities, 0.0);
    assert!(viz.entity_count() > 0);

    viz.clear();
    assert_eq!(viz.entity_count(), 0);
    assert_eq!(viz.instance_count(), 0);
}

#[test]
fn visualizer_remove_entity() {
    let mut viz = GeometryVisualizer::wgs84();
    let entities = make_test_entities();
    viz.update(&entities, 0.0);

    viz.remove_entity("box-1");
    assert!(viz.get_geometry("box-1").is_none());
}

// === StaticGeometryBatch ===

#[test]
fn batch_new_is_empty() {
    let batch = StaticGeometryBatch::new();
    assert!(batch.is_empty());
    assert_eq!(batch.len(), 0);
}

#[test]
fn batch_add_geometry() {
    let mut batch = StaticGeometryBatch::new();
    let mut viz = GeometryVisualizer::wgs84();
    let entities = make_test_entities();
    viz.update(&entities, 0.0);

    if let Some(geo) = viz.get_geometry("box-1") {
        batch.add(geo);
    }
    assert!(!batch.is_empty());
    assert!(batch.len() > 0);
}

#[test]
fn batch_clear() {
    let mut batch = StaticGeometryBatch::new();
    let mut viz = GeometryVisualizer::wgs84();
    let entities = make_test_entities();
    viz.update(&entities, 0.0);

    if let Some(geo) = viz.get_geometry("box-1") {
        batch.add(geo);
    }
    batch.clear();
    assert!(batch.is_empty());
    assert_eq!(batch.len(), 0);
}

// === DynamicGeometryUpdater ===

#[test]
fn dynamic_updater_add_remove() {
    let mut dynamic = DynamicGeometryUpdater::new(Ellipsoid::WGS84);
    assert_eq!(dynamic.entity_count(), 0);

    dynamic.add_entity("e1");
    assert_eq!(dynamic.entity_count(), 1);

    // Duplicate add should not increase count
    dynamic.add_entity("e1");
    assert_eq!(dynamic.entity_count(), 1);

    dynamic.add_entity("e2");
    assert_eq!(dynamic.entity_count(), 2);

    dynamic.remove_entity("e1");
    assert_eq!(dynamic.entity_count(), 1);
}

#[test]
fn dynamic_updater_update_generates_geometry() {
    let mut dynamic = DynamicGeometryUpdater::new(Ellipsoid::WGS84);
    let entities = make_test_entities();

    dynamic.add_entity("box-1");
    let results = dynamic.update(&entities, 0.0);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "box-1");
    assert!(!results[0].1.fill_instances.is_empty());
}

#[test]
fn dynamic_updater_nonexistent_entity_skipped() {
    let mut dynamic = DynamicGeometryUpdater::new(Ellipsoid::WGS84);
    let entities = make_test_entities();

    dynamic.add_entity("nonexistent");
    let results = dynamic.update(&entities, 0.0);
    assert_eq!(results.len(), 0);
}

// === MultiDataSourceDisplay ===

#[test]
fn multi_display_add_sources() {
    let mut multi = MultiDataSourceDisplay::wgs84();
    assert_eq!(multi.source_count(), 0);

    multi.add_data_source(DataSource::new("src-1"));
    assert_eq!(multi.source_count(), 1);

    multi.add_data_source(DataSource::new("src-2"));
    assert_eq!(multi.source_count(), 2);
}

#[test]
fn multi_display_remove_source() {
    let mut multi = MultiDataSourceDisplay::wgs84();
    multi.add_data_source(DataSource::new("temp"));

    let removed = multi.remove_data_source("temp");
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().name, "temp");
    assert_eq!(multi.source_count(), 0);

    // Remove non-existent
    assert!(multi.remove_data_source("nope").is_none());
}

#[test]
fn multi_display_merged_update() {
    let mut multi = MultiDataSourceDisplay::wgs84();

    let mut src1 = DataSource::new("s1");
    src1.entities.add(
        Entity::new("s1-box")
            .with_position(0.0, 0.0, 0.0)
            .with_box(BoxGraphics {
                dimensions: Property::Constant([10.0, 10.0, 10.0]),
                ..Default::default()
            }),
    );

    let mut src2 = DataSource::new("s2");
    src2.entities.add(
        Entity::new("s2-point")
            .with_position(0.1, 0.1, 0.0)
            .with_point(PointGraphics {
                pixel_size: Property::Constant(5.0),
                ..Default::default()
            }),
    );

    multi.add_data_source(src1);
    multi.add_data_source(src2);
    multi.update(0.0);

    assert!(multi.display().geometry_instance_count() >= 1);
    assert_eq!(multi.display().point_count(), 1);
}
