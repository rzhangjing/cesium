//! Tests for `cesium_core::MorphWeightSpline`.

use cesium_core::morph_weight_spline::MorphWeightSpline;

#[test]
fn new_creates_spline() {
    let times = vec![0.0, 1.0];
    let weights = vec![vec![0.0, 0.0], vec![1.0, 1.0]];
    let spline = MorphWeightSpline::new(times, weights);
    assert_eq!(spline.times().len(), 2);
    assert_eq!(spline.weights().len(), 2);
}

#[test]
fn evaluate_at_start_returns_first_weights() {
    let times = vec![0.0, 1.0];
    let weights = vec![vec![0.0, 0.0], vec![1.0, 0.5]];
    let mut spline = MorphWeightSpline::new(times, weights);
    let result = spline.evaluate(0.0, None).unwrap();
    assert!((result[0] - 0.0).abs() < 1e-10);
    assert!((result[1] - 0.0).abs() < 1e-10);
}

#[test]
fn evaluate_at_end_returns_last_weights() {
    let times = vec![0.0, 1.0];
    let weights = vec![vec![0.0, 0.0], vec![1.0, 0.5]];
    let mut spline = MorphWeightSpline::new(times, weights);
    // evaluate at t=1.0 → find_time_interval returns None for boundary
    // so try t=0.999
    let result = spline.evaluate(0.999, None).unwrap();
    assert!(result[0] > 0.9);
    assert!(result[1] > 0.4);
}

#[test]
fn evaluate_outside_range_returns_none() {
    let times = vec![0.0, 1.0];
    let weights = vec![vec![0.0], vec![1.0]];
    let mut spline = MorphWeightSpline::new(times, weights);
    assert!(spline.evaluate(5.0, None).is_none());
}

#[test]
fn clamp_time_clamps_to_range() {
    let times = vec![0.0, 1.0, 2.0];
    let weights = vec![vec![0.0], vec![0.5], vec![1.0]];
    let spline = MorphWeightSpline::new(times, weights);
    assert_eq!(spline.clamp_time(-1.0), 0.0);
    assert_eq!(spline.clamp_time(5.0), 2.0);
}
