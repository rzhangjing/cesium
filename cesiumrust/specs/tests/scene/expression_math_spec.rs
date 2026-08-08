//! Expression math function tests.
//!
//! Ports A-class tests from CesiumJS ExpressionSpec.js for math functions:
//! trig, rounding, exponential, interpolation, HSL color.

use cesium_tileset::styling::{EvalResult, Expression};
use serde_json::json;
use std::collections::HashMap;

fn empty() -> HashMap<String, serde_json::Value> {
    HashMap::new()
}

fn eval_num(expr: &str) -> f64 {
    let parsed = Expression::parse(expr);
    parsed.evaluate(&empty()).as_number()
}

fn eval_color(expr: &str) -> [f64; 4] {
    let parsed = Expression::parse(expr);
    parsed.evaluate(&empty()).as_color()
}

// === Trigonometric functions ===

#[test]
fn expression_cos_function() {
    let result = eval_num("cos(0.0)");
    assert!((result - 1.0).abs() < 1e-10);
}

#[test]
fn expression_sin_function() {
    let result = eval_num("sin(0.0)");
    assert!(result.abs() < 1e-10);
}

#[test]
fn expression_tan_function() {
    let result = eval_num("tan(0.0)");
    assert!(result.abs() < 1e-10);
}

#[test]
fn expression_acos_function() {
    let result = eval_num("acos(1.0)");
    assert!(result.abs() < 1e-10);
}

#[test]
fn expression_asin_function() {
    let result = eval_num("asin(0.0)");
    assert!(result.abs() < 1e-10);
}

#[test]
fn expression_atan_function() {
    let result = eval_num("atan(0.0)");
    assert!(result.abs() < 1e-10);
}

#[test]
fn expression_atan2_function() {
    let result = eval_num("atan2(1.0, 1.0)");
    assert!((result - std::f64::consts::FRAC_PI_4).abs() < 1e-10);
}

// === Angle conversion ===

#[test]
fn expression_radians_function() {
    let result = eval_num("radians(180.0)");
    assert!((result - std::f64::consts::PI).abs() < 1e-10);
}

#[test]
fn expression_degrees_function() {
    let result = eval_num("degrees(3.141592653589793)");
    assert!((result - 180.0).abs() < 1e-8);
}

// === Rounding / sign ===

#[test]
fn expression_sign_positive() {
    assert!((eval_num("sign(5.0)") - 1.0).abs() < 1e-10);
}

#[test]
fn expression_sign_negative() {
    assert!((eval_num("sign(-3.0)") - (-1.0)).abs() < 1e-10);
}

#[test]
fn expression_sign_zero() {
    assert!(eval_num("sign(0.0)").abs() < 1e-10);
}

#[test]
fn expression_floor_function() {
    assert!((eval_num("floor(2.7)") - 2.0).abs() < 1e-10);
}

#[test]
fn expression_ceil_function() {
    assert!((eval_num("ceil(2.1)") - 3.0).abs() < 1e-10);
}

#[test]
fn expression_round_function() {
    assert!((eval_num("round(2.5)") - 3.0).abs() < 1e-10);
    assert!((eval_num("round(2.4)") - 2.0).abs() < 1e-10);
}

#[test]
fn expression_fract_function() {
    let result = eval_num("fract(2.75)");
    assert!((result - 0.75).abs() < 1e-10);
}

// === Exponential / logarithmic ===

#[test]
fn expression_exp_function() {
    let result = eval_num("exp(1.0)");
    assert!((result - std::f64::consts::E).abs() < 1e-10);
}

#[test]
fn expression_exp2_function() {
    let result = eval_num("exp2(3.0)");
    assert!((result - 8.0).abs() < 1e-10);
}

#[test]
fn expression_log_function() {
    let result = eval_num("log(2.718281828459045)");
    assert!((result - 1.0).abs() < 1e-10);
}

#[test]
fn expression_log2_function() {
    let result = eval_num("log2(8.0)");
    assert!((result - 3.0).abs() < 1e-10);
}

#[test]
fn expression_pow_function() {
    let result = eval_num("pow(2.0, 10.0)");
    assert!((result - 1024.0).abs() < 1e-10);
}

#[test]
fn expression_mod_function() {
    let result = eval_num("mod(7.0, 3.0)");
    assert!((result - 1.0).abs() < 1e-10);
}

// === Interpolation ===

#[test]
fn expression_mix_function() {
    let result = eval_num("mix(0.0, 10.0, 0.5)");
    assert!((result - 5.0).abs() < 1e-10);
}

#[test]
fn expression_mix_function_endpoints() {
    assert!((eval_num("mix(2.0, 8.0, 0.0)") - 2.0).abs() < 1e-10);
    assert!((eval_num("mix(2.0, 8.0, 1.0)") - 8.0).abs() < 1e-10);
}

// === HSL color constructors ===

#[test]
fn expression_hsl_red() {
    let c = eval_color("hsl(0.0, 1.0, 0.5)");
    assert!((c[0] - 1.0).abs() < 1e-6, "r={}", c[0]);
    assert!(c[1].abs() < 1e-6, "g={}", c[1]);
    assert!(c[2].abs() < 1e-6, "b={}", c[2]);
    assert!((c[3] - 1.0).abs() < 1e-6);
}

#[test]
fn expression_hsl_green() {
    let c = eval_color("hsl(120.0, 1.0, 0.5)");
    assert!(c[0].abs() < 1e-6, "r={}", c[0]);
    assert!((c[1] - 1.0).abs() < 1e-6, "g={}", c[1]);
    assert!(c[2].abs() < 1e-6, "b={}", c[2]);
}

#[test]
fn expression_hsl_blue() {
    let c = eval_color("hsl(240.0, 1.0, 0.5)");
    assert!(c[0].abs() < 1e-6, "r={}", c[0]);
    assert!(c[1].abs() < 1e-6, "g={}", c[1]);
    assert!((c[2] - 1.0).abs() < 1e-6, "b={}", c[2]);
}

#[test]
fn expression_hsla_with_alpha() {
    let c = eval_color("hsla(0.0, 1.0, 0.5, 0.5)");
    assert!((c[0] - 1.0).abs() < 1e-6);
    assert!((c[3] - 0.5).abs() < 1e-6, "a={}", c[3]);
}

// === Combined expressions ===

#[test]
fn expression_nested_math() {
    // floor(sqrt(16.0)) = 4
    let result = eval_num("floor(sqrt(16.0))");
    assert!((result - 4.0).abs() < 1e-10);
}

#[test]
fn expression_math_with_arithmetic() {
    // pow(2.0, 3.0) + 1.0 = 9
    let result = eval_num("pow(2.0, 3.0) + 1.0");
    assert!((result - 9.0).abs() < 1e-10);
}

#[test]
fn expression_trig_identity() {
    // sin(x)^2 + cos(x)^2 = 1 for x = 0.7
    let s = eval_num("sin(0.7)");
    let c = eval_num("cos(0.7)");
    assert!((s * s + c * c - 1.0).abs() < 1e-10);
}

#[test]
fn expression_exp_log_inverse() {
    // log(exp(5.0)) = 5
    let result = eval_num("log(exp(5.0))");
    assert!((result - 5.0).abs() < 1e-10);
}
