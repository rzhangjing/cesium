//! Ported specs from `packages/engine/Specs/DataSources/GeoJsonDataSourceSpec.js`.
//!
//! Every test mirrors one `it()` of the original Jasmine spec; the test
//! names keep the original descriptions snake-cased so they stay mappable.
//! Assertions that depend on browser-only facilities (canvas pin images,
//! promise timing, owner back-references) are adapted as documented in the
//! individual DEVIATION comments.

use std::sync::{Arc, LazyLock, Mutex, MutexGuard};

use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;
use cesium_data_sources::geo_json_data_source::{
    default_describe, defaults, pin_from_color, pin_from_maki_icon_id, register_crs_link_href,
    reset_defaults, set_defaults, unregister_crs_link_href, CrsFunction, GeoJsonDataSource,
    GeoJsonLoadOptions,
};
use cesium_scene::height_reference::HeightReference;
use cesium_specs::data_path;
use serde_json::{json, Value};

// Serializes the tests of this file: several specs mutate the global
// styling defaults (the JS `GeoJsonDataSource.markerSize = ...` statics)
// which Jasmine ran sequentially. DEVIATION: Jasmine `beforeEach` reset is
// performed by `guard()` instead.
static TEST_GUARD: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn guard() -> MutexGuard<'static, ()> {
    // Recover from a poisoned guard so one failing spec does not cascade
    // into every other one.
    let g = TEST_GUARD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    reset_defaults();
    g
}

fn no_options() -> GeoJsonLoadOptions {
    GeoJsonLoadOptions::default()
}

// Mirror of the spec helper `coordinatesToCartesian`.
fn coordinates_to_cartesian(coordinates: &[f64]) -> Cartesian3 {
    Cartesian3::from_degrees_new(
        coordinates[0],
        coordinates[1],
        coordinates.get(2).copied(),
        None,
    )
}

// Mirror of the spec helper `coordinatesArrayToCartesian`.
fn coordinates_array_to_cartesian(coordinates: &[[f64; 2]]) -> Vec<Cartesian3> {
    coordinates
        .iter()
        .map(|c| coordinates_to_cartesian(&[c[0], c[1]]))
        .collect()
}

// Mirror of the spec helper `multiLineToCartesian`.
fn multi_line_to_cartesian(geometry: &Value) -> Vec<Vec<Cartesian3>> {
    geometry["coordinates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|line| {
            line.as_array()
                .unwrap()
                .iter()
                .map(|c| {
                    let arr = c.as_array().unwrap();
                    coordinates_to_cartesian(&[arr[0].as_f64().unwrap(), arr[1].as_f64().unwrap()])
                })
                .collect()
        })
        .collect()
}

// ============================================================================
// Spec fixtures (verbatim mirrors of the JS consts at the top of the spec).
// ============================================================================

fn point() -> Value {
    json!({ "type": "Point", "coordinates": [102.0, 0.5] })
}

fn point_named_crs() -> Value {
    json!({
        "type": "Point",
        "coordinates": [102.0, 0.5],
        "crs": { "type": "name", "properties": { "name": "EPSG:4326" } }
    })
}

fn point_named_crs_ogc() -> Value {
    json!({
        "type": "Point",
        "coordinates": [102.0, 0.5],
        "crs": { "type": "name", "properties": { "name": "urn:ogc:def:crs:OGC:1.3:CRS84" } }
    })
}

fn point_named_crs_epsg() -> Value {
    json!({
        "type": "Point",
        "coordinates": [102.0, 0.5],
        "crs": { "type": "name", "properties": { "name": "urn:ogc:def:crs:EPSG::4326" } }
    })
}

fn point_crs_link_href() -> Value {
    json!({
        "type": "Point",
        "coordinates": [102.0, 0.5],
        "crs": { "type": "link", "properties": { "href": "http://crs.invalid" } }
    })
}

fn point_crs_epsg() -> Value {
    json!({
        "type": "Point",
        "coordinates": [102.0, 0.5],
        "crs": { "type": "EPSG", "properties": { "code": 4326 } }
    })
}

fn line_string() -> Value {
    json!({ "type": "LineString", "coordinates": [[100.0, 0.0], [101.0, 1.0]] })
}

fn polygon() -> Value {
    json!({
        "type": "Polygon",
        "coordinates": [[[100.0, 0.0], [101.0, 0.0], [101.0, 1.0], [100.0, 1.0], [100.0, 0.0]]]
    })
}

fn polygon_with_holes() -> Value {
    json!({
        "type": "Polygon",
        "coordinates": [
            [[100.0, 0.0], [101.0, 0.0], [101.0, 1.0], [100.0, 1.0], [100.0, 0.0]],
            [[100.2, 0.2], [100.8, 0.2], [100.8, 0.8], [100.2, 0.8], [100.2, 0.2]]
        ]
    })
}

fn polygon_with_heights() -> Value {
    json!({
        "type": "Polygon",
        "coordinates": [[[100.0, 0.0, 1.0], [101.0, 0.0, 2.0], [101.0, 1.0, 1.0], [100.0, 1.0, 2.0], [100.0, 0.0, 3.0]]]
    })
}

const MULTI_POINT_COORDS: [[f64; 2]; 3] = [[100.0, 0.0], [101.0, 1.0], [101.0, 3.0]];

fn multi_point() -> Value {
    json!({ "type": "MultiPoint", "coordinates": [[100.0, 0.0], [101.0, 1.0], [101.0, 3.0]] })
}

fn multi_line_string() -> Value {
    json!({
        "type": "MultiLineString",
        "coordinates": [[[100.0, 0.0], [101.0, 1.0]], [[102.0, 2.0], [103.0, 3.0]]]
    })
}

fn multi_polygon() -> Value {
    json!({
        "type": "MultiPolygon",
        "coordinates": [
            [[[102.0, 2.0], [103.0, 2.0], [103.0, 3.0], [102.0, 3.0], [102.0, 2.0]]],
            [[[100.0, 0.0], [101.0, 0.0], [101.0, 1.0], [100.0, 1.0], [100.0, 0.0]]]
        ]
    })
}

fn geometry_collection() -> Value {
    json!({
        "type": "GeometryCollection",
        "geometries": [
            { "type": "Point", "coordinates": [100.0, 0.0] },
            { "type": "LineString", "coordinates": [[101.0, 0.0], [102.0, 1.0]] }
        ]
    })
}

fn feature() -> Value {
    json!({ "type": "Feature", "geometry": point() })
}

fn feature_with_null_name() -> Value {
    json!({ "type": "Feature", "geometry": point(), "properties": { "name": null } })
}

