//! SteppedSplineSpec.js → Rust integration tests
//!
//! Original: packages/engine/Specs/Core/SteppedSplineSpec.js (10 it())
//! A-class ported: 3 (evaluate_number, evaluate_cartesian3, evaluate_midpoint)
//! C-class omitted: 5 (constructor throws ×3, evaluate throws ×2)
//! Omitted: 2 quaternion tests (Rust SteppedSpline is DVec3-only; type-system limited)

use cesium_animation::spline::*;
use glam::DVec3;

fn setup() -> (Vec<f64>, Vec<DVec3>) {
    let times = vec![0.0, 1.0, 2.0, 3.0];
    let cartesian_points = vec![
        DVec3::new(-1.0, -1.0, 0.0),
        DVec3::new(-0.5, -0.125, 0.0),
        DVec3::new(0.5, 0.125, 0.0),
        DVec3::new(1.0, 1.0, 0.0),
    ];
    (times, cartesian_points)
}

/// "evaluate returns number value"
/// Uses DVec3 x-component to encode scalar (numberPoints = [10, -5, 8, 3]).
#[test]
fn evaluate_returns_number_value() {
    let times = vec![0.0, 1.0, 2.0, 3.0];
    let number_points: Vec<f64> = vec![10.0, -5.0, 8.0, 3.0];
    // Encode as DVec3 x-component
    let points: Vec<DVec3> = number_points.iter().map(|&v| DVec3::new(v, 0.0, 0.0)).collect();

    let spline = SteppedSpline::new(times.clone(), points);

    assert!((spline.evaluate(times[0]).x - number_points[0]).abs() < 1e-15);
    assert!((spline.evaluate(times[1]).x - number_points[1]).abs() < 1e-15);

    let time = (times[0] + times[1]) / 2.0;
    assert!((spline.evaluate(time).x - number_points[0]).abs() < 1e-15);
}

/// "evaluate returns cartesian3 value"
#[test]
fn evaluate_returns_cartesian3_value() {
    let (times, cartesian_points) = setup();
    let spline = SteppedSpline::new(times.clone(), cartesian_points.clone());

    let v = spline.evaluate(times[0]);
    assert!((v - cartesian_points[0]).length() < 1e-15);

    let v = spline.evaluate(times[1]);
    assert!((v - cartesian_points[1]).length() < 1e-15);

    let time = (times[0] + times[1]) / 2.0;
    let v = spline.evaluate(time);
    assert!((v - cartesian_points[0]).length() < 1e-15);
}

/// "evaluate returns cartesian3 value with result parameter"
/// Merged: verifies midpoint of [times[1], times[2]] holds points[1].
#[test]
fn evaluate_returns_cartesian3_midpoint() {
    let (times, cartesian_points) = setup();
    let spline = SteppedSpline::new(times.clone(), cartesian_points.clone());

    let time = (times[1] + times[2]) / 2.0;
    let v = spline.evaluate(time);
    assert!((v - cartesian_points[1]).length() < 1e-15);
}
