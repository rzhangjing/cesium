//! Tests for JulianDate – ported from Core/JulianDateSpec.js

use cesium_core::gregorian_date::GregorianDate;
use cesium_core::julian_date::JulianDate;
use cesium_core::math::CesiumMath;
use cesium_core::time_constants::*;
use cesium_core::time_standard::TimeStandard;

// ── Constructor ──────────────────────────────────────────────────────────────

#[test]
fn construct_default_date() {
    let default_date = JulianDate::default_date();
    assert_eq!(default_date.day_number, 0);
    assert_eq!(default_date.seconds_of_day, 10.0);
}

#[test]
fn construct_date_with_fractional_day() {
    let julian_date = JulianDate::new(2448257.75, 0.0, TimeStandard::UTC);
    let expected = JulianDate::new(2448257.0, 64826.0, TimeStandard::TAI);
    assert_eq!(julian_date, expected);
}

#[test]
fn construct_from_tai_components() {
    let day_number = 12;
    let seconds = 12.5;
    let julian_date = JulianDate::new(day_number as f64, seconds, TimeStandard::TAI);
    assert_eq!(julian_date.day_number, day_number);
    assert_eq!(julian_date.seconds_of_day, seconds);
}

#[test]
fn construct_utc_before_leap_second() {
    let expected = JulianDate::from_tai_components(2443874, 43216.0);
    let julian_date = JulianDate::new(2443874.0, 43199.0, TimeStandard::UTC);
    assert_eq!(julian_date.day_number, expected.day_number);
    assert_eq!(julian_date.seconds_of_day, expected.seconds_of_day);
}

#[test]
fn construct_utc_at_leap_second_entry() {
    let expected = JulianDate::from_tai_components(2443874, 43218.0);
    let julian_date = JulianDate::new(2443874.0, 43200.0, TimeStandard::UTC);
    assert_eq!(julian_date.day_number, expected.day_number);
    assert_eq!(julian_date.seconds_of_day, expected.seconds_of_day);
}

#[test]
fn construct_utc_after_leap_second() {
    let expected = JulianDate::from_tai_components(2443874, 43219.0);
    let julian_date = JulianDate::new(2443874.0, 43201.0, TimeStandard::UTC);
    assert_eq!(julian_date.day_number, expected.day_number);
    assert_eq!(julian_date.seconds_of_day, expected.seconds_of_day);
}

#[test]
fn construct_more_seconds_than_a_day() {
    let julian_date = JulianDate::new(12.0, 86401.0, TimeStandard::TAI);
    assert_eq!(julian_date.day_number, 13);
    assert_eq!(julian_date.seconds_of_day, 1.0);
}

#[test]
fn construct_negative_seconds() {
    let julian_date = JulianDate::new(12.0, -1.0, TimeStandard::TAI);
    assert_eq!(julian_date.day_number, 11);
    assert_eq!(julian_date.seconds_of_day, 86399.0);
}

#[test]
fn construct_partial_day_and_negative_seconds() {
    let julian_date = JulianDate::new(12.5, -1.0, TimeStandard::TAI);
    assert_eq!(julian_date.day_number, 12);
    assert_eq!(julian_date.seconds_of_day, 43199.0);
}

#[test]
fn construct_default_time_standard() {
    let default = JulianDate::new(12.0, 12.5, TimeStandard::UTC);
    let utc = JulianDate::new(12.0, 12.5, TimeStandard::UTC);
    assert_eq!(default, utc);
}

// ── from_date_components ─────────────────────────────────────────────────────

#[test]
fn from_date_1991_jan_1() {
    let jd = JulianDate::from_date_components(1991, 1, 1, 6, 0, 0, 0.0);
    assert_eq!(jd.day_number, 2448257);
    assert_eq!(jd.seconds_of_day, 64826.0);
}

#[test]
fn from_date_2011_jul_4() {
    let jd = JulianDate::from_date_components(2011, 7, 4, 12, 0, 0, 0.0);
    assert_eq!(jd.day_number, 2455747);
    assert_eq!(jd.seconds_of_day, 34.0);
}

