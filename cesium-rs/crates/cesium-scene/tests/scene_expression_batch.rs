//! Mirrors packages/engine/Specs/Scene/ExpressionSpec.js.
//!
//! DEVIATION: spec cases relying on plain-object feature properties
//! (`${feature.vector}`, `${address.street}`, object arrays, nested object
//! member chains) are not mirrored because the Rust `Value` type has no
//! object variant; scalar/vector/array property cases are mirrored instead.
//! DEVIATION: `tiles3d_tileset_time` always evaluates to 0.0 (no tileset
//! context is available on the CPU-side `ExpressionFeature` trait).

use std::collections::HashMap;

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartesian4::Cartesian4;
use cesium_core::color::Color;
use cesium_core::math::CesiumMath;
use cesium_scene::expression::{Expression, ExpressionFeature, ShaderState, Value};
use cesium_scene::expression_node_type::ExpressionNodeType;

// ---------------------------------------------------------------------------
// MockFeature (mirrors the spec's MockFeature)
// ---------------------------------------------------------------------------

struct MockFeature {
    properties: Vec<(String, Value)>,
    class_name: Option<String>,
    inherited_class_name: Option<String>,
}

impl MockFeature {
    fn new() -> Self {
        Self {
            properties: Vec::new(),
            class_name: None,
            inherited_class_name: None,
        }
    }

    fn add_property(&mut self, name: &str, value: Value) {
        self.properties.push((name.to_string(), value));
    }

    fn set_class(&mut self, class_name: &str) {
        self.class_name = Some(class_name.to_string());
    }

    fn set_inherited_class(&mut self, class_name: &str) {
        self.inherited_class_name = Some(class_name.to_string());
    }
}

impl ExpressionFeature for MockFeature {
    fn get_property_inherited(&self, name: &str) -> Option<Value> {
        self.properties
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
    }

    fn is_exact_class(&self, class_name: &Value) -> bool {
        match (&self.class_name, class_name) {
            (Some(class), Value::String(name)) => class == name,
            _ => false,
        }
    }

    fn is_class(&self, class_name: &Value) -> bool {
        match class_name {
            Value::String(name) => {
                self.class_name.as_deref() == Some(name)
                    || self.inherited_class_name.as_deref() == Some(name)
            }
            _ => false,
        }
    }

