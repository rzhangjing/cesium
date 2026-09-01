//! Tests for `cesium_core::TimeIntervalCollection`.
//!
//! Mirrors `packages/engine/Specs/Core/TimeIntervalCollectionSpec.js`.

use std::cell::Cell;

use cesium_core::iso8601::Iso8601;
use cesium_core::julian_date::JulianDate;
use cesium_core::time_interval::{IntervalData, TimeInterval};
use cesium_core::time_interval_collection::{
    FindIntervalOptions, FromIso8601DateArrayOptions, FromIso8601DurationArrayOptions,
    FromIso8601Options, FromJulianDateArrayOptions, TimeIntervalCollection,
};
use cesium_core::time_standard::TimeStandard;

fn jd(days: f64) -> JulianDate {
    JulianDate::new(days, 0.0, TimeStandard::UTC)
}

fn iso8601_to_julian_date_array(iso8601_dates: &[&str]) -> Vec<JulianDate> {
    iso8601_dates
        .iter()
        .map(|date| JulianDate::from_iso8601(date).unwrap())
        .collect()
}

/// Mirrors the JS `checkIntervals` helper.
fn check_intervals(
    intervals: &TimeIntervalCollection,
    julian_dates: &[JulianDate],
    is_start_included: bool,
    is_stop_included: bool,
) {
    let length = intervals.length();
    assert_eq!(length, julian_dates.len() - 1);
    for i in 0..length {
        let interval = intervals.get(i).unwrap();
        assert_eq!(JulianDate::compare(&interval.start, &julian_dates[i]), 0);
        assert_eq!(JulianDate::compare(&interval.stop, &julian_dates[i + 1]), 0);
        assert_eq!(interval.is_start_included, i == 0 && is_start_included || i != 0);
        assert_eq!(
            interval.is_stop_included,
            i != length - 1 && false || i == length - 1 && is_stop_included
        );
        assert_eq!(
            interval.data,
            Some(IntervalData::Number(i as f64)),
            "data at index {i}"
        );
    }
}

fn num(value: f64) -> Option<IntervalData> {
    Some(IntervalData::Number(value))
}

// --- constructor / property getters ---

#[test]
fn constructing_a_default_interval_collection_has_expected_property_values() {
    let intervals = TimeIntervalCollection::new();
    assert_eq!(intervals.length(), 0);
    assert!(intervals.start().is_none());
    assert!(intervals.stop().is_none());
    assert!(!intervals.is_start_included());
    assert!(!intervals.is_stop_included());
    assert!(intervals.is_empty());
    // changedEvent is defined
    assert_eq!(intervals.changed_event().number_of_listeners(), 0);
}

#[test]
fn constructing_an_interval_collection_from_array() {
    let arg = vec![
        TimeInterval::new(Some(jd(1.0)), Some(jd(2.0)), Some(true), Some(false)),
        TimeInterval::new(Some(jd(2.0)), Some(jd(3.0)), Some(false), Some(true)),
    ];
    let expected_start = arg[0].start.clone();
    let expected_stop = arg[1].stop.clone();
    let intervals = TimeIntervalCollection::from_intervals(arg);
    assert_eq!(intervals.length(), 2);
    assert_eq!(intervals.start(), Some(expected_start));
    assert_eq!(intervals.stop(), Some(expected_stop));
    assert!(intervals.is_start_included());
    assert!(intervals.is_stop_included());
    assert!(!intervals.is_empty());
}

#[test]
fn is_start_included_is_stop_included_works() {
    let mut intervals = TimeIntervalCollection::new();
    let interval1 = TimeInterval::new(Some(jd(1.0)), Some(jd(2.0)), Some(true), Some(false));
    let interval2 = TimeInterval::new(Some(jd(2.0)), Some(jd(3.0)), Some(false), Some(true));

    assert!(!intervals.is_start_included());
    assert!(!intervals.is_stop_included());

    intervals.add_interval(interval1);

    assert!(intervals.is_start_included());
    assert!(!intervals.is_stop_included());

    intervals.add_interval(interval2);

    assert!(intervals.is_start_included());
    assert!(intervals.is_stop_included());
}

// --- contains / indexOf ---

#[test]
fn contains_works_for_a_simple_interval_collection() {
    let mut intervals = TimeIntervalCollection::new();
    intervals.add_interval(TimeInterval::new(
        Some(jd(1.0)),
        Some(jd(2.0)),
        Some(true),
        Some(false),
    ));
    intervals.add_interval(TimeInterval::new(
        Some(jd(2.0)),
        Some(jd(3.0)),
        Some(false),
        Some(true),
    ));
    assert!(!intervals.contains(&jd(0.5)));
    assert!(intervals.contains(&jd(1.5)));
    assert!(!intervals.contains(&jd(2.0)));
    assert!(intervals.contains(&jd(2.5)));
    assert!(intervals.contains(&jd(3.0)));
    assert!(!intervals.contains(&jd(3.5)));
}

#[test]
fn contains_works_for_endpoints_of_a_closed_interval_collection() {
    let mut intervals = TimeIntervalCollection::new();
    let interval = TimeInterval::new(Some(jd(1.0)), Some(jd(2.0)), Some(true), Some(true));
    let start = interval.start.clone();
    let stop = interval.stop.clone();
    intervals.add_interval(interval);
    assert!(intervals.contains(&start));
    assert!(intervals.contains(&stop));
}

#[test]
fn contains_works_for_endpoints_of_an_open_interval_collection() {
    let mut intervals = TimeIntervalCollection::new();
    let interval = TimeInterval::new(Some(jd(1.0)), Some(jd(2.0)), Some(false), Some(false));
    let start = interval.start.clone();
    let stop = interval.stop.clone();
    intervals.add_interval(interval);
    assert!(!intervals.contains(&start));
    assert!(!intervals.contains(&stop));
}

#[test]
fn index_of_finds_the_correct_interval_for_a_valid_date() {
    let mut intervals = TimeIntervalCollection::new();
    intervals.add_interval(TimeInterval::new(
        Some(jd(1.0)),
        Some(jd(2.0)),
        Some(true),
        Some(false),
    ));
    intervals.add_interval(TimeInterval::new(
        Some(jd(2.0)),
        Some(jd(3.0)),
        Some(false),
        Some(true),
    ));
    assert_eq!(intervals.index_of(&jd(2.5)), 1);
}

#[test]
fn index_of_returns_complement_of_index_of_the_interval_that_a_missing_date_would_come_before() {
    let mut intervals = TimeIntervalCollection::new();
    intervals.add_interval(TimeInterval::new(
        Some(jd(1.0)),
        Some(jd(2.0)),
        Some(true),
        Some(false),
    ));
    intervals.add_interval(TimeInterval::new(
        Some(jd(2.0)),
        Some(jd(3.0)),
        Some(false),
        Some(true),
    ));
    assert_eq!(intervals.index_of(&jd(2.0)), !1i64);
}

#[test]
fn index_of_returns_complement_of_collection_length_if_the_date_is_after_all_intervals() {
    let mut intervals = TimeIntervalCollection::new();
    intervals.add_interval(TimeInterval::new(
        Some(jd(1.0)),
        Some(jd(2.0)),
        Some(true),
        Some(false),
    ));
    intervals.add_interval(TimeInterval::new(
        Some(jd(2.0)),
        Some(jd(3.0)),
        Some(false),
        Some(true),
    ));
    assert_eq!(intervals.index_of(&jd(4.0)), !2i64);
}

// --- get / findInterval ---

#[test]
fn get_returns_the_interval_at_the_correct_index() {
    let mut intervals = TimeIntervalCollection::new();
    let interval1 = TimeInterval::new(Some(jd(0.0)), Some(jd(1.0)), Some(false), Some(false));
    let interval2 = TimeInterval::new(Some(jd(2.0)), Some(jd(3.0)), Some(false), Some(false));
    let interval3 = TimeInterval::new(Some(jd(4.0)), Some(jd(5.0)), Some(false), Some(false));
    intervals.add_interval(interval1);
    intervals.add_interval(interval2.clone());
    intervals.add_interval(interval3);
    assert!(TimeInterval::equals(intervals.get(1).unwrap(), &interval2));
}

#[test]
fn get_is_none_for_an_out_of_range_index() {
    let intervals = TimeIntervalCollection::new();
    assert!(intervals.get(1).is_none());
}

#[test]
fn find_interval_works_when_looking_for_an_exact_interval() {
    let mut intervals = TimeIntervalCollection::new();
    let interval1 = TimeInterval::new_with_data(
        Some(jd(0.0)),
        Some(jd(1.0)),
        Some(false),
        Some(false),
        num(1.0),
    );
    let interval2 = TimeInterval::new_with_data(
        Some(jd(1.0)),
        Some(jd(2.0)),
        Some(true),
        Some(false),
        num(2.0),
    );
    let interval3 = TimeInterval::new_with_data(
        Some(jd(2.0)),
        Some(jd(3.0)),
        Some(false),
        Some(false),
        num(3.0),
    );
    intervals.add_interval(interval1);
    intervals.add_interval(interval2.clone());
    intervals.add_interval(interval3);
    let found = intervals
        .find_interval(&FindIntervalOptions {
            start: Some(interval2.start.clone()),
            stop: Some(interval2.stop.clone()),
            is_start_included: Some(true),
            is_stop_included: Some(false),
        })
        .unwrap();
    assert!(TimeInterval::equals(&found, &interval2));
}

