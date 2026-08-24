//! Ported specs from `packages/engine/Specs/DataSources/CzmlDataSourceSpec.js`.
//!
//! Every test mirrors one `it()` of the original Jasmine spec; the test
//! names keep the original descriptions snake-cased so they stay mappable.
//!
//! DEVIATION (simplified value model): CesiumJS stores CZML properties as
//! time-dynamic `Property` objects evaluated with `getValue(time)`. This
//! port materializes the constant subset directly, so assertions read the
//! stored values instead of calling `getValue(Iso8601.MINIMUM_VALUE)`.
//! Sampled/interval/reference properties are skipped by the implementation
//! and the corresponding specs are not mirrored.

use std::cell::Cell;
use std::rc::Rc;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::clock_range::ClockRange;
use cesium_core::clock_step::ClockStep;
use cesium_core::color::Color;
use cesium_core::julian_date::JulianDate;
use cesium_core::near_far_scalar::NearFarScalar;
use cesium_core::quaternion::Quaternion;
use cesium_core::time_interval::TimeInterval;
use cesium_data_sources::czml_data_source::{
    CzmlDataSource, CzmlLoadOptions, FIRST_PACKET_ERROR, VERSION_INVALID_ERROR,
};
use cesium_data_sources::data_source::DataSource;
use cesium_data_sources::property::PropertyResult;
use cesium_scene::height_reference::HeightReference;
use cesium_scene::horizontal_origin::HorizontalOrigin;
use cesium_scene::label_style::LabelStyle;
use cesium_scene::vertical_origin::VerticalOrigin;
use cesium_specs::data_path;
use serde_json::{json, Value};

// ============================================================================
// Spec fixtures (mirrors of the top-level constants in the JS spec)
// ============================================================================

/// Mirror of the spec helper `makeDocument`.
fn make_document(packet: Value) -> Value {
    json!([
        { "id": "document", "version": "1.0" },
        packet,
    ])
}

fn static_czml() -> Value {
    json!({
        "id": "test",
        "billboard": { "show": true },
    })
}

fn czml_delete() -> Value {
    json!({
        "id": "test",
        "delete": true,
    })
}

fn dynamic_czml() -> Value {
    json!({
        "id": "test",
        "availability": "2000-01-01/2001-01-01",
        "billboard": { "show": true },
    })
}

fn clock_czml() -> Value {
    json!({
        "id": "document",
        "version": "1.0",
        "clock": {
            "interval": "2012-03-15T10:00:00Z/2012-03-16T10:00:00Z",
            "currentTime": "2012-03-15T10:00:00Z",
            "multiplier": 60.0,
            "range": "LOOP_STOP",
            "step": "SYSTEM_CLOCK_MULTIPLIER",
        },
    })
}

fn clock_czml2() -> Value {
    json!({
        "id": "document",
        "version": "1.0",
        "clock": {
            "interval": "2013-03-15T10:00:00Z/2013-03-16T10:00:00Z",
            "currentTime": "2013-03-15T10:00:00Z",
            "multiplier": 30.0,
            "range": "UNBOUNDED",
            "step": "TICK_DEPENDENT",
        },
    })
}

fn name_czml() -> Value {
    json!({
        "id": "document",
        "version": "1.0",
        "name": "czmlName",
    })
}

/// Counter helper for event assertions (mirrors `jasmine.createSpy`).
fn counter() -> (Rc<Cell<u32>>, impl Fn(&Rc<Cell<u32>>)) {
    let count = Rc::new(Cell::new(0u32));
    let bump = move |c: &Rc<Cell<u32>>| c.set(c.get() + 1);
    (count, bump)
}

fn no_options() -> CzmlLoadOptions {
    CzmlLoadOptions::default()
}

// ============================================================================
// Constructor / name / credit specs
// ============================================================================

#[test]
fn default_constructor_has_expected_values() {
    let data_source = CzmlDataSource::new();
    assert!(data_source.display_name().is_none());
    assert!(data_source.clock().is_none());
    assert_eq!(data_source.entities().values().len(), 0);
    assert!(data_source.show());
    assert!(data_source.credit().is_none());
}

#[test]
fn show_sets_underlying_entity_collection_show() {
    let mut data_source = CzmlDataSource::new();

    data_source.set_show(false);
    assert!(!data_source.show());
    assert_eq!(data_source.show(), data_source.entities().show);

    data_source.set_show(true);
    assert!(data_source.show());
    assert_eq!(data_source.show(), data_source.entities().show);
}

#[test]
fn name_returns_czml_defined_name() {
    let data_source = CzmlDataSource::load(&name_czml(), None).unwrap();
    assert_eq!(data_source.display_name(), Some(name_czml()["name"].as_str().unwrap()));
}

