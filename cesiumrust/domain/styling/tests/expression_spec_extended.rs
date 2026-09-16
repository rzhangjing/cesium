//! M7-C: large-scale port of the upstream CesiumJS
//! `packages/engine/Specs/Scene/ExpressionSpec.js` (269 `it()` cases).
//!
//! This file ports the CPU-interpretable subset of the upstream spec against the
//! new jsep-based styling engine (`cesium_styling::Expression`). Each `#[test]`
//! maps to one upstream `it()` block (large blocks are split by category so the
//! ported-case count faithfully reflects the upstream assertions).
//!
//! ## Deliberately NOT ported here (see `#[ignore]` cases + notes)
//! * `getShaderFunction` / `getShaderExpression` (~60 upstream cases): GLSL
//!   codegen is **deferred** to a later milestone (the engine only interprets on
//!   the CPU), so those cases are marked `#[ignore = "GLSL codegen deferred"]`.
//! * `regExp("(?<=\\s)")` look-behind: the Rust `regex` crate has no
//!   look-around, so those cases are `#[ignore = "regex crate has no look-around"]`.
//! * Plain JS *object* feature properties (`{ street, city }`) and the
//!   `feature.content.tileset.timeSinceLoad` model: the isolated domain `Value`
//!   enum has no object/tileset variant, so only the vector/number/array/string
//!   member sub-assertions of those blocks are ported.
//!
//! Red-line: this file lives under `domain/styling/tests/` and only touches the
//! public API of `cesium-styling`; the legacy `tile_style` path is untouched.

use std::collections::HashMap;

use cesium_styling::{Expression, ExpressionFeature, Value};
use glam::{DVec2, DVec3, DVec4};

// ---------------------------------------------------------------------------
// MockFeature (mirrors the upstream `MockFeature`)
// ---------------------------------------------------------------------------

struct MockFeature {
    props: HashMap<String, Value>,
    class: Option<String>,
    inherited: Option<String>,
}

impl MockFeature {
    fn new() -> Self {
        MockFeature {
            props: HashMap::new(),
            class: None,
            inherited: None,
        }
    }
    fn prop(mut self, k: &str, v: Value) -> Self {
        self.props.insert(k.to_string(), v);
        self
    }
    fn set_class(mut self, c: &str) -> Self {
        self.class = Some(c.to_string());
        self
    }
    fn set_inherited_class(mut self, c: &str) -> Self {
        self.inherited = Some(c.to_string());
        self
    }
}

impl ExpressionFeature for MockFeature {
    fn get_property_inherited(&self, name: &str) -> Option<Value> {
        self.props.get(name).cloned()
    }
    fn is_exact_class(&self, class_name: &Value) -> bool {
        match (&self.class, class_name) {
            (Some(cls), Value::String(n)) => cls == n,
            _ => false,
        }
    }
    fn is_class(&self, class_name: &Value) -> bool {
        match class_name {
            Value::String(n) => {
                self.class.as_deref() == Some(n.as_str())
                    || self.inherited.as_deref() == Some(n.as_str())
            }
            _ => false,
        }
    }
    fn get_exact_class_name(&self) -> Option<Value> {
        self.class.clone().map(Value::String)
    }
}

// ---------------------------------------------------------------------------
// Evaluation helpers
// ---------------------------------------------------------------------------

const EPSILON7: f64 = 1e-7;
const EPSILON10: f64 = 1e-10;

fn ev(src: &str) -> Value {
    Expression::try_new(src, None)
        .unwrap_or_else(|e| panic!("parse {src:?}: {e}"))
        .evaluate(None)
        .unwrap_or_else(|e| panic!("eval {src:?}: {e}"))
}

fn ev_with(src: &str, defines: &HashMap<String, String>) -> Value {
    Expression::try_new(src, Some(defines))
        .unwrap_or_else(|e| panic!("parse {src:?}: {e}"))
        .evaluate(None)
        .unwrap_or_else(|e| panic!("eval {src:?}: {e}"))
}

fn evf(src: &str, f: &MockFeature) -> Value {
    Expression::try_new(src, None)
        .unwrap_or_else(|e| panic!("parse {src:?}: {e}"))
        .evaluate(Some(f))
        .unwrap_or_else(|e| panic!("eval {src:?}: {e}"))
}

fn evf_opt(src: &str, f: Option<&MockFeature>) -> Value {
    Expression::try_new(src, None)
        .unwrap_or_else(|e| panic!("parse {src:?}: {e}"))
        .evaluate(f.map(|x| x as &dyn ExpressionFeature))
        .unwrap_or_else(|e| panic!("eval {src:?}: {e}"))
}

fn n(src: &str) -> f64 {
    match ev(src) {
        Value::Number(x) => x,
        o => panic!("{src:?} not a number: {o}"),
    }
}
fn nf(src: &str, f: &MockFeature) -> f64 {
    match evf(src, f) {
        Value::Number(x) => x,
        o => panic!("{src:?} not a number: {o}"),
    }
}
fn b(src: &str) -> bool {
    match ev(src) {
        Value::Boolean(x) => x,
        o => panic!("{src:?} not a boolean: {o}"),
    }
}
fn bf(src: &str, f: &MockFeature) -> bool {
    match evf(src, f) {
        Value::Boolean(x) => x,
        o => panic!("{src:?} not a boolean: {o}"),
    }
}
fn s(src: &str) -> String {
    match ev(src) {
        Value::String(x) => x,
        o => panic!("{src:?} not a string: {o}"),
    }
}
fn sf(src: &str, f: &MockFeature) -> String {
    match evf(src, f) {
        Value::String(x) => x,
        o => panic!("{src:?} not a string: {o}"),
    }
}
fn c4(src: &str) -> DVec4 {
    match ev(src) {
        Value::Cartesian4(x) => x,
        o => panic!("{src:?} not a vec4: {o}"),
    }
}
fn v2(src: &str) -> DVec2 {
    match ev(src) {
        Value::Cartesian2(x) => x,
        o => panic!("{src:?} not a vec2: {o}"),
    }
}
fn v3(src: &str) -> DVec3 {
    match ev(src) {
        Value::Cartesian3(x) => x,
        o => panic!("{src:?} not a vec3: {o}"),
    }
}

/// Asserts construction (parse) fails with a RuntimeError, mirroring upstream
/// `expect(function () { return new Expression(src); }).toThrowError(RuntimeError)`.
fn parse_throws(src: &str) {
    assert!(
        Expression::try_new(src, None).is_err(),
        "expected a parse error for {src:?}"
    );
}

/// Asserts the expression parses but evaluation fails, mirroring upstream
/// `expression = new Expression(src); expect(() => expression.evaluate()).toThrowError(...)`.
fn eval_throws(src: &str) {
    let e = Expression::try_new(src, None).unwrap_or_else(|err| panic!("{src:?} should parse: {err}"));
    assert!(
        e.evaluate(None).is_err(),
        "expected an evaluate error for {src:?}"
    );
}

fn close(actual: f64, expected: f64, eps: f64) {
    assert!(
        (actual - expected).abs() <= eps,
        "{actual} != {expected} (eps {eps})"
    );
}
fn close2(actual: DVec2, expected: [f64; 2], eps: f64) {
    close(actual.x, expected[0], eps);
    close(actual.y, expected[1], eps);
}
fn close3(actual: DVec3, expected: [f64; 3], eps: f64) {
    close(actual.x, expected[0], eps);
    close(actual.y, expected[1], eps);
    close(actual.z, expected[2], eps);
}
fn close4(actual: DVec4, expected: [f64; 4], eps: f64) {
    close(actual.x, expected[0], eps);
    close(actual.y, expected[1], eps);
    close(actual.z, expected[2], eps);
    close(actual.w, expected[3], eps);
}

/// Standard HSL->RGB (matches Cesium `Color.fromHsl`), used to compute the
/// expected value of the `hsl()`/`hsla()` literal-color cases independently.
fn hsl_expected(h: f64, s: f64, l: f64, a: f64) -> [f64; 4] {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = (h % 1.0 + 1.0) % 1.0 * 6.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match hp as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    [r1 + m, g1 + m, b1 + m, a]
}

fn rgb_bytes(r: u8, g: u8, b: u8) -> [f64; 4] {
    [r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0, 1.0]
}

// ===========================================================================
// Backslashes / variables / defines
// ===========================================================================

