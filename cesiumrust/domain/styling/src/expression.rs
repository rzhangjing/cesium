//! Top-level `Expression`: parse a 3D Tiles Styling language source string into
//! a runtime AST and evaluate it against an optional feature.
//!
//! Ported from `cesium-rs/crates/cesium-scene/src/expression.rs` L3051-3179
//! (`Expression` struct + `try_new`/`new`/`expression`/`runtime_ast`/`evaluate`/
//! `evaluate_color`/`get_variables`), the Rust port of upstream
//! `packages/engine/Source/Scene/Expression.js` (the `Expression` constructor
//! and prototype).
//!
//! # DEVIATION (deps + deferred codegen)
//!
//! * The blueprint's `evaluate_color` writes into a `cesium_core::Color`. This
//!   isolated domain crate has no `Color` type (colors are `glam::DVec4` rgba
//!   0..1, see `literal.rs`), so `evaluate_color` returns the `DVec4` directly.
//! * `get_shader_function` / `get_shader_expression` (GLSL codegen) are
//!   **deferred** to a later milestone (Sam Q2); this CPU-side engine only
//!   implements interpretation via [`Node::evaluate`].
//! * A cached, de-duplicated `variables` list is stored on the struct (computed
//!   once in [`Expression::try_new`]) so [`Expression::get_variables`] is O(1).

use std::collections::HashMap;

use glam::DVec4;

use crate::ast::{create_runtime_ast, Node};
use crate::parser::Parser;
use crate::runtime::ExpressionFeature;
use crate::value::{runtime_error, RuntimeError, Value};
use crate::variables::{remove_backslashes, replace_defines, replace_variables};

/// An expression for a style applied to a `Cesium3DTileset`. Evaluates an
/// expression defined using the 3D Tiles Styling language. Implements the
/// `StyleExpression` interface.
pub struct Expression {
    expression_string: String,
    runtime_ast: Node,
    variables: Vec<String>,
}

impl Expression {
    /// null just needs to be some sentinel value that will cause
    /// "[expression] === null" to be false in nearly all cases. GLSL doesn't
    /// have a NaN constant so use czm_infinity.
    pub const NULL_SENTINEL: &'static str = "czm_infinity";

    /// Mirrors `new Expression(expression, defines)`; parse failures are
    /// returned as `Err` instead of being thrown.
    pub fn try_new(
        expression: &str,
        defines: Option<&HashMap<String, String>>,
    ) -> Result<Expression, RuntimeError> {
        let expression_string = expression.to_string();
        let mut processed = expression.to_string();
        if let Some(defines) = defines {
            processed = replace_defines(&processed, defines);
        }
        processed = replace_variables(&remove_backslashes(&processed))?;

        // jsep customization mirrored by the Pratt parser: addBinaryOp("=~", 0)
        // and addBinaryOp("!~", 0).
        let ast = Parser::parse(&processed)?;
        let runtime_ast = create_runtime_ast(&ast)?;

        // Cache the de-duplicated variable list (mirrors getVariables()).
        let mut variables = Vec::new();
        runtime_ast.get_variables(&mut variables, None);
        let mut deduped: Vec<String> = Vec::with_capacity(variables.len());
        for variable in variables.drain(..) {
            if !deduped.contains(&variable) {
                deduped.push(variable);
            }
        }

        Ok(Expression {
            expression_string,
            runtime_ast,
            variables: deduped,
        })
    }

    /// Mirrors `new Expression(expression, defines)`; panics on parse
    /// errors, like the JS constructor throws.
    pub fn new(expression: &str, defines: Option<&HashMap<String, String>>) -> Expression {
        match Self::try_new(expression, defines) {
            Ok(expression) => expression,
            Err(error) => panic!("{error}"),
        }
    }

    /// Gets the expression defined in the 3D Tiles Styling language.
    pub fn expression(&self) -> &str {
        &self.expression_string
    }

    /// Exposes the runtime AST, mirroring the spec's access to the private
    /// `_runtimeAst` field (used to assert node types such as LITERAL_REGEX).
    pub fn runtime_ast(&self) -> &Node {
        &self.runtime_ast
    }

    /// Mirrors `Expression.prototype.evaluate`.
    pub fn evaluate(
        &self,
        feature: Option<&dyn ExpressionFeature>,
    ) -> Result<Value, RuntimeError> {
        self.runtime_ast.evaluate(feature)
    }

