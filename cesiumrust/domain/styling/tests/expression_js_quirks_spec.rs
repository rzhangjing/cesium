//! M7-C cross-cutting spec: the **JavaScript quirks** the styling expression
//! engine must reproduce faithfully.
//!
//! CesiumJS's `Expression.js` is written in JavaScript, so its semantics are
//! inherited from ECMAScript's famously surprising coercion / equality / number
//! rules. A Rust port that naively used `f64` and `==` would silently diverge.
//! This file pins the **11 quirk classes** catalogued during the M7 research
//! pass (Sam #37 §3.3), asserting the engine's public API reproduces each one.
//!
//! Red-line: this file lives only in `domain/styling/tests/`, exercises only the
//! crate's public API ([`Value`], [`Expression`], [`js_round`] / [`js_min`] /
//! [`js_max`], [`js_parse_number`] / [`number_to_js_string`]), and never touches
//! the legacy `tile_style` path (that is M7-D / #45 territory).
//!
//! The 11 classes:
//!  1. NaN propagation & `isNaN`
//!  2. Loose (`==`) vs strict (`===`) equality
//!  3. `+` operator type coercion (string concatenation)
//!  4. Truthiness (`Boolean(value)`)
//!  5. Number -> String, JS style (`1.0` -> `"1"`, `NaN` -> `"NaN"`)
//!  6. String -> Number parsing (hex, empty -> 0, garbage -> NaN)
//!  7. Negative zero (`-0`)
//!  8. `Math.round` halves round towards +infinity
//!  9. `Math.min` / `Math.max` NaN propagation
//! 10. Modulo sign follows the dividend (JS `%`)
//! 11. `null` vs `undefined` distinction & their string forms

use cesium_styling::{
    js_max, js_min, js_parse_number, js_round, number_to_js_string, Expression, Value,
};

/// Evaluate a variable-free expression to a [`Value`], panicking on any error.
fn ev(src: &str) -> Value {
    Expression::try_new(src, None)
        .unwrap_or_else(|e| panic!("parse failed for {src:?}: {}", e.message()))
        .evaluate(None)
        .unwrap_or_else(|e| panic!("eval failed for {src:?}: {}", e.message()))
}

/// Evaluate and downcast to `f64`.
fn evn(src: &str) -> f64 {
    match ev(src) {
        Value::Number(n) => n,
        other => panic!("expected Number from {src:?}, got {other:?}"),
    }
}

/// Evaluate and downcast to `String`.
fn evs(src: &str) -> String {
    match ev(src) {
        Value::String(s) => s,
        other => panic!("expected String from {src:?}, got {other:?}"),
    }
}

/// Evaluate and downcast to `bool`.
fn evb(src: &str) -> bool {
    match ev(src) {
        Value::Boolean(b) => b,
        other => panic!("expected Boolean from {src:?}, got {other:?}"),
    }
}

fn nan() -> Value {
    Value::Number(f64::NAN)
}

// ===========================================================================
// Quirk 1: NaN propagation & isNaN
// ===========================================================================

#[test]
fn quirk1_nan_is_not_equal_to_itself() {
    // The single most famous JS quirk: NaN !== NaN under every comparison.
    assert!(!nan().equals_strict(&nan()));
    assert!(!nan().equals_loose(&nan()));
    assert!(nan() != nan()); // PartialEq is strict semantics
}

#[test]
fn quirk1_nan_propagates_through_arithmetic() {
    // Number("abc") is NaN; NaN + 1 stays NaN (via the expression engine).
    assert!(evn("Number('abc')").is_nan());
    assert!(evn("Number('abc') + 1").is_nan());
    assert!(evn("Number('abc') * 0").is_nan());
    // 0/0 is NaN in JS (and in f64).
    assert!(evn("0 / 0").is_nan());
    // x/0 for non-zero x is Infinity, NOT NaN.
    assert_eq!(evn("1 / 0"), f64::INFINITY);
    assert_eq!(evn("0 - 1 / 0"), f64::NEG_INFINITY);
}

#[test]
fn quirk1_isnan_and_isfinite() {
    // isNaN(Number("abc")) -> true; isNaN(1) -> false.
    assert!(evb("isNaN(Number('abc'))"));
    assert!(!evb("isNaN(1)"));
    assert!(evb("isNaN(0 / 0)"));
    // isFinite(Infinity) -> false; isFinite(1) -> true; isFinite(NaN) -> false.
    assert!(evb("isFinite(1)"));
    assert!(!evb("isFinite(1 / 0)"));
    assert!(!evb("isFinite(Number('abc'))"));
}

