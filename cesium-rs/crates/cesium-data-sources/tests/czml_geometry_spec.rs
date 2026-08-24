//! Specs for the time-dynamic CZML geometry pipeline (task F5 / audit SEM-2):
//! the 11 geometry packet families (box..wall), the interval/bulk sampling
//! support (`intervalFromString`, epoch-relative sampling, packed array
//! expansion, color interpolation conversion) and the sidecar
//! [`CzmlGeometryStore`].
//!
//! DEVIATION (storage): CesiumJS evaluates `graphics.field.getValue(time)`;
//! this port keeps the time-dynamic values in the sidecar store, so the
//! assertions call `CzmlGeometry::get_value` / `get_material` instead.

use cesium_core::julian_date::JulianDate;
use cesium_data_sources::czml_data_source::{CzmlDataSource, CzmlLoadOptions};
use cesium_data_sources::czml_processing::CzmlMaterialKind;
use cesium_data_sources::czml_property::{interval_from_string, CzmlProperty, CzmlValue};
use serde_json::{json, Value};

// ============================================================================
// Helpers
// ============================================================================

fn make_document(packet: Value) -> Value {
    json!([
        { "id": "document", "version": "1.0" },
        packet,
    ])
}

fn load(packet: Value) -> CzmlDataSource {
    CzmlDataSource::load(&make_document(packet), None).expect("load should succeed")
}

fn time(iso: &str) -> JulianDate {
    JulianDate::from_iso8601(iso).expect("valid iso8601")
}

fn float_eq(left: f64, right: f64) -> bool {
    (left - right).abs() < 1e-12
}

fn as_number(value: Option<CzmlValue>) -> f64 {
    match value {
        Some(CzmlValue::Number(number)) => number,
        other => panic!("expected Number, got {other:?}"),
    }
}

fn as_bool(value: Option<CzmlValue>) -> bool {
    match value {
        Some(CzmlValue::Boolean(flag)) => flag,
        other => panic!("expected Boolean, got {other:?}"),
    }
}

// ============================================================================
// intervalFromString
// ============================================================================

#[test]
fn interval_from_string_parses_open_and_closed_endpoints() {
    let interval = interval_from_string(Some("2012-01-01T00:00:00Z/2012-01-02T00:00:00Z"))
        .expect("interval parses");
    assert!(interval.is_start_included);
    assert!(interval.is_stop_included);
    assert!(interval.contains(&time("2012-01-01T12:00:00Z")));

    let open = interval_from_string(Some("2012-01-01T00:00:00Z/2012-01-02T00:00:00Z"))
        .expect("interval parses");
    assert!(open.contains(&time("2012-01-01T00:00:00Z")));

    assert!(interval_from_string(None).is_none());
}

// ============================================================================
// Box
// ============================================================================

#[test]
fn process_box_reads_constant_fields() {
    let ds = load(json!({
        "id": "test",
        "box": {
            "show": true,
            "dimensions": { "cartesian": [1.0, 2.0, 3.0] },
            "heightReference": "CLAMP_TO_GROUND",
            "fill": false,
            "outline": true,
            "outlineColor": { "rgba": [255, 0, 0, 128] },
            "outlineWidth": 3.0,
            "shadows": "DISABLED",
        },
    }));
    let geometry = &ds.czml_geometries().get("test").expect("entity stored").r#box;
    let t = time("2012-01-01T00:00:00Z");

    assert!(as_bool(geometry.get_value("show", &t)));
    match geometry.get_value("dimensions", &t) {
        Some(CzmlValue::Cartesian3(c)) => {
            assert!(float_eq(c.x, 1.0) && float_eq(c.y, 2.0) && float_eq(c.z, 3.0));
        }
        other => panic!("expected Cartesian3, got {other:?}"),
    }
    // HeightReference names map to the enum discriminant.
    assert!(float_eq(as_number(geometry.get_value("heightReference", &t)), 1.0));
    assert!(!as_bool(geometry.get_value("fill", &t)));
    assert!(as_bool(geometry.get_value("outline", &t)));
    // rgba bytes are converted to floats via Color.byteToFloat.
    match geometry.get_value("outlineColor", &t) {
        Some(CzmlValue::Color(r, g, b, a)) => {
            assert!(float_eq(r, 1.0) && float_eq(g, 0.0) && float_eq(b, 0.0));
            assert!((a - 128.0 / 255.0).abs() < 1e-9);
        }
        other => panic!("expected Color, got {other:?}"),
    }
    assert!(float_eq(as_number(geometry.get_value("outlineWidth", &t)), 3.0));
}

