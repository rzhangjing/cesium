//! Spec mirror of
//! `packages/engine/Specs/DataSources/TimeIntervalCollectionPropertySpec.js`.
//!
//! One `#[test]` per Jasmine `it(...)` (snake_case direct translation).
//!
//! Time convention: CesiumJS `JulianDate(day, seconds)` maps to plain `f64`
//! seconds (`day * 86400 + seconds`), matching the crate-wide time
//! convention. `definitionChanged` event assertions from the JS spec are
//! skipped: the event system is owned by a separate work item (#34).

use cesium_data_sources::property::{Property, PropertyResult};
use cesium_data_sources::time_interval_collection_property::{
    PropertyTimeInterval, TimeIntervalCollectionProperty,
};

const DAY: f64 = 86400.0;

#[test]
fn default_constructor_has_expected_values() {
    let property = TimeIntervalCollectionProperty::new();
    assert!(property.is_empty());
    // JS: getValue(JulianDate.now()) === undefined
    assert!(property.get_value_option(0.0).is_none());
    assert!(property.is_constant());
}

#[test]
fn works_with_basic_types() {
    // JS: JulianDate(10, 0) / JulianDate(12, 0) / JulianDate(14, 0)
    let interval1 = PropertyTimeInterval::new(10.0 * DAY, 12.0 * DAY, PropertyResult::Number(5.0));
    
    let mut interval2 = PropertyTimeInterval::new(12.0 * DAY, 14.0 * DAY, PropertyResult::Number(6.0));
    interval2.is_start_included = false;

    let mut property = TimeIntervalCollectionProperty::new();
    property.add_interval(interval1.clone());
    property.add_interval(interval2.clone());

    assert_eq!(
        property.get_value_option(interval1.start),
        Some(interval1.data.clone())
    );
    assert_eq!(
        property.get_value_option(interval2.stop),
        Some(interval2.data.clone())
    );
    assert!(!property.is_constant());
}

#[test]
fn works_with_clonable_objects() {
    let interval1 = PropertyTimeInterval::new(
        10.0 * DAY,
        12.0 * DAY,
        PropertyResult::Cartesian3(1.0, 2.0, 3.0),
    );
    let mut interval2 = PropertyTimeInterval::new(
        12.0 * DAY,
        14.0 * DAY,
        PropertyResult::Cartesian3(4.0, 5.0, 6.0),
    );
    interval2.is_start_included = false;

    let mut property = TimeIntervalCollectionProperty::new();
    property.add_interval(interval1.clone());
    property.add_interval(interval2.clone());

    // JS asserts result !== interval.data (identity) and deep equality.
    // DEVIATION: the Rust API returns an owned clone of the stored data, so
    // only the value-equality half of the JS assertion is testable here.
    let result1 = property.get_value_option(interval1.start);
    assert_eq!(result1, Some(interval1.data.clone()));

    let result2 = property.get_value_option(interval2.stop);
    assert_eq!(result2, Some(interval2.data.clone()));
}

#[test]
fn works_with_a_result_parameter() {
    let interval1 = PropertyTimeInterval::new(
        10.0 * DAY,
        12.0 * DAY,
        PropertyResult::Cartesian3(1.0, 2.0, 3.0),
    );
    let mut interval2 = PropertyTimeInterval::new(
        12.0 * DAY,
        14.0 * DAY,
        PropertyResult::Cartesian3(4.0, 5.0, 6.0),
    );
    interval2.is_start_included = false;

    let mut property = TimeIntervalCollectionProperty::new();
    property.add_interval(interval1.clone());
    property.add_interval(interval2.clone());

    // DEVIATION (out parameter folded): the JS `getValue(time, result)`
    // writes into `result` and returns it; the Rust port returns the value
    // directly, so the assertion reduces to value equality.
    let result1 = property.get_value_option(interval1.start);
    assert_eq!(result1, Some(interval1.data.clone()));

    let result2 = property.get_value_option(interval2.stop);
    assert_eq!(result2, Some(interval2.data.clone()));
}

#[test]
#[ignore = "DEVIATION: getValue() without a time parameter (JulianDate.now() default) is not part of the Rust API"]
fn get_value_uses_now_if_time_parameter_is_undefined() {
    // JS spies on JulianDate.now() when calling property.getValue().
}

#[test]
fn equals_works_for_differing_basic_type_intervals() {
    let interval1 = PropertyTimeInterval::new(10.0 * DAY, 12.0 * DAY, PropertyResult::Number(5.0));
    let mut interval2 = PropertyTimeInterval::new(12.0 * DAY, 14.0 * DAY, PropertyResult::Number(6.0));
    interval2.is_start_included = false;

    let mut left = TimeIntervalCollectionProperty::new();
    left.add_interval(interval1.clone());
    left.add_interval(interval2.clone());

    let mut right = TimeIntervalCollectionProperty::new();
    right.add_interval(interval1.clone());

    assert!(!left.equals(&right));
    right.add_interval(interval2);
    assert!(left.equals(&right));
}

#[test]
fn equals_works_for_differing_complex_type_intervals() {
    let interval1 = PropertyTimeInterval::new(
        10.0 * DAY,
        12.0 * DAY,
        PropertyResult::Cartesian3(1.0, 2.0, 3.0),
    );
    let mut interval2 = PropertyTimeInterval::new(
        12.0 * DAY,
        14.0 * DAY,
        PropertyResult::Cartesian3(4.0, 5.0, 6.0),
    );
    interval2.is_start_included = false;

    let mut left = TimeIntervalCollectionProperty::new();
    left.add_interval(interval1.clone());
    left.add_interval(interval2.clone());

    let mut right = TimeIntervalCollectionProperty::new();
    right.add_interval(interval1.clone());

    assert!(!left.equals(&right));
    right.add_interval(interval2);
    assert!(left.equals(&right));
}

#[test]
#[ignore = "DEVIATION: definitionChanged event not implemented (owned by work item #34)"]
fn raises_definition_changed_event() {
    // JS: addInterval / removeInterval / removeAll each fire
    // property.definitionChanged with the property as argument.
}