#[test]
fn quirk1_nan_is_falsy() {
    assert!(!Value::Number(f64::NAN).is_truthy());
    assert!(!nan().boolean_conversion());
    // Its number conversion is itself NaN.
    assert!(nan().number_conversion().is_nan());
}

// ===========================================================================
// Quirk 2: loose (==) vs strict (===) equality
// ===========================================================================

#[test]
fn quirk2_string_number_loose_but_not_strict() {
    // "1" == 1 (loose) is true, "1" === 1 (strict) is false.
    assert!(Value::String("1".to_string()).equals_loose(&Value::Number(1.0)));
    assert!(!Value::String("1".to_string()).equals_strict(&Value::Number(1.0)));
    // The styling language only exposes === / !==; both are strict.
    assert!(!evb("'1' === 1"));
    assert!(evb("'1' !== 1"));
    assert!(evb("1 === 1"));
    assert!(evb("'1' === '1'"));
}

#[test]
fn quirk2_boolean_coerces_to_number_loosely() {
    // true == 1 and false == 0 under loose equality; strict says no.
    assert!(Value::Boolean(true).equals_loose(&Value::Number(1.0)));
    assert!(Value::Boolean(false).equals_loose(&Value::Number(0.0)));
    assert!(!Value::Boolean(true).equals_strict(&Value::Number(1.0)));
    // true == "1" loosely (both funnel through Number).
    assert!(Value::Boolean(true).equals_loose(&Value::String("1".to_string())));
}

#[test]
fn quirk2_null_loose_equals_undefined_only() {
    // null == undefined (loose) true, but null == 0 is FALSE (a classic trap).
    assert!(Value::Null.equals_loose(&Value::Undefined));
    assert!(Value::Undefined.equals_loose(&Value::Null));
    assert!(!Value::Null.equals_loose(&Value::Number(0.0)));
    assert!(!Value::Null.equals_loose(&Value::Boolean(false)));
    assert!(!Value::Undefined.equals_loose(&Value::Number(0.0)));
}

// ===========================================================================
// Quirk 3: `+` operator type coercion (string concatenation)
// ===========================================================================

#[test]
fn quirk3_plus_concatenates_when_either_side_is_string() {
    // If either operand is a String, + concatenates (JS's most bug-prone rule).
    assert_eq!(evs("'a' + 1"), "a1");
    assert_eq!(evs("1 + '2'"), "12");
    assert_eq!(evs("'a' + 'b'"), "ab");
    // Number + Number is arithmetic.
    assert_eq!(evn("1 + 2"), 3.0);
    // The empty-string coercion of null/undefined inside concatenation.
    assert_eq!(evs("'' + true"), "true");
    assert_eq!(evs("'' + false"), "false");
}

#[test]
fn quirk3_plus_coercion_order_matters() {
    // 1 + 2 + "3" == "33" but "1" + 2 + 3 == "123" (left-to-right).
    assert_eq!(evs("1 + 2 + '3'"), "33");
    assert_eq!(evs("'1' + 2 + 3"), "123");
}

// ===========================================================================
// Quirk 4: truthiness (Boolean(value))
// ===========================================================================

#[test]
fn quirk4_truthy_and_falsy_values() {
    // Falsy: undefined, null, false, 0, -0, NaN, "".
    assert!(!Value::Undefined.is_truthy());
    assert!(!Value::Null.is_truthy());
    assert!(!Value::Boolean(false).is_truthy());
    assert!(!Value::Number(0.0).is_truthy());
    assert!(!Value::Number(-0.0).is_truthy());
    assert!(!Value::Number(f64::NAN).is_truthy());
    assert!(!Value::String(String::new()).is_truthy());
    // Truthy: everything else, including "0" and "false" (non-empty strings!).
    assert!(Value::Boolean(true).is_truthy());
    assert!(Value::Number(1.0).is_truthy());
    assert!(Value::Number(f64::INFINITY).is_truthy());
    assert!(Value::String("0".to_string()).is_truthy());
    assert!(Value::String("false".to_string()).is_truthy());
    assert!(Value::String(" ".to_string()).is_truthy());
}

#[test]
fn quirk4_boolean_conversion_matches_truthiness() {
    assert!(Value::String("x".to_string()).boolean_conversion());
    assert!(!Value::Number(0.0).boolean_conversion());
    // Boolean(...) as a unary expression call.
    assert!(evb("Boolean(1)"));
    assert!(!evb("Boolean(0)"));
    assert!(!evb("Boolean('')"));
    assert!(evb("Boolean('0')"));
}