#[test]
fn parses_backslashes() {
    // upstream: new Expression('"\\he\\\\\\ll\\\\o"') -> "\he\\\ll\\o"
    // The parser preserves the invalid escapes verbatim, so the evaluated
    // string equals the inner content byte-for-byte.
    assert_eq!(s(r#""\he\\\ll\\o""#), r"\he\\\ll\\o");
}

#[test]
fn evaluates_variable_numbers() {
    let f = MockFeature::new()
        .prop("height", Value::Number(10.0))
        .prop("width", Value::Number(5.0));
    assert_eq!(evf("${height}", &f), Value::Number(10.0));
    assert_eq!(sf("'${height}'", &f), "10");
    assert_eq!(nf("${height}/${width}", &f), 2.0);
}

#[test]
fn evaluates_variable_string_interpolation() {
    let f = MockFeature::new()
        .prop("string", Value::String("hello".into()))
        .prop("height", Value::Number(10.0));
    assert_eq!(sf("${string}", &f), "hello");
    assert_eq!(sf("'replace ${string}'", &f), "replace hello");
    assert_eq!(
        sf("'replace ${string} multiple ${height}'", &f),
        "replace hello multiple 10"
    );
    assert_eq!(sf("'replace ${height} ${string}'", &f), "replace 10 hello");
    assert_eq!(sf(r#""replace ${string}""#, &f), "replace hello");
    // unterminated ${ inside a string is left verbatim
    assert_eq!(sf("'replace ${string'", &f), "replace ${string");
}

#[test]
fn evaluates_variable_boolean_vector_null_undefined() {
    let f = MockFeature::new()
        .prop("boolean", Value::Boolean(true))
        .prop("vector", Value::Cartesian3(DVec3::new(1.0, 0.0, 0.0)))
        .prop("null", Value::Null)
        .prop("undefined", Value::Undefined);
    assert_eq!(evf("${boolean}", &f), Value::Boolean(true));
    assert_eq!(sf("'${boolean}'", &f), "true");
    assert_eq!(evf("${vector}", &f), Value::Cartesian3(DVec3::new(1.0, 0.0, 0.0)));
    assert_eq!(sf("'${vector}'", &f), "(1, 0, 0)");
    assert_eq!(evf("${null}", &f), Value::Null);
    assert_eq!(sf("'${null}'", &f), "");
    assert_eq!(evf("${undefined}", &f), Value::Undefined);
    assert_eq!(sf("'${undefined}'", &f), "");
}

#[test]
fn evaluates_variable_in_function_expression() {
    let f = MockFeature::new()
        .prop("height", Value::Number(10.0))
        .prop("width", Value::Number(5.0));
    // abs(-10) + max(10,5) + clamp(10,0,2) = 10 + 10 + 2 = 22
    assert_eq!(
        nf("abs(-${height}) + max(${height}, ${width}) + clamp(${height}, 0, 2)", &f),
        22.0
    );
}

#[test]
fn variable_unmatched_brace_throws() {
    parse_throws("${height");
}

#[test]
fn evaluates_variable_to_undefined_if_feature_undefined() {
    assert_eq!(evf_opt("${height}", None), Value::Undefined);
    assert_eq!(evf_opt("${vector.x}", None), Value::Undefined);
    assert_eq!(evf_opt("${feature}", None), Value::Undefined);
    assert_eq!(evf_opt("${feature.vector}", None), Value::Undefined);
    assert_eq!(evf_opt(r#"${vector["x"]}"#, None), Value::Undefined);
    assert_eq!(evf_opt(r#"${feature["vector"]}"#, None), Value::Undefined);
    // inside a string, undefined interpolates to "" (not "undefined")
    assert_eq!(sf("'${height}'", &MockFeature::new()), "");
}

#[test]
fn evaluates_with_defines() {
    let mut defines = HashMap::new();
    defines.insert("halfHeight".to_string(), "${Height}/2".to_string());
    let f = MockFeature::new().prop("Height", Value::Number(10.0));
    let e = Expression::try_new("${halfHeight}", Some(&defines)).unwrap();
    assert_eq!(e.evaluate(Some(&f)).unwrap(), Value::Number(5.0));
}

#[test]
fn evaluates_with_defines_honoring_order_of_operations() {
    let mut defines = HashMap::new();
    defines.insert("value".to_string(), "1 + 2".to_string());
    assert_eq!(ev_with("5.0 * ${value}", &defines), Value::Number(15.0));
}

#[test]
fn evaluate_takes_result_argument_value() {
    // The out-`result` identity (`toBe(result)`) is a renderer-side concern; the
    // returned *value* is portable.
    assert_eq!(ev("vec3(1.0)"), Value::Cartesian3(DVec3::new(1.0, 1.0, 1.0)));
}

#[test]
fn evaluate_takes_a_color_result_argument_value() {
    assert_eq!(
        Expression::try_new(r#"color("red")"#, None)
            .unwrap()
            .evaluate_color(None)
            .unwrap(),
        DVec4::new(1.0, 0.0, 0.0, 1.0)
    );
}

#[test]
fn gets_expressions() {
    let src = "(regExp('^Chest').test(${County})) && (${YearBuilt} >= 1970)";
    let e = Expression::try_new(src, None).unwrap();
    assert_eq!(e.expression(), src);
}

// ===========================================================================
// Parse errors
// ===========================================================================

#[test]
fn throws_on_invalid_expressions() {
    parse_throws("");
    parse_throws("this");
    parse_throws("2; 3;");
}

#[test]
fn throws_on_unknown_characters() {
    parse_throws("#");
}

#[test]
fn throws_on_unmatched_parenthesis() {
    parse_throws("((true)");
    parse_throws("(true))");
}

#[test]
fn throws_on_unknown_identifiers() {
    parse_throws("flse");
}

#[test]
fn throws_on_unknown_function_calls() {
    parse_throws("unknown()");
}

#[test]
fn throws_on_unknown_member_function_calls() {
    parse_throws("regExp().unknown()");
}

#[test]
fn throws_with_unsupported_operators() {
    parse_throws("~1");
    parse_throws("2 | 3");
    parse_throws("2 & 3");
    parse_throws("2 << 3");
    parse_throws("2 >> 3");
    parse_throws("2 >>> 3");
}

// ===========================================================================
// Literals + conversions
// ===========================================================================

#[test]
fn evaluates_literal_null() {
    assert_eq!(ev("null"), Value::Null);
}

#[test]
fn evaluates_literal_undefined() {
    assert_eq!(ev("undefined"), Value::Undefined);
}

#[test]
fn evaluates_literal_boolean() {
    assert_eq!(ev("true"), Value::Boolean(true));
    assert_eq!(ev("false"), Value::Boolean(false));
}

#[test]
fn converts_to_literal_boolean() {
    assert_eq!(ev("Boolean()"), Value::Boolean(false));
    assert_eq!(ev("Boolean(1)"), Value::Boolean(true));
    assert_eq!(ev(r#"Boolean("true")"#), Value::Boolean(true));
}

#[test]
fn evaluates_literal_number() {
    assert_eq!(n("1"), 1.0);
    assert_eq!(n("0"), 0.0);
    assert!(n("NaN").is_nan());
    assert_eq!(n("Infinity"), f64::INFINITY);
}

#[test]
fn evaluates_math_constants() {
    close(n("Math.PI"), std::f64::consts::PI, EPSILON10);
    close(n("Math.E"), std::f64::consts::E, EPSILON10);
}

#[test]
fn evaluates_number_constants() {
    assert_eq!(n("Number.POSITIVE_INFINITY"), f64::INFINITY);
}

#[test]
fn converts_to_literal_number() {
    assert_eq!(n("Number()"), 0.0);
    assert_eq!(n(r#"Number("1")"#), 1.0);
    assert_eq!(n("Number(true)"), 1.0);
}

#[test]
fn evaluates_literal_string() {
    assert_eq!(s("'hello'"), "hello");
    assert_eq!(s("'Cesium'"), "Cesium");
    assert_eq!(s(r#""Cesium""#), "Cesium");
}

#[test]
fn converts_to_literal_string() {
    assert_eq!(s("String()"), "");
    assert_eq!(s("String(1)"), "1");
    assert_eq!(s("String(true)"), "true");
}

// ===========================================================================
// Literal colors
// ===========================================================================

#[test]
fn evaluates_literal_color_hex() {
    close4(c4("color('#ffffff')"), [1.0, 1.0, 1.0, 1.0], EPSILON7);
    close4(c4("color('#00FFFF')"), [0.0, 1.0, 1.0, 1.0], EPSILON7);
    close4(c4("color('#fff')"), [1.0, 1.0, 1.0, 1.0], EPSILON7);
    close4(c4("color('#0FF')"), [0.0, 1.0, 1.0, 1.0], EPSILON7);
}

#[test]
fn evaluates_literal_color_keyword() {
    close4(c4("color('white')"), [1.0, 1.0, 1.0, 1.0], EPSILON7);
    close4(c4("color('cyan')"), [0.0, 1.0, 1.0, 1.0], EPSILON7);
}

#[test]
fn evaluates_literal_color_keyword_with_alpha() {
    close4(c4("color('white', 0.5)"), [1.0, 1.0, 1.0, 0.5], EPSILON7);
}

#[test]
fn evaluates_literal_color_rgb() {
    close4(c4("rgb(255, 255, 255)"), [1.0, 1.0, 1.0, 1.0], EPSILON7);
    close4(c4("rgb(100, 255, 190)"), rgb_bytes(100, 255, 190), EPSILON7);
}

#[test]
fn evaluates_literal_color_hsl() {
    close4(c4("hsl(0, 0, 1)"), [1.0, 1.0, 1.0, 1.0], EPSILON7);
    close4(c4("hsl(1.0, 0.6, 0.7)"), hsl_expected(1.0, 0.6, 0.7, 1.0), EPSILON7);
}

#[test]
fn evaluates_literal_color_rgba() {
    close4(c4("rgba(255, 255, 255, 0.5)"), [1.0, 1.0, 1.0, 0.5], EPSILON7);
    close4(
        c4("rgba(100, 255, 190, 0.25)"),
        [100.0 / 255.0, 1.0, 190.0 / 255.0, 0.25],
        EPSILON7,
    );
}

#[test]
fn evaluates_literal_color_hsla() {
    close4(c4("hsla(0, 0, 1, 0.5)"), [1.0, 1.0, 1.0, 0.5], EPSILON7);
    close4(
        c4("hsla(1.0, 0.6, 0.7, 0.75)"),
        hsl_expected(1.0, 0.6, 0.7, 0.75),
        EPSILON7,
    );
}

#[test]
fn evaluates_literal_color_default() {
    close4(c4("color()"), [1.0, 1.0, 1.0, 1.0], EPSILON7);
}

#[test]
fn color_constructors_throw_with_wrong_number_of_arguments() {
    parse_throws("rgb(255, 255)");
    parse_throws("hsl(1, 1)");
    parse_throws("rgba(255, 255, 255)");
    parse_throws("hsla(1, 1, 1)");
}

#[test]
fn evaluates_color_with_expressions_as_arguments() {
    let f = MockFeature::new()
        .prop("hex6", Value::String("#ffffff".into()))
        .prop("hex3", Value::String("#fff".into()))
        .prop("keyword", Value::String("white".into()))
        .prop("alpha", Value::Number(0.2));
    close4(
        match evf("color(${hex6})", &f) {
            Value::Cartesian4(v) => v,
            o => panic!("not a color: {o}"),
        },
        [1.0, 1.0, 1.0, 1.0],
        EPSILON7,
    );
    let c = match evf("color(${keyword}, ${alpha} + 0.6)", &f) {
        Value::Cartesian4(v) => v,
        o => panic!("not a color: {o}"),
    };
    close4(c, [1.0, 1.0, 1.0, 0.8], EPSILON7);
}

#[test]
fn evaluates_rgb_with_expressions_as_arguments() {
    let f = MockFeature::new()
        .prop("red", Value::Number(100.0))
        .prop("green", Value::Number(200.0))
        .prop("blue", Value::Number(255.0));
    let c = match evf("rgb(${red}, ${green}, ${blue})", &f) {
        Value::Cartesian4(v) => v,
        o => panic!("not a color: {o}"),
    };
    close4(c, rgb_bytes(100, 200, 255), EPSILON7);
    let c2 = match evf("rgb(${red}/2, ${green}/2, ${blue})", &f) {
        Value::Cartesian4(v) => v,
        o => panic!("not a color: {o}"),
    };
    close4(c2, rgb_bytes(50, 100, 255), EPSILON7);
}

#[test]
fn evaluates_hsl_with_expressions_as_arguments() {
    let f = MockFeature::new()
        .prop("h", Value::Number(0.0))
        .prop("s", Value::Number(0.0))
        .prop("l", Value::Number(1.0));
    let c = match evf("hsl(${h}, ${s}, ${l})", &f) {
        Value::Cartesian4(v) => v,
        o => panic!("not a color: {o}"),
    };
    close4(c, [1.0, 1.0, 1.0, 1.0], EPSILON7);
    let c2 = match evf("hsl(${h} + 0.2, ${s} + 1.0, ${l} - 0.5)", &f) {
        Value::Cartesian4(v) => v,
        o => panic!("not a color: {o}"),
    };
    close4(c2, hsl_expected(0.2, 1.0, 0.5, 1.0), EPSILON7);
}

#[test]
fn evaluates_rgba_with_expressions_as_arguments() {
    let f = MockFeature::new()
        .prop("red", Value::Number(100.0))
        .prop("green", Value::Number(200.0))
        .prop("blue", Value::Number(255.0))
        .prop("a", Value::Number(0.3));
    let c = match evf("rgba(${red}, ${green}, ${blue}, ${a})", &f) {
        Value::Cartesian4(v) => v,
        o => panic!("not a color: {o}"),
    };
    close4(c, [100.0 / 255.0, 200.0 / 255.0, 1.0, 0.3], EPSILON7);
}

#[test]
fn evaluates_hsla_with_expressions_as_arguments() {
    let f = MockFeature::new()
        .prop("h", Value::Number(0.0))
        .prop("s", Value::Number(0.0))
        .prop("l", Value::Number(1.0))
        .prop("a", Value::Number(1.0));
    let c = match evf("hsla(${h}, ${s}, ${l}, ${a})", &f) {
        Value::Cartesian4(v) => v,
        o => panic!("not a color: {o}"),
    };
    close4(c, [1.0, 1.0, 1.0, 1.0], EPSILON7);
    let c2 = match evf("hsla(${h} + 0.2, ${s} + 1.0, ${l} - 0.5, ${a} / 4)", &f) {
        Value::Cartesian4(v) => v,
        o => panic!("not a color: {o}"),
    };
    close4(c2, hsl_expected(0.2, 1.0, 0.5, 0.25), EPSILON7);
}

// ===========================================================================
// Color properties
// ===========================================================================

#[test]
fn evaluates_color_properties_rgba_names() {
    assert_eq!(n("color('#ffffff').r"), 1.0);
    assert_eq!(n("rgb(255, 255, 0).g"), 1.0);
    assert_eq!(n(r#"color("cyan").b"#), 1.0);
    assert_eq!(n("rgba(255, 255, 0, 0.5).a"), 0.5);
}

#[test]
fn evaluates_color_properties_xyzw_names() {
    assert_eq!(n("color('#ffffff').x"), 1.0);
    assert_eq!(n("rgb(255, 255, 0).y"), 1.0);
    assert_eq!(n(r#"color("cyan").z"#), 1.0);
    assert_eq!(n("rgba(255, 255, 0, 0.5).w"), 0.5);
}

#[test]
fn evaluates_color_properties_numeric_index() {
    assert_eq!(n("color('#ffffff')[0]"), 1.0);
    assert_eq!(n("rgb(255, 255, 0)[1]"), 1.0);
    assert_eq!(n(r#"color("cyan")[2]"#), 1.0);
    assert_eq!(n("rgba(255, 255, 0, 0.5)[3]"), 0.5);
}

#[test]
fn evaluates_color_properties_string_rgba_index() {
    assert_eq!(n(r#"color('#ffffff')["r"]"#), 1.0);
    assert_eq!(n(r#"rgb(255, 255, 0)["g"]"#), 1.0);
    assert_eq!(n(r#"color("cyan")["b"]"#), 1.0);
    assert_eq!(n(r#"rgba(255, 255, 0, 0.5)["a"]"#), 0.5);
}

#[test]
fn evaluates_color_properties_string_xyzw_index() {
    assert_eq!(n(r#"color('#ffffff')["x"]"#), 1.0);
    assert_eq!(n(r#"rgb(255, 255, 0)["y"]"#), 1.0);
    assert_eq!(n(r#"color("cyan")["z"]"#), 1.0);
    assert_eq!(n(r#"rgba(255, 255, 0, 0.5)["w"]"#), 0.5);
}

// ===========================================================================
// Vector literals
// ===========================================================================

#[test]
fn evaluates_vec2() {
    assert_eq!(v2("vec2(2.0)"), DVec2::new(2.0, 2.0));
    assert_eq!(v2("vec2(3.0, 4.0)"), DVec2::new(3.0, 4.0));
    assert_eq!(v2("vec2(vec2(3.0, 4.0))"), DVec2::new(3.0, 4.0));
    assert_eq!(v2("vec2(vec3(3.0, 4.0, 5.0))"), DVec2::new(3.0, 4.0));
    assert_eq!(v2("vec2(vec4(3.0, 4.0, 5.0, 6.0))"), DVec2::new(3.0, 4.0));
}

#[test]
fn throws_if_vec2_has_invalid_number_of_arguments() {
    eval_throws("vec2()");
    eval_throws("vec2(3.0, 4.0, 5.0)");
    eval_throws("vec2(vec2(3.0, 4.0), 5.0)");
}

#[test]
fn throws_if_vec2_has_invalid_argument() {
    eval_throws(r#"vec2("1")"#);
}

#[test]
fn evaluates_vec3() {
    assert_eq!(v3("vec3(2.0)"), DVec3::new(2.0, 2.0, 2.0));
    assert_eq!(v3("vec3(3.0, 4.0, 5.0)"), DVec3::new(3.0, 4.0, 5.0));
    assert_eq!(v3("vec3(vec2(3.0, 4.0), 5.0)"), DVec3::new(3.0, 4.0, 5.0));
    assert_eq!(v3("vec3(3.0, vec2(4.0, 5.0))"), DVec3::new(3.0, 4.0, 5.0));
    assert_eq!(v3("vec3(vec3(3.0, 4.0, 5.0))"), DVec3::new(3.0, 4.0, 5.0));
    assert_eq!(v3("vec3(vec4(3.0, 4.0, 5.0, 6.0))"), DVec3::new(3.0, 4.0, 5.0));
}

#[test]
fn throws_if_vec3_has_invalid_number_of_arguments() {
    eval_throws("vec3()");
    eval_throws("vec3(3.0, 4.0)");
    eval_throws("vec3(3.0, 4.0, 5.0, 6.0)");
    eval_throws("vec3(vec2(3.0, 4.0), vec2(5.0, 6.0))");
    eval_throws("vec3(vec4(3.0, 4.0, 5.0, 6.0), 1.0)");
}

#[test]
fn throws_if_vec3_has_invalid_argument() {
    eval_throws(r#"vec3(1.0, "1.0", 2.0)"#);
}

#[test]
fn evaluates_vec4() {
    assert_eq!(c4("vec4(2.0)"), DVec4::new(2.0, 2.0, 2.0, 2.0));
    assert_eq!(c4("vec4(3.0, 4.0, 5.0, 6.0)"), DVec4::new(3.0, 4.0, 5.0, 6.0));
    assert_eq!(c4("vec4(vec2(3.0, 4.0), 5.0, 6.0)"), DVec4::new(3.0, 4.0, 5.0, 6.0));
    assert_eq!(c4("vec4(3.0, vec2(4.0, 5.0), 6.0)"), DVec4::new(3.0, 4.0, 5.0, 6.0));
    assert_eq!(c4("vec4(3.0, 4.0, vec2(5.0, 6.0))"), DVec4::new(3.0, 4.0, 5.0, 6.0));
    assert_eq!(c4("vec4(vec3(3.0, 4.0, 5.0), 6.0)"), DVec4::new(3.0, 4.0, 5.0, 6.0));
    assert_eq!(c4("vec4(3.0, vec3(4.0, 5.0, 6.0))"), DVec4::new(3.0, 4.0, 5.0, 6.0));
    assert_eq!(c4("vec4(vec4(3.0, 4.0, 5.0, 6.0))"), DVec4::new(3.0, 4.0, 5.0, 6.0));
}

#[test]
fn throws_if_vec4_has_invalid_number_of_arguments() {
    eval_throws("vec4()");
    eval_throws("vec4(3.0, 4.0)");
    eval_throws("vec4(3.0, 4.0, 5.0)");
    eval_throws("vec4(3.0, 4.0, 5.0, 6.0, 7.0)");
    eval_throws("vec4(vec3(3.0, 4.0, 5.0))");
}

#[test]
fn throws_if_vec4_has_invalid_argument() {
    eval_throws(r#"vec4(1.0, "2.0", 3.0, 4.0)"#);
}

#[test]
fn evaluates_vector_with_expressions_as_arguments() {
    let f = MockFeature::new()
        .prop("height", Value::Number(2.0))
        .prop("width", Value::Number(4.0))
        .prop("depth", Value::Number(3.0))
        .prop("scale", Value::Number(1.0));
    assert_eq!(
        evf("vec4(${height}, ${width}, ${depth}, ${scale})", &f),
        Value::Cartesian4(DVec4::new(2.0, 4.0, 3.0, 1.0))
    );
}

#[test]
fn evaluates_expression_with_multiple_nested_vectors() {
    assert_eq!(
        c4("vec4(vec2(1, 2)[vec3(6, 1, 5).y], 2, vec4(1.0).w, 5)"),
        DVec4::new(2.0, 2.0, 1.0, 5.0)
    );
}

#[test]
fn evaluates_vector_properties_xyzw() {
    assert_eq!(n("vec4(1.0, 2.0, 3.0, 4.0).x"), 1.0);
    assert_eq!(n("vec4(1.0, 2.0, 3.0, 4.0).y"), 2.0);
    assert_eq!(n("vec4(1.0, 2.0, 3.0, 4.0).z"), 3.0);
    assert_eq!(n("vec4(1.0, 2.0, 3.0, 4.0).w"), 4.0);
}

#[test]
fn evaluates_vector_properties_rgba() {
    assert_eq!(n("vec4(1.0, 2.0, 3.0, 4.0).r"), 1.0);
    assert_eq!(n("vec4(1.0, 2.0, 3.0, 4.0).g"), 2.0);
    assert_eq!(n("vec4(1.0, 2.0, 3.0, 4.0).b"), 3.0);
    assert_eq!(n("vec4(1.0, 2.0, 3.0, 4.0).a"), 4.0);
}

#[test]
fn evaluates_vector_properties_numeric_index() {
    assert_eq!(n("vec4(1.0, 2.0, 3.0, 4.0)[0]"), 1.0);
    assert_eq!(n("vec4(1.0, 2.0, 3.0, 4.0)[1]"), 2.0);
    assert_eq!(n("vec4(1.0, 2.0, 3.0, 4.0)[2]"), 3.0);
    assert_eq!(n("vec4(1.0, 2.0, 3.0, 4.0)[3]"), 4.0);
}

#[test]
fn evaluates_vector_properties_string_xyzw_index() {
    assert_eq!(n(r#"vec4(1.0, 2.0, 3.0, 4.0)["x"]"#), 1.0);
    assert_eq!(n(r#"vec4(1.0, 2.0, 3.0, 4.0)["y"]"#), 2.0);
    assert_eq!(n(r#"vec4(1.0, 2.0, 3.0, 4.0)["z"]"#), 3.0);
    assert_eq!(n(r#"vec4(1.0, 2.0, 3.0, 4.0)["w"]"#), 4.0);
}

#[test]
fn evaluates_vector_properties_string_rgba_index() {
    assert_eq!(n(r#"vec4(1.0, 2.0, 3.0, 4.0)["r"]"#), 1.0);
    assert_eq!(n(r#"vec4(1.0, 2.0, 3.0, 4.0)["g"]"#), 2.0);
    assert_eq!(n(r#"vec4(1.0, 2.0, 3.0, 4.0)["b"]"#), 3.0);
    assert_eq!(n(r#"vec4(1.0, 2.0, 3.0, 4.0)["a"]"#), 4.0);
}

// ===========================================================================
// Unary operators
// ===========================================================================

#[test]
fn evaluates_unary_not() {
    assert!(!b("!true"));
    assert!(b("!!true"));
}

#[test]
fn throws_if_unary_not_takes_invalid_argument() {
    eval_throws(r#"!"true""#);
}

#[test]
fn evaluates_unary_negative() {
    assert_eq!(n("-5"), -5.0);
    assert_eq!(n("-(-5)"), 5.0);
}

#[test]
fn throws_if_unary_negative_takes_invalid_argument() {
    eval_throws(r#"-"56""#);
}

#[test]
fn evaluates_unary_positive() {
    assert_eq!(n("+5"), 5.0);
}

#[test]
fn throws_if_unary_positive_takes_invalid_argument() {
    eval_throws(r#"+ "56""#);
}

// ===========================================================================
// Binary arithmetic operators
// ===========================================================================

#[test]
fn evaluates_binary_addition() {
    assert_eq!(n("1 + 2"), 3.0);
    assert_eq!(n("1 + 2 + 3 + 4"), 10.0);
}

#[test]
fn evaluates_binary_addition_with_strings() {
    assert_eq!(s(r#"1 + "10""#), "110");
    assert_eq!(s(r#""10" + 1"#), "101");
    assert_eq!(s(r#""name_" + "building""#), "name_building");
    assert_eq!(s(r#""name_" + true"#), "name_true");
    assert_eq!(s(r#""name_" + null"#), "name_null");
    assert_eq!(s(r#""name_" + undefined"#), "name_undefined");
    assert_eq!(s(r#""name_" + vec2(1.1)"#), "name_(1.1, 1.1)");
    assert_eq!(s(r#""name_" + vec3(1.1)"#), "name_(1.1, 1.1, 1.1)");
    assert_eq!(s(r#""name_" + vec4(1.1)"#), "name_(1.1, 1.1, 1.1, 1.1)");
    assert_eq!(s(r#""name_" + regExp("a")"#), "name_/a/");
}

#[test]
fn throws_if_binary_addition_takes_invalid_arguments() {
    eval_throws("vec2(1.0) + vec3(1.0)");
    eval_throws("1.0 + vec3(1.0)");
}

#[test]
fn evaluates_binary_subtraction() {
    assert_eq!(n("2 - 1"), 1.0);
    assert_eq!(n("4 - 3 - 2 - 1"), -2.0);
}

#[test]
fn throws_if_binary_subtraction_takes_invalid_arguments() {
    eval_throws("vec2(1.0) - vec3(1.0)");
    eval_throws("1.0 - vec3(1.0)");
    eval_throws(r#""name1" - "name2""#);
}

#[test]
fn evaluates_binary_multiplication() {
    assert_eq!(n("1 * 2"), 2.0);
    assert_eq!(n("1 * 2 * 3 * 4"), 24.0);
}

#[test]
fn throws_if_binary_multiplication_takes_invalid_arguments() {
    eval_throws("vec2(1.0) * vec3(1.0)");
    eval_throws(r#"vec2(1.0) * "name""#);
}

#[test]
fn evaluates_binary_division() {
    assert_eq!(n("2 / 1"), 2.0);
    assert_eq!(n("1/2"), 0.5);
    assert_eq!(n("24 / -4 / 2"), -3.0);
}

#[test]
fn throws_if_binary_division_takes_invalid_arguments() {
    eval_throws("vec2(1.0) / vec3(1.0)");
    eval_throws(r#"vec2(1.0) / "2.0""#);
    eval_throws("1.0 / vec4(1.0)");
}

#[test]
fn evaluates_binary_modulus() {
    assert_eq!(n("2 % 1"), 0.0);
    assert_eq!(n("6 % 4 % 3"), 2.0);
}

#[test]
fn throws_if_binary_modulus_takes_invalid_arguments() {
    eval_throws("vec2(1.0) % vec3(1.0)");
    eval_throws(r#"vec2(1.0) % "2.0""#);
    eval_throws("1.0 % vec4(1.0)");
}

// ===========================================================================
// Comparison operators
// ===========================================================================

#[test]
fn evaluates_binary_equals_strict() {
    assert!(b("'hello' === 'hello'"));
    assert!(!b("1 === 2"));
    assert!(b("false === true === false"));
    assert!(!b(r#"1 === "1""#));
}

#[test]
fn evaluates_binary_not_equals_strict() {
    assert!(!b("'hello' !== 'hello'"));
    assert!(b("1 !== 2"));
    assert!(b("false !== true !== false"));
    assert!(b(r#"1 !== "1""#));
}

#[test]
fn evaluates_binary_less_than() {
    assert!(b("2 < 3"));
    assert!(!b("2 < 2"));
    assert!(!b("3 < 2"));
}

#[test]
fn throws_if_binary_less_than_takes_invalid_arguments() {
    eval_throws("vec2(1.0) < vec2(2.0)");
    eval_throws("1 < vec3(1.0)");
    eval_throws("true < false");
    eval_throws("color('blue') < 10");
}

#[test]
fn evaluates_binary_less_than_or_equals() {
    assert!(b("2 <= 3"));
    assert!(b("2 <= 2"));
    assert!(!b("3 <= 2"));
}

#[test]
fn throws_if_binary_less_than_or_equals_takes_invalid_arguments() {
    eval_throws("vec2(1.0) <= vec2(2.0)");
    eval_throws("1 <= vec3(1.0)");
    eval_throws(r#"1.0 <= "5""#);
    eval_throws("true <= false");
    eval_throws("color('blue') <= 10");
}

#[test]
fn evaluates_binary_greater_than() {
    assert!(!b("2 > 3"));
    assert!(!b("2 > 2"));
    assert!(b("3 > 2"));
}

#[test]
fn throws_if_binary_greater_than_takes_invalid_arguments() {
    eval_throws("vec2(1.0) > vec2(2.0)");
    eval_throws("1 > vec3(1.0)");
    eval_throws(r#"1.0 > "5""#);
    eval_throws("true > false");
    eval_throws("color('blue') > 10");
}

#[test]
fn evaluates_binary_greater_than_or_equals() {
    assert!(!b("2 >= 3"));
    assert!(b("2 >= 2"));
    assert!(b("3 >= 2"));
}

#[test]
fn throws_if_binary_greater_than_or_equals_takes_invalid_arguments() {
    eval_throws("vec2(1.0) >= vec2(2.0)");
    eval_throws("1 >= vec3(1.0)");
    eval_throws(r#"1.0 >= "5""#);
    eval_throws("true >= false");
    eval_throws("color('blue') >= 10");
}

// ===========================================================================
// Logical operators
// ===========================================================================

#[test]
fn evaluates_logical_and() {
    assert!(!b("false && false"));
    assert!(!b("false && true"));
    assert!(b("true && true"));
    eval_throws("2 && color('red')");
}

#[test]
fn throws_with_invalid_and_operands() {
    eval_throws("2 && true");
    eval_throws("true && color('red')");
}

#[test]
fn evaluates_logical_or() {
    assert!(!b("false || false"));
    assert!(b("false || true"));
    assert!(b("true || true"));
}

#[test]
fn throws_with_invalid_or_operands() {
    eval_throws("2 || false");
    eval_throws("false || color('red')");
}

// ===========================================================================
// Color / vector arithmetic operations
// ===========================================================================

#[test]
fn evaluates_color_operations_add_sub() {
    close4(c4("+rgba(255, 0, 0, 1.0)"), [1.0, 0.0, 0.0, 1.0], EPSILON7);
    close4(
        c4("rgba(255, 0, 0, 0.5) + rgba(0, 0, 255, 0.5)"),
        [1.0, 0.0, 1.0, 1.0],
        EPSILON7,
    );
    close4(
        c4("rgba(0, 255, 255, 1.0) - rgba(0, 255, 0, 0)"),
        [0.0, 0.0, 1.0, 1.0],
        EPSILON7,
    );
}

#[test]
fn evaluates_color_operations_mul_div_mod() {
    close4(
        c4("rgba(255, 255, 255, 1.0) * rgba(255, 0, 0, 1.0)"),
        [1.0, 0.0, 0.0, 1.0],
        EPSILON7,
    );
    close4(c4("rgba(255, 255, 0, 1.0) * 1.0"), [1.0, 1.0, 0.0, 1.0], EPSILON7);
    close4(c4("1 * rgba(255, 255, 0, 1.0)"), [1.0, 1.0, 0.0, 1.0], EPSILON7);
    close4(
        c4("rgba(255, 255, 255, 1.0) / rgba(255, 255, 255, 1.0)"),
        [1.0, 1.0, 1.0, 1.0],
        EPSILON7,
    );
    close4(
        c4("rgba(255, 255, 255, 1.0) / 2"),
        [0.5, 0.5, 0.5, 0.5],
        EPSILON7,
    );
    close4(
        c4("rgba(255, 255, 255, 1.0) % rgba(255, 255, 255, 1.0)"),
        [0.0, 0.0, 0.0, 0.0],
        EPSILON7,
    );
}

#[test]
fn evaluates_color_equality() {
    assert!(b("color('green') === color('green')"));
    assert!(!b("color('green') !== color('green')"));
}

#[test]
fn evaluates_vector_unary_operations() {
    assert_eq!(v2("+vec2(1, 2)"), DVec2::new(1.0, 2.0));
    assert_eq!(v3("+vec3(1, 2, 3)"), DVec3::new(1.0, 2.0, 3.0));
    assert_eq!(c4("+vec4(1, 2, 3, 4)"), DVec4::new(1.0, 2.0, 3.0, 4.0));
    assert_eq!(v2("-vec2(1, 2)"), DVec2::new(-1.0, -2.0));
    assert_eq!(v3("-vec3(1, 2, 3)"), DVec3::new(-1.0, -2.0, -3.0));
    assert_eq!(c4("-vec4(1, 2, 3, 4)"), DVec4::new(-1.0, -2.0, -3.0, -4.0));
}

#[test]
fn evaluates_vector_addition() {
    assert_eq!(v2("vec2(1, 2) + vec2(3, 4)"), DVec2::new(4.0, 6.0));
    assert_eq!(v3("vec3(1, 2, 3) + vec3(3, 4, 5)"), DVec3::new(4.0, 6.0, 8.0));
    assert_eq!(
        c4("vec4(1, 2, 3, 4) + vec4(3, 4, 5, 6)"),
        DVec4::new(4.0, 6.0, 8.0, 10.0)
    );
}

#[test]
fn evaluates_vector_subtraction() {
    assert_eq!(v2("vec2(1, 2) - vec2(3, 4)"), DVec2::new(-2.0, -2.0));
    assert_eq!(
        v3("vec3(1, 2, 3) - vec3(3, 4, 5)"),
        DVec3::new(-2.0, -2.0, -2.0)
    );
    assert_eq!(
        c4("vec4(1, 2, 3, 4) - vec4(3, 4, 5, 6)"),
        DVec4::new(-2.0, -2.0, -2.0, -2.0)
    );
}

#[test]
fn evaluates_vector_multiplication() {
    assert_eq!(v2("vec2(1, 2) * vec2(3, 4)"), DVec2::new(3.0, 8.0));
    assert_eq!(v2("vec2(1, 2) * 3.0"), DVec2::new(3.0, 6.0));
    assert_eq!(v2("3.0 * vec2(1, 2)"), DVec2::new(3.0, 6.0));
    assert_eq!(v3("vec3(1, 2, 3) * vec3(3, 4, 5)"), DVec3::new(3.0, 8.0, 15.0));
    assert_eq!(v3("vec3(1, 2, 3) * 3.0"), DVec3::new(3.0, 6.0, 9.0));
    assert_eq!(v3("3.0 * vec3(1, 2, 3)"), DVec3::new(3.0, 6.0, 9.0));
    assert_eq!(
        c4("vec4(1, 2, 3, 4) * vec4(3, 4, 5, 6)"),
        DVec4::new(3.0, 8.0, 15.0, 24.0)
    );
    assert_eq!(c4("vec4(1, 2, 3, 4) * 3.0"), DVec4::new(3.0, 6.0, 9.0, 12.0));
    assert_eq!(c4("3.0 * vec4(1, 2, 3, 4)"), DVec4::new(3.0, 6.0, 9.0, 12.0));
}

#[test]
fn evaluates_vector_division() {
    close2(v2("vec2(1, 2) / vec2(2, 5)"), [0.5, 0.4], EPSILON7);
    close2(v2("vec2(1, 2) / 2.0"), [0.5, 1.0], EPSILON7);
    close3(v3("vec3(1, 2, 3) / vec3(2, 5, 3)"), [0.5, 0.4, 1.0], EPSILON7);
    close3(v3("vec3(1, 2, 3) / 2.0"), [0.5, 1.0, 1.5], EPSILON7);
    close4(
        c4("vec4(1, 2, 3, 4) / vec4(2, 5, 3, 2)"),
        [0.5, 0.4, 1.0, 2.0],
        EPSILON7,
    );
    close4(c4("vec4(1, 2, 3, 4) / 2.0"), [0.5, 1.0, 1.5, 2.0], EPSILON7);
}

#[test]
fn evaluates_vector_modulus() {
    assert_eq!(v2("vec2(2, 3) % vec2(3, 3)"), DVec2::new(2.0, 0.0));
    assert_eq!(
        v3("vec3(2, 3, 4) % vec3(3, 3, 3)"),
        DVec3::new(2.0, 0.0, 1.0)
    );
    assert_eq!(
        c4("vec4(2, 3, 4, 5) % vec4(3, 3, 3, 2)"),
        DVec4::new(2.0, 0.0, 1.0, 1.0)
    );
}

#[test]
fn evaluates_vector_equality() {
    assert!(b("vec2(1, 2) === vec2(1, 2)"));
    assert!(b("vec3(1, 2, 3) === vec3(1, 2, 3)"));
    assert!(b("vec4(1, 2, 3, 4) === vec4(1, 2, 3, 4)"));
    assert!(!b("vec2(1, 2) !== vec2(1, 2)"));
    assert!(!b("vec3(1, 2, 3) !== vec3(1, 2, 3)"));
    assert!(!b("vec4(1, 2, 3, 4) !== vec4(1, 2, 3, 4)"));
}

#[test]
fn evaluates_color_to_string_function() {
    assert_eq!(s(r#"color("red").toString()"#), "(1, 0, 0, 1)");
    assert_eq!(s("rgba(0, 0, 255, 0.5).toString()"), "(0, 0, 1, 0.5)");
}

#[test]
fn evaluates_vector_to_string_function() {
    let f = MockFeature::new().prop("property", Value::Cartesian4(DVec4::new(1.0, 2.0, 3.0, 4.0)));
    assert_eq!(s("vec2(1, 2).toString()"), "(1, 2)");
    assert_eq!(s("vec3(1, 2, 3).toString()"), "(1, 2, 3)");
    assert_eq!(s("vec4(1, 2, 3, 4).toString()"), "(1, 2, 3, 4)");
    assert_eq!(sf("${property}.toString()", &f), "(1, 2, 3, 4)");
}

// ===========================================================================
// isNaN / isFinite  (upstream L1607-1657)
// ===========================================================================

#[test]
fn evaluates_is_nan_function() {
    // isNaN() folds to `true` at parse time (0 args).
    assert!(b("isNaN()"));
    assert!(b("isNaN(NaN)"));
    assert!(!b("isNaN(1)"));
    assert!(!b("isNaN(Infinity)"));
    assert!(!b("isNaN(null)"));
    assert!(!b("isNaN(true)"));
    assert!(b(r#"isNaN("hello")"#));
    assert!(b(r#"isNaN(color("white"))"#));
}

#[test]
fn evaluates_is_finite_function() {
    // isFinite() folds to `false` at parse time (0 args).
    assert!(!b("isFinite()"));
    assert!(!b("isFinite(NaN)"));
    assert!(b("isFinite(1)"));
    assert!(!b("isFinite(Infinity)"));
    assert!(b("isFinite(null)"));
    assert!(b("isFinite(true)"));
    assert!(!b(r#"isFinite("hello")"#));
    assert!(!b(r#"isFinite(color("white"))"#));
}

// ===========================================================================
// Class functions  (upstream L1659-1716)
// ===========================================================================

#[test]
fn evaluates_is_exact_class_function() {
    let f = MockFeature::new().set_class("door");
    assert!(bf(r#"isExactClass("door")"#, &f));
    assert!(!bf(r#"isExactClass("roof")"#, &f));
    // no feature -> false
    assert!(!b(r#"isExactClass("roof")"#));
}

#[test]
fn throws_if_is_exact_class_takes_invalid_number_of_arguments() {
    parse_throws("isExactClass()");
    parse_throws(r#"isExactClass("door", "roof")"#);
}

#[test]
fn evaluates_is_class_function() {
    let f = MockFeature::new()
        .set_class("door")
        .set_inherited_class("building");
    assert!(bf(r#"isClass("door") && isClass("building")"#, &f));
    // no feature -> false
    assert!(!b(r#"isClass("door") && isClass("building")"#));
}

#[test]
fn throws_if_is_class_takes_invalid_number_of_arguments() {
    parse_throws("isClass()");
    parse_throws(r#"isClass("door", "building")"#);
}

#[test]
fn evaluates_get_exact_class_name_function() {
    let f = MockFeature::new().set_class("door");
    assert_eq!(sf("getExactClassName()", &f), "door");
    // no feature -> undefined
    assert_eq!(ev("getExactClassName()"), Value::Undefined);
}

#[test]
fn throws_if_get_exact_class_name_takes_invalid_number_of_arguments() {
    parse_throws(r#"getExactClassName("door")"#);
}

#[test]
fn throws_if_builtin_unary_function_is_given_an_invalid_argument() {
    // Argument must be a number or vector.
    eval_throws(r#"abs("-1")"#);
}

// ===========================================================================
// Unary math functions  (upstream L1726-2452)
// ===========================================================================

const PI: f64 = std::f64::consts::PI;
const PI_OVER_TWO: f64 = std::f64::consts::FRAC_PI_2;
const PI_OVER_FOUR: f64 = std::f64::consts::FRAC_PI_4;

#[test]
fn evaluates_abs_function() {
    assert_eq!(n("abs(-1)"), 1.0);
    assert_eq!(n("abs(1)"), 1.0);
    assert_eq!(v2("abs(vec2(-1.0, 1.0))"), DVec2::new(1.0, 1.0));
    assert_eq!(v3("abs(vec3(-1.0, 1.0, 0.0))"), DVec3::new(1.0, 1.0, 0.0));
    assert_eq!(
        c4("abs(vec4(-1.0, 1.0, 0.0, -1.2))"),
        DVec4::new(1.0, 1.0, 0.0, 1.2)
    );
}

#[test]
fn throws_if_abs_function_takes_invalid_number_of_arguments() {
    parse_throws("abs()");
    parse_throws("abs(1, 2)");
}

#[test]
fn evaluates_cos_function() {
    close(n("cos(0)"), 1.0, EPSILON7);
    close2(v2("cos(vec2(0, Math.PI))"), [1.0, -1.0], EPSILON7);
    close3(v3("cos(vec3(0, Math.PI, -Math.PI))"), [1.0, -1.0, -1.0], EPSILON7);
    close4(
        c4("cos(vec4(0, Math.PI, -Math.PI, 0))"),
        [1.0, -1.0, -1.0, 1.0],
        EPSILON7,
    );
}

#[test]
fn throws_if_cos_function_takes_invalid_number_of_arguments() {
    parse_throws("cos()");
    parse_throws("cos(1, 2)");
}

#[test]
fn evaluates_sin_function() {
    close(n("sin(0)"), 0.0, EPSILON7);
    close2(v2("sin(vec2(0, Math.PI/2))"), [0.0, 1.0], EPSILON7);
    close3(v3("sin(vec3(0, Math.PI/2, -Math.PI/2))"), [0.0, 1.0, -1.0], EPSILON7);
    close4(
        c4("sin(vec4(0, Math.PI/2, -Math.PI/2, 0))"),
        [0.0, 1.0, -1.0, 0.0],
        EPSILON7,
    );
}

#[test]
fn throws_if_sin_function_takes_invalid_number_of_arguments() {
    parse_throws("sin()");
    parse_throws("sin(1, 2)");
}

#[test]
fn evaluates_tan_function() {
    close(n("tan(0)"), 0.0, EPSILON7);
    close2(v2("tan(vec2(0, Math.PI/4))"), [0.0, 1.0], EPSILON7);
    close3(v3("tan(vec3(0, Math.PI/4, Math.PI))"), [0.0, 1.0, 0.0], EPSILON7);
    close4(
        c4("tan(vec4(0, Math.PI/4, Math.PI, -Math.PI/4))"),
        [0.0, 1.0, 0.0, -1.0],
        EPSILON7,
    );
}

#[test]
fn throws_if_tan_function_takes_invalid_number_of_arguments() {
    parse_throws("tan()");
    parse_throws("tan(1, 2)");
}

#[test]
fn evaluates_acos_function() {
    close(n("acos(1)"), 0.0, EPSILON7);
    close2(v2("acos(vec2(1, 0))"), [0.0, PI_OVER_TWO], EPSILON7);
    close3(v3("acos(vec3(1, 0, 1))"), [0.0, PI_OVER_TWO, 0.0], EPSILON7);
    close4(
        c4("acos(vec4(1, 0, 1, 0))"),
        [0.0, PI_OVER_TWO, 0.0, PI_OVER_TWO],
        EPSILON7,
    );
}

#[test]
fn throws_if_acos_function_takes_invalid_number_of_arguments() {
    parse_throws("acos()");
    parse_throws("acos(1, 2)");
}

#[test]
fn evaluates_asin_function() {
    close(n("asin(0)"), 0.0, EPSILON7);
    close2(v2("asin(vec2(0, 1))"), [0.0, PI_OVER_TWO], EPSILON7);
    close3(v3("asin(vec3(0, 1, 0))"), [0.0, PI_OVER_TWO, 0.0], EPSILON7);
    close4(
        c4("asin(vec4(0, 1, 0, 1))"),
        [0.0, PI_OVER_TWO, 0.0, PI_OVER_TWO],
        EPSILON7,
    );
}

#[test]
fn throws_if_asin_function_takes_invalid_number_of_arguments() {
    parse_throws("asin()");
    parse_throws("asin(1, 2)");
}

#[test]
fn evaluates_atan_function() {
    close(n("atan(0)"), 0.0, EPSILON7);
    close2(v2("atan(vec2(0, 1))"), [0.0, PI_OVER_FOUR], EPSILON7);
    close3(v3("atan(vec3(0, 1, 0))"), [0.0, PI_OVER_FOUR, 0.0], EPSILON7);
    close4(
        c4("atan(vec4(0, 1, 0, 1))"),
        [0.0, PI_OVER_FOUR, 0.0, PI_OVER_FOUR],
        EPSILON7,
    );
}

#[test]
fn throws_if_atan_function_takes_invalid_number_of_arguments() {
    parse_throws("atan()");
    parse_throws("atan(1, 2)");
}

#[test]
fn evaluates_radians_function() {
    close(n("radians(180)"), PI, EPSILON10);
    close2(v2("radians(vec2(180, 90))"), [PI, PI_OVER_TWO], EPSILON7);
    close3(v3("radians(vec3(180, 90, 180))"), [PI, PI_OVER_TWO, PI], EPSILON7);
    close4(
        c4("radians(vec4(180, 90, 180, 90))"),
        [PI, PI_OVER_TWO, PI, PI_OVER_TWO],
        EPSILON7,
    );
}

#[test]
fn throws_if_radians_function_takes_invalid_number_of_arguments() {
    parse_throws("radians()");
    parse_throws("radians(1, 2)");
}

#[test]
fn evaluates_degrees_function() {
    close(n("degrees(2 * Math.PI)"), 360.0, EPSILON10);
    close2(v2("degrees(vec2(2 * Math.PI, Math.PI))"), [360.0, 180.0], EPSILON7);
    close3(
        v3("degrees(vec3(2 * Math.PI, Math.PI, 2 * Math.PI))"),
        [360.0, 180.0, 360.0],
        EPSILON7,
    );
    close4(
        c4("degrees(vec4(2 * Math.PI, Math.PI, 2 * Math.PI, Math.PI))"),
        [360.0, 180.0, 360.0, 180.0],
        EPSILON7,
    );
}

#[test]
fn throws_if_degrees_function_takes_invalid_number_of_arguments() {
    parse_throws("degrees()");
    parse_throws("degrees(1, 2)");
}

#[test]
fn evaluates_sqrt_function() {
    assert_eq!(n("sqrt(1.0)"), 1.0);
    assert_eq!(n("sqrt(4.0)"), 2.0);
    assert!(n("sqrt(-1.0)").is_nan());
    assert_eq!(v2("sqrt(vec2(1.0, 4.0))"), DVec2::new(1.0, 2.0));
    assert_eq!(v3("sqrt(vec3(1.0, 4.0, 9.0))"), DVec3::new(1.0, 2.0, 3.0));
    assert_eq!(
        c4("sqrt(vec4(1.0, 4.0, 9.0, 16.0))"),
        DVec4::new(1.0, 2.0, 3.0, 4.0)
    );
}

#[test]
fn throws_if_sqrt_function_takes_invalid_number_of_arguments() {
    parse_throws("sqrt()");
    parse_throws("sqrt(1, 2)");
}

#[test]
fn evaluates_sign_function() {
    assert_eq!(n("sign(5.0)"), 1.0);
    assert_eq!(n("sign(0.0)"), 0.0);
    assert_eq!(n("sign(-5.0)"), -1.0);
    assert_eq!(v2("sign(vec2(5.0, -5.0))"), DVec2::new(1.0, -1.0));
    assert_eq!(v3("sign(vec3(5.0, -5.0, 0.0))"), DVec3::new(1.0, -1.0, 0.0));
    assert_eq!(
        c4("sign(vec4(5.0, -5.0, 0.0, 1.0))"),
        DVec4::new(1.0, -1.0, 0.0, 1.0)
    );
}

#[test]
fn throws_if_sign_function_takes_invalid_number_of_arguments() {
    parse_throws("sign()");
    parse_throws("sign(1, 2)");
}

#[test]
fn evaluates_floor_function() {
    assert_eq!(n("floor(5.5)"), 5.0);
    assert_eq!(n("floor(0.0)"), 0.0);
    assert_eq!(n("floor(-1.2)"), -2.0);
    assert_eq!(v2("floor(vec2(5.5, -1.2))"), DVec2::new(5.0, -2.0));
    assert_eq!(
        v3("floor(vec3(5.5, -1.2, 0.0))"),
        DVec3::new(5.0, -2.0, 0.0)
    );
    assert_eq!(
        c4("floor(vec4(5.5, -1.2, 0.0, -2.9))"),
        DVec4::new(5.0, -2.0, 0.0, -3.0)
    );
}

#[test]
fn throws_if_floor_function_takes_invalid_number_of_arguments() {
    parse_throws("floor()");
    parse_throws("floor(1, 2)");
}

#[test]
fn evaluates_ceil_function() {
    assert_eq!(n("ceil(5.5)"), 6.0);
    assert_eq!(n("ceil(0.0)"), 0.0);
    assert_eq!(n("ceil(-1.2)"), -1.0);
    assert_eq!(v2("ceil(vec2(5.5, -1.2))"), DVec2::new(6.0, -1.0));
    assert_eq!(v3("ceil(vec3(5.5, -1.2, 0.0))"), DVec3::new(6.0, -1.0, 0.0));
    assert_eq!(
        c4("ceil(vec4(5.5, -1.2, 0.0, -2.9))"),
        DVec4::new(6.0, -1.0, 0.0, -2.0)
    );
}

#[test]
fn throws_if_ceil_function_takes_invalid_number_of_arguments() {
    parse_throws("ceil()");
    parse_throws("ceil(1, 2)");
}

#[test]
fn evaluates_round_function() {
    // JS Math.round: half towards +infinity (round(5.5)==6, round(-1.2)==-1).
    assert_eq!(n("round(5.5)"), 6.0);
    assert_eq!(n("round(0.0)"), 0.0);
    assert_eq!(n("round(1.2)"), 1.0);
    assert_eq!(v2("round(vec2(5.5, -1.2))"), DVec2::new(6.0, -1.0));
    assert_eq!(v3("round(vec3(5.5, -1.2, 0.0))"), DVec3::new(6.0, -1.0, 0.0));
    assert_eq!(
        c4("round(vec4(5.5, -1.2, 0.0, -2.9))"),
        DVec4::new(6.0, -1.0, 0.0, -3.0)
    );
}

#[test]
fn throws_if_round_function_takes_invalid_number_of_arguments() {
    parse_throws("round()");
    parse_throws("round(1, 2)");
}

#[test]
fn evaluates_exp_function() {
    close(n("exp(1.0)"), std::f64::consts::E, EPSILON10);
    close(n("exp(0.0)"), 1.0, EPSILON10);
    let v = v2("exp(vec2(1.0, 0.0))");
    close(v.x, std::f64::consts::E, EPSILON10);
    close(v.y, 1.0, EPSILON10);
    let v = v3("exp(vec3(1.0, 0.0, 1.0))");
    close(v.x, std::f64::consts::E, EPSILON10);
    close(v.y, 1.0, EPSILON10);
    close(v.z, std::f64::consts::E, EPSILON10);
}

#[test]
fn throws_if_exp_function_takes_invalid_number_of_arguments() {
    parse_throws("exp()");
    parse_throws("exp(1, 2)");
}

#[test]
fn evaluates_exp2_function() {
    assert_eq!(n("exp2(1.0)"), 2.0);
    assert_eq!(n("exp2(0.0)"), 1.0);
    assert_eq!(n("exp2(2.0)"), 4.0);
    assert_eq!(v2("exp2(vec2(1.0, 0.0))"), DVec2::new(2.0, 1.0));
    assert_eq!(v3("exp2(vec3(1.0, 0.0, 2.0))"), DVec3::new(2.0, 1.0, 4.0));
    assert_eq!(
        c4("exp2(vec4(1.0, 0.0, 2.0, 3.0))"),
        DVec4::new(2.0, 1.0, 4.0, 8.0)
    );
}

#[test]
fn throws_if_exp2_function_takes_invalid_number_of_arguments() {
    parse_throws("exp2()");
    parse_throws("exp2(1, 2)");
}

#[test]
fn evaluates_log_function() {
    close(n("log(1.0)"), 0.0, EPSILON7);
    close(n("log(10.0)"), std::f64::consts::LN_10, EPSILON7);
    let v = v2("log(vec2(1.0, Math.E))");
    close(v.x, 0.0, EPSILON7);
    close(v.y, 1.0, EPSILON7);
    let v = v3("log(vec3(1.0, Math.E, 1.0))");
    close(v.x, 0.0, EPSILON7);
    close(v.y, 1.0, EPSILON7);
    close(v.z, 0.0, EPSILON7);
}

#[test]
fn throws_if_log_function_takes_invalid_number_of_arguments() {
    parse_throws("log()");
    parse_throws("log(1, 2)");
}

#[test]
fn evaluates_log2_function() {
    assert_eq!(n("log2(1.0)"), 0.0);
    assert_eq!(n("log2(2.0)"), 1.0);
    assert_eq!(n("log2(4.0)"), 2.0);
    assert_eq!(v2("log2(vec2(1.0, 2.0))"), DVec2::new(0.0, 1.0));
    assert_eq!(v3("log2(vec3(1.0, 2.0, 4.0))"), DVec3::new(0.0, 1.0, 2.0));
    close4(c4("log2(vec4(1.0, 2.0, 4.0, 8.0))"), [0.0, 1.0, 2.0, 3.0], EPSILON10);
}

#[test]
fn throws_if_log2_function_takes_invalid_number_of_arguments() {
    parse_throws("log2()");
    parse_throws("log2(1, 2)");
}

#[test]
fn evaluates_fract_function() {
    close(n("fract(1.0)"), 0.0, EPSILON7);
    close(n("fract(2.25)"), 0.25, EPSILON7);
    close(n("fract(-2.25)"), 0.75, EPSILON7);
    close2(v2("fract(vec2(1.0, 2.25))"), [0.0, 0.25], EPSILON7);
    close3(v3("fract(vec3(1.0, 2.25, -2.25))"), [0.0, 0.25, 0.75], EPSILON7);
    close4(
        c4("fract(vec4(1.0, 2.25, -2.25, 1.0))"),
        [0.0, 0.25, 0.75, 0.0],
        EPSILON7,
    );
}

#[test]
fn throws_if_fract_function_takes_invalid_number_of_arguments() {
    parse_throws("fract()");
    parse_throws("fract(1, 2)");
}

#[test]
fn evaluates_length_function() {
    assert_eq!(n("length(-3.0)"), 3.0);
    assert_eq!(n("length(vec2(-3.0, 4.0))"), 5.0);
    assert_eq!(n("length(vec3(2.0, 3.0, 6.0))"), 7.0);
    assert_eq!(n("length(vec4(2.0, 4.0, 7.0, 10.0))"), 13.0);
}

#[test]
fn throws_if_length_function_takes_invalid_number_of_arguments() {
    parse_throws("length()");
    parse_throws("length(1, 2)");
}

#[test]
fn evaluates_normalize_function() {
    assert_eq!(n("normalize(5.0)"), 1.0);
    close2(v2("normalize(vec2(3.0, 4.0))"), [0.6, 0.8], EPSILON10);
    let len = (2.0_f64 * 2.0 + 3.0 * 3.0 + 4.0 * 4.0).sqrt();
    close3(
        v3("normalize(vec3(2.0, 3.0, -4.0))"),
        [2.0 / len, 3.0 / len, -4.0 / len],
        EPSILON10,
    );
    let len4 = (2.0_f64 * 2.0 + 3.0 * 3.0 + 4.0 * 4.0 + 5.0 * 5.0).sqrt();
    close4(
        c4("normalize(vec4(-2.0, 3.0, -4.0, 5.0))"),
        [-2.0 / len4, 3.0 / len4, -4.0 / len4, 5.0 / len4],
        EPSILON10,
    );
}

#[test]
fn throws_if_normalize_function_takes_invalid_number_of_arguments() {
    parse_throws("normalize()");
    parse_throws("normalize(1, 2)");
}

// ===========================================================================
// Ternary math functions: clamp / mix  (upstream L2464-2594)
// ===========================================================================

#[test]
fn evaluates_clamp_function() {
    assert_eq!(n("clamp(50.0, 0.0, 100.0)"), 50.0);
    assert_eq!(n("clamp(50.0, 0.0, 25.0)"), 25.0);
    assert_eq!(n("clamp(50.0, 75.0, 100.0)"), 75.0);
    assert_eq!(
        v2("clamp(vec2(50.0,50.0), vec2(0.0,75.0), 100.0)"),
        DVec2::new(50.0, 75.0)
    );
    assert_eq!(
        v2("clamp(vec2(50.0,50.0), vec2(0.0,75.0), vec2(25.0,100.0))"),
        DVec2::new(25.0, 75.0)
    );
    assert_eq!(
        v3("clamp(vec3(50.0, 50.0, 50.0), vec3(0.0, 0.0, 75.0), vec3(100.0, 25.0, 100.0))"),
        DVec3::new(50.0, 25.0, 75.0)
    );
    assert_eq!(
        c4("clamp(vec4(50.0, 50.0, 50.0, 100.0), vec4(0.0, 0.0, 75.0, 75.0), vec4(100.0, 25.0, 100.0, 85.0))"),
        DVec4::new(50.0, 25.0, 75.0, 85.0)
    );
}

#[test]
fn throws_if_clamp_function_takes_invalid_number_of_arguments() {
    parse_throws("clamp()");
    parse_throws("clamp(1)");
    parse_throws("clamp(1, 2)");
    parse_throws("clamp(1, 2, 3, 4)");
}

#[test]
fn throws_if_clamp_function_takes_mismatching_types() {
    eval_throws("clamp(0.0,vec2(0,1),0.0)");
    eval_throws("clamp(vec2(0,1),vec3(0,1,2),0.0)");
    eval_throws("clamp(vec2(0,1),vec2(0,1), vec3(1,2,3))");
}

#[test]
fn evaluates_mix_function() {
    assert_eq!(n("mix(0.0, 2.0, 0.5)"), 1.0);
    assert_eq!(
        v2("mix(vec2(0.0,1.0), vec2(2.0,3.0), 0.5)"),
        DVec2::new(1.0, 2.0)
    );
    assert_eq!(
        v2("mix(vec2(0.0,1.0), vec2(2.0,3.0), vec2(0.5,4.0))"),
        DVec2::new(1.0, 9.0)
    );
    assert_eq!(
        v3("mix(vec3(0.0,1.0,2.0), vec3(2.0,3.0,4.0), vec3(0.5,4.0,5.0))"),
        DVec3::new(1.0, 9.0, 12.0)
    );
    assert_eq!(
        c4("mix(vec4(0.0,1.0,2.0,1.5), vec4(2.0,3.0,4.0,2.5), vec4(0.5,4.0,5.0,3.5))"),
        DVec4::new(1.0, 9.0, 12.0, 5.0)
    );
}

#[test]
fn throws_if_mix_function_takes_mismatching_types() {
    eval_throws("mix(0.0,vec2(0,1),0.0)");
    eval_throws("mix(vec2(0,1),vec3(0,1,2),0.0)");
    eval_throws("mix(vec2(0,1),vec2(0,1), vec3(1,2,3))");
}

#[test]
fn throws_if_mix_function_takes_invalid_number_of_arguments() {
    parse_throws("mix()");
    parse_throws("mix(1)");
    parse_throws("mix(1, 2)");
    parse_throws("mix(1, 2, 3, 4)");
}

// ===========================================================================
// Binary math functions: atan2 / distance / dot / cross  (upstream L2596-2920)
// ===========================================================================

#[test]
fn evaluates_atan2_function() {
    close(n("atan2(0,1)"), 0.0, EPSILON10);
    close(n("atan2(1,0)"), PI_OVER_TWO, EPSILON10);
    close2(v2("atan2(vec2(0,1),vec2(1,0))"), [0.0, PI_OVER_TWO], EPSILON10);
    close3(
        v3("atan2(vec3(0,1,0.5),vec3(1,0,0.5))"),
        [0.0, PI_OVER_TWO, PI_OVER_FOUR],
        EPSILON10,
    );
    close4(
        c4("atan2(vec4(0,1,0.5,1),vec4(1,0,0.5,0))"),
        [0.0, PI_OVER_TWO, PI_OVER_FOUR, PI_OVER_TWO],
        EPSILON10,
    );
}

#[test]
fn throws_if_atan2_function_takes_invalid_number_of_arguments() {
    parse_throws("atan2(0.0)");
    parse_throws("atan2(1, 2, 0)");
}

#[test]
fn throws_if_atan2_function_takes_mismatching_types() {
    eval_throws("atan2(0.0,vec2(0,1))");
    eval_throws("atan2(vec2(0,1),0.0)");
    eval_throws("atan2(vec2(0,1),vec3(0,1,2))");
}

#[test]
fn evaluates_distance_function() {
    assert_eq!(n("distance(0, 1)"), 1.0);
    assert_eq!(n("distance(vec2(1.0, 0.0), vec2(0.0, 0.0))"), 1.0);
    assert_eq!(n("distance(vec3(3.0, 2.0, 1.0), vec3(1.0, 0.0, 0.0))"), 3.0);
    assert_eq!(
        n("distance(vec4(5.0, 5.0, 5.0, 5.0), vec4(0.0, 0.0, 0.0, 0.0))"),
        10.0
    );
}

#[test]
fn throws_if_distance_function_takes_invalid_number_of_arguments() {
    parse_throws("distance(0.0)");
    parse_throws("distance(1, 3, 0)");
}

#[test]
fn throws_if_distance_function_takes_mismatching_types() {
    eval_throws("distance(1, vec2(3.0, 2.0))");
    eval_throws("distance(vec4(5.0, 2.0, 3.0, 1.0), vec3(4.0, 4.0, 4.0))");
}

#[test]
fn evaluates_dot_function() {
    assert_eq!(n("dot(1, 2)"), 2.0);
    assert_eq!(n("dot(vec2(1.0, 1.0), vec2(2.0, 2.0))"), 4.0);
    assert_eq!(n("dot(vec3(1.0, 2.0, 3.0), vec3(2.0, 2.0, 1.0))"), 9.0);
    assert_eq!(n("dot(vec4(5.0, 5.0, 2.0, 3.0), vec4(1.0, 2.0, 1.0, 1.0))"), 20.0);
}

#[test]
fn throws_if_dot_function_takes_invalid_number_of_arguments() {
    parse_throws("dot(0.0)");
    parse_throws("dot(1, 3, 0)");
}

#[test]
fn throws_if_dot_function_takes_mismatching_types() {
    eval_throws("dot(1, vec2(3.0, 2.0))");
    eval_throws("dot(vec4(5.0, 2.0, 3.0, 1.0), vec3(4.0, 4.0, 4.0))");
}

#[test]
fn evaluates_cross_function() {
    assert_eq!(
        v3("cross(vec3(1.0, 1.0, 1.0), vec3(2.0, 2.0, 2.0))"),
        DVec3::new(0.0, 0.0, 0.0)
    );
    assert_eq!(
        v3("cross(vec3(-1.0, -1.0, -1.0), vec3(0.0, -2.0, -5.0))"),
        DVec3::new(3.0, -5.0, 2.0)
    );
    assert_eq!(
        v3("cross(vec3(5.0, -2.0, 1.0), vec3(-2.0, -6.0, -8.0))"),
        DVec3::new(22.0, 38.0, -34.0)
    );
}

#[test]
fn throws_if_cross_function_takes_invalid_number_of_arguments() {
    parse_throws("cross(vec3(0.0, 0.0, 0.0))");
    parse_throws("cross(vec3(0.0, 0.0, 0.0), vec3(1.0, 1.0, 1.0), vec3(2.0, 2.0, 2.0))");
}

#[test]
fn throws_if_cross_function_does_not_take_vec3_arguments() {
    eval_throws("cross(vec2(1.0, 2.0), vec2(3.0, 2.0))");
    eval_throws("cross(vec4(5.0, 2.0, 3.0, 1.0), vec3(4.0, 4.0, 4.0))");
}

// ===========================================================================
// Ternary conditional  (upstream L2922-2933)
// ===========================================================================

#[test]
fn evaluates_ternary_conditional() {
    assert_eq!(s(r#"true ? "first" : "second""#), "first");
    assert_eq!(s(r#"false ? "first" : "second""#), "second");
    assert_eq!(
        n("(!(1 + 2 > 3)) ? (2 > 1 ? 1 + 1 : 0) : (2 > 1 ? -1 + -1 : 0)"),
        2.0
    );
}

#[test]
fn throws_if_conditional_test_is_not_boolean() {
    eval_throws(r#"1 ? "first" : "second""#);
}

// ===========================================================================
// Regular expressions  (upstream L3091-3355)
// ===========================================================================

#[test]
fn constructs_regex() {
    let f = MockFeature::new().prop("pattern", Value::String("[abc]".into()));
    assert_eq!(s(r#"regExp("a").toString()"#), "/a/");
    assert_eq!(s(r#"regExp("\w").toString()"#), r"/\w/");
    assert_eq!(s("regExp(1 + 1).toString()"), "/2/");
    assert_eq!(s("regExp(true).toString()"), "/true/");
    assert_eq!(s("regExp().toString()"), "/(?:)/");
    assert_eq!(sf("regExp(${pattern}).toString()", &f), "/[abc]/");
}

#[test]
fn constructs_regex_with_flags() {
    assert_eq!(s(r#"regExp("a", "i").toString()"#), "/a/i");
    // flags are sorted ("dgimsuy"): "m"+"g" -> "gm"
    assert_eq!(s(r#"regExp("a", "m" + "g").toString()"#), "/a/gm");
}

#[test]
#[ignore = "regex crate has no look-around (upstream uses (?<=\\s) look-behind)"]
fn does_not_throw_syntax_error_if_regex_constructor_has_invalid_pattern() {
    // Upstream: `regExp("(?<=\\s)")` must not throw a *SyntaxError* (JS defers
    // the compile). The Rust `regex` crate has no look-behind, so the literal
    // pattern fails to compile at parse time; this case is not portable.
    let _ = Expression::try_new(r#"regExp("(?<=\s)")"#, None);
}

#[test]
fn throws_if_regex_constructor_has_invalid_flags() {
    // computed pattern -> compiled at evaluate time
    eval_throws(r#"regExp("a" + "b", "q")"#);
    // literal pattern -> compiled at parse time
    parse_throws(r#"regExp("a", "q")"#);
}

#[test]
fn evaluates_regex_test_function() {
    let f = MockFeature::new().prop("property", Value::String("abc".into()));
    assert!(b(r#"regExp("a").test("abc")"#));
    assert!(!b(r#"regExp("a").test("bcd")"#));
    assert!(b(
        r#"regExp("quick\s(brown).+?(jumps)", "ig").test("The Quick Brown Fox Jumps Over The Lazy Dog")"#
    ));
    // test() with no argument folds to false at parse time
    assert!(!b(r#"regExp("a").test()"#));
    assert!(bf("regExp(${property}).test(${property})", &f));
}

#[test]
fn throws_if_regex_test_function_has_invalid_arguments() {
    eval_throws(r#"regExp("1").test(1)"#);
    eval_throws(r#"regExp("a").test(regExp("b"))"#);
}

#[test]
fn evaluates_regex_exec_function() {
    let f = MockFeature::new()
        .prop("property", Value::String("abc".into()))
        .prop("Name", Value::String("Building 1".into()));
    assert_eq!(s(r#"regExp("a(.)", "i").exec("Abc")"#), "b");
    assert_eq!(ev(r#"regExp("a(.)").exec("qbc")"#), Value::Null);
    // exec() with no argument folds to null at parse time
    assert_eq!(ev(r#"regExp("a(.)").exec()"#), Value::Null);
    assert_eq!(
        s(r#"regExp("quick\s(b.*n).+?(jumps)", "ig").exec("The Quick Brown Fox Jumps Over The Lazy Dog")"#),
        "Brown"
    );
    assert_eq!(sf(r#"regExp("(" + ${property} + ")").exec(${property})"#, &f), "abc");
    assert_eq!(sf(r#"regExp("Building\s(\d)").exec(${Name})"#, &f), "1");
}

#[test]
fn throws_if_regex_exec_function_has_invalid_arguments() {
    eval_throws(r#"regExp("1").exec(1)"#);
    eval_throws(r#"regExp("a").exec(regExp("b"))"#);
}

#[test]
fn evaluates_regex_match_operator() {
    let f = MockFeature::new().prop("property", Value::String("abc".into()));
    assert!(b(r#"regExp("a") =~ "abc""#));
    assert!(b(r#""abc" =~ regExp("a")"#));
    assert!(!b(r#"regExp("a") =~ "bcd""#));
    assert!(!b(r#""bcd" =~ regExp("a")"#));
    assert!(b(
        r#"regExp("quick\s(brown).+?(jumps)", "ig") =~ "The Quick Brown Fox Jumps Over The Lazy Dog""#
    ));
    assert!(bf("regExp(${property}) =~ ${property}", &f));
}

#[test]
fn throws_if_regex_match_operator_has_invalid_arguments() {
    eval_throws(r#"regExp("a") =~ 1"#);
    eval_throws(r#"1 =~ regExp("a")"#);
    eval_throws("1 =~ 1");
}

#[test]
fn evaluates_regex_not_match_operator() {
    let f = MockFeature::new().prop("property", Value::String("abc".into()));
    assert!(!b(r#"regExp("a") !~ "abc""#));
    assert!(!b(r#""abc" !~ regExp("a")"#));
    assert!(b(r#"regExp("a") !~ "bcd""#));
    assert!(b(r#""bcd" !~ regExp("a")"#));
    assert!(!b(
        r#"regExp("quick\s(brown).+?(jumps)", "ig") !~ "The Quick Brown Fox Jumps Over The Lazy Dog""#
    ));
    assert!(!bf("regExp(${property}) !~ ${property}", &f));
}

#[test]
fn throws_if_regex_not_match_operator_has_invalid_arguments() {
    eval_throws(r#"regExp("a") !~ 1"#);
    eval_throws(r#"1 !~ regExp("a")"#);
    eval_throws("1 !~ 1");
}

#[test]
fn throws_if_test_is_not_called_with_a_reg_exp() {
    parse_throws(r#"color("blue").test()"#);
    parse_throws(r#""blue".test()"#);
}

#[test]
fn evaluates_reg_exp_to_string_function() {
    let f = MockFeature::new().prop("property", Value::String("abc".into()));
    assert_eq!(s("regExp().toString()"), "/(?:)/");
    assert_eq!(s(r#"regExp("\d\s\d", "ig").toString()"#), r"/\d\s\d/gi");
    assert_eq!(sf("regExp(${property}).toString()", &f), "/abc/");
}

#[test]
fn throws_when_using_to_string_on_other_type() {
    let f = MockFeature::new().prop("property", Value::String("abc".into()));
    // toString() is only defined for RegExp/vector values, not a plain string.
    let e = Expression::try_new("${property}.toString()", None).unwrap();
    assert!(e.evaluate(Some(&f)).is_err());
}

// ===========================================================================
// Member expressions  (upstream L2935-3074; object-property sub-cases skipped:
// the isolated domain `Value` enum has no JS-object variant)
// ===========================================================================

#[test]
fn evaluates_member_expression_with_dot() {
    let f = MockFeature::new()
        .prop("vector", Value::Cartesian4(DVec4::new(1.0, 0.0, 0.0, 0.0)))
        .prop("height", Value::Number(10.0))
        .prop("undefined", Value::Undefined);
    assert_eq!(nf("${vector.x}", &f), 1.0);
    assert_eq!(nf("${vector.z}", &f), 0.0);
    // a number has no `.z` component -> undefined
    assert_eq!(evf("${height.z}", &f), Value::Undefined);
    assert_eq!(evf("${undefined.z}", &f), Value::Undefined);
}

#[test]
fn evaluates_member_expression_with_brackets() {
    let f = MockFeature::new()
        .prop("vector", Value::Cartesian4(DVec4::new(1.0, 0.0, 0.0, 0.0)))
        .prop("height", Value::Number(10.0))
        .prop("undefined", Value::Undefined);
    assert_eq!(nf(r#"${vector["x"]}"#, &f), 1.0);
    assert_eq!(nf(r#"${vector["z"]}"#, &f), 0.0);
    assert_eq!(evf(r#"${height["z"]}"#, &f), Value::Undefined);
    assert_eq!(evf(r#"${undefined["z"]}"#, &f), Value::Undefined);
}

#[test]
fn member_expressions_throw_without_variable_notation() {
    parse_throws("color.r");
    parse_throws(r#"color["r"]"#);
}

// ===========================================================================
// Array expressions  (upstream L3357-3411; object-element sub-cases skipped)
// ===========================================================================

#[test]
fn evaluates_array_expression() {
    assert_eq!(
        ev("[1, 2, 3]"),
        Value::Array(vec![Value::Number(1.0), Value::Number(2.0), Value::Number(3.0)])
    );
}

#[test]
fn evaluates_array_expression_with_mixed_elements() {
    let f = MockFeature::new().prop("property", Value::String("value".into()));
    match evf(r#"[1+2, "hello", 2 < 3, color("blue"), ${property}]"#, &f) {
        Value::Array(items) => {
            assert_eq!(items.len(), 5);
            assert_eq!(items[0], Value::Number(3.0));
            assert_eq!(items[1], Value::String("hello".into()));
            assert_eq!(items[2], Value::Boolean(true));
            match &items[3] {
                Value::Cartesian4(c) => close4(*c, [0.0, 0.0, 1.0, 1.0], EPSILON7),
                o => panic!("expected a color, got {o}"),
            }
            assert_eq!(items[4], Value::String("value".into()));
        }
        o => panic!("expected an array, got {o}"),
    }
}

#[test]
fn evaluates_array_index_member_access() {
    let f = MockFeature::new().prop(
        "array",
        Value::Array(vec![
            Value::Cartesian4(DVec4::new(1.0, 0.0, 0.0, 0.0)),
            Value::Cartesian4(DVec4::new(0.0, 1.0, 0.0, 0.0)),
            Value::Cartesian4(DVec4::new(0.0, 0.0, 1.0, 0.0)),
        ]),
    );
    assert_eq!(
        evf("${array[1]}", &f),
        Value::Cartesian4(DVec4::new(0.0, 1.0, 0.0, 0.0))
    );
}

// ===========================================================================
// Built-in variables  (upstream L3413-3420)
// ===========================================================================

#[test]
fn evaluates_tiles3d_tileset_time_expression() {
    // DEVIATION: the CPU-side domain port has no tileset context, so
    // `tiles3d_tileset_time` is always 0.0 (matches the undefined-feature case).
    let f = MockFeature::new();
    assert_eq!(nf("${tiles3d_tileset_time}", &f), 0.0);
    assert_eq!(n("${tiles3d_tileset_time}"), 0.0);
}

// ===========================================================================
// getVariables  (upstream L4143-4149)
// ===========================================================================

#[test]
fn gets_variables() {
    let e = Expression::try_new(
        r#"${feature["w"]} + ${feature.x} + ${y} + ${y} + "${z}""#,
        None,
    )
    .unwrap();
    let mut vars = e.get_variables();
    vars.sort();
    assert_eq!(
        vars,
        vec!["w".to_string(), "x".to_string(), "y".to_string(), "z".to_string()]
    );
}

// ===========================================================================
// Deferred: GLSL shader codegen (upstream L3422-4234, ~60 cases)
//
// `getShaderFunction` / `getShaderExpression` emit GLSL source. The engine only
// interprets on the CPU (no codegen path yet), so these are represented by a
// single ignored marker documenting the deferral rather than 60 stubs.
// ===========================================================================

#[test]
#[ignore = "GLSL codegen (getShaderFunction/getShaderExpression) deferred to a later milestone"]
fn gets_shader_function_deferred() {
    // Upstream `getShaderFunction("getShow()", {}, {}, "bool")` on `true`
    // yields "bool getShow()\n{\n    return true;\n}\n". Not implemented.
    unimplemented!("GLSL codegen deferred");
}