fn feature_with_id() -> Value {
    json!({ "id": "myId", "type": "Feature", "geometry": geometry_collection() })
}

fn feature_undefined_geometry() -> Value {
    json!({ "type": "Feature" })
}

fn feature_null_geometry() -> Value {
    json!({ "type": "Feature", "geometry": null })
}

fn unknown_geometry() -> Value {
    json!({ "type": "TimeyWimey", "coordinates": [0, 0] })
}

fn feature_unknown_geometry() -> Value {
    json!({ "type": "Feature", "geometry": unknown_geometry() })
}

fn geometry_collection_unknown_type() -> Value {
    json!({ "type": "GeometryCollection", "geometries": [unknown_geometry()] })
}

fn topo_json() -> Value {
    json!({
        "type": "Topology",
        "transform": { "scale": [1, 1], "translate": [0, 0] },
        "objects": {
            "polygon": {
                "type": "Polygon",
                "arcs": [[0, 1, 2, 3]],
                "properties": { "myProps": 0 }
            },
            "lineString": {
                "type": "LineString",
                "arcs": [4],
                "properties": { "myProps": 1 }
            }
        },
        "arcs": [
            [[0, 0], [1, 0], [0, 1], [-1, 0], [0, -1]],
            [[0, 0], [1, 0], [0, 1]],
            [[1, 1], [-1, 0], [0, -1]],
            [[1, 1]],
            [[0, 0]]
        ]
    })
}

fn mixed_geometries() -> Value {
    json!({
        "type": "GeometryCollection",
        "geometries": [line_string(), polygon(), point()]
    })
}

// ============================================================================
// it("default constructor has expected values")
// ============================================================================
#[test]
fn default_constructor_has_expected_values() {
    let _g = guard();
    let data_source = GeoJsonDataSource::new();
    // DEVIATION: `changedEvent`/`errorEvent` being Event instances is
    // guaranteed by the type system; assert they are usable instead.
    assert_eq!(data_source.changed_event().number_of_listeners(), 0);
    assert_eq!(data_source.error_event().number_of_listeners(), 0);
    // `clock` is always undefined for this data source: the Rust port has
    // no clock member at all.
    assert!(data_source.display_name().is_none());
    assert_eq!(data_source.entities().values().len(), 0);
    assert!(data_source.show());
    assert!(data_source.credit().is_none());
}

// ============================================================================
// it("credit gets set from options")
// ============================================================================
#[test]
fn credit_gets_set_from_options() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    let options = GeoJsonLoadOptions {
        credit: Some("This is my credit".to_string()),
        ..Default::default()
    };
    data_source.load_value(&point(), &options).unwrap();
    assert!(data_source.credit().is_some());
}

// ============================================================================
// it("setting name raises changed event")
// ============================================================================
#[test]
fn setting_name_raises_changed_event() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();

    let count = std::rc::Rc::new(std::cell::Cell::new(0u32));
    let counter = count.clone();
    let _remove = data_source
        .changed_event()
        .add_listener(move |_a: &()| counter.set(counter.get() + 1));

    let new_name = "chester";
    data_source.set_name(new_name);
    assert_eq!(data_source.display_name(), Some(new_name));
    assert_eq!(count.get(), 1);
}

// ============================================================================
// it("show sets underlying entity collection show.")
// ============================================================================
#[test]
fn show_sets_underlying_entity_collection_show() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();

    data_source.set_show(false);
    assert!(!data_source.show());
    assert_eq!(data_source.show(), data_source.entities().show);

    data_source.set_show(true);
    assert!(data_source.show());
    assert_eq!(data_source.show(), data_source.entities().show);
}

// ============================================================================
// it("Works with null geometry")
// ============================================================================
#[test]
fn works_with_null_geometry() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    data_source
        .load_value(&feature_null_geometry(), &no_options())
        .unwrap();
    let entity = &data_source.entities().values()[0];
    // DEVIATION: JS compares reference identity of `properties` (undefined
    // here); the Rust PropertyBag is simply empty.
    assert_eq!(entity.properties.length(), 0);
    assert!(entity.position.is_none());
}

// ============================================================================
// it("Works with feature")
// ============================================================================
#[test]
fn works_with_feature() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    data_source.load_value(&feature(), &no_options()).unwrap();
    let entity = &data_source.entities().values()[0];
    assert_eq!(entity.properties.length(), 0);
    assert_eq!(
        entity.position,
        Some(coordinates_to_cartesian(&[102.0, 0.5]))
    );
    assert!(entity.billboard.is_some());
}

// ============================================================================
// it("Adds a feature without removing existing entities")
// ============================================================================
#[test]
fn adds_a_feature_without_removing_existing_entities() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    data_source.load_value(&feature(), &no_options()).unwrap();
    // `feature` has one Entity, `mixedGeometries` has 3
    data_source
        .process_value(&mixed_geometries(), &no_options())
        .unwrap();
    assert_eq!(data_source.entities().values().len(), 4);
}

// ============================================================================
// it("Creates default description from properties")
// ============================================================================
#[test]
fn creates_default_description_from_properties() {
    let _g = guard();
    let feature_with_properties = json!({
        "type": "Feature",
        "geometry": point(),
        "properties": { "prop1": "dog", "prop2": "cat", "prop3": "liger" }
    });

    let mut data_source = GeoJsonDataSource::new();
    data_source
        .load_value(&feature_with_properties, &no_options())
        .unwrap();
    let entity = &data_source.entities().values()[0];
    let description = entity.description.as_ref().expect("description defined");
    for needle in ["prop1", "prop2", "prop3", "dog", "cat", "liger"] {
        assert!(description.contains(needle), "missing {}", needle);
    }
}

// ============================================================================
// it("Creates custom description string from properties")
// ============================================================================
#[test]
fn creates_custom_description_string_from_properties() {
    let _g = guard();
    let feature_with_properties = json!({
        "type": "Feature",
        "geometry": point(),
        "properties": { "prop1": "dog", "prop2": "cat" }
    });

    fn test_describe(properties: &Value, _name_property: Option<&str>) -> String {
        let mut desc = String::new();
        for (key, value) in properties.as_object().unwrap() {
            desc.push_str(&format!("{} = {}. ", key, value.as_str().unwrap()));
        }
        desc
    }

    let mut data_source = GeoJsonDataSource::new();
    let options = GeoJsonLoadOptions {
        describe: Some(Arc::new(test_describe)),
        ..Default::default()
    };
    data_source
        .load_value(&feature_with_properties, &options)
        .unwrap();
    let entity = &data_source.entities().values()[0];
    let description = entity.description.as_ref().expect("description defined");
    assert!(description.contains("prop1 = dog."));
    assert!(description.contains("prop2 = cat."));
}