#[test]
fn name_uses_source_name_if_czml_name_is_undefined() {
    let options = CzmlLoadOptions {
        source_uri: Some("Gallery/simple.czml?asd=true".to_string()),
        credit: None,
    };
    let data_source = CzmlDataSource::load(&clock_czml(), Some(&options)).unwrap();
    assert_eq!(data_source.display_name(), Some("simple.czml"));
}

#[test]
fn credit_gets_set_from_options() {
    let options = CzmlLoadOptions {
        source_uri: None,
        credit: Some("This is my credit".to_string()),
    };
    let data_source = CzmlDataSource::load(&name_czml(), Some(&options)).unwrap();
    assert!(data_source.credit().is_some());
}

#[test]
fn does_not_overwrite_existing_name_if_czml_name_is_undefined() {
    let name = "myName";
    let mut data_source = CzmlDataSource::with_name(Some(name));
    let options = CzmlLoadOptions {
        source_uri: Some("Gallery/simple.czml".to_string()),
        credit: None,
    };
    data_source.load_value(&clock_czml(), Some(&options)).unwrap();
    assert_eq!(data_source.display_name(), Some(name));
}

// ============================================================================
// Clock specs
// ============================================================================

#[test]
fn clock_returns_undefined_for_static_czml() {
    let data_source = CzmlDataSource::load(&make_document(static_czml()), None).unwrap();
    assert!(data_source.clock().is_none());
}

#[test]
fn clock_returns_czml_defined_clock() {
    let parsed_interval =
        TimeInterval::from_iso8601("2012-03-15T10:00:00Z/2012-03-16T10:00:00Z", None, None)
            .unwrap();
    let parsed_current = JulianDate::from_iso8601("2012-03-15T10:00:00Z").unwrap();

    let parsed_interval2 =
        TimeInterval::from_iso8601("2013-03-15T10:00:00Z/2013-03-16T10:00:00Z", None, None)
            .unwrap();
    let parsed_current2 = JulianDate::from_iso8601("2013-03-15T10:00:00Z").unwrap();

    let mut data_source = CzmlDataSource::load(&clock_czml(), None).unwrap();
    {
        let clock = data_source.clock().expect("clock should be defined");
        assert!(JulianDate::equals(&clock.start_time, &parsed_interval.start));
        assert!(JulianDate::equals(&clock.stop_time, &parsed_interval.stop));
        assert!(JulianDate::equals(&clock.current_time, &parsed_current));
        assert_eq!(clock.clock_range, ClockRange::LoopStop);
        assert_eq!(clock.clock_step, ClockStep::SystemClockMultiplier);
        assert_eq!(clock.multiplier, 60.0);
    }

    data_source.process_value(&clock_czml2(), None).unwrap();
    {
        let clock = data_source.clock().expect("clock should be defined");
        assert!(JulianDate::equals(&clock.start_time, &parsed_interval2.start));
        assert!(JulianDate::equals(&clock.stop_time, &parsed_interval2.stop));
        assert!(JulianDate::equals(&clock.current_time, &parsed_current2));
        assert_eq!(clock.clock_range, ClockRange::Unbounded);
        assert_eq!(clock.clock_step, ClockStep::TickDependent);
        assert_eq!(clock.multiplier, 30.0);
    }
}

#[test]
fn clock_returns_data_interval_if_no_clock_defined() {
    let interval = TimeInterval::from_iso8601("2000-01-01/2001-01-01", None, None).unwrap();

    let data_source = CzmlDataSource::load(&make_document(dynamic_czml()), None).unwrap();
    let clock = data_source.clock().expect("clock should be defined");
    assert!(JulianDate::equals(&clock.start_time, &interval.start));
    assert!(JulianDate::equals(&clock.stop_time, &interval.stop));
    assert!(JulianDate::equals(&clock.current_time, &interval.start));
    assert_eq!(clock.clock_range, ClockRange::LoopStop);
    assert_eq!(clock.clock_step, ClockStep::SystemClockMultiplier);
    assert_eq!(
        clock.multiplier,
        JulianDate::seconds_difference(&interval.stop, &interval.start) / 120.0
    );
}

// ============================================================================
// Load / process specs
// ============================================================================

#[test]
fn process_loads_expected_data() {
    let mut data_source = CzmlDataSource::new();
    let path = data_path("CZML/simple.czml");
    data_source
        .process_file(&path.to_string_lossy(), None)
        .unwrap();
    assert_eq!(data_source.entities().values().len(), 10);
}

