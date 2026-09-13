//! TileStyle / StyleExpression extended specs - ported from Cesium3DTileStyleSpec.js
//! and ExpressionSpec.js
//!
//! Tests all comparison operators, arithmetic operators, built-in functions,
//! CSS/hex color parsing, property references, truthiness, conditionals,
//! and TileStyle evaluation.

use cesium_styling::{
    ArithmeticOp, CompareOp, PropertyValue, StyleExpression, TileStyle,
};
use std::collections::HashMap;

fn empty_props() -> HashMap<String, PropertyValue> {
    HashMap::new()
}

fn props_with_height(h: f64) -> HashMap<String, PropertyValue> {
    let mut m = HashMap::new();
    m.insert("height".to_string(), PropertyValue::Number(h));
    m
}

// ─── Comparison Operators ──────────────────────────────────────────────────

#[test]
fn compare_equal_true() {
    let expr = StyleExpression::Compare {
        left: Box::new(StyleExpression::Number(42.0)),
        op: CompareOp::Equal,
        right: Box::new(StyleExpression::Number(42.0)),
    };
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Bool(true));
}

#[test]
fn compare_equal_false() {
    let expr = StyleExpression::Compare {
        left: Box::new(StyleExpression::Number(42.0)),
        op: CompareOp::Equal,
        right: Box::new(StyleExpression::Number(43.0)),
    };
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Bool(false));
}

#[test]
fn compare_not_equal() {
    let expr = StyleExpression::Compare {
        left: Box::new(StyleExpression::Number(1.0)),
        op: CompareOp::NotEqual,
        right: Box::new(StyleExpression::Number(2.0)),
    };
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Bool(true));
}

#[test]
fn compare_less_than() {
    let expr = StyleExpression::Compare {
        left: Box::new(StyleExpression::Number(5.0)),
        op: CompareOp::LessThan,
        right: Box::new(StyleExpression::Number(10.0)),
    };
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Bool(true));
}

#[test]
fn compare_less_than_or_equal() {
    let expr = StyleExpression::Compare {
        left: Box::new(StyleExpression::Number(10.0)),
        op: CompareOp::LessThanOrEqual,
        right: Box::new(StyleExpression::Number(10.0)),
    };
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Bool(true));
}

#[test]
fn compare_greater_than() {
    let expr = StyleExpression::Compare {
        left: Box::new(StyleExpression::Number(20.0)),
        op: CompareOp::GreaterThan,
        right: Box::new(StyleExpression::Number(10.0)),
    };
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Bool(true));
}

#[test]
fn compare_greater_than_or_equal() {
    let expr = StyleExpression::Compare {
        left: Box::new(StyleExpression::Number(10.0)),
        op: CompareOp::GreaterThanOrEqual,
        right: Box::new(StyleExpression::Number(10.0)),
    };
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Bool(true));
}

#[test]
fn compare_strings_equal() {
    let expr = StyleExpression::Compare {
        left: Box::new(StyleExpression::String("building".to_string())),
        op: CompareOp::Equal,
        right: Box::new(StyleExpression::String("building".to_string())),
    };
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Bool(true));
}

#[test]
fn compare_strings_not_equal() {
    let expr = StyleExpression::Compare {
        left: Box::new(StyleExpression::String("road".to_string())),
        op: CompareOp::NotEqual,
        right: Box::new(StyleExpression::String("building".to_string())),
    };
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Bool(true));
}

#[test]
fn compare_bools() {
    let expr = StyleExpression::Compare {
        left: Box::new(StyleExpression::Bool(true)),
        op: CompareOp::Equal,
        right: Box::new(StyleExpression::Bool(true)),
    };
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Bool(true));
}

// ─── Arithmetic Operators ──────────────────────────────────────────────────

#[test]
fn arithmetic_add() {
    let expr = StyleExpression::Arithmetic {
        left: Box::new(StyleExpression::Number(3.0)),
        op: ArithmeticOp::Add,
        right: Box::new(StyleExpression::Number(7.0)),
    };
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Number(10.0));
}

#[test]
fn arithmetic_subtract() {
    let expr = StyleExpression::Arithmetic {
        left: Box::new(StyleExpression::Number(10.0)),
        op: ArithmeticOp::Subtract,
        right: Box::new(StyleExpression::Number(4.0)),
    };
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Number(6.0));
}

