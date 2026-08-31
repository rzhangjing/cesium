//! Ported specs from `packages/engine/Specs/DataSources/`.
//!
//! This file covers pure-logic specs that don't require a scene/wgpu context.
//! Scene-dependent specs (Visualizer, GeometryUpdater, DataSourceDisplay with scene)
//! are listed as `#[ignore]` placeholders.
//!
//! Spec coverage matrix: 92 original CesiumJS specs → Rust tests.

use cesium_data_sources::callback_property::CallbackProperty;
use cesium_data_sources::constant_property::ConstantProperty;
use cesium_data_sources::composite_property::CompositeProperty;
use cesium_data_sources::property::Property;
use cesium_data_sources::property::PropertyResult;
use cesium_data_sources::property_array::PropertyArray;
use cesium_data_sources::property_bag::PropertyBag;
use cesium_data_sources::entity::Entity;
use cesium_data_sources::entity_collection::EntityCollection;
use cesium_data_sources::data_source_clock::DataSourceClock;

// ============================================================================
// ConstantPropertySpec (ported from 106 lines)
// ============================================================================

#[test]
fn constant_property_get_value() {
    let prop = ConstantProperty::new(PropertyResult::Number(42.0));
    match prop.get_value(0.0) {
        PropertyResult::Number(v) => assert_eq!(v, 42.0),
        _ => panic!("Expected Number"),
    }
}

#[test]
fn constant_property_is_constant() {
    let prop = ConstantProperty::new(PropertyResult::Boolean(true));
    assert!(prop.is_constant());
}

#[test]
fn constant_property_equals() {
    let a = ConstantProperty::new(PropertyResult::Number(1.0));
    let b = ConstantProperty::new(PropertyResult::Number(1.0));
    let c = ConstantProperty::new(PropertyResult::Number(2.0));
    assert!(a.equals(&b));
    assert!(!a.equals(&c));
}

#[test]
fn constant_property_string_value() {
    let prop = ConstantProperty::new(PropertyResult::String("hello".to_string()));
    match prop.get_value(0.0) {
        PropertyResult::String(ref s) => assert_eq!(s, "hello"),
        _ => panic!("Expected String"),
    }
}

#[test]
fn constant_property_color_value() {
    let prop = ConstantProperty::new(PropertyResult::Color(1.0, 0.0, 0.0, 1.0));
    match prop.get_value(0.0) {
        PropertyResult::Color(r, g, b, a) => {
            assert_eq!(r, 1.0);
            assert_eq!(g, 0.0);
            assert_eq!(b, 0.0);
            assert_eq!(a, 1.0);
        }
        _ => panic!("Expected Color"),
    }
}

// ============================================================================
// CallbackPropertySpec (ported from 81 lines)
// ============================================================================

#[test]
fn callback_property_get_value() {
    let prop = CallbackProperty::new(
        Box::new(|_time: f64| PropertyResult::Number(99.0)),
        false,
    );
    match prop.get_value(0.0) {
        PropertyResult::Number(v) => assert_eq!(v, 99.0),
        _ => panic!("Expected Number"),
    }
}

#[test]
fn callback_property_is_constant() {
    // Second param `true` means the callback IS constant
    let constant_cb = CallbackProperty::new(
        Box::new(|_| PropertyResult::Number(1.0)),
        true,
    );
    assert!(constant_cb.is_constant());

    // Second param `false` means the callback is NOT constant (dynamic)
    let dynamic_cb = CallbackProperty::new(
        Box::new(|_| PropertyResult::Number(1.0)),
        false,
    );
    assert!(!dynamic_cb.is_constant());
}

// ============================================================================
// CompositePropertySpec (ported from 169 lines)
// ============================================================================

#[test]
fn composite_property_default() {
    let prop = CompositeProperty::new();
    assert!(!prop.is_destroyed());
}

// ============================================================================
// PropertyBagSpec (ported from 282 lines)
// ============================================================================

#[test]
fn property_bag_set_and_get() {
    let mut bag = PropertyBag::new();
    bag.set("key", PropertyResult::Number(42.0));
    assert!(bag.has("key"));
    let val = bag.get("key");
    assert!(val.is_some());
    match val.unwrap() {
        PropertyResult::Number(v) => assert_eq!(*v, 42.0),
        _ => panic!("Expected Number"),
    }
}

#[test]
fn property_bag_remove() {
    let mut bag = PropertyBag::new();
    bag.set("x", PropertyResult::Number(1.0));
    assert!(bag.has("x"));
    bag.remove("x");
    assert!(!bag.has("x"));
}

#[test]
fn property_bag_keys() {
    let mut bag = PropertyBag::new();
    bag.set("a", PropertyResult::Number(1.0));
    bag.set("b", PropertyResult::Number(2.0));
    let keys: Vec<&String> = bag.keys().collect();
    assert_eq!(keys.len(), 2);
    assert!(keys.iter().any(|k| *k == "a"));
    assert!(keys.iter().any(|k| *k == "b"));
}

#[test]
fn property_bag_clone() {
    let mut bag = PropertyBag::new();
    bag.set("k", PropertyResult::Number(7.0));
    let cloned = bag.clone();
    assert!(cloned.has("k"));
}

#[test]
fn property_bag_length() {
    let mut bag = PropertyBag::new();
    assert_eq!(bag.length(), 0);
    bag.set("a", PropertyResult::Number(1.0));
    bag.set("b", PropertyResult::Number(2.0));
    assert_eq!(bag.length(), 2);
}

#[test]
fn property_bag_clear() {
    let mut bag = PropertyBag::new();
    bag.set("a", PropertyResult::Number(1.0));
    bag.clear();
    assert_eq!(bag.length(), 0);
}

// ============================================================================
// PropertyArraySpec (ported from 96 lines)
// ============================================================================

#[test]
fn property_array_default() {
    let arr = PropertyArray::new();
    assert!(!arr.is_destroyed());
}

// ============================================================================
// EntitySpec (ported from 574 lines)
// ============================================================================

#[test]
fn entity_creation() {
    let entity = Entity::new("test-id");
    assert_eq!(entity.id, "test-id");
    assert!(entity.show);
    assert!(entity.name.is_none());
    assert!(entity.description.is_none());
    assert!(entity.position.is_none());
    assert!(entity.billboard.is_none());
    assert!(entity.label.is_none());
}

#[test]
fn entity_with_name() {
    let mut entity = Entity::new("e1");
    entity.name = Some("My Entity".to_string());
    assert_eq!(entity.name.as_deref(), Some("My Entity"));
}

#[test]
fn entity_availability() {
    use cesium_core::julian_date::JulianDate;
    use cesium_core::time_interval::TimeInterval;
    let mut entity = Entity::new("e1");
    let interval = TimeInterval::new(
        JulianDate::from_iso8601("2000-01-01T00:00:00Z"),
        JulianDate::from_iso8601("2000-01-02T00:00:00Z"),
        None,
        None,
    );
    entity.availability.push(interval);
    assert_eq!(entity.availability.len(), 1);
    let inside = JulianDate::from_iso8601("2000-01-01T12:00:00Z").unwrap();
    let outside = JulianDate::from_iso8601("2000-01-03T00:00:00Z").unwrap();
    assert!(entity.is_available(&inside));
    assert!(!entity.is_available(&outside));
}

#[test]
fn entity_properties() {
    let mut entity = Entity::new("e1");
    entity.properties.set("population", PropertyResult::Number(1000000.0));
    assert!(entity.properties.has("population"));
}

// ============================================================================
// EntityCollectionSpec (ported from 520 lines)
// ============================================================================

#[test]
fn entity_collection_add_remove() {
    let mut col = EntityCollection::new();
    col.add(Entity::new("a"));
    col.add(Entity::new("b"));
    assert_eq!(col.length(), 2);

    col.remove("a");
    assert_eq!(col.length(), 1);
    assert!(!col.contains_entity("a"));
    assert!(col.contains_entity("b"));
}

#[test]
fn entity_collection_get_by_id() {
    let mut col = EntityCollection::new();
    let mut e = Entity::new("e1");
    e.name = Some("Test".to_string());
    col.add(e);

    let found = col.get_by_id("e1");
    assert!(found.is_some());
    assert_eq!(found.unwrap().name.as_deref(), Some("Test"));

    assert!(col.get_by_id("nonexistent").is_none());
}

#[test]
fn entity_collection_values_order() {
    let mut col = EntityCollection::new();
    col.add(Entity::new("c"));
    col.add(Entity::new("a"));
    col.add(Entity::new("b"));

    let values = col.values();
    assert_eq!(values.len(), 3);
    // Values should be in insertion order
    assert_eq!(values[0].id, "c");
    assert_eq!(values[1].id, "a");
    assert_eq!(values[2].id, "b");
}

#[test]
fn entity_collection_remove_all() {
    let mut col = EntityCollection::new();
    col.add(Entity::new("x"));
    col.add(Entity::new("y"));
    col.add(Entity::new("z"));
    col.remove_all();
    assert_eq!(col.length(), 0);
    assert!(col.is_empty());
}

#[test]
fn entity_collection_suspend_resume_events() {
    let mut col = EntityCollection::new();
    col.suspend_events();
    col.add(Entity::new("a"));
    col.add(Entity::new("b"));
    // Events are suspended, no notifications fired yet
    col.resume_events();
    // After resume, pending changes would be flushed
    assert_eq!(col.length(), 2);
}

#[test]
fn entity_collection_destroy() {
    let mut col = EntityCollection::new();
    col.add(Entity::new("a"));
    col.destroy();
    assert!(col.is_destroyed());
    assert_eq!(col.length(), 0);
}

// ============================================================================
// DataSourceClockSpec (ported from 117 lines)
// ============================================================================

#[test]
fn data_source_clock_default() {
    use cesium_core::clock_range::ClockRange;
    use cesium_core::clock_step::ClockStep;
    let clock = DataSourceClock::new();
    assert_eq!(clock.clock_range, ClockRange::Unbounded);
    assert_eq!(clock.clock_step, ClockStep::SystemClockMultiplier);
    assert_eq!(clock.multiplier, 1.0);
}

