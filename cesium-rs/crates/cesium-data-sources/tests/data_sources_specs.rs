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

#[test]
#[ignore = "Requires XML parsing GPX + track/route handling"]
fn gpx_data_source_spec() {}

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
    assert!(m.is_constant());
    assert!(!m.is_destroyed());
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
    assert!(m.is_constant());
    assert!(!m.is_destroyed());
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
    use cesium_data_sources::composite_material_property::CompositeMaterialProperty;
    use cesium_data_sources::color_material_property::ColorMaterialProperty;
    use cesium_data_sources::material_property::MaterialProperty;
    use cesium_core::color::Color;

    let mut cmp = CompositeMaterialProperty::new();
    assert_eq!(cmp.type_name(), "Composite");
    assert!(cmp.is_constant()); // 0 or 1 intervals = constant

    cmp.add_interval(0.0, 10.0, Box::new(ColorMaterialProperty::new(Color::new(1.0, 0.0, 0.0, 1.0))));
    assert!(cmp.is_constant()); // exactly 1 interval

    cmp.add_interval(10.0, 20.0, Box::new(ColorMaterialProperty::new(Color::new(0.0, 1.0, 0.0, 1.0))));
    assert!(!cmp.is_constant()); // 2 intervals = dynamic

    // get_material_at returns the active material
    let mat = cmp.get_material_at(5.0);
    assert!(mat.is_some());
    assert_eq!(mat.unwrap().type_name(), "Color");

    let mat2 = cmp.get_material_at(25.0);
    assert!(mat2.is_none());
}

#[test]
fn grid_material_property_spec() {
    use cesium_data_sources::grid_material_property::GridMaterialProperty;
    use cesium_data_sources::material_property::MaterialProperty;

    let m = GridMaterialProperty::new();
    assert_eq!(m.type_name(), "Grid");
    assert!(m.is_constant());
    assert!(!m.is_destroyed());
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
    assert!(m.is_constant());
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
    assert!(m.is_constant());
    assert!(!m.is_destroyed());
}

#[test]
fn polyline_dash_material_property_spec() {
    use cesium_data_sources::polyline_dash_material_property::PolylineDashMaterialProperty;
    use cesium_data_sources::material_property::MaterialProperty;

    let m = PolylineDashMaterialProperty::new();
    assert_eq!(m.type_name(), "PolylineDash");
    assert!(m.is_constant());
    assert!(!m.is_destroyed());
}

#[test]
fn polyline_glow_material_property_spec() {
    use cesium_data_sources::polyline_glow_material_property::PolylineGlowMaterialProperty;
    use cesium_data_sources::material_property::MaterialProperty;

    let m = PolylineGlowMaterialProperty::new();
    assert_eq!(m.type_name(), "PolylineGlow");
    assert!(m.is_constant());
    assert!(!m.is_destroyed());
}

#[test]
fn polyline_outline_material_property_spec() {
    use cesium_data_sources::polyline_outline_material_property::PolylineOutlineMaterialProperty;
    use cesium_data_sources::material_property::MaterialProperty;

    let m = PolylineOutlineMaterialProperty::new();
    assert_eq!(m.type_name(), "PolylineOutline");
    assert!(m.is_constant());
    assert!(!m.is_destroyed());
}

#[test]
fn stripe_material_property_spec() {
    use cesium_data_sources::stripe_material_property::StripeMaterialProperty;
    use cesium_data_sources::material_property::MaterialProperty;
    use cesium_data_sources::stripe_orientation::StripeOrientation;

    let m = StripeMaterialProperty::new();
    assert_eq!(m.type_name(), "Stripe");
    assert!(m.is_constant());
    assert!(!m.is_destroyed());
    assert_eq!(m.orientation, StripeOrientation::Horizontal);
    assert_eq!(m.repeat, 1.0);
}

// Position/Property specs (partially covered above, full specs need interpolation)
#[test]
#[ignore = "Requires full interpolation (Lagrange/Hermite)"]
fn sampled_property_spec() {}

#[test]
#[ignore = "Requires full interpolation + Cartesian3"]
fn sampled_position_property_spec() {}

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
    use cesium_data_sources::composite_position_property::CompositePositionProperty;
    use cesium_data_sources::constant_position_property::ConstantPositionProperty;
    use cesium_data_sources::position_property::{PositionProperty, PositionReferenceFrame};
    use cesium_data_sources::property::Property;
    use cesium_core::cartesian3::Cartesian3;

    let mut composite = CompositePositionProperty::new(PositionReferenceFrame::Fixed);

    // Not constant when empty? No: is_constant() returns true when intervals.len() <= 1
    assert!(composite.is_constant()); // 0 intervals
    assert!(!composite.is_destroyed());
    assert_eq!(composite.reference_frame(), PositionReferenceFrame::Fixed);

    composite.add_interval(0.0, 10.0, Box::new(ConstantPositionProperty::new(Cartesian3::new(1.0, 2.0, 3.0))));
    assert!(composite.is_constant()); // 1 interval

    composite.add_interval(10.0, 20.0, Box::new(ConstantPositionProperty::new(Cartesian3::new(4.0, 5.0, 6.0))));
    assert!(!composite.is_constant()); // 2 intervals

    // get_value in first interval
    let val = composite.get_value(5.0);
    match val {
        cesium_data_sources::property::PropertyResult::Position(x, y, z) => {
            assert_eq!(x, 1.0);
            assert_eq!(y, 2.0);
            assert_eq!(z, 3.0);
        }
        _ => panic!("Expected Position in first interval"),
    }

    // get_value in second interval
    let val2 = composite.get_value(15.0);
    match val2 {
        cesium_data_sources::property::PropertyResult::Position(x, y, z) => {
            assert_eq!(x, 4.0);
            assert_eq!(y, 5.0);
            assert_eq!(z, 6.0);
        }
        _ => panic!("Expected Position in second interval"),
    }

    // get_value outside all intervals
    let val3 = composite.get_value(25.0);
    assert!(matches!(val3, cesium_data_sources::property::PropertyResult::None));
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
#[ignore = "Requires position property array"]
fn position_property_array_spec() {}

#[test]
#[ignore = "Requires velocity computation"]
fn velocity_orientation_property_spec() {}

#[test]
#[ignore = "Requires velocity computation"]
fn velocity_vector_property_spec() {}

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

// KML sub-specs
#[test]
#[ignore = "Requires KML tour logic"]
fn kml_tour_spec() {}

#[test]
#[ignore = "Requires KML tour fly-to logic"]
fn kml_tour_fly_to_spec() {}