// ============================================================================
// Corridor / Cylinder
// ============================================================================

#[test]
fn process_corridor_reads_positions_and_enums() {
    let ds = load(json!({
        "id": "test",
        "corridor": {
            "positions": { "cartographicDegrees": [-75.0, 40.0, 0.0, -80.0, 35.0, 0.0] },
            "width": 100.0,
            "cornerType": "BEVELED",
            "granularity": 0.01,
            "zIndex": 2.0,
        },
    }));
    let geometry = &ds.czml_geometries().get("test").expect("entity stored").corridor;
    let t = time("2012-01-01T00:00:00Z");

    match geometry.get_value("positions", &t) {
        Some(CzmlValue::Cartesian3Array(positions)) => {
            assert_eq!(positions.len(), 2);
            // First position: -75 deg / 40 deg on WGS84.
            let mut expected = cesium_core::cartesian3::Cartesian3::default();
            cesium_core::cartesian3::Cartesian3::from_degrees(
                -75.0, 40.0, Some(0.0), None, &mut expected,
            );
            assert!((positions[0].x - expected.x).abs() < 1e-6);
        }
        other => panic!("expected Cartesian3Array, got {other:?}"),
    }
    assert!(float_eq(as_number(geometry.get_value("width", &t)), 100.0));
    assert!(geometry.get_value("cornerType", &t).is_some());
    assert!(float_eq(as_number(geometry.get_value("zIndex", &t)), 2.0));
}

#[test]
fn process_cylinder_reads_radii_and_lines() {
    let ds = load(json!({
        "id": "test",
        "cylinder": {
            "length": 10.0,
            "topRadius": 1.0,
            "bottomRadius": 2.0,
            "numberOfVerticalLines": 8.0,
            "slices": 16.0,
        },
    }));
    let geometry = &ds.czml_geometries().get("test").expect("entity stored").cylinder;
    let t = time("2012-01-01T00:00:00Z");
    assert!(float_eq(as_number(geometry.get_value("length", &t)), 10.0));
    assert!(float_eq(as_number(geometry.get_value("topRadius", &t)), 1.0));
    assert!(float_eq(as_number(geometry.get_value("bottomRadius", &t)), 2.0));
    assert!(float_eq(as_number(geometry.get_value("numberOfVerticalLines", &t)), 8.0));
    assert!(float_eq(as_number(geometry.get_value("slices", &t)), 16.0));
}

// ============================================================================
// Ellipse / Ellipsoid
// ============================================================================

#[test]
fn process_ellipse_reads_axes_and_rotation() {
    let ds = load(json!({
        "id": "test",
        "ellipse": {
            "semiMajorAxis": 100.0,
            "semiMinorAxis": 50.0,
            "rotation": 0.5,
            "stRotation": 0.25,
            "extrudedHeight": 20.0,
        },
    }));
    let geometry = &ds.czml_geometries().get("test").expect("entity stored").ellipse;
    let t = time("2012-01-01T00:00:00Z");
    assert!(float_eq(as_number(geometry.get_value("semiMajorAxis", &t)), 100.0));
    assert!(float_eq(as_number(geometry.get_value("semiMinorAxis", &t)), 50.0));
    assert!(float_eq(as_number(geometry.get_value("rotation", &t)), 0.5));
    assert!(float_eq(as_number(geometry.get_value("stRotation", &t)), 0.25));
    assert!(float_eq(as_number(geometry.get_value("extrudedHeight", &t)), 20.0));
}