    fn get_exact_class_name(&self) -> Option<Value> {
        self.class_name
            .as_ref()
            .map(|name| Value::String(name.clone()))
    }
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

fn eval_str(expression: &str) -> Value {
    Expression::new(expression, None).evaluate(None).unwrap()
}

fn eval_with(expression: &str, feature: &MockFeature) -> Value {
    Expression::new(expression, None)
        .evaluate(Some(feature))
        .unwrap()
}

fn assert_number(value: Value, expected: f64) {
    match value {
        Value::Number(actual) => assert!(
            (actual - expected).abs() < 1e-12,
            "expected {expected}, got {actual}"
        ),
        other => panic!("expected number {expected}, got {other:?}"),
    }
}

fn assert_number_epsilon(value: Value, expected: f64, epsilon: f64) {
    match value {
        Value::Number(actual) => assert!(
            (actual - expected).abs() <= epsilon,
            "expected {expected}, got {actual}"
        ),
        other => panic!("expected number {expected}, got {other:?}"),
    }
}

fn assert_nan(value: Value) {
    match value {
        Value::Number(actual) => assert!(actual.is_nan(), "expected NaN, got {actual}"),
        other => panic!("expected NaN, got {other:?}"),
    }
}

fn assert_string(value: Value, expected: &str) {
    match value {
        Value::String(actual) => assert_eq!(actual, expected),
        other => panic!("expected string {expected:?}, got {other:?}"),
    }
}

fn assert_boolean(value: Value, expected: bool) {
    match value {
        Value::Boolean(actual) => assert_eq!(actual, expected),
        other => panic!("expected boolean {expected}, got {other:?}"),
    }
}

fn assert_undefined(value: Value) {
    assert!(matches!(value, Value::Undefined), "expected undefined, got {value:?}");
}

fn assert_null(value: Value) {
    assert!(matches!(value, Value::Null), "expected null, got {value:?}");
}

fn assert_cartesian2(value: Value, x: f64, y: f64) {
    match value {
        Value::Cartesian2(v) => assert_eq!((v.x, v.y), (x, y)),
        other => panic!("expected Cartesian2({x}, {y}), got {other:?}"),
    }
}

fn assert_cartesian3(value: Value, x: f64, y: f64, z: f64) {
    match value {
        Value::Cartesian3(v) => assert_eq!((v.x, v.y, v.z), (x, y, z)),
        other => panic!("expected Cartesian3({x}, {y}, {z}), got {other:?}"),
    }
}

fn assert_cartesian4(value: Value, x: f64, y: f64, z: f64, w: f64) {
    match value {
        Value::Cartesian4(v) => assert_eq!((v.x, v.y, v.z, v.w), (x, y, z, w)),
        other => panic!("expected Cartesian4({x}, {y}, {z}, {w}), got {other:?}"),
    }
}

fn assert_cartesian2_epsilon(value: Value, x: f64, y: f64, epsilon: f64) {
    match value {
        Value::Cartesian2(v) => {
            assert!((v.x - x).abs() <= epsilon, "x: expected {x}, got {}", v.x);
            assert!((v.y - y).abs() <= epsilon, "y: expected {y}, got {}", v.y);
        }
        other => panic!("expected Cartesian2, got {other:?}"),
    }
}

fn assert_cartesian3_epsilon(value: Value, x: f64, y: f64, z: f64, epsilon: f64) {
    match value {
        Value::Cartesian3(v) => {
            assert!((v.x - x).abs() <= epsilon, "x: expected {x}, got {}", v.x);
            assert!((v.y - y).abs() <= epsilon, "y: expected {y}, got {}", v.y);
            assert!((v.z - z).abs() <= epsilon, "z: expected {z}, got {}", v.z);
        }
        other => panic!("expected Cartesian3, got {other:?}"),
    }
}

fn assert_cartesian4_epsilon(value: Value, x: f64, y: f64, z: f64, w: f64, epsilon: f64) {
    match value {
        Value::Cartesian4(v) => {
            assert!((v.x - x).abs() <= epsilon, "x: expected {x}, got {}", v.x);
            assert!((v.y - y).abs() <= epsilon, "y: expected {y}, got {}", v.y);
            assert!((v.z - z).abs() <= epsilon, "z: expected {z}, got {}", v.z);
            assert!((v.w - w).abs() <= epsilon, "w: expected {w}, got {}", v.w);
        }
        other => panic!("expected Cartesian4, got {other:?}"),
    }
}

fn assert_construct_error(expression: &str) {
    assert!(
        Expression::try_new(expression, None).is_err(),
        "expected construction error for {expression:?}"
    );
}

fn assert_eval_error(expression: &str) {
    assert!(
        Expression::new(expression, None).evaluate(None).is_err(),
        "expected evaluation error for {expression:?}"
    );
}

fn cartesian4_to_value(color: &Color) -> Value {
    Value::Cartesian4(Cartesian4::from_elements_new(
        color.red,
        color.green,
        color.blue,
        color.alpha,
    ))
}

// ---------------------------------------------------------------------------
// Basic parsing / variables
// ---------------------------------------------------------------------------

#[test]
fn parses_backslashes() {
    let expression = Expression::new("\"\\he\\\\\\ll\\\\o\"", None);
    assert_string(expression.evaluate(None).unwrap(), "\\he\\\\\\ll\\\\o");
}

#[test]
fn evaluates_variable() {
    let mut feature = MockFeature::new();
    feature.add_property("height", Value::Number(10.0));
    feature.add_property("width", Value::Number(5.0));
    feature.add_property("string", Value::String("hello".to_string()));
    feature.add_property("boolean", Value::Boolean(true));
    feature.add_property("vector", Value::Cartesian3(Cartesian3::UNIT_X));
    feature.add_property("null", Value::Null);
    feature.add_property("undefined", Value::Undefined);

    assert_number(eval_with("${height}", &feature), 10.0);
    assert_string(eval_with("'${height}'", &feature), "10");
    assert_number(eval_with("${height}/${width}", &feature), 2.0);
    assert_string(eval_with("${string}", &feature), "hello");
    assert_string(eval_with("'replace ${string}'", &feature), "replace hello");
    assert_string(
        eval_with("'replace ${string} multiple ${height}'", &feature),
        "replace hello multiple 10",
    );
    assert_string(
        eval_with("'replace ${height} ${string}'", &feature),
        "replace 10 hello",
    );
    assert_string(eval_with("\"replace ${string}\"", &feature), "replace hello");
    assert_string(eval_with("'replace ${string'", &feature), "replace ${string");
    assert_boolean(eval_with("${boolean}", &feature), true);
    assert_string(eval_with("'${boolean}'", &feature), "true");
    assert_cartesian3(eval_with("${vector}", &feature), 1.0, 0.0, 0.0);
    assert_string(eval_with("'${vector}'", &feature), "(1, 0, 0)");
    assert_null(eval_with("${null}", &feature));
    assert_string(eval_with("'${null}'", &feature), "");
    assert_undefined(eval_with("${undefined}", &feature));
    assert_string(eval_with("'${undefined}'", &feature), "");
    assert_number(
        eval_with(
            "abs(-${height}) + max(${height}, ${width}) + clamp(${height}, 0, 2)",
            &feature,
        ),
        22.0,
    );
    assert_construct_error("${height");
}

#[test]
fn evaluates_variable_to_undefined_if_feature_is_undefined() {
    assert_undefined(eval_str("${height}"));
    assert_undefined(eval_str("${vector.x}"));
    assert_undefined(eval_str("${feature}"));
    assert_undefined(eval_str("${feature.vector}"));
    assert_undefined(eval_str("${vector[\"x\"]}"));
    assert_undefined(eval_str("${feature[\"vector\"]}"));
    // Evaluating inside a string is an exception. "" is returned instead of "undefined"
    assert_string(eval_str("'${height}'"), "");
}

#[test]
fn evaluates_with_defines() {
    let defines = HashMap::from([("halfHeight".to_string(), "${Height}/2".to_string())]);
    let mut feature = MockFeature::new();
    feature.add_property("Height", Value::Number(10.0));
    let expression = Expression::new("${halfHeight}", Some(&defines));
    assert_number(expression.evaluate(Some(&feature)).unwrap(), 5.0);
}

#[test]
fn evaluates_with_defines_honoring_order_of_operations() {
    let defines = HashMap::from([("value".to_string(), "1 + 2".to_string())]);
    let expression = Expression::new("5.0 * ${value}", Some(&defines));
    assert_number(expression.evaluate(None).unwrap(), 15.0);
}

#[test]
fn evaluate_takes_a_color_result_argument() {
    let expression = Expression::new("color(\"red\")", None);
    let mut result = Color::default();
    let color = expression.evaluate_color(None, &mut result).unwrap();
    assert_eq!(*color, Color::RED);
}

#[test]
fn gets_expressions() {
    let expression_string = "(regExp('^Chest').test(${County})) && (${YearBuilt} >= 1970)";
    let expression = Expression::new(expression_string, None);
    assert_eq!(expression.expression(), expression_string);
}

#[test]
fn throws_on_invalid_expressions() {
    assert_construct_error("");
    assert_construct_error("this");
    assert_construct_error("2; 3;");
}

#[test]
fn throws_on_unknown_characters() {
    assert_construct_error("#");
}

#[test]
fn throws_on_unmatched_parenthesis() {
    assert_construct_error("((true)");
    assert_construct_error("(true))");
}

#[test]
fn throws_on_unknown_identifiers() {
    assert_construct_error("flse");
}

#[test]
fn throws_on_unknown_function_calls() {
    assert_construct_error("unknown()");
}

#[test]
fn throws_on_unknown_member_function_calls() {
    assert_construct_error("regExp().unknown()");
}

#[test]
fn throws_with_unsupported_operators() {
    assert_construct_error("~1");
    assert_construct_error("2 | 3");
    assert_construct_error("2 & 3");
    assert_construct_error("2 << 3");
    assert_construct_error("2 >> 3");
    assert_construct_error("2 >>> 3");
}

// ---------------------------------------------------------------------------
// Literals
// ---------------------------------------------------------------------------

#[test]
fn evaluates_literal_null() {
    assert_null(eval_str("null"));
}

#[test]
fn evaluates_literal_undefined() {
    assert_undefined(eval_str("undefined"));
}

#[test]
fn evaluates_literal_boolean() {
    assert_boolean(eval_str("true"), true);
    assert_boolean(eval_str("false"), false);
}

#[test]
fn converts_to_literal_boolean() {
    assert_boolean(eval_str("Boolean()"), false);
    assert_boolean(eval_str("Boolean(1)"), true);
    assert_boolean(eval_str("Boolean(\"true\")"), true);
}

#[test]
fn evaluates_literal_number() {
    assert_number(eval_str("1"), 1.0);
    assert_number(eval_str("0"), 0.0);
    assert_nan(eval_str("NaN"));
    match eval_str("Infinity") {
        Value::Number(n) => assert!(n.is_infinite() && n > 0.0),
        other => panic!("expected Infinity, got {other:?}"),
    }
}

#[test]
fn evaluates_math_constants() {
    assert_number(eval_str("Math.PI"), std::f64::consts::PI);
    assert_number(eval_str("Math.E"), std::f64::consts::E);
}

#[test]
fn evaluates_number_constants() {
    match eval_str("Number.POSITIVE_INFINITY") {
        Value::Number(n) => assert!(n.is_infinite() && n > 0.0),
        other => panic!("expected Infinity, got {other:?}"),
    }
}

#[test]
fn converts_to_literal_number() {
    assert_number(eval_str("Number()"), 0.0);
    assert_number(eval_str("Number(\"1\")"), 1.0);
    assert_number(eval_str("Number(true)"), 1.0);
}

#[test]
fn evaluates_literal_string() {
    assert_string(eval_str("'hello'"), "hello");
    assert_string(eval_str("'Cesium'"), "Cesium");
    assert_string(eval_str("\"Cesium\""), "Cesium");
}

#[test]
fn converts_to_literal_string() {
    assert_string(eval_str("String()"), "");
    assert_string(eval_str("String(1)"), "1");
    assert_string(eval_str("String(true)"), "true");
}

// ---------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------

#[test]
fn evaluates_literal_color() {
    assert_eq!(eval_str("color('#ffffff')"), cartesian4_to_value(&Color::WHITE));
    assert_eq!(eval_str("color('#00FFFF')"), cartesian4_to_value(&Color::CYAN));
    assert_eq!(eval_str("color('#fff')"), cartesian4_to_value(&Color::WHITE));
    assert_eq!(eval_str("color('#0FF')"), cartesian4_to_value(&Color::CYAN));
    assert_eq!(eval_str("color('white')"), cartesian4_to_value(&Color::WHITE));
    assert_eq!(eval_str("color('cyan')"), cartesian4_to_value(&Color::CYAN));
    assert_eq!(
        eval_str("color('white', 0.5)"),
        cartesian4_to_value(&Color::from_alpha(&Color::WHITE, 0.5))
    );
    assert_eq!(eval_str("rgb(255, 255, 255)"), cartesian4_to_value(&Color::WHITE));
    assert_eq!(
        eval_str("rgb(100, 255, 190)"),
        cartesian4_to_value(&Color::from_bytes(100, 255, 190, 255))
    );
    assert_eq!(eval_str("hsl(0, 0, 1)"), cartesian4_to_value(&Color::WHITE));
    assert_eq!(
        eval_str("hsl(1.0, 0.6, 0.7)"),
        cartesian4_to_value(&Color::from_hsl(1.0, 0.6, 0.7, 1.0))
    );
    assert_eq!(
        eval_str("rgba(255, 255, 255, 0.5)"),
        cartesian4_to_value(&Color::from_alpha(&Color::WHITE, 0.5))
    );
    assert_eq!(
        eval_str("rgba(100, 255, 190, 0.25)"),
        Value::Cartesian4(Cartesian4::from_elements_new(
            100.0 / 255.0,
            1.0,
            190.0 / 255.0,
            0.25
        ))
    );
    assert_eq!(
        eval_str("hsla(0, 0, 1, 0.5)"),
        cartesian4_to_value(&Color::from_hsl(0.0, 0.0, 1.0, 0.5))
    );
    assert_eq!(
        eval_str("hsla(1.0, 0.6, 0.7, 0.75)"),
        cartesian4_to_value(&Color::from_hsl(1.0, 0.6, 0.7, 0.75))
    );
    assert_eq!(eval_str("color()"), cartesian4_to_value(&Color::WHITE));
}

#[test]
fn evaluates_literal_color_with_result_parameter() {
    let cases = [
        ("color('#0000ff')", Color::BLUE),
        ("color('#f00')", Color::RED),
        ("color('cyan')", Color::CYAN),
        ("color('white', 0.5)", Color::from_alpha(&Color::WHITE, 0.5)),
        ("rgb(0, 0, 0)", Color::BLACK),
        ("hsl(0, 0, 1)", Color::WHITE),
        ("rgba(255, 0, 255, 0.5)", Color::new(1.0, 0.0, 1.0, 0.5)),
        ("hsla(0, 0, 1, 0.5)", Color::from_hsl(0.0, 0.0, 1.0, 0.5)),
        ("color()", Color::WHITE),
    ];
    for (source, expected) in cases {
        let expression = Expression::new(source, None);
        let mut color = Color::default();
        let value = expression.evaluate_color(None, &mut color).unwrap();
        assert_eq!(*value, expected, "case {source}");
        assert_eq!(color, expected, "case {source}");
    }
}

#[test]
fn evaluates_color_with_expressions_as_arguments() {
    let mut feature = MockFeature::new();
    feature.add_property("hex6", Value::String("#ffffff".to_string()));
    feature.add_property("hex3", Value::String("#fff".to_string()));
    feature.add_property("keyword", Value::String("white".to_string()));
    feature.add_property("alpha", Value::Number(0.2));

    assert_eq!(
        eval_with("color(${hex6})", &feature),
        cartesian4_to_value(&Color::WHITE)
    );
    assert_eq!(
        eval_with("color(${hex3})", &feature),
        cartesian4_to_value(&Color::WHITE)
    );
    assert_eq!(
        eval_with("color(${keyword})", &feature),
        cartesian4_to_value(&Color::WHITE)
    );
    assert_cartesian4(
        eval_with("color(${keyword}, ${alpha} + 0.6)", &feature),
        1.0,
        1.0,
        1.0,
        0.8,
    );
}

#[test]
fn evaluates_rgb_with_expressions_as_arguments() {
    let mut feature = MockFeature::new();
    feature.add_property("red", Value::Number(100.0));
    feature.add_property("green", Value::Number(200.0));
    feature.add_property("blue", Value::Number(255.0));

    assert_eq!(
        eval_with("rgb(${red}, ${green}, ${blue})", &feature),
        cartesian4_to_value(&Color::from_bytes(100, 200, 255, 255))
    );
    assert_eq!(
        eval_with("rgb(${red}/2, ${green}/2, ${blue})", &feature),
        cartesian4_to_value(&Color::from_bytes(50, 100, 255, 255))
    );
}

#[test]
fn evaluates_hsl_with_expressions_as_arguments() {
    let mut feature = MockFeature::new();
    feature.add_property("h", Value::Number(0.0));
    feature.add_property("s", Value::Number(0.0));
    feature.add_property("l", Value::Number(1.0));

    assert_eq!(
        eval_with("hsl(${h}, ${s}, ${l})", &feature),
        cartesian4_to_value(&Color::WHITE)
    );
    assert_eq!(
        eval_with("hsl(${h} + 0.2, ${s} + 1.0, ${l} - 0.5)", &feature),
        cartesian4_to_value(&Color::from_hsl(0.2, 1.0, 0.5, 1.0))
    );
}

#[test]
fn evaluates_rgba_with_expressions_as_arguments() {
    let mut feature = MockFeature::new();
    feature.add_property("red", Value::Number(100.0));
    feature.add_property("green", Value::Number(200.0));
    feature.add_property("blue", Value::Number(255.0));
    feature.add_property("a", Value::Number(0.3));

    assert_cartesian4(
        eval_with("rgba(${red}, ${green}, ${blue}, ${a})", &feature),
        100.0 / 255.0,
        200.0 / 255.0,
        1.0,
        0.3,
    );
    assert_cartesian4(
        eval_with("rgba(${red}/2, ${green}/2, ${blue}, ${a} * 2)", &feature),
        50.0 / 255.0,
        100.0 / 255.0,
        1.0,
        0.6,
    );
}

#[test]
fn evaluates_hsla_with_expressions_as_arguments() {
    let mut feature = MockFeature::new();
    feature.add_property("h", Value::Number(0.0));
    feature.add_property("s", Value::Number(0.0));
    feature.add_property("l", Value::Number(1.0));
    feature.add_property("a", Value::Number(1.0));

    assert_eq!(
        eval_with("hsla(${h}, ${s}, ${l}, ${a})", &feature),
        cartesian4_to_value(&Color::WHITE)
    );
    assert_eq!(
        eval_with(
            "hsla(${h} + 0.2, ${s} + 1.0, ${l} - 0.5, ${a} / 4)",
            &feature
        ),
        cartesian4_to_value(&Color::from_hsl(0.2, 1.0, 0.5, 0.25))
    );
}

#[test]
fn color_constructors_throw_with_wrong_number_of_arguments() {
    assert_construct_error("rgb(255, 255)");
    assert_construct_error("hsl(1, 1)");
    assert_construct_error("rgba(255, 255, 255)");
    assert_construct_error("hsla(1, 1, 1)");
}

#[test]
fn evaluates_color_properties_r_g_b_a() {
    assert_number(eval_str("color('#ffffff').r"), 1.0);
    assert_number(eval_str("rgb(255, 255, 0).g"), 1.0);
    assert_number(eval_str("color(\"cyan\").b"), 1.0);
    assert_number(eval_str("rgba(255, 255, 0, 0.5).a"), 0.5);
}

#[test]
fn evaluates_color_properties_x_y_z_w() {
    assert_number(eval_str("color('#ffffff').x"), 1.0);
    assert_number(eval_str("rgb(255, 255, 0).y"), 1.0);
    assert_number(eval_str("color(\"cyan\").z"), 1.0);
    assert_number(eval_str("rgba(255, 255, 0, 0.5).w"), 0.5);
}

#[test]
fn evaluates_color_properties_numeric_index() {
    assert_number(eval_str("color('#ffffff')[0]"), 1.0);
    assert_number(eval_str("rgb(255, 255, 0)[1]"), 1.0);
    assert_number(eval_str("color(\"cyan\")[2]"), 1.0);
    assert_number(eval_str("rgba(255, 255, 0, 0.5)[3]"), 0.5);
}

#[test]
fn evaluates_color_properties_string_index_rgba() {
    assert_number(eval_str("color('#ffffff')[\"r\"]"), 1.0);
    assert_number(eval_str("rgb(255, 255, 0)[\"g\"]"), 1.0);
    assert_number(eval_str("color(\"cyan\")[\"b\"]"), 1.0);
    assert_number(eval_str("rgba(255, 255, 0, 0.5)[\"a\"]"), 0.5);
}

#[test]
fn evaluates_color_properties_string_index_xyzw() {
    assert_number(eval_str("color('#ffffff')[\"x\"]"), 1.0);
    assert_number(eval_str("rgb(255, 255, 0)[\"y\"]"), 1.0);
    assert_number(eval_str("color(\"cyan\")[\"z\"]"), 1.0);
    assert_number(eval_str("rgba(255, 255, 0, 0.5)[\"w\"]"), 0.5);
}

// ---------------------------------------------------------------------------
// Vectors
// ---------------------------------------------------------------------------

#[test]
fn evaluates_vec2() {
    assert_cartesian2(eval_str("vec2(2.0)"), 2.0, 2.0);
    assert_cartesian2(eval_str("vec2(3.0, 4.0)"), 3.0, 4.0);
    assert_cartesian2(eval_str("vec2(vec2(3.0, 4.0))"), 3.0, 4.0);
    assert_cartesian2(eval_str("vec2(vec3(3.0, 4.0, 5.0))"), 3.0, 4.0);
    assert_cartesian2(eval_str("vec2(vec4(3.0, 4.0, 5.0, 6.0))"), 3.0, 4.0);
}

#[test]
fn throws_if_vec2_has_invalid_number_of_arguments() {
    assert_eval_error("vec2()");
    assert_eval_error("vec2(3.0, 4.0, 5.0)");
    assert_eval_error("vec2(vec2(3.0, 4.0), 5.0)");
}

#[test]
fn throws_if_vec2_has_invalid_argument() {
    assert_eval_error("vec2(\"1\")");
}

#[test]
fn evaluates_vec3() {
    assert_cartesian3(eval_str("vec3(2.0)"), 2.0, 2.0, 2.0);
    assert_cartesian3(eval_str("vec3(3.0, 4.0, 5.0)"), 3.0, 4.0, 5.0);
    assert_cartesian3(eval_str("vec3(vec2(3.0, 4.0), 5.0)"), 3.0, 4.0, 5.0);
    assert_cartesian3(eval_str("vec3(3.0, vec2(4.0, 5.0))"), 3.0, 4.0, 5.0);
    assert_cartesian3(eval_str("vec3(vec3(3.0, 4.0, 5.0))"), 3.0, 4.0, 5.0);
    assert_cartesian3(eval_str("vec3(vec4(3.0, 4.0, 5.0, 6.0))"), 3.0, 4.0, 5.0);
}

#[test]
fn throws_if_vec3_has_invalid_number_of_arguments() {
    assert_eval_error("vec3()");
    assert_eval_error("vec3(3.0, 4.0)");
    assert_eval_error("vec3(3.0, 4.0, 5.0, 6.0)");
    assert_eval_error("vec3(vec2(3.0, 4.0), vec2(5.0, 6.0))");
    assert_eval_error("vec3(vec4(3.0, 4.0, 5.0, 6.0), 1.0)");
}

#[test]
fn throws_if_vec3_has_invalid_argument() {
    assert_eval_error("vec3(1.0, \"1.0\", 2.0)");
}

#[test]
fn evaluates_vec4() {
    assert_cartesian4(eval_str("vec4(2.0)"), 2.0, 2.0, 2.0, 2.0);
    assert_cartesian4(eval_str("vec4(3.0, 4.0, 5.0, 6.0)"), 3.0, 4.0, 5.0, 6.0);
    assert_cartesian4(eval_str("vec4(vec2(3.0, 4.0), 5.0, 6.0)"), 3.0, 4.0, 5.0, 6.0);
    assert_cartesian4(eval_str("vec4(3.0, vec2(4.0, 5.0), 6.0)"), 3.0, 4.0, 5.0, 6.0);
    assert_cartesian4(eval_str("vec4(3.0, 4.0, vec2(5.0, 6.0))"), 3.0, 4.0, 5.0, 6.0);
    assert_cartesian4(eval_str("vec4(vec3(3.0, 4.0, 5.0), 6.0)"), 3.0, 4.0, 5.0, 6.0);
    assert_cartesian4(eval_str("vec4(3.0, vec3(4.0, 5.0, 6.0))"), 3.0, 4.0, 5.0, 6.0);
    assert_cartesian4(eval_str("vec4(vec4(3.0, 4.0, 5.0, 6.0))"), 3.0, 4.0, 5.0, 6.0);
}

#[test]
fn throws_if_vec4_has_invalid_number_of_arguments() {
    assert_eval_error("vec4()");
    assert_eval_error("vec4(3.0, 4.0)");
    assert_eval_error("vec4(3.0, 4.0, 5.0)");
    assert_eval_error("vec4(3.0, 4.0, 5.0, 6.0, 7.0)");
    assert_eval_error("vec4(vec3(3.0, 4.0, 5.0))");
}

#[test]
fn throws_if_vec4_has_invalid_argument() {
    assert_eval_error("vec4(1.0, \"2.0\", 3.0, 4.0)");
}

#[test]
fn evaluates_vector_with_expressions_as_arguments() {
    let mut feature = MockFeature::new();
    feature.add_property("height", Value::Number(2.0));
    feature.add_property("width", Value::Number(4.0));
    feature.add_property("depth", Value::Number(3.0));
    feature.add_property("scale", Value::Number(1.0));

    assert_cartesian4(
        eval_with("vec4(${height}, ${width}, ${depth}, ${scale})", &feature),
        2.0,
        4.0,
        3.0,
        1.0,
    );
}

#[test]
fn evaluates_expression_with_multiple_nested_vectors() {
    assert_cartesian4(
        eval_str("vec4(vec2(1, 2)[vec3(6, 1, 5).y], 2, vec4(1.0).w, 5)"),
        2.0,
        2.0,
        1.0,
        5.0,
    );
}

#[test]
fn evaluates_vector_properties_xyzw() {
    assert_number(eval_str("vec4(1.0, 2.0, 3.0, 4.0).x"), 1.0);
    assert_number(eval_str("vec4(1.0, 2.0, 3.0, 4.0).y"), 2.0);
    assert_number(eval_str("vec4(1.0, 2.0, 3.0, 4.0).z"), 3.0);
    assert_number(eval_str("vec4(1.0, 2.0, 3.0, 4.0).w"), 4.0);
}

#[test]
fn evaluates_vector_properties_rgba() {
    assert_number(eval_str("vec4(1.0, 2.0, 3.0, 4.0).r"), 1.0);
    assert_number(eval_str("vec4(1.0, 2.0, 3.0, 4.0).g"), 2.0);
    assert_number(eval_str("vec4(1.0, 2.0, 3.0, 4.0).b"), 3.0);
    assert_number(eval_str("vec4(1.0, 2.0, 3.0, 4.0).a"), 4.0);
}

#[test]
fn evaluates_vector_properties_numeric_index() {
    assert_number(eval_str("vec4(1.0, 2.0, 3.0, 4.0)[0]"), 1.0);
    assert_number(eval_str("vec4(1.0, 2.0, 3.0, 4.0)[1]"), 2.0);
    assert_number(eval_str("vec4(1.0, 2.0, 3.0, 4.0)[2]"), 3.0);
    assert_number(eval_str("vec4(1.0, 2.0, 3.0, 4.0)[3]"), 4.0);
}

#[test]
fn evaluates_vector_properties_string_index() {
    assert_number(eval_str("vec4(1.0, 2.0, 3.0, 4.0)[\"x\"]"), 1.0);
    assert_number(eval_str("vec4(1.0, 2.0, 3.0, 4.0)[\"y\"]"), 2.0);
    assert_number(eval_str("vec4(1.0, 2.0, 3.0, 4.0)[\"z\"]"), 3.0);
    assert_number(eval_str("vec4(1.0, 2.0, 3.0, 4.0)[\"w\"]"), 4.0);
    assert_number(eval_str("vec4(1.0, 2.0, 3.0, 4.0)[\"r\"]"), 1.0);
    assert_number(eval_str("vec4(1.0, 2.0, 3.0, 4.0)[\"g\"]"), 2.0);
    assert_number(eval_str("vec4(1.0, 2.0, 3.0, 4.0)[\"b\"]"), 3.0);
    assert_number(eval_str("vec4(1.0, 2.0, 3.0, 4.0)[\"a\"]"), 4.0);
}

// ---------------------------------------------------------------------------
// Unary operators
// ---------------------------------------------------------------------------

#[test]
fn evaluates_unary_not() {
    assert_boolean(eval_str("!true"), false);
    assert_boolean(eval_str("!!true"), true);
}

#[test]
fn throws_if_unary_not_takes_invalid_argument() {
    assert_eval_error("!\"true\"");
}

#[test]
fn evaluates_unary_negative() {
    assert_number(eval_str("-5"), -5.0);
    assert_number(eval_str("-(-5)"), 5.0);
}

#[test]
fn throws_if_unary_negative_takes_invalid_argument() {
    assert_eval_error("-\"56\"");
}

#[test]
fn evaluates_unary_positive() {
    assert_number(eval_str("+5"), 5.0);
}

#[test]
fn throws_if_unary_positive_takes_invalid_argument() {
    assert_eval_error("+\"56\"");
}

// ---------------------------------------------------------------------------
// Binary operators
// ---------------------------------------------------------------------------

#[test]
fn evaluates_binary_addition() {
    assert_number(eval_str("1 + 2"), 3.0);
    assert_number(eval_str("1 + 2 + 3 + 4"), 10.0);
}

#[test]
fn evaluates_binary_addition_with_strings() {
    assert_string(eval_str("1 + \"10\""), "110");
    assert_string(eval_str("\"10\" + 1"), "101");
    assert_string(eval_str("\"name_\" + \"building\""), "name_building");
    assert_string(eval_str("\"name_\" + true"), "name_true");
    assert_string(eval_str("\"name_\" + null"), "name_null");
    assert_string(eval_str("\"name_\" + undefined"), "name_undefined");
    assert_string(eval_str("\"name_\" + vec2(1.1)"), "name_(1.1, 1.1)");
    assert_string(eval_str("\"name_\" + vec3(1.1)"), "name_(1.1, 1.1, 1.1)");
    assert_string(
        eval_str("\"name_\" + vec4(1.1)"),
        "name_(1.1, 1.1, 1.1, 1.1)",
    );
    assert_string(eval_str("\"name_\" + regExp(\"a\")"), "name_/a/");
}

#[test]
fn throws_if_binary_addition_takes_invalid_arguments() {
    assert_eval_error("vec2(1.0) + vec3(1.0)");
    assert_eval_error("1.0 + vec3(1.0)");
}

#[test]
fn evaluates_binary_subtraction() {
    assert_number(eval_str("2 - 1"), 1.0);
    assert_number(eval_str("4 - 3 - 2 - 1"), -2.0);
}

#[test]
fn throws_if_binary_subtraction_takes_invalid_arguments() {
    assert_eval_error("vec2(1.0) - vec3(1.0)");
    assert_eval_error("1.0 - vec3(1.0)");
    assert_eval_error("\"name1\" - \"name2\"");
}

#[test]
fn evaluates_binary_multiplication() {
    assert_number(eval_str("1 * 2"), 2.0);
    assert_number(eval_str("1 * 2 * 3 * 4"), 24.0);
}

#[test]
fn throws_if_binary_multiplication_takes_invalid_arguments() {
    assert_eval_error("vec2(1.0) * vec3(1.0)");
    assert_eval_error("vec2(1.0) * \"name\"");
}

#[test]
fn evaluates_binary_division() {
    assert_number(eval_str("2 / 1"), 2.0);
    assert_number(eval_str("1/2"), 0.5);
    assert_number(eval_str("24 / -4 / 2"), -3.0);
}

#[test]
fn throws_if_binary_division_takes_invalid_arguments() {
    assert_eval_error("vec2(1.0) / vec3(1.0)");
    assert_eval_error("vec2(1.0) / \"2.0\"");
    assert_eval_error("1.0 / vec4(1.0)");
}

#[test]
fn evaluates_binary_modulus() {
    assert_number(eval_str("2 % 1"), 0.0);
    assert_number(eval_str("6 % 4 % 3"), 2.0);
}

#[test]
fn throws_if_binary_modulus_takes_invalid_arguments() {
    assert_eval_error("vec2(1.0) % vec3(1.0)");
    assert_eval_error("vec2(1.0) % \"2.0\"");
    assert_eval_error("1.0 % vec4(1.0)");
}

#[test]
fn evaluates_binary_equals_strict() {
    assert_boolean(eval_str("'hello' === 'hello'"), true);
    assert_boolean(eval_str("1 === 2"), false);
    assert_boolean(eval_str("false === true === false"), true);
    assert_boolean(eval_str("1 === \"1\""), false);
}

#[test]
fn evaluates_binary_not_equals_strict() {
    assert_boolean(eval_str("'hello' !== 'hello'"), false);
    assert_boolean(eval_str("1 !== 2"), true);
    assert_boolean(eval_str("false !== true !== false"), true);
    assert_boolean(eval_str("1 !== \"1\""), true);
}

#[test]
fn evaluates_binary_less_than() {
    assert_boolean(eval_str("2 < 3"), true);
    assert_boolean(eval_str("2 < 2"), false);
    assert_boolean(eval_str("3 < 2"), false);
}

#[test]
fn throws_if_binary_less_than_takes_invalid_arguments() {
    assert_eval_error("vec2(1.0) < vec2(2.0)");
    assert_eval_error("1 < vec3(1.0)");
    assert_eval_error("true < false");
    assert_eval_error("color('blue') < 10");
}

#[test]
fn evaluates_binary_less_than_or_equals() {
    assert_boolean(eval_str("2 <= 3"), true);
    assert_boolean(eval_str("2 <= 2"), true);
    assert_boolean(eval_str("3 <= 2"), false);
}

#[test]
fn throws_if_binary_less_than_or_equals_takes_invalid_arguments() {
    assert_eval_error("vec2(1.0) <= vec2(2.0)");
    assert_eval_error("1 <= vec3(1.0)");
    assert_eval_error("1.0 <= \"5\"");
    assert_eval_error("true <= false");
    assert_eval_error("color('blue') <= 10");
}

#[test]
fn evaluates_binary_greater_than() {
    assert_boolean(eval_str("2 > 3"), false);
    assert_boolean(eval_str("2 > 2"), false);
    assert_boolean(eval_str("3 > 2"), true);
}

#[test]
fn throws_if_binary_greater_than_takes_invalid_arguments() {
    assert_eval_error("vec2(1.0) > vec2(2.0)");
    assert_eval_error("1 > vec3(1.0)");
    assert_eval_error("1.0 > \"5\"");
    assert_eval_error("true > false");
    assert_eval_error("color('blue') > 10");
}

#[test]
fn evaluates_binary_greater_than_or_equals() {
    assert_boolean(eval_str("2 >= 3"), false);
    assert_boolean(eval_str("2 >= 2"), true);
    assert_boolean(eval_str("3 >= 2"), true);
}

#[test]
fn throws_if_binary_greater_than_or_equals_takes_invalid_arguments() {
    assert_eval_error("vec2(1.0) >= vec2(2.0)");
    assert_eval_error("1 >= vec3(1.0)");
    assert_eval_error("1.0 >= \"5\"");
    assert_eval_error("true >= false");
    assert_eval_error("color('blue') >= 10");
}

#[test]
fn evaluates_logical_and() {
    assert_boolean(eval_str("false && false"), false);
    assert_boolean(eval_str("false && true"), false);
    assert_boolean(eval_str("true && true"), true);
    assert_eval_error("2 && color('red')");
}

#[test]
fn throws_with_invalid_and_operands() {
    assert_eval_error("2 && true");
    assert_eval_error("true && color('red')");
}

#[test]
fn evaluates_logical_or() {
    assert_boolean(eval_str("false || false"), false);
    assert_boolean(eval_str("false || true"), true);
    assert_boolean(eval_str("true || true"), true);
}

#[test]
fn throws_with_invalid_or_operands() {
    assert_eval_error("2 || false");
    assert_eval_error("false || color('red')");
}

#[test]
fn evaluates_color_operations() {
    assert_cartesian4(eval_str("+rgba(255, 0, 0, 1.0)"), 1.0, 0.0, 0.0, 1.0);
    assert_cartesian4(
        eval_str("rgba(255, 0, 0, 0.5) + rgba(0, 0, 255, 0.5)"),
        1.0,
        0.0,
        1.0,
        1.0,
    );
    assert_cartesian4(
        eval_str("rgba(0, 255, 255, 1.0) - rgba(0, 255, 0, 0)"),
        0.0,
        0.0,
        1.0,
        1.0,
    );
    assert_cartesian4(
        eval_str("rgba(255, 255, 255, 1.0) * rgba(255, 0, 0, 1.0)"),
        1.0,
        0.0,
        0.0,
        1.0,
    );
    assert_cartesian4(eval_str("rgba(255, 255, 0, 1.0) * 1.0"), 1.0, 1.0, 0.0, 1.0);
    assert_cartesian4(eval_str("1 * rgba(255, 255, 0, 1.0)"), 1.0, 1.0, 0.0, 1.0);
    assert_cartesian4(
        eval_str("rgba(255, 255, 255, 1.0) / rgba(255, 255, 255, 1.0)"),
        1.0,
        1.0,
        1.0,
        1.0,
    );
    assert_cartesian4(
        eval_str("rgba(255, 255, 255, 1.0) / 2"),
        0.5,
        0.5,
        0.5,
        0.5,
    );
    assert_cartesian4(
        eval_str("rgba(255, 255, 255, 1.0) % rgba(255, 255, 255, 1.0)"),
        0.0,
        0.0,
        0.0,
        0.0,
    );
    assert_boolean(eval_str("color('green') === color('green')"), true);
    assert_boolean(eval_str("color('green') !== color('green')"), false);
}

#[test]
fn evaluates_vector_operations() {
    assert_cartesian2(eval_str("+vec2(1, 2)"), 1.0, 2.0);
    assert_cartesian3(eval_str("+vec3(1, 2, 3)"), 1.0, 2.0, 3.0);
    assert_cartesian4(eval_str("+vec4(1, 2, 3, 4)"), 1.0, 2.0, 3.0, 4.0);
    assert_cartesian2(eval_str("-vec2(1, 2)"), -1.0, -2.0);
    assert_cartesian3(eval_str("-vec3(1, 2, 3)"), -1.0, -2.0, -3.0);
    assert_cartesian4(eval_str("-vec4(1, 2, 3, 4)"), -1.0, -2.0, -3.0, -4.0);
    assert_cartesian2(eval_str("vec2(1, 2) + vec2(3, 4)"), 4.0, 6.0);
    assert_cartesian3(eval_str("vec3(1, 2, 3) + vec3(3, 4, 5)"), 4.0, 6.0, 8.0);
    assert_cartesian4(
        eval_str("vec4(1, 2, 3, 4) + vec4(3, 4, 5, 6)"),
        4.0,
        6.0,
        8.0,
        10.0,
    );
    assert_cartesian2(eval_str("vec2(1, 2) - vec2(3, 4)"), -2.0, -2.0);
    assert_cartesian3(eval_str("vec3(1, 2, 3) - vec3(3, 4, 5)"), -2.0, -2.0, -2.0);
    assert_cartesian4(
        eval_str("vec4(1, 2, 3, 4) - vec4(3, 4, 5, 6)"),
        -2.0,
        -2.0,
        -2.0,
        -2.0,
    );
    assert_cartesian2(eval_str("vec2(1, 2) * vec2(3, 4)"), 3.0, 8.0);
    assert_cartesian2(eval_str("vec2(1, 2) * 3.0"), 3.0, 6.0);
    assert_cartesian2(eval_str("3.0 * vec2(1, 2)"), 3.0, 6.0);
    assert_cartesian3(eval_str("vec3(1, 2, 3) * vec3(3, 4, 5)"), 3.0, 8.0, 15.0);
    assert_cartesian3(eval_str("vec3(1, 2, 3) * 3.0"), 3.0, 6.0, 9.0);
    assert_cartesian3(eval_str("3.0 * vec3(1, 2, 3)"), 3.0, 6.0, 9.0);
    assert_cartesian4(
        eval_str("vec4(1, 2, 3, 4) * vec4(3, 4, 5, 6)"),
        3.0,
        8.0,
        15.0,
        24.0,
    );
    assert_cartesian4(eval_str("vec4(1, 2, 3, 4) * 3.0"), 3.0, 6.0, 9.0, 12.0);
    assert_cartesian4(eval_str("3.0 * vec4(1, 2, 3, 4)"), 3.0, 6.0, 9.0, 12.0);
    assert_cartesian2(eval_str("vec2(1, 2) / vec2(2, 5)"), 0.5, 0.4);
    assert_cartesian2(eval_str("vec2(1, 2) / 2.0"), 0.5, 1.0);
    assert_cartesian3(eval_str("vec3(1, 2, 3) / vec3(2, 5, 3)"), 0.5, 0.4, 1.0);
    assert_cartesian3(eval_str("vec3(1, 2, 3) / 2.0"), 0.5, 1.0, 1.5);
    assert_cartesian4(
        eval_str("vec4(1, 2, 3, 4) / vec4(2, 5, 3, 2)"),
        0.5,
        0.4,
        1.0,
        2.0,
    );
    assert_cartesian4(eval_str("vec4(1, 2, 3, 4) / 2.0"), 0.5, 1.0, 1.5, 2.0);
    assert_cartesian2(eval_str("vec2(2, 3) % vec2(3, 3)"), 2.0, 0.0);
    assert_cartesian3(eval_str("vec3(2, 3, 4) % vec3(3, 3, 3)"), 2.0, 0.0, 1.0);
    assert_cartesian4(
        eval_str("vec4(2, 3, 4, 5) % vec4(3, 3, 3, 2)"),
        2.0,
        0.0,
        1.0,
        1.0,
    );
    assert_boolean(eval_str("vec2(1, 2) === vec2(1, 2)"), true);
    assert_boolean(eval_str("vec3(1, 2, 3) === vec3(1, 2, 3)"), true);
    assert_boolean(eval_str("vec4(1, 2, 3, 4) === vec4(1, 2, 3, 4)"), true);
    assert_boolean(eval_str("vec2(1, 2) !== vec2(1, 2)"), false);
    assert_boolean(eval_str("vec3(1, 2, 3) !== vec3(1, 2, 3)"), false);
    assert_boolean(eval_str("vec4(1, 2, 3, 4) !== vec4(1, 2, 3, 4)"), false);
}

#[test]
fn evaluates_color_to_string_function() {
    assert_string(eval_str("color(\"red\").toString()"), "(1, 0, 0, 1)");
    assert_string(eval_str("rgba(0, 0, 255, 0.5).toString()"), "(0, 0, 1, 0.5)");
}

#[test]
fn evaluates_vector_to_string_function() {
    let mut feature = MockFeature::new();
    feature.add_property(
        "property",
        Value::Cartesian4(Cartesian4::from_elements_new(1.0, 2.0, 3.0, 4.0)),
    );

    assert_string(eval_str("vec2(1, 2).toString()"), "(1, 2)");
    assert_string(eval_str("vec3(1, 2, 3).toString()"), "(1, 2, 3)");
    assert_string(eval_str("vec4(1, 2, 3, 4).toString()"), "(1, 2, 3, 4)");
    assert_string(eval_with("${property}.toString()", &feature), "(1, 2, 3, 4)");
}

#[test]
fn evaluates_is_nan_function() {
    assert_boolean(eval_str("isNaN()"), true);
    assert_boolean(eval_str("isNaN(NaN)"), true);
    assert_boolean(eval_str("isNaN(1)"), false);
    assert_boolean(eval_str("isNaN(Infinity)"), false);
    assert_boolean(eval_str("isNaN(null)"), false);
    assert_boolean(eval_str("isNaN(true)"), false);
    assert_boolean(eval_str("isNaN(\"hello\")"), true);
    assert_boolean(eval_str("isNaN(color(\"white\"))"), true);
}

#[test]
fn evaluates_is_finite_function() {
    assert_boolean(eval_str("isFinite()"), false);
    assert_boolean(eval_str("isFinite(NaN)"), false);
    assert_boolean(eval_str("isFinite(1)"), true);
    assert_boolean(eval_str("isFinite(Infinity)"), false);
    assert_boolean(eval_str("isFinite(null)"), true);
    assert_boolean(eval_str("isFinite(true)"), true);
    assert_boolean(eval_str("isFinite(\"hello\")"), false);
    assert_boolean(eval_str("isFinite(color(\"white\"))"), false);
}

#[test]
fn evaluates_is_exact_class_function() {
    let mut feature = MockFeature::new();
    feature.set_class("door");

    assert_boolean(eval_with("isExactClass(\"door\")", &feature), true);
    assert_boolean(eval_with("isExactClass(\"roof\")", &feature), false);
    assert_boolean(eval_str("isExactClass(\"roof\")"), false);
}

#[test]
fn throws_if_is_exact_class_takes_an_invalid_number_of_arguments() {
    assert_construct_error("isExactClass()");
    assert_construct_error("isExactClass(\"door\", \"roof\")");
}

#[test]
fn evaluates_is_class_function() {
    let mut feature = MockFeature::new();
    feature.set_class("door");
    feature.set_inherited_class("building");

    assert_boolean(
        eval_with("isClass(\"door\") && isClass(\"building\")", &feature),
        true,
    );
    assert_boolean(eval_str("isClass(\"door\") && isClass(\"building\")"), false);
}

#[test]
fn throws_if_is_class_takes_an_invalid_number_of_arguments() {
    assert_construct_error("isClass()");
    assert_construct_error("isClass(\"door\", \"building\")");
}

#[test]
fn evaluates_get_exact_class_name_function() {
    let mut feature = MockFeature::new();
    feature.set_class("door");
    assert_string(eval_with("getExactClassName()", &feature), "door");
    assert_undefined(eval_str("getExactClassName()"));
}

#[test]
fn throws_if_get_exact_class_name_takes_an_invalid_number_of_arguments() {
    assert_construct_error("getExactClassName(\"door\")");
}

#[test]
fn throws_if_builtin_unary_function_is_given_an_invalid_argument() {
    assert_eval_error("abs(\"-1\")");
}

// ---------------------------------------------------------------------------
// Math functions
// ---------------------------------------------------------------------------

#[test]
fn evaluates_abs_function() {
    assert_number(eval_str("abs(-1)"), 1.0);
    assert_number(eval_str("abs(1)"), 1.0);
    assert_cartesian2(eval_str("abs(vec2(-1.0, 1.0))"), 1.0, 1.0);
    assert_cartesian3(eval_str("abs(vec3(-1.0, 1.0, 0.0))"), 1.0, 1.0, 0.0);
    assert_cartesian4(eval_str("abs(vec4(-1.0, 1.0, 0.0, -1.2))"), 1.0, 1.0, 0.0, 1.2);
}

#[test]
fn throws_if_abs_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("abs()");
    assert_construct_error("abs(1, 2)");
}