#[test]
fn find_interval_works_when_you_do_not_care_about_end_points() {
    let mut intervals = TimeIntervalCollection::new();
    let interval1 = TimeInterval::new_with_data(
        Some(jd(0.0)),
        Some(jd(1.0)),
        Some(false),
        Some(false),
        num(1.0),
    );
    let interval2 = TimeInterval::new_with_data(
        Some(jd(1.0)),
        Some(jd(2.0)),
        Some(true),
        Some(false),
        num(2.0),
    );
    let interval3 = TimeInterval::new_with_data(
        Some(jd(2.0)),
        Some(jd(3.0)),
        Some(false),
        Some(false),
        num(3.0),
    );
    intervals.add_interval(interval1);
    intervals.add_interval(interval2.clone());
    intervals.add_interval(interval3);
    let found = intervals
        .find_interval(&FindIntervalOptions {
            start: Some(interval2.start.clone()),
            stop: Some(interval2.stop.clone()),
            ..Default::default()
        })
        .unwrap();
    assert!(TimeInterval::equals(&found, &interval2));
}

// --- start/stop getters / isEmpty / length ---

#[test]
fn get_start_and_get_stop_return_expected_values() {
    let mut intervals = TimeIntervalCollection::new();
    let interval1 = TimeInterval::new(Some(jd(1.0)), Some(jd(2.0)), Some(true), Some(false));
    let interval2 = TimeInterval::new(Some(jd(2.0)), Some(jd(3.0)), Some(true), Some(false));
    let start1 = interval1.start.clone();
    let stop2 = interval2.stop.clone();
    intervals.add_interval(interval1);
    intervals.add_interval(interval2);
    assert_eq!(intervals.start(), Some(start1));
    assert_eq!(intervals.stop(), Some(stop2));
}

#[test]
fn is_empty_and_clear_return_expected_values() {
    let mut intervals = TimeIntervalCollection::new();
    intervals.add_interval(TimeInterval::new(
        Some(jd(1.0)),
        Some(jd(2.0)),
        Some(false),
        Some(true),
    ));
    assert!(!intervals.is_empty());
    intervals.remove_all();
    assert!(intervals.is_empty());
}

#[test]
fn length_returns_the_correct_interval_length_when_adding_intervals_with_different_data() {
    let mut intervals = TimeIntervalCollection::new();
    assert_eq!(intervals.length(), 0);

    intervals.add_interval(TimeInterval::new_with_data(
        Some(jd(1.0)),
        Some(jd(4.0)),
        Some(true),
        Some(true),
        num(1.0),
    ));
    assert_eq!(intervals.length(), 1);

    intervals.add_interval(TimeInterval::new_with_data(
        Some(jd(2.0)),
        Some(jd(3.0)),
        Some(true),
        Some(true),
        num(2.0),
    ));
    assert_eq!(intervals.length(), 3);

    intervals.remove_all();
    assert_eq!(intervals.length(), 0);
}

#[test]
fn length_returns_the_correct_length_after_two_intervals_with_the_same_data_are_merged() {
    let mut intervals = TimeIntervalCollection::new();

    intervals.add_interval(TimeInterval::new_with_data(
        Some(jd(1.0)),
        Some(jd(4.0)),
        Some(true),
        Some(true),
        num(1.0),
    ));
    assert_eq!(intervals.length(), 1);

    intervals.add_interval(TimeInterval::new_with_data(
        Some(jd(2.0)),
        Some(jd(3.0)),
        Some(true),
        Some(true),
        num(1.0),
    ));
    assert_eq!(intervals.length(), 1);

    intervals.remove_all();
    assert_eq!(intervals.length(), 0);
}

// --- addInterval / findIntervalContainingDate / findDataForIntervalContainingDate ---

#[test]
fn add_interval_and_find_interval_containing_date_work_when_using_non_overlapping_intervals() {
    let interval1 = TimeInterval::new_with_data(
        Some(jd(1.0)),
        Some(jd(2.0)),
        Some(true),
        Some(true),
        num(1.0),
    );
    let interval2 = TimeInterval::new_with_data(
        Some(jd(2.0)),
        Some(jd(3.0)),
        Some(false),
        Some(true),
        num(2.0),
    );
    let interval3 = TimeInterval::new_with_data(
        Some(jd(4.0)),
        Some(jd(5.0)),
        Some(true),
        Some(true),
        num(3.0),
    );

    let mut intervals = TimeIntervalCollection::new();

    intervals.add_interval(interval1.clone());
    assert_eq!(intervals.length(), 1);
    assert_eq!(intervals.start(), Some(interval1.start.clone()));
    assert_eq!(intervals.stop(), Some(interval1.stop.clone()));
    assert!(!intervals.is_empty());

    assert!(TimeInterval::equals(
        &intervals
            .find_interval_containing_date(&interval1.start)
            .unwrap(),
        &interval1
    ));
    assert!(TimeInterval::equals(
        &intervals
            .find_interval_containing_date(&interval1.stop)
            .unwrap(),
        &interval1
    ));

    intervals.add_interval(interval2.clone());

    assert_eq!(intervals.length(), 2);
    assert_eq!(intervals.start(), Some(interval1.start.clone()));
    assert_eq!(intervals.stop(), Some(interval2.stop.clone()));
    assert!(!intervals.is_empty());

    assert!(TimeInterval::equals(
        &intervals
            .find_interval_containing_date(&interval2.stop)
            .unwrap(),
        &interval2
    ));

    intervals.add_interval(interval3.clone());
    assert_eq!(intervals.length(), 3);
    assert_eq!(intervals.start(), Some(interval1.start.clone()));
    assert_eq!(intervals.stop(), Some(interval3.stop.clone()));
    assert!(!intervals.is_empty());

    assert!(TimeInterval::equals(
        &intervals
            .find_interval_containing_date(&interval3.start)
            .unwrap(),
        &interval3
    ));
    assert!(TimeInterval::equals(
        &intervals
            .find_interval_containing_date(&interval3.stop)
            .unwrap(),
        &interval3
    ));
}

#[test]
fn add_interval_and_find_interval_containing_date_work_when_using_overlapping_intervals() {
    let interval1 = TimeInterval::new_with_data(
        Some(jd(1.0)),
        Some(jd(2.5)),
        Some(true),
        Some(true),
        num(1.0),
    );
    let interval2 = TimeInterval::new_with_data(
        Some(jd(2.0)),
        Some(jd(3.0)),
        Some(false),
        Some(true),
        num(2.0),
    );
    let interval3 = TimeInterval::new_with_data(
        Some(interval1.start.clone()),
        Some(interval2.stop.clone()),
        Some(true),
        Some(true),
        num(3.0),
    );

    let mut intervals = TimeIntervalCollection::new();

    intervals.add_interval(interval1.clone());
    assert_eq!(intervals.length(), 1);

    assert_eq!(
        intervals
            .find_interval_containing_date(&interval1.start)
            .unwrap()
            .data,
        num(1.0)
    );
    assert_eq!(
        intervals
            .find_interval_containing_date(&interval1.stop)
            .unwrap()
            .data,
        num(1.0)
    );

    intervals.add_interval(interval2.clone());

    assert_eq!(intervals.length(), 2);
    assert_eq!(intervals.start(), Some(interval1.start.clone()));
    assert_eq!(intervals.stop(), Some(interval2.stop.clone()));

    assert_eq!(
        intervals
            .find_interval_containing_date(&interval1.start)
            .unwrap()
            .data,
        num(1.0)
    );
    assert_eq!(
        intervals
            .find_interval_containing_date(&interval1.stop)
            .unwrap()
            .data,
        num(2.0)
    );
    assert_eq!(
        intervals
            .find_interval_containing_date(&interval2.stop)
            .unwrap()
            .data,
        num(2.0)
    );

    intervals.add_interval(interval3.clone());
    assert_eq!(intervals.length(), 1);
    assert_eq!(intervals.start(), Some(interval3.start.clone()));
    assert_eq!(intervals.stop(), Some(interval3.stop.clone()));

    for date in [
        &interval1.start,
        &interval1.stop,
        &interval2.start,
        &interval2.stop,
        &interval3.start,
        &interval3.stop,
    ] {
        assert_eq!(
            intervals.find_interval_containing_date(date).unwrap().data,
            num(3.0)
        );
    }
}

#[test]
fn find_data_for_interval_containing_date_works() {
    let interval1 = TimeInterval::new_with_data(
        Some(jd(1.0)),
        Some(jd(2.5)),
        Some(true),
        Some(true),
        num(1.0),
    );
    let interval2 = TimeInterval::new_with_data(
        Some(jd(2.0)),
        Some(jd(3.0)),
        Some(false),
        Some(true),
        num(2.0),
    );

    let mut intervals = TimeIntervalCollection::new();
    intervals.add_interval(interval1.clone());
    assert_eq!(
        intervals.find_data_for_interval_containing_date(&interval1.start),
        num(1.0)
    );
    assert_eq!(
        intervals.find_data_for_interval_containing_date(&interval1.stop),
        num(1.0)
    );

    intervals.add_interval(interval2.clone());
    assert_eq!(
        intervals.find_data_for_interval_containing_date(&interval1.start),
        num(1.0)
    );
    assert_eq!(
        intervals.find_data_for_interval_containing_date(&interval1.stop),
        num(2.0)
    );
    assert_eq!(
        intervals.find_data_for_interval_containing_date(&interval2.stop),
        num(2.0)
    );

    assert_eq!(
        intervals.find_data_for_interval_containing_date(&jd(5.0)),
        None
    );
}

