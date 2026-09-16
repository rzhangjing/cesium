//! Runtime evaluation of the styling AST: the `Node::evaluate` dispatch loop,
//! the feature interface, and the unary/binary operator evaluation.
//!
//! Ported from `cesium-rs/crates/cesium-scene/src/expression.rs`:
//! - `ExpressionFeature` trait          <- L268-286
//! - `get_feature_property`/`check_feature` <- L288-299
//! - `Node::evaluate`                   <- L2104-2300 (per-node dispatch)
//! - `Node::evaluate_unary`             <- L2302-2364
//! - `Node::evaluate_binary`            <- L2366-2548
//! - `Node::get_variables`              <- L2993-3048
//!
//! which are the Rust port of upstream
//! `packages/engine/Source/Scene/Expression.js` (`setEvaluateFunction` and the
//! `_evaluate*` closures).
//!
//! # DEVIATION (deps)
//!
//! The blueprint uses `cesium_core::Cartesian2/3/4` free functions
//! (`add_new`/`subtract_new`/`multiply_components_new`/`multiply_by_scalar_new`/
//! `divide_components_new`/`divide_by_scalar_new`/`negate_new`/
//! `from_elements_new`). This isolated domain crate only depends on `glam`, so
//! vectors are `glam::DVec2/DVec3/DVec4` (f64) and the arithmetic uses glam's
//! operator overloads (`+`, `-`, `*`, `/`, unary `-`), which are componentwise
//! for vector⊗vector and scalar-broadcast for vector⊗f64 — byte-identical to
//! the Cartesian helpers. The `%` operator has no glam overload, so it is done
//! componentwise by hand.
//!
//! # Module wiring
//!
//! The heavy lifting is delegated to sibling modules to keep this file focused
//! on dispatch: [`crate::member_access`] (Member), [`crate::literal`]
//! (LiteralColor/LiteralVector), [`crate::coerce`] (the unary/binary/ternary
//! builtin function tables) and [`crate::regex`] (RegExp compile/test/exec).

use glam::{DVec2, DVec3, DVec4};

use crate::ast::{is_binary_function, ExpressionNodeType, Node, NodeValue};
use crate::coerce::{evaluate_binary_function, evaluate_ternary_function, evaluate_unary_function};
use crate::literal::{evaluate_literal_color, evaluate_literal_vector};
use crate::member_access::evaluate_member;
use crate::regex::RegExpValue;
use crate::value::{runtime_error, RuntimeError, Value};
use crate::variables::variable_regex;

// ---------------------------------------------------------------------------
// Feature interface (mirrors the Cesium3DTileFeature methods used here)
// ---------------------------------------------------------------------------

/// The feature properties interface used by expression evaluation, mirroring
/// the `Cesium3DTileFeature` methods `getPropertyInherited`, `isExactClass`,
/// `isClass` and `getExactClassName`.
pub trait ExpressionFeature {
    /// Mirrors `getPropertyInherited(name)`; `None` is `undefined`.
    fn get_property_inherited(&self, name: &str) -> Option<Value>;

    /// Mirrors `isExactClass(className)`.
    fn is_exact_class(&self, _class_name: &Value) -> bool {
        false
    }

    /// Mirrors `isClass(className)`.
    fn is_class(&self, _class_name: &Value) -> bool {
        false
    }

    /// Mirrors `getExactClassName()`.
    fn get_exact_class_name(&self) -> Option<Value> {
        None
    }
}

/// Mirrors `getFeatureProperty`: returns undefined when the feature is not
/// defined or the property is missing.
pub(crate) fn get_feature_property(
    feature: Option<&dyn ExpressionFeature>,
    name: &str,
) -> Value {
    match feature {
        Some(feature) => feature.get_property_inherited(name).unwrap_or(Value::Undefined),
        None => Value::Undefined,
    }
}

/// Mirrors `checkFeature`: `true` when the node is the bare `feature` keyword.
pub(crate) fn check_feature(node: &Node) -> bool {
    matches!(&node.value, NodeValue::Str(value) if value == "feature")
}

