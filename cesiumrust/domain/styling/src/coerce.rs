//! Componentwise math function evaluation for the styling language: the
//! unary / binary / ternary builtin function tables plus JS coercion helpers.
//!
//! Ported from `cesium-rs/crates/cesium-scene/src/expression.rs` L1598-1885
//! (`unary_arg_error` / `binary_arg_error` / `ternary_arg_error` /
//! `evaluate_unary_function` / `evaluate_binary_function` /
//! `evaluate_ternary_function`), the Rust port of upstream
//! `packages/engine/Source/Scene/Expression.js`
//! (`getEvaluateUnaryComponentwise` / `getEvaluateBinaryComponentwise` /
//! `getEvaluateTernaryComponentwise`).
//!
//! # DEVIATION (deps)
//!
//! The blueprint uses `cesium_core::{CesiumMath, Cartesian2/3/4}`. This isolated
//! domain crate only depends on `glam`, so:
//! * `Cartesian2/3/4` -> `glam::DVec2/DVec3/DVec4` (f64, domain precision).
//! * `CesiumMath::{to_radians, to_degrees, sign, log2, clamp, lerp}` -> local
//!   [`js_to_radians`] / [`js_to_degrees`] / [`js_sign`] / [`js_log2`] /
//!   [`js_clamp`] / [`js_lerp`] with identical ECMAScript/Cesium semantics.
//! * `Math.round` / `Math.min` / `Math.max` -> `crate::js_math` (M7-A), which
//!   already encode the JS half-towards-+infinity and NaN-propagation quirks.
//!
//! # Equality note
//!
//! The styling language defines only `===` / `!==` (strict); `Value::PartialEq`
//! and `equals_strict` stay strict. This module does NOT perform loose equality —
//! it only evaluates the numeric/vector function tables.

use glam::{DVec2, DVec3, DVec4};

use crate::js_math::{js_max, js_min, js_round};
use crate::value::{runtime_error, RuntimeError, Value};

// ---------------------------------------------------------------------------
// CesiumMath equivalents (ECMAScript/Cesium semantics, f64)
// ---------------------------------------------------------------------------

/// `CesiumMath.toRadians` (== JS `x * PI / 180`).
fn js_to_radians(degrees: f64) -> f64 {
    degrees.to_radians()
}

/// `CesiumMath.toDegrees` (== JS `x * 180 / PI`).
fn js_to_degrees(radians: f64) -> f64 {
    radians.to_degrees()
}

/// `CesiumMath.sign`: `-1` for negatives, `1` for positives, and the input
/// itself for `0` / `NaN` (neither `< 0` nor `> 0`), matching Cesium.
fn js_sign(value: f64) -> f64 {
    if value < 0.0 {
        -1.0
    } else if value > 0.0 {
        1.0
    } else {
        value
    }
}

/// `CesiumMath.log2` (== JS `Math.log2`).
fn js_log2(value: f64) -> f64 {
    value.log2()
}