#[test]
fn evaluates_cos_function() {
    assert_number(eval_str("cos(0)"), 1.0);
    assert_cartesian2_epsilon(
        eval_str("cos(vec2(0, Math.PI))"),
        1.0,
        -1.0,
        CesiumMath::EPSILON7,
    );
    assert_cartesian3_epsilon(
        eval_str("cos(vec3(0, Math.PI, -Math.PI))"),
        1.0,
        -1.0,
        -1.0,
        CesiumMath::EPSILON7,
    );
    assert_cartesian4_epsilon(
        eval_str("cos(vec4(0, Math.PI, -Math.PI, 0))"),
        1.0,
        -1.0,
        -1.0,
        1.0,
        CesiumMath::EPSILON7,
    );
}

#[test]
fn throws_if_cos_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("cos()");
    assert_construct_error("cos(1, 2)");
}

#[test]
fn evaluates_sin_function() {
    assert_number_epsilon(eval_str("sin(0)"), 0.0, CesiumMath::EPSILON10);
    assert_cartesian2_epsilon(
        eval_str("sin(vec2(0, Math.PI/2))"),
        0.0,
        1.0,
        CesiumMath::EPSILON7,
    );
    assert_cartesian3_epsilon(
        eval_str("sin(vec3(0, Math.PI/2, -Math.PI/2))"),
        0.0,
        1.0,
        -1.0,
        CesiumMath::EPSILON7,
    );
    assert_cartesian4_epsilon(
        eval_str("sin(vec4(0, Math.PI/2, -Math.PI/2, 0))"),
        0.0,
        1.0,
        -1.0,
        0.0,
        CesiumMath::EPSILON7,
    );
}

