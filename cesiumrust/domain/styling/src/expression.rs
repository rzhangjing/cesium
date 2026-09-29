//! 顶层 `Expression`：把 3D Tiles Styling 语言的源字符串解析为运行时
//! AST，并针对可选的 feature 求值。
//!
//! 移植自 `cesium-rs/crates/cesium-scene/src/expression.rs` L3051-3179
//! （`Expression` 结构体 + `try_new`/`new`/`expression`/`runtime_ast`/`evaluate`/
//! `evaluate_color`/`get_variables`），它是上游
//! `packages/engine/Source/Scene/Expression.js`（`Expression` 构造函数
//! 与原型）的 Rust 移植。
//!
//! # 偏离（依赖 + 延后的 codegen）
//!
//! * blueprint 的 `evaluate_color` 写入一个 `cesium_core::Color`。这个
//!   孤立的 domain crate 没有 `Color` 类型（颜色是 `glam::DVec4` rgba
//!   0..1，见 `literal.rs`），所以 `evaluate_color` 直接返回 `DVec4`。
//! * `get_shader_function` / `get_shader_expression`（GLSL codegen）**延后**
//!   到后续里程碑（Sam Q2）；这个 CPU 侧引擎只通过 [`Node::evaluate`]
//!   实现解释求值。
//! * 结构体上缓存了一份去重的 `variables` 列表（在 [`Expression::try_new`]
//!   中一次性计算），因此 [`Expression::get_variables`] 是 O(1)。

use std::collections::HashMap;

use glam::DVec4;

use crate::ast::{create_runtime_ast, Node};
use crate::parser::Parser;
use crate::runtime::ExpressionFeature;
use crate::value::{runtime_error, RuntimeError, Value};
use crate::variables::{remove_backslashes, replace_defines, replace_variables};

/// 应用于 `Cesium3DTileset` 的样式表达式。对以 3D Tiles Styling 语言定义的
/// 表达式求值。实现 `StyleExpression` 接口。
pub struct Expression {
    expression_string: String,
    runtime_ast: Node,
    variables: Vec<String>,
}

impl Expression {
    /// null 只需要是一个哨兵值，使 "[expression] === null" 在几乎所有情形下
    /// 都为 false。GLSL 没有 NaN 常量，所以用 czm_infinity。
    pub const NULL_SENTINEL: &'static str = "czm_infinity";

    /// 镜像 `new Expression(expression, defines)`；解析失败以 `Err` 返回，
    /// 而非抛出。
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

        // Pratt 解析器所镜像的 jsep 定制：addBinaryOp("=~", 0)
        // 与 addBinaryOp("!~", 0)。
        let ast = Parser::parse(&processed)?;
        let runtime_ast = create_runtime_ast(&ast)?;

        // 缓存去重后的变量列表（镜像 getVariables()）。
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

    /// 镜像 `new Expression(expression, defines)`；解析出错时 panic，
    /// 如同 JS 构造函数抛出那样。
    pub fn new(expression: &str, defines: Option<&HashMap<String, String>>) -> Expression {
        match Self::try_new(expression, defines) {
            Ok(expression) => expression,
            Err(error) => panic!("{error}"),
        }
    }

    /// 获取以 3D Tiles Styling 语言定义的表达式。
    pub fn expression(&self) -> &str {
        &self.expression_string
    }

    /// 暴露运行时 AST，镜像 spec 对私有 `_runtimeAst` 字段的访问
    /// （用于断言诸如 LITERAL_REGEX 的节点类型）。
    pub fn runtime_ast(&self) -> &Node {
        &self.runtime_ast
    }

    /// 镜像 `Expression.prototype.evaluate`。
    pub fn evaluate(
        &self,
        feature: Option<&dyn ExpressionFeature>,
    ) -> Result<Value, RuntimeError> {
        self.runtime_ast.evaluate(feature)
    }

    /// 镜像 `Expression.prototype.evaluateColor`。
    ///
    /// 偏离：以 `glam::DVec4`（分量 0..1）返回 rgba 颜色，
    /// 而非写入本 domain crate 并不依赖的 `cesium_core::Color`。
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

    /// 镜像 `Expression.prototype.getVariables`：该表达式所引用的 `${name}`
    /// 变量的去重列表。
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

    /// 解析并求值一个无 feature 的源字符串。
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

    // --- 11 个 JS 怪癖的端到端用例 -----------------------------------