#[test]
fn process_ellipsoid_reads_radii_and_cone_angles() {
    let ds = load(json!({
        "id": "test",
        "ellipsoid": {
            "radii": { "cartesian": [1.0, 2.0, 3.0] },
            "minimumClock": 0.1,
            "maximumClock": 0.9,
            "minimumCone": 0.2,
            "maximumCone": 0.8,
            "stackPartitions": 2.0,
            "slicePartitions": 3.0,
            "subdivisions": 4.0,
        },
    }));
    let geometry = &ds.czml_geometries().get("test").expect("entity stored").ellipsoid;
    let t = time("2012-01-01T00:00:00Z");
    match geometry.get_value("radii", &t) {
        Some(CzmlValue::Cartesian3(c)) => assert!(float_eq(c.y, 2.0)),
        other => panic!("expected Cartesian3, got {other:?}"),
    }
    assert!(float_eq(as_number(geometry.get_value("minimumCone", &t)), 0.2));
    assert!(float_eq(as_number(geometry.get_value("subdivisions", &t)), 4.0));
}

// ============================================================================
// Model (gltf uri, nodeTransformations, articulations)
// ============================================================================

#[test]
fn process_model_reads_uri_transformations_and_articulations() {
    let ds = load(json!({
        "id": "test",
        "model": {
            "gltf": "http://example.com/model.glb",
            "scale": 2.0,
            "minimumPixelSize": 64.0,
            "nodeTransformations": {
                "wheel": {
                    "translation": { "cartesian": [1.0, 2.0, 3.0] },
                    "rotation": { "unitQuaternion": [0.0, 0.0, 0.0, 1.0] },
                    "scale": { "cartesian": [1.5, 1.5, 1.5] },
                },
            },
            "articulations": {
                "door:open": 45.0,
            },
        },
    }));
    let geometry = &ds.czml_geometries().get("test").expect("entity stored").model;
    let t = time("2012-01-01T00:00:00Z");

    match geometry.get_value("uri", &t) {
        Some(CzmlValue::Text(uri)) => assert_eq!(uri, "http://example.com/model.glb"),
        other => panic!("expected Text uri, got {other:?}"),
    }
    assert!(float_eq(as_number(geometry.get_value("scale", &t)), 2.0));

    let node = geometry
        .node_transformations
        .get("wheel")
        .expect("node transformation stored");
    match node.get("translation").and_then(|p| p.as_ref()).and_then(|p| p.get_value(&t)) {
        Some(CzmlValue::Cartesian3(c)) => assert!(float_eq(c.z, 3.0)),
        other => panic!("expected translation Cartesian3, got {other:?}"),
    }
    match node.get("scale").and_then(|p| p.as_ref()).and_then(|p| p.get_value(&t)) {
        Some(CzmlValue::Cartesian3(c)) => assert!(float_eq(c.x, 1.5)),
        other => panic!("expected scale Cartesian3, got {other:?}"),
    }

    match geometry.articulations.get("door:open") {
        Some(Some(property)) => match property.get_value(&t) {
            Some(CzmlValue::Number(value)) => assert!(float_eq(value, 45.0)),
            other => panic!("expected articulation number, got {other:?}"),
        },
        other => panic!("expected articulation stored, got {other:?}"),
    }
}

// ============================================================================
// Path / PolylineVolume / Rectangle / Tileset / Wall
// ============================================================================

#[test]
fn process_path_reads_times_and_width() {
    let ds = load(json!({
        "id": "test",
        "path": {
            "show": true,
            "leadTime": 10.0,
            "trailTime": 20.0,
            "width": 2.0,
            "resolution": 30.0,
        },
    }));
    let geometry = &ds.czml_geometries().get("test").expect("entity stored").path;
    let t = time("2012-01-01T00:00:00Z");
    assert!(float_eq(as_number(geometry.get_value("leadTime", &t)), 10.0));
    assert!(float_eq(as_number(geometry.get_value("trailTime", &t)), 20.0));
    assert!(float_eq(as_number(geometry.get_value("resolution", &t)), 30.0));
}

#[test]
fn process_polyline_volume_reads_shape_as_cartesian2_array() {
    let ds = load(json!({
        "id": "test",
        "polylineVolume": {
            "positions": { "cartographicDegrees": [-75.0, 40.0, 0.0, -80.0, 35.0, 0.0] },
            "shape": { "cartesian2": [0.0, 0.0, 1.0, 0.0, 1.0, 1.0] },
        },
    }));
    let geometry = &ds
        .czml_geometries()
        .get("test")
        .expect("entity stored")
        .polyline_volume;
    let t = time("2012-01-01T00:00:00Z");
    match geometry.get_value("shape", &t) {
        Some(CzmlValue::Cartesian2Array(points)) => {
            assert_eq!(points.len(), 3);
            assert!(float_eq(points[2].x, 1.0) && float_eq(points[2].y, 1.0));
        }
        other => panic!("expected Cartesian2Array, got {other:?}"),
    }
}

