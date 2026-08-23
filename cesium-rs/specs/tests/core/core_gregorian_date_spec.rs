//! Tests for `cesium_core::GregorianDate`.

use cesium_core::gregorian_date::GregorianDate;

#[test]
fn default_is_jan_1_year_1() {
    let d = GregorianDate::default();
    assert_eq!(d.year, 1);
    assert_eq!(d.month, 1);
    assert_eq!(d.day, 1);
}

#[test]
fn new_sets_all_fields() {
    let d = GregorianDate::new(2024, 6, 15, 10, 30, 45, 123.0, false);
    assert_eq!(d.year, 2024);
    assert_eq!(d.month, 6);
    assert_eq!(d.day, 15);
    assert_eq!(d.hour, 10);
    assert_eq!(d.minute, 30);
    assert_eq!(d.second, 45);
    assert_eq!(d.millisecond, 123.0);
    assert!(!d.is_leap_second);
}

#[test]
fn validate_accepts_valid_date() {
    let d = GregorianDate::new(2024, 2, 29, 0, 0, 0, 0.0, false);
    assert!(d.validate()); // 2024 is a leap year
}

#[test]
fn validate_rejects_feb_29_non_leap_year() {
    let d = GregorianDate::new(2023, 2, 29, 0, 0, 0, 0.0, false);
    assert!(!d.validate());
}

#[test]
fn validate_rejects_invalid_month() {
    let d = GregorianDate::new(2024, 13, 1, 0, 0, 0, 0.0, false);
    assert!(!d.validate());
}

#[test]
fn validate_accepts_leap_second() {
    let d = GregorianDate::new(2024, 6, 30, 23, 59, 60, 0.0, true);
    assert!(d.validate());
}

#[test]
fn validate_rejects_second_60_without_leap_second() {
    let d = GregorianDate::new(2024, 6, 30, 23, 59, 60, 0.0, false);
    assert!(!d.validate());
}