#[test]
fn throws_if_sin_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("sin()");
    assert_construct_error("sin(1, 2)");
}

#[test]
fn evaluates_tan_function() {
    assert_number_epsilon(eval_str("tan(0)"), 0.0, CesiumMath::EPSILON10);
    assert_cartesian2_epsilon(
        eval_str("tan(vec2(0, Math.PI/4))"),
        0.0,
        1.0,
        CesiumMath::EPSILON7,
    );
    assert_cartesian3_epsilon(
        eval_str("tan(vec3(0, Math.PI/4, Math.PI))"),
        0.0,
        1.0,
        0.0,
        CesiumMath::EPSILON7,
    );
    assert_cartesian4_epsilon(
        eval_str("tan(vec4(0, Math.PI/4, Math.PI, -Math.PI/4))"),
        0.0,
        1.0,
        0.0,
        -1.0,
        CesiumMath::EPSILON7,
    );
}

#[test]
fn throws_if_tan_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("tan()");
    assert_construct_error("tan(1, 2)");
}

#[test]
fn evaluates_acos_function() {
    assert_number(eval_str("acos(1)"), 0.0);
    assert_cartesian2_epsilon(
        eval_str("acos(vec2(1, 0))"),
        0.0,
        CesiumMath::PI_OVER_TWO,
        CesiumMath::EPSILON7,
    );
    assert_cartesian3_epsilon(
        eval_str("acos(vec3(1, 0, 1))"),
        0.0,
        CesiumMath::PI_OVER_TWO,
        0.0,
        CesiumMath::EPSILON7,
    );
    assert_cartesian4_epsilon(
        eval_str("acos(vec4(1, 0, 1, 0))"),
        0.0,
        CesiumMath::PI_OVER_TWO,
        0.0,
        CesiumMath::PI_OVER_TWO,
        CesiumMath::EPSILON7,
    );
}

#[test]
fn throws_if_acos_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("acos()");
    assert_construct_error("acos(1, 2)");
}

#[test]
fn evaluates_asin_function() {
    assert_number(eval_str("asin(0)"), 0.0);
    assert_cartesian2_epsilon(
        eval_str("asin(vec2(0, 1))"),
        0.0,
        CesiumMath::PI_OVER_TWO,
        CesiumMath::EPSILON7,
    );
    assert_cartesian3_epsilon(
        eval_str("asin(vec3(0, 1, 0))"),
        0.0,
        CesiumMath::PI_OVER_TWO,
        0.0,
        CesiumMath::EPSILON7,
    );
    assert_cartesian4_epsilon(
        eval_str("asin(vec4(0, 1, 0, 1))"),
        0.0,
        CesiumMath::PI_OVER_TWO,
        0.0,
        CesiumMath::PI_OVER_TWO,
        CesiumMath::EPSILON7,
    );
}