#[test]
fn from_date_2021_dec_31() {
    let jd = JulianDate::from_date_components(2021, 12, 31, 18, 0, 0, 0.0);
    assert_eq!(jd.day_number, 2459580);
    assert_eq!(jd.seconds_of_day, 21637.0);
}

#[test]
fn from_date_2011_sep_1() {
    let jd = JulianDate::from_date_components(2011, 9, 1, 12, 0, 0, 0.0);
    assert_eq!(jd.day_number, 2455806);
    assert_eq!(jd.seconds_of_day, 34.0);
}

#[test]
fn from_date_2039_nov_17() {
    let jd = JulianDate::from_date_components(2039, 11, 17, 0, 0, 0, 0.0);
    assert_eq!(jd.day_number, 2466109);
    assert_eq!(jd.seconds_of_day, 43237.0);
}

// ── clone ────────────────────────────────────────────────────────────────────

#[test]
fn clone_works() {
    let jd = JulianDate::from_tai_components(100, 200.0);
    let cloned = jd.clone_instance();
    assert_eq!(jd, cloned);
}

// ── ISO 8601 ─────────────────────────────────────────────────────────────────

#[test]
fn from_iso8601_calendar_date_basic() {
    let expected = JulianDate::from_date_components(2009, 8, 1, 0, 0, 0, 0.0);
    let computed = JulianDate::from_iso8601("20090801").unwrap();
    assert_eq!(computed, expected);
}

#[test]
fn from_iso8601_calendar_date_extended() {
    let expected = JulianDate::from_date_components(2009, 8, 1, 0, 0, 0, 0.0);
    let computed = JulianDate::from_iso8601("2009-08-01").unwrap();
    assert_eq!(computed, expected);
}

#[test]
fn from_iso8601_calendar_date_feb29_basic() {
    let expected = JulianDate::from_date_components(2000, 2, 29, 0, 0, 0, 0.0);
    let computed = JulianDate::from_iso8601("20000229").unwrap();
    assert_eq!(computed, expected);
}

#[test]
fn from_iso8601_calendar_date_feb29_extended() {
    let expected = JulianDate::from_date_components(2000, 2, 29, 0, 0, 0, 0.0);
    let computed = JulianDate::from_iso8601("2000-02-29").unwrap();
    assert_eq!(computed, expected);
}

#[test]
fn from_iso8601_ordinal_date_basic() {
    let expected = JulianDate::from_date_components(1985, 4, 12, 0, 0, 0, 0.0);
    let computed = JulianDate::from_iso8601("1985102").unwrap();
    assert_eq!(computed, expected);
}

#[test]
fn from_iso8601_ordinal_date_extended() {
    let expected = JulianDate::from_date_components(1985, 4, 12, 0, 0, 0, 0.0);
    let computed = JulianDate::from_iso8601("1985-102").unwrap();
    assert_eq!(computed, expected);
}

#[test]
fn from_iso8601_utc_datetime_basic() {
    let expected = JulianDate::from_date_components(2009, 8, 1, 12, 30, 25, 0.0);
    let computed = JulianDate::from_iso8601("20090801T123025Z").unwrap();
    assert_eq!(computed, expected);
}

#[test]
fn from_iso8601_utc_datetime_extended() {
    let expected = JulianDate::from_date_components(2009, 8, 1, 12, 30, 25, 0.0);
    let computed = JulianDate::from_iso8601("2009-08-01T12:30:25Z").unwrap();
    assert_eq!(computed, expected);
}

#[test]
fn from_iso8601_utc_datetime_fractional_seconds_basic() {
    let expected = JulianDate::new(2455045.0, 1825.5125423, TimeStandard::UTC);
    let computed = JulianDate::from_iso8601("20090801T123025.5125423Z").unwrap();
    assert_eq!(computed, expected);
}

#[test]
fn from_iso8601_utc_datetime_fractional_seconds_extended() {
    let expected = JulianDate::new(2455045.0, 1825.5125423, TimeStandard::UTC);
    let computed = JulianDate::from_iso8601("2009-08-01T12:30:25.5125423Z").unwrap();
    assert_eq!(computed, expected);
}

