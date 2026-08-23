//! Tests for `cesium_core::TimeInterval`.

use cesium_core::julian_date::JulianDate;
use cesium_core::time_interval::TimeInterval;

#[test]
fn default_is_empty() {
    let ti = TimeInterval::empty();
    assert!(ti.is_empty());
}

#[test]
fn new_with_dates_is_not_empty() {
    let start = JulianDate::from_iso8601("2000-01-01T00:00:00Z").unwrap();
    let stop = JulianDate::from_iso8601("2010-01-01T00:00:00Z").unwrap();
    let ti = TimeInterval::new(Some(start), Some(stop), None, None);
    assert!(!ti.is_empty());
}

#[test]
fn contains_returns_true_for_interior_point() {
    let start = JulianDate::from_iso8601("2000-01-01T00:00:00Z").unwrap();
    let stop = JulianDate::from_iso8601("2010-01-01T00:00:00Z").unwrap();
    let ti = TimeInterval::new(Some(start), Some(stop), None, None);
    let mid = JulianDate::from_iso8601("2005-01-01T00:00:00Z").unwrap();
    assert!(ti.contains(&mid));
}

#[test]
fn contains_returns_false_for_outside_point() {
    let start = JulianDate::from_iso8601("2000-01-01T00:00:00Z").unwrap();
    let stop = JulianDate::from_iso8601("2010-01-01T00:00:00Z").unwrap();
    let ti = TimeInterval::new(Some(start), Some(stop), None, None);
    let outside = JulianDate::from_iso8601("2015-01-01T00:00:00Z").unwrap();
    assert!(!ti.contains(&outside));
}

#[test]
fn from_iso8601_roundtrip() {
    let ti = TimeInterval::from_iso8601("2000/2010", None, None).unwrap();
    let iso = ti.to_iso8601(None);
    assert!(iso.contains("2000"));
    assert!(iso.contains("2010"));
}

#[test]
fn equals_returns_true_for_same_intervals() {
    let a = TimeInterval::from_iso8601("2000/2010", None, None).unwrap();
    let b = TimeInterval::from_iso8601("2000/2010", None, None).unwrap();
    assert!(TimeInterval::equals(&a, &b));
}

#[test]
fn intersect_returns_overlap() {
    let a = TimeInterval::from_iso8601("2000/2010", None, None).unwrap();
    let b = TimeInterval::from_iso8601("2005/2015", None, None).unwrap();
    let c = TimeInterval::intersect(&a, &b);
    assert!(!c.is_empty());
}