#[test]
fn arithmetic_multiply() {
    let expr = StyleExpression::Arithmetic {
        left: Box::new(StyleExpression::Number(6.0)),
        op: ArithmeticOp::Multiply,
        right: Box::new(StyleExpression::Number(7.0)),
    };
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Number(42.0));
}

#[test]
fn arithmetic_divide() {
    let expr = StyleExpression::Arithmetic {
        left: Box::new(StyleExpression::Number(20.0)),
        op: ArithmeticOp::Divide,
        right: Box::new(StyleExpression::Number(4.0)),
    };
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Number(5.0));
}

#[test]
fn arithmetic_divide_by_zero() {
    let expr = StyleExpression::Arithmetic {
        left: Box::new(StyleExpression::Number(10.0)),
        op: ArithmeticOp::Divide,
        right: Box::new(StyleExpression::Number(0.0)),
    };
    // Division by zero returns 0.0
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Number(0.0));
}

#[test]
fn arithmetic_modulo() {
    let expr = StyleExpression::Arithmetic {
        left: Box::new(StyleExpression::Number(17.0)),
        op: ArithmeticOp::Modulo,
        right: Box::new(StyleExpression::Number(5.0)),
    };
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Number(2.0));
}

// ─── Built-in Functions ────────────────────────────────────────────────────

#[test]
fn function_abs() {
    let expr = StyleExpression::Function {
        name: "abs".to_string(),
        args: vec![StyleExpression::Number(-42.0)],
    };
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Number(42.0));
}

#[test]
fn function_sqrt() {
    let expr = StyleExpression::Function {
        name: "sqrt".to_string(),
        args: vec![StyleExpression::Number(144.0)],
    };
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Number(12.0));
}

#[test]
fn function_min() {
    let expr = StyleExpression::Function {
        name: "min".to_string(),
        args: vec![StyleExpression::Number(3.0), StyleExpression::Number(7.0)],
    };
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Number(3.0));
}

#[test]
fn function_max() {
    let expr = StyleExpression::Function {
        name: "max".to_string(),
        args: vec![StyleExpression::Number(3.0), StyleExpression::Number(7.0)],
    };
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Number(7.0));
}

#[test]
fn function_clamp() {
    let expr = StyleExpression::Function {
        name: "clamp".to_string(),
        args: vec![
            StyleExpression::Number(-5.0),
            StyleExpression::Number(0.0),
            StyleExpression::Number(100.0),
        ],
    };
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Number(0.0));
}

#[test]
fn function_rgba() {
    let expr = StyleExpression::Function {
        name: "rgba".to_string(),
        args: vec![
            StyleExpression::Number(255.0),
            StyleExpression::Number(0.0),
            StyleExpression::Number(128.0),
            StyleExpression::Number(0.5),
        ],
    };
    if let PropertyValue::Color(c) = expr.evaluate(&empty_props()) {
        assert!((c[0] - 1.0).abs() < 0.01);
        assert!((c[1] - 0.0).abs() < 0.01);
        assert!((c[2] - 128.0 / 255.0).abs() < 0.01);
        assert!((c[3] - 0.5).abs() < 0.01);
    } else {
        panic!("Expected Color");
    }
}

#[test]
fn function_color_rgba_components() {
    let expr = StyleExpression::Function {
        name: "color".to_string(),
        args: vec![
            StyleExpression::Number(0.2),
            StyleExpression::Number(0.4),
            StyleExpression::Number(0.6),
            StyleExpression::Number(0.8),
        ],
    };
    if let PropertyValue::Color(c) = expr.evaluate(&empty_props()) {
        assert!((c[0] - 0.2).abs() < 1e-10);
        assert!((c[1] - 0.4).abs() < 1e-10);
        assert!((c[2] - 0.6).abs() < 1e-10);
        assert!((c[3] - 0.8).abs() < 1e-10);
    } else {
        panic!("Expected Color");
    }
}

// ─── Color Parsing ─────────────────────────────────────────────────────────

