//! styling 语言的逐分量数学函数求值：一元 / 二元 / 三元内建函数表，
//! 外加 JS 强制类型转换辅助函数。
//!
//! 主要入口：`unary_arg_error` / `binary_arg_error` / `ternary_arg_error` 构造
//! 参数形态错误；`evaluate_unary_function` / `evaluate_binary_function` /
//! `evaluate_ternary_function` 逐分量地求值一元/二元/三元内建函数。
//!
//! # 偏离（依赖）
//!
//! blueprint 使用 `cesium_core::{CesiumMath, Cartesian2/3/4}`。这个孤立的
//! domain crate 只依赖 `glam`，所以：
//! * `Cartesian2/3/4` -> `glam::DVec2/DVec3/DVec4`（f64，domain 精度）。
//! * `CesiumMath::{to_radians, to_degrees, sign, log2, clamp, lerp}` -> 本地
//!   [`js_to_radians`] / [`js_to_degrees`] / [`js_sign`] / [`js_log2`] /
//!   [`js_clamp`] / [`js_lerp`]，语义与 ECMAScript/Cesium 完全一致。
//! * `Math.round` / `Math.min` / `Math.max` -> `crate::js_math`（M7-A），其
//!   已编码 JS 的"半值向 +infinity 取整"与 NaN 传播等怪癖。
//!
//! # 相等性说明
//!
//! styling 语言只定义 `===` / `!==`（严格）；`Value::PartialEq`
//! 和 `equals_strict` 保持严格。本模块**不**执行宽松相等 ——
//! 它只求值数值/向量函数表。

use glam::{DVec2, DVec3, DVec4};

use crate::js_math::{js_max, js_min, js_round};
use crate::value::{runtime_error, RuntimeError, Value};

// ---------------------------------------------------------------------------
// CesiumMath 等价实现（ECMAScript/Cesium 语义，f64）
// ---------------------------------------------------------------------------

/// `CesiumMath.toRadians`（== JS `x * PI / 180`）。
fn js_to_radians(degrees: f64) -> f64 {
    // 直接用 std 的 to_radians，与 x*PI/180 逐位一致。
    degrees.to_radians()
}

/// `CesiumMath.toDegrees`（== JS `x * 180 / PI`）。
fn js_to_degrees(radians: f64) -> f64 {
    radians.to_degrees()
}

/// `CesiumMath.sign`：负数返回 `-1`，正数返回 `1`，而 `0` / `NaN`
/// 返回输入本身（既不 `< 0` 也不 `> 0`），与 Cesium 一致。
fn js_sign(value: f64) -> f64 {
    // 严格小于/大于零才判负/正；零与 NaN 落入 else 原样返回。
    if value < 0.0 {
        -1.0
    } else if value > 0.0 {
        1.0
    } else {
        value
    }
}

/// `CesiumMath.log2`（== JS `Math.log2`）。
fn js_log2(value: f64) -> f64 {
    value.log2()
}