#[test]
fn from_iso8601_utc_datetime_no_seconds_basic() {
    let expected = JulianDate::from_date_components(2009, 8, 1, 12, 30, 0, 0.0);
    let computed = JulianDate::from_iso8601("20090801T1230Z").unwrap();
    assert_eq!(computed, expected);
}

#[test]
fn from_iso8601_utc_datetime_no_seconds_extended() {
    let expected = JulianDate::from_date_components(2009, 8, 1, 12, 30, 0, 0.0);
    let computed = JulianDate::from_iso8601("2009-08-01T12:30Z").unwrap();
    assert_eq!(computed, expected);
}

#[test]
fn from_iso8601_utc_datetime_no_minutes_seconds_basic() {
    let expected = JulianDate::from_date_components(2009, 8, 1, 12, 0, 0, 0.0);
    let computed = JulianDate::from_iso8601("20090801T12Z").unwrap();
    assert_eq!(computed, expected);
}

#[test]
fn from_iso8601_utc_datetime_no_minutes_seconds_extended() {
    let expected = JulianDate::from_date_components(2009, 8, 1, 12, 0, 0, 0.0);
    let computed = JulianDate::from_iso8601("2009-08-01T12Z").unwrap();
    assert_eq!(computed, expected);
}

#[test]
fn from_iso8601_leap_second() {
    let computed = JulianDate::from_iso8601("2008-12-31T23:59:60Z").unwrap();
    let expected = JulianDate::from_tai_components(2454832, 43233.0);
    assert_eq!(computed, expected);
}

#[test]
fn from_iso8601_24_hour_midnight() {
    let expected = JulianDate::from_date_components(2009, 8, 2, 0, 0, 0, 0.0);
    let computed = JulianDate::from_iso8601("2009-08-01T24:00:00Z").unwrap();
    assert_eq!(computed, expected);
}

#[test]
fn from_iso8601_utc_offset_positive() {
    let expected = JulianDate::from_date_components(2008, 11, 10, 12, 0, 0, 0.0);
    let computed = JulianDate::from_iso8601("2008-11-10T14:00:00+02").unwrap();
    assert_eq!(computed, expected);
}

#[test]
fn from_iso8601_utc_offset_extended() {
    let expected = JulianDate::from_date_components(2008, 11, 10, 11, 30, 0, 0.0);
    let computed = JulianDate::from_iso8601("2008-11-10T14:00:00+02:30").unwrap();
    assert_eq!(computed, expected);
}

#[test]
fn from_iso8601_utc_offset_crosses_month_back() {
    // "1985-04-01T00:59:00+01" = 1985-03-31 23:59 UTC
    let expected = JulianDate::from_date_components(1985, 3, 31, 23, 59, 0, 0.0);
    let computed = JulianDate::from_iso8601("1985-04-01T00:59:00+01");
    assert!(computed.is_some(), "from_iso8601 should parse '1985-04-01T00:59:00+01'");
    assert_eq!(computed.unwrap(), expected);
}

#[test]
fn from_iso8601_utc_offset_crosses_month_forward() {
    // "1985-03-31T23:59:00-01" = 1985-04-01 00:59 UTC
    let expected = JulianDate::from_date_components(1985, 4, 1, 0, 59, 0, 0.0);
    let computed = JulianDate::from_iso8601("1985-03-31T23:59:00-01");
    assert!(computed.is_some(), "from_iso8601 should parse '1985-03-31T23:59:00-01'");
    assert_eq!(computed.unwrap(), expected);
}

#[test]
fn from_iso8601_utc_offset_crosses_year_forward() {
    // "2009-01-01T01:00:00+02" = 2008-12-31 23:00 UTC
    let expected = JulianDate::from_date_components(2008, 12, 31, 23, 0, 0, 0.0);
    let computed = JulianDate::from_iso8601("2009-01-01T01:00:00+02");
    assert!(computed.is_some(), "from_iso8601 should parse '2009-01-01T01:00:00+02'");
    assert_eq!(computed.unwrap(), expected);
}