#[test]
fn css_color_named() {
    let expr = StyleExpression::Function {
        name: "color".to_string(),
        args: vec![StyleExpression::String("blue".to_string())],
    };
    assert_eq!(
        expr.evaluate(&empty_props()),
        PropertyValue::Color([0.0, 0.0, 1.0, 1.0])
    );
}

#[test]
fn css_color_hex6() {
    let expr = StyleExpression::Function {
        name: "color".to_string(),
        args: vec![StyleExpression::String("#00FF00".to_string())],
    };
    if let PropertyValue::Color(c) = expr.evaluate(&empty_props()) {
        assert!((c[0] - 0.0).abs() < 0.01);
        assert!((c[1] - 1.0).abs() < 0.01);
        assert!((c[2] - 0.0).abs() < 0.01);
        assert!((c[3] - 1.0).abs() < 0.01);
    } else {
        panic!("Expected Color");
    }
}

#[test]
fn css_color_hex8_with_alpha() {
    let expr = StyleExpression::Function {
        name: "color".to_string(),
        args: vec![StyleExpression::String("#FF000080".to_string())],
    };
    if let PropertyValue::Color(c) = expr.evaluate(&empty_props()) {
        assert!((c[0] - 1.0).abs() < 0.01);
        assert!((c[1] - 0.0).abs() < 0.01);
        assert!((c[2] - 0.0).abs() < 0.01);
        assert!((c[3] - 128.0 / 255.0).abs() < 0.01);
    } else {
        panic!("Expected Color");
    }
}

// ─── Property References & Truthiness ─────────────────────────────────────

#[test]
fn property_missing_defaults_to_zero() {
    let expr = StyleExpression::Property("nonexistent".to_string());
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Number(0.0));
}

#[test]
fn property_reference_number() {
    let props = props_with_height(75.0);
    let expr = StyleExpression::Property("height".to_string());
    assert_eq!(expr.evaluate(&props), PropertyValue::Number(75.0));
}

#[test]
fn truthiness_number_zero_is_false() {
    // Conditional with number 0 as condition → false branch
    let expr = StyleExpression::Conditional {
        condition: Box::new(StyleExpression::Number(0.0)),
        true_expr: Box::new(StyleExpression::String("yes".to_string())),
        false_expr: Box::new(StyleExpression::String("no".to_string())),
    };
    assert_eq!(
        expr.evaluate(&empty_props()),
        PropertyValue::String("no".to_string())
    );
}

#[test]
fn truthiness_nonzero_number_is_true() {
    let expr = StyleExpression::Conditional {
        condition: Box::new(StyleExpression::Number(42.0)),
        true_expr: Box::new(StyleExpression::String("yes".to_string())),
        false_expr: Box::new(StyleExpression::String("no".to_string())),
    };
    assert_eq!(
        expr.evaluate(&empty_props()),
        PropertyValue::String("yes".to_string())
    );
}

#[test]
fn truthiness_empty_string_is_false() {
    let expr = StyleExpression::Conditional {
        condition: Box::new(StyleExpression::String("".to_string())),
        true_expr: Box::new(StyleExpression::Number(1.0)),
        false_expr: Box::new(StyleExpression::Number(0.0)),
    };
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Number(0.0));
}

// ─── Logical Operators ─────────────────────────────────────────────────────

#[test]
fn logical_and_both_true() {
    let expr = StyleExpression::And(
        Box::new(StyleExpression::Bool(true)),
        Box::new(StyleExpression::Bool(true)),
    );
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Bool(true));
}

#[test]
fn logical_or_both_false() {
    let expr = StyleExpression::Or(
        Box::new(StyleExpression::Bool(false)),
        Box::new(StyleExpression::Bool(false)),
    );
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Bool(false));
}

#[test]
fn logical_not_false() {
    let expr = StyleExpression::Not(Box::new(StyleExpression::Bool(false)));
    assert_eq!(expr.evaluate(&empty_props()), PropertyValue::Bool(true));
}

// ─── TileStyle Evaluation ──────────────────────────────────────────────────

#[test]
fn tile_style_default_show_true() {
    let style = TileStyle::new();
    assert!(style.evaluate_show(&empty_props()));
}