#[test]
fn data_source_clock_merge() {
    use cesium_core::julian_date::JulianDate;
    let mut clock = DataSourceClock::new();
    let mut other = DataSourceClock::new();
    other.start_time = JulianDate::from_iso8601("2012-03-15T10:00:00Z").unwrap();
    other.stop_time = JulianDate::from_iso8601("2012-03-16T10:00:00Z").unwrap();
    other.multiplier = 2.0;

    clock.merge(&other);
    assert!(JulianDate::equals(&clock.start_time, &other.start_time));
    assert!(JulianDate::equals(&clock.stop_time, &other.stop_time));
    assert_eq!(clock.multiplier, 2.0);
}

// ============================================================================
// Scene-dependent specs (ported as #[ignore] placeholders)
// These require wgpu headless rendering or full scene integration.
// ============================================================================

// ---------------------------------------------------------------------------
// Visualizer specs (8 specs) — UN-IGNORED: visualizers now track entity
// lifecycle internally (create/update/destroy/entity-tracking).
// ---------------------------------------------------------------------------

#[test]
fn billboard_visualizer_spec() {
    use cesium_data_sources::billboard_visualizer::BillboardVisualizer;
    use cesium_data_sources::billboard_graphics::BillboardGraphics;
    use cesium_data_sources::visualizer::Visualizer;

    // Construction
    let mut viz = BillboardVisualizer::new();
    assert!(!viz.is_destroyed());
    assert_eq!(viz.entity_count(), 0);
    assert_eq!(viz.update_count(), 0);

    // Update with no entities
    assert!(viz.update(0.0));
    assert_eq!(viz.update_count(), 1);

    // Entity tracking
    let mut entity = Entity::new("bb-1");
    entity.billboard = Some(BillboardGraphics::new());
    viz.on_entity_added_or_updated(&entity);
    assert_eq!(viz.entity_count(), 1);

    // Update flushes pending changes
    assert!(viz.update(1.0));
    assert_eq!(viz.update_count(), 2);

    // Entity without billboard graphics is ignored
    let plain = Entity::new("plain-1");
    viz.on_entity_added_or_updated(&plain);
    assert_eq!(viz.entity_count(), 1);

    // Entity removal
    viz.on_entity_removed("bb-1");
    assert_eq!(viz.entity_count(), 0);

    // Destroy
    viz.destroy();
    assert!(viz.is_destroyed());
    assert!(!viz.update(2.0)); // update after destroy returns false
}

#[test]
fn geometry_visualizer_spec() {
    use cesium_data_sources::geometry_visualizer::GeometryVisualizer;
    use cesium_data_sources::visualizer::Visualizer;

    let mut viz = GeometryVisualizer::new();
    assert!(!viz.is_destroyed());
    assert_eq!(viz.entity_count(), 0);

    // Track entity with visuals
    let mut e = Entity::new("g-1");
    e.billboard = Some(cesium_data_sources::billboard_graphics::BillboardGraphics::new());
    viz.on_entity_added_or_updated(&e);
    assert_eq!(viz.entity_count(), 1);

    assert!(viz.update(0.0));
    assert_eq!(viz.update_count(), 1);

    viz.on_entity_removed("g-1");
    assert_eq!(viz.entity_count(), 0);

    viz.destroy();
    assert!(viz.is_destroyed());
}

#[test]
fn label_visualizer_spec() {
    use cesium_data_sources::label_visualizer::LabelVisualizer;
    use cesium_data_sources::label_graphics::LabelGraphics;
    use cesium_data_sources::visualizer::Visualizer;

    let mut viz = LabelVisualizer::new();
    assert!(!viz.is_destroyed());

    let mut entity = Entity::new("lbl-1");
    entity.label = Some(LabelGraphics::new());
    viz.on_entity_added_or_updated(&entity);
    assert_eq!(viz.entity_count(), 1);

    assert!(viz.update(0.0));

    // Duplicate notification does not double-count
    viz.on_entity_added_or_updated(&entity);
    assert_eq!(viz.entity_count(), 1);

    viz.destroy();
    assert!(viz.is_destroyed());
}

#[test]
fn model_visualizer_spec() {
    use cesium_data_sources::model_visualizer::ModelVisualizer;
    use cesium_data_sources::model_graphics::ModelGraphics;
    use cesium_data_sources::visualizer::Visualizer;

    let mut viz = ModelVisualizer::new();
    assert!(!viz.is_destroyed());

    let mut entity = Entity::new("m-1");
    entity.model = Some(ModelGraphics::new());
    viz.on_entity_added_or_updated(&entity);
    assert_eq!(viz.entity_count(), 1);
    assert!(viz.update(0.0));

    viz.on_entity_removed("m-1");
    assert_eq!(viz.entity_count(), 0);
    viz.destroy();
    assert!(viz.is_destroyed());
    assert!(!viz.update(1.0));
}

#[test]
fn cesium3_d_tileset_visualizer_spec() {
    use cesium_data_sources::cesium3_d_tileset_visualizer::Cesium3DTilesetVisualizer;
    use cesium_data_sources::visualizer::Visualizer;

    let mut viz = Cesium3DTilesetVisualizer::new();
    assert!(!viz.is_destroyed());
    assert_eq!(viz.update_count(), 0);
    assert!(viz.update(0.0));
    assert_eq!(viz.update_count(), 1);

    viz.destroy();
    assert!(viz.is_destroyed());
}

#[test]
fn point_visualizer_spec() {
    use cesium_data_sources::point_visualizer::PointVisualizer;
    use cesium_data_sources::point_graphics::PointGraphics;
    use cesium_data_sources::visualizer::Visualizer;

    let mut viz = PointVisualizer::new();
    assert!(!viz.is_destroyed());

    let mut entity = Entity::new("p-1");
    entity.point = Some(PointGraphics::new());
    viz.on_entity_added_or_updated(&entity);
    assert_eq!(viz.entity_count(), 1);

    assert!(viz.update(0.0));

    viz.on_entity_removed("p-1");
    assert_eq!(viz.entity_count(), 0);

    viz.destroy();
    assert!(viz.is_destroyed());
    assert!(!viz.update(1.0));
}

#[test]
fn path_visualizer_spec() {
    use cesium_data_sources::path_visualizer::PathVisualizer;
    use cesium_data_sources::visualizer::Visualizer;
    use cesium_core::cartesian3::Cartesian3;

    let mut viz = PathVisualizer::new();
    assert!(!viz.is_destroyed());

    // Path visualizer tracks entities with positions
    let mut entity = Entity::new("path-1");
    entity.position = Some(Cartesian3::new(1.0, 2.0, 3.0));
    viz.on_entity_added_or_updated(&entity);
    assert_eq!(viz.entity_count(), 1);
    assert!(viz.update(0.0));

    viz.destroy();
    assert!(viz.is_destroyed());
}

#[test]
fn polyline_visualizer_spec() {
    use cesium_data_sources::polyline_visualizer::PolylineVisualizer;
    use cesium_data_sources::polyline_graphics::PolylineGraphics;
    use cesium_data_sources::visualizer::Visualizer;

    let mut viz = PolylineVisualizer::new();
    assert!(!viz.is_destroyed());

    let mut entity = Entity::new("pl-1");
    entity.polyline = Some(PolylineGraphics::new());
    viz.on_entity_added_or_updated(&entity);
    assert_eq!(viz.entity_count(), 1);
    assert!(viz.update(0.0));

    viz.on_entity_removed("pl-1");
    assert_eq!(viz.entity_count(), 0);

    viz.destroy();
    assert!(viz.is_destroyed());
    assert!(!viz.update(1.0));
}

// GeometryUpdater specs (14 specs)
#[test]
#[ignore = "Requires scene + geometry instance creation"]
fn box_geometry_updater_spec() {}

#[test]
#[ignore = "Requires scene + corridor geometry"]
fn corridor_geometry_updater_spec() {}

#[test]
#[ignore = "Requires scene + cylinder geometry"]
fn cylinder_geometry_updater_spec() {}

#[test]
#[ignore = "Requires scene + ellipse geometry"]
fn ellipse_geometry_updater_spec() {}

#[test]
#[ignore = "Requires scene + ellipsoid geometry"]
fn ellipsoid_geometry_updater_spec() {}

#[test]
#[ignore = "Requires scene + polygon geometry"]
fn polygon_geometry_updater_spec() {}

#[test]
#[ignore = "Requires scene + polyline geometry"]
fn polyline_geometry_updater_spec() {}

#[test]
#[ignore = "Requires scene + rectangle geometry"]
fn rectangle_geometry_updater_spec() {}

#[test]
#[ignore = "Requires scene + plane geometry"]
fn plane_geometry_updater_spec() {}

#[test]
#[ignore = "Requires scene + wall geometry"]
fn wall_geometry_updater_spec() {}

#[test]
#[ignore = "Requires scene + polyline volume geometry"]
fn polyline_volume_geometry_updater_spec() {}

#[test]
#[ignore = "Requires scene + ground geometry + terrain heights"]
fn ground_geometry_updater_spec() {}

#[test]
#[ignore = "Requires scene + GeometryUpdaterSet"]
fn geometry_updater_set_spec() {}

#[test]
#[ignore = "Requires scene + GeometryUpdater interface"]
fn geometry_updater_spec() {}

// Data source loading specs (require JSON/XML parsing + HTTP)
#[test]
#[ignore = "Requires serde_json CZML parsing + packet processing"]
fn czml_data_source_spec() {}

// GeoJSON specs fully ported to `tests/geo_json_data_source_spec.rs`
// (one `#[test]` per original Jasmine `it()`).

#[test]
#[ignore = "Requires XML parsing KML + network link handling"]
fn kml_data_source_spec() {}

// GPX specs fully ported to `tests/gpx_data_source_spec.rs`
// (one `#[test]` per original Jasmine `it()`). The exportKml specs live in
// `tests/export_kml_spec.rs`.

// Display/Cluster specs (require scene)
#[test]
#[ignore = "Requires scene + PrimitiveCollection + visualizer wiring"]
fn data_source_display_spec() {}

#[test]
#[ignore = "Requires scene + BillboardCollection + LabelCollection + clustering algorithm"]
fn entity_cluster_spec() {}

#[test]
#[ignore = "Requires scene + EntityView + camera tracking"]
fn entity_view_spec() {}