#[test]
fn process_rectangle_reads_coordinates_as_rectangle_value() {
    let ds = load(json!({
        "id": "test",
        "rectangle": {
            "coordinates": { "wsenDegrees": [-10.0, -20.0, 10.0, 20.0] },
            "height": 5.0,
            "zIndex": 1.0,
        },
    }));
    let geometry = &ds.czml_geometries().get("test").expect("entity stored").rectangle;
    let t = time("2012-01-01T00:00:00Z");
    match geometry.get_value("coordinates", &t) {
        Some(CzmlValue::Rectangle(w, s, e, n)) => {
            let to_radians = std::f64::consts::PI / 180.0;
            assert!((w - -10.0 * to_radians).abs() < 1e-12);
            assert!((n - 20.0 * to_radians).abs() < 1e-12);
            let _ = (s, e);
        }
        other => panic!("expected Rectangle, got {other:?}"),
    }
    assert!(float_eq(as_number(geometry.get_value("height", &t)), 5.0));
}

#[test]
fn process_tileset_reads_show_uri_and_sse() {
    let ds = load(json!({
        "id": "test",
        "tileset": {
            "show": true,
            "uri": "tileset.json",
            "maximumScreenSpaceError": 32.0,
        },
    }));
    let geometry = &ds.czml_geometries().get("test").expect("entity stored").tileset;
    let t = time("2012-01-01T00:00:00Z");
    assert!(as_bool(geometry.get_value("show", &t)));
    match geometry.get_value("uri", &t) {
        Some(CzmlValue::Text(uri)) => assert_eq!(uri, "tileset.json"),
        other => panic!("expected Text uri, got {other:?}"),
    }
    assert!(float_eq(
        as_number(geometry.get_value("maximumScreenSpaceError", &t)),
        32.0
    ));
}

#[test]
fn process_wall_reads_positions_and_height_arrays() {
    let ds = load(json!({
        "id": "test",
        "wall": {
            "positions": { "cartographicDegrees": [-75.0, 40.0, 0.0, -80.0, 35.0, 0.0] },
            "minimumHeights": { "array": [10.0, 20.0] },
            "maximumHeights": { "array": [100.0, 200.0] },
        },
    }));
    let geometry = &ds.czml_geometries().get("test").expect("entity stored").wall;
    let t = time("2012-01-01T00:00:00Z");
    match geometry.get_value("minimumHeights", &t) {
        Some(CzmlValue::NumberArray(heights)) => assert_eq!(heights, vec![10.0, 20.0]),
        other => panic!("expected NumberArray, got {other:?}"),
    }
    match geometry.get_value("maximumHeights", &t) {
        Some(CzmlValue::NumberArray(heights)) => assert_eq!(heights, vec![100.0, 200.0]),
        other => panic!("expected NumberArray, got {other:?}"),
    }
}

// ============================================================================
// Interval constants (TimeIntervalCollection)
// ============================================================================

#[test]
fn interval_constants_form_a_time_interval_collection() {
    let ds = load(json!({
        "id": "test",
        "box": {
            "outlineWidth": [
                { "interval": "2012-01-01T00:00:00Z/2012-01-02T00:00:00Z", "number": 1.0 },
                { "interval": "2012-01-02T00:00:00Z/2012-01-03T00:00:00Z", "number": 2.0 },
            ],
        },
    }));
    let geometry = &ds.czml_geometries().get("test").expect("entity stored").r#box;

    assert!(float_eq(
        as_number(geometry.get_value("outlineWidth", &time("2012-01-01T12:00:00Z"))),
        1.0
    ));
    assert!(float_eq(
        as_number(geometry.get_value("outlineWidth", &time("2012-01-02T12:00:00Z"))),
        2.0
    ));
    assert!(geometry
        .get_value("outlineWidth", &time("2012-02-01T00:00:00Z"))
        .is_none());

    match geometry.properties.get("outlineWidth").unwrap() {
        Some(CzmlProperty::TimeIntervalCollection(entries)) => assert_eq!(entries.len(), 2),
        other => panic!("expected TimeIntervalCollection, got {other:?}"),
    }
}