// ============================================================================
// it("Creates custom description from properties, using a describeProperty")
// ============================================================================
#[test]
fn creates_custom_description_from_properties_using_a_describe_property() {
    let _g = guard();
    let feature_with_properties = json!({
        "type": "Feature",
        "geometry": point(),
        "properties": { "prop1": "dog", "prop2": "cat" }
    });

    // DEVIATION: the JS spec wraps the describe function in a
    // CallbackProperty; description values are plain strings in this port,
    // so the same describe function is exercised directly.
    fn test_describe(properties: &Value, _name_property: Option<&str>) -> String {
        let mut desc = String::new();
        for (key, value) in properties.as_object().unwrap() {
            desc.push_str(&format!("{} = {}; ", key, value.as_str().unwrap()));
        }
        desc
    }

    let mut data_source = GeoJsonDataSource::new();
    let options = GeoJsonLoadOptions {
        describe: Some(Arc::new(test_describe)),
        ..Default::default()
    };
    data_source
        .load_value(&feature_with_properties, &options)
        .unwrap();
    let entity = &data_source.entities().values()[0];
    let description = entity.description.as_ref().expect("description defined");
    assert!(description.contains("prop1 = dog;"));
    assert!(description.contains("prop2 = cat;"));
}

// ============================================================================
// it("Uses description if present")
// ============================================================================
#[test]
fn uses_description_if_present() {
    let _g = guard();
    let feature_with_description = json!({
        "type": "Feature",
        "geometry": point(),
        "properties": {
            "prop1": "dog",
            "prop2": "cat",
            "prop3": "liger",
            "description": "This is my descriptiong!"
        }
    });

    let mut data_source = GeoJsonDataSource::new();
    data_source
        .load_value(&feature_with_description, &no_options())
        .unwrap();
    let entity = &data_source.entities().values()[0];
    assert_eq!(
        entity.description.as_deref(),
        Some("This is my descriptiong!")
    );
}

// ============================================================================
// it("Handles null description")
// ============================================================================
#[test]
fn handles_null_description() {
    let _g = guard();
    let feature_with_null_description = json!({
        "type": "Feature",
        "geometry": point(),
        "properties": { "description": null }
    });

    let mut data_source = GeoJsonDataSource::new();
    data_source
        .load_value(&feature_with_null_description, &no_options())
        .unwrap();
    let entity = &data_source.entities().values()[0];
    assert!(entity.description.is_none());
}

// ============================================================================
// it('Does not use "name" property as the object's name if it is null')
// ============================================================================
#[test]
fn does_not_use_name_property_as_the_objects_name_if_it_is_null() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    data_source
        .load_value(&feature_with_null_name(), &no_options())
        .unwrap();
    let entity = &data_source.entities().values()[0];
    assert!(entity.name.is_none());
    assert!(entity.properties.has("name"));
    assert_eq!(
        entity.position,
        Some(coordinates_to_cartesian(&[102.0, 0.5]))
    );
    assert!(entity.billboard.is_some());
}

// ============================================================================
// it("Works with feature with id")
// ============================================================================
#[test]
fn works_with_feature_with_id() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    data_source
        .load_value(&feature_with_id(), &no_options())
        .unwrap();
    let entities = data_source.entities().values();
    assert_eq!(entities[0].id, "myId");
    assert_eq!(entities[1].id, "myId_2");
}

// ============================================================================
// it("Works with null id")
// ============================================================================
#[test]
fn works_with_null_id() {
    let _g = guard();
    let geojson = json!({ "id": null, "type": "Feature", "geometry": null });

    let mut data_source = GeoJsonDataSource::new();
    data_source.load_value(&geojson, &no_options()).unwrap();
    let entity = &data_source.entities().values()[0];
    assert!(!entity.id.is_empty());
}

// ============================================================================
// it("Works with null properties")
// ============================================================================
#[test]
fn works_with_null_properties() {
    let _g = guard();
    let geojson = json!({ "type": "Feature", "geometry": null, "properties": null });

    let mut data_source = GeoJsonDataSource::new();
    data_source.load_value(&geojson, &no_options()).unwrap();
    let entity = &data_source.entities().values()[0];
    // DEVIATION: JS asserts `entity.properties` toBeUndefined; the Rust
    // entity always carries a PropertyBag, which stays empty here.
    assert_eq!(entity.properties.length(), 0);
}

// DEVIATION: it("Has entity collection with link to data source") and
// it("Has entity with link to entity collection") are not mirrored: the
// Rust EntityCollection/Entity ports carry no owner back-references
// (ownership is expressed through the DataSource owning the collection).

// ============================================================================
// it("Works with point geometry")
// ============================================================================
#[test]
fn works_with_point_geometry() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    data_source.load_value(&point(), &no_options()).unwrap();
    let entity = &data_source.entities().values()[0];
    assert_eq!(entity.properties.length(), 0);
    assert_eq!(
        entity.position,
        Some(coordinates_to_cartesian(&[102.0, 0.5]))
    );
    let billboard = entity.billboard.as_ref().expect("billboard defined");
    assert!(billboard.image.is_some());
}

// ============================================================================
// it("Works with point geometry clamped to ground")
// ============================================================================
#[test]
fn works_with_point_geometry_clamped_to_ground() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    let options = GeoJsonLoadOptions {
        clamp_to_ground: Some(true),
        ..Default::default()
    };
    data_source.load_value(&point(), &options).unwrap();
    let entity = &data_source.entities().values()[0];
    assert_eq!(
        entity.position,
        Some(coordinates_to_cartesian(&[102.0, 0.5]))
    );
    let billboard = entity.billboard.as_ref().expect("billboard defined");
    assert!(billboard.image.is_some());
    assert_eq!(
        billboard.height_reference,
        HeightReference::ClampToGround as i32
    );
}

// ============================================================================
// it("Works with point geometry with simplystyle")
// ============================================================================
#[test]
fn works_with_point_geometry_with_simplestyle() {
    let _g = guard();
    let geojson = json!({
        "type": "Point",
        "coordinates": [102.0, 0.5],
        "properties": {
            "marker-size": "large",
            "marker-symbol": "bus",
            "marker-color": "#ffffff"
        }
    });

    let mut data_source = GeoJsonDataSource::new();
    data_source.load_value(&geojson, &no_options()).unwrap();
    let entity = &data_source.entities().values()[0];
    let billboard = entity.billboard.as_ref().expect("billboard defined");
    // DEVIATION: the JS spec compares the rendered canvas pin; this port
    // compares the deterministic pin descriptor with identical inputs.
    let expected = pin_from_maki_icon_id("bus", Color::WHITE, 64.0)
        .expect("bus is a valid maki icon");
    assert_eq!(billboard.image.as_deref(), Some(expected.as_str()));
}