#[test]
fn throws_if_asin_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("asin()");
    assert_construct_error("asin(1, 2)");
}

#[test]
fn evaluates_atan_function() {
    assert_number(eval_str("atan(0)"), 0.0);
    assert_cartesian2_epsilon(
        eval_str("atan(vec2(0, 1))"),
        0.0,
        CesiumMath::PI_OVER_FOUR,
        CesiumMath::EPSILON7,
    );
    assert_cartesian3_epsilon(
        eval_str("atan(vec3(0, 1, 0))"),
        0.0,
        CesiumMath::PI_OVER_FOUR,
        0.0,
        CesiumMath::EPSILON7,
    );
    assert_cartesian4_epsilon(
        eval_str("atan(vec4(0, 1, 0, 1))"),
        0.0,
        CesiumMath::PI_OVER_FOUR,
        0.0,
        CesiumMath::PI_OVER_FOUR,
        CesiumMath::EPSILON7,
    );
}

#[test]
fn throws_if_atan_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("atan()");
    assert_construct_error("atan(1, 2)");
}

#[test]
fn evaluates_radians_function() {
    assert_number_epsilon(eval_str("radians(180)"), std::f64::consts::PI, CesiumMath::EPSILON10);
    assert_cartesian2_epsilon(
        eval_str("radians(vec2(180, 90))"),
        std::f64::consts::PI,
        CesiumMath::PI_OVER_TWO,
        CesiumMath::EPSILON7,
    );
    assert_cartesian3_epsilon(
        eval_str("radians(vec3(180, 90, 180))"),
        std::f64::consts::PI,
        CesiumMath::PI_OVER_TWO,
        std::f64::consts::PI,
        CesiumMath::EPSILON7,
    );
    assert_cartesian4_epsilon(
        eval_str("radians(vec4(180, 90, 180, 90))"),
        std::f64::consts::PI,
        CesiumMath::PI_OVER_TWO,
        std::f64::consts::PI,
        CesiumMath::PI_OVER_TWO,
        CesiumMath::EPSILON7,
    );
}

#[test]
fn throws_if_radians_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("radians()");
    assert_construct_error("radians(1, 2)");
}

#[test]
fn evaluates_degrees_function() {
    assert_number_epsilon(eval_str("degrees(2 * Math.PI)"), 360.0, CesiumMath::EPSILON10);
    assert_cartesian2_epsilon(
        eval_str("degrees(vec2(2 * Math.PI, Math.PI))"),
        360.0,
        180.0,
        CesiumMath::EPSILON7,
    );
    assert_cartesian3_epsilon(
        eval_str("degrees(vec3(2 * Math.PI, Math.PI, 2 * Math.PI))"),
        360.0,
        180.0,
        360.0,
        CesiumMath::EPSILON7,
    );
    assert_cartesian4_epsilon(
        eval_str("degrees(vec4(2 * Math.PI, Math.PI, 2 * Math.PI, Math.PI))"),
        360.0,
        180.0,
        360.0,
        180.0,
        CesiumMath::EPSILON7,
    );
}

#[test]
fn throws_if_degrees_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("degrees()");
    assert_construct_error("degrees(1, 2)");
}

#[test]
fn evaluates_sqrt_function() {
    assert_number(eval_str("sqrt(1.0)"), 1.0);
    assert_number(eval_str("sqrt(4.0)"), 2.0);
    assert_nan(eval_str("sqrt(-1.0)"));
    assert_cartesian2(eval_str("sqrt(vec2(1.0, 4.0))"), 1.0, 2.0);
    assert_cartesian3(eval_str("sqrt(vec3(1.0, 4.0, 9.0))"), 1.0, 2.0, 3.0);
    assert_cartesian4(eval_str("sqrt(vec4(1.0, 4.0, 9.0, 16.0))"), 1.0, 2.0, 3.0, 4.0);
}

#[test]
fn throws_if_sqrt_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("sqrt()");
    assert_construct_error("sqrt(1, 2)");
}

#[test]
fn evaluates_sign_function() {
    assert_number(eval_str("sign(5.0)"), 1.0);
    assert_number(eval_str("sign(0.0)"), 0.0);
    assert_number(eval_str("sign(-5.0)"), -1.0);
    assert_cartesian2(eval_str("sign(vec2(5.0, -5.0))"), 1.0, -1.0);
    assert_cartesian3(eval_str("sign(vec3(5.0, -5.0, 0.0))"), 1.0, -1.0, 0.0);
    assert_cartesian4(eval_str("sign(vec4(5.0, -5.0, 0.0, 1.0))"), 1.0, -1.0, 0.0, 1.0);
}

#[test]
fn throws_if_sign_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("sign()");
    assert_construct_error("sign(1, 2)");
}

#[test]
fn evaluates_floor_function() {
    assert_number(eval_str("floor(5.5)"), 5.0);
    assert_number(eval_str("floor(0.0)"), 0.0);
    assert_number(eval_str("floor(-1.2)"), -2.0);
    assert_cartesian2(eval_str("floor(vec2(5.5, -1.2))"), 5.0, -2.0);
    assert_cartesian3(eval_str("floor(vec3(5.5, -1.2, 0.0))"), 5.0, -2.0, 0.0);
    assert_cartesian4(eval_str("floor(vec4(5.5, -1.2, 0.0, -2.9))"), 5.0, -2.0, 0.0, -3.0);
}

#[test]
fn throws_if_floor_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("floor()");
    assert_construct_error("floor(1, 2)");
}

#[test]
fn evaluates_ceil_function() {
    assert_number(eval_str("ceil(5.5)"), 6.0);
    assert_number(eval_str("ceil(0.0)"), 0.0);
    assert_number(eval_str("ceil(-1.2)"), -1.0);
    assert_cartesian2(eval_str("ceil(vec2(5.5, -1.2))"), 6.0, -1.0);
    assert_cartesian3(eval_str("ceil(vec3(5.5, -1.2, 0.0))"), 6.0, -1.0, 0.0);
    assert_cartesian4(eval_str("ceil(vec4(5.5, -1.2, 0.0, -2.9))"), 6.0, -1.0, 0.0, -2.0);
}

#[test]
fn throws_if_ceil_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("ceil()");
    assert_construct_error("ceil(1, 2)");
}

#[test]
fn evaluates_round_function() {
    assert_number(eval_str("round(5.5)"), 6.0);
    assert_number(eval_str("round(0.0)"), 0.0);
    assert_number(eval_str("round(1.2)"), 1.0);
    assert_cartesian2(eval_str("round(vec2(5.5, -1.2))"), 6.0, -1.0);
    assert_cartesian3(eval_str("round(vec3(5.5, -1.2, 0.0))"), 6.0, -1.0, 0.0);
    assert_cartesian4(eval_str("round(vec4(5.5, -1.2, 0.0, -2.9))"), 6.0, -1.0, 0.0, -3.0);
}

#[test]
fn throws_if_round_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("round()");
    assert_construct_error("round(1, 2)");
}

#[test]
fn evaluates_exp_function() {
    assert_number_epsilon(eval_str("exp(1.0)"), std::f64::consts::E, CesiumMath::EPSILON10);
    assert_number_epsilon(eval_str("exp(0.0)"), 1.0, CesiumMath::EPSILON10);
    assert_cartesian2_epsilon(
        eval_str("exp(vec2(1.0, 0.0))"),
        std::f64::consts::E,
        1.0,
        CesiumMath::EPSILON10,
    );
    assert_cartesian3_epsilon(
        eval_str("exp(vec3(1.0, 0.0, 1.0))"),
        std::f64::consts::E,
        1.0,
        std::f64::consts::E,
        CesiumMath::EPSILON10,
    );
    assert_cartesian4_epsilon(
        eval_str("exp(vec4(1.0, 0.0, 1.0, 0.0))"),
        std::f64::consts::E,
        1.0,
        std::f64::consts::E,
        1.0,
        CesiumMath::EPSILON10,
    );
}

#[test]
fn throws_if_exp_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("exp()");
    assert_construct_error("exp(1, 2)");
}

#[test]
fn evaluates_exp2_function() {
    assert_number(eval_str("exp2(1.0)"), 2.0);
    assert_number(eval_str("exp2(0.0)"), 1.0);
    assert_number(eval_str("exp2(2.0)"), 4.0);
    assert_cartesian2(eval_str("exp2(vec2(1.0, 0.0))"), 2.0, 1.0);
    assert_cartesian3(eval_str("exp2(vec3(1.0, 0.0, 2.0))"), 2.0, 1.0, 4.0);
    assert_cartesian4(eval_str("exp2(vec4(1.0, 0.0, 2.0, 3.0))"), 2.0, 1.0, 4.0, 8.0);
}

#[test]
fn throws_if_exp2_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("exp2()");
    assert_construct_error("exp2(1, 2)");
}

#[test]
fn evaluates_log_function() {
    assert_number(eval_str("log(1.0)"), 0.0);
    assert_number_epsilon(eval_str("log(10.0)"), 2.302585092994046, CesiumMath::EPSILON7);
    assert_cartesian2(eval_str("log(vec2(1.0, Math.E))"), 0.0, 1.0);
    assert_cartesian3(eval_str("log(vec3(1.0, Math.E, 1.0))"), 0.0, 1.0, 0.0);
    assert_cartesian4(eval_str("log(vec4(1.0, Math.E, 1.0, Math.E))"), 0.0, 1.0, 0.0, 1.0);
}

#[test]
fn throws_if_log_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("log()");
    assert_construct_error("log(1, 2)");
}

#[test]
fn evaluates_log2_function() {
    assert_number(eval_str("log2(1.0)"), 0.0);
    assert_number(eval_str("log2(2.0)"), 1.0);
    assert_number(eval_str("log2(4.0)"), 2.0);
    assert_cartesian2(eval_str("log2(vec2(1.0, 2.0))"), 0.0, 1.0);
    assert_cartesian3(eval_str("log2(vec3(1.0, 2.0, 4.0))"), 0.0, 1.0, 2.0);
    assert_cartesian4_epsilon(
        eval_str("log2(vec4(1.0, 2.0, 4.0, 8.0))"),
        0.0,
        1.0,
        2.0,
        3.0,
        CesiumMath::EPSILON10,
    );
}

#[test]
fn throws_if_log2_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("log2()");
    assert_construct_error("log2(1, 2)");
}

#[test]
fn evaluates_fract_function() {
    assert_number(eval_str("fract(1.0)"), 0.0);
    assert_number(eval_str("fract(2.25)"), 0.25);
    assert_number(eval_str("fract(-2.25)"), 0.75);
    assert_cartesian2(eval_str("fract(vec2(1.0, 2.25))"), 0.0, 0.25);
    assert_cartesian3(eval_str("fract(vec3(1.0, 2.25, -2.25))"), 0.0, 0.25, 0.75);
    assert_cartesian4(eval_str("fract(vec4(1.0, 2.25, -2.25, 1.0))"), 0.0, 0.25, 0.75, 0.0);
}

#[test]
fn throws_if_fract_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("fract()");
    assert_construct_error("fract(1, 2)");
}

#[test]
fn evaluates_length_function() {
    assert_number(eval_str("length(-3.0)"), 3.0);
    assert_number(eval_str("length(vec2(-3.0, 4.0))"), 5.0);
    assert_number(eval_str("length(vec3(2.0, 3.0, 6.0))"), 7.0);
    assert_number(eval_str("length(vec4(2.0, 4.0, 7.0, 10.0))"), 13.0);
}

#[test]
fn throws_if_length_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("length()");
    assert_construct_error("length(1, 2)");
}

#[test]
fn evaluates_normalize_function() {
    assert_number(eval_str("normalize(5.0)"), 1.0);
    assert_cartesian2_epsilon(
        eval_str("normalize(vec2(3.0, 4.0))"),
        0.6,
        0.8,
        CesiumMath::EPSILON10,
    );
    let length: f64 = (2.0_f64 * 2.0 + 3.0 * 3.0 + 4.0 * 4.0).sqrt();
    assert_cartesian3_epsilon(
        eval_str("normalize(vec3(2.0, 3.0, -4.0))"),
        2.0 / length,
        3.0 / length,
        -4.0 / length,
        CesiumMath::EPSILON10,
    );
    let length: f64 = (2.0_f64 * 2.0 + 3.0 * 3.0 + 4.0 * 4.0 + 5.0 * 5.0).sqrt();
    assert_cartesian4_epsilon(
        eval_str("normalize(vec4(-2.0, 3.0, -4.0, 5.0))"),
        -2.0 / length,
        3.0 / length,
        -4.0 / length,
        5.0 / length,
        CesiumMath::EPSILON10,
    );
}

#[test]
fn evaluates_clamp_function() {
    assert_number(eval_str("clamp(50.0, 0.0, 100.0)"), 50.0);
    assert_number(eval_str("clamp(50.0, 0.0, 25.0)"), 25.0);
    assert_number(eval_str("clamp(50.0, 75.0, 100.0)"), 75.0);
    assert_cartesian2(
        eval_str("clamp(vec2(50.0,50.0), vec2(0.0,75.0), 100.0)"),
        50.0,
        75.0,
    );
    assert_cartesian2(
        eval_str("clamp(vec2(50.0,50.0), vec2(0.0,75.0), vec2(25.0,100.0))"),
        25.0,
        75.0,
    );
    assert_cartesian3(
        eval_str(
            "clamp(vec3(50.0, 50.0, 50.0), vec3(0.0, 0.0, 75.0), vec3(100.0, 25.0, 100.0))",
        ),
        50.0,
        25.0,
        75.0,
    );
    assert_cartesian4(
        eval_str(
            "clamp(vec4(50.0, 50.0, 50.0, 100.0), vec4(0.0, 0.0, 75.0, 75.0), vec4(100.0, 25.0, 100.0, 85.0))",
        ),
        50.0,
        25.0,
        75.0,
        85.0,
    );
}