#[test]
fn add_interval_correctly_merges_intervals_that_have_the_same_data_when_using_equals_callback() {
    let mut intervals = TimeIntervalCollection::new();

    // JS TestObject(value) with TestObject.equals/merge — mirrored with
    // IntervalData::Number payloads and value closures.
    let equals = |_left: Option<&IntervalData>, right: Option<&IntervalData>| -> bool {
        // compares values like TestObject.equals (both defined here)
        _left == right
    };

    let interval1 = TimeInterval::new_with_data(
        Some(jd(1.0)),
        Some(jd(4.0)),
        Some(true),
        Some(true),
        num(2.0),
    );
    let interval2 = TimeInterval::new_with_data(
        Some(jd(1.0)),
        Some(jd(3.0)),
        Some(true),
        Some(false),
        num(2.0),
    );
    let interval3 = TimeInterval::new_with_data(
        Some(jd(3.0)),
        Some(jd(4.0)),
        Some(false),
        Some(true),
        num(2.0),
    );
    let interval4 = TimeInterval::new_with_data(
        Some(jd(3.0)),
        Some(jd(4.0)),
        Some(true),
        Some(true),
        num(3.0),
    );

    intervals.add_interval_with(interval1.clone(), Some(&equals));
    assert_eq!(intervals.length(), 1);
    assert_eq!(intervals.start(), Some(interval1.start.clone()));
    assert_eq!(intervals.stop(), Some(interval1.stop.clone()));
    assert_eq!(intervals.get(0).unwrap().data, num(2.0));

    intervals.add_interval_with(interval2, Some(&equals));
    assert_eq!(intervals.length(), 1);
    assert_eq!(intervals.get(0).unwrap().data, num(2.0));

    intervals.add_interval_with(interval3, Some(&equals));
    assert_eq!(intervals.length(), 1);
    assert_eq!(intervals.get(0).unwrap().data, num(2.0));

    intervals.add_interval_with(interval4.clone(), Some(&equals));
    assert_eq!(intervals.length(), 2);
    assert_eq!(
        intervals.get(0).unwrap().stop,
        interval4.start
    );
    assert!(intervals.get(0).unwrap().is_start_included);
    assert!(!intervals.get(0).unwrap().is_stop_included);
    assert_eq!(intervals.get(0).unwrap().data, num(2.0));

    assert_eq!(intervals.get(1).unwrap().start, interval4.start);
    assert_eq!(intervals.get(1).unwrap().stop, interval4.stop);
    assert!(intervals.get(1).unwrap().is_start_included);
    assert!(intervals.get(1).unwrap().is_stop_included);
    assert_eq!(intervals.get(1).unwrap().data, num(3.0));
}

// --- removeInterval ---

fn create_time_interval(start_days: f64, stop_days: f64) -> TimeInterval {
    TimeInterval::new(
        Some(JulianDate::new(start_days, 0.0, TimeStandard::TAI)),
        Some(JulianDate::new(stop_days, 0.0, TimeStandard::TAI)),
        None,
        None,
    )
}

fn create_time_interval_included(
    start_days: f64,
    stop_days: f64,
    is_start_included: bool,
    is_stop_included: bool,
) -> TimeInterval {
    TimeInterval::new(
        Some(JulianDate::new(start_days, 0.0, TimeStandard::TAI)),
        Some(JulianDate::new(stop_days, 0.0, TimeStandard::TAI)),
        Some(is_start_included),
        Some(is_stop_included),
    )
}

#[test]
fn remove_interval_works_correctly() {
    // test cases derived from STK Components test suite

    let mut intervals = TimeIntervalCollection::new();
    intervals.add_interval(create_time_interval(10.0, 20.0));
    intervals.add_interval(create_time_interval(30.0, 40.0));

    // Empty
    assert!(!intervals.remove_interval(&TimeInterval::empty()));
    assert_eq!(intervals.length(), 2);

    // Before first
    assert!(!intervals.remove_interval(&create_time_interval(1.0, 5.0)));
    assert_eq!(intervals.length(), 2);

    // After last
    assert!(!intervals.remove_interval(&create_time_interval(50.0, 60.0)));
    assert_eq!(intervals.length(), 2);

    // Inside hole
    assert!(!intervals.remove_interval(&create_time_interval(22.0, 28.0)));
    assert_eq!(intervals.length(), 2);

    // From beginning
    assert!(intervals.remove_interval(&create_time_interval(5.0, 15.0)));
    assert_eq!(intervals.length(), 2);
    assert_eq!(JulianDate::total_days(&intervals.get(0).unwrap().start), 15.0);
    assert_eq!(JulianDate::total_days(&intervals.get(0).unwrap().stop), 20.0);

    // From end
    assert!(intervals.remove_interval(&create_time_interval(35.0, 45.0)));
    assert_eq!(intervals.length(), 2);
    assert_eq!(JulianDate::total_days(&intervals.get(1).unwrap().start), 30.0);
    assert_eq!(JulianDate::total_days(&intervals.get(1).unwrap().stop), 35.0);

    intervals.remove_all();
    intervals.add_interval(create_time_interval(10.0, 20.0));
    intervals.add_interval(create_time_interval(30.0, 40.0));

    // From middle of single interval
    assert!(intervals.remove_interval(&create_time_interval(12.0, 18.0)));
    assert_eq!(intervals.length(), 3);
    assert_eq!(JulianDate::total_days(&intervals.get(0).unwrap().stop), 12.0);
    assert!(!intervals.get(0).unwrap().is_stop_included);
    assert_eq!(JulianDate::total_days(&intervals.get(1).unwrap().start), 18.0);
    assert!(!intervals.get(1).unwrap().is_start_included);

    intervals.remove_all();
    intervals.add_interval(create_time_interval(10.0, 20.0));
    intervals.add_interval(create_time_interval(30.0, 40.0));
    intervals.add_interval(create_time_interval(45.0, 50.0));

    // Span an entire interval and into part of next
    assert!(intervals.remove_interval(&create_time_interval(25.0, 46.0)));
    assert_eq!(intervals.length(), 2);
    assert_eq!(JulianDate::total_days(&intervals.get(1).unwrap().start), 46.0);
    assert!(!intervals.get(1).unwrap().is_start_included);

    intervals.remove_all();
    intervals.add_interval(create_time_interval(10.0, 20.0));
    intervals.add_interval(create_time_interval(30.0, 40.0));
    intervals.add_interval(create_time_interval(45.0, 50.0));

    // Interval ends at same date as an existing interval
    assert!(intervals.remove_interval(&create_time_interval(25.0, 40.0)));
    assert_eq!(intervals.length(), 2);
    assert_eq!(JulianDate::total_days(&intervals.get(0).unwrap().stop), 20.0);
    assert_eq!(JulianDate::total_days(&intervals.get(1).unwrap().start), 45.0);

    intervals.remove_all();
    intervals.add_interval(create_time_interval(10.0, 20.0));
    intervals.add_interval(create_time_interval(30.0, 40.0));
    intervals.add_interval(create_time_interval(45.0, 50.0));

    // Interval ends at same date as an existing interval and single point of
    // existing interval survives.
    assert!(intervals.remove_interval(&create_time_interval_included(25.0, 40.0, true, false)));
    assert_eq!(intervals.length(), 3);
    assert_eq!(JulianDate::total_days(&intervals.get(0).unwrap().stop), 20.0);
    assert_eq!(JulianDate::total_days(&intervals.get(1).unwrap().start), 40.0);
    assert_eq!(JulianDate::total_days(&intervals.get(1).unwrap().stop), 40.0);
    assert!(intervals.get(1).unwrap().is_start_included);
    assert!(intervals.get(1).unwrap().is_stop_included);
    assert_eq!(JulianDate::total_days(&intervals.get(2).unwrap().start), 45.0);

    intervals.remove_all();
    intervals.add_interval(create_time_interval(10.0, 20.0));
    intervals.add_interval(create_time_interval(30.0, 40.0));
    intervals.add_interval(create_time_interval_included(40.0, 50.0, false, true));

    // Interval ends at same date as an existing interval, single point of
    // existing interval survives, and single point can be combined with the
    // next interval.
    assert!(intervals.remove_interval(&create_time_interval_included(25.0, 40.0, true, false)));
    assert_eq!(intervals.length(), 2);
    assert_eq!(JulianDate::total_days(&intervals.get(0).unwrap().stop), 20.0);
    assert_eq!(JulianDate::total_days(&intervals.get(1).unwrap().start), 40.0);
    assert!(intervals.get(1).unwrap().is_start_included);

    intervals.remove_all();
    intervals.add_interval(create_time_interval(10.0, 20.0));

    // End point of removal interval overlaps first point of existing interval.
    assert!(intervals.remove_interval(&create_time_interval(0.0, 10.0)));
    assert_eq!(intervals.length(), 1);
    assert_eq!(JulianDate::total_days(&intervals.get(0).unwrap().start), 10.0);
    assert_eq!(JulianDate::total_days(&intervals.get(0).unwrap().stop), 20.0);
    assert!(!intervals.get(0).unwrap().is_start_included);
    assert!(intervals.get(0).unwrap().is_stop_included);

    intervals.remove_all();
    intervals.add_interval(create_time_interval(10.0, 20.0));

    // Start point of removal interval does NOT overlap last point of
    // existing interval because the start point is not included.
    assert!(!intervals.remove_interval(&create_time_interval_included(20.0, 30.0, false, true)));
    assert_eq!(intervals.length(), 1);
    assert_eq!(JulianDate::total_days(&intervals.get(0).unwrap().start), 10.0);
    assert_eq!(JulianDate::total_days(&intervals.get(0).unwrap().stop), 20.0);
    assert!(intervals.get(0).unwrap().is_start_included);
    assert!(intervals.get(0).unwrap().is_stop_included);

    // Removing an open interval from an otherwise identical closed interval
    intervals.remove_all();
    intervals.add_interval(create_time_interval(0.0, 20.0));
    assert!(intervals.remove_interval(&create_time_interval_included(0.0, 20.0, false, false)));
    assert_eq!(intervals.length(), 2);
    assert_eq!(JulianDate::total_days(&intervals.get(0).unwrap().start), 0.0);
    assert_eq!(JulianDate::total_days(&intervals.get(0).unwrap().stop), 0.0);
    assert!(intervals.get(0).unwrap().is_start_included);
    assert!(intervals.get(0).unwrap().is_stop_included);
    assert_eq!(JulianDate::total_days(&intervals.get(1).unwrap().start), 20.0);
    assert_eq!(JulianDate::total_days(&intervals.get(1).unwrap().stop), 20.0);
    assert!(intervals.get(1).unwrap().is_start_included);
    assert!(intervals.get(1).unwrap().is_stop_included);
}