#[test]
fn from_iso8601_utc_offset_crosses_year_back() {
    // "2008-12-31T23:00:00-02" = 2009-01-01 01:00 UTC
    let expected = JulianDate::from_date_components(2009, 1, 1, 1, 0, 0, 0.0);
    let computed = JulianDate::from_iso8601("2008-12-31T23:00:00-02");
    assert!(computed.is_some(), "from_iso8601 should parse '2008-12-31T23:00:00-02'");
    assert_eq!(computed.unwrap(), expected);
}

#[test]
fn from_iso8601_invalid_returns_none() {
    assert!(JulianDate::from_iso8601("garbage").is_none());
    assert!(JulianDate::from_iso8601("2009-13-19").is_none());
    assert!(JulianDate::from_iso8601("2009-00-19").is_none());
    assert!(JulianDate::from_iso8601("2009-02-29").is_none());
    assert!(JulianDate::from_iso8601("2000-02-30").is_none());
    assert!(JulianDate::from_iso8601("2000-12-15T24:00:01").is_none());
    assert!(JulianDate::from_iso8601("2000-12-15T12:60").is_none());
    assert!(JulianDate::from_iso8601("2000-12-15T12:59:61").is_none());
}

// ── to_gregorian_date ────────────────────────────────────────────────────────

#[test]
fn to_gregorian_date_round_trip() {
    let iso1 = "2017-01-01T10:01:01.5Z";
    let jd1 = JulianDate::from_iso8601(iso1).unwrap();
    let gd = jd1.to_gregorian_date();
    let jd2 = JulianDate::from_gregorian_date(&gd);
    assert_eq!(JulianDate::compare(&jd1, &jd2), 0);
}

// ── to_iso8601 ───────────────────────────────────────────────────────────────

#[test]
fn to_iso8601_before_leap_second() {
    let expected = "1997-06-30T23:59:59Z";
    let date = JulianDate::from_iso8601(expected).unwrap();
    assert_eq!(date.to_iso8601(None), expected);
}

#[test]
fn to_iso8601_on_leap_second() {
    let expected = "1997-06-30T23:59:60Z";
    let date = JulianDate::from_iso8601(expected).unwrap();
    assert_eq!(date.to_iso8601(None), expected);
}

#[test]
fn to_iso8601_after_leap_second() {
    let expected = "1997-07-01T00:00:00Z";
    let date = JulianDate::from_iso8601(expected).unwrap();
    assert_eq!(date.to_iso8601(None), expected);
}

#[test]
fn to_iso8601_no_milliseconds() {
    let expected = "0950-01-02T03:04:05Z";
    let date = JulianDate::from_iso8601(expected).unwrap();
    assert_eq!(date.to_iso8601(None), expected);
}

#[test]
fn to_iso8601_with_precision() {
    let iso = "0950-01-02T03:04:05.012345Z";
    let jd = JulianDate::from_iso8601(iso).unwrap();
    // precision=0 → no fractional seconds
    assert_eq!(JulianDate::to_iso8601(&jd, Some(0)), "0950-01-02T03:04:05Z");
    // precision=3 → 3 fractional digits
    assert_eq!(JulianDate::to_iso8601(&jd, Some(3)), "0950-01-02T03:04:05.012Z");
    // precision=6 → 6 fractional digits
    assert_eq!(JulianDate::to_iso8601(&jd, Some(6)), "0950-01-02T03:04:05.012345Z");
    // Round-trip: from_iso8601 → to_iso8601 → from_iso8601 should give same JulianDate
    let iso_out = JulianDate::to_iso8601(&jd, None);
    let jd2 = JulianDate::from_iso8601(&iso_out).unwrap();
    assert_eq!(JulianDate::compare(&jd, &jd2), 0);
}

// ── secondsDifference / daysDifference ───────────────────────────────────────

#[test]
fn seconds_difference_works() {
    let start = JulianDate::from_date_components(2011, 7, 4, 12, 0, 0, 0.0);
    let end = JulianDate::from_date_components(2011, 7, 5, 12, 1, 0, 0.0);
    let diff = JulianDate::seconds_difference(&end, &start);
    assert!((diff - (SECONDS_PER_DAY + SECONDS_PER_MINUTE)).abs() < CesiumMath::EPSILON5);
}

