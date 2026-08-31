//! Tests for `cesium_core::TimeInterval`.
//!
//! Mirrors `packages/engine/Specs/Core/TimeIntervalSpec.js`.

use cesium_core::julian_date::JulianDate;
use cesium_core::time_interval::{IntervalData, TimeInterval};
use cesium_core::time_standard::TimeStandard;

fn jd(days: f64) -> JulianDate {
    JulianDate::new(days, 0.0, TimeStandard::UTC)
}

fn jd_with_seconds(days: f64, seconds: f64) -> JulianDate {
    JulianDate::new(days, seconds, TimeStandard::UTC)
}

// --- constructor ---

#[test]
fn constructor_sets_expected_defaults() {
    let interval = TimeInterval::new(None, None, None, None);
    assert!(JulianDate::equals(
        &interval.start,
        &JulianDate::default_date()
    ));
    assert!(JulianDate::equals(&interval.stop, &JulianDate::default_date()));
    assert!(interval.is_start_included);
    assert!(interval.is_stop_included);
    assert!(interval.data.is_none());
}

#[test]
fn constructor_assigns_all_options() {
    let start = JulianDate::now();
    let stop = JulianDate::add_days(&start, 1.0);
    let data = IntervalData::object();

    let interval = TimeInterval::new_with_data(
        Some(start.clone()),
        Some(stop.clone()),
        Some(false),
        Some(false),
        Some(data.clone()),
    );

    assert!(JulianDate::equals(&interval.start, &start));
    assert!(JulianDate::equals(&interval.stop, &stop));
    assert!(!interval.is_start_included);
    assert!(!interval.is_stop_included);
    assert_eq!(interval.data, Some(data));
}

// --- fromIso8601 ---

#[test]
fn from_iso8601_assigns_expected_defaults() {
    let start = JulianDate::from_iso8601("2013").unwrap();
    let stop = JulianDate::from_iso8601("2014").unwrap();

    let interval = TimeInterval::from_iso8601("2013/2014", None, None).unwrap();

    assert!(JulianDate::equals(&interval.start, &start));
    assert!(JulianDate::equals(&interval.stop, &stop));
    assert!(interval.is_start_included);
    assert!(interval.is_stop_included);
    assert!(interval.data.is_none());
}

#[test]
fn from_iso8601_assigns_all_options() {
    let start = JulianDate::from_iso8601("2013").unwrap();
    let stop = JulianDate::from_iso8601("2014").unwrap();
    let data = IntervalData::object();

    let interval = TimeInterval::from_iso8601_with_data(
        "2013/2014",
        Some(false),
        Some(false),
        Some(data.clone()),
    )
    .unwrap();

    assert!(JulianDate::equals(&interval.start, &start));
    assert!(JulianDate::equals(&interval.stop, &stop));
    assert!(!interval.is_start_included);
    assert!(!interval.is_stop_included);
    assert_eq!(interval.data, Some(data));
}

// DEVIATION: the JS `fromIso8601` works with a `result` out-parameter; the
// Rust port returns an owned value, so the "works with result parameter" spec
// has no counterpart.

#[test]
fn from_iso8601_returns_none_when_given_invalid_iso8601_date() {
    // DEVIATION (error type): the JS version throws a DeveloperError; the
    // Rust port returns `None`.
    assert!(TimeInterval::from_iso8601("2020-08-29T00:00:00+00:00", None, None).is_none());
}

// --- toIso8601 ---

#[test]
fn to_iso8601_works() {
    let iso_date1 = "0950-01-02T03:04:05Z";
    let iso_date2 = "0950-01-03T03:04:05Z";
    let interval = TimeInterval::new(
        Some(JulianDate::from_iso8601(iso_date1).unwrap()),
        Some(JulianDate::from_iso8601(iso_date2).unwrap()),
        None,
        None,
    );
    assert_eq!(
        interval.to_iso8601(None),
        "0950-01-02T03:04:05Z/0950-01-03T03:04:05Z"
    );
}

#[test]
fn can_round_trip_with_iso8601() {
    let interval = TimeInterval::new(Some(JulianDate::now()), Some(JulianDate::now()), None, None);
    let round_tripped =
        TimeInterval::from_iso8601(&interval.to_iso8601(None), None, None).unwrap();
    assert!(TimeInterval::equals(&round_tripped, &interval));
}

#[test]
fn to_iso8601_works_with_specified_precision() {
    let iso_date1 = "0950-01-02T03:04:05.012345Z";
    let iso_date2 = "0950-01-03T03:04:05.012345Z";
    let interval = TimeInterval::new(
        Some(JulianDate::from_iso8601(iso_date1).unwrap()),
        Some(JulianDate::from_iso8601(iso_date2).unwrap()),
        None,
        None,
    );
    assert_eq!(
        interval.to_iso8601(Some(0)),
        "0950-01-02T03:04:05Z/0950-01-03T03:04:05Z"
    );
    assert_eq!(
        interval.to_iso8601(Some(7)),
        "0950-01-02T03:04:05.0123450Z/0950-01-03T03:04:05.0123450Z"
    );
}

