//! Port of `Core/IntervalSpec.js`.

use cesium_core::interval::Interval;

#[test]
fn constructs_without_arguments() {
    let interval = Interval::default();
    assert_eq!(interval.start, 0.0);
    assert_eq!(interval.stop, 0.0);
}

#[test]
fn constructs_with_arguments() {
    let interval = Interval::new(1.0, 2.0);
    assert_eq!(interval.start, 1.0);
    assert_eq!(interval.stop, 2.0);
}
