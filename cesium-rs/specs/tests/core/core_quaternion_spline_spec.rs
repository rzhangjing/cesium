//! Tests for `cesium_core::QuaternionSpline`.

use cesium_core::quaternion::Quaternion;
use cesium_core::quaternion_spline::QuaternionSpline;

#[test]
fn new_creates_spline() {
    let times = vec![0.0, 1.0, 2.0];
    let points = vec![
        Quaternion::new(1.0, 0.0, 0.0, 0.0),
        Quaternion::new(0.0, 1.0, 0.0, 0.0),
        Quaternion::new(0.0, 0.0, 1.0, 0.0),
    ];
    let spline = QuaternionSpline::new(times, points);
    assert_eq!(spline.times().len(), 3);
    assert_eq!(spline.points().len(), 3);
}

#[test]
fn evaluate_at_start_returns_first_point() {
    let times = vec![0.0, 1.0];
    let q0 = Quaternion::new(1.0, 0.0, 0.0, 0.0);
    let q1 = Quaternion::new(0.0, 1.0, 0.0, 0.0);
    let points = vec![q0, q1];
    let mut spline = QuaternionSpline::new(times, points);
    let mut result = Quaternion::default();
    spline.evaluate(0.0, &mut result);
    // At t=0, should be close to q0
    assert!((result.w - 1.0).abs() < 1e-10 || (result.x - 1.0).abs() < 1e-10
        || (result.y - 1.0).abs() < 1e-10 || (result.z - 1.0).abs() < 1e-10);
}

#[test]
fn evaluate_outside_range_returns_none() {
    let times = vec![0.0, 1.0];
    let points = vec![
        Quaternion::new(1.0, 0.0, 0.0, 0.0),
        Quaternion::new(0.0, 1.0, 0.0, 0.0),
    ];
    let mut spline = QuaternionSpline::new(times, points);
    let mut result = Quaternion::default();
    assert!(spline.evaluate(5.0, &mut result).is_none());
}

#[test]
fn clamp_time_clamps_to_range() {
    let times = vec![0.0, 1.0, 2.0];
    let points = vec![
        Quaternion::new(1.0, 0.0, 0.0, 0.0),
        Quaternion::new(0.0, 1.0, 0.0, 0.0),
        Quaternion::new(0.0, 0.0, 1.0, 0.0),
    ];
    let spline = QuaternionSpline::new(times, points);
    assert_eq!(spline.clamp_time(-1.0), 0.0);
    assert_eq!(spline.clamp_time(5.0), 2.0);
}