// --- isEmpty ---

#[test]
fn is_empty_is_false_for_a_non_empty_interval() {
    let interval = TimeInterval::new(Some(jd(1.0)), Some(jd(2.0)), None, None);
    assert!(!interval.is_empty());
}

#[test]
fn is_empty_is_false_for_an_instantaneous_interval_closed_on_both_ends() {
    let interval = TimeInterval::new(Some(jd(1.0)), Some(jd(1.0)), None, None);
    assert!(!interval.is_empty());
}

#[test]
fn is_empty_is_true_for_an_instantaneous_interval_open_on_both_ends() {
    let interval = TimeInterval::new(Some(jd(1.0)), Some(jd(1.0)), Some(false), Some(false));
    assert!(interval.is_empty());
}

#[test]
fn is_empty_is_true_for_an_instantaneous_interval_open_on_start() {
    let interval = TimeInterval::new(Some(jd(1.0)), Some(jd(1.0)), Some(false), Some(true));
    assert!(interval.is_empty());
}

#[test]
fn is_empty_is_true_for_an_instantaneous_interval_open_on_stop() {
    let interval = TimeInterval::new(Some(jd(1.0)), Some(jd(1.0)), Some(true), Some(false));
    assert!(interval.is_empty());
}

#[test]
fn is_empty_is_true_for_an_interval_with_stop_before_start() {
    let interval = TimeInterval::new(Some(jd(5.0)), Some(jd(4.0)), None, None);
    assert!(interval.is_empty());
}

#[test]
fn is_empty_is_true_for_an_instantaneous_interval_only_closed_on_stop_end() {
    let interval = TimeInterval::new(Some(jd(5.0)), Some(jd(5.0)), Some(false), Some(true));
    assert!(interval.is_empty());
}

#[test]
fn is_empty_is_true_for_an_instantaneous_interval_only_closed_on_start_end() {
    let interval = TimeInterval::new(Some(jd(5.0)), Some(jd(5.0)), Some(true), Some(false));
    assert!(interval.is_empty());
}

// --- contains ---

#[test]
fn contains_works_for_a_non_empty_interval() {
    let interval = TimeInterval::new(Some(jd(2_451_545.0)), Some(jd(2_451_546.0)), None, None);
    assert!(interval.contains(&jd(2_451_545.5)));
    assert!(!interval.contains(&jd(2_451_546.5)));
}

#[test]
fn contains_works_for_an_empty_interval() {
    assert!(!TimeInterval::empty().contains(&JulianDate::default_date()));
}

#[test]
fn contains_returns_true_at_start_and_stop_times_of_a_closed_interval() {
    let interval = TimeInterval::new(
        Some(jd(2_451_545.0)),
        Some(jd(2_451_546.0)),
        Some(true),
        Some(true),
    );
    assert!(interval.contains(&jd(2_451_545.0)));
    assert!(interval.contains(&jd(2_451_546.0)));
}

#[test]
fn contains_returns_false_at_start_and_stop_times_of_an_open_interval() {
    let interval = TimeInterval::new(
        Some(jd(2_451_545.0)),
        Some(jd(2_451_546.0)),
        Some(false),
        Some(false),
    );
    assert!(!interval.contains(&jd(2_451_545.0)));
    assert!(!interval.contains(&jd(2_451_546.0)));
}

// --- equals / equalsEpsilon ---

#[test]
fn equals_and_equals_epsilon_work() {
    let left = TimeInterval::new(None, None, None, None);
    let right = TimeInterval::new(None, None, None, None);

    assert!(TimeInterval::equals(&left, &right));
    assert!(TimeInterval::equals_epsilon(&left, &right, 0.0));

    let right = TimeInterval::new(Some(jd(-1.0)), None, None, None);
    assert!(!TimeInterval::equals(&left, &right));
    assert!(!TimeInterval::equals_epsilon(&left, &right, 0.0));

    let right = TimeInterval::new(None, Some(jd(1.0)), None, None);
    assert!(!TimeInterval::equals(&left, &right));
    assert!(!TimeInterval::equals_epsilon(&left, &right, 0.0));

    let right = TimeInterval::new(None, None, Some(false), None);
    assert!(!TimeInterval::equals(&left, &right));
    assert!(!TimeInterval::equals_epsilon(&left, &right, 0.0));

    let right = TimeInterval::new(None, None, None, Some(false));
    assert!(!TimeInterval::equals(&left, &right));
    assert!(!TimeInterval::equals_epsilon(&left, &right, 0.0));

    // JS: right.data = {}; objects are never `===` to undefined.
    let left = TimeInterval::new(None, None, None, None);
    let right = TimeInterval::new_with_data(None, None, None, None, Some(IntervalData::object()));
    assert!(!TimeInterval::equals(&left, &right));
    assert!(!TimeInterval::equals_epsilon(&left, &right, 0.0));

    let return_true = |_left: Option<&IntervalData>, _right: Option<&IntervalData>| true;
    assert!(TimeInterval::equals_with(&left, &right, Some(&return_true)));
    assert!(TimeInterval::equals_epsilon_with(
        &left,
        &right,
        0.0,
        Some(&return_true)
    ));
}