// Graphics specs (property validation, no scene needed but large)
// UN-IGNORED: these are value-model tests (construction, defaults, clone).

#[test]
fn billboard_graphics_spec() {
    use cesium_data_sources::billboard_graphics::BillboardGraphics;
    use cesium_core::color::Color;

    let bb = BillboardGraphics::new();
    assert!(bb.show);
    assert_eq!(bb.scale, 1.0);
    assert_eq!(bb.rotation, 0.0);
    assert!(bb.image.is_none());
    assert!(bb.color.is_none());
    assert_eq!(bb.horizontal_origin, 0);
    assert_eq!(bb.vertical_origin, 0);
    assert!(bb.pixel_offset.is_none());
    assert!(bb.eye_offset.is_none());
    assert_eq!(bb.height_reference, 0);

    // Clone preserves values
    let mut bb2 = bb.clone();
    bb2.scale = 2.5;
    bb2.image = Some("icon.png".to_string());
    bb2.color = Some(Color::new(1.0, 0.0, 0.0, 1.0));
    assert_eq!(bb2.scale, 2.5);
    assert_eq!(bb2.image.as_deref(), Some("icon.png"));
}

#[test]
fn box_graphics_spec() {
    use cesium_data_sources::box_graphics::BoxGraphics;
    let g = BoxGraphics::new();
    assert!(g.show);
    let mut g2 = g.clone();
    g2.show = false;
    assert!(!g2.show);
}

#[test]
fn corridor_graphics_spec() {
    use cesium_data_sources::corridor_graphics::CorridorGraphics;
    let g = CorridorGraphics::new();
    assert!(g.show);
    let g2 = g.clone();
    assert!(g2.show);
}

#[test]
fn cylinder_graphics_spec() {
    use cesium_data_sources::cylinder_graphics::CylinderGraphics;
    let g = CylinderGraphics::new();
    assert!(g.show);
    let g2 = g.clone();
    assert!(g2.show);
}

#[test]
fn ellipse_graphics_spec() {
    use cesium_data_sources::ellipse_graphics::EllipseGraphics;
    let g = EllipseGraphics::new();
    assert!(g.show);
    let g2 = g.clone();
    assert!(g2.show);
}

#[test]
fn ellipsoid_graphics_spec() {
    use cesium_data_sources::ellipsoid_graphics::EllipsoidGraphics;
    let g = EllipsoidGraphics::new();
    assert!(g.show);
    let g2 = g.clone();
    assert!(g2.show);
}

#[test]
fn label_graphics_spec() {
    use cesium_data_sources::label_graphics::LabelGraphics;
    use cesium_core::color::Color;

    let lbl = LabelGraphics::new();
    assert!(lbl.show);
    assert_eq!(lbl.scale, 1.0);
    assert_eq!(lbl.outline_width, 1.0);
    assert!(lbl.text.is_none());
    assert!(lbl.font.is_none());
    assert_eq!(lbl.style, 0);

    let mut lbl2 = lbl.clone();
    lbl2.text = Some("Hello".to_string());
    lbl2.fill_color = Color::new(0.0, 1.0, 0.0, 1.0);
    assert_eq!(lbl2.text.as_deref(), Some("Hello"));
}

#[test]
fn model_graphics_spec() {
    use cesium_data_sources::model_graphics::ModelGraphics;

    let m = ModelGraphics::new();
    assert!(m.show);
    assert_eq!(m.scale, 1.0);
    assert!(m.uri.is_none());
    assert_eq!(m.minimum_pixel_size, 0.0);
    assert_eq!(m.maximum_scale, f64::MAX);
    assert!(m.show_outline);
    assert_eq!(m.shadows, 0);

    let mut m2 = m.clone();
    m2.uri = Some("model.glb".to_string());
    m2.scale = 3.0;
    assert_eq!(m2.uri.as_deref(), Some("model.glb"));
    assert_eq!(m2.scale, 3.0);
}

#[test]
fn path_graphics_spec() {
    use cesium_data_sources::path_graphics::PathGraphics;
    let g = PathGraphics::new();
    assert!(g.show);
    let g2 = g.clone();
    assert!(g2.show);
}

#[test]
fn plane_graphics_spec() {
    use cesium_data_sources::plane_graphics::PlaneGraphics;
    let g = PlaneGraphics::new();
    assert!(g.show);
    let g2 = g.clone();
    assert!(g2.show);
}

#[test]
fn point_graphics_spec() {
    use cesium_data_sources::point_graphics::PointGraphics;
    use cesium_core::color::Color;

    let pt = PointGraphics::new();
    assert!(pt.show);
    assert_eq!(pt.pixel_size, 5.0);
    assert_eq!(pt.outline_width, 0.0);
    assert_eq!(pt.height_reference, 0);

    let mut pt2 = pt.clone();
    pt2.pixel_size = 10.0;
    pt2.color = Color::new(1.0, 0.0, 0.0, 1.0);
    assert_eq!(pt2.pixel_size, 10.0);
}

#[test]
fn polygon_graphics_spec() {
    use cesium_data_sources::polygon_graphics::PolygonGraphics;
    use cesium_core::cartesian3::Cartesian3;

    let pg = PolygonGraphics::new();
    assert!(pg.show);
    assert!(pg.fill);
    assert!(!pg.outline);
    assert!(pg.hierarchy.is_empty());
    assert!(pg.holes.is_empty());
    assert_eq!(pg.outline_width, 1.0);

    let mut pg2 = pg.clone();
    pg2.hierarchy.push(Cartesian3::new(1.0, 2.0, 3.0));
    pg2.height = Some(100.0);
    assert_eq!(pg2.hierarchy.len(), 1);
    assert_eq!(pg2.height, Some(100.0));
}

#[test]
fn polyline_graphics_spec() {
    use cesium_data_sources::polyline_graphics::PolylineGraphics;
    use cesium_core::cartesian3::Cartesian3;

    let pl = PolylineGraphics::new();
    assert!(pl.show);
    assert_eq!(pl.width, 1.0);
    assert!(!pl.clamp_to_ground);
    assert!(!pl.loop_);
    assert!(pl.positions.is_empty());

    let mut pl2 = pl.clone();
    pl2.width = 5.0;
    pl2.positions.push(Cartesian3::new(0.0, 0.0, 0.0));
    pl2.positions.push(Cartesian3::new(1.0, 1.0, 1.0));
    assert_eq!(pl2.width, 5.0);
    assert_eq!(pl2.positions.len(), 2);
}

#[test]
fn polyline_volume_graphics_spec() {
    use cesium_data_sources::polyline_volume_graphics::PolylineVolumeGraphics;
    let g = PolylineVolumeGraphics::new();
    assert!(g.show);
    let g2 = g.clone();
    assert!(g2.show);
}

#[test]
fn wall_graphics_spec() {
    use cesium_data_sources::wall_graphics::WallGraphics;
    let g = WallGraphics::new();
    assert!(g.show);
    let g2 = g.clone();
    assert!(g2.show);
}

#[test]
fn cesium3_d_tileset_graphics_spec() {
    use cesium_data_sources::cesium3_d_tileset_graphics::Cesium3DTilesetGraphics;

    let ts = Cesium3DTilesetGraphics::new();
    assert!(ts.show);
    assert!(ts.uri.is_none());
    assert_eq!(ts.maximum_screen_space_error, 16.0);

    let mut ts2 = ts.clone();
    ts2.uri = Some("tileset.json".to_string());
    ts2.maximum_screen_space_error = 8.0;
    assert_eq!(ts2.uri.as_deref(), Some("tileset.json"));
    assert_eq!(ts2.maximum_screen_space_error, 8.0);
}

// Material property specs — UN-IGNORED: value-model tests (type_name, is_constant, defaults).

#[test]
fn checkerboard_material_property_spec() {
    use cesium_data_sources::checkerboard_material_property::CheckerboardMaterialProperty;
    use cesium_data_sources::material_property::MaterialProperty;

    let m = CheckerboardMaterialProperty::new();
    assert_eq!(m.type_name(), "Checkerboard");
    assert!(MaterialProperty::is_constant(&m));
    assert!(!MaterialProperty::is_destroyed(&m));
    assert_eq!(m.repeat_x, 5.0);
    assert_eq!(m.repeat_y, 5.0);
}

#[test]
fn color_material_property_spec() {
    use cesium_data_sources::color_material_property::ColorMaterialProperty;
    use cesium_data_sources::material_property::MaterialProperty;
    use cesium_core::color::Color;

    let m = ColorMaterialProperty::new(Color::new(1.0, 0.0, 0.0, 1.0));
    assert_eq!(m.type_name(), "Color");
    assert!(MaterialProperty::is_constant(&m));
    assert!(!MaterialProperty::is_destroyed(&m));
    assert_eq!(m.color.red, 1.0);
    assert_eq!(m.color.green, 0.0);

    // Default is white
    let d = ColorMaterialProperty::default();
    assert_eq!(d.color.red, 1.0);
    assert_eq!(d.color.green, 1.0);
    assert_eq!(d.color.blue, 1.0);
    assert_eq!(d.color.alpha, 1.0);
}