// ===========================================================================
// Quirk 5: Number -> String, JS style
// ===========================================================================

#[test]
fn quirk5_number_to_string_drops_trailing_zero() {
    // JS String(1.0) === "1" (no ".0"), String(2.5) === "2.5".
    assert_eq!(number_to_js_string(1.0), "1");
    assert_eq!(number_to_js_string(0.0), "0");
    assert_eq!(number_to_js_string(2.5), "2.5");
    assert_eq!(number_to_js_string(-3.0), "-3");
    assert_eq!(number_to_js_string(100.0), "100");
}

#[test]
fn quirk5_number_to_string_specials() {
    assert_eq!(number_to_js_string(f64::NAN), "NaN");
    assert_eq!(number_to_js_string(f64::INFINITY), "Infinity");
    assert_eq!(number_to_js_string(f64::NEG_INFINITY), "-Infinity");
}

#[test]
fn quirk5_string_conversion_of_value_types() {
    assert_eq!(Value::Number(1.0).string_conversion(), "1");
    assert_eq!(Value::Boolean(true).string_conversion(), "true");
    assert_eq!(Value::Boolean(false).string_conversion(), "false");
    // String(...) unary call through the engine.
    assert_eq!(evs("String(1)"), "1");
    assert_eq!(evs("String(2.5)"), "2.5");
}

// ===========================================================================
// Quirk 6: String -> Number parsing
// ===========================================================================

#[test]
fn quirk6_string_to_number_parsing() {
    // Leading/trailing whitespace is trimmed; decimal & exponent parse.
    assert_eq!(js_parse_number("  42  "), 42.0);
    assert_eq!(js_parse_number("3.5"), 3.5);
    assert_eq!(js_parse_number("1e3"), 1000.0);
    assert_eq!(js_parse_number("-2.5"), -2.5);
}

#[test]
fn quirk6_string_to_number_edge_cases() {
    // Empty (or whitespace-only) string is 0, NOT NaN — a JS trap.
    assert_eq!(js_parse_number(""), 0.0);
    assert_eq!(js_parse_number("   "), 0.0);
    // Hex literals parse.
    assert_eq!(js_parse_number("0x10"), 16.0);
    assert_eq!(js_parse_number("0XFF"), 255.0);
    // Garbage is NaN.
    assert!(js_parse_number("abc").is_nan());
    assert!(js_parse_number("12abc").is_nan());
    // Number("...") unary call reflects the same rules.
    assert_eq!(evn("Number('42')"), 42.0);
    assert_eq!(evn("Number('')"), 0.0);
    assert!(evn("Number('abc')").is_nan());
}

// ===========================================================================
// Quirk 7: negative zero (-0)
// ===========================================================================

#[test]
fn quirk7_negative_zero_is_falsy_and_equals_positive_zero() {
    let neg_zero = Value::Number(-0.0);
    // -0 == 0 under both equalities (f64 -0.0 == 0.0).
    assert!(neg_zero.equals_strict(&Value::Number(0.0)));
    assert!(neg_zero.equals_loose(&Value::Number(0.0)));
    // -0 is falsy.
    assert!(!neg_zero.is_truthy());
}

#[test]
fn quirk7_negative_zero_reciprocal_is_negative_infinity() {
    // 1 / -0 === -Infinity (the observable difference from +0).
    let r: f64 = 1.0 / -0.0;
    assert!(r.is_infinite() && r.is_sign_negative());
    assert_eq!(number_to_js_string(1.0 / -0.0), "-Infinity");
    assert_eq!(number_to_js_string(1.0 / 0.0), "Infinity");
}

#[test]
fn quirk7_negative_zero_from_expression() {
    // 0 * -1 yields -0.0 (sign preserved through f64 multiply).
    let n = evn("0 * (0 - 1)");
    assert_eq!(n, 0.0);
    assert!(n.is_sign_negative());
}

// ===========================================================================
// Quirk 8: Math.round halves round towards +infinity
// ===========================================================================

#[test]
fn quirk8_round_halves_go_up_not_away_from_zero() {
    // Math.round(-0.5) === 0 (NOT -1). Rust's f64::round would give -1.
    assert_eq!(js_round(-0.5), 0.0);
    assert_eq!(js_round(0.5), 1.0);
    assert_eq!(js_round(1.5), 2.0);
    assert_eq!(js_round(2.5), 3.0);
    assert_eq!(js_round(-1.5), -1.0);
    assert_eq!(js_round(-2.5), -2.0);
    // Direct evidence Rust's round differs.
    assert_eq!((-0.5f64).round(), -1.0);
    assert_ne!(js_round(-0.5), (-0.5f64).round());
}