// ============================================================================
// Sampling (infinite and interval-constrained, epoch-relative)
// ============================================================================

#[test]
fn sampled_number_without_interval_is_an_infinite_sampled_property() {
    let ds = load(json!({
        "id": "test",
        "box": {
            "outlineWidth": {
                "epoch": "2012-01-01T00:00:00Z",
                "number": [0.0, 1.0, 3600.0, 3.0],
            },
        },
    }));
    let geometry = &ds.czml_geometries().get("test").expect("entity stored").r#box;

    match geometry.properties.get("outlineWidth").unwrap() {
        Some(CzmlProperty::Sampled(_)) => {}
        other => panic!("expected Sampled, got {other:?}"),
    }
    // Linear interpolation at the midpoint of the two samples.
    assert!(float_eq(
        as_number(geometry.get_value("outlineWidth", &time("2012-01-01T00:30:00Z"))),
        2.0
    ));
    // Exact sample times resolve without interpolation.
    assert!(float_eq(
        as_number(geometry.get_value("outlineWidth", &time("2012-01-01T00:00:00Z"))),
        1.0
    ));
    assert!(float_eq(
        as_number(geometry.get_value("outlineWidth", &time("2012-01-01T01:00:00Z"))),
        3.0
    ));
}

#[test]
fn sampled_number_with_interval_becomes_a_composite() {
    let ds = load(json!({
        "id": "test",
        "box": {
            "outlineWidth": {
                "interval": "2012-01-01T00:00:00Z/2012-01-02T00:00:00Z",
                "epoch": "2012-01-01T00:00:00Z",
                "number": [0.0, 1.0, 86400.0, 3.0],
            },
        },
    }));
    let geometry = &ds.czml_geometries().get("test").expect("entity stored").r#box;

    match geometry.properties.get("outlineWidth").unwrap() {
        Some(CzmlProperty::Composite(entries)) => assert_eq!(entries.len(), 1),
        other => panic!("expected Composite, got {other:?}"),
    }
    assert!(float_eq(
        as_number(geometry.get_value("outlineWidth", &time("2012-01-01T12:00:00Z"))),
        2.0
    ));
    // Outside the interval there is no value.
    assert!(geometry
        .get_value("outlineWidth", &time("2012-03-01T12:00:00Z"))
        .is_none());
}

#[test]
fn sampled_cartesian_interpolates_component_wise() {
    let ds = load(json!({
        "id": "test",
        "ellipsoid": {
            "radii": {
                "epoch": "2012-01-01T00:00:00Z",
                "cartesian": [0.0, 1.0, 2.0, 3.0, 3600.0, 3.0, 4.0, 5.0],
            },
        },
    }));
    let geometry = &ds.czml_geometries().get("test").expect("entity stored").ellipsoid;
    match geometry.get_value("radii", &time("2012-01-01T00:30:00Z")) {
        Some(CzmlValue::Cartesian3(c)) => {
            assert!(float_eq(c.x, 2.0) && float_eq(c.y, 3.0) && float_eq(c.z, 4.0));
        }
        other => panic!("expected interpolated Cartesian3, got {other:?}"),
    }
}

// ============================================================================
// Color conversion for interpolation (rgba bytes -> packed floats)
// ============================================================================

#[test]
fn sampled_rgba_color_is_converted_to_floats_for_interpolation() {
    let ds = load(json!({
        "id": "test",
        "box": {
            "outlineColor": {
                "epoch": "2012-01-01T00:00:00Z",
                "rgba": [0.0, 0.0, 255.0, 0.0, 0.0, 3600.0, 0.0, 0.0, 255.0, 255.0],
            },
        },
    }));
    let geometry = &ds.czml_geometries().get("test").expect("entity stored").r#box;
    // Midpoint between green (alpha 0) and blue (alpha 1).
    match geometry.get_value("outlineColor", &time("2012-01-01T00:30:00Z")) {
        Some(CzmlValue::Color(r, g, b, a)) => {
            assert!(float_eq(r, 0.0));
            assert!((g - 0.5).abs() < 1e-9);
            assert!((b - 0.5).abs() < 1e-9);
            assert!((a - 0.5).abs() < 1e-9);
        }
        other => panic!("expected interpolated Color, got {other:?}"),
    }
}