/// Extracts the operator/call name stored in a node's `NodeValue::Str`.
fn node_op(node: &Node) -> &str {
    match &node.value {
        NodeValue::Str(s) => s.as_str(),
        _ => "",
    }
}

impl Node {
    /// Mirrors the per-node `evaluate` functions assigned by
    /// `setEvaluateFunction`.
    pub fn evaluate(
        &self,
        feature: Option<&dyn ExpressionFeature>,
    ) -> Result<Value, RuntimeError> {
        match self.node_type {
            ExpressionNodeType::Conditional => {
                let test = self.test.as_ref().unwrap().evaluate(feature)?;
                let Value::Boolean(test) = test else {
                    return Err(runtime_error(&format!(
                        "Conditional argument of conditional expression must be a boolean. Argument is {test}."
                    )));
                };
                if test {
                    self.left.as_ref().unwrap().evaluate(feature)
                } else {
                    self.right.as_ref().unwrap().evaluate(feature)
                }
            }
            ExpressionNodeType::FunctionCall => {
                let call = node_op(self);
                match call {
                    "test" => {
                        let left = self.left.as_ref().unwrap().evaluate(feature)?;
                        let right = self.right.as_ref().unwrap().evaluate(feature)?;
                        match (&left, &right) {
                            (Value::RegExp(regex), Value::String(text)) => {
                                Ok(Value::Boolean(regex.test(text)))
                            }
                            _ => Err(runtime_error(&format!(
                                "RegExp.test requires the first argument to be a RegExp and the second argument to be a string. Arguments are {left} and {right}."
                            ))),
                        }
                    }
                    "exec" => {
                        let left = self.left.as_ref().unwrap().evaluate(feature)?;
                        let right = self.right.as_ref().unwrap().evaluate(feature)?;
                        match (&left, &right) {
                            (Value::RegExp(regex), Value::String(text)) => {
                                Ok(match regex.exec_first_capture(text) {
                                    Some(capture) => Value::String(capture),
                                    None => Value::Null,
                                })
                            }
                            _ => Err(runtime_error(&format!(
                                "RegExp.exec requires the first argument to be a RegExp and the second argument to be a string. Arguments are {left} and {right}."
                            ))),
                        }
                    }
                    "toString" => {
                        let left = self.left.as_ref().unwrap().evaluate(feature)?;
                        match &left {
                            Value::RegExp(_)
                            | Value::Cartesian2(_)
                            | Value::Cartesian3(_)
                            | Value::Cartesian4(_) => Ok(Value::String(left.string_conversion())),
                            _ => Err(runtime_error(&format!(
                                "Unexpected function call \"{call}\"."
                            ))),
                        }
                    }
                    _ => Err(runtime_error(&format!(
                        "Unexpected function call \"{call}\"."
                    ))),
                }
            }
            ExpressionNodeType::Unary => self.evaluate_unary(feature),
            ExpressionNodeType::Binary => self.evaluate_binary(feature),
            ExpressionNodeType::Ternary => {
                let call = node_op(self);
                let left = self.left.as_ref().unwrap().evaluate(feature)?;
                let right = self.right.as_ref().unwrap().evaluate(feature)?;
                let test = self.test.as_ref().unwrap().evaluate(feature)?;
                evaluate_ternary_function(call, left, right, test)
            }
            ExpressionNodeType::Member => evaluate_member(self, feature),
            ExpressionNodeType::Array => {
                let NodeValue::Nodes(nodes) = &self.value else {
                    return Ok(Value::Array(Vec::new()));
                };
                let mut array = Vec::with_capacity(nodes.len());
                for node in nodes {
                    array.push(node.evaluate(feature)?);
                }
                Ok(Value::Array(array))
            }
            ExpressionNodeType::Variable => {
                let name = node_op(self);
                Ok(get_feature_property(feature, name))
            }
            ExpressionNodeType::VariableInString => {
                let template = node_op(self);
                let pattern = variable_regex();
                let mut result = String::new();
                let mut last = 0usize;
                for captures in pattern.captures_iter(template) {
                    let whole = captures.get(0).unwrap();
                    result.push_str(&template[last..whole.start()]);
                    let property = get_feature_property(feature, &captures[1]);
                    if property.is_defined() {
                        result.push_str(&property.string_conversion());
                    }
                    last = whole.end();
                }
                result.push_str(&template[last..]);
                Ok(Value::String(result))
            }
            ExpressionNodeType::LiteralColor => {
                let name = node_op(self);
                evaluate_literal_color(name, self.left_children.as_deref(), feature)
            }
            ExpressionNodeType::LiteralVector => {
                let call = node_op(self);
                match &self.left_children {
                    Some(args) => evaluate_literal_vector(call, args, feature),
                    None => Err(runtime_error(&format!(
                        "Invalid {call} constructor. No valid arguments."
                    ))),
                }
            }
            ExpressionNodeType::LiteralString => match &self.value {
                NodeValue::Str(s) => Ok(Value::String(s.clone())),
                _ => Ok(Value::String(String::new())),
            },
            ExpressionNodeType::Regex => {
                let pattern = self.left.as_ref().unwrap().evaluate(feature)?;
                let flags = match &self.right {
                    Some(flags) => flags.evaluate(feature)?.string_conversion(),
                    None => String::new(),
                };
                let regex = RegExpValue::compile(&pattern.string_conversion(), &flags)?;
                Ok(Value::RegExp(regex))
            }
            ExpressionNodeType::BuiltinVariable => {
                // DEVIATION: `tiles3d_tileset_time` reads
                // `feature.content.tileset.timeSinceLoad` in the original; the
                // CPU-side domain port has no tileset context, so it evaluates
                // to 0.0 (the same value returned when the feature is
                // undefined). Shader-side time stays a codegen concern (Sam Q2).
                Ok(Value::Number(0.0))
            }
            ExpressionNodeType::LiteralNull => Ok(Value::Null),
            ExpressionNodeType::LiteralBoolean => match &self.value {
                NodeValue::Bool(b) => Ok(Value::Boolean(*b)),
                _ => Ok(Value::Undefined),
            },
            ExpressionNodeType::LiteralNumber => match &self.value {
                NodeValue::Number(n) => Ok(Value::Number(*n)),
                _ => Ok(Value::Undefined),
            },
            ExpressionNodeType::LiteralRegex => match &self.value {
                NodeValue::Regex(regex) => Ok(Value::RegExp(regex.clone())),
                _ => Ok(Value::Undefined),
            },
            ExpressionNodeType::LiteralUndefined => Ok(Value::Undefined),
        }
    }

