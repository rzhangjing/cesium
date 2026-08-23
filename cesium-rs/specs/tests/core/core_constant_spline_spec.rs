//! Port of `Core/ConstantSplineSpec.js`.
use cesium_core::cartesian3::Cartesian3;
use cesium_core::constant_spline::ConstantSpline;
use cesium_core::spline::SplinePoint;

#[test]
fn value_returns_the_input_value_scalar() {
    let spline = ConstantSpline::new(SplinePoint::Scalar(10.0));
    match spline.value() {
        SplinePoint::Scalar(v) => assert_eq!(*v, 10.0),
        _ => panic!("expected Scalar"),
    }
}

#[test]
fn value_returns_the_input_value_cartesian3() {
    let value = Cartesian3::new(1.0, 2.0, 3.0);
    let spline = ConstantSpline::new(SplinePoint::Cartesian3(value));
    match spline.value() {
        SplinePoint::Cartesian3(v) => assert_eq!(*v, value),
        _ => panic!("expected Cartesian3"),
    }
}

#[test]
fn wrap_time_returns_zero() {
    let spline = ConstantSpline::new(SplinePoint::Scalar(10.0));
    assert_eq!(spline.wrap_time(-0.5), 0.0);
    assert_eq!(spline.wrap_time(2.5), 0.0);
}

#[test]
fn clamp_time_returns_zero() {
    let spline = ConstantSpline::new(SplinePoint::Scalar(10.0));
    assert_eq!(spline.clamp_time(-0.5), 0.0);
    assert_eq!(spline.clamp_time(2.5), 0.0);
}

#[test]
fn evaluate_returns_scalar_value() {
    let spline = ConstantSpline::new(SplinePoint::Scalar(10.0));
    match spline.evaluate(0.0) {
        SplinePoint::Scalar(v) => assert_eq!(v, 10.0),
        _ => panic!("expected Scalar"),
    }
}

#[test]
fn evaluate_returns_cartesian3_value() {
    let value = Cartesian3::new(1.0, 2.0, 3.0);
    let spline = ConstantSpline::new(SplinePoint::Cartesian3(value));
    match spline.evaluate(0.0) {
        SplinePoint::Cartesian3(v) => assert_eq!(v, value),
        _ => panic!("expected Cartesian3"),
    }
}