#[test]
fn process_loads_data_on_top_of_existing() {
    let mut data_source = CzmlDataSource::new();
    let path = data_path("CZML/simple.czml");
    data_source
        .process_file(&path.to_string_lossy(), None)
        .unwrap();
    assert_eq!(data_source.entities().values().len(), 10);

    let path = data_path("CZML/Vehicle.czml");
    data_source
        .process_file(&path.to_string_lossy(), None)
        .unwrap();
    assert_eq!(data_source.entities().values().len(), 11);
}

#[test]
fn load_replaces_data() {
    let mut data_source = CzmlDataSource::new();
    let path = data_path("CZML/simple.czml");
    data_source
        .process_file(&path.to_string_lossy(), None)
        .unwrap();
    assert_eq!(data_source.entities().values().len(), 10);

    let path = data_path("CZML/Vehicle.czml");
    data_source
        .load_file(&path.to_string_lossy(), None)
        .unwrap();
    assert_eq!(data_source.entities().values().len(), 1);
}

// ============================================================================
// Changed event specs
// ============================================================================

#[test]
fn raises_changed_event_when_loading_czml() {
    let mut data_source = CzmlDataSource::new();

    let count = Rc::new(Cell::new(0u32));
    let spy = count.clone();
    let _remove = data_source
        .changed_event()
        .add_listener(move |_a: &()| spy.set(spy.get() + 1));

    data_source.load_value(&clock_czml(), None).unwrap();
    assert_eq!(count.get(), 1);
}

#[test]
fn raises_changed_event_when_name_changes_in_czml() {
    let mut data_source = CzmlDataSource::new();

    let original_czml = json!({
        "id": "document",
        "version": "1.0",
        "name": "czmlName",
    });
    data_source.load_value(&original_czml, None).unwrap();

    let count = Rc::new(Cell::new(0u32));
    let spy = count.clone();
    let _remove = data_source
        .changed_event()
        .add_listener(move |_a: &()| spy.set(spy.get() + 1));

    let new_czml = json!({
        "id": "document",
        "name": "newCzmlName",
    });
    data_source.process_value(&new_czml, None).unwrap();
    assert_eq!(count.get(), 1);
}

#[test]
fn does_not_raise_changed_event_when_name_does_not_change_in_czml() {
    let mut data_source = CzmlDataSource::new();
    data_source.load_value(&name_czml(), None).unwrap();

    let count = Rc::new(Cell::new(0u32));
    let spy = count.clone();
    let _remove = data_source
        .changed_event()
        .add_listener(move |_a: &()| spy.set(spy.get() + 1));

    data_source.load_value(&name_czml(), None).unwrap();
    assert_eq!(count.get(), 0);
}

#[test]
fn raises_changed_event_when_clock_changes_in_czml() {
    let mut data_source = CzmlDataSource::new();

    let original_czml = json!({
        "id": "document",
        "version": "1.0",
        "clock": {
            "interval": "2012-03-15T10:00:00Z/2012-03-16T10:00:00Z",
            "currentTime": "2012-03-15T10:00:00Z",
            "multiplier": 60.0,
            "range": "LOOP_STOP",
            "step": "SYSTEM_CLOCK_MULTIPLIER",
        },
    });
    data_source.load_value(&original_czml, None).unwrap();

    let count = Rc::new(Cell::new(0u32));
    let spy = count.clone();
    let _remove = data_source
        .changed_event()
        .add_listener(move |_a: &()| spy.set(spy.get() + 1));

    let new_czml = json!({
        "id": "document",
        "version": "1.0",
        "clock": {
            "interval": "2013-03-15T10:00:00Z/2013-03-16T10:00:00Z",
            "currentTime": "2012-03-15T10:00:00Z",
            "multiplier": 60.0,
            "range": "LOOP_STOP",
            "step": "SYSTEM_CLOCK_MULTIPLIER",
        },
    });
    data_source.load_value(&new_czml, None).unwrap();
    assert_eq!(count.get(), 1);
}

#[test]
fn does_not_raise_changed_event_when_clock_does_not_change_in_czml() {
    let mut data_source = CzmlDataSource::new();
    data_source.load_value(&clock_czml(), None).unwrap();

    let count = Rc::new(Cell::new(0u32));
    let spy = count.clone();
    let _remove = data_source
        .changed_event()
        .add_listener(move |_a: &()| spy.set(spy.get() + 1));

    data_source.load_value(&clock_czml(), None).unwrap();
    assert_eq!(count.get(), 0);
}

// ============================================================================
// Error event specs
// ============================================================================

