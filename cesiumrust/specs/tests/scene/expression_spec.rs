//! Scene/ExpressionSpec.js + ConditionsExpressionSpec.js + Cesium3DTileStyleSpec.js
//! → Rust integration tests for tileset styling expression system.
//!
//! CesiumJS ExpressionSpec.js (4235 lines, 15 it() with many assertions each)
//! uses a full JavaScript expression parser. The Rust implementation
//! (cesium_tileset::styling) supports a subset: property refs, comparisons,
//! arithmetic, logical ops, unary ops, function calls (color/rgb/rgba/abs/sqrt/
//! min/max/clamp), and conditions expressions.
//!
//! A-class tests ported: expression parsing + evaluation + conditions + TileStyle.
//! C-class omitted: RegExp, member access (vec.x), template strings with
//! interpolation, hsl/hsla, Boolean()/Number()/String() constructors,
//! shader generation, getVariables, result-parameter variants.

use cesium_tileset::styling::{
    ConditionsExpression, EvalResult, Expression, StyleExpression, TileStyle,
};
use serde_json::json;
use std::collections::HashMap;

fn props(pairs: Vec<(&str, serde_json::Value)>) -> HashMap<String, serde_json::Value> {
    pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

fn empty() -> HashMap<String, serde_json::Value> {
    HashMap::new()
}

// === Expression: Literal Parsing ===

#[test]
fn expression_parses_boolean_true() {
    let expr = Expression::parse("true");
    assert_eq!(expr.evaluate(&empty()), EvalResult::Bool(true));
}

#[test]
fn expression_parses_boolean_false() {
    let expr = Expression::parse("false");
    assert_eq!(expr.evaluate(&empty()), EvalResult::Bool(false));
}

#[test]
fn expression_parses_integer_number() {
    let expr = Expression::parse("42");
    assert_eq!(expr.evaluate(&empty()), EvalResult::Number(42.0));
}

#[test]
fn expression_parses_float_number() {
    let expr = Expression::parse("3.14");
    assert_eq!(expr.evaluate(&empty()), EvalResult::Number(3.14));
}

#[test]
fn expression_parses_negative_number() {
    // Negative numbers: parsed as unary negate or as part of arithmetic
    let expr = Expression::parse("-5.0");
    // The parser may treat this as UnaryOp(Negate, 5.0) or NumberConstant(-5.0)
    let result = expr.evaluate(&empty());
    assert_eq!(result, EvalResult::Number(-5.0));
}

#[test]
fn expression_parses_string_single_quotes() {
    let expr = Expression::parse("'hello'");
    assert_eq!(expr.evaluate(&empty()), EvalResult::String("hello".to_string()));
}

#[test]
fn expression_parses_string_double_quotes() {
    let expr = Expression::parse("\"Cesium\"");
    assert_eq!(expr.evaluate(&empty()), EvalResult::String("Cesium".to_string()));
}

// === Expression: Property References ===

#[test]
fn expression_evaluates_property_ref_number() {
    let expr = Expression::parse("${Height}");
    let p = props(vec![("Height", json!(100.0))]);
    assert_eq!(expr.evaluate(&p), EvalResult::Number(100.0));
}

#[test]
fn expression_evaluates_property_ref_string() {
    let expr = Expression::parse("${name}");
    let p = props(vec![("name", json!("building"))]);
    assert_eq!(expr.evaluate(&p), EvalResult::String("building".to_string()));
}

#[test]
fn expression_evaluates_property_ref_boolean() {
    let expr = Expression::parse("${visible}");
    let p = props(vec![("visible", json!(true))]);
    assert_eq!(expr.evaluate(&p), EvalResult::Bool(true));
}

#[test]
fn expression_property_ref_missing_returns_zero() {
    // CesiumJS returns undefined; Rust returns Number(0.0) as default
    let expr = Expression::parse("${missing}");
    assert_eq!(expr.evaluate(&empty()), EvalResult::Number(0.0));
}

// === Expression: Comparisons ===

#[test]
fn expression_comparison_greater_than() {
    let expr = Expression::parse("${Height} > 100");
    assert_eq!(expr.evaluate(&props(vec![("Height", json!(150.0))])), EvalResult::Bool(true));
    assert_eq!(expr.evaluate(&props(vec![("Height", json!(50.0))])), EvalResult::Bool(false));
}

#[test]
fn expression_comparison_greater_than_or_equal() {
    let expr = Expression::parse("${Height} >= 100");
    assert_eq!(expr.evaluate(&props(vec![("Height", json!(100.0))])), EvalResult::Bool(true));
    assert_eq!(expr.evaluate(&props(vec![("Height", json!(99.0))])), EvalResult::Bool(false));
}

#[test]
fn expression_comparison_less_than() {
    let expr = Expression::parse("${Height} < 50");
    assert_eq!(expr.evaluate(&props(vec![("Height", json!(30.0))])), EvalResult::Bool(true));
    assert_eq!(expr.evaluate(&props(vec![("Height", json!(60.0))])), EvalResult::Bool(false));
}

#[test]
fn expression_comparison_less_than_or_equal() {
    let expr = Expression::parse("${Height} <= 50");
    assert_eq!(expr.evaluate(&props(vec![("Height", json!(50.0))])), EvalResult::Bool(true));
    assert_eq!(expr.evaluate(&props(vec![("Height", json!(51.0))])), EvalResult::Bool(false));
}

#[test]
fn expression_comparison_equal() {
    let expr = Expression::parse("${Type} == 3");
    assert_eq!(expr.evaluate(&props(vec![("Type", json!(3.0))])), EvalResult::Bool(true));
    assert_eq!(expr.evaluate(&props(vec![("Type", json!(4.0))])), EvalResult::Bool(false));
}

#[test]
fn expression_comparison_not_equal() {
    let expr = Expression::parse("${Type} != 3");
    assert_eq!(expr.evaluate(&props(vec![("Type", json!(4.0))])), EvalResult::Bool(true));
    assert_eq!(expr.evaluate(&props(vec![("Type", json!(3.0))])), EvalResult::Bool(false));
}

// === Expression: Arithmetic ===

#[test]
fn expression_arithmetic_addition() {
    let expr = Expression::parse("${a} + ${b}");
    let p = props(vec![("a", json!(10.0)), ("b", json!(5.0))]);
    assert_eq!(expr.evaluate(&p), EvalResult::Number(15.0));
}

#[test]
fn expression_arithmetic_subtraction() {
    let expr = Expression::parse("${a} - ${b}");
    let p = props(vec![("a", json!(10.0)), ("b", json!(3.0))]);
    assert_eq!(expr.evaluate(&p), EvalResult::Number(7.0));
}

#[test]
fn expression_arithmetic_multiplication() {
    let expr = Expression::parse("${Height} * 2.0");
    let p = props(vec![("Height", json!(50.0))]);
    assert_eq!(expr.evaluate(&p), EvalResult::Number(100.0));
}

#[test]
fn expression_arithmetic_division() {
    let expr = Expression::parse("${Height} / 2");
    let p = props(vec![("Height", json!(100.0))]);
    assert_eq!(expr.evaluate(&p), EvalResult::Number(50.0));
}

#[test]
fn expression_arithmetic_division_by_zero() {
    // CesiumJS returns Infinity; Rust returns 0.0 (safe default)
    let expr = Expression::parse("${a} / ${b}");
    let p = props(vec![("a", json!(10.0)), ("b", json!(0.0))]);
    assert_eq!(expr.evaluate(&p), EvalResult::Number(0.0));
}

#[test]
fn expression_arithmetic_modulo_not_parsed() {
    // NOTE: The Rust parser does not support '%' as an operator (unlike CesiumJS).
    // BinaryOperator::Mod exists but is only reachable via direct AST construction.
    // Parsing "${a} % ${b}" falls through to StringConstant.
    let expr = Expression::parse("${a} % ${b}");
    let p = props(vec![("a", json!(10.0)), ("b", json!(3.0))]);
    // Falls back to string since % is not a recognized operator
    let result = expr.evaluate(&p);
    assert!(matches!(result, EvalResult::String(_)));
}

// === Expression: Logical Operators ===

#[test]
fn expression_logical_and_both_true() {
    let expr = Expression::parse("${A} && ${B}");
    let p = props(vec![("A", json!(true)), ("B", json!(true))]);
    assert_eq!(expr.evaluate(&p), EvalResult::Bool(true));
}

#[test]
fn expression_logical_and_one_false() {
    let expr = Expression::parse("${A} && ${B}");
    let p = props(vec![("A", json!(true)), ("B", json!(false))]);
    assert_eq!(expr.evaluate(&p), EvalResult::Bool(false));
}

#[test]
fn expression_logical_or() {
    let expr = Expression::parse("${A} || ${B}");
    let p_false = props(vec![("A", json!(false)), ("B", json!(false))]);
    let p_one_true = props(vec![("A", json!(false)), ("B", json!(true))]);
    assert_eq!(expr.evaluate(&p_false), EvalResult::Bool(false));
    assert_eq!(expr.evaluate(&p_one_true), EvalResult::Bool(true));
}

// === Expression: Unary Operators ===

#[test]
fn expression_unary_not() {
    let expr = Expression::parse("!${visible}");
    assert_eq!(expr.evaluate(&props(vec![("visible", json!(true))])), EvalResult::Bool(false));
    assert_eq!(expr.evaluate(&props(vec![("visible", json!(false))])), EvalResult::Bool(true));
}

// === Expression: Function Calls ===

#[test]
fn expression_color_by_name_red() {
    let expr = Expression::parse("color('red')");
    assert_eq!(expr.evaluate(&empty()), EvalResult::Color([1.0, 0.0, 0.0, 1.0]));
}

#[test]
fn expression_color_by_name_blue() {
    let expr = Expression::parse("color('blue')");
    assert_eq!(expr.evaluate(&empty()), EvalResult::Color([0.0, 0.0, 1.0, 1.0]));
}

#[test]
fn expression_color_by_name_white() {
    let expr = Expression::parse("color('white')");
    assert_eq!(expr.evaluate(&empty()), EvalResult::Color([1.0, 1.0, 1.0, 1.0]));
}

#[test]
fn expression_color_by_name_lime() {
    let expr = Expression::parse("color('lime')");
    assert_eq!(expr.evaluate(&empty()), EvalResult::Color([0.0, 1.0, 0.0, 1.0]));
}

#[test]
fn expression_color_with_alpha() {
    let expr = Expression::parse("color('white', 0.5)");
    assert_eq!(expr.evaluate(&empty()), EvalResult::Color([1.0, 1.0, 1.0, 0.5]));
}

#[test]
fn expression_color_rgba_components() {
    let expr = Expression::parse("color(1.0, 0.5, 0.0, 1.0)");
    assert_eq!(expr.evaluate(&empty()), EvalResult::Color([1.0, 0.5, 0.0, 1.0]));
}

#[test]
fn expression_color_no_args_returns_white() {
    let expr = Expression::parse("color()");
    assert_eq!(expr.evaluate(&empty()), EvalResult::Color([1.0, 1.0, 1.0, 1.0]));
}

#[test]
fn expression_rgb_function() {
    let expr = Expression::parse("rgb(255, 0, 0)");
    let result = expr.evaluate(&empty());
    if let EvalResult::Color(c) = result {
        assert!((c[0] - 1.0).abs() < 0.01);
        assert!((c[1] - 0.0).abs() < 0.01);
        assert!((c[2] - 0.0).abs() < 0.01);
        assert!((c[3] - 1.0).abs() < 0.01);
    } else {
        panic!("Expected Color, got {:?}", result);
    }
}

#[test]
fn expression_rgba_function() {
    let expr = Expression::parse("rgba(255, 255, 255, 0.5)");
    let result = expr.evaluate(&empty());
    if let EvalResult::Color(c) = result {
        assert!((c[0] - 1.0).abs() < 0.01);
        assert!((c[1] - 1.0).abs() < 0.01);
        assert!((c[2] - 1.0).abs() < 0.01);
        assert!((c[3] - 0.5).abs() < 0.01);
    } else {
        panic!("Expected Color, got {:?}", result);
    }
}

#[test]
fn expression_abs_function() {
    let expr = Expression::parse("abs(${Value})");
    let p = props(vec![("Value", json!(-5.0))]);
    assert_eq!(expr.evaluate(&p), EvalResult::Number(5.0));
}

#[test]
fn expression_sqrt_function() {
    let expr = Expression::parse("sqrt(16.0)");
    assert_eq!(expr.evaluate(&empty()), EvalResult::Number(4.0));
}

#[test]
fn expression_min_function() {
    let expr = Expression::parse("min(${a}, ${b})");
    let p = props(vec![("a", json!(3.0)), ("b", json!(7.0))]);
    assert_eq!(expr.evaluate(&p), EvalResult::Number(3.0));
}

#[test]
fn expression_max_function() {
    let expr = Expression::parse("max(${a}, ${b})");
    let p = props(vec![("a", json!(3.0)), ("b", json!(7.0))]);
    assert_eq!(expr.evaluate(&p), EvalResult::Number(7.0));
}

#[test]
fn expression_clamp_function() {
    let expr = Expression::parse("clamp(${Value}, 0.0, 10.0)");
    let p = props(vec![("Value", json!(15.0))]);
    assert_eq!(expr.evaluate(&p), EvalResult::Number(10.0));

    let p2 = props(vec![("Value", json!(-5.0))]);
    assert_eq!(expr.evaluate(&p2), EvalResult::Number(0.0));
}

// === Expression: Combined expressions ===

#[test]
fn expression_combined_comparison_and_arithmetic() {
    // ${Height} / 2 > 50
    let expr = Expression::parse("${Height} / 2 > 50");
    let p = props(vec![("Height", json!(200.0))]);
    assert_eq!(expr.evaluate(&p), EvalResult::Bool(true));
}

#[test]
fn expression_nested_function_with_property() {
    // abs(${Height} - 100)
    let expr = Expression::parse("abs(${Height} - 100)");
    let p = props(vec![("Height", json!(80.0))]);
    assert_eq!(expr.evaluate(&p), EvalResult::Number(20.0));
}

// === EvalResult conversions ===

#[test]
fn eval_result_as_bool_truthy() {
    assert!(EvalResult::Bool(true).as_bool());
    assert!(EvalResult::Number(1.0).as_bool());
    assert!(EvalResult::Number(-1.0).as_bool());
    assert!(EvalResult::String("hello".to_string()).as_bool());
    assert!(EvalResult::Color([1.0, 0.0, 0.0, 1.0]).as_bool());
}

#[test]
fn eval_result_as_bool_falsy() {
    assert!(!EvalResult::Bool(false).as_bool());
    assert!(!EvalResult::Number(0.0).as_bool());
    assert!(!EvalResult::String(String::new()).as_bool());
}

#[test]
fn eval_result_as_number() {
    assert_eq!(EvalResult::Number(42.0).as_number(), 42.0);
    assert_eq!(EvalResult::Bool(true).as_number(), 1.0);
    assert_eq!(EvalResult::Bool(false).as_number(), 0.0);
}

#[test]
fn eval_result_as_color() {
    assert_eq!(
        EvalResult::Color([0.5, 0.5, 0.5, 1.0]).as_color(),
        [0.5, 0.5, 0.5, 1.0]
    );
    // Number -> grayscale
    assert_eq!(EvalResult::Number(0.5).as_color(), [0.5, 0.5, 0.5, 1.0]);
}

// === ConditionsExpression ===

#[test]
fn conditions_expression_evaluates_first_match() {
    // Maps to ConditionsExpressionSpec "evaluates conditional"
    let json_val = json!({
        "conditions": [
            ["${Height} > 100", "color('blue')"],
            ["${Height} > 50", "color('red')"],
            ["true", "color('lime')"]
        ]
    });
    let conds = ConditionsExpression::from_json(&json_val).unwrap();

    assert_eq!(
        conds.evaluate(&props(vec![("Height", json!(101.0))])),
        EvalResult::Color([0.0, 0.0, 1.0, 1.0])
    );
    assert_eq!(
        conds.evaluate(&props(vec![("Height", json!(52.0))])),
        EvalResult::Color([1.0, 0.0, 0.0, 1.0])
    );
    assert_eq!(
        conds.evaluate(&props(vec![("Height", json!(3.0))])),
        EvalResult::Color([0.0, 1.0, 0.0, 1.0])
    );
}

#[test]
fn conditions_expression_empty_conditions() {
    // Maps to ConditionsExpressionSpec "constructs and evaluates empty conditional"
    let json_val = json!({ "conditions": [] });
    let conds = ConditionsExpression::from_json(&json_val).unwrap();
    // No conditions match -> default white
    assert_eq!(
        conds.evaluate(&props(vec![("Height", json!(101.0))])),
        EvalResult::Color([1.0, 1.0, 1.0, 1.0])
    );
}

#[test]
fn conditions_expression_with_arithmetic_condition() {
    let json_val = json!({
        "conditions": [
            ["${Height} * 2 > 100", "color('red')"],
            ["true", "color('blue')"]
        ]
    });
    let conds = ConditionsExpression::from_json(&json_val).unwrap();

    // Height=60 -> 60*2=120 > 100 -> red
    assert_eq!(
        conds.evaluate(&props(vec![("Height", json!(60.0))])),
        EvalResult::Color([1.0, 0.0, 0.0, 1.0])
    );
    // Height=40 -> 40*2=80 < 100 -> blue
    assert_eq!(
        conds.evaluate(&props(vec![("Height", json!(40.0))])),
        EvalResult::Color([0.0, 0.0, 1.0, 1.0])
    );
}

// === StyleExpression ===

#[test]
fn style_expression_from_json_string() {
    let json_val = json!("${Height} > 50");
    let expr = StyleExpression::from_json(&json_val).unwrap();
    assert!(expr.evaluate(&props(vec![("Height", json!(100.0))])).as_bool());
    assert!(!expr.evaluate(&props(vec![("Height", json!(30.0))])).as_bool());
}

#[test]
fn style_expression_from_json_bool() {
    let json_val = json!(true);
    let expr = StyleExpression::from_json(&json_val).unwrap();
    assert_eq!(expr.evaluate(&empty()), EvalResult::Bool(true));
}

#[test]
fn style_expression_from_json_number() {
    let json_val = json!(2.5);
    let expr = StyleExpression::from_json(&json_val).unwrap();
    assert_eq!(expr.evaluate(&empty()), EvalResult::Number(2.5));
}

#[test]
fn style_expression_from_json_conditions() {
    let json_val = json!({
        "conditions": [
            ["${Type} == 1", "color('red')"],
            ["true", "color('white')"]
        ]
    });
    let expr = StyleExpression::from_json(&json_val).unwrap();
    assert_eq!(
        expr.evaluate(&props(vec![("Type", json!(1.0))])),
        EvalResult::Color([1.0, 0.0, 0.0, 1.0])
    );
    assert_eq!(
        expr.evaluate(&props(vec![("Type", json!(2.0))])),
        EvalResult::Color([1.0, 1.0, 1.0, 1.0])
    );
}

// === TileStyle ===

#[test]
fn tile_style_from_json_show_expression() {
    // Maps to Cesium3DTileStyleSpec "sets show value to expression"
    let json_val = json!({
        "show": "${Height} > 0"
    });
    let style = TileStyle::from_json(&json_val);
    assert!(style.evaluate_show(&props(vec![("Height", json!(50.0))])));
    assert!(!style.evaluate_show(&props(vec![("Height", json!(0.0))])));
}

#[test]
fn tile_style_from_json_show_default_true() {
    // No show expression -> default true
    let json_val = json!({});
    let style = TileStyle::from_json(&json_val);
    assert!(style.evaluate_show(&empty()));
}

#[test]
fn tile_style_from_json_color_conditions() {
    // Maps to Cesium3DTileStyleSpec "sets color value to conditional"
    let json_val = json!({
        "color": {
            "conditions": [
                ["${Height} >= 100", "color('red')"],
                ["${Height} >= 50", "color('yellow')"],
                ["true", "color('blue')"]
            ]
        }
    });
    let style = TileStyle::from_json(&json_val);

    assert_eq!(
        style.evaluate_color(&props(vec![("Height", json!(150.0))])),
        [1.0, 0.0, 0.0, 1.0]
    );
    assert_eq!(
        style.evaluate_color(&props(vec![("Height", json!(75.0))])),
        [1.0, 1.0, 0.0, 1.0]
    );
    assert_eq!(
        style.evaluate_color(&props(vec![("Height", json!(25.0))])),
        [0.0, 0.0, 1.0, 1.0]
    );
}

#[test]
fn tile_style_from_json_color_default_white() {
    let json_val = json!({});
    let style = TileStyle::from_json(&json_val);
    assert_eq!(style.evaluate_color(&empty()), [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn tile_style_from_json_point_size() {
    let json_val = json!({ "pointSize": 5.0 });
    let style = TileStyle::from_json(&json_val);
    assert_eq!(style.evaluate_point_size(&empty()), 5.0);
}

#[test]
fn tile_style_from_json_point_size_default() {
    let json_val = json!({});
    let style = TileStyle::from_json(&json_val);
    assert_eq!(style.evaluate_point_size(&empty()), 1.0);
}

#[test]
fn tile_style_from_json_point_size_expression() {
    let json_val = json!({ "pointSize": "${Size}" });
    let style = TileStyle::from_json(&json_val);
    assert_eq!(style.evaluate_point_size(&props(vec![("Size", json!(3.0))])), 3.0);
}

#[test]
fn tile_style_from_json_meta() {
    let json_val = json!({
        "meta": {
            "description": "${name}"
        }
    });
    let style = TileStyle::from_json(&json_val);
    let result = style.evaluate_meta("description", &props(vec![("name", json!("Tower"))]));
    assert!(result.is_some());
    assert_eq!(result.unwrap(), EvalResult::String("Tower".to_string()));
}

#[test]
fn tile_style_from_json_color_with_alpha() {
    // Maps to Cesium3DTileStyleSpec "sets color value to expression" with alpha
    let json_val = json!({
        "color": "color('purple', 0.5)"
    });
    let style = TileStyle::from_json(&json_val);
    let color = style.evaluate_color(&empty());
    // purple = [0.502, 0.0, 0.502], alpha = 0.5
    assert!((color[0] - 0.502).abs() < 0.01);
    assert!((color[1] - 0.0).abs() < 0.01);
    assert!((color[2] - 0.502).abs() < 0.01);
    assert!((color[3] - 0.5).abs() < 0.01);
}

#[test]
fn tile_style_combined_show_and_color() {
    // Full style with show + color + pointSize
    let json_val = json!({
        "show": "${Height} > 0",
        "color": {
            "conditions": [
                ["${Height} >= 100", "color('red')"],
                ["true", "color('blue')"]
            ]
        },
        "pointSize": 2.0
    });
    let style = TileStyle::from_json(&json_val);
    let p = props(vec![("Height", json!(150.0))]);

    assert!(style.evaluate_show(&p));
    assert_eq!(style.evaluate_color(&p), [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(style.evaluate_point_size(&p), 2.0);
}