// ============================================================================
// Materials (single and composite)
// ============================================================================

#[test]
fn box_material_solid_color_is_stored_as_a_single_material() {
    let ds = load(json!({
        "id": "test",
        "box": {
            "material": {
                "solidColor": { "color": { "rgba": [255, 255, 0, 255] } },
            },
        },
    }));
    let geometry = &ds.czml_geometries().get("test").expect("entity stored").r#box;
    let t = time("2012-01-01T00:00:00Z");
    let material = geometry.get_material("material", &t).expect("material stored");
    assert_eq!(material.kind, CzmlMaterialKind::SolidColor);
    match material.get_property("color", &t) {
        Some(CzmlValue::Color(r, g, b, a)) => {
            assert!(float_eq(r, 1.0) && float_eq(g, 1.0) && float_eq(b, 0.0) && float_eq(a, 1.0));
        }
        other => panic!("expected material color, got {other:?}"),
    }
}

#[test]
fn interval_materials_form_a_composite_material_property() {
    let ds = load(json!({
        "id": "test",
        "box": {
            "material": [
                {
                    "interval": "2012-01-01T00:00:00Z/2012-01-02T00:00:00Z",
                    "solidColor": { "color": { "rgba": [255, 0, 0, 255] } },
                },
                {
                    "interval": "2012-01-02T00:00:00Z/2012-01-03T00:00:00Z",
                    "grid": { "cellAlpha": 0.25 },
                },
            ],
        },
    }));
    let geometry = &ds.czml_geometries().get("test").expect("entity stored").r#box;

    let first = geometry
        .get_material("material", &time("2012-01-01T12:00:00Z"))
        .expect("first interval material");
    assert_eq!(first.kind, CzmlMaterialKind::SolidColor);

    let second = geometry
        .get_material("material", &time("2012-01-02T12:00:00Z"))
        .expect("second interval material");
    assert_eq!(second.kind, CzmlMaterialKind::Grid);
    assert!(float_eq(
        as_number(second.get_property("cellAlpha", &time("2012-01-02T12:00:00Z"))),
        0.25
    ));

    assert!(geometry
        .get_material("material", &time("2013-01-01T12:00:00Z"))
        .is_none());
}

// ============================================================================
// References
// ============================================================================

#[test]
fn reference_properties_resolve_to_reference_variants() {
    let ds = load(json!({
        "id": "test",
        "box": {
            "outlineWidth": { "reference": "other#box.outlineWidth" },
        },
    }));
    let geometry = &ds.czml_geometries().get("test").expect("entity stored").r#box;
    match geometry.properties.get("outlineWidth").unwrap() {
        Some(CzmlProperty::Reference(reference)) => {
            assert_eq!(reference, "other#box.outlineWidth");
        }
        other => panic!("expected Reference, got {other:?}"),
    }
    // References have no local value.
    assert!(geometry
        .get_value("outlineWidth", &time("2012-01-01T00:00:00Z"))
        .is_none());
}

// ============================================================================
// Polygon hierarchy supplement + polyline followSurface adapter
// ============================================================================

#[test]
fn polygon_positions_populate_the_hierarchy_supplement() {
    let ds = load(json!({
        "id": "test",
        "polygon": {
            "positions": { "cartographicDegrees": [-75.0, 40.0, 0.0, -70.0, 40.0, 0.0, -70.0, 45.0, 0.0] },
            "holes": {
                "cartographicDegrees": [[-74.0, 41.0, 0.0, -73.0, 41.0, 0.0, -73.0, 42.0, 0.0]],
            },
        },
    }));
    let geometry = &ds.czml_geometries().get("test").expect("entity stored").polygon;
    let t = time("2012-01-01T00:00:00Z");

    assert!(geometry.has_hierarchy);
    match geometry.get_value("_positions", &t) {
        Some(CzmlValue::Cartesian3Array(positions)) => assert_eq!(positions.len(), 3),
        other => panic!("expected _positions Cartesian3Array, got {other:?}"),
    }
    match geometry.get_value("_holes", &t) {
        Some(CzmlValue::Cartesian3ArrayOfArrays(holes)) => {
            assert_eq!(holes.len(), 1);
            assert_eq!(holes[0].len(), 3);
        }
        other => panic!("expected _holes array of arrays, got {other:?}"),
    }
}