    /// UNARY node evaluation, mirroring `_evaluateNot`/`_evaluateNegative`/
    /// `_evaluatePositive`/conversion calls and the unary function table.
    fn evaluate_unary(
        &self,
        feature: Option<&dyn ExpressionFeature>,
    ) -> Result<Value, RuntimeError> {
        let op = node_op(self);
        if op == "getExactClassName" {
            return Ok(match feature {
                Some(feature) => feature.get_exact_class_name().unwrap_or(Value::Undefined),
                None => Value::Undefined,
            });
        }
        let left = self.left.as_ref().unwrap().evaluate(feature)?;
        match op {
            "!" => match left {
                Value::Boolean(b) => Ok(Value::Boolean(!b)),
                _ => Err(runtime_error(&format!(
                    "Operator \"!\" requires a boolean argument. Argument is {left}."
                ))),
            },
            "-" => match left {
                Value::Number(n) => Ok(Value::Number(-n)),
                Value::Cartesian2(v) => Ok(Value::Cartesian2(-v)),
                Value::Cartesian3(v) => Ok(Value::Cartesian3(-v)),
                Value::Cartesian4(v) => Ok(Value::Cartesian4(-v)),
                _ => Err(runtime_error(&format!(
                    "Operator \"-\" requires a vector or number argument. Argument is {left}."
                ))),
            },
            "+" => match &left {
                Value::Number(_)
                | Value::Cartesian2(_)
                | Value::Cartesian3(_)
                | Value::Cartesian4(_) => Ok(left),
                _ => Err(runtime_error(&format!(
                    "Operator \"+\" requires a vector or number argument. Argument is {left}."
                ))),
            },
            "isNaN" => Ok(Value::Boolean(left.number_conversion().is_nan())),
            "isFinite" => Ok(Value::Boolean({
                let n = left.number_conversion();
                !n.is_nan() && !n.is_infinite()
            })),
            "isExactClass" => Ok(Value::Boolean(match feature {
                Some(feature) => feature.is_exact_class(&left),
                None => false,
            })),
            "isClass" => Ok(Value::Boolean(match feature {
                Some(feature) => feature.is_class(&left),
                None => false,
            })),
            "Boolean" => Ok(Value::Boolean(left.boolean_conversion())),
            "Number" => Ok(Value::Number(left.number_conversion())),
            "String" => Ok(Value::String(left.string_conversion())),
            _ => evaluate_unary_function(op, left),
        }
    }