// ============================================================================
// it("Works with point geometry with null simplystyle")
// ============================================================================
#[test]
fn works_with_point_geometry_with_null_simplestyle() {
    let _g = guard();
    let geojson = json!({
        "type": "Point",
        "coordinates": [102.0, 0.5],
        "properties": {
            "marker-size": null,
            "marker-symbol": null,
            "marker-color": null
        }
    });

    let defaults = defaults();
    let image = pin_from_color(defaults.marker_color, defaults.marker_size);

    let mut data_source = GeoJsonDataSource::new();
    data_source.load_value(&geojson, &no_options()).unwrap();
    let entity = &data_source.entities().values()[0];
    let billboard = entity.billboard.as_ref().expect("billboard defined");
    assert_eq!(billboard.image.as_deref(), Some(image.as_str()));
}

// ============================================================================
// it("Works with point geometry and unknown simplystyle")
// ============================================================================
#[test]
fn works_with_point_geometry_and_unknown_simplestyle() {
    let _g = guard();
    let geojson = json!({
        "type": "Point",
        "coordinates": [102.0, 0.5],
        "properties": {
            "marker-size": "large",
            "marker-symbol": "notAnIcon",
            "marker-color": "#ffffff"
        }
    });

    let mut data_source = GeoJsonDataSource::new();
    data_source.load_value(&geojson, &no_options()).unwrap();
    let entity = &data_source.entities().values()[0];
    let billboard = entity.billboard.as_ref().expect("billboard defined");
    // Unknown maki ids "fail to load" and fall back to the plain color pin
    // (mirror of the JS promise `.catch` fallback).
    let expected = pin_from_color(Color::WHITE, 64.0);
    assert_eq!(billboard.image.as_deref(), Some(expected.as_str()));
}

// ============================================================================
// it("Works with multipoint geometry")
// ============================================================================
#[test]
fn works_with_multipoint_geometry() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    data_source.load_value(&multi_point(), &no_options()).unwrap();
    let entities = data_source.entities().values();
    let expected_positions = coordinates_array_to_cartesian(&MULTI_POINT_COORDS);
    for (i, entity) in entities.iter().enumerate() {
        assert_eq!(entity.properties.length(), 0);
        assert_eq!(entity.position, Some(expected_positions[i]));
        let billboard = entity.billboard.as_ref().expect("billboard defined");
        assert!(billboard.image.is_some());
    }
}

// ============================================================================
// it("Works with multipoint geometry clamped to ground")
// ============================================================================
#[test]
fn works_with_multipoint_geometry_clamped_to_ground() {
    let _g = guard();
    set_defaults(|d| d.clamp_to_ground = true);
    let mut data_source = GeoJsonDataSource::new();
    data_source.load_value(&multi_point(), &no_options()).unwrap();
    let entities = data_source.entities().values();
    let expected_positions = coordinates_array_to_cartesian(&MULTI_POINT_COORDS);
    for (i, entity) in entities.iter().enumerate() {
        assert_eq!(entity.position, Some(expected_positions[i]));
        let billboard = entity.billboard.as_ref().expect("billboard defined");
        assert!(billboard.image.is_some());
        assert_eq!(
            billboard.height_reference,
            HeightReference::ClampToGround as i32
        );
    }
}

// ============================================================================
// it("Works with lineString geometry")
// ============================================================================
#[test]
fn works_with_linestring_geometry() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    data_source.load_value(&line_string(), &no_options()).unwrap();
    let entity = &data_source.entities().values()[0];
    let polyline = entity.polyline.as_ref().expect("polyline defined");
    assert_eq!(
        polyline.positions,
        coordinates_array_to_cartesian(&[[100.0, 0.0], [101.0, 1.0]])
    );
    assert_eq!(polyline.material_color, defaults().stroke);
    assert_eq!(polyline.width, 2.0);
}

// ============================================================================
// it("Works with lineString geometry clamped to ground")
// ============================================================================
#[test]
fn works_with_linestring_geometry_clamped_to_ground() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    let options = GeoJsonLoadOptions {
        clamp_to_ground: Some(true),
        ..Default::default()
    };
    data_source.load_value(&line_string(), &options).unwrap();
    let entity = &data_source.entities().values()[0];
    let polyline = entity.polyline.as_ref().expect("polyline defined");
    assert_eq!(
        polyline.positions,
        coordinates_array_to_cartesian(&[[100.0, 0.0], [101.0, 1.0]])
    );
    assert_eq!(polyline.material_color, defaults().stroke);
    assert_eq!(polyline.width, 2.0);
    assert!(polyline.clamp_to_ground);
}

// ============================================================================
// it("Works with multiLineString geometry")
// ============================================================================
#[test]
fn works_with_multilinestring_geometry() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    data_source
        .load_value(&multi_line_string(), &no_options())
        .unwrap();
    let entities = data_source.entities().values();
    let lines = multi_line_to_cartesian(&multi_line_string());
    for (i, entity) in entities.iter().enumerate() {
        let polyline = entity.polyline.as_ref().expect("polyline defined");
        assert_eq!(polyline.positions, lines[i]);
        assert_eq!(polyline.material_color, Color::YELLOW);
        assert_eq!(polyline.width, 2.0);
    }
}

// ============================================================================
// it("Works with multiLineString geometry clamped to ground")
// ============================================================================
#[test]
fn works_with_multilinestring_geometry_clamped_to_ground() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    let options = GeoJsonLoadOptions {
        clamp_to_ground: Some(true),
        ..Default::default()
    };
    data_source
        .load_value(&multi_line_string(), &options)
        .unwrap();
    let entities = data_source.entities().values();
    let lines = multi_line_to_cartesian(&multi_line_string());
    for (i, entity) in entities.iter().enumerate() {
        let polyline = entity.polyline.as_ref().expect("polyline defined");
        assert_eq!(polyline.positions, lines[i]);
        assert_eq!(polyline.material_color, Color::YELLOW);
        assert_eq!(polyline.width, 2.0);
        assert!(polyline.clamp_to_ground);
    }
}