#[test]
fn throws_if_clamp_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("clamp()");
    assert_construct_error("clamp(1)");
    assert_construct_error("clamp(1, 2)");
    assert_construct_error("clamp(1, 2, 3, 4)");
}

#[test]
fn throws_if_clamp_function_takes_mismatching_types() {
    assert_eval_error("clamp(0.0,vec2(0,1),0.0)");
    assert_eval_error("clamp(vec2(0,1),vec3(0,1,2),0.0)");
    assert_eval_error("clamp(vec2(0,1),vec2(0,1), vec3(1,2,3))");
}

#[test]
fn evaluates_mix_function() {
    assert_number(eval_str("mix(0.0, 2.0, 0.5)"), 1.0);
    assert_cartesian2(eval_str("mix(vec2(0.0,1.0), vec2(2.0,3.0), 0.5)"), 1.0, 2.0);
    assert_cartesian2(
        eval_str("mix(vec2(0.0,1.0), vec2(2.0,3.0), vec2(0.5,4.0))"),
        1.0,
        9.0,
    );
    assert_cartesian3(
        eval_str("mix(vec3(0.0,1.0,2.0), vec3(2.0,3.0,4.0), vec3(0.5,4.0,5.0))"),
        1.0,
        9.0,
        12.0,
    );
    assert_cartesian4(
        eval_str(
            "mix(vec4(0.0,1.0,2.0,1.5), vec4(2.0,3.0,4.0,2.5), vec4(0.5,4.0,5.0,3.5))",
        ),
        1.0,
        9.0,
        12.0,
        5.0,
    );
}

#[test]
fn throws_if_mix_function_takes_mismatching_types() {
    assert_eval_error("mix(0.0,vec2(0,1),0.0)");
    assert_eval_error("mix(vec2(0,1),vec3(0,1,2),0.0)");
    assert_eval_error("mix(vec2(0,1),vec2(0,1), vec3(1,2,3))");
}

#[test]
fn throws_if_mix_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("mix()");
    assert_construct_error("mix(1)");
    assert_construct_error("mix(1, 2)");
    assert_construct_error("mix(1, 2, 3, 4)");
}

#[test]
fn evaluates_atan2_function() {
    assert_number_epsilon(eval_str("atan2(0,1)"), 0.0, CesiumMath::EPSILON10);
    assert_number_epsilon(eval_str("atan2(1,0)"), 0.5 * std::f64::consts::PI, CesiumMath::EPSILON10);
    assert_cartesian2_epsilon(
        eval_str("atan2(vec2(0,1),vec2(1,0))"),
        0.0,
        0.5 * std::f64::consts::PI,
        CesiumMath::EPSILON10,
    );
    assert_cartesian3_epsilon(
        eval_str("atan2(vec3(0,1,0.5),vec3(1,0,0.5))"),
        0.0,
        0.5 * std::f64::consts::PI,
        0.25 * std::f64::consts::PI,
        CesiumMath::EPSILON10,
    );
    assert_cartesian4_epsilon(
        eval_str("atan2(vec4(0,1,0.5,1),vec4(1,0,0.5,0))"),
        0.0,
        0.5 * std::f64::consts::PI,
        0.25 * std::f64::consts::PI,
        0.5 * std::f64::consts::PI,
        CesiumMath::EPSILON10,
    );
}

#[test]
fn throws_if_atan2_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("atan2(0.0)");
    assert_construct_error("atan2(1, 2, 0)");
}

#[test]
fn throws_if_atan2_function_takes_mismatching_types() {
    assert_eval_error("atan2(0.0,vec2(0,1))");
    assert_eval_error("atan2(vec2(0,1),0.0)");
    assert_eval_error("atan2(vec2(0,1),vec3(0,1,2))");
}

#[test]
fn evaluates_pow_function() {
    assert_number(eval_str("pow(5,0)"), 1.0);
    assert_number(eval_str("pow(4,2)"), 16.0);
    assert_cartesian2(eval_str("pow(vec2(5,4),vec2(0,2))"), 1.0, 16.0);
    assert_cartesian3(eval_str("pow(vec3(5,4,3),vec3(0,2,3))"), 1.0, 16.0, 27.0);
    assert_cartesian4(eval_str("pow(vec4(5,4,3,2),vec4(0,2,3,5))"), 1.0, 16.0, 27.0, 32.0);
}

#[test]
fn throws_if_pow_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("pow(0.0)");
    assert_construct_error("pow(1, 2, 0)");
}

#[test]
fn throws_if_pow_function_takes_mismatching_types() {
    assert_eval_error("pow(0.0, vec2(0,1))");
    assert_eval_error("pow(vec2(0,1),0.0)");
    assert_eval_error("pow(vec2(0,1),vec3(0,1,2))");
}

#[test]
fn evaluates_min_function() {
    assert_number(eval_str("min(0,1)"), 0.0);
    assert_number(eval_str("min(-1,0)"), -1.0);
    assert_cartesian2(eval_str("min(vec2(-1,1),0)"), -1.0, 0.0);
    assert_cartesian2(eval_str("min(vec2(-1,2),vec2(0,1))"), -1.0, 1.0);
    assert_cartesian3(eval_str("min(vec3(-1,2,1),vec3(0,1,2))"), -1.0, 1.0, 1.0);
    assert_cartesian4(eval_str("min(vec4(-1,2,1,4),vec4(0,1,2,3))"), -1.0, 1.0, 1.0, 3.0);
}

#[test]
fn throws_if_min_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("min(0.0)");
    assert_construct_error("min(1, 2, 0)");
}

#[test]
fn throws_if_min_function_takes_mismatching_types() {
    assert_eval_error("min(0.0, vec2(0,1))");
    assert_eval_error("min(vec2(0,1),vec3(0,1,2))");
}

#[test]
fn evaluates_max_function() {
    assert_number(eval_str("max(0,1)"), 1.0);
    assert_number(eval_str("max(-1,0)"), 0.0);
    assert_cartesian2(eval_str("max(vec2(-1,1),0)"), 0.0, 1.0);
    assert_cartesian2(eval_str("max(vec2(-1,2),vec2(0,1))"), 0.0, 2.0);
    assert_cartesian3(eval_str("max(vec3(-1,2,1),vec3(0,1,2))"), 0.0, 2.0, 2.0);
    assert_cartesian4(eval_str("max(vec4(-1,2,1,4),vec4(0,1,2,3))"), 0.0, 2.0, 2.0, 4.0);
}

#[test]
fn throws_if_max_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("max(0.0)");
    assert_construct_error("max(1, 2, 0)");
}

#[test]
fn throws_if_max_function_takes_mismatching_types() {
    assert_eval_error("max(0.0, vec2(0,1))");
    assert_eval_error("max(vec2(0,1),vec3(0,1,2))");
}

#[test]
fn evaluates_the_distance_function() {
    assert_number(eval_str("distance(0, 1)"), 1.0);
    assert_number(eval_str("distance(vec2(1.0, 0.0), vec2(0.0, 0.0))"), 1.0);
    assert_number(eval_str("distance(vec3(3.0, 2.0, 1.0), vec3(1.0, 0.0, 0.0))"), 3.0);
    assert_number(
        eval_str("distance(vec4(5.0, 5.0, 5.0, 5.0), vec4(0.0, 0.0, 0.0, 0.0))"),
        10.0,
    );
}

#[test]
fn throws_if_distance_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("distance(0.0)");
    assert_construct_error("distance(1, 3, 0)");
}

#[test]
fn throws_if_distance_function_takes_mismatching_types_of_arguments() {
    assert_eval_error("distance(1, vec2(3.0, 2.0))");
    assert_eval_error("distance(vec4(5.0, 2.0, 3.0, 1.0), vec3(4.0, 4.0, 4.0))");
}

#[test]
fn evaluates_the_dot_function() {
    assert_number(eval_str("dot(1, 2)"), 2.0);
    assert_number(eval_str("dot(vec2(1.0, 1.0), vec2(2.0, 2.0))"), 4.0);
    assert_number(eval_str("dot(vec3(1.0, 2.0, 3.0), vec3(2.0, 2.0, 1.0))"), 9.0);
    assert_number(eval_str("dot(vec4(5.0, 5.0, 2.0, 3.0), vec4(1.0, 2.0, 1.0, 1.0))"), 20.0);
}

#[test]
fn throws_if_dot_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("dot(0.0)");
    assert_construct_error("dot(1, 3, 0)");
}

#[test]
fn throws_if_dot_function_takes_mismatching_types_of_arguments() {
    assert_eval_error("dot(1, vec2(3.0, 2.0))");
    assert_eval_error("dot(vec4(5.0, 2.0, 3.0, 1.0), vec3(4.0, 4.0, 4.0))");
}

#[test]
fn evaluates_the_cross_function() {
    assert_cartesian3(
        eval_str("cross(vec3(1.0, 1.0, 1.0), vec3(2.0, 2.0, 2.0))"),
        0.0,
        0.0,
        0.0,
    );
    assert_cartesian3(
        eval_str("cross(vec3(-1.0, -1.0, -1.0), vec3(0.0, -2.0, -5.0))"),
        3.0,
        -5.0,
        2.0,
    );
    assert_cartesian3(
        eval_str("cross(vec3(5.0, -2.0, 1.0), vec3(-2.0, -6.0, -8.0))"),
        22.0,
        38.0,
        -34.0,
    );
}

#[test]
fn throws_if_cross_function_takes_an_invalid_number_of_arguments() {
    assert_construct_error("cross(vec3(0.0, 0.0, 0.0))");
    assert_construct_error(
        "cross(vec3(0.0, 0.0, 0.0), vec3(1.0, 1.0, 1.0), vec3(2.0, 2.0, 2.0))",
    );
}

#[test]
fn throws_if_cross_function_does_not_take_vec3_arguments() {
    assert_eval_error("cross(vec2(1.0, 2.0), vec2(3.0, 2.0))");
    assert_eval_error("cross(vec4(5.0, 2.0, 3.0, 1.0), vec3(4.0, 4.0, 4.0))");
}

#[test]
fn evaluates_ternary_conditional() {
    assert_string(eval_str("true ? \"first\" : \"second\""), "first");
    assert_string(eval_str("false ? \"first\" : \"second\""), "second");
    assert_number(
        eval_str("(!(1 + 2 > 3)) ? (2 > 1 ? 1 + 1 : 0) : (2 > 1 ? -1 + -1 : 0)"),
        2.0,
    );
}

// ---------------------------------------------------------------------------
// Member expressions (object-property cases skipped per file header)
// ---------------------------------------------------------------------------

fn shader_expression(
    expression: &str,
    substitutions: &[(&str, &str)],
) -> String {
    let map: HashMap<String, String> = substitutions
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    let expression = Expression::new(expression, None);
    let mut state = ShaderState { translucent: false };
    expression
        .get_shader_expression(&map, &mut state)
        .unwrap()
        .unwrap()
}

fn shader_expression_with_state(
    expression: &str,
    substitutions: &[(&str, &str)],
) -> (String, ShaderState) {
    let map: HashMap<String, String> = substitutions
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    let expression = Expression::new(expression, None);
    let mut state = ShaderState { translucent: false };
    let result = expression
        .get_shader_expression(&map, &mut state)
        .unwrap()
        .unwrap();
    (result, state)
}

fn assert_shader_error(expression: &str) {
    let expression = Expression::new(expression, None);
    let mut state = ShaderState { translucent: false };
    assert!(
        expression
            .get_shader_expression(&HashMap::new(), &mut state)
            .is_err()
    );
}

fn member_dot_feature() -> MockFeature {
    let mut feature = MockFeature::new();
    feature.add_property("height", Value::Number(10.0));
    feature.add_property("width", Value::Number(5.0));
    feature.add_property("string", Value::String("hello".to_string()));
    feature.add_property("boolean", Value::Boolean(true));
    feature.add_property("vector", Value::Cartesian4(Cartesian4::UNIT_X));
    feature.add_property("vector.x", Value::String("something else".to_string()));
    feature.add_property("feature.vector", Value::Cartesian4(Cartesian4::UNIT_Y));
    feature.add_property("null", Value::Null);
    feature.add_property("undefined", Value::Undefined);
    feature
}

#[test]
fn evaluates_member_expression_with_dot() {
    let feature = member_dot_feature();
    assert_number(eval_with("${vector.x}", &feature), 1.0);
    assert_number(eval_with("${vector.z}", &feature), 0.0);
    assert_undefined(eval_with("${height.z}", &feature));
    assert_undefined(eval_with("${undefined.z}", &feature));
    assert_cartesian4(eval_with("${feature.vector}", &feature), 1.0, 0.0, 0.0, 0.0);
    assert_number(eval_with("${feature.vector.x}", &feature), 1.0);
}