#[test]
fn remove_interval_removes_the_first_interval_correctly() {
    let mut intervals = TimeIntervalCollection::new();
    let from_1_to_3 = TimeInterval::new_with_data(
        Some(jd(1.0)),
        Some(jd(3.0)),
        Some(true),
        Some(true),
        Some(IntervalData::Text("1-to-3".to_owned())),
    );
    let from_3_to_6 = TimeInterval::new_with_data(
        Some(jd(3.0)),
        Some(jd(6.0)),
        Some(true),
        Some(true),
        Some(IntervalData::Text("3-to-6".to_owned())),
    );

    intervals.add_interval(from_1_to_3);
    intervals.add_interval(from_3_to_6);

    assert_eq!(intervals.length(), 2);
    assert!(intervals.get(0).unwrap().is_start_included);
    // changed to false because 3-6 overlaps it
    assert!(!intervals.get(0).unwrap().is_stop_included);
    assert_eq!(intervals.get(0).unwrap().start.day_number, 1);
    assert_eq!(intervals.get(0).unwrap().stop.day_number, 3);
    assert_eq!(
        intervals.get(0).unwrap().data,
        Some(IntervalData::Text("1-to-3".to_owned()))
    );
    assert!(intervals.get(1).unwrap().is_start_included);
    assert!(intervals.get(1).unwrap().is_stop_included);
    assert_eq!(intervals.get(1).unwrap().start.day_number, 3);
    assert_eq!(intervals.get(1).unwrap().stop.day_number, 6);
    assert_eq!(
        intervals.get(1).unwrap().data,
        Some(IntervalData::Text("3-to-6".to_owned()))
    );

    let to_remove = TimeInterval::new(Some(jd(1.0)), Some(jd(3.0)), Some(true), Some(true));

    assert!(intervals.remove_interval(&to_remove));
    assert_eq!(intervals.length(), 1);
    assert_eq!(intervals.start().unwrap().day_number, 3);
    assert_eq!(intervals.stop().unwrap().day_number, 6);
    assert_eq!(intervals.get(0).unwrap().start.day_number, 3);
    assert_eq!(intervals.get(0).unwrap().stop.day_number, 6);
    assert!(!intervals.get(0).unwrap().is_start_included);
    assert!(intervals.get(0).unwrap().is_stop_included);
    assert_eq!(
        intervals.get(0).unwrap().data,
        Some(IntervalData::Text("3-to-6".to_owned()))
    );
}

#[test]
fn should_add_and_remove_intervals_correctly_integration() {
    // about the year 3000
    const CONST_DAY_NUM: f64 = 3000000.0;

    fn interval_from_seconds(seconds: f64, data: f64) -> TimeInterval {
        // make all intervals a few seconds in length
        TimeInterval::new_with_data(
            Some(JulianDate::new(CONST_DAY_NUM, seconds, TimeStandard::UTC)),
            Some(JulianDate::new(CONST_DAY_NUM, seconds + 4.0, TimeStandard::UTC)),
            Some(true),
            Some(true),
            num(data),
        )
    }

    fn add_intervals(collection: &mut TimeIntervalCollection, specs: &[(f64, f64)]) {
        for (sec, data) in specs {
            collection.add_interval(interval_from_seconds(*sec, *data));
        }
    }

    fn remove_interval(collection: &mut TimeIntervalCollection, from_second: f64, to_second: f64) {
        collection.remove_interval(&TimeInterval::new(
            Some(JulianDate::new(CONST_DAY_NUM, from_second, TimeStandard::UTC)),
            Some(JulianDate::new(CONST_DAY_NUM, to_second, TimeStandard::UTC)),
            Some(true),
            Some(true),
        ));
    }

    fn expect_collection(
        collection: &TimeIntervalCollection,
        count: usize,
        expectation: &[(f64, Option<f64>)],
    ) {
        for (sec, data) in expectation {
            let interval = collection
                .find_interval_containing_date(&JulianDate::new(CONST_DAY_NUM, *sec, TimeStandard::UTC));
            match (data, interval) {
                (None, None) => {}
                (None, Some(interval)) => {
                    panic!("expected None at {sec} seconds but it was {:?}", interval.data)
                }
                (Some(data), None) => {
                    panic!("expected {data} at {sec} seconds, but it was None")
                }
                (Some(data), Some(interval)) => {
                    assert_eq!(interval.data, num(*data), "at {sec} seconds");
                }
            }
        }
        assert_eq!(collection.length(), count);
    }

    let mut collection = TimeIntervalCollection::new();

    add_intervals(&mut collection, &[(0.0, 0.0), (2.0, 2.0), (4.0, 4.0), (6.0, 6.0)]);
    expect_collection(
        &collection,
        4,
        &[
            (0.0, Some(0.0)),
            (1.0, Some(0.0)),
            (2.0, Some(2.0)),
            (3.0, Some(2.0)),
            (4.0, Some(4.0)),
            (5.0, Some(4.0)),
            (6.0, Some(6.0)),
            (7.0, Some(6.0)),
            (8.0, Some(6.0)),
            (9.0, Some(6.0)),
            (10.0, Some(6.0)),
            (11.0, None),
        ],
    );

    add_intervals(&mut collection, &[(1.0, 1.0), (3.0, 3.0)]);
    expect_collection(
        &collection,
        4,
        &[
            (0.0, Some(0.0)),
            (1.0, Some(1.0)),
            (2.0, Some(1.0)),
            (3.0, Some(3.0)),
            (4.0, Some(3.0)),
            (5.0, Some(3.0)),
            (6.0, Some(3.0)),
            (7.0, Some(3.0)),
            (8.0, Some(6.0)),
            (9.0, Some(6.0)),
            (10.0, Some(6.0)),
            (11.0, None),
        ],
    );

    add_intervals(&mut collection, &[(3.0, 31.0)]);
    expect_collection(
        &collection,
        4,
        &[
            (0.0, Some(0.0)),
            (1.0, Some(1.0)),
            (2.0, Some(1.0)),
            (3.0, Some(31.0)),
            (4.0, Some(31.0)),
            (5.0, Some(31.0)),
            (6.0, Some(31.0)),
            (7.0, Some(31.0)),
            (8.0, Some(6.0)),
            (9.0, Some(6.0)),
            (10.0, Some(6.0)),
            (11.0, None),
        ],
    );

    remove_interval(&mut collection, 3.0, 8.0);
    expect_collection(
        &collection,
        3,
        &[
            (0.0, Some(0.0)),
            (1.0, Some(1.0)),
            (2.0, Some(1.0)),
            (3.0, None),
            (4.0, None),
            (5.0, None),
            (6.0, None),
            (7.0, None),
            (8.0, None),
            (9.0, Some(6.0)),
            (10.0, Some(6.0)),
            (11.0, None),
        ],
    );

    remove_interval(&mut collection, 0.0, 1.0);
    expect_collection(
        &collection,
        2,
        &[
            (0.0, None),
            (1.0, None),
            (2.0, Some(1.0)),
            (9.0, Some(6.0)),
            (10.0, Some(6.0)),
            (11.0, None),
        ],
    );

    remove_interval(&mut collection, 0.0, 11.0);
    expect_collection(&collection, 0, &[(0.0, None), (11.0, None)]);

    add_intervals(&mut collection, &[(1.0, 1.0), (12.0, 12.0)]);
    expect_collection(
        &collection,
        2,
        &[
            (0.0, None),
            (1.0, Some(1.0)),
            (2.0, Some(1.0)),
            (3.0, Some(1.0)),
            (4.0, Some(1.0)),
            (5.0, Some(1.0)),
            (6.0, None),
            (11.0, None),
            (12.0, Some(12.0)),
            (13.0, Some(12.0)),
            (14.0, Some(12.0)),
            (15.0, Some(12.0)),
            (16.0, Some(12.0)),
            (17.0, None),
        ],
    );

    remove_interval(&mut collection, 0.0, 3.0);
    expect_collection(
        &collection,
        2,
        &[
            (0.0, None),
            (1.0, None),
            (2.0, None),
            (3.0, None),
            (4.0, Some(1.0)),
            (5.0, Some(1.0)),
            (6.0, None),
            (12.0, Some(12.0)),
            (16.0, Some(12.0)),
            (17.0, None),
        ],
    );
}

