//! Port of `Core/HeadingPitchRangeSpec.js`.

use cesium_core::heading_pitch_range::HeadingPitchRange;

#[test]
fn construct_with_default_values() {
    let hpr = HeadingPitchRange::new(0.0, 0.0, 0.0);
    assert_eq!(hpr.heading, 0.0);
    assert_eq!(hpr.pitch, 0.0);
    assert_eq!(hpr.range, 0.0);
}

#[test]
fn construct_with_all_values() {
    let hpr = HeadingPitchRange::new(1.0, 2.0, 3.0);
    assert_eq!(hpr.heading, 1.0);
    assert_eq!(hpr.pitch, 2.0);
    assert_eq!(hpr.range, 3.0);
}

#[test]
fn clone_produces_equal_copy() {
    let hpr = HeadingPitchRange::new(1.0, 2.0, 3.0);
    let cloned = hpr.clone_hpr();
    assert_eq!(cloned.heading, hpr.heading);
    assert_eq!(cloned.pitch, hpr.pitch);
    assert_eq!(cloned.range, hpr.range);
}

#[test]
fn default_is_zero() {
    let hpr = HeadingPitchRange::default();
    assert_eq!(hpr.heading, 0.0);
    assert_eq!(hpr.pitch, 0.0);
    assert_eq!(hpr.range, 0.0);
}
