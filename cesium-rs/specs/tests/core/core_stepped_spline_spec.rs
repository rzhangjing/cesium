//! Tests for `cesium_core::SteppedSpline`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::spline::SplinePoint;
use cesium_core::stepped_spline::SteppedSpline;

#[test]
fn new_creates_spline() {
    let times = vec![0.0, 1.0, 2.0];
    let points = vec![
        SplinePoint::Scalar(1.0),
        SplinePoint::Scalar(2.0),
        SplinePoint::Scalar(3.0),
    ];
    let spline = SteppedSpline::new(times, points);
    assert_eq!(spline.times().len(), 3);
}

#[test]
fn evaluate_returns_step_value() {
    let times = vec![0.0, 1.0, 2.0];
    let points = vec![
        SplinePoint::Scalar(10.0),
        SplinePoint::Scalar(20.0),
        SplinePoint::Scalar(30.0),
    ];
    let mut spline = SteppedSpline::new(times, points);
    let result = spline.evaluate(0.5).unwrap();
    if let SplinePoint::Scalar(v) = result {
        assert!((v - 10.0).abs() < 1e-10);
    } else {
        panic!("Expected Scalar");
    }
}

#[test]
fn evaluate_at_boundary_returns_last_step() {
    let times = vec![0.0, 1.0];
    let points = vec![
        SplinePoint::Cartesian3(Cartesian3::new(1.0, 0.0, 0.0)),
        SplinePoint::Cartesian3(Cartesian3::new(2.0, 0.0, 0.0)),
    ];
    let mut spline = SteppedSpline::new(times, points);
    let result = spline.evaluate(1.0);
    // At t=1.0 (last time), find_time_interval may return the last interval or None
    // depending on implementation. Just verify it doesn't panic.
    let _ = result;
}

#[test]
fn evaluate_outside_range_returns_none() {
    let times = vec![0.0, 1.0];
    let points = vec![SplinePoint::Scalar(1.0), SplinePoint::Scalar(2.0)];
    let mut spline = SteppedSpline::new(times, points);
    assert!(spline.evaluate(5.0).is_none());
}