#[test]
fn quirk8_round_through_expression() {
    assert_eq!(evn("round(0 - 0.5)"), 0.0);
    assert_eq!(evn("round(2.5)"), 3.0);
    assert_eq!(evn("round(1.4)"), 1.0);
}

// ===========================================================================
// Quirk 9: Math.min / Math.max NaN propagation
// ===========================================================================

#[test]
fn quirk9_min_max_propagate_nan() {
    // JS Math.min/max return NaN if ANY operand is NaN (unlike f64::min/max).
    assert!(js_min(f64::NAN, 1.0).is_nan());
    assert!(js_min(1.0, f64::NAN).is_nan());
    assert!(js_max(f64::NAN, 1.0).is_nan());
    assert!(js_max(1.0, f64::NAN).is_nan());
    // f64 ignores NaN — the divergence being guarded against.
    assert_eq!(f64::NAN.min(1.0), 1.0);
    assert_eq!(f64::NAN.max(1.0), 1.0);
}

#[test]
fn quirk9_min_max_normal_values() {
    assert_eq!(js_min(1.0, 2.0), 1.0);
    assert_eq!(js_min(-1.0, -2.0), -2.0);
    assert_eq!(js_max(1.0, 2.0), 2.0);
    assert_eq!(js_max(-1.0, -2.0), -1.0);
    // Through the expression engine (scalar min/max).
    assert_eq!(evn("min(3, 7)"), 3.0);
    assert_eq!(evn("max(3, 7)"), 7.0);
    assert!(evn("min(3, Number('x'))").is_nan());
}

// ===========================================================================
// Quirk 10: modulo sign follows the dividend (JS `%`)
// ===========================================================================

#[test]
fn quirk10_modulo_sign_follows_dividend() {
    // JS: -7 % 3 === -1 and 7 % -3 === 1 (sign of the LEFT operand).
    // This matches Rust's f64 `%` (remainder), NOT rem_euclid.
    assert_eq!(evn("(0 - 7) % 3"), -1.0); // -7 % 3 === -1
    assert_eq!((-7.0f64) % 3.0, -1.0);
    assert_eq!(7.0 % -3.0, 1.0);
    assert_eq!((-7.0f64) % -3.0, -1.0);
    // rem_euclid would give a different (always non-negative) answer.
    assert_eq!((-7.0f64).rem_euclid(3.0), 2.0);
    assert_ne!((-7.0f64) % 3.0, (-7.0f64).rem_euclid(3.0));
}

#[test]
fn quirk10_modulo_through_expression() {
    assert_eq!(evn("7 % 3"), 1.0);
    assert_eq!(evn("(0 - 7) % 3"), -1.0);
    assert_eq!(evn("7 % (0 - 3)"), 1.0);
    // x % 0 is NaN in JS.
    assert!(evn("7 % 0").is_nan());
}

// ===========================================================================
// Quirk 11: null vs undefined distinction & their string forms
// ===========================================================================

#[test]
fn quirk11_null_and_undefined_are_distinct_strictly() {
    // null === undefined is false; null == undefined is true (quirk 2 overlap).
    assert!(!Value::Null.equals_strict(&Value::Undefined));
    assert!(Value::Null != Value::Undefined);
    assert!(Value::Null.equals_loose(&Value::Undefined));
    // Both are "not defined".
    assert!(!Value::Null.is_defined());
    assert!(!Value::Undefined.is_defined());
    assert!(Value::Number(0.0).is_defined());
}

#[test]
fn quirk11_null_and_undefined_string_forms() {
    // String(null) === "null", String(undefined) === "undefined".
    assert_eq!(Value::Null.string_conversion(), "null");
    assert_eq!(Value::Undefined.string_conversion(), "undefined");
}

#[test]
fn quirk11_null_and_undefined_number_forms() {
    // Number(null) === 0 but Number(undefined) === NaN — a subtle asymmetry.
    assert_eq!(Value::Null.number_conversion(), 0.0);
    assert!(Value::Undefined.number_conversion().is_nan());
}

#[test]
fn quirk11_null_and_undefined_in_arrays_join_as_empty() {
    // [null, undefined, 1].toString() === ",,1" (null/undefined -> "").
    let arr = Value::Array(vec![Value::Null, Value::Undefined, Value::Number(1.0)]);
    assert_eq!(arr.string_conversion(), ",,1");
}