#[test]
fn tile_style_default_color_white() {
    let style = TileStyle::new();
    assert_eq!(style.evaluate_color(&empty_props()), [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn tile_style_default_point_size_one() {
    let style = TileStyle::new();
    assert!((style.evaluate_point_size(&empty_props()) - 1.0).abs() < 1e-10);
}

#[test]
fn tile_style_with_color_constructor() {
    let style = TileStyle::with_color([0.0, 1.0, 0.0, 0.8]);
    assert_eq!(style.evaluate_color(&empty_props()), [0.0, 1.0, 0.0, 0.8]);
}

#[test]
fn tile_style_show_conditional() {
    let style = TileStyle {
        show: Some(StyleExpression::Compare {
            left: Box::new(StyleExpression::Property("height".to_string())),
            op: CompareOp::GreaterThan,
            right: Box::new(StyleExpression::Number(50.0)),
        }),
        ..Default::default()
    };

    assert!(style.evaluate_show(&props_with_height(100.0)));
    assert!(!style.evaluate_show(&props_with_height(30.0)));
}

#[test]
fn tile_style_point_size_expression() {
    let style = TileStyle {
        point_size: Some(StyleExpression::Arithmetic {
            left: Box::new(StyleExpression::Property("height".to_string())),
            op: ArithmeticOp::Divide,
            right: Box::new(StyleExpression::Number(10.0)),
        }),
        ..Default::default()
    };

    let size = style.evaluate_point_size(&props_with_height(50.0));
    assert!((size - 5.0).abs() < 1e-10);
}

#[test]
fn tile_style_color_conditional_by_property() {
    let mut props = HashMap::new();
    props.insert("type".to_string(), PropertyValue::String("road".to_string()));

    let style = TileStyle {
        color: Some(StyleExpression::Conditional {
            condition: Box::new(StyleExpression::Compare {
                left: Box::new(StyleExpression::Property("type".to_string())),
                op: CompareOp::Equal,
                right: Box::new(StyleExpression::String("road".to_string())),
            }),
            true_expr: Box::new(StyleExpression::Color([0.5, 0.5, 0.5, 1.0])),
            false_expr: Box::new(StyleExpression::Color([0.0, 1.0, 0.0, 1.0])),
        }),
        ..Default::default()
    };

    assert_eq!(style.evaluate_color(&props), [0.5, 0.5, 0.5, 1.0]);
}

// ─── Nested / Complex Expressions ─────────────────────────────────────────

#[test]
fn nested_conditional() {
    let props = props_with_height(75.0);

    // height > 100 → red, height > 50 → green, else → blue
    let expr = StyleExpression::Conditional {
        condition: Box::new(StyleExpression::Compare {
            left: Box::new(StyleExpression::Property("height".to_string())),
            op: CompareOp::GreaterThan,
            right: Box::new(StyleExpression::Number(100.0)),
        }),
        true_expr: Box::new(StyleExpression::Color([1.0, 0.0, 0.0, 1.0])),
        false_expr: Box::new(StyleExpression::Conditional {
            condition: Box::new(StyleExpression::Compare {
                left: Box::new(StyleExpression::Property("height".to_string())),
                op: CompareOp::GreaterThan,
                right: Box::new(StyleExpression::Number(50.0)),
            }),
            true_expr: Box::new(StyleExpression::Color([0.0, 1.0, 0.0, 1.0])),
            false_expr: Box::new(StyleExpression::Color([0.0, 0.0, 1.0, 1.0])),
        }),
    };

    assert_eq!(
        expr.evaluate(&props),
        PropertyValue::Color([0.0, 1.0, 0.0, 1.0])
    );
}

#[test]
fn arithmetic_with_property_reference() {
    let props = props_with_height(100.0);

    // (height + 50) * 2
    let expr = StyleExpression::Arithmetic {
        left: Box::new(StyleExpression::Arithmetic {
            left: Box::new(StyleExpression::Property("height".to_string())),
            op: ArithmeticOp::Add,
            right: Box::new(StyleExpression::Number(50.0)),
        }),
        op: ArithmeticOp::Multiply,
        right: Box::new(StyleExpression::Number(2.0)),
    };

    assert_eq!(expr.evaluate(&props), PropertyValue::Number(300.0));
}
