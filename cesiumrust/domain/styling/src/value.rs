//! Dynamic values, JS coercion/equality semantics and the runtime error type
//! for the 3D Tiles Styling expression engine.
//!
//! Ported from `cesium-rs/crates/cesium-scene/src/expression.rs`:
//! - `RegExpValue`            ← L47-104
//! - `Value` enum + impls     ← L109-224
//! - `number_to_js_string`    ← L228-240
//! - `js_parse_number`        ← L244-259
//!
//! which is itself the Rust port of upstream
//! `packages/engine/Source/Scene/Expression.js`.
//!
//! # M7-A scope notes (base layer)
//!
//! * DEVIATION (deps): the blueprint draws `RuntimeError` from `cesium_core` and
//!   `Cartesian2/3/4` from `cesium_core`. `cesium-styling` is an isolated domain
//!   crate (only `glam`), so a minimal local [`RuntimeError`] is defined here and
//!   the vector variants are backed by `glam::DVec2/DVec3/DVec4` (f64, domain
//!   precision). Componentwise compare / string form are identical.
//! * DEVIATION (regex → M7-B): [`RegExpValue`] now lives in `regex.rs` with a
//!   **real compiled** `::regex::Regex` (the `regex` crate is available in the
//!   offline registry). This module only re-uses it for the [`Value::RegExp`]
//!   variant, its `String()` form and equality; see `regex.rs` for
//!   `compile` / `test` / `exec_first_capture`.
//! * ADDITION (vs blueprint): [`Value::equals_loose`] implements JS abstract
//!   equality (`==`). The styling language itself only defines `===` / `!==`
//!   (Expression.js L1001-1004; blueprint `equals_strict`), so `PartialEq` and
//!   `equals_strict` stay **strict** (blueprint-faithful); `equals_loose` is
//!   provided so the full JS equality-quirk surface (loose coercion) can be
//!   expressed and unit-tested here and reused by M7-B / cross-cutting specs.

use glam::{DVec2, DVec3, DVec4};

use crate::regex::RegExpValue;

// ---------------------------------------------------------------------------
// Runtime error (mirrors `cesium_core::RuntimeError` / JS `RuntimeError`)
// ---------------------------------------------------------------------------

/// A runtime error raised while tokenizing, parsing or (in M7-B) evaluating an
/// expression. Mirrors `new RuntimeError(message)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    message: String,
}

impl RuntimeError {
    /// Mirrors `new RuntimeError(message)`; `None` yields an empty message.
    pub fn new(message: Option<&str>) -> RuntimeError {
        RuntimeError {
            message: message.unwrap_or("").to_string(),
        }
    }

    /// The error message text.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RuntimeError {}

/// Convenience constructor mirroring `new RuntimeError(message)`.
pub fn runtime_error(message: &str) -> RuntimeError {
    RuntimeError::new(Some(message))
}

// ---------------------------------------------------------------------------
// Dynamic values (mirrors the JS values the expression language produces)
// ---------------------------------------------------------------------------

/// The dynamic value type produced by expression evaluation, mirroring the
/// union of JS values the styling language returns.
///
/// NOTE: the blueprint `Value` has **no** `Color` variant — color literals
/// evaluate to [`Value::Cartesian4`] (rgba). The task brief's enum listing
/// mentioned `Color`; the blueprint (source of truth) is followed here.
#[derive(Debug, Clone)]
pub enum Value {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Cartesian2(DVec2),
    Cartesian3(DVec3),
    Cartesian4(DVec4),
    RegExp(RegExpValue),
    Array(Vec<Value>),
}

impl Value {
    /// `true` unless the value is `undefined` or `null`.
    pub fn is_defined(&self) -> bool {
        !matches!(self, Value::Undefined | Value::Null)
    }