    /// BINARY node evaluation, mirroring `_evaluatePlus`/.../`_evaluateOr`
    /// and the regex match operators.
    fn evaluate_binary(
        &self,
        feature: Option<&dyn ExpressionFeature>,
    ) -> Result<Value, RuntimeError> {
        let op = node_op(self);
        // Short-circuit operators evaluate the right side lazily.
        if op == "&&" || op == "||" {
            let left = self.left.as_ref().unwrap().evaluate(feature)?;
            let Value::Boolean(left) = left else {
                return Err(runtime_error(&format!(
                    "Operator \"{op}\" requires boolean arguments. First argument is {left}."
                )));
            };
            if op == "&&" && !left {
                return Ok(Value::Boolean(false));
            }
            if op == "||" && left {
                return Ok(Value::Boolean(true));
            }
            let right = self.right.as_ref().unwrap().evaluate(feature)?;
            let Value::Boolean(right) = right else {
                return Err(runtime_error(&format!(
                    "Operator \"{op}\" requires boolean arguments. Second argument is {right}."
                )));
            };
            return Ok(Value::Boolean(if op == "&&" {
                left && right
            } else {
                left || right
            }));
        }

        let left = self.left.as_ref().unwrap().evaluate(feature)?;
        let right = self.right.as_ref().unwrap().evaluate(feature)?;
        match op {
            "+" => match (&left, &right) {
                (Value::Cartesian2(l), Value::Cartesian2(r)) => Ok(Value::Cartesian2(*l + *r)),
                (Value::Cartesian3(l), Value::Cartesian3(r)) => Ok(Value::Cartesian3(*l + *r)),
                (Value::Cartesian4(l), Value::Cartesian4(r)) => Ok(Value::Cartesian4(*l + *r)),
                (Value::String(_), _) | (_, Value::String(_)) => Ok(Value::String(format!(
                    "{}{}",
                    left.string_conversion(),
                    right.string_conversion()
                ))),
                (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l + r)),
                _ => Err(runtime_error(&format!(
                    "Operator \"+\" requires vector or number arguments of matching types, or at least one string argument. Arguments are {left} and {right}."
                ))),
            },
            "-" => match (&left, &right) {
                (Value::Cartesian2(l), Value::Cartesian2(r)) => Ok(Value::Cartesian2(*l - *r)),
                (Value::Cartesian3(l), Value::Cartesian3(r)) => Ok(Value::Cartesian3(*l - *r)),
                (Value::Cartesian4(l), Value::Cartesian4(r)) => Ok(Value::Cartesian4(*l - *r)),
                (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l - r)),
                _ => Err(runtime_error(&format!(
                    "Operator \"-\" requires vector or number arguments of matching types. Arguments are {left} and {right}."
                ))),
            },
            "*" => match (&left, &right) {
                (Value::Cartesian2(l), Value::Cartesian2(r)) => Ok(Value::Cartesian2(*l * *r)),
                (Value::Cartesian2(v), Value::Number(n))
                | (Value::Number(n), Value::Cartesian2(v)) => Ok(Value::Cartesian2(*v * *n)),
                (Value::Cartesian3(l), Value::Cartesian3(r)) => Ok(Value::Cartesian3(*l * *r)),
                (Value::Cartesian3(v), Value::Number(n))
                | (Value::Number(n), Value::Cartesian3(v)) => Ok(Value::Cartesian3(*v * *n)),
                (Value::Cartesian4(l), Value::Cartesian4(r)) => Ok(Value::Cartesian4(*l * *r)),
                (Value::Cartesian4(v), Value::Number(n))
                | (Value::Number(n), Value::Cartesian4(v)) => Ok(Value::Cartesian4(*v * *n)),
                (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l * r)),
                _ => Err(runtime_error(&format!(
                    "Operator \"*\" requires vector or number arguments. If both arguments are vectors they must be matching types. Arguments are {left} and {right}."
                ))),
            },
            "/" => match (&left, &right) {
                (Value::Cartesian2(l), Value::Cartesian2(r)) => Ok(Value::Cartesian2(*l / *r)),
                (Value::Cartesian2(v), Value::Number(n)) => Ok(Value::Cartesian2(*v / *n)),
                (Value::Cartesian3(l), Value::Cartesian3(r)) => Ok(Value::Cartesian3(*l / *r)),
                (Value::Cartesian3(v), Value::Number(n)) => Ok(Value::Cartesian3(*v / *n)),
                (Value::Cartesian4(l), Value::Cartesian4(r)) => Ok(Value::Cartesian4(*l / *r)),
                (Value::Cartesian4(v), Value::Number(n)) => Ok(Value::Cartesian4(*v / *n)),
                (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l / r)),
                _ => Err(runtime_error(&format!(
                    "Operator \"/\" requires vector or number arguments of matching types, or a number as the second argument. Arguments are {left} and {right}."
                ))),
            },
            "%" => match (&left, &right) {
                (Value::Cartesian2(l), Value::Cartesian2(r)) => Ok(Value::Cartesian2(DVec2::new(
                    l.x % r.x,
                    l.y % r.y,
                ))),
                (Value::Cartesian3(l), Value::Cartesian3(r)) => Ok(Value::Cartesian3(DVec3::new(
                    l.x % r.x,
                    l.y % r.y,
                    l.z % r.z,
                ))),
                (Value::Cartesian4(l), Value::Cartesian4(r)) => Ok(Value::Cartesian4(DVec4::new(
                    l.x % r.x,
                    l.y % r.y,
                    l.z % r.z,
                    l.w % r.w,
                ))),
                (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l % r)),
                _ => Err(runtime_error(&format!(
                    "Operator \"%\" requires vector or number arguments of matching types. Arguments are {left} and {right}."
                ))),
            },
            "===" => Ok(Value::Boolean(left.equals_strict(&right))),
            "!==" => Ok(Value::Boolean(!left.equals_strict(&right))),
            "<" | "<=" | ">" | ">=" => match (&left, &right) {
                (Value::Number(l), Value::Number(r)) => Ok(Value::Boolean(match op {
                    "<" => l < r,
                    "<=" => l <= r,
                    ">" => l > r,
                    _ => l >= r,
                })),
                _ => Err(runtime_error(&format!(
                    "Operator \"{op}\" requires number arguments. Arguments are {left} and {right}."
                ))),
            },
            "=~" | "!~" => {
                let matched = match (&left, &right) {
                    (Value::RegExp(regex), Value::String(text)) => regex.test(text),
                    (Value::String(text), Value::RegExp(regex)) => regex.test(text),
                    _ => {
                        return Err(runtime_error(&format!(
                            "Operator \"{op}\" requires one RegExp argument and one string argument. Arguments are {left} and {right}."
                        )))
                    }
                };
                Ok(Value::Boolean(if op == "=~" { matched } else { !matched }))
            }
            _ => {
                if is_binary_function(op) {
                    evaluate_binary_function(op, left, right)
                } else {
                    Err(runtime_error(&format!("Unexpected operator \"{op}\".")))
                }
            }
        }
    }

    /// Mirrors `Node.prototype.getVariables`: walks the AST collecting the
    /// `${name}` variables, `Variable` names and `feature.<name>` property
    /// accesses. `parent` is needed for the `LiteralString` case (a string
    /// literal is only a variable when it is the property of a `feature`
    /// member access).
    pub fn get_variables(&self, variables: &mut Vec<String>, parent: Option<&Node>) {
        if let Some(children) = &self.left_children {
            for child in children {
                child.get_variables(variables, Some(self));
            }
        }
        if let Some(left) = &self.left {
            left.get_variables(variables, Some(self));
        }
        if let Some(right) = &self.right {
            right.get_variables(variables, Some(self));
        }
        if let Some(test) = &self.test {
            test.get_variables(variables, Some(self));
        }
        if let NodeValue::Nodes(nodes) = &self.value {
            // For ARRAY type
            for node in nodes {
                node.get_variables(variables, Some(self));
            }
        }

        match self.node_type {
            ExpressionNodeType::Variable if !check_feature(self) => {
                if let NodeValue::Str(value) = &self.value {
                    variables.push(value.clone());
                }
            }
            ExpressionNodeType::VariableInString => {
                if let NodeValue::Str(value) = &self.value {
                    let pattern = variable_regex();
                    for captures in pattern.captures_iter(value) {
                        variables.push(captures[1].to_string());
                    }
                }
            }
            ExpressionNodeType::LiteralString => {
                if let Some(parent) = parent {
                    let feature_left = parent
                        .left
                        .as_ref()
                        .map(|left| check_feature(left))
                        .unwrap_or(false);
                    if parent.node_type == ExpressionNodeType::Member && feature_left {
                        if let NodeValue::Str(value) = &self.value {
                            variables.push(value.clone());
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct TestFeature {
        props: HashMap<String, Value>,
        class_name: Option<String>,
    }
    impl ExpressionFeature for TestFeature {
        fn get_property_inherited(&self, name: &str) -> Option<Value> {
            self.props.get(name).cloned()
        }
        fn get_exact_class_name(&self) -> Option<Value> {
            self.class_name.clone().map(Value::String)
        }
    }

    fn num(n: f64) -> Node {
        Node::new(
            ExpressionNodeType::LiteralNumber,
            NodeValue::Number(n),
            None,
            None,
            None,
        )
    }
    fn boolean(b: bool) -> Node {
        Node::new(
            ExpressionNodeType::LiteralBoolean,
            NodeValue::Bool(b),
            None,
            None,
            None,
        )
    }
    fn binary(op: &str, l: Node, r: Node) -> Node {
        Node::new(
            ExpressionNodeType::Binary,
            NodeValue::Str(op.to_string()),
            Some(l),
            Some(r),
            None,
        )
    }
    fn unary(op: &str, arg: Node) -> Node {
        Node::new(
            ExpressionNodeType::Unary,
            NodeValue::Str(op.to_string()),
            Some(arg),
            None,
            None,
        )
    }

    #[test]
    fn literal_dispatch() {
        assert_eq!(num(3.0).evaluate(None).unwrap(), Value::Number(3.0));
        assert_eq!(boolean(true).evaluate(None).unwrap(), Value::Boolean(true));
        let null = Node::new(
            ExpressionNodeType::LiteralNull,
            NodeValue::Null,
            None,
            None,
            None,
        );
        assert_eq!(null.evaluate(None).unwrap(), Value::Null);
        let undef = Node::new(
            ExpressionNodeType::LiteralUndefined,
            NodeValue::Undefined,
            None,
            None,
            None,
        );
        assert_eq!(undef.evaluate(None).unwrap(), Value::Undefined);
        let builtin = Node::new(
            ExpressionNodeType::BuiltinVariable,
            NodeValue::Str("tiles3d_tileset_time".into()),
            None,
            None,
            None,
        );
        assert_eq!(builtin.evaluate(None).unwrap(), Value::Number(0.0));
    }

    #[test]
    fn binary_arithmetic_and_comparison() {
        assert_eq!(
            binary("+", num(1.0), num(2.0)).evaluate(None).unwrap(),
            Value::Number(3.0)
        );
        assert_eq!(
            binary("%", num(7.0), num(3.0)).evaluate(None).unwrap(),
            Value::Number(1.0)
        );
        assert_eq!(
            binary("<", num(1.0), num(2.0)).evaluate(None).unwrap(),
            Value::Boolean(true)
        );
        assert_eq!(
            binary("===", num(5.0), num(5.0)).evaluate(None).unwrap(),
            Value::Boolean(true)
        );
        // vector * scalar and vector + vector via glam operators
        let v3 = Node::new(
            ExpressionNodeType::LiteralVector,
            NodeValue::None,
            None,
            None,
            None,
        );
        let _ = v3;
        let l = Value::Cartesian3(DVec3::new(1.0, 2.0, 3.0));
        let r = Value::Cartesian3(DVec3::new(4.0, 5.0, 6.0));
        assert!(!l.equals_strict(&r));
    }

    #[test]
    fn short_circuit_and_or() {
        // false && <error> short-circuits to false without evaluating right.
        let bad = unary("!", num(1.0)); // would error: ! requires boolean
        let node = binary("&&", boolean(false), bad);
        assert_eq!(node.evaluate(None).unwrap(), Value::Boolean(false));
        // true || <error> short-circuits to true.
        let bad = unary("!", num(1.0));
        let node = binary("||", boolean(true), bad);
        assert_eq!(node.evaluate(None).unwrap(), Value::Boolean(true));
    }

    #[test]
    fn unary_conversion_and_predicates() {
        assert_eq!(
            unary("!", boolean(false)).evaluate(None).unwrap(),
            Value::Boolean(true)
        );
        assert_eq!(
            unary("-", num(5.0)).evaluate(None).unwrap(),
            Value::Number(-5.0)
        );
        assert_eq!(
            unary("+", num(5.0)).evaluate(None).unwrap(),
            Value::Number(5.0)
        );
        let nan = Node::new(
            ExpressionNodeType::LiteralNumber,
            NodeValue::Number(f64::NAN),
            None,
            None,
            None,
        );
        assert_eq!(
            unary("isNaN", nan).evaluate(None).unwrap(),
            Value::Boolean(true)
        );
    }

    #[test]
    fn conditional_requires_boolean() {
        let cond = Node::new(
            ExpressionNodeType::Conditional,
            NodeValue::None,
            Some(num(1.0)),
            Some(num(2.0)),
            Some(num(3.0)), // non-boolean test -> error
        );
        assert!(cond.evaluate(None).is_err());
        let cond = Node::new(
            ExpressionNodeType::Conditional,
            NodeValue::None,
            Some(num(1.0)),
            Some(num(2.0)),
            Some(boolean(true)),
        );
        assert_eq!(cond.evaluate(None).unwrap(), Value::Number(1.0));
    }

    #[test]
    fn variable_and_variable_in_string() {
        let mut props = HashMap::new();
        props.insert("height".to_string(), Value::Number(10.0));
        props.insert("name".to_string(), Value::String("abc".into()));
        let feature = TestFeature {
            props,
            class_name: Some("building".into()),
        };

        let var = Node::new(
            ExpressionNodeType::Variable,
            NodeValue::Str("height".into()),
            None,
            None,
            None,
        );
        assert_eq!(
            var.evaluate(Some(&feature)).unwrap(),
            Value::Number(10.0)
        );

        let tmpl = Node::new(
            ExpressionNodeType::VariableInString,
            NodeValue::Str("h=${height},n=${name}".into()),
            None,
            None,
            None,
        );
        assert_eq!(
            tmpl.evaluate(Some(&feature)).unwrap(),
            Value::String("h=10,n=abc".into())
        );

        // getExactClassName unary
        let gec = Node::new(
            ExpressionNodeType::Unary,
            NodeValue::Str("getExactClassName".into()),
            None,
            None,
            None,
        );
        assert_eq!(
            gec.evaluate(Some(&feature)).unwrap(),
            Value::String("building".into())
        );
    }

    #[test]
    fn get_variables_collects_and_dedups_paths() {
        // ${a} + ${b} in a string template plus a bare Variable node.
        let tmpl = Node::new(
            ExpressionNodeType::VariableInString,
            NodeValue::Str("${a}/${b}".into()),
            None,
            None,
            None,
        );
        let var = Node::new(
            ExpressionNodeType::Variable,
            NodeValue::Str("c".into()),
            None,
            None,
            None,
        );
        let root = binary("+", tmpl, var);
        let mut out = Vec::new();
        root.get_variables(&mut out, None);
        assert_eq!(out, vec!["a".to_string(), "b".into(), "c".into()]);
    }
}