// ============================================================================
// it("Works with polygon geometry")
// ============================================================================
#[test]
fn works_with_polygon_geometry() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    data_source.load_value(&polygon(), &no_options()).unwrap();
    let entity = &data_source.entities().values()[0];
    let polygon = entity.polygon.as_ref().expect("polygon defined");
    assert_eq!(
        polygon.hierarchy,
        coordinates_array_to_cartesian(&[
            [100.0, 0.0],
            [101.0, 0.0],
            [101.0, 1.0],
            [100.0, 1.0],
            [100.0, 0.0]
        ])
    );
    assert_eq!(polygon.per_position_height, None);
    assert_eq!(polygon.material_color, defaults().fill);
    assert!(polygon.outline);
    assert_eq!(polygon.outline_width, defaults().stroke_width);
    assert_eq!(polygon.outline_color, defaults().stroke);
    // JS asserts `height` is a ConstantProperty; here it is Some(0.0).
    assert_eq!(polygon.height, Some(0.0));
}

// ============================================================================
// it("Works with polygon geometry clamped to ground")
// ============================================================================
#[test]
fn works_with_polygon_geometry_clamped_to_ground() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    let options = GeoJsonLoadOptions {
        clamp_to_ground: Some(true),
        ..Default::default()
    };
    data_source.load_value(&polygon(), &options).unwrap();
    let entity = &data_source.entities().values()[0];
    let polygon = entity.polygon.as_ref().expect("polygon defined");
    assert_eq!(
        polygon.hierarchy,
        coordinates_array_to_cartesian(&[
            [100.0, 0.0],
            [101.0, 0.0],
            [101.0, 1.0],
            [100.0, 1.0],
            [100.0, 0.0]
        ])
    );
    assert_eq!(polygon.per_position_height, None);
    assert_eq!(polygon.material_color, defaults().fill);
    assert!(polygon.outline);
    assert_eq!(polygon.outline_width, defaults().stroke_width);
    assert_eq!(polygon.outline_color, defaults().stroke);
    assert_eq!(polygon.height, None);
}

// ============================================================================
// it("Works with polygon geometry with Heights")
// ============================================================================
#[test]
fn works_with_polygon_geometry_with_heights() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    data_source
        .load_value(&polygon_with_heights(), &no_options())
        .unwrap();
    let entity = &data_source.entities().values()[0];
    let polygon = entity.polygon.as_ref().expect("polygon defined");
    // Mirror of `polygonCoordinatesToCartesian(coordinates[0])`: the JS
    // helper forwards `coordinates[2]` (the height) to `fromDegrees`.
    let expected: Vec<Cartesian3> = [
        [100.0, 0.0, 1.0],
        [101.0, 0.0, 2.0],
        [101.0, 1.0, 1.0],
        [100.0, 1.0, 2.0],
        [100.0, 0.0, 3.0],
    ]
    .iter()
    .map(|c| coordinates_to_cartesian(c))
    .collect();
    assert_eq!(polygon.hierarchy, expected);
    assert_eq!(polygon.per_position_height, Some(true));
    assert_eq!(polygon.material_color, defaults().fill);
    assert!(polygon.outline);
    assert_eq!(polygon.outline_width, defaults().stroke_width);
    assert_eq!(polygon.outline_color, defaults().stroke);
}

// ============================================================================
// it("Works with polygon geometry with holes")
// ============================================================================
#[test]
fn works_with_polygon_geometry_with_holes() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    data_source
        .load_value(&polygon_with_holes(), &no_options())
        .unwrap();
    let entity = &data_source.entities().values()[0];
    let polygon = entity.polygon.as_ref().expect("polygon defined");
    assert_eq!(
        polygon.hierarchy,
        coordinates_array_to_cartesian(&[
            [100.0, 0.0],
            [101.0, 0.0],
            [101.0, 1.0],
            [100.0, 1.0],
            [100.0, 0.0]
        ])
    );
    assert_eq!(
        polygon.holes,
        vec![coordinates_array_to_cartesian(&[
            [100.2, 0.2],
            [100.8, 0.2],
            [100.8, 0.8],
            [100.2, 0.8],
            [100.2, 0.2]
        ])]
    );
}

// ============================================================================
// it("Works with multiPolygon geometry")
// ============================================================================
#[test]
fn works_with_multipolygon_geometry() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    data_source.load_value(&multi_polygon(), &no_options()).unwrap();
    let entities = data_source.entities().values();
    let positions = [
        coordinates_array_to_cartesian(&[
            [102.0, 2.0],
            [103.0, 2.0],
            [103.0, 3.0],
            [102.0, 3.0],
            [102.0, 2.0]
        ]),
        coordinates_array_to_cartesian(&[
            [100.0, 0.0],
            [101.0, 0.0],
            [101.0, 1.0],
            [100.0, 1.0],
            [100.0, 0.0]
        ]),
    ];
    for (i, entity) in entities.iter().enumerate() {
        let polygon = entity.polygon.as_ref().expect("polygon defined");
        assert_eq!(polygon.hierarchy, positions[i]);
    }
}

// ============================================================================
// it("Works with topojson geometry")
// ============================================================================
#[test]
fn works_with_topojson_geometry() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    data_source.load_value(&topo_json(), &no_options()).unwrap();
    let entities = data_source.entities().values();

    let polygon_entity = &entities[0];
    match polygon_entity.properties.get("myProps") {
        Some(cesium_data_sources::property::PropertyResult::Number(v)) => {
            assert_eq!(*v, 0.0);
        }
        other => panic!("expected myProps == 0, got {:?}", other),
    }
    assert!(polygon_entity.polygon.is_some());

    let line_string_entity = &entities[1];
    match line_string_entity.properties.get("myProps") {
        Some(cesium_data_sources::property::PropertyResult::Number(v)) => {
            assert_eq!(*v, 1.0);
        }
        other => panic!("expected myProps == 1, got {:?}", other),
    }
    assert!(line_string_entity.polyline.is_some());
}

// ============================================================================
// it("Can provide base styling options")
// ============================================================================
#[test]
fn can_provide_base_styling_options() {
    let _g = guard();
    let options = GeoJsonLoadOptions {
        marker_size: Some(10.0),
        marker_symbol: Some("bus".to_string()),
        marker_color: Some(Color::GREEN),
        stroke: Some(Color::ORANGE),
        stroke_width: Some(8.0),
        fill: Some(Color::RED),
        ..Default::default()
    };

    let mut data_source = GeoJsonDataSource::new();
    data_source
        .load_value(&mixed_geometries(), &options)
        .unwrap();
    let entities = data_source.entities().values();

    let polyline = entities[0].polyline.as_ref().expect("polyline defined");
    assert_eq!(polyline.material_color, Color::ORANGE);
    assert_eq!(polyline.width, 8.0);

    let polygon = entities[1].polygon.as_ref().expect("polygon defined");
    assert_eq!(polygon.material_color, Color::RED);
    assert_eq!(polygon.outline_color, Color::ORANGE);
    assert_eq!(polygon.outline_width, 8.0);

    let billboard = entities[2].billboard.as_ref().expect("billboard defined");
    // DEVIATION: the JS spec compares the rendered maki pin canvas.
    let expected = pin_from_maki_icon_id("bus", Color::GREEN, 10.0).unwrap();
    assert_eq!(billboard.image.as_deref(), Some(expected.as_str()));
}