#[test]
fn remove_interval_leaves_a_hole() {
    let mut intervals = TimeIntervalCollection::new();
    let interval = TimeInterval::new(Some(jd(1.0)), Some(jd(4.0)), Some(true), Some(true));
    let removed_interval = TimeInterval::new(Some(jd(2.0)), Some(jd(3.0)), Some(true), Some(false));
    intervals.add_interval(interval.clone());
    assert!(intervals.remove_interval(&removed_interval));

    assert_eq!(intervals.length(), 2);
    assert_eq!(intervals.get(0).unwrap().start, interval.start);
    assert_eq!(intervals.get(0).unwrap().stop, removed_interval.start);
    assert!(intervals.get(0).unwrap().is_start_included);
    assert!(!intervals.get(0).unwrap().is_stop_included);

    assert_eq!(intervals.get(1).unwrap().start, removed_interval.stop);
    assert_eq!(intervals.get(1).unwrap().stop, interval.stop);
    assert!(intervals.get(1).unwrap().is_start_included);
    assert!(intervals.get(1).unwrap().is_stop_included);
}

#[test]
fn remove_interval_with_an_interval_of_the_exact_same_size_works() {
    let mut intervals = TimeIntervalCollection::new();
    let interval = TimeInterval::new(Some(jd(1.0)), Some(jd(4.0)), Some(true), Some(false));

    intervals.add_interval(interval.clone());
    assert_eq!(intervals.length(), 1);

    intervals.remove_interval(&interval);
    assert_eq!(intervals.length(), 0);
}

#[test]
fn remove_interval_with_an_empty_interval_has_no_effect() {
    let mut intervals = TimeIntervalCollection::new();
    let interval = TimeInterval::new(Some(jd(1.0)), Some(jd(4.0)), Some(true), Some(true));
    intervals.add_interval(interval.clone());

    assert_eq!(intervals.length(), 1);

    assert!(!intervals.remove_interval(&TimeInterval::empty()));

    assert_eq!(intervals.length(), 1);
    assert_eq!(intervals.get(0).unwrap().start, interval.start);
    assert_eq!(intervals.get(0).unwrap().stop, interval.stop);
    assert!(intervals.get(0).unwrap().is_start_included);
    assert!(intervals.get(0).unwrap().is_stop_included);
}

#[test]
fn remove_interval_takes_is_start_included_and_is_stop_included_into_account() {
    let mut intervals = TimeIntervalCollection::new();

    let interval = TimeInterval::new(Some(jd(1.0)), Some(jd(4.0)), Some(true), Some(true));
    let removed_interval = TimeInterval::new(Some(jd(1.0)), Some(jd(4.0)), Some(false), Some(false));
    intervals.add_interval(interval.clone());
    assert!(intervals.remove_interval(&removed_interval));

    assert_eq!(intervals.length(), 2);
    assert_eq!(intervals.get(0).unwrap().start, interval.start);
    assert_eq!(intervals.get(0).unwrap().stop, interval.start);
    assert!(intervals.get(0).unwrap().is_start_included);
    assert!(intervals.get(0).unwrap().is_stop_included);

    assert_eq!(intervals.get(1).unwrap().start, interval.stop);
    assert_eq!(intervals.get(1).unwrap().stop, interval.stop);
    assert!(intervals.get(1).unwrap().is_start_included);
    assert!(intervals.get(1).unwrap().is_stop_included);
}

#[test]
fn remove_interval_removes_overlapped_intervals() {
    let mut intervals = TimeIntervalCollection::new();

    intervals.add_interval(TimeInterval::new(
        Some(jd(1.0)),
        Some(jd(2.0)),
        Some(true),
        Some(false),
    ));
    intervals.add_interval(TimeInterval::new(
        Some(jd(2.0)),
        Some(jd(3.0)),
        Some(false),
        Some(false),
    ));
    intervals.add_interval(TimeInterval::new(
        Some(jd(3.0)),
        Some(jd(4.0)),
        Some(false),
        Some(false),
    ));
    intervals.add_interval(TimeInterval::new(
        Some(jd(4.0)),
        Some(jd(5.0)),
        Some(false),
        Some(true),
    ));

    let removed_interval = TimeInterval::new(Some(jd(2.0)), Some(jd(4.0)), Some(false), Some(false));

    assert_eq!(intervals.length(), 4);
    assert!(intervals.remove_interval(&removed_interval));

    assert_eq!(intervals.length(), 2);
}

// --- intersect ---

#[test]
fn intersect_works_with_an_empty_collection() {
    let mut left = TimeIntervalCollection::new();
    left.add_interval(TimeInterval::new(
        Some(jd(1.0)),
        Some(jd(4.0)),
        Some(true),
        Some(true),
    ));
    assert_eq!(left.intersect(&TimeIntervalCollection::new(), None, None).length(), 0);
}

#[test]
fn intersect_works_with_non_overlapping_intervals() {
    let mut left = TimeIntervalCollection::new();
    left.add_interval(TimeInterval::new(
        Some(jd(1.0)),
        Some(jd(2.0)),
        Some(true),
        Some(false),
    ));

    let mut right = TimeIntervalCollection::new();
    right.add_interval(TimeInterval::new(
        Some(jd(2.0)),
        Some(jd(3.0)),
        Some(true),
        Some(true),
    ));
    assert_eq!(left.intersect(&right, None, None).length(), 0);
}

#[test]
fn intersect_works_with_intersecting_intervals_and_no_merge_callback() {
    let mut left = TimeIntervalCollection::new();
    left.add_interval(TimeInterval::new(
        Some(jd(1.0)),
        Some(jd(4.0)),
        Some(true),
        Some(true),
    ));

    let mut right = TimeIntervalCollection::new();
    right.add_interval(TimeInterval::new(
        Some(jd(2.0)),
        Some(jd(3.0)),
        Some(false),
        Some(false),
    ));

    let intersected_intervals = left.intersect(&right, None, None);

    assert_eq!(intersected_intervals.length(), 1);
    assert_eq!(
        intersected_intervals.get(0).unwrap().start,
        right.get(0).unwrap().start
    );
    assert_eq!(
        intersected_intervals.get(0).unwrap().stop,
        right.get(0).unwrap().stop
    );
    assert!(!intersected_intervals.get(0).unwrap().is_start_included);
    assert!(!intersected_intervals.get(0).unwrap().is_stop_included);
}

#[test]
fn intersect_works_with_intersecting_intervals_and_a_merge_callback() {
    let mut left = TimeIntervalCollection::new();
    left.add_interval(TimeInterval::new_with_data(
        Some(jd(1.0)),
        Some(jd(4.0)),
        Some(true),
        Some(true),
        num(1.0),
    ));

    let mut right = TimeIntervalCollection::new();
    right.add_interval(TimeInterval::new_with_data(
        Some(jd(2.0)),
        Some(jd(3.0)),
        Some(false),
        Some(false),
        num(2.0),
    ));

    // JS TestObject.equals / TestObject.merge mirrored with numbers.
    let equals = |left: Option<&IntervalData>, right: Option<&IntervalData>| -> bool {
        left == right
    };
    let merge = |left: Option<&IntervalData>, right: Option<&IntervalData>| -> Option<IntervalData> {
        match (left, right) {
            (Some(IntervalData::Number(l)), Some(IntervalData::Number(r))) => {
                Some(IntervalData::Number(l + r))
            }
            _ => None,
        }
    };

    let intersected_intervals = left.intersect(&right, Some(&equals), Some(&merge));

    assert_eq!(intersected_intervals.length(), 1);
    assert_eq!(intersected_intervals.get(0).unwrap().start, right.start().unwrap());
    assert_eq!(intersected_intervals.get(0).unwrap().stop, right.stop().unwrap());
    assert!(!intersected_intervals.get(0).unwrap().is_start_included);
    assert!(!intersected_intervals.get(0).unwrap().is_stop_included);
    assert_eq!(intersected_intervals.get(0).unwrap().data, num(3.0));
}

// --- equals ---

#[test]
fn equals_works_without_data() {
    let interval1 = TimeInterval::new(Some(jd(1.0)), Some(jd(2.0)), Some(true), Some(true));
    let interval2 = TimeInterval::new(Some(jd(2.0)), Some(jd(3.0)), Some(false), Some(true));
    let interval3 = TimeInterval::new(Some(jd(4.0)), Some(jd(5.0)), Some(true), Some(true));

    let mut left = TimeIntervalCollection::new();
    left.add_interval(interval1.clone());
    left.add_interval(interval2.clone());
    left.add_interval(interval3.clone());

    let mut right = TimeIntervalCollection::new();
    right.add_interval(interval1);
    right.add_interval(interval2);
    right.add_interval(interval3);
    assert!(left.equals(&right, None));
}

