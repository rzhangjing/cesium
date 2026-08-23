//! Tests for `cesium_core::TimeIntervalCollection`.

use cesium_core::time_interval::TimeInterval;
use cesium_core::time_interval_collection::TimeIntervalCollection;

#[test]
fn new_is_empty() {
    let c = TimeIntervalCollection::new();
    assert!(c.is_empty());
    assert_eq!(c.length(), 0);
}

#[test]
fn add_interval_increases_length() {
    let mut c = TimeIntervalCollection::new();
    let ti = TimeInterval::from_iso8601("2000/2010", None, None).unwrap();
    c.add_interval(ti);
    assert_eq!(c.length(), 1);
    assert!(!c.is_empty());
}

#[test]
fn get_returns_interval_at_index() {
    let mut c = TimeIntervalCollection::new();
    let ti = TimeInterval::from_iso8601("2000/2010", None, None).unwrap();
    c.add_interval(ti);
    assert!(c.get(0).is_some());
    assert!(c.get(1).is_none());
}

#[test]
fn remove_all_clears_collection() {
    let mut c = TimeIntervalCollection::new();
    c.add_interval(TimeInterval::from_iso8601("2000/2010", None, None).unwrap());
    c.add_interval(TimeInterval::from_iso8601("2010/2020", None, None).unwrap());
    assert_eq!(c.length(), 2);
    c.remove_all();
    assert_eq!(c.length(), 0);
}