    /// Mirrors `Expression.prototype.evaluateColor`.
    ///
    /// DEVIATION: returns the rgba color as a `glam::DVec4` (components 0..1)
    /// rather than writing into a `cesium_core::Color`, which this domain crate
    /// does not depend on.
    pub fn evaluate_color(
        &self,
        feature: Option<&dyn ExpressionFeature>,
    ) -> Result<DVec4, RuntimeError> {
        match self.runtime_ast.evaluate(feature)? {
            Value::Cartesian4(color) => Ok(color),
            other => Err(runtime_error(&format!(
                "Expression does not evaluate to a color. Result is {other}."
            ))),
        }
    }

    /// Mirrors `Expression.prototype.getVariables`: the de-duplicated list of
    /// `${name}` variables referenced by the expression.
    pub fn get_variables(&self) -> Vec<String> {
        self.variables.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::ExpressionNodeType;
    use std::collections::HashMap;

    struct TestFeature {
        props: HashMap<String, Value>,
    }
    impl ExpressionFeature for TestFeature {
        fn get_property_inherited(&self, name: &str) -> Option<Value> {
            self.props.get(name).cloned()
        }
    }
    fn feature(pairs: &[(&str, Value)]) -> TestFeature {
        TestFeature {
            props: pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        }
    }

    /// Parse + evaluate a source string with no feature.
    fn eval(src: &str) -> Value {
        Expression::try_new(src, None)
            .unwrap_or_else(|e| panic!("parse {src:?}: {e}"))
            .evaluate(None)
            .unwrap_or_else(|e| panic!("eval {src:?}: {e}"))
    }
    fn eval_with(src: &str, f: &dyn ExpressionFeature) -> Value {
        Expression::try_new(src, None)
            .unwrap()
            .evaluate(Some(f))
            .unwrap_or_else(|e| panic!("eval {src:?}: {e}"))
    }
    fn num(src: &str) -> f64 {
        match eval(src) {
            Value::Number(n) => n,
            other => panic!("{src:?} not a number: {other}"),
        }
    }
    fn bool_(src: &str) -> bool {
        match eval(src) {
            Value::Boolean(b) => b,
            other => panic!("{src:?} not a boolean: {other}"),
        }
    }

    // --- The 11 JS-quirk end-to-end cases -----------------------------------

    #[test]
    fn quirk_string_plus_number_concatenates() {
        // "1" + 1 == "11" (string concat wins when either side is a string).
        assert_eq!(eval(r#""1" + 1"#), Value::String("11".to_string()));
        assert_eq!(eval(r#"1 + "1""#), Value::String("11".to_string()));
        // But number + number is arithmetic.
        assert_eq!(num("1 + 1"), 2.0);
    }

    #[test]
    fn quirk_nan_not_equal_to_itself() {
        assert!(bool_("NaN !== NaN"));
        assert!(!bool_("NaN === NaN"));
        assert!(bool_("isNaN(NaN)"));
        assert!(bool_("(0 / 0) !== (0 / 0)"));
    }

    #[test]
    fn quirk_round_half_towards_positive_infinity() {
        // Math.round(-0.5) == 0 (not -1); Math.round(0.5) == 1.
        assert_eq!(num("round(-0.5)"), 0.0);
        assert_eq!(num("round(0.5)"), 1.0);
        assert_eq!(num("round(1.5)"), 2.0);
        assert_eq!(num("round(-1.5)"), -1.0);
    }

    #[test]
    fn quirk_min_max_nan_propagation() {
        // Math.min(NaN, 1) == NaN (NaN propagates, unlike Rust f64::min).
        assert!(bool_("isNaN(min(NaN, 1))"));
        assert!(bool_("isNaN(max(NaN, 1))"));
        assert_eq!(num("min(1, 2)"), 1.0);
        assert_eq!(num("max(1, 2)"), 2.0);
    }

    #[test]
    fn quirk_null_loose_equals_undefined_but_not_strict() {
        // The styling language only has === (strict): null === undefined is false.
        assert!(!bool_("null === undefined"));
        assert!(bool_("null === null"));
        assert!(bool_("undefined === undefined"));
        // The loose (==) quirk lives on Value::equals_loose (no == operator here).
        assert!(Value::Null.equals_loose(&Value::Undefined));
        assert!(Value::Undefined.equals_loose(&Value::Null));
        assert!(!Value::Null.equals_strict(&Value::Undefined));
    }

    #[test]
    fn quirk_strict_string_number_not_equal() {
        // "5" === 5 is false (strict, no coercion).
        assert!(!bool_(r#""5" === 5"#));
        assert!(bool_(r#""5" === "5""#));
        // Number("5") === 5 after explicit coercion.
        assert!(bool_(r#"Number("5") === 5"#));
        // The loose "1" == 1 quirk is on Value::equals_loose.
        assert!(Value::String("1".into()).equals_loose(&Value::Number(1.0)));
    }

    #[test]
    fn quirk_number_of_empty_string_is_zero() {
        // Number("") == 0 in JS (via number_conversion / js_parse_number).
        assert_eq!(num(r#"Number("")"#), 0.0);
        assert!(bool_(r#"Number("") === 0"#));
        assert!(bool_(r#"Number("abc") !== Number("abc")"#)); // NaN
    }

    #[test]
    fn quirk_math_pi_and_constants() {
        let pi = num("Math.PI");
        assert!((pi - std::f64::consts::PI).abs() < 1e-15);
        assert_eq!(num("Infinity"), f64::INFINITY);
        assert!(bool_("isNaN(NaN)"));
        // degrees(radians(180)) round-trips through the PI-based conversions.
        assert!((num("degrees(radians(180))") - 180.0).abs() < 1e-12);
    }

    #[test]
    fn quirk_color_red_components() {
        // color("red").r == 1.0, .g == 0, .b == 0, .a == 1.
        assert_eq!(num("color('red').r"), 1.0);
        assert_eq!(num("color('red').g"), 0.0);
        assert_eq!(num("color('red').b"), 0.0);
        assert_eq!(num("color('red').a"), 1.0);
        assert!(bool_("color('red').r === 1.0"));
    }

    #[test]
    fn quirk_unary_plus_requires_number() {
        // DEVIATION from JS: unary + does NOT coerce a string (the styling
        // language requires a number/vector), so +"" errors; Number("") is the
        // coercion path that yields 0.
        assert!(Expression::try_new(r#"+"""#, None)
            .unwrap()
            .evaluate(None)
            .is_err());
        assert_eq!(num(r#"Number("")"#), 0.0);
        assert_eq!(num("+5"), 5.0);
    }

    // --- Pratt precedence ---------------------------------------------------

    #[test]
    fn pratt_operator_precedence() {
        assert_eq!(num("2 + 3 * 4"), 14.0);
        assert_eq!(num("(2 + 3) * 4"), 20.0);
        assert_eq!(num("10 - 2 - 3"), 5.0); // left assoc
        assert_eq!(num("2 * 3 % 4"), 2.0); // * before %... (2*3)=6 %4=2
        assert!(bool_("1 + 2 === 3")); // arithmetic before comparison
        assert!(bool_("2 < 3 && 4 < 5")); // comparison before logical
        assert!(bool_("-2 < 0")); // unary minus
    }

    #[test]
    fn pratt_ternary_conditional() {
        assert_eq!(num("true ? 1 : 2"), 1.0);
        assert_eq!(num("false ? 1 : 2"), 2.0);
        // right associative
        assert_eq!(num("true ? false ? 1 : 2 : 3"), 2.0);
    }

    // --- Builtin function coverage -----------------------------------------

    #[test]
    fn builtin_unary_functions() {
        assert_eq!(num("abs(-3)"), 3.0);
        assert_eq!(num("sqrt(9)"), 3.0);
        assert_eq!(num("floor(1.7)"), 1.0);
        assert_eq!(num("ceil(1.2)"), 2.0);
        assert_eq!(num("sign(-5)"), -1.0);
        assert!((num("exp(0)") - 1.0).abs() < 1e-12);
        assert_eq!(num("exp2(3)"), 8.0);
        assert!((num("log2(8)") - 3.0).abs() < 1e-12);
        assert!((num("fract(1.25)") - 0.25).abs() < 1e-12);
        assert!((num("sin(0)")).abs() < 1e-12);
        assert!((num("cos(0)") - 1.0).abs() < 1e-12);
    }

    #[test]
    fn builtin_binary_and_ternary_functions() {
        assert_eq!(num("pow(2, 10)"), 1024.0);
        assert_eq!(num("atan2(0, 1)"), 0.0);
        assert_eq!(num("min(3, 7)"), 3.0);
        assert_eq!(num("max(3, 7)"), 7.0);
        assert_eq!(num("clamp(5, 0, 3)"), 3.0);
        assert_eq!(num("clamp(-5, 0, 3)"), 0.0);
        assert!((num("mix(0, 10, 0.5)") - 5.0).abs() < 1e-12);
        assert_eq!(num("dot(vec3(1, 0, 0), vec3(0, 1, 0))"), 0.0);
        assert_eq!(num("dot(vec3(1, 2, 3), vec3(1, 2, 3))"), 14.0);
    }

    #[test]
    fn vector_literals_and_components() {
        assert_eq!(num("vec3(1, 2, 3).z"), 3.0);
        assert_eq!(num("vec2(4, 5).x"), 4.0);
        assert_eq!(num("vec4(1, 2, 3, 4).w"), 4.0);
        assert_eq!(num("length(vec3(3, 4, 0))"), 5.0);
        // componentwise arithmetic
        assert_eq!(eval("vec2(1, 2) + vec2(3, 4)"), Value::Cartesian2(
            glam::DVec2::new(4.0, 6.0)
        ));
        assert_eq!(eval("vec3(1, 2, 3) * 2"), Value::Cartesian3(
            glam::DVec3::new(2.0, 4.0, 6.0)
        ));
    }

    // --- Regex operators / functions ---------------------------------------

    #[test]
    fn regex_test_and_match_operators() {
        assert!(bool_("regExp('^a').test('abc')"));
        assert!(!bool_("regExp('^b').test('abc')"));
        assert!(bool_("'abc' =~ regExp('b')"));
        assert!(bool_("'abc' !~ regExp('z')"));
    }

    // --- Round-trip: create_runtime_ast -> evaluate -------------------------

    #[test]
    fn create_runtime_ast_evaluate_roundtrip() {
        let jsep = Parser::parse("1 + 2 * 3").unwrap();
        let node = create_runtime_ast(&jsep).unwrap();
        assert_eq!(node.evaluate(None).unwrap(), Value::Number(7.0));

        let jsep = Parser::parse("color('lime').g").unwrap();
        let node = create_runtime_ast(&jsep).unwrap();
        assert_eq!(node.evaluate(None).unwrap(), Value::Number(1.0));
    }

    // --- ${...} variable substitution + defines -----------------------------

    #[test]
    fn variable_substitution_against_feature() {
        let f = feature(&[("height", Value::Number(10.0))]);
        assert_eq!(eval_with("${height} * 2", &f), Value::Number(20.0));
        // missing property -> undefined
        assert_eq!(eval_with("${missing}", &f), Value::Undefined);
    }

    #[test]
    fn variable_in_string_interpolation() {
        let f = feature(&[("name", Value::String("abc".into()))]);
        assert_eq!(
            eval_with("'x=${name}!'", &f),
            Value::String("x=abc!".to_string())
        );
    }

    #[test]
    fn defines_are_expanded_before_parse() {
        let mut defines = HashMap::new();
        defines.insert("x".to_string(), "1 + 2".to_string());
        let expr = Expression::try_new("${x} * 3", Some(&defines)).unwrap();
        assert_eq!(expr.evaluate(None).unwrap(), Value::Number(9.0));
    }

    #[test]
    fn get_variables_lists_referenced_names() {
        let expr = Expression::try_new("${a} + ${b} * ${a}", None).unwrap();
        let mut vars = expr.get_variables();
        vars.sort();
        assert_eq!(vars, vec!["a".to_string(), "b".to_string()]);
        // `${feature.<prop>}` member access also registers the property name.
        let expr2 = Expression::try_new("${feature.height}", None).unwrap();
        assert_eq!(expr2.get_variables(), vec!["height".to_string()]);
    }

    // --- evaluate_color + Expression accessors ------------------------------

    #[test]
    fn evaluate_color_returns_dvec4() {
        let expr = Expression::try_new("color('red')", None).unwrap();
        assert_eq!(expr.evaluate_color(None).unwrap(), DVec4::new(1.0, 0.0, 0.0, 1.0));
        // A non-color expression errors.
        let expr = Expression::try_new("1 + 1", None).unwrap();
        assert!(expr.evaluate_color(None).is_err());
    }

    #[test]
    fn expression_accessors_and_node_type() {
        let expr = Expression::try_new("color('red')", None).unwrap();
        assert_eq!(expr.expression(), "color('red')");
        assert_eq!(expr.runtime_ast().node_type, ExpressionNodeType::LiteralColor);
        assert_eq!(Expression::NULL_SENTINEL, "czm_infinity");
    }

    #[test]
    fn try_new_reports_parse_errors() {
        // Unterminated variable placeholder.
        assert!(Expression::try_new("${oops", None).is_err());
        // `new` panics on the same input.
        let panicked = std::panic::catch_unwind(|| Expression::new("${oops", None)).is_err();
        assert!(panicked);
    }
}