#[test]
fn equals_epsilon_works_within_threshold() {
    let left = TimeInterval::new(Some(jd(0.0)), Some(jd(1.0)), None, None);
    let right = TimeInterval::new(Some(jd(0.0)), Some(jd_with_seconds(1.0, 1.0)), None, None);
    assert!(TimeInterval::equals_epsilon(&left, &right, 1.0));
    assert!(!TimeInterval::equals_epsilon(&left, &right, 0.99));
}

// --- clone ---

#[test]
fn clone_returns_an_identical_interval() {
    let interval = TimeInterval::new_with_data(
        Some(jd(1.0)),
        Some(jd(2.5)),
        Some(true),
        Some(false),
        Some(IntervalData::Number(12.0)),
    );
    let cloned = interval.clone();
    assert!(TimeInterval::equals(&cloned, &interval));
}

// DEVIATION: the JS `clone(result)` out-parameter variant has no Rust
// counterpart; cloning is value-based via `Clone`.

// --- toString ---

#[test]
fn formats_as_iso8601_with_display() {
    let interval = TimeInterval::new(Some(jd(1.0)), Some(jd(2.5)), None, None);
    assert_eq!(interval.to_string(), interval.to_iso8601(None));
}

// --- intersect ---

#[test]
fn intersect_properly_intersects_with_an_exhaustive_set_of_cases() {
    // Mirrors the JS `testParameters` array (triples: left, right, expected).
    let test_parameters: Vec<TimeInterval> = [
        (1.0, 2.5, true, true),
        (1.5, 2.0, true, true),
        (1.5, 2.0, true, true),
        (1.0, 2.5, true, true),
        (3.0, 4.0, true, true),
        (0.0, 0.0, false, false),
        (1.0, 2.5, true, true),
        (2.0, 3.0, true, true),
        (2.0, 2.5, true, true),
        (1.0, 2.0, true, true),
        (1.0, 2.0, false, false),
        (1.0, 2.0, false, false),
        (1.0, 2.0, true, false),
        (1.0, 2.0, false, true),
        (1.0, 2.0, false, false),
        (1.0, 2.0, true, false),
        (1.0, 2.0, true, false),
        (1.0, 2.0, true, false),
        (1.0, 3.0, false, false),
        (2.0, 4.0, false, false),
        (2.0, 3.0, false, false),
        (1.0, 3.0, false, false),
        (2.0, 4.0, true, true),
        (2.0, 3.0, true, false),
        (1.0, 1.0, false, false),
        (1.0, 2.0, true, true),
        (0.0, 0.0, false, false),
        (1.0, 3.0, true, true),
        (2.0, 3.0, true, true),
        (2.0, 3.0, true, true),
        (3.0, 2.0, true, true),
        (3.0, 3.0, true, true),
        (0.0, 0.0, false, false),
    ]
    .iter()
    .map(|(start, stop, is_start_included, is_stop_included)| {
        TimeInterval::new(
            Some(jd(*start)),
            Some(jd(*stop)),
            Some(*is_start_included),
            Some(*is_stop_included),
        )
    })
    .collect();

    let mut i = 0;
    while i + 2 < test_parameters.len() {
        let first = &test_parameters[i];
        let second = &test_parameters[i + 1];
        let expected_result = &test_parameters[i + 2];
        let intersect1 = TimeInterval::intersect(first, second);
        let intersect2 = TimeInterval::intersect(second, first);
        assert!(
            TimeInterval::equals(&intersect1, &intersect2),
            "symmetry at case {i}"
        );
        assert!(
            TimeInterval::equals(expected_result, &intersect1),
            "expected result at case {i}"
        );
        i += 3;
    }
}

// DEVIATION: the JS "intersect with undefined results in an empty interval"
// spec relies on an optional `right` parameter; the Rust port requires both
// arguments, so there is no counterpart.

#[test]
fn intersect_with_a_merge_callback_properly_merges_data() {
    let one_to_three = TimeInterval::new_with_data(
        Some(jd(1.0)),
        Some(jd(3.0)),
        None,
        None,
        Some(IntervalData::Number(2.0)),
    );
    let two_to_four = TimeInterval::new_with_data(
        Some(jd(2.0)),
        Some(jd(4.0)),
        None,
        None,
        Some(IntervalData::Number(3.0)),
    );
    let merge = |left: Option<&IntervalData>, right: Option<&IntervalData>| match (left, right) {
        (Some(IntervalData::Number(left)), Some(IntervalData::Number(right))) => {
            Some(IntervalData::Number(left + right))
        }
        _ => None,
    };
    let two_to_three = TimeInterval::intersect_with_callback(&one_to_three, &two_to_four, Some(&merge));
    assert!(JulianDate::equals(&two_to_three.start, &two_to_four.start));
    assert!(JulianDate::equals(&two_to_three.stop, &one_to_three.stop));
    assert!(two_to_three.is_start_included);
    assert!(two_to_three.is_stop_included);
    assert_eq!(two_to_three.data, Some(IntervalData::Number(5.0)));
}