/// `CesiumMath.clamp(value, min, max)`：`value < min ? min : value > max ? max :
/// value`。NaN 两个比较都不成立，原样返回，与 Cesium 完全一致
/// （也不同于 `f64::clamp`——它虽同样返回 NaN，但在 `min > max` 时会 panic）。
fn js_clamp(value: f64, min: f64, max: f64) -> f64 {
    // 低于 min 取 min，高于 max 取 max；NaN 两比较皆 false，原样返回。
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// `CesiumMath.lerp(start, end, t)` == `(1 - t) * start + t * end`。
fn js_lerp(start: f64, end: f64, t: f64) -> f64 {
    // (1-t) 权重给 start、 t 权重给 end 的线性插值。
    (1.0 - t) * start + t * end
}

// ---------------------------------------------------------------------------
// 参数形态错误
// ---------------------------------------------------------------------------

/// 一元函数参数类型不符时构造运行时错误。
fn unary_arg_error(call: &str, value: &Value) -> RuntimeError {
    // 错误文案与 JS 运行时逐字一致，便于镜像 spec 用例。
    runtime_error(&format!(
        "Function \"{call}\" requires a vector or number argument. Argument is {value}."
    ))
}

/// 二元函数参数类型不符时构造运行时错误。
fn binary_arg_error(call: &str, left: &Value, right: &Value) -> RuntimeError {
    // 错误文案逐字对齐 JS。
    runtime_error(&format!(
        "Function \"{call}\" requires vector or number arguments of matching types. Arguments are {left} and {right}."
    ))
}

/// 三元函数参数类型不符时构造运行时错误。
fn ternary_arg_error(call: &str, left: &Value, right: &Value, test: &Value) -> RuntimeError {
    // 错误文案逐字对齐 JS。
    runtime_error(&format!(
        "Function \"{call}\" requires vector or number arguments of matching types. Arguments are {left}, {right}, and {test}."
    ))
}

// ---------------------------------------------------------------------------
// 一元函数表（镜像 getEvaluateUnaryComponentwise + length/normalize）
// ---------------------------------------------------------------------------

/// 求值单参数内建函数（`abs`、`sqrt`、三角函数、`length`、
/// `normalize` 等）。对向量逐分量执行；`length`/`normalize` 是
/// 两个做规约/变形的特例。
pub fn evaluate_unary_function(call: &str, left: Value) -> Result<Value, RuntimeError> {
    // length 特例：标量取绝对值，向量取欧几里得范数（均回标量）。
    if call == "length" {
        return match left {
            Value::Number(n) => Ok(Value::Number(n.abs())),
            Value::Cartesian2(v) => Ok(Value::Number(v.length())),
            Value::Cartesian3(v) => Ok(Value::Number(v.length())),
            Value::Cartesian4(v) => Ok(Value::Number(v.length())),
            _ => Err(unary_arg_error(call, &left)),
        };
    }
    // normalize 特例：标量恒为 1.0，向量除以自身长度。
    if call == "normalize" {
        // 向量乘以 1/长度；length 为 0 时会得 NaN（与 JS 一致）。
        return match left {
            Value::Number(_) => Ok(Value::Number(1.0)),
            Value::Cartesian2(v) => Ok(Value::Cartesian2(v * (1.0 / v.length()))),
            Value::Cartesian3(v) => Ok(Value::Cartesian3(v * (1.0 / v.length()))),
            Value::Cartesian4(v) => Ok(Value::Cartesian4(v * (1.0 / v.length()))),
            _ => Err(unary_arg_error(call, &left)),
        };
    }

    // 其余一元函数：先查出对应的标量操作（函数指针），
    // 再统一走逐分量映射（把标量函数应用到向量各分量）。
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
        // round 走 js_math 的 JS 语义（半值向 +∞ 取整）。
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

    // 把 operation 逐分量应用到标量/二维/三维/四维，其余类型报错。
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
// 二元函数表（镜像 getEvaluateBinaryComponentwise + distance/dot/cross）
// ---------------------------------------------------------------------------

/// 求值双参数内建函数（`atan2`、`pow`、`min`、`max`、`distance`、
/// `dot`、`cross`）。`min`/`max` 允许第二个参数为标量并逐分量应用；
/// `cross` 要求 `vec3`。
pub fn evaluate_binary_function(
    call: &str,
    left: Value,
    right: Value,
) -> Result<Value, RuntimeError> {
    // distance 特例：两侧同型，回标量（标量取绝对差，向量取差向量长度）。
    if call == "distance" {
        // 不同维度或类型不匹配 → binary_arg_error。
        return match (&left, &right) {
            (Value::Number(l), Value::Number(r)) => Ok(Value::Number((l - r).abs())),
            (Value::Cartesian2(l), Value::Cartesian2(r)) => Ok(Value::Number((l - r).length())),
            (Value::Cartesian3(l), Value::Cartesian3(r)) => Ok(Value::Number((l - r).length())),
            (Value::Cartesian4(l), Value::Cartesian4(r)) => Ok(Value::Number((l - r).length())),
            _ => Err(binary_arg_error(call, &left, &right)),
        };
    }
    // dot 特例：同型两侧点积，回标量。
    if call == "dot" {
        // 标量情形退化为普通乘法。
        return match (&left, &right) {
            (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l * r)),
            (Value::Cartesian2(l), Value::Cartesian2(r)) => Ok(Value::Number(l.dot(*r))),
            (Value::Cartesian3(l), Value::Cartesian3(r)) => Ok(Value::Number(l.dot(*r))),
            (Value::Cartesian4(l), Value::Cartesian4(r)) => Ok(Value::Number(l.dot(*r))),
            _ => Err(binary_arg_error(call, &left, &right)),
        };
    }
    // cross 特例：仅接受两个 vec3，回 vec3。
    if call == "cross" {
        return match (&left, &right) {
            (Value::Cartesian3(l), Value::Cartesian3(r)) => Ok(Value::Cartesian3(l.cross(*r))),
            _ => Err(runtime_error(&format!(
                "Function \"{call}\" requires vec3 arguments. Arguments are {left} and {right}."
            ))),
        };
    }

    // 其余二元函数：选出标量操作与是否允许第二参为标量（allow_scalar）。
    let (operation, allow_scalar): (fn(f64, f64) -> f64, bool) = match call {
        // atan2/pow 不允许标量广播；min/max 允许右侧标量。
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

    // min/max 允许右侧为标量：将同一标量逐分量作用于左侧向量。
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

    // 两侧同型时逐分量应用 operation；类型不匹配报错。
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
// 三元函数表（镜像 getEvaluateTernaryComponentwise：clamp/mix）
// ---------------------------------------------------------------------------

/// 求值三参数内建函数（`clamp(value, min, max)` /
/// `mix(start, end, t)`）。标量 `test` 对向量逐分量应用。
pub fn evaluate_ternary_function(
    call: &str,
    left: Value,
    right: Value,
    test: Value,
) -> Result<Value, RuntimeError> {
    // 选出三参数标量操作（clamp 或 mix）。
    // 未命中的函数名直接返回 Unexpected function call。
    let operation: fn(f64, f64, f64) -> f64 = match call {
        "clamp" => js_clamp,
        "mix" => js_lerp,
        _ => {
            return Err(runtime_error(&format!(
                "Unexpected function call \"{call}\"."
            )))
        }
    };

    // allowScalar：标量 `test` 逐分量应用。
    // test 为标量时，把它作用于 left/right 向量的每个分量。
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

    // 三侧均同型（全标量或同维向量）时逐分量应用；否则报错。
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

    /// 测试辅助：把 Rust f64 包装成 Value::Number。
    fn n(v: f64) -> Value {
        Value::Number(v)
    }

    // --- CesiumMath 等价实现 ---

    /// 验证 CesiumMath 等价实现（弧度/角度/符号/log2/clamp/lerp）。
    #[test]
    fn cesium_math_equivalents() {
        // 180° == PI 弧度，反向同理。
        assert!((js_to_radians(180.0) - std::f64::consts::PI).abs() < 1e-12);
        assert!((js_to_degrees(std::f64::consts::PI) - 180.0).abs() < 1e-12);
        // sign：正/负/零/NaN 四种情形。
        assert_eq!(js_sign(-3.0), -1.0);
        assert_eq!(js_sign(3.0), 1.0);
        assert_eq!(js_sign(0.0), 0.0);
        assert!(js_sign(f64::NAN).is_nan());
        assert_eq!(js_log2(8.0), 3.0);
        // clamp：上越/下越/区间内三种情形。
        assert_eq!(js_clamp(5.0, 0.0, 3.0), 3.0);
        assert_eq!(js_clamp(-5.0, 0.0, 3.0), 0.0);
        assert_eq!(js_clamp(1.0, 0.0, 3.0), 1.0);
        assert_eq!(js_lerp(0.0, 10.0, 0.5), 5.0);
    }

    // --- 一元函数 ---

    /// 一元标量函数：abs/sqrt/floor/ceil/round/exp2/log2/fract/sign。
    #[test]
    fn unary_scalar_functions() {
        // 常见一元数学函数在标量上的直接应用。
        assert_eq!(evaluate_unary_function("abs", n(-3.0)).unwrap(), n(3.0));
        assert_eq!(evaluate_unary_function("sqrt", n(9.0)).unwrap(), n(3.0));
        assert_eq!(evaluate_unary_function("floor", n(1.7)).unwrap(), n(1.0));
        assert_eq!(evaluate_unary_function("ceil", n(1.2)).unwrap(), n(2.0));
        // JS Math.round(-0.5) == 0（半值向 +infinity 取整）。
        assert_eq!(evaluate_unary_function("round", n(-0.5)).unwrap(), n(0.0));
        assert_eq!(evaluate_unary_function("exp2", n(3.0)).unwrap(), n(8.0));
        assert_eq!(evaluate_unary_function("log2", n(8.0)).unwrap(), n(3.0));
        assert_eq!(evaluate_unary_function("fract", n(1.25)).unwrap(), n(0.25));
        assert_eq!(evaluate_unary_function("sign", n(-9.0)).unwrap(), n(-1.0));
    }

    /// length/normalize 在标量与向量上的特例行为。
    #[test]
    fn unary_length_and_normalize() {
        // length 对标量取绝对值，对 vec3 取范数（3-4-0 直角边→5）。
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
        // normalize(number) == 1。
        assert_eq!(evaluate_unary_function("normalize", n(7.0)).unwrap(), n(1.0));
    }

    /// 一元函数对向量逐分量应用。
    #[test]
    fn unary_componentwise_vector() {
        // abs 作用到 vec2 的每个分量。
        let v = Value::Cartesian2(DVec2::new(-1.0, -2.0));
        match evaluate_unary_function("abs", v).unwrap() {
            Value::Cartesian2(u) => {
                assert_eq!(u.x, 1.0);
                assert_eq!(u.y, 2.0);
            }
            _ => panic!("expected Cartesian2"),
        }
    }

    /// 一元函数：错参数类型与未知函数名均报错。
    #[test]
    fn unary_wrong_arg_type_errors() {
        // 传入字符串不满足“向量或数字”约束。
        let err = evaluate_unary_function("abs", Value::String("x".into())).unwrap_err();
        assert!(err.message().contains("requires a vector or number argument"));
        let err = evaluate_unary_function("nope", n(1.0)).unwrap_err();
        assert!(err.message().contains("Unexpected function call"));
    }

    // --- 二元函数 ---

    /// 二元标量函数：pow/min/max/distance/dot 及 NaN 传播。
    #[test]
    fn binary_scalar_functions() {
        // 常规双参数标量运算。
        assert_eq!(evaluate_binary_function("pow", n(2.0), n(3.0)).unwrap(), n(8.0));
        assert_eq!(evaluate_binary_function("min", n(1.0), n(2.0)).unwrap(), n(1.0));
        assert_eq!(evaluate_binary_function("max", n(1.0), n(2.0)).unwrap(), n(2.0));
        // JS Math.min 的 NaN 传播。
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

    /// 二元向量函数：cross 叉积与 min 的标量逐分量应用。
    #[test]
    fn binary_vector_functions() {
        // x 轴 × y 轴 = z 轴。
        let a = Value::Cartesian3(DVec3::new(1.0, 0.0, 0.0));
        let b = Value::Cartesian3(DVec3::new(0.0, 1.0, 0.0));
        match evaluate_binary_function("cross", a, b).unwrap() {
            Value::Cartesian3(c) => assert_eq!(c, DVec3::new(0.0, 0.0, 1.0)),
            _ => panic!("expected Cartesian3"),
        }
        // min 的第二个参数为标量时逐分量应用。
        let v = Value::Cartesian2(DVec2::new(1.0, 5.0));
        match evaluate_binary_function("min", v, n(3.0)).unwrap() {
            Value::Cartesian2(u) => {
                assert_eq!(u.x, 1.0);
                assert_eq!(u.y, 3.0);
            }
            _ => panic!("expected Cartesian2"),
        }
    }

    /// cross 对非 vec3 参数应报错。
    #[test]
    fn binary_cross_requires_vec3() {
        // vec2 不满足 vec3 约束。
        let a = Value::Cartesian2(DVec2::new(1.0, 0.0));
        let b = Value::Cartesian2(DVec2::new(0.0, 1.0));
        let err = evaluate_binary_function("cross", a, b).unwrap_err();
        assert!(err.message().contains("requires vec3 arguments"));
    }

    // --- 三元函数 ---

    /// 三元函数：clamp 与 mix 的标量求值。
    #[test]
    fn ternary_clamp_and_mix() {
        // clamp(5,0,3)=3；mix(0,10,0.25)=2.5。
        assert_eq!(
            evaluate_ternary_function("clamp", n(5.0), n(0.0), n(3.0)).unwrap(),
            n(3.0)
        );
        assert_eq!(
            evaluate_ternary_function("mix", n(0.0), n(10.0), n(0.25)).unwrap(),
            n(2.5)
        );
    }

    /// 三元函数：标量 test 对向量逐分量应用。
    #[test]
    fn ternary_scalar_test_applies_componentwise() {
        // mix 的 t=0.5 标量逐分量作用于两个 vec2。
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

    /// 三元函数：左右维数不匹配应报错。
    #[test]
    fn ternary_mismatched_types_error() {
        // vec2 与 vec3 无法逐分量混合。
        let l = Value::Cartesian2(DVec2::new(0.0, 0.0));
        let r = Value::Cartesian3(DVec3::new(1.0, 2.0, 3.0));
        let err = evaluate_ternary_function("mix", l, r, n(0.5)).unwrap_err();
        assert!(err.message().contains("requires vector or number arguments"));
    }
}