#[test]
fn composite_material_property_spec() {
    use cesium_data_sources::composite_intervals::CompositeInterval;
    use cesium_data_sources::composite_material_property::CompositeMaterialProperty;
    use cesium_data_sources::color_material_property::ColorMaterialProperty;
    use cesium_data_sources::material_property::MaterialProperty;
    use cesium_data_sources::property::Property;
    use cesium_core::color::Color;
    use std::cell::Cell;
    use std::rc::Rc;

    // Mirrors CompositeMaterialPropertySpec.js
    let mut cmp = CompositeMaterialProperty::new();
    assert!(MaterialProperty::is_constant(&cmp));
    assert!(!MaterialProperty::is_destroyed(&cmp));
    assert_eq!(cmp.type_name(), "Composite");

    let raised = Rc::new(Cell::new(0u32));
    let raised_for_listener = Rc::clone(&raised);
    let _removal = cmp
        .definition_changed()
        .unwrap()
        .add_listener(move |_| raised_for_listener.set(raised_for_listener.get() + 1));

    let red = Rc::new(ColorMaterialProperty::new(Color::new(1.0, 0.0, 0.0, 1.0)))
        as Rc<dyn Property>;
    let green = Rc::new(ColorMaterialProperty::new(Color::new(0.0, 1.0, 0.0, 1.0)))
        as Rc<dyn Property>;

    cmp.intervals_mut().add_interval(
        CompositeInterval::new(0.0, 10.0, true, true, Rc::clone(&red)),
        None,
    );
    cmp.intervals_mut().add_interval(
        CompositeInterval::new(12.0, 14.0, true, true, Rc::clone(&green)),
        None,
    );
    assert_eq!(raised.get(), 2);
    assert!(!MaterialProperty::is_constant(&cmp));

    // getValue inside the first interval (material value model).
    match cmp.get_value_option(1.0) {
        Some(cesium_data_sources::property::PropertyResult::Color(r, g, b, _)) => {
            assert_eq!(r, 1.0);
            assert_eq!(g, 0.0);
            assert_eq!(b, 0.0);
        }
        other => panic!("Expected red color in first interval, got {:?}", other),
    }

    // getValue outside all intervals (JS undefined).
    assert!(cmp.get_value_option(11.0).is_none());

    // getType(time) via get_type_at.
    assert_eq!(cmp.get_type_at(1.0), Some("Color"));
    assert!(cmp.get_type_at(11.0).is_none());

    // equals
    let mut other = CompositeMaterialProperty::new();
    other.intervals_mut().add_interval(
        CompositeInterval::new(0.0, 10.0, true, true, Rc::clone(&red)),
        None,
    );
    other.intervals_mut().add_interval(
        CompositeInterval::new(12.0, 14.0, true, true, Rc::clone(&green)),
        None,
    );
    assert!(cmp.equals_composite_material(&other));

    let different = CompositeMaterialProperty::new();
    assert!(!cmp.equals_composite_material(&different));
}

#[test]
fn grid_material_property_spec() {
    use cesium_data_sources::grid_material_property::GridMaterialProperty;
    use cesium_data_sources::material_property::MaterialProperty;

    let m = GridMaterialProperty::new();
    assert_eq!(m.type_name(), "Grid");
    assert!(MaterialProperty::is_constant(&m));
    assert!(!MaterialProperty::is_destroyed(&m));
    assert_eq!(m.cell_alpha, 0.75);
    assert_eq!(m.repeat_x, 8.0);
    assert_eq!(m.repeat_y, 8.0);
}

#[test]
fn image_material_property_spec() {
    use cesium_data_sources::image_material_property::ImageMaterialProperty;
    use cesium_data_sources::material_property::MaterialProperty;

    let mut m = ImageMaterialProperty::new();
    assert_eq!(m.type_name(), "Image");
    assert!(MaterialProperty::is_constant(&m));
    assert!(m.image.is_none());
    assert_eq!(m.repeat_x, 1.0);
    assert_eq!(m.repeat_y, 1.0);

    m.image = Some("texture.png".to_string());
    assert_eq!(m.image.as_deref(), Some("texture.png"));
}

#[test]
fn polyline_arrow_material_property_spec() {
    use cesium_data_sources::polyline_arrow_material_property::PolylineArrowMaterialProperty;
    use cesium_data_sources::material_property::MaterialProperty;

    let m = PolylineArrowMaterialProperty::new();
    assert_eq!(m.type_name(), "PolylineArrow");
    assert!(MaterialProperty::is_constant(&m));
    assert!(!MaterialProperty::is_destroyed(&m));
}

#[test]
fn polyline_dash_material_property_spec() {
    use cesium_data_sources::polyline_dash_material_property::PolylineDashMaterialProperty;
    use cesium_data_sources::material_property::MaterialProperty;

    let m = PolylineDashMaterialProperty::new();
    assert_eq!(m.type_name(), "PolylineDash");
    assert!(MaterialProperty::is_constant(&m));
    assert!(!MaterialProperty::is_destroyed(&m));
}

#[test]
fn polyline_glow_material_property_spec() {
    use cesium_data_sources::polyline_glow_material_property::PolylineGlowMaterialProperty;
    use cesium_data_sources::material_property::MaterialProperty;

    let m = PolylineGlowMaterialProperty::new();
    assert_eq!(m.type_name(), "PolylineGlow");
    assert!(MaterialProperty::is_constant(&m));
    assert!(!MaterialProperty::is_destroyed(&m));
}

#[test]
fn polyline_outline_material_property_spec() {
    use cesium_data_sources::polyline_outline_material_property::PolylineOutlineMaterialProperty;
    use cesium_data_sources::material_property::MaterialProperty;

    let m = PolylineOutlineMaterialProperty::new();
    assert_eq!(m.type_name(), "PolylineOutline");
    assert!(MaterialProperty::is_constant(&m));
    assert!(!MaterialProperty::is_destroyed(&m));
}

#[test]
fn stripe_material_property_spec() {
    use cesium_data_sources::stripe_material_property::StripeMaterialProperty;
    use cesium_data_sources::material_property::MaterialProperty;
    use cesium_data_sources::stripe_orientation::StripeOrientation;

    let m = StripeMaterialProperty::new();
    assert_eq!(m.type_name(), "Stripe");
    assert!(MaterialProperty::is_constant(&m));
    assert!(!MaterialProperty::is_destroyed(&m));
    assert_eq!(m.orientation, StripeOrientation::Horizontal);
    assert_eq!(m.repeat, 1.0);
}

// Position/Property specs (partially covered above, full specs need interpolation)
#[test]
#[ignore = "Covered by tests/sampled_property_spec.rs (SampledPropertySpec mirror)"]
fn sampled_property_spec() {}

#[test]
fn sampled_position_property_spec() {
    // Mirrors SampledPositionPropertySpec.js
    use cesium_core::cartesian3::Cartesian3;
    use cesium_core::extrapolation_type::ExtrapolationType;
    use cesium_data_sources::position_property::{PositionProperty, PositionReferenceFrame};
    use cesium_data_sources::sampled_property::InterpolationAlgorithmKind;
    use cesium_data_sources::sampled_position_property::SampledPositionProperty;
    use std::cell::Cell;
    use std::rc::Rc;

    // constructor defaults
    let mut property = SampledPositionProperty::new(None, None);
    assert!(!property.is_destroyed());
    assert_eq!(property.reference_frame(), PositionReferenceFrame::Fixed);
    assert_eq!(property.interpolation_degree(), 1);
    assert_eq!(
        property.interpolation_algorithm(),
        InterpolationAlgorithmKind::Linear
    );
    assert_eq!(property.forward_extrapolation_type(), ExtrapolationType::None);
    assert_eq!(property.forward_extrapolation_duration(), 0.0);
    assert_eq!(property.backward_extrapolation_type(), ExtrapolationType::None);
    assert_eq!(property.backward_extrapolation_duration(), 0.0);
    assert_eq!(property.number_of_derivatives(), 0);

    // definitionChanged is raised by addSample
    let raised = Rc::new(Cell::new(0u32));
    let raised_for_listener = Rc::clone(&raised);
    let _removal = property
        .definition_changed_event()
        .add_listener(move |_| raised_for_listener.set(raised_for_listener.get() + 1));

    property.add_sample(0.0, &Cartesian3::new(1.0, 2.0, 3.0), &[]);
    property.add_sample(1.0, &Cartesian3::new(3.0, 4.0, 5.0), &[]);
    assert!(raised.get() >= 2);

    // getValue at a sample and at a linearly interpolated mid-point.
    let mut result = Cartesian3::ZERO;
    let value = property
        .get_value_in_reference_frame(0.0, PositionReferenceFrame::Fixed, &mut result)
        .unwrap();
    assert_eq!(*value, Cartesian3::new(1.0, 2.0, 3.0));

    let value = property
        .get_value_in_reference_frame(0.5, PositionReferenceFrame::Fixed, &mut result)
        .unwrap();
    assert_eq!(*value, Cartesian3::new(2.0, 3.0, 4.0));

    // outside the sample span with extrapolation NONE (JS undefined).
    assert!(property
        .get_value_in_reference_frame(2.0, PositionReferenceFrame::Fixed, &mut result)
        .is_none());

    // addSamplesPackedArray with an epoch offset (JS [0,7,8,9, 1,8,9,10]).
    let mut property = SampledPositionProperty::new(None, None);
    property.add_samples_packed_array(&[0.0, 7.0, 8.0, 9.0, 1.0, 8.0, 9.0, 10.0], Some(1.0));
    let mut result = Cartesian3::ZERO;
    let value = property
        .get_value_in_reference_frame(1.0, PositionReferenceFrame::Fixed, &mut result)
        .unwrap();
    assert_eq!(*value, Cartesian3::new(7.0, 8.0, 9.0));
    let value = property
        .get_value_in_reference_frame(1.5, PositionReferenceFrame::Fixed, &mut result)
        .unwrap();
    assert_eq!(*value, Cartesian3::new(7.5, 8.5, 9.5));

    // removeSample restores interpolation over the removed sample.
    let mut property = SampledPositionProperty::new(None, None);
    property.add_sample(0.0, &Cartesian3::new(0.0, 0.0, 0.0), &[]);
    property.add_sample(1.0, &Cartesian3::new(0.0, 5.0, 0.0), &[]);
    property.add_sample(2.0, &Cartesian3::new(0.0, 10.0, 0.0), &[]);
    assert!(property.remove_sample(1.0));
    let mut result = Cartesian3::ZERO;
    let value = property
        .get_value_in_reference_frame(1.0, PositionReferenceFrame::Fixed, &mut result)
        .unwrap();
    assert_eq!(*value, Cartesian3::new(0.0, 5.0, 0.0));

    // interpolation options setter
    let mut property = SampledPositionProperty::new(None, None);
    property.set_interpolation_options(Some(InterpolationAlgorithmKind::Lagrange), Some(3));
    assert_eq!(
        property.interpolation_algorithm(),
        InterpolationAlgorithmKind::Lagrange
    );
    assert_eq!(property.interpolation_degree(), 3);

    // extrapolation setters only raise when the value changes.
    let raised = Rc::new(Cell::new(0u32));
    let raised_for_listener = Rc::clone(&raised);
    let _removal = property
        .definition_changed_event()
        .add_listener(move |_| raised_for_listener.set(raised_for_listener.get() + 1));
    property.set_forward_extrapolation_type(ExtrapolationType::Hold);
    assert_eq!(raised.get(), 1);
    property.set_forward_extrapolation_type(ExtrapolationType::Hold);
    assert_eq!(raised.get(), 1);

    // equals (algorithm/degree/frame/samples)
    let mut lhs = SampledPositionProperty::new(None, None);
    lhs.add_sample(0.0, &Cartesian3::new(1.0, 2.0, 3.0), &[]);
    let mut rhs = SampledPositionProperty::new(None, None);
    rhs.add_sample(0.0, &Cartesian3::new(1.0, 2.0, 3.0), &[]);
    let mut different = SampledPositionProperty::new(None, None);
    different.add_sample(0.0, &Cartesian3::new(4.0, 5.0, 6.0), &[]);
    use cesium_data_sources::property::Property;
    assert!(lhs.equals(&rhs));
    assert!(!lhs.equals(&different));
}

