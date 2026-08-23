use cesium_core::easing_function;
use cesium_core::math::CesiumMath;

const EPS: f64 = CesiumMath::EPSILON10;

/// All easing functions should map 0.0 → 0.0
#[test]
fn all_functions_map_zero_to_zero() {
    let fns: Vec<(&str, fn(f64) -> f64)> = vec![
        ("linear_none", easing_function::linear_none),
        ("quadratic_in", easing_function::quadratic_in),
        ("quadratic_out", easing_function::quadratic_out),
        ("quadratic_in_out", easing_function::quadratic_in_out),
        ("cubic_in", easing_function::cubic_in),
        ("cubic_out", easing_function::cubic_out),
        ("cubic_in_out", easing_function::cubic_in_out),
        ("sinusoidal_in", easing_function::sinusoidal_in),
        ("sinusoidal_out", easing_function::sinusoidal_out),
        ("sinusoidal_in_out", easing_function::sinusoidal_in_out),
        ("exponential_in", easing_function::exponential_in),
        ("exponential_out", easing_function::exponential_out),
        ("exponential_in_out", easing_function::exponential_in_out),
        ("circular_in", easing_function::circular_in),
        ("circular_out", easing_function::circular_out),
        ("circular_in_out", easing_function::circular_in_out),
        ("bounce_out", easing_function::bounce_out),
        ("bounce_in", easing_function::bounce_in),
        ("bounce_in_out", easing_function::bounce_in_out),
    ];

    for (name, f) in &fns {
        let val = f(0.0);
        assert!(
            val.abs() < EPS,
            "{}(0.0) = {} expected 0.0",
            name,
            val
        );
    }
}

/// All easing functions should map 1.0 → 1.0
#[test]
fn all_functions_map_one_to_one() {
    let fns: Vec<(&str, fn(f64) -> f64)> = vec![
        ("linear_none", easing_function::linear_none),
        ("quadratic_in", easing_function::quadratic_in),
        ("quadratic_out", easing_function::quadratic_out),
        ("quadratic_in_out", easing_function::quadratic_in_out),
        ("cubic_in", easing_function::cubic_in),
        ("cubic_out", easing_function::cubic_out),
        ("cubic_in_out", easing_function::cubic_in_out),
        ("sinusoidal_in", easing_function::sinusoidal_in),
        ("sinusoidal_out", easing_function::sinusoidal_out),
        ("sinusoidal_in_out", easing_function::sinusoidal_in_out),
        ("exponential_in", easing_function::exponential_in),
        ("exponential_out", easing_function::exponential_out),
        ("exponential_in_out", easing_function::exponential_in_out),
        ("circular_in", easing_function::circular_in),
        ("circular_out", easing_function::circular_out),
        ("circular_in_out", easing_function::circular_in_out),
        ("bounce_out", easing_function::bounce_out),
        ("bounce_in", easing_function::bounce_in),
        ("bounce_in_out", easing_function::bounce_in_out),
    ];

    for (name, f) in &fns {
        let val = f(1.0);
        assert!(
            (val - 1.0).abs() < EPS,
            "{}(1.0) = {} expected 1.0",
            name,
            val
        );
    }
}

#[test]
fn linear_none_is_identity() {
    assert_eq!(easing_function::linear_none(0.25), 0.25);
    assert_eq!(easing_function::linear_none(0.5), 0.5);
    assert_eq!(easing_function::linear_none(0.75), 0.75);
}

#[test]
fn quadratic_in_at_half() {
    let val = easing_function::quadratic_in(0.5);
    assert!((val - 0.25).abs() < EPS);
}

#[test]
fn bounce_out_boundary_values() {
    // bounce_out(0) = 0, bounce_out(1) = 1
    assert!((easing_function::bounce_out(0.0)).abs() < EPS);
    assert!((easing_function::bounce_out(1.0) - 1.0).abs() < EPS);
    // bounce_out is non-negative on [0, 1]
    for i in 0..=100 {
        let t = i as f64 / 100.0;
        let val = easing_function::bounce_out(t);
        assert!(val >= -EPS, "bounce_out({}) = {} < 0", t, val);
        assert!(val <= 1.0 + EPS, "bounce_out({}) = {} > 1", t, val);
    }
}