/// `CesiumMath.clamp(value, min, max)`: `value < min ? min : value > max ? max :
/// value`. NaN falls through both comparisons and is returned unchanged, exactly
/// like Cesium (and unlike `f64::clamp`, which would also return NaN but panics
/// when `min > max`).
fn js_clamp(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// `CesiumMath.lerp(start, end, t)` == `(1 - t) * start + t * end`.
fn js_lerp(start: f64, end: f64, t: f64) -> f64 {
    (1.0 - t) * start + t * end
}

// ---------------------------------------------------------------------------
// Argument-shape errors
// ---------------------------------------------------------------------------

fn unary_arg_error(call: &str, value: &Value) -> RuntimeError {
    runtime_error(&format!(
        "Function \"{call}\" requires a vector or number argument. Argument is {value}."
    ))
}

fn binary_arg_error(call: &str, left: &Value, right: &Value) -> RuntimeError {
    runtime_error(&format!(
        "Function \"{call}\" requires vector or number arguments of matching types. Arguments are {left} and {right}."
    ))
}

fn ternary_arg_error(call: &str, left: &Value, right: &Value, test: &Value) -> RuntimeError {
    runtime_error(&format!(
        "Function \"{call}\" requires vector or number arguments of matching types. Arguments are {left}, {right}, and {test}."
    ))
}

// ---------------------------------------------------------------------------
// Unary function table (mirrors getEvaluateUnaryComponentwise + length/normalize)
// ---------------------------------------------------------------------------

/// Evaluates a single-argument builtin (`abs`, `sqrt`, trig, `length`,
/// `normalize`, ...). Componentwise for vectors; `length`/`normalize` are the
/// two special cases that reduce/reshape.
pub fn evaluate_unary_function(call: &str, left: Value) -> Result<Value, RuntimeError> {
    if call == "length" {
        return match left {
            Value::Number(n) => Ok(Value::Number(n.abs())),
            Value::Cartesian2(v) => Ok(Value::Number(v.length())),
            Value::Cartesian3(v) => Ok(Value::Number(v.length())),
            Value::Cartesian4(v) => Ok(Value::Number(v.length())),
            _ => Err(unary_arg_error(call, &left)),
        };
    }
    if call == "normalize" {
        return match left {
            Value::Number(_) => Ok(Value::Number(1.0)),
            Value::Cartesian2(v) => Ok(Value::Cartesian2(v * (1.0 / v.length()))),
            Value::Cartesian3(v) => Ok(Value::Cartesian3(v * (1.0 / v.length()))),
            Value::Cartesian4(v) => Ok(Value::Cartesian4(v * (1.0 / v.length()))),
            _ => Err(unary_arg_error(call, &left)),
        };
    }

    let operation: fn(f64) -> f64 = match call {
        "abs" => f64::abs,
        "sqrt" => f64::sqrt,
        "cos" => f64::cos,
        "sin" => f64::sin,
        "tan" => f64::tan,
        "acos" => f64::acos,
        "asin" => f64::asin,
        "atan" => f64::atan,
        "radians" => js_to_radians,
        "degrees" => js_to_degrees,
        "sign" => js_sign,
        "floor" => f64::floor,
        "ceil" => f64::ceil,
        "round" => js_round,
        "exp" => f64::exp,
        "exp2" => |x| 2.0_f64.powf(x),
        "log" => f64::ln,
        "log2" => js_log2,
        "fract" => |x| x - x.floor(),
        _ => {
            return Err(runtime_error(&format!(
                "Unexpected function call \"{call}\"."
            )))
        }
    };

    match left {
        Value::Number(n) => Ok(Value::Number(operation(n))),
        Value::Cartesian2(v) => Ok(Value::Cartesian2(DVec2::new(
            operation(v.x),
            operation(v.y),
        ))),
        Value::Cartesian3(v) => Ok(Value::Cartesian3(DVec3::new(
            operation(v.x),
            operation(v.y),
            operation(v.z),
        ))),
        Value::Cartesian4(v) => Ok(Value::Cartesian4(DVec4::new(
            operation(v.x),
            operation(v.y),
            operation(v.z),
            operation(v.w),
        ))),
        _ => Err(unary_arg_error(call, &left)),
    }
}

// ---------------------------------------------------------------------------
// Binary function table (mirrors getEvaluateBinaryComponentwise + distance/dot/cross)
// ---------------------------------------------------------------------------

/// Evaluates a two-argument builtin (`atan2`, `pow`, `min`, `max`, `distance`,
/// `dot`, `cross`). `min`/`max` allow a scalar second argument applied
/// componentwise; `cross` requires `vec3`.
pub fn evaluate_binary_function(
    call: &str,
    left: Value,
    right: Value,
) -> Result<Value, RuntimeError> {
    if call == "distance" {
        return match (&left, &right) {
            (Value::Number(l), Value::Number(r)) => Ok(Value::Number((l - r).abs())),
            (Value::Cartesian2(l), Value::Cartesian2(r)) => Ok(Value::Number((l - r).length())),
            (Value::Cartesian3(l), Value::Cartesian3(r)) => Ok(Value::Number((l - r).length())),
            (Value::Cartesian4(l), Value::Cartesian4(r)) => Ok(Value::Number((l - r).length())),
            _ => Err(binary_arg_error(call, &left, &right)),
        };
    }
    if call == "dot" {
        return match (&left, &right) {
            (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l * r)),
            (Value::Cartesian2(l), Value::Cartesian2(r)) => Ok(Value::Number(l.dot(*r))),
            (Value::Cartesian3(l), Value::Cartesian3(r)) => Ok(Value::Number(l.dot(*r))),
            (Value::Cartesian4(l), Value::Cartesian4(r)) => Ok(Value::Number(l.dot(*r))),
            _ => Err(binary_arg_error(call, &left, &right)),
        };
    }
    if call == "cross" {
        return match (&left, &right) {
            (Value::Cartesian3(l), Value::Cartesian3(r)) => Ok(Value::Cartesian3(l.cross(*r))),
            _ => Err(runtime_error(&format!(
                "Function \"{call}\" requires vec3 arguments. Arguments are {left} and {right}."
            ))),
        };
    }

    let (operation, allow_scalar): (fn(f64, f64) -> f64, bool) = match call {
        "atan2" => (f64::atan2, false),
        "pow" => (f64::powf, false),
        "min" => (js_min, true),
        "max" => (js_max, true),
        _ => {
            return Err(runtime_error(&format!(
                "Unexpected function call \"{call}\"."
            )))
        }
    };

    if allow_scalar {
        if let Value::Number(r) = &right {
            match &left {
                Value::Number(l) => return Ok(Value::Number(operation(*l, *r))),
                Value::Cartesian2(v) => {
                    return Ok(Value::Cartesian2(DVec2::new(
                        operation(v.x, *r),
                        operation(v.y, *r),
                    )))
                }
                Value::Cartesian3(v) => {
                    return Ok(Value::Cartesian3(DVec3::new(
                        operation(v.x, *r),
                        operation(v.y, *r),
                        operation(v.z, *r),
                    )))
                }
                Value::Cartesian4(v) => {
                    return Ok(Value::Cartesian4(DVec4::new(
                        operation(v.x, *r),
                        operation(v.y, *r),
                        operation(v.z, *r),
                        operation(v.w, *r),
                    )))
                }
                _ => {}
            }
        }
    }

    match (&left, &right) {
        (Value::Number(l), Value::Number(r)) => Ok(Value::Number(operation(*l, *r))),
        (Value::Cartesian2(l), Value::Cartesian2(r)) => Ok(Value::Cartesian2(DVec2::new(
            operation(l.x, r.x),
            operation(l.y, r.y),
        ))),
        (Value::Cartesian3(l), Value::Cartesian3(r)) => Ok(Value::Cartesian3(DVec3::new(
            operation(l.x, r.x),
            operation(l.y, r.y),
            operation(l.z, r.z),
        ))),
        (Value::Cartesian4(l), Value::Cartesian4(r)) => Ok(Value::Cartesian4(DVec4::new(
            operation(l.x, r.x),
            operation(l.y, r.y),
            operation(l.z, r.z),
            operation(l.w, r.w),
        ))),
        _ => Err(binary_arg_error(call, &left, &right)),
    }
}