    /// JS truthiness.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Undefined | Value::Null => false,
            Value::Boolean(b) => *b,
            Value::Number(n) => *n != 0.0 && !n.is_nan(),
            Value::String(s) => !s.is_empty(),
            _ => true,
        }
    }

    /// Mirrors `Boolean(value)`.
    pub fn boolean_conversion(&self) -> bool {
        self.is_truthy()
    }

    /// Mirrors `Number(value)` for the value types this language produces.
    pub fn number_conversion(&self) -> f64 {
        match self {
            Value::Number(n) => *n,
            Value::Boolean(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            Value::Null => 0.0,
            Value::String(s) => js_parse_number(s),
            // undefined, vectors, regex and arrays all become NaN when
            // coerced through Number().
            _ => f64::NAN,
        }
    }

    /// Mirrors `String(value)`.
    pub fn string_conversion(&self) -> String {
        match self {
            Value::Undefined => "undefined".to_string(),
            Value::Null => "null".to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Number(n) => number_to_js_string(*n),
            Value::String(s) => s.clone(),
            Value::Cartesian2(v) => format!("({}, {})", v.x, v.y),
            // NOTE: the blueprint (expression.rs L171) wrote `"({}, {})"` with
            // three args (a typo that does not compile); corrected to three
            // placeholders per upstream `Cartesian3.toString()` == "(x, y, z)".
            Value::Cartesian3(v) => format!("({}, {}, {})", v.x, v.y, v.z),
            Value::Cartesian4(v) => format!("({}, {}, {}, {})", v.x, v.y, v.z, v.w),
            Value::RegExp(r) => r.to_js_string(),
            Value::Array(items) => items
                .iter()
                .map(|item| match item {
                    Value::Undefined | Value::Null => String::new(),
                    other => other.string_conversion(),
                })
                .collect::<Vec<_>>()
                .join(","),
        }
    }

    /// Mirrors `left === right` (strict equality; Cartesian values compare
    /// componentwise, as in `_evaluateEqualsStrict`). `NaN === NaN` is `false`
    /// and `null === undefined` is `false`, exactly as in JS.
    pub fn equals_strict(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Undefined, Value::Undefined) | (Value::Null, Value::Null) => true,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Cartesian2(a), Value::Cartesian2(b)) => a.x == b.x && a.y == b.y,
            (Value::Cartesian3(a), Value::Cartesian3(b)) => {
                a.x == b.x && a.y == b.y && a.z == b.z
            }
            (Value::Cartesian4(a), Value::Cartesian4(b)) => {
                a.x == b.x && a.y == b.y && a.z == b.z && a.w == b.w
            }
            _ => false,
        }
    }

    /// JS abstract equality (`left == right`).
    ///
    /// ADDITION (see module docs): the styling language has no `==` operator, so
    /// the blueprint only defines [`Value::equals_strict`]. This method captures
    /// the ECMAScript Abstract Equality Comparison for the primitive types so the
    /// loose-equality quirks (`"1" == 1`, `null == undefined`, `NaN != NaN`) can
    /// be expressed and tested. Composite values (vectors / regex / array)
    /// compare structurally as in [`PartialEq`]; primitive-vs-composite is
    /// `false`.
    pub fn equals_loose(&self, other: &Value) -> bool {
        match (self, other) {
            // null/undefined are loosely equal to each other and to nothing else.
            (Value::Null, Value::Null)
            | (Value::Undefined, Value::Undefined)
            | (Value::Null, Value::Undefined)
            | (Value::Undefined, Value::Null) => true,
            (Value::Null, _) | (_, Value::Null) => false,
            (Value::Undefined, _) | (_, Value::Undefined) => false,
            // Boolean coerces to Number(1/0), then re-compares.
            (Value::Boolean(a), _) => {
                Value::Number(if *a { 1.0 } else { 0.0 }).equals_loose(other)
            }
            (_, Value::Boolean(b)) => {
                self.equals_loose(&Value::Number(if *b { 1.0 } else { 0.0 }))
            }
            // Number vs Number (NaN != NaN falls out of f64 `==`).
            (Value::Number(a), Value::Number(b)) => a == b,
            // String vs Number (either order): Number(string) == number.
            (Value::String(s), Value::Number(n)) => js_parse_number(s) == *n,
            (Value::Number(n), Value::String(s)) => *n == js_parse_number(s),
            // String vs String.
            (Value::String(a), Value::String(b)) => a == b,
            // Composites: same-kind structural compare; cross-kind is false.
            (Value::Cartesian2(a), Value::Cartesian2(b)) => a.x == b.x && a.y == b.y,
            (Value::Cartesian3(a), Value::Cartesian3(b)) => {
                a.x == b.x && a.y == b.y && a.z == b.z
            }
            (Value::Cartesian4(a), Value::Cartesian4(b)) => {
                a.x == b.x && a.y == b.y && a.z == b.z && a.w == b.w
            }
            (Value::RegExp(a), Value::RegExp(b)) => a.source == b.source && a.flags == b.flags,
            (Value::Array(a), Value::Array(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .zip(b.iter())
                        .all(|(left, right)| left.equals_loose(right))
            }
            _ => false,
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.string_conversion())
    }
}