// UN-IGNORED: callback_position_property works without scene
#[test]
fn callback_position_property_spec() {
    use cesium_data_sources::callback_position_property::CallbackPositionProperty;
    use cesium_data_sources::position_property::{PositionProperty, PositionReferenceFrame};
    use cesium_data_sources::property::Property;
    use cesium_core::cartesian3::Cartesian3;

    let prop = CallbackPositionProperty::new(
        Box::new(|t: f64| Cartesian3::new(t, t * 2.0, t * 3.0)),
        PositionReferenceFrame::Fixed,
    );

    assert!(!prop.is_constant());
    assert!(!prop.is_destroyed());
    assert_eq!(prop.reference_frame(), PositionReferenceFrame::Fixed);

    // get_value at t=1.0
    let val = prop.get_value(1.0);
    match val {
        cesium_data_sources::property::PropertyResult::Position(x, y, z) => {
            assert_eq!(x, 1.0);
            assert_eq!(y, 2.0);
            assert_eq!(z, 3.0);
        }
        _ => panic!("Expected Position"),
    }

    // position_value
    let mut scratch = Cartesian3::new(0.0, 0.0, 0.0);
    let result = prop.position_value(2.0, &mut scratch);
    assert!(result.is_some());
    let pos = result.unwrap();
    assert_eq!(pos.x, 2.0);
    assert_eq!(pos.y, 4.0);
    assert_eq!(pos.z, 6.0);
}

// UN-IGNORED: composite_position_property works without scene
#[test]
fn composite_position_property_spec() {
    use cesium_data_sources::composite_intervals::CompositeInterval;
    use cesium_data_sources::composite_position_property::CompositePositionProperty;
    use cesium_data_sources::constant_position_property::ConstantPositionProperty;
    use cesium_data_sources::position_property::{PositionProperty, PositionReferenceFrame};
    use cesium_data_sources::property::Property;
    use cesium_core::cartesian3::Cartesian3;
    use std::cell::Cell;
    use std::rc::Rc;

    // Mirrors CompositePositionPropertySpec.js
    let mut composite = CompositePositionProperty::new(None);
    assert_eq!(composite.reference_frame(), PositionReferenceFrame::Fixed);
    assert!(composite.is_constant());
    assert!(!composite.is_destroyed());

    let raised = Rc::new(Cell::new(0u32));
    let raised_for_listener = Rc::clone(&raised);
    let _removal = composite
        .definition_changed()
        .unwrap()
        .add_listener(move |_| raised_for_listener.set(raised_for_listener.get() + 1));

    let inner1 = Rc::new(ConstantPositionProperty::new(Cartesian3::new(1.0, 2.0, 3.0)))
        as Rc<dyn Property>;
    let inner2 = Rc::new(ConstantPositionProperty::new(Cartesian3::new(4.0, 5.0, 6.0)))
        as Rc<dyn Property>;

    composite.intervals_mut().add_interval(
        CompositeInterval::new(0.0, 10.0, true, true, Rc::clone(&inner1)),
        None,
    );
    composite.intervals_mut().add_interval(
        CompositeInterval::new(12.0, 14.0, true, true, Rc::clone(&inner2)),
        None,
    );
    assert_eq!(raised.get(), 2);
    assert!(!composite.is_constant());

    // getValue inside the first interval (fixed frame).
    let mut scratch = Cartesian3::ZERO;
    let value = composite
        .get_value_in_reference_frame(1.0, PositionReferenceFrame::Fixed, &mut scratch)
        .unwrap();
    assert_eq!(*value, Cartesian3::new(1.0, 2.0, 3.0));

    // getValue outside all intervals (JS undefined).
    assert!(composite
        .get_value_in_reference_frame(11.0, PositionReferenceFrame::Fixed, &mut scratch)
        .is_none());

    // referenceFrame setter (plain assignment, does not raise).
    composite.set_reference_frame(PositionReferenceFrame::Inertial);
    assert_eq!(composite.reference_frame(), PositionReferenceFrame::Inertial);
    assert_eq!(raised.get(), 2);

    // equals: reference frame mismatch first (composite switched to
    // Inertial above), then matching intervals + matching reference frame.
    let mut other = CompositePositionProperty::new(None);
    other.intervals_mut().add_interval(
        CompositeInterval::new(0.0, 10.0, true, true, Rc::clone(&inner1)),
        None,
    );
    other.intervals_mut().add_interval(
        CompositeInterval::new(12.0, 14.0, true, true, Rc::clone(&inner2)),
        None,
    );
    assert!(!composite.equals_composite_position(&other));
    other.set_reference_frame(PositionReferenceFrame::Inertial);
    assert!(composite.equals_composite_position(&other));

    // Property::get_value delegates to the fixed frame.
    match composite.get_value(1.0) {
        cesium_data_sources::property::PropertyResult::Cartesian3(x, y, z) => {
            assert_eq!((x, y, z), (1.0, 2.0, 3.0));
        }
        other => panic!("Expected Cartesian3, got {:?}", other),
    }
}

// UN-IGNORED: constant_position_property works without scene
#[test]
fn constant_position_property_spec() {
    use cesium_data_sources::constant_position_property::ConstantPositionProperty;
    use cesium_data_sources::position_property::{PositionProperty, PositionReferenceFrame};
    use cesium_data_sources::property::Property;
    use cesium_core::cartesian3::Cartesian3;

    let prop = ConstantPositionProperty::new(Cartesian3::new(10.0, 20.0, 30.0));
    assert!(prop.is_constant());
    assert!(!prop.is_destroyed());
    assert_eq!(prop.reference_frame(), PositionReferenceFrame::Fixed);

    let val = prop.get_value(0.0);
    match val {
        cesium_data_sources::property::PropertyResult::Position(x, y, z) => {
            assert_eq!(x, 10.0);
            assert_eq!(y, 20.0);
            assert_eq!(z, 30.0);
        }
        _ => panic!("Expected Position"),
    }

    let mut scratch = Cartesian3::new(0.0, 0.0, 0.0);
    let result = prop.position_value(0.0, &mut scratch);
    assert!(result.is_some());
    assert_eq!(result.unwrap().x, 10.0);
}

#[test]
#[ignore = "Requires TimeIntervalCollection"]
fn time_interval_collection_property_spec() {}

#[test]
#[ignore = "Requires TimeIntervalCollection + position"]
fn time_interval_collection_position_property_spec() {}

#[test]
fn position_property_array_spec() {
    // Mirrors PositionPropertyArraySpec.js
    use cesium_core::cartesian3::Cartesian3;
    use cesium_data_sources::constant_position_property::ConstantPositionProperty;
    use cesium_data_sources::position_property::{PositionProperty, PositionReferenceFrame};
    use cesium_data_sources::position_property_array::PositionPropertyArray;
    use cesium_data_sources::property::Property;
    use std::cell::Cell;
    use std::rc::Rc;

    // defaults
    let property = PositionPropertyArray::new(None, None);
    assert!(property.is_constant());
    assert!(!property.is_destroyed());
    assert_eq!(property.reference_frame(), PositionReferenceFrame::Fixed);

    // getValue with an undefined value (JS undefined).
    assert!(property.get_value_in_reference_frame(0.0, PositionReferenceFrame::Fixed).is_none());

    // constructor with values; definitionChanged raised by setValue.
    let v1: Rc<dyn PositionProperty> =
        Rc::new(ConstantPositionProperty::new(Cartesian3::new(1.0, 2.0, 3.0)));
    let v2: Rc<dyn PositionProperty> =
        Rc::new(ConstantPositionProperty::new(Cartesian3::new(4.0, 5.0, 6.0)));
    let mut property = PositionPropertyArray::new(Some(vec![Rc::clone(&v1)]), None);

    let raised = Rc::new(Cell::new(0u32));
    let raised_for_listener = Rc::clone(&raised);
    let _removal = property
        .definition_changed()
        .unwrap()
        .add_listener(move |_| raised_for_listener.set(raised_for_listener.get() + 1));

    property.set_value(Some(vec![Rc::clone(&v1), Rc::clone(&v2)]));
    assert_eq!(raised.get(), 1);

    let values = property
        .get_value_in_reference_frame(0.0, PositionReferenceFrame::Fixed)
        .unwrap();
    assert_eq!(values.len(), 2);
    assert_eq!(values[0], Cartesian3::new(1.0, 2.0, 3.0));
    assert_eq!(values[1], Cartesian3::new(4.0, 5.0, 6.0));

    // a member's definition change raises the array's definitionChanged.
    let inner = ConstantPositionProperty::new(Cartesian3::new(7.0, 8.0, 9.0));
    let member: Rc<dyn PositionProperty> = Rc::new(inner);
    property.set_value(Some(vec![Rc::clone(&member)]));
    assert_eq!(raised.get(), 2);
    let values = property
        .get_value_in_reference_frame(0.0, PositionReferenceFrame::Fixed)
        .unwrap();
    assert_eq!(values.len(), 1);
    assert_eq!(values[0], Cartesian3::new(7.0, 8.0, 9.0));

    // isConstant is false when any member is non-constant (a sampled
    // position property with samples is time-varying; with no samples it
    // is constant, matching CesiumJS).
    use cesium_data_sources::sampled_position_property::SampledPositionProperty;
    let mut sampled_property = SampledPositionProperty::new(None, None);
    sampled_property.add_sample(0.0, &Cartesian3::new(0.0, 0.0, 0.0), &[]);
    sampled_property.add_sample(1.0, &Cartesian3::new(1.0, 1.0, 1.0), &[]);
    let sampled: Rc<dyn PositionProperty> = Rc::new(sampled_property);
    property.set_value(Some(vec![Rc::clone(&v1), sampled]));
    assert!(!property.is_constant());

    // equals (reference frame + member values)
    let lhs = PositionPropertyArray::new(Some(vec![Rc::clone(&v1), Rc::clone(&v2)]), None);
    let rhs = PositionPropertyArray::new(Some(vec![Rc::clone(&v1), Rc::clone(&v2)]), None);
    let inertial = PositionPropertyArray::new(
        Some(vec![Rc::clone(&v1), Rc::clone(&v2)]),
        Some(PositionReferenceFrame::Inertial),
    );
    assert!(lhs.equals_position_property_array(&rhs));
    assert!(!lhs.equals_position_property_array(&inertial));
}