#[test]
fn equals_works_with_data() {
    // JS uses distinct `{}` objects (reference inequality) — mirrored with
    // distinct IntervalData::object() identities.
    let mut left = TimeIntervalCollection::new();
    left.add_interval(TimeInterval::new_with_data(
        Some(jd(1.0)),
        Some(jd(2.0)),
        Some(true),
        Some(true),
        Some(IntervalData::object()),
    ));
    left.add_interval(TimeInterval::new_with_data(
        Some(jd(2.0)),
        Some(jd(3.0)),
        Some(false),
        Some(true),
        Some(IntervalData::object()),
    ));
    left.add_interval(TimeInterval::new_with_data(
        Some(jd(4.0)),
        Some(jd(5.0)),
        Some(true),
        Some(true),
        Some(IntervalData::object()),
    ));

    let mut right = TimeIntervalCollection::new();
    right.add_interval(TimeInterval::new_with_data(
        Some(jd(1.0)),
        Some(jd(2.0)),
        Some(true),
        Some(true),
        Some(IntervalData::object()),
    ));
    right.add_interval(TimeInterval::new_with_data(
        Some(jd(2.0)),
        Some(jd(3.0)),
        Some(false),
        Some(true),
        Some(IntervalData::object()),
    ));
    right.add_interval(TimeInterval::new_with_data(
        Some(jd(4.0)),
        Some(jd(5.0)),
        Some(true),
        Some(true),
        Some(IntervalData::object()),
    ));

    assert!(!left.equals(&right, None));

    let return_true = |_left: Option<&IntervalData>, _right: Option<&IntervalData>| true;
    assert!(left.equals(&right, Some(&return_true)));

    let return_false = |_left: Option<&IntervalData>, _right: Option<&IntervalData>| false;
    assert!(!left.equals(&right, Some(&return_false)));
}

// --- changedEvent ---

#[test]
fn changed_event_is_raised_as_expected() {
    let interval = TimeInterval::new(Some(jd(10.0)), Some(jd(12.0)), None, None);

    let mut intervals = TimeIntervalCollection::new();

    let count = std::rc::Rc::new(Cell::new(0));
    let count_for_listener = count.clone();
    intervals
        .changed_event()
        .add_listener(move |_arg: &()| count_for_listener.set(count_for_listener.get() + 1));

    intervals.add_interval(interval.clone());
    assert_eq!(count.get(), 1);
    count.set(0);

    intervals.remove_interval(&interval);
    assert_eq!(count.get(), 1);

    intervals.add_interval(interval);
    count.set(0);
    intervals.remove_all();
    assert_eq!(count.get(), 1);
}

// --- fromIso8601 ---

#[test]
fn from_iso8601_returns_single_interval_if_no_duration() {
    let start = "2017-01-01T00:00:00Z";
    let stop = "2017-01-02T00:00:00Z";
    let julian_dates = iso8601_to_julian_date_array(&[start, stop]);

    let intervals = TimeIntervalCollection::from_iso8601(
        FromIso8601Options {
            iso8601: &format!("{start}/{stop}"),
            is_start_included: Some(false),
            is_stop_included: Some(false),
            ..default_iso8601_options(&format!("{start}/{stop}"))
        },
        None,
    );

    check_intervals(&intervals, &julian_dates, false, false);
}

fn default_iso8601_options(_iso8601: &str) -> FromIso8601Options<'static> {
    FromIso8601Options {
        iso8601: "",
        is_start_included: None,
        is_stop_included: None,
        leading_interval: false,
        trailing_interval: false,
        data_callback: None,
    }
}

#[test]
fn from_iso8601_works_with_just_year() {
    let iso8601_dates = [
        "2017-01-01T00:00:00Z",
        "2018-01-01T00:00:00Z",
        "2019-01-01T00:00:00Z",
        "2020-01-01T00:00:00Z",
    ];
    let julian_dates = iso8601_to_julian_date_array(&iso8601_dates);

    let intervals = TimeIntervalCollection::from_iso8601(
        FromIso8601Options {
            iso8601: &format!("{}/{}/P1Y", iso8601_dates[0], iso8601_dates[3]),
            ..default_iso8601_options("")
        },
        None,
    );

    check_intervals(&intervals, &julian_dates, true, true);
}

#[test]
fn from_iso8601_works_with_just_month() {
    let iso8601_dates = [
        "2016-12-02T10:00:01.5Z",
        "2017-01-02T10:00:01.5Z",
        "2017-02-02T10:00:01.5Z",
        "2017-03-02T10:00:01.5Z",
        "2017-04-02T10:00:01.5Z",
    ];
    let julian_dates = iso8601_to_julian_date_array(&iso8601_dates);

    let intervals = TimeIntervalCollection::from_iso8601(
        FromIso8601Options {
            iso8601: &format!("{}/{}/P1M", iso8601_dates[0], iso8601_dates[4]),
            ..default_iso8601_options("")
        },
        None,
    );

    check_intervals(&intervals, &julian_dates, true, true);
}

#[test]
fn from_iso8601_works_with_just_day() {
    let iso8601_dates = [
        "2016-12-31T10:01:01.5Z",
        "2017-01-01T10:01:01.5Z",
        "2017-01-02T10:01:01.5Z",
        "2017-01-03T10:01:01.5Z",
        "2017-01-04T10:01:01.5Z",
        "2017-01-05T10:01:01.5Z",
    ];
    let julian_dates = iso8601_to_julian_date_array(&iso8601_dates);

    let intervals = TimeIntervalCollection::from_iso8601(
        FromIso8601Options {
            iso8601: &format!("{}/{}/P1D", iso8601_dates[0], iso8601_dates[5]),
            is_start_included: Some(false),
            ..default_iso8601_options("")
        },
        None,
    );

    check_intervals(&intervals, &julian_dates, false, true);
}

#[test]
fn from_iso8601_works_with_all_date_components() {
    let iso8601_dates = [
        "2017-01-01T10:01:01.5Z",
        "2018-03-04T10:01:01.5Z",
        "2019-05-07T10:01:01.5Z",
        "2020-07-10T10:01:01.5Z",
    ];
    let julian_dates = iso8601_to_julian_date_array(&iso8601_dates);

    let intervals = TimeIntervalCollection::from_iso8601(
        FromIso8601Options {
            iso8601: &format!("{}/{}/P1Y2M3D", iso8601_dates[0], iso8601_dates[3]),
            is_stop_included: Some(false),
            ..default_iso8601_options("")
        },
        None,
    );

    check_intervals(&intervals, &julian_dates, true, false);
}

#[test]
fn from_iso8601_works_with_just_hour() {
    let iso8601_dates = [
        "2017-01-01T22:01:01.5Z",
        "2017-01-01T23:01:01.5Z",
        "2017-01-02T00:01:01.5Z",
        "2017-01-02T01:01:01.5Z",
    ];
    let julian_dates = iso8601_to_julian_date_array(&iso8601_dates);

    let intervals = TimeIntervalCollection::from_iso8601(
        FromIso8601Options {
            iso8601: &format!("{}/{}/PT1H", iso8601_dates[0], iso8601_dates[3]),
            is_start_included: Some(false),
            ..default_iso8601_options("")
        },
        None,
    );

    check_intervals(&intervals, &julian_dates, false, true);
}

#[test]
fn from_iso8601_works_with_just_minute() {
    let iso8601_dates = [
        "2016-12-31T23:58:01.5Z",
        "2016-12-31T23:59:01.5Z",
        "2017-01-01T00:00:01.5Z",
        "2017-01-01T00:01:01.5Z",
    ];
    let julian_dates = iso8601_to_julian_date_array(&iso8601_dates);

    let intervals = TimeIntervalCollection::from_iso8601(
        FromIso8601Options {
            iso8601: &format!("{}/{}/PT1M", iso8601_dates[0], iso8601_dates[3]),
            is_stop_included: Some(false),
            ..default_iso8601_options("")
        },
        None,
    );

    check_intervals(&intervals, &julian_dates, true, false);
}

#[test]
fn from_iso8601_works_with_just_second() {
    let iso8601_dates = [
        "2016-12-31T23:59:58.5Z",
        "2016-12-31T23:59:59.5Z",
        "2017-01-01T00:00:00.5Z",
        "2017-01-01T00:00:01.5Z",
    ];
    let julian_dates = iso8601_to_julian_date_array(&iso8601_dates);

    let intervals = TimeIntervalCollection::from_iso8601(
        FromIso8601Options {
            iso8601: &format!("{}/{}/PT1S", iso8601_dates[0], iso8601_dates[3]),
            is_start_included: Some(false),
            is_stop_included: Some(false),
            ..default_iso8601_options("")
        },
        None,
    );

    check_intervals(&intervals, &julian_dates, false, false);
}

#[test]
fn from_iso8601_works_with_just_millisecond() {
    let iso8601_dates = [
        "2016-12-31T23:59:58.5Z",
        "2016-12-31T23:59:59Z",
        "2016-12-31T23:59:59.5Z",
        "2017-01-01T00:00:00Z",
        "2017-01-01T00:00:00.5Z",
    ];
    let julian_dates = iso8601_to_julian_date_array(&iso8601_dates);

    let intervals = TimeIntervalCollection::from_iso8601(
        FromIso8601Options {
            iso8601: &format!("{}/{}/PT0.5S", iso8601_dates[0], iso8601_dates[4]),
            ..default_iso8601_options("")
        },
        None,
    );

    check_intervals(&intervals, &julian_dates, true, true);
}