// ============================================================================
// it("Can set default graphics")
// ============================================================================
#[test]
fn can_set_default_graphics() {
    let _g = guard();
    set_defaults(|d| {
        d.marker_size = 10.0;
        d.marker_symbol = Some("bus".to_string());
        d.marker_color = Color::GREEN;
        d.stroke = Color::ORANGE;
        d.stroke_width = 8.0;
        d.fill = Color::RED;
    });

    let mut data_source = GeoJsonDataSource::new();
    data_source
        .load_value(&mixed_geometries(), &no_options())
        .unwrap();
    let entities = data_source.entities().values();

    let polyline = entities[0].polyline.as_ref().expect("polyline defined");
    assert_eq!(polyline.material_color, Color::ORANGE);
    assert_eq!(polyline.width, 8.0);

    let polygon = entities[1].polygon.as_ref().expect("polygon defined");
    assert_eq!(polygon.material_color, Color::RED);
    assert_eq!(polygon.outline_color, Color::ORANGE);
    assert_eq!(polygon.outline_width, 8.0);

    let billboard = entities[2].billboard.as_ref().expect("billboard defined");
    let expected = pin_from_maki_icon_id("bus", Color::GREEN, 10.0).unwrap();
    assert_eq!(billboard.image.as_deref(), Some(expected.as_str()));
}

// ============================================================================
// it("Generates description")
// ============================================================================
#[test]
fn generates_description() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    data_source.load_value(&topo_json(), &no_options()).unwrap();
    let entities = data_source.entities().values();
    let polygon_entity = &entities[0];
    assert!(polygon_entity.description.is_some());
}

// ============================================================================
// it("Works with geometrycollection")
// ============================================================================
#[test]
fn works_with_geometrycollection() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    data_source
        .load_value(&geometry_collection(), &no_options())
        .unwrap();
    let entities = data_source.entities().values();

    let entity = &entities[0];
    assert_eq!(
        entity.position,
        Some(coordinates_to_cartesian(&[100.0, 0.0]))
    );
    assert!(entity.billboard.is_some());

    let entity = &entities[1];
    let polyline = entity.polyline.as_ref().expect("polyline defined");
    assert_eq!(
        polyline.positions,
        coordinates_array_to_cartesian(&[[101.0, 0.0], [102.0, 1.0]])
    );
}

// ============================================================================
// it("Works with named crs")
// ============================================================================
#[test]
fn works_with_named_crs() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    data_source
        .load_value(&point_named_crs(), &no_options())
        .unwrap();
    let entity = &data_source.entities().values()[0];
    assert_eq!(
        entity.position,
        Some(coordinates_to_cartesian(&[102.0, 0.5]))
    );
}

// ============================================================================
// it("Works with named crs OGC:1.3:CRS84")
// ============================================================================
#[test]
fn works_with_named_crs_ogc_1_3_crs84() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    data_source
        .load_value(&point_named_crs_ogc(), &no_options())
        .unwrap();
    let entity = &data_source.entities().values()[0];
    assert_eq!(
        entity.position,
        Some(coordinates_to_cartesian(&[102.0, 0.5]))
    );
}

// ============================================================================
// it("Works with named crs EPSG::4326")
// ============================================================================
#[test]
fn works_with_named_crs_epsg_4326() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    data_source
        .load_value(&point_named_crs_epsg(), &no_options())
        .unwrap();
    let entity = &data_source.entities().values()[0];
    assert_eq!(
        entity.position,
        Some(coordinates_to_cartesian(&[102.0, 0.5]))
    );
}

// ============================================================================
// it("Works with link crs href")
// ============================================================================
#[test]
fn works_with_link_crs_href() {
    let _g = guard();
    let projected_position = Cartesian3::new(1.0, 2.0, 3.0);

    let received_properties = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let received = received_properties.clone();
    register_crs_link_href(
        "http://crs.invalid",
        Arc::new(move |properties: &Value| {
            // Mirrors the JS spy asserting the crs properties are passed in.
            received.store(
                properties.get("href").is_some(),
                std::sync::atomic::Ordering::SeqCst,
            );
            let crs_function: CrsFunction =
                Arc::new(move |_coordinate: &[f64]| projected_position);
            Ok(crs_function)
        }),
    );

    let mut data_source = GeoJsonDataSource::new();
    let result = data_source.load_value(&point_crs_link_href(), &no_options());
    unregister_crs_link_href("http://crs.invalid");
    result.unwrap();

    assert!(received_properties.load(std::sync::atomic::Ordering::SeqCst));
    let entity = &data_source.entities().values()[0];
    assert_eq!(entity.position, Some(projected_position));
}

// ============================================================================
// it("Works with EPSG crs")
// ============================================================================
#[test]
fn works_with_epsg_crs() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    data_source
        .load_value(&point_crs_epsg(), &no_options())
        .unwrap();
    let entity = &data_source.entities().values()[0];
    assert_eq!(
        entity.position,
        Some(coordinates_to_cartesian(&[102.0, 0.5]))
    );
}

// ============================================================================
// it("Works with polyline using simplestyle")
// ============================================================================
#[test]
fn works_with_polyline_using_simplestyle() {
    let _g = guard();
    let geo_json = json!({
        "type": "Feature",
        "geometry": {
            "type": "LineString",
            "coordinates": [[100.0, 0.0], [101.0, 1.0]],
        },
        "properties": {
            "title": "textMarker",
            "description": "My description",
            "stroke": "#aabbcc",
            "stroke-opacity": 0.5,
            "stroke-width": 5,
        },
    });

    let mut data_source = GeoJsonDataSource::new();
    data_source.load_value(&geo_json, &no_options()).unwrap();
    let entity = &data_source.entities().values()[0];
    assert_eq!(entity.name, Some("textMarker".to_string()));
    assert_eq!(entity.description, Some("My description".to_string()));

    let polyline = entity.polyline.as_ref().expect("polyline defined");
    let expected_color = Color::from_css_color_string("#aabbcc")
        .expect("valid css color")
        .with_alpha(0.5);
    assert_eq!(polyline.material_color, expected_color);
    assert_eq!(polyline.width, 5.0);
}

