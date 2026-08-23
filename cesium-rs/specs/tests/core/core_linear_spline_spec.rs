//! Port of `Core/LinearSplineSpec.js`.
use cesium_core::cartesian3::Cartesian3;
use cesium_core::linear_spline::LinearSpline;
use cesium_core::spline::SplinePoint;

fn make_times() -> Vec<f64> {
    vec![0.0, 1.0, 2.0, 3.0]
}

fn make_cartesian_points() -> Vec<SplinePoint> {
    vec![
        SplinePoint::Cartesian3(Cartesian3::new(-1.0, -1.0, 0.0)),
        SplinePoint::Cartesian3(Cartesian3::new(-0.5, -0.125, 0.0)),
        SplinePoint::Cartesian3(Cartesian3::new(0.5, 0.125, 0.0)),
        SplinePoint::Cartesian3(Cartesian3::new(1.0, 1.0, 0.0)),
    ]
}

fn make_scalar_points() -> Vec<SplinePoint> {
    vec![
        SplinePoint::Scalar(3.0),
        SplinePoint::Scalar(5.0),
        SplinePoint::Scalar(1.0),
        SplinePoint::Scalar(10.0),
    ]
}

#[test]
fn evaluate_scalar_at_start() {
    let mut ls = LinearSpline::new(make_times(), make_scalar_points());
    match ls.evaluate(0.0).unwrap() {
        SplinePoint::Scalar(v) => assert_eq!(v, 3.0),
        _ => panic!("expected Scalar"),
    }
}

#[test]
fn evaluate_scalar_midpoint() {
    let mut ls = LinearSpline::new(make_times(), make_scalar_points());
    let time = 0.5; // midpoint of [0, 1]
    let t = (time - 0.0) / (1.0 - 0.0); // = 0.5
    let expected = (1.0 - t) * 3.0 + t * 5.0; // = 4.0
    match ls.evaluate(time).unwrap() {
        SplinePoint::Scalar(v) => assert!((v - expected).abs() < 1e-15),
        _ => panic!("expected Scalar"),
    }
}

#[test]
fn evaluate_cartesian3_at_start() {
    let mut ls = LinearSpline::new(make_times(), make_cartesian_points());
    match ls.evaluate(0.0).unwrap() {
        SplinePoint::Cartesian3(v) => {
            assert_eq!(v.x, -1.0);
            assert_eq!(v.y, -1.0);
            assert_eq!(v.z, 0.0);
        }
        _ => panic!("expected Cartesian3"),
    }
}

#[test]
fn evaluate_cartesian3_midpoint() {
    let mut ls = LinearSpline::new(make_times(), make_cartesian_points());
    let time = 0.5;
    let t = (time - 0.0) / (1.0 - 0.0);
    // lerp between (-1,-1,0) and (-0.5,-0.125,0) at t=0.5
    let expected_x = (1.0 - t) * (-1.0) + t * (-0.5);
    let expected_y = (1.0 - t) * (-1.0) + t * (-0.125);
    match ls.evaluate(time).unwrap() {
        SplinePoint::Cartesian3(v) => {
            assert!((v.x - expected_x).abs() < 1e-15);
            assert!((v.y - expected_y).abs() < 1e-15);
        }
        _ => panic!("expected Cartesian3"),
    }
}

#[test]
fn evaluate_out_of_range_returns_none() {
    let mut ls = LinearSpline::new(make_times(), make_scalar_points());
    assert!(ls.evaluate(-1.0).is_none());
    assert!(ls.evaluate(4.0).is_none());
}