#[test]
fn days_difference_works() {
    let start = JulianDate::from_date_components(2011, 7, 4, 12, 0, 0, 0.0);
    let end = JulianDate::from_date_components(2011, 7, 5, 14, 24, 0, 0.0);
    let diff = JulianDate::days_difference(&end, &start);
    assert!((diff - 1.1).abs() < 1e-10);
}

#[test]
fn days_difference_negative() {
    let end = JulianDate::from_date_components(2011, 7, 4, 12, 0, 0, 0.0);
    let start = JulianDate::from_date_components(2011, 7, 5, 14, 24, 0, 0.0);
    let diff = JulianDate::days_difference(&end, &start);
    assert!((diff - (-1.1)).abs() < 1e-10);
}

// ── addSeconds ───────────────────────────────────────────────────────────────

#[test]
fn add_seconds_whole() {
    let start = JulianDate::from_date_components(2011, 7, 4, 12, 0, 30, 0.0);
    let end = JulianDate::add_seconds(&start, 95.0);
    let gd = end.to_gregorian_date();
    assert_eq!(gd.second, 5);
    assert_eq!(gd.minute, 2);
}

#[test]
fn add_seconds_fractional() {
    let start = JulianDate::from_tai_components(2454832, 0.0);
    let end = JulianDate::add_seconds(&start, 1.5);
    let diff = JulianDate::seconds_difference(&end, &start);
    assert_eq!(diff, 1.5);
}

#[test]
fn add_seconds_negative() {
    let start = JulianDate::from_date_components(2011, 7, 4, 12, 1, 30, 0.0);
    let end = JulianDate::add_seconds(&start, -60.0);
    let diff = JulianDate::seconds_difference(&end, &start);
    assert_eq!(diff, -60.0);
}

#[test]
fn add_seconds_more_than_a_day() {
    let seconds = SECONDS_PER_DAY * 7.0 + 15.0;
    let start = JulianDate::new(2448444.0, 0.0, TimeStandard::UTC);
    let end = JulianDate::add_seconds(&start, seconds);
    let diff = JulianDate::seconds_difference(&end, &start);
    assert_eq!(diff, seconds);
}

// ── addMinutes ───────────────────────────────────────────────────────────────

#[test]
fn add_minutes_works() {
    let start = JulianDate::from_date_components(2011, 7, 4, 12, 0, 0, 0.0);
    let end = JulianDate::add_minutes(&start, 65.0);
    let gd = end.to_gregorian_date();
    assert_eq!(gd.minute, 5);
    assert_eq!(gd.hour, 13);
}

#[test]
fn add_minutes_negative() {
    let start = JulianDate::from_date_components(2011, 7, 4, 12, 0, 0, 0.0);
    let end = JulianDate::add_minutes(&start, -35.0);
    let gd = end.to_gregorian_date();
    assert_eq!(gd.minute, 25);
    assert_eq!(gd.hour, 11);
}

// ── addHours ─────────────────────────────────────────────────────────────────

#[test]
fn add_hours_works() {
    let start = JulianDate::from_date_components(2011, 7, 4, 12, 0, 0, 0.0);
    let end = JulianDate::add_hours(&start, 6.0);
    let gd = end.to_gregorian_date();
    assert_eq!(gd.hour, 18);
}

#[test]
fn add_hours_negative() {
    let start = JulianDate::from_date_components(2011, 7, 4, 12, 0, 0, 0.0);
    let end = JulianDate::add_hours(&start, -6.0);
    let gd = end.to_gregorian_date();
    assert_eq!(gd.hour, 6);
}

// ── addDays ──────────────────────────────────────────────────────────────────

#[test]
fn add_days_works() {
    let start = JulianDate::from_date_components(2011, 7, 4, 12, 0, 0, 0.0);
    let end = JulianDate::add_days(&start, 32.0);
    let gd = end.to_gregorian_date();
    assert_eq!(gd.day, 5);
    assert_eq!(gd.month, 8);
}