#[test]
fn velocity_orientation_property_spec() {
    // Mirrors VelocityOrientationPropertySpec.js (incl. Node golden quaternion)
    use cesium_core::cartesian3::Cartesian3;
    use cesium_core::ellipsoid::Ellipsoid;
    use cesium_core::math::CesiumMath;
    use cesium_core::quaternion::Quaternion;
    use cesium_data_sources::constant_position_property::ConstantPositionProperty;
    use cesium_data_sources::sampled_position_property::SampledPositionProperty;
    use cesium_data_sources::velocity_orientation_property::VelocityOrientationProperty;
    use std::cell::Cell;
    use std::rc::Rc;

    // defaults: WGS84 ellipsoid, constant, not destroyed
    let property = VelocityOrientationProperty::new(None, None);
    assert!(property.is_constant());
    assert!(!property.is_destroyed());
    assert!(property.ellipsoid().equals(&Ellipsoid::WGS84));

    // constructor with a unit sphere
    let property = VelocityOrientationProperty::new(None, Some(Ellipsoid::UNIT_SPHERE));
    assert!(property.ellipsoid().equals(&Ellipsoid::UNIT_SPHERE));

    // definitionChanged is raised when position changes. Start from a unit
    // sphere so the later setEllipsoid(WGS84) actually changes the value.
    let mut property = VelocityOrientationProperty::new(None, Some(Ellipsoid::UNIT_SPHERE));
    let raised = Rc::new(Cell::new(0u32));
    let raised_for_listener = Rc::clone(&raised);
    let _removal = property
        .definition_changed()
        .unwrap()
        .add_listener(move |_| raised_for_listener.set(raised_for_listener.get() + 1));
    property.set_position(Some(Box::new(ConstantPositionProperty::new(
        Cartesian3::new(1.0, 2.0, 3.0),
    ))));
    assert_eq!(raised.get(), 1);

    // definitionChanged is raised when the ellipsoid changes.
    property.set_ellipsoid(Ellipsoid::WGS84);
    assert_eq!(raised.get(), 2);
    property.set_ellipsoid(Ellipsoid::WGS84);
    assert_eq!(raised.get(), 2);

    // Golden quaternion generated from CesiumJS (Node): positions at t=0
    // and t=1/60s are fromDegrees(0,0,0)/fromDegrees(1,0,0), normalize=true.
    let mut sampled = SampledPositionProperty::new(None, None);
    sampled.add_sample(
        0.0,
        &Cartesian3::from_degrees_new(0.0, 0.0, Some(0.0), None),
        &[],
    );
    sampled.add_sample(
        1.0 / 60.0,
        &Cartesian3::from_degrees_new(1.0, 0.0, Some(0.0), None),
        &[],
    );
    let property = VelocityOrientationProperty::new(Some(Box::new(sampled)), None);
    let mut result = Quaternion::default();
    let value = property.get_value_quaternion(0.0, &mut result).unwrap();
    assert!(CesiumMath::equals_epsilon(
        value.x,
        -0.49781358571799506,
        Some(CesiumMath::EPSILON11),
        None
    ));
    assert!(CesiumMath::equals_epsilon(
        value.y,
        -0.5021768950027394,
        Some(CesiumMath::EPSILON11),
        None
    ));
    assert!(CesiumMath::equals_epsilon(
        value.z,
        -0.5021768950027394,
        Some(CesiumMath::EPSILON11),
        None
    ));
    assert!(CesiumMath::equals_epsilon(
        value.w,
        -0.4978135857179951,
        Some(CesiumMath::EPSILON11),
        None
    ));

    // constant position => zero velocity => undefined orientation.
    let property = VelocityOrientationProperty::new(
        Some(Box::new(ConstantPositionProperty::new(Cartesian3::new(
            1.0, 2.0, 3.0,
        )))),
        None,
    );
    assert!(property.get_value_quaternion(0.0, &mut result).is_none());

    // equals: same position + ellipsoid
    use cesium_data_sources::property::Property as _;
    let lhs = VelocityOrientationProperty::new(None, None);
    let rhs = VelocityOrientationProperty::new(None, None);
    assert!(lhs.equals(&rhs));
    let sphere = VelocityOrientationProperty::new(None, Some(Ellipsoid::UNIT_SPHERE));
    assert!(!lhs.equals(&sphere));
}

#[test]
fn velocity_vector_property_spec() {
    // Mirrors VelocityVectorPropertySpec.js
    use cesium_core::cartesian3::Cartesian3;
    use cesium_core::math::CesiumMath;
    use cesium_data_sources::constant_position_property::ConstantPositionProperty;
    use cesium_data_sources::position_property::{PositionProperty, PositionReferenceFrame};
    use cesium_data_sources::sampled_position_property::SampledPositionProperty;
    use cesium_data_sources::velocity_vector_property::VelocityVectorProperty;
    use std::cell::Cell;
    use std::rc::Rc;

    // defaults: normalize = true
    let property = VelocityVectorProperty::new(None, None);
    assert!(property.normalize());
    assert!(property.is_constant());
    assert!(!property.is_destroyed());

    // getValue with no position property (JS Cartesian3.ZERO).
    use cesium_data_sources::property::Property as _;
    match property.get_value(0.0) {
        cesium_data_sources::property::PropertyResult::Cartesian3(x, y, z) => {
            assert_eq!((x, y, z), (0.0, 0.0, 0.0));
        }
        other => panic!("Expected ZERO Cartesian3, got {:?}", other),
    }
    let mut velocity = Cartesian3::ZERO;
    assert!(property.get_value_with_position(0.0, &mut velocity, None).is_none());

    // definitionChanged is raised when position/normalize change.
    let mut property = VelocityVectorProperty::new(None, None);
    let raised = Rc::new(Cell::new(0u32));
    let raised_for_listener = Rc::clone(&raised);
    let _removal = property
        .definition_changed()
        .unwrap()
        .add_listener(move |_| raised_for_listener.set(raised_for_listener.get() + 1));
    property.set_position(Some(Box::new(ConstantPositionProperty::new(
        Cartesian3::new(1.0, 2.0, 3.0),
    ))));
    assert_eq!(raised.get(), 1);
    property.set_normalize(false);
    assert_eq!(raised.get(), 2);
    property.set_normalize(false);
    assert_eq!(raised.get(), 2);

    // normalized velocity of [0,7,8] -> [20,7,8] over 1s => UNIT_X.
    let mut sampled = SampledPositionProperty::new(None, None);
    sampled.add_sample(0.0, &Cartesian3::new(0.0, 7.0, 8.0), &[]);
    sampled.add_sample(1.0, &Cartesian3::new(20.0, 7.0, 8.0), &[]);
    let property = VelocityVectorProperty::new(Some(Box::new(sampled)), Some(true));
    let mut velocity = Cartesian3::ZERO;
    let value = property.get_value_with_position(0.0, &mut velocity, None).unwrap();
    assert!(CesiumMath::equals_epsilon(value.x, 1.0, Some(CesiumMath::EPSILON13), None));
    assert!(CesiumMath::equals_epsilon(value.y, 0.0, Some(CesiumMath::EPSILON13), None));
    assert!(CesiumMath::equals_epsilon(value.z, 0.0, Some(CesiumMath::EPSILON13), None));

    // unnormalized velocity => (20, 0, 0).
    let mut sampled = SampledPositionProperty::new(None, None);
    sampled.add_sample(0.0, &Cartesian3::new(0.0, 7.0, 8.0), &[]);
    sampled.add_sample(1.0, &Cartesian3::new(20.0, 7.0, 8.0), &[]);
    let property = VelocityVectorProperty::new(Some(Box::new(sampled)), Some(false));
    let mut velocity = Cartesian3::ZERO;
    let value = property.get_value_with_position(0.0, &mut velocity, None).unwrap();
    assert!(CesiumMath::equals_epsilon(value.x, 20.0, Some(CesiumMath::EPSILON13), None));
    assert!(CesiumMath::equals_epsilon(value.y, 0.0, Some(CesiumMath::EPSILON13), None));
    assert!(CesiumMath::equals_epsilon(value.z, 0.0, Some(CesiumMath::EPSILON13), None));

    // constant position: normalized => undefined, unnormalized => ZERO.
    let constant = || {
        Box::new(ConstantPositionProperty::new(Cartesian3::new(0.0, 7.0, 8.0)))
            as Box<dyn PositionProperty>
    };
    let property = VelocityVectorProperty::new(Some(constant()), Some(true));
    let mut velocity = Cartesian3::ZERO;
    assert!(property.get_value_with_position(0.0, &mut velocity, None).is_none());
    let property = VelocityVectorProperty::new(Some(constant()), Some(false));
    let mut velocity = Cartesian3::new(9.0, 9.0, 9.0);
    let value = property.get_value_with_position(0.0, &mut velocity, None).unwrap();
    assert_eq!(*value, Cartesian3::ZERO);

    // equals
    let lhs = VelocityVectorProperty::new(None, None);
    let rhs = VelocityVectorProperty::new(None, None);
    assert!(lhs.equals(&rhs));
}