#[test]
fn raises_error_when_an_error_occurs_in_load() {
    let mut data_source = CzmlDataSource::new();

    let count = Rc::new(Cell::new(0u32));
    let spy = count.clone();
    let _remove = data_source
        .error_event()
        .add_listener(move |_a: &()| spy.set(spy.get() + 1));

    // Blue.png is not JSON
    let path = data_path("Images/Blue.png");
    let result = data_source.load_file(&path.to_string_lossy(), None);
    assert!(result.is_err());
    assert_eq!(count.get(), 1);
}

#[test]
fn raises_error_when_an_error_occurs_in_process() {
    let mut data_source = CzmlDataSource::new();

    let count = Rc::new(Cell::new(0u32));
    let spy = count.clone();
    let _remove = data_source
        .error_event()
        .add_listener(move |_a: &()| spy.set(spy.get() + 1));

    // Blue.png is not JSON
    let path = data_path("Images/Blue.png");
    let result = data_source.process_file(&path.to_string_lossy(), None);
    assert!(result.is_err());
    assert_eq!(count.get(), 1);
}

// ============================================================================
// Version validation specs
// ============================================================================

#[test]
fn rejects_if_first_document_packet_lacks_version_information() {
    let result = CzmlDataSource::load(&json!({ "id": "document" }), None);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), VERSION_INVALID_ERROR);
}

#[test]
fn rejects_if_first_packet_is_not_document() {
    let result = CzmlDataSource::load(&json!({ "id": "someId" }), None);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), FIRST_PACKET_ERROR);
}

#[test]
fn rejects_if_document_packet_contains_bad_version() {
    let result = CzmlDataSource::load(&json!({ "id": "document" }), None);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains(VERSION_INVALID_ERROR));
}

#[test]
fn rejects_if_document_packet_contains_unsupported_major_version() {
    let result = CzmlDataSource::load(
        &json!({ "id": "document", "version": "2.0" }),
        None,
    );
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        "Cesium only supports CZML version 1."
    );
}

// ============================================================================
// Position / orientation / viewFrom / description specs
// ============================================================================