#[test]
fn from_iso8601_works_with_all_time_components() {
    let iso8601_dates = [
        "2017-01-01T10:01:01.5Z",
        "2017-01-01T11:03:05Z",
        "2017-01-01T12:05:08.5Z",
        "2017-01-01T13:07:12Z",
    ];
    let julian_dates = iso8601_to_julian_date_array(&iso8601_dates);

    let intervals = TimeIntervalCollection::from_iso8601(
        FromIso8601Options {
            iso8601: &format!("{}/{}/PT1H2M3.5S", iso8601_dates[0], iso8601_dates[3]),
            ..default_iso8601_options("")
        },
        None,
    );

    check_intervals(&intervals, &julian_dates, true, true);
}

#[test]
fn from_iso8601_works_with_all_date_and_time_components() {
    let iso8601_dates = [
        "2017-01-01T10:01:01.5Z",
        "2018-03-04T11:03:05Z",
        "2019-05-07T12:05:08.5Z",
        "2020-07-10T13:07:12Z",
    ];
    let julian_dates = iso8601_to_julian_date_array(&iso8601_dates);

    let intervals = TimeIntervalCollection::from_iso8601(
        FromIso8601Options {
            iso8601: &format!(
                "{}/{}/P1Y2M3DT1H2M3.5S",
                iso8601_dates[0],
                iso8601_dates[3]
            ),
            ..default_iso8601_options("")
        },
        None,
    );

    check_intervals(&intervals, &julian_dates, true, true);
}

#[test]
fn from_iso8601_works_with_a_date_string_for_duration() {
    let iso8601_dates = [
        "2017-01-01T10:01:01.5Z",
        "2018-03-04T11:03:05Z",
        "2019-05-07T12:05:08.5Z",
        "2020-07-10T13:07:12Z",
    ];
    let julian_dates = iso8601_to_julian_date_array(&iso8601_dates);

    let intervals = TimeIntervalCollection::from_iso8601(
        FromIso8601Options {
            iso8601: &format!(
                "{}/{}/0001-02-03T01:02:03.5",
                iso8601_dates[0],
                iso8601_dates[3]
            ),
            ..default_iso8601_options("")
        },
        None,
    );

    check_intervals(&intervals, &julian_dates, true, true);
}

fn data_callback(interval: &TimeInterval, _index: usize) -> Option<IntervalData> {
    if JulianDate::compare(Iso8601::minimum_value(), &interval.start) == 0 {
        return Some(IntervalData::Text("default".to_owned()));
    }
    Some(IntervalData::Text(interval.start.to_iso8601(None)))
}

#[test]
fn from_iso8601_calls_the_data_callback_on_interval_create() {
    let call_count = std::rc::Rc::new(Cell::new(0));
    let call_count_for_callback = call_count.clone();
    let callback = move |interval: &TimeInterval, index: usize| {
        call_count_for_callback.set(call_count_for_callback.get() + 1);
        data_callback(interval, index)
    };

    let iso8601_dates = [
        "2017-01-01T10:01:01.5Z",
        "2018-03-04T11:03:05Z",
        "2019-05-07T12:05:08.5Z",
        "2020-07-10T13:07:12Z",
    ];
    let julian_dates = iso8601_to_julian_date_array(&iso8601_dates);

    let intervals = TimeIntervalCollection::from_iso8601(
        FromIso8601Options {
            iso8601: &format!(
                "{}/{}/P1Y2M3DT1H2M3.5S",
                iso8601_dates[0],
                iso8601_dates[3]
            ),
            data_callback: Some(&callback),
            ..default_iso8601_options("")
        },
        None,
    );

    assert_eq!(call_count.get(), 3);
    for i in 0..3 {
        assert_eq!(intervals.get(i).unwrap().data, data_callback(intervals.get(i).unwrap(), i));
    }
}

#[test]
fn from_iso8601_handles_leading_interval_option() {
    let iso8601_dates = [
        "2016-12-31T23:58:01.5Z",
        "2016-12-31T23:59:01.5Z",
        "2017-01-01T00:00:01.5Z",
        "2017-01-01T00:01:01.5Z",
    ];
    let julian_dates = iso8601_to_julian_date_array(&iso8601_dates);
    let callback: Box<dyn Fn(&TimeInterval, usize) -> Option<IntervalData>> =
        Box::new(|interval, index| data_callback(interval, index));

    let intervals = TimeIntervalCollection::from_iso8601(
        FromIso8601Options {
            iso8601: &format!("{}/{}/PT1M", iso8601_dates[0], iso8601_dates[3]),
            is_start_included: Some(true),
            is_stop_included: Some(false),
            leading_interval: true,
            data_callback: Some(&callback),
            ..default_iso8601_options("")
        },
        None,
    );

    assert_eq!(intervals.length(), 4);

    // Check leading interval
    let leading = intervals.get(0).unwrap();
    assert_eq!(JulianDate::compare(&leading.start, Iso8601::minimum_value()), 0);
    assert_eq!(JulianDate::compare(&leading.stop, &julian_dates[0]), 0);
    assert!(leading.is_start_included);
    assert!(!leading.is_stop_included);
    assert_eq!(leading.data, data_callback(leading, 0));

    // The remaining intervals (collection without the leading one).
    // Mirrors JS `fromJulianDateArray`: only the interval at `startIndex`
    // receives `isStartIncluded` (here `true` for all); `isStopIncluded`
    // applies to the last interval only (here `false`).
    for i in 0..3 {
        let interval = intervals.get(i + 1).unwrap();
        assert_eq!(JulianDate::compare(&interval.start, &julian_dates[i]), 0);
        assert_eq!(JulianDate::compare(&interval.stop, &julian_dates[i + 1]), 0);
        assert!(interval.is_start_included);
        assert!(!interval.is_stop_included);
        assert_eq!(interval.data, data_callback(interval, i + 1));
    }
}

#[test]
fn from_iso8601_handles_trailing_interval_option() {
    let iso8601_dates = [
        "2016-12-31T23:58:01.5Z",
        "2016-12-31T23:59:01.5Z",
        "2017-01-01T00:00:01.5Z",
        "2017-01-01T00:01:01.5Z",
    ];
    let julian_dates = iso8601_to_julian_date_array(&iso8601_dates);
    let callback: Box<dyn Fn(&TimeInterval, usize) -> Option<IntervalData>> =
        Box::new(|interval, index| data_callback(interval, index));

    let intervals = TimeIntervalCollection::from_iso8601(
        FromIso8601Options {
            iso8601: &format!("{}/{}/PT1M", iso8601_dates[0], iso8601_dates[3]),
            is_start_included: Some(false),
            is_stop_included: Some(true),
            trailing_interval: true,
            data_callback: Some(&callback),
            ..default_iso8601_options("")
        },
        None,
    );

    assert_eq!(intervals.length(), 4);

    // Check trailing interval
    let trailing = intervals.get(3).unwrap();
    assert_eq!(JulianDate::compare(&trailing.start, &julian_dates[3]), 0);
    assert_eq!(JulianDate::compare(&trailing.stop, Iso8601::maximum_value()), 0);
    assert!(!trailing.is_start_included);
    assert!(trailing.is_stop_included);
    assert_eq!(trailing.data, data_callback(trailing, 3));

    // Mirrors JS `fromJulianDateArray`: the first interval (at `startIndex`
    // 0) receives `isStartIncluded` (`false`); later intervals get `true`.
    // `isStopIncluded` (`true`) applies to the last interval only.
    for i in 0..3 {
        let interval = intervals.get(i).unwrap();
        assert_eq!(JulianDate::compare(&interval.start, &julian_dates[i]), 0);
        assert_eq!(JulianDate::compare(&interval.stop, &julian_dates[i + 1]), 0);
        assert_eq!(interval.is_start_included, i != 0);
        assert_eq!(interval.is_stop_included, i == 2);
        assert_eq!(interval.data, data_callback(interval, i));
    }
}