// ============================================================================
// CompositePropertySpec full mirror (addInterval/getValue/definitionChanged/
// removeInterval/equals) — DS-10
// ============================================================================

#[test]
fn composite_property_full_spec() {
    use cesium_data_sources::composite_intervals::CompositeInterval;
    use cesium_data_sources::constant_property::ConstantProperty;
    use std::cell::Cell;
    use std::rc::Rc;

    let mut prop = CompositeProperty::new();
    assert!(prop.is_constant());
    assert!(!prop.is_destroyed());

    let raised = Rc::new(Cell::new(0u32));
    let raised_for_listener = Rc::clone(&raised);
    let _removal = prop
        .definition_changed()
        .unwrap()
        .add_listener(move |_| raised_for_listener.set(raised_for_listener.get() + 1));

    let inner1 = Rc::new(ConstantProperty::new(PropertyResult::Number(5.0)));
    let inner2 = Rc::new(ConstantProperty::new(PropertyResult::Number(6.0)));

    prop.intervals_mut().add_interval(
        CompositeInterval::new(10.0, 12.0, true, true, Rc::clone(&inner1) as Rc<dyn Property>),
        None,
    );
    prop.intervals_mut().add_interval(
        CompositeInterval::new(12.0, 14.0, false, false, Rc::clone(&inner2) as Rc<dyn Property>),
        None,
    );

    // Both addInterval calls changed the collection.
    assert_eq!(raised.get(), 2);
    assert!(!prop.is_constant());

    // getValue at the interval boundaries.
    match prop.get_value_option(10.0) {
        Some(PropertyResult::Number(v)) => assert_eq!(v, 5.0),
        other => panic!("Expected 5 at interval1 start, got {:?}", other),
    }
    // 12.0 belongs to interval1 (its stop is included); interval2 starts
    // open at 12.0, mirroring the JS `indexOf` boundary fallback to the
    // preceding interval whose stop is included.
    match prop.get_value_option(12.0) {
        Some(PropertyResult::Number(v)) => assert_eq!(v, 5.0),
        other => panic!("Expected 5 at shared boundary 12.0, got {:?}", other),
    }
    // 14.0 is excluded from interval2 (open stop) and covered nowhere else.
    assert!(prop.get_value_option(14.0).is_none());

    // removeInterval raises definitionChanged and removes the data.
    prop.intervals_mut().remove_interval(&CompositeInterval::new(
        13.0,
        14.0,
        true,
        true,
        Rc::clone(&inner2) as Rc<dyn Property>,
    ));
    assert_eq!(raised.get(), 3);
    assert!(prop.get_value_option(13.5).is_none());

    // Changing the current interval's data raises definitionChanged.
    inner1.set_value(PropertyResult::Number(7.0));
    assert_eq!(raised.get(), 4);
    match prop.get_value_option(10.5) {
        Some(PropertyResult::Number(v)) => assert_eq!(v, 7.0),
        other => panic!("Expected 7 after setValue, got {:?}", other),
    }

    // An overwritten interval's data change must not raise.
    let mut prop2 = CompositeProperty::new();
    let overwritten = Rc::new(ConstantProperty::new(PropertyResult::Number(1.0)));
    let winner = Rc::new(ConstantProperty::new(PropertyResult::Number(2.0)));
    prop2.intervals_mut().add_interval(
        CompositeInterval::new(11.0, 13.0, true, true, Rc::clone(&overwritten) as Rc<dyn Property>),
        None,
    );
    prop2.intervals_mut().add_interval(
        CompositeInterval::new(10.0, 14.0, true, true, Rc::clone(&winner) as Rc<dyn Property>),
        None,
    );
    let raised2 = Rc::new(Cell::new(0u32));
    let raised2_for_listener = Rc::clone(&raised2);
    let _removal2 = prop2
        .definition_changed()
        .unwrap()
        .add_listener(move |_| raised2_for_listener.set(raised2_for_listener.get() + 1));
    overwritten.set_value(PropertyResult::Number(9.0));
    assert_eq!(raised2.get(), 0);

    // removeAll raises when the collection was non-empty.
    prop.intervals_mut().remove_all();
    assert_eq!(raised.get(), 5);
    assert!(prop.is_constant());

    // equals over the interval collection.
    let mut lhs = CompositeProperty::new();
    lhs.intervals_mut().add_interval(
        CompositeInterval::new(0.0, 1.0, true, true, Rc::clone(&winner) as Rc<dyn Property>),
        None,
    );
    let mut rhs = CompositeProperty::new();
    rhs.intervals_mut().add_interval(
        CompositeInterval::new(0.0, 1.0, true, true, Rc::clone(&winner) as Rc<dyn Property>),
        None,
    );
    assert!(lhs.equals_composite(&rhs));
    rhs.intervals_mut().add_interval(
        CompositeInterval::new(1.0, 2.0, true, true, Rc::clone(&winner) as Rc<dyn Property>),
        None,
    );
    assert!(!lhs.equals_composite(&rhs));
}

#[test]
fn composite_interval_collection_spec() {
    // Mirrors the merge/split/remove semantics of CompositeIntervalCollection
    // (TimeIntervalCollection port used by the Composite* properties).
    use cesium_data_sources::composite_intervals::{
        CompositeInterval, CompositeIntervalCollection,
    };
    use cesium_data_sources::constant_property::ConstantProperty;
    use cesium_data_sources::property::Property;
    use std::cell::Cell;
    use std::rc::Rc;

    let data1 = Rc::new(ConstantProperty::new(PropertyResult::Number(1.0))) as Rc<dyn Property>;
    let data2 = Rc::new(ConstantProperty::new(PropertyResult::Number(2.0))) as Rc<dyn Property>;

    let mut collection = CompositeIntervalCollection::new();
    assert!(collection.is_empty());
    assert_eq!(collection.length(), 0);
    assert!(collection.start().is_none());
    assert!(collection.stop().is_none());

    let changed = Rc::new(Cell::new(0u32));
    let changed_for_listener = Rc::clone(&changed);
    let _removal = collection
        .changed_event()
        .add_listener(move |_| changed_for_listener.set(changed_for_listener.get() + 1));

    // Adjacent intervals with equal data merge.
    collection.add_interval(CompositeInterval::new(0.0, 1.0, true, true, Rc::clone(&data1)), None);
    collection.add_interval(CompositeInterval::new(1.0, 2.0, true, true, Rc::clone(&data1)), None);
    assert_eq!(collection.length(), 1);
    assert_eq!(collection.start(), Some(0.0));
    assert_eq!(collection.stop(), Some(2.0));
    assert_eq!(changed.get(), 2);

    // Different data splits the overlapped interval.
    collection.add_interval(CompositeInterval::new(0.5, 1.5, true, true, Rc::clone(&data2)), None);
    assert_eq!(collection.length(), 3);
    assert!(Rc::ptr_eq(
        &collection.get(0).unwrap().data,
        &data1
    ));
    assert!(Rc::ptr_eq(
        &collection.get(1).unwrap().data,
        &data2
    ));
    assert!(Rc::ptr_eq(
        &collection.get(2).unwrap().data,
        &data1
    ));

    // contains / find_interval_containing_date
    assert!(collection.contains(1.0));
    assert!(!collection.contains(2.5));
    assert!(collection.find_data_for_interval_containing_date(1.0).is_some());
    assert!(collection.find_data_for_interval_containing_date(5.0).is_none());

    // removeInterval punches a hole: [0,0.5][0.5,1.5][1.5,2] minus [0.75,1.25]
    let removed = collection
        .remove_interval(&CompositeInterval::new(0.75, 1.25, true, true, Rc::clone(&data2)));
    assert!(removed);
    assert_eq!(collection.length(), 4);
    assert!(!collection.contains(1.0));

    // removing a span that covers everything empties the collection.
    let removed = collection
        .remove_interval(&CompositeInterval::new(-1.0, 3.0, true, true, Rc::clone(&data1)));
    assert!(removed);
    assert_eq!(collection.length(), 0);

    // removing from an empty collection returns false and does not raise.
    let count_before = changed.get();
    let removed = collection
        .remove_interval(&CompositeInterval::new(0.0, 1.0, true, true, Rc::clone(&data1)));
    assert!(!removed);
    assert_eq!(changed.get(), count_before);

    // removeAll on an empty collection does not raise.
    collection.remove_all();
    assert_eq!(changed.get(), count_before);
}