// ============================================================================
// it("Works with polyline using null simplestyle values")
// ============================================================================
#[test]
fn works_with_polyline_using_null_simplestyle_values() {
    let _g = guard();
    let geo_json = json!({
        "type": "Feature",
        "geometry": {
            "type": "LineString",
            "coordinates": [[100.0, 0.0], [101.0, 1.0]],
        },
        "properties": {
            "title": null,
            "description": null,
            "stroke": null,
            "stroke-opacity": null,
            "stroke-width": null,
        },
    });

    let mut data_source = GeoJsonDataSource::new();
    data_source.load_value(&geo_json, &no_options()).unwrap();
    let entity = &data_source.entities().values()[0];
    assert_eq!(entity.name, None);
    assert_eq!(entity.description, None);

    let defaults = defaults();
    let polyline = entity.polyline.as_ref().expect("polyline defined");
    assert_eq!(polyline.material_color, defaults.stroke);
    assert_eq!(polyline.width, defaults.stroke_width);
}

// ============================================================================
// it("Works with polyline using null simplestyle values but with opacity")
// ============================================================================
#[test]
fn works_with_polyline_using_null_simplestyle_values_but_with_opacity() {
    let _g = guard();
    let geo_json = json!({
        "type": "Feature",
        "geometry": {
            "type": "LineString",
            "coordinates": [[100.0, 0.0], [101.0, 1.0]],
        },
        "properties": {
            "title": null,
            "description": null,
            "stroke": null,
            "stroke-opacity": 0.42,
            "stroke-width": null,
        },
    });

    let mut data_source = GeoJsonDataSource::new();
    data_source.load_value(&geo_json, &no_options()).unwrap();
    let entity = &data_source.entities().values()[0];
    assert_eq!(entity.name, None);
    assert_eq!(entity.description, None);

    let defaults = defaults();
    let polyline = entity.polyline.as_ref().expect("polyline defined");
    assert_eq!(
        polyline.material_color,
        defaults.stroke.with_alpha(0.42)
    );
    assert_eq!(polyline.width, defaults.stroke_width);
}

// ============================================================================
// it("Works with polygon using simplestyle")
// ============================================================================
#[test]
fn works_with_polygon_using_simplestyle() {
    let _g = guard();
    let geo_json = json!({
        "type": "Feature",
        "geometry": {
            "type": "Polygon",
            "coordinates": [[
                [100.0, 0.0], [101.0, 0.0], [101.0, 1.0], [100.0, 1.0], [100.0, 0.0],
            ]],
        },
        "properties": {
            "title": "textMarker",
            "description": "My description",
            "stroke": "#aabbcc",
            "stroke-opacity": 0.5,
            "stroke-width": 5,
            "fill": "#ccaabb",
            "fill-opacity": 0.25,
        },
    });

    let mut data_source = GeoJsonDataSource::new();
    data_source.load_value(&geo_json, &no_options()).unwrap();
    let entity = &data_source.entities().values()[0];
    assert_eq!(entity.name, Some("textMarker".to_string()));
    assert_eq!(entity.description, Some("My description".to_string()));

    let polygon = entity.polygon.as_ref().expect("polygon defined");
    let expected_fill = Color::from_css_color_string("#ccaabb")
        .expect("valid css color")
        .with_alpha(0.25);
    let expected_outline_color = Color::from_css_color_string("#aabbcc")
        .expect("valid css color")
        .with_alpha(0.5);
    assert_eq!(polygon.material_color, expected_fill);
    assert!(polygon.outline);
    assert_eq!(polygon.outline_width, 5.0);
    assert_eq!(polygon.outline_color, expected_outline_color);
}

// ============================================================================
// it("Works with polygon using null simplestyle")
// ============================================================================
#[test]
fn works_with_polygon_using_null_simplestyle() {
    let _g = guard();
    let geo_json = json!({
        "type": "Feature",
        "geometry": {
            "type": "Polygon",
            "coordinates": [[
                [100.0, 0.0], [101.0, 0.0], [101.0, 1.0], [100.0, 1.0], [100.0, 0.0],
            ]],
        },
        "properties": {
            "title": null,
            "description": null,
            "stroke": null,
            "stroke-opacity": null,
            "stroke-width": null,
            "fill": null,
            "fill-opacity": null,
        },
    });

    let mut data_source = GeoJsonDataSource::new();
    data_source.load_value(&geo_json, &no_options()).unwrap();
    let entity = &data_source.entities().values()[0];
    assert_eq!(entity.name, None);
    assert_eq!(entity.description, None);

    let defaults = defaults();
    let polygon = entity.polygon.as_ref().expect("polygon defined");
    assert_eq!(polygon.material_color, defaults.fill);
    assert!(polygon.outline);
    assert_eq!(polygon.outline_width, defaults.stroke_width);
    assert_eq!(polygon.outline_color, defaults.stroke);
}

// ============================================================================
// it("Works with polygons using null simplestyle but with an opacity")
// ============================================================================
#[test]
fn works_with_polygons_using_null_simplestyle_but_with_an_opacity() {
    let _g = guard();
    let geo_json = json!({
        "type": "Feature",
        "geometry": {
            "type": "Polygon",
            "coordinates": [[
                [100.0, 0.0], [101.0, 0.0], [101.0, 1.0], [100.0, 1.0], [100.0, 0.0],
            ]],
        },
        "properties": {
            "title": null,
            "description": null,
            "stroke": null,
            "stroke-opacity": 0.42,
            "stroke-width": null,
            "fill": null,
            "fill-opacity": 0.42,
        },
    });

    let mut data_source = GeoJsonDataSource::new();
    data_source.load_value(&geo_json, &no_options()).unwrap();
    let entity = &data_source.entities().values()[0];
    assert_eq!(entity.name, None);
    assert_eq!(entity.description, None);

    let defaults = defaults();
    let polygon = entity.polygon.as_ref().expect("polygon defined");
    assert_eq!(polygon.material_color, defaults.fill.with_alpha(0.42));
    assert!(polygon.outline);
    assert_eq!(polygon.outline_width, defaults.stroke_width);
    assert_eq!(polygon.outline_color, defaults.stroke.with_alpha(0.42));
}

// ============================================================================
// it("load works with a URL")
// DEVIATION: the browser URL is mirrored by a file path into the shared
// `Specs/Data` folder; `name` derives from the file name the same way.
// ============================================================================
#[test]
fn load_works_with_a_url() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    let path = data_path("test.geojson");
    data_source
        .load_file(&path.to_string_lossy(), &no_options())
        .unwrap();
    assert_eq!(data_source.display_name(), Some("test.geojson"));
}