    #[test]
    fn quirk_string_plus_number_concatenates() {
        // "1" + 1 == "11"（任一侧为字符串时字符串拼接优先）。
        assert_eq!(eval(r#""1" + 1"#), Value::String("11".to_string()));
        assert_eq!(eval(r#"1 + "1""#), Value::String("11".to_string()));
        // 但 数字 + 数字 是算术。
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
        // Math.round(-0.5) == 0（不是 -1）；Math.round(0.5) == 1。
        assert_eq!(num("round(-0.5)"), 0.0);
        assert_eq!(num("round(0.5)"), 1.0);
        assert_eq!(num("round(1.5)"), 2.0);
        assert_eq!(num("round(-1.5)"), -1.0);
    }

    #[test]
    fn quirk_min_max_nan_propagation() {
        // Math.min(NaN, 1) == NaN（NaN 会传播，不同于 Rust f64::min）。
        assert!(bool_("isNaN(min(NaN, 1))"));
        assert!(bool_("isNaN(max(NaN, 1))"));
        assert_eq!(num("min(1, 2)"), 1.0);
        assert_eq!(num("max(1, 2)"), 2.0);
    }

    #[test]
    fn quirk_null_loose_equals_undefined_but_not_strict() {
        // styling 语言只有 ===（严格）：null === undefined 为 false。
        assert!(!bool_("null === undefined"));
        assert!(bool_("null === null"));
        assert!(bool_("undefined === undefined"));
        // 宽松 (==) 怪癖位于 Value::equals_loose（此处没有 == 运算符）。
        assert!(Value::Null.equals_loose(&Value::Undefined));
        assert!(Value::Undefined.equals_loose(&Value::Null));
        assert!(!Value::Null.equals_strict(&Value::Undefined));
    }

    #[test]
    fn quirk_strict_string_number_not_equal() {
        // "5" === 5 为 false（严格，不做类型转换）。
        assert!(!bool_(r#""5" === 5"#));
        assert!(bool_(r#""5" === "5""#));
        // 显式类型转换后 Number("5") === 5。
        assert!(bool_(r#"Number("5") === 5"#));
        // 宽松的 "1" == 1 怪癖位于 Value::equals_loose。
        assert!(Value::String("1".into()).equals_loose(&Value::Number(1.0)));
    }

    #[test]
    fn quirk_number_of_empty_string_is_zero() {
        // JS 中 Number("") == 0（经由 number_conversion / js_parse_number）。
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
        // degrees(radians(180)) 经由基于 PI 的转换往返。
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
        // 偏离 JS：一元 + 不会把字符串做类型转换（styling 语言要求
        // 数字/向量），所以 +"" 报错；Number("") 才是产生 0 的类型转换路径。
        assert!(Expression::try_new(r#"+"""#, None)
            .unwrap()
            .evaluate(None)
            .is_err());
        assert_eq!(num(r#"Number("")"#), 0.0);
        assert_eq!(num("+5"), 5.0);
    }

    // --- Pratt 优先级 ---------------------------------------------------

    #[test]
    fn pratt_operator_precedence() {
        assert_eq!(num("2 + 3 * 4"), 14.0);
        assert_eq!(num("(2 + 3) * 4"), 20.0);
        assert_eq!(num("10 - 2 - 3"), 5.0); // 左结合
        assert_eq!(num("2 * 3 % 4"), 2.0); // * 先于 %... (2*3)=6 %4=2
        assert!(bool_("1 + 2 === 3")); // 算术先于比较
        assert!(bool_("2 < 3 && 4 < 5")); // 比较先于逻辑
        assert!(bool_("-2 < 0")); // 一元负号
    }

    #[test]
    fn pratt_ternary_conditional() {
        assert_eq!(num("true ? 1 : 2"), 1.0);
        assert_eq!(num("false ? 1 : 2"), 2.0);
        // 右结合
        assert_eq!(num("true ? false ? 1 : 2 : 3"), 2.0);
    }

    // --- 内建函数覆盖 -----------------------------------------

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
        // 逐分量算术
        assert_eq!(eval("vec2(1, 2) + vec2(3, 4)"), Value::Cartesian2(
            glam::DVec2::new(4.0, 6.0)
        ));
        assert_eq!(eval("vec3(1, 2, 3) * 2"), Value::Cartesian3(
            glam::DVec3::new(2.0, 4.0, 6.0)
        ));
    }

    // --- 正则运算符 / 函数 ---------------------------------------

    #[test]
    fn regex_test_and_match_operators() {
        assert!(bool_("regExp('^a').test('abc')"));
        assert!(!bool_("regExp('^b').test('abc')"));
        assert!(bool_("'abc' =~ regExp('b')"));
        assert!(bool_("'abc' !~ regExp('z')"));
    }

    // --- 往返：create_runtime_ast -> evaluate -------------------------

    #[test]
    fn create_runtime_ast_evaluate_roundtrip() {
        let jsep = Parser::parse("1 + 2 * 3").unwrap();
        let node = create_runtime_ast(&jsep).unwrap();
        assert_eq!(node.evaluate(None).unwrap(), Value::Number(7.0));

        let jsep = Parser::parse("color('lime').g").unwrap();
        let node = create_runtime_ast(&jsep).unwrap();
        assert_eq!(node.evaluate(None).unwrap(), Value::Number(1.0));
    }

    // --- ${...} 变量替换 + defines -----------------------------

    #[test]
    fn variable_substitution_against_feature() {
        let f = feature(&[("height", Value::Number(10.0))]);
        assert_eq!(eval_with("${height} * 2", &f), Value::Number(20.0));
        // 属性缺失 -> undefined
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
        // `${feature.<prop>}` 成员访问同样会登记属性名。
        let expr2 = Expression::try_new("${feature.height}", None).unwrap();
        assert_eq!(expr2.get_variables(), vec!["height".to_string()]);
    }

    // --- evaluate_color + Expression 访问器 ------------------------------

    #[test]
    fn evaluate_color_returns_dvec4() {
        let expr = Expression::try_new("color('red')", None).unwrap();
        assert_eq!(expr.evaluate_color(None).unwrap(), DVec4::new(1.0, 0.0, 0.0, 1.0));
        // 非颜色表达式会报错。
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
        // 未终止的变量占位符。
        assert!(Expression::try_new("${oops", None).is_err());
        // `new` 在相同输入上会 panic。
        let panicked = std::panic::catch_unwind(|| Expression::new("${oops", None)).is_err();
        assert!(panicked);
    }
}