#[test]
fn evaluates_member_expression_with_brackets() {
    let mut feature = member_dot_feature();
    feature.add_property("address.street", Value::String("Other Street".to_string()));
    assert_number(eval_with("${vector[\"x\"]}", &feature), 1.0);
    assert_number(eval_with("${vector[\"z\"]}", &feature), 0.0);
    assert_undefined(eval_with("${height[\"z\"]}", &feature));
    assert_undefined(eval_with("${undefined[\"z\"]}", &feature));
    assert_cartesian4(eval_with("${feature[\"vector\"]}", &feature), 1.0, 0.0, 0.0, 0.0);
    assert_number(eval_with("${feature.vector[\"x\"]}", &feature), 1.0);
    assert_number(eval_with("${feature[\"vector\"].x}", &feature), 1.0);
    assert_string(eval_with("${feature[\"vector.x\"]}", &feature), "something else");
    assert_cartesian4(eval_with("${feature[\"feature.vector\"]}", &feature), 0.0, 1.0, 0.0, 0.0);
}

#[test]
fn member_expressions_throw_without_variable_notation() {
    assert_construct_error("color.r");
    assert_construct_error("color[\"r\"]");
}

#[test]
fn member_expression_throws_with_variable_property() {
    assert_construct_error("${vector[${vectorName}]}");
}

// ---------------------------------------------------------------------------
// Regular expressions
// ---------------------------------------------------------------------------

fn assert_regex(value: Value, source: &str, flags: &str) {
    match value {
        Value::RegExp(actual) => {
            assert_eq!(actual.source, source);
            assert_eq!(actual.flags, flags);
        }
        other => panic!("expected regex /{source}/{flags}, got {other:?}"),
    }
}

#[test]
fn constructs_regex() {
    let mut feature = MockFeature::new();
    feature.add_property("pattern", Value::String("[abc]".to_string()));

    let expression = Expression::new("regExp(\"a\")", None);
    assert_regex(expression.evaluate(None).unwrap(), "a", "");
    assert_eq!(expression.runtime_ast().node_type, ExpressionNodeType::LiteralRegex);

    let expression = Expression::new("regExp(\"\\w\")", None);
    assert_regex(expression.evaluate(None).unwrap(), "\\w", "");
    assert_eq!(expression.runtime_ast().node_type, ExpressionNodeType::LiteralRegex);

    let expression = Expression::new("regExp(1 + 1)", None);
    assert_regex(expression.evaluate(None).unwrap(), "2", "");
    assert_eq!(expression.runtime_ast().node_type, ExpressionNodeType::Regex);

    let expression = Expression::new("regExp(true)", None);
    assert_regex(expression.evaluate(None).unwrap(), "true", "");
    assert_eq!(expression.runtime_ast().node_type, ExpressionNodeType::LiteralRegex);

    let expression = Expression::new("regExp()", None);
    assert_regex(expression.evaluate(None).unwrap(), "(?:)", "");
    assert_eq!(expression.runtime_ast().node_type, ExpressionNodeType::LiteralRegex);

    let expression = Expression::new("regExp(${pattern})", None);
    assert_regex(expression.evaluate(Some(&feature)).unwrap(), "[abc]", "");
    assert_eq!(expression.runtime_ast().node_type, ExpressionNodeType::Regex);
}

#[test]
fn constructs_regex_with_flags() {
    let expression = Expression::new("regExp(\"a\", \"i\")", None);
    assert_regex(expression.evaluate(None).unwrap(), "a", "i");
    assert_eq!(expression.runtime_ast().node_type, ExpressionNodeType::LiteralRegex);

    let expression = Expression::new("regExp(\"a\", \"m\" + \"g\")", None);
    assert_regex(expression.evaluate(None).unwrap(), "a", "mg");
    assert_eq!(expression.runtime_ast().node_type, ExpressionNodeType::Regex);
}

#[test]
fn does_not_throw_syntax_error_if_regex_constructor_has_invalid_pattern() {
    // Invalid patterns surface as RuntimeError (never a panic), mirroring the
    // JS spec which only asserts that no SyntaxError leaks out.
    let expression = Expression::new("regExp(\"(?<=\\s)\" + \".\")", None);
    assert!(expression.evaluate(None).is_err());
    assert_construct_error("regExp(\"(?<=\\s)\")");
}

#[test]
fn throws_if_regex_constructor_has_invalid_flags() {
    assert_eval_error("regExp(\"a\" + \"b\", \"q\")");
    assert_construct_error("regExp(\"a\", \"q\")");
}

#[test]
fn evaluates_regex_test_function() {
    let mut feature = MockFeature::new();
    feature.add_property("property", Value::String("abc".to_string()));

    assert_boolean(eval_str("regExp(\"a\").test(\"abc\")"), true);
    assert_boolean(eval_str("regExp(\"a\").test(\"bcd\")"), false);
    assert_boolean(
        eval_str("regExp(\"quick\\s(brown).+?(jumps)\", \"ig\").test(\"The Quick Brown Fox Jumps Over The Lazy Dog\")"),
        true,
    );
    assert_boolean(eval_str("regExp(\"a\").test()"), false);
    assert_boolean(eval_with("regExp(${property}).test(${property})", &feature), true);
}

#[test]
fn throws_if_regex_test_function_has_invalid_arguments() {
    assert_eval_error("regExp(\"1\").test(1)");
    assert_eval_error("regExp(\"a\").test(regExp(\"b\"))");
}

#[test]
fn evaluates_regex_exec_function() {
    let mut feature = MockFeature::new();
    feature.add_property("property", Value::String("abc".to_string()));
    feature.add_property("Name", Value::String("Building 1".to_string()));

    assert_string(eval_str("regExp(\"a(.)\", \"i\").exec(\"Abc\")"), "b");
    assert_null(eval_str("regExp(\"a(.)\").exec(\"qbc\")"));
    assert_null(eval_str("regExp(\"a(.)\").exec()"));
    assert_string(
        eval_str("regExp(\"quick\\s(b.*n).+?(jumps)\", \"ig\").exec(\"The Quick Brown Fox Jumps Over The Lazy Dog\")"),
        "Brown",
    );
    assert_string(
        eval_with("regExp(\"(\" + ${property} + \")\").exec(${property})", &feature),
        "abc",
    );
    assert_string(eval_with("regExp(\"Building\\s(\\d)\").exec(${Name})", &feature), "1");
}

#[test]
fn throws_if_regex_exec_function_has_invalid_arguments() {
    assert_eval_error("regExp(\"1\").exec(1)");
    assert_eval_error("regExp(\"a\").exec(regExp(\"b\"))");
}

#[test]
fn evaluates_regex_match_operator() {
    let mut feature = MockFeature::new();
    feature.add_property("property", Value::String("abc".to_string()));

    assert_boolean(eval_str("regExp(\"a\") =~ \"abc\""), true);
    assert_boolean(eval_str("\"abc\" =~ regExp(\"a\")"), true);
    assert_boolean(eval_str("regExp(\"a\") =~ \"bcd\""), false);
    assert_boolean(eval_str("\"bcd\" =~ regExp(\"a\")"), false);
    assert_boolean(
        eval_str("regExp(\"quick\\s(brown).+?(jumps)\", \"ig\") =~ \"The Quick Brown Fox Jumps Over The Lazy Dog\""),
        true,
    );
    assert_boolean(eval_with("regExp(${property}) =~ ${property}", &feature), true);
}

#[test]
fn throws_if_regex_match_operator_has_invalid_arguments() {
    assert_eval_error("regExp(\"a\") =~ 1");
    assert_eval_error("1 =~ regExp(\"a\")");
    assert_eval_error("1 =~ 1");
}

#[test]
fn evaluates_regex_not_match_operator() {
    let mut feature = MockFeature::new();
    feature.add_property("property", Value::String("abc".to_string()));

    assert_boolean(eval_str("regExp(\"a\") !~ \"abc\""), false);
    assert_boolean(eval_str("\"abc\" !~ regExp(\"a\")"), false);
    assert_boolean(eval_str("regExp(\"a\") !~ \"bcd\""), true);
    assert_boolean(eval_str("\"bcd\" !~ regExp(\"a\")"), true);
    assert_boolean(
        eval_str("regExp(\"quick\\s(brown).+?(jumps)\", \"ig\") !~ \"The Quick Brown Fox Jumps Over The Lazy Dog\""),
        false,
    );
    assert_boolean(eval_with("regExp(${property}) !~ ${property}", &feature), false);
}

#[test]
fn throws_if_regex_not_match_operator_has_invalid_arguments() {
    assert_eval_error("regExp(\"a\") !~ 1");
    assert_eval_error("1 !~ regExp(\"a\")");
    assert_eval_error("1 !~ 1");
}

#[test]
fn throws_if_test_is_not_called_with_a_regex() {
    assert_construct_error("color(\"blue\").test()");
    assert_construct_error("\"blue\".test()");
}

#[test]
fn evaluates_regex_to_string_function() {
    let mut feature = MockFeature::new();
    feature.add_property("property", Value::String("abc".to_string()));

    assert_string(eval_str("regExp().toString()"), "/(?:)/");
    assert_string(eval_str("regExp(\"\\d\\s\\d\", \"ig\").toString()"), "/\\d\\s\\d/gi");
    assert_string(eval_with("regExp(${property}).toString()", &feature), "/abc/");
}

#[test]
fn throws_when_using_to_string_on_other_type() {
    let mut feature = MockFeature::new();
    feature.add_property("property", Value::String("abc".to_string()));
    let expression = Expression::new("${property}.toString()", None);
    assert!(expression.evaluate(Some(&feature)).is_err());
}

// ---------------------------------------------------------------------------
// Arrays, built-in variables, getVariables
// ---------------------------------------------------------------------------

#[test]
fn evaluates_array_expression() {
    let mut feature = MockFeature::new();
    feature.add_property("property", Value::String("value".to_string()));
    feature.add_property(
        "array",
        Value::Array(vec![
            Value::Cartesian4(Cartesian4::UNIT_X),
            Value::Cartesian4(Cartesian4::UNIT_Y),
            Value::Cartesian4(Cartesian4::UNIT_Z),
        ]),
    );

    assert_eq!(
        eval_str("[1, 2, 3]"),
        Value::Array(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ]),
    );

    assert_eq!(
        eval_with("[1+2, \"hello\", 2 < 3, color(\"blue\"), ${property}]", &feature),
        Value::Array(vec![
            Value::Number(3.0),
            Value::String("hello".to_string()),
            Value::Boolean(true),
            Value::Cartesian4(Cartesian4::new(0.0, 0.0, 1.0, 1.0)),
            Value::String("value".to_string()),
        ]),
    );

    assert_cartesian4(eval_with("${array[1]}", &feature), 0.0, 1.0, 0.0, 0.0);
}

#[test]
fn evaluates_tiles3d_tileset_time_expression() {
    // DEVIATION: no tileset context on the CPU-side trait, so the builtin
    // always evaluates to 0.0 (see file header).
    let feature = MockFeature::new();
    assert_number(eval_with("${tiles3d_tileset_time}", &feature), 0.0);
    assert_number(eval_str("${tiles3d_tileset_time}"), 0.0);
}

#[test]
fn gets_variables() {
    let expression = Expression::new("${feature[\"w\"]} + ${feature.x} + ${y} + ${y} + \"${z}\"", None);
    let mut variables = expression.get_variables();
    variables.sort();
    assert_eq!(variables, vec!["w", "x", "y", "z"]);
}

// ---------------------------------------------------------------------------
// Shader expressions
// ---------------------------------------------------------------------------

#[test]
fn gets_shader_function() {
    let expression = Expression::new("true", None);
    let mut state = ShaderState { translucent: false };
    let shader_function = expression
        .get_shader_function("getShow()", &HashMap::new(), &mut state, "bool")
        .unwrap();
    let expected = "bool getShow()\n{\n    return true;\n}\n";
    assert_eq!(shader_function, expected);
}

#[test]
fn gets_shader_expression_for_variable() {
    assert_eq!(
        shader_expression("${property}", &[("property", "a_property")]),
        "a_property"
    );
}

#[test]
fn gets_shader_expression_for_feature_variable_with_bracket_notation() {
    assert_eq!(
        shader_expression("${feature['property']}", &[("property", "a_property")]),
        "a_property"
    );
}

#[test]
fn gets_shader_expression_for_feature_variable_with_dot_notation() {
    assert_eq!(
        shader_expression("${feature.property}", &[("property", "a_property")]),
        "a_property"
    );
}

#[test]
fn gets_shader_expression_for_non_existent_variable() {
    assert_eq!(shader_expression("${nonExistentProperty}", &[]), "czm_infinity");
}

#[test]
fn gets_shader_expression_for_unary_not() {
    assert_eq!(shader_expression("!true", &[]), "!true");
}

#[test]
fn gets_shader_expression_for_unary_negative() {
    assert_eq!(shader_expression("-5.0", &[]), "-5.0");
}

#[test]
fn gets_shader_expression_for_unary_positive() {
    assert_eq!(shader_expression("+5.0", &[]), "+5.0");
}

#[test]
fn gets_shader_expression_for_converting_to_literal_boolean() {
    assert_eq!(shader_expression("Boolean(1.0)", &[]), "bool(1.0)");
}

#[test]
fn gets_shader_expression_for_converting_to_literal_number() {
    assert_eq!(shader_expression("Number(true)", &[]), "float(true)");
}

#[test]
fn gets_shader_expression_for_binary_addition() {
    assert_eq!(shader_expression("1.0 + 2.0", &[]), "(1.0 + 2.0)");
}

#[test]
fn gets_shader_expression_for_binary_subtraction() {
    assert_eq!(shader_expression("1.0 - 2.0", &[]), "(1.0 - 2.0)");
}

#[test]
fn gets_shader_expression_for_binary_multiplication() {
    assert_eq!(shader_expression("1.0 * 2.0", &[]), "(1.0 * 2.0)");
}

#[test]
fn gets_shader_expression_for_binary_division() {
    assert_eq!(shader_expression("1.0 / 2.0", &[]), "(1.0 / 2.0)");
}