// ---------------------------------------------------------------------------
// Ternary function table (mirrors getEvaluateTernaryComponentwise: clamp/mix)
// ---------------------------------------------------------------------------

/// Evaluates a three-argument builtin (`clamp(value, min, max)` /
/// `mix(start, end, t)`). A scalar `test` applies componentwise to vectors.
pub fn evaluate_ternary_function(
    call: &str,
    left: Value,
    right: Value,
    test: Value,
) -> Result<Value, RuntimeError> {
    let operation: fn(f64, f64, f64) -> f64 = match call {
        "clamp" => js_clamp,
        "mix" => js_lerp,
        _ => {
            return Err(runtime_error(&format!(
                "Unexpected function call \"{call}\"."
            )))
        }
    };

    // allowScalar: a scalar `test` applies componentwise.
    if let Value::Number(t) = &test {
        match (&left, &right) {
            (Value::Number(l), Value::Number(r)) => {
                return Ok(Value::Number(operation(*l, *r, *t)))
            }
            (Value::Cartesian2(l), Value::Cartesian2(r)) => {
                return Ok(Value::Cartesian2(DVec2::new(
                    operation(l.x, r.x, *t),
                    operation(l.y, r.y, *t),
                )))
            }
            (Value::Cartesian3(l), Value::Cartesian3(r)) => {
                return Ok(Value::Cartesian3(DVec3::new(
                    operation(l.x, r.x, *t),
                    operation(l.y, r.y, *t),
                    operation(l.z, r.z, *t),
                )))
            }
            (Value::Cartesian4(l), Value::Cartesian4(r)) => {
                return Ok(Value::Cartesian4(DVec4::new(
                    operation(l.x, r.x, *t),
                    operation(l.y, r.y, *t),
                    operation(l.z, r.z, *t),
                    operation(l.w, r.w, *t),
                )))
            }
            _ => {}
        }
    }

    match (&left, &right, &test) {
        (Value::Number(l), Value::Number(r), Value::Number(t)) => {
            Ok(Value::Number(operation(*l, *r, *t)))
        }
        (Value::Cartesian2(l), Value::Cartesian2(r), Value::Cartesian2(t)) => {
            Ok(Value::Cartesian2(DVec2::new(
                operation(l.x, r.x, t.x),
                operation(l.y, r.y, t.y),
            )))
        }
        (Value::Cartesian3(l), Value::Cartesian3(r), Value::Cartesian3(t)) => {
            Ok(Value::Cartesian3(DVec3::new(
                operation(l.x, r.x, t.x),
                operation(l.y, r.y, t.y),
                operation(l.z, r.z, t.z),
            )))
        }
        (Value::Cartesian4(l), Value::Cartesian4(r), Value::Cartesian4(t)) => {
            Ok(Value::Cartesian4(DVec4::new(
                operation(l.x, r.x, t.x),
                operation(l.y, r.y, t.y),
                operation(l.z, r.z, t.z),
                operation(l.w, r.w, t.w),
            )))
        }
        _ => Err(ternary_arg_error(call, &left, &right, &test)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(v: f64) -> Value {
        Value::Number(v)
    }

    // --- CesiumMath equivalents ---

    #[test]
    fn cesium_math_equivalents() {
        assert!((js_to_radians(180.0) - std::f64::consts::PI).abs() < 1e-12);
        assert!((js_to_degrees(std::f64::consts::PI) - 180.0).abs() < 1e-12);
        assert_eq!(js_sign(-3.0), -1.0);
        assert_eq!(js_sign(3.0), 1.0);
        assert_eq!(js_sign(0.0), 0.0);
        assert!(js_sign(f64::NAN).is_nan());
        assert_eq!(js_log2(8.0), 3.0);
        assert_eq!(js_clamp(5.0, 0.0, 3.0), 3.0);
        assert_eq!(js_clamp(-5.0, 0.0, 3.0), 0.0);
        assert_eq!(js_clamp(1.0, 0.0, 3.0), 1.0);
        assert_eq!(js_lerp(0.0, 10.0, 0.5), 5.0);
    }

    // --- unary functions ---

    #[test]
    fn unary_scalar_functions() {
        assert_eq!(evaluate_unary_function("abs", n(-3.0)).unwrap(), n(3.0));
        assert_eq!(evaluate_unary_function("sqrt", n(9.0)).unwrap(), n(3.0));
        assert_eq!(evaluate_unary_function("floor", n(1.7)).unwrap(), n(1.0));
        assert_eq!(evaluate_unary_function("ceil", n(1.2)).unwrap(), n(2.0));
        // JS Math.round(-0.5) == 0 (half towards +infinity).
        assert_eq!(evaluate_unary_function("round", n(-0.5)).unwrap(), n(0.0));
        assert_eq!(evaluate_unary_function("exp2", n(3.0)).unwrap(), n(8.0));
        assert_eq!(evaluate_unary_function("log2", n(8.0)).unwrap(), n(3.0));
        assert_eq!(evaluate_unary_function("fract", n(1.25)).unwrap(), n(0.25));
        assert_eq!(evaluate_unary_function("sign", n(-9.0)).unwrap(), n(-1.0));
    }

    #[test]
    fn unary_length_and_normalize() {
        assert_eq!(evaluate_unary_function("length", n(-4.0)).unwrap(), n(4.0));
        let v3 = Value::Cartesian3(DVec3::new(3.0, 4.0, 0.0));
        assert_eq!(evaluate_unary_function("length", v3).unwrap(), n(5.0));
        let v = Value::Cartesian2(DVec2::new(3.0, 4.0));
        match evaluate_unary_function("normalize", v).unwrap() {
            Value::Cartesian2(u) => {
                assert!((u.length() - 1.0).abs() < 1e-12);
            }
            _ => panic!("expected Cartesian2"),
        }
        // normalize(number) == 1.
        assert_eq!(evaluate_unary_function("normalize", n(7.0)).unwrap(), n(1.0));
    }

    #[test]
    fn unary_componentwise_vector() {
        let v = Value::Cartesian2(DVec2::new(-1.0, -2.0));
        match evaluate_unary_function("abs", v).unwrap() {
            Value::Cartesian2(u) => {
                assert_eq!(u.x, 1.0);
                assert_eq!(u.y, 2.0);
            }
            _ => panic!("expected Cartesian2"),
        }
    }

    #[test]
    fn unary_wrong_arg_type_errors() {
        let err = evaluate_unary_function("abs", Value::String("x".into())).unwrap_err();
        assert!(err.message().contains("requires a vector or number argument"));
        let err = evaluate_unary_function("nope", n(1.0)).unwrap_err();
        assert!(err.message().contains("Unexpected function call"));
    }

    // --- binary functions ---

    #[test]
    fn binary_scalar_functions() {
        assert_eq!(evaluate_binary_function("pow", n(2.0), n(3.0)).unwrap(), n(8.0));
        assert_eq!(evaluate_binary_function("min", n(1.0), n(2.0)).unwrap(), n(1.0));
        assert_eq!(evaluate_binary_function("max", n(1.0), n(2.0)).unwrap(), n(2.0));
        // JS Math.min NaN propagation.
        assert!(evaluate_binary_function("min", n(f64::NAN), n(1.0))
            .unwrap()
            .number_conversion()
            .is_nan());
        assert_eq!(
            evaluate_binary_function("distance", n(3.0), n(7.0)).unwrap(),
            n(4.0)
        );
        assert_eq!(evaluate_binary_function("dot", n(2.0), n(3.0)).unwrap(), n(6.0));
    }

    #[test]
    fn binary_vector_functions() {
        let a = Value::Cartesian3(DVec3::new(1.0, 0.0, 0.0));
        let b = Value::Cartesian3(DVec3::new(0.0, 1.0, 0.0));
        match evaluate_binary_function("cross", a, b).unwrap() {
            Value::Cartesian3(c) => assert_eq!(c, DVec3::new(0.0, 0.0, 1.0)),
            _ => panic!("expected Cartesian3"),
        }
        // min with scalar second arg applies componentwise.
        let v = Value::Cartesian2(DVec2::new(1.0, 5.0));
        match evaluate_binary_function("min", v, n(3.0)).unwrap() {
            Value::Cartesian2(u) => {
                assert_eq!(u.x, 1.0);
                assert_eq!(u.y, 3.0);
            }
            _ => panic!("expected Cartesian2"),
        }
    }

    #[test]
    fn binary_cross_requires_vec3() {
        let a = Value::Cartesian2(DVec2::new(1.0, 0.0));
        let b = Value::Cartesian2(DVec2::new(0.0, 1.0));
        let err = evaluate_binary_function("cross", a, b).unwrap_err();
        assert!(err.message().contains("requires vec3 arguments"));
    }

    // --- ternary functions ---

    #[test]
    fn ternary_clamp_and_mix() {
        assert_eq!(
            evaluate_ternary_function("clamp", n(5.0), n(0.0), n(3.0)).unwrap(),
            n(3.0)
        );
        assert_eq!(
            evaluate_ternary_function("mix", n(0.0), n(10.0), n(0.25)).unwrap(),
            n(2.5)
        );
    }

    #[test]
    fn ternary_scalar_test_applies_componentwise() {
        let l = Value::Cartesian2(DVec2::new(0.0, 0.0));
        let r = Value::Cartesian2(DVec2::new(10.0, 20.0));
        match evaluate_ternary_function("mix", l, r, n(0.5)).unwrap() {
            Value::Cartesian2(u) => {
                assert_eq!(u.x, 5.0);
                assert_eq!(u.y, 10.0);
            }
            _ => panic!("expected Cartesian2"),
        }
    }

    #[test]
    fn ternary_mismatched_types_error() {
        let l = Value::Cartesian2(DVec2::new(0.0, 0.0));
        let r = Value::Cartesian3(DVec3::new(1.0, 2.0, 3.0));
        let err = evaluate_ternary_function("mix", l, r, n(0.5)).unwrap_err();
        assert!(err.message().contains("requires vector or number arguments"));
    }
}