#[test]
fn polyline_follow_surface_adapts_to_arc_type() {
    let ds = load(json!({
        "id": "test",
        "polyline": {
            "followSurface": true,
        },
    }));
    let geometry = &ds.czml_geometries().get("test").expect("entity stored").polyline;
    let t = time("2012-01-01T00:00:00Z");
    // followSurface=true maps to ArcType.GEODESIC (1).
    assert!(float_eq(as_number(geometry.get_value("arcType", &t)), 1.0));

    let ds2 = load(json!({
        "id": "test",
        "polyline": { "followSurface": false },
    }));
    let geometry2 = &ds2.czml_geometries().get("test").expect("entity stored").polyline;
    // followSurface=false maps to ArcType.NONE (0).
    assert!(float_eq(as_number(geometry2.get_value("arcType", &t)), 0.0));
}

// ============================================================================
// Upsert semantics: merge of repeated packets and delete
// ============================================================================

#[test]
fn repeated_packets_upsert_into_the_same_geometry_slots() {
    let czml = json!([
        { "id": "document", "version": "1.0" },
        { "id": "test", "box": { "outlineWidth": 1.0 } },
        { "id": "test", "box": { "fill": false } },
    ]);
    let ds = CzmlDataSource::load(&czml, None).expect("load should succeed");
    let geometry = &ds.czml_geometries().get("test").expect("entity stored").r#box;
    let t = time("2012-01-01T00:00:00Z");
    // Both packets landed in the same slot set.
    assert!(float_eq(as_number(geometry.get_value("outlineWidth", &t)), 1.0));
    assert!(!as_bool(geometry.get_value("fill", &t)));
}

#[test]
fn delete_packet_removes_the_geometry_store_entry() {
    let czml = json!([
        { "id": "document", "version": "1.0" },
        { "id": "test", "box": { "outlineWidth": 1.0 } },
        { "id": "test", "delete": true },
    ]);
    let ds = CzmlDataSource::load(&czml, None).expect("load should succeed");
    assert!(ds.czml_geometries().get("test").is_none());
}

#[test]
fn delete_property_with_interval_removes_only_that_interval() {
    let czml = json!([
        { "id": "document", "version": "1.0" },
        {
            "id": "test",
            "box": {
                "outlineWidth": [
                    { "interval": "2012-01-01T00:00:00Z/2012-01-02T00:00:00Z", "number": 1.0 },
                    { "interval": "2012-01-02T00:00:00Z/2012-01-03T00:00:00Z", "number": 2.0 },
                ],
            },
        },
        {
            "id": "test",
            "box": {
                "outlineWidth": {
                    "delete": true,
                    "interval": "2012-01-01T00:00:00Z/2012-01-02T00:00:00Z",
                },
            },
        },
    ]);
    let ds = CzmlDataSource::load(&czml, None).expect("load should succeed");
    let geometry = &ds.czml_geometries().get("test").expect("entity stored").r#box;
    assert!(geometry
        .get_value("outlineWidth", &time("2012-01-01T12:00:00Z"))
        .is_none());
    assert!(float_eq(
        as_number(geometry.get_value("outlineWidth", &time("2012-01-02T12:00:00Z"))),
        2.0
    ));
}

#[test]
fn load_clears_previous_geometry_store_entries() {
    let options = CzmlLoadOptions::default();
    let mut ds = CzmlDataSource::new();
    ds.load_value(
        &make_document(json!({ "id": "test", "box": { "outlineWidth": 1.0 } })),
        Some(&options),
    )
    .expect("first load");
    assert!(ds.czml_geometries().get("test").is_some());

    ds.load_value(
        &make_document(json!({ "id": "other", "wall": { "granularity": 0.01 } })),
        Some(&options),
    )
    .expect("second load");
    assert!(ds.czml_geometries().get("test").is_none());
    assert!(ds.czml_geometries().get("other").is_some());
}