/// Deep equality used by tests and callers: **strict**-equality semantics for
/// scalars/vectors (mirrors `_evaluateEqualsStrict`), plus source/flags
/// comparison for regex values and elementwise comparison for arrays.
///
/// This is blueprint-faithful (strict). Use [`Value::equals_loose`] for JS `==`.
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // RegExpValue holds a compiled Regex (no PartialEq); compare by
            // source + flags, exactly like the blueprint `PartialEq for Value`.
            (Value::RegExp(a), Value::RegExp(b)) => a.source == b.source && a.flags == b.flags,
            (Value::Array(a), Value::Array(b)) => a == b,
            _ => self.equals_strict(other),
        }
    }
}

/// Formats a number the way JS string interpolation does (mirrors the `${}`
/// interpolation used in all RuntimeError messages of Expression.js).
pub fn number_to_js_string(number: f64) -> String {
    if number.is_nan() {
        return "NaN".to_string();
    }
    if number.is_infinite() {
        return if number > 0.0 {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        };
    }
    format!("{number}")
}

/// Mirrors JS `Number("...")` string coercion (trimmed, decimal, exponent,
/// hex; empty string is `0`; anything else is `NaN`).
pub fn js_parse_number(text: &str) -> f64 {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return 0.0;
    }
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        if let Ok(value) = i64::from_str_radix(hex, 16) {
            return value as f64;
        }
        return f64::NAN;
    }
    trimmed.parse::<f64>().unwrap_or(f64::NAN)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nan() -> Value {
        Value::Number(f64::NAN)
    }

    // --- JS equality quirks (the four required by the task) ---

    #[test]
    fn nan_is_not_equal_to_nan() {
        // Strict, loose and PartialEq all agree: NaN != NaN.
        assert!(!nan().equals_strict(&nan()));
        assert!(!nan().equals_loose(&nan()));
        assert!(nan() != nan());
    }

    #[test]
    fn string_number_loose_equal() {
        // "1" == 1 (loose) is true ...
        assert!(Value::String("1".to_string()).equals_loose(&Value::Number(1.0)));
        // ... but strict (===) and the blueprint-faithful PartialEq are false.
        assert!(!Value::String("1".to_string()).equals_strict(&Value::Number(1.0)));
        assert!(Value::String("1".to_string()) != Value::Number(1.0));
    }

    #[test]
    fn null_loose_equals_undefined() {
        assert!(Value::Null.equals_loose(&Value::Undefined));
        assert!(Value::Undefined.equals_loose(&Value::Null));
        // Strict: null === undefined is false.
        assert!(!Value::Null.equals_strict(&Value::Undefined));
        assert!(Value::Null != Value::Undefined);
    }

    #[test]
    fn string_number_strict_not_equal() {
        // "5" === 5 is false (strict) ...
        assert!(!Value::String("5".to_string()).equals_strict(&Value::Number(5.0)));
        // ... while "5" == 5 is true (loose), demonstrating == vs ===.
        assert!(Value::String("5".to_string()).equals_loose(&Value::Number(5.0)));
    }

    #[test]
    fn boolean_loose_coercion() {
        assert!(Value::Boolean(true).equals_loose(&Value::Number(1.0)));
        assert!(Value::Boolean(false).equals_loose(&Value::Number(0.0)));
        assert!(Value::Boolean(true).equals_loose(&Value::String("1".to_string())));
        assert!(!Value::Boolean(true).equals_loose(&Value::Number(2.0)));
    }

    #[test]
    fn strict_same_type_equality() {
        assert!(Value::Number(3.0).equals_strict(&Value::Number(3.0)));
        assert!(Value::String("a".to_string()).equals_strict(&Value::String("a".to_string())));
        assert!(Value::Null.equals_strict(&Value::Null));
        assert!(Value::Undefined.equals_strict(&Value::Undefined));
        assert!(!Value::Number(3.0).equals_strict(&Value::Number(4.0)));
    }

    #[test]
    fn vector_strict_componentwise() {
        let a = Value::Cartesian3(DVec3::new(1.0, 2.0, 3.0));
        let b = Value::Cartesian3(DVec3::new(1.0, 2.0, 3.0));
        let c = Value::Cartesian3(DVec3::new(1.0, 2.0, 4.0));
        assert!(a.equals_strict(&b));
        assert!(!a.equals_strict(&c));
        // NaN component => not equal.
        let n = Value::Cartesian2(DVec2::new(f64::NAN, 0.0));
        assert!(!n.equals_strict(&n));
    }

    #[test]
    fn array_and_regex_partial_eq() {
        let a1 = Value::Array(vec![Value::Number(1.0), Value::Number(2.0)]);
        let a2 = Value::Array(vec![Value::Number(1.0), Value::Number(2.0)]);
        let a3 = Value::Array(vec![Value::Number(1.0)]);
        assert_eq!(a1, a2);
        assert_ne!(a1, a3);

        let r1 = Value::RegExp(RegExpValue::compile("ab", "gi").unwrap());
        let r2 = Value::RegExp(RegExpValue::compile("ab", "gi").unwrap());
        let r3 = Value::RegExp(RegExpValue::compile("ab", "g").unwrap());
        assert_eq!(r1, r2);
        assert_ne!(r1, r3);
    }

    // --- js_parse_number (JS Number() semantics) ---

    #[test]
    fn js_parse_number_cases() {
        assert_eq!(js_parse_number(""), 0.0);
        assert_eq!(js_parse_number("   "), 0.0);
        assert_eq!(js_parse_number("  42  "), 42.0);
        assert_eq!(js_parse_number("0x10"), 16.0);
        assert_eq!(js_parse_number("0XFF"), 255.0);
        assert_eq!(js_parse_number("1.25"), 1.25);
        assert_eq!(js_parse_number("1e3"), 1000.0);
        assert!(js_parse_number("abc").is_nan());
        assert!(js_parse_number("0xZZ").is_nan());
    }

    // --- number_to_js_string ---

    #[test]
    fn number_to_js_string_cases() {
        assert_eq!(number_to_js_string(f64::NAN), "NaN");
        assert_eq!(number_to_js_string(f64::INFINITY), "Infinity");
        assert_eq!(number_to_js_string(f64::NEG_INFINITY), "-Infinity");
        assert_eq!(number_to_js_string(5.0), "5");
        assert_eq!(number_to_js_string(-2.5), "-2.5");
    }

    // --- conversions / truthiness ---

    #[test]
    fn number_conversion_cases() {
        assert_eq!(Value::Null.number_conversion(), 0.0);
        assert_eq!(Value::Boolean(true).number_conversion(), 1.0);
        assert_eq!(Value::Boolean(false).number_conversion(), 0.0);
        assert_eq!(Value::String("7".to_string()).number_conversion(), 7.0);
        assert!(Value::Undefined.number_conversion().is_nan());
        assert!(Value::Cartesian2(DVec2::new(1.0, 2.0)).number_conversion().is_nan());
    }

    #[test]
    fn string_conversion_cases() {
        assert_eq!(Value::Undefined.string_conversion(), "undefined");
        assert_eq!(Value::Null.string_conversion(), "null");
        assert_eq!(Value::Boolean(true).string_conversion(), "true");
        assert_eq!(Value::Number(5.0).string_conversion(), "5");
        assert_eq!(Value::String("hi".to_string()).string_conversion(), "hi");
        assert_eq!(Value::Cartesian2(DVec2::new(1.0, 2.0)).string_conversion(), "(1, 2)");
        assert_eq!(
            Value::Cartesian4(DVec4::new(1.0, 2.0, 3.0, 4.0)).string_conversion(),
            "(1, 2, 3, 4)"
        );
        // Array: null/undefined render as empty, joined by ",".
        let arr = Value::Array(vec![Value::Number(1.0), Value::Null, Value::Number(3.0)]);
        assert_eq!(arr.string_conversion(), "1,,3");
    }

    #[test]
    fn truthiness_cases() {
        assert!(!Value::Undefined.is_truthy());
        assert!(!Value::Null.is_truthy());
        assert!(!Value::Boolean(false).is_truthy());
        assert!(Value::Boolean(true).is_truthy());
        assert!(!Value::Number(0.0).is_truthy());
        assert!(!Value::Number(f64::NAN).is_truthy());
        assert!(Value::Number(1.0).is_truthy());
        assert!(!Value::String("".to_string()).is_truthy());
        assert!(Value::String("a".to_string()).is_truthy());
        assert!(Value::Array(vec![]).is_truthy());
    }

    #[test]
    fn is_defined_cases() {
        assert!(!Value::Undefined.is_defined());
        assert!(!Value::Null.is_defined());
        assert!(Value::Number(0.0).is_defined());
        assert!(Value::String("".to_string()).is_defined());
    }

    #[test]
    fn regexp_to_js_string_sorts_flags() {
        assert_eq!(
            RegExpValue::compile("ab", "gi").unwrap().to_js_string(),
            "/ab/gi"
        );
        // Flags are sorted into `dgimsuy` order.
        assert_eq!(
            RegExpValue::compile("ab", "ig").unwrap().to_js_string(),
            "/ab/gi"
        );
        assert_eq!(RegExpValue::compile("x", "").unwrap().to_js_string(), "/x/");
    }

    #[test]
    fn runtime_error_message() {
        let e = runtime_error("boom");
        assert_eq!(e.message(), "boom");
        assert_eq!(format!("{e}"), "boom");
        assert_eq!(RuntimeError::new(None).message(), "");
    }
}