#[test]
fn from_iso8601_handles_leading_and_trailing_interval_options() {
    let iso8601_dates = [
        "2016-12-31T23:58:01.5Z",
        "2016-12-31T23:59:01.5Z",
        "2017-01-01T00:00:01.5Z",
        "2017-01-01T00:01:01.5Z",
    ];
    let julian_dates = iso8601_to_julian_date_array(&iso8601_dates);
    let callback: Box<dyn Fn(&TimeInterval, usize) -> Option<IntervalData>> =
        Box::new(|interval, index| data_callback(interval, index));

    let intervals = TimeIntervalCollection::from_iso8601(
        FromIso8601Options {
            iso8601: &format!("{}/{}/PT1M", iso8601_dates[0], iso8601_dates[3]),
            is_start_included: Some(false),
            is_stop_included: Some(false),
            leading_interval: true,
            trailing_interval: true,
            data_callback: Some(&callback),
            ..default_iso8601_options("")
        },
        None,
    );

    assert_eq!(intervals.length(), 5);

    // Check leading interval
    let leading = intervals.get(0).unwrap();
    assert_eq!(JulianDate::compare(&leading.start, Iso8601::minimum_value()), 0);
    assert_eq!(JulianDate::compare(&leading.stop, &julian_dates[0]), 0);
    assert!(leading.is_start_included);
    assert!(leading.is_stop_included);
    assert_eq!(leading.data, data_callback(leading, 0));

    // Check trailing interval
    let trailing = intervals.get(4).unwrap();
    assert_eq!(JulianDate::compare(&trailing.start, &julian_dates[3]), 0);
    assert_eq!(JulianDate::compare(&trailing.stop, Iso8601::maximum_value()), 0);
    assert!(trailing.is_start_included);
    assert!(trailing.is_stop_included);
    assert_eq!(trailing.data, data_callback(trailing, 4));

    // Mirrors JS `fromJulianDateArray`: only the interval at `startIndex`
    // receives `isStartIncluded` (`false`); later intervals get `true`.
    // `isStopIncluded` (`false`) applies to the last interval only.
    for i in 0..3 {
        let interval = intervals.get(i + 1).unwrap();
        assert_eq!(JulianDate::compare(&interval.start, &julian_dates[i]), 0);
        assert_eq!(JulianDate::compare(&interval.stop, &julian_dates[i + 1]), 0);
        assert_eq!(interval.is_start_included, i != 0);
        assert!(!interval.is_stop_included);
        assert_eq!(interval.data, data_callback(interval, i + 1));
    }
}

// --- fromIso8601DateArray ---

#[test]
fn from_iso8601_date_array_handles_leading_interval_option() {
    let iso8601_dates = [
        "2016-12-31T23:58:01.5Z",
        "2016-12-31T23:59:01.5Z",
        "2017-01-01T00:00:01.5Z",
        "2017-01-01T00:01:01.5Z",
    ];
    let julian_dates = iso8601_to_julian_date_array(&iso8601_dates);

    let intervals = TimeIntervalCollection::from_iso8601_date_array(
        FromIso8601DateArrayOptions {
            iso8601_dates: &iso8601_dates,
            is_start_included: Some(true),
            is_stop_included: Some(false),
            leading_interval: true,
            data_callback: None,
            trailing_interval: false,
        },
        None,
    );

    assert_eq!(intervals.length(), 4);

    let leading = intervals.get(0).unwrap();
    assert_eq!(JulianDate::compare(&leading.start, Iso8601::minimum_value()), 0);
    assert_eq!(JulianDate::compare(&leading.stop, &julian_dates[0]), 0);
    assert!(leading.is_start_included);
    assert!(!leading.is_stop_included);
    assert_eq!(leading.data, num(0.0));

    // Mirrors JS `fromJulianDateArray`: only the interval at `startIndex`
    // receives `isStartIncluded` (`true`); `isStopIncluded` (`false`)
    // applies to the last interval only.
    for i in 0..3 {
        let interval = intervals.get(i + 1).unwrap();
        assert_eq!(JulianDate::compare(&interval.start, &julian_dates[i]), 0);
        assert_eq!(JulianDate::compare(&interval.stop, &julian_dates[i + 1]), 0);
        assert!(interval.is_start_included);
        assert!(!interval.is_stop_included);
        assert_eq!(interval.data, num((i + 1) as f64));
    }
}

#[test]
fn from_iso8601_date_array_handles_trailing_interval_option() {
    let iso8601_dates = [
        "2016-12-31T23:58:01.5Z",
        "2016-12-31T23:59:01.5Z",
        "2017-01-01T00:00:01.5Z",
        "2017-01-01T00:01:01.5Z",
    ];
    let julian_dates = iso8601_to_julian_date_array(&iso8601_dates);

    let intervals = TimeIntervalCollection::from_iso8601_date_array(
        FromIso8601DateArrayOptions {
            iso8601_dates: &iso8601_dates,
            is_start_included: Some(false),
            is_stop_included: Some(true),
            trailing_interval: true,
            data_callback: None,
            leading_interval: false,
        },
        None,
    );

    assert_eq!(intervals.length(), 4);

    let trailing = intervals.get(3).unwrap();
    assert_eq!(JulianDate::compare(&trailing.start, &julian_dates[3]), 0);
    assert_eq!(JulianDate::compare(&trailing.stop, Iso8601::maximum_value()), 0);
    assert!(!trailing.is_start_included);
    assert!(trailing.is_stop_included);
    assert_eq!(trailing.data, num(3.0));

    // Mirrors JS `fromJulianDateArray`: the first interval (at `startIndex`
    // 0) receives `isStartIncluded` (`false`); later intervals get `true`.
    for i in 0..3 {
        let interval = intervals.get(i).unwrap();
        assert_eq!(JulianDate::compare(&interval.start, &julian_dates[i]), 0);
        assert_eq!(JulianDate::compare(&interval.stop, &julian_dates[i + 1]), 0);
        assert_eq!(interval.is_start_included, i != 0);
        assert_eq!(interval.is_stop_included, i == 2);
        assert_eq!(interval.data, num(i as f64));
    }
}

// --- fromIso8601DurationArray ---

#[test]
fn from_iso8601_duration_array_handles_relative_to_previous_set_to_false() {
    let iso8601_dates = [
        "2016-12-31T23:58:01.5Z",
        "2016-12-31T23:59:01.5Z",
        "2017-01-01T00:00:01.5Z",
        "2017-01-01T00:01:01.5Z",
    ];
    let julian_dates = iso8601_to_julian_date_array(&iso8601_dates);
    let iso8601_durations = ["PT0M", "PT1M", "PT2M", "PT3M"];

    let intervals = TimeIntervalCollection::from_iso8601_duration_array(
        FromIso8601DurationArrayOptions {
            epoch: julian_dates[0].clone(),
            iso8601_durations: &iso8601_durations,
            relative_to_previous: false,
            is_start_included: Some(false),
            is_stop_included: Some(false),
            leading_interval: true,
            trailing_interval: true,
            data_callback: None,
        },
        None,
    );

    assert_eq!(intervals.length(), 5);

    let leading = intervals.get(0).unwrap();
    assert_eq!(JulianDate::compare(&leading.start, Iso8601::minimum_value()), 0);
    assert_eq!(JulianDate::compare(&leading.stop, &julian_dates[0]), 0);
    assert!(leading.is_start_included);
    assert!(leading.is_stop_included);
    assert_eq!(leading.data, num(0.0));

    let trailing = intervals.get(4).unwrap();
    assert_eq!(JulianDate::compare(&trailing.start, &julian_dates[3]), 0);
    assert_eq!(JulianDate::compare(&trailing.stop, Iso8601::maximum_value()), 0);
    assert!(trailing.is_start_included);
    assert!(trailing.is_stop_included);
    assert_eq!(trailing.data, num(4.0));

    // Mirrors JS `fromJulianDateArray`: only the interval at `startIndex`
    // receives `isStartIncluded` (`false`); later intervals get `true`.
    // `isStopIncluded` (`false`) applies to the last interval only.
    for i in 0..3 {
        let interval = intervals.get(i + 1).unwrap();
        assert_eq!(JulianDate::compare(&interval.start, &julian_dates[i]), 0);
        assert_eq!(JulianDate::compare(&interval.stop, &julian_dates[i + 1]), 0);
        assert_eq!(interval.is_start_included, i != 0);
        assert!(!interval.is_stop_included);
        assert_eq!(interval.data, num((i + 1) as f64));
    }
}

#[test]
fn from_iso8601_duration_array_handles_relative_to_previous_set_to_true() {
    let iso8601_dates = [
        "2016-12-31T23:58:01.5Z",
        "2016-12-31T23:59:01.5Z",
        "2017-01-01T00:00:01.5Z",
        "2017-01-01T00:01:01.5Z",
    ];
    let julian_dates = iso8601_to_julian_date_array(&iso8601_dates);
    let iso8601_durations = ["PT0M", "PT1M", "PT1M", "PT1M"];

    let intervals = TimeIntervalCollection::from_iso8601_duration_array(
        FromIso8601DurationArrayOptions {
            epoch: julian_dates[0].clone(),
            iso8601_durations: &iso8601_durations,
            relative_to_previous: true,
            is_start_included: Some(false),
            is_stop_included: Some(false),
            leading_interval: true,
            trailing_interval: true,
            data_callback: None,
        },
        None,
    );

    assert_eq!(intervals.length(), 5);

    let leading = intervals.get(0).unwrap();
    assert_eq!(JulianDate::compare(&leading.start, Iso8601::minimum_value()), 0);
    assert_eq!(JulianDate::compare(&leading.stop, &julian_dates[0]), 0);

    let trailing = intervals.get(4).unwrap();
    assert_eq!(JulianDate::compare(&trailing.start, &julian_dates[3]), 0);
    assert_eq!(JulianDate::compare(&trailing.stop, Iso8601::maximum_value()), 0);

    // Mirrors JS `fromJulianDateArray`: only the interval at `startIndex`
    // receives `isStartIncluded` (`false`); later intervals get `true`.
    // `isStopIncluded` (`false`) applies to the last interval only.
    for i in 0..3 {
        let interval = intervals.get(i + 1).unwrap();
        assert_eq!(JulianDate::compare(&interval.start, &julian_dates[i]), 0);
        assert_eq!(JulianDate::compare(&interval.stop, &julian_dates[i + 1]), 0);
        assert_eq!(interval.is_start_included, i != 0);
        assert!(!interval.is_stop_included);
    }
}