#[test]
fn add_days_negative() {
    let start = JulianDate::from_date_components(2011, 7, 4, 12, 0, 0, 0.0);
    let end = JulianDate::add_days(&start, -4.0);
    let gd = end.to_gregorian_date();
    assert_eq!(gd.day, 30);
    assert_eq!(gd.month, 6);
}

// ── Comparison ───────────────────────────────────────────────────────────────

#[test]
fn less_than_works() {
    let start = JulianDate::from_date_components(1991, 7, 6, 12, 0, 0, 0.0);
    let end = JulianDate::from_date_components(2011, 7, 6, 12, 1, 0, 0.0);
    assert!(JulianDate::less_than(&start, &end));
}

#[test]
fn less_than_equal_values() {
    let start = JulianDate::from_date_components(1991, 7, 6, 12, 0, 0, 0.0);
    let end = JulianDate::from_date_components(1991, 7, 6, 12, 0, 0, 0.0);
    assert!(!JulianDate::less_than(&start, &end));
    let end_plus = JulianDate::add_seconds(&end, 1.0);
    assert!(JulianDate::less_than(&start, &end_plus));
}

#[test]
fn less_than_different_time_standards() {
    let start = JulianDate::new(0.0, 0.0, TimeStandard::TAI);
    let end = JulianDate::new(0.0, 0.0, TimeStandard::UTC);
    assert!(JulianDate::less_than(&start, &end));
}

#[test]
fn greater_than_works() {
    let start = JulianDate::from_date_components(2011, 7, 6, 12, 1, 0, 0.0);
    let end = JulianDate::from_date_components(1991, 7, 6, 12, 0, 0, 0.0);
    assert!(JulianDate::greater_than(&start, &end));
}

#[test]
fn greater_than_different_time_standards() {
    let start = JulianDate::new(0.0, 0.0, TimeStandard::UTC);
    let end = JulianDate::new(0.0, 0.0, TimeStandard::TAI);
    assert!(JulianDate::greater_than(&start, &end));
}

#[test]
fn equals_epsilon_works() {
    let d1 = JulianDate::from_date_components(2011, 9, 7, 12, 55, 0, 0.0);
    let d2 = JulianDate::add_seconds(&d1, 1.0);
    assert!(JulianDate::equals_epsilon(&d1, &d2, 2.0));
}

// ── totalDays ────────────────────────────────────────────────────────────────

#[test]
fn total_days_works() {
    let total_days = 2455784.7500058;
    let jd = JulianDate::new(total_days, 0.0, TimeStandard::TAI);
    assert!((JulianDate::total_days(&jd) - total_days).abs() < 1e-10);
}

// ── computeTaiMinusUtc ───────────────────────────────────────────────────────

#[test]
fn compute_tai_minus_utc_before_all_leap_seconds() {
    let jd = JulianDate::from_date_components(1970, 7, 11, 12, 0, 0, 0.0);
    assert_eq!(JulianDate::compute_tai_minus_utc(&jd), 10.0);
}

#[test]
fn compute_tai_minus_utc_before_leap_second() {
    let jd = JulianDate::from_tai_components(2456109, 43233.0);
    assert_eq!(JulianDate::compute_tai_minus_utc(&jd), 34.0);
}

#[test]
fn compute_tai_minus_utc_on_leap_second() {
    let jd = JulianDate::from_tai_components(2456109, 43234.0);
    assert_eq!(JulianDate::compute_tai_minus_utc(&jd), 34.0);
}

#[test]
fn compute_tai_minus_utc_after_leap_second() {
    let jd = JulianDate::from_tai_components(2456109, 43235.0);
    assert_eq!(JulianDate::compute_tai_minus_utc(&jd), 35.0);
}

#[test]
fn compute_tai_minus_utc_after_all_leap_seconds() {
    let jd = JulianDate::from_tai_components(2556109, 43237.0);
    assert_eq!(JulianDate::compute_tai_minus_utc(&jd), 37.0);
}

// ── toString / Display ───────────────────────────────────────────────────────

#[test]
fn to_string_works() {
    let jd = JulianDate::from_date_components(2011, 7, 4, 12, 0, 0, 0.0);
    let s = jd.to_string();
    assert!(s.contains("2011"));
    assert!(s.ends_with('Z'));
}