// ============================================================================
// it("Fails when encountering unknown geometry")
// ============================================================================
#[test]
fn fails_when_encountering_unknown_geometry() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    assert!(data_source
        .load_value(&feature_unknown_geometry(), &no_options())
        .is_err());
}

// ============================================================================
// it("Fails with undefined geometry")
// ============================================================================
#[test]
fn fails_with_undefined_geometry() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    let result = data_source.load_value(&feature_undefined_geometry(), &no_options());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("feature.geometry is required."));
}

// ============================================================================
// it("Fails with unknown geomeetry in geometryCollection")
// ============================================================================
#[test]
fn fails_with_unknown_geometry_in_geometry_collection() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    assert!(data_source
        .load_value(&geometry_collection_unknown_type(), &no_options())
        .is_err());
}

// ============================================================================
// it("rejects unknown geometry")
// ============================================================================
#[test]
fn rejects_unknown_geometry() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    let result = data_source.load_value(&unknown_geometry(), &no_options());
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("Unsupported GeoJSON object type: TimeyWimey"));
}

// ============================================================================
// it("rejects invalid url")
// DEVIATION: the 404 rejection is mirrored by an IO error for the missing
// file.
// ============================================================================
#[test]
fn rejects_invalid_url() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    assert!(data_source
        .load_file("invalid.geojson", &no_options())
        .is_err());
}

// ============================================================================
// it("rejects null CRS")
// ============================================================================
#[test]
fn rejects_null_crs() {
    let _g = guard();
    let geo_json = json!({
        "type": "Feature",
        "geometry": point(),
        "crs": null,
    });

    let mut data_source = GeoJsonDataSource::new();
    data_source.load_value(&geo_json, &no_options()).unwrap();
    assert_eq!(data_source.entities().values().len(), 0);
}

// ============================================================================
// it("rejects unknown CRS")
// ============================================================================
#[test]
fn rejects_unknown_crs() {
    let _g = guard();
    let geo_json = json!({
        "type": "Feature",
        "geometry": point(),
        "crs": { "type": "potato", "properties": {} },
    });

    let mut data_source = GeoJsonDataSource::new();
    let result = data_source.load_value(&geo_json, &no_options());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unknown crs type: potato"));
}

// ============================================================================
// it("rejects undefined CRS properties")
// ============================================================================
#[test]
fn rejects_undefined_crs_properties() {
    let _g = guard();
    let geo_json = json!({
        "type": "Feature",
        "geometry": point(),
        "crs": { "type": "name" },
    });

    let mut data_source = GeoJsonDataSource::new();
    let result = data_source.load_value(&geo_json, &no_options());
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("crs.properties is undefined."));
}

// ============================================================================
// it("rejects unknown CRS name")
// ============================================================================
#[test]
fn rejects_unknown_crs_name() {
    let _g = guard();
    let geo_json = json!({
        "type": "Feature",
        "geometry": point(),
        "crs": { "type": "name", "properties": { "name": "failMe" } },
    });

    let mut data_source = GeoJsonDataSource::new();
    let result = data_source.load_value(&geo_json, &no_options());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unknown crs name: failMe"));
}

// ============================================================================
// it("rejects unknown CRS link")
// ============================================================================
#[test]
fn rejects_unknown_crs_link() {
    let _g = guard();
    let geo_json = json!({
        "type": "Feature",
        "geometry": point(),
        "crs": {
            "type": "link",
            "properties": { "href": "failMe", "type": "failMeTwice" },
        },
    });

    let mut data_source = GeoJsonDataSource::new();
    let result = data_source.load_value(&geo_json, &no_options());
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("Unable to resolve crs link: {\"href\":\"failMe\",\"type\":\"failMeTwice\"}"));
}

// ============================================================================
// it("load rejects loading non json file")
// DEVIATION: the jasmine spy is mirrored by an invocation counter; the
// `(dataSource, error)` payload of the JS event is flattened to `()`.
// ============================================================================
#[test]
fn load_rejects_loading_non_json_file() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    let count = std::rc::Rc::new(std::cell::Cell::new(0u32));
    let counter = count.clone();
    let _remove = data_source
        .error_event()
        .add_listener(move |_a: &()| counter.set(counter.get() + 1));

    // Blue.png is not JSON.
    let path = data_path("Images/Blue.png");
    assert!(data_source
        .load_file(&path.to_string_lossy(), &no_options())
        .is_err());
    assert_eq!(count.get(), 1);
}

// ============================================================================
// it("load raises loading event")
// DEVIATION: the JS loadingEvent carries `(dataSource, isLoading)`; the
// Rust event payload is `()`, so the spec is mirrored by counting the two
// transitions (true then false) and asserting the final loading state.
// ============================================================================
#[test]
fn load_raises_loading_event() {
    let _g = guard();
    let mut data_source = GeoJsonDataSource::new();
    let count = std::rc::Rc::new(std::cell::Cell::new(0u32));
    let counter = count.clone();
    let _remove = data_source
        .loading_event()
        .add_listener(move |_a: &()| counter.set(counter.get() + 1));

    let path = data_path("test.geojson");
    data_source
        .load_file(&path.to_string_lossy(), &no_options())
        .unwrap();
    // One raise for `true`, one raise for `false`.
    assert_eq!(count.get(), 2);
    assert!(!data_source.is_loading());
}

// ============================================================================
// Supplementary unit test of `defaultDescribe`: it must skip the
// simplestyle identifiers and the property used as the entity name
// (the assertions spread over the "Creates default description..." specs).
// ============================================================================
#[test]
fn default_describe_skips_simplestyle_and_name_properties() {
    let _g = guard();
    let properties = json!({
        "title": "My title",
        "attribute": "My attribute",
        "marker-size": "large",
        "marker-symbol": "bus",
        "marker-color": "#aabbcc",
        "stroke": "#aabbcc",
        "stroke-opacity": 0.5,
        "stroke-width": 5,
        "fill": "#aabbcc",
        "fill-opacity": 0.5,
        "nested": { "inner": "value" },
        "nullProperty": null,
    });

    let html = default_describe(&properties, Some("title"));

    // The name property and all simplestyle identifiers are skipped.
    assert!(!html.contains("title"));
    assert!(!html.contains("marker-size"));
    assert!(!html.contains("marker-symbol"));
    assert!(!html.contains("marker-color"));
    assert!(!html.contains("stroke"));
    assert!(!html.contains("fill"));
    // Null values are skipped, regular ones kept.
    assert!(!html.contains("nullProperty"));
    assert!(html.contains("attribute"));
    assert!(html.contains("My attribute"));
    assert!(html.contains("nested"));
    assert!(html.contains("inner"));
}