#[test]
fn can_load_position() {
    let packet = json!({
        "position": {
            "cartesian": [1.0, 2.0, 3.0],
        },
    });

    let data_source = CzmlDataSource::load(&make_document(packet.clone()), None).unwrap();
    let entity = &data_source.entities().values()[0];
    let values: Vec<f64> = packet["position"]["cartesian"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    let expected = Cartesian3::unpack_new(&values, None);
    assert_eq!(entity.position, Some(expected));
}

#[test]
fn can_load_orientation() {
    let packet = json!({
        "orientation": {
            "unitQuaternion": [0.0, 0.0, 0.0, 1.0],
        },
    });

    let data_source = CzmlDataSource::load(&make_document(packet.clone()), None).unwrap();
    let entity = &data_source.entities().values()[0];
    let values: Vec<f64> = packet["orientation"]["unitQuaternion"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    let expected = Quaternion::unpack_new(&values, 0);
    assert!(entity.orientation.is_some());
    assert!(Quaternion::equals(&entity.orientation.unwrap(), &expected));
}

#[test]
fn normalizes_constant_orientation_on_load() {
    let packet = json!({
        "orientation": {
            "unitQuaternion": [0.0, 0.0, 0.7071067, 0.7071067],
        },
    });

    let values: Vec<f64> = packet["orientation"]["unitQuaternion"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    let expected = Quaternion::normalize_new(&Quaternion::unpack_new(&values, 0));

    let data_source = CzmlDataSource::load(&make_document(packet), None).unwrap();
    let entity = &data_source.entities().values()[0];
    assert!(entity.orientation.is_some());
    assert!(Quaternion::equals(&entity.orientation.unwrap(), &expected));
}

#[test]
fn can_load_view_from() {
    let packet = json!({
        "viewFrom": {
            "cartesian": [1.0, 2.0, 3.0],
        },
    });

    let data_source = CzmlDataSource::load(&make_document(packet.clone()), None).unwrap();
    let entity = &data_source.entities().values()[0];
    let values: Vec<f64> = packet["viewFrom"]["cartesian"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    let expected = Cartesian3::unpack_new(&values, None);
    assert_eq!(entity.view_from, Some(expected));
}

#[test]
fn can_load_description() {
    let packet = json!({
        "description": "this is a description",
    });

    let data_source = CzmlDataSource::load(&make_document(packet), None).unwrap();
    let entity = &data_source.entities().values()[0];
    assert_eq!(
        entity.description.as_deref(),
        Some("this is a description")
    );
}

// ============================================================================
// Custom properties specs
// ============================================================================

#[test]
fn can_load_constant_custom_properties() {
    let test_object = json!({
        "foo": 4,
        "bar": {
            "name": "bar",
        },
    });
    let test_array = json!([2, 4, 16, "test"]);

    let packet = json!({
        "properties": {
            "constant_name": "ABC",
            "constant_height": 8,
            "constant_object": {
                "value": test_object,
            },
            "constant_array": {
                "value": test_array,
            },
        },
    });

    let data_source = CzmlDataSource::load(&make_document(packet), None).unwrap();
    let entity = &data_source.entities().values()[0];
    assert_eq!(
        entity.properties.get("constant_name"),
        Some(&PropertyResult::String("ABC".to_string()))
    );
    assert_eq!(
        entity.properties.get("constant_height"),
        Some(&PropertyResult::Number(8.0))
    );
    assert_eq!(
        entity.properties.get("constant_object"),
        Some(&PropertyResult::Json(test_object))
    );
    assert_eq!(
        entity.properties.get("constant_array"),
        Some(&PropertyResult::Json(test_array))
    );
}

#[test]
fn can_load_custom_properties_which_are_constant_with_specified_type() {
    let test_object = json!({
        "foo": 4,
        "bar": {
            "name": "bar",
        },
    });
    let test_array = json!([2, 4, 16, "test"]);

    let packet = json!({
        "properties": {
            "constant_name": {
                "string": "ABC",
            },
            "constant_height": {
                "number": 8,
            },
            "constant_object": {
                "object": test_object,
            },
            "constant_array": {
                "array": test_array,
            },
        },
    });

    let data_source = CzmlDataSource::load(&make_document(packet), None).unwrap();
    let entity = &data_source.entities().values()[0];
    assert_eq!(
        entity.properties.get("constant_name"),
        Some(&PropertyResult::String("ABC".to_string()))
    );
    assert_eq!(
        entity.properties.get("constant_height"),
        Some(&PropertyResult::Number(8.0))
    );
    assert_eq!(
        entity.properties.get("constant_object"),
        Some(&PropertyResult::Json(test_object))
    );
    assert_eq!(
        entity.properties.get("constant_array"),
        Some(&PropertyResult::Json(test_array))
    );
}

// ============================================================================
// Availability specs
// ============================================================================

/// Mirror of the spec helper comparing a stored interval with a parsed one.
fn assert_interval_equals(actual: &TimeInterval, iso8601: &str) {
    let expected = TimeInterval::from_iso8601(iso8601, None, None).unwrap();
    assert!(JulianDate::equals(&actual.start, &expected.start));
    assert!(JulianDate::equals(&actual.stop, &expected.stop));
}

#[test]
fn can_load_and_modify_availability_from_a_single_interval() {
    let packet1 = json!({
        "id": "testObject",
        "availability": "2000-01-01/2001-01-01",
    });
    let packet2 = json!({
        "id": "testObject",
        "availability": "2000-02-02/2001-02-02",
    });

    let mut data_source = CzmlDataSource::new();
    data_source
        .process_value(&make_document(packet1), None)
        .unwrap();
    {
        let entity = &data_source.entities().values()[0];
        assert_eq!(entity.availability.len(), 1);
        assert_interval_equals(&entity.availability[0], "2000-01-01/2001-01-01");
    }

    data_source.process_value(&packet2, None).unwrap();
    {
        let entity = &data_source.entities().values()[0];
        assert_eq!(entity.availability.len(), 1);
        assert_interval_equals(&entity.availability[0], "2000-02-02/2001-02-02");
    }
}

#[test]
fn can_load_and_modify_availability_from_multiple_intervals() {
    let packet1 = json!({
        "id": "testObject",
        "availability": ["2000-01-01/2001-01-01", "2002-01-01/2003-01-01"],
    });
    let packet2 = json!({
        "id": "testObject",
        "availability": ["2003-01-01/2004-01-01", "2005-01-01/2006-01-01"],
    });

    let mut data_source = CzmlDataSource::new();
    data_source
        .process_value(&make_document(packet1), None)
        .unwrap();
    {
        let entity = &data_source.entities().values()[0];
        assert_eq!(entity.availability.len(), 2);
        assert_interval_equals(&entity.availability[0], "2000-01-01/2001-01-01");
        assert_interval_equals(&entity.availability[1], "2002-01-01/2003-01-01");
    }

    data_source.process_value(&packet2, None).unwrap();
    {
        let entity = &data_source.entities().values()[0];
        assert_eq!(entity.availability.len(), 2);
        assert_interval_equals(&entity.availability[0], "2003-01-01/2004-01-01");
        assert_interval_equals(&entity.availability[1], "2005-01-01/2006-01-01");
    }
}

// ============================================================================
// Delete / parent specs
// ============================================================================

#[test]
fn can_delete_an_existing_object() {
    let mut data_source = CzmlDataSource::new();
    data_source
        .load_value(&make_document(static_czml()), None)
        .unwrap();
    assert_eq!(data_source.entities().values().len(), 1);

    data_source
        .load_value(&make_document(czml_delete()), None)
        .unwrap();
    assert_eq!(data_source.entities().values().len(), 0);
}

#[test]
fn loads_parent() {
    let document = json!([
        {
            "id": "document",
            "version": "1.0",
        },
        {
            "id": "parent",
        },
        {
            "id": "child",
            "parent": "parent",
        },
    ]);

    let data_source = CzmlDataSource::load(&document, None).unwrap();
    let parent = data_source.entities().get_by_id("parent").unwrap();
    assert!(parent.parent_id.is_none());

    let child = data_source.entities().get_by_id("child").unwrap();
    assert_eq!(child.parent_id.as_deref(), Some("parent"));
}

#[test]
fn loads_parent_specified_out_of_order() {
    let document = json!([
        {
            "id": "document",
            "version": "1.0",
        },
        {
            "id": "child",
            "parent": "parent",
        },
        {
            "id": "child2",
            "parent": "parent",
        },
        {
            "id": "grandparent",
        },
        {
            "id": "grandparent2",
        },
        {
            "id": "parent",
            "parent": "grandparent",
        },
        {
            "id": "parent2",
            "parent": "grandparent",
        },
    ]);

    let data_source = CzmlDataSource::load(&document, None).unwrap();
    let grandparent = data_source.entities().get_by_id("grandparent").unwrap();
    assert!(grandparent.parent_id.is_none());

    let grandparent2 = data_source.entities().get_by_id("grandparent2").unwrap();
    assert!(grandparent2.parent_id.is_none());

    let parent = data_source.entities().get_by_id("parent").unwrap();
    assert_eq!(parent.parent_id.as_deref(), Some("grandparent"));

    let parent2 = data_source.entities().get_by_id("parent2").unwrap();
    assert_eq!(parent2.parent_id.as_deref(), Some("grandparent"));

    let child = data_source.entities().get_by_id("child").unwrap();
    assert_eq!(child.parent_id.as_deref(), Some("parent"));

    let child2 = data_source.entities().get_by_id("child2").unwrap();
    assert_eq!(child2.parent_id.as_deref(), Some("parent"));
}

// ============================================================================
// Billboard specs
// ============================================================================

#[test]
fn can_load_constant_data_for_billboard() {
    let source_uri = "http://someImage.invalid/";
    let packet = json!({
        "billboard": {
            "image": "image.png",
            "scale": 1.0,
            "rotation": 1.3,
            "heightReference": "CLAMP_TO_GROUND",
            "horizontalOrigin": "CENTER",
            "verticalOrigin": "CENTER",
            "color": {
                "rgbaf": [1.0, 1.0, 1.0, 1.0],
            },
            "eyeOffset": {
                "cartesian": [3.0, 4.0, 5.0],
            },
            "pixelOffset": {
                "cartesian2": [1.0, 2.0],
            },
            "alignedAxis": {
                "unitCartesian": [1.0, 0.0, 0.0],
            },
            "show": true,
            "sizeInMeters": false,
            "width": 10,
            "height": 11,
            "scaleByDistance": {
                "nearFarScalar": [1.0, 2.0, 10000.0, 3.0],
            },
            "translucencyByDistance": {
                "nearFarScalar": [1.0, 1.0, 10000.0, 0.0],
            },
            "pixelOffsetScaleByDistance": {
                "nearFarScalar": [1.0, 20.0, 10000.0, 30.0],
            },
            "imageSubRegion": {
                "boundingRectangle": [20, 30, 10, 11],
            },
        },
    });

    let options = CzmlLoadOptions {
        source_uri: Some(source_uri.to_string()),
        credit: None,
    };
    let data_source = CzmlDataSource::load(&make_document(packet), Some(&options)).unwrap();
    let entity = &data_source.entities().values()[0];

    let billboard = entity.billboard.as_ref().expect("billboard should be defined");
    assert_eq!(
        billboard.image.as_deref(),
        Some("http://someImage.invalid/image.png")
    );
    assert_eq!(billboard.rotation, 1.3);
    assert_eq!(billboard.scale, 1.0);
    assert_eq!(
        billboard.height_reference,
        HeightReference::ClampToGround as i32
    );
    assert_eq!(
        billboard.horizontal_origin,
        HorizontalOrigin::Center as i32
    );
    assert_eq!(billboard.vertical_origin, VerticalOrigin::Center as i32);
    assert_eq!(billboard.color, Some(Color::new(1.0, 1.0, 1.0, 1.0)));
    assert_eq!(
        billboard.eye_offset,
        Some(Cartesian3::unpack_new(&[3.0, 4.0, 5.0], None))
    );
    assert_eq!(billboard.pixel_offset, Some((1.0, 2.0)));
    assert_eq!(
        billboard.aligned_axis,
        Some(Cartesian3::unpack_new(&[1.0, 0.0, 0.0], None))
    );
    assert!(billboard.show);
    assert_eq!(billboard.size_in_meters, Some(false));
    assert_eq!(billboard.width, Some(10.0));
    assert_eq!(billboard.height, Some(11.0));
    assert_eq!(
        billboard.scale_by_distance,
        Some(NearFarScalar {
            near: 1.0,
            near_value: 2.0,
            far: 10000.0,
            far_value: 3.0,
        })
    );
    assert_eq!(
        billboard.translucency_by_distance,
        Some(NearFarScalar {
            near: 1.0,
            near_value: 1.0,
            far: 10000.0,
            far_value: 0.0,
        })
    );
    assert_eq!(
        billboard.pixel_offset_scale_by_distance,
        Some(NearFarScalar {
            near: 1.0,
            near_value: 20.0,
            far: 10000.0,
            far_value: 30.0,
        })
    );
    assert_eq!(billboard.image_sub_region, Some((20.0, 30.0, 10.0, 11.0)));
}

#[test]
fn can_handle_aligned_axis_expressed_as_a_cartesian() {
    // historically, CZML allowed alignedAxis to be defined as a cartesian,
    // even though that implied it could be non-unit magnitude (it can't).
    // but, we need to ensure that continues to work.
    let packet = json!({
        "billboard": {
            "alignedAxis": {
                "cartesian": [1.0, 0.0, 0.0],
            },
        },
    });

    let data_source = CzmlDataSource::load(&make_document(packet), None).unwrap();
    let entity = &data_source.entities().values()[0];
    let billboard = entity.billboard.as_ref().expect("billboard should be defined");
    assert_eq!(
        billboard.aligned_axis,
        Some(Cartesian3::unpack_new(&[1.0, 0.0, 0.0], None))
    );
}

#[test]
fn ignores_color_values_not_expressed_as_a_known_type() {
    let packet = json!({
        "billboard": {
            "color": {
                "invalidType": "someValue",
            },
        },
    });

    let data_source = CzmlDataSource::load(&make_document(packet), None).unwrap();
    let entity = &data_source.entities().values()[0];
    let billboard = entity.billboard.as_ref().expect("billboard should be defined");
    assert!(billboard.color.is_none());
}

// ============================================================================
// Label specs
// ============================================================================

#[test]
fn can_load_constant_data_for_label() {
    let packet = json!({
        "label": {
            "text": "TestFacility",
            "font": "10pt \"Open Sans\"",
            "style": "FILL",
            "fillColor": {
                "rgbaf": [0.1, 0.1, 0.1, 0.1],
            },
            "outlineColor": {
                "rgbaf": [0.2, 0.2, 0.2, 0.2],
            },
            "outlineWidth": 3.14,
            "horizontalOrigin": "LEFT",
            "verticalOrigin": "CENTER",
            "eyeOffset": {
                "cartesian": [1.0, 2.0, 3.0],
            },
            "pixelOffset": {
                "cartesian2": [4.0, 5.0],
            },
            "scale": 1.0,
            "show": true,
            "translucencyByDistance": {
                "nearFarScalar": [1.0, 1.0, 10000.0, 0.0],
            },
            "pixelOffsetScaleByDistance": {
                "nearFarScalar": [1.0, 20.0, 10000.0, 30.0],
            },
        },
    });

    let data_source = CzmlDataSource::load(&make_document(packet), None).unwrap();
    let entity = &data_source.entities().values()[0];

    let label = entity.label.as_ref().expect("label should be defined");
    assert_eq!(label.text.as_deref(), Some("TestFacility"));
    assert_eq!(label.font.as_deref(), Some("10pt \"Open Sans\""));
    assert_eq!(label.style, LabelStyle::Fill as i32);
    assert_eq!(label.fill_color, Color::new(0.1, 0.1, 0.1, 0.1));
    assert_eq!(label.outline_color, Color::new(0.2, 0.2, 0.2, 0.2));
    assert_eq!(label.outline_width, 3.14);
    assert_eq!(label.horizontal_origin, HorizontalOrigin::Left as i32);
    assert_eq!(label.vertical_origin, VerticalOrigin::Center as i32);
    assert_eq!(
        label.eye_offset,
        Some(Cartesian3::unpack_new(&[1.0, 2.0, 3.0], None))
    );
    assert_eq!(label.pixel_offset, Some((4.0, 5.0)));
    assert_eq!(label.scale, 1.0);
    assert!(label.show);
    assert_eq!(
        label.translucency_by_distance,
        Some(NearFarScalar {
            near: 1.0,
            near_value: 1.0,
            far: 10000.0,
            far_value: 0.0,
        })
    );
    assert_eq!(
        label.pixel_offset_scale_by_distance,
        Some(NearFarScalar {
            near: 1.0,
            near_value: 20.0,
            far: 10000.0,
            far_value: 30.0,
        })
    );
}

// ============================================================================
// Point specs
// ============================================================================

#[test]
fn can_load_constant_data_for_point() {
    let packet = json!({
        "point": {
            "color": {
                "rgbaf": [0.1, 0.1, 0.1, 0.1],
            },
            "pixelSize": 1.0,
            "outlineColor": {
                "rgbaf": [0.2, 0.2, 0.2, 0.2],
            },
            "outlineWidth": 1.0,
            "show": true,
            "scaleByDistance": {
                "nearFarScalar": [1.0, 2.0, 10000.0, 3.0],
            },
            "translucencyByDistance": {
                "nearFarScalar": [1.0, 1.0, 10000.0, 0.0],
            },
            "heightReference": "CLAMP_TO_GROUND",
        },
    });

    let data_source = CzmlDataSource::load(&make_document(packet), None).unwrap();
    let entity = &data_source.entities().values()[0];

    let point = entity.point.as_ref().expect("point should be defined");
    assert_eq!(point.color, Color::new(0.1, 0.1, 0.1, 0.1));
    assert_eq!(point.pixel_size, 1.0);
    assert_eq!(point.outline_color, Color::new(0.2, 0.2, 0.2, 0.2));
    assert_eq!(point.outline_width, 1.0);
    assert!(point.show);
    assert_eq!(
        point.scale_by_distance,
        Some(NearFarScalar {
            near: 1.0,
            near_value: 2.0,
            far: 10000.0,
            far_value: 3.0,
        })
    );
    assert_eq!(
        point.translucency_by_distance,
        Some(NearFarScalar {
            near: 1.0,
            near_value: 1.0,
            far: 10000.0,
            far_value: 0.0,
        })
    );
    assert_eq!(
        point.height_reference,
        HeightReference::ClampToGround as i32
    );
}

// ============================================================================
// Polyline position specs
// ============================================================================

/// Mirror of `Cartesian3.unpackArray`.
fn unpack_positions(values: &[f64]) -> Vec<Cartesian3> {
    values
        .chunks(3)
        .map(|chunk| Cartesian3::unpack_new(chunk, None))
        .collect()
}

#[test]
fn can_load_positions_expressed_as_cartesians() {
    let packet = json!({
        "polyline": {
            "positions": {
                "cartesian": [1.0, 2.0, 3.0, 5.0, 6.0, 7.0],
            },
        },
    });

    let data_source = CzmlDataSource::load(&make_document(packet), None).unwrap();
    let entity = &data_source.entities().values()[0];
    let polyline = entity.polyline.as_ref().expect("polyline should be defined");
    assert_eq!(
        polyline.positions,
        unpack_positions(&[1.0, 2.0, 3.0, 5.0, 6.0, 7.0])
    );
}

#[test]
fn can_load_positions_expressed_as_cartographic_radians() {
    let packet = json!({
        "polyline": {
            "positions": {
                "cartographicRadians": [1.0, 2.0, 4.0, 5.0, 6.0, 7.0],
            },
        },
    });

    let data_source = CzmlDataSource::load(&make_document(packet), None).unwrap();
    let entity = &data_source.entities().values()[0];
    let polyline = entity.polyline.as_ref().expect("polyline should be defined");
    assert_eq!(
        polyline.positions,
        Cartesian3::from_radians_array_heights(&[1.0, 2.0, 4.0, 5.0, 6.0, 7.0], None, None)
    );
}

#[test]
fn can_load_positions_expressed_as_cartographic_degrees() {
    let packet = json!({
        "polyline": {
            "positions": {
                "cartographicDegrees": [1.0, 2.0, 3.0, 5.0, 6.0, 7.0],
            },
        },
    });

    let data_source = CzmlDataSource::load(&make_document(packet), None).unwrap();
    let entity = &data_source.entities().values()[0];
    let polyline = entity.polyline.as_ref().expect("polyline should be defined");
    assert_eq!(
        polyline.positions,
        Cartesian3::from_degrees_array_heights(&[1.0, 2.0, 3.0, 5.0, 6.0, 7.0], None, None)
    );
}