#[test]
fn reference_property_spec() {
    // Mirrors ReferencePropertySpec.js (value-model subset)
    use cesium_core::cartesian3::Cartesian3;
    use cesium_data_sources::billboard_graphics::BillboardGraphics;
    use cesium_data_sources::entity::Entity;
    use cesium_data_sources::entity_collection::EntityCollection;
    use cesium_data_sources::property::{Property, PropertyResult};
    use cesium_data_sources::reference_property::ReferenceProperty;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    // constructor + getters
    let collection = Rc::new(RefCell::new(EntityCollection::new()));
    let property = ReferenceProperty::new(
        &collection,
        "id",
        vec!["billboard".to_string(), "scale".to_string()],
    );
    assert_eq!(property.target_id(), "id");
    assert_eq!(property.target_property_names(), ["billboard", "scale"]);
    assert!(property.is_constant());
    assert!(!property.is_destroyed());

    // fromString parses "id#property.subproperty"
    let property = ReferenceProperty::from_string(&collection, "id#billboard.scale").unwrap();
    assert_eq!(property.target_id(), "id");
    assert_eq!(property.target_property_names(), ["billboard", "scale"]);

    // fromString without a "#" yields no usable identifier (JS DeveloperError).
    assert!(ReferenceProperty::from_string(&collection, "idbillboardscale").is_none());

    // escaped identifiers
    let property = ReferenceProperty::from_string(&collection, "some\\#id#billboard.scale").unwrap();
    assert_eq!(property.target_id(), "some#id");

    // resolution against a collection entity
    let mut entity = Entity::new("id");
    entity.billboard = Some(BillboardGraphics::default());
    entity.billboard.as_mut().unwrap().scale = 2.0;
    collection.borrow_mut().add(entity);

    let property = ReferenceProperty::from_string(&collection, "id#billboard.scale").unwrap();
    match property.resolved_value() {
        Some(PropertyResult::Number(v)) => assert_eq!(v, 2.0),
        other => panic!("Expected billboard scale 2.0, got {:?}", other),
    }

    // unresolved reference (missing entity) => undefined
    let missing = ReferenceProperty::from_string(&collection, "missing#position").unwrap();
    assert!(missing.resolved_value().is_none());
    assert!(missing.get_value(0.0).is_none());

    // definitionChanged when the target entity's first property changes
    let raised = Rc::new(Cell::new(0u32));
    let raised_for_listener = Rc::clone(&raised);
    let _removal = property
        .definition_changed()
        .unwrap()
        .add_listener(move |_| raised_for_listener.set(raised_for_listener.get() + 1));
    // touch the resolution so the entity subscription is established
    let _ = property.resolved_value();
    {
        let mut borrowed = collection.borrow_mut();
        let entity = borrowed.get_by_id_mut("id").unwrap();
        let mut billboard = BillboardGraphics::default();
        billboard.scale = 3.0;
        entity.set_billboard(Some(billboard));
    }
    assert_eq!(raised.get(), 1);

    // position reference in the fixed frame
    let mut entity2 = Entity::new("p1");
    entity2.position = Some(Cartesian3::new(1.0, 2.0, 3.0));
    collection.borrow_mut().add(entity2);
    let position_ref = ReferenceProperty::from_string(&collection, "p1#position").unwrap();
    match position_ref.get_value_in_reference_frame(0.0) {
        Some(PropertyResult::Cartesian3(x, y, z)) => assert_eq!((x, y, z), (1.0, 2.0, 3.0)),
        other => panic!("Expected position, got {:?}", other),
    }

    // collection remove/add reconnects and raises definitionChanged
    let raised2 = Rc::new(Cell::new(0u32));
    let raised2_for_listener = Rc::clone(&raised2);
    let _removal2 = position_ref
        .definition_changed()
        .unwrap()
        .add_listener(move |_| raised2_for_listener.set(raised2_for_listener.get() + 1));
    let _ = position_ref.resolved_value();
    collection.borrow_mut().remove("p1");
    assert!(position_ref.resolved_value().is_none());
    let mut entity2 = Entity::new("p1");
    entity2.position = Some(Cartesian3::new(4.0, 5.0, 6.0));
    collection.borrow_mut().add(entity2);
    assert_eq!(raised2.get(), 1);

    // equals: same collection/id/names
    let lhs = ReferenceProperty::new(&collection, "id", vec!["position".to_string()]);
    let rhs = ReferenceProperty::new(&collection, "id", vec!["position".to_string()]);
    let other_names = ReferenceProperty::new(&collection, "id", vec!["show".to_string()]);
    let other_collection = Rc::new(RefCell::new(EntityCollection::new()));
    let other_coll_prop = ReferenceProperty::new(&other_collection, "id", vec!["position".to_string()]);
    assert!(lhs.equals(&rhs));
    assert!(!lhs.equals(&other_names));
    assert!(!lhs.equals(&other_coll_prop));
}

#[test]
fn teme_to_pseudo_fixed_golden_spec() {
    // Golden vectors generated from CesiumJS
    // (Transforms.computeTemeToPseudoFixedMatrix, Node gen_golden_ds49.mjs).
    use cesium_core::julian_date::JulianDate;
    use cesium_core::math::CesiumMath;
    use cesium_core::time_standard::TimeStandard;
    use cesium_core::transforms;

    let golden: &[(i32, f64, [f64; 9])] = &[
        (
            2451545,
            0.0,
            [
                0.18155965302975435,
                0.9833799328803263,
                0.0,
                -0.9833799328803263,
                0.18155965302975435,
                0.0,
                0.0,
                0.0,
                1.0,
            ],
        ),
        (
            2451545,
            43200.0,
            [
                -0.19001127264582623,
                -0.9817819087086059,
                0.0,
                0.9817819087086059,
                -0.19001127264582623,
                0.0,
                0.0,
                0.0,
                1.0,
            ],
        ),
        (
            2459000,
            12345.678,
            [
                -0.5006485068079232,
                -0.865650664316153,
                0.0,
                0.865650664316153,
                -0.5006485068079232,
                0.0,
                0.0,
                0.0,
                1.0,
            ],
        ),
    ];

    for (day, seconds, expected) in golden {
        // JS `new JulianDate(day, seconds)` defaults to UTC.
        let date = JulianDate::new(*day as f64, *seconds, TimeStandard::UTC);
        let mut result = cesium_core::matrix3::Matrix3::default();
        transforms::compute_teme_to_pseudo_fixed_matrix(&date, &mut result);
        for i in 0..9 {
            assert!(
                CesiumMath::equals_epsilon(result.elements[i], expected[i], Some(1e-12), None),
                "TEME mismatch at day={} seconds={} element {}: got {} expected {}",
                day,
                seconds,
                i,
                result.elements[i],
                expected[i]
            );
        }
    }
}

#[test]
#[ignore = "Requires terrain offset computation"]
fn terrain_offset_property_spec() {}

// UN-IGNORED: node_transformation_property works without scene
#[test]
fn node_transformation_property_spec() {
    use cesium_data_sources::node_transformation_property::NodeTransformationProperty;
    use cesium_data_sources::property::Property;
    use cesium_core::cartesian3::Cartesian3;
    use cesium_core::quaternion::Quaternion;

    let mut prop = NodeTransformationProperty::new();
    assert!(prop.is_constant());
    assert!(!prop.is_destroyed());
    assert!(prop.translation.is_none());
    assert!(prop.rotation.is_none());
    assert!(prop.scale.is_none());

    prop.translation = Some(Cartesian3::new(1.0, 2.0, 3.0));
    prop.rotation = Some(Quaternion::new(0.0, 0.0, 0.0, 1.0));
    prop.scale = Some(Cartesian3::new(2.0, 2.0, 2.0));
    assert!(prop.translation.is_some());
    assert!(prop.rotation.is_some());
    assert!(prop.scale.is_some());
}

// Batch specs (require scene + geometry pipeline)
#[test]
#[ignore = "Requires scene + static geometry batching"]
fn static_geometry_color_batch_spec() {}

#[test]
#[ignore = "Requires scene + static geometry batching"]
fn static_geometry_per_material_batch_spec() {}

#[test]
#[ignore = "Requires scene + ground geometry batching"]
fn static_ground_geometry_color_batch_spec() {}

#[test]
#[ignore = "Requires scene + ground geometry batching"]
fn static_ground_geometry_per_material_batch_spec() {}

#[test]
#[ignore = "Requires scene + ground polyline batching"]
fn static_ground_polyline_per_material_batch_spec() {}

#[test]
#[ignore = "Requires scene + outline geometry batching"]
fn static_outline_geometry_batch_spec() {}

#[test]
#[ignore = "Requires scene + dynamic geometry batching"]
fn dynamic_geometry_updater_spec() {}

// KML sub-specs (ported from KmlTourSpec.js / KmlTourFlyToSpec.js value
// models; playback is not materialized, see the DEVIATION notes in
// `kml_tour.rs` / `kml_tour_fly_to.rs`).
#[test]
fn kml_tour_spec() {
    use cesium_data_sources::kml_data_source::KmlDataSource;
    use cesium_data_sources::kml_tour::KmlTourEntry;

    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Document xmlns="http://www.opengis.net/kml/2.2"
                   xmlns:gx="http://www.google.com/kml/ext/2.2">
          <gx:Tour id="tour-1">
            <name>Test Tour</name>
            <gx:Playlist>
              <gx:Wait>
                <gx:duration>1.5</gx:duration>
              </gx:Wait>
              <gx:FlyTo>
                <gx:duration>3</gx:duration>
              </gx:FlyTo>
            </gx:Playlist>
          </gx:Tour>
        </Document>"#;

    let data_source = KmlDataSource::load(kml, None).unwrap();
    let tours = data_source.kml_tours();
    assert_eq!(tours.len(), 1);
    let tour = &tours[0];
    assert_eq!(tour.name.as_deref(), Some("Test Tour"));
    assert_eq!(tour.id.as_deref(), Some("tour-1"));
    assert_eq!(tour.playlist.len(), 2);
    match &tour.playlist[0] {
        KmlTourEntry::Wait(wait) => assert_eq!(wait.duration, Some(1.5)),
        other => panic!("expected KmlTourWait, got {:?}", other),
    }
    match &tour.playlist[1] {
        KmlTourEntry::FlyTo(fly_to) => assert_eq!(fly_to.duration, Some(3.0)),
        other => panic!("expected KmlTourFlyTo, got {:?}", other),
    }
}

#[test]
fn kml_tour_fly_to_spec() {
    use cesium_data_sources::kml_data_source::KmlDataSource;
    use cesium_data_sources::kml_tour::{KmlTourEntry, KmlTourView};

    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Document xmlns="http://www.opengis.net/kml/2.2"
                   xmlns:gx="http://www.google.com/kml/ext/2.2">
          <gx:Tour>
            <gx:Playlist>
              <gx:FlyTo>
                <gx:duration>5</gx:duration>
                <gx:flyToMode>bounce</gx:flyToMode>
                <LookAt>
                    <longitude>10</longitude>
                    <latitude>20</latitude>
                    <range>40</range>
                </LookAt>
              </gx:FlyTo>
            </gx:Playlist>
          </gx:Tour>
        </Document>"#;

    let data_source = KmlDataSource::load(kml, None).unwrap();
    let tours = data_source.kml_tours();
    assert_eq!(tours.len(), 1);
    let fly_to = match &tours[0].playlist[0] {
        KmlTourEntry::FlyTo(fly_to) => fly_to,
        other => panic!("expected KmlTourFlyTo, got {:?}", other),
    };
    assert_eq!(fly_to.duration, Some(5.0));
    assert_eq!(fly_to.fly_to_mode.as_deref(), Some("bounce"));
    match fly_to.view.as_ref().expect("view defined") {
        KmlTourView::LookAt(look_at) => {
            assert_eq!(look_at.heading_pitch_range.range, 40.0);
        }
        other => panic!("expected KmlLookAt view, got {:?}", other),
    }
}