#[test]
fn gets_shader_expression_for_binary_modulus() {
    assert_eq!(shader_expression("1.0 % 2.0", &[]), "mod(1.0, 2.0)");
}

#[test]
fn gets_shader_expression_for_binary_equals_strict() {
    assert_eq!(shader_expression("1.0 === 2.0", &[]), "(1.0 == 2.0)");
}

#[test]
fn gets_shader_expression_for_binary_not_equals_strict() {
    assert_eq!(shader_expression("1.0 !== 2.0", &[]), "(1.0 != 2.0)");
}

#[test]
fn gets_shader_expression_for_binary_less_than() {
    assert_eq!(shader_expression("1.0 < 2.0", &[]), "(1.0 < 2.0)");
}

#[test]
fn gets_shader_expression_for_binary_less_than_or_equals() {
    assert_eq!(shader_expression("1.0 <= 2.0", &[]), "(1.0 <= 2.0)");
}

#[test]
fn gets_shader_expression_for_binary_greater_than() {
    assert_eq!(shader_expression("1.0 > 2.0", &[]), "(1.0 > 2.0)");
}

#[test]
fn gets_shader_expression_for_binary_greater_than_or_equals() {
    assert_eq!(shader_expression("1.0 >= 2.0", &[]), "(1.0 >= 2.0)");
}

#[test]
fn gets_shader_expression_for_logical_and() {
    assert_eq!(shader_expression("true && false", &[]), "(true && false)");
}

#[test]
fn gets_shader_expression_for_logical_or() {
    assert_eq!(shader_expression("true || false", &[]), "(true || false)");
}

#[test]
fn gets_shader_expression_for_ternary_conditional() {
    assert_eq!(shader_expression("true ? 1.0 : 2.0", &[]), "(true ? 1.0 : 2.0)");
}

#[test]
fn gets_shader_expression_for_array_indexing() {
    assert_eq!(shader_expression("${property[0]}", &[("property", "property")]), "property[0]");
    assert_eq!(
        shader_expression("${property[4 / 2]}", &[("property", "property")]),
        "property[int((4.0 / 2.0))]"
    );
}

#[test]
fn gets_shader_expression_for_array() {
    assert_eq!(shader_expression("[1.0, 2.0]", &[]), "vec2(1.0, 2.0)");
    assert_eq!(shader_expression("[1.0, 2.0, 3.0]", &[]), "vec3(1.0, 2.0, 3.0)");
    assert_eq!(
        shader_expression("[1.0, 2.0, 3.0, 4.0]", &[]),
        "vec4(1.0, 2.0, 3.0, 4.0)"
    );
}

#[test]
fn throws_when_getting_shader_expression_for_array_of_invalid_length() {
    assert_shader_error("[]");
    assert_shader_error("[1.0]");
    assert_shader_error("[1.0, 2.0, 3.0, 4.0, 5.0]");
}

#[test]
fn gets_shader_expression_for_boolean() {
    assert_eq!(shader_expression("true || false", &[]), "(true || false)");
}

#[test]
fn gets_shader_expression_for_integer() {
    assert_eq!(shader_expression("1", &[]), "1.0");
}

#[test]
fn gets_shader_expression_for_float() {
    assert_eq!(shader_expression("1.02", &[]), "1.02");
}

#[test]
fn gets_shader_expression_for_color() {
    let property = [("property", "property")];

    let (result, state) = shader_expression_with_state("color()", &property);
    assert_eq!(result, "vec4(1.0)");
    assert!(!state.translucent);

    let (result, state) = shader_expression_with_state("color(\"red\")", &property);
    assert_eq!(result, "vec4(vec3(1.0, 0.0, 0.0), 1.0)");
    assert!(!state.translucent);

    let (result, state) = shader_expression_with_state("color(\"#FFF\")", &property);
    assert_eq!(result, "vec4(vec3(1.0, 1.0, 1.0), 1.0)");
    assert!(!state.translucent);

    let (result, state) = shader_expression_with_state("color(\"#FF0000\")", &property);
    assert_eq!(result, "vec4(vec3(1.0, 0.0, 0.0), 1.0)");
    assert!(!state.translucent);

    let (result, state) = shader_expression_with_state("color(\"rgb(255, 0, 0)\")", &property);
    assert_eq!(result, "vec4(vec3(1.0, 0.0, 0.0), 1.0)");
    assert!(!state.translucent);

    let (result, state) = shader_expression_with_state("color(\"red\", 0.5)", &property);
    assert_eq!(result, "vec4(vec3(1.0, 0.0, 0.0), 0.5)");
    assert!(state.translucent);

    let (result, state) = shader_expression_with_state("rgb(255, 0, 0)", &property);
    assert_eq!(result, "vec4(1.0, 0.0, 0.0, 1.0)");
    assert!(!state.translucent);

    let (result, state) = shader_expression_with_state("rgb(255, ${property}, 0)", &property);
    assert_eq!(result, "vec4(255.0 / 255.0, property / 255.0, 0.0 / 255.0, 1.0)");
    assert!(!state.translucent);

    let (result, state) = shader_expression_with_state("rgba(255, 0, 0, 0.5)", &property);
    assert_eq!(result, "vec4(1.0, 0.0, 0.0, 0.5)");
    assert!(state.translucent);

    let (result, state) = shader_expression_with_state("rgba(255, ${property}, 0, 0.5)", &property);
    assert_eq!(result, "vec4(255.0 / 255.0, property / 255.0, 0.0 / 255.0, 0.5)");
    assert!(state.translucent);

    let (result, state) = shader_expression_with_state("hsl(1.0, 0.5, 0.5)", &property);
    assert_eq!(result, "vec4(0.75, 0.25, 0.25, 1.0)");
    assert!(!state.translucent);

    let (result, state) = shader_expression_with_state("hsla(1.0, 0.5, 0.5, 0.5)", &property);
    assert_eq!(result, "vec4(0.75, 0.25, 0.25, 0.5)");
    assert!(state.translucent);

    let (result, state) = shader_expression_with_state("hsl(1.0, ${property}, 0.5)", &property);
    assert_eq!(result, "vec4(czm_HSLToRGB(vec3(1.0, property, 0.5)), 1.0)");
    assert!(!state.translucent);

    let (result, state) = shader_expression_with_state("hsla(1.0, ${property}, 0.5, 0.5)", &property);
    assert_eq!(result, "vec4(czm_HSLToRGB(vec3(1.0, property, 0.5)), 0.5)");
    assert!(state.translucent);
}

#[test]
fn gets_shader_expression_for_color_components() {
    let expected =
        "(((vec4(1.0)[0] + vec4(1.0)[1]) + vec4(1.0)[2]) + vec4(1.0)[3])";
    assert_eq!(
        shader_expression("color().r + color().g + color().b + color().a", &[]),
        expected
    );
    assert_eq!(
        shader_expression("color().x + color().y + color().z + color().w", &[]),
        expected
    );
    assert_eq!(
        shader_expression("color()[0] + color()[1] + color()[2] + color()[3]", &[]),
        expected
    );
}

#[test]
fn gets_shader_expression_for_vector() {
    let property = [("property", "property")];
    assert_eq!(shader_expression("vec4(1, 2, 3, 4)", &property), "vec4(1.0, 2.0, 3.0, 4.0)");
    assert_eq!(shader_expression("vec4(1) + vec4(2)", &property), "(vec4(1.0) + vec4(2.0))");
    assert_eq!(
        shader_expression("vec4(1, ${property}, vec2(1, 2).x, 0)", &property),
        "vec4(1.0, property, vec2(1.0, 2.0)[0], 0.0)"
    );
    assert_eq!(shader_expression("vec4(vec3(2), 1.0)", &property), "vec4(vec3(2.0), 1.0)");
}

#[test]
fn gets_shader_expression_for_vector_components() {
    let expected =
        "(((vec4(1.0)[0] + vec4(1.0)[1]) + vec4(1.0)[2]) + vec4(1.0)[3])";
    assert_eq!(
        shader_expression("vec4(1).x + vec4(1).y + vec4(1).z + vec4(1).w", &[]),
        expected
    );
    assert_eq!(
        shader_expression("vec4(1)[0] + vec4(1)[1] + vec4(1)[2] + vec4(1)[3]", &[]),
        expected
    );
}

#[test]
fn gets_shader_expression_for_tiles3d_tileset_time() {
    assert_eq!(shader_expression("${tiles3d_tileset_time}", &[]), "tiles3d_tileset_time");
}

#[test]
fn gets_shader_expression_for_math_functions() {
    assert_eq!(shader_expression("abs(-1.0)", &[]), "abs(-1.0)");
    assert_eq!(shader_expression("cos(0.0)", &[]), "cos(0.0)");
    assert_eq!(shader_expression("sin(0.0)", &[]), "sin(0.0)");
    assert_eq!(shader_expression("tan(0.0)", &[]), "tan(0.0)");
    assert_eq!(shader_expression("acos(0.0)", &[]), "acos(0.0)");
    assert_eq!(shader_expression("asin(0.0)", &[]), "asin(0.0)");
    assert_eq!(shader_expression("atan(0.0)", &[]), "atan(0.0)");
    assert_eq!(shader_expression("sqrt(1.0)", &[]), "sqrt(1.0)");
    assert_eq!(shader_expression("sign(1.0)", &[]), "sign(1.0)");
    assert_eq!(shader_expression("floor(1.5)", &[]), "floor(1.5)");
    assert_eq!(shader_expression("ceil(1.2)", &[]), "ceil(1.2)");
    assert_eq!(shader_expression("round(1.2)", &[]), "floor(1.2 + 0.5)");
    assert_eq!(shader_expression("exp(1.0)", &[]), "exp(1.0)");
    assert_eq!(shader_expression("exp2(1.0)", &[]), "exp2(1.0)");
    assert_eq!(shader_expression("log(1.0)", &[]), "log(1.0)");
    assert_eq!(shader_expression("log2(1.0)", &[]), "log2(1.0)");
    assert_eq!(shader_expression("fract(1.0)", &[]), "fract(1.0)");
    assert_eq!(shader_expression("clamp(50.0, 0.0, 100.0)", &[]), "clamp(50.0, 0.0, 100.0)");
    assert_eq!(shader_expression("mix(0.0, 2.0, 0.5)", &[]), "mix(0.0, 2.0, 0.5)");
    assert_eq!(shader_expression("atan2(0.0,1.0)", &[]), "atan(0.0, 1.0)");
    assert_eq!(shader_expression("pow(2.0,2.0)", &[]), "pow(2.0, 2.0)");
    assert_eq!(shader_expression("min(3.0,5.0)", &[]), "min(3.0, 5.0)");
    assert_eq!(shader_expression("max(3.0,5.0)", &[]), "max(3.0, 5.0)");
    assert_eq!(shader_expression("length(3.0)", &[]), "length(3.0)");
    assert_eq!(shader_expression("normalize(3.0)", &[]), "normalize(3.0)");
    assert_eq!(shader_expression("distance(0.0, 1.0)", &[]), "distance(0.0, 1.0)");
    assert_eq!(shader_expression("dot(1.0, 2.0)", &[]), "dot(1.0, 2.0)");
    assert_eq!(
        shader_expression("cross(vec3(1.0, 1.0, 1.0), vec3(2.0, 2.0, 2.0))", &[]),
        "cross(vec3(1.0, 1.0, 1.0), vec3(2.0, 2.0, 2.0))"
    );
}

#[test]
fn gets_shader_expression_for_is_nan() {
    assert_eq!(shader_expression("isNaN(1.0)", &[]), "(1.0 != 1.0)");
}

#[test]
fn gets_shader_expression_for_is_finite() {
    assert_eq!(shader_expression("isFinite(1.0)", &[]), "(abs(1.0) < czm_infinity)");
}

#[test]
fn gets_shader_expression_for_null() {
    assert_eq!(shader_expression("null", &[]), "czm_infinity");
}

#[test]
fn gets_shader_expression_for_undefined() {
    assert_eq!(shader_expression("undefined", &[]), "czm_infinity");
}

#[test]
fn throws_when_getting_shader_expression_for_regex() {
    assert_shader_error("regExp(\"a\").test(\"abc\")");
    assert_shader_error("regExp(\"a(.)\", \"i\").exec(\"Abc\")");
    assert_shader_error("regExp(\"a\") =~ \"abc\"");
    assert_shader_error("regExp(\"a\") !~ \"abc\"");
}

#[test]
fn throws_when_getting_shader_expression_for_member_expression_with_dot() {
    assert_shader_error("${property.name}");
}

#[test]
fn throws_when_getting_shader_expression_for_string_member_expression_with_brackets() {
    assert_shader_error("${property[\"name\"]}");
}

#[test]
fn throws_when_getting_shader_expression_for_string_conversion() {
    assert_shader_error("String(1.0)");
}

#[test]
fn throws_when_getting_shader_expression_for_to_string() {
    assert_shader_error("color(\"red\").toString()");
}

#[test]
fn throws_when_getting_shader_expression_for_literal_string() {
    assert_shader_error("\"name\"");
}

#[test]
fn throws_when_getting_shader_expression_for_variable_in_string() {
    assert_shader_error("\"${property}\"");
}

#[test]
fn throws_when_getting_shader_expression_for_is_exact_class() {
    assert_shader_error("isExactClass(\"door\")");
}

#[test]
fn throws_when_getting_shader_expression_for_is_class() {
    assert_shader_error("isClass(\"door\")");
}

#[test]
fn throws_when_getting_shader_expression_for_get_exact_class_name() {
    assert_shader_error("getExactClassName()");
}
